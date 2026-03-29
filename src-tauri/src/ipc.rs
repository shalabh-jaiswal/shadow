use crate::config::{self, AppConfig, DaemonConfig, GcsConfig, MachineConfig, NasConfig, S3Config};
use crate::daemon::stats::StatsSnapshot;
use crate::daemon::watcher;
use crate::providers::{gcs::GcsProvider, nas::NasProvider, s3::S3Provider, BackupProvider};
use serde::Serialize;
use std::path::Path;
use tauri::{AppHandle, Emitter, State};

use crate::DaemonHandle;

#[derive(Debug, Serialize, Clone)]
pub struct FileEvent {
    pub path: String,
    pub provider: Option<String>,
    pub error: Option<String>,
}

pub fn emit_file_event(app_handle: &AppHandle, event: &str, payload: FileEvent) {
    let _ = app_handle.emit(event, payload);
}

#[derive(Debug, Serialize)]
pub struct FolderStatus {
    pub path: String,
    pub status: String,
    /// Unix timestamp in milliseconds of the last successful upload from this folder.
    /// `None` means no file has been backed up from this folder yet.
    pub last_backup: Option<u64>,
}

#[tauri::command]
pub async fn ping() -> String {
    "pong".into()
}

#[tauri::command]
pub async fn add_folder(
    path: String,
    state: State<'_, DaemonHandle>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let mut daemon = state.0.lock().await;

    // Add to config and save
    {
        let mut cfg = daemon.config.write().await;
        if !cfg.watched_folders.paths.contains(&path) {
            cfg.watched_folders.paths.push(path.clone());
            config::save(&cfg).map_err(|e| e.to_string())?;
        }
    }

    // Register with watcher
    if let Some(ref mut w) = daemon.watcher {
        let p = Path::new(&path);
        if p.exists() {
            watcher::watch_path(w, p).map_err(|e| e.to_string())?;
        }
    }

    // Spawn initial scan for new files
    daemon.spawn_scan(Path::new(&path).to_path_buf());

    let _ = app_handle.emit("folder_added", serde_json::json!({ "path": path }));

    Ok(())
}

#[tauri::command]
pub async fn remove_folder(
    path: String,
    state: State<'_, DaemonHandle>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let mut daemon = state.0.lock().await;

    // Remove from config and save
    {
        let mut cfg = daemon.config.write().await;
        cfg.watched_folders.paths.retain(|p| p != &path);
        config::save(&cfg).map_err(|e| e.to_string())?;
    }

    // Unregister from watcher
    if let Some(ref mut w) = daemon.watcher {
        let p = Path::new(&path);
        let _ = watcher::unwatch_path(w, p);
    }

    let _ = app_handle.emit("folder_removed", serde_json::json!({ "path": path }));

    Ok(())
}

#[tauri::command]
pub async fn get_watched_folders(
    state: State<'_, DaemonHandle>,
) -> Result<Vec<FolderStatus>, String> {
    let daemon = state.0.lock().await;
    let cfg = daemon.config.read().await;
    let folders = cfg
        .watched_folders
        .paths
        .iter()
        .map(|p| {
            let last_backup = {
                let key = format!("last_backup:{p}");
                daemon.db.get(key.as_bytes())
                    .ok()
                    .flatten()
                    .and_then(|v| v.as_ref().try_into().ok().map(u64::from_le_bytes))
            };
            FolderStatus {
                path: p.clone(),
                status: "active".into(),
                last_backup,
            }
        })
        .collect();
    Ok(folders)
}

