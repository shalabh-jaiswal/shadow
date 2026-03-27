---
name: provider-patterns
description: |
  BackupProvider implementations and upload strategies for Shadow. Activates
  when implementing or modifying S3, GCS, or NAS providers, writing upload
  logic, handling multipart uploads, resumable uploads, large file chunking,
  credential chain setup, or test_connection implementations.
allowed-tools:
  - Read
---

# Provider Patterns for Shadow

## BackupProvider Trait (source of truth)

```rust
// src-tauri/src/providers/mod.rs
use async_trait::async_trait;
use std::path::Path;
use anyhow::Result;

#[async_trait]
pub trait BackupProvider: Send + Sync {
    fn name(&self) -> &'static str;
    async fn upload(&self, local_path: &Path, remote_key: &str) -> Result<()>;
    async fn test_connection(&self) -> Result<String>;
}

// Threshold for switching to multipart / resumable upload
pub const MULTIPART_THRESHOLD_BYTES: u64 = 10 * 1024 * 1024; // 10MB
pub const MULTIPART_CHUNK_BYTES: usize  =  8 * 1024 * 1024;  //  8MB
```

## S3 Provider

```rust
// src-tauri/src/providers/s3.rs
use aws_sdk_s3::{Client, primitives::ByteStream};
use aws_sdk_s3::operation::create_multipart_upload::CreateMultipartUploadOutput;
use std::path::Path;
use anyhow::{Context, Result};
use tokio::fs;
use super::{BackupProvider, MULTIPART_THRESHOLD_BYTES, MULTIPART_CHUNK_BYTES};

pub struct S3Provider {
    client: Client,
    bucket: String,
}

impl S3Provider {
    pub async fn new(bucket: String, region: String, endpoint: Option<String>) -> Result<Self> {
        // Uses standard AWS credential chain — env vars, ~/.aws/credentials, IAM role
        let mut loader = aws_config::from_env().region(
            aws_sdk_s3::config::Region::new(region)
        );
        if let Some(ep) = endpoint {
            loader = loader.endpoint_url(ep);
        }
        let config = loader.load().await;
        let client = Client::new(&config);
        Ok(Self { client, bucket })
    }
}

#[async_trait::async_trait]
impl BackupProvider for S3Provider {
    fn name(&self) -> &'static str { "S3" }

    async fn upload(&self, local_path: &Path, remote_key: &str) -> Result<()> {
        let metadata = fs::metadata(local_path).await?;
        if metadata.len() > MULTIPART_THRESHOLD_BYTES {
            self.multipart_upload(local_path, remote_key).await
        } else {
            self.simple_upload(local_path, remote_key).await
        }
    }

    async fn test_connection(&self) -> Result<String> {
        self.client
            .head_bucket()
            .bucket(&self.bucket)
            .send()
            .await
            .with_context(|| format!("Cannot access S3 bucket: {}", self.bucket))?;
        Ok(format!("Connected to S3 bucket: {}", self.bucket))
    }
}

impl S3Provider {
    async fn simple_upload(&self, path: &Path, key: &str) -> Result<()> {
        let body = ByteStream::from_path(path).await?;
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(body)
            .send()
            .await
            .with_context(|| format!("S3 PutObject failed for key: {key}"))?;
        Ok(())
    }

    async fn multipart_upload(&self, path: &Path, key: &str) -> Result<()> {
        // 1. Initiate
        let create: CreateMultipartUploadOutput = self.client
            .create_multipart_upload()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await?;
        let upload_id = create.upload_id().unwrap_or_default().to_string();

        // 2. Upload parts
        let data = fs::read(path).await?;
        let mut parts = Vec::new();
        for (i, chunk) in data.chunks(MULTIPART_CHUNK_BYTES).enumerate() {
            let part_num = (i + 1) as i32;
            let result = self.client
                .upload_part()
                .bucket(&self.bucket)
                .key(key)
                .upload_id(&upload_id)
                .part_number(part_num)
                .body(ByteStream::from(chunk.to_vec()))
                .send()
                .await?;
            parts.push(
                aws_sdk_s3::types::CompletedPart::builder()
                    .part_number(part_num)
                    .e_tag(result.e_tag().unwrap_or_default())
                    .build()
            );
        }

        // 3. Complete
        self.client
            .complete_multipart_upload()
            .bucket(&self.bucket)
            .key(key)
            .upload_id(&upload_id)
            .multipart_upload(
                aws_sdk_s3::types::CompletedMultipartUpload::builder()
                    .set_parts(Some(parts))
                    .build()
            )
            .send()
            .await
            .with_context(|| format!("S3 CompleteMultipartUpload failed for key: {key}"))?;
        Ok(())
    }
}
```

## GCS Provider

