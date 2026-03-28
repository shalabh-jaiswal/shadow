use crate::providers::BackupProvider;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub struct NasProvider {
    pub mount_path: PathBuf,
}

impl NasProvider {
    pub fn new(mount_path: impl Into<PathBuf>) -> Self {
        Self {
            mount_path: mount_path.into(),
        }
    }
}

#[async_trait::async_trait]
impl BackupProvider for NasProvider {
    fn name(&self) -> &'static str {
        "nas"
    }

    async fn upload(&self, local_path: &Path, remote_key: &str) -> Result<()> {
        let dest = self.mount_path.join(remote_key);
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .context("failed to create destination directories")?;
        }
        let src = local_path.to_path_buf();
        tokio::task::spawn_blocking(move || -> Result<()> {
            use std::io::BufReader;
            let input = std::fs::File::open(&src)
                .with_context(|| format!("failed to open source file: {}", src.display()))?;
            let mut reader = BufReader::with_capacity(1024 * 1024, input);
            let mut writer = std::fs::File::create(&dest)
                .with_context(|| format!("failed to create dest file: {}", dest.display()))?;
            std::io::copy(&mut reader, &mut writer)?;
            Ok(())
        })
        .await??;
        Ok(())
    }

    async fn test_connection(&self) -> Result<String> {
        let meta = tokio::fs::metadata(&self.mount_path)
            .await
            .with_context(|| {
                format!(
                    "NAS mount path not accessible: {}",
                    self.mount_path.display()
                )
            })?;
        if !meta.is_dir() {
            anyhow::bail!(
                "NAS mount path is not a directory: {}",
                self.mount_path.display()
            );
        }
        // write/delete probe file to confirm write access
        let probe = self.mount_path.join(".shadow_probe");
        tokio::fs::write(&probe, b"probe")
            .await
            .context("NAS mount path is not writable")?;
        tokio::fs::remove_file(&probe).await.ok();
        Ok(format!(
            "NAS reachable and writable at {}",
            self.mount_path.display()
        ))
    }
}
