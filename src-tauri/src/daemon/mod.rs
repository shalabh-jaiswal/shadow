pub mod debouncer;
pub mod filter;
pub mod hasher;
pub mod integration;
pub mod queue;
pub mod renamer;
pub mod scanner;
pub mod stats;
pub mod watcher;

use crate::config::SharedConfig;
use crate::providers::{gcs::GcsProvider, nas::NasProvider, s3::S3Provider, DynProvider};
use anyhow::Result;
use notify::RecommendedWatcher;
use sled::Db;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;
use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;

pub use stats::DaemonStats;

pub struct DaemonState {
    pub config: SharedConfig,
    pub app_handle: AppHandle,
    pub upload_tx: mpsc::Sender<std::path::PathBuf>,
    pub task_handles: Vec<JoinHandle<()>>,
    pub watcher: Option<RecommendedWatcher>,
    /// Sled hash database — exposed so `clear_hash_store` IPC command can flush it.
    pub db: Db,
    /// Live upload counters exposed via `get_stats`.
    pub stats: DaemonStats,
    /// Atomic flag to pause/resume backup processing.
    pub paused: Arc<AtomicBool>,
    /// Atomic flag to prevent concurrent manual/scheduled scans.
    pub is_scanning: Arc<AtomicBool>,
    /// Watch channel sender — push a new provider list whenever config changes.
    pub provider_tx: watch::Sender<Vec<DynProvider>>,
}

/// Build the active provider list from the current config.
/// Called at startup and whenever provider config changes.
pub async fn build_providers(config: &SharedConfig) -> Vec<DynProvider> {
    let cfg = config.read().await;
    let mut p: Vec<DynProvider> = Vec::new();
    if cfg.nas.enabled && !cfg.nas.mount_path.is_empty() {
        p.push(Arc::new(NasProvider::new(&cfg.nas.mount_path)));
    }
    if cfg.s3.enabled && !cfg.s3.bucket.is_empty() {
        match S3Provider::new(&cfg.s3.region, &cfg.s3.bucket, &cfg.s3.profile).await {
            Ok(provider) => p.push(Arc::new(provider)),
            Err(e) => tracing::error!(error = %e, "S3 provider init failed"),
        }
    }
    if cfg.gcs.enabled && !cfg.gcs.bucket.is_empty() {
        match GcsProvider::new(&cfg.gcs.bucket, &cfg.gcs.credentials_path).await {
            Ok(provider) => p.push(Arc::new(provider)),
            Err(e) => tracing::error!(error = %e, "GCS provider init failed"),
        }
    }
    p
}

pub async fn apply_autostart_setting(app: &tauri::AppHandle, enabled: bool) -> anyhow::Result<()> {
    let autostart = app.autolaunch();
    if enabled {
        autostart.enable()?;
    } else {
        autostart.disable()?;
    }
    Ok(())
}

pub async fn ensure_autostart(app: &tauri::AppHandle, config: &SharedConfig) -> anyhow::Result<()> {
    let autostart = app.autolaunch();
    let cfg = config.read().await;

    if cfg.daemon.start_on_login && !autostart.is_enabled()? {
        tracing::info!("First launch — registering autostart");
        autostart.enable()?;
    }
    drop(cfg);

    // Belt-and-suspenders: write the platform startup file directly so
    // autostart works even when tauri-plugin-autostart fails (mirrors the
    // Windows startup-folder script in the installer). Overwrites on every
    // launch so the path stays correct after app moves/updates.
    write_startup_entry().await;
    Ok(())
}

async fn write_startup_entry() {
    #[cfg(target_os = "macos")]
    {
        if let Err(e) = write_launchagent_plist() {
            tracing::warn!(error = %e, "Failed to write LaunchAgent plist");
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Err(e) = write_xdg_autostart() {
            tracing::warn!(error = %e, "Failed to write XDG autostart entry");
        }
    }
}

#[cfg(target_os = "macos")]
fn write_launchagent_plist() -> anyhow::Result<()> {
    let exe = std::env::current_exe()?;
    let exe_str = exe.to_string_lossy();

    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.shadow.app</string>
    <key>ProgramArguments</key>
    <array>
        <string>{exe_str}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <false/>
</dict>
</plist>
"#
    );

    let launch_agents = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("cannot find home dir"))?
        .join("Library/LaunchAgents");
    std::fs::create_dir_all(&launch_agents)?;
    std::fs::write(launch_agents.join("com.shadow.app.plist"), plist)?;
    tracing::info!("LaunchAgent plist written");
    Ok(())
}

