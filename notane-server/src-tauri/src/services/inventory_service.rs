use crate::models::inventory::{
    CancelStockTransferRequest, CreateGoodsReceiptRequest, CreatePurchaseInvoiceFromGrnRequest,
    CreatePurchaseOrderRequest, CreatePurchaseReturnRequest, CreateStockTransferRequest,
    GoodsReceiptNoteRequest, GoodsReceiptQueryFilter, PurchaseOrderQueryFilter, PurchaseReturnQueryFilter,
    ReceiveStockTransferRequest, StockAdjustmentRequest,
    StockMutation, StockTransferQueryFilter,
};
use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde_json::json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub async fn process_goods_receipt(
    pool: &PgPool,
    user_id: Uuid,
    req: GoodsReceiptNoteRequest,
) -> Result<String, String> {
    let mut tx = pool.begin().await.map_err(|e| format!("Transaction error: {}", e))?;

    let invoice_id = Uuid::new_v4();
    let invoice_number = format!("GRN-{}-{}", Utc::now().format("%Y%m%d%H%M%S"), &invoice_id.to_string()[..4].to_uppercase());

    let mut subtotal = Decimal::ZERO;
    let mut total_discount = Decimal::ZERO;

    struct GrnLine {
        product_id: Uuid,
        ordered_qty: Decimal,
        received_qty: Decimal,
        unit_price: Decimal,
        discount: Decimal,
        line_total: Decimal,
        new_barcode: Option<String>,
        new_barcode_desc: Option<String>,
        expiry_date: Option<chrono::NaiveDate>,
        batch_number: Option<String>,
    }

    let mut grn_lines = Vec::new();

    for it in &req.items {
        if it.received_qty <= Decimal::ZERO {
            continue;
        }
        let disc = it.discount_amount.unwrap_or(Decimal::ZERO);
        let ord_qty = it.ordered_qty.unwrap_or(it.received_qty);
        let line_tot = (it.unit_price * it.received_qty) - disc;
        subtotal += it.unit_price * it.received_qty;
        total_discount += disc;

        grn_lines.push(GrnLine {
            product_id: it.product_id,
            ordered_qty: ord_qty,
            received_qty: it.received_qty,
            unit_price: it.unit_price,
            discount: disc,
            line_total: line_tot,
            new_barcode: it.new_barcode.clone(),
            new_barcode_desc: it.new_barcode_desc.clone(),
            expiry_date: it.expiry_date,
            batch_number: it.batch_number.clone(),
        });
    }

    if grn_lines.is_empty() {
        return Err("Tidak ada barang masuk dengan kuantitas yang valid (Qty harus lebih dari 0)".to_string());
    }

    // Validasi proteksi periode tutup buku
    let today = chrono::Local::now().date_naive();
    crate::services::accounting_service::check_transaction_date_allowed(pool, today, req.outlet_id).await?;

    let total_amount = subtotal - total_discount;

    let outlet_id = if let Some(oid) = req.outlet_id {
        Some(oid)
    } else {
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM outlets WHERE is_active = TRUE ORDER BY created_at ASC LIMIT 1")
            .fetch_optional(&mut *tx)
            .await
            .unwrap_or(None)
    };

    // Stok barang masuk langsung dicatat ke Outlet Pemesan
    let target_wh_id = outlet_id;

    let is_paid_upfront = req.payment_status.as_deref().unwrap_or("UNPAID").to_uppercase() == "PAID";
    let (initial_payment_status, initial_paid_amount) = if is_paid_upfront {
        ("PAID", total_amount)
    } else {
        ("UNPAID", Decimal::ZERO)
    };

    sqlx::query(
        "INSERT INTO purchase_invoices (
            id, invoice_number, supplier_invoice_number, po_id, supplier_id, outlet_id, warehouse_id,
            invoice_date, due_date, subtotal, discount_amount, total_amount, paid_amount, payment_status,
            created_at, updated_at, created_by, updated_by
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14,
            CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $15, $15
        )",
    )
    .bind(invoice_id)
    .bind(&invoice_number)
    .bind(&req.supplier_invoice_number)
    .bind(req.po_id)
    .bind(req.supplier_id)
    .bind(outlet_id)
    .bind(target_wh_id)
    .bind(req.invoice_date)
    .bind(req.due_date)
    .bind(subtotal)
    .bind(total_discount)
    .bind(total_amount)
    .bind(initial_paid_amount)
    .bind(initial_payment_status)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal mencatat faktur pembelian: {}", e))?;

    for line in grn_lines {
        sqlx::query(
            "INSERT INTO purchase_invoice_items (
                id, invoice_id, product_id, ordered_qty, received_qty, unit_price, discount_amount, subtotal,
                created_at, updated_at, created_by, updated_by
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8,
                CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $9, $9
            )",
        )
        .bind(Uuid::new_v4())
        .bind(invoice_id)
        .bind(line.product_id)
        .bind(line.ordered_qty)
        .bind(line.received_qty)
        .bind(line.unit_price)
        .bind(line.discount)
        .bind(line.line_total)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal mencatat rincian barang masuk: {}", e))?;

        // Jika merujuk ke PO, update akumulasi received_qty pada item PO tersebut
        if let Some(poid) = req.po_id {
            let _ = sqlx::query(
                "UPDATE purchase_order_items 
                 SET received_qty = received_qty + $1, updated_at = CURRENT_TIMESTAMP 
                 WHERE po_id = $2 AND product_id = $3"
            )
            .bind(line.received_qty)
            .bind(poid)
            .bind(line.product_id)
            .execute(&mut *tx)
            .await;
        }

        // Jika staf memasukkan / scan barcode baru pada barang penerimaan, daftarkan otomatis ke product_barcodes
        if let Some(ref bc) = line.new_barcode {
            let clean_bc = bc.trim();
            if !clean_bc.is_empty() {
                let desc = line.new_barcode_desc.as_deref().unwrap_or("Ditambahkan saat Penerimaan Barang (GRN)");
                let _ = sqlx::query(
                    "INSERT INTO product_barcodes (id, product_id, barcode, description, created_at, updated_at, created_by, updated_by)
                     VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $5, $5)
                     ON CONFLICT (barcode) DO NOTHING"
                )
                .bind(Uuid::new_v4())
                .bind(line.product_id)
                .bind(clean_bc)
                .bind(desc)
                .bind(user_id)
                .execute(&mut *tx)
                .await;
            }
        }

        if let Some(wh_id) = target_wh_id {
            let actual_batch_num = match &line.batch_number {
                Some(bn) if !bn.trim().is_empty() => bn.trim(),
                _ => &format!("GRN-{}", &invoice_id.to_string()[..8]),
            };
            let cost_res = crate::services::costing_service::recalculate_and_apply_cost_on_receipt_ext(
                &mut tx,
                line.product_id,
                wh_id,
                line.received_qty,
                line.unit_price,
                "PURCHASE_GRN",
                Some(invoice_id),
                actual_batch_num,
                Some(&format!("GRN-{}", &invoice_id.to_string()[..8])),
                line.expiry_date,
            )
            .await?;

            let current_stock: Decimal = sqlx::query_scalar(
                "SELECT COALESCE(quantity, 0) FROM warehouse_stocks WHERE warehouse_id = $1 AND product_id = $2"
            )
            .bind(wh_id)
            .bind(line.product_id)
            .fetch_optional(&mut *tx)
            .await
            .unwrap_or(None)
            .unwrap_or(Decimal::ZERO);

            let new_stock = current_stock + line.received_qty;

            sqlx::query(
                "INSERT INTO warehouse_stocks (warehouse_id, product_id, quantity, updated_at)
                VALUES ($1, $2, $3, CURRENT_TIMESTAMP)
                ON CONFLICT (warehouse_id, product_id)
                DO UPDATE SET quantity = $3, updated_at = CURRENT_TIMESTAMP",
            )
            .bind(wh_id)
            .bind(line.product_id)
            .bind(new_stock)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Gagal menambah stok: {}", e))?;

            let mutation_notes = if cost_res.selling_price_adjusted {
                format!("Penerimaan Barang Masuk No {} [Metode: {}, HPP Baru: Rp {}, Harga Jual Otomatis Disesuaikan: Rp {}]", 
                    invoice_number, cost_res.costing_method, cost_res.new_cost_price, cost_res.new_selling_price)
            } else {
                format!("Penerimaan Barang Masuk No {} [Metode: {}, HPP Baru: Rp {}]", 
                    invoice_number, cost_res.costing_method, cost_res.new_cost_price)
            };

            sqlx::query(
                "INSERT INTO stock_mutations (
                    warehouse_id, product_id, reference_type, reference_id, qty_in, qty_out,
                    balance, cost_price, notes, created_at, updated_at, created_by, updated_by
                ) VALUES ($1, $2, 'PURCHASE_GRN', $3, $4, 0, $5, $6, $7, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $8, $8)",
            )
            .bind(wh_id)
            .bind(line.product_id)
            .bind(invoice_id)
            .bind(line.received_qty)
            .bind(new_stock)
            .bind(cost_res.new_cost_price)
            .bind(mutation_notes)
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Gagal mencatat mutasi stok: {}", e))?;
        }
    }

    // Jika ada PO terkait, evaluasi status PO (COMPLETED jika semua item terpenuhi, PARTIAL jika sebagian)
    if let Some(poid) = req.po_id {
        let unfulfilled_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM purchase_order_items WHERE po_id = $1 AND received_qty < ordered_qty"
        )
        .bind(poid)
        .fetch_one(&mut *tx)
        .await
        .unwrap_or(0);

        let any_received_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM purchase_order_items WHERE po_id = $1 AND received_qty > 0"
        )
        .bind(poid)
        .fetch_one(&mut *tx)
        .await
        .unwrap_or(0);

        let new_status = if unfulfilled_count == 0 {
            "COMPLETED"
        } else if any_received_count > 0 {
            "PARTIAL"
        } else {
            "PENDING"
        };

        let _ = sqlx::query("UPDATE purchase_orders SET status = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
            .bind(new_status)
            .bind(poid)
            .execute(&mut *tx)
            .await;
    }

    let journal_id = Uuid::new_v4();
    let jrn_number = format!("JRN-GRN-{}-{}", Utc::now().format("%Y%m%d%H%M%S"), &journal_id.to_string()[..4].to_uppercase());

    let journal_total = subtotal;

    sqlx::query(
        "INSERT INTO journal_entries (
            id, journal_number, journal_date, reference_type, reference_id, description,
            total_debit, total_credit, is_closing_entry, created_at, updated_at, created_by, updated_by
        ) VALUES (
            $1, $2, CURRENT_DATE, 'PURCHASE', $3, $4, $5, $5, FALSE, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $6, $6
        )",
    )
    .bind(journal_id)
    .bind(jrn_number)
    .bind(invoice_id)
    .bind(format!("Penerimaan Faktur Pembelian Supplier {}", req.supplier_invoice_number))
    .bind(journal_total)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal membuat header jurnal: {}", e))?;

    // Ambil akun CoA persediaan khusus outlet penerima jika tersedia, fallback ke 1201
    let outlet_inv_acc: Option<Uuid> = if let Some(wh_id) = target_wh_id {
        sqlx::query_scalar(
            "SELECT o.inventory_account_id
             FROM outlets o
             WHERE o.id = $1"
        )
        .bind(wh_id)
        .fetch_optional(&mut *tx)
        .await
        .unwrap_or(None)
        .flatten()
    } else {
        None
    };

    let acc_inv: Uuid = match outlet_inv_acc {
        Some(aid) => aid,
        None => sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '1201' LIMIT 1")
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| "Akun persediaan 1201 tidak ditemukan")?,
    };

    let target_outlet_name: String = if let Some(wh_id) = target_wh_id {
        sqlx::query_scalar("SELECT name FROM outlets WHERE id = $1")
            .bind(wh_id)
            .fetch_optional(&mut *tx)
            .await
            .unwrap_or(None)
            .unwrap_or_else(|| "Outlet".to_string())
    } else {
        "Outlet".to_string()
    };

    // 1. DEBET Persediaan Barang Dagang sebesar nilai bruto subtotal
    sqlx::query(
        "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, $3, 0, $4, $5)",
    )
    .bind(journal_id)
    .bind(acc_inv)
    .bind(subtotal)
    .bind(format!("Penambahan Persediaan - {}", target_outlet_name))
    .bind(target_wh_id)
    .execute(&mut *tx).await.map_err(|e| format!("Gagal mencatat baris jurnal: {}", e))?;

    // 2. KREDIT Potongan & Diskon Pembelian (5102) jika ada diskon
    if total_discount > Decimal::ZERO {
        let acc_discount: Uuid = match sqlx::query_scalar::<_, Uuid>("SELECT id FROM chart_of_accounts WHERE code = '5102' LIMIT 1")
            .fetch_optional(&mut *tx)
            .await
            .unwrap_or(None) {
                Some(id) => id,
                None => sqlx::query_scalar::<_, Uuid>("SELECT id FROM chart_of_accounts WHERE code = '5101' LIMIT 1")
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(|_| "Akun diskon pembelian tidak ditemukan")?,
            };
        sqlx::query(
            "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, 0, $3, 'Potongan & Diskon Pembelian Supplier', $4)",
        )
        .bind(journal_id)
        .bind(acc_discount)
        .bind(total_discount)
        .bind(target_wh_id)
        .execute(&mut *tx).await.map_err(|e| format!("Gagal mencatat baris jurnal diskon: {}", e))?;
    }

    // 3. KREDIT Kas atau Hutang Usaha sebesar total bersih faktur
    if is_paid_upfront {
        // Pembayaran lunas langsung saat penerimaan: Kredit Kas/Bank
        let acc_kas: Uuid = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '1101' LIMIT 1").fetch_one(&mut *tx).await.map_err(|_| "Akun 1101 tidak ditemukan")?;
        sqlx::query(
            "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, 0, $3, 'Pembayaran Tunai Pembelian', $4)",
        )
        .bind(journal_id)
        .bind(acc_kas)
        .bind(total_amount)
        .bind(target_wh_id)
        .execute(&mut *tx).await.map_err(|e| format!("Gagal mencatat baris jurnal: {}", e))?;
    } else {
        // Pembelian tempo / kredit (terhutang): Kredit Hutang Usaha Supplier
        let acc_ap: Uuid = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '2101' LIMIT 1").fetch_one(&mut *tx).await.map_err(|_| "Akun 2101 tidak ditemukan")?;
        sqlx::query(
            "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, 0, $3, 'Hutang Usaha Supplier (Faktur Tempo)', $4)",
        )
        .bind(journal_id)
        .bind(acc_ap)
        .bind(total_amount)
        .bind(target_wh_id)
        .execute(&mut *tx).await.map_err(|e| format!("Gagal mencatat baris jurnal: {}", e))?;
    }

    tx.commit().await.map_err(|e| format!("Commit error: {}", e))?;

    Ok(format!("Penerimaan barang {} berhasil diproses", invoice_number))
}

