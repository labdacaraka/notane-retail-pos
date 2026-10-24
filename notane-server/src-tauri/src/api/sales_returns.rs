use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::services::auth_service::verify_supervisor_auth;
use crate::utils::jwt::{require_any_permission, verify_jwt};

pub fn sales_return_routes() -> Router<PgPool> {
    Router::new()
        .route("/sales/lookup/{invoice_number}", get(lookup_sale_by_invoice_handler))
        .route("/sales/returns", get(list_sales_returns_handler).post(create_sales_return_handler))
        .route("/sales/returns/{id}", get(get_sales_return_detail_handler))
}

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
pub struct SalesReturnFilter {
    pub search: Option<String>,
    pub outlet_id: Option<Uuid>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct CreateReturnItemPayload {
    pub sale_item_id: Uuid,
    pub product_id: Uuid,
    pub quantity: Decimal,
    pub condition: String, // GOOD, DAMAGED
    pub restock: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSalesReturnPayload {
    pub sale_id: Uuid,
    pub supervisor_id: Uuid,
    pub supervisor_pin: Option<String>,
    pub supervisor_auth_token: Option<String>,
    pub return_type: Option<String>, // REFUND, EXCHANGE
    pub reason_id: Option<Uuid>,
    pub notes: Option<String>,
    pub items: Vec<CreateReturnItemPayload>,
}

pub async fn lookup_sale_by_invoice_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(invoice_number): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let _ = get_user_id(&headers)?;

    let clean_inv = invoice_number.trim().to_uppercase();

    // 1. Ambil data transaksi penjualan utama
    let sale_row = sqlx::query(
        r#"
        SELECT 
            s.id, s.invoice_number, s.status, s.subtotal, s.discount_amount, s.tax_amount,
            s.total_amount, s.paid_amount, s.payment_method, s.created_at,
            s.outlet_id, o.name as outlet_name,
            s.customer_id, c.name as customer_name, c.phone as customer_phone,
            s.cashier_id, u.full_name as cashier_name
        FROM sales s
        LEFT JOIN outlets o ON s.outlet_id = o.id
        LEFT JOIN customers c ON s.customer_id = c.id
        LEFT JOIN users u ON s.cashier_id = u.id
        WHERE UPPER(s.invoice_number) = $1
        "#,
    )
    .bind(&clean_inv)
    .fetch_optional(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))))?;

    let sale_row = match sale_row {
        Some(s) => s,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(json!({ "success": false, "message": format!("Faktur nota '{}' tidak ditemukan", clean_inv) })),
            ))
        }
    };

    let sale_id: Uuid = sale_row.get("id");
    let status: String = sale_row.get("status");

    if status == "VOID" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "Transaksi ini sudah dibatalkan (VOID) dan tidak dapat diretur." })),
        ));
    }

    // 2. Ambil rincian item transaksi beserta akumulasi jumlah yang sudah pernah diretur
    let items_rows = sqlx::query(
        r#"
        SELECT 
            si.id as sale_item_id, si.product_id, p.name as product_name, p.sku, p.barcode,
            p.unit_code, si.quantity as original_qty, si.unit_price, si.discount_amount, si.subtotal,
            COALESCE(SUM(sri.quantity), 0) as already_returned_qty
        FROM sale_items si
        JOIN products p ON si.product_id = p.id
        LEFT JOIN sales_return_items sri ON si.id = sri.sale_item_id
        WHERE si.sale_id = $1
        GROUP BY si.id, si.product_id, p.name, p.sku, p.barcode, p.unit_code, si.quantity, si.unit_price, si.discount_amount, si.subtotal
        ORDER BY si.created_at ASC
        "#,
    )
    .bind(sale_id)
    .fetch_all(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))))?;

    let mut items = Vec::new();
    for row in items_rows {
        let original_qty: Decimal = row.get("original_qty");
        let already_returned_qty: Decimal = row.get("already_returned_qty");
        let max_returnable_qty = (original_qty - already_returned_qty).max(Decimal::ZERO);

        items.push(json!({
            "sale_item_id": row.get::<Uuid, _>("sale_item_id"),
            "product_id": row.get::<Uuid, _>("product_id"),
            "product_name": row.get::<String, _>("product_name"),
            "sku": row.get::<String, _>("sku"),
            "barcode": row.get::<Option<String>, _>("barcode"),
            "unit_code": row.get::<Option<String>, _>("unit_code"),
            "original_qty": original_qty,
            "unit_price": row.get::<Decimal, _>("unit_price"),
            "discount_amount": row.get::<Decimal, _>("discount_amount"),
            "subtotal": row.get::<Decimal, _>("subtotal"),
            "already_returned_qty": already_returned_qty,
            "max_returnable_qty": max_returnable_qty
        }));
    }

    Ok(Json(json!({
        "success": true,
        "data": {
            "sale_id": sale_id,
            "invoice_number": sale_row.get::<String, _>("invoice_number"),
            "status": status,
            "outlet_id": sale_row.get::<Option<Uuid>, _>("outlet_id"),
            "outlet_name": sale_row.get::<Option<String>, _>("outlet_name"),
            "customer_id": sale_row.get::<Option<Uuid>, _>("customer_id"),
            "customer_name": sale_row.get::<Option<String>, _>("customer_name"),
            "customer_phone": sale_row.get::<Option<String>, _>("customer_phone"),
            "cashier_id": sale_row.get::<Option<Uuid>, _>("cashier_id"),
            "cashier_name": sale_row.get::<Option<String>, _>("cashier_name"),
            "created_at": sale_row.get::<DateTime<Utc>, _>("created_at"),
            "total_amount": sale_row.get::<Decimal, _>("total_amount"),
            "payment_method": sale_row.get::<Option<String>, _>("payment_method"),
            "items": items
        }
    })))
}

