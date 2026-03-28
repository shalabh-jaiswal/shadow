use crate::config;
use crate::daemon::watcher;
use crate::providers::{nas::NasProvider, BackupProvider};
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
        .map(|p| FolderStatus {
            path: p.clone(),
            status: "active".into(),
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
        "s3" | "gcs" => Ok("not configured".into()),
        _ => Err(format!("unknown provider: {provider_name}")),
    }
}
