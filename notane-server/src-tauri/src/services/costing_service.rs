use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row, Transaction, Postgres};
use uuid::Uuid;
use crate::models::product::ProductStockBatch;
use crate::models::settings::InventoryCostingSettings;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostRecalculationResult {
    pub product_id: Uuid,
    pub outlet_id: Uuid,
    pub costing_method: String,
    pub old_cost_price: Decimal,
    pub new_cost_price: Decimal,
    pub old_selling_price: Decimal,
    pub new_selling_price: Decimal,
    pub selling_price_adjusted: bool,
    pub batch_id: Option<Uuid>,
}

/// Ambil pengaturan global metode HPP & valuasi persediaan
pub async fn get_inventory_costing_settings(pool: &PgPool) -> Result<InventoryCostingSettings, String> {
    sqlx::query_as::<_, InventoryCostingSettings>(
        "SELECT id, default_costing_method, default_auto_adjust_selling_price, default_markup_percent, created_at, updated_at
         FROM inventory_costing_settings
         ORDER BY created_at ASC
         LIMIT 1"
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Database error fetching inventory costing settings: {}", e))?
    .ok_or_else(|| "Inventory costing settings tidak ditemukan".to_string())
}

/// Ambil pengaturan global metode HPP menggunakan koneksi transaksi
pub async fn get_inventory_costing_settings_conn(conn: &mut sqlx::PgConnection) -> Result<InventoryCostingSettings, String> {
    sqlx::query_as::<_, InventoryCostingSettings>(
        "SELECT id, default_costing_method, default_auto_adjust_selling_price, default_markup_percent, created_at, updated_at
         FROM inventory_costing_settings
         ORDER BY created_at ASC
         LIMIT 1"
    )
    .fetch_optional(conn)
    .await
    .map_err(|e| format!("Database error fetching inventory costing settings: {}", e))?
    .ok_or_else(|| "Inventory costing settings tidak ditemukan".to_string())
}

/// Evaluasi metode HPP efektif untuk suatu produk:
/// Jika produk diset 'GLOBAL' atau kosong, gunakan pengaturan global dari inventory_costing_settings.
/// Jika produk memiliki setting khusus ('WMA', 'FIFO', 'REPLACEMENT_COST'), gunakan setting produk tersebut.
pub async fn resolve_effective_costing_method_tx(
    conn: &mut sqlx::PgConnection,
    product_method: Option<&str>,
    product_auto_adjust: bool,
    product_markup: Option<Decimal>,
) -> Result<(String, bool, Option<Decimal>), String> {
    let raw_method = product_method.unwrap_or("GLOBAL").trim();
    if raw_method.is_empty() || raw_method.eq_ignore_ascii_case("GLOBAL") {
        let global = get_inventory_costing_settings_conn(conn).await.unwrap_or_else(|_| {
            InventoryCostingSettings {
                id: Uuid::nil(),
                default_costing_method: "WMA".to_string(),
                default_auto_adjust_selling_price: false,
                default_markup_percent: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            }
        });
        let effective_method = if global.default_costing_method.trim().is_empty() {
            "WMA".to_string()
        } else {
            global.default_costing_method.to_uppercase()
        };
        let effective_auto_adjust = if product_auto_adjust { true } else { global.default_auto_adjust_selling_price };
        let effective_markup = product_markup.or(global.default_markup_percent);
        Ok((effective_method, effective_auto_adjust, effective_markup))
    } else {
        Ok((raw_method.to_uppercase(), product_auto_adjust, product_markup))
    }
}

/// Rekalkulasi HPP (Cost Price) dan Harga Jual (Selling Price) per-outlet saat penerimaan barang (kulakan).
/// Mendukung metode:
/// 1. WMA (Weighted Moving Average / Rata-rata Tertimbang Bergerak)
/// 2. FIFO (First In, First Out)
/// 3. REPLACEMENT_COST (Harga Ganti / Last Purchase Price, dengan opsi penyesuaian harga jual otomatis)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchDeductionDetail {
    pub batch_id: Option<Uuid>,
    pub batch_number: String,
    pub quantity: Decimal,
    pub cost_price: Decimal,
    pub subtotal_cost: Decimal,
    pub source_reference_type: String,
    pub source_reference_id: Option<Uuid>,
    pub source_document_number: Option<String>,
    pub expiry_date: Option<chrono::NaiveDate>,
    pub allocation_method: String,
}