pub async fn create_sales_return_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<CreateSalesReturnPayload>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let cashier_id = get_user_id(&headers)?;
    require_any_permission(&pool, &headers, &["sales_returns:create", "super_admin"]).await?;

    if payload.items.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "Pilih minimal 1 barang untuk diretur" })),
        ));
    }

    // 1. Verifikasi Otorisasi Supervisor (Biometrik atau PIN)
    if let Err(e) = verify_supervisor_auth(
        &pool,
        Some(payload.supervisor_id),
        payload.supervisor_pin.as_deref(),
        payload.supervisor_auth_token.as_deref(),
        "SALES_RETURN",
    )
    .await
    {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({ "success": false, "message": format!("Otorisasi Supervisor gagal: {}", e) })),
        ));
    }

    // 2. Mulai transaksi database
    let mut tx = pool.begin().await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() })))
    })?;

    // Ambil data sale asli
    let sale_row = sqlx::query("SELECT id, outlet_id, status FROM sales WHERE id = $1 FOR UPDATE")
        .bind(payload.sale_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))))?;

    let (sale_outlet_id, sale_status) = match sale_row {
        Some(r) => (r.get::<Option<Uuid>, _>("outlet_id"), r.get::<String, _>("status")),
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(json!({ "success": false, "message": "Transaksi penjualan tidak ditemukan" })),
            ))
        }
    };

    if sale_status == "VOID" {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "Transaksi penjualan sudah VOID." })),
        ));
    }

    // Ambil shift kasir yang sedang aktif jika ada
    let active_shift_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM cashier_shifts WHERE user_id = $1 AND status = 'OPEN' ORDER BY opened_at DESC LIMIT 1",
    )
    .bind(cashier_id)
    .fetch_optional(&mut *tx)
    .await
    .unwrap_or(None);

    let return_id = Uuid::new_v4();
    let return_number = format!("RET-{}", Utc::now().format("%Y%m%d%H%M%S"));
    let return_type = payload.return_type.unwrap_or_else(|| "REFUND".to_string());

    let mut total_refund_amount = Decimal::ZERO;
    let mut return_items_data = Vec::new();

    // 3. Validasi kuantitas retur per item & hitung refund
    for it in &payload.items {
        if it.quantity <= Decimal::ZERO {
            continue;
        }

        let item_info = sqlx::query(
            r#"
            SELECT 
                si.quantity as original_qty, si.unit_price, si.discount_amount, p.cost_price,
                COALESCE((SELECT SUM(sri.quantity) FROM sales_return_items sri WHERE sri.sale_item_id = si.id), 0) as already_returned
            FROM sale_items si
            JOIN products p ON si.product_id = p.id
            WHERE si.id = $1 AND si.sale_id = $2
            "#,
        )
        .bind(it.sale_item_id)
        .bind(payload.sale_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))))?;

        let (orig_qty, unit_price, disc_amount, cost_price, already_ret): (Decimal, Decimal, Decimal, Decimal, Decimal) = match item_info {
            Some(r) => (
                r.get("original_qty"),
                r.get("unit_price"),
                r.get("discount_amount"),
                r.get("cost_price"),
                r.get("already_returned"),
            ),
            None => {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "success": false, "message": "Item transaksi tidak valid" })),
                ))
            }
        };

        if already_ret + it.quantity > orig_qty {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "message": format!("Jumlah retur ({}) melebihi sisa barang yang dapat diretur ({})", it.quantity, orig_qty - already_ret)
                })),
            ));
        }

        // Hitung harga bersih setelah diskon per unit
        let net_price_per_unit = if orig_qty > Decimal::ZERO {
            unit_price - (disc_amount / orig_qty)
        } else {
            unit_price
        };
        let item_refund = (net_price_per_unit * it.quantity).max(Decimal::ZERO);
        total_refund_amount += item_refund;

        return_items_data.push((it, unit_price, disc_amount, item_refund, cost_price));
    }

    if return_items_data.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "Tidak ada item retur yang valid" })),
        ));
    }

    // 4. Masukkan header sales_returns
    sqlx::query(
        r#"
        INSERT INTO sales_returns (
            id, return_number, sale_id, outlet_id, shift_id, cashier_id, supervisor_id,
            return_type, total_refund_amount, reason_id, notes
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        "#,
    )
    .bind(return_id)
    .bind(&return_number)
    .bind(payload.sale_id)
    .bind(sale_outlet_id)
    .bind(active_shift_id)
    .bind(cashier_id)
    .bind(payload.supervisor_id)
    .bind(&return_type)
    .bind(total_refund_amount)
    .bind(payload.reason_id)
    .bind(&payload.notes)
    .execute(&mut *tx)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))))?;

    // 5. Masukkan rincian item & perbarui stok
    for (it, unit_price, disc_amount, subtotal, _cost_price) in return_items_data {
        let restock = it.restock.unwrap_or(true);
        let condition = it.condition.trim().to_uppercase();

        sqlx::query(
            r#"
            INSERT INTO sales_return_items (
                sales_return_id, sale_item_id, product_id, quantity,
                unit_price, discount_amount, subtotal, condition, restock
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(return_id)
        .bind(it.sale_item_id)
        .bind(it.product_id)
        .bind(it.quantity)
        .bind(unit_price)
        .bind(disc_amount)
        .bind(subtotal)
        .bind(&condition)
        .bind(restock)
        .execute(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))))?;

        // Jika kondisi baik dan restock true, kembalikan stok fisik
        if restock && condition == "GOOD" {
            if let Some(oid) = sale_outlet_id {
                let _ = sqlx::query(
                    r#"
                    INSERT INTO warehouse_stocks (warehouse_id, product_id, quantity)
                    VALUES ($1, $2, $3)
                    ON CONFLICT (warehouse_id, product_id)
                    DO UPDATE SET quantity = warehouse_stocks.quantity + $3, updated_at = CURRENT_TIMESTAMP
                    "#,
                )
                .bind(oid)
                .bind(it.product_id)
                .bind(it.quantity)
                .execute(&mut *tx)
                .await;

                let _ = sqlx::query(
                    r#"
                    INSERT INTO stock_movements (
                        product_id, warehouse_id, reference_type, reference_id,
                        quantity_in, quantity_out, notes
                    ) VALUES ($1, $2, 'RETURN_IN', $3, $4, 0, $5)
                    "#,
                )
                .bind(it.product_id)
                .bind(oid)
                .bind(return_id)
                .bind(it.quantity)
                .bind(format!("Retur Penjualan Konsumen {}", return_number))
                .execute(&mut *tx)
                .await;
            }
        }
    }

    // 6. Pencatatan Jurnal Akuntansi Retur Penjualan
    // Debit: 4103 (Retur Penjualan)
    // Credit: 1101 (Kas Toko)
    let journal_id = Uuid::new_v4();
    let journal_number = format!("JRN-{}", Utc::now().format("%Y%m%d%H%M%S"));

    let _ = sqlx::query(
        r#"
        INSERT INTO general_journals (
            id, journal_number, reference_type, reference_id,
            transaction_date, description, total_amount, created_by
        ) VALUES ($1, $2, 'SALES_RETURN', $3, CURRENT_TIMESTAMP, $4, $5, $6)
        "#,
    )
    .bind(journal_id)
    .bind(&journal_number)
    .bind(return_id)
    .bind(format!("Retur Penjualan Konsumen {} ({})", return_number, return_type))
    .bind(total_refund_amount)
    .bind(cashier_id)
    .execute(&mut *tx)
    .await;

    // Ambil COA 4103 (Retur Penjualan) dan 1101 (Kas Toko)
    let coa_return: Option<Uuid> = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '4103' LIMIT 1")
        .fetch_optional(&mut *tx)
        .await
        .unwrap_or(None);

    let coa_cash: Option<Uuid> = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '1101' LIMIT 1")
        .fetch_optional(&mut *tx)
        .await
        .unwrap_or(None);

    if let (Some(c_ret), Some(c_csh)) = (coa_return, coa_cash) {
        if total_refund_amount > Decimal::ZERO {
            // Debit Retur Penjualan
            let _ = sqlx::query(
                r#"
                INSERT INTO journal_entries (
                    journal_id, coa_id, debit_amount, credit_amount, description
                ) VALUES ($1, $2, $3, 0, $4)
                "#,
            )
            .bind(journal_id)
            .bind(c_ret)
            .bind(total_refund_amount)
            .bind(format!("Beban Retur Penjualan {}", return_number))
            .execute(&mut *tx)
            .await;

            // Credit Kas
            let _ = sqlx::query(
                r#"
                INSERT INTO journal_entries (
                    journal_id, coa_id, debit_amount, credit_amount, description
                ) VALUES ($1, $2, 0, $3, $4)
                "#,
            )
            .bind(journal_id)
            .bind(c_csh)
            .bind(total_refund_amount)
            .bind(format!("Pengembalian Dana Kas Retur {}", return_number))
            .execute(&mut *tx)
            .await;
        }
    }

    tx.commit().await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() })))
    })?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "success": true,
            "message": format!("Retur penjualan {} berhasil diproses", return_number),
            "data": {
                "return_id": return_id,
                "return_number": return_number,
                "total_refund_amount": total_refund_amount,
                "return_type": return_type
            }
        })),
    ))
}

