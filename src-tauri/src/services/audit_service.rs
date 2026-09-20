use crate::models::audit::{AuditLog, AuditLogDetail};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn log_audit(
    pool: &PgPool,
    user_id: Option<Uuid>,
    authorized_by: Option<Uuid>,
    terminal_id: &str,
    action: &str,
    entity_name: &str,
    entity_id: Option<&str>,
    old_values: Option<Value>,
    new_values: Option<Value>,
    ip_address: Option<&str>,
) -> Result<i64, String> {
    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO audit_logs (
            user_id, authorized_by, terminal_id, action, entity_name, entity_id, 
            old_values, new_values, ip_address, created_at, updated_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
        RETURNING id",
    )
    .bind(user_id)
    .bind(authorized_by)
    .bind(terminal_id)
    .bind(action)
    .bind(entity_name)
    .bind(entity_id)
    .bind(old_values)
    .bind(new_values)
    .bind(ip_address)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Gagal mencatat audit log: {}", e))?;

    Ok(id)
}

pub async fn list_audit_logs(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<AuditLogDetail>, String> {
    let logs = sqlx::query_as::<_, AuditLog>(
        "SELECT * FROM audit_logs ORDER BY created_at DESC LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Gagal memuat audit logs: {}", e))?;

    let mut details = Vec::new();
    for log in logs {
        let user_name = if let Some(uid) = log.user_id {
            sqlx::query_scalar::<_, String>("SELECT full_name FROM users WHERE id = $1")
                .bind(uid)
                .fetch_optional(pool)
                .await
                .unwrap_or(None)
        } else {
            None
        };

        let spv_name = if let Some(spv_id) = log.authorized_by {
            sqlx::query_scalar::<_, String>("SELECT full_name FROM users WHERE id = $1")
                .bind(spv_id)
                .fetch_optional(pool)
                .await
                .unwrap_or(None)
        } else {
            None
        };

        details.push(AuditLogDetail {
            log,
            user_full_name: user_name,
            supervisor_full_name: spv_name,
        });
    }

    Ok(details)
}