pub async fn create_goods_receipt(
    pool: &PgPool,
    user_id: Uuid,
    req: CreateGoodsReceiptRequest,
) -> Result<serde_json::Value, String> {
    if req.delivery_order_number.trim().is_empty() {
        return Err("Nomor Surat Jalan / Pengiriman Pemasok wajib diisi".to_string());
    }
    if req.items.is_empty() {
        return Err("Rincian barang yang diterima tidak boleh kosong".to_string());
    }

    let mut tx = pool.begin().await.map_err(|e| format!("Transaction error: {}", e))?;

    // 1. Ambil data PO rujukan
    let po_row = sqlx::query(
        "SELECT id, po_number, supplier_id, outlet_id, status FROM purchase_orders WHERE id = $1 FOR UPDATE"
    )
    .bind(req.po_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| format!("Database error: {}", e))?
    .ok_or_else(|| "Dokumen Purchase Order (PO) tidak ditemukan".to_string())?;

    let po_status: String = po_row.get("status");
    if po_status == "CANCELLED" {
        return Err("Purchase Order yang sudah dibatalkan tidak dapat menerima barang".to_string());
    }
    if po_status == "COMPLETED" {
        return Err("Purchase Order ini sudah selesai (seluruh barang telah diterima 100%)".to_string());
    }

    let po_number: String = po_row.get("po_number");
    let supplier_id: Uuid = po_row.get("supplier_id");
    let outlet_id: Option<Uuid> = po_row.get("outlet_id");

    // Validasi proteksi periode tutup buku
    crate::services::accounting_service::check_transaction_date_allowed(pool, req.receive_date, outlet_id).await?;

    let grn_id = Uuid::new_v4();
    let grn_number = format!("GRN-{}-{}", Utc::now().format("%Y%m%d%H%M%S"), &grn_id.to_string()[..4].to_uppercase());

    let mut total_qty_received = Decimal::ZERO;
    let mut total_goods_value = Decimal::ZERO;
    let mut validated_items = Vec::new();

    for it in &req.items {
        if it.received_qty <= Decimal::ZERO {
            continue; // Lewati jika 0
        }

        // Ambil data PO item untuk mencocokkan ordered_qty dan unit_price
        let po_item = if let Some(p_item_id) = it.po_item_id {
            sqlx::query(
                "SELECT id, ordered_qty, received_qty, unit_price 
                 FROM purchase_order_items 
                 WHERE id = $1 AND po_id = $2"
            )
            .bind(p_item_id)
            .bind(req.po_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| format!("Database error: {}", e))?
        } else {
            sqlx::query(
                "SELECT id, ordered_qty, received_qty, unit_price 
                 FROM purchase_order_items 
                 WHERE po_id = $1 AND product_id = $2"
            )
            .bind(req.po_id)
            .bind(it.product_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| format!("Database error: {}", e))?
        }
        .ok_or_else(|| "Produk tidak terdaftar dalam dokumen PO ini".to_string())?;

        let po_item_id: Uuid = po_item.get("id");
        let ord_qty: Decimal = po_item.get("ordered_qty");
        let rec_qty_before: Decimal = po_item.get("received_qty");
        let unit_price: Decimal = po_item.get("unit_price");

        let remaining_qty = (ord_qty - rec_qty_before).max(Decimal::ZERO);
        if it.received_qty > remaining_qty {
            return Err(format!(
                "Kuantitas diterima ({}) melebihi sisa pesanan PO ({})",
                it.received_qty, remaining_qty
            ));
        }

        let line_subtotal = it.received_qty * unit_price;
        total_qty_received += it.received_qty;
        total_goods_value += line_subtotal;

        validated_items.push((
            po_item_id,
            it.product_id,
            ord_qty,
            it.received_qty,
            unit_price,
            line_subtotal,
            it.expiry_date,
            it.batch_number.clone(),
            it.new_barcode.clone(),
            it.new_barcode_desc.clone(),
        ));
    }

    if total_qty_received <= Decimal::ZERO {
        return Err("Tidak ada kuantitas barang masuk yang valid (> 0)".to_string());
    }

    // 2. Insert header goods_receipts TERLEBIH DAHULU agar foreign key constraint goods_receipt_items terpenuhi
    sqlx::query(
        "INSERT INTO goods_receipts (
            id, grn_number, po_id, supplier_id, outlet_id, delivery_order_number,
            receive_date, status, notes, total_received_qty, created_at, updated_at, created_by, updated_by
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, 'UNINVOICED', $8, $9, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $10, $10
        )",
    )
    .bind(grn_id)
    .bind(&grn_number)
    .bind(req.po_id)
    .bind(supplier_id)
    .bind(outlet_id)
    .bind(&req.delivery_order_number)
    .bind(req.receive_date)
    .bind(&req.notes)
    .bind(total_qty_received)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal mencatat dokumen penerimaan barang: {}", e))?;

    // 3. Insert items ke goods_receipt_items, update kuantitas PO, dan tambah stok fisik
    for (po_item_id, product_id, ord_qty, received_qty, unit_price, line_subtotal, expiry_date, batch_number, new_barcode, new_barcode_desc) in validated_items {
        // Insert ke goods_receipt_items
        sqlx::query(
            "INSERT INTO goods_receipt_items (
                id, goods_receipt_id, po_item_id, product_id, ordered_qty, received_qty,
                unit_price, invoiced_qty, subtotal, expiry_date, batch_number, created_at, updated_at, created_by, updated_by
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, 0, $8, $9, $10, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $11, $11
            )",
        )
        .bind(Uuid::new_v4())
        .bind(grn_id)
        .bind(po_item_id)
        .bind(product_id)
        .bind(ord_qty)
        .bind(received_qty)
        .bind(unit_price)
        .bind(line_subtotal)
        .bind(expiry_date)
        .bind(batch_number.as_deref())
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal mencatat rincian barang GRN: {}", e))?;

        // Update akumulasi received_qty pada item PO
        sqlx::query(
            "UPDATE purchase_order_items 
             SET received_qty = received_qty + $1, updated_at = CURRENT_TIMESTAMP 
             WHERE id = $2"
        )
        .bind(received_qty)
        .bind(po_item_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal mengupdate kuantitas PO: {}", e))?;

        // Tambah barcode jika ada input barcode baru saat scan/penerimaan
        if let Some(ref bc) = new_barcode {
            let clean_bc = bc.trim();
            if !clean_bc.is_empty() {
                let desc = new_barcode_desc.as_deref().unwrap_or("Ditambahkan saat Penerimaan Barang (GRN)");
                let _ = sqlx::query(
                    "INSERT INTO product_barcodes (id, product_id, barcode, description, created_at, updated_at, created_by, updated_by)
                     VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $5, $5)
                     ON CONFLICT (barcode) DO NOTHING"
                )
                .bind(Uuid::new_v4())
                .bind(product_id)
                .bind(clean_bc)
                .bind(desc)
                .bind(user_id)
                .execute(&mut *tx)
                .await;
            }
        }

        // Update Stok Fisik & Nilai Persediaan per Outlet sesuai metode biaya (WMA/FIFO/Replacement Cost)
        if let Some(wh_id) = outlet_id {
            let actual_batch_num = match &batch_number {
                Some(bn) if !bn.trim().is_empty() => bn.trim(),
                _ => &grn_number,
            };
            let cost_res = crate::services::costing_service::recalculate_and_apply_cost_on_receipt_ext(
                &mut tx,
                product_id,
                wh_id,
                received_qty,
                unit_price,
                "PURCHASE_GRN",
                Some(grn_id),
                actual_batch_num,
                Some(&grn_number),
                expiry_date,
            )
            .await?;

            let current_stock: Decimal = sqlx::query_scalar(
                "SELECT COALESCE(quantity, 0) FROM warehouse_stocks WHERE warehouse_id = $1 AND product_id = $2"
            )
            .bind(wh_id)
            .bind(product_id)
            .fetch_optional(&mut *tx)
            .await
            .unwrap_or(None)
            .unwrap_or(Decimal::ZERO);

            let new_stock = current_stock + received_qty;

            sqlx::query(
                "INSERT INTO warehouse_stocks (warehouse_id, product_id, quantity, updated_at)
                 VALUES ($1, $2, $3, CURRENT_TIMESTAMP)
                 ON CONFLICT (warehouse_id, product_id)
                 DO UPDATE SET quantity = $3, updated_at = CURRENT_TIMESTAMP",
            )
            .bind(wh_id)
            .bind(product_id)
            .bind(new_stock)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Gagal menambah stok fisik: {}", e))?;

            let mutation_notes = if cost_res.selling_price_adjusted {
                format!("Penerimaan Barang Fisik No {} (PO {}) [Metode: {}, HPP: Rp {}, Harga Jual Otomatis: Rp {}]", 
                    grn_number, po_number, cost_res.costing_method, cost_res.new_cost_price, cost_res.new_selling_price)
            } else {
                format!("Penerimaan Barang Fisik No {} (PO {}) [Metode: {}, HPP: Rp {}]", 
                    grn_number, po_number, cost_res.costing_method, cost_res.new_cost_price)
            };

            sqlx::query(
                "INSERT INTO stock_mutations (
                    warehouse_id, product_id, reference_type, reference_id, qty_in, qty_out,
                    balance, cost_price, notes, created_at, updated_at, created_by, updated_by
                ) VALUES ($1, $2, 'PURCHASE_GRN', $3, $4, 0, $5, $6, $7, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $8, $8)",
            )
            .bind(wh_id)
            .bind(product_id)
            .bind(grn_id)
            .bind(received_qty)
            .bind(new_stock)
            .bind(cost_res.new_cost_price)
            .bind(mutation_notes)
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Gagal mencatat mutasi stok: {}", e))?;
        }
    }

    // Evaluasi status PO: COMPLETED atau PARTIAL
    let unfulfilled_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM purchase_order_items WHERE po_id = $1 AND received_qty < ordered_qty"
    )
    .bind(req.po_id)
    .fetch_one(&mut *tx)
    .await
    .unwrap_or(0);

    let new_po_status = if unfulfilled_count == 0 {
        "COMPLETED"
    } else {
        "PARTIAL"
    };

    sqlx::query("UPDATE purchase_orders SET status = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
        .bind(new_po_status)
        .bind(req.po_id)
        .execute(&mut *tx)
        .await
        .ok();

    // Jurnal Akuntansi Goods Receipt (Penerimaan Barang):
    // DEBIT: Persediaan Barang Dagang (1201)
    // KREDIT: Hutang Penerimaan Barang Belum Ditagih / Unbilled GRN (2103)
    let journal_id = Uuid::new_v4();
    let jrn_number = format!("JRN-GRN-{}-{}", Utc::now().format("%Y%m%d%H%M%S"), &journal_id.to_string()[..4].to_uppercase());

    sqlx::query(
        "INSERT INTO journal_entries (
            id, journal_number, journal_date, reference_type, reference_id, description,
            total_debit, total_credit, is_closing_entry, created_at, updated_at, created_by, updated_by
        ) VALUES (
            $1, $2, $3, 'PURCHASE_GRN', $4, $5, $6, $6, FALSE, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $7, $7
        )",
    )
    .bind(journal_id)
    .bind(jrn_number)
    .bind(req.receive_date)
    .bind(grn_id)
    .bind(format!("Penerimaan Barang Fisik SJ {} (PO {})", req.delivery_order_number, po_number))
    .bind(total_goods_value)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal membuat jurnal penerimaan: {}", e))?;

    let acc_inv: Uuid = sqlx::query_scalar::<_, Uuid>("SELECT id FROM chart_of_accounts WHERE code = '1201' LIMIT 1")
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| "Akun persediaan 1201 tidak ditemukan")?;

    let acc_unbilled: Uuid = match sqlx::query_scalar::<_, Uuid>("SELECT id FROM chart_of_accounts WHERE code = '2103' LIMIT 1")
        .fetch_optional(&mut *tx)
        .await
        .unwrap_or(None) {
            Some(id) => id,
            None => sqlx::query_scalar::<_, Uuid>("SELECT id FROM chart_of_accounts WHERE code = '2101' LIMIT 1")
                .fetch_one(&mut *tx)
                .await
                .map_err(|_| "Akun hutang 2103/2101 tidak ditemukan")?,
        };

    sqlx::query(
        "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, $3, 0, 'Penambahan Persediaan Barang Dagang', $4)",
    )
    .bind(journal_id)
    .bind(acc_inv)
    .bind(total_goods_value)
    .bind(outlet_id)
    .execute(&mut *tx).await.map_err(|e| format!("Gagal baris jurnal persediaan: {}", e))?;

    sqlx::query(
        "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, 0, $3, 'Hutang Penerimaan Barang Belum Ditagih (Unbilled GRN)', $4)",
    )
    .bind(journal_id)
    .bind(acc_unbilled)
    .bind(total_goods_value)
    .bind(outlet_id)
    .execute(&mut *tx).await.map_err(|e| format!("Gagal baris jurnal hutang GRN: {}", e))?;

    tx.commit().await.map_err(|e| format!("Commit error: {}", e))?;

    Ok(serde_json::json!({
        "id": grn_id,
        "grn_number": grn_number,
        "po_id": req.po_id,
        "po_number": po_number,
        "po_status": new_po_status,
        "total_received_qty": total_qty_received,
        "total_goods_value": total_goods_value
    }))
}

pub async fn list_goods_receipts(
    pool: &PgPool,
    q: Option<GoodsReceiptQueryFilter>,
) -> Result<(Vec<serde_json::Value>, i64), String> {
    let q = q.unwrap_or_default();
    let mut conditions = vec!["1=1".to_string()];

    if let Some(ref s) = q.search {
        if !s.trim().is_empty() {
            let clean = s.trim().replace('\'', "''");
            conditions.push(format!(
                "(gr.grn_number ILIKE '%{}%' OR gr.delivery_order_number ILIKE '%{}%' OR po.po_number ILIKE '%{}%' OR s.name ILIKE '%{}%')",
                clean, clean, clean, clean
            ));
        }
    }
    if let Some(poid) = q.po_id {
        conditions.push(format!("gr.po_id = '{}'", poid));
    }
    if let Some(sid) = q.supplier_id {
        conditions.push(format!("gr.supplier_id = '{}'", sid));
    }
    if let Some(oid) = q.outlet_id {
        conditions.push(format!("gr.outlet_id = '{}'", oid));
    }
    if let Some(ref st) = q.status {
        if !st.trim().is_empty() && st != "ALL" {
            conditions.push(format!("gr.status = '{}'", st.trim()));
        }
    }
    if let Some(sd) = q.start_date {
        conditions.push(format!("gr.receive_date >= '{}'", sd));
    }
    if let Some(ed) = q.end_date {
        conditions.push(format!("gr.receive_date <= '{}'", ed));
    }

    let filter_sql = format!("WHERE {}", conditions.join(" AND "));

    let count_sql = format!(
        "SELECT COUNT(*)::bigint FROM goods_receipts gr
         JOIN purchase_orders po ON gr.po_id = po.id
         JOIN suppliers s ON gr.supplier_id = s.id {}",
        filter_sql
    );
    let total_items: i64 = sqlx::query_scalar(&count_sql).fetch_one(pool).await.unwrap_or(0);

    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(10).max(1);
    let offset = (page - 1) * page_size;

    let sort_col = match q.sort_by.as_deref() {
        Some("grn_number") => "gr.grn_number",
        Some("po_number") => "po.po_number",
        Some("delivery_order_number") => "gr.delivery_order_number",
        Some("receive_date") => "gr.receive_date",
        Some("status") => "gr.status",
        Some("supplier_name") => "s.name",
        Some("outlet_name") => "COALESCE(o.name, '')",
        Some("total_goods_value") => "total_goods_value",
        _ => "gr.created_at",
    };
    let sort_dir = match q.sort_dir.as_deref() {
        Some("asc") | Some("ASC") => "ASC",
        _ => "DESC",
    };

    let sql = format!(
        "SELECT gr.id, gr.grn_number, gr.po_id, gr.supplier_id, gr.outlet_id,
                gr.delivery_order_number, gr.receive_date, gr.status, gr.notes,
                gr.total_received_qty, gr.created_at,
                po.po_number, po.order_date as po_date,
                s.name as supplier_name, s.code as supplier_code,
                o.name as outlet_name, o.code as outlet_code,
                (SELECT COUNT(*)::bigint FROM goods_receipt_items gri WHERE gri.goods_receipt_id = gr.id) as item_count,
                (SELECT COALESCE(SUM(gri.subtotal), 0) FROM goods_receipt_items gri WHERE gri.goods_receipt_id = gr.id) as total_goods_value
         FROM goods_receipts gr
         JOIN purchase_orders po ON gr.po_id = po.id
         JOIN suppliers s ON gr.supplier_id = s.id
         LEFT JOIN outlets o ON gr.outlet_id = o.id
         {}
         ORDER BY {} {}
         LIMIT {} OFFSET {}",
        filter_sql, sort_col, sort_dir, page_size, offset
    );

    let rows = sqlx::query(&sql).fetch_all(pool).await.map_err(|e| format!("Database error: {}", e))?;

    let result = rows.into_iter().map(|r| {
        serde_json::json!({
            "id": r.get::<Uuid, _>("id"),
            "grn_number": r.get::<String, _>("grn_number"),
            "po_id": r.get::<Uuid, _>("po_id"),
            "po_number": r.get::<String, _>("po_number"),
            "po_date": r.get::<chrono::NaiveDate, _>("po_date").to_string(),
            "supplier_id": r.get::<Uuid, _>("supplier_id"),
            "supplier_name": r.get::<String, _>("supplier_name"),
            "supplier_code": r.get::<String, _>("supplier_code"),
            "outlet_id": r.get::<Option<Uuid>, _>("outlet_id"),
            "outlet_name": r.get::<Option<String>, _>("outlet_name"),
            "outlet_code": r.get::<Option<String>, _>("outlet_code"),
            "delivery_order_number": r.get::<String, _>("delivery_order_number"),
            "receive_date": r.get::<chrono::NaiveDate, _>("receive_date").to_string(),
            "status": r.get::<String, _>("status"),
            "notes": r.get::<Option<String>, _>("notes"),
            "total_received_qty": r.get::<Decimal, _>("total_received_qty"),
            "item_count": r.get::<i64, _>("item_count"),
            "total_goods_value": r.get::<Decimal, _>("total_goods_value"),
            "created_at": r.get::<chrono::DateTime<chrono::Utc>, _>("created_at").to_rfc3339(),
        })
    }).collect();

    Ok((result, total_items))
}

