use crate::models::audit::{
    AuditLog, AuditLogDetail, AuditLogPaginatedResponse, AuditLogSettingsResponse,
};
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{PgPool, QueryBuilder, Postgres};
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

#[derive(Debug, Default)]
pub struct AuditLogFilter {
    pub page: i64,
    pub limit: i64,
    pub search: Option<String>,
    pub method: Option<String>,
    pub entity_name: Option<String>,
    pub status_type: Option<String>, // "all", "success", "error"
}

pub async fn list_audit_logs_advanced(
    pool: &PgPool,
    filter: AuditLogFilter,
) -> Result<AuditLogPaginatedResponse, String> {
    let page = if filter.page < 1 { 1 } else { filter.page };
    let limit = if filter.limit < 1 { 20 } else { filter.limit.min(100) };
    let offset = (page - 1) * limit;

    // 1. Build Query Count
    let mut count_builder: QueryBuilder<Postgres> = QueryBuilder::new("SELECT COUNT(*) FROM audit_logs a WHERE 1=1 ");
    apply_filters(&mut count_builder, &filter);

    let total: i64 = count_builder
        .build_query_scalar()
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Gagal menghitung total audit log: {}", e))?;

    // 2. Build Query Data dengan LEFT JOIN untuk performa tanpa N+1 query
    let mut data_builder: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT 
            a.id, a.user_id, a.authorized_by, 
            COALESCE(a.terminal_id, 'SYSTEM') as terminal_id,
            a.action, a.entity_name, a.entity_id, 
            a.old_values, a.new_values, a.ip_address,
            a.http_method, a.endpoint, a.status_code, a.execution_time_ms,
            a.user_agent, a.user_name, a.role_name, a.created_at,
            u.full_name as user_full_name,
            spv.full_name as supervisor_full_name
        FROM audit_logs a
        LEFT JOIN users u ON a.user_id = u.id
        LEFT JOIN users spv ON a.authorized_by = spv.id
        WHERE 1=1 ",
    );
    apply_filters(&mut data_builder, &filter);

    data_builder.push(" ORDER BY a.created_at DESC ");
    data_builder.push(" LIMIT ");
    data_builder.push_bind(limit);
    data_builder.push(" OFFSET ");
    data_builder.push_bind(offset);

    #[derive(sqlx::FromRow)]
    struct RowWithNames {
        id: i64,
        user_id: Option<Uuid>,
        authorized_by: Option<Uuid>,
        terminal_id: String,
        action: String,
        entity_name: String,
        entity_id: Option<String>,
        old_values: Option<Value>,
        new_values: Option<Value>,
        ip_address: Option<String>,
        http_method: Option<String>,
        endpoint: Option<String>,
        status_code: Option<i32>,
        execution_time_ms: Option<i32>,
        user_agent: Option<String>,
        user_name: Option<String>,
        role_name: Option<String>,
        created_at: DateTime<Utc>,
        user_full_name: Option<String>,
        supervisor_full_name: Option<String>,
    }

    let rows: Vec<RowWithNames> = data_builder
        .build_query_as()
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Gagal mengambil data audit logs: {}", e))?;

    let items = rows
        .into_iter()
        .map(|r| {
            let log_user_name = r.user_name.clone();
            AuditLogDetail {
                log: AuditLog {
                    id: r.id,
                    user_id: r.user_id,
                    authorized_by: r.authorized_by,
                    terminal_id: r.terminal_id,
                    action: r.action,
                    entity_name: r.entity_name,
                    entity_id: r.entity_id,
                    old_values: r.old_values,
                    new_values: r.new_values,
                    ip_address: r.ip_address,
                    http_method: r.http_method,
                    endpoint: r.endpoint,
                    status_code: r.status_code,
                    execution_time_ms: r.execution_time_ms,
                    user_agent: r.user_agent,
                    user_name: r.user_name,
                    role_name: r.role_name,
                    created_at: r.created_at,
                },
                user_full_name: r.user_full_name.or(log_user_name),
                supervisor_full_name: r.supervisor_full_name,
            }
        })
        .collect();

    // 3. Ambil nilai retensi hari saat ini
    let retention_days: i32 = sqlx::query_scalar::<_, i32>(
        "SELECT COALESCE(audit_log_retention_days, 30) FROM receipt_settings LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .unwrap_or(Some(30))
    .unwrap_or(30);

    Ok(AuditLogPaginatedResponse {
        items,
        total,
        page,
        limit,
        retention_days,
    })
}

