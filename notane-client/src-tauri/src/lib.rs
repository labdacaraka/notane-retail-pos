pub mod tauri_cmds;

use tauri_cmds::{
    get_client_version, get_system_info, get_system_printers, get_system_storage_paths,
    open_cash_drawer, open_customer_display_window, open_folder, print_html_document,
    print_receipt, save_file_to_directory, select_folder, silent_print_raw,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run(context: tauri::Context<tauri::Wry>) {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_client_version,
            get_system_info,
            open_cash_drawer,
            print_receipt,
            open_customer_display_window,
            get_system_printers,
            silent_print_raw,
            print_html_document,
            get_system_storage_paths,
            select_folder,
            open_folder,
            save_file_to_directory
        ])
        .run(context)
        .expect("error while running tauri application");
}
