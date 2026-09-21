use crate::models::product::{
    CustomerGroupPriceDetail, Product, ProductDetail, ProductPriceTier, ProductUnitDetail,
};
use crate::models::sales::{
    CheckoutItemResponse, CheckoutPaymentResponse, CheckoutRequest, CheckoutResponse, Sale,
    SaleItem, VoidSaleRequest,
};
use crate::models::shift::{CashierShift, CloseShiftRequest, OpenShiftRequest, ShiftSummaryResponse};
use crate::models::user::SupervisorPinRequest;
use crate::services::audit_service::log_audit;
use crate::services::auth_service::verify_supervisor_pin;
use crate::utils::barcode::parse_barcode;
use chrono::Utc;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use serde_json::json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub async fn open_shift(
    pool: &PgPool,
    user_id: Uuid,
    req: OpenShiftRequest,
) -> Result<CashierShift, String> {
    let existing = sqlx::query_as::<_, CashierShift>(
        "SELECT cs.*, o.name as outlet_name, o.code as outlet_code, o.default_warehouse_id
         FROM cashier_shifts cs
         LEFT JOIN outlets o ON cs.outlet_id = o.id
         WHERE cs.user_id = $1 AND cs.status = 'OPEN'",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?;

    if let Some(shift) = existing {
        return Ok(shift);
    }

    let is_superadmin = sqlx::query_scalar::<_, bool>(
        "SELECT (r.name = 'SUPERADMIN') FROM users u JOIN roles r ON u.role_id = r.id WHERE u.id = $1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    let target_outlet_id = if let Some(oid) = req.outlet_id {
        if !is_superadmin {
            let has_access = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM user_outlets WHERE user_id = $1 AND outlet_id = $2)",
            )
            .bind(user_id)
            .bind(oid)
            .fetch_one(pool)
            .await
            .unwrap_or(false);

            if !has_access {
                return Err("Akses ditolak: Anda tidak memiliki izin untuk mengoperasikan kasir di outlet ini.".to_string());
            }
        }
        Some(oid)
    } else {
        if is_superadmin {
            sqlx::query_scalar::<_, Uuid>("SELECT id FROM outlets WHERE is_active = TRUE ORDER BY created_at ASC LIMIT 1")
                .fetch_optional(pool)
                .await
                .ok()
                .flatten()
        } else {
            sqlx::query_scalar::<_, Uuid>(
                "SELECT o.id FROM outlets o 
                 JOIN user_outlets uo ON o.id = uo.outlet_id 
                 WHERE uo.user_id = $1 AND o.is_active = TRUE 
                 ORDER BY o.created_at ASC LIMIT 1",
            )
            .bind(user_id)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten()
        }
    };

    if target_outlet_id.is_none() {
        return Err("Tidak ada outlet aktif yang dapat diakses oleh akun Anda.".to_string());
    }

    let mut supervisor_approval_id = None;
    if let Some(pin) = req.supervisor_pin {
        if !pin.trim().is_empty() {
            let spv_res = verify_supervisor_pin(
                pool,
                SupervisorPinRequest {
                    supervisor_id: req.supervisor_id,
                    pin,
                    action: Some("SHIFT_OPEN".into()),
                },
            )
            .await?;
            supervisor_approval_id = Some(spv_res.supervisor_id);
        }
    }

    let shift = sqlx::query_as::<_, CashierShift>(
        "INSERT INTO cashier_shifts (
            terminal_id, user_id, outlet_id, opened_at, opening_cash, status, supervisor_approval_id, created_at, updated_at, created_by, updated_by
        ) VALUES ($1, $2, $3, CURRENT_TIMESTAMP, $4, 'OPEN', $5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $2, $2)
        RETURNING *",
    )
    .bind(&req.terminal_id)
    .bind(user_id)
    .bind(target_outlet_id)
    .bind(req.opening_cash)
    .bind(supervisor_approval_id)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Gagal membuka shift baru: {}", e))?;

    log_audit(
        pool,
        Some(user_id),
        supervisor_approval_id,
        &req.terminal_id,
        "SHIFT_OPEN",
        "cashier_shifts",
        Some(&shift.id.to_string()),
        None,
        Some(json!({ "opening_cash": req.opening_cash, "outlet_id": target_outlet_id })),
        None,
    )
    .await
    .ok();

    let enriched_shift = get_current_shift(pool, user_id).await?.unwrap_or(shift);
    Ok(enriched_shift)
}

pub async fn get_current_shift(pool: &PgPool, user_id: Uuid) -> Result<Option<CashierShift>, String> {
    sqlx::query_as::<_, CashierShift>(
        "SELECT cs.*, o.name as outlet_name, o.code as outlet_code, o.default_warehouse_id 
         FROM cashier_shifts cs
         LEFT JOIN outlets o ON cs.outlet_id = o.id
         WHERE cs.user_id = $1 AND cs.status = 'OPEN' 
         ORDER BY cs.opened_at DESC LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))
}

