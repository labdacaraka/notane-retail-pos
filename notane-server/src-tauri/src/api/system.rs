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
use tracing::{error, info};

use super::ws::AppState;

#[derive(Debug, Deserialize)]
pub struct VerifyPairingRequest {
    pub pairing_code: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePairingRequest {
    pub pairing_code: String,
}

pub fn system_routes(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/status", get(get_server_status))
        .route("/version", get(get_version_info))
        .route("/license", get(get_license_info))
        .route("/license/activate", post(activate_license))
        .route("/license/activate-online", post(activate_license_online))
        .route("/provider-config", get(get_provider_config).post(save_provider_config))
        .route("/provider-config/test", post(test_provider_connection))
        .route("/updates/check", post(check_updates))
        .route("/updates/apply", post(apply_update))
        .route("/devices", get(list_devices))
        .route("/devices/{id}/disconnect", post(disconnect_device))
        .route("/server/start", post(start_server))
        .route("/server/stop", post(stop_server))
        .route("/server/restart", post(restart_server))
        .route("/pairing-code", get(get_pairing_code))
        .route("/pairing-code/regenerate", post(regenerate_pairing_code))
        .route("/pairing-code/update", post(update_pairing_code))
        .route("/pairing-code/verify", post(verify_pairing_code))
        .route("/branding", get(get_store_branding).post(save_store_branding))
        .route("/branding/upload-logo", post(upload_store_logo))
        .route("/setup-status", get(get_setup_status))
        .route("/initial-setup", post(perform_initial_setup))
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

async fn update_pairing_code(
    State(state): State<AppState>,
    Json(payload): Json<UpdatePairingRequest>,
) -> impl IntoResponse {
    let code = payload.pairing_code.trim().to_uppercase();
    if code.len() < 4 || code.len() > 32 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "status": "error",
                "message": "Kode pairing minimal 4 karakter dan maksimal 32 karakter."
            })),
        );
    }
    let updated = state.set_pairing_code(code).await;
    info!("[SERVER HOST] Kode pairing berhasil diperbarui secara manual: {}", updated);
    (
        StatusCode::OK,
        Json(json!({
            "status": "success",
            "pairing_code": updated,
            "message": "Kode pairing toko berhasil diperbarui."
        })),
    )
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
    let list: Vec<_> = devices
        .values()
        .filter(|d| d.device_type != "HOST" && d.device_id != "HOST-SERVER")
        .cloned()
        .collect();
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

    // 1. Pastikan PostgreSQL siap dan pool terhubung
    let mut pool_guard = state.pool.write().await;
    if pool_guard.is_none() {
        let db_url = state.db_url.read().await.clone();
        let pg_port = state.pg_port;

        if db_url.contains("127.0.0.1") || db_url.contains("localhost") {
            if !crate::db::embedded::EmbeddedPgManager::is_port_active("127.0.0.1", pg_port).await {
                info!("Memulai engine database PostgreSQL pada port {}...", pg_port);
                let embedded_manager = crate::db::embedded::EmbeddedPgManager::new();
                if let Err(e) = embedded_manager.start_if_needed(pg_port).await {
                    error!("Gagal menyalakan PostgreSQL: {}", e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({
                            "status": "error",
                            "message": format!("Gagal menyalakan database PostgreSQL: {}", e),
                            "is_running": false
                        })),
                    );
                }
            }
        }

        match crate::db::create_pool(&db_url).await {
            Ok(p) => {
                info!("Koneksi database PostgreSQL terisolasi berhasil.");
                if let Err(e) = crate::db::run_embedded_migrations(&p).await {
                    error!("Peringatan migrasi skema: {}", e);
                }
                *pool_guard = Some(p);
            }
            Err(e) => {
                error!("Gagal menghubungkan ke database: {}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "status": "error",
                        "message": format!("Gagal menghubungkan ke database: {}", e),
                        "is_running": false
                    })),
                );
            }
        }
    }
    drop(pool_guard);

    // 2. Pemeriksaan Lisensi: Wajib valid sebelum server POS boleh dijalankan
    let pool_opt = state.pool.read().await.clone();
    if let Some(ref pool) = pool_opt {
        let license_info = crate::services::license_service::LicenseService::check_current_license(pool).await;
        if !license_info.is_valid {
            drop(running);
            tracing::warn!("[SERVER HOST] Percobaan menyalakan server ditolak karena lisensi {:?}: {}", license_info.status, license_info.message);
            return (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "status": "error",
                    "error_code": "LICENSE_INVALID",
                    "license_status": license_info.status,
                    "message": format!("Server tidak dapat dijalankan: {}", license_info.message),
                    "is_running": false
                })),
            );
        }
    } else {
        drop(running);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "status": "error",
                "message": "Koneksi database belum tersedia.",
                "is_running": false
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

async fn restart_server(State(state): State<AppState>) -> impl IntoResponse {
    info!("[SERVER HOST] Permintaan Restart Server diterima...");

    // 1. Hentikan server POS lama jika sedang berjalan
    let mut running = state.is_running.write().await;
    *running = false;
    drop(running);

    let mut devs = state.devices.write().await;
    devs.clear();
    drop(devs);

    let _ = state.tx.send(json!({
        "type": "SERVER_RESTARTED",
        "message": "Host Server sedang dimuat ulang...",
        "timestamp": Utc::now()
    }).to_string());

    // Abort handle listener POS lama
    state.abort_server().await;

    // Jeda sejenak agar socket OS terbebas
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    // 2. Jalankan migrasi database otomatis jika pool aktif
    let pool_opt = state.pool.read().await.clone();
    if let Some(pool) = pool_opt.as_ref() {
        if let Err(e) = crate::db::run_embedded_migrations(pool).await {
            tracing::error!("[SERVER RESTART] Peringatan migrasi skema saat restart: {}", e);
        }
    }

    // 2b. Cek lisensi sebelum membangkitkan kembali server POS
    if let Some(ref pool) = pool_opt {
        let license_info = crate::services::license_service::LicenseService::check_current_license(pool).await;
        if !license_info.is_valid {
            tracing::warn!("[SERVER HOST] Restart server ditolak karena lisensi {:?}: {}", license_info.status, license_info.message);
            return (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "status": "error",
                    "error_code": "LICENSE_INVALID",
                    "license_status": license_info.status,
                    "message": format!("Server tidak dapat dimuat ulang: {}", license_info.message),
                    "is_running": false
                })),
            );
        }
    }

    // 3. Bangkitkan kembali POS Server di port 8080
    match crate::api::spawn_pos_server(state.clone()).await {
        Ok(handle) => {
            state.set_server_handle(handle).await;
            let mut running = state.is_running.write().await;
            *running = true;
            let mut started = state.started_at.write().await;
            *started = Some(Utc::now());
            drop(running);
            drop(started);

            let _ = state.tx.send(json!({
                "type": "SERVER_STARTED",
                "timestamp": Utc::now()
            }).to_string());

            info!("[SERVER HOST] Host Server berhasil dimuat ulang (Restarted) pada port {}", state.port);
            (
                StatusCode::OK,
                Json(json!({
                    "status": "success",
                    "message": "Host Server berhasil dimuat ulang (Restarted)",
                    "is_running": true
                })),
            )
        }
        Err(e) => {
            tracing::error!("[SERVER HOST] Gagal mengikat socket port {} saat restart: {}", state.port, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "status": "error",
                    "message": format!("Gagal menyalakan kembali server pada port {}: {}", state.port, e),
                    "is_running": false
                })),
            )
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ActivateLicenseRequest {
    pub token: String,
    pub signature: String,
}

