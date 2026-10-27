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

use crate::models::flash_sales::CreateFlashSaleRequest;
use crate::utils::jwt::{require_any_permission, require_license_feature, verify_jwt};
use crate::services::flash_sale_service::{
    create_flash_sale, delete_flash_sale, get_active_promos_for_outlet, get_flash_sale_detail,
    list_flash_sales, update_flash_sale,
};

pub fn flash_sale_routes() -> Router<PgPool> {
    Router::new()
        .route("/flash-sales", get(list_flash_sales_handler).post(create_flash_sale_handler))
        .route("/flash-sales/active-promos", get(active_promos_handler))
        .route(
            "/flash-sales/{id}",
            get(get_flash_sale_detail_handler)
                .put(update_flash_sale_handler)
                .delete(delete_flash_sale_handler),
        )
}

#[allow(dead_code)]
fn get_user_id(headers: &HeaderMap) -> Result<Uuid, (StatusCode, Json<serde_json::Value>)> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| if h.starts_with("Bearer ") { Some(&h[7..]) } else { None })
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "success": false, "message": "Autentikasi dibutuhkan. Silakan login." })),
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

#[derive(Debug, Deserialize)]
pub struct FlashSaleQueryFilter {
    pub status: Option<String>,
    pub search: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

async fn list_flash_sales_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(query): Query<FlashSaleQueryFilter>,
) -> impl IntoResponse {
    if let Err(resp) = require_license_feature(&pool, "promotions", "Promosi & Flash Sale").await {
        return resp.into_response();
    }

    if let Err(resp) = require_any_permission(&pool, &headers, &["promotions:read", "promo:read", "master:read", "pos:checkout"]).await {
        return resp.into_response();
    }

    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(10).max(1);

    match list_flash_sales(
        &pool,
        query.status,
        query.search,
        Some(page),
        Some(page_size),
    )
    .await
    {
        Ok((items, total_items)) => {
            let total_pages = if total_items > 0 {
                ((total_items as f64) / (page_size as f64)).ceil() as i64
            } else {
                1
            };
            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "data": items,
                    "total_items": total_items,
                    "page": page,
                    "page_size": page_size,
                    "total_pages": total_pages,
                })),
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

async fn get_flash_sale_detail_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    if let Err(resp) = require_license_feature(&pool, "promotions", "Promosi & Flash Sale").await {
        return resp.into_response();
    }

    if let Err(resp) = require_any_permission(&pool, &headers, &["promotions:read", "promo:read", "master:read", "pos:checkout"]).await {
        return resp.into_response();
    }

    match get_flash_sale_detail(&pool, id).await {
        Ok(detail) => (
            StatusCode::OK,
            Json(json!({ "success": true, "data": detail })),
        )
            .into_response(),
        Err(err) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

async fn create_flash_sale_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<CreateFlashSaleRequest>,
) -> impl IntoResponse {
    if let Err(resp) = require_license_feature(&pool, "promotions", "Promosi & Flash Sale").await {
        return resp.into_response();
    }

    // 1. Pengecekan Permission di Backend (Fail-Closed)
    let claims = match require_any_permission(&pool, &headers, &["promotions:create", "promotions:write", "promo:write", "master:write"]).await {
        Ok(c) => c,
        Err(resp) => return resp.into_response(),
    };

    // 2. Validasi Semantik Input Sisi Backend
    if payload.name.trim().len() < 3 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "Nama promosi flash sale wajib diisi minimal 3 karakter" })),
        )
            .into_response();
    }

    if payload.start_time >= payload.end_time {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "Waktu mulai flash sale harus lebih awal daripada waktu selesai" })),
        )
            .into_response();
    }

    if payload.items.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "Daftar produk promo flash sale tidak boleh kosong" })),
        )
            .into_response();
    }

    let user_id = Some(claims.sub);
    match create_flash_sale(&pool, payload, user_id).await {
        Ok(detail) => (
            StatusCode::CREATED,
            Json(json!({
                "success": true,
                "message": "Program flash sale berhasil dibuat",
                "data": detail
            })),
        )
            .into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

async fn update_flash_sale_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<CreateFlashSaleRequest>,
) -> impl IntoResponse {
    if let Err(resp) = require_license_feature(&pool, "promotions", "Promosi & Flash Sale").await {
        return resp.into_response();
    }

    // 1. Pengecekan Permission di Backend (Fail-Closed)
    let claims = match require_any_permission(&pool, &headers, &["promotions:update", "promotions:write", "promo:write", "master:write"]).await {
        Ok(c) => c,
        Err(resp) => return resp.into_response(),
    };

    // 2. Validasi Semantik Input Sisi Backend
    if payload.name.trim().len() < 3 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "Nama promosi flash sale wajib diisi minimal 3 karakter" })),
        )
            .into_response();
    }

    if payload.start_time >= payload.end_time {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "Waktu mulai flash sale harus lebih awal daripada waktu selesai" })),
        )
            .into_response();
    }

    let user_id = Some(claims.sub);
    match update_flash_sale(&pool, id, payload, user_id).await {
        Ok(detail) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "message": "Program flash sale berhasil diperbarui",
                "data": detail
            })),
        )
            .into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

async fn delete_flash_sale_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    if let Err(resp) = require_license_feature(&pool, "promotions", "Promosi & Flash Sale").await {
        return resp.into_response();
    }

    // 1. Pengecekan Permission di Backend (Fail-Closed)
    if let Err(resp) = require_any_permission(&pool, &headers, &["promotions:delete", "promotions:write", "promo:write", "master:write"]).await {
        return resp.into_response();
    }

    match delete_flash_sale(&pool, id).await {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "message": "Program flash sale berhasil dihapus"
            })),
        )
            .into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct ActivePromoQuery {
    pub outlet_id: Option<Uuid>,
}

async fn active_promos_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(query): Query<ActivePromoQuery>,
) -> impl IntoResponse {
    if !crate::services::license_service::LicenseService::has_feature(&pool, "promotions").await {
        return (
            StatusCode::OK,
            Json(json!({ "success": true, "data": [] })),
        )
            .into_response();
    }

    let effective_outlet_id = crate::utils::jwt::resolve_effective_outlet_id(&pool, &headers, query.outlet_id).await;
    match get_active_promos_for_outlet(&pool, effective_outlet_id).await {
        Ok(promos) => (
            StatusCode::OK,
            Json(json!({ "success": true, "data": promos })),
        )
            .into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}
