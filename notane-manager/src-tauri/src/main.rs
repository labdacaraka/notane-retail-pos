// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use notane_manager_lib::build_app_state;
use std::env;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "notane_manager=debug,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Muat konfigurasi .env.manager jika ada agar tidak menabrak file .env milik notane-server di root
    if dotenvy::from_filename(".env.manager").is_err() {
        dotenvy::dotenv().ok();
    }

    info!("============================================================");
    info!("   NOTANE LICENSE & SERVER MANAGEMENT STUDIO (PROVIDER)     ");
    info!("============================================================");

    let state = build_app_state().await?;

    let args: Vec<String> = env::args().collect();
    let is_headless = args.iter().any(|a| a == "--headless" || a == "--daemon" || a == "--server-only")
        || cfg!(not(feature = "gui"));

    #[cfg(feature = "gui")]
    if is_headless {
        info!("=== PROVIDER DAEMON RUNNING (Tekan Ctrl+C untuk keluar) ===");
        tokio::signal::ctrl_c().await?;
        info!("Provider Server dihentikan.");
        return Ok(());
    }

    #[cfg(feature = "gui")]
    notane_manager_lib::run_app(tauri::generate_context!(), state);

    #[cfg(not(feature = "gui"))]
    {
        info!("=== NOTANE PROVIDER DAEMON RUNNING ===");
        info!("Tekan Ctrl+C untuk menghentikan server.");
        tokio::signal::ctrl_c().await?;
        info!("Provider Server dihentikan.");
    }

    Ok(())
}
