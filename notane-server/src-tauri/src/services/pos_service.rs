use crate::models::product::{
    CustomerGroupPriceDetail, OutletCustomerGroupPriceDetail, Product, ProductDetail,
    ProductPriceTier, ProductUnitDetail,
};
use crate::models::sales::{
    CheckoutItemResponse, CheckoutPaymentResponse, CheckoutRequest, CheckoutResponse, Sale,
    SaleItem, VoidSaleRequest,
};
use crate::models::settings::LoyaltySettings;
use crate::models::shift::{CashierShift, CloseShiftRequest, OpenShiftRequest, ShiftSummaryResponse};
use crate::services::audit_service::log_audit;
use crate::services::auth_service::verify_supervisor_auth;
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
        "SELECT cs.*, o.name as outlet_name, o.code as outlet_code
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
    if req.supervisor_auth_token.is_some() || req.supervisor_pin.is_some() {
        let spv_id = verify_supervisor_auth(
            pool,
            req.supervisor_id,
            req.supervisor_pin.as_deref(),
            req.supervisor_auth_token.as_deref(),
            "SHIFT_OPEN",
        )
        .await?;
        supervisor_approval_id = Some(spv_id);
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
        "SELECT cs.*, o.name as outlet_name, o.code as outlet_code 
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
            pm.id as payment_method_id,
            pm.name as payment_method_name,
            pm.category,
            COALESCE(SUM(sp.amount), 0) as total_amount,
            COUNT(sp.id)::bigint as tx_count
        FROM sale_payments sp
        JOIN sales s ON sp.sale_id = s.id
        JOIN payment_methods pm ON sp.payment_method_id = pm.id
        WHERE s.shift_id = $1 AND s.status = 'COMPLETED'
        GROUP BY pm.id, pm.name, pm.category",
    )
    .bind(shift.id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| format!("Gagal mengkalkulasi rincian pembayaran: {}", e))?;

    let mut payment_breakdowns = Vec::new();
    let mut total_cash = Decimal::ZERO;
    let mut total_qris = Decimal::ZERO;
    let mut total_card = Decimal::ZERO;
    let mut total_points = Decimal::ZERO;
    let mut total_points_count: i64 = 0;

    for p in payments_rows {
        let payment_method_id: Uuid = p.get("payment_method_id");
        let payment_method_name: String = p.get("payment_method_name");
        let cat: String = p.get("category");
        let amt: Decimal = p.get("total_amount");
        let count: i64 = p.get("tx_count");
        match cat.as_str() {
            "CASH" => total_cash += amt,
            "QRIS" => total_qris += amt,
            "EDC" => total_card += amt,
            "POINTS" => {
                total_points += amt;
                total_points_count += count;
            }
            _ => (),
        }
        payment_breakdowns.push(crate::models::shift::ShiftPaymentBreakdown {
            payment_method_id,
            payment_method_name,
            category: cat,
            total_amount: amt,
            transaction_count: count,
        });
    }

    // Hitung total pengeluaran kas operasional yang diambil dari laci shift ini
    let shift_expenses: Decimal = sqlx::query_scalar::<_, Decimal>(
        "SELECT COALESCE(SUM(amount), 0) FROM expense_transactions WHERE shift_id = $1 AND status = 'COMPLETED'"
    )
    .bind(shift.id)
    .fetch_one(pool)
    .await
    .unwrap_or(Decimal::ZERO);

    // Hitung total setor kas (Cash Drop) laci kasir ke supervisor/brankas
    let shift_cash_drops: Decimal = sqlx::query_scalar::<_, Decimal>(
        "SELECT COALESCE(SUM(amount), 0) FROM cash_drops WHERE shift_id = $1"
    )
    .bind(shift.id)
    .fetch_one(pool)
    .await
    .unwrap_or(Decimal::ZERO);

    // Hitung total refund retur tunai konsumen dari laci
    let shift_sales_returns: Decimal = sqlx::query_scalar::<_, Decimal>(
        "SELECT COALESCE(SUM(refund_amount), 0) FROM sales_returns WHERE shift_id = $1 AND refund_method = 'CASH'"
    )
    .bind(shift.id)
    .fetch_one(pool)
    .await
    .unwrap_or(Decimal::ZERO);

    let expected_cash = shift.opening_cash + total_cash - shift_expenses - shift_cash_drops - shift_sales_returns;
    let discrepancy = req.actual_cash - expected_cash;

    let mut supervisor_id = None;
    if req.supervisor_auth_token.is_some() || req.supervisor_pin.is_some() {
        let spv_id = verify_supervisor_auth(
            pool,
            req.supervisor_id,
            req.supervisor_pin.as_deref(),
            req.supervisor_auth_token.as_deref(),
            "SHIFT_CLOSE",
        )
        .await?;
        supervisor_id = Some(spv_id);
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
                total_debit, total_credit, is_closing_entry, created_at, updated_at, created_by, updated_by
            ) VALUES ($1, $2, CURRENT_DATE, 'SHIFT_DISCREPANCY', $3, $4, $5, $5, FALSE, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $6, $6)",
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
                "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, $3, 0, 'Lebih kas fisik', $5), ($1, $4, 0, $3, 'Pendapatan lebih kas', $5)",
            )
            .bind(journal_id)
            .bind(kas_id)
            .bind(abs_diff)
            .bind(rev_id)
            .bind(shift.outlet_id)
            .execute(&mut *tx).await.map_err(|e| format!("Gagal mencatat baris jurnal: {}", e))?;
        } else {
            let exp_id: Uuid = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '5201'")
                .fetch_one(&mut *tx).await.map_err(|_| "Akun 5201 tidak ditemukan")?;
            sqlx::query(
                "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, $3, 0, 'Beban kurang kas fisik', $5), ($1, $4, 0, $3, 'Kredit kas fisik', $5)",
            )
            .bind(journal_id)
            .bind(exp_id)
            .bind(abs_diff)
            .bind(kas_id)
            .bind(shift.outlet_id)
            .execute(&mut *tx).await.map_err(|e| format!("Gagal mencatat baris jurnal: {}", e))?;
        }
    }

    tx.commit().await.map_err(|e| format!("Commit error: {}", e))?;

    let cashier_name = sqlx::query_scalar::<_, String>("SELECT full_name FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap_or_else(|_| "Kasir".to_string());

    let outlet_name = if let Some(oid) = shift.outlet_id {
        sqlx::query_scalar::<_, String>("SELECT name FROM outlets WHERE id = $1")
            .bind(oid)
            .fetch_optional(pool)
            .await
            .unwrap_or(None)
    } else {
        None
    };

    Ok(ShiftSummaryResponse {
        shift: updated_shift.clone(),
        cashier_name,
        outlet_name,
        opened_at: shift.opened_at,
        closed_at: updated_shift.closed_at,
        total_sales_count,
        total_gross_sales: gross_sales,
        total_discounts: discounts,
        total_tax: tax,
        total_net_sales: net_sales,
        opening_cash: shift.opening_cash,
        total_cash_tendered: total_cash,
        total_cash_change: Decimal::ZERO,
        net_cash_sales: total_cash,
        total_expenses: shift_expenses,
        expected_cash_in_drawer: expected_cash,
        actual_cash_counted: Some(req.actual_cash),
        discrepancy: Some(discrepancy),
        total_cash_received: total_cash,
        total_qris_received: total_qris,
        total_card_received: total_card,
        total_points_redeemed: total_points,
        total_points_count: total_points_count,
        total_voucher_discount: discounts,
        payment_breakdowns,
    })
}