#[derive(Debug, Deserialize)]
pub struct SaveProviderConfigRequest {
    pub provider_url: String,
    pub is_custom: bool,
}

#[derive(Debug, Deserialize)]
pub struct TestProviderConnectionRequest {
    pub provider_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ActivateLicenseOnlineRequest {
    pub license_key: String,
}

#[derive(Debug, Deserialize)]
pub struct ApplyUpdatePayloadRequest {
    pub download_url: String,
    pub checksum_sha256: String,
    pub signature: String,
    pub target_version: String,
}

async fn get_version_info() -> impl IntoResponse {
    let info = crate::services::update_service::UpdateService::get_version_info();
    Json(info)
}

async fn get_license_info(State(state): State<AppState>) -> impl IntoResponse {
    let pool_opt = state.pool.read().await.clone();
    if let Some(pool) = pool_opt {
        let license = crate::services::license_service::LicenseService::check_current_license(&pool).await;
        Json(license)
    } else {
        Json(crate::services::license_service::LicenseInfoResponse {
            status: crate::services::license_service::LicenseState::Unlicensed,
            is_valid: false,
            license_key: None,
            client_name: None,
            tier: None,
            max_terminals: 0,
            current_hardware_id: crate::services::license_service::LicenseService::get_hardware_id(),
            licensed_hardware_id: None,
            expires_at: None,
            days_remaining: None,
            grace_days_remaining: None,
            features: vec![],
            message: "Database belum aktif".to_string(),
            provider_server_url: "".to_string(),
        })
    }
}

async fn activate_license(
    State(state): State<AppState>,
    Json(req): Json<ActivateLicenseRequest>,
) -> impl IntoResponse {
    let pool_opt = state.pool.read().await.clone();
    if let Some(pool) = pool_opt {
        match crate::services::license_service::LicenseService::activate_license(&pool, &req.token, &req.signature).await {
            Ok(info) => {
                // Lisensi aktif! Otomatis nyalakan server POS jika belum berjalan
                let is_running = *state.is_running.read().await;
                if !is_running {
                    if let Ok(handle) = crate::api::spawn_pos_server(state.clone()).await {
                        state.set_server_handle(handle).await;
                        let mut running = state.is_running.write().await;
                        *running = true;
                        let mut started = state.started_at.write().await;
                        *started = Some(Utc::now());
                        info!("[SERVER HOST] Server POS otomatis dinyalakan setelah aktivasi lisensi berhasil.");
                        let _ = state.tx.send(json!({
                            "type": "SERVER_STARTED",
                            "timestamp": Utc::now()
                        }).to_string());
                    }
                }
                (
                    StatusCode::OK,
                    Json(json!({
                        "status": "success",
                        "message": "Lisensi berhasil diaktivasi secara resmi. Server Retail POS otomatis aktif!",
                        "data": info,
                        "is_running": true
                    })),
                )
            }
            Err(err) => (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "status": "error",
                    "message": err
                })),
            ),
        }
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "status": "error",
                "message": "Database server belum aktif. Silakan mulai server host terlebih dahulu."
            })),
        )
    }
}

