use crate::models::inventory::{
    CancelStockTransferRequest, CreateGoodsReceiptRequest, CreatePurchaseInvoiceFromGrnRequest,
    CreatePurchaseOrderRequest, CreatePurchaseReturnRequest, CreateStockTransferRequest,
    GoodsReceiptNoteRequest, GoodsReceiptQueryFilter, PurchaseOrderQueryFilter, PurchaseReturnQueryFilter,
    ReceiveStockTransferRequest, StockAdjustmentRequest, StockTransferQueryFilter,
};
use crate::services::inventory_service::{
    cancel_goods_receipt, cancel_purchase_invoice, cancel_purchase_order, cancel_stock_transfer,
    create_goods_receipt, create_purchase_invoice_from_grn, create_purchase_order,
    create_purchase_return, get_goods_receipt_detail, get_purchase_invoice_detail,
    get_purchase_order_detail, get_purchase_return_detail, get_stock_ledger,
    get_stock_transfer_detail, list_goods_receipts, list_purchase_invoices, list_purchase_orders,
    list_purchase_returns, list_stock_transfers, pay_purchase_invoice, process_goods_receipt,
    process_stock_adjustment, process_stock_transfer, receive_stock_transfer, PurchaseQueryFilter,
};
use crate::utils::jwt::{check_permission, require_any_permission, require_permission, verify_jwt};
use axum::{
    extract::{Path, Query, Request, State},
    http::{HeaderMap, Method, StatusCode},
    middleware::{from_fn_with_state, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

async fn inventory_auth_middleware(
    State(pool): State<PgPool>,
    req: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let method = req.method();
    if method == Method::OPTIONS {
        return Ok(next.run(req).await);
    }

    let path = req.uri().path();
    let is_write = method == Method::POST || method == Method::PUT || method == Method::DELETE;

    if path.contains("/adjustments") {
        require_permission(&pool, req.headers(), "inventory:adjust").await?;
    } else if path.contains("/cancel") && (path.contains("/purchase") || path.contains("/goods-receipts")) {
        require_any_permission(&pool, req.headers(), &["purchasing:void", "purchasing:write"]).await?;
    } else if path.contains("/purchase-returns") && is_write {
        require_any_permission(&pool, req.headers(), &["purchasing:void", "purchasing:write"]).await?;
    } else if path.contains("/pay") && (path.contains("/purchase") || path.contains("/purchases")) {
        require_any_permission(&pool, req.headers(), &["purchasing:pay", "purchasing:write"]).await?;
    } else if path.contains("/purchase") || path.contains("/purchases") {
        if is_write {
            require_any_permission(&pool, req.headers(), &["purchasing:create", "purchasing:update", "purchasing:write"]).await?;
        } else {
            require_permission(&pool, req.headers(), "purchasing:read").await?;
        }
    } else if path.contains("/transfers") {
        if path.contains("/cancel") {
            require_any_permission(&pool, req.headers(), &["inventory:transfer_cancel", "inventory:write"]).await?;
        } else if is_write {
            require_any_permission(&pool, req.headers(), &["inventory:transfer", "inventory:write"]).await?;
        } else {
            require_permission(&pool, req.headers(), "inventory:read").await?;
        }
    } else {
        // goods-receipts, grn, stock-ledger
        if is_write {
            require_any_permission(&pool, req.headers(), &["purchasing:create", "purchasing:update", "purchasing:write", "inventory:write"]).await?;
        } else {
            require_any_permission(&pool, req.headers(), &["purchasing:read", "inventory:read"]).await?;
        }
    }

    Ok(next.run(req).await)
}

pub fn inventory_routes(pool: PgPool) -> Router<PgPool> {
    Router::new()
        .route("/grn", post(goods_receipt_handler))
        .route("/goods-receipts", get(list_goods_receipts_handler).post(create_goods_receipt_handler))
        .route("/goods-receipts/{id}", get(get_goods_receipt_detail_handler))
        .route("/goods-receipts/{id}/cancel", post(cancel_goods_receipt_handler))
        .route("/purchase-invoices", post(create_purchase_invoice_from_grn_handler))
        .route("/purchase-invoices/{id}/cancel", post(cancel_purchase_invoice_handler))
        .route("/adjustments", post(stock_adjustment_handler))
        .route("/stock-ledger/{product_id}", get(stock_ledger_handler))
        .route("/purchases", get(list_purchases_handler))
        .route("/purchases/{id}", get(get_purchase_detail_handler))
        .route("/purchases/{id}/pay", post(pay_purchase_handler))
        .route("/purchase-orders", get(list_purchase_orders_handler).post(create_purchase_order_handler))
        .route("/purchase-orders/{id}", get(get_purchase_order_detail_handler))
        .route("/purchase-orders/{id}/cancel", post(cancel_purchase_order_handler))
        .route("/purchase-returns", get(list_purchase_returns_handler).post(create_purchase_return_handler))
        .route("/purchase-returns/{id}", get(get_purchase_return_detail_handler))
        .route("/transfers", get(list_stock_transfers_handler).post(create_stock_transfer_handler))
        .route("/transfers/{id}", get(get_stock_transfer_detail_handler))
        .route("/transfers/{id}/receive", post(receive_stock_transfer_handler))
        .route("/transfers/{id}/cancel", post(cancel_stock_transfer_handler))
        .route("/fefo-alerts", get(get_fefo_alerts_handler))
        .route("/reorder-suggestions", get(get_reorder_suggestions_handler))
        .layer(from_fn_with_state(pool, inventory_auth_middleware))
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

    if payload.items.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "Daftar barang penerimaan (GRN) tidak boleh kosong" })),
        )
            .into_response();
    }

    match process_goods_receipt(&pool, user_id, payload).await {
        Ok(msg) => (StatusCode::OK, Json(json!({ "success": true, "message": msg }))).into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

async fn list_goods_receipts_handler(
    State(pool): State<PgPool>,
    Query(query): Query<GoodsReceiptQueryFilter>,
) -> impl IntoResponse {
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(10).max(1);

    match list_goods_receipts(&pool, Some(query)).await {
        Ok((receipts, total_items)) => {
            let total_pages = if total_items > 0 {
                ((total_items as f64) / (page_size as f64)).ceil() as i64
            } else {
                1
            };
            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "data": receipts,
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

async fn create_goods_receipt_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<CreateGoodsReceiptRequest>,
) -> impl IntoResponse {
    let user_id = match get_user_id(&headers) {
        Ok(id) => id,
        Err(resp) => return resp.into_response(),
    };

    if payload.items.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "Daftar barang penerimaan (GRN) tidak boleh kosong" })),
        )
            .into_response();
    }

    match create_goods_receipt(&pool, user_id, payload).await {
        Ok(data) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "message": "Penerimaan barang (GRN) berhasil dicatat dan stok fisik telah bertambah",
                "data": data
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

async fn get_goods_receipt_detail_handler(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match get_goods_receipt_detail(&pool, id).await {
        Ok(data) => (StatusCode::OK, Json(json!({ "success": true, "data": data }))).into_response(),
        Err(err) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

async fn create_purchase_invoice_from_grn_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<CreatePurchaseInvoiceFromGrnRequest>,
) -> impl IntoResponse {
    let user_id = match get_user_id(&headers) {
        Ok(id) => id,
        Err(resp) => return resp.into_response(),
    };

    match create_purchase_invoice_from_grn(&pool, user_id, payload).await {
        Ok(data) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "message": "Faktur pembelian berhasil dicatat",
                "data": data
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

async fn stock_adjustment_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<StockAdjustmentRequest>,
) -> impl IntoResponse {
    let user_id = match get_user_id(&headers) {
        Ok(id) => id,
        Err(resp) => return resp.into_response(),
    };

    if payload.items.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "Daftar item penyesuaian stok tidak boleh kosong" })),
        )
            .into_response();
    }

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

