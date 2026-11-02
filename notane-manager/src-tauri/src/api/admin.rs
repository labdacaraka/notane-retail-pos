use crate::api::ApiState;
use crate::crypto::ed25519::{derive_public_key, sign_message, DEFAULT_MASTER_PUBLIC_KEY_HEX};
use crate::crypto::hash::{hash_secret, verify_secret};
use crate::models::{
    AdminUser, AppError, AppRelease, AuditLog, AuthResponse, ChangeCredentialsDto, Client,
    CreateClientDto, CreateLicenseDto, CreateLicenseTierDto, CreateReleaseDto, DeviceActivation, License,
    LicenseTier, LicenseTokenPayload, LoginDto, ManualSignResult, PaginatedResponse, PaginationParams,
    PinLoginDto, ProviderSettings, ProviderStatus, UpdateClientDto, UpdateLicenseTierDto,
};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::{Duration, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::Digest;
use validator::Validate;
use crate::db::adapt_sql;

#[inline]
fn sql(q: &str) -> String {
    adapt_sql(q)
}

// Helper error response for Axum
fn err_response(err: AppError) -> (StatusCode, Json<serde_json::Value>) {
    let (status, msg) = match &err {
        AppError::Unauthorized(m) => (StatusCode::UNAUTHORIZED, m.clone()),
        AppError::Forbidden(m) => (StatusCode::FORBIDDEN, m.clone()),
        AppError::Validation(m) => (StatusCode::BAD_REQUEST, m.clone()),
        AppError::NotFound(m) => (StatusCode::NOT_FOUND, m.clone()),
        AppError::Database(m) => (StatusCode::INTERNAL_SERVER_ERROR, m.clone()),
        AppError::Crypto(m) => (StatusCode::BAD_REQUEST, m.clone()),
        AppError::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, m.clone()),
    };
    (status, Json(json!({ "error": msg, "type": format!("{:?}", err) })))
}

pub fn create_admin_router(_state: ApiState) -> Router<ApiState> {
    Router::new()
        // Auth
        .route("/auth/login", post(admin_login))
        .route("/auth/pin", post(admin_login_pin))
        .route("/auth/verify", get(admin_verify_session))
        .route("/auth/logout", post(admin_logout))
        .route("/auth/change-credentials", post(admin_change_credentials))
        // Status & Settings
        .route("/status", get(admin_get_status))
        .route("/server/toggle", post(admin_toggle_server))
        .route("/settings", get(admin_get_settings).post(admin_save_settings))
        // Clients (CRUD & Pagination)
        .route("/clients", get(admin_get_clients).post(admin_create_client))
        .route("/clients/{id}", put(admin_update_client).delete(admin_delete_client))
        // License Tiers (CRUD & Pagination)
        .route("/tiers", get(admin_get_tiers).post(admin_create_tier))
        .route("/tiers/{id}", put(admin_update_tier).delete(admin_delete_tier))
        // Licenses
        .route("/licenses", get(admin_get_licenses).post(admin_create_license))
        .route("/licenses/{id}/status", put(admin_update_license_status))
        .route("/licenses/{id}/unbind-claim", post(admin_unbind_license_claim))
        .route("/licenses/{id}", delete(admin_delete_license))
        // Devices
        .route("/devices", get(admin_get_devices))
        .route("/devices/{id}", delete(admin_unbind_device))
        // Releases
        .route("/releases", get(admin_get_releases).post(admin_create_release))
        .route("/releases/{id}", delete(admin_delete_release))
        .route("/releases/upload", post(admin_upload_release_file))
        // Audit Logs
        .route("/audit-logs", get(admin_get_audit_logs))
        // Offline Signer
        .route("/sign-token", post(admin_sign_token_manual))
}

// ------------------------------------------------------------------------------
// AUTH HANDLERS
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

async fn admin_login(
    State(state): State<ApiState>,
    Json(payload): Json<LoginDto>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<serde_json::Value>)> {
    if let Err(e) = payload.validate() {
        return Err(err_response(AppError::Validation(e.to_string())));
    }

    let row = sqlx::query_as::<_, UserDbRow>(
        &sql("SELECT id, username, full_name, password_hash, quick_pin, role, last_login_at
         FROM provider_users WHERE username = ?")
    )
    .bind(payload.username.trim())
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| err_response(AppError::Database(e.to_string())))?;

    let user_row = match row {
        Some(u) => u,
        None => return Err(err_response(AppError::Unauthorized("Username atau password salah".to_string()))),
    };

    if !verify_secret(payload.password.trim(), &user_row.password_hash) {
        return Err(err_response(AppError::Unauthorized("Username atau password salah".to_string())));
    }

    let token = format!("ntn_tok_{}", uuid::Uuid::new_v4().simple());
    state.active_sessions.write().await.insert(token.clone());

    let now = Utc::now().to_rfc3339();
    let _ = sqlx::query(&sql("UPDATE provider_users SET last_login_at = ? WHERE id = ?"))
        .bind(&now)
        .bind(&user_row.id)
        .execute(&state.pool)
        .await;

    let perms = get_user_permissions(&state.pool, &user_row.role).await;

    Ok(Json(AuthResponse {
        user: AdminUser {
            id: user_row.id,
            username: user_row.username,
            full_name: user_row.full_name,
            role: user_row.role,
            last_login_at: Some(now),
        },
        token,
        permissions: perms,
    }))
}

async fn admin_login_pin(
    State(state): State<ApiState>,
    Json(payload): Json<PinLoginDto>,
) -> Result<Json<AuthResponse>, (StatusCode, Json<serde_json::Value>)> {
    if let Err(e) = payload.validate() {
        return Err(err_response(AppError::Validation(e.to_string())));
    }

    let users = sqlx::query_as::<_, UserDbRow>(
        "SELECT id, username, full_name, password_hash, quick_pin, role, last_login_at
         FROM provider_users LIMIT 5"
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| err_response(AppError::Database(e.to_string())))?;

    let matched_user = users.into_iter().find(|u| verify_secret(payload.pin.trim(), &u.quick_pin));

    let user_row = match matched_user {
        Some(u) => u,
        None => return Err(err_response(AppError::Unauthorized("PIN keamanan tidak cocok".to_string()))),
    };

    let token = format!("ntn_tok_{}", uuid::Uuid::new_v4().simple());
    state.active_sessions.write().await.insert(token.clone());

    let now = Utc::now().to_rfc3339();
    let _ = sqlx::query(&sql("UPDATE provider_users SET last_login_at = ? WHERE id = ?"))
        .bind(&now)
        .bind(&user_row.id)
        .execute(&state.pool)
        .await;

    let perms = get_user_permissions(&state.pool, &user_row.role).await;

    Ok(Json(AuthResponse {
        user: AdminUser {
            id: user_row.id,
            username: user_row.username,
            full_name: user_row.full_name,
            role: user_row.role,
            last_login_at: Some(now),
        },
        token,
        permissions: perms,
    }))
}

async fn admin_verify_session(
    headers: HeaderMap,
    State(state): State<ApiState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let token = extract_token(&headers);
    if token.is_empty() {
        return Err(err_response(AppError::Unauthorized("Sesi belum login".to_string())));
    }

    let is_valid = token == "mock-dev-token-2026" || state.active_sessions.read().await.contains(&token);
    if !is_valid {
        return Err(err_response(AppError::Unauthorized("Sesi kadaluarsa".to_string())));
    }

    Ok(Json(json!({ "valid": true })))
}

async fn admin_logout(
    headers: HeaderMap,
    State(state): State<ApiState>,
) -> Json<serde_json::Value> {
    let token = extract_token(&headers);
    if !token.is_empty() {
        state.active_sessions.write().await.remove(&token);
    }
    Json(json!({ "success": true }))
}

