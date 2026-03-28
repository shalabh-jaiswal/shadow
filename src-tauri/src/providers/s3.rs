use crate::providers::BackupProvider;
use anyhow::{Context, Result};
use aws_config::BehaviorVersion;
use aws_sdk_s3::config::Region;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::{CompletedMultipartUpload, CompletedPart};
use bytes::Bytes;
use std::path::Path;
use tokio::io::AsyncReadExt;

/// Files smaller than this are uploaded in a single PutObject request.
/// Files at or above this threshold use the multipart upload API.
const MULTIPART_THRESHOLD: u64 = 5 * 1024 * 1024; // 5 MB
const PART_SIZE: usize = 5 * 1024 * 1024; // 5 MB per part (minimum allowed by S3)

pub struct S3Provider {
    client: aws_sdk_s3::Client,
    bucket: String,
}

impl S3Provider {
    /// Build an S3 client from the named AWS credentials profile.
    /// Credentials are read from `~/.aws/credentials` under `[<profile>]`.
    pub async fn new(region: &str, bucket: &str, profile: &str) -> Result<Self> {
        let sdk_config = aws_config::defaults(BehaviorVersion::latest())
            .profile_name(profile)
            .region(Region::new(region.to_string()))
            .load()
            .await;
        let client = aws_sdk_s3::Client::new(&sdk_config);
        Ok(Self {
            client,
            bucket: bucket.to_string(),
        })
    }
}

#[async_trait::async_trait]
impl BackupProvider for S3Provider {
    fn name(&self) -> &'static str {
        "s3"
    }

    async fn upload(&self, local_path: &Path, remote_key: &str) -> Result<()> {
        let meta = tokio::fs::metadata(local_path)
            .await
            .with_context(|| format!("cannot stat {}", local_path.display()))?;

        if meta.len() < MULTIPART_THRESHOLD {
            self.upload_single(local_path, remote_key).await
        } else {
            self.upload_multipart(local_path, remote_key).await
        }
    }

    async fn test_connection(&self) -> Result<String> {
        self.client
            .head_bucket()
            .bucket(&self.bucket)
            .send()
            .await
            .with_context(|| {
                format!(
                    "S3 bucket '{}' not accessible — check bucket name, region, and credentials",
                    self.bucket
                )
            })?;
        Ok(format!("S3 OK: {}", self.bucket))
    }
}

impl S3Provider {
    async fn upload_single(&self, local_path: &Path, remote_key: &str) -> Result<()> {
        let stream = ByteStream::from_path(local_path)
            .await
            .with_context(|| format!("failed to open {} for S3 upload", local_path.display()))?;

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(remote_key)
            .body(stream)
            .send()
            .await
            .with_context(|| format!("S3 PutObject failed for key '{remote_key}'"))?;

        Ok(())
    }

    async fn upload_multipart(&self, local_path: &Path, remote_key: &str) -> Result<()> {
        // Create the multipart upload session
        let create = self
            .client
            .create_multipart_upload()
            .bucket(&self.bucket)
            .key(remote_key)
            .send()
            .await
            .context("failed to create S3 multipart upload")?;

        let upload_id = create
            .upload_id()
            .context("S3 did not return an upload_id")?
            .to_string();

        // Upload parts; abort the session on any error
        let result = self
            .upload_parts(local_path, remote_key, &upload_id)
            .await;

        match result {
            Ok(parts) => {
                let completed = CompletedMultipartUpload::builder()
                    .set_parts(Some(parts))
                    .build();

                self.client
                    .complete_multipart_upload()
                    .bucket(&self.bucket)
                    .key(remote_key)
                    .upload_id(&upload_id)
                    .multipart_upload(completed)
                    .send()
                    .await
                    .context("failed to complete S3 multipart upload")?;

                Ok(())
            }
            Err(e) => {
                let _ = self
                    .client
                    .abort_multipart_upload()
                    .bucket(&self.bucket)
                    .key(remote_key)
                    .upload_id(&upload_id)
                    .send()
                    .await;
                Err(e)
            }
        }
    }

    async fn upload_parts(
        &self,
        local_path: &Path,
        remote_key: &str,
        upload_id: &str,
    ) -> Result<Vec<CompletedPart>> {
        let mut file = tokio::fs::File::open(local_path)
            .await
            .with_context(|| format!("failed to open {} for multipart upload", local_path.display()))?;

        let mut parts: Vec<CompletedPart> = Vec::new();
        let mut part_number = 1i32;

        loop {
            // Read up to PART_SIZE bytes
            let mut chunk: Vec<u8> = Vec::with_capacity(PART_SIZE);
            (&mut file)
                .take(PART_SIZE as u64)
                .read_to_end(&mut chunk)
                .await
                .context("error reading file chunk for S3 multipart")?;

            if chunk.is_empty() {
                break;
            }

            let part_resp = self
                .client
                .upload_part()
                .bucket(&self.bucket)
                .key(remote_key)
                .upload_id(upload_id)
                .part_number(part_number)
                .body(ByteStream::from(Bytes::from(chunk)))
                .send()
                .await
                .with_context(|| format!("S3 upload_part {part_number} failed"))?;

            parts.push(
                CompletedPart::builder()
                    .e_tag(part_resp.e_tag().unwrap_or_default())
                    .part_number(part_number)
                    .build(),
            );
            part_number += 1;
        }

        Ok(parts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multipart_threshold_is_5mb() {
        assert_eq!(MULTIPART_THRESHOLD, 5 * 1024 * 1024);
        assert_eq!(PART_SIZE, 5 * 1024 * 1024);
    }
}
