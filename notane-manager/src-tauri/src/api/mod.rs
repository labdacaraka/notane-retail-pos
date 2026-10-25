use crate::crypto::ed25519::sign_message;
use crate::db::DbPool;
use crate::models::LicenseTokenPayload;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info};

pub mod admin;

use std::collections::HashSet;

#[derive(Clone)]
pub struct ApiState {
    pub pool: DbPool,
    pub master_private_key: Arc<RwLock<String>>,
    pub is_running: Arc<RwLock<bool>>,
    pub port: u16,
    pub server_handle: Arc<RwLock<Option<tokio::task::AbortHandle>>>,
    pub active_sessions: Arc<RwLock<HashSet<String>>>,
}

#[derive(Debug, Deserialize)]
pub struct ActivateRequest {
    pub license_key: String,
    pub hardware_id: String,
    pub device_name: Option<String>,
    pub app_version: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct VerifyRequest {
    pub license_key: String,
    pub hardware_id: String,
    pub app_version: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCheckRequest {
    pub current_version: String,
    pub component: Option<String>,      // "server" | "client" (defaults to "server")
    pub server_version: Option<String>, // sent by client to verify compatibility
    pub hardware_id: Option<String>,
    pub license_key: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct LicenseActivateRow {
    id: String,
    license_key: String,
    tier: String,
    max_terminals: i64,
    status: String,
    expires_at: String,
    grace_period_days: i64,
    features: String,
    client_name: String,
    company_name: String,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct DeviceCheckRow {
    id: String,
    is_active: i64,
}

#[derive(Debug, sqlx::FromRow)]
struct LicenseVerifyRow {
    id: String,
    status: String,
    expires_at: String,
    features: String,
}

#[derive(Debug, sqlx::FromRow)]
struct LatestReleaseRow {
    version: String,
    release_name: String,
    changelog: String,
    release_date: String,
    download_url: String,
    checksum_sha256: String,
    signature_ed25519: String,
    is_mandatory: i64,
    component: String,
    min_server_version: Option<String>,
}

pub fn create_api_router(state: ApiState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let admin_routes = admin::create_admin_router(state.clone());

    Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/system/status", get(system_status))
        .route("/api/v1/license/activate", post(handle_activate))
        .route("/api/v1/license/verify", post(handle_verify))
        .route("/api/v1/updates/check", post(handle_update_check))
        .nest("/api/v1/admin", admin_routes)
        .layer(cors)
        .with_state(state)
}

async fn health_check() -> impl IntoResponse {
    Json(json!({
        "status": "healthy",
        "service": "Notane License Provider Server",
        "timestamp": Utc::now().to_rfc3339()
    }))
}

async fn system_status(State(state): State<ApiState>) -> impl IntoResponse {
    let total_licenses: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM provider_licenses")
        .fetch_one(&state.pool)
        .await
        .unwrap_or((0,));

    let active_devices: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM provider_device_activations WHERE is_active = 1")
        .fetch_one(&state.pool)
        .await
        .unwrap_or((0,));

    Json(json!({
        "service": "Notane License Provider Server",
        "is_running": true,
        "port": state.port,
        "total_licenses": total_licenses.0,
        "active_devices": active_devices.0,
        "timestamp": Utc::now().to_rfc3339()
    }))
}

// ------------------------------------------------------------------------------
// POST /api/v1/license/activate
// ------------------------------------------------------------------------------
async fn handle_activate(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(payload): Json<ActivateRequest>,
) -> impl IntoResponse {
    let client_ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("127.0.0.1")
        .split(',')
        .next()
        .unwrap_or("127.0.0.1")
        .trim()
        .to_string();

    let clean_key = payload.license_key.trim();
    let clean_hwid = payload.hardware_id.trim();

    if clean_key.is_empty() || clean_hwid.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "message": "license_key dan hardware_id wajib diisi"
            })),
        );
    }

    // 1. Cari Lisensi di Database
    let license_row = sqlx::query_as::<_, LicenseActivateRow>(
        "SELECT l.id, l.license_key, l.tier, l.max_terminals, l.status, l.expires_at,
                l.grace_period_days, l.features, c.client_name, c.company_name
         FROM provider_licenses l
         JOIN provider_clients c ON l.client_id = c.id
         WHERE l.license_key = ?"
    )
    .bind(clean_key)
    .fetch_optional(&state.pool)
    .await;

    let license = match license_row {
        Ok(Some(l)) => l,
        Ok(None) => {
            log_audit(
                &state.pool,
                Some(clean_key),
                Some(clean_hwid),
                "ACTIVATE",
                Some(&client_ip),
                "REJECTED",
                "Lisensi tidak ditemukan di sistem provider",
            ).await;

            return (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "success": false,
                    "message": "Nomor lisensi tidak terdaftar atau salah input"
                })),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "success": false, "message": format!("Database error: {}", e) })),
            );
        }
    };

    // 2. Periksa Status Lisensi
    if license.status != "ACTIVE" {
        log_audit(
            &state.pool,
            Some(clean_key),
            Some(clean_hwid),
            "ACTIVATE",
            Some(&client_ip),
            "BLOCKED",
            &format!("Lisensi dalam status {}", license.status),
        ).await;

        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "success": false,
                "message": format!("Lisensi ini sedang dinonaktifkan ({})", license.status)
            })),
        );
    }

    // 3. Periksa Kedaluwarsa
    if let Ok(exp_time) = DateTime::parse_from_rfc3339(&license.expires_at) {
        if exp_time.with_timezone(&Utc) < Utc::now() {
            log_audit(
                &state.pool,
                Some(clean_key),
                Some(clean_hwid),
                "ACTIVATE",
                Some(&client_ip),
                "REJECTED",
                "Lisensi telah kedaluwarsa",
            ).await;

            return (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "success": false,
                    "message": "Masa aktif lisensi telah habis. Silakan perpanjang lisensi Anda."
                })),
            );
        }
    }

    // 4. Periksa Kuota Mesin (Node-Locking HWID)
    let existing_dev = sqlx::query_as::<_, DeviceCheckRow>(
        "SELECT id, is_active FROM provider_device_activations
         WHERE license_id = ? AND hardware_id = ?"
    )
    .bind(&license.id)
    .bind(clean_hwid)
    .fetch_optional(&state.pool)
    .await
    .unwrap_or(None);

    let now_str = Utc::now().to_rfc3339();

    if let Some(dev) = existing_dev {
        // Sudah terdaftar, re-aktifkan jika suspend & perbarui last_ping
        let _ = sqlx::query(
            "UPDATE provider_device_activations
             SET is_active = 1, last_ping_at = ?, ip_address = ?, device_name = COALESCE(?, device_name), app_version = ?
             WHERE id = ?"
        )
        .bind(&now_str)
        .bind(&client_ip)
        .bind(&payload.device_name)
        .bind(&payload.app_version)
        .bind(&dev.id)
        .execute(&state.pool)
        .await;
    } else {
        // Mesin baru: periksa kuota terminal
        let count_active: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM provider_device_activations WHERE license_id = ? AND is_active = 1"
        )
        .bind(&license.id)
        .fetch_one(&state.pool)
        .await
        .unwrap_or((0,));

        if count_active.0 >= license.max_terminals {
            log_audit(
                &state.pool,
                Some(clean_key),
                Some(clean_hwid),
                "ACTIVATE",
                Some(&client_ip),
                "REJECTED",
                &format!("Kuota terminal penuh ({}/{})", count_active.0, license.max_terminals),
            ).await;

            return (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "success": false,
                    "message": format!(
                        "Kuota aktivasi terminal untuk lisensi ini sudah penuh (Maksimal {} terminal). Hubungi Provider untuk menambah kuota kasir.",
                        license.max_terminals
                    )
                })),
            );
        }

        // Simpan aktivasi mesin baru
        let new_dev_id = uuid::Uuid::new_v4().to_string();
        let _ = sqlx::query(
            "INSERT INTO provider_device_activations (
                id, license_id, hardware_id, device_name, ip_address, os_info, app_version,
                activated_at, last_ping_at, is_active
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 1)"
        )
        .bind(new_dev_id)
        .bind(&license.id)
        .bind(clean_hwid)
        .bind(&payload.device_name)
        .bind(&client_ip)
        .bind("Desktop Client")
        .bind(&payload.app_version)
        .bind(&now_str)
        .bind(&now_str)
        .execute(&state.pool)
        .await;
    }

    // 5. Generate Signed License Token (Ed25519)
    let features: Vec<String> = serde_json::from_str(&license.features).unwrap_or_else(|_| {
        vec!["multi_outlet".to_string(), "offline_first".to_string(), "auto_update".to_string()]
    });

    let token_payload = LicenseTokenPayload {
        license_key: clean_key.to_string(),
        client_name: license.client_name,
        outlet_id: None,
        tier: license.tier,
        max_terminals: license.max_terminals,
        hardware_id: clean_hwid.to_string(),
        issued_at: now_str,
        expires_at: license.expires_at,
        offline_grace_days: license.grace_period_days,
        features,
    };

    let token_json = match serde_json::to_string(&token_payload) {
        Ok(j) => j,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "success": false, "message": format!("Gagal serialize token: {}", e) })),
            );
        }
    };

    let priv_key = state.master_private_key.read().await;
    let signature = match sign_message(&priv_key, &token_json) {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "success": false, "message": format!("Gagal menandatangani lisensi: {}", e) })),
            );
        }
    };

    log_audit(
        &state.pool,
        Some(clean_key),
        Some(clean_hwid),
        "ACTIVATE",
        Some(&client_ip),
        "SUCCESS",
        "Aktivasi lisensi dan HWID berhasil",
    ).await;

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "status": "success",
            "message": "Lisensi resmi berhasil diaktivasi!",
            "data": {
                "license_token": token_json,
                "signature": signature
            }
        })),
    )
}

