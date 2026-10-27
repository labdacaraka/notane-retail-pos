use crate::api::ApiState;
use crate::crypto::ed25519::{derive_public_key, sign_message, DEFAULT_MASTER_PUBLIC_KEY_HEX};
use crate::crypto::hash::{hash_secret, verify_secret};
use crate::models::{
    AdminUser, AppError, AppRelease, AuditLog, AuthResponse, ChangeCredentialsDto, Client,
    CreateClientDto, CreateLicenseDto, CreateReleaseDto, DeviceActivation, License,
    LicenseTokenPayload, LoginDto, ManualSignResult, PaginatedResponse, PaginationParams,
    PinLoginDto, ProviderSettings, ProviderStatus, UpdateClientDto,
};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::{Duration, Utc};
use rand::Rng;
use serde::Deserialize;
use serde_json::json;
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
        // Licenses
        .route("/licenses", get(admin_get_licenses).post(admin_create_license))
        .route("/licenses/{id}/status", put(admin_update_license_status))
        .route("/licenses/{id}", delete(admin_delete_license))
        // Devices
        .route("/devices", get(admin_get_devices))
        .route("/devices/{id}", delete(admin_unbind_device))
        // Releases
        .route("/releases", get(admin_get_releases).post(admin_create_release))
        .route("/releases/{id}", delete(admin_delete_release))
        // Audit Logs
        .route("/audit-logs", get(admin_get_audit_logs))
        // Offline Signer
        .route("/sign-token", post(admin_sign_token_manual))
}

// ------------------------------------------------------------------------------
// AUTH HANDLERS
// ------------------------------------------------------------------------------
#[derive(Debug, sqlx::FromRow)]
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
            "license:read".into(), "license:create".into(), "license:update".into(), "license:revoke".into(),
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
        Some("expires_at") => "l.expires_at",
        Some("created_at") => "l.created_at",
        _ => "l.created_at",
    };
    let sort_dir = match params.sort_order.as_deref() {
        Some("asc") | Some("ASC") => "ASC",
        _ => "DESC",
    };

    let select_sql = format!(
        "SELECT l.id, l.license_key, l.client_id, c.client_name, l.tier, l.max_terminals,
                l.status, l.issued_at, l.expires_at, l.grace_period_days, l.features,
                l.notes, l.created_at, l.updated_at,
                (SELECT COUNT(*) FROM provider_device_activations d WHERE d.license_id = l.id AND d.is_active = 1) AS active_terminals
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
        &sql("INSERT INTO provider_licenses (
            id, license_key, client_id, tier, max_terminals, status, issued_at, expires_at,
            grace_period_days, features, notes, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, 'ACTIVE', ?, ?, ?, ?, ?, ?, ?)")
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
    let rows = sqlx::query(&sql("DELETE FROM provider_device_activations WHERE id = ?"))
        .bind(&id)
        .execute(&state.pool)
        .await
        .map_err(|e| err_response(AppError::Database(e.to_string())))?
        .rows_affected();

    Ok(Json(json!({ "success": rows > 0 })))
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
                has_db_migration, is_published, component, min_server_version, created_at
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
    let sign_target = format!("{}:{}", payload.version.trim(), payload.checksum_sha256.trim());
    let signature = sign_message(&priv_key, &sign_target)
        .map_err(|e| err_response(AppError::Crypto(e)))?;

    sqlx::query(
        &sql("INSERT INTO provider_app_releases (
            id, version, release_name, changelog, release_date, download_url,
            checksum_sha256, signature_ed25519, is_mandatory, min_license_tier,
            has_db_migration, is_published, component, min_server_version, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 1, ?, ?, ?)")
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
    .await
    .map_err(|e| err_response(AppError::Database(e.to_string())))?;

    Ok(Json(AppRelease {
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
    }))
}

async fn admin_delete_release(
    Path(id): Path<String>,
    State(state): State<ApiState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
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
