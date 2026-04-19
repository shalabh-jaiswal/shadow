use crate::config::SharedConfig;
use notify::event::{ModifyKind, RenameMode};
use notify::{Event, EventKind};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub async fn start(
    mut rx: mpsc::Receiver<Event>,
    upload_tx: mpsc::Sender<PathBuf>,
    rename_tx: mpsc::Sender<(PathBuf, PathBuf)>,
    config: SharedConfig,
    paused: Arc<AtomicBool>,
) {
    let mut timers: HashMap<PathBuf, JoinHandle<()>> = HashMap::new();
    // Holds the source path of a RenameMode::From event waiting for its matching To
    let mut pending_rename_from: Option<PathBuf> = None;

    while let Some(event) = rx.recv().await {
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
                    pending_rename_from = Some(old);
                }
            }
            EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
                if let Some(new) = event.paths.into_iter().next() {
                    if !paused.load(Ordering::Relaxed) {
                        if let Some(old) = pending_rename_from.take() {
                            let _ = rename_tx.send((old, new)).await;
                        } else {
                            // No matching From — treat as a new file
                            let _ = upload_tx.send(new).await;
                        }
                    } else {
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
                        pending_rename_from = None;
                        continue;
                    }
                    if path.exists() {
                        // New path — pair with pending source if available
                        if let Some(old) = pending_rename_from.take() {
                            let _ = rename_tx.send((old, path)).await;
                        } else {
                            // No known source; treat as new file
                            let _ = upload_tx.send(path).await;
                        }
                    } else {
                        // Old path — store and wait for destination event
                        pending_rename_from = Some(path);
                    }
                }
            }
            _ => {
                let debounce_ms = config.read().await.daemon.debounce_ms;
                for path in event.paths {
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
                reconcile_interval_mins: 60,
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

        tokio::spawn(start(watcher_rx, upload_tx, rename_tx, config, paused));

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

        tokio::spawn(start(watcher_rx, upload_tx, rename_tx, config, paused));

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
        assert!(upload_rx.try_recv().is_err(), "rename must not go to upload");
    }

    #[tokio::test]
    async fn rename_from_to_routes_to_rename_channel() {
        let (watcher_tx, watcher_rx) = mpsc::channel(64);
        let (upload_tx, mut upload_rx) = mpsc::channel(64);
        let (rename_tx, mut rename_rx) = mpsc::channel(64);
        let config = make_config(50);
        let paused = Arc::new(AtomicBool::new(false));

        tokio::spawn(start(watcher_rx, upload_tx, rename_tx, config, paused));

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
        assert!(upload_rx.try_recv().is_err(), "rename must not go to upload");
    }

    #[tokio::test]
    async fn orphan_to_event_treated_as_upload() {
        let (watcher_tx, watcher_rx) = mpsc::channel(64);
        let (upload_tx, mut upload_rx) = mpsc::channel(64);
        let (rename_tx, mut rename_rx) = mpsc::channel(64);
        let config = make_config(50);
        let paused = Arc::new(AtomicBool::new(false));

        tokio::spawn(start(watcher_rx, upload_tx, rename_tx, config, paused));

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
}
