use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use sqlx::{PgPool, Row};
use uuid::Uuid;
use crate::utils::jwt::{require_permission, verify_jwt};
use crate::utils::hash::{hash_secret, verify_secret};

pub fn user_management_routes() -> Router<PgPool> {
    Router::new()
        // Users CRUD
        .route("/users", get(list_users_handler).post(create_user_handler))
        .route("/users/{id}", get(get_user_handler).put(update_user_handler).delete(delete_user_handler))
        .route("/users/{id}/reset-password", post(reset_password_handler))
        .route("/users/{id}/change-password", post(admin_change_password_handler))
        .route("/users/{id}/change-pin", post(admin_change_pin_handler))
        .route("/users/{id}/toggle-active", put(toggle_user_active_handler))

        // Self User Profile & Outlets
        .route("/me/outlets", get(get_my_outlets_handler))
        .route("/me/change-password", post(self_change_password_handler))
        .route("/me/change-pin", post(self_change_pin_handler))

        // Roles CRUD
        .route("/roles", get(list_roles_handler).post(create_role_handler))
        .route("/roles/{id}", get(get_role_handler).put(update_role_handler).delete(delete_role_handler))
        .route("/roles/{id}/permissions", get(get_role_permissions_handler).put(set_role_permissions_handler))

        // Permissions
        .route("/permissions", get(list_permissions_handler))
}

// ============================================================================
// USERS
// ============================================================================

#[derive(Deserialize)]
struct UserQuery {
    search: Option<String>,
    role_id: Option<i32>,
    is_active: Option<bool>,
}

