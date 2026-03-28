use crate::config::SharedConfig;
use crate::daemon::hasher::{self, HashCheckResult};
use crate::ipc::{emit_file_event, FileEvent};
use crate::path_utils::remote_key;
use crate::providers::DynProvider;
use anyhow::Result;
use sled::Db;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::{mpsc, Semaphore};
use tokio_retry::strategy::ExponentialBackoff;
use tokio_retry::Retry;

pub async fn start(
    mut rx: mpsc::Receiver<PathBuf>,
    providers: Vec<DynProvider>,
    db: Db,
    config: SharedConfig,
    app_handle: AppHandle,
) {
    let workers = config.read().await.daemon.upload_workers;
    let semaphore = Arc::new(Semaphore::new(workers));
    let providers = Arc::new(providers);

    let host = hostname::get()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

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

                tokio::spawn(async move {
                    let _permit = permit.acquire().await.unwrap();
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
                            }
                        }
                    }

                    if all_ok {
                        if let Err(e) = hasher::record_hash(&db, &path, hash) {
                            eprintln!("[shadow] failed to record hash for {}: {e}", path.display());
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
