use crate::providers::BackupProvider;
use anyhow::{Context, Result};
use google_cloud_auth::credentials::CredentialsFile;
use google_cloud_storage::client::{Client, ClientConfig};
use google_cloud_storage::http::buckets::get::GetBucketRequest;
use google_cloud_storage::http::objects::upload::{Media, UploadObjectRequest, UploadType};
use google_cloud_storage::http::objects::Object;
use std::path::Path;

/// Files smaller than this use GCS simple upload.
/// Files at or above this threshold use the GCS multipart upload protocol.
const RESUMABLE_THRESHOLD: u64 = 5 * 1024 * 1024; // 5 MB

pub struct GcsProvider {
    client: Client,
    bucket: String,
}

impl GcsProvider {
    /// Build a GCS client from a service account JSON key file.
    /// `credentials_path` must be the absolute path to the JSON key file
    /// downloaded from the GCP console (config: `[gcs] credentials_path`).
    pub async fn new(bucket: &str, credentials_path: &str) -> Result<Self> {
        let creds: CredentialsFile = CredentialsFile::new_from_file(credentials_path.to_string())
            .await
            .with_context(|| {
                format!("failed to load GCS credentials from '{credentials_path}'")
            })?;
        let config = ClientConfig::default()
            .with_credentials(creds)
            .await
            .context("failed to initialize GCS client")?;
        let client = Client::new(config);
        Ok(Self {
            client,
            bucket: bucket.to_string(),
        })
    }
}

#[async_trait::async_trait]
impl BackupProvider for GcsProvider {
    fn name(&self) -> &'static str {
        "gcs"
    }

    async fn upload(&self, local_path: &Path, remote_key: &str) -> Result<()> {
        let meta = tokio::fs::metadata(local_path)
            .await
            .with_context(|| format!("cannot stat {}", local_path.display()))?;

        let data = tokio::fs::read(local_path)
            .await
            .with_context(|| format!("failed to read {} for GCS upload", local_path.display()))?;

        let upload_type = if meta.len() < RESUMABLE_THRESHOLD {
            // Simple upload: single HTTP request, no metadata.
            UploadType::Simple(Media::new(remote_key.to_string()))
        } else {
            // Multipart upload: data + object metadata in one request.
            // Recommended by GCS for files above 5 MB.
            UploadType::Multipart(Box::new(Object {
                name: remote_key.to_string(),
                ..Default::default()
            }))
        };

        let req = UploadObjectRequest {
            bucket: self.bucket.clone(),
            ..Default::default()
        };

        self.client
            .upload_object(&req, data, &upload_type)
            .await
            .with_context(|| format!("GCS upload failed for key '{remote_key}'"))?;

        Ok(())
    }

    async fn test_connection(&self) -> Result<String> {
        self.client
            .get_bucket(&GetBucketRequest {
                bucket: self.bucket.clone(),
                ..Default::default()
            })
            .await
            .with_context(|| {
                format!(
                    "GCS bucket '{}' not accessible — check bucket name and credentials_path in config",
                    self.bucket
                )
            })?;
        Ok(format!("GCS OK: {}", self.bucket))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resumable_threshold_is_5mb() {
        assert_eq!(RESUMABLE_THRESHOLD, 5 * 1024 * 1024);
    }
}
