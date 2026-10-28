// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use notane_retail_server_lib::db::{create_pool, run_embedded_migrations};
use std::env;
use std::net::SocketAddr;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "notane_retail_server=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    dotenvy::dotenv().ok();

    let pg_port: u16 = env::var("POSTGRES_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(15432);

    let default_db_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        format!("postgres://postgres:postgres@127.0.0.1:{}/notane_pos", pg_port)
    });

    let port: u16 = env::var("PORT")
        .or_else(|_| env::var("SERVER_PORT"))
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(18080);
    let control_port: u16 = env::var("CONTROL_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(port + 1);

    let args: Vec<String> = env::args().collect();
    let is_headless = args.iter().any(|a| a == "--headless" || a == "--server-only" || a == "--daemon") || cfg!(not(feature = "gui"));
    let is_standby = args.iter().any(|a| a == "--standby");
    // Host Server otomatis aktif (Auto-Start) saat aplikasi dibuka baik GUI maupun Daemon
    let auto_start = !is_standby;

    info!("=====================================================");
    info!("   NOTANE RETAIL SERVER (HOST & DATABASE ENGINE)    ");
    info!("=====================================================");

    // Inisialisasi pool database opsional (hanya jika auto_start atau headless)
    let mut initial_pool: Option<sqlx::PgPool> = None;
    if auto_start || is_headless {
        let embedded_manager = notane_retail_server_lib::db::embedded::EmbeddedPgManager::new();
        if default_db_url.contains("127.0.0.1") || default_db_url.contains("localhost") {
            if !notane_retail_server_lib::db::embedded::EmbeddedPgManager::is_port_active("127.0.0.1", pg_port).await {
                let _ = embedded_manager.start_if_needed(pg_port).await;
            }
        }
        if let Ok(pool) = create_pool(&default_db_url).await {
            info!("Koneksi database PostgreSQL terisolasi berhasil (Port {}).", pg_port);
            if let Err(e) = run_embedded_migrations(&pool).await {
                error!("Peringatan migrasi database skema: {}", e);
            }
            notane_retail_server_lib::services::license_service::LicenseService::start_license_sync_scheduler(pool.clone());
            initial_pool = Some(pool);
        } else {
            error!("Gagal membuat database pool ke {}. Memeriksa konfigurasi...", default_db_url);
        }
    }

    // Periksa status lisensi sebelum menjalankan layanan transaksi POS
    let is_licensed = if let Some(ref pool) = initial_pool {
        let lic = notane_retail_server_lib::services::license_service::LicenseService::check_current_license(pool).await;
        if lic.is_valid {
            info!("Lisensi Server Terverifikasi: {:?} ({})", lic.status, lic.message);
            true
        } else {
            tracing::warn!("Server Retail POS terkunci! Status lisensi: {:?}. Pesan: {}", lic.status, lic.message);
            false
        }
    } else {
        false
    };

    let ws_state = notane_retail_server_lib::api::ws::AppState::new(
        initial_pool,
        port,
        default_db_url.clone(),
        pg_port,
    );
    ws_state.start_heartbeat_scheduler();
    ws_state.start_voucher_expiry_scheduler();

    // 1. Host Control Channel (Port 18081) - Selalu standby untuk mengontrol status server & aktivasi lisensi
    let control_addr = SocketAddr::from(([0, 0, 0, 0], control_port));
    let control_app = notane_retail_server_lib::api::create_control_router(ws_state.clone());
    tokio::spawn(async move {
        if let Ok(listener) = tokio::net::TcpListener::bind(control_addr).await {
            info!("Host Control Channel berjalan di http://{}", control_addr);
            let _ = axum::serve(listener, control_app).await;
        } else {
            error!("Gagal mengikat socket Host Control Channel pada port {}", control_port);
        }
    });

    // 2. POS API Server di port 18080: Selalu aktif otomatis saat auto_start (Host Manager & API)
    if auto_start {
        match notane_retail_server_lib::api::spawn_pos_server(ws_state.clone()).await {
            Ok(handle) => {
                ws_state.set_server_handle(handle).await;
                let mut running = ws_state.is_running.write().await;
                *running = is_licensed;
                let mut started = ws_state.started_at.write().await;
                *started = Some(chrono::Utc::now());
                if is_licensed {
                    info!("Server Retail POS API & WebSockets SIAP di http://0.0.0.0:{}", port);
                } else {
                    info!("Server Retail POS Host Manager SIAP di http://0.0.0.0:{} (Menunggu Aktivasi Lisensi)", port);
                }
            }
            Err(e) => {
                error!("Gagal mengikat port POS {}: {}", port, e);
            }
        }
    } else {
        info!("Server Host dalam status SIAGA (STANDBY). Menunggu perintah 'Nyalakan Server' dari antarmuka pengguna.");
    }

    // 3. Server Watchdog (Self-Healing Background Loop dengan auto-reconnect database & lisensi)
    let watchdog_state = ws_state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3));
        loop {
            interval.tick().await;

            // Auto-reconnect database jika saat startup pool masih None
            if watchdog_state.pool.read().await.is_none() {
                let db_url = watchdog_state.db_url.read().await.clone();
                if let Ok(pool) = notane_retail_server_lib::db::create_pool(&db_url).await {
                    let _ = notane_retail_server_lib::db::run_embedded_migrations(&pool).await;
                    notane_retail_server_lib::services::license_service::LicenseService::start_license_sync_scheduler(pool.clone());
                    watchdog_state.set_pool(pool).await;
                    tracing::info!("[WATCHDOG] Koneksi database PostgreSQL terpulihkan!");
                    // Respawn POS server dengan router yang sudah memiliki database pool
                    if let Ok(handle) = notane_retail_server_lib::api::spawn_pos_server(watchdog_state.clone()).await {
                        watchdog_state.set_server_handle(handle).await;
                    }
                }
            }

            // Verifikasi integritas lisensi
            let pool_opt = watchdog_state.pool.read().await.clone();
            let is_lic_valid = if let Some(ref p) = pool_opt {
                notane_retail_server_lib::services::license_service::LicenseService::check_current_license(p).await.is_valid
            } else {
                false
            };

            // Update status transaksi kasir secara dinamis tanpa mematikan socket HTTP
            {
                let mut running = watchdog_state.is_running.write().await;
                *running = is_lic_valid;
            }

            let is_port_live = tokio::net::TcpStream::connect(format!("127.0.0.1:{}", watchdog_state.port)).await.is_ok();
            if !is_port_live {
                tracing::warn!("[WATCHDOG] Port POS {} terputus mendadak! Memulihkan otomatis...", watchdog_state.port);
                match notane_retail_server_lib::api::spawn_pos_server(watchdog_state.clone()).await {
                    Ok(handle) => {
                        watchdog_state.set_server_handle(handle).await;
                        tracing::info!("[WATCHDOG] Server POS berhasil dipulihkan otomatis di port {}", watchdog_state.port);
                    }
                    Err(e) => {
                        tracing::error!("[WATCHDOG] Gagal memulihkan server POS: {}", e);
                    }
                }
            }
        }
    });

    // Jika dipanggil mode headless (daemon background)
    #[cfg(feature = "gui")]
    if is_headless {
        info!("=== DAEMON HOST RUNNING (Tekan Ctrl+C untuk keluar) ===");
        tokio::signal::ctrl_c().await?;
        info!("Notane Retail Server daemon dihentikan.");
        return Ok(());
    }

    #[cfg(feature = "gui")]
    notane_retail_server_lib::run_server_manager(tauri::generate_context!());

    #[cfg(not(feature = "gui"))]
    {
        info!("=== NOTANE RETAIL SERVER HEADLESS DAEMON RUNNING ===");
        info!("Tekan Ctrl+C untuk menghentikan server.");
        tokio::signal::ctrl_c().await?;
        info!("Notane Retail Server daemon dihentikan.");
    }

    Ok(())
}
