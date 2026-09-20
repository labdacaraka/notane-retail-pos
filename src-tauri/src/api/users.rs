use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub fn user_management_routes() -> Router<PgPool> {
    Router::new()
        // Users CRUD
        .route("/users", get(list_users_handler).post(create_user_handler))
        .route("/users/{id}", get(get_user_handler).put(update_user_handler).delete(delete_user_handler))
        .route("/users/{id}/reset-password", post(reset_password_handler))
        .route("/users/{id}/toggle-active", put(toggle_user_active_handler))

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
            let list: Vec<serde_json::Value> = rows.iter().map(|r| json!({
                "id": r.get::<Uuid, _>("id"),
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
            })).collect();
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
        Ok(r) => (StatusCode::CREATED, Json(json!({
            "success": true,
            "data": {
                "id": r.get::<Uuid, _>("id"),
                "username": r.get::<String, _>("username"),
                "full_name": r.get::<String, _>("full_name"),
            }
        }))).into_response(),
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

    match res {
        Ok(r) if r.rows_affected() > 0 => (StatusCode::OK, Json(json!({ "success": true, "message": "Data pengguna berhasil diperbarui" }))).into_response(),
        Ok(_) => (StatusCode::NOT_FOUND, Json(json!({ "success": false, "message": "Pengguna tidak ditemukan" }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
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
