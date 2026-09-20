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
    pub terminal_id: String,
    pub action: String, // LOGIN, VOID, PRICE_OVERRIDE, CASH_DRAWER_OPEN, SHIFT_CLOSE
    pub entity_name: String,
    pub entity_id: Option<String>,
    pub old_values: Option<Value>,
    pub new_values: Option<Value>,
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuditLogDetail {
    pub log: AuditLog,
    pub user_full_name: Option<String>,
    pub supervisor_full_name: Option<String>,
}
