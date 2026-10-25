use std::path::Path;
use std::sync::Arc;
use chrono::Utc;
use rust_decimal::Decimal;
use rust_xlsxwriter::{Format, FormatBorder, Workbook};
use serde_json::json;
use sqlx::{PgPool, Row};
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::models::export::ExportJob;

pub async fn create_export_job(
    pool: &PgPool,
    user_id: Option<Uuid>,
    outlet_id: Option<Uuid>,
    module: &str,
    title: &str,
    filter_params: serde_json::Value,
) -> Result<ExportJob, String> {
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let safe_module = module.replace(' ', "_").to_lowercase();
    let filename = format!("{}_{}.xlsx", safe_module, timestamp);

    let row = sqlx::query_as::<_, ExportJob>(
        "INSERT INTO export_jobs (
            user_id, outlet_id, module, title, filter_params, filename, status
        ) VALUES ($1, $2, $3, $4, $5, $6, 'QUEUED')
        RETURNING *"
    )
    .bind(user_id)
    .bind(outlet_id)
    .bind(module)
    .bind(title)
    .bind(filter_params)
    .bind(filename)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Gagal membuat antrean ekspor di database: {}", e))?;

    Ok(row)
}

pub fn spawn_export_worker(
    pool: PgPool,
    job_id: Uuid,
    tx: Option<Arc<broadcast::Sender<String>>>,
) {
    tokio::spawn(async move {
        if let Err(e) = execute_export_job(&pool, job_id, tx).await {
            tracing::error!("Pekerjaan ekspor {} gagal: {}", job_id, e);
            let _ = sqlx::query(
                "UPDATE export_jobs 
                 SET status = 'FAILED', error_message = $1, completed_at = CURRENT_TIMESTAMP 
                 WHERE id = $2"
            )
            .bind(e)
            .bind(job_id)
            .execute(&pool)
            .await;
        }
    });
}

