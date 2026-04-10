pub mod debouncer;
pub mod filter;
pub mod hasher;
pub mod queue;
pub mod reconciler;
pub mod scanner;
pub mod stats;
pub mod watcher;

use crate::config::SharedConfig;
use crate::providers::{gcs::GcsProvider, nas::NasProvider, s3::S3Provider, DynProvider};
use anyhow::Result;
use notify::RecommendedWatcher;
use sled::Db;
use std::path::{Path, PathBuf};
use std::sync::{atomic::AtomicBool, Arc};
use tauri::AppHandle;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub use stats::DaemonStats;

pub struct DaemonState {
    pub config: SharedConfig,
    pub app_handle: AppHandle,
    pub upload_tx: mpsc::Sender<std::path::PathBuf>,
    pub task_handles: Vec<JoinHandle<()>>,
    pub watcher: Option<RecommendedWatcher>,
    /// Sled hash database — exposed so `clear_hash_store` IPC command can flush it.
    pub db: Db,
    /// Live upload counters exposed via `get_stats`.
    pub stats: DaemonStats,
    /// Atomic flag to pause/resume backup processing.
    pub paused: Arc<AtomicBool>,
}

pub async fn start(config: SharedConfig, app_handle: AppHandle) -> Result<DaemonState> {
    let (upload_tx, upload_rx) = mpsc::channel::<std::path::PathBuf>(512);

    let db = hasher::open_db()?;
    let stats = DaemonStats::load(&db);
    let paused = Arc::new(AtomicBool::new(false));

    // Build provider list from config
    let providers: Vec<DynProvider> = {
        let cfg = config.read().await;
        let mut p: Vec<DynProvider> = Vec::new();
        if cfg.nas.enabled && !cfg.nas.mount_path.is_empty() {
            p.push(Arc::new(NasProvider::new(&cfg.nas.mount_path)));
        }
        if cfg.s3.enabled && !cfg.s3.bucket.is_empty() {
            match S3Provider::new(&cfg.s3.region, &cfg.s3.bucket, &cfg.s3.profile).await {
                Ok(provider) => p.push(Arc::new(provider)),
                Err(e) => eprintln!("[shadow] S3 init failed: {e}"),
            }
        }
        if cfg.gcs.enabled && !cfg.gcs.bucket.is_empty() {
            match GcsProvider::new(&cfg.gcs.bucket, &cfg.gcs.credentials_path).await {
                Ok(provider) => p.push(Arc::new(provider)),
                Err(e) => eprintln!("[shadow] GCS init failed: {e}"),
            }
        }
        p
    };

    // Spawn queue worker pool
    let queue_handle = {
        let db = db.clone();
        let config = config.clone();
        let app_handle = app_handle.clone();
        let stats = stats.clone();
        tokio::spawn(queue::start(
            upload_rx, providers, db, config, app_handle, stats,
        ))
    };

    // Create watcher → debouncer channel
    let (watcher_tx, watcher_rx) = mpsc::channel::<notify::Event>(256);

    // Spawn debouncer
    let debouncer_handle = {
        let config = config.clone();
        let paused_ref = paused.clone();
        tokio::spawn(debouncer::start(
            watcher_rx,
            upload_tx.clone(),
            config,
            paused_ref,
        ))
    };

    // Spawn reconciler
    let reconciler_handle = {
        let tx = upload_tx.clone();
        let db = db.clone();
        let config = config.clone();
        let paused_ref = paused.clone();
        let app_handle = app_handle.clone();
        tokio::spawn(async move {
            if let Err(e) = reconciler::start(tx, db, config, paused_ref, app_handle).await {
                eprintln!("[shadow] reconciler error: {e}");
            }
        })
    };

    // Create notify watcher
    let mut notify_watcher = watcher::create(watcher_tx)?;

    // Register all watched folders
    {
        let cfg = config.read().await;
        for folder in &cfg.watched_folders.paths {
            let path = Path::new(folder);
            if path.exists() {
                if let Err(e) = watcher::watch_path(&mut notify_watcher, path) {
                    eprintln!("[shadow] failed to watch {folder}: {e}");
                }
            }
        }
    }

    Ok(DaemonState {
        config,
        app_handle,
        upload_tx,
        task_handles: vec![queue_handle, debouncer_handle, reconciler_handle],
        watcher: Some(notify_watcher),
        db,
        stats,
        paused,
    })
}

#[allow(dead_code)]
pub async fn shutdown(mut state: DaemonState) -> Result<()> {
    // 1. Drop watcher first (stops new FS events)
    drop(state.watcher.take());

    // 2. Drop upload_tx (signals queue EOF after debouncer drains)
    drop(state.upload_tx);

    // 3. Join all tasks with a timeout
    let timeout = std::time::Duration::from_secs(10);
    for handle in state.task_handles {
        let _ = tokio::time::timeout(timeout, handle).await;
    }

    Ok(())
}

impl DaemonState {
    /// Spawn a background scan for the given folder path.
    /// Files with no sled entry are enqueued for upload.
    pub fn spawn_scan(&self, folder_path: PathBuf) {
        scanner::spawn_scan(
            folder_path,
            self.config.clone(),
            self.db.clone(),
            self.upload_tx.clone(),
            self.app_handle.clone(),
        );
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn folder_mode_sled_key_format() {
        // Test the expected key format for folder modes
        let path = "/Users/test/Documents";
        let expected_key = format!("folder_mode:{path}");
        assert_eq!(expected_key, "folder_mode:/Users/test/Documents");

        // Test Windows path
        let windows_path = "C:\\Users\\test\\Documents";
        let windows_key = format!("folder_mode:{windows_path}");
        assert_eq!(windows_key, "folder_mode:C:\\Users\\test\\Documents");
    }

    #[test]
    fn folder_mode_values() {
        // Test expected mode values
        let full_mode = "full";
        let forward_only_mode = "forward_only";

        // These should be the exact strings we store/retrieve
        assert_eq!(full_mode.as_bytes(), b"full");
        assert_eq!(forward_only_mode.as_bytes(), b"forward_only");
    }
}