```rust
// src-tauri/src/providers/gcs.rs
use google_cloud_storage::client::{Client, ClientConfig};
use google_cloud_storage::http::objects::upload::{UploadObjectRequest, UploadType, Media};
use std::path::Path;
use anyhow::{Context, Result};
use tokio::fs;
use super::{BackupProvider, MULTIPART_THRESHOLD_BYTES};

pub struct GcsProvider {
    client: Client,
    bucket: String,
}

impl GcsProvider {
    pub async fn new(bucket: String, credentials_path: Option<String>) -> Result<Self> {
        // Uses Application Default Credentials if credentials_path is None
        let config = if let Some(path) = credentials_path {
            ClientConfig::default()
                .with_credentials(
                    google_cloud_storage::client::google_cloud_auth::credentials::CredentialsFile::new_from_file(path).await?
                )
                .await?
        } else {
            ClientConfig::default().with_auth().await?
        };
        let client = Client::new(config);
        Ok(Self { client, bucket })
    }
}

#[async_trait::async_trait]
impl BackupProvider for GcsProvider {
    fn name(&self) -> &'static str { "GCS" }

    async fn upload(&self, local_path: &Path, remote_key: &str) -> Result<()> {
        let data = fs::read(local_path).await
            .with_context(|| format!("Failed to read: {}", local_path.display()))?;

        let upload_type = if data.len() as u64 > MULTIPART_THRESHOLD_BYTES {
            UploadType::Resumable(Media::new(remote_key.to_string()))
        } else {
            UploadType::Simple(Media::new(remote_key.to_string()))
        };

        self.client
            .upload_object(
                &UploadObjectRequest { bucket: self.bucket.clone(), ..Default::default() },
                data,
                &upload_type,
            )
            .await
            .with_context(|| format!("GCS upload failed for key: {remote_key}"))?;
        Ok(())
    }

    async fn test_connection(&self) -> Result<String> {
        use google_cloud_storage::http::buckets::get::GetBucketRequest;
        self.client
            .get_bucket(&GetBucketRequest { bucket: self.bucket.clone(), ..Default::default() })
            .await
            .with_context(|| format!("Cannot access GCS bucket: {}", self.bucket))?;
        Ok(format!("Connected to GCS bucket: {}", self.bucket))
    }
}
```

## NAS Provider

```rust
// src-tauri/src/providers/nas.rs
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use super::BackupProvider;

pub struct NasProvider {
    mount_path: PathBuf,
}

impl NasProvider {
    pub fn new(mount_path: String) -> Self {
        Self { mount_path: PathBuf::from(mount_path) }
    }

    fn destination(&self, remote_key: &str) -> PathBuf {
        // remote_key already contains bucket/hostname/path
        // NAS root replaces the bucket prefix
        self.mount_path.join(remote_key)
    }
}

#[async_trait::async_trait]
impl BackupProvider for NasProvider {
    fn name(&self) -> &'static str { "NAS" }

    async fn upload(&self, local_path: &Path, remote_key: &str) -> Result<()> {
        let dest = self.destination(remote_key);

        // Create parent directories
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await
                .with_context(|| format!("Failed to create NAS directory: {}", parent.display()))?;
        }

        // Stream copy with 1MB buffer
        let local = local_path.to_owned();
        let dest_clone = dest.clone();
        tokio::task::spawn_blocking(move || {
            let mut src = std::fs::File::open(&local)
                .with_context(|| format!("Cannot open source: {}", local.display()))?;
            let mut dst = std::fs::File::create(&dest_clone)
                .with_context(|| format!("Cannot create dest: {}", dest_clone.display()))?;
            let mut buf = vec![0u8; 1024 * 1024]; // 1MB buffer
            loop {
                use std::io::{Read, Write};
                let n = src.read(&mut buf)?;
                if n == 0 { break; }
                dst.write_all(&buf[..n])?;
            }
            Ok::<(), anyhow::Error>(())
        })
        .await??;

        Ok(())
    }

    async fn test_connection(&self) -> Result<String> {
        if !self.mount_path.exists() {
            anyhow::bail!("NAS mount path does not exist: {}", self.mount_path.display());
        }
        // Check write access with a probe file
        let probe = self.mount_path.join(".shadow_probe");
        tokio::fs::write(&probe, b"probe").await
            .with_context(|| "NAS mount path is not writable")?;
        tokio::fs::remove_file(&probe).await.ok();
        Ok(format!("NAS mount accessible: {}", self.mount_path.display()))
    }
}
```

## mod.rs — Provider Registry

```rust
// src-tauri/src/providers/mod.rs
pub mod s3;
pub mod gcs;
pub mod nas;

pub use s3::S3Provider;
pub use gcs::GcsProvider;
pub use nas::NasProvider;

use async_trait::async_trait;
use std::path::Path;
use anyhow::Result;
use std::sync::Arc;

pub const MULTIPART_THRESHOLD_BYTES: u64 = 10 * 1024 * 1024;
pub const MULTIPART_CHUNK_BYTES: usize  =  8 * 1024 * 1024;

#[async_trait]
pub trait BackupProvider: Send + Sync {
    fn name(&self) -> &'static str;
    async fn upload(&self, local_path: &Path, remote_key: &str) -> Result<()>;
    async fn test_connection(&self) -> Result<String>;
}

/// Upload to all providers in parallel; returns results per provider
pub async fn upload_all(
    providers: &[Arc<dyn BackupProvider>],
    local_path: &Path,
    remote_key: &str,
) -> Vec<(&'static str, Result<()>)> {
    let futures: Vec<_> = providers
        .iter()
        .map(|p| {
            let name = p.name();
            let fut = p.upload(local_path, remote_key);
            async move { (name, fut.await) }
        })
        .collect();
    futures::future::join_all(futures).await
}
```