async fn execute_export_job(
    pool: &PgPool,
    job_id: Uuid,
    tx: Option<Arc<broadcast::Sender<String>>>,
) -> Result<(), String> {
    // 1. Ambil data job
    let job = sqlx::query_as::<_, ExportJob>("SELECT * FROM export_jobs WHERE id = $1")
        .bind(job_id)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Job tidak ditemukan: {}", e))?;

    // Tandai sedang diproses
    sqlx::query("UPDATE export_jobs SET status = 'PROCESSING', started_at = CURRENT_TIMESTAMP WHERE id = $1")
        .bind(job_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    // Siapkan folder penyimpanan file
    let exports_dir = "uploads/exports";
    if !Path::new(exports_dir).exists() {
        std::fs::create_dir_all(exports_dir).map_err(|e| format!("Gagal membuat folder exports: {}", e))?;
    }
    let file_path = format!("{}/{}.xlsx", exports_dir, job_id);

    let mut workbook = Workbook::new();
    let header_format = Format::new()
        .set_bold()
        .set_border(FormatBorder::Thin)
        .set_background_color("1E293B")
        .set_font_color("FFFFFF");

    let mut total_exported_rows: usize = 0;

    match job.module.as_str() {
        "sales" => {
            total_exported_rows = export_sales_streaming(pool, &job, &mut workbook, &header_format).await?;
        }
        "purchases_orders" | "purchases" => {
            total_exported_rows = export_purchase_orders_streaming(pool, &job, &mut workbook, &header_format).await?;
        }
        "purchases_receipts" => {
            total_exported_rows = export_goods_receipts_streaming(pool, &job, &mut workbook, &header_format).await?;
        }
        "purchases_invoices" => {
            total_exported_rows = export_purchase_invoices_streaming(pool, &job, &mut workbook, &header_format).await?;
        }
        "purchases_returns" => {
            total_exported_rows = export_purchase_returns_streaming(pool, &job, &mut workbook, &header_format).await?;
        }
        "expenses" => {
            total_exported_rows = export_expenses_streaming(pool, &job, &mut workbook, &header_format).await?;
        }
        "products" => {
            total_exported_rows = export_products_streaming(pool, &job, &mut workbook, &header_format).await?;
        }
        _ => {
            return Err(format!("Modul ekspor '{}' tidak dikenali.", job.module));
        }
    }

    // Simpan workbook ke disk
    workbook
        .save(&file_path)
        .map_err(|e| format!("Gagal menyimpan file Excel: {}", e))?;

    let file_size = std::fs::metadata(&file_path)
        .map(|m| m.len() as i64)
        .unwrap_or(0);

    // Update status COMPLETED di database
    sqlx::query(
        "UPDATE export_jobs 
         SET status = 'COMPLETED', file_path = $1, file_size_bytes = $2, 
             total_rows = $3, processed_rows = $3, completed_at = CURRENT_TIMESTAMP 
         WHERE id = $4"
    )
    .bind(&file_path)
    .bind(file_size)
    .bind(total_exported_rows as i32)
    .bind(job_id)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    // Broadcast WebSocket event ke pengguna yang sedang online
    if let Some(broadcast_tx) = tx {
        let broadcast = json!({
            "type": "EXPORT_JOB_COMPLETED",
            "job_id": job.id,
            "user_id": job.user_id,
            "module": job.module,
            "title": job.title,
            "filename": job.filename,
            "total_rows": total_exported_rows,
            "download_url": format!("/api/v1/exports/{}/download", job.id),
            "timestamp": Utc::now().timestamp_millis()
        });
        let _ = broadcast_tx.send(broadcast.to_string());
    }

    Ok(())
}

// ============================================================================
// 1. STREAMING SALES EXPORT
// ============================================================================
async fn export_sales_streaming(
    pool: &PgPool,
    job: &ExportJob,
    wb: &mut Workbook,
    header_fmt: &Format,
) -> Result<usize, String> {
    let ws = wb.add_worksheet();
    ws.set_name("Penjualan").map_err(|e| e.to_string())?;

    let headers = [
        "No", "No. Faktur", "Cabang Outlet", "Waktu Transaksi", "Kasir",
        "Pelanggan", "Metode Bayar", "Total Item", "Subtotal (Rp)", "Diskon (Rp)",
        "Pajak PPN (Rp)", "Total Akhir (Rp)", "Status", "Alasan Void",
    ];

    for (col_idx, h) in headers.iter().enumerate() {
        ws.write_with_format(0, col_idx as u16, *h, header_fmt).map_err(|e| e.to_string())?;
    }

    let filter = &job.filter_params;
    let mut where_clause = " WHERE 1=1".to_string();

    if let Some(oid) = filter.get("outlet_id").and_then(|v| v.as_str()) {
        if let Ok(u) = Uuid::parse_str(oid) {
            where_clause.push_str(&format!(" AND s.outlet_id = '{}'", u));
        }
    }
    if let Some(st) = filter.get("status").and_then(|v| v.as_str()) {
        if st != "ALL" {
            where_clause.push_str(&format!(" AND s.status = '{}'", st.replace('\'', "''")));
        }
    }
    if let Some(sc) = filter.get("search").and_then(|v| v.as_str()) {
        if !sc.trim().is_empty() {
            let clean = sc.trim().replace('\'', "''");
            where_clause.push_str(&format!(
                " AND (s.invoice_number ILIKE '%{}%' OR u.full_name ILIKE '%{}%' OR c.name ILIKE '%{}%')",
                clean, clean, clean
            ));
        }
    }

    let sort_col = match filter.get("sort_by").and_then(|v| v.as_str()) {
        Some("invoice_number") => "s.invoice_number",
        Some("total_amount") => "s.total_amount",
        _ => "s.created_at",
    };
    let sort_dir = match filter.get("sort_dir").and_then(|v| v.as_str()) {
        Some("asc") | Some("ASC") => "ASC",
        _ => "DESC",
    };

    let mut current_row: u32 = 1;
    let batch_size: i64 = 5000;
    let mut offset: i64 = 0;

    loop {
        let query = format!(
            "SELECT s.invoice_number, s.created_at,
                    COALESCE((SELECT string_agg(pm.name, ', ') FROM sale_payments sp JOIN payment_methods pm ON sp.payment_method_id = pm.id WHERE sp.sale_id = s.id), 'CASH') as payment_method,
                    COALESCE((SELECT SUM(quantity) FROM sale_items WHERE sale_id = s.id AND is_voided = FALSE), 0) as total_items,
                    s.subtotal, s.discount_amount, s.tax_amount, s.total_amount, s.status, s.notes,
                    COALESCE(o.name, '-') as outlet_name,
                    COALESCE(u.full_name, '-') as cashier_name,
                    COALESCE(c.name, 'Pelanggan Umum') as customer_name
             FROM sales s
             LEFT JOIN outlets o ON s.outlet_id = o.id
             LEFT JOIN cashier_shifts cs ON s.shift_id = cs.id
             LEFT JOIN users u ON cs.user_id = u.id
             LEFT JOIN customers c ON s.customer_id = c.id
             {}
             ORDER BY {} {}
             LIMIT {} OFFSET {}",
            where_clause, sort_col, sort_dir, batch_size, offset
        );

        let rows = sqlx::query(&query).fetch_all(pool).await.map_err(|e| e.to_string())?;
        if rows.is_empty() {
            break;
        }

        for r in &rows {
            let inv_num: String = r.get("invoice_number");
            let outlet: String = r.get("outlet_name");
            let created_at: chrono::DateTime<Utc> = r.get("created_at");
            let cashier: String = r.get("cashier_name");
            let customer: String = r.get("customer_name");
            let payment: String = r.get("payment_method");
            let total_items: Decimal = r.get("total_items");
            let subtotal: Decimal = r.get("subtotal");
            let discount: Decimal = r.get("discount_amount");
            let tax: Decimal = r.get("tax_amount");
            let total_amount: Decimal = r.get("total_amount");
            let status: String = r.get("status");
            let notes: Option<String> = r.get("notes");

            ws.write(current_row, 0, current_row as f64).map_err(|e| e.to_string())?;
            ws.write(current_row, 1, inv_num).map_err(|e| e.to_string())?;
            ws.write(current_row, 2, outlet).map_err(|e| e.to_string())?;
            ws.write(current_row, 3, created_at.format("%d/%m/%Y %H:%M:%S").to_string()).map_err(|e| e.to_string())?;
            ws.write(current_row, 4, cashier).map_err(|e| e.to_string())?;
            ws.write(current_row, 5, customer).map_err(|e| e.to_string())?;
            ws.write(current_row, 6, payment).map_err(|e| e.to_string())?;
            ws.write(current_row, 7, total_items.to_string().parse::<f64>().unwrap_or(0.0)).map_err(|e| e.to_string())?;
            ws.write(current_row, 8, subtotal.to_string().parse::<f64>().unwrap_or(0.0)).map_err(|e| e.to_string())?;
            ws.write(current_row, 9, discount.to_string().parse::<f64>().unwrap_or(0.0)).map_err(|e| e.to_string())?;
            ws.write(current_row, 10, tax.to_string().parse::<f64>().unwrap_or(0.0)).map_err(|e| e.to_string())?;
            ws.write(current_row, 11, total_amount.to_string().parse::<f64>().unwrap_or(0.0)).map_err(|e| e.to_string())?;
            ws.write(current_row, 12, status).map_err(|e| e.to_string())?;
            ws.write(current_row, 13, notes.unwrap_or_else(|| "-".to_string())).map_err(|e| e.to_string())?;

            current_row += 1;
        }

        offset += batch_size;
    }

    Ok((current_row - 1) as usize)
}

// ============================================================================
// 2. STREAMING PURCHASE ORDERS EXPORT
// ============================================================================
async fn export_purchase_orders_streaming(
    pool: &PgPool,
    job: &ExportJob,
    wb: &mut Workbook,
    header_fmt: &Format,
) -> Result<usize, String> {
    let ws = wb.add_worksheet();
    ws.set_name("Purchase Orders").map_err(|e| e.to_string())?;

    let headers = [
        "No", "No. PO", "Tanggal PO", "Pemasok / Supplier", "Cabang Pemesan",
        "Estimasi Tiba", "Total Item", "Total Nilai (Rp)", "Status PO", "Catatan",
    ];

    for (col_idx, h) in headers.iter().enumerate() {
        ws.write_with_format(0, col_idx as u16, *h, header_fmt).map_err(|e| e.to_string())?;
    }

    let filter = &job.filter_params;
    let mut where_clause = " WHERE 1=1".to_string();

    if let Some(oid) = filter.get("outlet_id").and_then(|v| v.as_str()) {
        if let Ok(u) = Uuid::parse_str(oid) {
            where_clause.push_str(&format!(" AND po.outlet_id = '{}'", u));
        }
    }
    if let Some(st) = filter.get("status").and_then(|v| v.as_str()) {
        if st != "ALL" {
            where_clause.push_str(&format!(" AND po.status = '{}'", st.replace('\'', "''")));
        }
    }
    if let Some(sc) = filter.get("search").and_then(|v| v.as_str()) {
        if !sc.trim().is_empty() {
            let clean = sc.trim().replace('\'', "''");
            where_clause.push_str(&format!(
                " AND (po.po_number ILIKE '%{}%' OR sup.name ILIKE '%{}%' OR po.notes ILIKE '%{}%')",
                clean, clean, clean
            ));
        }
    }

    let sort_col = match filter.get("sort_by").and_then(|v| v.as_str()) {
        Some("po_number") => "po.po_number",
        Some("total_amount") => "po.total_amount",
        _ => "po.created_at",
    };
    let sort_dir = match filter.get("sort_dir").and_then(|v| v.as_str()) {
        Some("asc") | Some("ASC") => "ASC",
        _ => "DESC",
    };

    let mut current_row: u32 = 1;
    let batch_size: i64 = 5000;
    let mut offset: i64 = 0;

    loop {
        let query = format!(
            "SELECT po.po_number, po.order_date, po.expected_delivery,
                    COALESCE((SELECT SUM(ordered_qty) FROM purchase_order_items poi WHERE poi.po_id = po.id), 0) as total_items,
                    po.total_amount, po.status, po.notes,
                    COALESCE(sup.name, '-') as supplier_name,
                    COALESCE(o.name, '-') as outlet_name
             FROM purchase_orders po
             LEFT JOIN suppliers sup ON po.supplier_id = sup.id
             LEFT JOIN outlets o ON po.outlet_id = o.id
             {}
             ORDER BY {} {}
             LIMIT {} OFFSET {}",
            where_clause, sort_col, sort_dir, batch_size, offset
        );

        let rows = sqlx::query(&query).fetch_all(pool).await.map_err(|e| e.to_string())?;
        if rows.is_empty() {
            break;
        }

        for r in &rows {
            let po_num: String = r.get("po_number");
            let order_date: chrono::NaiveDate = r.get("order_date");
            let supplier: String = r.get("supplier_name");
            let outlet: String = r.get("outlet_name");
            let exp_delivery: Option<chrono::NaiveDate> = r.get("expected_delivery");
            let total_items: Decimal = r.get("total_items");
            let total_amount: Decimal = r.get("total_amount");
            let status: String = r.get("status");
            let notes: Option<String> = r.get("notes");

            ws.write(current_row, 0, current_row as f64).map_err(|e| e.to_string())?;
            ws.write(current_row, 1, po_num).map_err(|e| e.to_string())?;
            ws.write(current_row, 2, order_date.format("%d/%m/%Y").to_string()).map_err(|e| e.to_string())?;
            ws.write(current_row, 3, supplier).map_err(|e| e.to_string())?;
            ws.write(current_row, 4, outlet).map_err(|e| e.to_string())?;
            ws.write(current_row, 5, exp_delivery.map(|d| d.format("%d/%m/%Y").to_string()).unwrap_or_else(|| "-".to_string())).map_err(|e| e.to_string())?;
            ws.write(current_row, 6, total_items.to_string().parse::<f64>().unwrap_or(0.0)).map_err(|e| e.to_string())?;
            ws.write(current_row, 7, total_amount.to_string().parse::<f64>().unwrap_or(0.0)).map_err(|e| e.to_string())?;
            ws.write(current_row, 8, status).map_err(|e| e.to_string())?;
            ws.write(current_row, 9, notes.unwrap_or_else(|| "-".to_string())).map_err(|e| e.to_string())?;

            current_row += 1;
        }

        offset += batch_size;
    }

    Ok((current_row - 1) as usize)
}

// ============================================================================
// 3. STREAMING GOODS RECEIPTS EXPORT
// ============================================================================
async fn export_goods_receipts_streaming(
    pool: &PgPool,
    job: &ExportJob,
    wb: &mut Workbook,
    header_fmt: &Format,
) -> Result<usize, String> {
    let ws = wb.add_worksheet();
    ws.set_name("Penerimaan Barang").map_err(|e| e.to_string())?;

    let headers = [
        "No", "No. GRN", "No. Surat Jalan (DO)", "No. Referensi PO", "Tanggal Terima",
        "Pemasok / Supplier", "Gudang / Outlet", "Penerima / Petugas", "Total Diterima",
        "Total Ditolak", "Status GRN", "Catatan",
    ];

    for (col_idx, h) in headers.iter().enumerate() {
        ws.write_with_format(0, col_idx as u16, *h, header_fmt).map_err(|e| e.to_string())?;
    }

    let filter = &job.filter_params;
    let mut where_clause = " WHERE 1=1".to_string();

    if let Some(oid) = filter.get("outlet_id").and_then(|v| v.as_str()) {
        if let Ok(u) = Uuid::parse_str(oid) {
            where_clause.push_str(&format!(" AND gr.outlet_id = '{}'", u));
        }
    }
    if let Some(st) = filter.get("status").and_then(|v| v.as_str()) {
        if st != "ALL" {
            where_clause.push_str(&format!(" AND gr.status = '{}'", st.replace('\'', "''")));
        }
    }
    if let Some(sc) = filter.get("search").and_then(|v| v.as_str()) {
        if !sc.trim().is_empty() {
            let clean = sc.trim().replace('\'', "''");
            where_clause.push_str(&format!(
                " AND (gr.grn_number ILIKE '%{}%' OR gr.delivery_order_number ILIKE '%{}%' OR po.po_number ILIKE '%{}%' OR sup.name ILIKE '%{}%')",
                clean, clean, clean, clean
            ));
        }
    }

    let mut current_row: u32 = 1;
    let batch_size: i64 = 5000;
    let mut offset: i64 = 0;

    loop {
        let query = format!(
            "SELECT gr.grn_number, gr.delivery_order_number, gr.received_date, gr.status, gr.notes,
                    COALESCE(po.po_number, '-') as po_number,
                    COALESCE(sup.name, '-') as supplier_name,
                    COALESCE(o.name, '-') as outlet_name,
                    COALESCE(u.full_name, '-') as received_by_name,
                    COALESCE((SELECT SUM(gri.received_quantity) FROM goods_receipt_items gri WHERE gri.goods_receipt_id = gr.id), 0) as total_received_qty,
                    COALESCE((SELECT SUM(gri.rejected_quantity) FROM goods_receipt_items gri WHERE gri.goods_receipt_id = gr.id), 0) as total_rejected_qty
             FROM goods_receipts gr
             LEFT JOIN purchase_orders po ON gr.purchase_order_id = po.id
             LEFT JOIN suppliers sup ON gr.supplier_id = sup.id
             LEFT JOIN outlets o ON gr.outlet_id = o.id
             LEFT JOIN users u ON gr.received_by = u.id
             {}
             ORDER BY gr.created_at DESC
             LIMIT {} OFFSET {}",
            where_clause, batch_size, offset
        );

        let rows = sqlx::query(&query).fetch_all(pool).await.map_err(|e| e.to_string())?;
        if rows.is_empty() {
            break;
        }

        for r in &rows {
            let grn_num: String = r.get("grn_number");
            let do_num: Option<String> = r.get("delivery_order_number");
            let po_num: String = r.get("po_number");
            let rec_date: chrono::DateTime<Utc> = r.get("received_date");
            let sup_name: String = r.get("supplier_name");
            let out_name: String = r.get("outlet_name");
            let staff: String = r.get("received_by_name");
            let rec_qty: Decimal = r.get("total_received_qty");
            let rej_qty: Decimal = r.get("total_rejected_qty");
            let status: String = r.get("status");
            let notes: Option<String> = r.get("notes");

            ws.write(current_row, 0, current_row as f64).map_err(|e| e.to_string())?;
            ws.write(current_row, 1, grn_num).map_err(|e| e.to_string())?;
            ws.write(current_row, 2, do_num.unwrap_or_else(|| "-".to_string())).map_err(|e| e.to_string())?;
            ws.write(current_row, 3, po_num).map_err(|e| e.to_string())?;
            ws.write(current_row, 4, rec_date.format("%d/%m/%Y").to_string()).map_err(|e| e.to_string())?;
            ws.write(current_row, 5, sup_name).map_err(|e| e.to_string())?;
            ws.write(current_row, 6, out_name).map_err(|e| e.to_string())?;
            ws.write(current_row, 7, staff).map_err(|e| e.to_string())?;
            ws.write(current_row, 8, rec_qty.to_string().parse::<f64>().unwrap_or(0.0)).map_err(|e| e.to_string())?;
            ws.write(current_row, 9, rej_qty.to_string().parse::<f64>().unwrap_or(0.0)).map_err(|e| e.to_string())?;
            ws.write(current_row, 10, status).map_err(|e| e.to_string())?;
            ws.write(current_row, 11, notes.unwrap_or_else(|| "-".to_string())).map_err(|e| e.to_string())?;

            current_row += 1;
        }

        offset += batch_size;
    }

    Ok((current_row - 1) as usize)
}

// ============================================================================
// 4. STREAMING PURCHASE INVOICES EXPORT
// ============================================================================
async fn export_purchase_invoices_streaming(
    pool: &PgPool,
    job: &ExportJob,
    wb: &mut Workbook,
    header_fmt: &Format,
) -> Result<usize, String> {
    let ws = wb.add_worksheet();
    ws.set_name("Faktur Pembelian").map_err(|e| e.to_string())?;

    let headers = [
        "No", "No. Faktur Internal", "No. Faktur Supplier", "No. GRN Referensi",
        "Tanggal Faktur", "Jatuh Tempo", "Pemasok / Supplier", "Cabang Outlet",
        "Subtotal (Rp)", "Diskon (Rp)", "Pajak (Rp)", "Total Tagihan (Rp)",
        "Status Pembayaran", "Catatan",
    ];

    for (col_idx, h) in headers.iter().enumerate() {
        ws.write_with_format(0, col_idx as u16, *h, header_fmt).map_err(|e| e.to_string())?;
    }

    let filter = &job.filter_params;
    let mut where_clause = " WHERE 1=1".to_string();

    if let Some(oid) = filter.get("outlet_id").and_then(|v| v.as_str()) {
        if let Ok(u) = Uuid::parse_str(oid) {
            where_clause.push_str(&format!(" AND p.outlet_id = '{}'", u));
        }
    }
    if let Some(st) = filter.get("status").and_then(|v| v.as_str()) {
        where_clause.push_str(&format!(" AND p.payment_status = '{}'", st.replace('\'', "''")));
    }
    if let Some(sc) = filter.get("search").and_then(|v| v.as_str()) {
        if !sc.trim().is_empty() {
            let clean = sc.trim().replace('\'', "''");
            where_clause.push_str(&format!(
                " AND (p.invoice_number ILIKE '%{}%' OR p.supplier_invoice_number ILIKE '%{}%' OR sup.name ILIKE '%{}%')",
                clean, clean, clean
            ));
        }
    }

    let mut current_row: u32 = 1;
    let batch_size: i64 = 5000;
    let mut offset: i64 = 0;

    loop {
        let query = format!(
            "SELECT p.invoice_number, p.supplier_invoice_number, p.invoice_date, p.due_date,
                    p.subtotal, p.discount_amount, p.tax_amount, p.total_amount, p.payment_status, p.notes,
                    COALESCE(gr.grn_number, '-') as grn_number,
                    COALESCE(sup.name, '-') as supplier_name,
                    COALESCE(o.name, '-') as outlet_name
             FROM purchases p
             LEFT JOIN goods_receipts gr ON p.goods_receipt_id = gr.id
             LEFT JOIN suppliers sup ON p.supplier_id = sup.id
             LEFT JOIN outlets o ON p.outlet_id = o.id
             {}
             ORDER BY p.created_at DESC
             LIMIT {} OFFSET {}",
            where_clause, batch_size, offset
        );

        let rows = sqlx::query(&query).fetch_all(pool).await.map_err(|e| e.to_string())?;
        if rows.is_empty() {
            break;
        }

        for r in &rows {
            let inv_num: String = r.get("invoice_number");
            let sup_inv: Option<String> = r.get("supplier_invoice_number");
            let grn_num: String = r.get("grn_number");
            let inv_date: chrono::DateTime<Utc> = r.get("invoice_date");
            let due_date: Option<chrono::DateTime<Utc>> = r.get("due_date");
            let sup_name: String = r.get("supplier_name");
            let out_name: String = r.get("outlet_name");
            let subtotal: Decimal = r.get("subtotal");
            let discount: Decimal = r.get("discount_amount");
            let tax: Decimal = r.get("tax_amount");
            let total_amount: Decimal = r.get("total_amount");
            let pay_status: String = r.get("payment_status");
            let notes: Option<String> = r.get("notes");

            ws.write(current_row, 0, current_row as f64).map_err(|e| e.to_string())?;
            ws.write(current_row, 1, inv_num).map_err(|e| e.to_string())?;
            ws.write(current_row, 2, sup_inv.unwrap_or_else(|| "-".to_string())).map_err(|e| e.to_string())?;
            ws.write(current_row, 3, grn_num).map_err(|e| e.to_string())?;
            ws.write(current_row, 4, inv_date.format("%d/%m/%Y").to_string()).map_err(|e| e.to_string())?;
            ws.write(current_row, 5, due_date.map(|d| d.format("%d/%m/%Y").to_string()).unwrap_or_else(|| "-".to_string())).map_err(|e| e.to_string())?;
            ws.write(current_row, 6, sup_name).map_err(|e| e.to_string())?;
            ws.write(current_row, 7, out_name).map_err(|e| e.to_string())?;
            ws.write(current_row, 8, subtotal.to_string().parse::<f64>().unwrap_or(0.0)).map_err(|e| e.to_string())?;
            ws.write(current_row, 9, discount.to_string().parse::<f64>().unwrap_or(0.0)).map_err(|e| e.to_string())?;
            ws.write(current_row, 10, tax.to_string().parse::<f64>().unwrap_or(0.0)).map_err(|e| e.to_string())?;
            ws.write(current_row, 11, total_amount.to_string().parse::<f64>().unwrap_or(0.0)).map_err(|e| e.to_string())?;
            ws.write(current_row, 12, pay_status).map_err(|e| e.to_string())?;
            ws.write(current_row, 13, notes.unwrap_or_else(|| "-".to_string())).map_err(|e| e.to_string())?;

            current_row += 1;
        }

        offset += batch_size;
    }

    Ok((current_row - 1) as usize)
}

// ============================================================================
// 5. STREAMING PURCHASE RETURNS EXPORT
// ============================================================================
async fn export_purchase_returns_streaming(
    pool: &PgPool,
    job: &ExportJob,
    wb: &mut Workbook,
    header_fmt: &Format,
) -> Result<usize, String> {
    let ws = wb.add_worksheet();
    ws.set_name("Retur Pembelian").map_err(|e| e.to_string())?;

    let headers = [
        "No", "No. Retur", "No. Faktur", "Tanggal Retur", "Pemasok / Supplier",
        "Cabang Outlet", "Alasan Retur", "Metode Kompensasi", "Total Nilai Retur (Rp)",
        "Status Retur", "Catatan",
    ];

    for (col_idx, h) in headers.iter().enumerate() {
        ws.write_with_format(0, col_idx as u16, *h, header_fmt).map_err(|e| e.to_string())?;
    }

    let filter = &job.filter_params;
    let mut where_clause = " WHERE 1=1".to_string();

    if let Some(oid) = filter.get("outlet_id").and_then(|v| v.as_str()) {
        if let Ok(u) = Uuid::parse_str(oid) {
            where_clause.push_str(&format!(" AND pr.outlet_id = '{}'", u));
        }
    }
    if let Some(st) = filter.get("status").and_then(|v| v.as_str()) {
        if st != "ALL" {
            where_clause.push_str(&format!(" AND pr.status = '{}'", st.replace('\'', "''")));
        }
    }

    let mut current_row: u32 = 1;
    let batch_size: i64 = 5000;
    let mut offset: i64 = 0;

    loop {
        let query = format!(
            "SELECT pr.return_number, pr.return_date, pr.reason, pr.refund_method, pr.total_amount, pr.status, pr.notes,
                    COALESCE(p.invoice_number, '-') as invoice_number,
                    COALESCE(sup.name, '-') as supplier_name,
                    COALESCE(o.name, '-') as outlet_name
             FROM purchase_returns pr
             LEFT JOIN purchases p ON pr.purchase_id = p.id
             LEFT JOIN suppliers sup ON pr.supplier_id = sup.id
             LEFT JOIN outlets o ON pr.outlet_id = o.id
             {}
             ORDER BY pr.created_at DESC
             LIMIT {} OFFSET {}",
            where_clause, batch_size, offset
        );

        let rows = sqlx::query(&query).fetch_all(pool).await.map_err(|e| e.to_string())?;
        if rows.is_empty() {
            break;
        }

        for r in &rows {
            let ret_num: String = r.get("return_number");
            let inv_num: String = r.get("invoice_number");
            let ret_date: chrono::DateTime<Utc> = r.get("return_date");
            let sup_name: String = r.get("supplier_name");
            let out_name: String = r.get("outlet_name");
            let reason: Option<String> = r.get("reason");
            let refund_method: String = r.get("refund_method");
            let total_amount: Decimal = r.get("total_amount");
            let status: String = r.get("status");
            let notes: Option<String> = r.get("notes");

            ws.write(current_row, 0, current_row as f64).map_err(|e| e.to_string())?;
            ws.write(current_row, 1, ret_num).map_err(|e| e.to_string())?;
            ws.write(current_row, 2, inv_num).map_err(|e| e.to_string())?;
            ws.write(current_row, 3, ret_date.format("%d/%m/%Y").to_string()).map_err(|e| e.to_string())?;
            ws.write(current_row, 4, sup_name).map_err(|e| e.to_string())?;
            ws.write(current_row, 5, out_name).map_err(|e| e.to_string())?;
            ws.write(current_row, 6, reason.unwrap_or_else(|| "-".to_string())).map_err(|e| e.to_string())?;
            ws.write(current_row, 7, refund_method).map_err(|e| e.to_string())?;
            ws.write(current_row, 8, total_amount.to_string().parse::<f64>().unwrap_or(0.0)).map_err(|e| e.to_string())?;
            ws.write(current_row, 9, status).map_err(|e| e.to_string())?;
            ws.write(current_row, 10, notes.unwrap_or_else(|| "-".to_string())).map_err(|e| e.to_string())?;

            current_row += 1;
        }

        offset += batch_size;
    }

    Ok((current_row - 1) as usize)
}

// ============================================================================
// 6. STREAMING EXPENSES EXPORT
// ============================================================================
async fn export_expenses_streaming(
    pool: &PgPool,
    job: &ExportJob,
    wb: &mut Workbook,
    header_fmt: &Format,
) -> Result<usize, String> {
    let ws = wb.add_worksheet();
    ws.set_name("Pengeluaran").map_err(|e| e.to_string())?;

    let headers = [
        "No", "No. Transaksi", "Tanggal", "Gerai / Lokasi", "Kategori",
        "Kode Kategori", "Keterangan / Penerima", "No. Referensi / Bukti",
        "Sumber Pembayaran", "Akun Beban Akuntansi", "Nominal (Rp)",
        "Shift Kasir", "Dicatat Oleh", "Status", "Alasan Pembatalan",
    ];

    for (col_idx, h) in headers.iter().enumerate() {
        ws.write_with_format(0, col_idx as u16, *h, header_fmt).map_err(|e| e.to_string())?;
    }

    let filter = &job.filter_params;
    let mut where_clause = " WHERE 1=1".to_string();

    if let Some(oid) = filter.get("outlet_id").and_then(|v| v.as_str()) {
        if let Ok(u) = Uuid::parse_str(oid) {
            where_clause.push_str(&format!(" AND e.outlet_id = '{}'", u));
        }
    }
    if let Some(cid) = filter.get("category_id").and_then(|v| v.as_str()) {
        if let Ok(u) = Uuid::parse_str(cid) {
            where_clause.push_str(&format!(" AND e.category_id = '{}'", u));
        }
    }
    if let Some(st) = filter.get("status").and_then(|v| v.as_str()) {
        where_clause.push_str(&format!(" AND e.status = '{}'", st.replace('\'', "''")));
    }
    if let Some(sdate) = filter.get("start_date").and_then(|v| v.as_str()) {
        where_clause.push_str(&format!(" AND e.transaction_date >= '{}'", sdate.replace('\'', "''")));
    }
    if let Some(edate) = filter.get("end_date").and_then(|v| v.as_str()) {
        where_clause.push_str(&format!(" AND e.transaction_date <= '{}'", edate.replace('\'', "''")));
    }
    if let Some(sc) = filter.get("search").and_then(|v| v.as_str()) {
        if !sc.trim().is_empty() {
            let clean = sc.trim().replace('\'', "''");
            where_clause.push_str(&format!(
                " AND (e.transaction_no ILIKE '%{}%' OR e.recipient_name ILIKE '%{}%' OR e.notes ILIKE '%{}%')",
                clean, clean, clean
            ));
        }
    }

    let mut current_row: u32 = 1;
    let batch_size: i64 = 5000;
    let mut offset: i64 = 0;

    loop {
        let query = format!(
            "SELECT e.transaction_no as expense_number, e.transaction_date, e.recipient_name, e.notes, e.reference_no,
                    e.amount, e.status, e.cancellation_reason, e.shift_id,
                    COALESCE(o.name, '-') as outlet_name,
                    COALESCE(c.name, '-') as category_name,
                    COALESCE(c.code, '-') as category_code,
                    COALESCE(pa.name, 'Kas Kasir') as payment_account_name,
                    COALESCE(ea.name, '-') as expense_account_name,
                    COALESCE(u.full_name, '-') as creator_name
             FROM expense_transactions e
             LEFT JOIN outlets o ON e.outlet_id = o.id
             LEFT JOIN expense_categories c ON e.category_id = c.id
             LEFT JOIN chart_of_accounts pa ON e.payment_account_id = pa.id
             LEFT JOIN chart_of_accounts ea ON e.expense_account_id = ea.id
             LEFT JOIN users u ON e.created_by = u.id
             {}
             ORDER BY e.transaction_date DESC, e.created_at DESC
             LIMIT {} OFFSET {}",
            where_clause, batch_size, offset
        );

        let rows = sqlx::query(&query).fetch_all(pool).await.map_err(|e| e.to_string())?;
        if rows.is_empty() {
            break;
        }

        for r in &rows {
            let exp_num: String = r.get("expense_number");
            let t_date: chrono::NaiveDate = r.get("transaction_date");
            let outlet: String = r.get("outlet_name");
            let cat_name: String = r.get("category_name");
            let cat_code: String = r.get("category_code");
            let recipient: Option<String> = r.get("recipient_name");
            let notes: Option<String> = r.get("notes");
            let ref_no: Option<String> = r.get("reference_no");
            let pay_acc: String = r.get("payment_account_name");
            let exp_acc: String = r.get("expense_account_name");
            let amount: Decimal = r.get("amount");
            let shift_id: Option<Uuid> = r.get("shift_id");
            let creator: String = r.get("creator_name");
            let status: String = r.get("status");
            let cancel_reason: Option<String> = r.get("cancellation_reason");

            let desc_cell = if let Some(rec) = recipient {
                format!("{} - {}", rec, notes.unwrap_or_default())
            } else {
                notes.unwrap_or_else(|| "-".to_string())
            };

            ws.write(current_row, 0, current_row as f64).map_err(|e| e.to_string())?;
            ws.write(current_row, 1, exp_num).map_err(|e| e.to_string())?;
            ws.write(current_row, 2, t_date.format("%d/%m/%Y").to_string()).map_err(|e| e.to_string())?;
            ws.write(current_row, 3, outlet).map_err(|e| e.to_string())?;
            ws.write(current_row, 4, cat_name).map_err(|e| e.to_string())?;
            ws.write(current_row, 5, cat_code).map_err(|e| e.to_string())?;
            ws.write(current_row, 6, desc_cell).map_err(|e| e.to_string())?;
            ws.write(current_row, 7, ref_no.unwrap_or_else(|| "-".to_string())).map_err(|e| e.to_string())?;
            ws.write(current_row, 8, pay_acc).map_err(|e| e.to_string())?;
            ws.write(current_row, 9, exp_acc).map_err(|e| e.to_string())?;
            ws.write(current_row, 10, amount.to_string().parse::<f64>().unwrap_or(0.0)).map_err(|e| e.to_string())?;
            ws.write(current_row, 11, if shift_id.is_some() { "Terkait Shift" } else { "Non-Shift" }).map_err(|e| e.to_string())?;
            ws.write(current_row, 12, creator).map_err(|e| e.to_string())?;
            ws.write(current_row, 13, status).map_err(|e| e.to_string())?;
            ws.write(current_row, 14, cancel_reason.unwrap_or_else(|| "-".to_string())).map_err(|e| e.to_string())?;

            current_row += 1;
        }

        offset += batch_size;
    }

    Ok((current_row - 1) as usize)
}

// ============================================================================
// 7. STREAMING PRODUCTS EXPORT (100% TEMPLATE IMPORT COMPATIBLE)
// ============================================================================
async fn export_products_streaming(
    pool: &PgPool,
    job: &ExportJob,
    wb: &mut Workbook,
    header_fmt: &Format,
) -> Result<usize, String> {
    let ws = wb.add_worksheet();
    ws.set_name("Template Produk").map_err(|e| e.to_string())?;

    // 1. Ambil seluruh outlet dan customer groups aktif untuk kolom dinamis
    let outlets_list = sqlx::query("SELECT id, code, name FROM outlets WHERE is_active = true ORDER BY code ASC")
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;

    let groups_list = sqlx::query("SELECT id, name FROM customer_groups ORDER BY name ASC")
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;

    let mut headers = vec![
        "Barcode".to_string(),
        "SKU".to_string(),
        "Nama_Produk".to_string(),
        "Kategori".to_string(),
        "Satuan".to_string(),
        "HPP_Beli".to_string(),
        "Harga_Jual_Dasar".to_string(),
        "Stok_Awal".to_string(),
        "Min_Stok".to_string(),
        "Foto_File".to_string(),
    ];

    for o in &outlets_list {
        let code: String = o.get("code");
        headers.push(format!("Harga_Jual_{}", code));
    }

    for g in &groups_list {
        let name: String = g.get("name");
        let clean = name.replace(' ', "_");
        headers.push(format!("Harga_Member_{}", clean));
    }

    for (col_idx, h) in headers.iter().enumerate() {
        ws.write_with_format(0, col_idx as u16, h.as_str(), header_fmt).map_err(|e| e.to_string())?;
    }

    let filter = &job.filter_params;
    let mut where_clause = " WHERE p.is_active = true".to_string();

    if let Some(cid) = filter.get("category_id").and_then(|v| v.as_str()) {
        if let Ok(u) = Uuid::parse_str(cid) {
            where_clause.push_str(&format!(" AND p.category_id = '{}'", u));
        }
    }
    if let Some(sc) = filter.get("search").and_then(|v| v.as_str()) {
        if !sc.trim().is_empty() {
            let clean = sc.trim().replace('\'', "''");
            where_clause.push_str(&format!(
                " AND (p.name ILIKE '%{}%' OR p.barcode ILIKE '%{}%' OR p.sku ILIKE '%{}%')",
                clean, clean, clean
            ));
        }
    }

    let sort_col = match filter.get("sort_by").and_then(|v| v.as_str()) {
        Some("sku") => "p.sku",
        Some("barcode") => "p.barcode",
        Some("cost_price") => "p.cost_price",
        Some("base_selling_price") => "p.base_selling_price",
        _ => "p.name",
    };
    let sort_dir = match filter.get("sort_dir").and_then(|v| v.as_str()) {
        Some("desc") | Some("DESC") => "DESC",
        _ => "ASC",
    };

    let mut current_row: u32 = 1;
    let batch_size: i64 = 5000;
    let mut offset: i64 = 0;

    loop {
        let query = format!(
            "SELECT p.id, p.barcode, p.sku, p.name, p.cost_price, p.base_selling_price,
                    p.min_stock, p.image_url,
                    COALESCE(c.name, 'Umum') as category_name,
                    COALESCE(u.code, 'PCS') as unit_code,
                    COALESCE((SELECT SUM(ws.quantity) FROM warehouse_stocks ws WHERE ws.product_id = p.id), 0) as total_stock
             FROM products p
             LEFT JOIN categories c ON p.category_id = c.id
             LEFT JOIN units u ON p.base_unit_id = u.id
             {}
             ORDER BY {} {}
             LIMIT {} OFFSET {}",
            where_clause, sort_col, sort_dir, batch_size, offset
        );

        let rows = sqlx::query(&query).fetch_all(pool).await.map_err(|e| e.to_string())?;
        if rows.is_empty() {
            break;
        }

        for r in &rows {
            let barcode: String = r.get("barcode");
            let sku: String = r.get("sku");
            let name: String = r.get("name");
            let cat_name: String = r.get("category_name");
            let unit_code: String = r.get("unit_code");
            let cost_price: Decimal = r.get("cost_price");
            let selling_price: Decimal = r.get("base_selling_price");
            let stock_qty: Decimal = r.get("total_stock");
            let min_stock: Decimal = r.get("min_stock");
            let image_url: Option<String> = r.get("image_url");

            let photo_file = image_url
                .map(|u| u.split('/').last().unwrap_or("").to_string())
                .unwrap_or_default();

            ws.write(current_row, 0, barcode).map_err(|e| e.to_string())?;
            ws.write(current_row, 1, sku).map_err(|e| e.to_string())?;
            ws.write(current_row, 2, name).map_err(|e| e.to_string())?;
            ws.write(current_row, 3, cat_name).map_err(|e| e.to_string())?;
            ws.write(current_row, 4, unit_code).map_err(|e| e.to_string())?;
            ws.write(current_row, 5, cost_price.to_string().parse::<f64>().unwrap_or(0.0)).map_err(|e| e.to_string())?;
            ws.write(current_row, 6, selling_price.to_string().parse::<f64>().unwrap_or(0.0)).map_err(|e| e.to_string())?;
            ws.write(current_row, 7, stock_qty.to_string().parse::<f64>().unwrap_or(0.0)).map_err(|e| e.to_string())?;
            ws.write(current_row, 8, min_stock.to_string().parse::<f64>().unwrap_or(0.0)).map_err(|e| e.to_string())?;
            ws.write(current_row, 9, photo_file).map_err(|e| e.to_string())?;

            let mut col_cursor = 10;
            let base_price_val = selling_price.to_string().parse::<f64>().unwrap_or(0.0);

            // Kolom outlet prices
            for _ in &outlets_list {
                ws.write(current_row, col_cursor, base_price_val).map_err(|e| e.to_string())?;
                col_cursor += 1;
            }

            // Kolom member prices
            for _ in &groups_list {
                ws.write(current_row, col_cursor, base_price_val).map_err(|e| e.to_string())?;
                col_cursor += 1;
            }

            current_row += 1;
        }

        offset += batch_size;
    }

    Ok((current_row - 1) as usize)
}