pub async fn recalculate_and_apply_cost_on_receipt(
    tx: &mut Transaction<'_, Postgres>,
    product_id: Uuid,
    outlet_id: Uuid,
    received_qty: Decimal,
    purchase_unit_price: Decimal,
    reference_type: &str,
    reference_id: Option<Uuid>,
    batch_number: &str,
) -> Result<CostRecalculationResult, String> {
    recalculate_and_apply_cost_on_receipt_ext(
        tx,
        product_id,
        outlet_id,
        received_qty,
        purchase_unit_price,
        reference_type,
        reference_id,
        batch_number,
        Some(batch_number),
        None,
    )
    .await
}

/// Rekalkulasi HPP (Cost Price) dan Harga Jual (Selling Price) per-outlet saat penerimaan barang (kulakan).
/// Mendukung metode:
/// 1. WMA (Weighted Moving Average / Rata-rata Tertimbang Bergerak)
/// 2. FIFO (First In, First Out)
/// 3. REPLACEMENT_COST (Harga Ganti / Last Purchase Price, dengan opsi penyesuaian harga jual otomatis)
/// Selalu mencatat batch stok fisik agar pelacakan FEFO/FIFO dan referensi pembelian/transfer dapat dipetakan saat checkout.
pub async fn recalculate_and_apply_cost_on_receipt_ext(
    tx: &mut Transaction<'_, Postgres>,
    product_id: Uuid,
    outlet_id: Uuid,
    received_qty: Decimal,
    purchase_unit_price: Decimal,
    reference_type: &str,
    reference_id: Option<Uuid>,
    batch_number: &str,
    source_document_number: Option<&str>,
    expiry_date: Option<chrono::NaiveDate>,
) -> Result<CostRecalculationResult, String> {
    // 1. Ambil data produk master
    let prod_row = sqlx::query(
        "SELECT costing_method, cost_price, base_selling_price, auto_adjust_selling_price, markup_percent 
         FROM products 
         WHERE id = $1"
    )
    .bind(product_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| format!("Database error saat mengambil data produk: {}", e))?
    .ok_or_else(|| "Produk tidak ditemukan".to_string())?;

    let costing_method_opt = prod_row.get::<Option<String>, _>("costing_method");
    let global_cost: Decimal = prod_row.get("cost_price");
    let global_selling: Decimal = prod_row.get("base_selling_price");
    let auto_adjust_selling: bool = prod_row.get("auto_adjust_selling_price");
    let markup_percent: Option<Decimal> = prod_row.get("markup_percent");

    // Evaluasi metode HPP efektif (apakah mewarisi GLOBAL atau setting khusus produk)
    let (effective_method, effective_auto_adjust, effective_markup) = resolve_effective_costing_method_tx(
        &mut **tx,
        costing_method_opt.as_deref(),
        auto_adjust_selling,
        markup_percent,
    ).await?;

    // 2. Ambil harga & HPP aktif pada outlet yang bersangkutan (jika belum ada, gunakan harga global)
    let outlet_price_opt = sqlx::query(
        "SELECT cost_price, base_selling_price 
         FROM product_outlet_prices 
         WHERE outlet_id = $1 AND product_id = $2"
    )
    .bind(outlet_id)
    .bind(product_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| format!("Database error saat mengambil harga outlet: {}", e))?;

    let (current_cost, current_selling) = match outlet_price_opt {
        Some(r) => (r.get::<Decimal, _>("cost_price"), r.get::<Decimal, _>("base_selling_price")),
        None => (global_cost, global_selling),
    };

    // 3. Ambil stok fisik saat ini di outlet tersebut
    let current_stock: Decimal = sqlx::query_scalar(
        "SELECT COALESCE(quantity, 0) FROM warehouse_stocks WHERE warehouse_id = $1 AND product_id = $2"
    )
    .bind(outlet_id)
    .bind(product_id)
    .fetch_optional(&mut **tx)
    .await
    .unwrap_or(None)
    .unwrap_or(Decimal::ZERO);

    // Buat record batch stok fisik untuk SEMUA metode penerimaan
    // agar pelacakan FEFO, FIFO, dan referensi faktur/GRN/transfer selalu tersedia
    let batch_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO product_stock_batches (
            id, product_id, outlet_id, batch_number, initial_qty, remaining_qty,
            cost_price, reference_type, reference_id, source_document_number, expiry_date, received_at, created_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)"
    )
    .bind(batch_id)
    .bind(product_id)
    .bind(outlet_id)
    .bind(batch_number)
    .bind(received_qty)
    .bind(received_qty)
    .bind(purchase_unit_price)
    .bind(reference_type)
    .bind(reference_id)
    .bind(source_document_number)
    .bind(expiry_date)
    .execute(&mut **tx)
    .await
    .map_err(|e| format!("Gagal mencatat batch stok fisik: {}", e))?;

    let (new_cost, new_selling, selling_price_adjusted) = match effective_method.as_str() {
        "FIFO" | "FEFO" => {
            // Pada FIFO / FEFO, HPP aktif yang ditampilkan dan digunakan untuk penjualan berikutnya
            // adalah HPP dari batch tertua yang masih memiliki remaining_qty > 0
            let oldest_batch_cost = sqlx::query_scalar::<_, Decimal>(
                "SELECT cost_price 
                 FROM product_stock_batches 
                 WHERE product_id = $1 AND outlet_id = $2 AND remaining_qty > 0 
                 ORDER BY expiry_date ASC NULLS LAST, received_at ASC, id ASC 
                 LIMIT 1"
            )
            .bind(product_id)
            .bind(outlet_id)
            .fetch_optional(&mut **tx)
            .await
            .unwrap_or(None);

            let calculated_cost = oldest_batch_cost.unwrap_or(purchase_unit_price);
            (calculated_cost, current_selling, false)
        },
        "REPLACEMENT_COST" => {
            // Metode Harga Ganti: HPP langsung disamakan dengan harga kulakan terbaru
            let calculated_cost = purchase_unit_price;
            let mut calculated_selling = current_selling;
            let mut price_adjusted = false;

            // Jika fitur otomatis penyesuaian harga jual aktif:
            if effective_auto_adjust {
                if let Some(mp) = effective_markup {
                    // Jika ada target markup persentase: Selling = Cost * (1 + markup%)
                    let factor = Decimal::ONE + (mp / Decimal::from(100));
                    let adj = (calculated_cost * factor).round_dp(2);
                    if adj != current_selling {
                        calculated_selling = adj;
                        price_adjusted = true;
                    }
                } else if current_cost > Decimal::ZERO && current_selling > current_cost {
                    // Pertahankan rasio margin keuntungan yang lama: (Selling - Cost) / Cost
                    let margin_ratio = (current_selling - current_cost) / current_cost;
                    let adj = (calculated_cost * (Decimal::ONE + margin_ratio)).round_dp(2);
                    if adj != current_selling {
                        calculated_selling = adj;
                        price_adjusted = true;
                    }
                }
            }

            (calculated_cost, calculated_selling, price_adjusted)
        },
        _ => {
            // Default: WMA (Weighted Moving Average / Rata-rata Tertimbang)
            let new_total_stock = current_stock + received_qty;
            let calculated_cost = if new_total_stock > Decimal::ZERO && current_stock > Decimal::ZERO {
                let total_value = (current_stock * current_cost) + (received_qty * purchase_unit_price);
                (total_value / new_total_stock).round_dp(4)
            } else {
                purchase_unit_price
            };

            (calculated_cost, current_selling, false)
        }
    };

    // 4. Perbarui atau buat entri harga per-outlet di product_outlet_prices
    sqlx::query(
        "INSERT INTO product_outlet_prices (
            outlet_id, product_id, cost_price, base_selling_price, last_purchase_price, updated_at
        ) VALUES ($1, $2, $3, $4, $5, CURRENT_TIMESTAMP)
        ON CONFLICT (outlet_id, product_id) DO UPDATE SET
            cost_price = EXCLUDED.cost_price,
            base_selling_price = EXCLUDED.base_selling_price,
            last_purchase_price = EXCLUDED.last_purchase_price,
            updated_at = CURRENT_TIMESTAMP"
    )
    .bind(outlet_id)
    .bind(product_id)
    .bind(new_cost)
    .bind(new_selling)
    .bind(purchase_unit_price)
    .execute(&mut **tx)
    .await
    .map_err(|e| format!("Gagal memperbarui harga produk per outlet: {}", e))?;

    // 5. Perbarui juga cost_price global di tabel products sebagai nilai cadangan
    sqlx::query(
        "UPDATE products SET cost_price = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2"
    )
    .bind(new_cost)
    .bind(product_id)
    .execute(&mut **tx)
    .await
    .ok();

    Ok(CostRecalculationResult {
        product_id,
        outlet_id,
        costing_method: effective_method,
        old_cost_price: current_cost,
        new_cost_price: new_cost,
        old_selling_price: current_selling,
        new_selling_price: new_selling,
        selling_price_adjusted,
        batch_id: Some(batch_id),
    })
}

