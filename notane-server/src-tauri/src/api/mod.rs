pub mod auth;
pub mod flash_sales;
pub mod inventory;
pub mod master;
pub mod pos;
pub mod reports;
pub mod system;
pub mod users;
pub mod ws;
pub mod expenses;
pub mod capital;
pub mod banks;
pub mod promotions;
pub mod sales_returns;
pub mod cash_drops;
pub mod audit_middleware;
pub mod dividend;

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::{from_fn_with_state, Next},
    response::Response,
    Json, Router,
};
use serde_json::json;
use sqlx::PgPool;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use ws::{ws_routes, AppState};

async fn check_server_running_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    if !*state.is_running.read().await {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "error",
                "code": "SERVER_STOPPED",
                "message": "Host Server sedang dihentikan (Stopped). Akses ke data transaksi, produk, dan operasional kasir dinonaktifkan."
            })),
        ));
    }

    // Verifikasi Kode Pairing jika request menyertakan header X-Pairing-Code
    if let Some(pairing_header) = req.headers().get("X-Pairing-Code") {
        if let Ok(provided_code) = pairing_header.to_str() {
            let expected_code = state.pairing_code.read().await.clone();
            if !provided_code.trim().is_empty() && provided_code.trim() != expected_code.trim() {
                return Err((
                    StatusCode::UNAUTHORIZED,
                    Json(json!({
                        "status": "error",
                        "code": "INVALID_PAIRING_CODE",
                        "message": "Kode pairing client salah atau telah diperbarui oleh Host Server. Perbarui kode pairing di pengaturan koneksi."
                    })),
                ));
            }
        }
    }

    Ok(next.run(req).await)
}

pub fn create_pos_router(pool: PgPool, ws_state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api_routes = Router::new()
        .nest("/auth", auth::auth_routes())
        .merge(pos::pos_routes())
        .merge(flash_sales::flash_sale_routes())
        .merge(promotions::promotion_routes())
        .merge(sales_returns::sales_return_routes())
        .merge(cash_drops::cash_drop_routes())
        .merge(master::master_routes(pool.clone()))
        .nest("/inventory", inventory::inventory_routes(pool.clone()))
        .merge(inventory::inventory_routes(pool.clone()))
        .merge(reports::report_routes(pool.clone()))
        .merge(users::user_management_routes(pool.clone()))
        .merge(expenses::router())
        .merge(capital::router())
        .merge(banks::router())
        .merge(dividend::router())
        .layer(from_fn_with_state(pool.clone(), audit_middleware::audit_log_middleware))
        .with_state(pool)
        .layer(from_fn_with_state(ws_state.clone(), check_server_running_middleware));

    // Direktori gambar produk di filesystem lokal atau volume Docker
    let product_images_dir = std::env::var("PRODUCT_IMAGES_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join(".notane")
                .join("product_images")
        });
    // Buat direktori jika belum ada (sync OK di startup)
    let _ = std::fs::create_dir_all(&product_images_dir);

    let mut router = Router::new()
        .nest("/api/v1", api_routes)
        .nest("/api/v1/system", system::system_routes(ws_state.clone()))
        .nest("/ws/v1", ws_routes(ws_state))
        .nest_service("/uploads/product_images", ServeDir::new(product_images_dir));

    // Jika WEB_DIST ada (misal di-deploy via Docker / VPS), layani UI Web Host Manager secara langsung
    let web_dist_dir = std::env::var("WEB_DIST").unwrap_or_else(|_| "./dist".to_string());
    let dist_path = std::path::Path::new(&web_dist_dir);
    if dist_path.exists() {
        let index_file = dist_path.join("index.html");
        if index_file.exists() {
            router = router.fallback_service(
                ServeDir::new(dist_path).not_found_service(tower_http::services::ServeFile::new(index_file)),
            );
        } else {
            router = router.fallback_service(ServeDir::new(dist_path));
        }
    }

    router.layer(cors)
}

pub fn create_control_router(ws_state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .nest("/api/v1/system", system::system_routes(ws_state))
        .layer(cors)
}

pub async fn spawn_pos_server(ws_state: AppState) -> Result<tokio::task::AbortHandle, std::io::Error> {
    let pool = {
        let guard = ws_state.pool.read().await;
        guard.clone().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "Database pool belum terhubung atau belum aktif",
            )
        })?
    };
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], ws_state.port));
    let mut last_err = None;
    let mut listener_opt = None;
    for _ in 0..5 {
        match tokio::net::TcpListener::bind(addr).await {
            Ok(l) => {
                listener_opt = Some(l);
                break;
            }
            Err(e) => {
                last_err = Some(e);
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
        }
    }
    let listener = listener_opt.ok_or_else(|| {
        last_err.unwrap_or_else(|| std::io::Error::new(std::io::ErrorKind::AddrInUse, "Port sedang digunakan"))
    })?;

    // Muat atau sinkronkan kode pairing toko
    if let Ok(env_code) = std::env::var("PAIRING_CODE") {
        let code_clean = env_code.trim().to_uppercase();
        if !code_clean.is_empty() {
            let mut p = ws_state.pairing_code.write().await;
            *p = code_clean.clone();
            let _ = sqlx::query("UPDATE receipt_settings SET pairing_code = $1")
                .bind(&code_clean)
                .execute(&pool)
                .await;
            tracing::info!("[SERVER HOST] Kode pairing disinkronkan dari ENV PAIRING_CODE: {}", *p);
        }
    } else if let Ok(Some(code)) = sqlx::query_scalar::<_, String>(
        "SELECT pairing_code FROM receipt_settings WHERE pairing_code IS NOT NULL AND pairing_code != '' LIMIT 1"
    )
    .fetch_optional(&pool)
    .await {
        let mut p = ws_state.pairing_code.write().await;
        *p = code.trim().to_uppercase();
        tracing::info!("[SERVER HOST] Kode pairing aktif dimuat dari database: {}", *p);
    }

    let router = create_pos_router(pool, ws_state.clone());

    // Pastikan Host Heartbeat Scheduler & Voucher Expiry Scheduler aktif di background
    ws_state.start_heartbeat_scheduler();
    ws_state.start_voucher_expiry_scheduler();

    let handle = tokio::spawn(async move {
        tracing::info!("[POS SERVER] Listening on http://{}", addr);
        if let Err(e) = axum::serve(
            listener,
            router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        {
            tracing::error!("[POS SERVER] Error on port {}: {}", addr.port(), e);
        }
    });

    Ok(handle.abort_handle())
}

pub fn create_router(pool: PgPool, port: u16, db_url: String, pg_port: u16) -> Router {
    let ws_state = AppState::new(Some(pool.clone()), port, db_url, pg_port);
    ws_state.start_heartbeat_scheduler();
    ws_state.start_voucher_expiry_scheduler();
    create_pos_router(pool, ws_state)
}