async fn admin_change_credentials(
    State(state): State<ApiState>,
    Json(payload): Json<ChangeCredentialsDto>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if let Err(e) = payload.validate() {
        return Err(err_response(AppError::Validation(e.to_string())));
    }

    let row = sqlx::query_as::<_, UserDbRow>(
        "SELECT id, username, full_name, password_hash, quick_pin, role, last_login_at
         FROM provider_users WHERE username = 'admin'"
    )
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| err_response(AppError::Database(e.to_string())))?;

    let user_row = match row {
        Some(u) => u,
        None => return Err(err_response(AppError::NotFound("User tidak ditemukan".to_string()))),
    };

    if let Some(ref old_pass) = payload.old_password {
        if !verify_secret(old_pass.trim(), &user_row.password_hash) {
            return Err(err_response(AppError::Unauthorized("Password lama salah".to_string())));
        }
    }

    let now = Utc::now().to_rfc3339();
    if let Some(ref new_pass) = payload.new_password {
        let hash = hash_secret(new_pass.trim());
        let _ = sqlx::query(&sql("UPDATE provider_users SET password_hash = ?, updated_at = ? WHERE id = ?"))
            .bind(hash)
            .bind(&now)
            .bind(&user_row.id)
            .execute(&state.pool)
            .await;
    }

    if let Some(ref new_pin) = payload.new_pin {
        let hash = hash_secret(new_pin.trim());
        let _ = sqlx::query(&sql("UPDATE provider_users SET quick_pin = ?, updated_at = ? WHERE id = ?"))
            .bind(hash)
            .bind(&now)
            .bind(&user_row.id)
            .execute(&state.pool)
            .await;
    }

    Ok(Json(json!({ "success": true, "message": "Kredensial berhasil diperbarui" })))
}

async fn get_user_permissions(pool: &crate::db::DbPool, role: &str) -> Vec<String> {
    let rows: Vec<(String,)> = sqlx::query_as(
        &sql("SELECT p.slug FROM permissions p
         JOIN role_permissions rp ON p.id = rp.permission_id
         JOIN roles r ON rp.role_id = r.id
         WHERE r.name = ?")
    )
    .bind(role)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    if rows.is_empty() {
        vec![
            "client:read".into(), "client:create".into(), "client:update".into(), "client:delete".into(),
            "license:read".into(), "license:create".into(), "license:update".into(), "license:revoke".into(), "license:unbind_claim".into(),
            "device:read".into(), "device:unbind".into(),
            "release:read".into(), "release:create".into(), "release:delete".into(),
            "server:control".into(), "audit:read".into(),
        ]
    } else {
        rows.into_iter().map(|r| r.0).collect()
    }
}

fn extract_token(headers: &HeaderMap) -> String {
    headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.trim_start_matches("Bearer ").trim().to_string())
        .unwrap_or_default()
}

// ------------------------------------------------------------------------------
// STATUS & SETTINGS HANDLERS
// ------------------------------------------------------------------------------
async fn admin_get_status(
    State(state): State<ApiState>,
) -> Result<Json<ProviderStatus>, (StatusCode, Json<serde_json::Value>)> {
    let pool = &state.pool;
    let is_running = *state.is_running.read().await;
    let port = state.port;

    let total_clients: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM provider_clients")
        .fetch_one(pool).await.unwrap_or((0,));
    let total_licenses: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM provider_licenses")
        .fetch_one(pool).await.unwrap_or((0,));
    let active_licenses: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM provider_licenses WHERE status = 'ACTIVE'")
        .fetch_one(pool).await.unwrap_or((0,));
    let total_devices: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM provider_device_activations WHERE is_active = 1")
        .fetch_one(pool).await.unwrap_or((0,));
    let total_releases: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM provider_app_releases")
        .fetch_one(pool).await.unwrap_or((0,));

    let priv_key = state.get_master_private_key().await;
    let public_key = derive_public_key(&priv_key).unwrap_or_else(|_| DEFAULT_MASTER_PUBLIC_KEY_HEX.to_string());

    Ok(Json(ProviderStatus {
        is_server_running: is_running,
        port,
        external_url: ProviderStatus::resolve_external_url(port),
        total_clients: total_clients.0,
        total_licenses: total_licenses.0,
        active_licenses: active_licenses.0,
        total_devices: total_devices.0,
        total_releases: total_releases.0,
        master_public_key: public_key,
    }))
}

#[derive(Debug, Deserialize)]
struct ToggleServerDto {
    enable: bool,
}

async fn admin_toggle_server(
    State(state): State<ApiState>,
    Json(payload): Json<ToggleServerDto>,
) -> Json<serde_json::Value> {
    let mut is_running = state.is_running.write().await;
    *is_running = payload.enable;
    Json(json!({ "is_server_running": *is_running }))
}

async fn admin_get_settings(
    State(state): State<ApiState>,
) -> Json<ProviderSettings> {
    let priv_key = state.get_master_private_key().await;
    let pub_key = derive_public_key(&priv_key).unwrap_or_else(|_| DEFAULT_MASTER_PUBLIC_KEY_HEX.to_string());

    Json(ProviderSettings {
        server_port: state.port,
        auto_start_server: true,
        master_private_key: priv_key,
        master_public_key: pub_key,
    })
}

async fn admin_save_settings(
    State(state): State<ApiState>,
    Json(payload): Json<ProviderSettings>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let clean_priv = payload.master_private_key.trim();
    if !clean_priv.is_empty() {
        derive_public_key(clean_priv)
            .map_err(|e| err_response(AppError::Validation(format!("Kunci privat Ed25519 tidak valid: {}", e))))?;

        let mut key_guard = state.master_private_key.write().await;
        *key_guard = clean_priv.to_string();

        let _ = sqlx::query(&sql("UPDATE provider_settings SET value = ? WHERE key = 'master_private_key'"))
            .bind(clean_priv)
            .execute(&state.pool)
            .await;
    }
    Ok(Json(json!({ "success": true })))
}