pub async fn lookup_product(
    pool: &PgPool,
    code: &str,
    warehouse_id: Option<Uuid>,
    outlet_id: Option<Uuid>,
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
        } else if let Ok(uid) = Uuid::parse_str(&parsed.raw_barcode) {
            sqlx::query_scalar::<_, Uuid>("SELECT id FROM products WHERE id = $1")
                .bind(uid)
                .fetch_optional(pool)
                .await
                .map_err(|e| format!("Database error: {}", e))?
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

    let mut product = sqlx::query_as::<_, Product>("SELECT * FROM products WHERE id = $1")
        .bind(product_id)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    // Periksa apakah ada harga khusus outlet (Multi-Outlet Pricing)
    let outlet_price_opt = if let Some(oid) = outlet_id.or(warehouse_id) {
        sqlx::query_as::<_, (Decimal, Decimal, Option<Decimal>)>(
            "SELECT cost_price, base_selling_price, last_purchase_price FROM product_outlet_prices WHERE outlet_id = $1 AND product_id = $2"
        )
        .bind(oid)
        .bind(product.id)
        .fetch_optional(pool)
        .await
        .unwrap_or(None)
    } else {
        None
    };

    if let Some(ref op) = outlet_price_opt {
        product.cost_price = op.0;
        product.base_selling_price = op.1;
    }

    // Selaraskan HPP (cost_price) dengan metode costing yang berlaku
    let eff_method = if let Ok(mut conn) = pool.acquire().await {
        let (method, _, _) = crate::services::costing_service::resolve_effective_costing_method_tx(
            &mut conn,
            Some(&product.costing_method),
            false,
            None,
        ).await.unwrap_or_else(|_| ("WMA".to_string(), false, None));
        method
    } else {
        "WMA".to_string()
    };

    if let Some(toid) = outlet_id.or(warehouse_id) {
        if eff_method == "FIFO" || eff_method == "FEFO" {
            let active_batch_cost = sqlx::query_scalar::<_, Decimal>(
                "SELECT cost_price 
                 FROM product_stock_batches 
                 WHERE product_id = $1 AND outlet_id = $2 AND remaining_qty > 0 
                 ORDER BY expiry_date ASC NULLS LAST, received_at ASC, id ASC 
                 LIMIT 1"
            )
            .bind(product.id)
            .bind(toid)
            .fetch_optional(pool)
            .await
            .unwrap_or(None);

            if let Some(bc) = active_batch_cost {
                if bc > Decimal::ZERO {
                    product.cost_price = bc;
                }
            }
        } else if eff_method == "REPLACEMENT_COST" {
            if let Some((_, _, Some(last_p))) = outlet_price_opt {
                if last_p > Decimal::ZERO {
                    product.cost_price = last_p;
                }
            }
        }
    } else if eff_method == "FIFO" || eff_method == "FEFO" {
        let active_batch_cost = sqlx::query_scalar::<_, Decimal>(
            "SELECT cost_price 
             FROM product_stock_batches 
             WHERE product_id = $1 AND remaining_qty > 0 
             ORDER BY expiry_date ASC NULLS LAST, received_at ASC, id ASC 
             LIMIT 1"
        )
        .bind(product.id)
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

        if let Some(bc) = active_batch_cost {
            if bc > Decimal::ZERO {
                product.cost_price = bc;
            }
        }
    }

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

    // Harga member spesifik per outlet (prioritas lebih tinggi dari global group price)
    let outlet_cgp_rows = if let Some(oid) = outlet_id {
        sqlx::query(
            "SELECT ocgp.id, ocgp.outlet_id, ocgp.customer_group_id,
                    cg.name as customer_group_name, ocgp.product_id, ocgp.special_price
             FROM outlet_customer_group_prices ocgp
             JOIN customer_groups cg ON ocgp.customer_group_id = cg.id
             WHERE ocgp.product_id = $1 AND ocgp.outlet_id = $2 AND cg.is_active = true
             ORDER BY cg.id ASC",
        )
        .bind(product.id)
        .bind(oid)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
    } else {
        Vec::new()
    };

    let mut outlet_customer_group_prices = Vec::new();
    for r in outlet_cgp_rows {
        outlet_customer_group_prices.push(OutletCustomerGroupPriceDetail {
            id: r.get("id"),
            outlet_id: r.get("outlet_id"),
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
    } else if let Some(oid) = outlet_id {
        sqlx::query_scalar::<_, Decimal>(
            "SELECT quantity FROM warehouse_stocks WHERE warehouse_id = $1 AND product_id = $2",
        )
        .bind(oid)
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
        outlet_customer_group_prices,
        outlet_prices: None,
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
        return Err("Keranjang belanja kosong".to_string());
    }

    let mut tx = pool.begin().await.map_err(|e| format!("Database error: {}", e))?;

    // Check active shift: ambil dari req.shift_id jika ada, atau otomatis cari shift aktif kasir yang login
    let shift = if let Some(sid) = req.shift_id {
        sqlx::query_as::<_, CashierShift>(
            "SELECT * FROM cashier_shifts WHERE id = $1 AND status = 'OPEN' FOR UPDATE"
        )
        .bind(sid)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or_else(|| "Shift kasir tidak aktif atau sudah ditutup".to_string())?
    } else {
        sqlx::query_as::<_, CashierShift>(
            "SELECT * FROM cashier_shifts WHERE user_id = $1 AND status = 'OPEN' ORDER BY opened_at DESC LIMIT 1 FOR UPDATE"
        )
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or_else(|| "Tidak ditemukan shift kasir aktif untuk akun kasir ini. Harap buka shift kasir terlebih dahulu.".to_string())?
    };

    let now = Utc::now();
    let invoice_number = format!("INV-{}-{}", now.format("%Y%m%d"), &Uuid::new_v4().to_string()[..8].to_uppercase());
    let sale_id = Uuid::new_v4();

    // 1. Resolve outlet_id: prioritaskan outlet yang terikat pada shift aktif kasir
    let outlet_id = if let Some(shift_oid) = shift.outlet_id {
        shift_oid
    } else if let Some(oid) = req.outlet_id {
        oid
    } else {
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM outlets WHERE is_active = TRUE ORDER BY created_at ASC LIMIT 1")
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Tidak ada outlet aktif yang ditemukan".to_string())?
    };

    // Validasi proteksi tutup buku untuk outlet transaksi
    let today = chrono::Local::now().date_naive();
    crate::services::accounting_service::check_transaction_date_allowed(pool, today, Some(outlet_id)).await?;

    // 2. Resolve warehouse_id for physical inventory deduction (outlet dan gudang merupakan satu kesatuan)
    let target_warehouse_id = Some(outlet_id);

    // Pre-flight Stock Validation: Validasi akumulasi kebutuhan stok vs stok riil di gudang/outlet
    if let Some(wh_id) = target_warehouse_id {
        let mut required_stock_map: std::collections::HashMap<Uuid, (String, String, String, Decimal)> = std::collections::HashMap::new();

        for item in &req.items {
            let prod_row = sqlx::query_as::<_, (String, String, Uuid)>(
                "SELECT name, sku, base_unit_id FROM products WHERE id = $1"
            )
            .bind(item.product_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| format!("Database error: {}", e))?
            .ok_or_else(|| format!("Produk tidak ditemukan"))?;

            let (factor, base_unit_code) = if item.unit_id == prod_row.2 {
                let code = sqlx::query_scalar::<_, String>("SELECT code FROM units WHERE id = $1")
                    .bind(prod_row.2)
                    .fetch_optional(&mut *tx)
                    .await
                    .unwrap_or(None)
                    .unwrap_or_else(|| "PCS".to_string());
                (Decimal::ONE, code)
            } else {
                let row = sqlx::query_as::<_, (Decimal, String)>(
                    "SELECT pu.conversion_factor, u.code FROM product_units pu JOIN units u ON pu.unit_id = u.id WHERE pu.product_id = $1 AND pu.unit_id = $2"
                )
                .bind(item.product_id)
                .bind(item.unit_id)
                .fetch_optional(&mut *tx)
                .await
                .unwrap_or(None);

                row.unwrap_or((Decimal::ONE, "PCS".to_string()))
            };

            let entry = required_stock_map.entry(item.product_id).or_insert((prod_row.0, prod_row.1, base_unit_code, Decimal::ZERO));
            entry.3 += item.quantity * factor;
        }

        for (prod_id, (pname, psku, punit, req_qty)) in required_stock_map {
            let current_stock = sqlx::query_scalar::<_, Decimal>(
                "SELECT quantity FROM warehouse_stocks WHERE warehouse_id = $1 AND product_id = $2 FOR UPDATE"
            )
            .bind(wh_id)
            .bind(prod_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| format!("Database error: {}", e))?
            .unwrap_or(Decimal::ZERO);

            if current_stock < req_qty {
                return Err(format!(
                    "Stok tidak mencukupi untuk '{}' (SKU: {}). Stok tersedia: {} {}, total diminta: {} {}",
                    pname, psku, current_stock, punit, req_qty, punit
                ));
            }
        }
    }

    // =========================================================================
    // AUTHORITATIVE SERVER-SIDE PRICE & DISCOUNT RE-CALCULATION ENGINE (ZERO-TRUST)
    // Sisi klien tidak dipercaya untuk nominal harga, diskon member, promo, voucher, atau pajak.
    // Seluruh kalkulasi dihitung ulang 100% dari sumber otoritatif database.
    // =========================================================================

    // 1. Ambil Identitas Customer Group Pelanggan (jika terdaftar)
    let customer_group_id: Option<i32> = if let Some(cid) = req.customer_id {
        sqlx::query_scalar::<_, Option<i32>>("SELECT customer_group_id FROM customers WHERE id = $1")
            .bind(cid)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| format!("Database error fetching customer group: {}", e))?
            .flatten()
    } else {
        None
    };

    // 2. Ambil Promo Flash Sale yang Sedang Aktif di Outlet ini
    let active_flash_sales = crate::services::flash_sale_service::get_active_promos_for_outlet(
        pool,
        Some(outlet_id),
    )
    .await
    .unwrap_or_default();

    // 3. Pisahkan item yang diklaim sebagai hadiah gratis vs item bayar untuk validasi promo retail
    let mut claimed_free_items: std::collections::HashMap<Uuid, Decimal> = std::collections::HashMap::new();
    let mut paid_evaluation_items: Vec<crate::api::promotions::EvaluatePromotionItemInput> = Vec::new();

    for item in &req.items {
        let is_free = item.unit_selling_price <= Decimal::ZERO 
            || item.discount_amount.unwrap_or(Decimal::ZERO) >= (item.unit_selling_price * item.quantity);
        if is_free {
            *claimed_free_items.entry(item.product_id).or_insert(Decimal::ZERO) += item.quantity;
        } else {
            paid_evaluation_items.push(crate::api::promotions::EvaluatePromotionItemInput {
                product_id: item.product_id,
                unit_id: Some(item.unit_id),
                quantity: item.quantity,
                unit_selling_price: item.unit_selling_price,
            });
        }
    }

    // 4. Evaluasi Promo Retail Otomatis (Buy X Get Y, Hadiah Bonus, Diskon Paket)
    let promo_eval = crate::api::promotions::evaluate_promotions_logic(
        pool,
        &crate::api::promotions::EvaluatePromotionsRequest {
            outlet_id: Some(outlet_id),
            customer_id: req.customer_id,
            items: paid_evaluation_items,
            dismissed_rule_ids: Vec::new(),
        },
    )
    .await
    .map_err(|e| format!("Gagal memverifikasi promosi backend: {}", e))?;

    let mut allowed_free_items: std::collections::HashMap<Uuid, Decimal> = std::collections::HashMap::new();
    for gift in &promo_eval.auto_gifts {
        *allowed_free_items.entry(gift.product_id).or_insert(Decimal::ZERO) += gift.quantity;
    }
    for adj in &promo_eval.item_adjustments {
        if adj.is_free_item {
            *allowed_free_items.entry(adj.product_id).or_insert(Decimal::ZERO) += adj.free_quantity;
        }
    }

    // Validasi Zero-Trust: Pastikan klaim item hadiah gratis tidak melebihi kuota sah
    for (prod_id, claimed_qty) in &claimed_free_items {
        let allowed_qty = *allowed_free_items.get(prod_id).unwrap_or(&Decimal::ZERO);
        if *claimed_qty > allowed_qty {
            return Err(format!(
                "Validasi Keamanan Gagal: Item gratis/hadiah yang diklaim ({}) melebihi kuota promosi yang sah ({}) sesuai aturan sistem di database.",
                claimed_qty, allowed_qty
            ));
        }
    }

    let mut retail_promo_discounts: std::collections::HashMap<Uuid, Decimal> = std::collections::HashMap::new();
    for adj in &promo_eval.item_adjustments {
        if !adj.is_free_item {
            retail_promo_discounts.insert(adj.product_id, adj.discount_amount);
        }
    }

    let mut subtotal = Decimal::ZERO;
    let mut total_discount = Decimal::ZERO;
    let mut total_tax = Decimal::ZERO;
    let mut total_cost = Decimal::ZERO;
    let mut total_flash_sale_discount = Decimal::ZERO;
    let mut total_member_discount = Decimal::ZERO;
    let mut total_retail_promo_discount = Decimal::ZERO;

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
        flash_sale_id: Option<Uuid>,
        original_price: Decimal,
        flash_sale_discount: Decimal,
        member_discount: Decimal,
        batch_allocations: Vec<crate::services::costing_service::BatchDeductionDetail>,
    }

    let mut prepared_items = Vec::new();
    let mut allocated_free_items: std::collections::HashMap<Uuid, Decimal> = std::collections::HashMap::new();

    for item in &req.items {
        let mut product = sqlx::query_as::<_, Product>(
            "SELECT * FROM products WHERE id = $1",
        )
        .bind(item.product_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| format!("Produk tidak ditemukan: {}", e))?;

        // Check if outlet-specific price/cost exists
        let outlet_price_data = sqlx::query_as::<_, (Decimal, Decimal, Option<Decimal>)>(
            "SELECT cost_price, base_selling_price, last_purchase_price FROM product_outlet_prices WHERE outlet_id = $1 AND product_id = $2"
        )
        .bind(outlet_id)
        .bind(product.id)
        .fetch_optional(&mut *tx)
        .await
        .unwrap_or(None);

        let last_purchase_price = outlet_price_data.as_ref().and_then(|op| op.2);
        if let Some(ref op) = outlet_price_data {
            product.cost_price = op.0;
            product.base_selling_price = op.1;
        }

        let (factor, unit_code, unit_specific_price) = if item.unit_id == product.base_unit_id {
            let uc = sqlx::query_scalar::<_, String>("SELECT code FROM units WHERE id = $1")
                .bind(product.base_unit_id)
                .fetch_optional(&mut *tx)
                .await
                .unwrap_or(None)
                .unwrap_or_else(|| "PCS".to_string());
            (Decimal::ONE, uc, None)
        } else {
            let row = sqlx::query("SELECT pu.conversion_factor, pu.selling_price, u.code as unit_code FROM product_units pu JOIN units u ON pu.unit_id = u.id WHERE pu.product_id = $1 AND pu.unit_id = $2")
                .bind(product.id)
                .bind(item.unit_id)
                .fetch_optional(&mut *tx)
                .await
                .unwrap_or(None);
            if let Some(r) = row {
                (
                    r.get::<Decimal, _>("conversion_factor"),
                    r.get::<String, _>("unit_code"),
                    r.get::<Option<Decimal>, _>("selling_price"),
                )
            } else {
                (Decimal::ONE, "PCS".to_string(), None)
            }
        };

        let base_qty = item.quantity * factor;

        let (effective_costing_method, _, _) = crate::services::costing_service::resolve_effective_costing_method_tx(
            &mut *tx,
            Some(&product.costing_method),
            false,
            None,
        ).await.unwrap_or_else(|_| ("WMA".to_string(), false, None));

        // Alokasikan batch stok fisik (FEFO/FIFO) dan hitung HPP
        let (fifo_cogs, fifo_unit_cost, batch_allocations) = if let Some(wh_id) = target_warehouse_id {
            crate::services::costing_service::allocate_and_consume_stock_batches(
                &mut tx,
                product.id,
                wh_id,
                base_qty,
                product.cost_price,
            ).await?
        } else {
            (product.cost_price * base_qty, product.cost_price, Vec::new())
        };

        let (line_cost, effective_cost_price_base) = match effective_costing_method.as_str() {
            "FIFO" | "FEFO" => (fifo_cogs, fifo_unit_cost),
            "REPLACEMENT_COST" => {
                let repl_cost_base = last_purchase_price
                    .filter(|p| *p > Decimal::ZERO)
                    .unwrap_or(product.cost_price);
                (repl_cost_base * base_qty, repl_cost_base)
            }
            _ => {
                // WMA (Weighted Moving Average) / Default
                (product.cost_price * base_qty, product.cost_price)
            }
        };

        let unit_cost_for_item = (effective_cost_price_base * factor).round_dp(4);

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

            if current_stock < base_qty {
                return Err(format!(
                    "Stok tidak mencukupi untuk '{}' (SKU: {}). Tersedia: {} {}, Diminta: {} {}",
                    product.name, product.sku, current_stock, unit_code, base_qty, unit_code
                ));
            }

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
            .bind(effective_cost_price_base)
            .bind(format!("Penjualan POS {}", invoice_number))
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Gagal mencatat mutasi stok: {}", e))?;
        }

        // =====================================================================
        // KALKULASI HARGA & DISKON RESMI SERVER
        // =====================================================================
        let is_claimed_free = item.unit_selling_price <= Decimal::ZERO 
            || item.discount_amount.unwrap_or(Decimal::ZERO) >= (item.unit_selling_price * item.quantity);

        let is_valid_free_gift = if is_claimed_free {
            let allowed = *allowed_free_items.get(&product.id).unwrap_or(&Decimal::ZERO);
            let allocated = *allocated_free_items.get(&product.id).unwrap_or(&Decimal::ZERO);
            if allocated + item.quantity <= allowed {
                *allocated_free_items.entry(product.id).or_insert(Decimal::ZERO) += item.quantity;
                true
            } else {
                false
            }
        } else {
            false
        };

        // 1. Harga Acuan Normal Asli Database
        let normal_base_price = if let Some(sp) = unit_specific_price {
            if sp > Decimal::ZERO { sp } else { product.base_selling_price * factor }
        } else {
            product.base_selling_price * factor
        };

        let (
            line_selling_price,
            item_orig_price,
            item_fs_disc,
            item_mem_disc,
            line_disc,
            line_tax,
            line_subtotal,
            applied_fs_id,
        ) = if is_valid_free_gift {
            // Item Hadiah Gratis Resmi:
            (
                Decimal::ZERO,
                normal_base_price,
                Decimal::ZERO,
                Decimal::ZERO,
                normal_base_price * item.quantity,
                Decimal::ZERO,
                Decimal::ZERO,
                None,
            )
        } else {
            // 2. Evaluasi Tier Harga Grosir (Quantity Breaks)
            let mut active_unit_base_price = normal_base_price;
            let tier_price_opt = sqlx::query_scalar::<_, Decimal>(
                "SELECT tier_price FROM product_price_tiers WHERE product_id = $1 AND min_quantity <= $2 ORDER BY min_quantity DESC LIMIT 1"
            )
            .bind(product.id)
            .bind(item.quantity)
            .fetch_optional(&mut *tx)
            .await
            .unwrap_or(None);

            if let Some(tp) = tier_price_opt {
                active_unit_base_price = tp * factor;
            }

            // 3. Evaluasi Harga Khusus Member
            let mut member_savings_unit = Decimal::ZERO;
            let mut member_price_unit = active_unit_base_price;
            if let Some(cg_id) = customer_group_id {
                let outlet_cgp = sqlx::query_scalar::<_, Decimal>(
                    "SELECT special_price FROM outlet_customer_group_prices WHERE product_id = $1 AND customer_group_id = $2 AND outlet_id = $3"
                )
                .bind(product.id)
                .bind(cg_id)
                .bind(outlet_id)
                .fetch_optional(&mut *tx)
                .await
                .unwrap_or(None);

                let member_special = if outlet_cgp.is_some() {
                    outlet_cgp
                } else {
                    sqlx::query_scalar::<_, Decimal>(
                        "SELECT special_price FROM customer_group_prices WHERE product_id = $1 AND customer_group_id = $2"
                    )
                    .bind(product.id)
                    .bind(cg_id)
                    .fetch_optional(&mut *tx)
                    .await
                    .unwrap_or(None)
                };

                if let Some(sp) = member_special {
                    let sp_unit = sp * factor;
                    if sp_unit < active_unit_base_price {
                        member_savings_unit = active_unit_base_price - sp_unit;
                        member_price_unit = sp_unit;
                    }
                }
            }

            // 4. Evaluasi Promo Flash Sale Aktif
            let mut fs_savings_unit = Decimal::ZERO;
            let mut fs_price_unit = active_unit_base_price;
            let mut fs_id_found = None;
            let mut fs_is_stackable = false;

            if let Some(fs) = active_flash_sales.iter().find(|f| f.product_id == product.id) {
                let quota_ok = match fs.remaining_quota {
                    Some(q) => q > Decimal::ZERO,
                    None => true,
                };
                if quota_ok {
                    let promo_unit = fs.promo_price * factor;
                    if promo_unit < active_unit_base_price {
                        fs_savings_unit = active_unit_base_price - promo_unit;
                        fs_price_unit = promo_unit;
                        fs_id_found = Some(fs.flash_sale_id);
                        fs_is_stackable = fs.is_stackable_with_member;
                    }
                }
            }

            // 5. Kombinasi Harga Member vs Flash Sale (Best Value Rule / Stackable)
            let (final_unit_price, fs_disc_total, mem_disc_total, final_fs_id) = if fs_id_found.is_some() && member_savings_unit > Decimal::ZERO {
                if fs_is_stackable {
                    let combined = (fs_savings_unit + member_savings_unit).min(active_unit_base_price);
                    let final_p = active_unit_base_price - combined;
                    (final_p, fs_savings_unit * item.quantity, member_savings_unit * item.quantity, fs_id_found)
                } else {
                    // Best Value Rule: Pilih yang paling murah untuk konsumen
                    if fs_price_unit <= member_price_unit {
                        (fs_price_unit, fs_savings_unit * item.quantity, Decimal::ZERO, fs_id_found)
                    } else {
                        (member_price_unit, Decimal::ZERO, member_savings_unit * item.quantity, None)
                    }
                }
            } else if fs_id_found.is_some() {
                (fs_price_unit, fs_savings_unit * item.quantity, Decimal::ZERO, fs_id_found)
            } else if member_savings_unit > Decimal::ZERO {
                (member_price_unit, Decimal::ZERO, member_savings_unit * item.quantity, None)
            } else {
                (active_unit_base_price, Decimal::ZERO, Decimal::ZERO, None)
            };

            // 6. Evaluasi Diskon Promo Retail Otomatis (Buy X Get Y)
            let promo_retail_disc = *retail_promo_discounts.get(&product.id).unwrap_or(&Decimal::ZERO);

            // 7. Evaluasi Diskon Manual Kasir (jika ada)
            let line_gross_before_manual = final_unit_price * item.quantity;
            let manual_disc_amount = if let Some(pct) = item.discount_percent {
                if pct > Decimal::ZERO {
                    (line_gross_before_manual * pct) / Decimal::from(100)
                } else {
                    item.discount_amount.unwrap_or(Decimal::ZERO)
                }
            } else {
                item.discount_amount.unwrap_or(Decimal::ZERO)
            };
            let safe_manual_disc = manual_disc_amount.min(line_gross_before_manual).max(Decimal::ZERO);

            // 8. Total Diskon & Pajak Resmi
            let total_item_disc = fs_disc_total + mem_disc_total + promo_retail_disc + safe_manual_disc;
            let line_gross_total = normal_base_price * item.quantity;
            let taxable_amount = (line_gross_total - total_item_disc).max(Decimal::ZERO);

            let calculated_tax = if product.tax_percent > Decimal::ZERO {
                ((taxable_amount * product.tax_percent) / Decimal::from(100)).round_dp(2)
            } else {
                Decimal::ZERO
            };

            let calculated_subtotal = taxable_amount + calculated_tax;

            (
                final_unit_price,
                normal_base_price,
                fs_disc_total,
                mem_disc_total,
                total_item_disc,
                calculated_tax,
                calculated_subtotal,
                final_fs_id,
            )
        };

        total_flash_sale_discount += item_fs_disc;
        total_member_discount += item_mem_disc;
        if let Some(r_disc) = retail_promo_discounts.get(&product.id) {
            total_retail_promo_discount += *r_disc;
        }

        // Jika item ini merupakan bagian dari promo Flash Sale, update kuota promo yang terjual
        if let Some(fs_id) = applied_fs_id {
            let _ = sqlx::query(
                "UPDATE flash_sale_items SET sold_qty = sold_qty + $1 WHERE flash_sale_id = $2 AND product_id = $3"
            )
            .bind(item.quantity)
            .bind(fs_id)
            .bind(product.id)
            .execute(&mut *tx)
            .await;
        }

        let line_gross = normal_base_price * item.quantity;
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
            cost_price: unit_cost_for_item,
            selling_price: line_selling_price,
            disc_percent: item.discount_percent.unwrap_or(Decimal::ZERO),
            disc_amount: line_disc,
            tax_amount: line_tax,
            item_subtotal: line_subtotal,
            flash_sale_id: applied_fs_id,
            original_price: item_orig_price,
            flash_sale_discount: item_fs_disc,
            member_discount: item_mem_disc,
            batch_allocations,
        });
    }

    // =========================================================================
    // VOUCHER HANDLING (SERVER RE-EVALUATION)
    // =========================================================================
    let mut voucher_id = None;
    let mut voucher_discount = Decimal::ZERO;
    if let Some(vcode) = &req.voucher_code {
        if !vcode.trim().is_empty() {
            let clean_vcode = vcode.trim().to_uppercase();
            let v_opt = sqlx::query(
                "SELECT id, discount_type, discount_value, min_purchase_amount, max_discount_amount, quota, used_count 
                 FROM vouchers WHERE code = $1 AND status = 'ACTIVE' AND valid_from <= CURRENT_TIMESTAMP AND valid_until >= CURRENT_TIMESTAMP",
            )
            .bind(&clean_vcode)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| format!("Database error: {}", e))?;

            if let Some(v) = v_opt {
                let quota: i32 = v.get::<Option<i32>, _>("quota").unwrap_or(0);
                let used_count: i32 = v.get::<Option<i32>, _>("used_count").unwrap_or(0);

                if quota > 0 && used_count >= quota {
                    return Err(format!(
                        "Kuota penggunaan voucher '{}' telah habis ({}/{})",
                        clean_vcode, used_count, quota
                    ));
                }

                let min_purchase: Decimal = v.get("min_purchase_amount");
                if subtotal < min_purchase {
                    return Err(format!(
                        "Syarat minimal belanja untuk voucher '{}' adalah Rp {}, total belanja saat ini: Rp {}",
                        clean_vcode, min_purchase, subtotal
                    ));
                }

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

                voucher_discount = v_disc;
                total_discount += v_disc;
                voucher_id = Some(v.get::<Uuid, _>("id"));
            } else {
                return Err(format!("Kode voucher '{}' tidak valid atau telah kadaluarsa", clean_vcode));
            }
        }
    }

    let final_total = (subtotal - total_discount) + total_tax;

    let loyalty_setting: Option<LoyaltySettings> = sqlx::query_as::<_, LoyaltySettings>(
        "SELECT * FROM loyalty_settings ORDER BY created_at ASC LIMIT 1",
    )
    .fetch_optional(&mut *tx)
    .await
    .unwrap_or(None);

    #[derive(Debug, Clone)]
    struct ResolvedPayment {
        payment_method_id: Uuid,
        payment_method_name: String,
        category: String,
        amount: Decimal,
        points_redeemed: Option<i32>,
        card_number_masked: Option<String>,
        reference_number: Option<String>,
        payment_proof_url: Option<String>,
    }

    let mut resolved_payments: Vec<ResolvedPayment> = Vec::new();
    for p in &req.payments {
        let (pm_name, pm_cat) = sqlx::query_as::<_, (String, String)>(
            "SELECT name, category FROM payment_methods WHERE id = $1",
        )
        .bind(p.payment_method_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| format!("Gagal memverifikasi metode pembayaran: {}", e))?
        .ok_or_else(|| "Metode pembayaran tidak ditemukan".to_string())?;

        if pm_cat == "POINTS" {
            let cid = req.customer_id.ok_or_else(|| {
                "Pembayaran menggunakan poin reward memerlukan data pelanggan (member) yang valid.".to_string()
            })?;

            let loyalty = loyalty_setting.as_ref().ok_or_else(|| {
                "Pengaturan poin reward belum dikonfigurasi.".to_string()
            })?;

            if !loyalty.is_enabled {
                return Err("Program pembayaran dengan poin reward sedang dinonaktifkan oleh administrator.".to_string());
            }

            if loyalty.point_value_rupiah <= Decimal::ZERO {
                return Err("Nilai konversi poin ke rupiah tidak valid (harus lebih dari 0).".to_string());
            }

            // Poin yang ditukarkan diprioritaskan dari p.points
            let points_to_redeem = if let Some(pts) = p.points {
                pts
            } else if p.amount > Decimal::ZERO {
                (p.amount / loyalty.point_value_rupiah).ceil().to_i32().unwrap_or(0)
            } else {
                0
            };

            if points_to_redeem <= 0 {
                return Err("Jumlah poin yang ditukarkan harus lebih dari 0 poin.".to_string());
            }

            if points_to_redeem < loyalty.min_redeem_points {
                return Err(format!(
                    "Minimal penukaran poin dalam satu transaksi adalah {} poin.",
                    loyalty.min_redeem_points
                ));
            }

            // Backend mengonversi jumlah poin resmi ke nominal rupiah
            let point_amount = (Decimal::from(points_to_redeem) * loyalty.point_value_rupiah).round_dp(2);

            // Validasi batas persentase maksimal penggunaan poin dari total belanja
            let max_allowed = (final_total * loyalty.max_point_usage_percent / Decimal::from(100)).round_dp(2);
            if point_amount > max_allowed {
                return Err(format!(
                    "Penggunaan poin (Rp {}) melebihi batas maksimal {}% dari total belanja (Maksimal: Rp {}).",
                    point_amount, loyalty.max_point_usage_percent, max_allowed
                ));
            }

            // Kunci baris pelanggan & cek kecukupan saldo poin saat ini
            let current_pts = sqlx::query_scalar::<_, i32>(
                "SELECT reward_points FROM customers WHERE id = $1 FOR UPDATE",
            )
            .bind(cid)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| format!("Gagal memverifikasi saldo poin member: {}", e))?;

            if current_pts < points_to_redeem {
                return Err(format!(
                    "Poin reward member tidak mencukupi. Tersedia: {} poin, Ditukarkan: {} poin.",
                    current_pts, points_to_redeem
                ));
            }

            resolved_payments.push(ResolvedPayment {
                payment_method_id: p.payment_method_id,
                payment_method_name: pm_name,
                category: pm_cat,
                amount: point_amount,
                points_redeemed: Some(points_to_redeem),
                card_number_masked: p.card_number_masked.clone(),
                reference_number: p.reference_number.clone(),
                payment_proof_url: p.payment_proof_url.clone(),
            });
        } else {
            if p.amount <= Decimal::ZERO {
                return Err(format!(
                    "Nominal pembayaran untuk metode '{}' harus lebih dari 0",
                    pm_name
                ));
            }

            resolved_payments.push(ResolvedPayment {
                payment_method_id: p.payment_method_id,
                payment_method_name: pm_name,
                category: pm_cat,
                amount: p.amount,
                points_redeemed: None,
                card_number_masked: p.card_number_masked.clone(),
                reference_number: p.reference_number.clone(),
                payment_proof_url: p.payment_proof_url.clone(),
            });
        }
    }

    let total_paid: Decimal = resolved_payments.iter().map(|p| p.amount).sum();
    if total_paid < final_total {
        return Err(format!(
            "Pembayaran kurang! Total belanja: Rp {}, Pembayaran: Rp {}",
            final_total, total_paid
        ));
    }
    let change_amount = total_paid - final_total;

    // Total penghematan belanja: flash sale + diskon member + diskon voucher/item
    let total_savings_amount = total_flash_sale_discount + total_member_discount + total_discount;

    let sale = sqlx::query_as::<_, Sale>(
        "INSERT INTO sales (
            id, invoice_number, outlet_id, shift_id, customer_id, warehouse_id, voucher_id,
            subtotal, discount_amount, tax_amount, total_amount, paid_amount, change_amount,
            total_cost_amount, status, notes, total_flash_sale_discount, total_member_discount,
            total_savings_amount, created_at, updated_at, created_by, updated_by
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, 'COMPLETED', $15,
            $16, $17, $18, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $19, $19
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
    .bind(total_flash_sale_discount)
    .bind(total_member_discount)
    .bind(total_savings_amount)
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| format!("Gagal menyimpan data penjualan: {}", e))?;

    // Jika transaksi menggunakan voucher, perbarui used_count voucher secara atomik
    if let Some(v_id) = voucher_id {
        sqlx::query(
            "UPDATE vouchers 
             SET used_count = used_count + 1,
                 status = CASE WHEN quota > 0 AND (used_count + 1) >= quota THEN 'USED' ELSE status END,
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = $1",
        )
        .bind(v_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal memperbarui kuota voucher: {}", e))?;
    }

    for pi in &prepared_items {
        let sale_item_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO sale_items (
                id, sale_id, product_id, unit_id, quantity, unit_cost_price, unit_selling_price,
                discount_percent, discount_amount, tax_amount, subtotal, is_voided,
                flash_sale_id, original_price, flash_sale_discount, member_discount,
                created_at, updated_at, created_by, updated_by
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, false,
                $12, $13, $14, $15, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $16, $16
            )",
        )
        .bind(sale_item_id)
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
        .bind(pi.flash_sale_id)
        .bind(pi.original_price)
        .bind(pi.flash_sale_discount)
        .bind(pi.member_discount)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal menyimpan baris penjualan: {}", e))?;

        // Simpan pemetaan batch stok (FEFO/FIFO) ke tabel sale_item_stock_batches
        for alloc in &pi.batch_allocations {
            sqlx::query(
                "INSERT INTO sale_item_stock_batches (
                    id, sale_id, sale_item_id, product_id, outlet_id,
                    batch_id, batch_number, quantity, cost_price, subtotal_cost,
                    source_reference_type, source_reference_id, source_document_number,
                    expiry_date, allocation_method, created_at
                ) VALUES (
                    $1, $2, $3, $4, $5,
                    $6, $7, $8, $9, $10,
                    $11, $12, $13,
                    $14, $15, CURRENT_TIMESTAMP
                )",
            )
            .bind(Uuid::new_v4())
            .bind(sale.id)
            .bind(sale_item_id)
            .bind(pi.product_id)
            .bind(outlet_id)
            .bind(alloc.batch_id)
            .bind(&alloc.batch_number)
            .bind(alloc.quantity)
            .bind(alloc.cost_price)
            .bind(alloc.subtotal_cost)
            .bind(&alloc.source_reference_type)
            .bind(alloc.source_reference_id)
            .bind(&alloc.source_document_number)
            .bind(alloc.expiry_date)
            .bind(&alloc.allocation_method)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Gagal mencatat pemetaan batch stok penjualan: {}", e))?;
        }
    }

    let mut total_cash_paid = Decimal::ZERO;
    let mut total_non_cash_paid = Decimal::ZERO;
    let mut total_points_paid = Decimal::ZERO;

    for p in &resolved_payments {
        sqlx::query(
            "INSERT INTO sale_payments (
                id, sale_id, payment_method_id, amount, card_number_masked, reference_number, payment_proof_url,
                created_at, updated_at, created_by, updated_by
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $8, $8
            )",
        )
        .bind(Uuid::new_v4())
        .bind(sale.id)
        .bind(p.payment_method_id)
        .bind(p.amount)
        .bind(&p.card_number_masked)
        .bind(&p.reference_number)
        .bind(&p.payment_proof_url)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal menyimpan pembayaran: {}", e))?;

        if p.category == "POINTS" {
            let cid = req.customer_id.unwrap();
            let points_needed = p.points_redeemed.unwrap();

            let current_pts = sqlx::query_scalar::<_, i32>(
                "SELECT reward_points FROM customers WHERE id = $1 FOR UPDATE",
            )
            .bind(cid)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| format!("Gagal memverifikasi saldo poin member: {}", e))?;

            let new_balance = current_pts - points_needed;
            sqlx::query("UPDATE customers SET reward_points = $1 WHERE id = $2")
                .bind(new_balance)
                .bind(cid)
                .execute(&mut *tx)
                .await
                .map_err(|e| format!("Gagal memperbarui saldo poin member: {}", e))?;

            sqlx::query(
                "INSERT INTO customer_point_ledgers (
                    customer_id, transaction_type, reference_id, points_in, points_out, balance, notes, created_at, created_by
                ) VALUES ($1, 'REDEEMED', $2, 0, $3, $4, $5, CURRENT_TIMESTAMP, $6)",
            )
            .bind(cid)
            .bind(sale.id)
            .bind(points_needed)
            .bind(new_balance)
            .bind(format!("Penukaran {} poin untuk pembayaran nota {}", points_needed, invoice_number))
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Gagal mencatat mutasi penukaran poin: {}", e))?;

            total_points_paid += p.amount;
        } else if p.category == "CASH" {
            total_cash_paid += p.amount;
        } else {
            total_non_cash_paid += p.amount;
        }
    }

    let net_cash_in_drawer = (total_cash_paid - change_amount).max(Decimal::ZERO);

    let mut earned_points = 0;
    if let Some(cid) = req.customer_id {
        if let Some(loyalty) = &loyalty_setting {
            if loyalty.is_enabled && loyalty.min_spend_per_point > Decimal::ZERO {
                // Perolehan poin baru dihitung dari sisa belanja riil yang dibayar di luar penukaran poin
                let eligible_spend = (final_total - total_points_paid).max(Decimal::ZERO);
                let pts = (eligible_spend / loyalty.min_spend_per_point).to_i32().unwrap_or(0);
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
        }
    }

    // Double-Entry Auto-Journaling
    let journal_id = Uuid::new_v4();
    let journal_number = format!("JRN-POS-{}-{}", now.format("%Y%m%d%H%M%S"), &journal_id.to_string()[..4].to_uppercase());

    let manual_item_discount: Decimal = prepared_items.iter().map(|pi| pi.disc_amount).sum();
    let gross_sales_revenue = subtotal + total_flash_sale_discount + total_member_discount;

    let total_debits = net_cash_in_drawer
        + total_non_cash_paid
        + total_points_paid
        + total_flash_sale_discount
        + total_member_discount
        + voucher_discount
        + manual_item_discount
        + total_cost;
    let total_credits = gross_sales_revenue + total_tax + total_cost;

    sqlx::query(
        "INSERT INTO journal_entries (
            id, journal_number, journal_date, reference_type, reference_id, description,
            total_debit, total_credit, is_closing_entry, created_at, updated_at, created_by, updated_by
        ) VALUES (
            $1, $2, CURRENT_DATE, 'SALES', $3, $4, $5, $6, FALSE, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $7, $7
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
    let acc_disc_fs: Option<Uuid> = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '4102'").fetch_optional(&mut *tx).await.unwrap_or(None);
    let acc_disc_mem: Option<Uuid> = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '4103'").fetch_optional(&mut *tx).await.unwrap_or(None);
    let acc_disc_vch: Option<Uuid> = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '4104'").fetch_optional(&mut *tx).await.unwrap_or(None);
    let acc_disc_item: Option<Uuid> = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '4105'").fetch_optional(&mut *tx).await.unwrap_or(None);
    let acc_disc_pts: Option<Uuid> = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '4106'").fetch_optional(&mut *tx).await.unwrap_or(None);
    let acc_cogs: Uuid = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '5101'").fetch_one(&mut *tx).await.map_err(|_| "Akun 5101 HPP tidak ditemukan")?;

    let line_outlet_id = outlet_id;

    if net_cash_in_drawer > Decimal::ZERO {
        sqlx::query(
            "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, $3, 0, 'Penerimaan Kas Kasir', $4)",
        )
        .bind(journal_id)
        .bind(acc_kas)
        .bind(net_cash_in_drawer)
        .bind(line_outlet_id)
        .execute(&mut *tx).await.map_err(|e| format!("Gagal membuat baris jurnal: {}", e))?;
    }

    if total_non_cash_paid > Decimal::ZERO {
        sqlx::query(
            "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, $3, 0, 'Penerimaan Non-Tunai / QRIS / EDC', $4)",
        )
        .bind(journal_id)
        .bind(acc_bank)
        .bind(total_non_cash_paid)
        .bind(line_outlet_id)
        .execute(&mut *tx).await.map_err(|e| format!("Gagal membuat baris jurnal: {}", e))?;
    }

    if total_points_paid > Decimal::ZERO {
        let acc_pts_target = acc_disc_pts.or(acc_disc_mem).unwrap_or(acc_sales);
        sqlx::query(
            "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, $3, 0, 'Potongan Penjualan Penukaran Poin Reward', $4)",
        )
        .bind(journal_id)
        .bind(acc_pts_target)
        .bind(total_points_paid)
        .bind(line_outlet_id)
        .execute(&mut *tx).await.map_err(|e| format!("Gagal membuat baris jurnal potongan poin: {}", e))?;
    }

    if total_flash_sale_discount > Decimal::ZERO {
        if let Some(acc_id) = acc_disc_fs {
            sqlx::query(
                "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, $3, 0, 'Potongan Penjualan Flash Sale', $4)",
            )
            .bind(journal_id)
            .bind(acc_id)
            .bind(total_flash_sale_discount)
            .bind(line_outlet_id)
            .execute(&mut *tx).await.map_err(|e| format!("Gagal membuat baris jurnal: {}", e))?;
        }
    }

    if total_member_discount > Decimal::ZERO {
        if let Some(acc_id) = acc_disc_mem {
            sqlx::query(
                "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, $3, 0, 'Potongan Penjualan Diskon Member', $4)",
            )
            .bind(journal_id)
            .bind(acc_id)
            .bind(total_member_discount)
            .bind(line_outlet_id)
            .execute(&mut *tx).await.map_err(|e| format!("Gagal membuat baris jurnal: {}", e))?;
        }
    }

    if voucher_discount > Decimal::ZERO {
        if let Some(acc_id) = acc_disc_vch {
            let memo = if let Some(vc) = &req.voucher_code {
                format!("Potongan Penjualan Voucher ({})", vc)
            } else {
                "Potongan Penjualan Voucher".to_string()
            };
            sqlx::query(
                "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, $3, 0, $4, $5)",
            )
            .bind(journal_id)
            .bind(acc_id)
            .bind(voucher_discount)
            .bind(memo)
            .bind(line_outlet_id)
            .execute(&mut *tx).await.map_err(|e| format!("Gagal membuat baris jurnal: {}", e))?;
        }
    }

    if manual_item_discount > Decimal::ZERO {
        if let Some(acc_id) = acc_disc_item {
            sqlx::query(
                "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, $3, 0, 'Potongan Penjualan Diskon Kasir / Item', $4)",
            )
            .bind(journal_id)
            .bind(acc_id)
            .bind(manual_item_discount)
            .bind(line_outlet_id)
            .execute(&mut *tx).await.map_err(|e| format!("Gagal membuat baris jurnal: {}", e))?;
        }
    }

    if total_cost > Decimal::ZERO {
        sqlx::query(
            "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, $3, 0, 'Beban Pokok Penjualan (HPP)', $4)",
        )
        .bind(journal_id)
        .bind(acc_cogs)
        .bind(total_cost)
        .bind(line_outlet_id)
        .execute(&mut *tx).await.map_err(|e| format!("Gagal membuat baris jurnal: {}", e))?;

        sqlx::query(
            "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, 0, $3, 'Pengurangan Persediaan Barang Dagang', $4)",
        )
        .bind(journal_id)
        .bind(acc_inv)
        .bind(total_cost)
        .bind(line_outlet_id)
        .execute(&mut *tx).await.map_err(|e| format!("Gagal membuat baris jurnal: {}", e))?;
    }

    if gross_sales_revenue > Decimal::ZERO {
        sqlx::query(
            "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, 0, $3, 'Pendapatan Penjualan Retail POS', $4)",
        )
        .bind(journal_id)
        .bind(acc_sales)
        .bind(gross_sales_revenue)
        .bind(line_outlet_id)
        .execute(&mut *tx).await.map_err(|e| format!("Gagal membuat baris jurnal: {}", e))?;
    }

    if total_tax > Decimal::ZERO {
        sqlx::query(
            "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, 0, $3, 'Hutang PPN Keluaran', $4)",
        )
        .bind(journal_id)
        .bind(acc_ppn)
        .bind(total_tax)
        .bind(line_outlet_id)
        .execute(&mut *tx).await.map_err(|e| format!("Gagal membuat baris jurnal: {}", e))?;
    }

    let mut response_payments = Vec::new();
    for p in &resolved_payments {
        response_payments.push(CheckoutPaymentResponse {
            payment_method_id: p.payment_method_id,
            payment_method_name: p.payment_method_name.clone(),
            category: p.category.clone(),
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
            original_price: Some(pi.original_price),
            flash_sale_discount: pi.flash_sale_discount,
            member_discount: pi.member_discount,
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
        total_savings_amount,
        message: "Transaksi Checkout POS Berhasil".to_string(),
    })
}

pub async fn void_sale(
    pool: &PgPool,
    sale_id: Uuid,
    user_id: Uuid,
    req: VoidSaleRequest,
) -> Result<String, String> {
    let spv_id = verify_supervisor_auth(
        pool,
        req.supervisor_id,
        req.supervisor_pin.as_deref(),
        req.supervisor_auth_token.as_deref(),
        "VOID_SALE",
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

            let cost_per_base = if factor > Decimal::ZERO {
                (it.unit_cost_price / factor).round_dp(4)
            } else {
                it.unit_cost_price
            };

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
            .bind(cost_per_base)
            .bind(format!("Pembatalan / Void Nota {}", sale.invoice_number))
            .bind(user_id)
            .execute(&mut *tx).await.map_err(|e| format!("Gagal mencatat mutasi void: {}", e))?;
        }
    }

    // Pulihkan remaining_qty pada masing-masing batch fisik di product_stock_batches
    let allocated_batches = sqlx::query(
        "SELECT batch_id, quantity FROM sale_item_stock_batches WHERE sale_id = $1 AND batch_id IS NOT NULL"
    )
    .bind(sale.id)
    .fetch_all(&mut *tx)
    .await
    .unwrap_or_default();

    for ab in allocated_batches {
        let b_id: Uuid = ab.get("batch_id");
        let b_qty: Decimal = ab.get("quantity");
        let _ = sqlx::query("UPDATE product_stock_batches SET remaining_qty = remaining_qty + $1 WHERE id = $2")
            .bind(b_qty)
            .bind(b_id)
            .execute(&mut *tx)
            .await;
    }

    if let Some(cid) = sale.customer_id {
        // Kembalikan poin yang telah ditukarkan (REDEEMED) saat void
        let redeemed_pts = sqlx::query_scalar::<_, i32>(
            "SELECT points_out FROM customer_point_ledgers WHERE reference_id = $1 AND transaction_type = 'REDEEMED'",
        )
        .bind(sale.id)
        .fetch_optional(&mut *tx)
        .await
        .unwrap_or(None)
        .unwrap_or(0);

        if redeemed_pts > 0 {
            let current_pts = sqlx::query_scalar::<_, i32>(
                "SELECT reward_points FROM customers WHERE id = $1 FOR UPDATE",
            )
            .bind(cid)
            .fetch_one(&mut *tx)
            .await
            .unwrap_or(0);

            let new_pts = current_pts + redeemed_pts;
            sqlx::query("UPDATE customers SET reward_points = $1 WHERE id = $2")
                .bind(new_pts)
                .bind(cid)
                .execute(&mut *tx).await.ok();

            sqlx::query(
                "INSERT INTO customer_point_ledgers (
                    customer_id, transaction_type, reference_id, points_in, points_out, balance, notes, created_at, created_by
                ) VALUES ($1, 'CORRECTION', $2, $3, 0, $4, $5, CURRENT_TIMESTAMP, $6)",
            )
            .bind(cid)
            .bind(sale.id)
            .bind(redeemed_pts)
            .bind(new_pts)
            .bind(format!("Pengembalian poin pembatalan nota {}", sale.invoice_number))
            .bind(user_id)
            .execute(&mut *tx).await.ok();
        }

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
    .bind(spv_id)
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
                total_debit, total_credit, is_closing_entry, created_at, updated_at, created_by, updated_by
            ) VALUES ($1, $2, CURRENT_DATE, 'VOID', $3, $4, $5, $6, FALSE, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $7, $7)",
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
            "SELECT account_id, debit, credit, memo, outlet_id FROM journal_lines WHERE journal_id = $1",
        )
        .bind(orig_id)
        .fetch_all(&mut *tx).await.map_err(|e| format!("Gagal memuat baris jurnal lama: {}", e))?;

        for line in orig_lines {
            let acc_id: Uuid = line.get("account_id");
            let deb: Decimal = line.get("debit");
            let cred: Decimal = line.get("credit");
            let memo: Option<String> = line.get("memo");
            let line_outlet_id: Option<Uuid> = line.get("outlet_id");

            sqlx::query(
                "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(reversal_journal_id)
            .bind(acc_id)
            .bind(cred)
            .bind(deb)
            .bind(format!("Pembalik {}", memo.unwrap_or_default()))
            .bind(line_outlet_id.or(Some(sale.outlet_id)))
            .execute(&mut *tx).await.map_err(|e| format!("Gagal membalik baris jurnal: {}", e))?;
        }
    }

    log_audit(
        pool,
        Some(user_id),
        Some(spv_id),
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

#[derive(Debug, Default, serde::Deserialize)]
pub struct SalesQueryFilter {
    pub shift_id: Option<Uuid>,
    pub outlet_id: Option<Uuid>,
    pub status: Option<String>,
    pub search: Option<String>,
    pub sort_by: Option<String>,
    pub sort_dir: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub limit: Option<i64>,
}

pub async fn list_sales(
    pool: &PgPool,
    query: Option<SalesQueryFilter>,
    owner_id: Option<Uuid>,
) -> Result<(Vec<serde_json::Value>, i64), String> {
    let q = query.unwrap_or_default();

    let mut filter_sql = " WHERE 1=1".to_string();

    if let Some(uid) = owner_id {
        filter_sql.push_str(&format!(" AND (s.created_by = '{uid}' OR s.updated_by = '{uid}' OR cs.user_id = '{uid}')"));
    }

    if let Some(sid) = q.shift_id {
        filter_sql.push_str(&format!(" AND s.shift_id = '{}'", sid));
    }
    if let Some(oid) = q.outlet_id {
        filter_sql.push_str(&format!(" AND s.outlet_id = '{}'", oid));
    }
    if let Some(st) = &q.status {
        if !st.trim().is_empty() && st != "ALL" {
            filter_sql.push_str(&format!(" AND s.status = '{}'", st.trim()));
        }
    }
    if let Some(s) = &q.search {
        if !s.trim().is_empty() {
            let clean = s.trim().replace('\'', "''");
            filter_sql.push_str(&format!(
                " AND (s.invoice_number ILIKE '%{}%' OR c.name ILIKE '%{}%' OR u.full_name ILIKE '%{}%' OR s.notes ILIKE '%{}%')",
                clean, clean, clean, clean
            ));
        }
    }

    let count_sql = format!(
        "SELECT COUNT(*)::bigint 
         FROM sales s
         LEFT JOIN customers c ON s.customer_id = c.id
         LEFT JOIN cashier_shifts cs ON s.shift_id = cs.id
         LEFT JOIN users u ON cs.user_id = u.id
         {}",
        filter_sql
    );
    let total_items: i64 = sqlx::query_scalar(&count_sql)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let sort_col = match q.sort_by.as_deref() {
        Some("invoice_number") => "s.invoice_number",
        Some("customer_name") => "c.name",
        Some("cashier_name") => "u.full_name",
        Some("total_amount") => "s.total_amount",
        Some("status") => "s.status",
        _ => "s.created_at",
    };
    let sort_dir = match q.sort_dir.as_deref() {
        Some("asc") | Some("ASC") => "ASC",
        _ => "DESC",
    };

    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(q.limit.unwrap_or(10)).max(1);
    let offset = (page - 1) * page_size;

    let sql = format!(
        "SELECT s.*, c.name as customer_name, c.phone as customer_phone,
                cs.terminal_id, u.full_name as cashier_name,
                o.name as outlet_name, o.code as outlet_code
         FROM sales s
         LEFT JOIN outlets o ON s.outlet_id = o.id
         LEFT JOIN customers c ON s.customer_id = c.id
         LEFT JOIN cashier_shifts cs ON s.shift_id = cs.id
         LEFT JOIN users u ON cs.user_id = u.id
         {}
         ORDER BY {} {}
         LIMIT {} OFFSET {}",
        filter_sql, sort_col, sort_dir, page_size, offset
    );

    let rows = sqlx::query(&sql)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

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

    Ok((sales, total_items))
}

pub async fn get_sale_detail(
    pool: &PgPool,
    sale_id: Uuid,
    owner_id: Option<Uuid>,
) -> Result<serde_json::Value, String> {
    let sale_row = sqlx::query(
        "SELECT s.*, c.name as customer_name, c.phone as customer_phone,
                cs.terminal_id, cs.user_id as cashier_user_id, u.full_name as cashier_name,
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

    if let Some(uid) = owner_id {
        let created_by = sale_row.try_get::<Option<Uuid>, _>("created_by").unwrap_or(None);
        let updated_by = sale_row.try_get::<Option<Uuid>, _>("updated_by").unwrap_or(None);
        let cashier_uid = sale_row.try_get::<Option<Uuid>, _>("cashier_user_id").unwrap_or(None);
        let is_owner = created_by == Some(uid) || updated_by == Some(uid) || cashier_uid == Some(uid);
        if !is_owner {
            return Err("Akses ditolak: Anda tidak memiliki hak akses untuk melihat data transaksi penjualan staf lain.".to_string());
        }
    }

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
            "flash_sale_id": r.try_get::<Option<Uuid>, _>("flash_sale_id").unwrap_or(None),
            "original_price": r.try_get::<Option<Decimal>, _>("original_price").unwrap_or(None),
            "flash_sale_discount": r.try_get::<Decimal, _>("flash_sale_discount").unwrap_or(Decimal::ZERO),
            "member_discount": r.try_get::<Decimal, _>("member_discount").unwrap_or(Decimal::ZERO),
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
            "payment_proof_url": r.get::<Option<String>, _>("payment_proof_url"),
            "created_at": r.get::<chrono::DateTime<chrono::Utc>, _>("created_at").to_rfc3339(),
        })
    }).collect();

    let batch_rows = sqlx::query(
        "SELECT sisb.*, p.name as product_name, p.sku as product_sku
         FROM sale_item_stock_batches sisb
         JOIN products p ON sisb.product_id = p.id
         WHERE sisb.sale_id = $1
         ORDER BY sisb.created_at ASC, sisb.id ASC",
    )
    .bind(sale_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let stock_batches: Vec<serde_json::Value> = batch_rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.get::<Uuid, _>("id"),
                "sale_id": r.get::<Uuid, _>("sale_id"),
                "sale_item_id": r.get::<Uuid, _>("sale_item_id"),
                "product_id": r.get::<Uuid, _>("product_id"),
                "product_name": r.get::<String, _>("product_name"),
                "product_sku": r.get::<String, _>("product_sku"),
                "batch_id": r.try_get::<Option<Uuid>, _>("batch_id").unwrap_or(None),
                "batch_number": r.get::<String, _>("batch_number"),
                "quantity": r.get::<Decimal, _>("quantity"),
                "cost_price": r.get::<Decimal, _>("cost_price"),
                "subtotal_cost": r.get::<Decimal, _>("subtotal_cost"),
                "source_reference_type": r.get::<String, _>("source_reference_type"),
                "source_reference_id": r.try_get::<Option<Uuid>, _>("source_reference_id").unwrap_or(None),
                "source_document_number": r.try_get::<Option<String>, _>("source_document_number").unwrap_or(None),
                "expiry_date": r.try_get::<Option<chrono::NaiveDate>, _>("expiry_date").unwrap_or(None),
                "allocation_method": r.get::<String, _>("allocation_method"),
                "created_at": r.get::<chrono::DateTime<chrono::Utc>, _>("created_at").to_rfc3339(),
            })
        })
        .collect();

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
        "total_flash_sale_discount": sale_row.try_get::<Decimal, _>("total_flash_sale_discount").unwrap_or(Decimal::ZERO),
        "total_member_discount": sale_row.try_get::<Decimal, _>("total_member_discount").unwrap_or(Decimal::ZERO),
        "total_savings_amount": sale_row.try_get::<Decimal, _>("total_savings_amount").unwrap_or(Decimal::ZERO),
        "created_at": sale_row.get::<chrono::DateTime<chrono::Utc>, _>("created_at").to_rfc3339(),
        "items": items,
        "payments": payments,
        "stock_batches": stock_batches,
    });

    Ok(sale_data)
}