#[tauri::command]
pub async fn test_provider(
    provider_name: String,
    state: State<'_, DaemonHandle>,
) -> Result<String, String> {
    match provider_name.to_lowercase().as_str() {
        "nas" => {
            let daemon = state.0.lock().await;
            let cfg = daemon.config.read().await;
            if !cfg.nas.enabled || cfg.nas.mount_path.is_empty() {
                return Err("NAS is not configured".into());
            }
            let provider = NasProvider::new(&cfg.nas.mount_path);
            provider.test_connection().await.map_err(|e| e.to_string())
        }
        "s3" => {
            let (bucket, region, profile) = {
                let daemon = state.0.lock().await;
                let cfg = daemon.config.read().await;
                if !cfg.s3.enabled || cfg.s3.bucket.is_empty() {
                    return Err("S3 is not configured".into());
                }
                (
                    cfg.s3.bucket.clone(),
                    cfg.s3.region.clone(),
                    cfg.s3.profile.clone(),
                )
            };
            let provider = S3Provider::new(&region, &bucket, &profile)
                .await
                .map_err(|e| e.to_string())?;
            provider.test_connection().await.map_err(|e| e.to_string())
        }
        "gcs" => {
            let (bucket, credentials_path) = {
                let daemon = state.0.lock().await;
                let cfg = daemon.config.read().await;
                if !cfg.gcs.enabled || cfg.gcs.bucket.is_empty() {
                    return Err("GCS is not configured".into());
                }
                (cfg.gcs.bucket.clone(), cfg.gcs.credentials_path.clone())
            };
            let provider = GcsProvider::new(&bucket, &credentials_path)
                .await
                .map_err(|e| e.to_string())?;
            provider.test_connection().await.map_err(|e| e.to_string())
        }
        _ => Err(format!("unknown provider: {provider_name}")),
    }
}

/// Update and persist a provider's configuration block.
/// The running daemon continues using its current providers until the app restarts.
#[tauri::command]
pub async fn set_provider_config(
    provider: String,
    config_json: String,
    state: State<'_, DaemonHandle>,
) -> Result<(), String> {
    let daemon = state.0.lock().await;
    let mut cfg = daemon.config.write().await;

    match provider.to_lowercase().as_str() {
        "s3" => {
            let s3: S3Config = serde_json::from_str(&config_json).map_err(|e| e.to_string())?;
            cfg.s3 = s3;
        }
        "gcs" => {
            let gcs: GcsConfig = serde_json::from_str(&config_json).map_err(|e| e.to_string())?;
            cfg.gcs = gcs;
        }
        "nas" => {
            let nas: NasConfig = serde_json::from_str(&config_json).map_err(|e| e.to_string())?;
            cfg.nas = nas;
        }
        _ => return Err(format!("unknown provider: {provider}")),
    }

    config::save(&cfg).map_err(|e| e.to_string())
}

/// Return the full app configuration (no secrets — credentials are file paths only).
#[tauri::command]
pub async fn get_config(state: State<'_, DaemonHandle>) -> Result<AppConfig, String> {
    let daemon = state.0.lock().await;
    let cfg = daemon.config.read().await;
    Ok(cfg.clone())
}

/// Persist updated daemon and machine settings. Provider configs are unchanged.
/// Note: upload_workers change takes effect on next app restart.
#[tauri::command]
pub async fn set_daemon_config(
    daemon: DaemonConfig,
    machine: MachineConfig,
    state: State<'_, DaemonHandle>,
) -> Result<(), String> {
    let handle = state.0.lock().await;
    let mut cfg = handle.config.write().await;
    cfg.daemon = daemon;
    cfg.machine = machine;
    config::save(&cfg).map_err(|e| e.to_string())
}

/// Return a live snapshot of upload counters.
#[tauri::command]
pub async fn get_stats(state: State<'_, DaemonHandle>) -> Result<StatsSnapshot, String> {
    let daemon = state.0.lock().await;
    Ok(daemon.stats.snapshot())
}

/// Clear the blake3 hash store, forcing a full re-upload on next scan.
#[tauri::command]
pub async fn clear_hash_store(state: State<'_, DaemonHandle>) -> Result<(), String> {
    let daemon = state.0.lock().await;
    daemon.db.clear().map_err(|e| e.to_string())?;
    Ok(())
}