async fn get_provider_config() -> impl IntoResponse {
    let current_url = crate::services::license_service::LicenseService::get_provider_url();
    let has_env_override = std::env::var("PROVIDER_SERVER_URL").is_ok()
        || std::env::var("NOTANE_PROVIDER_SERVER_URL").is_ok();

    Json(json!({
        "current_url": current_url,
        "has_env_override": has_env_override,
        "default_local_url": crate::services::license_service::LicenseService::DEFAULT_LOCAL_PROVIDER_URL,
        "default_production_url": crate::services::license_service::LicenseService::OFFICIAL_PRODUCTION_PROVIDER_URL,
        "is_production": !cfg!(debug_assertions)
    }))
}

async fn save_provider_config() -> impl IntoResponse {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({
            "status": "error",
            "message": "URL Provider diamankan dari manipulasi database lokal. Untuk mengubah alamat provider, setel variabel lingkungan PROVIDER_SERVER_URL di environment server (brands/<brand>.env)."
        })),
    )
}

async fn test_provider_connection(
    Json(payload): Json<TestProviderConnectionRequest>,
) -> impl IntoResponse {
    let target_url = if let Some(url) = payload.provider_url {
        let u = url.trim().trim_end_matches('/').to_string();
        if u.is_empty() {
            crate::services::license_service::LicenseService::get_provider_url()
        } else {
            u
        }
    } else {
        crate::services::license_service::LicenseService::get_provider_url()
    };

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(4))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "message": format!("Gagal inisialisasi HTTP client: {}", e)
                })),
            );
        }
    };

    let start = std::time::Instant::now();
    let health_url = format!("{}/health", target_url);

    match client.get(&health_url).send().await {
        Ok(resp) => {
            let latency_ms = start.elapsed().as_millis();
            if resp.status().is_success() {
                let body: serde_json::Value = resp.json().await.unwrap_or_else(|_| json!({}));
                (
                    StatusCode::OK,
                    Json(json!({
                        "success": true,
                        "status": "connected",
                        "target_url": target_url,
                        "latency_ms": latency_ms,
                        "service": body.get("service").and_then(|v| v.as_str()).unwrap_or("Notane Provider Server"),
                        "message": format!("Terhubung ke Provider Server ({} ms)", latency_ms)
                    })),
                )
            } else {
                (
                    StatusCode::BAD_GATEWAY,
                    Json(json!({
                        "success": false,
                        "status": "error_response",
                        "target_url": target_url,
                        "message": format!("Server provider merespons kode HTTP {}", resp.status())
                    })),
                )
            }
        }
        Err(e) => {
            let err_msg = if e.is_timeout() {
                "Koneksi timeout (server provider tidak merespons dalam 4 detik)".to_string()
            } else if e.is_connect() {
                format!("Gagal terhubung ke {} (Connection refused / Host unreachable)", target_url)
            } else {
                format!("Error koneksi ke provider: {}", e)
            };

            (
                StatusCode::BAD_GATEWAY,
                Json(json!({
                    "success": false,
                    "status": "unreachable",
                    "target_url": target_url,
                    "message": err_msg
                })),
            )
        }
    }
}

async fn activate_license_online(
    State(state): State<AppState>,
    Json(payload): Json<ActivateLicenseOnlineRequest>,
) -> impl IntoResponse {
    let pool_opt = state.pool.read().await.clone();
    let Some(pool) = pool_opt else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "status": "error", "message": "Database PostgreSQL belum aktif" })),
        );
    };

    let license_key = payload.license_key.trim().to_uppercase();
    if license_key.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "status": "error", "message": "Nomor lisensi tidak boleh kosong" })),
        );
    }

    let hwid = crate::services::license_service::LicenseService::get_hardware_id();
    let provider_url = crate::services::license_service::LicenseService::get_provider_url();
    let activate_endpoint = format!("{}/api/v1/license/activate", provider_url);

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "status": "error", "message": format!("HTTP client error: {}", e) })),
            );
        }
    };

    let req_body = serde_json::json!({
        "license_key": license_key,
        "hardware_id": hwid,
        "device_name": format!("Host-{}", std::env::consts::OS),
        "app_version": crate::services::update_service::APP_VERSION
    });

    let resp = match client.post(&activate_endpoint).json(&req_body).send().await {
        Ok(r) => r,
        Err(e) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({
                    "status": "error",
                    "message": format!("Gagal menghubungi Server Provider di {} ({}). Pastikan provider server aktif dan dapat dijangkau.", provider_url, e)
                })),
            );
        }
    };

    if !resp.status().is_success() {
        let err_body: serde_json::Value = resp.json().await.unwrap_or_else(|_| json!({}));
        let err_msg = err_body.get("message").and_then(|v| v.as_str()).unwrap_or("Server provider menolak permintaan aktivasi");
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "status": "error",
                "message": format!("Aktivasi Ditolak Provider: {}", err_msg)
            })),
        );
    }

    let resp_data: serde_json::Value = match resp.json().await {
        Ok(d) => d,
        Err(e) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "status": "error", "message": format!("Format respons provider tidak valid: {}", e) })),
            );
        }
    };

    let token = resp_data.pointer("/data/license_token").and_then(|v| v.as_str());
    let sig = resp_data.pointer("/data/signature").and_then(|v| v.as_str());

    let (token_str, sig_str) = match (token, sig) {
        (Some(t), Some(s)) => (t, s),
        _ => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "status": "error", "message": "Respons provider tidak memuat token dan signature yang lengkap" })),
            );
        }
    };

    match crate::services::license_service::LicenseService::activate_license(&pool, token_str, sig_str).await {
        Ok(info) => {
            // Lisensi aktif! Otomatis nyalakan server POS jika belum berjalan
            let is_running = *state.is_running.read().await;
            if !is_running {
                if let Ok(handle) = crate::api::spawn_pos_server(state.clone()).await {
                    state.set_server_handle(handle).await;
                    let mut running = state.is_running.write().await;
                    *running = true;
                    let mut started = state.started_at.write().await;
                    *started = Some(Utc::now());
                    info!("[SERVER HOST] Server POS otomatis dinyalakan setelah aktivasi lisensi online berhasil.");
                    let _ = state.tx.send(json!({
                        "type": "SERVER_STARTED",
                        "timestamp": Utc::now()
                    }).to_string());
                }
            }
            (
                StatusCode::OK,
                Json(json!({
                    "status": "success",
                    "message": "Lisensi resmi berhasil diaktivasi secara online dari Provider! Server Retail POS otomatis aktif.",
                    "data": info,
                    "is_running": true
                })),
            )
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "status": "error",
                "message": format!("Gagal menyimpan aktivasi: {}", e)
            })),
        ),
    }
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateCheckPayload {
    pub component: Option<String>,
    pub current_version: Option<String>,
}

