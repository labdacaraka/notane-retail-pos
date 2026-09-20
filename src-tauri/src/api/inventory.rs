use crate::models::inventory::{GoodsReceiptNoteRequest, StockAdjustmentRequest};
use crate::services::inventory_service::{
    get_purchase_invoice_detail, get_stock_ledger, list_purchase_invoices, list_purchase_orders,
    pay_purchase_invoice, process_goods_receipt, process_stock_adjustment,
};
use crate::utils::jwt::verify_jwt;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

pub fn inventory_routes() -> Router<PgPool> {
    Router::new()
        .route("/grn", post(goods_receipt_handler))
        .route("/adjustments", post(stock_adjustment_handler))
        .route("/stock-ledger/{product_id}", get(stock_ledger_handler))
        .route("/purchases", get(list_purchases_handler))
        .route("/purchases/{id}", get(get_purchase_detail_handler))
        .route("/purchases/{id}/pay", post(pay_purchase_handler))
        .route("/purchase-orders", get(list_purchase_orders_handler))
}

fn get_user_id(headers: &HeaderMap) -> Result<Uuid, (StatusCode, Json<serde_json::Value>)> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| if h.starts_with("Bearer ") { Some(&h[7..]) } else { None })
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "success": false, "message": "Autentikasi dibutuhkan" })),
            )
        })?;

    let claims = verify_jwt(auth_header).map_err(|e| {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "success": false, "message": e })),
        )
    })?;

    Ok(claims.sub)
}

async fn goods_receipt_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<GoodsReceiptNoteRequest>,
) -> impl IntoResponse {
    let user_id = match get_user_id(&headers) {
        Ok(id) => id,
        Err(resp) => return resp.into_response(),
    };

    match process_goods_receipt(&pool, user_id, payload).await {
        Ok(msg) => (StatusCode::OK, Json(json!({ "success": true, "message": msg }))).into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

async fn stock_adjustment_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<StockAdjustmentRequest>,
) -> impl IntoResponse {
    let user_id = match get_user_id(&headers) {
        Ok(id) => id,
        Err(resp) => return resp.into_response(),
    };

    match process_stock_adjustment(&pool, user_id, payload).await {
        Ok(msg) => (StatusCode::OK, Json(json!({ "success": true, "message": msg }))).into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
struct LedgerQuery {
    warehouse_id: Option<Uuid>,
}

async fn stock_ledger_handler(
    State(pool): State<PgPool>,
    Path(product_id): Path<Uuid>,
    Query(query): Query<LedgerQuery>,
) -> impl IntoResponse {
    match get_stock_ledger(&pool, product_id, query.warehouse_id).await {
        Ok(mutations) => (StatusCode::OK, Json(json!({ "success": true, "data": mutations }))).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

async fn list_purchases_handler(State(pool): State<PgPool>) -> impl IntoResponse {
    match list_purchase_invoices(&pool).await {
        Ok(invoices) => (StatusCode::OK, Json(json!({ "success": true, "data": invoices }))).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

async fn get_purchase_detail_handler(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match get_purchase_invoice_detail(&pool, id).await {
        Ok(inv) => (StatusCode::OK, Json(json!({ "success": true, "data": inv }))).into_response(),
        Err(err) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

async fn pay_purchase_handler(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match pay_purchase_invoice(&pool, id).await {
        Ok(msg) => (StatusCode::OK, Json(json!({ "success": true, "message": msg }))).into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

async fn list_purchase_orders_handler(State(pool): State<PgPool>) -> impl IntoResponse {
    match list_purchase_orders(&pool).await {
        Ok(orders) => (StatusCode::OK, Json(json!({ "success": true, "data": orders }))).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}
