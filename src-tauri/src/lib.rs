pub mod api;
pub mod db;
pub mod models;
pub mod services;
pub mod tauri_cmds;
pub mod utils;

use tauri_cmds::{get_system_info, open_cash_drawer, print_receipt};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_system_info,
            open_cash_drawer,
            print_receipt
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