pub async fn close_shift(
    pool: &PgPool,
    user_id: Uuid,
    req: CloseShiftRequest,
) -> Result<ShiftSummaryResponse, String> {
    let mut tx = pool.begin().await.map_err(|e| format!("Transaction error: {}", e))?;

    let shift = sqlx::query_as::<_, CashierShift>(
        "SELECT * FROM cashier_shifts WHERE id = $1 AND status = 'OPEN' FOR UPDATE",
    )
    .bind(req.shift_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| format!("Database error: {}", e))?
    .ok_or_else(|| "Shift aktif tidak ditemukan atau sudah ditutup".to_string())?;

    // Check sales in this shift
    let sales_row = sqlx::query(
        "SELECT 
            COUNT(*)::bigint as total_count,
            COALESCE(SUM(subtotal), 0) as gross_sales,
            COALESCE(SUM(discount_amount), 0) as discounts,
            COALESCE(SUM(tax_amount), 0) as tax,
            COALESCE(SUM(total_amount), 0) as net_sales
        FROM sales 
        WHERE shift_id = $1 AND status = 'COMPLETED'",
    )
    .bind(shift.id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| format!("Gagal mengkalkulasi penjualan shift: {}", e))?;

    let total_sales_count: i64 = sales_row.get("total_count");
    let gross_sales: Decimal = sales_row.get("gross_sales");
    let discounts: Decimal = sales_row.get("discounts");
    let tax: Decimal = sales_row.get("tax");
    let net_sales: Decimal = sales_row.get("net_sales");

    // Check payment breakdowns
    let payments_rows = sqlx::query(
        "SELECT 
            pm.category,
            COALESCE(SUM(sp.amount), 0) as total_amount
        FROM sale_payments sp
        JOIN sales s ON sp.sale_id = s.id
        JOIN payment_methods pm ON sp.payment_method_id = pm.id
        WHERE s.shift_id = $1 AND s.status = 'COMPLETED'
        GROUP BY pm.category",
    )
    .bind(shift.id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| format!("Gagal mengkalkulasi rincian pembayaran: {}", e))?;

    let mut total_cash = Decimal::ZERO;
    let mut total_qris = Decimal::ZERO;
    let mut total_card = Decimal::ZERO;
    let mut total_points = Decimal::ZERO;

    for p in payments_rows {
        let cat: String = p.get("category");
        let amt: Decimal = p.get("total_amount");
        match cat.as_str() {
            "CASH" => total_cash += amt,
            "QRIS" => total_qris += amt,
            "EDC" => total_card += amt,
            "POINTS" => total_points += amt,
            _ => (),
        }
    }

    let expected_cash = shift.opening_cash + total_cash;
    let discrepancy = req.actual_cash - expected_cash;

    let mut supervisor_id = None;
    if let Some(pin) = req.supervisor_pin {
        if !pin.trim().is_empty() {
            let spv_res = verify_supervisor_pin(
                pool,
                SupervisorPinRequest {
                    supervisor_id: req.supervisor_id,
                    pin,
                    action: Some("SHIFT_CLOSE".into()),
                },
            )
            .await?;
            supervisor_id = Some(spv_res.supervisor_id);
        }
    }

    let updated_shift = sqlx::query_as::<_, CashierShift>(
        "UPDATE cashier_shifts SET
            closed_at = CURRENT_TIMESTAMP,
            expected_cash = $1,
            actual_cash = $2,
            discrepancy = $3,
            status = 'CLOSED',
            supervisor_approval_id = $4,
            updated_at = CURRENT_TIMESTAMP,
            updated_by = $5
        WHERE id = $6
        RETURNING *",
    )
    .bind(expected_cash)
    .bind(req.actual_cash)
    .bind(discrepancy)
    .bind(supervisor_id)
    .bind(user_id)
    .bind(shift.id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| format!("Gagal menutup shift: {}", e))?;

    // Auto-journal for discrepancy if any
    if !discrepancy.is_zero() {
        let abs_diff = discrepancy.abs();
        let journal_number = format!("JRN-SHT-{}", Utc::now().format("%Y%m%d%H%M%S"));
        let journal_id = Uuid::new_v4();

        let desc = if discrepancy > Decimal::ZERO {
            format!("Lebih Kas Shift {} oleh Kasir {}", shift.id, user_id)
        } else {
            format!("Kurang Kas Shift {} oleh Kasir {}", shift.id, user_id)
        };

        sqlx::query(
            "INSERT INTO journal_entries (
                id, journal_number, journal_date, reference_type, reference_id, description,
                total_debit, total_credit, created_at, updated_at, created_by, updated_by
            ) VALUES ($1, $2, CURRENT_DATE, 'SHIFT_DISCREPANCY', $3, $4, $5, $5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $6, $6)",
        )
        .bind(journal_id)
        .bind(journal_number)
        .bind(shift.id)
        .bind(desc)
        .bind(abs_diff)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal mencatat jurnal selisih kas: {}", e))?;

        let kas_id: Uuid = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '1101'")
            .fetch_one(&mut *tx).await.map_err(|_| "Akun 1101 Kas tidak ditemukan")?;

        if discrepancy > Decimal::ZERO {
            let rev_id: Uuid = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '4201'")
                .fetch_one(&mut *tx).await.map_err(|_| "Akun 4201 tidak ditemukan")?;
            sqlx::query(
                "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo) VALUES ($1, $2, $3, 0, 'Lebih kas fisik'), ($1, $4, 0, $3, 'Pendapatan lebih kas')",
            )
            .bind(journal_id)
            .bind(kas_id)
            .bind(abs_diff)
            .bind(rev_id)
            .execute(&mut *tx).await.map_err(|e| format!("Gagal mencatat baris jurnal: {}", e))?;
        } else {
            let exp_id: Uuid = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '5201'")
                .fetch_one(&mut *tx).await.map_err(|_| "Akun 5201 tidak ditemukan")?;
            sqlx::query(
                "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo) VALUES ($1, $2, $3, 0, 'Beban kurang kas fisik'), ($1, $4, 0, $3, 'Kredit kas fisik')",
            )
            .bind(journal_id)
            .bind(exp_id)
            .bind(abs_diff)
            .bind(kas_id)
            .execute(&mut *tx).await.map_err(|e| format!("Gagal mencatat baris jurnal: {}", e))?;
        }
    }

    tx.commit().await.map_err(|e| format!("Commit error: {}", e))?;

    let cashier_name = sqlx::query_scalar::<_, String>("SELECT full_name FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap_or_else(|_| "Kasir".to_string());

    Ok(ShiftSummaryResponse {
        shift: updated_shift,
        cashier_name,
        total_sales_count,
        total_gross_sales: gross_sales,
        total_discounts: discounts,
        total_tax: tax,
        total_net_sales: net_sales,
        total_cash_received: total_cash,
        total_qris_received: total_qris,
        total_card_received: total_card,
        total_points_redeemed: total_points,
        expected_cash_in_drawer: expected_cash,
        actual_cash_counted: Some(req.actual_cash),
        discrepancy: Some(discrepancy),
    })
}

