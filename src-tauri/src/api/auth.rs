use crate::models::user::{LoginRequest, SupervisorPinRequest};
use crate::services::audit_service::list_audit_logs;
use crate::services::auth_service::{authenticate_user, get_user_profile, verify_supervisor_pin};
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
        .route("/supervisor-approve", post(supervisor_approve_handler))
        .route("/me", get(me_handler))
        .route("/audit-logs", get(audit_logs_handler))
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
struct AuditQuery {
    limit: Option<i64>,
}

async fn audit_logs_handler(
    State(pool): State<PgPool>,
    Query(query): Query<AuditQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(100);
    match list_audit_logs(&pool, limit).await {
        Ok(logs) => (StatusCode::OK, Json(json!({ "success": true, "data": logs }))),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": err })),
        ),
    }
}