pub async fn get_goods_receipt_detail(pool: &PgPool, id: Uuid) -> Result<serde_json::Value, String> {
    let gr_row = sqlx::query(
        "SELECT gr.*, po.po_number, po.order_date as po_date,
                s.name as supplier_name, s.code as supplier_code, s.phone as supplier_phone,
                o.name as outlet_name, o.code as outlet_code
         FROM goods_receipts gr
         JOIN purchase_orders po ON gr.po_id = po.id
         JOIN suppliers s ON gr.supplier_id = s.id
         LEFT JOIN outlets o ON gr.outlet_id = o.id
         WHERE gr.id = $1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?
    .ok_or_else(|| "Dokumen Penerimaan Barang (GRN) tidak ditemukan".to_string())?;

    let items_rows = sqlx::query(
        "SELECT gri.*, p.name as product_name, p.sku as product_sku, p.barcode as product_barcode, u.code as unit_code
         FROM goods_receipt_items gri
         JOIN products p ON gri.product_id = p.id
         JOIN units u ON p.base_unit_id = u.id
         WHERE gri.goods_receipt_id = $1"
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?;

    let items: Vec<serde_json::Value> = items_rows.into_iter().map(|r| {
        let rec: Decimal = r.get("received_qty");
        let inv: Decimal = r.get("invoiced_qty");
        let uninv = (rec - inv).max(Decimal::ZERO);
        serde_json::json!({
            "id": r.get::<Uuid, _>("id"),
            "po_item_id": r.get::<Option<Uuid>, _>("po_item_id"),
            "product_id": r.get::<Uuid, _>("product_id"),
            "product_name": r.get::<String, _>("product_name"),
            "product_sku": r.get::<String, _>("product_sku"),
            "product_barcode": r.get::<Option<String>, _>("product_barcode"),
            "unit_code": r.get::<String, _>("unit_code"),
            "ordered_qty": r.get::<Decimal, _>("ordered_qty"),
            "received_qty": rec,
            "invoiced_qty": inv,
            "uninvoiced_qty": uninv,
            "unit_price": r.get::<Decimal, _>("unit_price"),
            "subtotal": r.get::<Decimal, _>("subtotal"),
            "expiry_date": r.get::<Option<chrono::NaiveDate>, _>("expiry_date"),
            "batch_number": r.get::<Option<String>, _>("batch_number"),
        })
    }).collect();

    Ok(serde_json::json!({
        "id": gr_row.get::<Uuid, _>("id"),
        "grn_number": gr_row.get::<String, _>("grn_number"),
        "po_id": gr_row.get::<Uuid, _>("po_id"),
        "po_number": gr_row.get::<String, _>("po_number"),
        "po_date": gr_row.get::<chrono::NaiveDate, _>("po_date").to_string(),
        "supplier_id": gr_row.get::<Uuid, _>("supplier_id"),
        "supplier_name": gr_row.get::<String, _>("supplier_name"),
        "supplier_code": gr_row.get::<String, _>("supplier_code"),
        "supplier_phone": gr_row.get::<Option<String>, _>("supplier_phone"),
        "outlet_id": gr_row.get::<Option<Uuid>, _>("outlet_id"),
        "outlet_name": gr_row.get::<Option<String>, _>("outlet_name"),
        "outlet_code": gr_row.get::<Option<String>, _>("outlet_code"),
        "delivery_order_number": gr_row.get::<String, _>("delivery_order_number"),
        "receive_date": gr_row.get::<chrono::NaiveDate, _>("receive_date").to_string(),
        "status": gr_row.get::<String, _>("status"),
        "notes": gr_row.get::<Option<String>, _>("notes"),
        "total_received_qty": gr_row.get::<Decimal, _>("total_received_qty"),
        "created_at": gr_row.get::<chrono::DateTime<chrono::Utc>, _>("created_at").to_rfc3339(),
        "items": items,
    }))
}

pub async fn create_purchase_invoice_from_grn(
    pool: &PgPool,
    user_id: Uuid,
    req: CreatePurchaseInvoiceFromGrnRequest,
) -> Result<serde_json::Value, String> {
    if req.supplier_invoice_number.trim().is_empty() {
        return Err("Nomor Faktur / Invoice Pemasok wajib diisi".to_string());
    }

    let mut tx = pool.begin().await.map_err(|e| format!("Transaction error: {}", e))?;

    // 1. Ambil data GRN
    let grn_row = sqlx::query(
        "SELECT gr.id, gr.grn_number, gr.po_id, gr.supplier_id, gr.outlet_id, gr.delivery_order_number,
                gr.status, po.po_number
         FROM goods_receipts gr
         JOIN purchase_orders po ON gr.po_id = po.id
         WHERE gr.id = $1 FOR UPDATE"
    )
    .bind(req.goods_receipt_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| format!("Database error: {}", e))?
    .ok_or_else(|| "Dokumen Penerimaan Barang (GRN) tidak ditemukan".to_string())?;

    let grn_status: String = grn_row.get("status");
    if grn_status == "INVOICED" {
        return Err("Penerimaan barang ini sudah seluruhnya dibuatkan faktur pembelian".to_string());
    }

    let po_id: Uuid = grn_row.get("po_id");
    let supplier_id: Uuid = grn_row.get("supplier_id");
    let outlet_id: Option<Uuid> = grn_row.get("outlet_id");
    let grn_number: String = grn_row.get("grn_number");
    let do_number: String = grn_row.get("delivery_order_number");

    // Validasi tutup buku
    crate::services::accounting_service::check_transaction_date_allowed(pool, req.invoice_date, outlet_id).await?;

    // Ambil item GRN
    let grn_items = sqlx::query(
        "SELECT id, po_item_id, product_id, ordered_qty, received_qty, unit_price, invoiced_qty, subtotal
         FROM goods_receipt_items
         WHERE goods_receipt_id = $1 AND invoiced_qty < received_qty"
    )
    .bind(req.goods_receipt_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| format!("Database error: {}", e))?;

    if grn_items.is_empty() {
        return Err("Tidak ada sisa barang penerimaan yang dapat difakturkan".to_string());
    }

    let invoice_id = Uuid::new_v4();
    let invoice_number = format!("INV-{}-{}", Utc::now().format("%Y%m%d%H%M%S"), &invoice_id.to_string()[..4].to_uppercase());

    let mut subtotal = Decimal::ZERO;
    struct InvoiceLine {
        grn_item_id: Uuid,
        product_id: Uuid,
        ordered_qty: Decimal,
        invoicing_qty: Decimal,
        unit_price: Decimal,
        line_total: Decimal,
    }

    let mut lines = Vec::new();
    for row in grn_items {
        let grn_item_id: Uuid = row.get("id");
        let product_id: Uuid = row.get("product_id");
        let ord_qty: Decimal = row.get("ordered_qty");
        let rec_qty: Decimal = row.get("received_qty");
        let inv_qty: Decimal = row.get("invoiced_qty");
        let unit_price: Decimal = row.get("unit_price");

        let uninv_qty = (rec_qty - inv_qty).max(Decimal::ZERO);
        if uninv_qty <= Decimal::ZERO {
            continue;
        }

        let line_tot = uninv_qty * unit_price;
        subtotal += line_tot;

        lines.push(InvoiceLine {
            grn_item_id,
            product_id,
            ordered_qty: ord_qty,
            invoicing_qty: uninv_qty,
            unit_price,
            line_total: line_tot,
        });
    }

    let discount = req.discount_amount.unwrap_or(Decimal::ZERO);
    let tax = req.tax_amount.unwrap_or(Decimal::ZERO);
    let total_amount = (subtotal - discount + tax).max(Decimal::ZERO);

    let is_paid_upfront = req.payment_status.as_deref().unwrap_or("UNPAID").to_uppercase() == "PAID";
    let (initial_payment_status, initial_paid_amount) = if is_paid_upfront {
        ("PAID", total_amount)
    } else {
        ("UNPAID", Decimal::ZERO)
    };

    // Insert purchase_invoices
    sqlx::query(
        "INSERT INTO purchase_invoices (
            id, invoice_number, supplier_invoice_number, po_id, goods_receipt_id,
            supplier_id, outlet_id, warehouse_id, invoice_date, due_date, subtotal,
            discount_amount, tax_amount, total_amount, paid_amount, payment_status,
            created_at, updated_at, created_by, updated_by
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $7, $8, $9, $10, $11, $12, $13, $14, $15,
            CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $16, $16
        )",
    )
    .bind(invoice_id)
    .bind(&invoice_number)
    .bind(&req.supplier_invoice_number)
    .bind(po_id)
    .bind(req.goods_receipt_id)
    .bind(supplier_id)
    .bind(outlet_id)
    .bind(req.invoice_date)
    .bind(req.due_date)
    .bind(subtotal)
    .bind(discount)
    .bind(tax)
    .bind(total_amount)
    .bind(initial_paid_amount)
    .bind(initial_payment_status)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal mencatat faktur pembelian: {}", e))?;

    // Insert items & update invoiced_qty di goods_receipt_items
    for line in lines {
        sqlx::query(
            "INSERT INTO purchase_invoice_items (
                id, invoice_id, product_id, ordered_qty, received_qty, unit_price,
                discount_amount, subtotal, goods_receipt_item_id, created_at, updated_at, created_by, updated_by
            ) VALUES (
                $1, $2, $3, $4, $5, $6, 0, $7, $8, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $9, $9
            )",
        )
        .bind(Uuid::new_v4())
        .bind(invoice_id)
        .bind(line.product_id)
        .bind(line.ordered_qty)
        .bind(line.invoicing_qty)
        .bind(line.unit_price)
        .bind(line.line_total)
        .bind(line.grn_item_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal mencatat item faktur: {}", e))?;

        sqlx::query(
            "UPDATE goods_receipt_items 
             SET invoiced_qty = invoiced_qty + $1, updated_at = CURRENT_TIMESTAMP 
             WHERE id = $2"
        )
        .bind(line.invoicing_qty)
        .bind(line.grn_item_id)
        .execute(&mut *tx)
        .await
        .ok();
    }

    // Update status GRN menjadi INVOICED
    sqlx::query("UPDATE goods_receipts SET status = 'INVOICED', updated_at = CURRENT_TIMESTAMP WHERE id = $1")
        .bind(req.goods_receipt_id)
        .execute(&mut *tx)
        .await
        .ok();

    // Jurnal Akuntansi Faktur Pembelian:
    // DEBIT: Hutang Penerimaan Barang Belum Ditagih / Unbilled GRN (2103) sebesar subtotal
    // DEBIT (jika ada): Pajak Masukan / PPN Masukan (1104) sebesar tax
    // KREDIT (jika ada): Potongan & Diskon Pembelian (5102) sebesar discount
    // KREDIT: Hutang Usaha Supplier (2101) jika UNPAID, atau Kas/Bank (1101) jika PAID sebesar total_amount
    let journal_id = Uuid::new_v4();
    let jrn_number = format!("JRN-INV-{}-{}", Utc::now().format("%Y%m%d%H%M%S"), &journal_id.to_string()[..4].to_uppercase());
    let journal_total = subtotal + tax;

    sqlx::query(
        "INSERT INTO journal_entries (
            id, journal_number, journal_date, reference_type, reference_id, description,
            total_debit, total_credit, is_closing_entry, created_at, updated_at, created_by, updated_by
        ) VALUES (
            $1, $2, $3, 'PURCHASE_INVOICE', $4, $5, $6, $6, FALSE, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $7, $7
        )",
    )
    .bind(journal_id)
    .bind(jrn_number)
    .bind(req.invoice_date)
    .bind(invoice_id)
    .bind(format!("Faktur Pembelian No {} (Surat Jalan {})", req.supplier_invoice_number, do_number))
    .bind(journal_total)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal membuat header jurnal faktur: {}", e))?;

    let acc_unbilled: Uuid = match sqlx::query_scalar::<_, Uuid>("SELECT id FROM chart_of_accounts WHERE code = '2103' LIMIT 1")
        .fetch_optional(&mut *tx)
        .await
        .unwrap_or(None) {
            Some(id) => id,
            None => sqlx::query_scalar::<_, Uuid>("SELECT id FROM chart_of_accounts WHERE code = '2101' LIMIT 1")
                .fetch_one(&mut *tx)
                .await
                .map_err(|_| "Akun unbilled/AP tidak ditemukan")?,
        };

    // 1. DEBET Kliring Unbilled GRN sebesar subtotal (nilai barang fisik)
    sqlx::query(
        "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, $3, 0, 'Kliring Hutang Penerimaan Barang Belum Ditagih', $4)",
    )
    .bind(journal_id)
    .bind(acc_unbilled)
    .bind(subtotal)
    .bind(outlet_id)
    .execute(&mut *tx).await.map_err(|e| format!("Gagal baris jurnal kliring: {}", e))?;

    // 2. DEBET Pajak Masukan (PPN Masukan) jika ada nilai pajak
    if tax > Decimal::ZERO {
        let acc_tax: Uuid = match sqlx::query_scalar::<_, Uuid>("SELECT id FROM chart_of_accounts WHERE code = '1104' LIMIT 1")
            .fetch_optional(&mut *tx)
            .await
            .unwrap_or(None) {
                Some(id) => id,
                None => sqlx::query_scalar::<_, Uuid>("SELECT id FROM chart_of_accounts WHERE code = '2102' LIMIT 1")
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(|_| "Akun pajak tidak ditemukan")?,
            };
        sqlx::query(
            "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, $3, 0, 'Pajak Masukan (PPN) Faktur Pembelian', $4)",
        )
        .bind(journal_id)
        .bind(acc_tax)
        .bind(tax)
        .bind(outlet_id)
        .execute(&mut *tx).await.map_err(|e| format!("Gagal baris jurnal PPN masukan: {}", e))?;
    }

    // 3. KREDIT Potongan & Diskon Pembelian (5102) jika ada diskon
    if discount > Decimal::ZERO {
        let acc_discount: Uuid = match sqlx::query_scalar::<_, Uuid>("SELECT id FROM chart_of_accounts WHERE code = '5102' LIMIT 1")
            .fetch_optional(&mut *tx)
            .await
            .unwrap_or(None) {
                Some(id) => id,
                None => sqlx::query_scalar::<_, Uuid>("SELECT id FROM chart_of_accounts WHERE code = '5101' LIMIT 1")
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(|_| "Akun diskon pembelian tidak ditemukan")?,
            };
        sqlx::query(
            "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, 0, $3, 'Potongan & Diskon Faktur Pembelian Supplier', $4)",
        )
        .bind(journal_id)
        .bind(acc_discount)
        .bind(discount)
        .bind(outlet_id)
        .execute(&mut *tx).await.map_err(|e| format!("Gagal baris jurnal diskon pembelian: {}", e))?;
    }

    // 4. KREDIT Hutang Usaha Supplier (atau Kas jika bayar tunai) sebesar total tagihan bersih
    if is_paid_upfront {
        let acc_kas: Uuid = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '1101' LIMIT 1")
            .fetch_one(&mut *tx).await.map_err(|_| "Akun 1101 tidak ditemukan")?;
        sqlx::query(
            "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, 0, $3, 'Pembayaran Lunas Faktur Pembelian', $4)",
        )
        .bind(journal_id)
        .bind(acc_kas)
        .bind(total_amount)
        .bind(outlet_id)
        .execute(&mut *tx).await.map_err(|e| format!("Gagal baris jurnal kas: {}", e))?;
    } else {
        let acc_ap: Uuid = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '2101' LIMIT 1")
            .fetch_one(&mut *tx).await.map_err(|_| "Akun 2101 tidak ditemukan")?;
        sqlx::query(
            "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, 0, $3, 'Hutang Usaha Supplier (Faktur Pembelian Tempo)', $4)",
        )
        .bind(journal_id)
        .bind(acc_ap)
        .bind(total_amount)
        .bind(outlet_id)
        .execute(&mut *tx).await.map_err(|e| format!("Gagal baris jurnal AP: {}", e))?;
    }

    tx.commit().await.map_err(|e| format!("Commit error: {}", e))?;

    Ok(serde_json::json!({
        "id": invoice_id,
        "invoice_number": invoice_number,
        "supplier_invoice_number": req.supplier_invoice_number,
        "grn_id": req.goods_receipt_id,
        "grn_number": grn_number,
        "total_amount": total_amount,
        "payment_status": initial_payment_status
    }))
}