async fn check_updates(
    State(state): State<AppState>,
    body: Option<Json<UpdateCheckPayload>>,
) -> impl IntoResponse {
    let payload = body.map(|b| b.0).unwrap_or_default();
    let comp = payload.component.as_deref().unwrap_or("server");
    let current_v = payload.current_version.as_deref();

    let pool_opt = state.pool.read().await.clone();
    if let Some(pool) = pool_opt {
        let updates = crate::services::update_service::UpdateService::check_updates_for(
            &pool,
            Some(comp),
            current_v,
        ).await;
        Json(updates)
    } else {
        Json(crate::services::update_service::UpdateCheckResponse {
            current_version: current_v.unwrap_or(crate::services::update_service::APP_VERSION).to_string(),
            has_update: false,
            latest_version: current_v.unwrap_or(crate::services::update_service::APP_VERSION).to_string(),
            release_name: None,
            changelog: None,
            release_date: None,
            download_url: None,
            checksum_sha256: None,
            signature: None,
            is_mandatory: false,
            license_status: crate::services::license_service::LicenseState::Unlicensed,
            can_update: false,
            message: "Database belum aktif".to_string(),
            component: Some(comp.to_string()),
            min_server_version: None,
            server_compatible: Some(true),
        })
    }
}

async fn apply_update(
    State(state): State<AppState>,
    Json(req): Json<ApplyUpdatePayloadRequest>,
) -> impl IntoResponse {
    let manifest_check = format!("target_version:{}:checksum:{}", req.target_version, req.checksum_sha256);
    if let Err(e) = crate::services::update_service::UpdateService::verify_update_payload(&manifest_check, &req.signature) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "status": "error",
                "message": e
            })),
        );
    }

    let pool_opt = state.pool.read().await.clone();
    let pool = match pool_opt {
        Some(p) => p,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({
                    "status": "error",
                    "message": "Database server belum aktif. Silakan mulai server host terlebih dahulu."
                })),
            );
        }
    };

    // Jalankan migrasi database otomatis & seeder jika ada perubahan skema
    let migration_res = crate::db::run_embedded_migrations(&pool).await;
    let migration_status = match migration_res {
        Ok(_) => "Migrasi skema database berhasil dieksekusi secara otomatis.".to_string(),
        Err(e) => {
            tracing::error!("Peringatan migrasi database saat apply_update: {}", e);
            format!("Pembaruan disiapkan dengan catatan migrasi: {}", e)
        }
    };

    let _ = crate::services::update_service::UpdateService::log_update_applied(
        &pool,
        crate::services::update_service::APP_VERSION,
        &req.target_version,
        Some("Release"),
        Some(&req.checksum_sha256),
        Some(&format!("Pembaruan One-Click Notane Server berhasil diterapkan. {}", migration_status))
    ).await;

    (
        StatusCode::OK,
        Json(json!({
            "status": "success",
            "message": format!("Pembaruan Notane Server ke versi {} berhasil diterapkan. {} Silakan restart aplikasi server untuk memuat proses baru.", req.target_version, migration_status),
            "updated_to": req.target_version,
            "migration_status": migration_status,
            "restart_required": true
        })),
    )
}

