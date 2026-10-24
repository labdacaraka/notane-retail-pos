use serde::{Deserialize, Serialize};
use validator::Validate;

// ==============================================================================
// ERROR HANDLING (Standard Error Enum deriving serde::Serialize)
// ==============================================================================
#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "type", content = "details")]
pub enum AppError {
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Not Found: {0}")]
    NotFound(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Cryptographic error: {0}")]
    Crypto(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Database(err.to_string())
    }
}

// ==============================================================================
// PERMISSION SYSTEM (RBAC)
// ==============================================================================
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Permission {
    ClientRead,
    ClientCreate,
    ClientUpdate,
    ClientDelete,
    LicenseRead,
    LicenseCreate,
    LicenseUpdate,
    LicenseRevoke,
    DeviceRead,
    DeviceUnbind,
    ReleaseRead,
    ReleaseCreate,
    ReleaseDelete,
    ServerControl,
    AuditRead,
}

impl Permission {
    pub fn slug(&self) -> &'static str {
        match self {
            Permission::ClientRead => "client:read",
            Permission::ClientCreate => "client:create",
            Permission::ClientUpdate => "client:update",
            Permission::ClientDelete => "client:delete",
            Permission::LicenseRead => "license:read",
            Permission::LicenseCreate => "license:create",
            Permission::LicenseUpdate => "license:update",
            Permission::LicenseRevoke => "license:revoke",
            Permission::DeviceRead => "device:read",
            Permission::DeviceUnbind => "device:unbind",
            Permission::ReleaseRead => "release:read",
            Permission::ReleaseCreate => "release:create",
            Permission::ReleaseDelete => "release:delete",
            Permission::ServerControl => "server:control",
            Permission::AuditRead => "audit:read",
        }
    }
}

// ==============================================================================
// PAGINATION & SORTING MODELS
// ==============================================================================
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PaginationParams {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>, // "asc" or "desc"
    pub search: Option<String>,
    pub status: Option<String>,
    pub tier: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub page: i64,
    pub page_size: i64,
    pub total_items: i64,
    pub total_pages: i64,
}