pub async fn process_stock_adjustment(
    pool: &PgPool,
    user_id: Uuid,
    req: StockAdjustmentRequest,
) -> Result<String, String> {
    // 1. Resolusi warehouse_id: jika tidak dikirim dari klien, ambil dari preferensi current_outlet_id user di database
    let target_warehouse_id = match req.warehouse_id {
        Some(wid) => wid,
        None => {
            let user_outlet = sqlx::query_scalar::<_, Option<Uuid>>(
                "SELECT current_outlet_id FROM users WHERE id = $1"
            )
            .bind(user_id)
            .fetch_one(pool)
            .await
            .map_err(|e| format!("Gagal membaca outlet aktif pengguna: {}", e))?;

            match user_outlet {
                Some(oid) => {
                    let default_wh = sqlx::query_scalar::<_, Option<Uuid>>(
                        "SELECT default_warehouse_id FROM outlets WHERE id = $1"
                    )
                    .bind(oid)
                    .fetch_optional(pool)
                    .await
                    .unwrap_or(None)
                    .flatten();

                    default_wh.unwrap_or(oid)
                }
                None => {
                    let fallback_outlet = sqlx::query_scalar::<_, Option<Uuid>>(
                        "SELECT outlet_id FROM user_outlets WHERE user_id = $1 LIMIT 1"
                    )
                    .bind(user_id)
                    .fetch_optional(pool)
                    .await
                    .unwrap_or(None)
                    .flatten();

                    match fallback_outlet {
                        Some(foid) => foid,
                        None => {
                            sqlx::query_scalar::<_, Uuid>(
                                "SELECT id FROM outlets WHERE is_active = TRUE ORDER BY is_warehouse ASC, code ASC LIMIT 1"
                            )
                            .fetch_one(pool)
                            .await
                            .map_err(|_| "Lokasi outlet aktif belum ditentukan untuk pengguna ini. Silakan pilih lokasi outlet terlebih dahulu.".to_string())?
                        }
                    }
                }
            }
        }
    };

    // Validasi proteksi periode tutup buku
    let today = chrono::Local::now().date_naive();
    crate::services::accounting_service::check_transaction_date_allowed(pool, today, Some(target_warehouse_id)).await?;

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
    .bind(target_warehouse_id)
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
        .bind(target_warehouse_id)
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
        .bind(target_warehouse_id)
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
        .bind(target_warehouse_id)
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
                total_debit, total_credit, is_closing_entry, created_at, updated_at, created_by, updated_by
            ) VALUES ($1, $2, CURRENT_DATE, 'INVENTORY_DISCREPANCY', $3, $4, $5, $5, FALSE, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $6, $6)",
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

        let outlet_inv_acc: Option<Uuid> = sqlx::query_scalar(
            "SELECT o.inventory_account_id
             FROM outlets o
             WHERE o.id = $1"
        )
        .bind(target_warehouse_id)
        .fetch_optional(&mut *tx)
        .await
        .unwrap_or(None)
        .flatten();

        let acc_inv: Uuid = match outlet_inv_acc {
            Some(aid) => aid,
            None => sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '1201'")
                .fetch_one(&mut *tx)
                .await
                .map_err(|_| "Akun 1201 tidak ditemukan")?,
        };
        let acc_loss: Uuid = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '5301'").fetch_one(&mut *tx).await.map_err(|_| "Akun 5301 tidak ditemukan")?;

        if net_cost_discrepancy < Decimal::ZERO {
            sqlx::query(
                "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, $3, 0, 'Beban Selisih Persediaan (Kurang Fisik)', $4)",
            )
            .bind(journal_id)
            .bind(acc_loss)
            .bind(abs_cost)
            .bind(Some(target_warehouse_id))
            .execute(&mut *tx).await.map_err(|e| format!("Gagal baris jurnal: {}", e))?;

            sqlx::query(
                "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, 0, $3, 'Pengurangan Persediaan', $4)",
            )
            .bind(journal_id)
            .bind(acc_inv)
            .bind(abs_cost)
            .bind(Some(target_warehouse_id))
            .execute(&mut *tx).await.map_err(|e| format!("Gagal baris jurnal: {}", e))?;
        } else {
            sqlx::query(
                "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, $3, 0, 'Penambahan Persediaan Fisik', $4)",
            )
            .bind(journal_id)
            .bind(acc_inv)
            .bind(abs_cost)
            .bind(Some(target_warehouse_id))
            .execute(&mut *tx).await.map_err(|e| format!("Gagal baris jurnal: {}", e))?;

            sqlx::query(
                "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, 0, $3, 'Penyesuaian Surplus Fisik', $4)",
            )
            .bind(journal_id)
            .bind(acc_loss)
            .bind(abs_cost)
            .bind(Some(target_warehouse_id))
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

#[derive(Debug, Default, serde::Deserialize)]
pub struct PurchaseQueryFilter {
    pub search: Option<String>,
    pub status: Option<String>,
    pub supplier_id: Option<Uuid>,
    pub outlet_id: Option<Uuid>,
    pub sort_by: Option<String>,
    pub sort_dir: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

pub async fn list_purchase_invoices(
    pool: &PgPool,
    query: Option<PurchaseQueryFilter>,
    owner_id: Option<Uuid>,
) -> Result<(Vec<serde_json::Value>, i64), String> {
    let q = query.unwrap_or_default();

    let mut filter_sql = " WHERE 1=1".to_string();

    if let Some(uid) = owner_id {
        filter_sql.push_str(&format!(" AND (pi.created_by = '{uid}' OR pi.updated_by = '{uid}')"));
    }

    if let Some(s) = &q.search {
        if !s.trim().is_empty() {
            let clean = s.trim().replace('\'', "''");
            filter_sql.push_str(&format!(
                " AND (pi.invoice_number ILIKE '%{}%' OR pi.supplier_invoice_number ILIKE '%{}%' OR s.name ILIKE '%{}%')",
                clean, clean, clean
            ));
        }
    }

    if let Some(status) = &q.status {
        if !status.trim().is_empty() && status != "ALL" {
            filter_sql.push_str(&format!(" AND pi.payment_status = '{}'", status.trim()));
        }
    }

    if let Some(sup_id) = q.supplier_id {
        filter_sql.push_str(&format!(" AND pi.supplier_id = '{}'", sup_id));
    }

    if let Some(out_id) = q.outlet_id {
        filter_sql.push_str(&format!(" AND pi.outlet_id = '{}'", out_id));
    }

    let count_sql = format!(
        "SELECT COUNT(*)::bigint FROM purchase_invoices pi JOIN suppliers s ON pi.supplier_id = s.id {}",
        filter_sql
    );
    let total_items: i64 = sqlx::query_scalar(&count_sql)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let sort_col = match q.sort_by.as_deref() {
        Some("invoice_number") => "pi.invoice_number",
        Some("supplier_name") => "s.name",
        Some("invoice_date") => "pi.invoice_date",
        Some("due_date") => "pi.due_date",
        Some("total_amount") => "pi.total_amount",
        Some("paid_amount") => "pi.paid_amount",
        Some("payment_status") => "pi.payment_status",
        _ => "pi.created_at",
    };
    let sort_dir = match q.sort_dir.as_deref() {
        Some("asc") | Some("ASC") => "ASC",
        _ => "DESC",
    };

    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(10).max(1);
    let offset = (page - 1) * page_size;

    let sql = format!(
        "SELECT pi.id, pi.invoice_number, pi.supplier_invoice_number, pi.po_id, pi.goods_receipt_id,
                pi.supplier_id, pi.outlet_id, pi.warehouse_id, pi.invoice_date, pi.due_date, pi.subtotal,
                pi.tax_amount, pi.discount_amount, pi.total_amount, pi.paid_amount, pi.payment_status,
                pi.created_at, s.name as supplier_name, s.code as supplier_code,
                COALESCE(o.name, w.name) as outlet_name, COALESCE(o.code, w.code) as outlet_code,
                COALESCE(w.name, o.name) as warehouse_name,
                gr.grn_number, gr.delivery_order_number,
                COALESCE(po.po_number, po_direct.po_number) as po_number
         FROM purchase_invoices pi
         JOIN suppliers s ON pi.supplier_id = s.id
         LEFT JOIN goods_receipts gr ON pi.goods_receipt_id = gr.id
         LEFT JOIN purchase_orders po ON gr.po_id = po.id
         LEFT JOIN purchase_orders po_direct ON pi.po_id = po_direct.id
         LEFT JOIN outlets o ON pi.outlet_id = o.id
         LEFT JOIN outlets w ON pi.warehouse_id = w.id
         {}
         ORDER BY {} {}
         LIMIT {} OFFSET {}",
        filter_sql, sort_col, sort_dir, page_size, offset
    );

    let rows = sqlx::query(&sql)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let result = rows.into_iter().map(|r| {
        serde_json::json!({
            "id": r.get::<Uuid, _>("id"),
            "invoice_number": r.get::<String, _>("invoice_number"),
            "supplier_invoice_number": r.get::<String, _>("supplier_invoice_number"),
            "po_id": r.get::<Option<Uuid>, _>("po_id"),
            "po_number": r.get::<Option<String>, _>("po_number"),
            "goods_receipt_id": r.get::<Option<Uuid>, _>("goods_receipt_id"),
            "grn_number": r.get::<Option<String>, _>("grn_number"),
            "delivery_order_number": r.get::<Option<String>, _>("delivery_order_number"),
            "supplier_id": r.get::<Uuid, _>("supplier_id"),
            "supplier_name": r.get::<String, _>("supplier_name"),
            "supplier_code": r.get::<String, _>("supplier_code"),
            "outlet_id": r.get::<Option<Uuid>, _>("outlet_id"),
            "outlet_name": r.get::<Option<String>, _>("outlet_name"),
            "outlet_code": r.get::<Option<String>, _>("outlet_code"),
            "warehouse_id": r.get::<Option<Uuid>, _>("warehouse_id"),
            "warehouse_name": r.get::<Option<String>, _>("warehouse_name"),
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

    Ok((result, total_items))
}

pub async fn get_purchase_invoice_detail(pool: &PgPool, id: Uuid, owner_id: Option<Uuid>) -> Result<serde_json::Value, String> {
    let pi_row = sqlx::query(
        "SELECT pi.*, s.name as supplier_name, s.code as supplier_code, s.phone as supplier_phone,
                COALESCE(o.name, w.name) as outlet_name, COALESCE(o.code, w.code) as outlet_code,
                COALESCE(w.name, o.name) as warehouse_name,
                gr.grn_number, gr.delivery_order_number,
                COALESCE(po.po_number, po_direct.po_number) as po_number
         FROM purchase_invoices pi
         JOIN suppliers s ON pi.supplier_id = s.id
         LEFT JOIN goods_receipts gr ON pi.goods_receipt_id = gr.id
         LEFT JOIN purchase_orders po ON gr.po_id = po.id
         LEFT JOIN purchase_orders po_direct ON pi.po_id = po_direct.id
         LEFT JOIN outlets o ON pi.outlet_id = o.id
         LEFT JOIN outlets w ON pi.warehouse_id = w.id
         WHERE pi.id = $1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?
    .ok_or_else(|| "Faktur pembelian tidak ditemukan".to_string())?;

    if let Some(uid) = owner_id {
        let created_by = pi_row.try_get::<Option<Uuid>, _>("created_by").unwrap_or(None);
        let updated_by = pi_row.try_get::<Option<Uuid>, _>("updated_by").unwrap_or(None);
        let is_owner = created_by == Some(uid) || updated_by == Some(uid);
        if !is_owner {
            return Err("Akses ditolak: Anda tidak memiliki hak akses untuk melihat data faktur pembelian staf lain.".to_string());
        }
    }

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
            "ordered_qty": r.get::<Decimal, _>("ordered_qty"),
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
        "po_id": pi_row.get::<Option<Uuid>, _>("po_id"),
        "supplier_id": pi_row.get::<Uuid, _>("supplier_id"),
        "supplier_name": pi_row.get::<String, _>("supplier_name"),
        "supplier_code": pi_row.get::<String, _>("supplier_code"),
        "supplier_phone": pi_row.get::<Option<String>, _>("supplier_phone"),
        "outlet_id": pi_row.get::<Option<Uuid>, _>("outlet_id"),
        "outlet_name": pi_row.get::<Option<String>, _>("outlet_name"),
        "outlet_code": pi_row.get::<Option<String>, _>("outlet_code"),
        "warehouse_id": pi_row.get::<Option<Uuid>, _>("warehouse_id"),
        "warehouse_name": pi_row.get::<Option<String>, _>("warehouse_name"),
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

pub async fn pay_purchase_invoice(
    pool: &PgPool,
    id: Uuid,
    user_id: Option<Uuid>,
) -> Result<String, String> {
    let mut tx = pool.begin().await.map_err(|e| format!("Transaction error: {}", e))?;

    let pi = sqlx::query(
        "SELECT invoice_number, total_amount, paid_amount, payment_status, outlet_id 
         FROM purchase_invoices WHERE id = $1 FOR UPDATE"
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| format!("Database error: {}", e))?
    .ok_or_else(|| "Faktur pembelian tidak ditemukan".to_string())?;

    let status: String = pi.get("payment_status");
    if status == "PAID" {
        return Err("Faktur pembelian sudah lunas".to_string());
    }
    if status == "CANCELLED" {
        return Err("Faktur pembelian yang telah dibatalkan tidak dapat dibayar".to_string());
    }

    let inv_number: String = pi.get("invoice_number");
    let total: Decimal = pi.get("total_amount");
    let inv_outlet_id: Option<Uuid> = pi.get("outlet_id");

    sqlx::query(
        "UPDATE purchase_invoices 
         SET paid_amount = $1, payment_status = 'PAID', updated_at = CURRENT_TIMESTAMP 
         WHERE id = $2"
    )
    .bind(total)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal memperbarui status bayar: {}", e))?;

    // Catat Jurnal Pelunasan Hutang Usaha: DEBIT Hutang Usaha 2101, KREDIT Kas 1101
    let journal_id = Uuid::new_v4();
    let jrn_number = format!("JRN-PAY-{}-{}", Utc::now().format("%Y%m%d%H%M%S"), &journal_id.to_string()[..4].to_uppercase());

    sqlx::query(
        "INSERT INTO journal_entries (
            id, journal_number, journal_date, reference_type, reference_id, description,
            total_debit, total_credit, is_closing_entry, created_at, updated_at, created_by, updated_by
        ) VALUES (
            $1, $2, CURRENT_DATE, 'PAYMENT', $3, $4, $5, $5, FALSE, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $6, $6
        )",
    )
    .bind(journal_id)
    .bind(jrn_number)
    .bind(id)
    .bind(format!("Pelunasan Faktur Pembelian Supplier No {}", inv_number))
    .bind(total)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal membuat jurnal pelunasan: {}", e))?;

    let acc_ap: Uuid = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '2101'").fetch_one(&mut *tx).await.map_err(|_| "Akun 2101 Hutang tidak ditemukan")?;
    let acc_kas: Uuid = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '1101'").fetch_one(&mut *tx).await.map_err(|_| "Akun 1101 Kas tidak ditemukan")?;

    sqlx::query(
        "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, $3, 0, 'Pelunasan Hutang Usaha Supplier', $4)",
    )
    .bind(journal_id)
    .bind(acc_ap)
    .bind(total)
    .bind(inv_outlet_id)
    .execute(&mut *tx).await.map_err(|e| format!("Gagal mencatat baris jurnal: {}", e))?;

    sqlx::query(
        "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, 0, $3, 'Pengeluaran Kas Pelunasan Faktur', $4)",
    )
    .bind(journal_id)
    .bind(acc_kas)
    .bind(total)
    .bind(inv_outlet_id)
    .execute(&mut *tx).await.map_err(|e| format!("Gagal mencatat baris jurnal: {}", e))?;

    tx.commit().await.map_err(|e| format!("Commit error: {}", e))?;

    Ok(format!("Faktur {} berhasil dilunasi", inv_number))
}

pub async fn create_purchase_order(
    pool: &PgPool,
    user_id: Uuid,
    req: CreatePurchaseOrderRequest,
) -> Result<serde_json::Value, String> {
    if req.items.is_empty() {
        return Err("Item pesanan PO tidak boleh kosong".to_string());
    }

    // Validasi proteksi tutup buku untuk tanggal PO
    crate::services::accounting_service::check_transaction_date_allowed(pool, req.order_date, req.outlet_id).await?;

    let mut tx = pool.begin().await.map_err(|e| format!("Transaction error: {}", e))?;

    let po_id = Uuid::new_v4();
    let po_number = format!("PO-{}-{}", Utc::now().format("%Y%m%d%H%M%S"), &po_id.to_string()[..4].to_uppercase());

    let outlet_id = if let Some(oid) = req.outlet_id {
        Some(oid)
    } else {
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM outlets WHERE is_active = TRUE ORDER BY created_at ASC LIMIT 1")
            .fetch_optional(&mut *tx)
            .await
            .unwrap_or(None)
    };

    let mut total_amount = Decimal::ZERO;
    struct PreparedPoItem {
        product_id: Uuid,
        ordered_qty: Decimal,
        unit_price: Decimal,
        subtotal: Decimal,
    }

    let mut prep_items = Vec::new();
    for it in &req.items {
        if it.ordered_qty <= Decimal::ZERO {
            return Err("Kuantitas dipesan harus lebih besar dari 0".to_string());
        }
        let line_tot = it.ordered_qty * it.unit_price;
        total_amount += line_tot;
        prep_items.push(PreparedPoItem {
            product_id: it.product_id,
            ordered_qty: it.ordered_qty,
            unit_price: it.unit_price,
            subtotal: line_tot,
        });
    }

    sqlx::query(
        "INSERT INTO purchase_orders (
            id, po_number, supplier_id, outlet_id, warehouse_id, order_date, expected_delivery,
            total_amount, status, notes, created_at, updated_at, created_by, updated_by
        ) VALUES (
            $1, $2, $3, $4, $4, $5, $6, $7, 'PENDING', $8, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $9, $9
        )",
    )
    .bind(po_id)
    .bind(&po_number)
    .bind(req.supplier_id)
    .bind(outlet_id)
    .bind(req.order_date)
    .bind(req.expected_delivery)
    .bind(total_amount)
    .bind(req.notes)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal mencatat Purchase Order: {}", e))?;

    for item in prep_items {
        sqlx::query(
            "INSERT INTO purchase_order_items (
                id, po_id, product_id, ordered_qty, unit_price, subtotal, received_qty,
                created_at, updated_at, created_by, updated_by
            ) VALUES (
                $1, $2, $3, $4, $5, $6, 0, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $7, $7
            )",
        )
        .bind(Uuid::new_v4())
        .bind(po_id)
        .bind(item.product_id)
        .bind(item.ordered_qty)
        .bind(item.unit_price)
        .bind(item.subtotal)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal mencatat item PO: {}", e))?;
    }

    tx.commit().await.map_err(|e| format!("Commit error: {}", e))?;

    Ok(serde_json::json!({
        "id": po_id,
        "po_number": po_number,
        "total_amount": total_amount,
        "status": "PENDING"
    }))
}

pub async fn get_purchase_order_detail(pool: &PgPool, id: Uuid) -> Result<serde_json::Value, String> {
    let po_row = sqlx::query(
        "SELECT po.*, s.name as supplier_name, s.code as supplier_code, s.phone as supplier_phone,
                o.name as outlet_name, o.code as outlet_code
         FROM purchase_orders po
         JOIN suppliers s ON po.supplier_id = s.id
         LEFT JOIN outlets o ON po.outlet_id = o.id
         WHERE po.id = $1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?
    .ok_or_else(|| "Dokumen PO tidak ditemukan".to_string())?;

    let items_rows = sqlx::query(
        "SELECT poi.*, p.name as product_name, p.sku as product_sku, p.cost_price as current_cost_price, u.code as unit_code
         FROM purchase_order_items poi
         JOIN products p ON poi.product_id = p.id
         JOIN units u ON p.base_unit_id = u.id
         WHERE poi.po_id = $1"
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?;

    let items: Vec<serde_json::Value> = items_rows.into_iter().map(|r| {
        let ord: Decimal = r.get("ordered_qty");
        let rec: Decimal = r.get("received_qty");
        let rem = (ord - rec).max(Decimal::ZERO);
        serde_json::json!({
            "id": r.get::<Uuid, _>("id"),
            "product_id": r.get::<Uuid, _>("product_id"),
            "product_name": r.get::<String, _>("product_name"),
            "product_sku": r.get::<String, _>("product_sku"),
            "unit_code": r.get::<String, _>("unit_code"),
            "ordered_qty": ord,
            "received_qty": rec,
            "remaining_qty": rem,
            "unit_price": r.get::<Decimal, _>("unit_price"),
            "subtotal": r.get::<Decimal, _>("subtotal"),
        })
    }).collect();

    Ok(serde_json::json!({
        "id": po_row.get::<Uuid, _>("id"),
        "po_number": po_row.get::<String, _>("po_number"),
        "supplier_id": po_row.get::<Uuid, _>("supplier_id"),
        "supplier_name": po_row.get::<String, _>("supplier_name"),
        "supplier_code": po_row.get::<String, _>("supplier_code"),
        "supplier_phone": po_row.get::<Option<String>, _>("supplier_phone"),
        "outlet_id": po_row.get::<Option<Uuid>, _>("outlet_id"),
        "outlet_name": po_row.get::<Option<String>, _>("outlet_name"),
        "outlet_code": po_row.get::<Option<String>, _>("outlet_code"),
        "order_date": po_row.get::<chrono::NaiveDate, _>("order_date").to_string(),
        "expected_delivery": po_row.get::<Option<chrono::NaiveDate>, _>("expected_delivery").map(|d| d.to_string()),
        "total_amount": po_row.get::<Decimal, _>("total_amount"),
        "status": po_row.get::<String, _>("status"),
        "notes": po_row.get::<Option<String>, _>("notes"),
        "created_at": po_row.get::<chrono::DateTime<chrono::Utc>, _>("created_at").to_rfc3339(),
        "items": items,
    }))
}

pub async fn cancel_purchase_order(pool: &PgPool, id: Uuid) -> Result<String, String> {
    let po = sqlx::query("SELECT status FROM purchase_orders WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or_else(|| "PO tidak ditemukan".to_string())?;

    let status: String = po.get("status");
    if status == "CANCELLED" {
        return Err("Purchase Order ini sudah dibatalkan sebelumnya".to_string());
    }
    if status == "COMPLETED" || status == "PARTIAL" {
        return Err("PO yang sudah memiliki penerimaan barang tidak dapat dibatalkan".to_string());
    }

    sqlx::query("UPDATE purchase_orders SET status = 'CANCELLED', updated_at = CURRENT_TIMESTAMP WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| format!("Gagal membatalkan PO: {}", e))?;

    Ok("Purchase Order berhasil dibatalkan".to_string())
}

pub async fn list_purchase_orders(
    pool: &PgPool,
    q: Option<PurchaseOrderQueryFilter>,
) -> Result<(Vec<serde_json::Value>, i64), String> {
    let q = q.unwrap_or_default();
    let mut conditions = vec!["1=1".to_string()];

    if let Some(ref s) = q.search {
        if !s.trim().is_empty() {
            let clean = s.trim().replace('\'', "''");
            conditions.push(format!(
                "(po.po_number ILIKE '%{0}%' OR s.name ILIKE '%{0}%' OR s.code ILIKE '%{0}%' OR o.name ILIKE '%{0}%' OR w.name ILIKE '%{0}%')",
                clean
            ));
        }
    }
    if let Some(sid) = q.supplier_id {
        conditions.push(format!("po.supplier_id = '{}'", sid));
    }
    if let Some(oid) = q.outlet_id {
        conditions.push(format!("(po.outlet_id = '{0}' OR po.warehouse_id = '{0}')", oid));
    }
    if let Some(ref st) = q.status {
        if !st.trim().is_empty() && st != "ALL" {
            conditions.push(format!("po.status = '{}'", st.trim()));
        }
    }
    if let Some(sd) = q.start_date {
        conditions.push(format!("po.order_date >= '{}'", sd));
    }
    if let Some(ed) = q.end_date {
        conditions.push(format!("po.order_date <= '{}'", ed));
    }

    let filter_sql = format!("WHERE {}", conditions.join(" AND "));

    let count_sql = format!(
        "SELECT COUNT(*)::bigint FROM purchase_orders po
         JOIN suppliers s ON po.supplier_id = s.id
         LEFT JOIN outlets o ON po.outlet_id = o.id
         LEFT JOIN outlets w ON po.warehouse_id = w.id {}",
        filter_sql
    );
    let total_items: i64 = sqlx::query_scalar(&count_sql).fetch_one(pool).await.unwrap_or(0);

    let sort_col = match q.sort_by.as_deref() {
        Some("po_number") => "po.po_number",
        Some("order_date") => "po.order_date",
        Some("expected_delivery") => "po.expected_delivery",
        Some("total_amount") => "po.total_amount",
        Some("status") => "po.status",
        Some("supplier_name") => "s.name",
        Some("outlet_name") => "COALESCE(o.name, w.name)",
        _ => "po.created_at",
    };
    let sort_dir = match q.sort_dir.as_deref() {
        Some("asc") | Some("ASC") => "ASC",
        _ => "DESC",
    };

    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(10).max(1);

    let mut sql = format!(
        "SELECT po.id, po.po_number, po.supplier_id, po.outlet_id, po.warehouse_id, po.order_date,
                po.expected_delivery, po.total_amount, po.status, po.notes, po.created_at,
                s.name as supplier_name, s.code as supplier_code,
                COALESCE(o.name, w.name) as outlet_name, COALESCE(o.code, w.code) as outlet_code,
                COALESCE(w.name, o.name) as warehouse_name
         FROM purchase_orders po
         JOIN suppliers s ON po.supplier_id = s.id
         LEFT JOIN outlets o ON po.outlet_id = o.id
         LEFT JOIN outlets w ON po.warehouse_id = w.id
         {}
         ORDER BY {} {}",
        filter_sql, sort_col, sort_dir
    );

    if q.page.is_some() || q.page_size.is_some() {
        let offset = (page - 1) * page_size;
        sql.push_str(&format!(" LIMIT {} OFFSET {}", page_size, offset));
    }

    let rows = sqlx::query(&sql)
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
            "outlet_id": r.get::<Option<Uuid>, _>("outlet_id"),
            "outlet_name": r.get::<Option<String>, _>("outlet_name"),
            "outlet_code": r.get::<Option<String>, _>("outlet_code"),
            "warehouse_id": r.get::<Option<Uuid>, _>("warehouse_id"),
            "warehouse_name": r.get::<Option<String>, _>("warehouse_name"),
            "order_date": r.get::<chrono::NaiveDate, _>("order_date").to_string(),
            "expected_delivery": r.get::<Option<chrono::NaiveDate>, _>("expected_delivery").map(|d| d.to_string()),
            "total_amount": r.get::<Decimal, _>("total_amount"),
            "status": r.get::<String, _>("status"),
            "notes": r.get::<Option<String>, _>("notes"),
            "created_at": r.get::<chrono::DateTime<chrono::Utc>, _>("created_at").to_rfc3339(),
        })
    }).collect();

    Ok((result, total_items))
}

pub async fn process_stock_transfer(
    pool: &PgPool,
    user_id: Uuid,
    req: CreateStockTransferRequest,
) -> Result<serde_json::Value, String> {
    if req.from_outlet_id == req.to_outlet_id {
        return Err("Outlet asal dan outlet tujuan tidak boleh sama".to_string());
    }

    if req.items.is_empty() {
        return Err("Daftar barang transfer tidak boleh kosong".to_string());
    }

    // Validasi proteksi tutup buku untuk outlet asal & outlet tujuan
    crate::services::accounting_service::check_transaction_date_allowed(pool, req.transfer_date, Some(req.from_outlet_id)).await?;
    crate::services::accounting_service::check_transaction_date_allowed(pool, req.transfer_date, Some(req.to_outlet_id)).await?;

    let is_immediate = req.is_immediate.unwrap_or(false);
    let mut tx = pool.begin().await.map_err(|e| format!("Database error: {}", e))?;

    // Dapatkan info outlet asal & tujuan
    let from_outlet = sqlx::query("SELECT id, code, name, inventory_account_id FROM outlets WHERE id = $1")
        .bind(req.from_outlet_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or_else(|| "Outlet asal tidak ditemukan".to_string())?;

    let to_outlet = sqlx::query("SELECT id, code, name, inventory_account_id FROM outlets WHERE id = $1")
        .bind(req.to_outlet_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or_else(|| "Outlet tujuan tidak ditemukan".to_string())?;

    let from_name: String = from_outlet.get("name");
    let to_name: String = to_outlet.get("name");

    let from_inv_acc: Uuid = match from_outlet.get::<Option<Uuid>, _>("inventory_account_id") {
        Some(aid) => aid,
        None => sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '1201' LIMIT 1")
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| "Akun persediaan 1201 tidak ditemukan")?,
    };

    let to_inv_acc: Uuid = match to_outlet.get::<Option<Uuid>, _>("inventory_account_id") {
        Some(aid) => aid,
        None => sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '1201' LIMIT 1")
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| "Akun persediaan 1201 tidak ditemukan")?,
    };

    let acc_in_transit: Uuid = match sqlx::query_scalar::<_, Uuid>("SELECT id FROM chart_of_accounts WHERE code = '1202' LIMIT 1")
        .fetch_optional(&mut *tx)
        .await
        .unwrap_or(None)
    {
        Some(aid) => aid,
        None => from_inv_acc,
    };

    let transfer_id = Uuid::new_v4();
    let now_str = Utc::now().format("%Y%m%d%H%M%S").to_string();
    let transfer_number = format!("TRF-{}-{}", now_str, &transfer_id.to_string()[..4].to_uppercase());

    let initial_status = if is_immediate { "COMPLETED" } else { "IN_TRANSIT" };

    // 0. Simpan header stock_transfers
    sqlx::query(
        "INSERT INTO stock_transfers (
            id, transfer_number, from_outlet_id, to_outlet_id, transfer_date, status, notes, total_amount,
            shipped_at, received_date, received_by, received_at, created_by, updated_by
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, 0.0000, CURRENT_TIMESTAMP, $8, $9, $10, $11, $11)"
    )
    .bind(transfer_id)
    .bind(&transfer_number)
    .bind(req.from_outlet_id)
    .bind(req.to_outlet_id)
    .bind(req.transfer_date)
    .bind(initial_status)
    .bind(&req.notes)
    .bind(if is_immediate { Some(req.transfer_date) } else { None })
    .bind(if is_immediate { Some(user_id) } else { None })
    .bind(if is_immediate { Some(Utc::now()) } else { None })
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal menyimpan dokumen transfer: {}", e))?;

    let mut total_transfer_amount = Decimal::ZERO;

    for item in &req.items {
        if item.quantity <= Decimal::ZERO {
            return Err("Kuantitas transfer barang harus lebih dari 0".to_string());
        }

        // Cek ketersediaan stok fisik di lokasi asal
        let current_stock: Decimal = sqlx::query_scalar(
            "SELECT quantity FROM warehouse_stocks WHERE warehouse_id = $1 AND product_id = $2 FOR UPDATE"
        )
        .bind(req.from_outlet_id)
        .bind(item.product_id)
        .fetch_optional(&mut *tx)
        .await
        .unwrap_or(None)
        .unwrap_or(Decimal::ZERO);

        let prod = sqlx::query("SELECT name, cost_price FROM products WHERE id = $1")
            .bind(item.product_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| format!("Database error: {}", e))?
            .ok_or_else(|| format!("Produk tidak ditemukan: {}", item.product_id))?;

        let prod_name: String = prod.get("name");
        let cost_price: Decimal = prod.get("cost_price");

        if current_stock < item.quantity {
            return Err(format!(
                "Stok produk '{}' di lokasi asal ({}) tidak mencukupi. Tersedia: {}, Diminta: {}",
                prod_name, from_name, current_stock, item.quantity
            ));
        }

        let subtotal = item.quantity * cost_price;
        total_transfer_amount += subtotal;

        let out_mutation_note = if is_immediate {
            format!("Transfer Keluar ke {} ({})", to_name, transfer_number)
        } else {
            format!("Transfer Keluar ke {} ({}) - Dalam Perjalanan", to_name, transfer_number)
        };

        let new_from_balance = current_stock - item.quantity;
        sqlx::query(
            "UPDATE warehouse_stocks SET quantity = $1, updated_at = CURRENT_TIMESTAMP WHERE warehouse_id = $2 AND product_id = $3"
        )
        .bind(new_from_balance)
        .bind(req.from_outlet_id)
        .bind(item.product_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal memotong stok lokasi asal: {}", e))?;

        sqlx::query(
            "INSERT INTO stock_mutations (
                warehouse_id, product_id, reference_type, reference_id, qty_in, qty_out, balance, cost_price, notes, created_by
            ) VALUES ($1, $2, 'TRANSFER', $3, 0, $4, $5, $6, $7, $8)"
        )
        .bind(req.from_outlet_id)
        .bind(item.product_id)
        .bind(transfer_id)
        .bind(item.quantity)
        .bind(new_from_balance)
        .bind(cost_price)
        .bind(&out_mutation_note)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal mencatat mutasi stok keluar: {}", e))?;

        // 2. Jika transfer instan (is_immediate), langsung tambahkan stok di outlet tujuan
        if is_immediate {
            let current_to_stock: Decimal = sqlx::query_scalar(
                "SELECT quantity FROM warehouse_stocks WHERE warehouse_id = $1 AND product_id = $2"
            )
            .bind(req.to_outlet_id)
            .bind(item.product_id)
            .fetch_optional(&mut *tx)
            .await
            .unwrap_or(None)
            .unwrap_or(Decimal::ZERO);

            let new_to_balance = current_to_stock + item.quantity;

            sqlx::query(
                "INSERT INTO warehouse_stocks (warehouse_id, product_id, quantity, created_at, updated_at)
                 VALUES ($1, $2, $3, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                 ON CONFLICT (warehouse_id, product_id)
                 DO UPDATE SET quantity = warehouse_stocks.quantity + $3, updated_at = CURRENT_TIMESTAMP"
            )
            .bind(req.to_outlet_id)
            .bind(item.product_id)
            .bind(item.quantity)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Gagal menambah stok outlet tujuan: {}", e))?;

            // Catat mutasi masuk di outlet tujuan
            sqlx::query(
                "INSERT INTO stock_mutations (
                    warehouse_id, product_id, reference_type, reference_id, qty_in, qty_out, balance, cost_price, notes, created_by
                ) VALUES ($1, $2, 'TRANSFER', $3, $4, 0, $5, $6, $7, $8)"
            )
            .bind(req.to_outlet_id)
            .bind(item.product_id)
            .bind(transfer_id)
            .bind(item.quantity)
            .bind(new_to_balance)
            .bind(cost_price)
            .bind(format!("Transfer Masuk dari {} ({})", from_name, transfer_number))
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Gagal mencatat mutasi stok masuk: {}", e))?;
        }

        // Simpan item transfer
        let received_qty = if is_immediate { item.quantity } else { Decimal::ZERO };
        sqlx::query(
            "INSERT INTO stock_transfer_items (transfer_id, product_id, quantity, cost_price, subtotal, received_quantity)
             VALUES ($1, $2, $3, $4, $5, $6)"
        )
        .bind(transfer_id)
        .bind(item.product_id)
        .bind(item.quantity)
        .bind(cost_price)
        .bind(subtotal)
        .bind(received_qty)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal menyimpan baris transfer: {}", e))?;
    }

    // Perbarui total_amount di header stock_transfers
    sqlx::query("UPDATE stock_transfers SET total_amount = $1 WHERE id = $2")
        .bind(total_transfer_amount)
        .bind(transfer_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal memperbarui total transfer: {}", e))?;

    // Bukukan Jurnal Akuntansi Double-Entry Berpasangan (Per-Outlet via Rekening Antar-Kantor / RAK)
    let mut journal_entry_id: Option<Uuid> = None;
    if total_transfer_amount > Decimal::ZERO {
        let from_rak_acc: Uuid = match sqlx::query_scalar::<_, Option<Uuid>>(
            "SELECT rak_account_id FROM outlets WHERE id = $1"
        )
        .bind(req.from_outlet_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .flatten() {
            Some(id) => id,
            None => sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '1901' LIMIT 1")
                .fetch_one(&mut *tx)
                .await
                .map_err(|_| "Akun Rekening Antar-Kantor (1901) tidak ditemukan. Pastikan migrasi akun RAK telah dijalankan.".to_string())?,
        };

        let to_rak_acc: Uuid = match sqlx::query_scalar::<_, Option<Uuid>>(
            "SELECT rak_account_id FROM outlets WHERE id = $1"
        )
        .bind(req.to_outlet_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .flatten() {
            Some(id) => id,
            None => sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '1901' LIMIT 1")
                .fetch_one(&mut *tx)
                .await
                .map_err(|_| "Akun Rekening Antar-Kantor (1901) tidak ditemukan. Pastikan migrasi akun RAK telah dijalankan.".to_string())?,
        };

        if is_immediate {
            // 1. JURNAL CABANG PENGIRIM (Outlet Asal)
            let journal_from_id = Uuid::new_v4();
            journal_entry_id = Some(journal_from_id);
            let jrn_number_from = format!("JRN-TRF-OUT-{}-{}", now_str, &journal_from_id.to_string()[..4].to_uppercase());

            sqlx::query(
                "INSERT INTO journal_entries (
                    id, journal_number, journal_date, reference_type, reference_id, description,
                    total_debit, total_credit, is_closing_entry, created_at, updated_at, created_by, updated_by
                ) VALUES ($1, $2, $3, 'TRANSFER', $4, $5, $6, $6, FALSE, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $7, $7)"
            )
            .bind(journal_from_id)
            .bind(&jrn_number_from)
            .bind(req.transfer_date)
            .bind(transfer_id)
            .bind(format!("Pengiriman Transfer Stok ke {} (No: {})", to_name, transfer_number))
            .bind(total_transfer_amount)
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Gagal membuat jurnal transfer cabang pengirim: {}", e))?;

            // Baris DEBIT: Rekening Antar-Kantor (RAK) Cabang Asal
            sqlx::query(
                "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, $3, 0, $4, $5)"
            )
            .bind(journal_from_id)
            .bind(from_rak_acc)
            .bind(total_transfer_amount)
            .bind(format!("Piutang RAK transfer stok ke {}", to_name))
            .bind(Some(req.from_outlet_id))
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Gagal mencatat debit RAK asal: {}", e))?;

            // Baris KREDIT: Persediaan Outlet Asal
            sqlx::query(
                "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, 0, $3, $4, $5)"
            )
            .bind(journal_from_id)
            .bind(from_inv_acc)
            .bind(total_transfer_amount)
            .bind(format!("Pengeluaran transfer persediaan dari {}", from_name))
            .bind(Some(req.from_outlet_id))
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Gagal mencatat kredit persediaan asal: {}", e))?;

            // 2. JURNAL CABANG PENERIMA (Outlet Tujuan)
            let journal_to_id = Uuid::new_v4();
            let jrn_number_to = format!("JRN-TRF-IN-{}-{}", now_str, &journal_to_id.to_string()[..4].to_uppercase());

            sqlx::query(
                "INSERT INTO journal_entries (
                    id, journal_number, journal_date, reference_type, reference_id, description,
                    total_debit, total_credit, is_closing_entry, created_at, updated_at, created_by, updated_by
                ) VALUES ($1, $2, $3, 'TRANSFER', $4, $5, $6, $6, FALSE, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $7, $7)"
            )
            .bind(journal_to_id)
            .bind(&jrn_number_to)
            .bind(req.transfer_date)
            .bind(transfer_id)
            .bind(format!("Penerimaan Transfer Stok dari {} (No: {})", from_name, transfer_number))
            .bind(total_transfer_amount)
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Gagal membuat jurnal transfer cabang penerima: {}", e))?;

            // Baris DEBIT: Persediaan Outlet Tujuan
            sqlx::query(
                "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, $3, 0, $4, $5)"
            )
            .bind(journal_to_id)
            .bind(to_inv_acc)
            .bind(total_transfer_amount)
            .bind(format!("Penerimaan transfer persediaan di {}", to_name))
            .bind(Some(req.to_outlet_id))
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Gagal mencatat debit persediaan tujuan: {}", e))?;

            // Baris KREDIT: Rekening Antar-Kantor (RAK) Cabang Tujuan
            sqlx::query(
                "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, 0, $3, $4, $5)"
            )
            .bind(journal_to_id)
            .bind(to_rak_acc)
            .bind(total_transfer_amount)
            .bind(format!("Kewajiban RAK transfer stok dari {}", from_name))
            .bind(Some(req.to_outlet_id))
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Gagal mencatat kredit RAK tujuan: {}", e))?;
        } else {
            // Transfer 2-Tahap (Kirim):
            // 1. JURNAL CABANG PENGIRIM (Outlet Asal)
            let journal_from_id = Uuid::new_v4();
            journal_entry_id = Some(journal_from_id);
            let jrn_number_from = format!("JRN-TRF-OUT-{}-{}", now_str, &journal_from_id.to_string()[..4].to_uppercase());

            sqlx::query(
                "INSERT INTO journal_entries (
                    id, journal_number, journal_date, reference_type, reference_id, description,
                    total_debit, total_credit, is_closing_entry, created_at, updated_at, created_by, updated_by
                ) VALUES ($1, $2, $3, 'TRANSFER', $4, $5, $6, $6, FALSE, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $7, $7)"
            )
            .bind(journal_from_id)
            .bind(&jrn_number_from)
            .bind(req.transfer_date)
            .bind(transfer_id)
            .bind(format!("Pengiriman Transfer Persediaan {} -> {} [In-Transit] (No: {})", from_name, to_name, transfer_number))
            .bind(total_transfer_amount)
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Gagal membuat jurnal pengiriman transfer asal: {}", e))?;

            // Baris DEBIT: Rekening Antar-Kantor (RAK) Cabang Asal
            sqlx::query(
                "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, $3, 0, $4, $5)"
            )
            .bind(journal_from_id)
            .bind(from_rak_acc)
            .bind(total_transfer_amount)
            .bind(format!("Piutang RAK transfer persediaan dalam perjalanan ke {}", to_name))
            .bind(Some(req.from_outlet_id))
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Gagal mencatat debit RAK pengiriman: {}", e))?;

            // Baris KREDIT: Persediaan Outlet Asal
            sqlx::query(
                "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, 0, $3, $4, $5)"
            )
            .bind(journal_from_id)
            .bind(from_inv_acc)
            .bind(total_transfer_amount)
            .bind(format!("Pengeluaran transfer barang dari {}", from_name))
            .bind(Some(req.from_outlet_id))
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Gagal mencatat kredit persediaan asal: {}", e))?;

            // 2. JURNAL CABANG PENERIMA (Outlet Tujuan - In-Transit)
            let journal_to_id = Uuid::new_v4();
            let jrn_number_to = format!("JRN-TRF-IN-TRN-{}-{}", now_str, &journal_to_id.to_string()[..4].to_uppercase());

            sqlx::query(
                "INSERT INTO journal_entries (
                    id, journal_number, journal_date, reference_type, reference_id, description,
                    total_debit, total_credit, is_closing_entry, created_at, updated_at, created_by, updated_by
                ) VALUES ($1, $2, $3, 'TRANSFER', $4, $5, $6, $6, FALSE, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $7, $7)"
            )
            .bind(journal_to_id)
            .bind(&jrn_number_to)
            .bind(req.transfer_date)
            .bind(transfer_id)
            .bind(format!("Penerimaan In-Transit Transfer Persediaan dari {} (No: {})", from_name, transfer_number))
            .bind(total_transfer_amount)
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Gagal membuat jurnal in-transit penerima: {}", e))?;

            // Baris DEBIT: Persediaan Dalam Perjalanan (1202)
            sqlx::query(
                "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, $3, 0, $4, $5)"
            )
            .bind(journal_to_id)
            .bind(acc_in_transit)
            .bind(total_transfer_amount)
            .bind(format!("Persediaan dalam perjalanan ke {} dari {}", to_name, from_name))
            .bind(Some(req.to_outlet_id))
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Gagal mencatat debit persediaan dalam perjalanan: {}", e))?;

            // Baris KREDIT: Rekening Antar-Kantor (RAK) Cabang Tujuan
            sqlx::query(
                "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, 0, $3, $4, $5)"
            )
            .bind(journal_to_id)
            .bind(to_rak_acc)
            .bind(total_transfer_amount)
            .bind(format!("Kewajiban RAK persediaan dalam perjalanan dari {}", from_name))
            .bind(Some(req.to_outlet_id))
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Gagal mencatat kredit RAK in-transit: {}", e))?;
        }
    }

    tx.commit().await.map_err(|e| format!("Commit error: {}", e))?;

    let success_message = if is_immediate {
        format!("Transfer stok '{}' berhasil diproses secara langsung ke {}", transfer_number, to_name)
    } else {
        format!("Transfer stok '{}' berhasil dikirim. Status saat ini: Dalam Perjalanan (In-Transit)", transfer_number)
    };

    Ok(json!({
        "id": transfer_id,
        "transfer_number": transfer_number,
        "status": initial_status,
        "total_amount": total_transfer_amount,
        "items_count": req.items.len(),
        "journal_entry_id": journal_entry_id,
        "message": success_message
    }))
}

pub async fn receive_stock_transfer(
    pool: &PgPool,
    id: Uuid,
    user_id: Uuid,
    req: ReceiveStockTransferRequest,
) -> Result<serde_json::Value, String> {
    let mut tx = pool.begin().await.map_err(|e| format!("Transaction error: {}", e))?;

    let header = sqlx::query(
        "SELECT st.id, st.transfer_number, st.from_outlet_id, st.to_outlet_id, st.transfer_date,
                st.status, st.total_amount,
                fo.name as from_outlet_name, to_out.name as to_outlet_name,
                to_out.inventory_account_id as to_inv_acc_id
         FROM stock_transfers st
         JOIN outlets fo ON st.from_outlet_id = fo.id
         JOIN outlets to_out ON st.to_outlet_id = to_out.id
         WHERE st.id = $1 FOR UPDATE"
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| format!("Database error: {}", e))?
    .ok_or_else(|| "Dokumen transfer stok tidak ditemukan".to_string())?;

    let status: String = header.get("status");
    if status != "IN_TRANSIT" {
        return Err(format!("Transfer tidak dapat diterima karena status saat ini adalah '{}' (hanya transfer berstatus 'IN_TRANSIT' yang dapat diterima)", status));
    }

    let transfer_number: String = header.get("transfer_number");
    let from_name: String = header.get("from_outlet_name");
    let to_name: String = header.get("to_outlet_name");
    let to_outlet_id: Uuid = header.get("to_outlet_id");

    // Cek batas periode pembukuan untuk outlet penerima
    crate::services::accounting_service::check_transaction_date_allowed(pool, req.receive_date, Some(to_outlet_id)).await?;

    let to_inv_acc: Uuid = match header.get::<Option<Uuid>, _>("to_inv_acc_id") {
        Some(aid) => aid,
        None => sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '1201' LIMIT 1")
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| "Akun persediaan 1201 tidak ditemukan")?,
    };

    let acc_in_transit: Uuid = match sqlx::query_scalar::<_, Uuid>("SELECT id FROM chart_of_accounts WHERE code = '1202' LIMIT 1")
        .fetch_optional(&mut *tx)
        .await
        .unwrap_or(None)
    {
        Some(aid) => aid,
        None => to_inv_acc,
    };

    let acc_loss: Uuid = match sqlx::query_scalar::<_, Uuid>("SELECT id FROM chart_of_accounts WHERE code = '5301' LIMIT 1")
        .fetch_optional(&mut *tx)
        .await
        .unwrap_or(None)
    {
        Some(aid) => aid,
        None => match sqlx::query_scalar::<_, Uuid>("SELECT id FROM chart_of_accounts WHERE code = '5101' LIMIT 1")
            .fetch_optional(&mut *tx)
            .await
            .unwrap_or(None)
        {
            Some(aid) => aid,
            None => to_inv_acc,
        },
    };

    let existing_items = sqlx::query(
        "SELECT id, product_id, quantity, cost_price FROM stock_transfer_items WHERE transfer_id = $1"
    )
    .bind(id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| format!("Database error: {}", e))?;

    let mut total_received_amount = Decimal::ZERO;
    let mut total_shipped_amount = Decimal::ZERO;

    for row in existing_items {
        let item_id: Uuid = row.get("id");
        let product_id: Uuid = row.get("product_id");
        let shipped_qty: Decimal = row.get("quantity");
        let cost_price: Decimal = row.get("cost_price");
        total_shipped_amount += shipped_qty * cost_price;

        let (rec_qty, item_notes) = if let Some(items_input) = &req.items {
            if let Some(inp) = items_input.iter().find(|i| i.product_id == product_id) {
                if inp.received_quantity < Decimal::ZERO {
                    return Err("Kuantitas barang yang diterima tidak boleh bernilai negatif".to_string());
                }
                (inp.received_quantity, inp.notes.clone())
            } else {
                (shipped_qty, None)
            }
        } else {
            (shipped_qty, None)
        };

        let subtotal = rec_qty * cost_price;
        total_received_amount += subtotal;

        // Update baris stock_transfer_items
        sqlx::query(
            "UPDATE stock_transfer_items SET received_quantity = $1, notes = $2 WHERE id = $3"
        )
        .bind(rec_qty)
        .bind(&item_notes)
        .bind(item_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal memperbarui rincian barang yang diterima: {}", e))?;

        if rec_qty > Decimal::ZERO {
            // Rekalkulasi HPP & batch stok outlet tujuan sesuai metode penilaian
            let cost_res = crate::services::costing_service::recalculate_and_apply_cost_on_receipt(
                &mut tx,
                product_id,
                to_outlet_id,
                rec_qty,
                cost_price,
                "TRANSFER",
                Some(id),
                &transfer_number,
            )
            .await?;

            // Tambahkan stok fisik di outlet tujuan
            let current_to_stock: Decimal = sqlx::query_scalar(
                "SELECT quantity FROM warehouse_stocks WHERE warehouse_id = $1 AND product_id = $2 FOR UPDATE"
            )
            .bind(to_outlet_id)
            .bind(product_id)
            .fetch_optional(&mut *tx)
            .await
            .unwrap_or(None)
            .unwrap_or(Decimal::ZERO);

            let new_to_balance = current_to_stock + rec_qty;

            sqlx::query(
                "INSERT INTO warehouse_stocks (warehouse_id, product_id, quantity, created_at, updated_at)
                 VALUES ($1, $2, $3, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                 ON CONFLICT (warehouse_id, product_id)
                 DO UPDATE SET quantity = warehouse_stocks.quantity + $3, updated_at = CURRENT_TIMESTAMP"
            )
            .bind(to_outlet_id)
            .bind(product_id)
            .bind(rec_qty)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Gagal menambah stok di outlet tujuan: {}", e))?;

            // Catat mutasi TRANSFER masuk di outlet tujuan
            sqlx::query(
                "INSERT INTO stock_mutations (
                    warehouse_id, product_id, reference_type, reference_id, qty_in, qty_out, balance, cost_price, notes, created_by
                ) VALUES ($1, $2, 'TRANSFER', $3, $4, 0, $5, $6, $7, $8)"
            )
            .bind(to_outlet_id)
            .bind(product_id)
            .bind(id)
            .bind(rec_qty)
            .bind(new_to_balance)
            .bind(cost_res.new_cost_price)
            .bind(format!("Penerimaan Transfer Masuk dari {} ({}) [Metode: {}, HPP: Rp {}]", from_name, transfer_number, cost_res.costing_method, cost_res.new_cost_price))
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Gagal mencatat mutasi stok penerimaan: {}", e))?;
        }
    }

    // Update status stock_transfers
    let notes_to_save = match &req.notes {
        Some(n) if !n.trim().is_empty() => Some(n.trim().to_string()),
        _ => None,
    };

    sqlx::query(
        "UPDATE stock_transfers SET
            status = 'COMPLETED',
            received_date = $1,
            received_by = $2,
            received_at = CURRENT_TIMESTAMP,
            notes = COALESCE($3, notes),
            updated_by = $2,
            updated_at = CURRENT_TIMESTAMP
         WHERE id = $4"
    )
    .bind(req.receive_date)
    .bind(user_id)
    .bind(&notes_to_save)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal memperbarui status dokumen transfer: {}", e))?;

    // Catat Jurnal Penerimaan Transfer
    let now_str = Utc::now().format("%Y%m%d%H%M%S").to_string();
    let journal_id = Uuid::new_v4();
    let jrn_number = format!("JRN-RCV-TRF-{}-{}", now_str, &journal_id.to_string()[..4].to_uppercase());

    let shortage_amount = if total_shipped_amount > total_received_amount {
        total_shipped_amount - total_received_amount
    } else {
        Decimal::ZERO
    };

    if total_shipped_amount > Decimal::ZERO {
        sqlx::query(
            "INSERT INTO journal_entries (
                id, journal_number, journal_date, reference_type, reference_id, description,
                total_debit, total_credit, is_closing_entry, created_at, updated_at, created_by, updated_by
            ) VALUES ($1, $2, $3, 'TRANSFER', $4, $5, $6, $6, FALSE, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $7, $7)"
        )
        .bind(journal_id)
        .bind(&jrn_number)
        .bind(req.receive_date)
        .bind(id)
        .bind(format!("Penerimaan Transfer Persediaan di {} dari {} (No: {})", to_name, from_name, transfer_number))
        .bind(total_shipped_amount)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal membuat jurnal penerimaan transfer: {}", e))?;

        // Debit Persediaan Outlet Tujuan
        if total_received_amount > Decimal::ZERO {
            sqlx::query(
                "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, $3, 0, $4, $5)"
            )
            .bind(journal_id)
            .bind(to_inv_acc)
            .bind(total_received_amount)
            .bind(format!("Penerimaan fisik barang transfer di {}", to_name))
            .bind(Some(to_outlet_id))
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Gagal mencatat debit persediaan tujuan: {}", e))?;
        }

        // Debit Selisih/Kerusakan jika ada barang kurang saat transfer
        if shortage_amount > Decimal::ZERO {
            sqlx::query(
                "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, $3, 0, $4, $5)"
            )
            .bind(journal_id)
            .bind(acc_loss)
            .bind(shortage_amount)
            .bind("Beban selisih atau kerusakan barang selama pengiriman transfer".to_string())
            .bind(Some(to_outlet_id))
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Gagal mencatat beban selisih transfer: {}", e))?;
        }

        // Kredit In-Transit (menghapus saldo 1202)
        sqlx::query(
            "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, 0, $3, $4, $5)"
        )
        .bind(journal_id)
        .bind(acc_in_transit)
        .bind(total_shipped_amount)
        .bind("Penyelesaian persediaan dalam perjalanan (in-transit)".to_string())
        .bind(Some(to_outlet_id))
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal mencatat kredit in-transit: {}", e))?;
    }

    tx.commit().await.map_err(|e| format!("Commit error: {}", e))?;

    Ok(json!({
        "id": id,
        "transfer_number": transfer_number,
        "status": "COMPLETED",
        "total_received_amount": total_received_amount,
        "shortage_amount": shortage_amount,
        "journal_entry_id": journal_id,
        "message": format!("Barang transfer '{}' berhasil diterima dan dimasukkan ke stok {}", transfer_number, to_name)
    }))
}

pub async fn cancel_stock_transfer(
    pool: &PgPool,
    id: Uuid,
    user_id: Uuid,
    req: CancelStockTransferRequest,
) -> Result<serde_json::Value, String> {
    let mut tx = pool.begin().await.map_err(|e| format!("Transaction error: {}", e))?;

    let header = sqlx::query(
        "SELECT st.id, st.transfer_number, st.from_outlet_id, st.to_outlet_id, st.transfer_date,
                st.status, st.total_amount,
                fo.name as from_outlet_name, fo.inventory_account_id as from_inv_acc_id
         FROM stock_transfers st
         JOIN outlets fo ON st.from_outlet_id = fo.id
         WHERE st.id = $1 FOR UPDATE"
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| format!("Database error: {}", e))?
    .ok_or_else(|| "Dokumen transfer stok tidak ditemukan".to_string())?;

    let status: String = header.get("status");
    if status == "COMPLETED" {
        return Err("Transfer yang sudah berstatus SELESAI / COMPLETED tidak dapat dibatalkan langsung. Silakan buat transfer pengembalian stok.".to_string());
    }
    if status == "CANCELLED" {
        return Err("Transfer ini sudah dibatalkan sebelumnya".to_string());
    }
    if status != "IN_TRANSIT" {
        return Err(format!("Hanya transfer berstatus 'IN_TRANSIT' yang dapat dibatalkan (status saat ini: {})", status));
    }

    let transfer_number: String = header.get("transfer_number");
    let from_outlet_id: Uuid = header.get("from_outlet_id");
    let to_outlet_id: Uuid = header.get("to_outlet_id");
    let from_name: String = header.get("from_outlet_name");
    let total_amount: Decimal = header.get("total_amount");

    let from_rak_acc: Uuid = match sqlx::query_scalar::<_, Option<Uuid>>(
        "SELECT rak_account_id FROM outlets WHERE id = $1"
    )
    .bind(from_outlet_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| format!("Database error: {}", e))?
    .flatten() {
        Some(id) => id,
        None => sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '1901' LIMIT 1")
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| "Akun Rekening Antar-Kantor (1901) tidak ditemukan".to_string())?,
    };

    let to_rak_acc: Uuid = match sqlx::query_scalar::<_, Option<Uuid>>(
        "SELECT rak_account_id FROM outlets WHERE id = $1"
    )
    .bind(to_outlet_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| format!("Database error: {}", e))?
    .flatten() {
        Some(id) => id,
        None => sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '1901' LIMIT 1")
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| "Akun Rekening Antar-Kantor (1901) tidak ditemukan".to_string())?,
    };

    let from_inv_acc: Uuid = match header.get::<Option<Uuid>, _>("from_inv_acc_id") {
        Some(aid) => aid,
        None => sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '1201' LIMIT 1")
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| "Akun persediaan 1201 tidak ditemukan")?,
    };

    let acc_in_transit: Uuid = match sqlx::query_scalar::<_, Uuid>("SELECT id FROM chart_of_accounts WHERE code = '1202' LIMIT 1")
        .fetch_optional(&mut *tx)
        .await
        .unwrap_or(None)
    {
        Some(aid) => aid,
        None => from_inv_acc,
    };

    let items = sqlx::query(
        "SELECT id, product_id, quantity, cost_price FROM stock_transfer_items WHERE transfer_id = $1"
    )
    .bind(id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| format!("Database error: {}", e))?;

    // Kembalikan stok fisik ke outlet/lokasi asal
    for row in items {
        let product_id: Uuid = row.get("product_id");
        let quantity: Decimal = row.get("quantity");
        let cost_price: Decimal = row.get("cost_price");

        let current_stock: Decimal = sqlx::query_scalar(
            "SELECT quantity FROM warehouse_stocks WHERE warehouse_id = $1 AND product_id = $2 FOR UPDATE"
        )
        .bind(from_outlet_id)
        .bind(product_id)
        .fetch_optional(&mut *tx)
        .await
        .unwrap_or(None)
        .unwrap_or(Decimal::ZERO);

        let new_balance = current_stock + quantity;

        sqlx::query(
            "INSERT INTO warehouse_stocks (warehouse_id, product_id, quantity, created_at, updated_at)
             VALUES ($1, $2, $3, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
             ON CONFLICT (warehouse_id, product_id)
             DO UPDATE SET quantity = warehouse_stocks.quantity + $3, updated_at = CURRENT_TIMESTAMP"
        )
        .bind(from_outlet_id)
        .bind(product_id)
        .bind(quantity)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal mengembalikan stok ke outlet asal: {}", e))?;

        sqlx::query(
            "INSERT INTO stock_mutations (
                warehouse_id, product_id, reference_type, reference_id, qty_in, qty_out, balance, cost_price, notes, created_by
            ) VALUES ($1, $2, 'TRANSFER', $3, $4, 0, $5, $6, $7, $8)"
        )
        .bind(from_outlet_id)
        .bind(product_id)
        .bind(id)
        .bind(quantity)
        .bind(new_balance)
        .bind(cost_price)
        .bind(format!("Pembatalan Transfer Keluar ({})", transfer_number))
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal mencatat mutasi pengembalian stok: {}", e))?;
    }

    // Jurnal Pembalik Berpasangan (Per-Outlet via Rekening Antar-Kantor / RAK):
    let now_str = Utc::now().format("%Y%m%d%H%M%S").to_string();

    if total_amount > Decimal::ZERO {
        let cancel_reason_str = req.reason.as_deref().unwrap_or("Dibatalkan oleh pengguna");

        // 1. JURNAL PEMBALIK CABANG PENGIRIM (Outlet Asal)
        let journal_from_id = Uuid::new_v4();
        let jrn_number_from = format!("JRN-CNL-TRF-OUT-{}-{}", now_str, &journal_from_id.to_string()[..4].to_uppercase());

        sqlx::query(
            "INSERT INTO journal_entries (
                id, journal_number, journal_date, reference_type, reference_id, description,
                total_debit, total_credit, is_closing_entry, created_at, updated_at, created_by, updated_by
            ) VALUES ($1, $2, CURRENT_DATE, 'TRANSFER', $3, $4, $5, $5, FALSE, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $6, $6)"
        )
        .bind(journal_from_id)
        .bind(&jrn_number_from)
        .bind(id)
        .bind(format!("Pembalik Pembatalan Transfer Stok ke {} (No: {}, Alasan: {})", to_outlet_id, transfer_number, cancel_reason_str))
        .bind(total_amount)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal membuat jurnal pembatalan transfer asal: {}", e))?;

        // Debit Persediaan Outlet Asal
        sqlx::query(
            "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, $3, 0, $4, $5)"
        )
        .bind(journal_from_id)
        .bind(from_inv_acc)
        .bind(total_amount)
        .bind(format!("Pengembalian persediaan ke outlet asal {}", from_name))
        .bind(Some(from_outlet_id))
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal mencatat debit jurnal pembatalan: {}", e))?;

        // Kredit RAK Cabang Asal
        sqlx::query(
            "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, 0, $3, $4, $5)"
        )
        .bind(journal_from_id)
        .bind(from_rak_acc)
        .bind(total_amount)
        .bind("Pembalik piutang RAK transfer batal".to_string())
        .bind(Some(from_outlet_id))
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal mencatat kredit RAK jurnal pembatalan: {}", e))?;

        // 2. JURNAL PEMBALIK CABANG PENERIMA (Outlet Tujuan)
        let journal_to_id = Uuid::new_v4();
        let jrn_number_to = format!("JRN-CNL-TRF-IN-{}-{}", now_str, &journal_to_id.to_string()[..4].to_uppercase());

        sqlx::query(
            "INSERT INTO journal_entries (
                id, journal_number, journal_date, reference_type, reference_id, description,
                total_debit, total_credit, is_closing_entry, created_at, updated_at, created_by, updated_by
            ) VALUES ($1, $2, CURRENT_DATE, 'TRANSFER', $3, $4, $5, $5, FALSE, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $6, $6)"
        )
        .bind(journal_to_id)
        .bind(&jrn_number_to)
        .bind(id)
        .bind(format!("Pembalik Pembatalan In-Transit dari {} (No: {}, Alasan: {})", from_name, transfer_number, cancel_reason_str))
        .bind(total_amount)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal membuat jurnal pembatalan transfer tujuan: {}", e))?;

        // Debit RAK Cabang Tujuan
        sqlx::query(
            "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, $3, 0, $4, $5)"
        )
        .bind(journal_to_id)
        .bind(to_rak_acc)
        .bind(total_amount)
        .bind("Pembalik kewajiban RAK transfer batal".to_string())
        .bind(Some(to_outlet_id))
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal mencatat debit RAK jurnal pembatalan: {}", e))?;

        // Kredit In-Transit
        sqlx::query(
            "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, 0, $3, $4, $5)"
        )
        .bind(journal_to_id)
        .bind(acc_in_transit)
        .bind(total_amount)
        .bind("Pembalik persediaan dalam perjalanan (in-transit)".to_string())
        .bind(Some(to_outlet_id))
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal mencatat kredit in-transit jurnal pembatalan: {}", e))?;
    }

    // Update status transfer
    sqlx::query(
        "UPDATE stock_transfers SET
            status = 'CANCELLED',
            cancellation_reason = $1,
            cancelled_by = $2,
            cancelled_at = CURRENT_TIMESTAMP,
            updated_by = $2,
            updated_at = CURRENT_TIMESTAMP
         WHERE id = $3"
    )
    .bind(&req.reason)
    .bind(user_id)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal memperbarui status pembatalan transfer: {}", e))?;

    tx.commit().await.map_err(|e| format!("Commit error: {}", e))?;

    Ok(json!({
        "id": id,
        "transfer_number": transfer_number,
        "status": "CANCELLED",
        "message": format!("Transfer stok '{}' berhasil dibatalkan dan barang telah dikembalikan ke stok {}", transfer_number, from_name)
    }))
}