// ============================================================================
// STORE BRANDING & LOGO MANAGEMENT
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct SaveBrandingRequest {
    pub store_name: String,
    pub store_tagline: Option<String>,
    pub store_logo_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UploadStoreLogoRequest {
    pub filename: String,
    pub data_base64: String,
}

async fn get_store_branding(State(state): State<AppState>) -> impl IntoResponse {
    let pool_opt = state.pool.read().await.clone();
    let pool = match pool_opt {
        Some(p) => p,
        None => {
            return (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "data": {
                        "store_name": "NOTANE RETAIL STORE",
                        "store_tagline": "Pusat Belanja Retail Modern & Terpercaya",
                        "store_logo_url": null,
                        "remove_branding": false
                    }
                })),
            );
        }
    };

    let license = crate::services::license_service::LicenseService::check_current_license(&pool).await;
    let remove_branding = license.features.iter().any(|f| f == "remove_branding" || f == "white_label");

    let default_store_name = if remove_branding {
        "RETAIL STORE".to_string()
    } else {
        "NOTANE RETAIL STORE".to_string()
    };

    let default_store_tagline = if remove_branding {
        "Pusat Belanja Retail Modern & Terpercaya".to_string()
    } else {
        "Pusat Belanja Retail Modern & Terpercaya (Powered by Notane)".to_string()
    };

    let row = sqlx::query(
        "SELECT store_name, store_tagline, store_logo_url FROM receipt_settings ORDER BY created_at ASC LIMIT 1"
    )
    .fetch_optional(&pool)
    .await;

    match row {
        Ok(Some(r)) => {
            use sqlx::Row;
            let s_name: Option<String> = r.try_get("store_name").ok().flatten();
            let s_tag: Option<String> = r.try_get("store_tagline").ok().flatten();
            let s_logo: Option<String> = r.try_get("store_logo_url").ok().flatten();

            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "data": {
                        "store_name": s_name.unwrap_or(default_store_name),
                        "store_tagline": s_tag.unwrap_or(default_store_tagline),
                        "store_logo_url": s_logo,
                        "remove_branding": remove_branding,
                    }
                })),
            )
        }
        Ok(None) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "data": {
                    "store_name": default_store_name,
                    "store_tagline": default_store_tagline,
                    "store_logo_url": null,
                    "remove_branding": remove_branding,
                }
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "message": format!("Gagal mengambil profil branding toko: {}", e)
            })),
        ),
    }
}

async fn save_store_branding(
    State(state): State<AppState>,
    Json(payload): Json<SaveBrandingRequest>,
) -> impl IntoResponse {
    let pool_opt = state.pool.read().await.clone();
    let pool = match pool_opt {
        Some(p) => p,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({
                    "success": false,
                    "message": "Database server belum aktif. Silakan jalankan server host terlebih dahulu."
                })),
            );
        }
    };

    let clean_name = payload.store_name.trim();
    if clean_name.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "message": "Nama toko tidak boleh kosong"
            })),
        );
    }

    let existing_id = sqlx::query_scalar::<_, uuid::Uuid>(
        "SELECT id FROM receipt_settings ORDER BY created_at ASC LIMIT 1"
    )
    .fetch_optional(&pool)
    .await
    .unwrap_or(None);

    let res = if let Some(id) = existing_id {
        sqlx::query(
            "UPDATE receipt_settings
             SET store_name = $1, store_tagline = $2, store_logo_url = $3, updated_at = CURRENT_TIMESTAMP
             WHERE id = $4"
        )
        .bind(clean_name)
        .bind(&payload.store_tagline)
        .bind(&payload.store_logo_url)
        .bind(id)
        .execute(&pool)
        .await
    } else {
        sqlx::query(
            "INSERT INTO receipt_settings (store_name, store_tagline, store_logo_url)
             VALUES ($1, $2, $3)"
        )
        .bind(clean_name)
        .bind(&payload.store_tagline)
        .bind(&payload.store_logo_url)
        .execute(&pool)
        .await
    };

    match res {
        Ok(_) => {
            // Broadcast event real-time ke semua terminal kasir
            let broadcast = json!({
                "type": "STORE_BRANDING_UPDATED",
                "store_name": clean_name,
                "store_tagline": payload.store_tagline,
                "store_logo_url": payload.store_logo_url,
                "timestamp": Utc::now().timestamp_millis()
            });
            let _ = state.tx.send(broadcast.to_string());

            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "message": "Branding dan logo toko berhasil disimpan dan disinkronkan ke seluruh terminal!",
                    "data": {
                        "store_name": clean_name,
                        "store_tagline": payload.store_tagline,
                        "store_logo_url": payload.store_logo_url,
                    }
                })),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "message": format!("Gagal menyimpan profil branding toko ke database: {}", e)
            })),
        ),
    }
}

