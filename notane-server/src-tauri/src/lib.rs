pub mod api;
pub mod db;
pub mod models;
pub mod services;
pub mod utils;

#[cfg(feature = "gui")]
pub mod tauri_cmds;

#[cfg(feature = "gui")]
use tauri_cmds::{get_system_info, get_system_printers};

#[cfg(feature = "gui")]
pub fn run_server_manager(context: tauri::Context<tauri::Wry>) {
    use tauri::Manager;
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_system_info,
            get_system_printers
        ])
        .run(context)
        .expect("error while running notane server manager application");
}
