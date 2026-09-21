use crate::models::sales::{CheckoutRequest, VoidSaleRequest};
use crate::models::shift::{CloseShiftRequest, OpenShiftRequest};
use crate::services::pos_service::{
    close_shift, get_current_shift, get_sale_detail, get_sales_stats, list_sales, lookup_product,
    open_shift, void_sale,
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

pub fn pos_routes() -> Router<PgPool> {
    Router::new()
        .route("/shifts/open", post(open_shift_handler))
        .route("/shifts/close", post(close_shift_handler))
        .route("/shifts/current", get(current_shift_handler))
        .route("/products/barcode/{code}", get(barcode_lookup_handler))
        .route("/sales/checkout", post(checkout_handler))
        .route("/sales", get(list_sales_handler))
        .route("/sales/stats", get(sales_stats_handler))
        .route("/sales/{id}", get(get_sale_detail_handler))
        .route("/sales/{id}/void", post(void_sale_handler))
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

async fn open_shift_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<OpenShiftRequest>,
) -> impl IntoResponse {
    let user_id = match get_user_id(&headers) {
        Ok(id) => id,
        Err(resp) => return resp.into_response(),
    };

    match open_shift(&pool, user_id, payload).await {
        Ok(shift) => (StatusCode::OK, Json(json!({ "success": true, "data": shift }))).into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

async fn close_shift_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<CloseShiftRequest>,
) -> impl IntoResponse {
    let user_id = match get_user_id(&headers) {
        Ok(id) => id,
        Err(resp) => return resp.into_response(),
    };

    match close_shift(&pool, user_id, payload).await {
        Ok(summary) => (StatusCode::OK, Json(json!({ "success": true, "data": summary }))).into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

async fn current_shift_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user_id = match get_user_id(&headers) {
        Ok(id) => id,
        Err(resp) => return resp.into_response(),
    };

    match get_current_shift(&pool, user_id).await {
        Ok(shift) => (StatusCode::OK, Json(json!({ "success": true, "data": shift }))).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
struct BarcodeQuery {
    warehouse_id: Option<Uuid>,
    outlet_id: Option<Uuid>,
}

async fn barcode_lookup_handler(
    State(pool): State<PgPool>,
    Path(code): Path<String>,
    Query(query): Query<BarcodeQuery>,
) -> impl IntoResponse {
    match lookup_product(&pool, &code, query.warehouse_id, query.outlet_id).await {
        Ok(prod) => (StatusCode::OK, Json(json!({ "success": true, "data": prod }))).into_response(),
        Err(err) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

async fn checkout_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<CheckoutRequest>,
) -> impl IntoResponse {
    let user_id = match get_user_id(&headers) {
        Ok(id) => id,
        Err(resp) => return resp.into_response(),
    };

    match crate::services::pos_service::checkout(&pool, user_id, payload).await {
        Ok(res) => (StatusCode::OK, Json(json!({ "success": true, "data": res }))).into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
struct ListSalesQuery {
    shift_id: Option<Uuid>,
    outlet_id: Option<Uuid>,
    limit: Option<i64>,
}

async fn list_sales_handler(
    State(pool): State<PgPool>,
    Query(query): Query<ListSalesQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(50);
    match list_sales(&pool, query.shift_id, query.outlet_id, limit).await {
        Ok(sales) => (StatusCode::OK, Json(json!({ "success": true, "data": sales }))).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

async fn sales_stats_handler(State(pool): State<PgPool>) -> impl IntoResponse {
    match get_sales_stats(&pool).await {
        Ok(stats) => (StatusCode::OK, Json(json!({ "success": true, "data": stats }))).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

async fn get_sale_detail_handler(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match get_sale_detail(&pool, id).await {
        Ok(sale_data) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "data": sale_data
            })),
        )
            .into_response(),
        Err(err) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

async fn void_sale_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<VoidSaleRequest>,
) -> impl IntoResponse {
    let user_id = match get_user_id(&headers) {
        Ok(id) => id,
        Err(resp) => return resp.into_response(),
    };

    match void_sale(&pool, id, user_id, payload).await {
        Ok(msg) => (StatusCode::OK, Json(json!({ "success": true, "message": msg }))).into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}