// ------------------------------------------------------------------------------
// POST /api/v1/license/verify
// ------------------------------------------------------------------------------
async fn handle_verify(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(payload): Json<VerifyRequest>,
) -> impl IntoResponse {
    let client_ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("127.0.0.1")
        .split(',')
        .next()
        .unwrap_or("127.0.0.1")
        .trim()
        .to_string();

    let clean_key = payload.license_key.trim();
    let clean_hwid = payload.hardware_id.trim();

    let row = sqlx::query_as::<_, LicenseVerifyRow>(
        "SELECT l.id, l.status, l.expires_at, l.features
         FROM provider_licenses l
         WHERE l.license_key = ?"
    )
    .bind(clean_key)
    .fetch_optional(&state.pool)
    .await;

    let license = match row {
        Ok(Some(l)) => l,
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "success": false, "status": "UNLICENSED", "message": "Lisensi tidak ditemukan" })),
            );
        }
    };

    // Perbarui ping perangkat
    let now_str = Utc::now().to_rfc3339();
    let _ = sqlx::query(
        "UPDATE provider_device_activations
         SET last_ping_at = ?, ip_address = ?, app_version = COALESCE(?, app_version)
         WHERE license_id = ? AND hardware_id = ?"
    )
    .bind(&now_str)
    .bind(&client_ip)
    .bind(&payload.app_version)
    .bind(&license.id)
    .bind(clean_hwid)
    .execute(&state.pool)
    .await;

    let days_remaining = if let Ok(exp) = DateTime::parse_from_rfc3339(&license.expires_at) {
        let diff = exp.with_timezone(&Utc) - Utc::now();
        diff.num_days().max(0)
    } else {
        0
    };

    let features: Vec<String> = serde_json::from_str(&license.features).unwrap_or_default();

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "status": license.status,
            "expires_at": license.expires_at,
            "days_remaining": days_remaining,
            "features": features
        })),
    )
}