pub async fn list_stock_transfers(
    pool: &PgPool,
    filter: StockTransferQueryFilter,
    owner_id: Option<Uuid>,
) -> Result<serde_json::Value, String> {
    let page = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * page_size;

    let mut conditions = vec!["1=1".to_string()];
    if let Some(uid) = owner_id {
        conditions.push(format!("st.created_by = '{uid}'"));
    }
    if let Some(s) = &filter.search {
        if !s.trim().is_empty() {
            let escaped = s.trim().replace('\'', "''");
            conditions.push(format!("(st.transfer_number ILIKE '%{}%' OR st.notes ILIKE '%{}%')", escaped, escaped));
        }
    }
    if let Some(from_id) = filter.from_outlet_id {
        conditions.push(format!("st.from_outlet_id = '{}'", from_id));
    }
    if let Some(to_id) = filter.to_outlet_id {
        conditions.push(format!("st.to_outlet_id = '{}'", to_id));
    }
    if let Some(status) = &filter.status {
        if !status.trim().is_empty() && status.trim() != "ALL" {
            let escaped = status.trim().replace('\'', "''");
            conditions.push(format!("st.status = '{}'", escaped));
        }
    }
    if let Some(start) = filter.start_date {
        conditions.push(format!("st.transfer_date >= '{}'", start));
    }
    if let Some(end) = filter.end_date {
        conditions.push(format!("st.transfer_date <= '{}'", end));
    }

    let where_clause = conditions.join(" AND ");

    let count_sql = format!("SELECT COUNT(*) FROM stock_transfers st WHERE {}", where_clause);
    let total_items: i64 = sqlx::query_scalar(&count_sql).fetch_one(pool).await.unwrap_or(0);

    let select_sql = format!(
        "SELECT st.id, st.transfer_number, st.from_outlet_id, st.to_outlet_id, st.transfer_date,
                st.status, st.notes, st.total_amount, st.created_at,
                st.shipped_at, st.received_date, st.received_at, st.cancellation_reason, st.cancelled_at,
                fo.name as from_outlet_name, fo.code as from_outlet_code,
                to_out.name as to_outlet_name, to_out.code as to_outlet_code,
                u.full_name as creator_name,
                ru.full_name as receiver_name,
                cu.full_name as canceller_name,
                (SELECT COUNT(*) FROM stock_transfer_items WHERE transfer_id = st.id) as items_count,
                (SELECT COALESCE(SUM(quantity), 0) FROM stock_transfer_items WHERE transfer_id = st.id) as total_shipped_qty,
                (SELECT COALESCE(SUM(received_quantity), 0) FROM stock_transfer_items WHERE transfer_id = st.id) as total_received_qty
         FROM stock_transfers st
         JOIN outlets fo ON st.from_outlet_id = fo.id
         JOIN outlets to_out ON st.to_outlet_id = to_out.id
         LEFT JOIN users u ON st.created_by = u.id
         LEFT JOIN users ru ON st.received_by = ru.id
         LEFT JOIN users cu ON st.cancelled_by = cu.id
         WHERE {}
         ORDER BY st.created_at DESC
         LIMIT {} OFFSET {}",
        where_clause, page_size, offset
    );

    let rows = sqlx::query(&select_sql).fetch_all(pool).await.map_err(|e| format!("Database error: {}", e))?;

    let list: Vec<serde_json::Value> = rows.iter().map(|r| {
        json!({
            "id": r.get::<Uuid, _>("id"),
            "transfer_number": r.get::<String, _>("transfer_number"),
            "from_outlet_id": r.get::<Uuid, _>("from_outlet_id"),
            "from_outlet_name": r.get::<String, _>("from_outlet_name"),
            "from_outlet_code": r.get::<String, _>("from_outlet_code"),
            "to_outlet_id": r.get::<Uuid, _>("to_outlet_id"),
            "to_outlet_name": r.get::<String, _>("to_outlet_name"),
            "to_outlet_code": r.get::<String, _>("to_outlet_code"),
            "transfer_date": r.get::<NaiveDate, _>("transfer_date"),
            "status": r.get::<String, _>("status"),
            "notes": r.get::<Option<String>, _>("notes"),
            "total_amount": r.get::<Decimal, _>("total_amount"),
            "creator_name": r.get::<Option<String>, _>("creator_name"),
            "receiver_name": r.get::<Option<String>, _>("receiver_name"),
            "canceller_name": r.get::<Option<String>, _>("canceller_name"),
            "shipped_at": r.get::<Option<DateTime<Utc>>, _>("shipped_at"),
            "received_date": r.get::<Option<NaiveDate>, _>("received_date"),
            "received_at": r.get::<Option<DateTime<Utc>>, _>("received_at"),
            "cancellation_reason": r.get::<Option<String>, _>("cancellation_reason"),
            "cancelled_at": r.get::<Option<DateTime<Utc>>, _>("cancelled_at"),
            "items_count": r.get::<i64, _>("items_count"),
            "total_shipped_qty": r.get::<Decimal, _>("total_shipped_qty"),
            "total_received_qty": r.get::<Decimal, _>("total_received_qty"),
            "created_at": r.get::<DateTime<Utc>, _>("created_at"),
        })
    }).collect();

    Ok(json!({
        "items": list,
        "total": total_items,
        "page": page,
        "page_size": page_size,
        "total_pages": (total_items as f64 / page_size as f64).ceil() as i64
    }))
}

