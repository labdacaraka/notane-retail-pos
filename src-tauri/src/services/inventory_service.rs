use crate::models::inventory::{GoodsReceiptNoteRequest, StockAdjustmentRequest, StockMutation};
use chrono::Utc;
use rust_decimal::Decimal;
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub async fn process_goods_receipt(
    pool: &PgPool,
    user_id: Uuid,
    req: GoodsReceiptNoteRequest,
) -> Result<String, String> {
    let mut tx = pool.begin().await.map_err(|e| format!("Transaction error: {}", e))?;

    let invoice_id = Uuid::new_v4();
    let invoice_number = format!("GRN-{}", Utc::now().format("%Y%m%d%H%M%S"));

    let mut subtotal = Decimal::ZERO;
    let mut total_discount = Decimal::ZERO;

    struct GrnLine {
        product_id: Uuid,
        received_qty: Decimal,
        unit_price: Decimal,
        discount: Decimal,
        line_total: Decimal,
    }

    let mut grn_lines = Vec::new();

    for it in &req.items {
        let disc = it.discount_amount.unwrap_or(Decimal::ZERO);
        let line_tot = (it.unit_price * it.received_qty) - disc;
        subtotal += it.unit_price * it.received_qty;
        total_discount += disc;

        grn_lines.push(GrnLine {
            product_id: it.product_id,
            received_qty: it.received_qty,
            unit_price: it.unit_price,
            discount: disc,
            line_total: line_tot,
        });
    }

    let total_amount = subtotal - total_discount;

    sqlx::query(
        "INSERT INTO purchase_invoices (
            id, invoice_number, supplier_invoice_number, po_id, supplier_id, warehouse_id,
            invoice_date, due_date, subtotal, discount_amount, total_amount, payment_status,
            created_at, updated_at, created_by, updated_by
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 'UNPAID',
            CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $12, $12
        )",
    )
    .bind(invoice_id)
    .bind(&invoice_number)
    .bind(&req.supplier_invoice_number)
    .bind(req.po_id)
    .bind(req.supplier_id)
    .bind(req.warehouse_id)
    .bind(req.invoice_date)
    .bind(req.due_date)
    .bind(subtotal)
    .bind(total_discount)
    .bind(total_amount)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal mencatat faktur pembelian: {}", e))?;

    for line in grn_lines {
        sqlx::query(
            "INSERT INTO purchase_invoice_items (
                id, invoice_id, product_id, received_qty, unit_price, discount_amount, subtotal,
                created_at, updated_at, created_by, updated_by
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7,
                CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $8, $8
            )",
        )
        .bind(Uuid::new_v4())
        .bind(invoice_id)
        .bind(line.product_id)
        .bind(line.received_qty)
        .bind(line.unit_price)
        .bind(line.discount)
        .bind(line.line_total)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal mencatat rincian barang masuk: {}", e))?;

        let cost_row = sqlx::query(
            "SELECT COALESCE(ws.quantity, 0) as current_qty, p.cost_price 
             FROM products p 
             LEFT JOIN warehouse_stocks ws ON p.id = ws.product_id AND ws.warehouse_id = $1
             WHERE p.id = $2",
        )
        .bind(req.warehouse_id)
        .bind(line.product_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

        let current_stock: Decimal = cost_row.get("current_qty");
        let current_cost: Decimal = cost_row.get("cost_price");

        let new_stock = current_stock + line.received_qty;
        let new_cost = if new_stock > Decimal::ZERO {
            ((current_stock * current_cost) + (line.received_qty * line.unit_price)) / new_stock
        } else {
            line.unit_price
        };

        sqlx::query(
            "UPDATE products SET cost_price = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2",
        )
        .bind(new_cost)
        .bind(line.product_id)
        .execute(&mut *tx)
        .await
        .ok();

        sqlx::query(
            "INSERT INTO warehouse_stocks (warehouse_id, product_id, quantity, updated_at)
            VALUES ($1, $2, $3, CURRENT_TIMESTAMP)
            ON CONFLICT (warehouse_id, product_id)
            DO UPDATE SET quantity = $3, updated_at = CURRENT_TIMESTAMP",
        )
        .bind(req.warehouse_id)
        .bind(line.product_id)
        .bind(new_stock)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal menambah stok: {}", e))?;

        sqlx::query(
            "INSERT INTO stock_mutations (
                warehouse_id, product_id, reference_type, reference_id, qty_in, qty_out,
                balance, cost_price, notes, created_at, updated_at, created_by, updated_by
            ) VALUES ($1, $2, 'PURCHASE_GRN', $3, $4, 0, $5, $6, $7, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $8, $8)",
        )
        .bind(req.warehouse_id)
        .bind(line.product_id)
        .bind(invoice_id)
        .bind(line.received_qty)
        .bind(new_stock)
        .bind(line.unit_price)
        .bind(format!("Penerimaan Barang Masuk No {}", invoice_number))
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal mencatat mutasi stok: {}", e))?;
    }

    let journal_id = Uuid::new_v4();
    let jrn_number = format!("JRN-GRN-{}", Utc::now().format("%Y%m%d%H%M%S"));

    sqlx::query(
        "INSERT INTO journal_entries (
            id, journal_number, journal_date, reference_type, reference_id, description,
            total_debit, total_credit, created_at, updated_at, created_by, updated_by
        ) VALUES (
            $1, $2, CURRENT_DATE, 'PURCHASE', $3, $4, $5, $5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $6, $6
        )",
    )
    .bind(journal_id)
    .bind(jrn_number)
    .bind(invoice_id)
    .bind(format!("Penerimaan Faktur Pembelian Supplier {}", req.supplier_invoice_number))
    .bind(total_amount)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal membuat header jurnal: {}", e))?;

    let acc_inv: Uuid = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '1201'").fetch_one(&mut *tx).await.map_err(|_| "Akun 1201 tidak ditemukan")?;
    let acc_ap: Uuid = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '2101'").fetch_one(&mut *tx).await.map_err(|_| "Akun 2101 tidak ditemukan")?;

    sqlx::query(
        "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo) VALUES ($1, $2, $3, 0, 'Penambahan Persediaan')",
    )
    .bind(journal_id)
    .bind(acc_inv)
    .bind(total_amount)
    .execute(&mut *tx).await.map_err(|e| format!("Gagal mencatat baris jurnal: {}", e))?;

    sqlx::query(
        "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo) VALUES ($1, $2, 0, $3, 'Hutang Usaha Supplier')",
    )
    .bind(journal_id)
    .bind(acc_ap)
    .bind(total_amount)
    .execute(&mut *tx).await.map_err(|e| format!("Gagal mencatat baris jurnal: {}", e))?;

    tx.commit().await.map_err(|e| format!("Commit error: {}", e))?;

    Ok(format!("Penerimaan barang {} berhasil diproses", invoice_number))
}