pub async fn lookup_product(
    pool: &PgPool,
    code: &str,
    warehouse_id: Option<Uuid>,
) -> Result<ProductDetail, String> {
    let parsed = parse_barcode(code);

    let product_id_opt: Option<Uuid> = if parsed.is_weight_embedded {
        let pat1 = format!("2{}%", parsed.item_code);
        let pat2 = format!("%{}%", parsed.item_code);
        sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM products WHERE is_weight_embedded = true AND (barcode LIKE $1 OR barcode LIKE $2 OR sku LIKE $2) LIMIT 1",
        )
        .bind(&pat1)
        .bind(&pat2)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?
    } else {
        let direct = sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM products WHERE barcode = $1 OR sku = $1",
        )
        .bind(&parsed.raw_barcode)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

        if direct.is_some() {
            direct
        } else {
            sqlx::query_scalar::<_, Uuid>(
                "SELECT product_id FROM product_barcodes WHERE barcode = $1",
            )
            .bind(&parsed.raw_barcode)
            .fetch_optional(pool)
            .await
            .map_err(|e| format!("Database error: {}", e))?
        }
    };

    let product_id = product_id_opt.ok_or_else(|| format!("Produk dengan barcode '{}' tidak ditemukan", code))?;

    let product = sqlx::query_as::<_, Product>("SELECT * FROM products WHERE id = $1")
        .bind(product_id)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let category_name = sqlx::query_scalar::<_, String>("SELECT name FROM categories WHERE id = $1")
        .bind(product.category_id)
        .fetch_one(pool)
        .await
        .unwrap_or_else(|_| "Umum".to_string());

    let (base_unit_code, base_unit_name, is_fractional) = sqlx::query_as::<_, (String, String, bool)>(
        "SELECT code, name, is_fractional FROM units WHERE id = $1",
    )
    .bind(product.base_unit_id)
    .fetch_one(pool)
    .await
    .unwrap_or_else(|_| ("PCS".to_string(), "Pieces".to_string(), false));

    let brand_name = if let Some(bid) = product.brand_id {
        sqlx::query_scalar::<_, String>("SELECT name FROM brands WHERE id = $1")
            .bind(bid)
            .fetch_optional(pool)
            .await
            .unwrap_or(None)
    } else {
        None
    };

    let unit_rows = sqlx::query(
        "SELECT pu.id, pu.unit_id, u.code as unit_code, u.name as unit_name, pu.conversion_factor, pu.selling_price, pu.barcode
        FROM product_units pu
        JOIN units u ON pu.unit_id = u.id
        WHERE pu.product_id = $1",
    )
    .bind(product.id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut units = vec![ProductUnitDetail {
        id: Uuid::nil(),
        unit_id: product.base_unit_id,
        unit_code: base_unit_code.clone(),
        unit_name: base_unit_name.clone(),
        conversion_factor: Decimal::ONE,
        selling_price: product.base_selling_price,
        barcode: Some(product.barcode.clone()),
    }];

    for r in unit_rows {
        units.push(ProductUnitDetail {
            id: r.get("id"),
            unit_id: r.get("unit_id"),
            unit_code: r.get("unit_code"),
            unit_name: r.get("unit_name"),
            conversion_factor: r.get("conversion_factor"),
            selling_price: r.get("selling_price"),
            barcode: r.get("barcode"),
        });
    }

    let price_tiers = sqlx::query_as::<_, ProductPriceTier>(
        "SELECT * FROM product_price_tiers WHERE product_id = $1 ORDER BY min_quantity ASC",
    )
    .bind(product.id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let cgp_rows = sqlx::query(
        "SELECT cgp.id, cgp.customer_group_id, cg.name as customer_group_name, cgp.product_id, cgp.special_price
         FROM customer_group_prices cgp
         JOIN customer_groups cg ON cgp.customer_group_id = cg.id
         WHERE cgp.product_id = $1 AND cg.is_active = true
         ORDER BY cg.id ASC",
    )
    .bind(product.id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut customer_group_prices = Vec::new();
    for r in cgp_rows {
        customer_group_prices.push(CustomerGroupPriceDetail {
            id: r.get("id"),
            customer_group_id: r.get("customer_group_id"),
            customer_group_name: r.get("customer_group_name"),
            product_id: r.get("product_id"),
            special_price: r.get("special_price"),
        });
    }

    let current_stock = if let Some(wid) = warehouse_id {
        sqlx::query_scalar::<_, Decimal>(
            "SELECT quantity FROM warehouse_stocks WHERE warehouse_id = $1 AND product_id = $2",
        )
        .bind(wid)
        .bind(product.id)
        .fetch_optional(pool)
        .await
        .unwrap_or(None)
        .unwrap_or(Decimal::ZERO)
    } else {
        sqlx::query_scalar::<_, Decimal>(
            "SELECT COALESCE(SUM(quantity), 0) FROM warehouse_stocks WHERE product_id = $1",
        )
        .bind(product.id)
        .fetch_one(pool)
        .await
        .unwrap_or(Decimal::ZERO)
    };

    Ok(ProductDetail {
        product,
        category_name,
        base_unit_code,
        base_unit_name,
        is_fractional,
        brand_name,
        units,
        price_tiers,
        customer_group_prices,
        current_stock,
        scanned_quantity: parsed.weight,
    })
}

pub async fn checkout(
    pool: &PgPool,
    user_id: Uuid,
    req: CheckoutRequest,
) -> Result<CheckoutResponse, String> {
    if req.items.is_empty() {
        return Err("Keranjang belanja tidak boleh kosong".to_string());
    }

    let mut tx = pool.begin().await.map_err(|e| format!("Transaction error: {}", e))?;

    let shift = sqlx::query_as::<_, CashierShift>(
        "SELECT * FROM cashier_shifts WHERE id = $1 AND status = 'OPEN' FOR UPDATE",
    )
    .bind(req.shift_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| format!("Database error: {}", e))?
    .ok_or_else(|| "Shift kasir tidak aktif atau sudah ditutup".to_string())?;

    let now = Utc::now();
    let invoice_number = format!("INV-{}-{}", now.format("%Y%m%d"), &Uuid::new_v4().to_string()[..8].to_uppercase());
    let sale_id = Uuid::new_v4();

    // 1. Resolve outlet_id
    let outlet_id = if let Some(oid) = req.outlet_id {
        oid
    } else if let Some(shift_oid) = shift.outlet_id {
        shift_oid
    } else {
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM outlets WHERE is_active = TRUE ORDER BY created_at ASC LIMIT 1")
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Tidak ada outlet aktif yang ditemukan".to_string())?
    };

    // 2. Resolve warehouse_id for physical inventory deduction
    let target_warehouse_id = if let Some(wid) = req.warehouse_id {
        Some(wid)
    } else {
        let default_wh = sqlx::query_scalar::<_, Option<Uuid>>(
            "SELECT default_warehouse_id FROM outlets WHERE id = $1"
        )
        .bind(outlet_id)
        .fetch_optional(&mut *tx)
        .await
        .unwrap_or(None)
        .flatten();

        if default_wh.is_some() {
            default_wh
        } else {
            sqlx::query_scalar::<_, Uuid>(
                "SELECT id FROM outlets WHERE is_warehouse = TRUE AND is_active = TRUE ORDER BY created_at ASC LIMIT 1"
            )
            .fetch_optional(&mut *tx)
            .await
            .unwrap_or(None)
        }
    };

    let mut subtotal = Decimal::ZERO;
    let mut total_discount = Decimal::ZERO;
    let mut total_tax = Decimal::ZERO;
    let mut total_cost = Decimal::ZERO;

    struct PreparedItem {
        product_id: Uuid,
        product_name: String,
        unit_id: Uuid,
        unit_code: String,
        qty: Decimal,
        cost_price: Decimal,
        selling_price: Decimal,
        disc_percent: Decimal,
        disc_amount: Decimal,
        tax_amount: Decimal,
        item_subtotal: Decimal,
    }

    let mut prepared_items = Vec::new();

    for item in &req.items {
        let product = sqlx::query_as::<_, Product>(
            "SELECT * FROM products WHERE id = $1",
        )
        .bind(item.product_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| format!("Produk tidak ditemukan: {}", e))?;

        let (factor, unit_code) = if item.unit_id == product.base_unit_id {
            let uc = sqlx::query_scalar::<_, String>("SELECT code FROM units WHERE id = $1")
                .bind(product.base_unit_id)
                .fetch_optional(&mut *tx)
                .await
                .unwrap_or(None)
                .unwrap_or_else(|| "PCS".to_string());
            (Decimal::ONE, uc)
        } else {
            let row = sqlx::query("SELECT pu.conversion_factor, u.code as unit_code FROM product_units pu JOIN units u ON pu.unit_id = u.id WHERE pu.product_id = $1 AND pu.unit_id = $2")
                .bind(product.id)
                .bind(item.unit_id)
                .fetch_optional(&mut *tx)
                .await
                .unwrap_or(None);
            if let Some(r) = row {
                (r.get::<Decimal, _>("conversion_factor"), r.get::<String, _>("unit_code"))
            } else {
                (Decimal::ONE, "PCS".to_string())
            }
        };

        let base_qty = item.quantity * factor;

        if let Some(wh_id) = target_warehouse_id {
            let current_stock = sqlx::query_scalar::<_, Decimal>(
                "SELECT quantity FROM warehouse_stocks WHERE warehouse_id = $1 AND product_id = $2 FOR UPDATE",
            )
            .bind(wh_id)
            .bind(product.id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| format!("Database error: {}", e))?
            .unwrap_or(Decimal::ZERO);

            let new_balance = current_stock - base_qty;

            sqlx::query(
                "INSERT INTO warehouse_stocks (warehouse_id, product_id, quantity, updated_at)
                VALUES ($1, $2, $3, CURRENT_TIMESTAMP)
                ON CONFLICT (warehouse_id, product_id)
                DO UPDATE SET quantity = $3, updated_at = CURRENT_TIMESTAMP",
            )
            .bind(wh_id)
            .bind(product.id)
            .bind(new_balance)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Gagal memotong stok: {}", e))?;

            sqlx::query(
                "INSERT INTO stock_mutations (
                    warehouse_id, product_id, reference_type, reference_id, qty_in, qty_out, 
                    balance, cost_price, notes, created_at, updated_at, created_by, updated_by
                ) VALUES (
                    $1, $2, 'SALES', $3, 0, $4, $5, $6, $7, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $8, $8
                )",
            )
            .bind(wh_id)
            .bind(product.id)
            .bind(sale_id)
            .bind(base_qty)
            .bind(new_balance)
            .bind(product.cost_price)
            .bind(format!("Penjualan POS {}", invoice_number))
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Gagal mencatat mutasi stok: {}", e))?;
        }

        let line_gross = item.unit_selling_price * item.quantity;
        let line_disc = item.discount_amount.unwrap_or(Decimal::ZERO);
        let line_tax = if product.tax_percent > Decimal::ZERO {
            ((line_gross - line_disc) * product.tax_percent) / Decimal::from(100)
        } else {
            Decimal::ZERO
        };
        let line_subtotal = (line_gross - line_disc) + line_tax;
        let line_cost = product.cost_price * base_qty;

        subtotal += line_gross;
        total_discount += line_disc;
        total_tax += line_tax;
        total_cost += line_cost;

        prepared_items.push(PreparedItem {
            product_id: product.id,
            product_name: product.name.clone(),
            unit_id: item.unit_id,
            unit_code,
            qty: item.quantity,
            cost_price: product.cost_price,
            selling_price: item.unit_selling_price,
            disc_percent: item.discount_percent.unwrap_or(Decimal::ZERO),
            disc_amount: line_disc,
            tax_amount: line_tax,
            item_subtotal: line_subtotal,
        });
    }

    // Voucher handling
    let mut voucher_id = None;
    if let Some(vcode) = &req.voucher_code {
        let v_opt = sqlx::query(
            "SELECT id, discount_type, discount_value, min_purchase_amount, max_discount_amount 
             FROM vouchers WHERE code = $1 AND status = 'ACTIVE' AND valid_from <= CURRENT_TIMESTAMP AND valid_until >= CURRENT_TIMESTAMP",
        )
        .bind(vcode)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

        if let Some(v) = v_opt {
            let min_purchase: Decimal = v.get("min_purchase_amount");
            if subtotal >= min_purchase {
                let dtype: String = v.get("discount_type");
                let dval: Decimal = v.get("discount_value");
                let max_disc: Option<Decimal> = v.get("max_discount_amount");

                let v_disc = match dtype.as_str() {
                    "PERCENTAGE" => {
                        let calc = (subtotal * dval) / Decimal::from(100);
                        if let Some(max_d) = max_disc {
                            calc.min(max_d)
                        } else {
                            calc
                        }
                    }
                    _ => dval,
                };
                total_discount += v_disc;
                voucher_id = Some(v.get::<Uuid, _>("id"));
            }
        }
    }

    let final_total = (subtotal - total_discount) + total_tax;
    let total_paid: Decimal = req.payments.iter().map(|p| p.amount).sum();
    if total_paid < final_total {
        return Err(format!(
            "Pembayaran kurang! Total belanja: Rp {}, Pembayaran: Rp {}",
            final_total, total_paid
        ));
    }
    let change_amount = total_paid - final_total;

    let sale = sqlx::query_as::<_, Sale>(
        "INSERT INTO sales (
            id, invoice_number, outlet_id, shift_id, customer_id, warehouse_id, voucher_id,
            subtotal, discount_amount, tax_amount, total_amount, paid_amount, change_amount,
            total_cost_amount, status, notes, created_at, updated_at, created_by, updated_by
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, 'COMPLETED', $15,
            CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $16, $16
        )
        RETURNING *",
    )
    .bind(sale_id)
    .bind(&invoice_number)
    .bind(outlet_id)
    .bind(shift.id)
    .bind(req.customer_id)
    .bind(target_warehouse_id)
    .bind(voucher_id)
    .bind(subtotal)
    .bind(total_discount)
    .bind(total_tax)
    .bind(final_total)
    .bind(total_paid)
    .bind(change_amount)
    .bind(total_cost)
    .bind(req.notes)
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| format!("Gagal menyimpan data penjualan: {}", e))?;

    for pi in &prepared_items {
        sqlx::query(
            "INSERT INTO sale_items (
                id, sale_id, product_id, unit_id, quantity, unit_cost_price, unit_selling_price,
                discount_percent, discount_amount, tax_amount, subtotal, is_voided,
                created_at, updated_at, created_by, updated_by
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, false,
                CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $12, $12
            )",
        )
        .bind(Uuid::new_v4())
        .bind(sale.id)
        .bind(pi.product_id)
        .bind(pi.unit_id)
        .bind(pi.qty)
        .bind(pi.cost_price)
        .bind(pi.selling_price)
        .bind(pi.disc_percent)
        .bind(pi.disc_amount)
        .bind(pi.tax_amount)
        .bind(pi.item_subtotal)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal menyimpan baris penjualan: {}", e))?;
    }

    let mut total_cash_paid = Decimal::ZERO;
    let mut total_non_cash_paid = Decimal::ZERO;

    for p in &req.payments {
        sqlx::query(
            "INSERT INTO sale_payments (
                id, sale_id, payment_method_id, amount, card_number_masked, reference_number,
                created_at, updated_at, created_by, updated_by
            ) VALUES (
                $1, $2, $3, $4, $5, $6, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $7, $7
            )",
        )
        .bind(Uuid::new_v4())
        .bind(sale.id)
        .bind(p.payment_method_id)
        .bind(p.amount)
        .bind(&p.card_number_masked)
        .bind(&p.reference_number)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal menyimpan pembayaran: {}", e))?;

        let pm_cat = sqlx::query_scalar::<_, String>(
            "SELECT category FROM payment_methods WHERE id = $1",
        )
        .bind(p.payment_method_id)
        .fetch_one(&mut *tx)
        .await
        .unwrap_or_else(|_| "CASH".to_string());

        if pm_cat == "CASH" {
            total_cash_paid += p.amount;
        } else {
            total_non_cash_paid += p.amount;
        }
    }

    let net_cash_in_drawer = (total_cash_paid - change_amount).max(Decimal::ZERO);

    let mut earned_points = 0;
    if let Some(cid) = req.customer_id {
        let pts = (final_total / Decimal::from(10000)).to_i32().unwrap_or(0);
        if pts > 0 {
            earned_points = pts;
            let current_pts = sqlx::query_scalar::<_, i32>(
                "SELECT reward_points FROM customers WHERE id = $1 FOR UPDATE",
            )
            .bind(cid)
            .fetch_one(&mut *tx)
            .await
            .unwrap_or(0);

            let new_balance = current_pts + earned_points;

            sqlx::query("UPDATE customers SET reward_points = $1 WHERE id = $2")
                .bind(new_balance)
                .bind(cid)
                .execute(&mut *tx)
                .await
                .ok();

            sqlx::query(
                "INSERT INTO customer_point_ledgers (
                    customer_id, transaction_type, reference_id, points_in, points_out, balance, notes, created_at, created_by
                ) VALUES ($1, 'EARNED', $2, $3, 0, $4, $5, CURRENT_TIMESTAMP, $6)",
            )
            .bind(cid)
            .bind(sale.id)
            .bind(earned_points)
            .bind(new_balance)
            .bind(format!("Poin dari transaksi {}", invoice_number))
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .ok();
        }
    }

    // Double-Entry Auto-Journaling
    let journal_id = Uuid::new_v4();
    let journal_number = format!("JRN-POS-{}", now.format("%Y%m%d%H%M%S"));

    let net_sales_revenue = subtotal - total_discount;
    let total_debits = net_cash_in_drawer + total_non_cash_paid + total_cost;
    let total_credits = net_sales_revenue + total_tax + total_cost;

    sqlx::query(
        "INSERT INTO journal_entries (
            id, journal_number, journal_date, reference_type, reference_id, description,
            total_debit, total_credit, created_at, updated_at, created_by, updated_by
        ) VALUES (
            $1, $2, CURRENT_DATE, 'SALES', $3, $4, $5, $6, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $7, $7
        )",
    )
    .bind(journal_id)
    .bind(journal_number)
    .bind(sale.id)
    .bind(format!("Penjualan POS Struk {}", invoice_number))
    .bind(total_debits)
    .bind(total_credits)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal membentuk header jurnal double-entry: {}", e))?;

    let acc_kas: Uuid = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '1101'").fetch_one(&mut *tx).await.map_err(|_| "Akun 1101 Kas tidak ditemukan")?;
    let acc_bank: Uuid = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '1102'").fetch_one(&mut *tx).await.map_err(|_| "Akun 1102 Bank tidak ditemukan")?;
    let acc_inv: Uuid = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '1201'").fetch_one(&mut *tx).await.map_err(|_| "Akun 1201 Persediaan tidak ditemukan")?;
    let acc_ppn: Uuid = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '2102'").fetch_one(&mut *tx).await.map_err(|_| "Akun 2102 PPN tidak ditemukan")?;
    let acc_sales: Uuid = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '4101'").fetch_one(&mut *tx).await.map_err(|_| "Akun 4101 Penjualan tidak ditemukan")?;
    let acc_cogs: Uuid = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '5101'").fetch_one(&mut *tx).await.map_err(|_| "Akun 5101 HPP tidak ditemukan")?;

    if net_cash_in_drawer > Decimal::ZERO {
        sqlx::query(
            "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo) VALUES ($1, $2, $3, 0, 'Penerimaan Kas Kasir')",
        )
        .bind(journal_id)
        .bind(acc_kas)
        .bind(net_cash_in_drawer)
        .execute(&mut *tx).await.map_err(|e| format!("Gagal membuat baris jurnal: {}", e))?;
    }

    if total_non_cash_paid > Decimal::ZERO {
        sqlx::query(
            "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo) VALUES ($1, $2, $3, 0, 'Penerimaan Non-Tunai / QRIS / EDC')",
        )
        .bind(journal_id)
        .bind(acc_bank)
        .bind(total_non_cash_paid)
        .execute(&mut *tx).await.map_err(|e| format!("Gagal membuat baris jurnal: {}", e))?;
    }

    if total_cost > Decimal::ZERO {
        sqlx::query(
            "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo) VALUES ($1, $2, $3, 0, 'Beban Pokok Penjualan (HPP)')",
        )
        .bind(journal_id)
        .bind(acc_cogs)
        .bind(total_cost)
        .execute(&mut *tx).await.map_err(|e| format!("Gagal membuat baris jurnal: {}", e))?;

        sqlx::query(
            "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo) VALUES ($1, $2, 0, $3, 'Pengurangan Persediaan Barang Dagang')",
        )
        .bind(journal_id)
        .bind(acc_inv)
        .bind(total_cost)
        .execute(&mut *tx).await.map_err(|e| format!("Gagal membuat baris jurnal: {}", e))?;
    }

    if net_sales_revenue > Decimal::ZERO {
        sqlx::query(
            "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo) VALUES ($1, $2, 0, $3, 'Pendapatan Penjualan Bersih')",
        )
        .bind(journal_id)
        .bind(acc_sales)
        .bind(net_sales_revenue)
        .execute(&mut *tx).await.map_err(|e| format!("Gagal membuat baris jurnal: {}", e))?;
    }

    if total_tax > Decimal::ZERO {
        sqlx::query(
            "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo) VALUES ($1, $2, 0, $3, 'Hutang PPN Keluaran')",
        )
        .bind(journal_id)
        .bind(acc_ppn)
        .bind(total_tax)
        .execute(&mut *tx).await.map_err(|e| format!("Gagal membuat baris jurnal: {}", e))?;
    }

    let mut response_payments = Vec::new();
    for p in &req.payments {
        let (pm_name, pm_cat) = sqlx::query_as::<_, (String, String)>(
            "SELECT name, category FROM payment_methods WHERE id = $1",
        )
        .bind(p.payment_method_id)
        .fetch_optional(&mut *tx)
        .await
        .unwrap_or(None)
        .unwrap_or_else(|| ("Tunai".to_string(), "CASH".to_string()));

        response_payments.push(CheckoutPaymentResponse {
            payment_method_id: p.payment_method_id,
            payment_method_name: pm_name,
            category: pm_cat,
            amount: p.amount,
        });
    }

    let customer_name = if let Some(cid) = req.customer_id {
        sqlx::query_scalar::<_, String>("SELECT name FROM customers WHERE id = $1")
            .bind(cid)
            .fetch_optional(&mut *tx)
            .await
            .unwrap_or(None)
    } else {
        None
    };

    let response_items: Vec<CheckoutItemResponse> = prepared_items
        .into_iter()
        .map(|pi| CheckoutItemResponse {
            product_id: pi.product_id,
            product_name: pi.product_name,
            unit_id: pi.unit_id,
            unit_code: pi.unit_code,
            quantity: pi.qty,
            unit_selling_price: pi.selling_price,
            discount_percent: pi.disc_percent,
            discount_amount: pi.disc_amount,
            tax_amount: pi.tax_amount,
            subtotal: pi.item_subtotal,
        })
        .collect();

    tx.commit().await.map_err(|e| format!("Commit error: {}", e))?;

    let items_len = response_items.len();

    Ok(CheckoutResponse {
        sale,
        items: response_items,
        payments: response_payments,
        customer_name,
        items_count: items_len,
        change_amount,
        earned_points,
        invoice_number,
        message: "Transaksi Checkout POS Berhasil".to_string(),
    })
}

