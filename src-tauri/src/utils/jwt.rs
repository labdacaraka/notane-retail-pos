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
