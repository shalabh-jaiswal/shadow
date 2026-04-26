use crate::config::SharedConfig;
use crate::daemon::hasher;
use crate::daemon::stats::DaemonStats;
use crate::ipc::{emit_file_event, FileEvent};
use crate::path_utils::remote_key;
use crate::providers::DynProvider;
use anyhow::Result;
use sled::Db;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, watch, Semaphore};
use tokio_retry::strategy::ExponentialBackoff;
use tokio_retry::Retry;

pub async fn start(
    mut rx: mpsc::Receiver<PathBuf>,
    provider_rx: watch::Receiver<Vec<DynProvider>>,
    db: Db,
    config: SharedConfig,
    app_handle: AppHandle,
    stats: DaemonStats,
) {
    let workers = config.read().await.daemon.upload_workers;
    let semaphore = Arc::new(Semaphore::new(workers));

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
        // Skip 0-byte files — these are transient placeholders (e.g. the Windows
        // "New Text Document.txt" created before the user renames/writes to it).
        // A Modify event will fire once the user actually writes content.
        match tokio::fs::metadata(&path).await {
            Ok(meta) if meta.is_dir() => {
                // Silently skip directories — we only upload files.
                // This prevents hash checks from failing on folders.
                continue;
            }
            Ok(meta) if meta.len() == 0 => {
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
            _ => {}
        }

        let provider_names: Vec<String> = provider_rx
            .borrow()
            .iter()
            .map(|p| p.name().to_string())
            .collect();

        match hasher::check_and_hash(&db, &path, &provider_names).await {
            Ok((hash, missing_providers)) => {
                if missing_providers.is_empty() {
                    emit_file_event(
                        &app_handle,
                        "file_skipped",
                        FileEvent {
                            path: path.to_string_lossy().to_string(),
                            provider: None,
                            error: None,
                        },
                    );

                    // Even if unchanged, we should update the mtime for all providers
                    // so the scanner's fast-path works correctly if only mtime was touched.
                    let mtime_millis = tokio::fs::metadata(&path)
                        .await
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0);

                    for provider in &provider_names {
                        let _ = hasher::record_hash(&db, &path, provider, hash, mtime_millis);
                    }
                    continue;
                }

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
                let db = db.clone();
                let app_handle = app_handle.clone();
                let path = path.clone();
                let host = host.clone();
                let stats = stats.clone();
                let config = Arc::clone(&config);

                // Snapshot the current providers, filtered by what actually needs uploading
                let providers: Vec<DynProvider> = provider_rx
                    .borrow()
                    .iter()
                    .filter(|p| missing_providers.contains(&p.name().to_string()))
                    .cloned()
                    .collect();

                tokio::spawn(async move {
                    let _permit = permit.acquire().await.unwrap();

                    // Capture file size once for stats — best-effort, 0 on error.
                    let file_bytes = tokio::fs::metadata(&path)
                        .await
                        .map(|m| m.len())
                        .unwrap_or(0);

                    stats.upload_started();

                    let rkey = remote_key(&host, &path);

                    // Upload to all providers in parallel. Each future emits its
                    // own final status immediately on completion so the UI updates
                    // per-provider without waiting for slower providers to finish.
                    let upload_futures = providers.iter().map(|provider| {
                        let provider = Arc::clone(provider);
                        let path = path.clone();
                        let rkey = rkey.clone();
                        let app_handle = app_handle.clone();
                        async move {
                            let name = provider.name().to_string();
                            emit_file_event(
                                &app_handle,
                                "file_uploading",
                                FileEvent {
                                    path: path.to_string_lossy().to_string(),
                                    provider: Some(name.clone()),
                                    error: None,
                                },
                            );
                            match upload_with_retry(&provider, &path, &rkey, &app_handle).await {
                                Ok(()) => {
                                    emit_file_event(
                                        &app_handle,
                                        "file_uploaded",
                                        FileEvent {
                                            path: path.to_string_lossy().to_string(),
                                            provider: Some(name.clone()),
                                            error: None,
                                        },
                                    );
                                    let _ = app_handle.emit(
                                        "provider_status",
                                        serde_json::json!({ "provider": name, "status": "ok" }),
                                    );
                                    (name, true)
                                }
                                Err(e) => {
                                    emit_file_event(
                                        &app_handle,
                                        "file_failed",
                                        FileEvent {
                                            path: path.to_string_lossy().to_string(),
                                            provider: Some(name.clone()),
                                            error: Some(e.to_string()),
                                        },
                                    );
                                    let _ = app_handle.emit(
                                        "provider_status",
                                        serde_json::json!({
                                            "provider": name,
                                            "status": "error",
                                            "error": e.to_string()
                                        }),
                                    );
                                    (name, false)
                                }
                            }
                        }
                    });
                    let results = futures::future::join_all(upload_futures).await;
                    let any_ok = results.iter().any(|(_, ok)| *ok);

                    stats.upload_finished();

                    // Record hash as long as at least one provider succeeded.
                    // This prevents infinite re-upload loops when one provider is
                    // permanently broken but others are healthy.
                    if any_ok {
                        stats.record_upload(file_bytes);
                        stats.persist(&db);

                        let mtime_millis = tokio::fs::metadata(&path)
                            .await
                            .ok()
                            .and_then(|m| m.modified().ok())
                            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                            .map(|d| d.as_millis() as u64)
                            .unwrap_or(0);

                        for (provider_name, succeeded) in &results {
                            if *succeeded {
                                if let Err(e) = hasher::record_hash(
                                    &db,
                                    &path,
                                    provider_name,
                                    hash,
                                    mtime_millis,
                                ) {
                                    tracing::error!(
                                        path = %path.display(),
                                        provider = %provider_name,
                                        error = %e,
                                        "failed to record hash after upload"
                                    );
                                }
                            }
                        }

                        // Record last-backup timestamp for the parent watched folder,
                        // then emit folder_updated so the frontend re-fetches AFTER
                        // the sled write is complete (avoids the file_uploaded race).
                        let ts = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64;
                        if let Some(folder) = watched_folder_for(&path, &config).await {
                            let key = format!("last_backup:{folder}");
                            let _ = db.insert(key.as_bytes(), &ts.to_le_bytes());
                            let _ = app_handle
                                .emit("folder_updated", serde_json::json!({ "folder": folder }));
                        }
                    }
                });
            }
            Err(e) => {
                tracing::error!(path = %path.display(), error = %e, "hash check failed");
            }
        }
    }
}

/// Return the watched folder path that is the longest prefix of `path`.
async fn watched_folder_for(path: &Path, config: &SharedConfig) -> Option<String> {
    let cfg = config.read().await;
    let result = cfg
        .watched_folders
        .paths
        .iter()
        .filter(|f| path.starts_with(f.as_str()))
        .max_by_key(|f| f.len())
        .cloned();
    result
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
