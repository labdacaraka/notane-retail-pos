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

impl ApiState {
    pub async fn get_master_private_key(&self) -> String {
        let guard = self.master_private_key.read().await;
        if guard.trim().len() == 64 {
            return guard.trim().to_string();
        }
        let db_key: Option<(String,)> = sqlx::query_as("SELECT value FROM provider_settings WHERE key = 'master_private_key'")
            .fetch_optional(&self.pool)
            .await
            .unwrap_or(None);

        db_key
            .map(|(k,)| k.trim().to_string())
            .filter(|k| k.len() == 64)
            .unwrap_or_else(|| crate::crypto::ed25519::DEFAULT_MASTER_PRIVATE_KEY_HEX.to_string())
    }
}

#[derive(Debug, Deserialize)]
pub struct ActivateRequest {
    #[serde(alias = "licenseKey")]
    pub license_key: String,
    #[serde(alias = "hardwareId", alias = "hwid")]
    pub hardware_id: String,
    #[serde(alias = "deviceName")]
    pub device_name: Option<String>,
    #[serde(alias = "appVersion")]
    pub app_version: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct VerifyRequest {
    #[serde(alias = "licenseKey")]
    pub license_key: String,
    #[serde(alias = "hardwareId", alias = "hwid")]
    pub hardware_id: String,
    #[serde(alias = "appVersion")]
    pub app_version: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCheckRequest {
    #[serde(alias = "currentVersion")]
    pub current_version: String,
    pub component: Option<String>,      // "server" | "client" (defaults to "server")
    #[serde(alias = "serverVersion")]
    pub server_version: Option<String>, // sent by client to verify compatibility
    #[serde(alias = "hardwareId", alias = "hwid")]
    pub hardware_id: Option<String>,
    #[serde(alias = "licenseKey")]
    pub license_key: Option<String>,
    #[serde(default)]
    pub os: Option<String>,
    #[serde(default)]
    pub arch: Option<String>,
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
    is_claimed: Option<i64>,
    claimed_hardware_id: Option<String>,
    claimed_device_name: Option<String>,
    claimed_at: Option<String>,
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
    is_claimed: Option<i64>,
    claimed_hardware_id: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct LatestReleaseRow {
    id: String,
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

    let base_router = Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/system/status", get(system_status))
        .route("/api/v1/license/activate", post(handle_activate))
        .route("/api/v1/license/verify", post(handle_verify))
        .route("/api/v1/updates/check", post(handle_update_check))
        .route("/api/v1/updates/download/{id}", get(handle_update_download))
        .nest("/api/v1/admin", admin_routes);

    let dist_dir = std::env::var("WEB_DIST").unwrap_or_else(|_| "dist".to_string());
    let router = if std::path::Path::new(&dist_dir).exists() {
        let serve_dir = tower_http::services::ServeDir::new(&dist_dir)
            .fallback(tower_http::services::ServeFile::new(format!("{}/index.html", dist_dir)));
        base_router.fallback_service(serve_dir)
    } else {
        base_router
    };

    router.layer(cors).with_state(state)
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
                l.grace_period_days, l.features, c.client_name, c.company_name,
                l.is_claimed, l.claimed_hardware_id, l.claimed_device_name, l.claimed_at
         FROM provider_licenses l
         JOIN provider_clients c ON l.client_id = c.id
         WHERE l.license_key = $1"
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

    // 4. Periksa Status Klaim Mesin (Mesin Baru vs Mesin yang Sudah Diklaim)
    if license.is_claimed.unwrap_or(0) == 1 {
        if let Some(claimed_hw) = &license.claimed_hardware_id {
            if !claimed_hw.trim().is_empty() && claimed_hw.trim() != clean_hwid {
                let dev_name = license.claimed_device_name.as_deref().unwrap_or("komputer sebelumnya");
                log_audit(
                    &state.pool,
                    Some(clean_key),
                    Some(clean_hwid),
                    "ACTIVATE",
                    Some(&client_ip),
                    "BLOCKED",
                    &format!("Lisensi telah diklaim oleh mesin lain: {} (HWID: {})", dev_name, claimed_hw),
                ).await;

                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "success": false,
                        "code": "LICENSE_ALREADY_CLAIMED",
                        "message": format!(
                            "Lisensi ini sudah diklaim pada perangkat lain ({}). Jika komputer lama Anda rusak atau diganti, silakan hubungi Administrator Notane Provider untuk melepaskan klaim mesin lama terlebih dahulu.",
                            dev_name
                        )
                    })),
                );
            }
        }
    }

    // 5. Periksa Node-Locking Server HWID (1 Lisensi = 1 Notane Server)
    let bound_active_server = sqlx::query_as::<_, DeviceCheckRow>(
        "SELECT id, is_active FROM provider_device_activations
         WHERE license_id = $1 AND is_active = 1"
    )
    .bind(&license.id)
    .fetch_optional(&state.pool)
    .await
    .unwrap_or(None);

    let existing_dev = sqlx::query_as::<_, DeviceCheckRow>(
        "SELECT id, is_active FROM provider_device_activations
         WHERE license_id = $1 AND hardware_id = $2"
    )
    .bind(&license.id)
    .bind(clean_hwid)
    .fetch_optional(&state.pool)
    .await
    .unwrap_or(None);

    let now_str = Utc::now().to_rfc3339();
    let effective_device_name = payload.device_name.as_deref().unwrap_or("Notane Server");

    if let Some(dev) = existing_dev {
        // Server ini sudah terdaftar: perbarui ping & status aktif
        let _ = sqlx::query(
            "UPDATE provider_device_activations
             SET is_active = 1, last_ping_at = $1, ip_address = $2, device_name = COALESCE($3, device_name), app_version = $4
             WHERE id = $5"
        )
        .bind(&now_str)
        .bind(&client_ip)
        .bind(&payload.device_name)
        .bind(&payload.app_version)
        .bind(&dev.id)
        .execute(&state.pool)
        .await;

        // Pastikan status klaim sinkron di tabel provider_licenses
        let _ = sqlx::query(
            "UPDATE provider_licenses
             SET is_claimed = 1, claimed_hardware_id = $1, claimed_device_name = COALESCE($2, claimed_device_name), claimed_at = COALESCE(claimed_at, $3), updated_at = $4
             WHERE id = $5"
        )
        .bind(clean_hwid)
        .bind(&payload.device_name)
        .bind(&now_str)
        .bind(&now_str)
        .bind(&license.id)
        .execute(&state.pool)
        .await;
    } else if bound_active_server.is_some() {
        // Lisensi ini sudah terikat aktif ke mesin server lain (Node-Locking)
        log_audit(
            &state.pool,
            Some(clean_key),
            Some(clean_hwid),
            "ACTIVATE",
            Some(&client_ip),
            "REJECTED",
            "Lisensi sudah terikat pada mesin server lain (Node-Locking)",
        ).await;

        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "success": false,
                "code": "LICENSE_ALREADY_CLAIMED",
                "message": "Lisensi ini telah terikat secara permanen ke mesin Notane Server lain (Node-Locked). Setiap lisensi hanya dapat diaktivasi pada 1 mesin server. Hubungi Provider untuk melepaskan ikatan server lama."
            })),
        );
    } else {
        // Mesin server baru: catat aktivasi host node
        let new_dev_id = uuid::Uuid::new_v4().to_string();
        let _ = sqlx::query(
            "INSERT INTO provider_device_activations (
                id, license_id, hardware_id, device_name, ip_address, os_info, app_version,
                activated_at, last_ping_at, is_active
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 1)"
        )
        .bind(new_dev_id)
        .bind(&license.id)
        .bind(clean_hwid)
        .bind(effective_device_name)
        .bind(&client_ip)
        .bind("Desktop Client")
        .bind(&payload.app_version)
        .bind(&now_str)
        .bind(&now_str)
        .execute(&state.pool)
        .await;

        // Tandai lisensi sebagai TERKLAIM pada mesin ini
        let _ = sqlx::query(
            "UPDATE provider_licenses
             SET is_claimed = 1, claimed_hardware_id = $1, claimed_device_name = $2, claimed_at = COALESCE(claimed_at, $3), updated_at = $4
             WHERE id = $5"
        )
        .bind(clean_hwid)
        .bind(effective_device_name)
        .bind(&now_str)
        .bind(&now_str)
        .bind(&license.id)
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

    let priv_key = state.get_master_private_key().await;
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
        "SELECT l.id, l.status, l.expires_at, l.features, l.is_claimed, l.claimed_hardware_id
         FROM provider_licenses l
         WHERE l.license_key = $1"
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

    // Jika lisensi telah diklaim oleh HWID lain, tolak verifikasi
    if license.is_claimed.unwrap_or(0) == 1 {
        if let Some(claimed_hw) = &license.claimed_hardware_id {
            if !claimed_hw.trim().is_empty() && claimed_hw.trim() != clean_hwid {
                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "success": false,
                        "status": "REVOKED",
                        "message": "Lisensi telah dipindahkan atau diklaim oleh mesin lain"
                    })),
                );
            }
        }
    }

    // Perbarui ping perangkat
    let now_str = Utc::now().to_rfc3339();
    let _ = sqlx::query(
        "UPDATE provider_device_activations
         SET last_ping_at = $1, ip_address = $2, app_version = COALESCE($3, app_version)
         WHERE license_id = $4 AND hardware_id = $5"
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
        "SELECT id, version, release_name, changelog, release_date, download_url,
                checksum_sha256, signature_ed25519, is_mandatory, component, min_server_version
         FROM provider_app_releases
         WHERE is_published = 1 AND component = $1
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

    // Periksa apakah ada aset spesifik platform OS dan Arsitektur yang cocok
    let mut final_download_url = latest.download_url.clone();
    let mut final_checksum = latest.checksum_sha256.clone();
    let mut final_signature = latest.signature_ed25519.clone();
    let mut matched_platform: Option<String> = None;

    if req_component == "server" {
        let assets_query = crate::db::sql(
            "SELECT id, release_id, platform, os, arch, download_url, checksum_sha256, signature_ed25519, file_path, file_size, original_filename, created_at
             FROM provider_release_assets WHERE release_id = ?"
        );
        let assets_res = sqlx::query_as::<_, crate::models::ReleaseAsset>(&assets_query)
            .bind(&latest.id)
            .fetch_all(&state.pool)
            .await;

        if let Ok(assets) = assets_res {
            if !assets.is_empty() {
                let req_os = payload.os.as_deref().unwrap_or("").trim().to_lowercase();
                let req_arch = payload.arch.as_deref().unwrap_or("").trim().to_lowercase();

                let matched_asset = assets.iter().find(|a| {
                    let a_os = a.os.trim().to_lowercase();
                    let a_arch = a.arch.trim().to_lowercase();

                    let os_match = match (a_os.as_str(), req_os.as_str()) {
                        (x, y) if x == y => true,
                        ("macos" | "darwin", "macos" | "darwin") => true,
                        ("all" | "universal", _) => true,
                        _ => false,
                    };

                    let arch_match = match (a_arch.as_str(), req_arch.as_str()) {
                        (x, y) if x == y => true,
                        ("x86_64" | "x64" | "amd64", "x86_64" | "x64" | "amd64") => true,
                        ("x86" | "i686" | "i386", "x86" | "i686" | "i386") => true,
                        ("aarch64" | "arm64", "aarch64" | "arm64") => true,
                        ("all" | "universal", _) => true,
                        _ => false,
                    };

                    os_match && arch_match
                });

                if let Some(asset) = matched_asset {
                    final_download_url = asset.download_url.clone();
                    final_checksum = asset.checksum_sha256.clone();
                    final_signature = asset.signature_ed25519.clone();
                    matched_platform = Some(asset.platform.clone());
                }
            }
        }
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
        "download_url": final_download_url,
        "checksum_sha256": final_checksum,
        "signature": final_signature,
        "is_mandatory": latest.is_mandatory == 1,
        "component": req_component,
        "matched_platform": matched_platform,
        "min_server_version": latest.min_server_version,
        "server_compatible": server_compatible,
        "can_update": can_update,
        "message": update_message
    }))
}

