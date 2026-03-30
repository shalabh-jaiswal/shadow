use crate::config::{self, AppConfig, DaemonConfig, GcsConfig, MachineConfig, NasConfig, S3Config};
use crate::daemon::stats::StatsSnapshot;
use crate::daemon::watcher;
use crate::providers::{gcs::GcsProvider, nas::NasProvider, s3::S3Provider, BackupProvider};
use serde::Serialize;
use std::path::Path;
use std::sync::atomic::Ordering;
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
    /// Backup mode for this folder: "full" or "forward_only"
    pub scan_mode: String,
}

#[tauri::command]
pub async fn ping() -> String {
    "pong".into()
}

#[tauri::command]
pub async fn add_folder(
    path: String,
    scan_existing: bool,
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

    // Store folder mode in sled
    let mode_key = format!("folder_mode:{}", path);
    let mode_value = if scan_existing {
        "full"
    } else {
        "forward_only"
    };
    daemon
        .db
        .insert(mode_key.as_bytes(), mode_value.as_bytes())
        .map_err(|e| e.to_string())?;

    // Register with watcher
    if let Some(ref mut w) = daemon.watcher {
        let p = Path::new(&path);
        if p.exists() {
            watcher::watch_path(w, p).map_err(|e| e.to_string())?;
        }
    }

    // Spawn initial scan for new files only if scan_existing is true
    if scan_existing {
        daemon.spawn_scan(Path::new(&path).to_path_buf());
    }

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

    // Remove folder mode from sled
    let mode_key = format!("folder_mode:{}", path);
    let _ = daemon.db.remove(mode_key.as_bytes());

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
                daemon
                    .db
                    .get(key.as_bytes())
                    .ok()
                    .flatten()
                    .and_then(|v| v.as_ref().try_into().ok().map(u64::from_le_bytes))
            };
            let scan_mode = daemon
                .db
                .get(format!("folder_mode:{p}").as_bytes())
                .ok()
                .flatten()
                .and_then(|v| std::str::from_utf8(&v).ok().map(|s| s.to_string()))
                .unwrap_or_else(|| "full".to_string()); // default: full for legacy folders
            FolderStatus {
                path: p.clone(),
                status: "active".into(),
                last_backup,
                scan_mode,
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

/// Set whether backup is paused.
#[tauri::command]
pub async fn set_paused(
    paused: bool,
    state: State<'_, DaemonHandle>,
    app_handle: AppHandle,
) -> Result<(), String> {
    let daemon = state.0.lock().await;
    daemon.paused.store(paused, Ordering::Relaxed);

    // Emit events for frontend and tray menu updates
    if paused {
        let _ = app_handle.emit("daemon_paused", ());
    } else {
        let _ = app_handle.emit("daemon_resumed", ());
    }

    Ok(())
}

/// Get current paused state.
#[tauri::command]
pub async fn get_paused(state: State<'_, DaemonHandle>) -> Result<bool, String> {
    let daemon = state.0.lock().await;
    Ok(daemon.paused.load(Ordering::Relaxed))
}

/// Enable or disable autostart on login.
#[tauri::command]
pub async fn set_autostart(
    enabled: bool,
    state: State<'_, DaemonHandle>,
    app: AppHandle,
) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    if enabled {
        app.autolaunch().enable().map_err(|e| e.to_string())?;
    } else {
        app.autolaunch().disable().map_err(|e| e.to_string())?;
    }

    // Save to config
    let daemon = state.0.lock().await;
    let mut cfg = daemon.config.write().await;
    cfg.daemon.start_on_login = enabled;
    crate::config::save(&cfg).map_err(|e| e.to_string())
}

/// Check for updates. Returns the version string if an update is available, null otherwise.
#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_updater::UpdaterExt;
    let updater = app.updater().map_err(|e| e.to_string())?;
    let update = updater.check().await.map_err(|e| e.to_string())?;
    Ok(update.map(|u| u.version.to_string()))
}