pub async fn get_sale_stock_batches(
    pool: &PgPool,
    sale_id: Uuid,
) -> Result<Vec<crate::models::sales::SaleItemStockBatchAuditView>, String> {
    sqlx::query_as::<_, crate::models::sales::SaleItemStockBatchAuditView>(
        "SELECT sisb.id, sisb.sale_id, sisb.sale_item_id, sisb.product_id,
                p.name as product_name, p.sku as product_sku,
                sisb.batch_id, sisb.batch_number, sisb.quantity,
                sisb.cost_price, sisb.subtotal_cost,
                sisb.source_reference_type, sisb.source_reference_id,
                sisb.source_document_number, sisb.expiry_date,
                sisb.allocation_method, sisb.created_at
         FROM sale_item_stock_batches sisb
         JOIN products p ON sisb.product_id = p.id
         WHERE sisb.sale_id = $1
         ORDER BY sisb.created_at ASC, sisb.id ASC",
    )
    .bind(sale_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Database error fetching sale stock batches: {}", e))
}

pub async fn get_batch_sales_usage(
    pool: &PgPool,
    batch_id: Uuid,
) -> Result<crate::models::sales::BatchSaleUsageSummary, String> {
    // 1. Ambil informasi detail batch
    let batch = sqlx::query_as::<_, crate::models::sales::BatchHeaderRow>(
        "SELECT 
            psb.id, psb.batch_number, psb.product_id, p.name as product_name,
            psb.outlet_id, o.name as outlet_name,
            psb.initial_qty, psb.remaining_qty, psb.cost_price,
            psb.expiry_date, psb.source_document_number
         FROM product_stock_batches psb
         JOIN products p ON psb.product_id = p.id
         JOIN outlets o ON psb.outlet_id = o.id
         WHERE psb.id = $1"
    )
    .bind(batch_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Database error fetching batch info: {}", e))?
    .ok_or_else(|| format!("Batch dengan ID {} tidak ditemukan", batch_id))?;

    // 2. Ambil transaksi penjualan yang mengonsumsi batch ini
    let sales = sqlx::query_as::<_, crate::models::sales::BatchSaleUsageItem>(
        "SELECT 
            sisb.id,
            sisb.sale_id,
            sisb.sale_item_id,
            s.invoice_number,
            s.status as sale_status,
            s.created_at as sale_created_at,
            sisb.outlet_id,
            COALESCE(o.name, 'Outlet') as outlet_name,
            c.name as customer_name,
            COALESCE(u.full_name, u.username, 'Kasir') as cashier_name,
            sisb.quantity,
            sisb.cost_price,
            sisb.subtotal_cost,
            sisb.allocation_method,
            sisb.created_at
         FROM sale_item_stock_batches sisb
         JOIN sales s ON sisb.sale_id = s.id
         LEFT JOIN outlets o ON sisb.outlet_id = o.id
         LEFT JOIN customers c ON s.customer_id = c.id
         LEFT JOIN users u ON s.created_by = u.id
         WHERE sisb.batch_id = $1
         ORDER BY sisb.created_at DESC, sisb.id DESC"
    )
    .bind(batch_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Database error fetching batch sales usage: {}", e))?;

    let total_consumed_qty: Decimal = sales.iter().map(|s| s.quantity).sum();
    let total_consumed_cost: Decimal = sales.iter().map(|s| s.subtotal_cost).sum();

    Ok(crate::models::sales::BatchSaleUsageSummary {
        batch_id: batch.id,
        batch_number: batch.batch_number,
        product_id: batch.product_id,
        product_name: batch.product_name,
        outlet_id: batch.outlet_id,
        outlet_name: batch.outlet_name,
        initial_qty: batch.initial_qty,
        remaining_qty: batch.remaining_qty,
        cost_price: batch.cost_price,
        expiry_date: batch.expiry_date,
        source_document_number: batch.source_document_number,
        total_consumed_qty,
        total_consumed_cost,
        sales,
    })
}

pub async fn get_sales_stats(
    pool: &PgPool,
    owner_id: Option<Uuid>,
) -> Result<serde_json::Value, String> {
    let owner_clause = if let Some(uid) = owner_id {
        format!(" AND (s.created_by = '{uid}' OR s.updated_by = '{uid}')")
    } else {
        "".to_string()
    };

    let sql = format!(
        "SELECT 
            COALESCE(SUM(s.total_amount), 0) as total_revenue,
            COUNT(*) as total_count,
            COALESCE(SUM(s.subtotal), 0) as total_subtotal,
            COALESCE(SUM(s.discount_amount), 0) as total_discount
         FROM sales s
         WHERE s.status = 'COMPLETED'{}",
        owner_clause
    );
    let row = sqlx::query(&sql)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let total_revenue: Decimal = row.get("total_revenue");
    let total_count: i64 = row.get("total_count");
    let total_discount: Decimal = row.get("total_discount");

    let items_sql = format!(
        "SELECT COALESCE(SUM(si.quantity), 0) 
         FROM sale_items si 
         JOIN sales s ON si.sale_id = s.id 
         WHERE s.status = 'COMPLETED'{}",
        owner_clause
    );
    let items_sold = sqlx::query_scalar::<_, Decimal>(&items_sql)
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
