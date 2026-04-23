use std::path::PathBuf;
use std::time::{Duration, Instant};

use anyhow::Result;
use sled::Db;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use walkdir::WalkDir;

use crate::config::SharedConfig;
use crate::daemon::filter;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScanTrigger {
    Initial,
    Manual,
    Scheduled,
}

impl ScanTrigger {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScanTrigger::Initial => "initial",
            ScanTrigger::Manual => "manual",
            ScanTrigger::Scheduled => "scheduled",
        }
    }
}

#[derive(Clone, serde::Serialize)]
pub struct ScanProgressPayload {
    pub folder: String,
    pub scanned: u64,
    pub queued: u64,
    pub total: u64, // 0 means unknown (emit as we go)
    pub trigger: String,
}

#[derive(Clone, serde::Serialize)]
pub struct ScanCompletePayload {
    pub folder: String,
    pub total_files: u64,
    pub total_bytes: u64,
    pub files_uploaded: u64, // We will use queued as a proxy for uploaded for now to match UI expectations
    pub files_skipped: u64,
    pub trigger: String,
}

#[derive(Default)]
struct ScanStats {
    scanned: u64,
    queued: u64,
    total_bytes: u64,
}

/// Start a full scan across all watched folders
pub async fn scan_all_folders(
    config: &SharedConfig,
    db: &Db,
    tx: &mpsc::Sender<PathBuf>,
    app_handle: &AppHandle,
    trigger: ScanTrigger,
    provider_names: Vec<String>,
) {
    let watched_folders = {
        let cfg = config.read().await;
        cfg.watched_folders.paths.clone()
    };

    let mut aggregate_stats = ScanStats::default();
    let trigger_str = trigger.as_str().to_string();
    let start_time = Instant::now();

    for folder in &watched_folders {
        let folder_path = PathBuf::from(folder);
        if folder_path.exists() {
            let mode_key = format!("folder_mode:{folder}");
            let mode = db
                .get(mode_key.as_bytes())
                .ok()
                .flatten()
                .and_then(|v| std::str::from_utf8(&v).ok().map(|s| s.to_string()))
                .unwrap_or_else(|| "full".to_string());

            let added_at = {
                let key = format!("folder_added_at:{folder}");
                db.get(key.as_bytes())
                    .ok()
                    .flatten()
                    .and_then(|v| v.as_ref().try_into().ok().map(u64::from_le_bytes))
            };

            // In forward_only mode, we only scan if we have an added_at timestamp
            // to compare against. If no timestamp exists (legacy folder), we skip it.
            if mode == "forward_only" && added_at.is_none() {
                continue;
            }

            match run_scan(
                folder_path.clone(),
                config.clone(),
                db.clone(),
                tx.clone(),
                app_handle.clone(),
                trigger,
                added_at,
                provider_names.clone(),
            )
            .await
            {
                Ok(stats) => {
                    aggregate_stats.scanned += stats.scanned;
                    aggregate_stats.queued += stats.queued;
                    aggregate_stats.total_bytes += stats.total_bytes;
                }
                Err(e) => {
                    tracing::error!(folder = %folder, error = %e, "folder scan failed");
                }
            }
        }
    }

    let files_skipped = aggregate_stats
        .scanned
        .saturating_sub(aggregate_stats.queued);

    let duration = start_time.elapsed();
    tracing::info!(
        trigger = %trigger_str,
        scanned = aggregate_stats.scanned,
        queued = aggregate_stats.queued,
        skipped = files_skipped,
        duration_ms = duration.as_millis(),
        "Recovery scan complete"
    );

    let _ = app_handle.emit(
        "scan_complete",
        ScanCompletePayload {
            folder: "All Folders".to_string(),
            total_files: aggregate_stats.scanned,
            total_bytes: aggregate_stats.total_bytes,
            files_uploaded: aggregate_stats.queued,
            files_skipped,
            trigger: trigger_str,
        },
    );
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
    trigger: ScanTrigger,
    provider_names: Vec<String>,
) {
    tokio::spawn(async move {
        let folder_str = folder_path.to_string_lossy().to_string();
        let trigger_str = trigger.as_str().to_string();
        let start_time = Instant::now();

        let added_at = {
            let key = format!("folder_added_at:{}", folder_str);
            db.get(key.as_bytes())
                .ok()
                .flatten()
                .and_then(|v| v.as_ref().try_into().ok().map(u64::from_le_bytes))
        };

        match run_scan(
            folder_path,
            cfg,
            db,
            tx,
            app.clone(),
            trigger,
            added_at,
            provider_names,
        )
        .await
        {
            Ok(stats) => {
                let files_skipped = stats.scanned.saturating_sub(stats.queued);
                let duration = start_time.elapsed();

                tracing::info!(
                    folder = %folder_str,
                    trigger = %trigger_str,
                    scanned = stats.scanned,
                    queued = stats.queued,
                    skipped = files_skipped,
                    duration_ms = duration.as_millis(),
                    "Folder scan complete"
                );

                let _ = app.emit(
                    "scan_complete",
                    ScanCompletePayload {
                        folder: folder_str,
                        total_files: stats.scanned,
                        total_bytes: stats.total_bytes,
                        files_uploaded: stats.queued,
                        files_skipped,
                        trigger: trigger_str,
                    },
                );
            }
            Err(e) => {
                tracing::error!(error = %e, "folder scan failed");
            }
        }
    });
}

