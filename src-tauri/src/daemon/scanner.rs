use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::Result;
use sled::Db;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use walkdir::WalkDir;

use crate::config::SharedConfig;
use crate::daemon::filter;

#[derive(Clone, serde::Serialize)]
pub struct ScanProgressPayload {
    pub folder: String,
    pub scanned: u64,
    pub queued: u64,
    pub total: u64, // 0 means unknown (emit as we go)
}

#[derive(Clone, serde::Serialize)]
pub struct ScanCompletePayload {
    pub folder: String,
    pub total_files: u64,
    pub total_bytes: u64,
}

/// Spawn a background scan of `folder_path`.
/// Files with no sled entry are enqueued. Files with a matching hash are skipped.
/// Emits `scan_progress` events every 100 files or 500ms, and `scan_complete` when done.
pub fn spawn_scan(
    folder_path: PathBuf,
    cfg: SharedConfig,
    db: Db,
    tx: mpsc::Sender<PathBuf>,
    app: AppHandle,
) {
    tokio::spawn(async move {
        if let Err(e) = run_scan(folder_path, cfg, db, tx, app).await {
            tracing::error!(error = %e, "initial folder scan failed");
        }
    });
}

async fn run_scan(
    folder_path: PathBuf,
    cfg: SharedConfig,
    db: Db,
    tx: mpsc::Sender<PathBuf>,
    app: AppHandle,
) -> Result<()> {
    let folder_str = folder_path.to_string_lossy().to_string();
    let follow_symlinks = cfg.read().await.daemon.follow_symlinks;

    let mut scanned: u64 = 0;
    let mut queued: u64 = 0;
    let mut total_bytes: u64 = 0;
    let mut last_emit = Instant::now();

    // Collect files via walkdir (blocking) — run in spawn_blocking
    let folder_clone = folder_path.clone();
    let files: Vec<PathBuf> = tokio::task::spawn_blocking(move || {
        WalkDir::new(&folder_clone)
            .follow_links(follow_symlinks)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter(|e| !filter::should_ignore(e.path()))
            .map(|e| e.into_path())
            .collect()
    })
    .await?;

    let total = files.len() as u64;

    // Emit initial progress
    let _ = app.emit(
        "scan_progress",
        ScanProgressPayload {
            folder: folder_str.clone(),
            scanned: 0,
            queued: 0,
            total,
        },
    );

    for path in &files {
        scanned += 1;

        // Check sled — if entry exists and hash matches, skip (resumable)
        let needs_upload = tokio::task::spawn_blocking({
            let path = path.clone();
            let db = db.clone();
            move || check_needs_upload(&path, &db)
        })
        .await??;

        if needs_upload {
            if let Ok(meta) = tokio::fs::metadata(path).await {
                total_bytes += meta.len();
            }
            // Don't block if queue is full — skip and let live watcher catch it later
            let _ = tx.try_send(path.clone());
            queued += 1;
        }

        // Emit progress every 100 files or 500ms
        if scanned % 100 == 0 || last_emit.elapsed() >= Duration::from_millis(500) {
            let _ = app.emit(
                "scan_progress",
                ScanProgressPayload {
                    folder: folder_str.clone(),
                    scanned,
                    queued,
                    total,
                },
            );
            last_emit = Instant::now();
        }
    }

    // Final progress + complete
    let _ = app.emit(
        "scan_progress",
        ScanProgressPayload {
            folder: folder_str.clone(),
            scanned,
            queued,
            total,
        },
    );
    let _ = app.emit(
        "scan_complete",
        ScanCompletePayload {
            folder: folder_str,
            total_files: scanned,
            total_bytes,
        },
    );

    Ok(())
}

/// Returns true if the file needs to be uploaded (no stored hash entry).
/// This is a synchronous function meant to run in spawn_blocking.
/// The live watcher + hasher handles the "changed since last backup" case.
/// The scanner's job is just to find files that have NEVER been uploaded (no sled entry).
fn check_needs_upload(path: &Path, db: &Db) -> Result<bool> {
    let key = path.to_string_lossy();
    Ok(db.get(key.as_bytes())?.is_none())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn open_temp_db() -> sled::Db {
        sled::Config::new().temporary(true).open().unwrap()
    }

    #[test]
    fn new_file_needs_upload() {
        let db = open_temp_db();
        let f = NamedTempFile::new().unwrap();
        std::fs::write(f.path(), b"hello").unwrap();
        assert!(check_needs_upload(f.path(), &db).unwrap());
    }

    #[test]
    fn file_with_stored_hash_skipped() {
        let db = open_temp_db();
        let f = NamedTempFile::new().unwrap();
        std::fs::write(f.path(), b"hello").unwrap();
        // Store a fake hash
        let key = f.path().to_string_lossy();
        db.insert(key.as_bytes(), b"fakehash").unwrap();
        assert!(!check_needs_upload(f.path(), &db).unwrap());
    }
}
