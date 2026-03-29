use crate::config::SharedConfig;
use crate::daemon::hasher::{self, HashCheckResult};
use crate::daemon::stats::DaemonStats;
use crate::ipc::{emit_file_event, FileEvent};
use crate::path_utils::remote_key;
use crate::providers::DynProvider;
use anyhow::Result;
use sled::Db;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, Semaphore};
use tokio_retry::strategy::ExponentialBackoff;
use tokio_retry::Retry;

pub async fn start(
    mut rx: mpsc::Receiver<PathBuf>,
    providers: Vec<DynProvider>,
    db: Db,
    config: SharedConfig,
    app_handle: AppHandle,
    stats: DaemonStats,
) {
    let workers = config.read().await.daemon.upload_workers;
    let semaphore = Arc::new(Semaphore::new(workers));
    let providers = Arc::new(providers);

    // Use the user-configured machine name to avoid leaking the real hostname
    // to cloud storage. Fall back to the OS hostname only when not configured.
    let host = {
        let name = config.read().await.machine.name.clone();
        if name.is_empty() {
            hostname::get()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        } else {
            name
        }
    };

    while let Some(path) = rx.recv().await {
        match hasher::check_and_hash(&db, &path).await {
            Ok(HashCheckResult::Unchanged) => {
                emit_file_event(
                    &app_handle,
                    "file_skipped",
                    FileEvent {
                        path: path.to_string_lossy().to_string(),
                        provider: None,
                        error: None,
                    },
                );
                continue;
            }
            Ok(HashCheckResult::Changed(hash)) => {
                emit_file_event(
                    &app_handle,
                    "file_queued",
                    FileEvent {
                        path: path.to_string_lossy().to_string(),
                        provider: None,
                        error: None,
                    },
                );

                let permit = Arc::clone(&semaphore);
                let providers = Arc::clone(&providers);
                let db = db.clone();
                let app_handle = app_handle.clone();
                let path = path.clone();
                let host = host.clone();
                let stats = stats.clone();
                let config = Arc::clone(&config);

                tokio::spawn(async move {
                    let _permit = permit.acquire().await.unwrap();

                    // Capture file size once for stats — best-effort, 0 on error.
                    let file_bytes = tokio::fs::metadata(&path)
                        .await
                        .map(|m| m.len())
                        .unwrap_or(0);

                    stats.upload_started();

                    let rkey = remote_key(&host, &path);
                    let mut all_ok = true;

                    for provider in providers.iter() {
                        emit_file_event(
                            &app_handle,
                            "file_uploading",
                            FileEvent {
                                path: path.to_string_lossy().to_string(),
                                provider: Some(provider.name().to_string()),
                                error: None,
                            },
                        );

                        match upload_with_retry(provider, &path, &rkey, &app_handle).await {
                            Ok(()) => {
                                emit_file_event(
                                    &app_handle,
                                    "file_uploaded",
                                    FileEvent {
                                        path: path.to_string_lossy().to_string(),
                                        provider: Some(provider.name().to_string()),
                                        error: None,
                                    },
                                );
                                let _ = app_handle.emit(
                                    "provider_status",
                                    serde_json::json!({
                                        "provider": provider.name(),
                                        "status": "ok"
                                    }),
                                );
                            }
                            Err(e) => {
                                all_ok = false;
                                emit_file_event(
                                    &app_handle,
                                    "file_failed",
                                    FileEvent {
                                        path: path.to_string_lossy().to_string(),
                                        provider: Some(provider.name().to_string()),
                                        error: Some(e.to_string()),
                                    },
                                );
                                let _ = app_handle.emit(
                                    "provider_status",
                                    serde_json::json!({
                                        "provider": provider.name(),
                                        "status": "error",
                                        "error": e.to_string()
                                    }),
                                );
                            }
                        }
                    }

                    stats.upload_finished();

                    if all_ok {
                        stats.record_upload(file_bytes);
                        stats.persist(&db);
                        if let Err(e) = hasher::record_hash(&db, &path, hash) {
                            eprintln!("[shadow] failed to record hash for {}: {e}", path.display());
                        }
                        // Record last-backup timestamp for the parent watched folder.
                        // Key format: "last_backup:<folder_path>"
                        let ts = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64;
                        if let Some(folder) = watched_folder_for(&path, &config).await {
                            let key = format!("last_backup:{folder}");
                            let _ = db.insert(key.as_bytes(), &ts.to_le_bytes());
                        }
                    }
                });
            }
            Err(e) => {
                eprintln!("[shadow] hash check error for {}: {e}", path.display());
            }
        }
    }
}

/// Return the watched folder path that is the longest prefix of `path`.
async fn watched_folder_for(path: &Path, config: &SharedConfig) -> Option<String> {
    let cfg = config.read().await;
    cfg.watched_folders
        .paths
        .iter()
        .filter(|f| path.starts_with(f.as_str()))
        .max_by_key(|f| f.len())
        .cloned()
}

async fn upload_with_retry(
    provider: &DynProvider,
    path: &Path,
    remote_key: &str,
    app_handle: &AppHandle,
) -> Result<()> {
    let strategy = ExponentialBackoff::from_millis(1000).factor(4).take(3);
    let path = path.to_path_buf();
    let remote_key = remote_key.to_string();
    let app_handle = app_handle.clone();
    let provider = Arc::clone(provider);

    let mut attempt = 0u32;
    Retry::spawn(strategy, || {
        let provider = Arc::clone(&provider);
        let path = path.clone();
        let remote_key = remote_key.clone();
        let app_handle = app_handle.clone();
        attempt += 1;

        async move {
            match provider.upload(&path, &remote_key).await {
                Ok(()) => Ok(()),
                Err(e) => {
                    if attempt < 3 {
                        emit_file_event(
                            &app_handle,
                            "file_error",
                            FileEvent {
                                path: path.to_string_lossy().to_string(),
                                provider: Some(provider.name().to_string()),
                                error: Some(format!("attempt {attempt}: {e}")),
                            },
                        );
                    }
                    Err(e)
                }
            }
        }
    })
    .await
}
