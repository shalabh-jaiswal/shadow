use crate::config::SharedConfig;
use crate::daemon::hasher;
use crate::ipc::{FileRenameErrorEvent, FileRenamedEvent};
use crate::path_utils::remote_key;
use crate::providers::DynProvider;
use sled::Db;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, watch};

pub async fn start(
    mut rx: mpsc::Receiver<(PathBuf, PathBuf)>,
    upload_tx: mpsc::Sender<PathBuf>,
    provider_rx: watch::Receiver<Vec<DynProvider>>,
    db: Db,
    config: SharedConfig,
    app_handle: AppHandle,
) {
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

    while let Some((old_path, new_path)) = rx.recv().await {
        let old_in_sled = hasher::has_entry(&db, &old_path).unwrap_or(false);

        if !old_in_sled {
            // Never backed up — treat as a new file upload
            let _ = upload_tx.send(new_path).await;
            continue;
        }

        let old_rkey = remote_key(&host, &old_path);
        let new_rkey = remote_key(&host, &new_path);
        let providers: Vec<DynProvider> = provider_rx.borrow().clone();
        let db = db.clone();
        let app_handle = app_handle.clone();
        let old_path_str = old_path.to_string_lossy().to_string();
        let new_path_str = new_path.to_string_lossy().to_string();

        tokio::spawn(async move {
            let rename_futures = providers.iter().map(|provider| {
                let provider = Arc::clone(provider);
                let old_rkey = old_rkey.clone();
                let new_rkey = new_rkey.clone();
                let old_path_str = old_path_str.clone();
                let new_path_str = new_path_str.clone();
                let app_handle = app_handle.clone();
                async move {
                    let name = provider.name().to_string();
                    match provider.rename(&old_rkey, &new_rkey).await {
                        Ok(()) => {
                            let _ = app_handle.emit(
                                "file_renamed",
                                FileRenamedEvent {
                                    old_path: old_path_str,
                                    new_path: new_path_str,
                                    provider: name,
                                    old_remote_key: old_rkey,
                                    new_remote_key: new_rkey,
                                },
                            );
                            true
                        }
                        Err(e) => {
                            let _ = app_handle.emit(
                                "file_rename_error",
                                FileRenameErrorEvent {
                                    old_path: old_path_str,
                                    new_path: new_path_str,
                                    provider: name,
                                    error: e.to_string(),
                                },
                            );
                            false
                        }
                    }
                }
            });
            let results = futures::future::join_all(rename_futures).await;
            let all_ok = results.iter().all(|&ok| ok);

            if all_ok {
                if let Err(e) = hasher::rename_hash_entry(&db, &old_path, &new_path) {
                    tracing::error!(
                        old_path = %old_path.display(),
                        new_path = %new_path.display(),
                        error = %e,
                        "failed to update hash store after rename"
                    );
                }
            }
        });
    }
}