pub async fn list_sales_returns_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(filter): Query<SalesReturnFilter>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    require_any_permission(&pool, &headers, &["sales_returns:read", "super_admin"]).await?;

    let page = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(15).max(1);
    let offset = (page - 1) * page_size;

    let search_pattern = filter.search.as_ref().map(|s| format!("%{}%", s.trim().to_lowercase()));

    let query_str = r#"
        SELECT 
            sr.id, sr.return_number, sr.sale_id, s.invoice_number,
            sr.outlet_id, o.name as outlet_name,
            sr.return_type, sr.total_refund_amount, sr.created_at,
            u.full_name as cashier_name, spv.full_name as supervisor_name,
            rc.description as reason_name, sr.notes
        FROM sales_returns sr
        JOIN sales s ON sr.sale_id = s.id
        LEFT JOIN outlets o ON sr.outlet_id = o.id
        LEFT JOIN users u ON sr.cashier_id = u.id
        LEFT JOIN users spv ON sr.supervisor_id = spv.id
        LEFT JOIN reason_codes rc ON sr.reason_id = rc.id
        WHERE ($1::text IS NULL OR LOWER(sr.return_number) LIKE $1 OR LOWER(s.invoice_number) LIKE $1)
          AND ($2::uuid IS NULL OR sr.outlet_id = $2)
        ORDER BY sr.created_at DESC
        LIMIT $3 OFFSET $4
    "#;

    let rows = sqlx::query(query_str)
        .bind(&search_pattern)
        .bind(filter.outlet_id)
        .bind(page_size)
        .bind(offset)
        .fetch_all(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))))?;

    let count_query = r#"
        SELECT COUNT(*)
        FROM sales_returns sr
        JOIN sales s ON sr.sale_id = s.id
        WHERE ($1::text IS NULL OR LOWER(sr.return_number) LIKE $1 OR LOWER(s.invoice_number) LIKE $1)
          AND ($2::uuid IS NULL OR sr.outlet_id = $2)
    "#;

    let total: i64 = sqlx::query_scalar(count_query)
        .bind(&search_pattern)
        .bind(filter.outlet_id)
        .fetch_one(&pool)
        .await
        .unwrap_or(0);

    let mut list = Vec::new();
    for r in rows {
        list.push(json!({
            "id": r.get::<Uuid, _>("id"),
            "return_number": r.get::<String, _>("return_number"),
            "sale_id": r.get::<Uuid, _>("sale_id"),
            "invoice_number": r.get::<String, _>("invoice_number"),
            "outlet_id": r.get::<Option<Uuid>, _>("outlet_id"),
            "outlet_name": r.get::<Option<String>, _>("outlet_name"),
            "return_type": r.get::<String, _>("return_type"),
            "total_refund_amount": r.get::<Decimal, _>("total_refund_amount"),
            "cashier_name": r.get::<Option<String>, _>("cashier_name"),
            "supervisor_name": r.get::<Option<String>, _>("supervisor_name"),
            "reason_name": r.get::<Option<String>, _>("reason_name"),
            "notes": r.get::<Option<String>, _>("notes"),
            "created_at": r.get::<DateTime<Utc>, _>("created_at")
        }));
    }

    Ok(Json(json!({
        "success": true,
        "data": list,
        "meta": {
            "page": page,
            "page_size": page_size,
            "total": total,
            "total_pages": (total as f64 / page_size as f64).ceil() as i64
        }
    })))
}