async fn list_users_handler(
    State(pool): State<PgPool>,
    Query(q): Query<UserQuery>,
) -> impl IntoResponse {
    let mut sql = "SELECT u.id, u.username, u.full_name, u.email, u.phone, u.is_active,
                          u.role_id, r.name as role_name, r.description as role_description,
                          u.created_at, u.updated_at
                   FROM users u
                   JOIN roles r ON u.role_id = r.id
                   WHERE 1=1".to_string();

    if let Some(s) = &q.search {
        sql.push_str(&format!(" AND (u.full_name ILIKE '%{s}%' OR u.username ILIKE '%{s}%' OR u.email ILIKE '%{s}%')"));
    }
    if let Some(rid) = q.role_id {
        sql.push_str(&format!(" AND u.role_id = {rid}"));
    }
    if let Some(active) = q.is_active {
        sql.push_str(&format!(" AND u.is_active = {active}"));
    }
    sql.push_str(" ORDER BY u.full_name ASC");

    match sqlx::query(&sql).fetch_all(&pool).await {
        Ok(rows) => {
            let mut list = Vec::new();
            for r in rows {
                let uid: Uuid = r.get("id");
                let outlet_rows = sqlx::query(
                    "SELECT o.id, o.code, o.name 
                     FROM user_outlets uo 
                     JOIN outlets o ON uo.outlet_id = o.id 
                     WHERE uo.user_id = $1 
                     ORDER BY o.code ASC",
                )
                .bind(uid)
                .fetch_all(&pool)
                .await
                .unwrap_or_default();

                let outlets: Vec<serde_json::Value> = outlet_rows.iter().map(|or| json!({
                    "id": or.get::<Uuid, _>("id"),
                    "code": or.get::<String, _>("code"),
                    "name": or.get::<String, _>("name"),
                })).collect();

                let outlet_ids: Vec<Uuid> = outlet_rows.iter().map(|or| or.get::<Uuid, _>("id")).collect();

                list.push(json!({
                    "id": uid,
                    "username": r.get::<String, _>("username"),
                    "full_name": r.get::<String, _>("full_name"),
                    "email": r.get::<Option<String>, _>("email"),
                    "phone": r.get::<Option<String>, _>("phone"),
                    "is_active": r.get::<bool, _>("is_active"),
                    "role_id": r.get::<i32, _>("role_id"),
                    "role_name": r.get::<String, _>("role_name"),
                    "role_description": r.get::<Option<String>, _>("role_description"),
                    "created_at": r.get::<chrono::DateTime<chrono::Utc>, _>("created_at"),
                    "updated_at": r.get::<chrono::DateTime<chrono::Utc>, _>("updated_at"),
                    "outlet_ids": outlet_ids,
                    "outlets": outlets,
                }));
            }
            (StatusCode::OK, Json(json!({ "success": true, "data": list }))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn get_user_handler(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let sql = "SELECT u.id, u.username, u.full_name, u.email, u.phone, u.is_active,
                      u.role_id, r.name as role_name,
                      u.created_at, u.updated_at,
                      COALESCE(
                        json_agg(p.code) FILTER (WHERE p.code IS NOT NULL),
                        '[]'::json
                      ) as permissions
               FROM users u
               JOIN roles r ON u.role_id = r.id
               LEFT JOIN role_permissions rp ON r.id = rp.role_id
               LEFT JOIN permissions p ON rp.permission_id = p.id
               WHERE u.id = $1
               GROUP BY u.id, r.name";

    match sqlx::query(sql).bind(id).fetch_optional(&pool).await {
        Ok(Some(r)) => {
            let outlet_rows = sqlx::query(
                "SELECT o.id, o.code, o.name 
                 FROM user_outlets uo 
                 JOIN outlets o ON uo.outlet_id = o.id 
                 WHERE uo.user_id = $1 
                 ORDER BY o.code ASC",
            )
            .bind(id)
            .fetch_all(&pool)
            .await
            .unwrap_or_default();

            let outlets: Vec<serde_json::Value> = outlet_rows.iter().map(|or| json!({
                "id": or.get::<Uuid, _>("id"),
                "code": or.get::<String, _>("code"),
                "name": or.get::<String, _>("name"),
            })).collect();

            let outlet_ids: Vec<Uuid> = outlet_rows.iter().map(|or| or.get::<Uuid, _>("id")).collect();

            let data = json!({
                "id": r.get::<Uuid, _>("id"),
                "username": r.get::<String, _>("username"),
                "full_name": r.get::<String, _>("full_name"),
                "email": r.get::<Option<String>, _>("email"),
                "phone": r.get::<Option<String>, _>("phone"),
                "is_active": r.get::<bool, _>("is_active"),
                "role_id": r.get::<i32, _>("role_id"),
                "role_name": r.get::<String, _>("role_name"),
                "permissions": r.get::<serde_json::Value, _>("permissions"),
                "created_at": r.get::<chrono::DateTime<chrono::Utc>, _>("created_at"),
                "updated_at": r.get::<chrono::DateTime<chrono::Utc>, _>("updated_at"),
                "outlet_ids": outlet_ids,
                "outlets": outlets,
            });
            (StatusCode::OK, Json(json!({ "success": true, "data": data }))).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({ "success": false, "message": "Pengguna tidak ditemukan" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

#[derive(Deserialize)]
struct CreateUserPayload {
    username: String,
    full_name: String,
    password: String,
    pin: String,
    role_id: i32,
    email: Option<String>,
    phone: Option<String>,
    outlet_ids: Option<Vec<Uuid>>,
}

async fn create_user_handler(
    State(pool): State<PgPool>,
    Json(payload): Json<CreateUserPayload>,
) -> impl IntoResponse {
    // Hash password & pin using Argon2id
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let password_hash = match argon2.hash_password(payload.password.as_bytes(), &salt) {
        Ok(h) => h.to_string(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    };

    let pin_salt = SaltString::generate(&mut OsRng);
    let pin_hash = match argon2.hash_password(payload.pin.as_bytes(), &pin_salt) {
        Ok(h) => h.to_string(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    };

    let res = sqlx::query(
        "INSERT INTO users (role_id, username, password_hash, pin_hash, full_name, email, phone)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING id, username, full_name"
    )
    .bind(payload.role_id)
    .bind(&payload.username)
    .bind(&password_hash)
    .bind(&pin_hash)
    .bind(&payload.full_name)
    .bind(&payload.email)
    .bind(&payload.phone)
    .fetch_one(&pool)
    .await;

    match res {
        Ok(r) => {
            let new_user_id: Uuid = r.get("id");
            if let Some(oids) = payload.outlet_ids {
                for oid in oids {
                    sqlx::query("INSERT INTO user_outlets (user_id, outlet_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
                        .bind(new_user_id)
                        .bind(oid)
                        .execute(&pool)
                        .await
                        .ok();
                }
            }
            (StatusCode::CREATED, Json(json!({
                "success": true,
                "data": {
                    "id": new_user_id,
                    "username": r.get::<String, _>("username"),
                    "full_name": r.get::<String, _>("full_name"),
                }
            }))).into_response()
        }
        Err(e) => {
            let msg = if e.to_string().contains("unique") {
                "Username sudah digunakan, silahkan pilih username lain".to_string()
            } else {
                e.to_string()
            };
            (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": msg }))).into_response()
        }
    }
}

#[derive(Deserialize)]
struct UpdateUserPayload {
    full_name: String,
    role_id: i32,
    email: Option<String>,
    phone: Option<String>,
    outlet_ids: Option<Vec<Uuid>>,
}

async fn update_user_handler(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateUserPayload>,
) -> impl IntoResponse {
    let res = sqlx::query(
        "UPDATE users SET full_name = $1, role_id = $2, email = $3, phone = $4,
         updated_at = CURRENT_TIMESTAMP WHERE id = $5"
    )
    .bind(&payload.full_name)
    .bind(payload.role_id)
    .bind(&payload.email)
    .bind(&payload.phone)
    .bind(id)
    .execute(&pool)
    .await;

    if let Some(oids) = payload.outlet_ids {
        sqlx::query("DELETE FROM user_outlets WHERE user_id = $1")
            .bind(id)
            .execute(&pool)
            .await
            .ok();

        for oid in oids {
            sqlx::query("INSERT INTO user_outlets (user_id, outlet_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
                .bind(id)
                .bind(oid)
                .execute(&pool)
                .await
                .ok();
        }
    }

    match res {
        Ok(r) if r.rows_affected() > 0 => (StatusCode::OK, Json(json!({ "success": true, "message": "Data pengguna berhasil diperbarui" }))).into_response(),
        Ok(_) => (StatusCode::NOT_FOUND, Json(json!({ "success": false, "message": "Pengguna tidak ditemukan" }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn get_my_outlets_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| if h.starts_with("Bearer ") { Some(&h[7..]) } else { None });

    let token = match auth_header {
        Some(t) => t,
        None => return (StatusCode::UNAUTHORIZED, Json(json!({ "success": false, "message": "Autentikasi dibutuhkan" }))).into_response(),
    };

    let claims = match verify_jwt(token) {
        Ok(c) => c,
        Err(e) => return (StatusCode::UNAUTHORIZED, Json(json!({ "success": false, "message": e }))).into_response(),
    };

    let is_superadmin = sqlx::query_scalar::<_, bool>(
        "SELECT (r.name = 'SUPERADMIN') FROM users u JOIN roles r ON u.role_id = r.id WHERE u.id = $1",
    )
    .bind(claims.sub)
    .fetch_one(&pool)
    .await
    .unwrap_or(false);

    let rows = if is_superadmin {
        sqlx::query(
            "SELECT id, code, name, is_warehouse, default_warehouse_id FROM outlets WHERE is_active = TRUE ORDER BY code ASC"
        )
        .fetch_all(&pool)
        .await
    } else {
        sqlx::query(
            "SELECT o.id, o.code, o.name, o.is_warehouse, o.default_warehouse_id 
             FROM outlets o 
             JOIN user_outlets uo ON o.id = uo.outlet_id 
             WHERE uo.user_id = $1 AND o.is_active = TRUE 
             ORDER BY o.code ASC"
        )
        .bind(claims.sub)
        .fetch_all(&pool)
        .await
    };

    match rows {
        Ok(r) => {
            let list: Vec<serde_json::Value> = r.iter().map(|row| json!({
                "id": row.get::<Uuid, _>("id"),
                "code": row.get::<String, _>("code"),
                "name": row.get::<String, _>("name"),
                "is_warehouse": row.get::<bool, _>("is_warehouse"),
                "default_warehouse_id": row.get::<Option<Uuid>, _>("default_warehouse_id"),
            })).collect();
            (StatusCode::OK, Json(json!({ "success": true, "data": list }))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn delete_user_handler(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    // Soft delete: deactivate user, never delete
    match sqlx::query("UPDATE users SET is_active = false, updated_at = CURRENT_TIMESTAMP WHERE id = $1")
        .bind(id).execute(&pool).await
    {
        Ok(r) if r.rows_affected() > 0 => (StatusCode::OK, Json(json!({ "success": true, "message": "Pengguna berhasil dinonaktifkan" }))).into_response(),
        Ok(_) => (StatusCode::NOT_FOUND, Json(json!({ "success": false, "message": "Pengguna tidak ditemukan" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

#[derive(Deserialize)]
struct ResetPasswordPayload {
    new_password: String,
    new_pin: Option<String>,
}

async fn reset_password_handler(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Json(payload): Json<ResetPasswordPayload>,
) -> impl IntoResponse {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };

    let argon2 = Argon2::default();
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = match argon2.hash_password(payload.new_password.as_bytes(), &salt) {
        Ok(h) => h.to_string(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    };

    if let Some(pin) = payload.new_pin {
        let pin_salt = SaltString::generate(&mut OsRng);
        let pin_hash = match argon2.hash_password(pin.as_bytes(), &pin_salt) {
            Ok(h) => h.to_string(),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
        };
        let _ = sqlx::query("UPDATE users SET pin_hash = $1 WHERE id = $2")
            .bind(&pin_hash).bind(id).execute(&pool).await;
    }

    match sqlx::query("UPDATE users SET password_hash = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
        .bind(&password_hash).bind(id).execute(&pool).await
    {
        Ok(r) if r.rows_affected() > 0 => (StatusCode::OK, Json(json!({ "success": true, "message": "Password berhasil direset" }))).into_response(),
        Ok(_) => (StatusCode::NOT_FOUND, Json(json!({ "success": false, "message": "Pengguna tidak ditemukan" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

// ----------------------------------------------------------------------------
// Admin Ganti Password Pengguna
// ----------------------------------------------------------------------------
#[derive(Deserialize)]
struct AdminChangePasswordPayload {
    new_password: String,
}

async fn admin_change_password_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<AdminChangePasswordPayload>,
) -> impl IntoResponse {
    if let Err(res) = require_permission(&pool, &headers, "users:manage").await {
        return res.into_response();
    }

    if payload.new_password.len() < 6 {
        return (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": "Password baru minimal 6 karakter" }))).into_response();
    }

    let password_hash = match hash_secret(&payload.new_password) {
        Ok(h) => h,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e }))).into_response(),
    };

    match sqlx::query("UPDATE users SET password_hash = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
        .bind(&password_hash)
        .bind(id)
        .execute(&pool)
        .await
    {
        Ok(r) if r.rows_affected() > 0 => (StatusCode::OK, Json(json!({ "success": true, "message": "Password pengguna berhasil diperbarui" }))).into_response(),
        Ok(_) => (StatusCode::NOT_FOUND, Json(json!({ "success": false, "message": "Pengguna tidak ditemukan" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

// ----------------------------------------------------------------------------
// Admin Ganti PIN Pengguna (Memerlukan permission users:change_pin)
// ----------------------------------------------------------------------------
#[derive(Deserialize)]
struct AdminChangePinPayload {
    new_pin: String,
}

async fn admin_change_pin_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<AdminChangePinPayload>,
) -> impl IntoResponse {
    if let Err(res) = require_permission(&pool, &headers, "users:change_pin").await {
        return res.into_response();
    }

    let pin = payload.new_pin.trim();
    if pin.len() < 4 || pin.len() > 6 || !pin.chars().all(|c| c.is_ascii_digit()) {
        return (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": "PIN harus berupa 4-6 digit angka" }))).into_response();
    }

    let pin_hash = match hash_secret(pin) {
        Ok(h) => h,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e }))).into_response(),
    };

    match sqlx::query("UPDATE users SET pin_hash = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
        .bind(&pin_hash)
        .bind(id)
        .execute(&pool)
        .await
    {
        Ok(r) if r.rows_affected() > 0 => (StatusCode::OK, Json(json!({ "success": true, "message": "PIN pengguna berhasil diperbarui" }))).into_response(),
        Ok(_) => (StatusCode::NOT_FOUND, Json(json!({ "success": false, "message": "Pengguna tidak ditemukan" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

// ----------------------------------------------------------------------------
// Pengguna Mandiri (Self): Ganti Password Sendiri
// ----------------------------------------------------------------------------
#[derive(Deserialize)]
struct SelfChangePasswordPayload {
    current_password: String,
    new_password: String,
}

async fn self_change_password_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<SelfChangePasswordPayload>,
) -> impl IntoResponse {
    let auth_header = match headers.get("Authorization").and_then(|h| h.to_str().ok()) {
        Some(h) if h.starts_with("Bearer ") => &h[7..],
        _ => return (StatusCode::UNAUTHORIZED, Json(json!({ "success": false, "message": "Autentikasi dibutuhkan" }))).into_response(),
    };

    let claims = match verify_jwt(auth_header) {
        Ok(c) => c,
        Err(e) => return (StatusCode::UNAUTHORIZED, Json(json!({ "success": false, "message": e }))).into_response(),
    };

    if payload.new_password.len() < 6 {
        return (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": "Password baru minimal 6 karakter" }))).into_response();
    }

    // Ambil password_hash pengguna saat ini
    let current_hash: Option<String> = match sqlx::query_scalar("SELECT password_hash FROM users WHERE id = $1")
        .bind(claims.sub)
        .fetch_optional(&pool)
        .await
    {
        Ok(h) => h,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    };

    let current_hash = match current_hash {
        Some(h) => h,
        None => return (StatusCode::NOT_FOUND, Json(json!({ "success": false, "message": "Pengguna tidak ditemukan" }))).into_response(),
    };

    if !verify_secret(&payload.current_password, &current_hash) {
        return (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": "Password lama yang dimasukkan tidak sesuai" }))).into_response();
    }

    let new_hash = match hash_secret(&payload.new_password) {
        Ok(h) => h,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e }))).into_response(),
    };

    match sqlx::query("UPDATE users SET password_hash = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
        .bind(&new_hash)
        .bind(claims.sub)
        .execute(&pool)
        .await
    {
        Ok(_) => (StatusCode::OK, Json(json!({ "success": true, "message": "Password Anda berhasil diperbarui" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

// ----------------------------------------------------------------------------
// Pengguna Mandiri (Self): Ganti PIN Sendiri
// ----------------------------------------------------------------------------
#[derive(Deserialize)]
struct SelfChangePinPayload {
    current_pin: Option<String>,
    current_password: Option<String>,
    new_pin: String,
}

async fn self_change_pin_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<SelfChangePinPayload>,
) -> impl IntoResponse {
    let auth_header = match headers.get("Authorization").and_then(|h| h.to_str().ok()) {
        Some(h) if h.starts_with("Bearer ") => &h[7..],
        _ => return (StatusCode::UNAUTHORIZED, Json(json!({ "success": false, "message": "Autentikasi dibutuhkan" }))).into_response(),
    };

    let claims = match verify_jwt(auth_header) {
        Ok(c) => c,
        Err(e) => return (StatusCode::UNAUTHORIZED, Json(json!({ "success": false, "message": e }))).into_response(),
    };

    let new_pin = payload.new_pin.trim();
    if new_pin.len() < 4 || new_pin.len() > 6 || !new_pin.chars().all(|c| c.is_ascii_digit()) {
        return (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": "PIN baru harus berupa 4-6 digit angka" }))).into_response();
    }

    // Ambil data user
    let user_row = match sqlx::query("SELECT password_hash, pin_hash FROM users WHERE id = $1")
        .bind(claims.sub)
        .fetch_optional(&pool)
        .await
    {
        Ok(Some(r)) => r,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(json!({ "success": false, "message": "Pengguna tidak ditemukan" }))).into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    };

    let db_pwd_hash: String = user_row.get("password_hash");
    let db_pin_hash: String = user_row.get("pin_hash");

    // Verifikasi otentikasi pemilik akun: bisa lewat PIN lama atau password akun
    let mut verified = false;
    if let Some(cp) = &payload.current_pin {
        if verify_secret(cp, &db_pin_hash) {
            verified = true;
        }
    }
    if !verified {
        if let Some(pwd) = &payload.current_password {
            if verify_secret(pwd, &db_pwd_hash) {
                verified = true;
            }
        }
    }

    if !verified {
        return (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": "PIN lama atau password akun yang dimasukkan tidak sesuai" }))).into_response();
    }

    let new_pin_hash = match hash_secret(new_pin) {
        Ok(h) => h,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e }))).into_response(),
    };

    match sqlx::query("UPDATE users SET pin_hash = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
        .bind(&new_pin_hash)
        .bind(claims.sub)
        .execute(&pool)
        .await
    {
        Ok(_) => (StatusCode::OK, Json(json!({ "success": true, "message": "PIN Anda berhasil diperbarui" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

#[derive(Deserialize)]
struct ToggleActivePayload {
    is_active: bool,
}

async fn toggle_user_active_handler(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Json(payload): Json<ToggleActivePayload>,
) -> impl IntoResponse {
    match sqlx::query("UPDATE users SET is_active = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
        .bind(payload.is_active).bind(id).execute(&pool).await
    {
        Ok(r) if r.rows_affected() > 0 => {
            let status = if payload.is_active { "diaktifkan" } else { "dinonaktifkan" };
            (StatusCode::OK, Json(json!({ "success": true, "message": format!("Pengguna berhasil {status}") }))).into_response()
        }
        Ok(_) => (StatusCode::NOT_FOUND, Json(json!({ "success": false, "message": "Pengguna tidak ditemukan" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

// ============================================================================
// ROLES
// ============================================================================

async fn list_roles_handler(State(pool): State<PgPool>) -> impl IntoResponse {
    let sql = "SELECT r.id, r.name, r.description, r.is_active,
                      r.created_at, r.updated_at,
                      COUNT(DISTINCT u.id) as user_count,
                      COUNT(DISTINCT rp.permission_id) as permission_count
               FROM roles r
               LEFT JOIN users u ON r.id = u.role_id AND u.is_active = true
               LEFT JOIN role_permissions rp ON r.id = rp.role_id
               GROUP BY r.id
               ORDER BY r.id ASC";

    match sqlx::query(sql).fetch_all(&pool).await {
        Ok(rows) => {
            let list: Vec<serde_json::Value> = rows.iter().map(|r| json!({
                "id": r.get::<i32, _>("id"),
                "name": r.get::<String, _>("name"),
                "description": r.get::<Option<String>, _>("description"),
                "is_active": r.get::<bool, _>("is_active"),
                "user_count": r.get::<i64, _>("user_count"),
                "permission_count": r.get::<i64, _>("permission_count"),
                "created_at": r.get::<chrono::DateTime<chrono::Utc>, _>("created_at"),
                "updated_at": r.get::<chrono::DateTime<chrono::Utc>, _>("updated_at"),
            })).collect();
            (StatusCode::OK, Json(json!({ "success": true, "data": list }))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn get_role_handler(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    let role_sql = "SELECT r.*, COUNT(DISTINCT u.id) as user_count FROM roles r LEFT JOIN users u ON r.id = u.role_id AND u.is_active = true WHERE r.id = $1 GROUP BY r.id";
    let perm_sql = "SELECT p.id, p.code, p.description FROM permissions p JOIN role_permissions rp ON p.id = rp.permission_id WHERE rp.role_id = $1 ORDER BY p.code";

    let (role_res, perm_res) = tokio::join!(
        sqlx::query(role_sql).bind(id).fetch_optional(&pool),
        sqlx::query(perm_sql).bind(id).fetch_all(&pool)
    );

    match role_res {
        Ok(Some(r)) => {
            let permissions: Vec<serde_json::Value> = perm_res.unwrap_or_default().iter().map(|p| json!({
                "id": p.get::<i32, _>("id"),
                "code": p.get::<String, _>("code"),
                "description": p.get::<Option<String>, _>("description"),
            })).collect();

            let data = json!({
                "id": r.get::<i32, _>("id"),
                "name": r.get::<String, _>("name"),
                "description": r.get::<Option<String>, _>("description"),
                "is_active": r.get::<bool, _>("is_active"),
                "user_count": r.get::<i64, _>("user_count"),
                "permissions": permissions,
                "created_at": r.get::<chrono::DateTime<chrono::Utc>, _>("created_at"),
            });
            (StatusCode::OK, Json(json!({ "success": true, "data": data }))).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({ "success": false, "message": "Role tidak ditemukan" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

#[derive(Deserialize)]
struct RoleInput {
    name: String,
    description: Option<String>,
}

async fn create_role_handler(
    State(pool): State<PgPool>,
    Json(payload): Json<RoleInput>,
) -> impl IntoResponse {
    let name = payload.name.to_uppercase();
    match sqlx::query("INSERT INTO roles (name, description) VALUES ($1, $2) RETURNING id, name")
        .bind(&name).bind(&payload.description)
        .fetch_one(&pool).await
    {
        Ok(r) => (StatusCode::CREATED, Json(json!({
            "success": true,
            "data": { "id": r.get::<i32, _>("id"), "name": r.get::<String, _>("name") }
        }))).into_response(),
        Err(e) => {
            let msg = if e.to_string().contains("unique") { format!("Role '{name}' sudah ada") } else { e.to_string() };
            (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": msg }))).into_response()
        }
    }
}

async fn update_role_handler(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
    Json(payload): Json<RoleInput>,
) -> impl IntoResponse {
    match sqlx::query("UPDATE roles SET name = $1, description = $2, updated_at = CURRENT_TIMESTAMP WHERE id = $3")
        .bind(&payload.name.to_uppercase()).bind(&payload.description).bind(id)
        .execute(&pool).await
    {
        Ok(r) if r.rows_affected() > 0 => (StatusCode::OK, Json(json!({ "success": true, "message": "Role berhasil diperbarui" }))).into_response(),
        Ok(_) => (StatusCode::NOT_FOUND, Json(json!({ "success": false, "message": "Role tidak ditemukan" }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn delete_role_handler(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    // Check if role is in use
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE role_id = $1 AND is_active = true")
        .bind(id).fetch_one(&pool).await.unwrap_or(0);

    if count > 0 {
        return (StatusCode::CONFLICT, Json(json!({ "success": false, "message": format!("Role tidak bisa dihapus karena masih digunakan oleh {count} pengguna aktif") }))).into_response();
    }

    match sqlx::query("UPDATE roles SET is_active = false, updated_at = CURRENT_TIMESTAMP WHERE id = $1")
        .bind(id).execute(&pool).await
    {
        Ok(r) if r.rows_affected() > 0 => (StatusCode::OK, Json(json!({ "success": true, "message": "Role berhasil dinonaktifkan" }))).into_response(),
        Ok(_) => (StatusCode::NOT_FOUND, Json(json!({ "success": false, "message": "Role tidak ditemukan" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn get_role_permissions_handler(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    match sqlx::query_scalar::<_, i32>("SELECT id FROM roles WHERE id = $1 AND is_active = true")
        .bind(id).fetch_optional(&pool).await
    {
        Ok(None) => return (StatusCode::NOT_FOUND, Json(json!({ "success": false, "message": "Role tidak ditemukan" }))).into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
        _ => {}
    }

    let sql = "SELECT p.id, p.code, p.description,
               EXISTS(SELECT 1 FROM role_permissions rp WHERE rp.role_id = $1 AND rp.permission_id = p.id) as has_permission
               FROM permissions p ORDER BY p.code ASC";

    match sqlx::query(sql).bind(id).fetch_all(&pool).await {
        Ok(rows) => {
            let list: Vec<serde_json::Value> = rows.iter().map(|r| json!({
                "id": r.get::<i32, _>("id"),
                "code": r.get::<String, _>("code"),
                "description": r.get::<Option<String>, _>("description"),
                "has_permission": r.get::<bool, _>("has_permission"),
            })).collect();
            (StatusCode::OK, Json(json!({ "success": true, "data": list }))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

#[derive(Deserialize)]
struct SetRolePermissionsPayload {
    permission_ids: Vec<i32>,
}

async fn set_role_permissions_handler(
    State(pool): State<PgPool>,
    Path(role_id): Path<i32>,
    Json(payload): Json<SetRolePermissionsPayload>,
) -> impl IntoResponse {
    let mut tx = match pool.begin().await {
        Ok(t) => t,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    };

    // Remove all existing permissions for this role
    if let Err(e) = sqlx::query("DELETE FROM role_permissions WHERE role_id = $1")
        .bind(role_id).execute(&mut *tx).await
    {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response();
    }

    // Re-insert selected permissions
    for perm_id in &payload.permission_ids {
        if let Err(e) = sqlx::query("INSERT INTO role_permissions (role_id, permission_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(role_id).bind(perm_id).execute(&mut *tx).await
        {
            return (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": e.to_string() }))).into_response();
        }
    }

    match tx.commit().await {
        Ok(_) => (StatusCode::OK, Json(json!({
            "success": true,
            "message": format!("Berhasil menetapkan {} permission ke role", payload.permission_ids.len())
        }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

// ============================================================================
// PERMISSIONS
// ============================================================================

async fn list_permissions_handler(State(pool): State<PgPool>) -> impl IntoResponse {
    match sqlx::query("SELECT id, code, description, created_at FROM permissions ORDER BY code ASC")
        .fetch_all(&pool).await
    {
        Ok(rows) => {
            let list: Vec<serde_json::Value> = rows.iter().map(|r| json!({
                "id": r.get::<i32, _>("id"),
                "code": r.get::<String, _>("code"),
                "description": r.get::<Option<String>, _>("description"),
            })).collect();
            (StatusCode::OK, Json(json!({ "success": true, "data": list }))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}
