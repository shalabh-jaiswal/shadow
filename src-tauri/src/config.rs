use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WatchedFolders {
    #[serde(default)]
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub daemon: DaemonConfig,
    #[serde(default)]
    pub nas: NasConfig,
    #[serde(default)]
    pub watched_folders: WatchedFolders,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub debounce_ms: u64,
    pub upload_workers: usize,
    pub log_level: String,
    pub follow_symlinks: bool,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            debounce_ms: 200,
            upload_workers: 4,
            log_level: "info".into(),
            follow_symlinks: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NasConfig {
    pub enabled: bool,
    pub mount_path: String,
}

pub type SharedConfig = Arc<RwLock<AppConfig>>;

pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .expect("cannot find config dir")
        .join("shadow")
        .join("config.toml")
}

pub fn load() -> Result<SharedConfig> {
    let path = config_path();
    let config = if path.exists() {
        let text = std::fs::read_to_string(&path)?;
        let mut cfg: AppConfig = toml::from_str(&text)?;
        // clamp values
        cfg.daemon.debounce_ms = cfg.daemon.debounce_ms.max(50);
        cfg.daemon.upload_workers = cfg.daemon.upload_workers.clamp(1, 16);
        cfg
    } else {
        let cfg = AppConfig::default();
        save(&cfg)?;
        cfg
    };
    Ok(Arc::new(RwLock::new(config)))
}

pub fn save(config: &AppConfig) -> Result<()> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = toml::to_string_pretty(config)?;
    let tmp = path.with_extension("toml.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(tmp, path)?;
    Ok(())
}