async fn list_purchases_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(query): Query<PurchaseQueryFilter>,
) -> impl IntoResponse {
    let claims = match require_any_permission(&pool, &headers, &["purchasing:read", "inventory:read"]).await {
        Ok(c) => c,
        Err(resp) => return resp.into_response(),
    };

    let has_all_ownership = check_permission(&pool, &claims, "purchasing:all_ownership").await;
    let owner_id = if has_all_ownership { None } else { Some(claims.sub) };

    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(10).max(1);

    match list_purchase_invoices(&pool, Some(query), owner_id).await {
        Ok((invoices, total_items)) => {
            let total_pages = if total_items > 0 {
                ((total_items as f64) / (page_size as f64)).ceil() as i64
            } else {
                1
            };
            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "data": invoices,
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

async fn get_purchase_detail_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let claims = match require_any_permission(&pool, &headers, &["purchasing:read", "inventory:read"]).await {
        Ok(c) => c,
        Err(resp) => return resp.into_response(),
    };

    let has_all_ownership = check_permission(&pool, &claims, "purchasing:all_ownership").await;
    let owner_id = if has_all_ownership { None } else { Some(claims.sub) };

    match get_purchase_invoice_detail(&pool, id, owner_id).await {
        Ok(inv) => (StatusCode::OK, Json(json!({ "success": true, "data": inv }))).into_response(),
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

async fn pay_purchase_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let user_id = get_user_id(&headers).ok();
    match pay_purchase_invoice(&pool, id, user_id).await {
        Ok(msg) => (StatusCode::OK, Json(json!({ "success": true, "message": msg }))).into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

async fn list_purchase_orders_handler(
    State(pool): State<PgPool>,
    Query(query): Query<PurchaseOrderQueryFilter>,
) -> impl IntoResponse {
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(10).max(1);

    match list_purchase_orders(&pool, Some(query)).await {
        Ok((orders, total_items)) => {
            let total_pages = if total_items > 0 {
                ((total_items as f64) / (page_size as f64)).ceil() as i64
            } else {
                1
            };
            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "data": orders,
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

async fn create_purchase_order_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<CreatePurchaseOrderRequest>,
) -> impl IntoResponse {
    let user_id = match get_user_id(&headers) {
        Ok(id) => id,
        Err(resp) => return resp.into_response(),
    };

    match create_purchase_order(&pool, user_id, payload).await {
        Ok(po) => (
            StatusCode::CREATED,
            Json(json!({ "success": true, "message": "Purchase Order berhasil dibuat", "data": po })),
        )
            .into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

async fn get_purchase_order_detail_handler(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match get_purchase_order_detail(&pool, id).await {
        Ok(po) => (StatusCode::OK, Json(json!({ "success": true, "data": po }))).into_response(),
        Err(err) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

async fn cancel_purchase_order_handler(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match cancel_purchase_order(&pool, id).await {
        Ok(msg) => (StatusCode::OK, Json(json!({ "success": true, "message": msg }))).into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

async fn create_stock_transfer_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<CreateStockTransferRequest>,
) -> impl IntoResponse {
    let user_id = match get_user_id(&headers) {
        Ok(id) => id,
        Err(resp) => return resp.into_response(),
    };

    if payload.from_outlet_id == payload.to_outlet_id {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "Outlet asal dan outlet tujuan transfer tidak boleh sama" })),
        )
            .into_response();
    }

    if payload.items.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "Daftar item transfer stok tidak boleh kosong" })),
        )
            .into_response();
    }

    match process_stock_transfer(&pool, user_id, payload).await {
        Ok(res) => (
            StatusCode::CREATED,
            Json(json!({ "success": true, "message": "Transfer stok berhasil disimpan dan dijurnal", "data": res })),
        )
            .into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

async fn list_stock_transfers_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(filter): Query<StockTransferQueryFilter>,
) -> impl IntoResponse {
    let claims = match require_any_permission(&pool, &headers, &["inventory:read", "inventory:transfer"]).await {
        Ok(c) => c,
        Err(resp) => return resp.into_response(),
    };

    let has_all_ownership = check_permission(&pool, &claims, "inventory:all_ownership").await;
    let owner_id = if has_all_ownership { None } else { Some(claims.sub) };

    match list_stock_transfers(&pool, filter, owner_id).await {
        Ok(res) => (StatusCode::OK, Json(json!({ "success": true, "data": res }))).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

async fn get_stock_transfer_detail_handler(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match get_stock_transfer_detail(&pool, id).await {
        Ok(res) => (StatusCode::OK, Json(json!({ "success": true, "data": res }))).into_response(),
        Err(err) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

async fn cancel_goods_receipt_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let user_id = match get_user_id(&headers) {
        Ok(id) => Some(id),
        Err(_) => None,
    };

    match cancel_goods_receipt(&pool, id, user_id).await {
        Ok(msg) => (StatusCode::OK, Json(json!({ "success": true, "message": msg }))).into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

async fn cancel_purchase_invoice_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let user_id = match get_user_id(&headers) {
        Ok(id) => Some(id),
        Err(_) => None,
    };

    match cancel_purchase_invoice(&pool, id, user_id).await {
        Ok(msg) => (StatusCode::OK, Json(json!({ "success": true, "message": msg }))).into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

async fn list_purchase_returns_handler(
    State(pool): State<PgPool>,
    Query(query): Query<PurchaseReturnQueryFilter>,
) -> impl IntoResponse {
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(10).max(1);

    match list_purchase_returns(&pool, Some(query)).await {
        Ok((returns, total_items)) => {
            let total_pages = if total_items > 0 {
                ((total_items as f64) / (page_size as f64)).ceil() as i64
            } else {
                1
            };
            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "data": returns,
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

async fn create_purchase_return_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<CreatePurchaseReturnRequest>,
) -> impl IntoResponse {
    let user_id = match get_user_id(&headers) {
        Ok(id) => id,
        Err(resp) => return resp.into_response(),
    };

    match create_purchase_return(&pool, user_id, payload).await {
        Ok(data) => (
            StatusCode::CREATED,
            Json(json!({
                "success": true,
                "message": "Retur pembelian berhasil dicatat",
                "data": data
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

async fn get_purchase_return_detail_handler(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match get_purchase_return_detail(&pool, id).await {
        Ok(data) => (StatusCode::OK, Json(json!({ "success": true, "data": data }))).into_response(),
        Err(err) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

async fn receive_stock_transfer_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<ReceiveStockTransferRequest>,
) -> impl IntoResponse {
    let user_id = match get_user_id(&headers) {
        Ok(id) => id,
        Err(resp) => return resp.into_response(),
    };

    match receive_stock_transfer(&pool, id, user_id, payload).await {
        Ok(res) => (StatusCode::OK, Json(json!({ "success": true, "data": res }))).into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

async fn cancel_stock_transfer_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<CancelStockTransferRequest>,
) -> impl IntoResponse {
    let user_id = match get_user_id(&headers) {
        Ok(id) => id,
        Err(resp) => return resp.into_response(),
    };

    match cancel_stock_transfer(&pool, id, user_id, payload).await {
        Ok(res) => (StatusCode::OK, Json(json!({ "success": true, "data": res }))).into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct FefoAlertQuery {
    pub days: Option<i32>,
    pub outlet_id: Option<Uuid>,
}

async fn get_fefo_alerts_handler(
    State(pool): State<PgPool>,
    Query(query): Query<FefoAlertQuery>,
) -> impl IntoResponse {
    let days_threshold = query.days.unwrap_or(60);

    let rows_result = sqlx::query(
        r#"
        SELECT 
            gri.id as grn_item_id,
            gr.grn_number,
            gri.product_id,
            p.name as product_name,
            p.sku as product_sku,
            p.barcode as product_barcode,
            u.code as unit_code,
            gri.batch_number,
            gri.expiry_date,
            gri.received_qty,
            (gri.expiry_date - CURRENT_DATE)::int as days_remaining,
            gr.outlet_id,
            o.name as outlet_name,
            s.name as supplier_name
        FROM goods_receipt_items gri
        JOIN goods_receipts gr ON gri.goods_receipt_id = gr.id
        JOIN products p ON gri.product_id = p.id
        LEFT JOIN units u ON p.base_unit_id = u.id
        LEFT JOIN outlets o ON gr.outlet_id = o.id
        LEFT JOIN suppliers s ON gr.supplier_id = s.id
        WHERE gri.expiry_date IS NOT NULL
          AND gr.status != 'CANCELLED'
          AND gri.expiry_date <= (CURRENT_DATE + ($1 || ' days')::interval)
          AND ($2::uuid IS NULL OR gr.outlet_id = $2)
        ORDER BY gri.expiry_date ASC
        LIMIT 200
        "#
    )
    .bind(days_threshold.to_string())
    .bind(query.outlet_id)
    .fetch_all(&pool)
    .await;

    match rows_result {
        Ok(rows) => {
            use sqlx::Row;
            let data: Vec<serde_json::Value> = rows.into_iter().map(|r| {
                let days_rem: i32 = r.get("days_remaining");
                let urgency = if days_rem < 0 {
                    "EXPIRED"
                } else if days_rem <= 14 {
                    "CRITICAL"
                } else if days_rem <= 30 {
                    "WARNING"
                } else {
                    "NOTICE"
                };

                let exp: Option<chrono::NaiveDate> = r.get("expiry_date");
                let rec_qty: rust_decimal::Decimal = r.get("received_qty");

                json!({
                    "grn_item_id": r.get::<Uuid, _>("grn_item_id"),
                    "grn_number": r.get::<String, _>("grn_number"),
                    "product_id": r.get::<Uuid, _>("product_id"),
                    "product_name": r.get::<String, _>("product_name"),
                    "product_sku": r.get::<String, _>("product_sku"),
                    "product_barcode": r.get::<Option<String>, _>("product_barcode"),
                    "unit_code": r.get::<Option<String>, _>("unit_code"),
                    "batch_number": r.get::<Option<String>, _>("batch_number"),
                    "expiry_date": exp,
                    "received_qty": rec_qty,
                    "days_remaining": days_rem,
                    "urgency": urgency,
                    "outlet_id": r.get::<Option<Uuid>, _>("outlet_id"),
                    "outlet_name": r.get::<Option<String>, _>("outlet_name"),
                    "supplier_name": r.get::<Option<String>, _>("supplier_name")
                })
            }).collect();

            (StatusCode::OK, Json(json!({ "success": true, "data": data }))).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": format!("Database error: {}", e) })),
        ).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct ReorderQuery {
    pub supplier_id: Option<Uuid>,
    pub outlet_id: Option<Uuid>,
}

async fn get_reorder_suggestions_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(query): Query<ReorderQuery>,
) -> impl IntoResponse {
    let effective_outlet_id = crate::utils::jwt::resolve_effective_outlet_id(&pool, &headers, query.outlet_id).await;

    let (pop_join, buy_price_col, pop_group) = if let Some(oid) = effective_outlet_id {
        (
            format!("LEFT JOIN product_outlet_prices pop ON pop.product_id = p.id AND pop.outlet_id = '{oid}'"),
            "COALESCE(NULLIF(pop.last_purchase_price, 0), NULLIF(pop.cost_price, 0), p.cost_price, 0) as purchase_price".to_string(),
            ", pop.last_purchase_price, pop.cost_price".to_string(),
        )
    } else {
        (
            "".to_string(),
            "COALESCE(p.cost_price, 0) as purchase_price".to_string(),
            "".to_string(),
        )
    };

    let ws_join = if let Some(oid) = effective_outlet_id {
        format!("LEFT JOIN warehouse_stocks ws ON p.id = ws.product_id AND ws.warehouse_id = '{oid}'")
    } else {
        "LEFT JOIN warehouse_stocks ws ON p.id = ws.product_id".to_string()
    };

    let sql = format!(
        r#"
        SELECT 
            p.id as product_id,
            p.sku as product_sku,
            p.barcode as product_barcode,
            p.name as product_name,
            p.primary_supplier_id as supplier_id,
            s.name as supplier_name,
            u.code as unit_code,
            COALESCE(SUM(ws.quantity), 0) as current_stock,
            p.min_stock,
            {buy_price_col},
            GREATEST(1, (p.min_stock * 2) - COALESCE(SUM(ws.quantity), 0)) as suggested_qty
        FROM products p
        {ws_join}
        {pop_join}
        LEFT JOIN suppliers s ON p.primary_supplier_id = s.id
        LEFT JOIN units u ON p.base_unit_id = u.id
        WHERE p.is_active = true 
          AND p.min_stock > 0
          AND (
              $1::uuid IS NULL 
              OR p.primary_supplier_id = $1
              OR EXISTS (
                  SELECT 1 FROM purchase_order_items poi 
                  JOIN purchase_orders po ON poi.po_id = po.id 
                  WHERE poi.product_id = p.id AND po.supplier_id = $1
              )
              OR (
                  p.primary_supplier_id IS NULL 
                  AND NOT EXISTS (
                      SELECT 1 FROM purchase_order_items poi 
                      JOIN purchase_orders po ON poi.po_id = po.id 
                      WHERE poi.product_id = p.id
                  )
              )
          )
        GROUP BY p.id, p.sku, p.barcode, p.name, p.primary_supplier_id, s.name, u.code, p.min_stock, p.cost_price{pop_group}
        HAVING COALESCE(SUM(ws.quantity), 0) <= p.min_stock
        ORDER BY (COALESCE(SUM(ws.quantity), 0) - p.min_stock) ASC
        LIMIT 200
        "#
    );

    let rows_result = sqlx::query(&sql)
        .bind(query.supplier_id)
        .fetch_all(&pool)
        .await;

    match rows_result {
        Ok(rows) => {
            use sqlx::Row;
            let data: Vec<serde_json::Value> = rows.into_iter().map(|r| {
                let suggested: rust_decimal::Decimal = r.get("suggested_qty");
                let buy_price: rust_decimal::Decimal = r.get("purchase_price");
                let est_cost = suggested * buy_price;

                json!({
                    "product_id": r.get::<Uuid, _>("product_id"),
                    "product_sku": r.get::<String, _>("product_sku"),
                    "product_barcode": r.get::<Option<String>, _>("product_barcode"),
                    "product_name": r.get::<String, _>("product_name"),
                    "supplier_id": r.get::<Option<Uuid>, _>("supplier_id"),
                    "supplier_name": r.get::<Option<String>, _>("supplier_name"),
                    "unit_code": r.get::<Option<String>, _>("unit_code"),
                    "current_stock": r.get::<rust_decimal::Decimal, _>("current_stock"),
                    "min_stock": r.get::<rust_decimal::Decimal, _>("min_stock"),
                    "purchase_price": buy_price,
                    "suggested_qty": suggested,
                    "estimated_subtotal": est_cost
                })
            }).collect();

            (StatusCode::OK, Json(json!({ "success": true, "data": data }))).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": format!("Database error: {}", e) })),
        ).into_response(),
    }
}