pub async fn process_stock_adjustment(
    pool: &PgPool,
    user_id: Uuid,
    req: StockAdjustmentRequest,
) -> Result<String, String> {
    let mut tx = pool.begin().await.map_err(|e| format!("Transaction error: {}", e))?;

    let adj_id = Uuid::new_v4();
    let adj_number = format!("ADJ-{}", Utc::now().format("%Y%m%d%H%M%S"));

    sqlx::query(
        "INSERT INTO stock_adjustments (
            id, adjustment_number, warehouse_id, adjustment_date, reason_id, status, notes,
            created_at, updated_at, created_by, updated_by
        ) VALUES (
            $1, $2, $3, CURRENT_DATE, $4, 'APPROVED', $5,
            CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $6, $6
        )",
    )
    .bind(adj_id)
    .bind(&adj_number)
    .bind(req.warehouse_id)
    .bind(req.reason_id)
    .bind(&req.notes)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal membuat penyesuaian stok: {}", e))?;

    let mut net_cost_discrepancy = Decimal::ZERO;

    for it in &req.items {
        let row = sqlx::query(
            "SELECT COALESCE(ws.quantity, 0) as current_qty, p.cost_price 
             FROM products p 
             LEFT JOIN warehouse_stocks ws ON p.id = ws.product_id AND ws.warehouse_id = $1
             WHERE p.id = $2",
        )
        .bind(req.warehouse_id)
        .bind(it.product_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

        let system_qty: Decimal = row.get("current_qty");
        let cost_price: Decimal = row.get("cost_price");

        let discrepancy_qty = it.actual_qty - system_qty;
        let line_cost_discrepancy = discrepancy_qty * cost_price;
        net_cost_discrepancy += line_cost_discrepancy;

        sqlx::query(
            "INSERT INTO stock_adjustment_items (
                id, adjustment_id, product_id, system_qty, actual_qty, discrepancy_qty,
                cost_price, total_cost_discrepancy, created_at, updated_at, created_by, updated_by
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $9, $9)",
        )
        .bind(Uuid::new_v4())
        .bind(adj_id)
        .bind(it.product_id)
        .bind(system_qty)
        .bind(it.actual_qty)
        .bind(discrepancy_qty)
        .bind(cost_price)
        .bind(line_cost_discrepancy)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal mencatat rincian opname: {}", e))?;

        sqlx::query(
            "INSERT INTO warehouse_stocks (warehouse_id, product_id, quantity, updated_at)
            VALUES ($1, $2, $3, CURRENT_TIMESTAMP)
            ON CONFLICT (warehouse_id, product_id)
            DO UPDATE SET quantity = $3, updated_at = CURRENT_TIMESTAMP",
        )
        .bind(req.warehouse_id)
        .bind(it.product_id)
        .bind(it.actual_qty)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal memperbarui stok aktual: {}", e))?;

        let (qty_in, qty_out) = if discrepancy_qty >= Decimal::ZERO {
            (discrepancy_qty, Decimal::ZERO)
        } else {
            (Decimal::ZERO, discrepancy_qty.abs())
        };

        sqlx::query(
            "INSERT INTO stock_mutations (
                warehouse_id, product_id, reference_type, reference_id, qty_in, qty_out,
                balance, cost_price, notes, created_at, updated_at, created_by, updated_by
            ) VALUES ($1, $2, 'OPNAME', $3, $4, $5, $6, $7, $8, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $9, $9)",
        )
        .bind(req.warehouse_id)
        .bind(it.product_id)
        .bind(adj_id)
        .bind(qty_in)
        .bind(qty_out)
        .bind(it.actual_qty)
        .bind(cost_price)
        .bind(format!("Penyesuaian Opname {}", adj_number))
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal mencatat mutasi stok: {}", e))?;
    }

    if !net_cost_discrepancy.is_zero() {
        let abs_cost = net_cost_discrepancy.abs();
        let journal_id = Uuid::new_v4();
        let jrn_number = format!("JRN-ADJ-{}", Utc::now().format("%Y%m%d%H%M%S"));

        sqlx::query(
            "INSERT INTO journal_entries (
                id, journal_number, journal_date, reference_type, reference_id, description,
                total_debit, total_credit, created_at, updated_at, created_by, updated_by
            ) VALUES ($1, $2, CURRENT_DATE, 'INVENTORY_DISCREPANCY', $3, $4, $5, $5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $6, $6)",
        )
        .bind(journal_id)
        .bind(jrn_number)
        .bind(adj_id)
        .bind(format!("Penyesuaian Selisih Stock Opname {}", adj_number))
        .bind(abs_cost)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal mencatat jurnal opname: {}", e))?;

        let acc_inv: Uuid = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '1201'").fetch_one(&mut *tx).await.map_err(|_| "Akun 1201 tidak ditemukan")?;
        let acc_loss: Uuid = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '5301'").fetch_one(&mut *tx).await.map_err(|_| "Akun 5301 tidak ditemukan")?;

        if net_cost_discrepancy < Decimal::ZERO {
            sqlx::query(
                "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo) VALUES ($1, $2, $3, 0, 'Beban Selisih Persediaan (Kurang Fisik)')",
            )
            .bind(journal_id)
            .bind(acc_loss)
            .bind(abs_cost)
            .execute(&mut *tx).await.map_err(|e| format!("Gagal baris jurnal: {}", e))?;

            sqlx::query(
                "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo) VALUES ($1, $2, 0, $3, 'Pengurangan Persediaan')",
            )
            .bind(journal_id)
            .bind(acc_inv)
            .bind(abs_cost)
            .execute(&mut *tx).await.map_err(|e| format!("Gagal baris jurnal: {}", e))?;
        } else {
            sqlx::query(
                "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo) VALUES ($1, $2, $3, 0, 'Penambahan Persediaan Fisik')",
            )
            .bind(journal_id)
            .bind(acc_inv)
            .bind(abs_cost)
            .execute(&mut *tx).await.map_err(|e| format!("Gagal baris jurnal: {}", e))?;

            sqlx::query(
                "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo) VALUES ($1, $2, 0, $3, 'Penyesuaian Surplus Fisik')",
            )
            .bind(journal_id)
            .bind(acc_loss)
            .bind(abs_cost)
            .execute(&mut *tx).await.map_err(|e| format!("Gagal baris jurnal: {}", e))?;
        }
    }

    tx.commit().await.map_err(|e| format!("Commit error: {}", e))?;

    Ok(format!("Stock Opname {} berhasil disimpan dan dibukukan", adj_number))
}

