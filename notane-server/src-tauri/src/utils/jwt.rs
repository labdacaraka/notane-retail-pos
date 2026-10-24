use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const JWT_SECRET: &[u8] = b"notane-retail-pos-enterprise-secret-key-2026";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: Uuid,
    pub username: String,
    pub role_id: i32,
    pub role_name: String,
    pub terminal_id: Option<String>,
    pub exp: usize,
    pub iat: usize,
}

pub fn create_jwt(
    user_id: Uuid,
    username: &str,
    role_id: i32,
    role_name: &str,
    terminal_id: Option<String>,
    duration_hours: i64,
) -> Result<String, String> {
    let now = Utc::now();
    let exp = (now + Duration::hours(duration_hours)).timestamp() as usize;
    let iat = now.timestamp() as usize;

    let claims = Claims {
        sub: user_id,
        username: username.to_string(),
        role_id,
        role_name: role_name.to_string(),
        terminal_id,
        exp,
        iat,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET),
    )
    .map_err(|e| format!("Gagal membuat JWT token: {}", e))
}

pub fn verify_jwt(token: &str) -> Result<Claims, String> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(JWT_SECRET),
        &Validation::default(),
    )
    .map_err(|e| format!("Token tidak valid atau telah kedaluwarsa: {}", e))?;

    Ok(token_data.claims)
}

pub async fn require_permission(
    pool: &sqlx::PgPool,
    headers: &axum::http::HeaderMap,
    required_perm: &str,
) -> Result<Claims, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| if h.starts_with("Bearer ") { Some(&h[7..]) } else { None })
        .ok_or_else(|| {
            (
                axum::http::StatusCode::UNAUTHORIZED,
                axum::Json(serde_json::json!({ "success": false, "message": "Autentikasi dibutuhkan. Silakan login." })),
            )
        })?;

    let claims = verify_jwt(auth_header).map_err(|e| {
        (
            axum::http::StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({ "success": false, "message": e })),
        )
    })?;

    if claims.role_name == "SUPERADMIN" {
        return Ok(claims);
    }

    let has_perm: bool = sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM role_permissions rp
            JOIN permissions p ON rp.permission_id = p.id
            WHERE rp.role_id = $1 AND p.code = $2
        )"
    )
    .bind(claims.role_id)
    .bind(required_perm)
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    if !has_perm {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            axum::Json(serde_json::json!({
                "success": false,
                "message": format!("Akses ditolak: Peran '{}' tidak memiliki hak akses '{}'", claims.role_name, required_perm)
            })),
        ));
    }

    Ok(claims)
}

pub async fn require_any_permission(
    pool: &sqlx::PgPool,
    headers: &axum::http::HeaderMap,
    required_perms: &[&str],
) -> Result<Claims, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| if h.starts_with("Bearer ") { Some(&h[7..]) } else { None })
        .ok_or_else(|| {
            (
                axum::http::StatusCode::UNAUTHORIZED,
                axum::Json(serde_json::json!({ "success": false, "message": "Autentikasi dibutuhkan. Silakan login." })),
            )
        })?;

    let claims = verify_jwt(auth_header).map_err(|e| {
        (
            axum::http::StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({ "success": false, "message": e })),
        )
    })?;

    if claims.role_name == "SUPERADMIN" {
        return Ok(claims);
    }

    let perm_strings: Vec<String> = required_perms.iter().map(|s| s.to_string()).collect();
    let has_perm: bool = sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM role_permissions rp
            JOIN permissions p ON rp.permission_id = p.id
            WHERE rp.role_id = $1 AND p.code = ANY($2)
        )"
    )
    .bind(claims.role_id)
    .bind(&perm_strings)
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    if !has_perm {
        return Err((
            axum::http::StatusCode::FORBIDDEN,
            axum::Json(serde_json::json!({
                "success": false,
                "message": format!("Akses ditolak: Peran '{}' tidak memiliki salah satu hak akses: {}", claims.role_name, required_perms.join(", "))
            })),
        ));
    }

    Ok(claims)
}

pub fn extract_claims(headers: &axum::http::HeaderMap) -> Result<Claims, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| if h.starts_with("Bearer ") { Some(&h[7..]) } else { None })
        .ok_or_else(|| {
            (
                axum::http::StatusCode::UNAUTHORIZED,
                axum::Json(serde_json::json!({ "success": false, "message": "Autentikasi dibutuhkan. Silakan login." })),
            )
        })?;

    verify_jwt(auth_header).map_err(|e| {
        (
            axum::http::StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({ "success": false, "message": e })),
        )
    })
}

pub async fn check_permission(
    pool: &sqlx::PgPool,
    claims: &Claims,
    perm: &str,
) -> bool {
    if claims.role_name == "SUPERADMIN" {
        return true;
    }
    sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM role_permissions rp
            JOIN permissions p ON rp.permission_id = p.id
            WHERE rp.role_id = $1 AND p.code = $2
        )"
    )
    .bind(claims.role_id)
    .bind(perm)
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}

pub async fn resolve_effective_outlet_id(
    pool: &sqlx::PgPool,
    headers: &axum::http::HeaderMap,
    provided_outlet_id: Option<Uuid>,
) -> Option<Uuid> {
    if let Some(oid) = provided_outlet_id {
        return Some(oid);
    }

    if let Ok(claims) = extract_claims(headers) {
        let user_id = claims.sub;

        // 1. Jika ada open shift kasir untuk user ini, gunakan outlet dari shift tersebut
        if let Ok(Some(shift_outlet_id)) = sqlx::query_scalar::<_, Uuid>(
            "SELECT outlet_id FROM cashier_shifts WHERE cashier_id = $1 AND status = 'OPEN' ORDER BY start_time DESC LIMIT 1",
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await
        {
            return Some(shift_outlet_id);
        }

        // 2. Ambil preferensi current_outlet_id user yang tersimpan di database
        if let Ok(Some(user_outlet_id)) = sqlx::query_scalar::<_, Option<Uuid>>(
            "SELECT current_outlet_id FROM users WHERE id = $1",
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await
        {
            if let Some(oid) = user_outlet_id {
                return Some(oid);
            }
        }
    }

    // 3. Fallback: Ambil outlet ritel aktif pertama di database
    sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM outlets WHERE is_active = TRUE ORDER BY is_warehouse ASC, code ASC LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .unwrap_or(None)
}


