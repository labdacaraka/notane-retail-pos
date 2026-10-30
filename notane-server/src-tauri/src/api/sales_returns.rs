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
    pub supervisor_id: Option<Uuid>,
    pub supervisor_pin: Option<String>,
    pub supervisor_auth_token: Option<String>,
    pub return_type: Option<String>, // REFUND, EXCHANGE
    pub refund_method: Option<String>, // CASH, STORE_CREDIT
    pub reason_code: Option<String>,
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
            s.total_amount, s.paid_amount,
            COALESCE(pm.name, 'Tunai') as payment_method,
            s.created_at,
            s.outlet_id, o.name as outlet_name,
            s.customer_id, c.name as customer_name, c.phone as customer_phone,
            s.created_by as cashier_id, u.full_name as cashier_name
        FROM sales s
        LEFT JOIN outlets o ON s.outlet_id = o.id
        LEFT JOIN customers c ON s.customer_id = c.id
        LEFT JOIN users u ON s.created_by = u.id
        LEFT JOIN LATERAL (
            SELECT pm.name
            FROM sale_payments sp
            JOIN payment_methods pm ON sp.payment_method_id = pm.id
            WHERE sp.sale_id = s.id
            ORDER BY sp.created_at ASC
            LIMIT 1
        ) pm ON true
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
            u.code as unit_code, si.quantity as original_qty, si.unit_selling_price as unit_price, si.discount_amount, si.subtotal,
            COALESCE(SUM(sri.quantity), 0) as already_returned_qty
        FROM sale_items si
        JOIN products p ON si.product_id = p.id
        LEFT JOIN units u ON si.unit_id = u.id
        LEFT JOIN sales_return_items sri ON si.id = sri.sale_item_id
        WHERE si.sale_id = $1
        GROUP BY si.id, si.product_id, p.name, p.sku, p.barcode, u.code, si.quantity, si.unit_selling_price, si.discount_amount, si.subtotal, si.created_at
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
            "id": row.get::<Uuid, _>("sale_item_id"),
            "sale_item_id": row.get::<Uuid, _>("sale_item_id"),
            "product_id": row.get::<Uuid, _>("product_id"),
            "product_name": row.get::<String, _>("product_name"),
            "sku": row.get::<String, _>("sku"),
            "barcode": row.get::<Option<String>, _>("barcode"),
            "unit_code": row.get::<Option<String>, _>("unit_code").unwrap_or_else(|| "PCS".to_string()),
            "quantity": original_qty,
            "original_qty": original_qty,
            "unit_price": row.get::<Decimal, _>("unit_price"),
            "discount_amount": row.get::<Decimal, _>("discount_amount"),
            "subtotal": row.get::<Decimal, _>("subtotal"),
            "returned_quantity": already_returned_qty,
            "already_returned_qty": already_returned_qty,
            "remaining_returnable_qty": max_returnable_qty,
            "max_returnable_qty": max_returnable_qty
        }));
    }

    Ok(Json(json!({
        "success": true,
        "data": {
            "id": sale_id,
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
    let verified_spv_id = verify_supervisor_auth(
        &pool,
        payload.supervisor_id,
        payload.supervisor_pin.as_deref(),
        payload.supervisor_auth_token.as_deref(),
        "SALES_RETURN",
    )
    .await
    .map_err(|e| {
        (
            StatusCode::FORBIDDEN,
            Json(json!({ "success": false, "message": format!("Otorisasi Supervisor gagal: {}", e) })),
        )
    })?;

    // 2. Mulai transaksi database
    let mut tx = pool.begin().await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() })))
    })?;

    // Ambil data sale asli
    let sale_row = sqlx::query("SELECT id, invoice_number, outlet_id, status FROM sales WHERE id = $1 FOR UPDATE")
        .bind(payload.sale_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))))?;

    let (sale_outlet_id, sale_status, sale_invoice_number) = match sale_row {
        Some(r) => (
            r.get::<Option<Uuid>, _>("outlet_id"),
            r.get::<String, _>("status"),
            r.get::<String, _>("invoice_number"),
        ),
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
                si.quantity as original_qty, si.unit_selling_price as unit_price, si.discount_amount, p.cost_price,
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

    // Lookup reason_id by reason_code jika reason_id belum terisi
    let mut resolved_reason_id = payload.reason_id;
    if resolved_reason_id.is_none() {
        if let Some(ref rcode) = payload.reason_code {
            resolved_reason_id = sqlx::query_scalar("SELECT id FROM reason_codes WHERE code = $1 LIMIT 1")
                .bind(rcode)
                .fetch_optional(&mut *tx)
                .await
                .unwrap_or(None);
        }
    }

    let refund_method = payload.refund_method.unwrap_or_else(|| "CASH".to_string()).to_uppercase();

    // 4. Masukkan header sales_returns
    sqlx::query(
        r#"
        INSERT INTO sales_returns (
            id, return_number, sale_id, outlet_id, shift_id, cashier_id, supervisor_id,
            return_type, refund_method, refund_amount, total_refund_amount, reason_id, notes
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        "#,
    )
    .bind(return_id)
    .bind(&return_number)
    .bind(payload.sale_id)
    .bind(sale_outlet_id)
    .bind(active_shift_id)
    .bind(cashier_id)
    .bind(verified_spv_id)
    .bind(&return_type)
    .bind(&refund_method)
    .bind(total_refund_amount)
    .bind(total_refund_amount)
    .bind(resolved_reason_id)
    .bind(&payload.notes)
    .execute(&mut *tx)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))))?;

    // 5. Masukkan rincian item & perbarui stok
    let mut total_restocked_cogs = Decimal::ZERO;
    for (it, unit_price, disc_amount, subtotal, cost_price) in return_items_data {
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

        // Jika kondisi baik dan restock true, kembalikan stok fisik & akumulasi HPP yang dipulihkan
        if restock && condition == "GOOD" {
            total_restocked_cogs += it.quantity * cost_price;

            if let Some(oid) = sale_outlet_id {
                let updated_stock: (Decimal,) = sqlx::query_as(
                    r#"
                    INSERT INTO warehouse_stocks (warehouse_id, product_id, quantity)
                    VALUES ($1, $2, $3)
                    ON CONFLICT (warehouse_id, product_id)
                    DO UPDATE SET quantity = warehouse_stocks.quantity + $3, updated_at = CURRENT_TIMESTAMP
                    RETURNING quantity
                    "#,
                )
                .bind(oid)
                .bind(it.product_id)
                .bind(it.quantity)
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": format!("Gagal memulihkan stok toko: {}", e) }))))?;

                let new_balance = updated_stock.0;

                sqlx::query(
                    r#"
                    INSERT INTO stock_mutations (
                        warehouse_id, product_id, reference_type, reference_id,
                        qty_in, qty_out, balance, cost_price, notes, created_at, updated_at, created_by, updated_by
                    ) VALUES (
                        $1, $2, 'SALES_RETURN', $3, $4, 0, $5, $6, $7, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $8, $8
                    )
                    "#,
                )
                .bind(oid)
                .bind(it.product_id)
                .bind(return_id)
                .bind(it.quantity)
                .bind(new_balance)
                .bind(cost_price)
                .bind(format!("Retur Penjualan Konsumen {} Faktur {}", return_number, sale_invoice_number))
                .bind(cashier_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": format!("Gagal mencatat mutasi stok retur: {}", e) }))))?;

                // Kembalikan sisa kuantitas batch fisik jika item dialokasikan dari batch
                let allocated_batch: Option<(Uuid,)> = sqlx::query_as(
                    "SELECT batch_id FROM sale_item_stock_batches WHERE sale_item_id = $1 AND batch_id IS NOT NULL LIMIT 1"
                )
                .bind(it.sale_item_id)
                .fetch_optional(&mut *tx)
                .await
                .unwrap_or(None);

                if let Some((b_id,)) = allocated_batch {
                    let _ = sqlx::query("UPDATE product_stock_batches SET remaining_qty = remaining_qty + $1 WHERE id = $2")
                        .bind(it.quantity)
                        .bind(b_id)
                        .execute(&mut *tx)
                        .await;
                }
            }
        }
    }

    // 6. Update status transaksi penjualan (sales)
    // Hitung akumulasi retur pada transaksi penjualan ini
    let (total_sold_qty, total_returned_qty): (Decimal, Decimal) = sqlx::query_as(
        r#"
        SELECT 
            COALESCE(SUM(si.quantity), 0) as total_sold_qty,
            COALESCE((
                SELECT SUM(sri.quantity) 
                FROM sales_return_items sri 
                JOIN sale_items si2 ON sri.sale_item_id = si2.id 
                WHERE si2.sale_id = $1
            ), 0) as total_returned_qty
        FROM sale_items si
        WHERE si.sale_id = $1
        "#
    )
    .bind(payload.sale_id)
    .fetch_one(&mut *tx)
    .await
    .unwrap_or((Decimal::ZERO, Decimal::ZERO));

    let return_status = if total_returned_qty >= total_sold_qty && total_sold_qty > Decimal::ZERO {
        "FULL"
    } else if total_returned_qty > Decimal::ZERO {
        "PARTIAL"
    } else {
        "NONE"
    };

    let accumulated_refund: Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(total_refund_amount), 0) FROM sales_returns WHERE sale_id = $1"
    )
    .bind(payload.sale_id)
    .fetch_one(&mut *tx)
    .await
    .unwrap_or(total_refund_amount);

    sqlx::query(
        "UPDATE sales SET return_status = $1, total_refund_amount = $2, updated_at = CURRENT_TIMESTAMP WHERE id = $3"
    )
    .bind(return_status)
    .bind(accumulated_refund)
    .bind(payload.sale_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": format!("Gagal memperbarui status transaksi penjualan: {}", e) }))))?;

    // 7. Pencatatan Jurnal Akuntansi Retur Penjualan (Double-Entry Bookkeeping Standar)
    // A. Pengembalian Dana Konsumen:
    //    Debet:  4107 Retur Penjualan Konsumen (Sales Returns - Kontra Pendapatan)
    //    Kredit: 1101 Kas Operasional Laci Kasir (CASH) / 1102 Bank / Piutang (STORE_CREDIT)
    // B. Pemulihan Nilai Persediaan & HPP (Khusus item fisik BAIK yang direstok):
    //    Debet:  1201 Persediaan Barang Dagang Retail
    //    Kredit: 5101 Beban Pokok Penjualan (HPP / COGS)
    let journal_id = Uuid::new_v4();
    let journal_number = format!("JRN-RET-{}", Utc::now().format("%Y%m%d%H%M%S"));

    let journal_desc = format!(
        "Retur Penjualan {} Faktur {} ({}, Refund: {})",
        return_number,
        sale_invoice_number,
        refund_method,
        total_refund_amount
    );

    let total_debit = total_refund_amount + total_restocked_cogs;
    let total_credit = total_debit;

    if total_debit > Decimal::ZERO {
        // Ambil akun COA terkait
        let coa_return_opt: Option<Uuid> = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '4107' LIMIT 1")
            .fetch_optional(&mut *tx)
            .await
            .unwrap_or(None);

        let acc_return: Uuid = match coa_return_opt {
            Some(id) => id,
            None => sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '4101' LIMIT 1")
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": format!("Akun COA Penjualan tidak ditemukan: {}", e) }))))?,
        };

        let acc_cash: Uuid = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '1101' LIMIT 1")
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": format!("Akun 1101 Kas Toko tidak ditemukan: {}", e) }))))?;

        let acc_bank: Uuid = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '1102' LIMIT 1")
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": format!("Akun 1102 Bank tidak ditemukan: {}", e) }))))?;

        let acc_refund_source = if refund_method == "CASH" {
            acc_cash
        } else {
            acc_bank
        };

        // Header journal_entries
        sqlx::query(
            r#"
            INSERT INTO journal_entries (
                id, journal_number, journal_date, reference_type, reference_id, description,
                total_debit, total_credit, is_closing_entry, created_at, updated_at, created_by, updated_by
            ) VALUES (
                $1, $2, CURRENT_DATE, 'SALES_RETURN', $3, $4, $5, $6, FALSE, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $7, $7
            )
            "#,
        )
        .bind(journal_id)
        .bind(&journal_number)
        .bind(return_id)
        .bind(&journal_desc)
        .bind(total_debit)
        .bind(total_credit)
        .bind(cashier_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": format!("Gagal mencatat header jurnal retur: {}", e) }))))?;

        // 1. Debet Retur Penjualan (Kontra Pendapatan)
        if total_refund_amount > Decimal::ZERO {
            sqlx::query(
                r#"
                INSERT INTO journal_lines (
                    id, journal_id, account_id, debit, credit, memo, outlet_id, created_at, updated_at, created_by, updated_by
                ) VALUES (
                    gen_random_uuid(), $1, $2, $3, 0, $4, $5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $6, $6
                )
                "#,
            )
            .bind(journal_id)
            .bind(acc_return)
            .bind(total_refund_amount)
            .bind(format!("Retur Penjualan Konsumen {} (Faktur {})", return_number, sale_invoice_number))
            .bind(sale_outlet_id)
            .bind(cashier_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": format!("Gagal mencatat baris debet retur: {}", e) }))))?;

            // 2. Kredit Kas Laci / Bank
            sqlx::query(
                r#"
                INSERT INTO journal_lines (
                    id, journal_id, account_id, debit, credit, memo, outlet_id, created_at, updated_at, created_by, updated_by
                ) VALUES (
                    gen_random_uuid(), $1, $2, 0, $3, $4, $5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $6, $6
                )
                "#,
            )
            .bind(journal_id)
            .bind(acc_refund_source)
            .bind(total_refund_amount)
            .bind(format!("Pengeluaran Refund {} Retur {}", refund_method, return_number))
            .bind(sale_outlet_id)
            .bind(cashier_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": format!("Gagal mencatat baris kredit kas retur: {}", e) }))))?;
        }

        // 3. Debet Persediaan & Kredit HPP jika ada barang yang direstok
        if total_restocked_cogs > Decimal::ZERO {
            let acc_inv: Uuid = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '1201' LIMIT 1")
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": format!("Akun COA 1201 Persediaan Barang tidak ditemukan: {}", e) }))))?;

            let acc_cogs: Uuid = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '5101' LIMIT 1")
                .fetch_one(&mut *tx)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": format!("Akun COA 5101 HPP tidak ditemukan: {}", e) }))))?;

            // Debet Persediaan Barang Dagang
            sqlx::query(
                r#"
                INSERT INTO journal_lines (
                    id, journal_id, account_id, debit, credit, memo, outlet_id, created_at, updated_at, created_by, updated_by
                ) VALUES (
                    gen_random_uuid(), $1, $2, $3, 0, $4, $5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $6, $6
                )
                "#,
            )
            .bind(journal_id)
            .bind(acc_inv)
            .bind(total_restocked_cogs)
            .bind(format!("Pemulihan Persediaan Masuk Retur {}", return_number))
            .bind(sale_outlet_id)
            .bind(cashier_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": format!("Gagal mencatat baris debet persediaan retur: {}", e) }))))?;

            // Kredit Beban Pokok Penjualan (HPP)
            sqlx::query(
                r#"
                INSERT INTO journal_lines (
                    id, journal_id, account_id, debit, credit, memo, outlet_id, created_at, updated_at, created_by, updated_by
                ) VALUES (
                    gen_random_uuid(), $1, $2, 0, $3, $4, $5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $6, $6
                )
                "#,
            )
            .bind(journal_id)
            .bind(acc_cogs)
            .bind(total_restocked_cogs)
            .bind(format!("Kredit Balik HPP Barang Retur {}", return_number))
            .bind(sale_outlet_id)
            .bind(cashier_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": format!("Gagal mencatat baris kredit HPP retur: {}", e) }))))?;
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
            sri.id, sri.product_id, p.name as product_name, p.sku, p.barcode, u.code as unit_code,
            sri.quantity, sri.unit_price, sri.discount_amount, sri.subtotal,
            sri.condition, sri.restock
        FROM sales_return_items sri
        JOIN products p ON sri.product_id = p.id
        LEFT JOIN units u ON p.base_unit_id = u.id
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