pub async fn get_stock_transfer_detail(pool: &PgPool, id: Uuid) -> Result<serde_json::Value, String> {
    let header = sqlx::query(
        "SELECT st.id, st.transfer_number, st.from_outlet_id, st.to_outlet_id, st.transfer_date,
                st.status, st.notes, st.total_amount, st.created_at,
                st.shipped_at, st.received_date, st.received_at, st.cancellation_reason, st.cancelled_at,
                fo.name as from_outlet_name, fo.code as from_outlet_code,
                to_out.name as to_outlet_name, to_out.code as to_outlet_code,
                u.full_name as creator_name,
                ru.full_name as receiver_name,
                cu.full_name as canceller_name
         FROM stock_transfers st
         JOIN outlets fo ON st.from_outlet_id = fo.id
         JOIN outlets to_out ON st.to_outlet_id = to_out.id
         LEFT JOIN users u ON st.created_by = u.id
         LEFT JOIN users ru ON st.received_by = ru.id
         LEFT JOIN users cu ON st.cancelled_by = cu.id
         WHERE st.id = $1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?
    .ok_or_else(|| "Transfer stok tidak ditemukan".to_string())?;

    let item_rows = sqlx::query(
        "SELECT sti.id, sti.product_id, sti.quantity, sti.received_quantity, sti.cost_price, sti.subtotal, sti.notes,
                p.sku as product_code, p.name as product_name, p.barcode,
                u.code as unit_code
         FROM stock_transfer_items sti
         JOIN products p ON sti.product_id = p.id
         LEFT JOIN units u ON p.base_unit_id = u.id
         WHERE sti.transfer_id = $1
         ORDER BY p.name ASC"
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?;

    let items: Vec<serde_json::Value> = item_rows.iter().map(|r| {
        json!({
            "id": r.get::<Uuid, _>("id"),
            "product_id": r.get::<Uuid, _>("product_id"),
            "product_code": r.get::<String, _>("product_code"),
            "product_name": r.get::<String, _>("product_name"),
            "barcode": r.get::<Option<String>, _>("barcode"),
            "unit_code": r.get::<Option<String>, _>("unit_code").unwrap_or_else(|| "PCS".to_string()),
            "quantity": r.get::<Decimal, _>("quantity"),
            "received_quantity": r.get::<Option<Decimal>, _>("received_quantity"),
            "cost_price": r.get::<Decimal, _>("cost_price"),
            "subtotal": r.get::<Decimal, _>("subtotal"),
            "notes": r.get::<Option<String>, _>("notes"),
        })
    }).collect();

    // Ambil semua jurnal terkait jika ada (jurnal kirim, terima, batal)
    let journal_rows = sqlx::query(
        "SELECT id, journal_number, journal_date, total_debit, total_credit, description
         FROM journal_entries WHERE reference_type = 'TRANSFER' AND reference_id = $1
         ORDER BY created_at ASC"
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut journals = Vec::new();
    for j in journal_rows {
        let jid: Uuid = j.get("id");
        let lines = sqlx::query(
            "SELECT jl.id, jl.account_id, coa.code as account_code, coa.name as account_name,
                    jl.debit, jl.credit, jl.memo
             FROM journal_lines jl
             JOIN chart_of_accounts coa ON jl.account_id = coa.id
             WHERE jl.journal_id = $1
             ORDER BY jl.debit DESC"
        )
        .bind(jid)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        let line_details: Vec<serde_json::Value> = lines.iter().map(|l| json!({
            "id": l.get::<Uuid, _>("id"),
            "account_code": l.get::<String, _>("account_code"),
            "account_name": l.get::<String, _>("account_name"),
            "debit": l.get::<Decimal, _>("debit"),
            "credit": l.get::<Decimal, _>("credit"),
            "memo": l.get::<Option<String>, _>("memo"),
        })).collect();

        journals.push(json!({
            "id": jid,
            "journal_number": j.get::<String, _>("journal_number"),
            "journal_date": j.get::<NaiveDate, _>("journal_date"),
            "description": j.get::<String, _>("description"),
            "total_debit": j.get::<Decimal, _>("total_debit"),
            "total_credit": j.get::<Decimal, _>("total_credit"),
            "lines": line_details,
        }));
    }

    Ok(json!({
        "id": header.get::<Uuid, _>("id"),
        "transfer_number": header.get::<String, _>("transfer_number"),
        "from_outlet_id": header.get::<Uuid, _>("from_outlet_id"),
        "from_outlet_name": header.get::<String, _>("from_outlet_name"),
        "from_outlet_code": header.get::<String, _>("from_outlet_code"),
        "to_outlet_id": header.get::<Uuid, _>("to_outlet_id"),
        "to_outlet_name": header.get::<String, _>("to_outlet_name"),
        "to_outlet_code": header.get::<String, _>("to_outlet_code"),
        "transfer_date": header.get::<NaiveDate, _>("transfer_date"),
        "status": header.get::<String, _>("status"),
        "notes": header.get::<Option<String>, _>("notes"),
        "total_amount": header.get::<Decimal, _>("total_amount"),
        "creator_name": header.get::<Option<String>, _>("creator_name"),
        "receiver_name": header.get::<Option<String>, _>("receiver_name"),
        "canceller_name": header.get::<Option<String>, _>("canceller_name"),
        "shipped_at": header.get::<Option<DateTime<Utc>>, _>("shipped_at"),
        "received_date": header.get::<Option<NaiveDate>, _>("received_date"),
        "received_at": header.get::<Option<DateTime<Utc>>, _>("received_at"),
        "cancellation_reason": header.get::<Option<String>, _>("cancellation_reason"),
        "cancelled_at": header.get::<Option<DateTime<Utc>>, _>("cancelled_at"),
        "created_at": header.get::<DateTime<Utc>, _>("created_at"),
        "items": items,
        "journals": journals,
        "journal": journals.first().cloned(),
    }))
}

pub async fn cancel_goods_receipt(
    pool: &PgPool,
    id: Uuid,
    user_id: Option<Uuid>,
) -> Result<String, String> {
    let mut tx = pool.begin().await.map_err(|e| format!("Transaction error: {}", e))?;

    let grn_row = sqlx::query(
        "SELECT id, grn_number, po_id, outlet_id, status FROM goods_receipts WHERE id = $1 FOR UPDATE"
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| format!("Database error: {}", e))?
    .ok_or_else(|| "Dokumen penerimaan barang tidak ditemukan".to_string())?;

    let status: String = grn_row.get("status");
    if status == "INVOICED" || status == "PARTIALLY_INVOICED" {
        return Err("Penerimaan barang yang sudah diterbitkan faktur pembelian tidak dapat dibatalkan langsung. Batalkan faktur terlebih dahulu.".to_string());
    }
    if status == "CANCELLED" {
        return Err("Penerimaan barang ini sudah dibatalkan sebelumnya".to_string());
    }

    let grn_number: String = grn_row.get("grn_number");
    let po_id: Uuid = grn_row.get("po_id");
    let outlet_id: Option<Uuid> = grn_row.get("outlet_id");

    let items = sqlx::query(
        "SELECT id, po_item_id, product_id, received_qty, unit_price, subtotal FROM goods_receipt_items WHERE goods_receipt_id = $1"
    )
    .bind(id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| format!("Database error: {}", e))?;

    let mut total_revert_value = Decimal::ZERO;

    for it in items {
        let po_item_id: Option<Uuid> = it.get("po_item_id");
        let product_id: Uuid = it.get("product_id");
        let rec_qty: Decimal = it.get("received_qty");
        let unit_price: Decimal = it.get("unit_price");
        let subtotal: Decimal = it.get("subtotal");

        total_revert_value += subtotal;

        // Kurangi kembali received_qty di purchase_order_items
        if let Some(p_item_id) = po_item_id {
            sqlx::query(
                "UPDATE purchase_order_items 
                 SET received_qty = GREATEST(0, received_qty - $1), updated_at = CURRENT_TIMESTAMP 
                 WHERE id = $2"
            )
            .bind(rec_qty)
            .bind(p_item_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Gagal membalik kuantitas PO: {}", e))?;
        }

        // Kembalikan stok fisik jika ada outlet
        if let Some(wh_id) = outlet_id {
            let current_stock: Decimal = sqlx::query_scalar(
                "SELECT COALESCE(quantity, 0) FROM warehouse_stocks WHERE warehouse_id = $1 AND product_id = $2"
            )
            .bind(wh_id)
            .bind(product_id)
            .fetch_optional(&mut *tx)
            .await
            .unwrap_or(None)
            .unwrap_or(Decimal::ZERO);

            let new_stock = (current_stock - rec_qty).max(Decimal::ZERO);

            sqlx::query(
                "UPDATE warehouse_stocks SET quantity = $1, updated_at = CURRENT_TIMESTAMP 
                 WHERE warehouse_id = $2 AND product_id = $3"
            )
            .bind(new_stock)
            .bind(wh_id)
            .bind(product_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Gagal mengurangi stok fisik: {}", e))?;

            // Mutasi stok pembalik
            sqlx::query(
                "INSERT INTO stock_mutations (
                    warehouse_id, product_id, reference_type, reference_id, qty_in, qty_out,
                    balance, cost_price, notes, created_at, updated_at, created_by, updated_by
                ) VALUES ($1, $2, 'PURCHASE_GRN_CANCEL', $3, 0, $4, $5, $6, $7, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $8, $8)"
            )
            .bind(wh_id)
            .bind(product_id)
            .bind(id)
            .bind(rec_qty)
            .bind(new_stock)
            .bind(unit_price)
            .bind(format!("Pembatalan Penerimaan Barang Fisik No {}", grn_number))
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Gagal mencatat mutasi stok pembatalan: {}", e))?;
        }
    }

    // Evaluasi ulang status PO
    let any_received: Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(received_qty), 0) FROM purchase_order_items WHERE po_id = $1"
    )
    .bind(po_id)
    .fetch_one(&mut *tx)
    .await
    .unwrap_or(Decimal::ZERO);

    let new_po_status = if any_received > Decimal::ZERO {
        "PARTIAL"
    } else {
        "PENDING"
    };

    sqlx::query("UPDATE purchase_orders SET status = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
        .bind(new_po_status)
        .bind(po_id)
        .execute(&mut *tx)
        .await
        .ok();

    // Update status GRN menjadi CANCELLED
    sqlx::query("UPDATE goods_receipts SET status = 'CANCELLED', updated_at = CURRENT_TIMESTAMP WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal mengubah status GRN: {}", e))?;

    // Jurnal Pembalik GRN:
    // DEBIT: 2103 Unbilled GRN
    // KREDIT: 1201 Persediaan
    let journal_id = Uuid::new_v4();
    let jrn_number = format!("JRN-REV-GRN-{}-{}", Utc::now().format("%Y%m%d%H%M%S"), &journal_id.to_string()[..4].to_uppercase());

    sqlx::query(
        "INSERT INTO journal_entries (
            id, journal_number, journal_date, reference_type, reference_id, description,
            total_debit, total_credit, is_closing_entry, created_at, updated_at, created_by, updated_by
        ) VALUES (
            $1, $2, CURRENT_DATE, 'PURCHASE_GRN_CANCEL', $3, $4, $5, $5, FALSE, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $6, $6
        )",
    )
    .bind(journal_id)
    .bind(jrn_number)
    .bind(id)
    .bind(format!("Pembalikan Jurnal Penerimaan Barang (Pembatalan GRN No {})", grn_number))
    .bind(total_revert_value)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal membuat jurnal pembalik GRN: {}", e))?;

    let acc_unbilled: Uuid = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '2103' LIMIT 1")
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| "Akun 2103 tidak ditemukan")?;

    let acc_inv: Uuid = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '1201' LIMIT 1")
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| "Akun 1201 tidak ditemukan")?;

    sqlx::query("INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, $3, 0, 'Pembalikan Hutang Unbilled GRN', $4)")
        .bind(journal_id).bind(acc_unbilled).bind(total_revert_value).bind(outlet_id)
        .execute(&mut *tx).await.map_err(|e| format!("Gagal baris jurnal: {}", e))?;

    sqlx::query("INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, 0, $3, 'Pembalikan Persediaan Barang Dagang', $4)")
        .bind(journal_id).bind(acc_inv).bind(total_revert_value).bind(outlet_id)
        .execute(&mut *tx).await.map_err(|e| format!("Gagal baris jurnal: {}", e))?;

    tx.commit().await.map_err(|e| format!("Commit error: {}", e))?;

    Ok(format!("Penerimaan barang {} berhasil dibatalkan", grn_number))
}

