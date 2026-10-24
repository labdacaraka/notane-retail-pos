use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::dividend::{CreateDividendRequest, DividendFilter};
use crate::services::dividend_service::{
    cancel_dividend, create_dividend, get_dividend_by_id, get_dividend_summary, list_dividends,
};
use crate::utils::jwt::require_any_permission;

pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/dividends", get(list_dividends_handler).post(create_dividend_handler))
        .route("/dividends/summary", get(get_dividend_summary_handler))
        .route("/dividends/{id}", get(get_dividend_by_id_handler))
        .route("/dividends/{id}/cancel", axum::routing::post(cancel_dividend_handler))
}

async fn list_dividends_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(query): Query<DividendFilter>,
) -> impl IntoResponse {
    if let Err(resp) = require_any_permission(
        &pool,
        &headers,
        &["accounting:dividend:read", "accounting:read", "master:read"],
    )
    .await
    {
        return resp.into_response();
    }

    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(20);

    match list_dividends(&pool, query).await {
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

async fn get_dividend_summary_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(resp) = require_any_permission(
        &pool,
        &headers,
        &["accounting:dividend:read", "accounting:read", "master:read"],
    )
    .await
    {
        return resp.into_response();
    }

    match get_dividend_summary(&pool).await {
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

async fn get_dividend_by_id_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    if let Err(resp) = require_any_permission(
        &pool,
        &headers,
        &["accounting:dividend:read", "accounting:read", "master:read"],
    )
    .await
    {
        return resp.into_response();
    }

    match get_dividend_by_id(&pool, id).await {
        Ok(item) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "data": item
            })),
        )
            .into_response(),
        Err(err) => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "message": err
            })),
        )
            .into_response(),
    }
}

async fn create_dividend_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<CreateDividendRequest>,
) -> impl IntoResponse {
    let claims = match require_any_permission(
        &pool,
        &headers,
        &["accounting:dividend:create", "accounting:write"],
    )
    .await
    {
        Ok(c) => c,
        Err(resp) => return resp.into_response(),
    };

    let user_id = Some(claims.sub);

    match create_dividend(&pool, payload, user_id).await {
        Ok(created) => (
            StatusCode::CREATED,
            Json(json!({
                "success": true,
                "message": "Pembagian dividen berhasil diproses dan dijurnal ke Laba Ditahan.",
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

async fn cancel_dividend_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let claims = match require_any_permission(
        &pool,
        &headers,
        &["accounting:dividend:delete", "accounting:write"],
    )
    .await
    {
        Ok(c) => c,
        Err(resp) => return resp.into_response(),
    };

    let user_id = Some(claims.sub);

    match cancel_dividend(&pool, id, user_id).await {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "message": "Pembagian dividen dan jurnal terkait berhasil dibatalkan."
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
