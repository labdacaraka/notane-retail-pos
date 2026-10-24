use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct AuditLog {
    pub id: i64,
    pub user_id: Option<Uuid>,
    pub authorized_by: Option<Uuid>,
    #[serde(default = "default_terminal_id")]
    pub terminal_id: String,
    pub action: String,
    pub entity_name: String,
    pub entity_id: Option<String>,
    pub old_values: Option<Value>,
    pub new_values: Option<Value>,
    pub ip_address: Option<String>,
    pub http_method: Option<String>,
    pub endpoint: Option<String>,
    pub status_code: Option<i32>,
    pub execution_time_ms: Option<i32>,
    pub user_agent: Option<String>,
    pub user_name: Option<String>,
    pub role_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

fn default_terminal_id() -> String {
    "SYSTEM".to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuditLogDetail {
    pub log: AuditLog,
    pub user_full_name: Option<String>,
    pub supervisor_full_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuditLogPaginatedResponse {
    pub items: Vec<AuditLogDetail>,
    pub total: i64,
    pub page: i64,
    pub limit: i64,
    pub retention_days: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuditLogSettingsResponse {
    pub retention_days: i32,
    pub total_logs: i64,
    pub oldest_log_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateAuditLogSettingsRequest {
    pub retention_days: i32,
}