pub async fn cancel_purchase_invoice(
    pool: &PgPool,
    id: Uuid,
    user_id: Option<Uuid>,
) -> Result<String, String> {
    let mut tx = pool.begin().await.map_err(|e| format!("Transaction error: {}", e))?;

    let inv = sqlx::query(
        "SELECT invoice_number, goods_receipt_id, subtotal, discount_amount, tax_amount, total_amount, paid_amount, payment_status, outlet_id 
         FROM purchase_invoices WHERE id = $1 FOR UPDATE"
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| format!("Database error: {}", e))?
    .ok_or_else(|| "Faktur pembelian tidak ditemukan".to_string())?;

    let inv_outlet_id: Option<Uuid> = inv.get("outlet_id");

    let status: String = inv.get("payment_status");
    if status == "CANCELLED" {
        return Err("Faktur pembelian ini sudah dibatalkan".to_string());
    }
    let paid: Decimal = inv.get("paid_amount");
    if paid > Decimal::ZERO {
        return Err("Faktur pembelian yang sudah dibayar/dilunasi tidak dapat dibatalkan langsung. Gunakan menu Retur Pembelian.".to_string());
    }

    let inv_number: String = inv.get("invoice_number");
    let grn_id: Option<Uuid> = inv.get("goods_receipt_id");
    let subtotal: Decimal = inv.get("subtotal");
    let discount: Decimal = inv.get("discount_amount");
    let tax: Decimal = inv.get("tax_amount");
    let total_amount: Decimal = inv.get("total_amount");

    // Jika memiliki rujukan GRN: kembalikan invoiced_qty di goods_receipt_items dan status GRN kembali ke UNINVOICED
    if let Some(gid) = grn_id {
        let items = sqlx::query(
            "SELECT goods_receipt_item_id, received_qty FROM purchase_invoice_items WHERE invoice_id = $1"
        )
        .bind(id)
        .fetch_all(&mut *tx)
        .await
        .unwrap_or_default();

        for it in items {
            let grn_item_id: Option<Uuid> = it.get("goods_receipt_item_id");
            let rec_qty: Decimal = it.get("received_qty");
            if let Some(git_id) = grn_item_id {
                sqlx::query(
                    "UPDATE goods_receipt_items SET invoiced_qty = GREATEST(0, invoiced_qty - $1), updated_at = CURRENT_TIMESTAMP WHERE id = $2"
                )
                .bind(rec_qty)
                .bind(git_id)
                .execute(&mut *tx)
                .await
                .ok();
            }
        }

        sqlx::query(
            "UPDATE goods_receipts SET status = 'UNINVOICED', updated_at = CURRENT_TIMESTAMP WHERE id = $1"
        )
        .bind(gid)
        .execute(&mut *tx)
        .await
        .ok();
    }

    // Ubah status invoice menjadi CANCELLED
    sqlx::query(
        "UPDATE purchase_invoices SET payment_status = 'CANCELLED', updated_at = CURRENT_TIMESTAMP WHERE id = $1"
    )
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal membatalkan faktur: {}", e))?;

    // Jurnal Pembalik Faktur Pembelian:
    // DEBIT: 2101 Hutang Usaha (total_amount)
    // DEBIT (jika ada diskon): 5102 Potongan Pembelian (discount)
    // KREDIT: 2103 Unbilled GRN (subtotal)
    // KREDIT (jika ada pajak): 1104 Pajak Masukan (tax)
    let journal_id = Uuid::new_v4();
    let jrn_number = format!("JRN-REV-INV-{}-{}", Utc::now().format("%Y%m%d%H%M%S"), &journal_id.to_string()[..4].to_uppercase());
    let journal_total = subtotal + tax;

    sqlx::query(
        "INSERT INTO journal_entries (
            id, journal_number, journal_date, reference_type, reference_id, description,
            total_debit, total_credit, is_closing_entry, created_at, updated_at, created_by, updated_by
        ) VALUES (
            $1, $2, CURRENT_DATE, 'PURCHASE_INVOICE_CANCEL', $3, $4, $5, $5, FALSE, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $6, $6
        )",
    )
    .bind(journal_id)
    .bind(jrn_number)
    .bind(id)
    .bind(format!("Pembalikan Jurnal Pembatalan Faktur Pembelian No {}", inv_number))
    .bind(journal_total)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal membuat jurnal pembalik faktur: {}", e))?;

    let acc_ap: Uuid = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '2101' LIMIT 1")
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| "Akun 2101 tidak ditemukan")?;

    let acc_unbilled: Uuid = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '2103' LIMIT 1")
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| "Akun 2103 tidak ditemukan")?;

    // 1. DEBET Hutang Usaha (membatalkan hutang)
    sqlx::query(
        "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, $3, 0, 'Pembalikan Hutang Usaha Supplier', $4)",
    )
    .bind(journal_id)
    .bind(acc_ap)
    .bind(total_amount)
    .bind(inv_outlet_id)
    .execute(&mut *tx).await.map_err(|e| format!("Gagal baris jurnal: {}", e))?;

    // 2. DEBET Potongan Pembelian (jika sebelumnya dikreditkan)
    if discount > Decimal::ZERO {
        let acc_discount: Uuid = match sqlx::query_scalar::<_, Uuid>("SELECT id FROM chart_of_accounts WHERE code = '5102' LIMIT 1")
            .fetch_optional(&mut *tx)
            .await
            .unwrap_or(None) {
                Some(i) => i,
                None => sqlx::query_scalar::<_, Uuid>("SELECT id FROM chart_of_accounts WHERE code = '5101' LIMIT 1").fetch_one(&mut *tx).await.map_err(|_| "Akun diskon tidak ditemukan")?,
            };
        sqlx::query(
            "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, $3, 0, 'Pembalikan Potongan Diskon Pembelian', $4)",
        )
        .bind(journal_id)
        .bind(acc_discount)
        .bind(discount)
        .bind(inv_outlet_id)
        .execute(&mut *tx).await.map_err(|e| format!("Gagal baris jurnal: {}", e))?;
    }

    // 3. KREDIT Unbilled GRN (mengembalikan saldo unbilled)
    sqlx::query(
        "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, 0, $3, 'Pengembalian Saldo Hutang Unbilled GRN', $4)",
    )
    .bind(journal_id)
    .bind(acc_unbilled)
    .bind(subtotal)
    .bind(inv_outlet_id)
    .execute(&mut *tx).await.map_err(|e| format!("Gagal baris jurnal: {}", e))?;

    // 4. KREDIT Pajak Masukan (jika sebelumnya didebetkan)
    if tax > Decimal::ZERO {
        let acc_tax: Uuid = match sqlx::query_scalar::<_, Uuid>("SELECT id FROM chart_of_accounts WHERE code = '1104' LIMIT 1")
            .fetch_optional(&mut *tx)
            .await
            .unwrap_or(None) {
                Some(i) => i,
                None => sqlx::query_scalar::<_, Uuid>("SELECT id FROM chart_of_accounts WHERE code = '2102' LIMIT 1").fetch_one(&mut *tx).await.map_err(|_| "Akun pajak tidak ditemukan")?,
            };
        sqlx::query(
            "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, 0, $3, 'Pembalikan Pajak Masukan (PPN)', $4)",
        )
        .bind(journal_id)
        .bind(acc_tax)
        .bind(tax)
        .bind(inv_outlet_id)
        .execute(&mut *tx).await.map_err(|e| format!("Gagal baris jurnal: {}", e))?;
    }

    tx.commit().await.map_err(|e| format!("Commit error: {}", e))?;

    Ok(format!("Faktur pembelian {} berhasil dibatalkan", inv_number))
}

