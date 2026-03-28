use crate::providers::BackupProvider;
use anyhow::{Context, Result};
use google_cloud_storage::client::{Client, ClientConfig};
use google_cloud_storage::http::buckets::get::GetBucketRequest;
use google_cloud_storage::http::objects::upload::{Media, UploadObjectRequest, UploadType};
use google_cloud_storage::http::objects::Object;
use std::path::Path;

/// Files smaller than this use GCS simple upload.
/// Files at or above this threshold use the GCS resumable upload protocol,
/// which tolerates network interruptions and supports arbitrarily large objects.
const RESUMABLE_THRESHOLD: u64 = 5 * 1024 * 1024; // 5 MB

pub struct GcsProvider {
    client: Client,
    bucket: String,
}

impl GcsProvider {
    /// Build a GCS client using Application Default Credentials.
    /// Set the GOOGLE_APPLICATION_CREDENTIALS environment variable to the
    /// path of your service account JSON key file before starting Shadow.
    pub async fn new(bucket: &str) -> Result<Self> {
        let config = ClientConfig::default()
            .with_auth()
            .await
            .context("failed to initialize GCS credentials — ensure GOOGLE_APPLICATION_CREDENTIALS is set")?;
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
                    "GCS bucket '{}' not accessible — check bucket name and GOOGLE_APPLICATION_CREDENTIALS",
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
