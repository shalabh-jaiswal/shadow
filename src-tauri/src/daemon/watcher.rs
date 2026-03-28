use anyhow::Result;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use tokio::sync::mpsc;

pub fn create(tx: mpsc::Sender<notify::Event>) -> Result<RecommendedWatcher> {
    let watcher =
        notify::recommended_watcher(move |res: notify::Result<notify::Event>| match res {
            Ok(event) => {
                let relevant = matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_));
                if relevant {
                    let _ = tx.blocking_send(event);
                }
            }
            Err(e) => {
                eprintln!("[shadow] watcher error: {e}");
            }
        })?;
    Ok(watcher)
}

pub fn watch_path(watcher: &mut RecommendedWatcher, path: &Path) -> Result<()> {
    watcher.watch(path, RecursiveMode::Recursive)?;
    Ok(())
}

pub fn unwatch_path(watcher: &mut RecommendedWatcher, path: &Path) -> Result<()> {
    watcher.unwatch(path)?;
    Ok(())
}
