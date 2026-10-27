use crate::models::user::{
    LoginRequest, SupervisorBiometricApproveRequest, SupervisorPinRequest,
};
use crate::services::auth_service::{
    authenticate_user, get_user_profile, list_store_managers, list_supervisors,
    verify_supervisor_biometric, verify_supervisor_pin,
};
use crate::utils::jwt::verify_jwt;
use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;

pub fn auth_routes() -> Router<PgPool> {
    Router::new()
        .route("/login", post(login_handler))
        .route("/supervisors", get(list_supervisors_handler))
        .route("/store-managers", get(list_store_managers_handler))
        .route("/supervisor-approve", post(supervisor_approve_handler))
        .route("/supervisor-biometric-approve", post(supervisor_biometric_approve_handler))
        .route("/biometric/challenge", get(biometric_challenge_handler))
        .route("/me", get(me_handler))
        .route("/audit-logs", get(audit_logs_handler))
        .route("/audit-logs/settings", get(get_audit_settings_handler).put(update_audit_settings_handler))
        .route("/audit-logs/cleanup", post(cleanup_audit_logs_handler))
}

async fn login_handler(
    State(pool): State<PgPool>,
    Json(payload): Json<LoginRequest>,
) -> impl IntoResponse {
    match authenticate_user(&pool, payload).await {
        Ok(res) => (StatusCode::OK, Json(json!({ "success": true, "data": res }))),
        Err(err) => (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "success": false, "message": err })),
        ),
    }
}

async fn supervisor_approve_handler(
    State(pool): State<PgPool>,
    Json(payload): Json<SupervisorPinRequest>,
) -> impl IntoResponse {
    match verify_supervisor_pin(&pool, payload).await {
        Ok(res) => (StatusCode::OK, Json(json!({ "success": true, "data": res }))),
        Err(err) => (
            StatusCode::FORBIDDEN,
            Json(json!({ "success": false, "message": err })),
        ),
    }
}

async fn supervisor_biometric_approve_handler(
    State(pool): State<PgPool>,
    Json(payload): Json<SupervisorBiometricApproveRequest>,
) -> impl IntoResponse {
    match verify_supervisor_biometric(&pool, payload).await {
        Ok(res) => (StatusCode::OK, Json(json!({ "success": true, "data": res }))),
        Err(err) => (
            StatusCode::FORBIDDEN,
            Json(json!({ "success": false, "message": err })),
        ),
    }
}

async fn biometric_challenge_handler() -> impl IntoResponse {
    // Generate secure 32-byte cryptographic challenge
    use rand::RngCore;
    let mut challenge = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut challenge);
    let challenge_b64 = base64::Engine::encode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        &challenge,
    );
    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "data": {
                "challenge": challenge_b64,
                "rp_name": "Notane Retail POS",
                "timeout": 60000
            }
        })),
    )
}

async fn me_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let auth_header = match headers.get("Authorization").and_then(|h| h.to_str().ok()) {
        Some(h) if h.starts_with("Bearer ") => &h[7..],
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "success": false, "message": "Token tidak ditemukan" })),
            )
        }
    };

    match verify_jwt(auth_header) {
        Ok(claims) => match get_user_profile(&pool, claims.sub).await {
            Ok(profile) => (StatusCode::OK, Json(json!({ "success": true, "data": profile }))),
            Err(err) => (
                StatusCode::NOT_FOUND,
                Json(json!({ "success": false, "message": err })),
            ),
        },
        Err(err) => (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "success": false, "message": err })),
        ),
    }
}

#[derive(Deserialize)]
pub struct AuditQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub search: Option<String>,
    pub method: Option<String>,
    pub entity_name: Option<String>,
    pub status_type: Option<String>,
}

async fn audit_logs_handler(
    State(pool): State<PgPool>,
    Query(query): Query<AuditQuery>,
) -> impl IntoResponse {
    if let Err(resp) = crate::utils::jwt::require_license_feature(&pool, "audit_log", "Audit Trail & Log Keamanan").await {
        return resp.into_response();
    }

    let filter = crate::services::audit_service::AuditLogFilter {
        page: query.page.unwrap_or(1),
        limit: query.limit.unwrap_or(20),
        search: query.search,
        method: query.method,
        entity_name: query.entity_name,
        status_type: query.status_type,
    };
    match crate::services::audit_service::list_audit_logs_advanced(&pool, filter).await {
        Ok(res) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "data": res.items,
                "total": res.total,
                "page": res.page,
                "limit": res.limit,
                "retention_days": res.retention_days
            })),
        )
            .into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

async fn get_audit_settings_handler(
    State(pool): State<PgPool>,
) -> impl IntoResponse {
    if let Err(resp) = crate::utils::jwt::require_license_feature(&pool, "audit_log", "Audit Trail & Log Keamanan").await {
        return resp.into_response();
    }

    match crate::services::audit_service::get_audit_settings(&pool).await {
        Ok(data) => (StatusCode::OK, Json(json!({ "success": true, "data": data }))).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

async fn update_audit_settings_handler(
    State(pool): State<PgPool>,
    Json(payload): Json<crate::models::audit::UpdateAuditLogSettingsRequest>,
) -> impl IntoResponse {
    if let Err(resp) = crate::utils::jwt::require_license_feature(&pool, "audit_log", "Audit Trail & Log Keamanan").await {
        return resp.into_response();
    }

    match crate::services::audit_service::update_audit_settings(&pool, payload.retention_days).await {
        Ok(_) => (StatusCode::OK, Json(json!({ "success": true, "message": "Pengaturan retensi log audit berhasil disimpan" }))).into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct CleanupQuery {
    pub days: Option<i32>,
}

async fn cleanup_audit_logs_handler(
    State(pool): State<PgPool>,
    Query(query): Query<CleanupQuery>,
) -> impl IntoResponse {
    if let Err(resp) = crate::utils::jwt::require_license_feature(&pool, "audit_log", "Audit Trail & Log Keamanan").await {
        return resp.into_response();
    }

    match crate::services::audit_service::cleanup_expired_audit_logs(&pool, query.days).await {
        Ok(deleted_count) => {
            let message = if deleted_count > 0 {
                format!("Berhasil membersihkan {} baris log audit", deleted_count)
            } else if query.days == Some(0) {
                "Tabel riwayat log sudah kosong".to_string()
            } else {
                format!(
                    "Tidak ada log yang dihapus karena tidak ditemukan data yang berumur lebih dari {} hari",
                    query.days.unwrap_or(30)
                )
            };
            (
                StatusCode::OK,
                Json(json!({ "success": true, "message": message, "deleted_count": deleted_count })),
            )
                .into_response()
        }
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}


#[derive(Debug, Deserialize)]
pub struct ListSupervisorsQuery {
    pub outlet_id: Option<uuid::Uuid>,
    pub search: Option<String>,
}

async fn list_supervisors_handler(
    State(pool): State<PgPool>,
    Query(query): Query<ListSupervisorsQuery>,
) -> impl IntoResponse {
    match list_supervisors(&pool, query.outlet_id, query.search).await {
        Ok(list) => (StatusCode::OK, Json(json!({ "success": true, "data": list }))).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

async fn list_store_managers_handler(
    State(pool): State<PgPool>,
) -> impl IntoResponse {
    match list_store_managers(&pool).await {
        Ok(list) => (StatusCode::OK, Json(json!({ "success": true, "data": list }))).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}
