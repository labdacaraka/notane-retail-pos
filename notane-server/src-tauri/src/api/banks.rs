use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::bank::{CreateBankAccountRequest, UpdateBankAccountRequest};
use crate::services::bank_service::{
    create_bank_account, delete_bank_account, get_bank_account_by_id, list_bank_accounts,
    update_bank_account,
};
use crate::utils::jwt::require_any_permission;

#[derive(Debug, Deserialize)]
pub struct BankFilterQuery {
    pub outlet_id: Option<Uuid>,
}

pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/banks", get(list_banks_handler).post(create_bank_handler))
        .route(
            "/banks/{id}",
            get(get_bank_handler)
                .put(update_bank_handler)
                .delete(delete_bank_handler),
        )
}

async fn list_banks_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(query): Query<BankFilterQuery>,
) -> impl IntoResponse {
    if let Err(resp) = require_any_permission(
        &pool,
        &headers,
        &["bank:read", "accounting:read", "master:read", "capital:read"],
    )
    .await
    {
        return resp.into_response();
    }

    match list_bank_accounts(&pool, query.outlet_id).await {
        Ok(banks) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "data": banks
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

async fn get_bank_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    if let Err(resp) = require_any_permission(
        &pool,
        &headers,
        &["bank:read", "accounting:read", "master:read"],
    )
    .await
    {
        return resp.into_response();
    }

    match get_bank_account_by_id(&pool, id).await {
        Ok(bank) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "data": bank
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

async fn create_bank_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<CreateBankAccountRequest>,
) -> impl IntoResponse {
    if let Err(resp) = require_any_permission(
        &pool,
        &headers,
        &["bank:create", "accounting:write", "master:write"],
    )
    .await
    {
        return resp.into_response();
    }

    match create_bank_account(&pool, payload).await {
        Ok(bank) => (
            StatusCode::CREATED,
            Json(json!({
                "success": true,
                "data": bank,
                "message": "Rekening bank penerimaan berhasil ditambahkan"
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

async fn update_bank_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateBankAccountRequest>,
) -> impl IntoResponse {
    if let Err(resp) = require_any_permission(
        &pool,
        &headers,
        &["bank:update", "accounting:write", "master:write"],
    )
    .await
    {
        return resp.into_response();
    }

    match update_bank_account(&pool, id, payload).await {
        Ok(bank) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "data": bank,
                "message": "Rekening bank penerimaan berhasil diperbarui"
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

async fn delete_bank_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    if let Err(resp) = require_any_permission(
        &pool,
        &headers,
        &["bank:delete", "accounting:write", "master:write"],
    )
    .await
    {
        return resp.into_response();
    }

    match delete_bank_account(&pool, id).await {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "message": "Rekening bank penerimaan berhasil dihapus"
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
