use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use tracing::info;

use super::ws::AppState;

#[derive(Debug, Deserialize)]
pub struct VerifyPairingRequest {
    pub pairing_code: String,
}

pub fn system_routes(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/status", get(get_server_status))
        .route("/devices", get(list_devices))
        .route("/devices/{id}/disconnect", post(disconnect_device))
        .route("/server/start", post(start_server))
        .route("/server/stop", post(stop_server))
        .route("/pairing-code", get(get_pairing_code))
        .route("/pairing-code/regenerate", post(regenerate_pairing_code))
        .route("/pairing-code/verify", post(verify_pairing_code))
        .with_state(state)
}

async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    let is_running = *state.is_running.read().await;
    if !is_running {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "stopped",
                "app": "Notane Retail POS Server",
                "version": "1.0.0",
                "message": "Host Server sedang dihentikan (Stopped)",
                "timestamp": Utc::now()
            })),
        );
    }
    (
        StatusCode::OK,
        Json(json!({
            "status": "ok",
            "app": "Notane Retail POS Server",
            "version": "1.0.0",
            "timestamp": Utc::now()
        })),
    )
}

async fn get_server_status(State(state): State<AppState>) -> impl IntoResponse {
    let status = state.get_status().await;
    Json(status)
}

async fn get_pairing_code(State(state): State<AppState>) -> impl IntoResponse {
    let code = state.pairing_code.read().await.clone();
    Json(json!({
        "status": "success",
        "pairing_code": code
    }))
}

async fn regenerate_pairing_code(State(state): State<AppState>) -> impl IntoResponse {
    let new_code = state.regenerate_pairing_code().await;
    info!("[SERVER HOST] Kode pencocokan baru dibuat: {}", new_code);
    Json(json!({
        "status": "success",
        "pairing_code": new_code,
        "message": "Kode pencocokan berhasil diacak ulang."
    }))
}

async fn verify_pairing_code(
    State(state): State<AppState>,
    Json(payload): Json<VerifyPairingRequest>,
) -> impl IntoResponse {
    if !*state.is_running.read().await {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "valid": false,
                "message": "Host Server sedang dihentikan (Stopped). Tidak dapat menerima koneksi baru."
            })),
        );
    }
    let current_code = state.pairing_code.read().await.clone();
    if payload.pairing_code.trim() == current_code {
        (
            StatusCode::OK,
            Json(json!({
                "valid": true,
                "message": "Kode pencocokan valid. Izin koneksi diberikan."
            })),
        )
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "valid": false,
                "message": "Kode pencocokan salah! Perangkat tidak diizinkan terhubung ke server ini."
            })),
        )
    }
}

async fn list_devices(State(state): State<AppState>) -> impl IntoResponse {
    let devices = state.devices.read().await;
    let list: Vec<_> = devices.values().cloned().collect();
    Json(list)
}

async fn disconnect_device(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let success = state.kick_device(&id).await;
    if success {
        info!("[SERVER HOST] Terminal {} berhasil diputuskan secara paksa oleh Host", id);
        (
            StatusCode::OK,
            Json(json!({
                "status": "success",
                "message": format!("Terminal {} berhasil diputuskan dari jaringan", id)
            })),
        )
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({
                "status": "error",
                "message": format!("Terminal {} tidak ditemukan dalam daftar aktif", id)
            })),
        )
    }
}

async fn start_server(State(state): State<AppState>) -> impl IntoResponse {
    let mut running = state.is_running.write().await;
    if *running {
        return (
            StatusCode::OK,
            Json(json!({
                "status": "success",
                "message": "Host Server sudah dalam status berjalan (Active)",
                "is_running": true
            })),
        );
    }

    match crate::api::spawn_pos_server(state.clone()).await {
        Ok(handle) => {
            state.set_server_handle(handle).await;
            *running = true;
            let mut started = state.started_at.write().await;
            *started = Some(Utc::now());
            drop(running);
            drop(started);

            let _ = state.tx.send(json!({
                "type": "SERVER_STARTED",
                "timestamp": Utc::now()
            }).to_string());

            info!("[SERVER HOST] Status Server diubah ke: RUNNING / ACTIVE pada port {}", state.port);
            (
                StatusCode::OK,
                Json(json!({
                    "status": "success",
                    "message": "Host Server berhasil dijalankan (Active)",
                    "is_running": true
                })),
            )
        }
        Err(e) => {
            drop(running);
            tracing::error!("[SERVER HOST] Gagal mengikat socket port {}: {}", state.port, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "status": "error",
                    "message": format!("Gagal menyalakan server pada port {}: {}", state.port, e),
                    "is_running": false
                })),
            )
        }
    }
}

async fn stop_server(State(state): State<AppState>) -> impl IntoResponse {
    let mut running = state.is_running.write().await;
    *running = false;
    drop(running);

    // Kosongkan seluruh daftar perangkat aktif
    let mut devs = state.devices.write().await;
    devs.clear();
    drop(devs);

    let _ = state.tx.send(json!({
        "type": "SERVER_STOPPED",
        "message": "Host Server telah dihentikan oleh Administrator.",
        "timestamp": Utc::now()
    }).to_string());

    info!("[SERVER HOST] Status Server diubah ke: STOPPED / SUSPENDED");

    // Lakukan Hard Socket Shutdown pada port 8080 (abort handle & drop TcpListener)
    let state_clone = state.clone();
    tokio::spawn(async move {
        // Berikan jeda singkat 120ms agar response HTTP berhasil terkirim sebelum socket TCP ditutup total
        tokio::time::sleep(tokio::time::Duration::from_millis(120)).await;
        if state_clone.abort_server().await {
            info!("[SERVER HOST] Socket TCP port {} berhasil ditutup total (Connection Refused).", state_clone.port);
        }
    });

    Json(json!({
        "status": "success",
        "message": "Host Server berhasil dihentikan (Stopped) dan socket port ditutup total",
        "is_running": false
    }))
}