pub async fn void_sale(
    pool: &PgPool,
    sale_id: Uuid,
    user_id: Uuid,
    req: VoidSaleRequest,
) -> Result<String, String> {
    let spv_res = verify_supervisor_pin(
        pool,
        SupervisorPinRequest {
            supervisor_id: req.supervisor_id,
            pin: req.supervisor_pin,
            action: Some("VOID_SALE".into()),
        },
    )
    .await?;

    let mut tx = pool.begin().await.map_err(|e| format!("Transaction error: {}", e))?;

    let sale = sqlx::query_as::<_, Sale>(
        "SELECT * FROM sales WHERE id = $1 AND status = 'COMPLETED' FOR UPDATE",
    )
    .bind(sale_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| format!("Database error: {}", e))?
    .ok_or_else(|| "Nota penjualan tidak ditemukan atau sudah dibatalkan".to_string())?;

    let items = sqlx::query_as::<_, SaleItem>(
        "SELECT * FROM sale_items WHERE sale_id = $1",
    )
    .bind(sale.id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| format!("Database error: {}", e))?;

    for it in &items {
        let factor = sqlx::query_scalar::<_, Decimal>(
            "SELECT conversion_factor FROM product_units WHERE product_id = $1 AND unit_id = $2",
        )
        .bind(it.product_id)
        .bind(it.unit_id)
        .fetch_optional(&mut *tx)
        .await
        .unwrap_or(None)
        .unwrap_or(Decimal::ONE);

        let base_qty = it.quantity * factor;

        if let Some(wh_id) = sale.warehouse_id {
            let current_stock = sqlx::query_scalar::<_, Decimal>(
                "SELECT quantity FROM warehouse_stocks WHERE warehouse_id = $1 AND product_id = $2 FOR UPDATE",
            )
            .bind(wh_id)
            .bind(it.product_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| format!("Database error: {}", e))?
            .unwrap_or(Decimal::ZERO);

            let new_balance = current_stock + base_qty;

            sqlx::query(
                "UPDATE warehouse_stocks SET quantity = $1, updated_at = CURRENT_TIMESTAMP WHERE warehouse_id = $2 AND product_id = $3",
            )
            .bind(new_balance)
            .bind(wh_id)
            .bind(it.product_id)
            .execute(&mut *tx).await.map_err(|e| format!("Gagal memulihkan stok: {}", e))?;

            sqlx::query(
                "INSERT INTO stock_mutations (
                    warehouse_id, product_id, reference_type, reference_id, qty_in, qty_out,
                    balance, cost_price, notes, created_at, updated_at, created_by, updated_by
                ) VALUES ($1, $2, 'RETURN_IN', $3, $4, 0, $5, $6, $7, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $8, $8)",
            )
            .bind(wh_id)
            .bind(it.product_id)
            .bind(sale.id)
            .bind(base_qty)
            .bind(new_balance)
            .bind(it.unit_cost_price)
            .bind(format!("Pembatalan / Void Nota {}", sale.invoice_number))
            .bind(user_id)
            .execute(&mut *tx).await.map_err(|e| format!("Gagal mencatat mutasi void: {}", e))?;
        }
    }

    if let Some(cid) = sale.customer_id {
        let earned_pts = sqlx::query_scalar::<_, i32>(
            "SELECT points_in FROM customer_point_ledgers WHERE reference_id = $1 AND transaction_type = 'EARNED'",
        )
        .bind(sale.id)
        .fetch_optional(&mut *tx)
        .await
        .unwrap_or(None)
        .unwrap_or(0);

        if earned_pts > 0 {
            let current_pts = sqlx::query_scalar::<_, i32>(
                "SELECT reward_points FROM customers WHERE id = $1 FOR UPDATE",
            )
            .bind(cid)
            .fetch_one(&mut *tx)
            .await
            .unwrap_or(0);

            let new_pts = (current_pts - earned_pts).max(0);
            sqlx::query("UPDATE customers SET reward_points = $1 WHERE id = $2")
                .bind(new_pts)
                .bind(cid)
                .execute(&mut *tx).await.ok();

            sqlx::query(
                "INSERT INTO customer_point_ledgers (
                    customer_id, transaction_type, reference_id, points_in, points_out, balance, notes, created_at, created_by
                ) VALUES ($1, 'CORRECTION', $2, 0, $3, $4, $5, CURRENT_TIMESTAMP, $6)",
            )
            .bind(cid)
            .bind(sale.id)
            .bind(earned_pts)
            .bind(new_pts)
            .bind(format!("Koreksi pembatalan nota {}", sale.invoice_number))
            .bind(user_id)
            .execute(&mut *tx).await.ok();
        }
    }

    let effective_reason_id = match req.reason_id {
        Some(rid) => rid,
        None => {
            sqlx::query_scalar::<_, Uuid>(
                "SELECT id FROM reason_codes WHERE type = 'VOID' ORDER BY code ASC LIMIT 1",
            )
            .fetch_optional(pool)
            .await
            .unwrap_or(None)
            .unwrap_or_else(|| Uuid::parse_str("f1000000-0000-0000-0000-000000000001").unwrap())
        }
    };

    let void_notes = match &req.notes {
        Some(n) if !n.trim().is_empty() => format!(" | Alasan Void: {}", n.trim()),
        _ => String::new(),
    };

    sqlx::query(
        "UPDATE sales SET status = 'VOID', notes = COALESCE(notes, '') || $1, updated_at = CURRENT_TIMESTAMP, updated_by = $2 WHERE id = $3",
    )
    .bind(void_notes)
    .bind(user_id)
    .bind(sale.id)
    .execute(&mut *tx).await.map_err(|e| format!("Gagal update status nota: {}", e))?;

    sqlx::query(
        "UPDATE sale_items SET is_voided = TRUE, void_reason_id = $1, void_authorized_by = $2, updated_at = CURRENT_TIMESTAMP, updated_by = $3 WHERE sale_id = $4",
    )
    .bind(effective_reason_id)
    .bind(spv_res.supervisor_id)
    .bind(user_id)
    .bind(sale.id)
    .execute(&mut *tx).await.map_err(|e| format!("Gagal update status item void: {}", e))?;

    let reversal_journal_id = Uuid::new_v4();
    let reversal_number = format!("REV-{}", sale.invoice_number);

    let original_journal = sqlx::query(
        "SELECT id, total_debit, total_credit FROM journal_entries WHERE reference_type = 'SALES' AND reference_id = $1",
    )
    .bind(sale.id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| format!("Database error: {}", e))?;

    if let Some(orig) = original_journal {
        let orig_id: Uuid = orig.get("id");
        let orig_debit: Decimal = orig.get("total_debit");
        let orig_credit: Decimal = orig.get("total_credit");

        sqlx::query(
            "INSERT INTO journal_entries (
                id, journal_number, journal_date, reference_type, reference_id, description,
                total_debit, total_credit, created_at, updated_at, created_by, updated_by
            ) VALUES ($1, $2, CURRENT_DATE, 'VOID', $3, $4, $5, $6, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $7, $7)",
        )
        .bind(reversal_journal_id)
        .bind(reversal_number)
        .bind(sale.id)
        .bind(format!("Jurnal Pembalik Pembatalan Nota {}", sale.invoice_number))
        .bind(orig_credit)
        .bind(orig_debit)
        .bind(user_id)
        .execute(&mut *tx).await.map_err(|e| format!("Gagal membuat reversal journal: {}", e))?;

        let orig_lines = sqlx::query(
            "SELECT account_id, debit, credit, memo FROM journal_lines WHERE journal_id = $1",
        )
        .bind(orig_id)
        .fetch_all(&mut *tx).await.map_err(|e| format!("Gagal memuat baris jurnal lama: {}", e))?;

        for line in orig_lines {
            let acc_id: Uuid = line.get("account_id");
            let deb: Decimal = line.get("debit");
            let cred: Decimal = line.get("credit");
            let memo: Option<String> = line.get("memo");

            sqlx::query(
                "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo) VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(reversal_journal_id)
            .bind(acc_id)
            .bind(cred)
            .bind(deb)
            .bind(format!("Pembalik {}", memo.unwrap_or_default()))
            .execute(&mut *tx).await.map_err(|e| format!("Gagal membalik baris jurnal: {}", e))?;
        }
    }

    log_audit(
        pool,
        Some(user_id),
        Some(spv_res.supervisor_id),
        "TERMINAL",
        "VOID_SALE",
        "sales",
        Some(&sale.id.to_string()),
        Some(json!({ "invoice_number": sale.invoice_number, "total_amount": sale.total_amount })),
        Some(json!({ "status": "VOID", "reason_id": effective_reason_id, "notes": req.notes })),
        None,
    ).await.ok();

    tx.commit().await.map_err(|e| format!("Commit error: {}", e))?;

    Ok(format!("Nota {} berhasil dibatalkan (Void)", sale.invoice_number))
}

