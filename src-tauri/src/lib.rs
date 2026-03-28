mod config;
mod daemon;
mod ipc;
mod path_utils;
mod providers;

use daemon::DaemonState;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::Manager;
use tokio::sync::Mutex;

pub struct DaemonHandle(pub Mutex<DaemonState>);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Tray
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit])?;
            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| {
                    if event.id == "quit" {
                        app.exit(0);
                    }
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
        .invoke_handler(tauri::generate_handler![
            ipc::ping,
            ipc::add_folder,
            ipc::remove_folder,
            ipc::get_watched_folders,
            ipc::test_provider,
            ipc::set_provider_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