async fn upload_store_logo(
    State(_state): State<AppState>,
    Json(payload): Json<UploadStoreLogoRequest>,
) -> impl IntoResponse {
    let data_b64 = payload.data_base64.trim();
    let raw_b64 = if let Some(idx) = data_b64.find(',') {
        &data_b64[idx + 1..]
    } else {
        data_b64
    };

    use base64::Engine;
    let decoded = match base64::engine::general_purpose::STANDARD.decode(raw_b64) {
        Ok(b) => b,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "success": false, "message": format!("Format Base64 gambar tidak valid: {}", e) })),
            );
        }
    };

    let ext = if data_b64.starts_with("data:image/png") {
        "png"
    } else if data_b64.starts_with("data:image/jpeg") || data_b64.starts_with("data:image/jpg") {
        "jpg"
    } else if data_b64.starts_with("data:image/webp") {
        "webp"
    } else if data_b64.starts_with("data:image/svg") {
        "svg"
    } else {
        std::path::Path::new(&payload.filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("png")
    };

    let product_images_dir = std::env::var("PRODUCT_IMAGES_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join(".notane")
                .join("product_images")
        });

    let logos_dir = product_images_dir.join("logos");
    let _ = std::fs::create_dir_all(&logos_dir);

    let filename = format!("store_logo_{}.{}", uuid::Uuid::new_v4().simple(), ext);
    let target_path = logos_dir.join(&filename);

    if let Err(e) = std::fs::write(&target_path, decoded) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": format!("Gagal menyimpan file logo: {}", e) })),
        );
    }

    let public_url = format!("/uploads/product_images/logos/{}", filename);
    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": "Logo toko berhasil diupload",
            "image_url": public_url,
            "logo_url": public_url
        })),
    )
}

#[derive(Debug, Deserialize)]
pub struct InitialSetupRequest {
    pub store_name: String,
    pub store_tagline: Option<String>,
    pub store_logo_url: Option<String>,
    pub store_address: String,
    pub store_phone: String,
    pub pairing_code: String,
    pub main_outlet_name: String,
    pub main_outlet_address: Option<String>,
    pub main_outlet_phone: Option<String>,
    pub include_dummy_products: bool,
    pub admin_full_name: Option<String>,
    pub admin_password: Option<String>,
    pub supervisor_pin: Option<String>,
    pub cash_rounding_mode: Option<String>,
}

async fn get_setup_status(State(state): State<AppState>) -> impl IntoResponse {
    let pool_opt = state.pool.read().await.clone();
    let pool = match pool_opt {
        Some(p) => p,
        None => {
            return (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "data": {
                        "is_setup_completed": false,
                        "completed_at": null,
                        "has_existing_sales": false,
                        "current_config": null
                    }
                })),
            );
        }
    };

    use sqlx::Row;

    // Check receipt_settings
    let setting_row = sqlx::query(
        "SELECT store_name, store_tagline, store_logo_url, store_address, store_phone, 
                pairing_code, cash_rounding_mode, is_initial_setup_completed, initial_setup_completed_at 
         FROM receipt_settings ORDER BY created_at ASC LIMIT 1",
    )
    .fetch_optional(&pool)
    .await
    .ok()
    .flatten();

    let has_sales: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM sales LIMIT 1)")
        .fetch_one(&pool)
        .await
        .unwrap_or(false);

    let dummy_products_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM products WHERE id IN (
            'fb000000-0000-0000-0000-000000000001'::uuid, 
            'fb000000-0000-0000-0000-000000000002'::uuid, 
            'fb000000-0000-0000-0000-000000000003'::uuid
        )",
    )
    .fetch_one(&pool)
    .await
    .unwrap_or(0);

    // Outlet utama
    let outlet_row = sqlx::query(
        "SELECT id, code, name, address, phone FROM outlets WHERE code = 'OUT-001' OR is_warehouse = false ORDER BY created_at ASC LIMIT 1",
    )
    .fetch_optional(&pool)
    .await
    .ok()
    .flatten();

    // Admin info
    let admin_row = sqlx::query("SELECT full_name, username FROM users WHERE username = 'admin' LIMIT 1")
        .fetch_optional(&pool)
        .await
        .ok()
        .flatten();

    let is_completed = setting_row
        .as_ref()
        .and_then(|r| r.try_get::<bool, _>("is_initial_setup_completed").ok())
        .unwrap_or(false);
    let completed_at: Option<chrono::DateTime<Utc>> = setting_row
        .as_ref()
        .and_then(|r| r.try_get("initial_setup_completed_at").ok());

    let current_pairing = state.pairing_code.read().await.clone();

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "data": {
                "is_setup_completed": is_completed,
                "completed_at": completed_at,
                "has_existing_sales": has_sales,
                "dummy_products_count": dummy_products_count,
                "current_config": {
                    "store_name": setting_row.as_ref().and_then(|r| r.try_get::<String, _>("store_name").ok()).unwrap_or_else(|| "NOTANE RETAIL STORE".to_string()),
                    "store_tagline": setting_row.as_ref().and_then(|r| r.try_get::<Option<String>, _>("store_tagline").ok()).flatten().unwrap_or_else(|| "Pusat Belanja Retail Modern & Terpercaya".to_string()),
                    "store_logo_url": setting_row.as_ref().and_then(|r| r.try_get::<Option<String>, _>("store_logo_url").ok()).flatten(),
                    "store_address": setting_row.as_ref().and_then(|r| r.try_get::<String, _>("store_address").ok()).unwrap_or_else(|| "Jl. Sudirman No. 123, Jakarta".to_string()),
                    "store_phone": setting_row.as_ref().and_then(|r| r.try_get::<String, _>("store_phone").ok()).unwrap_or_else(|| "021-5551234".to_string()),
                    "pairing_code": setting_row.as_ref().and_then(|r| r.try_get::<String, _>("pairing_code").ok()).unwrap_or(current_pairing),
                    "cash_rounding_mode": setting_row.as_ref().and_then(|r| r.try_get::<String, _>("cash_rounding_mode").ok()).unwrap_or_else(|| "NONE".to_string()),
                    "main_outlet_id": outlet_row.as_ref().and_then(|r| r.try_get::<uuid::Uuid, _>("id").ok()),
                    "main_outlet_code": outlet_row.as_ref().and_then(|r| r.try_get::<String, _>("code").ok()).unwrap_or_else(|| "OUT-001".to_string()),
                    "main_outlet_name": outlet_row.as_ref().and_then(|r| r.try_get::<String, _>("name").ok()).unwrap_or_else(|| "Notane Retail Flagship Store".to_string()),
                    "main_outlet_address": outlet_row.as_ref().and_then(|r| r.try_get::<Option<String>, _>("address").ok()).flatten(),
                    "main_outlet_phone": outlet_row.as_ref().and_then(|r| r.try_get::<Option<String>, _>("phone").ok()).flatten(),
                    "admin_full_name": admin_row.as_ref().and_then(|r| r.try_get::<String, _>("full_name").ok()).unwrap_or_else(|| "Administrator Utama".to_string()),
                    "admin_username": "admin"
                }
            }
        })),
    )
}

