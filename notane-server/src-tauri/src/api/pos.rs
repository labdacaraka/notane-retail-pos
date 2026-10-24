use crate::models::sales::{CheckoutRequest, VoidSaleRequest};
use crate::models::shift::{CloseShiftRequest, OpenShiftRequest};
use crate::services::pos_service::{
    close_shift, get_current_shift, get_sale_detail, get_sales_stats, list_sales, lookup_product,
    open_shift, void_sale, SalesQueryFilter,
};
use crate::utils::jwt::{require_any_permission, verify_jwt};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use rust_decimal::prelude::*;
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

pub fn pos_routes() -> Router<PgPool> {
    Router::new()
        .route("/shifts/open", post(open_shift_handler))
        .route("/shifts/close", post(close_shift_handler))
        .route("/shifts/current", get(current_shift_handler))
        .route("/pos/current-shift", get(current_shift_handler))
        .route("/products/barcode/{code}", get(barcode_lookup_handler))
        .route("/sales/checkout", post(checkout_handler))
        .route("/sales/upload-proof", post(upload_payment_proof_handler))
        .route("/sales", get(list_sales_handler))
        .route("/sales/stats", get(sales_stats_handler))
        .route("/sales/{id}", get(get_sale_detail_handler))
        .route("/sales/{id}/stock-batches", get(get_sale_stock_batches_handler))
        .route("/sales/{id}/void", post(void_sale_handler))
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

async fn open_shift_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<OpenShiftRequest>,
) -> impl IntoResponse {
    // 1. Pengecekan Permission di Backend (Fail-Closed)
    let claims = match require_any_permission(&pool, &headers, &["shift:open"]).await {
        Ok(c) => c,
        Err(resp) => return resp.into_response(),
    };

    // 2. Validasi Input Semantik Sisi Backend
    if payload.opening_cash < Decimal::ZERO {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "Modal awal kas (opening cash) tidak boleh negatif" })),
        )
            .into_response();
    }

    let user_id = claims.sub;
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
    // 1. Pengecekan Permission di Backend (Fail-Closed)
    let claims = match require_any_permission(&pool, &headers, &["shift:close"]).await {
        Ok(c) => c,
        Err(resp) => return resp.into_response(),
    };

    // 2. Validasi Sisi Backend
    if payload.actual_cash < Decimal::ZERO {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "Uang fisik kas akhir tidak boleh bernilai negatif" })),
        )
            .into_response();
    }

    let user_id = claims.sub;
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
    let claims = match require_any_permission(&pool, &headers, &["shift:read", "shift:view", "shift:open", "shift:close", "pos:checkout", "expenses:read", "expenses:create"]).await {
        Ok(c) => c,
        Err(resp) => return resp.into_response(),
    };

    let user_id = claims.sub;
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
    headers: HeaderMap,
    Path(code): Path<String>,
    Query(query): Query<BarcodeQuery>,
) -> impl IntoResponse {
    let claims = match require_any_permission(&pool, &headers, &["pos:checkout", "master:read", "inventory:read"]).await {
        Ok(c) => c,
        Err(resp) => return resp.into_response(),
    };

    // Ambil shift aktif user kasir (jika ada)
    let active_shift = get_current_shift(&pool, claims.sub).await.ok().flatten();

    // Tentukan target outlet: prioritaskan shift aktif jika ada, fallback ke query.outlet_id / current outlet
    let target_outlet_id = if let Some(ref s) = active_shift {
        s.outlet_id.or(query.outlet_id)
    } else {
        crate::utils::jwt::resolve_effective_outlet_id(&pool, &headers, query.outlet_id).await
    };

    // Jika user tanpa shift aktif dan tidak ada outlet yang dapat di-resolve, tolak pencarian
    if target_outlet_id.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "message": "Shift kasir belum dibuka atau outlet aktif belum ditentukan. Silakan buka shift kasir terlebih dahulu untuk memindai barang."
            })),
        )
            .into_response();
    }

    let target_warehouse_id = query.warehouse_id.or(target_outlet_id);

    match lookup_product(&pool, &code, target_warehouse_id, target_outlet_id).await {
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
    // 1. Pengecekan Permission di Backend (Fail-Closed)
    let claims = match require_any_permission(&pool, &headers, &["sales:create", "pos:checkout"]).await {
        Ok(c) => c,
        Err(resp) => return resp.into_response(),
    };

    // 2. Validasi Sisi Backend (Domain & Schema Validation)
    if payload.items.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "Keranjang belanja tidak boleh kosong" })),
        )
            .into_response();
    }

    for item in &payload.items {
        if item.quantity <= Decimal::ZERO {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "success": false, "message": "Kuantitas item produk harus lebih besar dari 0" })),
            )
                .into_response();
        }
        if item.unit_selling_price < Decimal::ZERO {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "success": false, "message": "Harga satuan produk tidak boleh bernilai negatif" })),
            )
                .into_response();
        }
    }

    if payload.payments.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "Rincian metode pembayaran wajib diisi" })),
        )
            .into_response();
    }

    let mut uses_points = false;
    for pay in &payload.payments {
        let is_points: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM payment_methods WHERE id = $1 AND category = 'POINTS')"
        )
        .bind(pay.payment_method_id)
        .fetch_one(&pool)
        .await
        .unwrap_or(false);

        if is_points {
            uses_points = true;
            let pts = pay.points.unwrap_or_else(|| pay.amount.to_i32().unwrap_or(0));
            if pts <= 0 {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "success": false, "message": "Jumlah poin yang ditukarkan harus lebih besar dari 0 poin" })),
                )
                    .into_response();
            }
        } else {
            if pay.amount <= Decimal::ZERO {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "success": false, "message": "Nominal pembayaran pada setiap metode harus lebih besar dari 0" })),
                )
                    .into_response();
            }
        }
    }

    if uses_points {
        // Verifikasi hak akses otorisasi penukaran poin
        if let Err(resp) = require_any_permission(&pool, &headers, &["loyalty:redeem", "pos:checkout"]).await {
            return resp.into_response();
        }

        if payload.customer_id.is_none() {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "success": false, "message": "Penukaran poin loyalitas membutuhkan identitas member/pelanggan yang terdaftar" })),
            )
                .into_response();
        }
    }

    let user_id = claims.sub;
    match crate::services::pos_service::checkout(&pool, user_id, payload).await {
        Ok(res) => (StatusCode::OK, Json(json!({ "success": true, "data": res }))).into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

async fn list_sales_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(query): Query<SalesQueryFilter>,
) -> impl IntoResponse {
    let claims = match require_any_permission(&pool, &headers, &["sales:read", "report:view", "pos:checkout"]).await {
        Ok(c) => c,
        Err(resp) => return resp.into_response(),
    };

    let has_all_ownership = crate::utils::jwt::check_permission(&pool, &claims, "sales:all_ownership").await;
    let owner_id = if has_all_ownership { None } else { Some(claims.sub) };

    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(query.limit.unwrap_or(10)).max(1);

    match list_sales(&pool, Some(query), owner_id).await {
        Ok((sales, total_items)) => {
            let total_pages = if total_items > 0 {
                ((total_items as f64) / (page_size as f64)).ceil() as i64
            } else {
                1
            };
            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "data": sales,
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

async fn sales_stats_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let claims = match require_any_permission(&pool, &headers, &["sales:read", "report:view", "pos:checkout"]).await {
        Ok(c) => c,
        Err(resp) => return resp.into_response(),
    };

    let has_all_ownership = crate::utils::jwt::check_permission(&pool, &claims, "sales:all_ownership").await;
    let owner_id = if has_all_ownership { None } else { Some(claims.sub) };

    match get_sales_stats(&pool, owner_id).await {
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
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let claims = match require_any_permission(&pool, &headers, &["sales:read", "report:view", "pos:checkout"]).await {
        Ok(c) => c,
        Err(resp) => return resp.into_response(),
    };

    let has_all_ownership = crate::utils::jwt::check_permission(&pool, &claims, "sales:all_ownership").await;
    let owner_id = if has_all_ownership { None } else { Some(claims.sub) };

    match get_sale_detail(&pool, id, owner_id).await {
        Ok(sale_data) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "data": sale_data
            })),
        )
            .into_response(),
        Err(err) => {
            let status = if err.contains("Akses ditolak") {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::NOT_FOUND
            };
            (
                status,
                Json(json!({ "success": false, "message": err })),
            )
                .into_response()
        }
    }
}

