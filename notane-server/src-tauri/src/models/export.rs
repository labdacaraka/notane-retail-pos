use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ExportJob {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub outlet_id: Option<Uuid>,
    pub module: String,
    pub title: String,
    pub filter_params: serde_json::Value,
    pub filename: String,
    pub file_path: Option<String>,
    pub file_size_bytes: Option<i64>,
    pub total_rows: Option<i32>,
    pub processed_rows: Option<i32>,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateExportJobRequest {
    pub module: String,
    pub title: String,
    pub filter_params: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ExportJobListQuery {
    pub module: Option<String>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}