#[cfg(target_os = "linux")]
fn write_xdg_autostart() -> anyhow::Result<()> {
    let exe = std::env::current_exe()?;
    let exe_str = exe.to_string_lossy();

    let desktop = format!(
        "[Desktop Entry]\nType=Application\nName=Shadow\nExec={exe_str}\nX-GNOME-Autostart-enabled=true\n"
    );

    let autostart_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("cannot find home dir"))?
        .join(".config/autostart");
    std::fs::create_dir_all(&autostart_dir)?;
    std::fs::write(autostart_dir.join("shadow.desktop"), desktop)?;
    tracing::info!("XDG autostart entry written");
    Ok(())
}

pub async fn start(config: SharedConfig, app_handle: AppHandle) -> Result<DaemonState> {
    let (upload_tx, upload_rx) = mpsc::channel::<std::path::PathBuf>(512);
    let (rename_tx, rename_rx) = mpsc::channel::<(std::path::PathBuf, std::path::PathBuf)>(256);

    let db = hasher::open_db()?;
    let stats = DaemonStats::load(&db);
    let paused = Arc::new(AtomicBool::new(false));
    let is_scanning = Arc::new(AtomicBool::new(false));

    // First launch autostart registration
    if let Err(e) = ensure_autostart(&app_handle, &config).await {
        tracing::warn!(error = %e, "Failed to ensure autostart setting");
    }

    // Set up OS explorer integration (Send To, Quick Actions, etc.)
    if let Err(e) = integration::setup_os_integration() {
        tracing::warn!(error = %e, "Failed to set up OS integration");
    }

    // Process any lingering spool jobs from a previous crash
    let jobs_dir = crate::path_utils::get_jobs_dir();
    if let Ok(entries) = std::fs::read_dir(&jobs_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("shadow_job") {
                if let Ok(contents) = std::fs::read_to_string(&path) {
                    let target_path = PathBuf::from(contents.trim());
                    if target_path.exists() {
                        tracing::debug!("Spool recovered job on startup: {:?}", target_path);
                        let _ = upload_tx.try_send(target_path);
                    }
                    // Always delete the job file to avoid infinitely retrying dead paths
                    let _ = std::fs::remove_file(&path);
                }
            }
        }
    }

    let initial_providers = build_providers(&config).await;
    let (provider_tx, provider_rx) = watch::channel(initial_providers);

    let mut task_handles = Vec::new();

    // Spawn queue worker pool
    let queue_handle = {
        let db = db.clone();
        let config = config.clone();
        let app_handle = app_handle.clone();
        let stats = stats.clone();
        tokio::spawn(queue::start(
            upload_rx,
            provider_rx.clone(),
            db,
            config,
            app_handle,
            stats,
        ))
    };
    task_handles.push(queue_handle);

    // Spawn rename worker
    let renamer_handle = {
        let db = db.clone();
        let config = config.clone();
        let app_handle = app_handle.clone();
        let upload_tx_clone = upload_tx.clone();
        tokio::spawn(renamer::start(
            rename_rx,
            upload_tx_clone,
            provider_rx.clone(),
            db,
            config,
            app_handle,
        ))
    };
    task_handles.push(renamer_handle);

    // Create watcher → debouncer channel
    let (watcher_tx, watcher_rx) = mpsc::channel::<notify::Event>(256);

    // Spawn debouncer
    let debouncer_handle = {
        let config = config.clone();
        let paused_ref = paused.clone();
        let app_handle_clone = app_handle.clone();
        tokio::spawn(debouncer::start(
            watcher_rx,
            upload_tx.clone(),
            rename_tx,
            config,
            paused_ref,
            app_handle_clone,
        ))
    };
    task_handles.push(debouncer_handle);

    // Spawn periodic scheduled scan task
    let scan_interval_mins = config.read().await.daemon.scan_interval_mins;
    if scan_interval_mins > 0 {
        let interval_secs = scan_interval_mins * 60;
        let config_clone = config.clone();
        let db_clone = db.clone();
        let tx_clone = upload_tx.clone();
        let app_handle_clone = app_handle.clone();
        let paused_clone = paused.clone();
        let is_scanning_clone = is_scanning.clone();
        let provider_rx_clone = provider_rx.clone();

        let scheduled_scan_handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
            interval.tick().await; // consume immediate first tick

            loop {
                interval.tick().await;

                if paused_clone.load(Ordering::Relaxed) {
                    continue;
                }

                if is_scanning_clone.load(Ordering::SeqCst) {
                    tracing::info!("Skipping periodic scan: a scan is already in progress");
                    continue;
                }

                tracing::info!(
                    "Periodic scan triggered (interval: {}min)",
                    config_clone.read().await.daemon.scan_interval_mins
                );

                is_scanning_clone.store(true, Ordering::SeqCst);
                let provider_names: Vec<String> = provider_rx_clone
                    .borrow()
                    .iter()
                    .map(|p| p.name().to_string())
                    .collect();
                scanner::scan_all_folders(
                    &config_clone,
                    &db_clone,
                    &tx_clone,
                    &app_handle_clone,
                    scanner::ScanTrigger::Scheduled,
                    provider_names,
                )
                .await;
                is_scanning_clone.store(false, Ordering::SeqCst);
            }
        });
        task_handles.push(scheduled_scan_handle);
    }

    // Create notify watcher
    let mut notify_watcher = watcher::create(watcher_tx)?;

    // Ensure the jobs directory exists and is watched for ad-hoc backups
    let jobs_dir = crate::path_utils::get_jobs_dir();
    if let Err(e) = std::fs::create_dir_all(&jobs_dir) {
        tracing::error!(error = %e, "failed to create jobs directory");
    } else if let Err(e) = watcher::watch_path(&mut notify_watcher, &jobs_dir) {
        tracing::error!(dir = %jobs_dir.display(), error = %e, "failed to register jobs directory with watcher");
    }

    // Register all configured watched folders
    {
        let cfg = config.read().await;
        for folder in &cfg.watched_folders.paths {
            let path = Path::new(folder);
            if path.exists() {
                if let Err(e) = watcher::watch_path(&mut notify_watcher, path) {
                    tracing::error!(folder = %folder, error = %e, "failed to register folder with watcher");
                }
            }
        }
    }

    Ok(DaemonState {
        config,
        app_handle,
        upload_tx,
        task_handles,
        watcher: Some(notify_watcher),
        db,
        stats,
        paused,
        is_scanning,
        provider_tx,
    })
}

