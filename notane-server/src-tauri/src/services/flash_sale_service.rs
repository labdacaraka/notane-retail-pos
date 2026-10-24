use chrono::{Local, NaiveDate, NaiveTime, Utc};
use rust_decimal::Decimal;
use serde_json::{json, Value};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::models::flash_sales::{
    ActivePromoProduct, CreateFlashSaleRequest, FlashSale, FlashSaleDetailResponse,
    FlashSaleItemDetail, FlashSaleOutletDetail,
};

pub async fn list_flash_sales(
    pool: &PgPool,
    status: Option<String>,
    search: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
) -> Result<(Vec<Value>, i64), String> {
    let p = page.unwrap_or(1).max(1);
    let ps = page_size.unwrap_or(10).max(1);
    let offset = (p - 1) * ps;

    let now_date = Local::now().date_naive();
    let now_time = Local::now().time();

    let mut conditions = Vec::new();

    if let Some(ref s) = search {
        let clean = s.trim().replace('\'', "''");
        if !clean.is_empty() {
            conditions.push(format!(
                "(fs.name ILIKE '%{clean}%' OR fs.description ILIKE '%{clean}%')"
            ));
        }
    }

    if let Some(ref st) = status {
        match st.to_uppercase().as_str() {
            "ACTIVE" => {
                conditions.push(format!(
                    "fs.is_active = true AND fs.start_date <= '{now_date}' AND fs.end_date >= '{now_date}'"
                ));
            }
            "UPCOMING" => {
                conditions.push(format!(
                    "fs.is_active = true AND fs.start_date > '{now_date}'"
                ));
            }
            "EXPIRED" => {
                conditions.push(format!("fs.end_date < '{now_date}'"));
            }
            "INACTIVE" => {
                conditions.push("fs.is_active = false".to_string());
            }
            _ => {}
        }
    }

    let where_clause = if conditions.is_empty() {
        "".to_string()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // Proaktif auto-expire program flash sale yang masa berlakunya telah habis
    let _ = sqlx::query(
        "UPDATE flash_sales 
         SET is_active = false, updated_at = CURRENT_TIMESTAMP 
         WHERE is_active = true 
           AND (
               end_date < CURRENT_DATE 
               OR (end_date = CURRENT_DATE AND is_all_day = false AND end_time IS NOT NULL AND end_time < CURRENT_TIME)
           )",
    )
    .execute(pool)
    .await;

    let count_query = format!("SELECT COUNT(*)::bigint FROM flash_sales fs {where_clause}");
    let total_items: i64 = sqlx::query_scalar(&count_query)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Gagal menghitung flash sales: {e}"))?;

    let query_str = format!(
        r#"
        SELECT 
            fs.id,
            fs.name,
            fs.description,
            fs.start_date,
            fs.end_date,
            fs.is_all_day,
            fs.start_time,
            fs.end_time,
            fs.is_active,
            fs.is_stackable_with_member,
            fs.created_at,
            (SELECT COUNT(*)::int FROM flash_sale_items WHERE flash_sale_id = fs.id) AS total_items,
            (SELECT COUNT(*)::int FROM flash_sale_outlets WHERE flash_sale_id = fs.id) AS total_outlets
        FROM flash_sales fs
        {where_clause}
        ORDER BY fs.created_at DESC
        LIMIT {ps} OFFSET {offset}
        "#
    );

    let rows = sqlx::query(&query_str)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Gagal memuat list flash sales: {e}"))?;

    let mut result = Vec::new();
    for row in rows {
        let id: Uuid = row.get("id");
        let name: String = row.get("name");
        let description: Option<String> = row.get("description");
        let start_date: NaiveDate = row.get("start_date");
        let end_date: NaiveDate = row.get("end_date");
        let is_all_day: bool = row.get("is_all_day");
        let start_time: Option<NaiveTime> = row.get("start_time");
        let end_time: Option<NaiveTime> = row.get("end_time");
        let is_active: bool = row.get("is_active");
        let is_stackable_with_member: bool = row.get("is_stackable_with_member");
        let created_at: chrono::DateTime<Utc> = row.get("created_at");
        let total_items_count: i32 = row.get("total_items");
        let total_outlets_count: i32 = row.get("total_outlets");

        // Evaluasi status real-time
        let is_running = is_active
            && now_date >= start_date
            && now_date <= end_date
            && (is_all_day
                || match (start_time, end_time) {
                    (Some(st), Some(et)) => now_time >= st && now_time <= et,
                    (Some(st), None) => now_time >= st,
                    (None, Some(et)) => now_time <= et,
                    _ => true,
                });

        let status_label = if !is_active {
            "NONAKTIF"
        } else if now_date < start_date {
            "AKAN_DATANG"
        } else if now_date > end_date {
            "BERAKHIR"
        } else if is_running {
            "BERJALAN"
        } else {
            "MENUNGGU_JAM_AKTIF"
        };

        result.push(json!({
            "id": id,
            "name": name,
            "description": description,
            "start_date": start_date.to_string(),
            "end_date": end_date.to_string(),
            "is_all_day": is_all_day,
            "start_time": start_time.map(|t| t.format("%H:%M").to_string()),
            "end_time": end_time.map(|t| t.format("%H:%M").to_string()),
            "is_active": is_active,
            "is_stackable_with_member": is_stackable_with_member,
            "created_at": created_at.to_rfc3339(),
            "total_items": total_items_count,
            "total_outlets": total_outlets_count,
            "is_running": is_running,
            "status_label": status_label,
        }));
    }

    Ok((result, total_items))
}

pub async fn get_flash_sale_detail(
    pool: &PgPool,
    id: Uuid,
) -> Result<FlashSaleDetailResponse, String> {
    let fs: FlashSale = sqlx::query_as(
        r#"
        SELECT id, name, description, start_date, end_date, is_all_day,
               start_time, end_time, is_active, is_stackable_with_member,
               created_at, updated_at, created_by
        FROM flash_sales
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Query error: {e}"))?
    .ok_or_else(|| "Program Flash Sale tidak ditemukan".to_string())?;

    let now_date = Local::now().date_naive();
    let now_time = Local::now().time();
    let is_currently_running = fs.is_active
        && now_date >= fs.start_date
        && now_date <= fs.end_date
        && (fs.is_all_day
            || match (fs.start_time, fs.end_time) {
                (Some(st), Some(et)) => now_time >= st && now_time <= et,
                (Some(st), None) => now_time >= st,
                (None, Some(et)) => now_time <= et,
                _ => true,
            });

    // Ambil items produk dengan harga normal & perhitungan promo price
    let items_rows = sqlx::query(
        r#"
        SELECT 
            fsi.id,
            fsi.flash_sale_id,
            fsi.product_id,
            p.sku AS product_sku,
            p.name AS product_name,
            p.barcode AS product_barcode,
            u.code AS unit_code,
            p.base_selling_price AS normal_price,
            fsi.discount_type,
            fsi.discount_value,
            fsi.max_qty_per_transaction,
            fsi.promo_quota,
            fsi.sold_qty
        FROM flash_sale_items fsi
        JOIN products p ON fsi.product_id = p.id
        LEFT JOIN units u ON p.base_unit_id = u.id
        WHERE fsi.flash_sale_id = $1
        ORDER BY p.name ASC
        "#,
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Gagal memuat item flash sale: {e}"))?;

    let mut items = Vec::new();
    for row in items_rows {
        let normal_price: Decimal = row.get("normal_price");
        let discount_type: String = row.get("discount_type");
        let discount_value: Decimal = row.get("discount_value");

        let promo_price = match discount_type.as_str() {
            "FIXED_PRICE" => discount_value,
            "PERCENTAGE" => {
                let cut = (normal_price * discount_value) / Decimal::from(100);
                (normal_price - cut).max(Decimal::ZERO)
            }
            "NOMINAL_CUT" => (normal_price - discount_value).max(Decimal::ZERO),
            _ => normal_price,
        };

        items.push(FlashSaleItemDetail {
            id: row.get("id"),
            flash_sale_id: row.get("flash_sale_id"),
            product_id: row.get("product_id"),
            product_sku: row.get("product_sku"),
            product_name: row.get("product_name"),
            product_barcode: row.get("product_barcode"),
            unit_code: row.get("unit_code"),
            normal_price,
            discount_type,
            discount_value,
            promo_price,
            max_qty_per_transaction: row.get("max_qty_per_transaction"),
            promo_quota: row.get("promo_quota"),
            sold_qty: row.get("sold_qty"),
        });
    }

    // Ambil outlets
    let outlets_rows = sqlx::query(
        r#"
        SELECT fso.outlet_id, o.code AS outlet_code, o.name AS outlet_name
        FROM flash_sale_outlets fso
        JOIN outlets o ON fso.outlet_id = o.id
        WHERE fso.flash_sale_id = $1
        ORDER BY o.name ASC
        "#,
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Gagal memuat outlet flash sale: {e}"))?;

    let mut outlets = Vec::new();
    for row in outlets_rows {
        outlets.push(FlashSaleOutletDetail {
            outlet_id: row.get("outlet_id"),
            outlet_code: row.get("outlet_code"),
            outlet_name: row.get("outlet_name"),
        });
    }

    let total_products = items.len();

    Ok(FlashSaleDetailResponse {
        flash_sale: fs,
        items,
        outlets,
        total_products,
        is_currently_running,
    })
}

pub async fn create_flash_sale(
    pool: &PgPool,
    req: CreateFlashSaleRequest,
    user_id: Option<Uuid>,
) -> Result<FlashSaleDetailResponse, String> {
    if req.name.trim().is_empty() {
        return Err("Nama program flash sale wajib diisi".to_string());
    }
    if req.start_date > req.end_date {
        return Err("Tanggal mulai tidak boleh lebih besar dari tanggal berakhir".to_string());
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("Gagal memulai transaksi DB: {e}"))?;

    let now = chrono::Local::now();
    let now_date = now.date_naive();
    let now_time = now.time();
    let is_expired = req.end_date < now_date
        || (req.end_date == now_date && !req.is_all_day && req.end_time.is_some() && req.end_time.unwrap() < now_time);

    let is_active = if is_expired {
        false
    } else {
        req.is_active.unwrap_or(true)
    };
    let is_stackable = req.is_stackable_with_member.unwrap_or(false);

    let fs: FlashSale = sqlx::query_as(
        r#"
        INSERT INTO flash_sales (
            name, description, start_date, end_date, is_all_day,
            start_time, end_time, is_active, is_stackable_with_member, created_by
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING id, name, description, start_date, end_date, is_all_day,
                  start_time, end_time, is_active, is_stackable_with_member,
                  created_at, updated_at, created_by
        "#,
    )
    .bind(req.name.trim())
    .bind(req.description.as_deref())
    .bind(req.start_date)
    .bind(req.end_date)
    .bind(req.is_all_day)
    .bind(req.start_time)
    .bind(req.end_time)
    .bind(is_active)
    .bind(is_stackable)
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| format!("Gagal menyimpan flash sale: {e}"))?;

    // Simpan target outlets
    for outlet_id in &req.outlet_ids {
        sqlx::query(
            "INSERT INTO flash_sale_outlets (flash_sale_id, outlet_id) VALUES ($1, $2) ON CONFLICT DO NOTHING"
        )
        .bind(fs.id)
        .bind(outlet_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal menyimpan outlet flash sale: {e}"))?;
    }

    // Simpan item-item produk
    for item in &req.items {
        sqlx::query(
            r#"
            INSERT INTO flash_sale_items (
                flash_sale_id, product_id, discount_type, discount_value,
                max_qty_per_transaction, promo_quota
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (flash_sale_id, product_id) DO UPDATE SET
                discount_type = EXCLUDED.discount_type,
                discount_value = EXCLUDED.discount_value,
                max_qty_per_transaction = EXCLUDED.max_qty_per_transaction,
                promo_quota = EXCLUDED.promo_quota
            "#,
        )
        .bind(fs.id)
        .bind(item.product_id)
        .bind(&item.discount_type)
        .bind(item.discount_value)
        .bind(item.max_qty_per_transaction)
        .bind(item.promo_quota)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal menyimpan item flash sale: {e}"))?;
    }

    tx.commit()
        .await
        .map_err(|e| format!("Gagal commit transaksi: {e}"))?;

    get_flash_sale_detail(pool, fs.id).await
}

pub async fn update_flash_sale(
    pool: &PgPool,
    id: Uuid,
    req: CreateFlashSaleRequest,
    _user_id: Option<Uuid>,
) -> Result<FlashSaleDetailResponse, String> {
    if req.name.trim().is_empty() {
        return Err("Nama program flash sale wajib diisi".to_string());
    }
    if req.start_date > req.end_date {
        return Err("Tanggal mulai tidak boleh lebih besar dari tanggal berakhir".to_string());
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("Gagal memulai transaksi DB: {e}"))?;

    let now = chrono::Local::now();
    let now_date = now.date_naive();
    let now_time = now.time();
    let is_expired = req.end_date < now_date
        || (req.end_date == now_date && !req.is_all_day && req.end_time.is_some() && req.end_time.unwrap() < now_time);

    let is_active = if is_expired {
        false
    } else {
        req.is_active.unwrap_or(true)
    };
    let is_stackable = req.is_stackable_with_member.unwrap_or(false);

    sqlx::query(
        r#"
        UPDATE flash_sales
        SET name = $1,
            description = $2,
            start_date = $3,
            end_date = $4,
            is_all_day = $5,
            start_time = $6,
            end_time = $7,
            is_active = $8,
            is_stackable_with_member = $9,
            updated_at = CURRENT_TIMESTAMP
        WHERE id = $10
        "#,
    )
    .bind(req.name.trim())
    .bind(req.description.as_deref())
    .bind(req.start_date)
    .bind(req.end_date)
    .bind(req.is_all_day)
    .bind(req.start_time)
    .bind(req.end_time)
    .bind(is_active)
    .bind(is_stackable)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal mengupdate master flash sale: {e}"))?;

    // Refresh outlets
    sqlx::query("DELETE FROM flash_sale_outlets WHERE flash_sale_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal reset outlets flash sale: {e}"))?;

    for outlet_id in &req.outlet_ids {
        sqlx::query(
            "INSERT INTO flash_sale_outlets (flash_sale_id, outlet_id) VALUES ($1, $2) ON CONFLICT DO NOTHING"
        )
        .bind(id)
        .bind(outlet_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal menyimpan outlet flash sale: {e}"))?;
    }

    // Refresh items
    sqlx::query("DELETE FROM flash_sale_items WHERE flash_sale_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal reset items flash sale: {e}"))?;

    for item in &req.items {
        sqlx::query(
            r#"
            INSERT INTO flash_sale_items (
                flash_sale_id, product_id, discount_type, discount_value,
                max_qty_per_transaction, promo_quota
            )
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(id)
        .bind(item.product_id)
        .bind(&item.discount_type)
        .bind(item.discount_value)
        .bind(item.max_qty_per_transaction)
        .bind(item.promo_quota)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal menyimpan item flash sale: {e}"))?;
    }

    tx.commit()
        .await
        .map_err(|e| format!("Gagal commit update: {e}"))?;

    get_flash_sale_detail(pool, id).await
}

pub async fn delete_flash_sale(pool: &PgPool, id: Uuid) -> Result<(), String> {
    sqlx::query("DELETE FROM flash_sales WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| format!("Gagal menghapus program flash sale: {e}"))?;
    Ok(())
}

/// Endpoint kasir POS untuk mendapatkan produk diskon flash sale aktif saat ini di outlet tertentu
pub async fn get_active_promos_for_outlet(
    pool: &PgPool,
    outlet_id: Option<Uuid>,
) -> Result<Vec<ActivePromoProduct>, String> {
    let now_date = Local::now().date_naive();
    let now_time = Local::now().time();

    let outlet_filter = if let Some(oid) = outlet_id {
        format!(
            "AND (
                NOT EXISTS (SELECT 1 FROM flash_sale_outlets fso_check WHERE fso_check.flash_sale_id = fs.id)
                OR EXISTS (SELECT 1 FROM flash_sale_outlets fso_target WHERE fso_target.flash_sale_id = fs.id AND fso_target.outlet_id = '{oid}')
            )"
        )
    } else {
        "".to_string()
    };

    let query_str = format!(
        r#"
        SELECT 
            fs.id AS flash_sale_id,
            fs.name AS flash_sale_name,
            fs.is_stackable_with_member,
            fsi.product_id,
            fsi.discount_type,
            fsi.discount_value,
            fsi.max_qty_per_transaction,
            fsi.promo_quota,
            fsi.sold_qty,
            COALESCE(pop.base_selling_price, p.base_selling_price) AS current_base_price
        FROM flash_sales fs
        JOIN flash_sale_items fsi ON fs.id = fsi.flash_sale_id
        JOIN products p ON fsi.product_id = p.id
        LEFT JOIN product_outlet_prices pop ON p.id = pop.product_id AND pop.outlet_id = $1
        WHERE fs.is_active = true
          AND fs.start_date <= $2
          AND fs.end_date >= $2
          AND (
              fs.is_all_day = true
              OR (
                  (fs.start_time IS NULL OR fs.start_time <= $3)
                  AND (fs.end_time IS NULL OR fs.end_time >= $3)
              )
          )
          AND (fsi.promo_quota IS NULL OR fsi.sold_qty < fsi.promo_quota)
          {outlet_filter}
        ORDER BY fsi.discount_value DESC
        "#
    );

    let rows = sqlx::query(&query_str)
        .bind(outlet_id)
        .bind(now_date)
        .bind(now_time)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Gagal memuat active promos: {e}"))?;

    let mut promos = Vec::new();
    for row in rows {
        let base_price: Decimal = row.get("current_base_price");
        let discount_type: String = row.get("discount_type");
        let discount_value: Decimal = row.get("discount_value");

        let promo_price = match discount_type.as_str() {
            "FIXED_PRICE" => discount_value,
            "PERCENTAGE" => {
                let cut = (base_price * discount_value) / Decimal::from(100);
                (base_price - cut).max(Decimal::ZERO)
            }
            "NOMINAL_CUT" => (base_price - discount_value).max(Decimal::ZERO),
            _ => base_price,
        };

        let promo_quota: Option<Decimal> = row.get("promo_quota");
        let sold_qty: Decimal = row.get("sold_qty");
        let remaining_quota = promo_quota.map(|q| (q - sold_qty).max(Decimal::ZERO));

        promos.push(ActivePromoProduct {
            flash_sale_id: row.get("flash_sale_id"),
            flash_sale_name: row.get("flash_sale_name"),
            product_id: row.get("product_id"),
            discount_type,
            discount_value,
            promo_price,
            is_stackable_with_member: row.get("is_stackable_with_member"),
            max_qty_per_transaction: row.get("max_qty_per_transaction"),
            remaining_quota,
        });
    }

    Ok(promos)
}
