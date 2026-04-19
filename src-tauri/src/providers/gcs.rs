use crate::providers::BackupProvider;
use anyhow::{Context, Result};
use google_cloud_auth::credentials::CredentialsFile;
use google_cloud_storage::client::{Client, ClientConfig};
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
            .with_context(|| format!("failed to load GCS credentials from '{credentials_path}'"))?;
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

    async fn rename(&self, old_remote_key: &str, new_remote_key: &str) -> Result<()> {
        use google_cloud_storage::http::objects::copy::CopyObjectRequest;
        use google_cloud_storage::http::objects::delete::DeleteObjectRequest;

        let copy_req = CopyObjectRequest {
            source_bucket: self.bucket.clone(),
            source_object: old_remote_key.to_string(),
            destination_bucket: self.bucket.clone(),
            destination_object: new_remote_key.to_string(),
            ..Default::default()
        };

        match self.client.copy_object(&copy_req).await {
            Ok(_) => {
                if let Err(e) = self
                    .client
                    .delete_object(&DeleteObjectRequest {
                        bucket: self.bucket.clone(),
                        object: old_remote_key.to_string(),
                        ..Default::default()
                    })
                    .await
                {
                    eprintln!(
                        "[shadow] GCS rename: copy succeeded but delete of old key '{}' failed: {}. Old key is now an orphan.",
                        old_remote_key,
                        e
                    );
                }
                Ok(())
            }
            Err(e) => Err(anyhow::anyhow!(
                "GCS rename failed during copy, old key preserved: {}",
                e
            )),
        }
    }

    async fn test_connection(&self) -> Result<String> {
        // get_bucket requires storage.buckets.get IAM permission, which many
        // service accounts lack (they may only have storage.objects.create).
        // Instead, verify connectivity by uploading a tiny probe object and
        // then deleting it — this only needs object-level permissions.
        use google_cloud_storage::http::objects::delete::DeleteObjectRequest;

        let probe_key = format!(
            ".shadow-probe-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );

        let req = UploadObjectRequest {
            bucket: self.bucket.clone(),
            ..Default::default()
        };
        self.client
            .upload_object(
                &req,
                b"shadow-probe".to_vec(),
                &UploadType::Simple(Media::new(probe_key.clone())),
            )
            .await
            .with_context(|| format!("GCS bucket '{}' not accessible", self.bucket))?;

        // Best-effort cleanup — ignore errors
        let _ = self
            .client
            .delete_object(&DeleteObjectRequest {
                bucket: self.bucket.clone(),
                object: probe_key,
                ..Default::default()
            })
            .await;

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
