use crate::config::SharedConfig;
use crate::daemon::filter;
use notify::event::{ModifyKind, RenameMode};
use notify::{Event, EventKind};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc};
use std::time::{Duration, Instant};
use tauri_plugin_notification::NotificationExt;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use walkdir::WalkDir;

pub async fn start<R: tauri::Runtime>(
    mut rx: mpsc::Receiver<Event>,
    upload_tx: mpsc::Sender<PathBuf>,
    rename_tx: mpsc::Sender<(PathBuf, PathBuf)>,
    config: SharedConfig,
    paused: Arc<AtomicBool>,
    app_handle: tauri::AppHandle<R>,
) {
    let mut timers: HashMap<PathBuf, JoinHandle<()>> = HashMap::new();

    // Windows/Linux tracked renames
    let mut tracked_renames: HashMap<usize, (PathBuf, Instant)> = HashMap::new();
    // Fallback for single untracked renames
    let mut pending_rename_from: Option<(PathBuf, Instant)> = None;
    // macOS FSEvents (RenameMode::Any) FIFO queue
    let mut macos_renames: VecDeque<(PathBuf, Instant)> = VecDeque::new();

    while let Some(event) = rx.recv().await {
        let now = Instant::now();
        let timeout = Duration::from_secs(5);

        // Cleanup stale entries
        tracked_renames.retain(|_, (_, ts)| now.duration_since(*ts) < timeout);
        if let Some((_, ts)) = &pending_rename_from {
            if now.duration_since(*ts) >= timeout {
                pending_rename_from = None;
            }
        }
        macos_renames.retain(|(_, ts)| now.duration_since(*ts) < timeout);

        let tracker = event.tracker();

        match event.kind {
            EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
                if event.paths.len() >= 2 {
                    let old = event.paths[0].clone();
                    let new = event.paths[1].clone();
                    if !paused.load(Ordering::Relaxed) {
                        let _ = rename_tx.send((old, new)).await;
                    }
                }
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                if let Some(old) = event.paths.into_iter().next() {
                    if paused.load(Ordering::Relaxed) {
                        continue;
                    }
                    if let Some(id) = tracker {
                        tracked_renames.insert(id, (old, now));
                    } else {
                        pending_rename_from = Some((old, now));
                    }
                }
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
                if let Some(new) = event.paths.into_iter().next() {
                    if !paused.load(Ordering::Relaxed) {
                        let mut paired_old = None;

                        if let Some(id) = tracker {
                            if let Some((old, _)) = tracked_renames.remove(&id) {
                                paired_old = Some(old);
                            }
                        }

                        if paired_old.is_none() {
                            if let Some((old, _)) = pending_rename_from.take() {
                                paired_old = Some(old);
                            }
                        }

                        if let Some(old) = paired_old {
                            let _ = rename_tx.send((old, new)).await;
                        } else {
                            // No matching From — treat as a new file
                            let _ = upload_tx.send(new).await;
                        }
                    } else {
                        if let Some(id) = tracker {
                            tracked_renames.remove(&id);
                        }
                        pending_rename_from = None;
                    }
                }
            }
            // macOS FSEvents emits RenameMode::Any (one path per event, two events per rename).
            // Determine direction by checking whether the path still exists on disk:
            //   - missing = source (old path), store as pending
            //   - present = destination (new path), pair with pending
            EventKind::Modify(ModifyKind::Name(RenameMode::Any)) => {
                if let Some(path) = event.paths.into_iter().next() {
                    if paused.load(Ordering::Relaxed) {
                        macos_renames.clear();
                        continue;
                    }
                    if path.exists() {
                        // New path — pair with pending source if available
                        if let Some((old, _)) = macos_renames.pop_front() {
                            let _ = rename_tx.send((old, path)).await;
                        } else {
                            // No known source; treat as new file
                            let _ = upload_tx.send(path).await;
                        }
                    } else {
                        // Old path — store and wait for destination event
                        macos_renames.push_back((path, now));
                    }
                }
            }
            _ => {
                let debounce_ms = config.read().await.daemon.debounce_ms;
                let follow_symlinks = config.read().await.daemon.follow_symlinks;

                for path in event.paths {
                    if path.extension().and_then(|s| s.to_str()) == Some("shadow_job") {
                        let jobs_dir = crate::path_utils::get_jobs_dir();
                        if path.starts_with(&jobs_dir) && !paused.load(Ordering::Relaxed) {
                            if let Ok(contents) = std::fs::read_to_string(&path) {
                                let target_path = std::path::PathBuf::from(contents.trim());
                                if target_path.exists() {
                                    tracing::debug!(
                                        "Spool intercepted job: {:?} -> {:?}",
                                        path,
                                        target_path
                                    );

                                    let display_name = target_path
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("Unknown")
                                        .to_string();

                                    let success = if target_path.is_dir() {
                                        // Walk directory and queue all files
                                        let upload_tx_clone = upload_tx.clone();
                                        let target_path_clone = target_path.clone();
                                        tokio::task::spawn_blocking(move || {
                                            for entry in WalkDir::new(&target_path_clone)
                                                .follow_links(follow_symlinks)
                                                .into_iter()
                                                .filter_map(|e| e.ok())
                                                .filter(|e| e.file_type().is_file())
                                                .filter(|e| !filter::should_ignore(e.path()))
                                            {
                                                let _ = upload_tx_clone.try_send(entry.path().to_path_buf());
                                            }
                                        });
                                        let _ = app_handle.notification().builder()
                                            .title("Shadow Backup")
                                            .body(format!("Backing up folder: {}", display_name))
                                            .show();
                                        true
                                    } else {
                                        if upload_tx.try_send(target_path.clone()).is_ok() {
                                            let _ = app_handle.notification().builder()
                                                .title("Shadow Backup")
                                                .body(format!("Backing up file: {}", display_name))
                                                .show();
                                            true
                                        } else {
                                            false
                                        }
                                    };

                                    if success {
                                        // Acknowledge and destroy
                                        if let Err(e) = std::fs::remove_file(&path) {
                                            tracing::error!(
                                                "Failed to delete job file {:?}: {}",
                                                path,
                                                e
                                            );
                                        }
                                    } else {
                                        tracing::warn!(
                                            "Upload queue full, keeping job file for later: {:?}",
                                            path
                                        );
                                    }
                                } else {
                                    tracing::warn!("Target path in job file does not exist, deleting job: {:?}", path);
                                    let _ = std::fs::remove_file(&path);
                                }
                            }
                            continue; // Skip normal debouncing for job files
                        }
                    }

                    if let Some(handle) = timers.remove(&path) {
                        handle.abort();
                    }
                    let tx = upload_tx.clone();
                    let p = path.clone();
                    let paused_ref = paused.clone();
                    let handle = tokio::spawn(async move {
                        tokio::time::sleep(Duration::from_millis(debounce_ms)).await;
                        if !paused_ref.load(Ordering::Relaxed) {
                            let _ = tx.send(p).await;
                        }
                    });
                    timers.insert(path, handle);
                }
            }
        }
    }
    // rx closed — all pending timer handles are dropped (aborted) here
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri::Manager;
    use crate::config::{AppConfig, DaemonConfig};
    use std::sync::{atomic::AtomicBool, Arc};
    use tokio::sync::RwLock;

    fn make_config(debounce_ms: u64) -> SharedConfig {
        Arc::new(RwLock::new(AppConfig {
            daemon: DaemonConfig {
                debounce_ms,
                upload_workers: 4,
                log_level: "info".into(),
                follow_symlinks: false,
                start_on_login: false,
                scan_interval_mins: 60,
            },
            ..Default::default()
        }))
    }

    #[tokio::test]
    async fn coalesces_rapid_events() {
        let (watcher_tx, watcher_rx) = mpsc::channel(64);
        let (upload_tx, mut upload_rx) = mpsc::channel(64);
        let (rename_tx, _rename_rx) = mpsc::channel(64);
        let config = make_config(50);
        let paused = Arc::new(AtomicBool::new(false));

        let app = tauri::test::mock_app();
        tokio::spawn(start(watcher_rx, upload_tx, rename_tx, config, paused, app.app_handle().clone()));

        let path = PathBuf::from("/tmp/test_file.txt");

        for _ in 0..5 {
            let event = notify::Event {
                kind: notify::EventKind::Modify(notify::event::ModifyKind::Any),
                paths: vec![path.clone()],
                attrs: Default::default(),
            };
            watcher_tx.send(event).await.unwrap();
        }

        tokio::time::sleep(Duration::from_millis(200)).await;

        let item = upload_rx.try_recv().unwrap();
        assert_eq!(item, path);
        assert!(
            upload_rx.try_recv().is_err(),
            "expected only 1 upload, got more"
        );
    }

    #[tokio::test]
    async fn rename_both_routes_to_rename_channel() {
        let (watcher_tx, watcher_rx) = mpsc::channel(64);
        let (upload_tx, mut upload_rx) = mpsc::channel(64);
        let (rename_tx, mut rename_rx) = mpsc::channel(64);
        let config = make_config(50);
        let paused = Arc::new(AtomicBool::new(false));

        let app = tauri::test::mock_app();
        tokio::spawn(start(watcher_rx, upload_tx, rename_tx, config, paused, app.app_handle().clone()));

        let old = PathBuf::from("/tmp/old.txt");
        let new = PathBuf::from("/tmp/new.txt");

        let event = notify::Event {
            kind: notify::EventKind::Modify(notify::event::ModifyKind::Name(
                notify::event::RenameMode::Both,
            )),
            paths: vec![old.clone(), new.clone()],
            attrs: Default::default(),
        };
        watcher_tx.send(event).await.unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;

        let (got_old, got_new) = rename_rx.try_recv().unwrap();
        assert_eq!(got_old, old);
        assert_eq!(got_new, new);
        assert!(
            upload_rx.try_recv().is_err(),
            "rename must not go to upload"
        );
    }

    #[tokio::test]
    async fn rename_from_to_routes_to_rename_channel() {
        let (watcher_tx, watcher_rx) = mpsc::channel(64);
        let (upload_tx, mut upload_rx) = mpsc::channel(64);
        let (rename_tx, mut rename_rx) = mpsc::channel(64);
        let config = make_config(50);
        let paused = Arc::new(AtomicBool::new(false));

        let app = tauri::test::mock_app();
        tokio::spawn(start(watcher_rx, upload_tx, rename_tx, config, paused, app.app_handle().clone()));

        let old = PathBuf::from("/tmp/from.txt");
        let new = PathBuf::from("/tmp/to.txt");

        watcher_tx
            .send(notify::Event {
                kind: notify::EventKind::Modify(notify::event::ModifyKind::Name(
                    notify::event::RenameMode::From,
                )),
                paths: vec![old.clone()],
                attrs: Default::default(),
            })
            .await
            .unwrap();

        watcher_tx
            .send(notify::Event {
                kind: notify::EventKind::Modify(notify::event::ModifyKind::Name(
                    notify::event::RenameMode::To,
                )),
                paths: vec![new.clone()],
                attrs: Default::default(),
            })
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;

        let (got_old, got_new) = rename_rx.try_recv().unwrap();
        assert_eq!(got_old, old);
        assert_eq!(got_new, new);
        assert!(
            upload_rx.try_recv().is_err(),
            "rename must not go to upload"
        );
    }

    #[tokio::test]
    async fn orphan_to_event_treated_as_upload() {
        let (watcher_tx, watcher_rx) = mpsc::channel(64);
        let (upload_tx, mut upload_rx) = mpsc::channel(64);
        let (rename_tx, mut rename_rx) = mpsc::channel(64);
        let config = make_config(50);
        let paused = Arc::new(AtomicBool::new(false));

        let app = tauri::test::mock_app();
        tokio::spawn(start(watcher_rx, upload_tx, rename_tx, config, paused, app.app_handle().clone()));

        let new = PathBuf::from("/tmp/orphan_to.txt");

        watcher_tx
            .send(notify::Event {
                kind: notify::EventKind::Modify(notify::event::ModifyKind::Name(
                    notify::event::RenameMode::To,
                )),
                paths: vec![new.clone()],
                attrs: Default::default(),
            })
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;

        let item = upload_rx.try_recv().unwrap();
        assert_eq!(item, new);
        assert!(rename_rx.try_recv().is_err(), "no rename pair available");
    }

    #[tokio::test]
    async fn tracked_interleaved_renames_paired_correctly() {
        let (watcher_tx, watcher_rx) = mpsc::channel(64);
        let (upload_tx, mut upload_rx) = mpsc::channel(64);
        let (rename_tx, mut rename_rx) = mpsc::channel(64);
        let config = make_config(50);
        let paused = Arc::new(AtomicBool::new(false));

        let app = tauri::test::mock_app();
        tokio::spawn(start(watcher_rx, upload_tx, rename_tx, config, paused, app.app_handle().clone()));

        let old1 = PathBuf::from("/tmp/old1.txt");
        let new1 = PathBuf::from("/tmp/new1.txt");
        let old2 = PathBuf::from("/tmp/old2.txt");
        let new2 = PathBuf::from("/tmp/new2.txt");

        let mut event1_from = notify::Event {
            kind: notify::EventKind::Modify(notify::event::ModifyKind::Name(
                notify::event::RenameMode::From,
            )),
            paths: vec![old1.clone()],
            attrs: Default::default(),
        };
        event1_from.attrs.set_tracker(1);

        let mut event2_from = notify::Event {
            kind: notify::EventKind::Modify(notify::event::ModifyKind::Name(
                notify::event::RenameMode::From,
            )),
            paths: vec![old2.clone()],
            attrs: Default::default(),
        };
        event2_from.attrs.set_tracker(2);

        let mut event1_to = notify::Event {
            kind: notify::EventKind::Modify(notify::event::ModifyKind::Name(
                notify::event::RenameMode::To,
            )),
            paths: vec![new1.clone()],
            attrs: Default::default(),
        };
        event1_to.attrs.set_tracker(1);

        let mut event2_to = notify::Event {
            kind: notify::EventKind::Modify(notify::event::ModifyKind::Name(
                notify::event::RenameMode::To,
            )),
            paths: vec![new2.clone()],
            attrs: Default::default(),
        };
        event2_to.attrs.set_tracker(2);

        // Send out of order: From1, From2, To2, To1
        watcher_tx.send(event1_from).await.unwrap();
        watcher_tx.send(event2_from).await.unwrap();
        watcher_tx.send(event2_to).await.unwrap();
        watcher_tx.send(event1_to).await.unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;

        let mut pairs = Vec::new();
        while let Ok(pair) = rename_rx.try_recv() {
            pairs.push(pair);
        }

        assert_eq!(pairs.len(), 2);
        assert!(pairs.contains(&(old1, new1)));
        assert!(pairs.contains(&(old2, new2)));
        assert!(upload_rx.try_recv().is_err());
    }
}
