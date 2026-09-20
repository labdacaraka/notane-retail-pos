// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use notane_retail_pos_lib::db::{create_pool, run_embedded_migrations};
use std::env;
use std::net::SocketAddr;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "notane_retail_pos=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    dotenvy::dotenv().ok();

    let default_db_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://postgres:postgres@127.0.0.1:5432/notane_pos".to_string()
    });

    info!("=== MEMULAI NOTANE RETAIL POS ENTERPRISE ===");

    let pool_res = create_pool(&default_db_url).await;
    match pool_res {
        Ok(pool) => {
            info!("Koneksi database PostgreSQL berhasil.");
            if let Err(e) = run_embedded_migrations(&pool).await {
                error!("Peringatan migrasi skema: {}", e);
            }

            let port: u16 = env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080);
            let control_port: u16 = port + 1;

            let ws_state = notane_retail_pos_lib::api::ws::AppState::new(pool.clone(), port);

            // 1. Jalankan Host Control Listener di control_port (port 8081) - selalu standby melayani perintah manajemen
            let control_addr = SocketAddr::from(([0, 0, 0, 0], control_port));
            let control_app = notane_retail_pos_lib::api::create_control_router(ws_state.clone());
            tokio::spawn(async move {
                if let Ok(listener) = tokio::net::TcpListener::bind(control_addr).await {
                    info!("Host Control Channel berjalan di http://{}", control_addr);
                    let _ = axum::serve(listener, control_app).await;
                }
            });

            // 2. Jalankan POS & Retail Service di port 8080 (dapat dimatikan total / dinyalakan kembali)
            match notane_retail_pos_lib::api::spawn_pos_server(ws_state.clone()).await {
                Ok(handle) => {
                    ws_state.set_server_handle(handle).await;
                    info!("Server Retail POS & WebSockets berjalan di http://0.0.0.0:{}", port);
                }
                Err(e) => {
                    error!("Gagal mengikat port POS {}: {}", port, e);
                }
            }

            // Check arguments: if '--server-only', run server until Ctrl+C
            let args: Vec<String> = env::args().collect();
            if args.iter().any(|a| a == "--server-only") {
                info!("=== DAEMON HOST RUNNING (Tekan Ctrl+C untuk keluar) ===");
                tokio::signal::ctrl_c().await?;
                info!("Daemon dihentikan.");
                return Ok(());
            }
        }
        Err(e) => {
            error!(
                "Tidak dapat terhubung ke database di {}: {}. Menjalankan dalam mode Thin Client...",
                default_db_url, e
            );
        }
    }

    let args: Vec<String> = env::args().collect();
    if !args.iter().any(|a| a == "--server-only") {
        notane_retail_pos_lib::run();
    }

    Ok(())
}
