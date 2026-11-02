pub mod api;
pub mod db;
pub mod models;
pub mod services;
pub mod utils;

#[cfg(feature = "gui")]
pub mod tauri_cmds;

#[cfg(feature = "gui")]
use tauri_cmds::{
    get_system_info, get_system_printers, get_system_storage_paths, open_folder,
    save_file_to_directory, select_folder,
};

#[cfg(feature = "gui")]
pub fn run_server_manager(context: tauri::Context<tauri::Wry>) {
    use tauri::{
        menu::{MenuBuilder, MenuItemBuilder},
        tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
        Manager,
    };

    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Setup System Tray
            let show_item = MenuItemBuilder::with_id("show", "Tampilkan Notane Server").build(app)?;
            let browser_item = MenuItemBuilder::with_id("browser", "Buka di Web Browser").build(app)?;
            let status_item = MenuItemBuilder::with_id("status", "Status: Engine Aktif (Port 18080)")
                .enabled(false)
                .build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "Keluar Aplikasi").build(app)?;

            let tray_menu = MenuBuilder::new(app)
                .item(&status_item)
                .separator()
                .item(&show_item)
                .item(&browser_item)
                .separator()
                .item(&quit_item)
                .build()?;

            let mut tray_builder = TrayIconBuilder::new()
                .menu(&tray_menu)
                .tooltip("Notane Retail Server (Host Engine)")
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    match event.id().as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.unminimize();
                                let _ = window.set_focus();
                            }
                        }
                        "browser" => {
                            use tauri_plugin_opener::OpenerExt;
                            let _ = app.opener().open_url("http://localhost:18080", None::<&str>);
                        }
                        "quit" => {
                            tracing::info!("Menghentikan Notane Server dari System Tray...");
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.unminimize();
                                let _ = window.set_focus();
                            }
                        }
                    }
                });

            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            }

            let _ = tray_builder.build(app)?;

            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Jangan keluar aplikasi, sembunyikan window ke background / system tray
                let _ = window.hide();
                api.prevent_close();
                tracing::info!("Notane Server disembunyikan ke System Tray (Engine tetap berjalan di background).");
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_system_info,
            get_system_printers,
            get_system_storage_paths,
            select_folder,
            open_folder,
            save_file_to_directory
        ]);

    let app = builder
        .build(context)
        .expect("error while running notane server manager application");

    app.run(|_app_handle, _event| {
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Reopen { .. } = _event {
            if let Some(window) = _app_handle.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }
    });
}
