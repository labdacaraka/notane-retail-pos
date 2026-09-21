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