async fn perform_initial_setup(
    State(state): State<AppState>,
    Json(payload): Json<InitialSetupRequest>,
) -> impl IntoResponse {
    let pool_opt = state.pool.read().await.clone();
    let pool = match pool_opt {
        Some(p) => p,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "success": false, "message": "Database server belum aktif." })),
            );
        }
    };

    let clean_store_name = payload.store_name.trim();
    if clean_store_name.len() < 3 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "Nama toko minimal 3 karakter." })),
        );
    }

    let clean_outlet_name = payload.main_outlet_name.trim();
    if clean_outlet_name.len() < 3 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "Nama cabang utama minimal 3 karakter." })),
        );
    }

    let clean_pairing_code = payload.pairing_code.trim().to_uppercase();
    if clean_pairing_code.len() < 4 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "Kode pairing minimal 4 karakter." })),
        );
    }

    let mut tx = match pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "success": false, "message": format!("Gagal memulai transaksi: {}", e) })),
            );
        }
    };

    // 1. Update receipt_settings
    let store_address = payload.store_address.trim();
    let store_phone = payload.store_phone.trim();
    let store_tagline = payload.store_tagline.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty());
    let store_logo = payload.store_logo_url.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty());
    let rounding_mode = payload.cash_rounding_mode.as_deref().unwrap_or("NONE");

    let res_settings = sqlx::query(
        "UPDATE receipt_settings
         SET store_name = $1,
             store_tagline = COALESCE($2, store_tagline),
             store_logo_url = $3,
             store_address = $4,
             store_phone = $5,
             pairing_code = $6,
             cash_rounding_mode = $7,
             is_initial_setup_completed = TRUE,
             initial_setup_completed_at = CURRENT_TIMESTAMP,
             updated_at = CURRENT_TIMESTAMP",
    )
    .bind(clean_store_name)
    .bind(store_tagline)
    .bind(store_logo)
    .bind(store_address)
    .bind(store_phone)
    .bind(&clean_pairing_code)
    .bind(rounding_mode)
    .execute(&mut *tx)
    .await;

    if let Err(e) = res_settings {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": format!("Gagal menyimpan pengaturan toko: {}", e) })),
        );
    }

    // 2. Update Cabang Utama
    let outlet_addr = payload
        .main_outlet_address
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or(store_address);
    let outlet_ph = payload
        .main_outlet_phone
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or(store_phone);

    let res_outlet = sqlx::query(
        "UPDATE outlets
         SET name = $1,
             address = $2,
             phone = $3,
             updated_at = CURRENT_TIMESTAMP
         WHERE code = 'OUT-001' OR id = (SELECT id FROM outlets WHERE is_warehouse = false ORDER BY created_at ASC LIMIT 1)",
    )
    .bind(clean_outlet_name)
    .bind(outlet_addr)
    .bind(outlet_ph)
    .execute(&mut *tx)
    .await;

    if let Err(e) = res_outlet {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": format!("Gagal memperbarui data cabang utama: {}", e) })),
        );
    }

    // 3. Tangani Dummy / Demo Products
    if !payload.include_dummy_products {
        let dummy_ids = [
            uuid::Uuid::parse_str("fb000000-0000-0000-0000-000000000001").unwrap(),
            uuid::Uuid::parse_str("fb000000-0000-0000-0000-000000000002").unwrap(),
            uuid::Uuid::parse_str("fb000000-0000-0000-0000-000000000003").unwrap(),
        ];

        // Cek apakah produk demo sudah pernah dijual di POS
        let sales_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sale_items WHERE product_id = ANY($1)",
        )
        .bind(&dummy_ids[..])
        .fetch_one(&mut *tx)
        .await
        .unwrap_or(0);

        if sales_count == 0 {
            // Hapus batch stok demo
            let _ = sqlx::query("DELETE FROM product_stock_batches WHERE product_id = ANY($1)")
                .bind(&dummy_ids[..])
                .execute(&mut *tx)
                .await;

            // Hapus mutasi stok demo
            let _ = sqlx::query("DELETE FROM stock_mutations WHERE product_id = ANY($1)")
                .bind(&dummy_ids[..])
                .execute(&mut *tx)
                .await;

            // Hapus stok fisik gudang demo
            let _ = sqlx::query("DELETE FROM warehouse_stocks WHERE product_id = ANY($1)")
                .bind(&dummy_ids[..])
                .execute(&mut *tx)
                .await;

            // Hapus harga khusus outlet demo
            let _ = sqlx::query("DELETE FROM product_outlet_prices WHERE product_id = ANY($1)")
                .bind(&dummy_ids[..])
                .execute(&mut *tx)
                .await;

            // Hapus harga tier demo
            let _ = sqlx::query("DELETE FROM product_price_tiers WHERE product_id = ANY($1)")
                .bind(&dummy_ids[..])
                .execute(&mut *tx)
                .await;

            // Hapus multi unit demo
            let _ = sqlx::query("DELETE FROM product_units WHERE product_id = ANY($1)")
                .bind(&dummy_ids[..])
                .execute(&mut *tx)
                .await;

            // Hapus produk demo
            let _ = sqlx::query("DELETE FROM products WHERE id = ANY($1)")
                .bind(&dummy_ids[..])
                .execute(&mut *tx)
                .await;

            // Hapus voucher demo jika belum ada klaim
            let _ = sqlx::query(
                "DELETE FROM vouchers WHERE code = 'HEMAT10K' AND NOT EXISTS (SELECT 1 FROM voucher_claims WHERE voucher_id = vouchers.id)",
            )
            .execute(&mut *tx)
            .await;

            // Hapus pelanggan member demo jika belum ada riwayat penjualan
            let _ = sqlx::query(
                "DELETE FROM customers WHERE code = 'MBR-001' AND NOT EXISTS (SELECT 1 FROM sales WHERE customer_id = customers.id)",
            )
            .execute(&mut *tx)
            .await;

            // Hapus jurnal saldo awal demo produk
            let _ = sqlx::query(
                "DELETE FROM journal_lines WHERE journal_id IN (
                    SELECT id FROM journal_entries WHERE journal_number = 'JRN-INIT-OUT-001'
                )",
            )
            .execute(&mut *tx)
            .await;

            let _ = sqlx::query(
                "DELETE FROM journal_entries WHERE journal_number = 'JRN-INIT-OUT-001'",
            )
            .execute(&mut *tx)
            .await;
        }
    }

    // 4. Update Akun Admin
    if let Some(ref pwd) = payload.admin_password {
        let clean_pwd = pwd.trim();
        if clean_pwd.len() >= 6 {
            match crate::utils::hash::hash_secret(clean_pwd) {
                Ok(h) => {
                    let _ = sqlx::query(
                        "UPDATE users SET password_hash = $1, updated_at = CURRENT_TIMESTAMP WHERE username = 'admin'",
                    )
                    .bind(h)
                    .execute(&mut *tx)
                    .await;
                }
                Err(e) => {
                    tracing::error!("Gagal meng-hash password admin: {}", e);
                }
            }
        }
    }

    if let Some(ref pin) = payload.supervisor_pin {
        let clean_pin = pin.trim();
        if clean_pin.len() >= 4 {
            match crate::utils::hash::hash_secret(clean_pin) {
                Ok(h) => {
                    let _ = sqlx::query(
                        "UPDATE users SET pin_hash = $1, updated_at = CURRENT_TIMESTAMP WHERE username IN ('admin', 'spv')",
                    )
                    .bind(h)
                    .execute(&mut *tx)
                    .await;
                }
                Err(e) => {
                    tracing::error!("Gagal meng-hash PIN supervisor: {}", e);
                }
            }
        }
    }

    if let Some(ref name) = payload.admin_full_name {
        let clean_name = name.trim();
        if !clean_name.is_empty() {
            let _ = sqlx::query(
                "UPDATE users SET full_name = $1, updated_at = CURRENT_TIMESTAMP WHERE username = 'admin'",
            )
            .bind(clean_name)
            .execute(&mut *tx)
            .await;
        }
    }

    // 5. Catat ke Audit Log
    let setup_summary = json!({
        "store_name": clean_store_name,
        "main_outlet_name": clean_outlet_name,
        "pairing_code": clean_pairing_code,
        "include_dummy_products": payload.include_dummy_products,
        "has_logo": store_logo.is_some()
    });

    let _ = sqlx::query(
        "INSERT INTO audit_logs (id, user_id, action, entity_type, entity_id, new_values, created_at)
         VALUES (
            gen_random_uuid(),
            (SELECT id FROM users WHERE username = 'admin' LIMIT 1),
            'INITIAL_SETUP_WIZARD_COMPLETED',
            'SYSTEM',
            '00000000-0000-0000-0000-000000000000'::uuid,
            $1,
            CURRENT_TIMESTAMP
         )",
    )
    .bind(setup_summary.to_string())
    .execute(&mut *tx)
    .await;

    if let Err(e) = tx.commit().await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": format!("Gagal menyimpan transaksi: {}", e) })),
        );
    }

    // Update in-memory state
    state.set_pairing_code(clean_pairing_code).await;

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": "Konfigurasi awal toko dan server berhasil disimpan!"
        })),
    )
}

