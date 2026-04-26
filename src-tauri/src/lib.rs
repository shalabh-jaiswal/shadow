mod config;
mod daemon;
mod ipc;
mod logger;
mod path_utils;
mod providers;

use daemon::DaemonState;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager};
use tokio::sync::Mutex;

pub struct DaemonHandle(pub Mutex<DaemonState>);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Load config early so log_level is available before the Tauri builder starts.
    let cfg = config::load().expect("failed to load config");
    let log_level = cfg.blocking_read().daemon.log_level.clone();
    let _log_guard = logger::init(&log_level);

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(move |app| {
            // Tray menu: Show Window | Pause Backup | --- | Quit Shadow
            let show = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
            let pause = MenuItem::with_id(app, "pause", "Pause Backup", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let quit = MenuItem::with_id(app, "quit", "Quit Shadow", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &pause, &sep, &quit])?;

            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Shadow")
                .menu(&menu)
                .on_menu_event(move |app, event| {
                    let app_handle = app.clone();
                    tauri::async_runtime::spawn(async move {
                        match event.id.as_ref() {
                            "show" => {
                                if let Some(window) = app_handle.get_webview_window("main") {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                            }
                            "pause" => {
                                // Toggle pause state
                                if let Some(state) = app_handle.try_state::<DaemonHandle>() {
                                    let daemon = state.0.lock().await;
                                    let current_paused =
                                        daemon.paused.load(std::sync::atomic::Ordering::Relaxed);
                                    daemon.paused.store(
                                        !current_paused,
                                        std::sync::atomic::Ordering::Relaxed,
                                    );

                                    // Emit appropriate event
                                    if !current_paused {
                                        let _ = app_handle.emit("daemon_paused", ());
                                    } else {
                                        let _ = app_handle.emit("daemon_resumed", ());
                                    }
                                }
                            }
                            "quit" => app_handle.exit(0),
                            _ => {}
                        }
                    });
                })
                .build(app)?;

            // Start daemon via Tauri's managed async runtime
            let app_handle = app.handle().clone();
            let state = tauri::async_runtime::block_on(daemon::start(cfg, app_handle))
                .expect("failed to start daemon");

            app.manage(DaemonHandle(Mutex::new(state)));
            Ok(())
        })
        // Closing the window hides it; the daemon keeps running in the background.
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            ipc::ping,
            ipc::add_folder,
            ipc::remove_folder,
            ipc::get_watched_folders,
            ipc::test_provider,
            ipc::set_provider_config,
            ipc::get_config,
            ipc::set_daemon_config,
            ipc::trigger_recovery_scan,
            ipc::get_stats,
            ipc::clear_hash_store,
            ipc::set_paused,
            ipc::get_paused,
            ipc::set_autostart,
            ipc::setup_os_integration,
            ipc::check_for_updates,
            ipc::open_url,
            ipc::open_config_folder,
            ipc::open_data_folder,
            ipc::get_log_path,
            ipc::open_log_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