#[allow(clippy::too_many_arguments)]
async fn run_scan(
    folder_path: PathBuf,
    cfg: SharedConfig,
    db: Db,
    tx: mpsc::Sender<PathBuf>,
    app: AppHandle,
    trigger: ScanTrigger,
    added_at: Option<u64>,
    provider_names: Vec<String>,
) -> Result<ScanStats> {
    let folder_str = folder_path.to_string_lossy().to_string();
    let follow_symlinks = cfg.read().await.daemon.follow_symlinks;
    let trigger_str = trigger.as_str().to_string();

    let mut scanned: u64 = 0;
    let mut queued: u64 = 0;
    let mut total_bytes: u64 = 0;
    let mut last_emit = Instant::now();

    // In forward_only mode, we can optimize by skipping files that haven't been modified
    // since the folder was added.
    let folder_mode = {
        let key = format!("folder_mode:{}", folder_str);
        db.get(key.as_bytes())
            .ok()
            .flatten()
            .and_then(|v| std::str::from_utf8(&v).ok().map(|s| s.to_string()))
            .unwrap_or_else(|| "full".to_string())
    };

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
            trigger: trigger_str.clone(),
        },
    );

    for path in &files {
        scanned += 1;

        let mut needs_upload = true;
        let current_mtime;

        if let Ok(meta) = tokio::fs::metadata(path).await {
            // Skip 0-byte files to match live watcher/queue behavior
            if meta.len() == 0 {
                continue;
            }

            current_mtime = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            // In forward_only mode, skip files modified before the folder was added
            if folder_mode == "forward_only" {
                if let Some(added_ts) = added_at {
                    if current_mtime > 0 && current_mtime < added_ts {
                        continue;
                    }
                }
            }

            total_bytes += meta.len();
        } else {
            continue;
        }

        // Fast-path mtime check across all providers
        let mtime_check_result = tokio::task::spawn_blocking({
            let path = path.clone();
            let db = db.clone();
            let provider_names = provider_names.clone();
            move || {
                let refs: Vec<&str> = provider_names.iter().map(|s| s.as_str()).collect();
                crate::daemon::hasher::needs_upload_for_providers(&db, &path, &refs, current_mtime)
            }
        })
        .await??;

        if !mtime_check_result {
            // All providers have an entry and mtimes match perfectly
            needs_upload = false;
        }

        if needs_upload {
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
                    trigger: trigger_str.clone(),
                },
            );
            last_emit = Instant::now();
        }
    }

    // Final progress
    let _ = app.emit(
        "scan_progress",
        ScanProgressPayload {
            folder: folder_str.clone(),
            scanned,
            queued,
            total,
            trigger: trigger_str.clone(),
        },
    );

    Ok(ScanStats {
        scanned,
        queued,
        total_bytes,
    })
}