pub async fn get_stock_ledger(
    pool: &PgPool,
    product_id: Uuid,
    warehouse_id: Option<Uuid>,
) -> Result<Vec<StockMutation>, String> {
    let mutations = if let Some(wid) = warehouse_id {
        sqlx::query_as::<_, StockMutation>(
            "SELECT * FROM stock_mutations WHERE product_id = $1 AND warehouse_id = $2 ORDER BY created_at DESC LIMIT 100",
        )
        .bind(product_id)
        .bind(wid)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?
    } else {
        sqlx::query_as::<_, StockMutation>(
            "SELECT * FROM stock_mutations WHERE product_id = $1 ORDER BY created_at DESC LIMIT 100",
        )
        .bind(product_id)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?
    };

    Ok(mutations)
}

pub async fn list_purchase_invoices(pool: &PgPool) -> Result<Vec<serde_json::Value>, String> {
    let rows = sqlx::query(
        "SELECT pi.id, pi.invoice_number, pi.supplier_invoice_number, pi.po_id, pi.supplier_id,
                pi.warehouse_id, pi.invoice_date, pi.due_date, pi.subtotal, pi.tax_amount,
                pi.discount_amount, pi.total_amount, pi.paid_amount, pi.payment_status,
                pi.created_at, s.name as supplier_name, s.code as supplier_code,
                w.name as warehouse_name
         FROM purchase_invoices pi
         JOIN suppliers s ON pi.supplier_id = s.id
         JOIN warehouses w ON pi.warehouse_id = w.id
         ORDER BY pi.created_at DESC"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?;

    let result = rows.into_iter().map(|r| {
        serde_json::json!({
            "id": r.get::<Uuid, _>("id"),
            "invoice_number": r.get::<String, _>("invoice_number"),
            "supplier_invoice_number": r.get::<String, _>("supplier_invoice_number"),
            "po_id": r.get::<Option<Uuid>, _>("po_id"),
            "supplier_id": r.get::<Uuid, _>("supplier_id"),
            "supplier_name": r.get::<String, _>("supplier_name"),
            "supplier_code": r.get::<String, _>("supplier_code"),
            "warehouse_id": r.get::<Uuid, _>("warehouse_id"),
            "warehouse_name": r.get::<String, _>("warehouse_name"),
            "invoice_date": r.get::<chrono::NaiveDate, _>("invoice_date").to_string(),
            "due_date": r.get::<chrono::NaiveDate, _>("due_date").to_string(),
            "subtotal": r.get::<Decimal, _>("subtotal"),
            "tax_amount": r.get::<Decimal, _>("tax_amount"),
            "discount_amount": r.get::<Decimal, _>("discount_amount"),
            "total_amount": r.get::<Decimal, _>("total_amount"),
            "paid_amount": r.get::<Decimal, _>("paid_amount"),
            "payment_status": r.get::<String, _>("payment_status"),
            "created_at": r.get::<chrono::DateTime<chrono::Utc>, _>("created_at").to_rfc3339(),
        })
    }).collect();

    Ok(result)
}

pub async fn get_purchase_invoice_detail(pool: &PgPool, id: Uuid) -> Result<serde_json::Value, String> {
    let pi_row = sqlx::query(
        "SELECT pi.*, s.name as supplier_name, s.code as supplier_code, s.phone as supplier_phone,
                w.name as warehouse_name
         FROM purchase_invoices pi
         JOIN suppliers s ON pi.supplier_id = s.id
         JOIN warehouses w ON pi.warehouse_id = w.id
         WHERE pi.id = $1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?
    .ok_or_else(|| "Faktur pembelian tidak ditemukan".to_string())?;

    let items_rows = sqlx::query(
        "SELECT pii.*, p.name as product_name, p.sku as product_sku, u.code as unit_code
         FROM purchase_invoice_items pii
         JOIN products p ON pii.product_id = p.id
         JOIN units u ON p.base_unit_id = u.id
         WHERE pii.invoice_id = $1"
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?;

    let items: Vec<serde_json::Value> = items_rows.into_iter().map(|r| {
        serde_json::json!({
            "id": r.get::<Uuid, _>("id"),
            "product_id": r.get::<Uuid, _>("product_id"),
            "product_name": r.get::<String, _>("product_name"),
            "product_sku": r.get::<String, _>("product_sku"),
            "unit_code": r.get::<String, _>("unit_code"),
            "received_qty": r.get::<Decimal, _>("received_qty"),
            "unit_price": r.get::<Decimal, _>("unit_price"),
            "discount_amount": r.get::<Decimal, _>("discount_amount"),
            "subtotal": r.get::<Decimal, _>("subtotal"),
        })
    }).collect();

    let invoice = serde_json::json!({
        "id": pi_row.get::<Uuid, _>("id"),
        "invoice_number": pi_row.get::<String, _>("invoice_number"),
        "supplier_invoice_number": pi_row.get::<String, _>("supplier_invoice_number"),
        "supplier_id": pi_row.get::<Uuid, _>("supplier_id"),
        "supplier_name": pi_row.get::<String, _>("supplier_name"),
        "supplier_code": pi_row.get::<String, _>("supplier_code"),
        "supplier_phone": pi_row.get::<Option<String>, _>("supplier_phone"),
        "warehouse_id": pi_row.get::<Uuid, _>("warehouse_id"),
        "warehouse_name": pi_row.get::<String, _>("warehouse_name"),
        "invoice_date": pi_row.get::<chrono::NaiveDate, _>("invoice_date").to_string(),
        "due_date": pi_row.get::<chrono::NaiveDate, _>("due_date").to_string(),
        "subtotal": pi_row.get::<Decimal, _>("subtotal"),
        "tax_amount": pi_row.get::<Decimal, _>("tax_amount"),
        "discount_amount": pi_row.get::<Decimal, _>("discount_amount"),
        "total_amount": pi_row.get::<Decimal, _>("total_amount"),
        "paid_amount": pi_row.get::<Decimal, _>("paid_amount"),
        "payment_status": pi_row.get::<String, _>("payment_status"),
        "created_at": pi_row.get::<chrono::DateTime<chrono::Utc>, _>("created_at").to_rfc3339(),
        "items": items,
    });

    Ok(invoice)
}

pub async fn pay_purchase_invoice(pool: &PgPool, id: Uuid) -> Result<String, String> {
    let pi = sqlx::query("SELECT total_amount FROM purchase_invoices WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or_else(|| "Faktur pembelian tidak ditemukan".to_string())?;

    let total: Decimal = pi.get("total_amount");

    sqlx::query(
        "UPDATE purchase_invoices 
         SET paid_amount = $1, payment_status = 'PAID', updated_at = CURRENT_TIMESTAMP 
         WHERE id = $2"
    )
    .bind(total)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| format!("Gagal memperbarui status bayar: {}", e))?;

    Ok("Faktur pembelian berhasil ditandai lunas".to_string())
}

pub async fn list_purchase_orders(pool: &PgPool) -> Result<Vec<serde_json::Value>, String> {
    let rows = sqlx::query(
        "SELECT po.id, po.po_number, po.supplier_id, po.warehouse_id, po.order_date,
                po.expected_delivery, po.total_amount, po.status, po.notes, po.created_at,
                s.name as supplier_name, s.code as supplier_code, w.name as warehouse_name
         FROM purchase_orders po
         JOIN suppliers s ON po.supplier_id = s.id
         JOIN warehouses w ON po.warehouse_id = w.id
         ORDER BY po.created_at DESC"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?;

    let result = rows.into_iter().map(|r| {
        serde_json::json!({
            "id": r.get::<Uuid, _>("id"),
            "po_number": r.get::<String, _>("po_number"),
            "supplier_id": r.get::<Uuid, _>("supplier_id"),
            "supplier_name": r.get::<String, _>("supplier_name"),
            "supplier_code": r.get::<String, _>("supplier_code"),
            "warehouse_id": r.get::<Uuid, _>("warehouse_id"),
            "warehouse_name": r.get::<String, _>("warehouse_name"),
            "order_date": r.get::<chrono::NaiveDate, _>("order_date").to_string(),
            "expected_delivery": r.get::<Option<chrono::NaiveDate>, _>("expected_delivery").map(|d| d.to_string()),
            "total_amount": r.get::<Decimal, _>("total_amount"),
            "status": r.get::<String, _>("status"),
            "notes": r.get::<Option<String>, _>("notes"),
            "created_at": r.get::<chrono::DateTime<chrono::Utc>, _>("created_at").to_rfc3339(),
        })
    }).collect();

    Ok(result)
}