pub async fn list_sales(
    pool: &PgPool,
    shift_id: Option<Uuid>,
    outlet_id: Option<Uuid>,
    limit: i64,
) -> Result<Vec<serde_json::Value>, String> {
    let rows = if let Some(sid) = shift_id {
        sqlx::query(
            "SELECT s.*, c.name as customer_name, c.phone as customer_phone,
                    cs.terminal_id, u.full_name as cashier_name,
                    o.name as outlet_name, o.code as outlet_code
             FROM sales s
             LEFT JOIN outlets o ON s.outlet_id = o.id
             LEFT JOIN customers c ON s.customer_id = c.id
             LEFT JOIN cashier_shifts cs ON s.shift_id = cs.id
             LEFT JOIN users u ON cs.user_id = u.id
             WHERE s.shift_id = $1
             ORDER BY s.created_at DESC LIMIT $2",
        )
        .bind(sid)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?
    } else if let Some(oid) = outlet_id {
        sqlx::query(
            "SELECT s.*, c.name as customer_name, c.phone as customer_phone,
                    cs.terminal_id, u.full_name as cashier_name,
                    o.name as outlet_name, o.code as outlet_code
             FROM sales s
             LEFT JOIN outlets o ON s.outlet_id = o.id
             LEFT JOIN customers c ON s.customer_id = c.id
             LEFT JOIN cashier_shifts cs ON s.shift_id = cs.id
             LEFT JOIN users u ON cs.user_id = u.id
             WHERE s.outlet_id = $1
             ORDER BY s.created_at DESC LIMIT $2",
        )
        .bind(oid)
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?
    } else {
        sqlx::query(
            "SELECT s.*, c.name as customer_name, c.phone as customer_phone,
                    cs.terminal_id, u.full_name as cashier_name,
                    o.name as outlet_name, o.code as outlet_code
             FROM sales s
             LEFT JOIN outlets o ON s.outlet_id = o.id
             LEFT JOIN customers c ON s.customer_id = c.id
             LEFT JOIN cashier_shifts cs ON s.shift_id = cs.id
             LEFT JOIN users u ON cs.user_id = u.id
             ORDER BY s.created_at DESC LIMIT $1",
        )
        .bind(limit)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?
    };

    let sales = rows.into_iter().map(|r| {
        serde_json::json!({
            "id": r.get::<Uuid, _>("id"),
            "invoice_number": r.get::<String, _>("invoice_number"),
            "outlet_id": r.get::<Uuid, _>("outlet_id"),
            "outlet_name": r.get::<Option<String>, _>("outlet_name"),
            "outlet_code": r.get::<Option<String>, _>("outlet_code"),
            "shift_id": r.get::<Uuid, _>("shift_id"),
            "customer_id": r.get::<Option<Uuid>, _>("customer_id"),
            "customer_name": r.get::<Option<String>, _>("customer_name"),
            "customer_phone": r.get::<Option<String>, _>("customer_phone"),
            "terminal_id": r.get::<Option<String>, _>("terminal_id"),
            "cashier_name": r.get::<Option<String>, _>("cashier_name"),
            "warehouse_id": r.get::<Uuid, _>("warehouse_id"),
            "voucher_id": r.get::<Option<Uuid>, _>("voucher_id"),
            "subtotal": r.get::<Decimal, _>("subtotal"),
            "discount_amount": r.get::<Decimal, _>("discount_amount"),
            "tax_amount": r.get::<Decimal, _>("tax_amount"),
            "total_amount": r.get::<Decimal, _>("total_amount"),
            "paid_amount": r.get::<Decimal, _>("paid_amount"),
            "change_amount": r.get::<Decimal, _>("change_amount"),
            "total_cost_amount": r.get::<Decimal, _>("total_cost_amount"),
            "status": r.get::<String, _>("status"),
            "notes": r.get::<Option<String>, _>("notes"),
            "created_at": r.get::<chrono::DateTime<chrono::Utc>, _>("created_at").to_rfc3339(),
        })
    }).collect();

    Ok(sales)
}