pub async fn get_sales_return_detail_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    require_any_permission(&pool, &headers, &["sales_returns:read", "super_admin"]).await?;

    let return_row = sqlx::query(
        r#"
        SELECT 
            sr.id, sr.return_number, sr.sale_id, s.invoice_number,
            sr.outlet_id, o.name as outlet_name,
            sr.return_type, sr.total_refund_amount, sr.created_at,
            u.full_name as cashier_name, spv.full_name as supervisor_name,
            rc.description as reason_name, sr.notes
        FROM sales_returns sr
        JOIN sales s ON sr.sale_id = s.id
        LEFT JOIN outlets o ON sr.outlet_id = o.id
        LEFT JOIN users u ON sr.cashier_id = u.id
        LEFT JOIN users spv ON sr.supervisor_id = spv.id
        LEFT JOIN reason_codes rc ON sr.reason_id = rc.id
        WHERE sr.id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))))?;

    let return_row = match return_row {
        Some(r) => r,
        None => return Err((StatusCode::NOT_FOUND, Json(json!({ "success": false, "message": "Retur tidak ditemukan" })))),
    };

    let items_rows = sqlx::query(
        r#"
        SELECT 
            sri.id, sri.product_id, p.name as product_name, p.sku, p.barcode, p.unit_code,
            sri.quantity, sri.unit_price, sri.discount_amount, sri.subtotal,
            sri.condition, sri.restock
        FROM sales_return_items sri
        JOIN products p ON sri.product_id = p.id
        WHERE sri.sales_return_id = $1
        "#,
    )
    .bind(id)
    .fetch_all(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))))?;

    let mut items = Vec::new();
    for r in items_rows {
        items.push(json!({
            "id": r.get::<Uuid, _>("id"),
            "product_id": r.get::<Uuid, _>("product_id"),
            "product_name": r.get::<String, _>("product_name"),
            "sku": r.get::<String, _>("sku"),
            "barcode": r.get::<Option<String>, _>("barcode"),
            "unit_code": r.get::<Option<String>, _>("unit_code"),
            "quantity": r.get::<Decimal, _>("quantity"),
            "unit_price": r.get::<Decimal, _>("unit_price"),
            "discount_amount": r.get::<Decimal, _>("discount_amount"),
            "subtotal": r.get::<Decimal, _>("subtotal"),
            "condition": r.get::<String, _>("condition"),
            "restock": r.get::<bool, _>("restock")
        }));
    }

    Ok(Json(json!({
        "success": true,
        "data": {
            "id": return_row.get::<Uuid, _>("id"),
            "return_number": return_row.get::<String, _>("return_number"),
            "sale_id": return_row.get::<Uuid, _>("sale_id"),
            "invoice_number": return_row.get::<String, _>("invoice_number"),
            "outlet_id": return_row.get::<Option<Uuid>, _>("outlet_id"),
            "outlet_name": return_row.get::<Option<String>, _>("outlet_name"),
            "return_type": return_row.get::<String, _>("return_type"),
            "total_refund_amount": return_row.get::<Decimal, _>("total_refund_amount"),
            "cashier_name": return_row.get::<Option<String>, _>("cashier_name"),
            "supervisor_name": return_row.get::<Option<String>, _>("supervisor_name"),
            "reason_name": return_row.get::<Option<String>, _>("reason_name"),
            "notes": return_row.get::<Option<String>, _>("notes"),
            "created_at": return_row.get::<DateTime<Utc>, _>("created_at"),
            "items": items
        }
    })))
}
