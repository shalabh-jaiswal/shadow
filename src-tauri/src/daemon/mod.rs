pub mod debouncer;
pub mod hasher;
pub mod queue;
pub mod watcher;

use crate::config::SharedConfig;
use crate::providers::{nas::NasProvider, DynProvider};
use anyhow::Result;
use notify::RecommendedWatcher;
use std::path::Path;
use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub struct DaemonState {
    pub config: SharedConfig,
    pub app_handle: AppHandle,
    pub upload_tx: mpsc::Sender<std::path::PathBuf>,
    pub task_handles: Vec<JoinHandle<()>>,
    pub watcher: Option<RecommendedWatcher>,
}

pub async fn start(config: SharedConfig, app_handle: AppHandle) -> Result<DaemonState> {
    let (upload_tx, upload_rx) = mpsc::channel::<std::path::PathBuf>(512);

    let db = hasher::open_db()?;

    // Build provider list from config
    let providers: Vec<DynProvider> = {
        let cfg = config.read().await;
        let mut p: Vec<DynProvider> = Vec::new();
        if cfg.nas.enabled && !cfg.nas.mount_path.is_empty() {
            p.push(Arc::new(NasProvider::new(&cfg.nas.mount_path)));
        }
        p
    };

    // Spawn queue worker pool
    let queue_handle = {
        let db = db.clone();
        let config = config.clone();
        let app_handle = app_handle.clone();
        tokio::spawn(queue::start(upload_rx, providers, db, config, app_handle))
    };

    // Create watcher → debouncer channel
    let (watcher_tx, watcher_rx) = mpsc::channel::<notify::Event>(256);

    // Spawn debouncer
    let debouncer_handle = {
        let config = config.clone();
        tokio::spawn(debouncer::start(watcher_rx, upload_tx.clone(), config))
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
        task_handles: vec![queue_handle, debouncer_handle],
        watcher: Some(notify_watcher),
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