#[derive(Debug, Deserialize)]
pub struct DownloadQuery {
    pub token: Option<String>,
}

async fn handle_update_download(
    State(state): State<ApiState>,
    axum::extract::Path(release_id_or_version): axum::extract::Path<String>,
    headers: HeaderMap,
    axum::extract::Query(query): axum::extract::Query<DownloadQuery>,
) -> impl IntoResponse {
    let token = query.token
        .or_else(|| headers.get("X-Notane-Token").and_then(|v| v.to_str().ok().map(|s| s.to_string())))
        .or_else(|| headers.get("Authorization").and_then(|v| v.to_str().ok().and_then(|s| s.strip_prefix("Bearer ")).map(|s| s.to_string())));

    let token_str = match token {
        Some(t) if !t.trim().is_empty() => t.trim().to_string(),
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "status": "error",
                    "code": "MISSING_ENCRYPTED_TOKEN",
                    "message": "Akses pembaruan ditolak: Token enkripsi nomor mesin dan nomor lisensi wajib disertakan."
                })),
            ).into_response();
        }
    };

    // 1. Dekripsi parameter token menggunakan kunci master
    let master_pub_key = crate::crypto::ed25519::DEFAULT_MASTER_PUBLIC_KEY_HEX;
    let payload = match crate::crypto::ota::decrypt_ota_payload(&token_str, master_pub_key) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "status": "error",
                    "code": "INVALID_ENCRYPTION_TOKEN",
                    "message": format!("Akses ditolak: Dekripsi token otorisasi gagal ({})", e)
                })),
            ).into_response();
        }
    };

    // 2. Proteksi Anti-Replay: timestamp harus dalam rentang 10 menit
    let now_ts = Utc::now().timestamp();
    if (now_ts - payload.timestamp).abs() > 600 {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "status": "error",
                "code": "EXPIRED_TOKEN_TIMESTAMP",
                "message": "Akses ditolak: Token otorisasi telah kedaluwarsa (> 10 menit). Silakan sinkronkan jam sistem Anda."
            })),
        ).into_response();
    }

    // 3. Validasi Lisensi di Database Notane Manager
    let lic_query = crate::db::sql("SELECT id, status, claimed_hardware_id, expires_at FROM provider_licenses WHERE license_key = ?");
    let lic_row = sqlx::query_as::<_, (String, String, Option<String>, Option<String>)>(&lic_query)
        .bind(payload.license_key.trim())
        .fetch_optional(&state.pool)
        .await;

    let (lic_id, lic_status, claimed_hwid, expires_at) = match lic_row {
        Ok(Some(r)) => r,
        _ => {
            log_audit(
                &state.pool,
                Some(&payload.license_key),
                Some(&payload.machine_id),
                "DOWNLOAD_UPDATE",
                None,
                "REJECTED",
                "Nomor lisensi tidak terdaftar di database provider",
            ).await;
            return (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "status": "error",
                    "code": "LICENSE_NOT_FOUND",
                    "message": "Nomor lisensi tidak terdaftar pada Server Provider Notane."
                })),
            ).into_response();
        }
    };

    if lic_status == "SUSPENDED" || lic_status == "REVOKED" {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "status": "error",
                "code": "LICENSE_INACTIVE",
                "message": format!("Lisensi Anda tidak aktif (Status: {}). Hubungi penyedia lisensi.", lic_status)
            })),
        ).into_response();
    }

    // Periksa masa berlaku jika ada
    if let Some(ref exp_str) = expires_at {
        if let Ok(exp_date) = chrono::DateTime::parse_from_rfc3339(exp_str) {
            if Utc::now() > exp_date.with_timezone(&Utc) {
                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "status": "error",
                        "code": "LICENSE_EXPIRED",
                        "message": "Lisensi telah kedaluwarsa. Perpanjang lisensi Anda untuk mendapatkan paket pembaruan."
                    })),
                ).into_response();
            }
        }
    }

    // 4. Validasi Hardware / Machine ID
    let mut machine_matched = false;
    if let Some(ref chw) = claimed_hwid {
        if chw.trim().eq_ignore_ascii_case(payload.machine_id.trim()) {
            machine_matched = true;
        }
    }

    if !machine_matched {
        let dev_query = crate::db::sql("SELECT COUNT(*) FROM provider_device_activations WHERE license_id = ? AND hardware_id = ? AND is_active = 1");
        let active_dev_count: Result<(i64,), _> = sqlx::query_as(&dev_query)
            .bind(&lic_id)
            .bind(payload.machine_id.trim())
            .fetch_one(&state.pool)
            .await;

        if let Ok((cnt,)) = active_dev_count {
            if cnt > 0 {
                machine_matched = true;
            }
        }
    }

    if !machine_matched {
        log_audit(
            &state.pool,
            Some(&payload.license_key),
            Some(&payload.machine_id),
            "DOWNLOAD_UPDATE",
            None,
            "REJECTED",
            "Nomor mesin tidak cocok dengan lisensi terdaftar",
        ).await;

        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "status": "error",
                "code": "HARDWARE_ID_MISMATCH",
                "message": "Akses ditolak: Nomor mesin (Hardware ID) ini tidak cocok dengan lisensi terdaftar."
            })),
        ).into_response();
    }

    // 5. Cari rilis aplikasi atau aset spesifik yang diminta
    let asset_query = crate::db::sql(
        "SELECT id, release_id, platform, os, arch, download_url, file_path, file_size, original_filename
         FROM provider_release_assets
         WHERE id = ?"
    );
    let asset_row = sqlx::query_as::<_, (String, String, String, String, String, String, Option<String>, Option<i64>, Option<String>)>(&asset_query)
        .bind(&release_id_or_version)
        .fetch_optional(&state.pool)
        .await;

    let (rel_ver, download_url, file_path, file_size, orig_name) = if let Ok(Some((_a_id, rel_id, _plat, _os, _arch, d_url, a_file_path, a_file_size, a_orig_name))) = asset_row {
        // Ambil versi dari release induk
        let ver_q = crate::db::sql("SELECT version FROM provider_app_releases WHERE id = ?");
        let parent_ver: Option<(String,)> = sqlx::query_as(&ver_q).bind(&rel_id).fetch_optional(&state.pool).await.unwrap_or(None);
        let v = parent_ver.map(|p| p.0).unwrap_or_else(|| "latest".to_string());
        (v, d_url, a_file_path, a_file_size, a_orig_name)
    } else {
        // Cari di tabel utama provider_app_releases
        let rel_query = crate::db::sql(
            "SELECT id, version, download_url, file_path, file_size, original_filename
             FROM provider_app_releases
             WHERE id = ? OR version = ?"
        );
        let release_row = sqlx::query_as::<_, (String, String, String, Option<String>, Option<i64>, Option<String>)>(&rel_query)
            .bind(&release_id_or_version)
            .bind(&release_id_or_version)
            .fetch_optional(&state.pool)
            .await;

        let (rel_id, rel_ver, download_url, mut f_path, mut f_size, mut f_orig_name) = match release_row {
            Ok(Some(r)) => r,
            _ => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(json!({
                        "status": "error",
                        "code": "RELEASE_NOT_FOUND",
                        "message": "Berkas rilis yang diminta tidak ditemukan."
                    })),
                ).into_response();
            }
        };

        // Jika request membawa os & arch terenkripsi di token, cari aset platform yang cocok
        if let (Some(ref req_os), Some(ref req_arch)) = (&payload.os, &payload.arch) {
            let assets_q = crate::db::sql(
                "SELECT id, platform, os, arch, file_path, file_size, original_filename
                 FROM provider_release_assets WHERE release_id = ?"
            );
            let assets_list = sqlx::query_as::<_, (String, String, String, String, Option<String>, Option<i64>, Option<String>)>(&assets_q)
                .bind(&rel_id)
                .fetch_all(&state.pool)
                .await;

            if let Ok(assets) = assets_list {
                let req_os_clean = req_os.trim().to_lowercase();
                let req_arch_clean = req_arch.trim().to_lowercase();
                let matched = assets.into_iter().find(|(_, _, a_os, a_arch, _, _, _)| {
                    let os_match = match (a_os.trim().to_lowercase().as_str(), req_os_clean.as_str()) {
                        (x, y) if x == y => true,
                        ("macos" | "darwin", "macos" | "darwin") => true,
                        ("all" | "universal", _) => true,
                        _ => false,
                    };
                    let arch_match = match (a_arch.trim().to_lowercase().as_str(), req_arch_clean.as_str()) {
                        (x, y) if x == y => true,
                        ("x86_64" | "x64" | "amd64", "x86_64" | "x64" | "amd64") => true,
                        ("x86" | "i686" | "i386", "x86" | "i686" | "i386") => true,
                        ("aarch64" | "arm64", "aarch64" | "arm64") => true,
                        ("all" | "universal", _) => true,
                        _ => false,
                    };
                    os_match && arch_match
                });

                if let Some((_, _, _, _, a_fp, a_fs, a_fn)) = matched {
                    if a_fp.is_some() {
                        f_path = a_fp;
                        f_size = a_fs;
                        f_orig_name = a_fn;
                    }
                }
            }
        }

        (rel_ver, download_url, f_path, f_size, f_orig_name)
    };

    // 6. Alirkan berkas dari penyimpanan lokal jika tersedia
    if let Some(ref fp) = file_path {
        let p = std::path::Path::new(fp);
        if p.exists() && p.is_file() {
            match tokio::fs::File::open(p).await {
                Ok(file) => {
                    log_audit(
                        &state.pool,
                        Some(&payload.license_key),
                        Some(&payload.machine_id),
                        "DOWNLOAD_UPDATE",
                        None,
                        "SUCCESS",
                        &format!("Berhasil mengunduh rilis v{} ({})", rel_ver, orig_name.as_deref().unwrap_or("file")),
                    ).await;

                    let filename = orig_name.unwrap_or_else(|| format!("notane-release-{}.zip", rel_ver));
                    let stream = tokio_util::io::ReaderStream::new(file);
                    let body = axum::body::Body::from_stream(stream);

                    let mut resp = body.into_response();
                    let headers = resp.headers_mut();
                    headers.insert(
                        axum::http::header::CONTENT_TYPE,
                        axum::http::HeaderValue::from_static("application/octet-stream"),
                    );
                    let disposition = format!("attachment; filename=\"{}\"", filename);
                    if let Ok(val) = axum::http::HeaderValue::from_str(&disposition) {
                        headers.insert(axum::http::header::CONTENT_DISPOSITION, val);
                    }
                    if let Some(sz) = file_size {
                        headers.insert(
                            axum::http::header::CONTENT_LENGTH,
                            axum::http::HeaderValue::from(sz),
                        );
                    }
                    return resp;
                }
                Err(e) => {
                    tracing::error!("Gagal membuka file rilis di {}: {}", fp, e);
                }
            }
        }
    }

    // 7. Jika rilis menggunakan tautan eksternal (fallback)
    if download_url.starts_with("http://") || download_url.starts_with("https://") {
        log_audit(
            &state.pool,
            Some(&payload.license_key),
            Some(&payload.machine_id),
            "DOWNLOAD_UPDATE",
            None,
            "REDIRECT",
            &format!("Redirect ke tautan rilis v{}", rel_ver),
        ).await;
        return axum::response::Redirect::temporary(&download_url).into_response();
    }

    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "status": "error",
            "message": "File fisik pembaruan belum diunggah atau tidak ditemukan di server provider."
        })),
    ).into_response()
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
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
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