pub async fn create_purchase_return(
    pool: &PgPool,
    user_id: Uuid,
    req: CreatePurchaseReturnRequest,
) -> Result<serde_json::Value, String> {
    if req.reason.trim().is_empty() {
        return Err("Alasan pengembalian barang wajib diisi".to_string());
    }
    if req.items.is_empty() {
        return Err("Pilih minimal satu barang yang akan diretur".to_string());
    }

    let mut tx = pool.begin().await.map_err(|e| format!("Transaction error: {}", e))?;

    // Validasi periode tutup buku
    crate::services::accounting_service::check_transaction_date_allowed(pool, req.return_date, req.outlet_id).await?;

    let return_id = Uuid::new_v4();
    let return_number = format!("RET-PUR-{}-{}", Utc::now().format("%Y%m%d%H%M%S"), &return_id.to_string()[..4].to_uppercase());

    let refund_method = req.refund_method.unwrap_or_else(|| "DEBT_REDUCTION".to_string());

    let mut subtotal = Decimal::ZERO;
    let mut validated_items = Vec::new();

    for it in &req.items {
        if it.returned_qty <= Decimal::ZERO {
            continue;
        }

        // Cek stok fisik di outlet
        if let Some(wh_id) = req.outlet_id {
            let current_stock: Decimal = sqlx::query_scalar(
                "SELECT COALESCE(quantity, 0) FROM warehouse_stocks WHERE warehouse_id = $1 AND product_id = $2"
            )
            .bind(wh_id)
            .bind(it.product_id)
            .fetch_optional(&mut *tx)
            .await
            .unwrap_or(None)
            .unwrap_or(Decimal::ZERO);

            if it.returned_qty > current_stock {
                let p_name: String = sqlx::query_scalar("SELECT name FROM products WHERE id = $1")
                    .bind(it.product_id)
                    .fetch_optional(&mut *tx)
                    .await
                    .unwrap_or(None)
                    .unwrap_or_else(|| "Produk".to_string());
                return Err(format!(
                    "Stok fisik untuk {} tidak mencukupi untuk diretur (Tersedia: {}, Diretur: {})",
                    p_name, current_stock, it.returned_qty
                ));
            }
        }

        let line_subtotal = it.returned_qty * it.unit_price;
        subtotal += line_subtotal;

        validated_items.push((it.product_id, it.returned_qty, it.unit_price, line_subtotal, it.notes.clone()));
    }

    if subtotal <= Decimal::ZERO || validated_items.is_empty() {
        return Err("Tidak ada kuantitas retur barang yang valid (> 0)".to_string());
    }

    let total_amount = subtotal;

    // 1. Simpan Header purchase_returns
    sqlx::query(
        "INSERT INTO purchase_returns (
            id, return_number, supplier_id, outlet_id, warehouse_id, invoice_id, goods_receipt_id,
            return_date, status, refund_method, reason, subtotal, tax_amount, total_amount, notes,
            created_at, updated_at, created_by, updated_by
        ) VALUES (
            $1, $2, $3, $4, $4, $5, $6, $7, 'COMPLETED', $8, $9, $10, 0, $11, $12,
            CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $13, $13
        )",
    )
    .bind(return_id)
    .bind(&return_number)
    .bind(req.supplier_id)
    .bind(req.outlet_id)
    .bind(req.invoice_id)
    .bind(req.goods_receipt_id)
    .bind(req.return_date)
    .bind(&refund_method)
    .bind(&req.reason)
    .bind(subtotal)
    .bind(total_amount)
    .bind(&req.notes)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal mencatat retur pembelian: {}", e))?;

    // 2. Simpan Item purchase_return_items, kurangi stok, dan catat mutasi
    for (product_id, ret_qty, unit_price, line_subtotal, notes) in validated_items {
        sqlx::query(
            "INSERT INTO purchase_return_items (
                id, purchase_return_id, product_id, returned_qty, unit_price, subtotal, notes,
                created_at, updated_at, created_by, updated_by
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $8, $8
            )",
        )
        .bind(Uuid::new_v4())
        .bind(return_id)
        .bind(product_id)
        .bind(ret_qty)
        .bind(unit_price)
        .bind(line_subtotal)
        .bind(notes)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal mencatat item retur: {}", e))?;

        if let Some(wh_id) = req.outlet_id {
            let current_stock: Decimal = sqlx::query_scalar(
                "SELECT COALESCE(quantity, 0) FROM warehouse_stocks WHERE warehouse_id = $1 AND product_id = $2"
            )
            .bind(wh_id)
            .bind(product_id)
            .fetch_optional(&mut *tx)
            .await
            .unwrap_or(None)
            .unwrap_or(Decimal::ZERO);

            let new_stock = (current_stock - ret_qty).max(Decimal::ZERO);

            sqlx::query(
                "UPDATE warehouse_stocks SET quantity = $1, updated_at = CURRENT_TIMESTAMP 
                 WHERE warehouse_id = $2 AND product_id = $3"
            )
            .bind(new_stock)
            .bind(wh_id)
            .bind(product_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Gagal mengurangi stok retur: {}", e))?;

            sqlx::query(
                "INSERT INTO stock_mutations (
                    warehouse_id, product_id, reference_type, reference_id, qty_in, qty_out,
                    balance, cost_price, notes, created_at, updated_at, created_by, updated_by
                ) VALUES ($1, $2, 'PURCHASE_RETURN', $3, 0, $4, $5, $6, $7, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $8, $8)"
            )
            .bind(wh_id)
            .bind(product_id)
            .bind(return_id)
            .bind(ret_qty)
            .bind(new_stock)
            .bind(unit_price)
            .bind(format!("Retur Pembelian No {} ke Supplier", return_number))
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Gagal mencatat mutasi stok retur: {}", e))?;
        }
    }

    // Jika refund_method == 'DEBT_REDUCTION' dan ada invoice_id: sesuaikan total_amount faktur terkait
    if refund_method == "DEBT_REDUCTION" {
        if let Some(inv_id) = req.invoice_id {
            let _ = sqlx::query(
                "UPDATE purchase_invoices 
                 SET total_amount = GREATEST(0, total_amount - $1), updated_at = CURRENT_TIMESTAMP 
                 WHERE id = $2"
            )
            .bind(total_amount)
            .bind(inv_id)
            .execute(&mut *tx)
            .await;
        }
    }

    // 3. Jurnal Akuntansi Retur Pembelian:
    // DEBIT: Hutang Usaha 2101 (jika Potong Hutang) atau Kas 1101 (jika Pengembalian Kas)
    // KREDIT: Persediaan Barang Dagang 1201
    let journal_id = Uuid::new_v4();
    let jrn_number = format!("JRN-RET-{}-{}", Utc::now().format("%Y%m%d%H%M%S"), &journal_id.to_string()[..4].to_uppercase());

    sqlx::query(
        "INSERT INTO journal_entries (
            id, journal_number, journal_date, reference_type, reference_id, description,
            total_debit, total_credit, is_closing_entry, created_at, updated_at, created_by, updated_by
        ) VALUES (
            $1, $2, $3, 'PURCHASE_RETURN', $4, $5, $6, $6, FALSE, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $7, $7
        )",
    )
    .bind(journal_id)
    .bind(jrn_number)
    .bind(req.return_date)
    .bind(return_id)
    .bind(format!("Nota Retur Pembelian No {} ({})", return_number, req.reason))
    .bind(total_amount)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal membuat header jurnal retur: {}", e))?;

    let debit_account_id: Uuid = if refund_method == "CASH_REFUND" {
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM chart_of_accounts WHERE code = '1101' LIMIT 1")
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| "Akun kas 1101 tidak ditemukan")?
    } else {
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM chart_of_accounts WHERE code = '2101' LIMIT 1")
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| "Akun hutang 2101 tidak ditemukan")?
    };

    let credit_account_id: Uuid = sqlx::query_scalar::<_, Uuid>("SELECT id FROM chart_of_accounts WHERE code = '1201' LIMIT 1")
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| "Akun persediaan 1201 tidak ditemukan")?;

    let debit_memo = if refund_method == "CASH_REFUND" {
        "Penerimaan Pengembalian Kas dari Retur Supplier"
    } else {
        "Pengurangan Hutang Usaha atas Retur Barang"
    };

    sqlx::query(
        "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, $3, 0, $4, $5)",
    )
    .bind(journal_id)
    .bind(debit_account_id)
    .bind(total_amount)
    .bind(debit_memo)
    .bind(req.outlet_id)
    .execute(&mut *tx).await.map_err(|e| format!("Gagal baris jurnal debit: {}", e))?;

    sqlx::query(
        "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, 0, $3, 'Pengurangan Persediaan Barang Dagang Diretur', $4)",
    )
    .bind(journal_id)
    .bind(credit_account_id)
    .bind(total_amount)
    .bind(req.outlet_id)
    .execute(&mut *tx).await.map_err(|e| format!("Gagal baris jurnal kredit: {}", e))?;

    tx.commit().await.map_err(|e| format!("Commit error: {}", e))?;

    Ok(json!({
        "id": return_id,
        "return_number": return_number,
        "total_amount": total_amount,
        "refund_method": refund_method,
        "status": "COMPLETED"
    }))
}

pub async fn list_purchase_returns(
    pool: &PgPool,
    q: Option<PurchaseReturnQueryFilter>,
) -> Result<(Vec<serde_json::Value>, i64), String> {
    let q = q.unwrap_or_default();
    let mut conditions = vec!["1=1".to_string()];

    if let Some(ref s) = q.search {
        if !s.trim().is_empty() {
            let term = format!("%{}%", s.trim().to_lowercase());
            conditions.push(format!(
                "(LOWER(pr.return_number) LIKE '{term}' OR LOWER(s.name) LIKE '{term}' OR LOWER(COALESCE(pi.invoice_number, '')) LIKE '{term}')"
            ));
        }
    }
    if let Some(sid) = q.supplier_id {
        conditions.push(format!("pr.supplier_id = '{sid}'"));
    }
    if let Some(oid) = q.outlet_id {
        conditions.push(format!("pr.outlet_id = '{oid}'"));
    }
    if let Some(invid) = q.invoice_id {
        conditions.push(format!("pr.invoice_id = '{invid}'"));
    }
    if let Some(ref st) = q.status {
        if !st.trim().is_empty() && st != "ALL" {
            conditions.push(format!("pr.status = '{}'", st.trim()));
        }
    }
    if let Some(sd) = q.start_date {
        conditions.push(format!("pr.return_date >= '{sd}'"));
    }
    if let Some(ed) = q.end_date {
        conditions.push(format!("pr.return_date <= '{ed}'"));
    }

    let where_clause = conditions.join(" AND ");

    let total: i64 = sqlx::query_scalar(&format!(
        "SELECT COUNT(*)::bigint FROM purchase_returns pr 
         JOIN suppliers s ON pr.supplier_id = s.id 
         LEFT JOIN purchase_invoices pi ON pr.invoice_id = pi.id 
         WHERE {where_clause}"
    ))
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?;

    let sort_col = match q.sort_by.as_deref() {
        Some("return_number") => "pr.return_number",
        Some("return_date") => "pr.return_date",
        Some("total_amount") => "pr.total_amount",
        Some("status") => "pr.status",
        Some("refund_method") => "pr.refund_method",
        Some("supplier_name") => "s.name",
        Some("outlet_name") => "COALESCE(o.name, '')",
        _ => "pr.return_date DESC, pr.created_at",
    };
    let sort_dir = match q.sort_dir.as_deref() {
        Some("asc") | Some("ASC") => "ASC",
        _ => "DESC",
    };

    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(10).max(1);
    let offset = (page - 1) * page_size;

    let query_str = format!(
        "SELECT pr.id, pr.return_number, pr.supplier_id, pr.outlet_id, pr.invoice_id, pr.goods_receipt_id,
                pr.return_date, pr.status, pr.refund_method, pr.reason, pr.subtotal, pr.tax_amount, pr.total_amount,
                pr.notes, pr.created_at,
                s.name as supplier_name, s.code as supplier_code,
                o.name as outlet_name, o.code as outlet_code,
                pi.invoice_number as invoice_number,
                gr.grn_number as grn_number,
                (SELECT COUNT(*)::int FROM purchase_return_items pri WHERE pri.purchase_return_id = pr.id) as item_count
         FROM purchase_returns pr
         JOIN suppliers s ON pr.supplier_id = s.id
         LEFT JOIN outlets o ON pr.outlet_id = o.id
         LEFT JOIN purchase_invoices pi ON pr.invoice_id = pi.id
         LEFT JOIN goods_receipts gr ON pr.goods_receipt_id = gr.id
         WHERE {where_clause}
         ORDER BY {sort_col} {sort_dir}
         LIMIT {page_size} OFFSET {offset}"
    );

    let rows = sqlx::query(&query_str)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let list: Vec<serde_json::Value> = rows.into_iter().map(|r| {
        json!({
            "id": r.get::<Uuid, _>("id"),
            "return_number": r.get::<String, _>("return_number"),
            "supplier_id": r.get::<Uuid, _>("supplier_id"),
            "supplier_name": r.get::<String, _>("supplier_name"),
            "supplier_code": r.get::<String, _>("supplier_code"),
            "outlet_id": r.get::<Option<Uuid>, _>("outlet_id"),
            "outlet_name": r.get::<Option<String>, _>("outlet_name"),
            "outlet_code": r.get::<Option<String>, _>("outlet_code"),
            "invoice_id": r.get::<Option<Uuid>, _>("invoice_id"),
            "invoice_number": r.get::<Option<String>, _>("invoice_number"),
            "goods_receipt_id": r.get::<Option<Uuid>, _>("goods_receipt_id"),
            "grn_number": r.get::<Option<String>, _>("grn_number"),
            "return_date": r.get::<NaiveDate, _>("return_date").to_string(),
            "status": r.get::<String, _>("status"),
            "refund_method": r.get::<String, _>("refund_method"),
            "reason": r.get::<String, _>("reason"),
            "subtotal": r.get::<Decimal, _>("subtotal"),
            "tax_amount": r.get::<Decimal, _>("tax_amount"),
            "total_amount": r.get::<Decimal, _>("total_amount"),
            "notes": r.get::<Option<String>, _>("notes"),
            "item_count": r.get::<i32, _>("item_count"),
            "created_at": r.get::<DateTime<Utc>, _>("created_at").to_rfc3339(),
        })
    }).collect();

    Ok((list, total))
}

pub async fn get_purchase_return_detail(pool: &PgPool, id: Uuid) -> Result<serde_json::Value, String> {
    let row = sqlx::query(
        "SELECT pr.*, s.name as supplier_name, s.code as supplier_code, s.phone as supplier_phone,
                o.name as outlet_name, o.code as outlet_code,
                pi.invoice_number, gr.grn_number
         FROM purchase_returns pr
         JOIN suppliers s ON pr.supplier_id = s.id
         LEFT JOIN outlets o ON pr.outlet_id = o.id
         LEFT JOIN purchase_invoices pi ON pr.invoice_id = pi.id
         LEFT JOIN goods_receipts gr ON pr.goods_receipt_id = gr.id
         WHERE pr.id = $1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?
    .ok_or_else(|| "Dokumen retur pembelian tidak ditemukan".to_string())?;

    let items_rows = sqlx::query(
        "SELECT pri.*, p.name as product_name, p.sku as product_sku, u.code as unit_code
         FROM purchase_return_items pri
         JOIN products p ON pri.product_id = p.id
         LEFT JOIN units u ON p.base_unit_id = u.id
         WHERE pri.purchase_return_id = $1"
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?;

    let items: Vec<serde_json::Value> = items_rows.into_iter().map(|r| {
        json!({
            "id": r.get::<Uuid, _>("id"),
            "product_id": r.get::<Uuid, _>("product_id"),
            "product_name": r.get::<String, _>("product_name"),
            "product_sku": r.get::<String, _>("product_sku"),
            "unit_code": r.get::<Option<String>, _>("unit_code"),
            "returned_qty": r.get::<Decimal, _>("returned_qty"),
            "unit_price": r.get::<Decimal, _>("unit_price"),
            "subtotal": r.get::<Decimal, _>("subtotal"),
            "notes": r.get::<Option<String>, _>("notes"),
        })
    }).collect();

    Ok(json!({
        "id": row.get::<Uuid, _>("id"),
        "return_number": row.get::<String, _>("return_number"),
        "supplier_id": row.get::<Uuid, _>("supplier_id"),
        "supplier_name": row.get::<String, _>("supplier_name"),
        "supplier_code": row.get::<String, _>("supplier_code"),
        "supplier_phone": row.get::<Option<String>, _>("supplier_phone"),
        "outlet_id": row.get::<Option<Uuid>, _>("outlet_id"),
        "outlet_name": row.get::<Option<String>, _>("outlet_name"),
        "outlet_code": row.get::<Option<String>, _>("outlet_code"),
        "invoice_id": row.get::<Option<Uuid>, _>("invoice_id"),
        "invoice_number": row.get::<Option<String>, _>("invoice_number"),
        "goods_receipt_id": row.get::<Option<Uuid>, _>("goods_receipt_id"),
        "grn_number": row.get::<Option<String>, _>("grn_number"),
        "return_date": row.get::<NaiveDate, _>("return_date").to_string(),
        "status": row.get::<String, _>("status"),
        "refund_method": row.get::<String, _>("refund_method"),
        "reason": row.get::<String, _>("reason"),
        "subtotal": row.get::<Decimal, _>("subtotal"),
        "tax_amount": row.get::<Decimal, _>("tax_amount"),
        "total_amount": row.get::<Decimal, _>("total_amount"),
        "notes": row.get::<Option<String>, _>("notes"),
        "created_at": row.get::<DateTime<Utc>, _>("created_at").to_rfc3339(),
        "items": items,
    }))
}