// ==============================================================================
// CLIENT / TENANT MODELS & DTOS
// ==============================================================================
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Client {
    pub id: String,
    pub client_name: String,
    pub company_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateClientDto {
    #[validate(length(min = 2, max = 150, message = "Nama klien minimal 2 karakter"))]
    pub client_name: String,

    #[validate(length(min = 2, max = 150, message = "Nama perusahaan minimal 2 karakter"))]
    pub company_name: String,

    #[validate(email(message = "Format email tidak valid"))]
    pub email: String,

    pub phone: Option<String>,
    pub address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateClientDto {
    pub id: String,

    #[validate(length(min = 2, max = 150, message = "Nama klien minimal 2 karakter"))]
    pub client_name: String,

    #[validate(length(min = 2, max = 150, message = "Nama perusahaan minimal 2 karakter"))]
    pub company_name: String,

    #[validate(email(message = "Format email tidak valid"))]
    pub email: String,

    pub phone: Option<String>,
    pub address: Option<String>,
}

// ==============================================================================
// LICENSE MODELS & DTOS
// ==============================================================================
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct License {
    pub id: String,
    pub license_key: String,
    pub client_id: String,
    pub client_name: Option<String>,
    pub tier: String, // BASIC, PRO, ENTERPRISE
    pub max_terminals: i64,
    pub status: String, // ACTIVE, SUSPENDED, EXPIRED, REVOKED
    pub issued_at: String,
    pub expires_at: String,
    pub grace_period_days: i64,
    pub features: String, // JSON array string
    pub notes: Option<String>,
    pub active_terminals: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateLicenseDto {
    pub client_id: String,

    #[validate(length(min = 2, max = 50, message = "Tier harus dipilih"))]
    pub tier: String,

    #[validate(range(min = 1, max = 1000, message = "Maksimal terminal antara 1 dan 1000"))]
    pub max_terminals: i64,

    pub expires_in_days: i64,
    pub grace_period_days: Option<i64>,
    pub features: Vec<String>,
    pub notes: Option<String>,
    pub custom_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseTokenPayload {
    pub license_key: String,
    pub client_name: String,
    pub outlet_id: Option<String>,
    pub tier: String,
    pub max_terminals: i64,
    pub hardware_id: String,
    pub issued_at: String,
    pub expires_at: String,
    pub offline_grace_days: i64,
    pub features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualSignResult {
    pub license_token_json: String,
    pub signature_hex: String,
    pub public_key_hex: String,
}

// ==============================================================================
// DEVICE ACTIVATION (Node-Locking HWID)
// ==============================================================================
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DeviceActivation {
    pub id: String,
    pub license_id: String,
    pub license_key: Option<String>,
    pub client_name: Option<String>,
    pub hardware_id: String,
    pub device_name: Option<String>,
    pub ip_address: Option<String>,
    pub os_info: Option<String>,
    pub app_version: Option<String>,
    pub activated_at: String,
    pub last_ping_at: String,
    pub is_active: i64, // 1 or 0 in sqlite
}

// ==============================================================================
// APP RELEASE (OTA Updates)
// ==============================================================================
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AppRelease {
    pub id: String,
    pub version: String,
    pub release_name: String,
    pub changelog: String,
    pub release_date: String,
    pub download_url: String,
    pub checksum_sha256: String,
    pub signature_ed25519: String,
    pub is_mandatory: i64, // 1 or 0
    pub min_license_tier: String,
    pub has_db_migration: i64, // 1 or 0
    pub is_published: i64,     // 1 or 0
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateReleaseDto {
    #[validate(length(min = 1, max = 30, message = "Versi harus diisi (contoh: 1.2.0)"))]
    pub version: String,

    #[validate(length(min = 2, max = 150, message = "Nama rilis harus diisi"))]
    pub release_name: String,

    #[validate(length(min = 5, message = "Changelog minimal 5 karakter"))]
    pub changelog: String,

    pub download_url: String,
    pub checksum_sha256: String,
    pub is_mandatory: bool,
    pub min_license_tier: String,
    pub has_db_migration: bool,
}

// ==============================================================================
// AUDIT LOGS
// ==============================================================================
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AuditLog {
    pub id: String,
    pub license_key: Option<String>,
    pub hardware_id: Option<String>,
    pub action: String, // ACTIVATE, VERIFY, CHECK_UPDATE, TAMPER_ALERT
    pub ip_address: Option<String>,
    pub status: String, // SUCCESS, REJECTED, BLOCKED
    pub details: Option<String>,
    pub created_at: String,
}

// ==============================================================================
// PROVIDER SETTINGS & SERVER STATUS
// ==============================================================================
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatus {
    pub is_server_running: bool,
    pub port: u16,
    pub external_url: String,
    pub total_clients: i64,
    pub total_licenses: i64,
    pub active_licenses: i64,
    pub total_devices: i64,
    pub total_releases: i64,
    pub master_public_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSettings {
    pub server_port: u16,
    pub auto_start_server: bool,
    pub master_private_key: String,
    pub master_public_key: String,
}

// ==============================================================================
// AUTHENTICATION & ADMIN SESSION
// ==============================================================================
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AdminUser {
    pub id: String,
    pub username: String,
    pub full_name: String,
    pub role: String,
    pub last_login_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct LoginDto {
    #[validate(length(min = 1, message = "Username wajib diisi"))]
    pub username: String,

    #[validate(length(min = 1, message = "Password wajib diisi"))]
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PinLoginDto {
    #[validate(length(min = 4, max = 10, message = "PIN keamanan minimal 4 digit"))]
    pub pin: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ChangeCredentialsDto {
    pub old_password: Option<String>,

    #[validate(length(min = 4, message = "Password baru minimal 4 karakter"))]
    pub new_password: Option<String>,

    #[validate(length(min = 4, max = 10, message = "PIN baru harus 4-10 digit"))]
    pub new_pin: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub user: AdminUser,
    pub token: String,
    pub permissions: Vec<String>,
}