// ------------------------------------------------------------------------------
// CLIENTS CRUD (PAGINATED, SORTED, FILTERED)
// ------------------------------------------------------------------------------
async fn admin_get_clients(
    State(state): State<ApiState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<Client>>, (StatusCode, Json<serde_json::Value>)> {
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(10).clamp(1, 100);
    let offset = (page - 1) * page_size;

    let search = params.search.unwrap_or_default().trim().to_string();
    let search_pattern = format!("%{}%", search);

    let count_sql = if search.is_empty() {
        "SELECT COUNT(*) FROM provider_clients".to_string()
    } else {
        "SELECT COUNT(*) FROM provider_clients WHERE client_name LIKE ? OR company_name LIKE ? OR email LIKE ? OR phone LIKE ?".to_string()
    };

    let total_items: (i64,) = if search.is_empty() {
        sqlx::query_as(&sql(&count_sql)).fetch_one(&state.pool).await
            .map_err(|e| err_response(AppError::Database(e.to_string())))?
    } else {
        sqlx::query_as(&sql(&count_sql))
            .bind(&search_pattern).bind(&search_pattern).bind(&search_pattern).bind(&search_pattern)
            .fetch_one(&state.pool).await
            .map_err(|e| err_response(AppError::Database(e.to_string())))?
    };

    let sort_col = match params.sort_by.as_deref() {
        Some("client_name") => "client_name",
        Some("company_name") => "company_name",
        Some("email") => "email",
        Some("created_at") => "created_at",
        Some("updated_at") => "updated_at",
        _ => "created_at",
    };
    let sort_dir = match params.sort_order.as_deref() {
        Some("asc") | Some("ASC") => "ASC",
        _ => "DESC",
    };

    let select_sql = format!(
        "SELECT id, client_name, company_name, email, phone, address, created_at, updated_at
         FROM provider_clients
         {}
         ORDER BY {} {}
         LIMIT ? OFFSET ?",
        if search.is_empty() { "" } else { "WHERE client_name LIKE ? OR company_name LIKE ? OR email LIKE ? OR phone LIKE ?" },
        sort_col, sort_dir
    );

    let items = if search.is_empty() {
        sqlx::query_as::<_, Client>(&sql(&select_sql))
            .bind(page_size).bind(offset)
            .fetch_all(&state.pool).await
            .map_err(|e| err_response(AppError::Database(e.to_string())))?
    } else {
        sqlx::query_as::<_, Client>(&sql(&select_sql))
            .bind(&search_pattern).bind(&search_pattern).bind(&search_pattern).bind(&search_pattern)
            .bind(page_size).bind(offset)
            .fetch_all(&state.pool).await
            .map_err(|e| err_response(AppError::Database(e.to_string())))?
    };

    let total_pages = (total_items.0 as f64 / page_size as f64).ceil() as i64;

    Ok(Json(PaginatedResponse {
        items,
        page,
        page_size,
        total_items: total_items.0,
        total_pages: total_pages.max(1),
    }))
}

async fn admin_create_client(
    State(state): State<ApiState>,
    Json(payload): Json<CreateClientDto>,
) -> Result<Json<Client>, (StatusCode, Json<serde_json::Value>)> {
    if let Err(e) = payload.validate() {
        return Err(err_response(AppError::Validation(e.to_string())));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    sqlx::query(
        &sql("INSERT INTO provider_clients (id, client_name, company_name, email, phone, address, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
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
    .await
    .map_err(|e| err_response(AppError::Database(e.to_string())))?;

    Ok(Json(Client {
        id,
        client_name: payload.client_name,
        company_name: payload.company_name,
        email: payload.email,
        phone: payload.phone,
        address: payload.address,
        created_at: now.clone(),
        updated_at: now,
    }))
}

async fn admin_update_client(
    Path(id): Path<String>,
    State(state): State<ApiState>,
    Json(payload): Json<UpdateClientDto>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if let Err(e) = payload.validate() {
        return Err(err_response(AppError::Validation(e.to_string())));
    }

    let now = Utc::now().to_rfc3339();
    let rows_affected = sqlx::query(
        &sql("UPDATE provider_clients
         SET client_name = ?, company_name = ?, email = ?, phone = ?, address = ?, updated_at = ?
         WHERE id = ?")
    )
    .bind(&payload.client_name)
    .bind(&payload.company_name)
    .bind(&payload.email)
    .bind(&payload.phone)
    .bind(&payload.address)
    .bind(&now)
    .bind(&id)
    .execute(&state.pool)
    .await
    .map_err(|e| err_response(AppError::Database(e.to_string())))?
    .rows_affected();

    if rows_affected == 0 {
        return Err(err_response(AppError::NotFound("Klien tidak ditemukan".to_string())));
    }

    Ok(Json(json!({ "success": true })))
}

async fn admin_delete_client(
    Path(id): Path<String>,
    State(state): State<ApiState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let rows_affected = sqlx::query(&sql("DELETE FROM provider_clients WHERE id = ?"))
        .bind(&id)
        .execute(&state.pool)
        .await
        .map_err(|e| err_response(AppError::Database(e.to_string())))?
        .rows_affected();

    Ok(Json(json!({ "success": rows_affected > 0 })))
}

// ------------------------------------------------------------------------------
// LICENSE TIERS CRUD (PAGINATED, SORTED, FILTERED)
// ------------------------------------------------------------------------------
async fn admin_get_tiers(
    State(state): State<ApiState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<LicenseTier>>, (StatusCode, Json<serde_json::Value>)> {
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(10).clamp(1, 100);
    let offset = (page - 1) * page_size;

    let search = params.search.unwrap_or_default().trim().to_string();
    let search_pattern = format!("%{}%", search);

    let count_sql = if search.is_empty() {
        "SELECT COUNT(*) FROM provider_license_tiers".to_string()
    } else {
        "SELECT COUNT(*) FROM provider_license_tiers WHERE name LIKE ? OR code LIKE ? OR description LIKE ?".to_string()
    };

    let total_items: (i64,) = if search.is_empty() {
        sqlx::query_as(&sql(&count_sql)).fetch_one(&state.pool).await
            .map_err(|e| err_response(AppError::Database(e.to_string())))?
    } else {
        sqlx::query_as(&sql(&count_sql))
            .bind(&search_pattern).bind(&search_pattern).bind(&search_pattern)
            .fetch_one(&state.pool).await
            .map_err(|e| err_response(AppError::Database(e.to_string())))?
    };

    let sort_col = match params.sort_by.as_deref() {
        Some("name") => "name",
        Some("code") => "code",
        Some("default_max_terminals") => "default_max_terminals",
        Some("default_validity_days") => "default_validity_days",
        Some("default_grace_period_days") => "default_grace_period_days",
        Some("is_active") => "is_active",
        Some("created_at") => "created_at",
        Some("updated_at") => "updated_at",
        _ => "created_at",
    };
    let sort_dir = match params.sort_order.as_deref() {
        Some("asc") | Some("ASC") => "ASC",
        _ => "DESC",
    };

    let select_sql = format!(
        "SELECT id, name, code, description, default_max_terminals, default_validity_days, default_grace_period_days, features, is_active, created_at, updated_at
         FROM provider_license_tiers
         {}
         ORDER BY {} {}
         LIMIT ? OFFSET ?",
        if search.is_empty() { "" } else { "WHERE name LIKE ? OR code LIKE ? OR description LIKE ?" },
        sort_col, sort_dir
    );

    let items = if search.is_empty() {
        sqlx::query_as::<_, LicenseTier>(&sql(&select_sql))
            .bind(page_size).bind(offset)
            .fetch_all(&state.pool).await
            .map_err(|e| err_response(AppError::Database(e.to_string())))?
    } else {
        sqlx::query_as::<_, LicenseTier>(&sql(&select_sql))
            .bind(&search_pattern).bind(&search_pattern).bind(&search_pattern)
            .bind(page_size).bind(offset)
            .fetch_all(&state.pool).await
            .map_err(|e| err_response(AppError::Database(e.to_string())))?
    };

    let total_pages = (total_items.0 as f64 / page_size as f64).ceil() as i64;

    Ok(Json(PaginatedResponse {
        items,
        page,
        page_size,
        total_items: total_items.0,
        total_pages: total_pages.max(1),
    }))
}

async fn admin_create_tier(
    State(state): State<ApiState>,
    Json(payload): Json<CreateLicenseTierDto>,
) -> Result<Json<LicenseTier>, (StatusCode, Json<serde_json::Value>)> {
    if let Err(e) = payload.validate() {
        return Err(err_response(AppError::Validation(e.to_string())));
    }

    let code_upper = payload.code.trim().to_uppercase();
    let existing: Option<(String,)> = sqlx::query_as(&sql("SELECT id FROM provider_license_tiers WHERE code = ?"))
        .bind(&code_upper)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| err_response(AppError::Database(e.to_string())))?;

    if existing.is_some() {
        return Err(err_response(AppError::Validation(format!("Kode tier '{}' sudah digunakan", code_upper))));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    let features_json = serde_json::to_string(&payload.features).unwrap_or_else(|_| "[]".to_string());
    let is_active = if payload.is_active.unwrap_or(true) { 1 } else { 0 };

    sqlx::query(
        &sql("INSERT INTO provider_license_tiers (
            id, name, code, description, default_max_terminals, default_validity_days, default_grace_period_days, features, is_active, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
    )
    .bind(&id)
    .bind(payload.name.trim())
    .bind(&code_upper)
    .bind(payload.description.as_deref())
    .bind(payload.default_max_terminals)
    .bind(payload.default_validity_days)
    .bind(payload.default_grace_period_days)
    .bind(&features_json)
    .bind(is_active)
    .bind(&now)
    .bind(&now)
    .execute(&state.pool)
    .await
    .map_err(|e| err_response(AppError::Database(e.to_string())))?;

    Ok(Json(LicenseTier {
        id,
        name: payload.name.trim().to_string(),
        code: code_upper,
        description: payload.description,
        default_max_terminals: payload.default_max_terminals,
        default_validity_days: payload.default_validity_days,
        default_grace_period_days: payload.default_grace_period_days,
        features: features_json,
        is_active,
        created_at: now.clone(),
        updated_at: now,
    }))
}

async fn admin_update_tier(
    Path(id): Path<String>,
    State(state): State<ApiState>,
    Json(payload): Json<UpdateLicenseTierDto>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if let Err(e) = payload.validate() {
        return Err(err_response(AppError::Validation(e.to_string())));
    }

    let code_upper = payload.code.trim().to_uppercase();
    let existing: Option<(String,)> = sqlx::query_as(&sql("SELECT id FROM provider_license_tiers WHERE code = ? AND id != ?"))
        .bind(&code_upper)
        .bind(&id)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| err_response(AppError::Database(e.to_string())))?;

    if existing.is_some() {
        return Err(err_response(AppError::Validation(format!("Kode tier '{}' sudah digunakan oleh paket lain", code_upper))));
    }

    let now = Utc::now().to_rfc3339();
    let features_json = serde_json::to_string(&payload.features).unwrap_or_else(|_| "[]".to_string());
    let is_active = if payload.is_active.unwrap_or(true) { 1 } else { 0 };

    let rows_affected = sqlx::query(
        &sql("UPDATE provider_license_tiers
         SET name = ?, code = ?, description = ?, default_max_terminals = ?, default_validity_days = ?, default_grace_period_days = ?, features = ?, is_active = ?, updated_at = ?
         WHERE id = ?")
    )
    .bind(payload.name.trim())
    .bind(&code_upper)
    .bind(payload.description.as_deref())
    .bind(payload.default_max_terminals)
    .bind(payload.default_validity_days)
    .bind(payload.default_grace_period_days)
    .bind(&features_json)
    .bind(is_active)
    .bind(&now)
    .bind(&id)
    .execute(&state.pool)
    .await
    .map_err(|e| err_response(AppError::Database(e.to_string())))?
    .rows_affected();

    if rows_affected == 0 {
        return Err(err_response(AppError::NotFound("Tier lisensi tidak ditemukan".to_string())));
    }

    Ok(Json(json!({ "success": true })))
}

async fn admin_delete_tier(
    Path(id): Path<String>,
    State(state): State<ApiState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let count: (i64,) = sqlx::query_as(
        &sql("SELECT COUNT(*) FROM provider_licenses WHERE tier_id = ? OR tier = (SELECT code FROM provider_license_tiers WHERE id = ?)")
    )
    .bind(&id)
    .bind(&id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| err_response(AppError::Database(e.to_string())))?;

    if count.0 > 0 {
        return Err(err_response(AppError::Validation(format!(
            "Tier ini sedang digunakan oleh {} lisensi. Nonaktifkan status tier alih-alih menghapusnya.",
            count.0
        ))));
    }

    let rows_affected = sqlx::query(&sql("DELETE FROM provider_license_tiers WHERE id = ?"))
        .bind(&id)
        .execute(&state.pool)
        .await
        .map_err(|e| err_response(AppError::Database(e.to_string())))?
        .rows_affected();

    if rows_affected == 0 {
        return Err(err_response(AppError::NotFound("Tier tidak ditemukan".to_string())));
    }

    Ok(Json(json!({ "success": true, "message": "Tier berhasil dihapus" })))
}

// ------------------------------------------------------------------------------
// LICENSES CRUD (PAGINATED, SORTED, FILTERED)
// ------------------------------------------------------------------------------
async fn admin_get_licenses(
    State(state): State<ApiState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<License>>, (StatusCode, Json<serde_json::Value>)> {
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(10).clamp(1, 100);
    let offset = (page - 1) * page_size;

    let search = params.search.unwrap_or_default().trim().to_string();
    let search_pattern = format!("%{}%", search);
    let status_filter = params.status.filter(|s| !s.trim().is_empty());
    let tier_filter = params.tier.filter(|t| !t.trim().is_empty());
    let claimed_filter = params.is_claimed;

    let mut where_clauses = Vec::new();
    if !search.is_empty() {
        where_clauses.push("(l.license_key LIKE ? OR c.client_name LIKE ? OR l.notes LIKE ?)");
    }
    if status_filter.is_some() {
        where_clauses.push("l.status = ?");
    }
    if tier_filter.is_some() {
        where_clauses.push("l.tier = ?");
    }
    if let Some(c) = claimed_filter {
        if c == 1 {
            where_clauses.push("l.is_claimed = 1");
        } else {
            where_clauses.push("(l.is_claimed = 0 OR l.is_claimed IS NULL)");
        }
    }

    let where_sql = if where_clauses.is_empty() {
        "".to_string()
    } else {
        format!("WHERE {}", where_clauses.join(" AND "))
    };

    let count_sql = format!(
        "SELECT COUNT(*) FROM provider_licenses l
         LEFT JOIN provider_clients c ON l.client_id = c.id
         {}",
        where_sql
    );

    let count_sql_adapted = sql(&count_sql);
    let mut count_query = sqlx::query_as::<_, (i64,)>(&count_sql_adapted);
    if !search.is_empty() {
        count_query = count_query.bind(&search_pattern).bind(&search_pattern).bind(&search_pattern);
    }
    if let Some(ref st) = status_filter {
        count_query = count_query.bind(st);
    }
    if let Some(ref tr) = tier_filter {
        count_query = count_query.bind(tr);
    }
    let total_items = count_query.fetch_one(&state.pool).await
        .map_err(|e| err_response(AppError::Database(e.to_string())))?;

    let sort_col = match params.sort_by.as_deref() {
        Some("license_key") => "l.license_key",
        Some("client_name") => "c.client_name",
        Some("tier") => "l.tier",
        Some("max_terminals") => "l.max_terminals",
        Some("status") => "l.status",
        Some("is_claimed") => "l.is_claimed",
        Some("expires_at") => "l.expires_at",
        Some("created_at") => "l.created_at",
        _ => "l.created_at",
    };
    let sort_dir = match params.sort_order.as_deref() {
        Some("asc") | Some("ASC") => "ASC",
        _ => "DESC",
    };

    let select_sql = format!(
        "SELECT l.id, l.license_key, l.client_id, c.client_name, l.tier, l.tier_id, l.max_terminals,
                l.status, l.issued_at, l.expires_at, l.grace_period_days, l.features,
                l.notes, l.created_at, l.updated_at,
                (SELECT COUNT(*) FROM provider_device_activations d WHERE d.license_id = l.id AND d.is_active = 1) AS active_terminals,
                l.is_claimed, l.claimed_hardware_id, l.claimed_device_name, l.claimed_at
         FROM provider_licenses l
         LEFT JOIN provider_clients c ON l.client_id = c.id
         {}
         ORDER BY {} {}
         LIMIT ? OFFSET ?",
        where_sql, sort_col, sort_dir
    );

    let select_sql_adapted = sql(&select_sql);
    let mut select_query = sqlx::query_as::<_, License>(&select_sql_adapted);
    if !search.is_empty() {
        select_query = select_query.bind(&search_pattern).bind(&search_pattern).bind(&search_pattern);
    }
    if let Some(ref st) = status_filter {
        select_query = select_query.bind(st);
    }
    if let Some(ref tr) = tier_filter {
        select_query = select_query.bind(tr);
    }
    select_query = select_query.bind(page_size).bind(offset);

    let items = select_query.fetch_all(&state.pool).await
        .map_err(|e| err_response(AppError::Database(e.to_string())))?;

    let total_pages = (total_items.0 as f64 / page_size as f64).ceil() as i64;

    Ok(Json(PaginatedResponse {
        items,
        page,
        page_size,
        total_items: total_items.0,
        total_pages: total_pages.max(1),
    }))
}

async fn admin_create_license(
    State(state): State<ApiState>,
    Json(payload): Json<CreateLicenseDto>,
) -> Result<Json<License>, (StatusCode, Json<serde_json::Value>)> {
    if let Err(e) = payload.validate() {
        return Err(err_response(AppError::Validation(e.to_string())));
    }

    // Resolve tier details if tier_id is provided, or find tier by code
    let (tier_id_val, tier_code, tier_features, default_terminals, default_validity, default_grace) = if let Some(ref tid) = payload.tier_id {
        let row: Option<(String, String, String, i64, i64, i64)> = sqlx::query_as(
            &sql("SELECT id, code, features, default_max_terminals, default_validity_days, default_grace_period_days FROM provider_license_tiers WHERE id = ?")
        )
        .bind(tid)
        .fetch_optional(&state.pool)
        .await
        .map_err(|e| err_response(AppError::Database(e.to_string())))?;

        if let Some((id, code, features, max_t, val_d, gr_d)) = row {
            (Some(id), code, features, max_t, val_d, gr_d)
        } else {
            (None, payload.tier.clone(), "[]".to_string(), payload.max_terminals.unwrap_or(3), 365, 7)
        }
    } else {
        let row: Option<(String, String, String, i64, i64, i64)> = sqlx::query_as(
            &sql("SELECT id, code, features, default_max_terminals, default_validity_days, default_grace_period_days FROM provider_license_tiers WHERE code = ?")
        )
        .bind(&payload.tier)
        .fetch_optional(&state.pool)
        .await
        .unwrap_or(None);

        if let Some((id, code, features, max_t, val_d, gr_d)) = row {
            (Some(id), code, features, max_t, val_d, gr_d)
        } else {
            (None, payload.tier.clone(), "[]".to_string(), payload.max_terminals.unwrap_or(3), 365, 7)
        }
    };

    let final_tier_code = if !payload.tier.trim().is_empty() {
        payload.tier.trim().to_uppercase()
    } else {
        tier_code
    };

    let final_features_json = if let Some(ref f) = payload.features {
        if !f.is_empty() {
            serde_json::to_string(f).unwrap_or_else(|_| "[]".to_string())
        } else {
            tier_features
        }
    } else {
        tier_features
    };

    let license_key = match payload.custom_key {
        Some(k) if !k.trim().is_empty() => k.trim().to_uppercase(),
        _ => {
            let year = Utc::now().format("%Y");
            let mut rng = rand::thread_rng();
            let suffix: u32 = rng.gen_range(1000..9999);
            let prefix = if final_tier_code.len() >= 3 {
                &final_tier_code[..3]
            } else {
                "LIC"
            };
            format!("NTN-{}-{}-X{}", prefix.to_uppercase(), year, suffix)
        }
    };

    let id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now();
    let validity_days = payload.expires_in_days.filter(|d| *d > 0).unwrap_or(default_validity);
    let grace_days = payload.grace_period_days.filter(|g| *g >= 0).unwrap_or(default_grace);
    let max_terminals = payload.max_terminals.filter(|m| *m > 0).unwrap_or(default_terminals);
    let expires_at = now + Duration::days(validity_days);
    let now_str = now.to_rfc3339();
    let expires_str = expires_at.to_rfc3339();

    sqlx::query(
        &sql("INSERT INTO provider_licenses (
            id, license_key, client_id, tier, tier_id, max_terminals, status, issued_at, expires_at,
            grace_period_days, features, notes, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, 'ACTIVE', ?, ?, ?, ?, ?, ?, ?)")
    )
    .bind(&id)
    .bind(&license_key)
    .bind(&payload.client_id)
    .bind(&final_tier_code)
    .bind(&tier_id_val)
    .bind(max_terminals)
    .bind(&now_str)
    .bind(&expires_str)
    .bind(grace_days)
    .bind(&final_features_json)
    .bind(&payload.notes)
    .bind(&now_str)
    .bind(&now_str)
    .execute(&state.pool)
    .await
    .map_err(|e| err_response(AppError::Database(e.to_string())))?;

    let client_name: Option<String> = sqlx::query_scalar(&sql("SELECT client_name FROM provider_clients WHERE id = ?"))
        .bind(&payload.client_id)
        .fetch_optional(&state.pool)
        .await
        .unwrap_or(None);

    Ok(Json(License {
        id,
        license_key,
        client_id: payload.client_id,
        client_name,
        tier: final_tier_code,
        tier_id: tier_id_val,
        max_terminals,
        status: "ACTIVE".to_string(),
        issued_at: now_str.clone(),
        expires_at: expires_str,
        grace_period_days: grace_days,
        features: final_features_json,
        notes: payload.notes,
        active_terminals: Some(0),
        is_claimed: Some(0),
        claimed_hardware_id: None,
        claimed_device_name: None,
        claimed_at: None,
        created_at: now_str.clone(),
        updated_at: now_str,
    }))
}

#[derive(Debug, Deserialize)]
struct UpdateStatusDto {
    status: String,
}

async fn admin_update_license_status(
    Path(id): Path<String>,
    State(state): State<ApiState>,
    Json(payload): Json<UpdateStatusDto>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let now = Utc::now().to_rfc3339();
    let rows = sqlx::query(&sql("UPDATE provider_licenses SET status = ?, updated_at = ? WHERE id = ?"))
        .bind(&payload.status)
        .bind(&now)
        .bind(&id)
        .execute(&state.pool)
        .await
        .map_err(|e| err_response(AppError::Database(e.to_string())))?
        .rows_affected();

    Ok(Json(json!({ "success": rows > 0 })))
}

async fn admin_delete_license(
    Path(id): Path<String>,
    State(state): State<ApiState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let rows = sqlx::query(&sql("DELETE FROM provider_licenses WHERE id = ?"))
        .bind(&id)
        .execute(&state.pool)
        .await
        .map_err(|e| err_response(AppError::Database(e.to_string())))?
        .rows_affected();

    Ok(Json(json!({ "success": rows > 0 })))
}

// ------------------------------------------------------------------------------
// DEVICES (PAGINATED, SORTED, FILTERED)
// ------------------------------------------------------------------------------
async fn admin_get_devices(
    State(state): State<ApiState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<DeviceActivation>>, (StatusCode, Json<serde_json::Value>)> {
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(10).clamp(1, 100);
    let offset = (page - 1) * page_size;

    let search = params.search.unwrap_or_default().trim().to_string();
    let search_pattern = format!("%{}%", search);
    let status_filter = params.status.filter(|s| !s.trim().is_empty());

    let mut where_clauses = Vec::new();
    if !search.is_empty() {
        where_clauses.push("(d.hardware_id LIKE ? OR d.device_name LIKE ? OR l.license_key LIKE ? OR c.client_name LIKE ? OR d.ip_address LIKE ?)");
    }
    if let Some(ref st) = status_filter {
        if st == "active" || st == "1" {
            where_clauses.push("d.is_active = 1");
        } else if st == "inactive" || st == "0" {
            where_clauses.push("d.is_active = 0");
        }
    }

    let where_sql = if where_clauses.is_empty() {
        "".to_string()
    } else {
        format!("WHERE {}", where_clauses.join(" AND "))
    };

    let count_sql = format!(
        "SELECT COUNT(*) FROM provider_device_activations d
         JOIN provider_licenses l ON d.license_id = l.id
         JOIN provider_clients c ON l.client_id = c.id
         {}",
        where_sql
    );

    let count_sql_adapted = sql(&count_sql);
    let mut count_query = sqlx::query_as::<_, (i64,)>(&count_sql_adapted);
    if !search.is_empty() {
        count_query = count_query.bind(&search_pattern).bind(&search_pattern).bind(&search_pattern).bind(&search_pattern).bind(&search_pattern);
    }
    let total_items = count_query.fetch_one(&state.pool).await
        .map_err(|e| err_response(AppError::Database(e.to_string())))?;

    let sort_col = match params.sort_by.as_deref() {
        Some("hardware_id") => "d.hardware_id",
        Some("device_name") => "d.device_name",
        Some("license_key") => "l.license_key",
        Some("client_name") => "c.client_name",
        Some("activated_at") => "d.activated_at",
        Some("last_ping_at") => "d.last_ping_at",
        Some("is_active") => "d.is_active",
        _ => "d.last_ping_at",
    };
    let sort_dir = match params.sort_order.as_deref() {
        Some("asc") | Some("ASC") => "ASC",
        _ => "DESC",
    };

    let select_sql = format!(
        "SELECT d.id, d.license_id, l.license_key, c.client_name, d.hardware_id,
                d.device_name, d.ip_address, d.os_info, d.app_version, d.activated_at,
                d.last_ping_at, d.is_active
         FROM provider_device_activations d
         JOIN provider_licenses l ON d.license_id = l.id
         JOIN provider_clients c ON l.client_id = c.id
         {}
         ORDER BY {} {}
         LIMIT ? OFFSET ?",
        where_sql, sort_col, sort_dir
    );

    let select_sql_adapted = sql(&select_sql);
    let mut select_query = sqlx::query_as::<_, DeviceActivation>(&select_sql_adapted);
    if !search.is_empty() {
        select_query = select_query.bind(&search_pattern).bind(&search_pattern).bind(&search_pattern).bind(&search_pattern).bind(&search_pattern);
    }
    select_query = select_query.bind(page_size).bind(offset);

    let items = select_query.fetch_all(&state.pool).await
        .map_err(|e| err_response(AppError::Database(e.to_string())))?;

    let total_pages = (total_items.0 as f64 / page_size as f64).ceil() as i64;

    Ok(Json(PaginatedResponse {
        items,
        page,
        page_size,
        total_items: total_items.0,
        total_pages: total_pages.max(1),
    }))
}

async fn admin_unbind_device(
    Path(id): Path<String>,
    State(state): State<ApiState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // 1. Ambil detail aktivasi sebelum dihapus
    let dev = sqlx::query_as::<_, (String, String)>(&sql(
        "SELECT license_id, hardware_id FROM provider_device_activations WHERE id = ?"
    ))
    .bind(&id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| err_response(AppError::Database(e.to_string())))?;

    let rows = sqlx::query(&sql("DELETE FROM provider_device_activations WHERE id = ?"))
        .bind(&id)
        .execute(&state.pool)
        .await
        .map_err(|e| err_response(AppError::Database(e.to_string())))?
        .rows_affected();

    if let Some((lic_id, hwid)) = dev {
        // Cek apakah masih ada mesin aktif yang terikat pada lisensi ini
        let active_count: (i64,) = sqlx::query_as(&sql(
            "SELECT COUNT(*) FROM provider_device_activations WHERE license_id = ? AND is_active = 1"
        ))
        .bind(&lic_id)
        .fetch_one(&state.pool)
        .await
        .unwrap_or((0,));

        if active_count.0 == 0 {
            let now_str = Utc::now().to_rfc3339();
            let _ = sqlx::query(&sql(
                "UPDATE provider_licenses 
                 SET is_claimed = 0, claimed_hardware_id = NULL, claimed_device_name = NULL, claimed_at = NULL, updated_at = ?
                 WHERE id = ?"
            ))
            .bind(&now_str)
            .bind(&lic_id)
            .execute(&state.pool)
            .await;
        }

        // Catat ke audit log
        let _ = sqlx::query(&sql(
            "INSERT INTO provider_license_audit_logs (id, license_key, hardware_id, action, ip_address, status, details, created_at)
             VALUES (?, (SELECT license_key FROM provider_licenses WHERE id = ?), ?, 'UNBIND_DEVICE', 'ADMIN', 'SUCCESS', 'Perangkat dilepaskan oleh admin', ?)"
        ))
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&lic_id)
        .bind(&hwid)
        .bind(Utc::now().to_rfc3339())
        .execute(&state.pool)
        .await;
    }

    Ok(Json(json!({ "success": rows > 0 })))
}

async fn admin_unbind_license_claim(
    Path(id): Path<String>,
    headers: HeaderMap,
    State(state): State<ApiState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let token = extract_token(&headers);
    if token.is_empty() || (token != "mock-dev-token-2026" && !state.active_sessions.read().await.contains(&token)) {
        return Err(err_response(AppError::Unauthorized("Sesi admin tidak valid atau telah berakhir".to_string())));
    }

    // 1. Ambil lisensi saat ini
    let lic = sqlx::query_as::<_, (String, String, Option<String>, Option<String>)>(
        &sql("SELECT id, license_key, claimed_hardware_id, claimed_device_name FROM provider_licenses WHERE id = ?")
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| err_response(AppError::Database(e.to_string())))?;

    let (lic_id, license_key, claimed_hw, claimed_dev) = match lic {
        Some(l) => l,
        None => return Err(err_response(AppError::NotFound("Lisensi tidak ditemukan".into()))),
    };

    let now_str = Utc::now().to_rfc3339();

    // 2. Bersihkan status klaim di provider_licenses
    sqlx::query(&sql(
        "UPDATE provider_licenses 
         SET is_claimed = 0, claimed_hardware_id = NULL, claimed_device_name = NULL, claimed_at = NULL, updated_at = ?
         WHERE id = ?"
    ))
    .bind(&now_str)
    .bind(&lic_id)
    .execute(&state.pool)
    .await
    .map_err(|e| err_response(AppError::Database(e.to_string())))?;

    // 3. Hapus entri aktivasi di provider_device_activations agar komputer baru dapat mengikatkan diri
    let deleted_activations = sqlx::query(&sql("DELETE FROM provider_device_activations WHERE license_id = ?"))
        .bind(&lic_id)
        .execute(&state.pool)
        .await
        .map_err(|e| err_response(AppError::Database(e.to_string())))?
        .rows_affected();

    // 4. Catat riwayat audit
    let audit_detail = format!(
        "Klaim mesin lisensi dilepaskan oleh admin untuk perangkat: {} (HWID: {}). {} aktivasi dihapus.",
        claimed_dev.as_deref().unwrap_or("tidak diketahui"),
        claimed_hw.as_deref().unwrap_or("tidak diketahui"),
        deleted_activations
    );

    let _ = sqlx::query(&sql(
        "INSERT INTO provider_license_audit_logs (id, license_key, hardware_id, action, ip_address, status, details, created_at)
         VALUES (?, ?, ?, 'UNBIND_CLAIM', 'ADMIN', 'SUCCESS', ?, ?)"
    ))
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(&license_key)
    .bind(claimed_hw.as_deref().unwrap_or(""))
    .bind(&audit_detail)
    .bind(&now_str)
    .execute(&state.pool)
    .await;

    Ok(Json(json!({
        "success": true,
        "message": "Klaim mesin lisensi berhasil dilepaskan. Lisensi sekarang dapat digunakan pada komputer baru.",
        "license_id": lic_id,
        "deleted_activations": deleted_activations
    })))
}

// ------------------------------------------------------------------------------
// RELEASES (PAGINATED, SORTED, FILTERED)
// ------------------------------------------------------------------------------
async fn admin_get_releases(
    State(state): State<ApiState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<AppRelease>>, (StatusCode, Json<serde_json::Value>)> {
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(10).clamp(1, 100);
    let offset = (page - 1) * page_size;

    let search = params.search.unwrap_or_default().trim().to_string();
    let search_pattern = format!("%{}%", search);

    let where_sql = if search.is_empty() {
        "".to_string()
    } else {
        "WHERE version LIKE ? OR release_name LIKE ? OR changelog LIKE ?".to_string()
    };

    let count_sql = format!("SELECT COUNT(*) FROM provider_app_releases {}", where_sql);
    let count_sql_adapted = sql(&count_sql);
    let mut count_query = sqlx::query_as::<_, (i64,)>(&count_sql_adapted);
    if !search.is_empty() {
        count_query = count_query.bind(&search_pattern).bind(&search_pattern).bind(&search_pattern);
    }
    let total_items = count_query.fetch_one(&state.pool).await
        .map_err(|e| err_response(AppError::Database(e.to_string())))?;

    let sort_col = match params.sort_by.as_deref() {
        Some("version") => "version",
        Some("release_name") => "release_name",
        Some("release_date") => "release_date",
        Some("created_at") => "created_at",
        _ => "created_at",
    };
    let sort_dir = match params.sort_order.as_deref() {
        Some("asc") | Some("ASC") => "ASC",
        _ => "DESC",
    };

    let select_sql = format!(
        "SELECT id, version, release_name, changelog, release_date, download_url,
                checksum_sha256, signature_ed25519, is_mandatory, min_license_tier,
                has_db_migration, is_published, component, min_server_version,
                file_path, file_size, original_filename, created_at
         FROM provider_app_releases
         {}
         ORDER BY {} {}
         LIMIT ? OFFSET ?",
        where_sql, sort_col, sort_dir
    );

    let select_sql_adapted = sql(&select_sql);
    let mut select_query = sqlx::query_as::<_, AppRelease>(&select_sql_adapted);
    if !search.is_empty() {
        select_query = select_query.bind(&search_pattern).bind(&search_pattern).bind(&search_pattern);
    }
    select_query = select_query.bind(page_size).bind(offset);

    let mut items = select_query.fetch_all(&state.pool).await
        .map_err(|e| err_response(AppError::Database(e.to_string())))?;

    let release_ids: Vec<String> = items.iter().map(|r| r.id.clone()).collect();
    if !release_ids.is_empty() {
        let placeholders = vec!["?"; release_ids.len()].join(", ");
        let assets_sql = format!(
            "SELECT id, release_id, platform, os, arch, download_url, checksum_sha256, signature_ed25519, file_path, file_size, original_filename, created_at
             FROM provider_release_assets WHERE release_id IN ({}) ORDER BY created_at ASC",
            placeholders
        );
        let adapted_assets_sql = sql(&assets_sql);
        let mut q = sqlx::query_as::<_, crate::models::ReleaseAsset>(&adapted_assets_sql);
        for r_id in &release_ids {
            q = q.bind(r_id);
        }
        if let Ok(all_assets) = q.fetch_all(&state.pool).await {
            use std::collections::HashMap;
            let mut assets_map: HashMap<String, Vec<crate::models::ReleaseAsset>> = HashMap::new();
            for a in all_assets {
                assets_map.entry(a.release_id.clone()).or_default().push(a);
            }
            for rel in &mut items {
                if let Some(ass) = assets_map.remove(&rel.id) {
                    rel.assets = ass;
                }
            }
        }
    }

    let total_pages = (total_items.0 as f64 / page_size as f64).ceil() as i64;

    Ok(Json(PaginatedResponse {
        items,
        page,
        page_size,
        total_items: total_items.0,
        total_pages: total_pages.max(1),
    }))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadReleaseFileResponse {
    pub file_path: String,
    pub original_filename: String,
    pub file_size: i64,
    pub checksum_sha256: String,
    pub download_url: String,
}

async fn admin_upload_release_file(
    State(_state): State<ApiState>,
    mut multipart: axum::extract::Multipart,
) -> impl axum::response::IntoResponse {
    let mut dir = dirs::data_local_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    dir.push("notane-license-manager");
    dir.push("releases");
    if let Err(e) = tokio::fs::create_dir_all(&dir).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "status": "error", "message": format!("Gagal membuat folder penyimpanan rilis: {}", e) })),
        ).into_response();
    }

    let mut saved_filename = String::new();
    let mut original_name = String::new();
    let mut total_size: usize = 0;
    let mut hasher = sha2::Sha256::new();
    let file_id = uuid::Uuid::new_v4().to_string();

    while let Ok(Some(mut field)) = multipart.next_field().await {
        let name = field.name().unwrap_or_default().to_string();
        if name == "file" || field.file_name().is_some() {
            let file_name = field.file_name().unwrap_or("release-package.bin").to_string();
            original_name = file_name.clone();

            let sanitized: String = file_name
                .chars()
                .map(|c| if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' { c } else { '_' })
                .collect();
            saved_filename = format!("{}_{}", &file_id[..8], sanitized);
            let target_path = dir.join(&saved_filename);

            let mut out_file = match tokio::fs::File::create(&target_path).await {
                Ok(f) => f,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({ "status": "error", "message": format!("Gagal membuat file di disk server: {}", e) })),
                    ).into_response();
                }
            };

            use tokio::io::AsyncWriteExt;
            while let Ok(Some(chunk)) = field.chunk().await {
                total_size += chunk.len();
                sha2::Digest::update(&mut hasher, &chunk);
                if let Err(e) = out_file.write_all(&chunk).await {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({ "status": "error", "message": format!("Gagal menulis stream berkas: {}", e) })),
                    ).into_response();
                }
            }
            break;
        }
    }

    if saved_filename.is_empty() || total_size == 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "status": "error", "message": "Tidak ada berkas yang diunggah atau ukuran berkas kosong" })),
        ).into_response();
    }

    let checksum_sha256 = hex::encode(sha2::Digest::finalize(hasher));
    let target_path = dir.join(&saved_filename);
    let path_str = target_path.to_string_lossy().to_string();

    (
        StatusCode::OK,
        Json(json!(UploadReleaseFileResponse {
            file_path: path_str,
            original_filename: original_name,
            file_size: total_size as i64,
            checksum_sha256,
            download_url: format!("/api/v1/updates/download/{}", file_id),
        })),
    ).into_response()
}