#[allow(dead_code)]
pub async fn shutdown(mut state: DaemonState) -> Result<()> {
    // 1. Drop watcher first (stops new FS events)
    drop(state.watcher.take());

    // 2. Drop upload_tx (signals queue EOF after debouncer drains)
    drop(state.upload_tx);

    // 3. Join all tasks with a timeout
    let timeout = std::time::Duration::from_secs(10);
    for handle in state.task_handles {
        let _ = tokio::time::timeout(timeout, handle).await;
    }

    Ok(())
}

impl DaemonState {
    /// Rebuild the provider list from the current config and broadcast it to
    /// queue and renamer workers. Call this after any provider config change.
    pub async fn rebuild_providers(&self) -> anyhow::Result<()> {
        let providers = build_providers(&self.config).await;
        tracing::info!(count = providers.len(), "Provider list rebuilt");
        let _ = self.provider_tx.send(providers);
        Ok(())
    }

    /// Spawn a background scan for the given folder path.
    /// Files with no sled entry are enqueued for upload.
    pub fn spawn_scan(&self, folder_path: PathBuf) {
        let provider_names: Vec<String> = self
            .provider_tx
            .subscribe()
            .borrow()
            .iter()
            .map(|p| p.name().to_string())
            .collect();
        scanner::spawn_scan(
            folder_path,
            self.config.clone(),
            self.db.clone(),
            self.upload_tx.clone(),
            self.app_handle.clone(),
            scanner::ScanTrigger::Initial,
            provider_names,
        );
    }

    pub async fn trigger_manual_scan(&self) -> anyhow::Result<()> {
        // Prevent concurrent manual scans
        if self.is_scanning.load(Ordering::SeqCst) {
            return Err(anyhow::anyhow!("A scan is already in progress"));
        }

        tracing::info!("Manual recovery scan triggered by user");

        let config = self.config.clone();
        let db = self.db.clone();
        let tx = self.upload_tx.clone();
        let app_handle = self.app_handle.clone();
        let is_scanning = self.is_scanning.clone();
        let provider_rx = self.provider_tx.subscribe();

        tokio::spawn(async move {
            is_scanning.store(true, Ordering::SeqCst);
            let provider_names: Vec<String> = provider_rx
                .borrow()
                .iter()
                .map(|p| p.name().to_string())
                .collect();
            scanner::scan_all_folders(
                &config,
                &db,
                &tx,
                &app_handle,
                scanner::ScanTrigger::Manual,
                provider_names,
            )
            .await;
            is_scanning.store(false, Ordering::SeqCst);
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn folder_mode_sled_key_format() {
        // Test the expected key format for folder modes
        let path = "/Users/test/Documents";
        let expected_key = format!("folder_mode:{path}");
        assert_eq!(expected_key, "folder_mode:/Users/test/Documents");

        // Test Windows path
        let windows_path = "C:\\Users\\test\\Documents";
        let windows_key = format!("folder_mode:{windows_path}");
        assert_eq!(windows_key, "folder_mode:C:\\Users\\test\\Documents");
    }

    #[test]
    fn folder_mode_values() {
        // Test expected mode values
        let full_mode = "full";
        let forward_only_mode = "forward_only";

        // These should be the exact strings we store/retrieve
        assert_eq!(full_mode.as_bytes(), b"full");
        assert_eq!(forward_only_mode.as_bytes(), b"forward_only");
    }
}
