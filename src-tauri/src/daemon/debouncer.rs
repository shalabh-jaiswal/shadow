use crate::config::SharedConfig;
use notify::Event;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

pub async fn start(
    mut rx: mpsc::Receiver<Event>,
    upload_tx: mpsc::Sender<PathBuf>,
    config: SharedConfig,
    paused: Arc<AtomicBool>,
) {
    let mut timers: HashMap<PathBuf, JoinHandle<()>> = HashMap::new();

    while let Some(event) = rx.recv().await {
        let debounce_ms = config.read().await.daemon.debounce_ms;

        for path in event.paths {
            // abort any existing pending timer for this path
            if let Some(handle) = timers.remove(&path) {
                handle.abort();
            }

            let tx = upload_tx.clone();
            let p = path.clone();
            let paused_ref = paused.clone();
            let handle = tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(debounce_ms)).await;
                // Check if backup is paused before sending to upload queue
                if !paused_ref.load(Ordering::Relaxed) {
                    let _ = tx.send(p).await;
                }
            });

            timers.insert(path, handle);
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
            },
            ..Default::default()
        }))
    }

    #[tokio::test]
    async fn coalesces_rapid_events() {
        let (watcher_tx, watcher_rx) = mpsc::channel(64);
        let (upload_tx, mut upload_rx) = mpsc::channel(64);
        let config = make_config(50); // 50ms debounce for fast test
        let paused = Arc::new(AtomicBool::new(false));

        tokio::spawn(start(watcher_rx, upload_tx, config, paused));

        let path = PathBuf::from("/tmp/test_file.txt");

        // Send 5 rapid events for the same path
        for _ in 0..5 {
            let event = notify::Event {
                kind: notify::EventKind::Modify(notify::event::ModifyKind::Any),
                paths: vec![path.clone()],
                attrs: Default::default(),
            };
            watcher_tx.send(event).await.unwrap();
        }

        // Wait for debounce to settle
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Should have received exactly 1 item
        let item = upload_rx.try_recv().unwrap();
        assert_eq!(item, path);
        assert!(
            upload_rx.try_recv().is_err(),
            "expected only 1 upload, got more"
        );
    }
}