async fn get_sale_stock_batches_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let _claims = match require_any_permission(&pool, &headers, &["inventory:batch_tracking", "sales:read", "report:view"]).await {
        Ok(c) => c,
        Err(resp) => return resp.into_response(),
    };

    match crate::services::pos_service::get_sale_stock_batches(&pool, id).await {
        Ok(batches) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "data": batches
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

async fn void_sale_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<VoidSaleRequest>,
) -> impl IntoResponse {
    // 1. Pengecekan Permission di Backend (Fail-Closed)
    let claims = match require_any_permission(&pool, &headers, &["sales:void", "pos:void"]).await {
        Ok(c) => c,
        Err(resp) => return resp.into_response(),
    };

    // 2. Validasi Sisi Backend
    let has_pin = payload.supervisor_pin.as_ref().map(|p| !p.trim().is_empty()).unwrap_or(false);
    let has_token = payload.supervisor_auth_token.as_ref().map(|t| !t.trim().is_empty()).unwrap_or(false);
    if !has_pin && !has_token {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "Otorisasi supervisor (PIN atau Biometrik) wajib dilakukan" })),
        )
            .into_response();
    }

    let user_id = claims.sub;
    match void_sale(&pool, id, user_id, payload).await {
        Ok(msg) => (StatusCode::OK, Json(json!({ "success": true, "message": msg }))).into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct UploadPaymentProofPayload {
    pub filename: Option<String>,
    pub data_base64: String,
}

async fn upload_payment_proof_handler(
    headers: HeaderMap,
    Json(payload): Json<UploadPaymentProofPayload>,
) -> impl IntoResponse {
    if let Err(err_resp) = get_user_id(&headers) {
        return err_resp.into_response();
    }

    use base64::Engine;

    let b64_data = if let Some(comma_pos) = payload.data_base64.find(',') {
        &payload.data_base64[comma_pos + 1..]
    } else {
        &payload.data_base64
    };

    let image_bytes = match base64::engine::general_purpose::STANDARD.decode(b64_data) {
        Ok(b) => b,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "message": format!("Data gambar bukti base64 tidak valid: {}", e)
                })),
            ).into_response();
        }
    };

    // Deteksi ekstensi dari MIME type
    let ext = if payload.data_base64.starts_with("data:image/png") {
        "png"
    } else if payload.data_base64.starts_with("data:image/webp") {
        "webp"
    } else if payload.data_base64.starts_with("data:image/gif") {
        "gif"
    } else {
        "jpg"
    };

    let unique_id = uuid::Uuid::new_v4().to_string().replace('-', "")[..12].to_string();
    let final_filename = format!("proof_{}.{}", unique_id, ext);

    let images_dir = match dirs::home_dir() {
        Some(h) => h.join(".notane").join("product_images").join("receipts"),
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "success": false, "message": "Tidak dapat menentukan direktori home" })),
            ).into_response();
        }
    };

    if let Err(e) = tokio::fs::create_dir_all(&images_dir).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": format!("Gagal membuat folder bukti: {}", e) })),
        ).into_response();
    }

    let file_path = images_dir.join(&final_filename);
    if let Err(e) = tokio::fs::write(&file_path, &image_bytes).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": format!("Gagal menyimpan file bukti: {}", e) })),
        ).into_response();
    }

    let proof_url = format!("/uploads/product_images/receipts/{}", final_filename);
    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "proof_url": proof_url,
            "filename": final_filename,
            "size_bytes": image_bytes.len()
        })),
    ).into_response()
}