fn apply_filters(builder: &mut QueryBuilder<Postgres>, filter: &AuditLogFilter) {
    if let Some(ref search) = filter.search {
        let pattern = format!("%{}%", search.trim());
        builder.push(" AND (a.action ILIKE ");
        builder.push_bind(pattern.clone());
        builder.push(" OR a.entity_name ILIKE ");
        builder.push_bind(pattern.clone());
        builder.push(" OR a.endpoint ILIKE ");
        builder.push_bind(pattern.clone());
        builder.push(" OR a.user_name ILIKE ");
        builder.push_bind(pattern.clone());
        builder.push(" OR a.terminal_id ILIKE ");
        builder.push_bind(pattern);
        builder.push(") ");
    }

    if let Some(ref m) = filter.method {
        let m_clean = m.trim().to_uppercase();
        if !m_clean.is_empty() && m_clean != "ALL" {
            builder.push(" AND a.http_method = ");
            builder.push_bind(m_clean);
        }
    }

    if let Some(ref ent) = filter.entity_name {
        let ent_clean = ent.trim().to_lowercase();
        if !ent_clean.is_empty() && ent_clean != "all" {
            builder.push(" AND LOWER(a.entity_name) = ");
            builder.push_bind(ent_clean);
        }
    }

    if let Some(ref st) = filter.status_type {
        match st.to_lowercase().as_str() {
            "success" => {
                builder.push(" AND (a.status_code IS NULL OR a.status_code < 400) ");
            }
            "error" => {
                builder.push(" AND (a.status_code >= 400) ");
            }
            _ => {}
        }
    }
}

pub async fn list_audit_logs(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<AuditLogDetail>, String> {
    let res = list_audit_logs_advanced(
        pool,
        AuditLogFilter {
            page: 1,
            limit,
            search: None,
            method: None,
            entity_name: None,
            status_type: None,
        },
    )
    .await?;
    Ok(res.items)
}

pub async fn get_audit_settings(pool: &PgPool) -> Result<AuditLogSettingsResponse, String> {
    let retention_days: i32 = sqlx::query_scalar::<_, i32>(
        "SELECT COALESCE(audit_log_retention_days, 30) FROM receipt_settings LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .unwrap_or(Some(30))
    .unwrap_or(30);

    let total_logs: i64 = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM audit_logs")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let oldest_log_date: Option<DateTime<Utc>> = sqlx::query_scalar::<_, Option<DateTime<Utc>>>(
        "SELECT MIN(created_at) FROM audit_logs",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(None);

    Ok(AuditLogSettingsResponse {
        retention_days,
        total_logs,
        oldest_log_date,
    })
}

pub async fn update_audit_settings(pool: &PgPool, retention_days: i32) -> Result<(), String> {
    if retention_days < 0 {
        return Err("Nilai retensi hari tidak boleh negatif".to_string());
    }

    sqlx::query("UPDATE receipt_settings SET audit_log_retention_days = $1")
        .bind(retention_days)
        .execute(pool)
        .await
        .map_err(|e| format!("Gagal memperbarui pengaturan retensi audit log: {}", e))?;

    Ok(())
}

pub async fn cleanup_expired_audit_logs(
    pool: &PgPool,
    custom_days: Option<i32>,
) -> Result<u64, String> {
    let result = if let Some(days) = custom_days {
        if days <= 0 {
            // Nilai 0 atau negatif pada pembersihan manual berarti kosongkan seluruh riwayat log
            sqlx::query("DELETE FROM audit_logs")
                .execute(pool)
                .await
        } else {
            sqlx::query(
                "DELETE FROM audit_logs WHERE created_at < CURRENT_TIMESTAMP - ($1 || ' days')::INTERVAL",
            )
            .bind(days)
            .execute(pool)
            .await
        }
    } else {
        let current_days: i32 = sqlx::query_scalar::<_, i32>(
            "SELECT COALESCE(audit_log_retention_days, 30) FROM receipt_settings LIMIT 1",
        )
        .fetch_optional(pool)
        .await
        .unwrap_or(Some(30))
        .unwrap_or(30);

        if current_days <= 0 {
            return Ok(0); // 0 berarti retensi permanen pada scheduler otomatis (jangan dihapus)
        }

        sqlx::query(
            "DELETE FROM audit_logs WHERE created_at < CURRENT_TIMESTAMP - ($1 || ' days')::INTERVAL",
        )
        .bind(current_days)
        .execute(pool)
        .await
    }
    .map_err(|e| format!("Gagal membersihkan log audit: {}", e))?;

    Ok(result.rows_affected())
}
