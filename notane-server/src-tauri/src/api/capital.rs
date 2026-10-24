use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::capital::{
    CancelCapitalDepositRequest, CapitalDepositFilter, CreateCapitalDepositRequest,
};
use crate::services::capital_service::{
    cancel_capital_deposit, create_capital_deposit, get_capital_deposit_by_id,
    get_capital_eligible_accounts, get_capital_summary, list_capital_deposits,
};
use crate::utils::jwt::require_any_permission;

pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/capital", get(list_capital_deposits_handler).post(create_capital_deposit_handler))
        .route("/capital/summary", get(get_capital_summary_handler))
        .route("/capital/accounts", get(get_capital_accounts_handler))
        .route("/capital/{id}", get(get_capital_deposit_handler))
        .route("/capital/{id}/cancel", post(cancel_capital_deposit_handler))
}

async fn list_capital_deposits_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(query): Query<CapitalDepositFilter>,
) -> impl IntoResponse {
    if let Err(resp) = require_any_permission(
        &pool,
        &headers,
        &["capital:read", "accounting:read", "master:read"],
    )
    .await
    {
        return resp.into_response();
    }

    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(20);

    match list_capital_deposits(&pool, query).await {
        Ok((items, total)) => {
            let total_pages = if total == 0 { 0 } else { (total + limit - 1) / limit };
            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "data": items,
                    "total": total,
                    "page": page,
                    "limit": limit,
                    "total_pages": total_pages,
                })),
            )
                .into_response()
        }
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "message": err
            })),
        )
            .into_response(),
    }
}

async fn get_capital_summary_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    if let Err(resp) = require_any_permission(
        &pool,
        &headers,
        &["capital:read", "accounting:read", "master:read"],
    )
    .await
    {
        return resp.into_response();
    }

    let outlet_id = params
        .get("outlet_id")
        .and_then(|s| Uuid::parse_str(s).ok());

    match get_capital_summary(&pool, outlet_id).await {
        Ok(summary) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "data": summary
            })),
        )
            .into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "message": err
            })),
        )
            .into_response(),
    }
}

async fn get_capital_accounts_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(resp) = require_any_permission(
        &pool,
        &headers,
        &[
            "capital:read",
            "capital:create",
            "accounting:read",
            "accounting:write",
        ],
    )
    .await
    {
        return resp.into_response();
    }

    match get_capital_eligible_accounts(&pool).await {
        Ok(data) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "data": data
            })),
        )
            .into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "message": err
            })),
        )
            .into_response(),
    }
}

async fn get_capital_deposit_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    if let Err(resp) = require_any_permission(
        &pool,
        &headers,
        &["capital:read", "accounting:read", "master:read"],
    )
    .await
    {
        return resp.into_response();
    }

    match get_capital_deposit_by_id(&pool, id).await {
        Ok(Some(item)) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "data": item
            })),
        )
            .into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "message": "Transaksi setoran modal tidak ditemukan."
            })),
        )
            .into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "message": err
            })),
        )
            .into_response(),
    }
}

async fn create_capital_deposit_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<CreateCapitalDepositRequest>,
) -> impl IntoResponse {
    let claims = match require_any_permission(
        &pool,
        &headers,
        &["capital:create", "accounting:write"],
    )
    .await
    {
        Ok(c) => c,
        Err(resp) => return resp.into_response(),
    };

    let user_id = Some(claims.sub);

    match create_capital_deposit(&pool, payload, user_id).await {
        Ok(created) => (
            StatusCode::CREATED,
            Json(json!({
                "success": true,
                "message": "Setoran modal usaha berhasil dibukukan.",
                "data": created
            })),
        )
            .into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "message": err
            })),
        )
            .into_response(),
    }
}

async fn cancel_capital_deposit_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<CancelCapitalDepositRequest>,
) -> impl IntoResponse {
    let claims = match require_any_permission(
        &pool,
        &headers,
        &["capital:delete", "accounting:write"],
    )
    .await
    {
        Ok(c) => c,
        Err(resp) => return resp.into_response(),
    };

    let user_id = Some(claims.sub);

    match cancel_capital_deposit(&pool, id, payload, user_id).await {
        Ok(updated) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "message": "Transaksi setoran modal berhasil dibatalkan dan jurnal pembalik telah dibukukan.",
                "data": updated
            })),
        )
            .into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "message": err
            })),
        )
            .into_response(),
    }
}
