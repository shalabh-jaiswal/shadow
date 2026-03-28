use std::path::Path;
use std::sync::Arc;

#[async_trait::async_trait]
pub trait BackupProvider: Send + Sync {
    fn name(&self) -> &'static str;
    async fn upload(&self, local_path: &Path, remote_key: &str) -> anyhow::Result<()>;
    async fn test_connection(&self) -> anyhow::Result<String>;
}

pub type DynProvider = Arc<dyn BackupProvider>;

pub mod gcs;
pub mod nas;
pub mod s3;