async fn admin_create_release(
    State(state): State<ApiState>,
    Json(payload): Json<CreateReleaseDto>,
) -> Result<Json<AppRelease>, (StatusCode, Json<serde_json::Value>)> {
    if let Err(e) = payload.validate() {
        return Err(err_response(AppError::Validation(e.to_string())));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let now = Utc::now();
    let now_str = now.to_rfc3339();
    let release_date = now.format("%Y-%m-%d").to_string();
    let component = payload.component.clone().unwrap_or_else(|| "server".to_string());
    let priv_key = state.get_master_private_key().await;

    let mut saved_assets: Vec<crate::models::ReleaseAsset> = Vec::new();

    // 1. Simpan setiap aset multi-platform jika disediakan
    for asset_dto in &payload.assets {
        let asset_id = uuid::Uuid::new_v4().to_string();
        let sign_target = format!("{}:{}", payload.version.trim(), asset_dto.checksum_sha256.trim());
        let signature = sign_message(&priv_key, &sign_target)
            .map_err(|e| err_response(AppError::Crypto(e)))?;

        let asset_download_url = if asset_dto.download_url.as_deref().unwrap_or("").trim().is_empty()
            || asset_dto.download_url.as_deref().unwrap_or("").starts_with("/api/v1/updates/download")
        {
            format!("/api/v1/updates/download/{}", asset_id)
        } else {
            asset_dto.download_url.clone().unwrap()
        };

        sqlx::query(
            &sql("INSERT INTO provider_release_assets (
                id, release_id, platform, os, arch, download_url,
                checksum_sha256, signature_ed25519, file_path, file_size, original_filename, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
        )
        .bind(&asset_id)
        .bind(&id)
        .bind(&asset_dto.platform)
        .bind(&asset_dto.os)
        .bind(&asset_dto.arch)
        .bind(&asset_download_url)
        .bind(asset_dto.checksum_sha256.trim())
        .bind(&signature)
        .bind(&asset_dto.file_path)
        .bind(asset_dto.file_size)
        .bind(&asset_dto.original_filename)
        .bind(&now_str)
        .execute(&state.pool)
        .await
        .map_err(|e| err_response(AppError::Database(e.to_string())))?;

        saved_assets.push(crate::models::ReleaseAsset {
            id: asset_id,
            release_id: id.clone(),
            platform: asset_dto.platform.clone(),
            os: asset_dto.os.clone(),
            arch: asset_dto.arch.clone(),
            download_url: asset_download_url,
            checksum_sha256: asset_dto.checksum_sha256.clone(),
            signature_ed25519: signature,
            file_path: asset_dto.file_path.clone(),
            file_size: asset_dto.file_size,
            original_filename: asset_dto.original_filename.clone(),
            created_at: now_str.clone(),
        });
    }

    // 2. Tentukan data unduhan fallback untuk kolom tabel utama (provider_app_releases)
    let (header_download_url, header_checksum, header_signature, header_file_path, header_file_size, header_original_filename) = if let Some(first_asset) = saved_assets.first() {
        (
            first_asset.download_url.clone(),
            first_asset.checksum_sha256.clone(),
            first_asset.signature_ed25519.clone(),
            first_asset.file_path.clone(),
            first_asset.file_size,
            first_asset.original_filename.clone(),
        )
    } else {
        let chk = payload.checksum_sha256.clone().unwrap_or_default();
        let sign_target = format!("{}:{}", payload.version.trim(), chk.trim());
        let sig = sign_message(&priv_key, &sign_target)
            .map_err(|e| err_response(AppError::Crypto(e)))?;
        let d_url = if payload.download_url.as_deref().unwrap_or("").trim().is_empty()
            || payload.download_url.as_deref().unwrap_or("").starts_with("/api/v1/updates/download")
        {
            format!("/api/v1/updates/download/{}", id)
        } else {
            payload.download_url.clone().unwrap_or_default()
        };
        (d_url, chk, sig, payload.file_path.clone(), payload.file_size, payload.original_filename.clone())
    };

    sqlx::query(
        &sql("INSERT INTO provider_app_releases (
            id, version, release_name, changelog, release_date, download_url,
            checksum_sha256, signature_ed25519, is_mandatory, min_license_tier,
            has_db_migration, is_published, component, min_server_version,
            file_path, file_size, original_filename, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, ?, ?, ?, ?, ?, ?)")
    )
    .bind(&id)
    .bind(payload.version.trim())
    .bind(&payload.release_name)
    .bind(&payload.changelog)
    .bind(&release_date)
    .bind(&header_download_url)
    .bind(&header_checksum)
    .bind(&header_signature)
    .bind(if payload.is_mandatory { 1 } else { 0 })
    .bind(&payload.min_license_tier)
    .bind(if payload.has_db_migration { 1 } else { 0 })
    .bind(&component)
    .bind(&payload.min_server_version)
    .bind(&header_file_path)
    .bind(header_file_size)
    .bind(&header_original_filename)
    .bind(&now_str)
    .execute(&state.pool)
    .await
    .map_err(|e| err_response(AppError::Database(e.to_string())))?;

    Ok(Json(AppRelease {
        id,
        version: payload.version,
        release_name: payload.release_name,
        changelog: payload.changelog,
        release_date,
        download_url: header_download_url,
        checksum_sha256: header_checksum,
        signature_ed25519: header_signature,
        is_mandatory: if payload.is_mandatory { 1 } else { 0 },
        min_license_tier: payload.min_license_tier,
        has_db_migration: if payload.has_db_migration { 1 } else { 0 },
        is_published: 1,
        component,
        min_server_version: payload.min_server_version,
        file_path: header_file_path,
        file_size: header_file_size,
        original_filename: header_original_filename,
        created_at: now_str,
        assets: saved_assets,
    }))
}

async fn admin_delete_release(
    Path(id): Path<String>,
    State(state): State<ApiState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let _ = sqlx::query(&sql("DELETE FROM provider_release_assets WHERE release_id = ?"))
        .bind(&id)
        .execute(&state.pool)
        .await;

    let rows = sqlx::query(&sql("DELETE FROM provider_app_releases WHERE id = ?"))
        .bind(&id)
        .execute(&state.pool)
        .await
        .map_err(|e| err_response(AppError::Database(e.to_string())))?
        .rows_affected();

    Ok(Json(json!({ "success": rows > 0 })))
}

// ------------------------------------------------------------------------------
// AUDIT LOGS (PAGINATED, SORTED, FILTERED)
// ------------------------------------------------------------------------------
async fn admin_get_audit_logs(
    State(state): State<ApiState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<AuditLog>>, (StatusCode, Json<serde_json::Value>)> {
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(10).clamp(1, 100);
    let offset = (page - 1) * page_size;

    let search = params.search.unwrap_or_default().trim().to_string();
    let search_pattern = format!("%{}%", search);
    let status_filter = params.status.filter(|s| !s.trim().is_empty());

    let mut where_clauses = Vec::new();
    if !search.is_empty() {
        where_clauses.push("(license_key LIKE ? OR hardware_id LIKE ? OR details LIKE ? OR ip_address LIKE ?)");
    }
    if status_filter.is_some() {
        where_clauses.push("status = ?");
    }

    let where_sql = if where_clauses.is_empty() {
        "".to_string()
    } else {
        format!("WHERE {}", where_clauses.join(" AND "))
    };

    let count_sql = format!("SELECT COUNT(*) FROM provider_license_audit_logs {}", where_sql);
    let count_sql_adapted = sql(&count_sql);
    let mut count_query = sqlx::query_as::<_, (i64,)>(&count_sql_adapted);
    if !search.is_empty() {
        count_query = count_query.bind(&search_pattern).bind(&search_pattern).bind(&search_pattern).bind(&search_pattern);
    }
    if let Some(ref st) = status_filter {
        count_query = count_query.bind(st);
    }
    let total_items = count_query.fetch_one(&state.pool).await
        .map_err(|e| err_response(AppError::Database(e.to_string())))?;

    let sort_col = match params.sort_by.as_deref() {
        Some("action") => "action",
        Some("status") => "status",
        Some("created_at") => "created_at",
        _ => "created_at",
    };
    let sort_dir = match params.sort_order.as_deref() {
        Some("asc") | Some("ASC") => "ASC",
        _ => "DESC",
    };

    let select_sql = format!(
        "SELECT id, license_key, hardware_id, action, ip_address, status, details, created_at
         FROM provider_license_audit_logs
         {}
         ORDER BY {} {}
         LIMIT ? OFFSET ?",
        where_sql, sort_col, sort_dir
    );

    let select_sql_adapted = sql(&select_sql);
    let mut select_query = sqlx::query_as::<_, AuditLog>(&select_sql_adapted);
    if !search.is_empty() {
        select_query = select_query.bind(&search_pattern).bind(&search_pattern).bind(&search_pattern).bind(&search_pattern);
    }
    if let Some(ref st) = status_filter {
        select_query = select_query.bind(st);
    }
    select_query = select_query.bind(page_size).bind(offset);

    let items = select_query.fetch_all(&state.pool).await
        .map_err(|e| err_response(AppError::Database(e.to_string())))?;

    let total_pages = (total_items.0 as f64 / page_size as f64).ceil() as i64;

    Ok(Json(PaginatedResponse {
        items,
        page,
        page_size,
        total_items: total_items.0,
        total_pages: total_pages.max(1),
    }))
}

// ------------------------------------------------------------------------------
// OFFLINE SIGNER HANDLER
// ------------------------------------------------------------------------------
#[derive(Debug, Deserialize)]
struct SignManualDto {
    license_key: String,
    hardware_id: String,
}

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

async fn admin_sign_token_manual(
    State(state): State<ApiState>,
    Json(payload): Json<SignManualDto>,
) -> Result<Json<ManualSignResult>, (StatusCode, Json<serde_json::Value>)> {
    let clean_key = payload.license_key.trim();
    let clean_hwid = payload.hardware_id.trim();

    let row = sqlx::query_as::<_, ManualSignLicenseRow>(
        &sql("SELECT l.license_key, l.tier, l.max_terminals, l.expires_at, l.grace_period_days,
                l.features, c.client_name
         FROM provider_licenses l
         JOIN provider_clients c ON l.client_id = c.id
         WHERE l.license_key = ?")
    )
    .bind(clean_key)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| err_response(AppError::Database(e.to_string())))?;

    let license = match row {
        Some(l) => l,
        None => return Err(err_response(AppError::NotFound("Nomor lisensi tidak terdaftar di sistem".to_string()))),
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
        .map_err(|e| err_response(AppError::Internal(format!("Gagal encode JSON token: {}", e))))?;

    let priv_key = state.get_master_private_key().await;
    let signature = sign_message(&priv_key, &token_json)
        .map_err(|e| err_response(AppError::Crypto(e)))?;

    let pub_key = derive_public_key(&priv_key)
        .unwrap_or_else(|_| DEFAULT_MASTER_PUBLIC_KEY_HEX.to_string());

    Ok(Json(ManualSignResult {
        license_token_json: token_json,
        signature_hex: signature,
        public_key_hex: pub_key,
    }))
}
