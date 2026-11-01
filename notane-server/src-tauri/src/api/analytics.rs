use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde_json::json;
use sqlx::PgPool;

use crate::models::analytics::{AnalyticsPaginationQuery, AnalyticsQuery};
use crate::services::analytics_service::{
    get_analytics_overview, get_daily_ledger_analytics, get_expenses_analytics,
    get_other_income_analytics, get_products_analytics,
};
use crate::utils::jwt::{require_any_permission, require_license_feature};

pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/analytics/overview", get(get_analytics_overview_handler))
        .route("/analytics/products", get(get_products_analytics_handler))
        .route("/analytics/expenses", get(get_expenses_analytics_handler))
        .route("/analytics/other-income", get(get_other_income_analytics_handler))
        .route("/analytics/daily-ledger", get(get_daily_ledger_analytics_handler))
}

async fn get_analytics_overview_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(query): Query<AnalyticsQuery>,
) -> impl IntoResponse {
    if let Err(resp) = require_license_feature(&pool, "analytics", "Statistik & Analisis Bisnis").await {
        return resp.into_response();
    }

    if let Err(resp) = require_any_permission(
        &pool,
        &headers,
        &["analytics:read", "report:read", "report:view", "accounting:read", "sales:read"],
    )
    .await
    {
        return resp.into_response();
    }

    match get_analytics_overview(&pool, query).await {
        Ok(data) => (StatusCode::OK, Json(json!({ "success": true, "data": data }))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": e })),
        )
            .into_response(),
    }
}

async fn get_products_analytics_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(query): Query<AnalyticsPaginationQuery>,
) -> impl IntoResponse {
    if let Err(resp) = require_license_feature(&pool, "analytics", "Statistik & Analisis Bisnis").await {
        return resp.into_response();
    }

    if let Err(resp) = require_any_permission(
        &pool,
        &headers,
        &["analytics:read", "report:read", "report:view", "accounting:read", "sales:read"],
    )
    .await
    {
        return resp.into_response();
    }

    match get_products_analytics(&pool, query).await {
        Ok(data) => (StatusCode::OK, Json(json!({ "success": true, "data": data }))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": e })),
        )
            .into_response(),
    }
}

async fn get_expenses_analytics_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(query): Query<AnalyticsPaginationQuery>,
) -> impl IntoResponse {
    if let Err(resp) = require_license_feature(&pool, "analytics", "Statistik & Analisis Bisnis").await {
        return resp.into_response();
    }

    if let Err(resp) = require_any_permission(
        &pool,
        &headers,
        &["analytics:read", "report:read", "report:view", "accounting:read", "sales:read"],
    )
    .await
    {
        return resp.into_response();
    }

    match get_expenses_analytics(&pool, query).await {
        Ok(data) => (StatusCode::OK, Json(json!({ "success": true, "data": data }))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": e })),
        )
            .into_response(),
    }
}

async fn get_other_income_analytics_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(query): Query<AnalyticsPaginationQuery>,
) -> impl IntoResponse {
    if let Err(resp) = require_license_feature(&pool, "analytics", "Statistik & Analisis Bisnis").await {
        return resp.into_response();
    }

    if let Err(resp) = require_any_permission(
        &pool,
        &headers,
        &["analytics:read", "report:read", "report:view", "accounting:read", "sales:read"],
    )
    .await
    {
        return resp.into_response();
    }

    match get_other_income_analytics(&pool, query).await {
        Ok(data) => (StatusCode::OK, Json(json!({ "success": true, "data": data }))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": e })),
        )
            .into_response(),
    }
}

async fn get_daily_ledger_analytics_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(query): Query<AnalyticsPaginationQuery>,
) -> impl IntoResponse {
    if let Err(resp) = require_license_feature(&pool, "analytics", "Statistik & Analisis Bisnis").await {
        return resp.into_response();
    }

    if let Err(resp) = require_any_permission(
        &pool,
        &headers,
        &["analytics:read", "report:read", "report:view", "accounting:read", "sales:read"],
    )
    .await
    {
        return resp.into_response();
    }

    match get_daily_ledger_analytics(&pool, query).await {
        Ok(data) => (StatusCode::OK, Json(json!({ "success": true, "data": data }))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": e })),
        )
            .into_response(),
    }
}