/// Alokasikan dan konsumsi batch stok dengan prioritas FEFO (First Expired, First Out)
/// lalu FIFO (First In, First Out) saat transaksi penjualan POS.
/// Mengembalikan total Beban Pokok Penjualan (HPP), HPP satuan efektif, dan rincian alokasi batch yang terpotong.
pub async fn allocate_and_consume_stock_batches(
    tx: &mut Transaction<'_, Postgres>,
    product_id: Uuid,
    outlet_id: Uuid,
    qty_to_consume: Decimal,
    fallback_cost: Decimal,
) -> Result<(Decimal, Decimal, Vec<BatchDeductionDetail>), String> {
    if qty_to_consume <= Decimal::ZERO {
        return Ok((Decimal::ZERO, fallback_cost, Vec::new()));
    }

    // Ambil semua batch aktif untuk produk dan outlet ini secara berurutan:
    // 1. FEFO: expiry_date ASC NULLS LAST (yang ada tanggal kedaluwarsa paling dekat diprioritaskan)
    // 2. FIFO: received_at ASC, id ASC (yang masuk lebih lama diambil duluan)
    let batches = sqlx::query(
        "SELECT id, batch_number, remaining_qty, cost_price, reference_type, reference_id, source_document_number, expiry_date 
         FROM product_stock_batches 
         WHERE product_id = $1 AND outlet_id = $2 AND remaining_qty > 0 
         ORDER BY expiry_date ASC NULLS LAST, received_at ASC, id ASC 
         FOR UPDATE"
    )
    .bind(product_id)
    .bind(outlet_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(|e| format!("Database error saat mengambil batch stok: {}", e))?;

    let mut remaining_needed = qty_to_consume;
    let mut total_cogs = Decimal::ZERO;
    let mut allocations: Vec<BatchDeductionDetail> = Vec::new();

    for b in batches {
        if remaining_needed <= Decimal::ZERO {
            break;
        }
        let batch_id: Uuid = b.get("id");
        let batch_number: String = b.get("batch_number");
        let rem_qty: Decimal = b.get("remaining_qty");
        let batch_cost: Decimal = b.get("cost_price");
        let ref_type: String = b.get("reference_type");
        let ref_id: Option<Uuid> = b.get("reference_id");
        let source_doc: Option<String> = b.try_get("source_document_number").unwrap_or(None);
        let exp_date: Option<chrono::NaiveDate> = b.try_get("expiry_date").unwrap_or(None);

        let take_qty = rem_qty.min(remaining_needed);
        let subtotal_cost = take_qty * batch_cost;
        total_cogs += subtotal_cost;
        let new_rem = rem_qty - take_qty;

        sqlx::query("UPDATE product_stock_batches SET remaining_qty = $1 WHERE id = $2")
            .bind(new_rem)
            .bind(batch_id)
            .execute(&mut **tx)
            .await
            .map_err(|e| format!("Gagal memotong kuantitas batch stok: {}", e))?;

        let alloc_method = if exp_date.is_some() { "FEFO".to_string() } else { "FIFO".to_string() };

        allocations.push(BatchDeductionDetail {
            batch_id: Some(batch_id),
            batch_number,
            quantity: take_qty,
            cost_price: batch_cost,
            subtotal_cost,
            source_reference_type: ref_type,
            source_reference_id: ref_id,
            source_document_number: source_doc,
            expiry_date: exp_date,
            allocation_method: alloc_method,
        });

        remaining_needed -= take_qty;
    }

    // Jika stok fisik batch tidak mencukupi (stok awal untracked atau minus), alokasikan sisa ke fallback
    if remaining_needed > Decimal::ZERO {
        let fallback_subtotal = remaining_needed * fallback_cost;
        total_cogs += fallback_subtotal;
        allocations.push(BatchDeductionDetail {
            batch_id: None,
            batch_number: "UNTRACKED-STOCK".to_string(),
            quantity: remaining_needed,
            cost_price: fallback_cost,
            subtotal_cost: fallback_subtotal,
            source_reference_type: "INITIAL_STOCK".to_string(),
            source_reference_id: None,
            source_document_number: None,
            expiry_date: None,
            allocation_method: "FALLBACK".to_string(),
        });
    }

    let effective_unit_cost = if qty_to_consume > Decimal::ZERO {
        (total_cogs / qty_to_consume).round_dp(4)
    } else {
        fallback_cost
    };

    // Update HPP aktif di product_outlet_prices HANYA jika metode yang berlaku adalah FIFO atau FEFO
    let prod_costing_method = sqlx::query_scalar::<_, Option<String>>(
        "SELECT costing_method FROM products WHERE id = $1"
    )
    .bind(product_id)
    .fetch_optional(&mut **tx)
    .await
    .unwrap_or(None)
    .flatten();

    let (eff_method, _, _) = resolve_effective_costing_method_tx(
        &mut **tx,
        prod_costing_method.as_deref(),
        false,
        None,
    ).await.unwrap_or_else(|_| ("WMA".to_string(), false, None));

    if eff_method == "FIFO" || eff_method == "FEFO" {
        let next_batch_cost = sqlx::query_scalar::<_, Decimal>(
            "SELECT cost_price 
             FROM product_stock_batches 
             WHERE product_id = $1 AND outlet_id = $2 AND remaining_qty > 0 
             ORDER BY expiry_date ASC NULLS LAST, received_at ASC, id ASC 
             LIMIT 1"
        )
        .bind(product_id)
        .bind(outlet_id)
        .fetch_optional(&mut **tx)
        .await
        .unwrap_or(None);

        if let Some(nc) = next_batch_cost {
            let _ = sqlx::query(
                "UPDATE product_outlet_prices SET cost_price = $1, updated_at = CURRENT_TIMESTAMP WHERE outlet_id = $2 AND product_id = $3"
            )
            .bind(nc)
            .bind(outlet_id)
            .bind(product_id)
            .execute(&mut **tx)
            .await;
        }
    }


    Ok((total_cogs, effective_unit_cost, allocations))
}

/// Konsumsi batch stok FIFO saat transaksi penjualan POS.
/// Wrapper backward-compatible yang memanggil allocate_and_consume_stock_batches.
pub async fn consume_fifo_batches(
    tx: &mut Transaction<'_, Postgres>,
    product_id: Uuid,
    outlet_id: Uuid,
    qty_to_consume: Decimal,
    fallback_cost: Decimal,
) -> Result<(Decimal, Decimal), String> {
    let (total_cogs, unit_cost, _) = allocate_and_consume_stock_batches(
        tx, product_id, outlet_id, qty_to_consume, fallback_cost
    ).await?;
    Ok((total_cogs, unit_cost))
}


/// Ambil daftar batch stok FIFO untuk suatu produk pada outlet tertentu
pub async fn get_product_stock_batches(
    pool: &PgPool,
    product_id: Uuid,
    outlet_id: Option<Uuid>,
) -> Result<Vec<ProductStockBatch>, String> {
    let rows = if let Some(oid) = outlet_id {
        sqlx::query_as::<_, ProductStockBatch>(
            "SELECT psb.*, o.name as outlet_name, p.name as product_name
             FROM product_stock_batches psb
             JOIN outlets o ON psb.outlet_id = o.id
             JOIN products p ON psb.product_id = p.id
             WHERE psb.product_id = $1 AND psb.outlet_id = $2
             ORDER BY psb.remaining_qty > 0 DESC, psb.expiry_date ASC NULLS LAST, psb.received_at ASC, psb.id ASC"
        )
        .bind(product_id)
        .bind(oid)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, ProductStockBatch>(
            "SELECT psb.*, o.name as outlet_name, p.name as product_name
             FROM product_stock_batches psb
             JOIN outlets o ON psb.outlet_id = o.id
             JOIN products p ON psb.product_id = p.id
             WHERE psb.product_id = $1
             ORDER BY psb.remaining_qty > 0 DESC, psb.expiry_date ASC NULLS LAST, psb.received_at ASC, psb.id ASC"
        )
        .bind(product_id)
        .fetch_all(pool)
        .await
    };

    rows.map_err(|e| format!("Gagal memuat batch stok FIFO: {}", e))
}
