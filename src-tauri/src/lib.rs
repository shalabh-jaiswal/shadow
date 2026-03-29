mod config;
mod daemon;
mod ipc;
mod path_utils;
mod providers;

use daemon::DaemonState;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::Manager;
use tokio::sync::Mutex;

pub struct DaemonHandle(pub Mutex<DaemonState>);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Tray menu: Open Shadow | --- | Quit
            let show = MenuItem::with_id(app, "show", "Open Shadow", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &sep, &quit])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Shadow")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;

            // Load config
            let cfg = config::load().expect("failed to load config");

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
            ipc::get_stats,
            ipc::clear_hash_store,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
