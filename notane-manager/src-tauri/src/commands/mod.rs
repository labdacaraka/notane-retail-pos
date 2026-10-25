use crate::api::{spawn_api_server, ApiState};
use crate::crypto::ed25519::{
    derive_public_key, sign_message, DEFAULT_MASTER_PUBLIC_KEY_HEX,
};
use crate::crypto::hash::{hash_secret, verify_secret};
use crate::db::DbPool;
use crate::models::{
    AdminUser, AppError, AppRelease, AuditLog, AuthResponse, ChangeCredentialsDto, Client,
    CreateClientDto, CreateLicenseDto, CreateReleaseDto, DeviceActivation, License,
    LicenseTokenPayload, LoginDto, ManualSignResult, PinLoginDto, ProviderSettings, ProviderStatus,
    UpdateClientDto,
};
use chrono::{Duration, Utc};
use rand::Rng;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use validator::Validate;

#[derive(Clone)]
pub struct AppState {
    pub pool: DbPool,
    pub api_state: ApiState,
    pub active_sessions: Arc<RwLock<HashSet<String>>>,
}

// ------------------------------------------------------------------------------
// PROVIDER STATUS & SERVER CONTROL
// ------------------------------------------------------------------------------
#[tauri::command]
pub async fn get_provider_status(state: tauri::State<'_, AppState>) -> Result<ProviderStatus, AppError> {
    let pool = &state.pool;

    let is_running = *state.api_state.is_running.read().await;
    let port = state.api_state.port;

    let total_clients: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM provider_clients")
        .fetch_one(pool)
        .await
        .unwrap_or((0,));

    let total_licenses: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM provider_licenses")
        .fetch_one(pool)
        .await
        .unwrap_or((0,));

    let active_licenses: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM provider_licenses WHERE status = 'ACTIVE'")
        .fetch_one(pool)
        .await
        .unwrap_or((0,));

    let total_devices: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM provider_device_activations WHERE is_active = 1")
        .fetch_one(pool)
        .await
        .unwrap_or((0,));

    let total_releases: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM provider_app_releases")
        .fetch_one(pool)
        .await
        .unwrap_or((0,));

    let priv_key = state.api_state.master_private_key.read().await;
    let public_key = derive_public_key(&priv_key).unwrap_or_else(|_| DEFAULT_MASTER_PUBLIC_KEY_HEX.to_string());

    Ok(ProviderStatus {
        is_server_running: is_running,
        port,
        external_url: format!("http://127.0.0.1:{}", port),
        total_clients: total_clients.0,
        total_licenses: total_licenses.0,
        active_licenses: active_licenses.0,
        total_devices: total_devices.0,
        total_releases: total_releases.0,
        master_public_key: public_key,
    })
}

#[tauri::command]
pub async fn toggle_provider_server(
    state: tauri::State<'_, AppState>,
    enable: bool,
) -> Result<bool, AppError> {
    let mut is_running = state.api_state.is_running.write().await;
    let mut handle_guard = state.api_state.server_handle.write().await;

    if enable && !*is_running {
        match spawn_api_server(state.api_state.clone()).await {
            Ok(handle) => {
                *handle_guard = Some(handle);
                *is_running = true;
                Ok(true)
            }
            Err(e) => Err(AppError::Internal(format!("Gagal menyalakan server API: {}", e))),
        }
    } else if !enable && *is_running {
        if let Some(handle) = handle_guard.take() {
            handle.abort();
        }
        *is_running = false;
        Ok(false)
    } else {
        Ok(*is_running)
    }
}

