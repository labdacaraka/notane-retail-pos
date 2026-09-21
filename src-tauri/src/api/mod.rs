pub mod auth;
pub mod inventory;
pub mod master;
pub mod pos;
pub mod reports;
pub mod system;
pub mod users;
pub mod ws;

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
        .merge(master::master_routes(pool.clone()))
        .nest("/inventory", inventory::inventory_routes())
        .merge(inventory::inventory_routes())
        .merge(reports::report_routes())
        .merge(users::user_management_routes())
        .with_state(pool)
        .layer(from_fn_with_state(ws_state.clone(), check_server_running_middleware));

    Router::new()
        .nest("/api/v1", api_routes)
        .nest("/api/v1/system", system::system_routes(ws_state.clone()))
        .nest("/ws/v1", ws_routes(ws_state))
        .layer(cors)
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
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], ws_state.port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let router = create_pos_router(ws_state.pool.clone(), ws_state.clone());

    let handle = tokio::spawn(async move {
        tracing::info!("[POS SERVER] Listening on http://{}", addr);
        if let Err(e) = axum::serve(listener, router).await {
            tracing::error!("[POS SERVER] Error on port {}: {}", addr.port(), e);
        }
    });

    Ok(handle.abort_handle())
}

pub fn create_router(pool: PgPool, port: u16) -> Router {
    let ws_state = AppState::new(pool.clone(), port);
    create_pos_router(pool, ws_state)
}



