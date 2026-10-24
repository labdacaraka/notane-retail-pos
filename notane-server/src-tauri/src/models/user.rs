use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Role {
    pub id: i32,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Permission {
    pub id: i32,
    pub code: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct User {
    pub id: Uuid,
    pub role_id: i32,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    #[serde(skip_serializing)]
    pub pin_hash: String,
    pub full_name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub is_active: bool,
    pub biometric_credential_id: Option<String>,
    pub biometric_public_key: Option<String>,
    pub face_descriptor: Option<String>,
    pub face_photo_preview: Option<String>,
    #[serde(default)]
    pub has_biometric: bool,
    #[serde(default)]
    pub has_face_recognition: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserProfile {
    pub id: Uuid,
    pub role_id: i32,
    pub role_name: String,
    pub username: String,
    pub full_name: String,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub permissions: Vec<String>,
    pub outlet_ids: Vec<Uuid>,
    #[serde(default)]
    pub has_biometric: bool,
    #[serde(default)]
    pub has_face_recognition: bool,
    pub face_photo_preview: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    pub terminal_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserProfile,
}

#[derive(Debug, Deserialize)]
pub struct SupervisorPinRequest {
    pub supervisor_id: Option<Uuid>,
    pub pin: String,
    pub action: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SupervisorPinResponse {
    pub success: bool,
    pub supervisor_id: Uuid,
    pub supervisor_name: String,
    pub message: String,
    pub auth_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SupervisorBiometricApproveRequest {
    pub supervisor_id: Option<Uuid>,
    pub biometric_type: String, // "FINGERPRINT" or "FACE"
    pub credential_id: Option<String>,
    pub live_face_descriptor: Option<String>,
    pub challenge: Option<String>,
    pub action: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EnrollBiometricRequest {
    pub biometric_type: String, // "FINGERPRINT" or "FACE"
    pub credential_id: Option<String>,
    pub public_key: Option<String>,
    pub face_descriptor: Option<String>,
    pub face_photo_preview: Option<String>,
}