fn parse_version_tuple(v: &str) -> (u32, u32, u32) {
    let clean = v.trim().trim_start_matches('v').trim_start_matches('V');
    let parts: Vec<&str> = clean.split('.').collect();
    let major = parts.get(0).and_then(|s| s.split('-').next()).and_then(|s| s.parse().ok()).unwrap_or(0);
    let minor = parts.get(1).and_then(|s| s.split('-').next()).and_then(|s| s.parse().ok()).unwrap_or(0);
    let patch = parts.get(2).and_then(|s| s.split('-').next()).and_then(|s| s.parse().ok()).unwrap_or(0);
    (major, minor, patch)
}

fn is_version_newer(latest: &str, current: &str) -> bool {
    parse_version_tuple(latest) > parse_version_tuple(current)
}

fn is_server_compatible(server_version: &str, min_server_version: &str) -> bool {
    parse_version_tuple(server_version) >= parse_version_tuple(min_server_version)
}

// ------------------------------------------------------------------------------
// POST /api/v1/updates/check
// ------------------------------------------------------------------------------
async fn handle_update_check(
    State(state): State<ApiState>,
    Json(payload): Json<UpdateCheckRequest>,
) -> impl IntoResponse {
    let req_component = payload.component.as_deref().unwrap_or("server").to_lowercase();

    let latest_row = sqlx::query_as::<_, LatestReleaseRow>(
        "SELECT version, release_name, changelog, release_date, download_url,
                checksum_sha256, signature_ed25519, is_mandatory, component, min_server_version
         FROM provider_app_releases
         WHERE is_published = 1 AND component = ?
         ORDER BY created_at DESC LIMIT 1"
    )
    .bind(&req_component)
    .fetch_optional(&state.pool)
    .await;

    let latest = match latest_row {
        Ok(Some(r)) => r,
        _ => {
            return Json(json!({
                "has_update": false,
                "latest_version": payload.current_version,
                "component": req_component,
                "can_update": false,
                "server_compatible": true,
                "message": format!("Belum ada rilis update tersedia untuk komponen {}", req_component)
            }));
        }
    };

    let has_update = is_version_newer(&latest.version, &payload.current_version);

    if !has_update {
        return Json(json!({
            "has_update": false,
            "latest_version": payload.current_version,
            "component": req_component,
            "can_update": false,
            "server_compatible": true,
            "message": format!("Komponen {} Anda sudah dalam versi terbaru ({})", req_component, payload.current_version)
        }));
    }

    // Periksa kompatibilitas server jika komponen adalah client
    let mut server_compatible = true;
    let mut can_update = true;
    let mut update_message = format!("Pembaruan versi resmi {} ({}) siap diunduh.", req_component, latest.version);

    if req_component == "client" {
        if let Some(ref min_srv) = latest.min_server_version {
            let srv_ver = payload.server_version.as_deref().unwrap_or("0.0.0");
            if !is_server_compatible(srv_ver, min_srv) {
                server_compatible = false;
                can_update = false;
                update_message = format!(
                    "Pembaruan Notane Client ({}) memerlukan Notane Server minimal versi {}. Server saat ini terdeteksi versi {}. Harap perbarui Notane Server terlebih dahulu sebelum memperbarui UI Klien.",
                    latest.version, min_srv, srv_ver
                );
            }
        }
    }

    Json(json!({
        "has_update": true,
        "latest_version": latest.version,
        "release_name": latest.release_name,
        "changelog": latest.changelog,
        "release_date": latest.release_date,
        "download_url": latest.download_url,
        "checksum_sha256": latest.checksum_sha256,
        "signature": latest.signature_ed25519,
        "is_mandatory": latest.is_mandatory == 1,
        "component": req_component,
        "min_server_version": latest.min_server_version,
        "server_compatible": server_compatible,
        "can_update": can_update,
        "message": update_message
    }))
}

async fn log_audit(
    pool: &DbPool,
    license_key: Option<&str>,
    hardware_id: Option<&str>,
    action: &str,
    ip_address: Option<&str>,
    status: &str,
    details: &str,
) {
    let id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let _ = sqlx::query(
        "INSERT INTO provider_license_audit_logs (
            id, license_key, hardware_id, action, ip_address, status, details, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(id)
    .bind(license_key)
    .bind(hardware_id)
    .bind(action)
    .bind(ip_address)
    .bind(status)
    .bind(details)
    .bind(now)
    .execute(pool)
    .await;
}

/// Spawns the embedded Axum HTTP server on background tokio task
pub async fn spawn_api_server(state: ApiState) -> Result<tokio::task::AbortHandle, Box<dyn std::error::Error + Send + Sync>> {
    let port = state.port;
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let app = create_api_router(state.clone());

    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("🚀 Notane License & OTA Provider Server aktif di http://0.0.0.0:{}", port);

    let join_handle = tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await {
            error!("Error pada Provider Server: {}", e);
        }
    });

    let abort_handle = join_handle.abort_handle();
    Ok(abort_handle)
}