pub async fn get_sale_detail(
    pool: &PgPool,
    sale_id: Uuid,
) -> Result<serde_json::Value, String> {
    let sale_row = sqlx::query(
        "SELECT s.*, c.name as customer_name, c.phone as customer_phone,
                cs.terminal_id, u.full_name as cashier_name,
                o.name as outlet_name, o.code as outlet_code
         FROM sales s
         LEFT JOIN outlets o ON s.outlet_id = o.id
         LEFT JOIN customers c ON s.customer_id = c.id
         LEFT JOIN cashier_shifts cs ON s.shift_id = cs.id
         LEFT JOIN users u ON cs.user_id = u.id
         WHERE s.id = $1",
    )
    .bind(sale_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?
    .ok_or_else(|| "Nota tidak ditemukan".to_string())?;

    let item_rows = sqlx::query(
        "SELECT si.*, p.name as product_name, p.sku as product_sku, p.barcode as product_barcode,
                u.code as unit_code
         FROM sale_items si
         JOIN products p ON si.product_id = p.id
         JOIN units u ON si.unit_id = u.id
         WHERE si.sale_id = $1",
    )
    .bind(sale_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?;

    let items: Vec<serde_json::Value> = item_rows.into_iter().map(|r| {
        serde_json::json!({
            "id": r.get::<Uuid, _>("id"),
            "product_id": r.get::<Uuid, _>("product_id"),
            "product_name": r.get::<String, _>("product_name"),
            "product_sku": r.get::<String, _>("product_sku"),
            "product_barcode": r.get::<String, _>("product_barcode"),
            "unit_id": r.get::<Uuid, _>("unit_id"),
            "unit_code": r.get::<String, _>("unit_code"),
            "quantity": r.get::<Decimal, _>("quantity"),
            "unit_cost_price": r.get::<Decimal, _>("unit_cost_price"),
            "unit_selling_price": r.get::<Decimal, _>("unit_selling_price"),
            "discount_percent": r.get::<Decimal, _>("discount_percent"),
            "discount_amount": r.get::<Decimal, _>("discount_amount"),
            "tax_amount": r.get::<Decimal, _>("tax_amount"),
            "subtotal": r.get::<Decimal, _>("subtotal"),
            "is_voided": r.get::<bool, _>("is_voided"),
        })
    }).collect();

    let payment_rows = sqlx::query(
        "SELECT sp.*, pm.name as payment_method_name, pm.category as payment_category
         FROM sale_payments sp
         JOIN payment_methods pm ON sp.payment_method_id = pm.id
         WHERE sp.sale_id = $1",
    )
    .bind(sale_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?;

    let payments: Vec<serde_json::Value> = payment_rows.into_iter().map(|r| {
        serde_json::json!({
            "id": r.get::<Uuid, _>("id"),
            "payment_method_id": r.get::<Uuid, _>("payment_method_id"),
            "payment_method_name": r.get::<String, _>("payment_method_name"),
            "category": r.get::<String, _>("payment_category"),
            "amount": r.get::<Decimal, _>("amount"),
            "card_number_masked": r.get::<Option<String>, _>("card_number_masked"),
            "reference_number": r.get::<Option<String>, _>("reference_number"),
            "created_at": r.get::<chrono::DateTime<chrono::Utc>, _>("created_at").to_rfc3339(),
        })
    }).collect();

    let sale_data = serde_json::json!({
        "id": sale_row.get::<Uuid, _>("id"),
        "invoice_number": sale_row.get::<String, _>("invoice_number"),
        "outlet_id": sale_row.get::<Uuid, _>("outlet_id"),
        "outlet_name": sale_row.get::<Option<String>, _>("outlet_name"),
        "outlet_code": sale_row.get::<Option<String>, _>("outlet_code"),
        "shift_id": sale_row.get::<Uuid, _>("shift_id"),
        "customer_id": sale_row.get::<Option<Uuid>, _>("customer_id"),
        "customer_name": sale_row.get::<Option<String>, _>("customer_name"),
        "customer_phone": sale_row.get::<Option<String>, _>("customer_phone"),
        "terminal_id": sale_row.get::<Option<String>, _>("terminal_id"),
        "cashier_name": sale_row.get::<Option<String>, _>("cashier_name"),
        "warehouse_id": sale_row.get::<Uuid, _>("warehouse_id"),
        "voucher_id": sale_row.get::<Option<Uuid>, _>("voucher_id"),
        "subtotal": sale_row.get::<Decimal, _>("subtotal"),
        "discount_amount": sale_row.get::<Decimal, _>("discount_amount"),
        "tax_amount": sale_row.get::<Decimal, _>("tax_amount"),
        "total_amount": sale_row.get::<Decimal, _>("total_amount"),
        "paid_amount": sale_row.get::<Decimal, _>("paid_amount"),
        "change_amount": sale_row.get::<Decimal, _>("change_amount"),
        "total_cost_amount": sale_row.get::<Decimal, _>("total_cost_amount"),
        "status": sale_row.get::<String, _>("status"),
        "notes": sale_row.get::<Option<String>, _>("notes"),
        "created_at": sale_row.get::<chrono::DateTime<chrono::Utc>, _>("created_at").to_rfc3339(),
        "items": items,
        "payments": payments,
    });

    Ok(sale_data)
}

pub async fn get_sales_stats(pool: &PgPool) -> Result<serde_json::Value, String> {
    let row = sqlx::query(
        "SELECT 
            COALESCE(SUM(total_amount), 0) as total_revenue,
            COUNT(*) as total_count,
            COALESCE(SUM(subtotal), 0) as total_subtotal,
            COALESCE(SUM(discount_amount), 0) as total_discount
         FROM sales 
         WHERE status = 'COMPLETED'"
    )
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?;

    let total_revenue: Decimal = row.get("total_revenue");
    let total_count: i64 = row.get("total_count");
    let total_discount: Decimal = row.get("total_discount");

    let items_sold = sqlx::query_scalar::<_, Decimal>(
        "SELECT COALESCE(SUM(si.quantity), 0) 
         FROM sale_items si 
         JOIN sales s ON si.sale_id = s.id 
         WHERE s.status = 'COMPLETED'"
    )
    .fetch_one(pool)
    .await
    .unwrap_or(Decimal::ZERO);

    let avg_basket = if total_count > 0 {
        total_revenue / Decimal::from(total_count)
    } else {
        Decimal::ZERO
    };

    Ok(serde_json::json!({
        "total_revenue": total_revenue,
        "total_transactions": total_count,
        "total_items_sold": items_sold,
        "total_discount": total_discount,
        "avg_basket_size": avg_basket,
    }))
}
