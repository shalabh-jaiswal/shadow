use crate::config::SharedConfig;
use crate::daemon::scanner;
use anyhow::Result;
use sled::Db;
use std::path::PathBuf;
use std::sync::{atomic::AtomicBool, Arc};
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

#[derive(Clone, serde::Serialize)]
pub struct ReconcileStartedPayload {}

#[derive(Clone, serde::Serialize)]
pub struct ReconcileCompletePayload {
    pub folders_scanned: usize,
}

/// Start the reconciler task that periodically re-scans all watched folders
/// to find files that failed all upload retries (have no sled hash entry).
pub async fn start(
    tx: mpsc::Sender<PathBuf>,
    db: Db,
    config: SharedConfig,
    paused: Arc<AtomicBool>,
    app_handle: AppHandle,
) -> Result<()> {
    loop {
        // Read the current interval from config each iteration
        let interval_mins = {
            let cfg = config.read().await;
            cfg.daemon.reconcile_interval_mins
        };

        // Sleep for the configured interval first (don't run on startup immediately)
        tokio::time::sleep(Duration::from_secs(interval_mins * 60)).await;

        // Check if daemon is paused
        if paused.load(std::sync::atomic::Ordering::Relaxed) {
            continue;
        }

        // Emit reconcile started event
        let _ = app_handle.emit("reconcile_started", ReconcileStartedPayload {});

        // Get watched folders from config
        let watched_folders = {
            let cfg = config.read().await;
            cfg.watched_folders.paths.clone()
        };

        // Scan each watched folder
        let mut folders_scanned = 0;
        for folder in &watched_folders {
            let folder_path = PathBuf::from(folder);
            if folder_path.exists() {
                scanner::spawn_scan(
                    folder_path,
                    config.clone(),
                    db.clone(),
                    tx.clone(),
                    app_handle.clone(),
                );
                folders_scanned += 1;
            }
        }

        // Emit reconcile complete event
        let _ = app_handle.emit(
            "reconcile_complete",
            ReconcileCompletePayload { folders_scanned },
        );
    }
}