// ------------------------------------------------------------------------------
// CLIENTS (CRUD)
// ------------------------------------------------------------------------------
#[tauri::command]
pub async fn get_clients(state: tauri::State<'_, AppState>) -> Result<Vec<Client>, AppError> {
    let clients = sqlx::query_as::<_, Client>(
        "SELECT id, client_name, company_name, email, phone, address, created_at, updated_at
         FROM provider_clients
         ORDER BY created_at DESC"
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(clients)
}

#[tauri::command]
pub async fn save_client(
    state: tauri::State<'_, AppState>,
    payload: CreateClientDto,
) -> Result<Client, AppError> {
    payload.validate().map_err(|e| AppError::Validation(e.to_string()))?;

    let id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        "INSERT INTO provider_clients (id, client_name, company_name, email, phone, address, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(&payload.client_name)
    .bind(&payload.company_name)
    .bind(&payload.email)
    .bind(&payload.phone)
    .bind(&payload.address)
    .bind(&now)
    .bind(&now)
    .execute(&state.pool)
    .await?;

    Ok(Client {
        id,
        client_name: payload.client_name,
        company_name: payload.company_name,
        email: payload.email,
        phone: payload.phone,
        address: payload.address,
        created_at: now.clone(),
        updated_at: now,
    })
}

#[tauri::command]
pub async fn update_client(
    state: tauri::State<'_, AppState>,
    payload: UpdateClientDto,
) -> Result<bool, AppError> {
    payload.validate().map_err(|e| AppError::Validation(e.to_string()))?;

    let now = Utc::now().to_rfc3339();

    let rows_affected = sqlx::query(
        "UPDATE provider_clients
         SET client_name = ?, company_name = ?, email = ?, phone = ?, address = ?, updated_at = ?
         WHERE id = ?"
    )
    .bind(&payload.client_name)
    .bind(&payload.company_name)
    .bind(&payload.email)
    .bind(&payload.phone)
    .bind(&payload.address)
    .bind(&now)
    .bind(&payload.id)
    .execute(&state.pool)
    .await?
    .rows_affected();

    if rows_affected == 0 {
        return Err(AppError::NotFound("Klien tidak ditemukan".to_string()));
    }

    Ok(true)
}

#[tauri::command]
pub async fn delete_client(
    state: tauri::State<'_, AppState>,
    client_id: String,
) -> Result<bool, AppError> {
    let rows_affected = sqlx::query("DELETE FROM provider_clients WHERE id = ?")
        .bind(&client_id)
        .execute(&state.pool)
        .await?
        .rows_affected();

    Ok(rows_affected > 0)
}

// ------------------------------------------------------------------------------
// LICENSES
// ------------------------------------------------------------------------------
#[tauri::command]
pub async fn get_licenses(state: tauri::State<'_, AppState>) -> Result<Vec<License>, AppError> {
    let rows = sqlx::query_as::<_, License>(
        "SELECT l.id, l.license_key, l.client_id, c.client_name, l.tier, l.max_terminals,
                l.status, l.issued_at, l.expires_at, l.grace_period_days, l.features,
                l.notes, l.created_at, l.updated_at,
                (SELECT COUNT(*) FROM provider_device_activations d WHERE d.license_id = l.id AND d.is_active = 1) AS active_terminals
         FROM provider_licenses l
         LEFT JOIN provider_clients c ON l.client_id = c.id
         ORDER BY l.created_at DESC"
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(rows)
}

#[tauri::command]
pub async fn generate_license(
    state: tauri::State<'_, AppState>,
    payload: CreateLicenseDto,
) -> Result<License, AppError> {
    payload.validate().map_err(|e| AppError::Validation(e.to_string()))?;

    let license_key = match payload.custom_key {
        Some(k) if !k.trim().is_empty() => k.trim().to_uppercase(),
        _ => {
            let year = Utc::now().format("%Y");
            let mut rng = rand::thread_rng();
            let suffix: u32 = rng.gen_range(1000..9999);
            format!("NTN-{}-{}-X{}", payload.tier[..3].to_uppercase(), year, suffix)
        }
    };

    let id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now();
    let expires_at = now + Duration::days(payload.expires_in_days);
    let now_str = now.to_rfc3339();
    let expires_str = expires_at.to_rfc3339();

    let features_json = serde_json::to_string(&payload.features).unwrap_or_else(|_| "[]".to_string());
    let grace_days = payload.grace_period_days.unwrap_or(7);

    sqlx::query(
        "INSERT INTO provider_licenses (
            id, license_key, client_id, tier, max_terminals, status, issued_at, expires_at,
            grace_period_days, features, notes, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, 'ACTIVE', ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(&license_key)
    .bind(&payload.client_id)
    .bind(&payload.tier)
    .bind(payload.max_terminals)
    .bind(&now_str)
    .bind(&expires_str)
    .bind(grace_days)
    .bind(&features_json)
    .bind(&payload.notes)
    .bind(&now_str)
    .bind(&now_str)
    .execute(&state.pool)
    .await?;

    let client_name: Option<String> = sqlx::query_scalar("SELECT client_name FROM provider_clients WHERE id = ?")
        .bind(&payload.client_id)
        .fetch_optional(&state.pool)
        .await
        .unwrap_or(None);

    Ok(License {
        id,
        license_key,
        client_id: payload.client_id,
        client_name,
        tier: payload.tier,
        max_terminals: payload.max_terminals,
        status: "ACTIVE".to_string(),
        issued_at: now_str.clone(),
        expires_at: expires_str,
        grace_period_days: grace_days,
        features: features_json,
        notes: payload.notes,
        active_terminals: Some(0),
        created_at: now_str.clone(),
        updated_at: now_str,
    })
}

#[tauri::command]
pub async fn update_license_status(
    state: tauri::State<'_, AppState>,
    license_id: String,
    status: String,
) -> Result<bool, AppError> {
    let now = Utc::now().to_rfc3339();
    let rows = sqlx::query(
        "UPDATE provider_licenses SET status = ?, updated_at = ? WHERE id = ?"
    )
    .bind(&status)
    .bind(&now)
    .bind(&license_id)
    .execute(&state.pool)
    .await?
    .rows_affected();

    Ok(rows > 0)
}

// ------------------------------------------------------------------------------
// OFFLINE TOKEN & SIGNATURE GENERATOR
// ------------------------------------------------------------------------------
#[derive(Debug, sqlx::FromRow)]
struct ManualSignLicenseRow {
    license_key: String,
    tier: String,
    max_terminals: i64,
    expires_at: String,
    grace_period_days: i64,
    features: String,
    client_name: String,
}

#[tauri::command]
pub async fn sign_license_token_manual(
    state: tauri::State<'_, AppState>,
    license_key: String,
    hardware_id: String,
) -> Result<ManualSignResult, AppError> {
    let clean_key = license_key.trim();
    let clean_hwid = hardware_id.trim();

    let row = sqlx::query_as::<_, ManualSignLicenseRow>(
        "SELECT l.license_key, l.tier, l.max_terminals, l.expires_at, l.grace_period_days,
                l.features, c.client_name
         FROM provider_licenses l
         JOIN provider_clients c ON l.client_id = c.id
         WHERE l.license_key = ?"
    )
    .bind(clean_key)
    .fetch_optional(&state.pool)
    .await?;

    let license = match row {
        Some(l) => l,
        None => return Err(AppError::NotFound("Nomor lisensi tidak terdaftar di sistem".to_string())),
    };

    let features: Vec<String> = serde_json::from_str(&license.features).unwrap_or_default();

    let token_payload = LicenseTokenPayload {
        license_key: license.license_key,
        client_name: license.client_name,
        outlet_id: None,
        tier: license.tier,
        max_terminals: license.max_terminals,
        hardware_id: clean_hwid.to_string(),
        issued_at: Utc::now().to_rfc3339(),
        expires_at: license.expires_at,
        offline_grace_days: license.grace_period_days,
        features,
    };

    let token_json = serde_json::to_string(&token_payload)
        .map_err(|e| AppError::Internal(format!("Gagal encode JSON token: {}", e)))?;

    let priv_key = state.api_state.master_private_key.read().await;
    let signature = sign_message(&priv_key, &token_json)
        .map_err(|e| AppError::Crypto(e))?;

    let pub_key = derive_public_key(&priv_key)
        .unwrap_or_else(|_| DEFAULT_MASTER_PUBLIC_KEY_HEX.to_string());

    Ok(ManualSignResult {
        license_token_json: token_json,
        signature_hex: signature,
        public_key_hex: pub_key,
    })
}

// ------------------------------------------------------------------------------
// DEVICE ACTIVATIONS (Node-Locking HWID)
// ------------------------------------------------------------------------------
#[tauri::command]
pub async fn get_device_activations(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<DeviceActivation>, AppError> {
    let rows = sqlx::query_as::<_, DeviceActivation>(
        "SELECT d.id, d.license_id, l.license_key, c.client_name, d.hardware_id,
                d.device_name, d.ip_address, d.os_info, d.app_version, d.activated_at,
                d.last_ping_at, d.is_active
         FROM provider_device_activations d
         JOIN provider_licenses l ON d.license_id = l.id
         JOIN provider_clients c ON l.client_id = c.id
         ORDER BY d.last_ping_at DESC"
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(rows)
}

#[tauri::command]
pub async fn unbind_device(
    state: tauri::State<'_, AppState>,
    device_id: String,
) -> Result<bool, AppError> {
    let rows = sqlx::query("DELETE FROM provider_device_activations WHERE id = ?")
        .bind(&device_id)
        .execute(&state.pool)
        .await?
        .rows_affected();

    Ok(rows > 0)
}

// ------------------------------------------------------------------------------
// OTA APP RELEASES
// ------------------------------------------------------------------------------
#[tauri::command]
pub async fn get_releases(state: tauri::State<'_, AppState>) -> Result<Vec<AppRelease>, AppError> {
    let rows = sqlx::query_as::<_, AppRelease>(
        "SELECT id, version, release_name, changelog, release_date, download_url,
                checksum_sha256, signature_ed25519, is_mandatory, min_license_tier,
                has_db_migration, is_published, component, min_server_version, created_at
         FROM provider_app_releases
         ORDER BY created_at DESC"
    )
    .fetch_all(&state.pool)
    .await?;

    Ok(rows)
}

#[tauri::command]
pub async fn create_release(
    state: tauri::State<'_, AppState>,
    payload: CreateReleaseDto,
) -> Result<AppRelease, AppError> {
    payload.validate().map_err(|e| AppError::Validation(e.to_string()))?;

    let id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now();
    let now_str = now.to_rfc3339();
    let release_date = now.format("%Y-%m-%d").to_string();
    let component = payload.component.clone().unwrap_or_else(|| "server".to_string());

    let priv_key = state.api_state.master_private_key.read().await;
    let sign_target = format!("{}:{}", payload.version.trim(), payload.checksum_sha256.trim());
    let signature = sign_message(&priv_key, &sign_target)
        .map_err(|e| AppError::Crypto(e))?;

    sqlx::query(
        "INSERT INTO provider_app_releases (
            id, version, release_name, changelog, release_date, download_url,
            checksum_sha256, signature_ed25519, is_mandatory, min_license_tier,
            has_db_migration, is_published, component, min_server_version, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, ?, ?, ?)"
    )
    .bind(&id)
    .bind(payload.version.trim())
    .bind(&payload.release_name)
    .bind(&payload.changelog)
    .bind(&release_date)
    .bind(&payload.download_url)
    .bind(payload.checksum_sha256.trim())
    .bind(&signature)
    .bind(if payload.is_mandatory { 1 } else { 0 })
    .bind(&payload.min_license_tier)
    .bind(if payload.has_db_migration { 1 } else { 0 })
    .bind(&component)
    .bind(&payload.min_server_version)
    .bind(&now_str)
    .execute(&state.pool)
    .await?;

    Ok(AppRelease {
        id,
        version: payload.version,
        release_name: payload.release_name,
        changelog: payload.changelog,
        release_date,
        download_url: payload.download_url,
        checksum_sha256: payload.checksum_sha256,
        signature_ed25519: signature,
        is_mandatory: if payload.is_mandatory { 1 } else { 0 },
        min_license_tier: payload.min_license_tier,
        has_db_migration: if payload.has_db_migration { 1 } else { 0 },
        is_published: 1,
        component,
        min_server_version: payload.min_server_version,
        created_at: now_str,
    })
}

#[tauri::command]
pub async fn delete_release(
    state: tauri::State<'_, AppState>,
    release_id: String,
) -> Result<bool, AppError> {
    let rows = sqlx::query("DELETE FROM provider_app_releases WHERE id = ?")
        .bind(&release_id)
        .execute(&state.pool)
        .await?
        .rows_affected();

    Ok(rows > 0)
}

// ------------------------------------------------------------------------------
// AUDIT LOGS
// ------------------------------------------------------------------------------
#[tauri::command]
pub async fn get_audit_logs(
    state: tauri::State<'_, AppState>,
    limit: Option<i64>,
) -> Result<Vec<AuditLog>, AppError> {
    let l = limit.unwrap_or(50);
    let rows = sqlx::query_as::<_, AuditLog>(
        "SELECT id, license_key, hardware_id, action, ip_address, status, details, created_at
         FROM provider_license_audit_logs
         ORDER BY created_at DESC LIMIT ?"
    )
    .bind(l)
    .fetch_all(&state.pool)
    .await?;

    Ok(rows)
}

// ------------------------------------------------------------------------------
// SETTINGS
// ------------------------------------------------------------------------------
#[tauri::command]
pub async fn get_provider_settings(
    state: tauri::State<'_, AppState>,
) -> Result<ProviderSettings, AppError> {
    let priv_key = state.api_state.master_private_key.read().await.clone();
    let pub_key = derive_public_key(&priv_key).unwrap_or_else(|_| DEFAULT_MASTER_PUBLIC_KEY_HEX.to_string());

    Ok(ProviderSettings {
        server_port: state.api_state.port,
        auto_start_server: true,
        master_private_key: priv_key,
        master_public_key: pub_key,
    })
}

#[tauri::command]
pub async fn save_provider_settings(
    state: tauri::State<'_, AppState>,
    settings: ProviderSettings,
) -> Result<bool, AppError> {
    let clean_priv = settings.master_private_key.trim();
    if !clean_priv.is_empty() {
        derive_public_key(clean_priv)
            .map_err(|e| AppError::Validation(format!("Kunci privat Ed25519 tidak valid: {}", e)))?;

        let mut key_guard = state.api_state.master_private_key.write().await;
        *key_guard = clean_priv.to_string();

        let _ = sqlx::query("UPDATE provider_settings SET value = ? WHERE key = 'master_private_key'")
            .bind(clean_priv)
            .execute(&state.pool)
            .await;
    }

    Ok(true)
}

// ------------------------------------------------------------------------------
// AUTHENTICATION & ACCESS CONTROL
// ------------------------------------------------------------------------------
#[derive(Debug, sqlx::FromRow)]
#[allow(dead_code)]
struct UserDbRow {
    id: String,
    username: String,
    full_name: String,
    password_hash: String,
    quick_pin: String,
    role: String,
    last_login_at: Option<String>,
}

#[tauri::command]
pub async fn login_admin(
    state: tauri::State<'_, AppState>,
    payload: LoginDto,
) -> Result<AuthResponse, AppError> {
    payload.validate().map_err(|e| AppError::Validation(e.to_string()))?;

    let row = sqlx::query_as::<_, UserDbRow>(
        "SELECT id, username, full_name, password_hash, quick_pin, role, last_login_at
         FROM provider_users
         WHERE username = ?"
    )
    .bind(payload.username.trim())
    .fetch_optional(&state.pool)
    .await?;

    let user = match row {
        Some(u) => u,
        None => return Err(AppError::Unauthorized("Username atau password salah".to_string())),
    };

    if !verify_secret(payload.password.trim(), &user.password_hash) {
        return Err(AppError::Unauthorized("Username atau password salah".to_string()));
    }

    let token = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    let _ = sqlx::query("UPDATE provider_users SET last_login_at = ? WHERE id = ?")
        .bind(&now)
        .bind(&user.id)
        .execute(&state.pool)
        .await;

    let mut sessions = state.active_sessions.write().await;
    sessions.insert(token.clone());

    let permissions = vec![
        "client:read".to_string(), "client:create".to_string(), "client:update".to_string(), "client:delete".to_string(),
        "license:read".to_string(), "license:create".to_string(), "license:update".to_string(), "license:revoke".to_string(),
        "device:read".to_string(), "device:unbind".to_string(),
        "release:read".to_string(), "release:create".to_string(), "release:delete".to_string(),
        "server:control".to_string(), "audit:read".to_string(),
    ];

    Ok(AuthResponse {
        user: AdminUser {
            id: user.id,
            username: user.username,
            full_name: user.full_name,
            role: user.role,
            last_login_at: Some(now),
        },
        token,
        permissions,
    })
}

#[tauri::command]
pub async fn login_with_pin(
    state: tauri::State<'_, AppState>,
    payload: PinLoginDto,
) -> Result<AuthResponse, AppError> {
    payload.validate().map_err(|e| AppError::Validation(e.to_string()))?;

    let row = sqlx::query_as::<_, UserDbRow>(
        "SELECT id, username, full_name, password_hash, quick_pin, role, last_login_at
         FROM provider_users
         ORDER BY id ASC LIMIT 1"
    )
    .fetch_optional(&state.pool)
    .await?;

    let user = match row {
        Some(u) => u,
        None => return Err(AppError::Unauthorized("Akun admin tidak ditemukan".to_string())),
    };

    if !verify_secret(payload.pin.trim(), &user.quick_pin) {
        return Err(AppError::Unauthorized("PIN keamanan salah".to_string()));
    }

    let token = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    let _ = sqlx::query("UPDATE provider_users SET last_login_at = ? WHERE id = ?")
        .bind(&now)
        .bind(&user.id)
        .execute(&state.pool)
        .await;

    let mut sessions = state.active_sessions.write().await;
    sessions.insert(token.clone());

    let permissions = vec![
        "client:read".to_string(), "client:create".to_string(), "client:update".to_string(), "client:delete".to_string(),
        "license:read".to_string(), "license:create".to_string(), "license:update".to_string(), "license:revoke".to_string(),
        "device:read".to_string(), "device:unbind".to_string(),
        "release:read".to_string(), "release:create".to_string(), "release:delete".to_string(),
        "server:control".to_string(), "audit:read".to_string(),
    ];

    Ok(AuthResponse {
        user: AdminUser {
            id: user.id,
            username: user.username,
            full_name: user.full_name,
            role: user.role,
            last_login_at: Some(now),
        },
        token,
        permissions,
    })
}

#[tauri::command]
pub async fn verify_admin_session(
    state: tauri::State<'_, AppState>,
    token: String,
) -> Result<bool, AppError> {
    let sessions = state.active_sessions.read().await;
    Ok(sessions.contains(&token))
}

#[tauri::command]
pub async fn logout_admin(
    state: tauri::State<'_, AppState>,
    token: String,
) -> Result<bool, AppError> {
    let mut sessions = state.active_sessions.write().await;
    sessions.remove(&token);
    Ok(true)
}

#[tauri::command]
pub async fn change_admin_credentials(
    state: tauri::State<'_, AppState>,
    payload: ChangeCredentialsDto,
) -> Result<bool, AppError> {
    payload.validate().map_err(|e| AppError::Validation(e.to_string()))?;

    let row = sqlx::query_as::<_, UserDbRow>(
        "SELECT id, username, full_name, password_hash, quick_pin, role, last_login_at
         FROM provider_users
         ORDER BY id ASC LIMIT 1"
    )
    .fetch_optional(&state.pool)
    .await?;

    let user = match row {
        Some(u) => u,
        None => return Err(AppError::NotFound("Admin tidak ditemukan".to_string())),
    };

    // Jika ganti password atau pin, verifikasi password lama jika diset
    if let Some(old) = payload.old_password {
        if !verify_secret(old.trim(), &user.password_hash) {
            return Err(AppError::Unauthorized("Password lama tidak sesuai".to_string()));
        }
    }

    let now = Utc::now().to_rfc3339();

    if let Some(new_pass) = payload.new_password {
        let hash = hash_secret(new_pass.trim());
        let _ = sqlx::query("UPDATE provider_users SET password_hash = ?, updated_at = ? WHERE id = ?")
            .bind(hash)
            .bind(&now)
            .bind(&user.id)
            .execute(&state.pool)
            .await;
    }

    if let Some(new_pin) = payload.new_pin {
        let pin_hash = hash_secret(new_pin.trim());
        let _ = sqlx::query("UPDATE provider_users SET quick_pin = ?, updated_at = ? WHERE id = ?")
            .bind(pin_hash)
            .bind(&now)
            .bind(&user.id)
            .execute(&state.pool)
            .await;
    }

    Ok(true)
}
