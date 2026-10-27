use crate::models::product::{Brand, Category, CreateProductRequest, Product, Unit};
use crate::models::settings::{
    InventoryCostingSettings, LoyaltySettings, ReceiptSettings,
    UpdateBiometricSettingsRequest, UpdateInventoryCostingSettingsRequest,
    UpdateLoyaltySettingsRequest, UpdateReceiptSettingsRequest,
};
use crate::utils::jwt::{extract_claims, require_any_permission, require_permission};
use axum::{
    extract::{Path, Query, Request, State},
    http::{HeaderMap, Method, StatusCode},
    middleware::{from_fn_with_state, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

async fn master_write_middleware(
    State(pool): State<PgPool>,
    req: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let method = req.method();
    if method == Method::OPTIONS {
        return Ok(next.run(req).await);
    }

    let path = req.uri().path();

    // Special case: barcode-scan is POST lookup (can be accessed by cashiers and managers)
    if path.contains("/products/barcode-scan") {
        require_any_permission(&pool, req.headers(), &["products:barcodes", "pos:checkout", "master:read", "master:write"]).await?;
        return Ok(next.run(req).await);
    }

    // License Module Guards (Non-Core Features)
    if path.contains("/customers") || path.contains("/customer-groups") {
        crate::utils::jwt::require_license_feature(&pool, "membership", "Pelanggan & Membership").await?;
    } else if path.contains("/price-labels") || path.contains("/print-labels") {
        crate::utils::jwt::require_license_feature(&pool, "label_printing", "Cetak Label Harga").await?;
    } else if path.contains("/vouchers") {
        crate::utils::jwt::require_license_feature(&pool, "promotions", "Voucher & Promosi").await?;
    } else if path.contains("/outlets") && method == Method::POST {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM outlets").fetch_one(&pool).await.unwrap_or(0);
        if count >= 1 {
            crate::utils::jwt::require_license_feature(&pool, "multi_outlet", "Multi-Outlet Cabang").await?;
        }
    }

    if method == Method::GET {
        return Ok(next.run(req).await);
    }

    // Granular permissions for specific action features
    if path.contains("/customer-group-prices") || path.contains("/outlet-customer-group-prices") {
        require_any_permission(&pool, req.headers(), &["products:customer_group_prices", "products:update", "master:write"]).await?;
    } else if path.contains("/outlet-prices") {
        require_any_permission(&pool, req.headers(), &["products:outlet_prices", "products:update", "master:write"]).await?;
    } else if path.contains("/price-tiers") {
        require_any_permission(&pool, req.headers(), &["products:price_tiers", "products:update", "master:write"]).await?;
    } else if path.contains("/barcodes") {
        require_any_permission(&pool, req.headers(), &["products:barcodes", "products:update", "master:write"]).await?;
    } else if path.contains("/products/import") || path.contains("/products/upload-photo") {
        require_any_permission(&pool, req.headers(), &["products:import", "products:create", "products:update", "master:write"]).await?;
    } else if path.contains("/customers") || path.contains("/customer-groups") {
        if path.contains("/adjust-points") {
            require_any_permission(&pool, req.headers(), &["loyalty:adjust", "loyalty:manage", "customers:update", "customers:manage", "master:write"]).await?;
        } else if method == Method::POST {
            require_any_permission(&pool, req.headers(), &["customers:create", "customers:manage", "master:write"]).await?;
        } else if method == Method::PUT {
            require_any_permission(&pool, req.headers(), &["customers:update", "customers:manage", "master:write"]).await?;
        } else if method == Method::DELETE {
            require_any_permission(&pool, req.headers(), &["customers:delete", "customers:manage", "master:write"]).await?;
        } else {
            require_any_permission(&pool, req.headers(), &["customers:manage", "master:write"]).await?;
        }
    } else if path.contains("/vouchers") {
        if method == Method::POST {
            require_any_permission(&pool, req.headers(), &["vouchers:create", "vouchers:manage", "master:write"]).await?;
        } else if method == Method::PUT {
            require_any_permission(&pool, req.headers(), &["vouchers:update", "vouchers:manage", "master:write"]).await?;
        } else if method == Method::DELETE {
            require_any_permission(&pool, req.headers(), &["vouchers:delete", "vouchers:manage", "master:write"]).await?;
        } else {
            require_any_permission(&pool, req.headers(), &["vouchers:read", "vouchers:claims", "vouchers:manage", "master:write", "master:read"]).await?;
        }
    } else if path.contains("/settings/biometric") {
        require_any_permission(&pool, req.headers(), &["settings:biometric", "settings:update", "settings:manage", "master:write"]).await?;
    } else if path.contains("/settings/loyalty") || path.contains("/loyalty/settings") {
        require_any_permission(&pool, req.headers(), &["loyalty:manage", "settings:update", "settings:manage", "master:write"]).await?;
    } else if path.contains("/settings/inventory-costing") {
        require_any_permission(&pool, req.headers(), &["inventory:costing", "settings:update", "settings:manage", "master:write"]).await?;
    } else if path.contains("/settings/") {
        require_any_permission(&pool, req.headers(), &["settings:update", "settings:manage", "master:write"]).await?;
    } else if path.contains("/suppliers") {
        if method == Method::POST {
            require_any_permission(&pool, req.headers(), &["suppliers:create", "purchasing:create", "purchasing:write", "master:write"]).await?;
        } else if method == Method::PUT {
            require_any_permission(&pool, req.headers(), &["suppliers:update", "purchasing:update", "purchasing:write", "master:write"]).await?;
        } else if method == Method::DELETE {
            require_any_permission(&pool, req.headers(), &["suppliers:delete", "purchasing:write", "master:write"]).await?;
        } else {
            require_any_permission(&pool, req.headers(), &["purchasing:write", "master:write"]).await?;
        }
    } else if path.contains("/categories") {
        if method == Method::POST {
            require_any_permission(&pool, req.headers(), &["categories:create", "master:write"]).await?;
        } else if method == Method::PUT {
            require_any_permission(&pool, req.headers(), &["categories:update", "master:write"]).await?;
        } else if method == Method::DELETE {
            require_any_permission(&pool, req.headers(), &["categories:delete", "master:write"]).await?;
        } else {
            require_any_permission(&pool, req.headers(), &["master:write"]).await?;
        }
    } else if path.contains("/brands") {
        if method == Method::POST {
            require_any_permission(&pool, req.headers(), &["brands:create", "master:write"]).await?;
        } else if method == Method::PUT {
            require_any_permission(&pool, req.headers(), &["brands:update", "master:write"]).await?;
        } else if method == Method::DELETE {
            require_any_permission(&pool, req.headers(), &["brands:delete", "master:write"]).await?;
        } else {
            require_any_permission(&pool, req.headers(), &["master:write"]).await?;
        }
    } else if path.contains("/units") {
        if method == Method::POST {
            require_any_permission(&pool, req.headers(), &["units:create", "master:write"]).await?;
        } else if method == Method::PUT {
            require_any_permission(&pool, req.headers(), &["units:update", "master:write"]).await?;
        } else if method == Method::DELETE {
            require_any_permission(&pool, req.headers(), &["units:delete", "master:write"]).await?;
        } else {
            require_any_permission(&pool, req.headers(), &["master:write"]).await?;
        }
    } else if path.contains("/outlets") {
        if method == Method::POST {
            require_any_permission(&pool, req.headers(), &["outlets:create", "outlets:manage", "master:write"]).await?;
        } else if method == Method::PUT {
            require_any_permission(&pool, req.headers(), &["outlets:update", "outlets:manage", "master:write"]).await?;
        } else if method == Method::DELETE {
            require_any_permission(&pool, req.headers(), &["outlets:delete", "outlets:manage", "master:write"]).await?;
        } else {
            require_any_permission(&pool, req.headers(), &["outlets:manage", "master:write"]).await?;
        }
    } else if path.contains("/products") {
        if method == Method::POST {
            require_any_permission(&pool, req.headers(), &["products:create", "master:write"]).await?;
        } else if method == Method::PUT {
            require_any_permission(&pool, req.headers(), &["products:update", "master:write"]).await?;
        } else if method == Method::DELETE {
            require_any_permission(&pool, req.headers(), &["products:delete", "master:write"]).await?;
        } else {
            require_permission(&pool, req.headers(), "master:write").await?;
        }
    } else {
        require_permission(&pool, req.headers(), "master:write").await?;
    }

    Ok(next.run(req).await)
}

pub fn master_routes(pool: PgPool) -> Router<PgPool> {
    Router::new()
        // Products
        .route("/products", get(list_products_handler).post(create_product_handler))
        .route("/products/import", post(import_products_handler))
        .route("/products/upload-photo", post(upload_product_photo_handler))
        .route("/products/{id}", put(update_product_handler).delete(delete_product_handler))
        .route("/products/{id}/units", post(add_product_unit_handler))
        .route("/products/units/{id}", delete(delete_product_unit_handler))
        .route("/products/{id}/price-tiers", post(add_product_tier_handler))
        .route("/products/price-tiers/{id}", delete(delete_product_tier_handler))
        .route("/products/{id}/customer-group-prices", get(get_product_group_prices_handler).post(save_product_group_price_handler))
        .route("/products/customer-group-prices/{id}", delete(delete_product_group_price_handler))
        .route("/products/{id}/outlet-prices", get(get_product_outlet_prices_handler).post(save_product_outlet_price_handler))
        .route("/products/outlet-prices/{id}", delete(delete_product_outlet_price_handler))
        .route("/products/{id}/batches", get(get_product_batches_handler))
        .route("/products/batches/{id}/sales", get(get_batch_sales_usage_handler))
        .route("/batches/{id}/sales", get(get_batch_sales_usage_handler))
        .route("/products/{id}/outlet-customer-group-prices", get(get_outlet_cgp_handler).post(save_outlet_cgp_handler))
        .route("/products/outlet-customer-group-prices/{id}", delete(delete_outlet_cgp_handler))

        // Multi-Barcodes
        .route("/products/barcode-scan", post(check_scanned_barcode_handler))
        .route("/products/{id}/barcodes", get(get_product_barcodes_handler).post(add_product_barcode_handler))
        .route("/products/{id}/barcodes/{barcode_id}", delete(delete_product_barcode_handler))
        .route("/products/{id}/barcodes/{barcode_id}/set-primary", put(set_primary_barcode_handler))

        // Customers & Customer Groups
        .route("/customers/point-ledgers", get(list_all_point_ledgers_handler))
        .route("/customers", get(list_customers_handler).post(create_customer_handler))
        .route("/customers/{id}", put(update_customer_handler).delete(delete_customer_handler))
        .route("/customers/{id}/point-ledgers", get(get_customer_point_history_handler))
        .route("/customers/{id}/adjust-points", post(adjust_customer_points_handler))
        .route("/customer-groups", get(list_customer_groups_handler).post(create_customer_group_handler))
        .route("/customer-groups/{id}", put(update_customer_group_handler).delete(delete_customer_group_handler))

        // Vouchers & Promotions
        .route("/vouchers", get(list_vouchers_handler).post(create_voucher_handler))
        .route("/vouchers/{id}", put(update_voucher_handler).delete(delete_voucher_handler))
        .route("/vouchers/{id}/claims", get(get_voucher_claims_handler))
        .route("/vouchers/validate/{code}", get(validate_voucher_handler))

        // Units (Satuan)
        .route("/units", get(list_units_handler).post(create_unit_handler))
        .route("/units/{id}", put(update_unit_handler).delete(delete_unit_handler))

        // Categories (Kategori)
        .route("/categories", get(list_categories_handler).post(create_category_handler))
        .route("/categories/{id}", put(update_category_handler).delete(delete_category_handler))

        // Brands (Merek)
        .route("/brands", get(list_brands_handler).post(create_brand_handler))
        .route("/brands/{id}", put(update_brand_handler).delete(delete_brand_handler))

        // Suppliers (Pemasok)
        .route("/suppliers", get(list_suppliers_handler).post(create_supplier_handler))
        .route("/suppliers/{id}", put(update_supplier_handler).delete(delete_supplier_handler))

        // Payment Methods & Reasons
        .route("/payment-methods", get(list_payment_methods_handler))
        .route("/reason-codes", get(list_reason_codes_handler))
        .route("/warehouses", get(list_warehouses_handler))
        .route("/outlets", get(list_outlets_handler).post(create_outlet_handler))
        .route("/outlets/{id}", get(get_outlet_handler).put(update_outlet_handler).delete(delete_outlet_handler))

        // Receipt, Loyalty & Store Settings
        .route("/settings/receipt", get(get_receipt_settings_handler).post(save_receipt_settings_handler).put(save_receipt_settings_handler))
        .route("/settings/biometric", get(get_biometric_settings_handler).post(save_biometric_settings_handler).put(save_biometric_settings_handler))
        .route("/settings/loyalty", get(get_loyalty_settings_handler).post(save_loyalty_settings_handler).put(save_loyalty_settings_handler))
        .route("/loyalty/settings", get(get_loyalty_settings_handler).post(save_loyalty_settings_handler).put(save_loyalty_settings_handler))
        .route("/settings/inventory-costing", get(get_inventory_costing_settings_handler).post(save_inventory_costing_settings_handler).put(save_inventory_costing_settings_handler))
        .layer(from_fn_with_state(pool, master_write_middleware))
}

// ============================================================================
// 1. PRODUCTS CRUD
// ============================================================================

#[derive(Deserialize)]
struct ProductQuery {
    category_id: Option<Uuid>,
    brand_id: Option<Uuid>,
    search: Option<String>,
    outlet_id: Option<Uuid>,
    stock_status: Option<String>,
    sort_by: Option<String>,
    sort_dir: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
    limit: Option<i64>,
}

async fn list_products_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(query): Query<ProductQuery>,
) -> impl IntoResponse {
    let effective_outlet_id = crate::utils::jwt::resolve_effective_outlet_id(&pool, &headers, query.outlet_id).await;

    let default_costing_method = sqlx::query_scalar::<_, String>(
        "SELECT default_costing_method FROM inventory_costing_settings ORDER BY created_at ASC LIMIT 1"
    )
    .fetch_optional(&pool)
    .await
    .unwrap_or(None)
    .unwrap_or_else(|| "WMA".to_string());

    let (pop_join, cost_col, price_col, pop_group) = if let Some(oid) = effective_outlet_id {
        (
            format!("LEFT JOIN product_outlet_prices pop ON pop.product_id = p.id AND pop.outlet_id = '{}'", oid),
            format!(
                "CASE \
                    WHEN (CASE WHEN UPPER(COALESCE(p.costing_method, 'GLOBAL')) IN ('FIFO', 'FEFO', 'REPLACEMENT_COST', 'WMA') THEN UPPER(p.costing_method) ELSE '{default_costing_method}' END) IN ('FIFO', 'FEFO') \
                    THEN COALESCE((SELECT psb.cost_price FROM product_stock_batches psb WHERE psb.product_id = p.id AND psb.outlet_id = '{oid}' AND psb.remaining_qty > 0 ORDER BY psb.expiry_date ASC NULLS LAST, psb.received_at ASC, psb.id ASC LIMIT 1), NULLIF(pop.cost_price, 0), p.cost_price, 0) \
                    WHEN (CASE WHEN UPPER(COALESCE(p.costing_method, 'GLOBAL')) IN ('FIFO', 'FEFO', 'REPLACEMENT_COST', 'WMA') THEN UPPER(p.costing_method) ELSE '{default_costing_method}' END) = 'REPLACEMENT_COST' \
                    THEN COALESCE(NULLIF(pop.last_purchase_price, 0), NULLIF(pop.cost_price, 0), p.cost_price, 0) \
                    ELSE COALESCE(NULLIF(pop.cost_price, 0), p.cost_price, 0) \
                END as cost_price"
            ),
            "COALESCE(NULLIF(pop.base_selling_price, 0), p.base_selling_price) as base_selling_price".to_string(),
            ", pop.cost_price, pop.base_selling_price, pop.last_purchase_price".to_string(),
        )
    } else {
        (
            "".to_string(),
            format!(
                "CASE \
                    WHEN (CASE WHEN UPPER(COALESCE(p.costing_method, 'GLOBAL')) IN ('FIFO', 'FEFO', 'REPLACEMENT_COST', 'WMA') THEN UPPER(p.costing_method) ELSE '{default_costing_method}' END) IN ('FIFO', 'FEFO') \
                    THEN COALESCE((SELECT psb.cost_price FROM product_stock_batches psb WHERE psb.product_id = p.id AND psb.remaining_qty > 0 ORDER BY psb.expiry_date ASC NULLS LAST, psb.received_at ASC, psb.id ASC LIMIT 1), p.cost_price, 0) \
                    ELSE COALESCE(p.cost_price, 0) \
                END as cost_price"
            ),
            "p.base_selling_price".to_string(),
            "".to_string(),
        )
    };

    let mut filter_sql = " WHERE p.is_active = true".to_string();

    if let Some(cat) = query.category_id {
        filter_sql.push_str(&format!(" AND p.category_id = '{}'", cat));
    }
    if let Some(brand) = query.brand_id {
        filter_sql.push_str(&format!(" AND p.brand_id = '{}'", brand));
    }
    if let Some(s) = &query.search {
        if !s.trim().is_empty() {
            let clean = s.trim().replace('\'', "''");
            filter_sql.push_str(&format!(
                " AND (p.name ILIKE '%{}%' OR p.barcode ILIKE '%{}%' OR p.sku ILIKE '%{}%' OR EXISTS (SELECT 1 FROM product_barcodes pb WHERE pb.product_id = p.id AND pb.barcode ILIKE '%{}%'))",
                clean, clean, clean, clean
            ));
        }
    }

    let ws_join = if let Some(oid) = effective_outlet_id {
        format!(
            "LEFT JOIN warehouse_stocks ws ON p.id = ws.product_id AND ws.warehouse_id = '{oid}'"
        )
    } else {
        "LEFT JOIN warehouse_stocks ws ON p.id = ws.product_id".to_string()
    };

    // Filter status stok (HAVING)
    let having_clause = match query.stock_status.as_deref() {
        Some("LOW") => " HAVING COALESCE(SUM(ws.quantity), 0) <= p.min_stock AND COALESCE(SUM(ws.quantity), 0) > 0",
        Some("EMPTY") => " HAVING COALESCE(SUM(ws.quantity), 0) <= 0",
        Some("AVAILABLE") => " HAVING COALESCE(SUM(ws.quantity), 0) > 0",
        _ => "",
    };

    // Hitung total items untuk server-side pagination
    let count_query = if query.stock_status.is_some() {
        format!(
            "SELECT COUNT(*)::bigint FROM (
                SELECT p.id
                FROM products p
                {}
                {}
                GROUP BY p.id
                {}
            ) sub",
            ws_join, filter_sql, having_clause
        )
    } else {
        format!("SELECT COUNT(*)::bigint as total FROM products p {}", filter_sql)
    };
    let total_items: i64 = sqlx::query_scalar(&count_query)
        .fetch_one(&pool)
        .await
        .unwrap_or(0);

    let mut sql = format!(
        "SELECT p.id, p.category_id, p.base_unit_id, p.brand_id, p.sku, p.barcode, p.name, p.image_url,
                p.min_stock, p.max_stock, p.tax_percent, p.is_weight_embedded,
                p.costing_method, p.auto_adjust_selling_price, p.markup_percent,
                c.name as category_name, u.code as unit_code,
                {}, {},
                COALESCE(SUM(ws.quantity), 0) as stock_qty,
                (SELECT COUNT(*)::bigint FROM product_barcodes pb WHERE pb.product_id = p.id) as barcode_count
         FROM products p 
         JOIN categories c ON p.category_id = c.id 
         JOIN units u ON p.base_unit_id = u.id 
         {}
         {} 
         {}",
        cost_col, price_col, pop_join, ws_join, filter_sql
    );

    sql.push_str(&format!(" GROUP BY p.id, c.name, u.code{}{}", pop_group, having_clause));

    // Dynamic sorting
    let sort_col = match query.sort_by.as_deref() {
        Some("sku") => "p.sku",
        Some("barcode") => "p.barcode",
        Some("category_name") => "category_name",
        Some("cost_price") => "cost_price",
        Some("base_selling_price") => "base_selling_price",
        Some("stock_qty") => "stock_qty",
        _ => "p.name",
    };
    let sort_dir = match query.sort_dir.as_deref() {
        Some("desc") | Some("DESC") => "DESC",
        _ => "ASC",
    };
    sql.push_str(&format!(" ORDER BY {} {}", sort_col, sort_dir));

    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(query.limit.unwrap_or(10)).max(1);

    if query.page.is_some() || query.page_size.is_some() {
        let offset = (page - 1) * page_size;
        sql.push_str(&format!(" LIMIT {} OFFSET {}", page_size, offset));
    } else if let Some(lim) = query.limit {
        if lim > 0 {
            sql.push_str(&format!(" LIMIT {}", lim));
        }
    }

    let rows = sqlx::query(&sql).fetch_all(&pool).await;

    match rows {
        Ok(r) => {
            let list: Vec<serde_json::Value> = r
                .iter()
                .map(|row| {
                    json!({
                        "id": row.get::<Uuid, _>("id"),
                        "category_id": row.get::<Uuid, _>("category_id"),
                        "base_unit_id": row.get::<Uuid, _>("base_unit_id"),
                        "brand_id": row.get::<Option<Uuid>, _>("brand_id"),
                        "sku": row.get::<String, _>("sku"),
                        "barcode": row.get::<String, _>("barcode"),
                        "name": row.get::<String, _>("name"),
                        "image_url": row.get::<Option<String>, _>("image_url"),
                        "category_name": row.get::<String, _>("category_name"),
                        "unit_code": row.get::<String, _>("unit_code"),
                        "costing_method": row.get::<String, _>("costing_method"),
                        "auto_adjust_selling_price": row.get::<bool, _>("auto_adjust_selling_price"),
                        "markup_percent": row.get::<Option<Decimal>, _>("markup_percent"),
                        "cost_price": row.get::<Decimal, _>("cost_price"),
                        "base_selling_price": row.get::<Decimal, _>("base_selling_price"),
                        "min_stock": row.get::<Decimal, _>("min_stock"),
                        "max_stock": row.get::<Decimal, _>("max_stock"),
                        "tax_percent": row.get::<Decimal, _>("tax_percent"),
                        "is_weight_embedded": row.get::<bool, _>("is_weight_embedded"),
                        "stock_qty": row.get::<Decimal, _>("stock_qty"),
                        "barcode_count": row.get::<i64, _>("barcode_count"),
                    })
                })
                .collect();

            let total_pages = if total_items > 0 {
                ((total_items as f64) / (page_size as f64)).ceil() as i64
            } else {
                1
            };

            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "data": list,
                    "total_items": total_items,
                    "page": page,
                    "page_size": page_size,
                    "total_pages": total_pages,
                })),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": e.to_string() })),
        )
            .into_response(),
    }
}

async fn record_initial_stock_journal(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    outlet_id: Uuid,
    product_id: Uuid,
    product_name: &str,
    product_sku: &str,
    qty: Decimal,
    cost_price: Decimal,
    user_id: Option<Uuid>,
) -> Result<(), String> {
    if qty <= Decimal::ZERO || cost_price <= Decimal::ZERO {
        return Ok(());
    }

    let total_cost = qty * cost_price;
    if total_cost <= Decimal::ZERO {
        return Ok(());
    }

    // 1. Dapatkan akun Persediaan Barang Dagang (1201) untuk outlet ini
    let inv_account_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT COALESCE(inventory_account_id, (SELECT id FROM chart_of_accounts WHERE code = '1201' LIMIT 1))
         FROM outlets WHERE id = $1",
    )
    .bind(outlet_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| e.to_string())?
    .flatten();

    let inv_account_id = match inv_account_id {
        Some(id) => id,
        None => sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '1201' LIMIT 1")
            .fetch_optional(&mut **tx)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Akun Persediaan (1201) tidak ditemukan di Chart of Accounts".to_string())?,
    };

    // 2. Dapatkan akun Modal Awal Pemilik (3101)
    let capital_account_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT id FROM chart_of_accounts WHERE code = '3101' LIMIT 1",
    )
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| e.to_string())?;

    let capital_account_id = match capital_account_id {
        Some(id) => id,
        None => sqlx::query_scalar(
            "SELECT id FROM chart_of_accounts WHERE account_category_id = 3 ORDER BY code ASC LIMIT 1",
        )
        .fetch_optional(&mut **tx)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Akun Modal (3101 / Equity) tidak ditemukan di Chart of Accounts".to_string())?,
    };

    let journal_id = Uuid::new_v4();
    let rand_suffix = &Uuid::new_v4().to_string()[..6].to_uppercase();
    let journal_number = format!("JRN-INIT-{}-{}", Utc::now().format("%Y%m%d%H%M%S"), rand_suffix);
    let desc = format!("Saldo Awal Persediaan Produk {} ({}) Qty {}", product_name, product_sku, qty);

    // 3. Insert header jurnal
    sqlx::query(
        "INSERT INTO journal_entries (
            id, journal_number, journal_date, reference_type, reference_id, description,
            total_debit, total_credit, is_closing_entry, created_at, updated_at, created_by, updated_by
        ) VALUES (
            $1, $2, CURRENT_DATE, 'INITIAL_STOCK', $3, $4, $5, $5, FALSE, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $6, $6
        )",
    )
    .bind(journal_id)
    .bind(&journal_number)
    .bind(product_id)
    .bind(&desc)
    .bind(total_cost)
    .bind(user_id)
    .execute(&mut **tx)
    .await
    .map_err(|e| format!("Gagal mencatat header jurnal saldo awal: {}", e))?;

    // 4. Line 1: Debet Persediaan Barang Dagang (1201)
    sqlx::query(
        "INSERT INTO journal_lines (
            id, journal_id, account_id, debit, credit, memo, outlet_id, created_at, updated_at, created_by, updated_by
        ) VALUES (
            gen_random_uuid(), $1, $2, $3, 0, $4, $5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $6, $6
        )",
    )
    .bind(journal_id)
    .bind(inv_account_id)
    .bind(total_cost)
    .bind(format!("Debet Persediaan Awal {} ({})", product_name, product_sku))
    .bind(outlet_id)
    .bind(user_id)
    .execute(&mut **tx)
    .await
    .map_err(|e| format!("Gagal mencatat baris debet persediaan: {}", e))?;

    // 5. Line 2: Kredit Modal Awal Pemilik (3101)
    sqlx::query(
        "INSERT INTO journal_lines (
            id, journal_id, account_id, debit, credit, memo, outlet_id, created_at, updated_at, created_by, updated_by
        ) VALUES (
            gen_random_uuid(), $1, $2, 0, $3, $4, $5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $6, $6
        )",
    )
    .bind(journal_id)
    .bind(capital_account_id)
    .bind(total_cost)
    .bind(format!("Kredit Modal Awal Persediaan {} ({})", product_name, product_sku))
    .bind(outlet_id)
    .bind(user_id)
    .execute(&mut **tx)
    .await
    .map_err(|e| format!("Gagal mencatat baris kredit modal awal: {}", e))?;

    Ok(())
}

async fn create_product_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<CreateProductRequest>,
) -> impl IntoResponse {
    let user_id = extract_claims(&headers).ok().map(|c| c.sub);
    let mut tx = match pool.begin().await {
        Ok(t) => t,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    };

    let prod_id = Uuid::new_v4();

    let res = sqlx::query_as::<_, Product>(
        "INSERT INTO products (
            id, category_id, base_unit_id, sku, barcode, name, brand_id, costing_method,
            cost_price, base_selling_price, min_stock, max_stock, tax_percent, is_weight_embedded, image_url,
            auto_adjust_selling_price, markup_percent
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
        RETURNING *",
    )
    .bind(prod_id)
    .bind(payload.category_id)
    .bind(payload.base_unit_id)
    .bind(&payload.sku)
    .bind(&payload.barcode)
    .bind(&payload.name)
    .bind(payload.brand_id)
    .bind(payload.costing_method.as_deref().unwrap_or("GLOBAL"))
    .bind(payload.cost_price)
    .bind(payload.base_selling_price)
    .bind(payload.min_stock.unwrap_or(Decimal::ZERO))
    .bind(payload.max_stock.unwrap_or(Decimal::ZERO))
    .bind(payload.tax_percent.unwrap_or_else(|| Decimal::from(11)))
    .bind(payload.is_weight_embedded.unwrap_or(false))
    .bind(payload.image_url)
    .bind(payload.auto_adjust_selling_price.unwrap_or(false))
    .bind(payload.markup_percent)
    .fetch_one(&mut *tx)
    .await;

    let product = match res {
        Ok(p) => p,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    };

    if let (Some(stock), Some(wh_id)) = (payload.initial_stock, payload.warehouse_id) {
        if stock > Decimal::ZERO {
            sqlx::query(
                "INSERT INTO warehouse_stocks (warehouse_id, product_id, quantity) VALUES ($1, $2, $3)",
            )
            .bind(wh_id)
            .bind(prod_id)
            .bind(stock)
            .execute(&mut *tx).await.ok();

            sqlx::query(
                "INSERT INTO stock_mutations (
                    warehouse_id, product_id, reference_type, reference_id, qty_in, qty_out, balance, cost_price, notes
                ) VALUES ($1, $2, 'INITIAL_STOCK', $3, $4, 0, $4, $5, 'Saldo Awal Produk')",
            )
            .bind(wh_id)
            .bind(prod_id)
            .bind(prod_id)
            .bind(stock)
            .bind(payload.cost_price)
            .execute(&mut *tx).await.ok();

            // Simpan initial batch untuk pelacakan FIFO
            let _ = sqlx::query(
                "INSERT INTO product_stock_batches (
                    product_id, outlet_id, batch_number, initial_qty, remaining_qty,
                    cost_price, reference_type, received_at
                ) VALUES ($1, $2, 'SALDO-AWAL', $3, $3, $4, 'INITIAL_STOCK', CURRENT_TIMESTAMP)"
            )
            .bind(prod_id)
            .bind(wh_id)
            .bind(stock)
            .bind(payload.cost_price)
            .execute(&mut *tx).await;

            // Inisialisasi harga khusus outlet awal
            let _ = sqlx::query(
                "INSERT INTO product_outlet_prices (
                    outlet_id, product_id, cost_price, base_selling_price, last_purchase_price
                ) VALUES ($1, $2, $3, $4, $3)
                ON CONFLICT (outlet_id, product_id) DO NOTHING"
            )
            .bind(wh_id)
            .bind(prod_id)
            .bind(payload.cost_price)
            .bind(payload.base_selling_price)
            .execute(&mut *tx).await;

            // Buat Jurnal Akuntansi Saldo Awal Persediaan
            if let Err(e) = record_initial_stock_journal(
                &mut tx,
                wh_id,
                prod_id,
                &payload.name,
                &payload.sku,
                stock,
                payload.cost_price,
                user_id,
            )
            .await
            {
                tracing::error!("Gagal membuat jurnal saldo awal produk {}: {}", payload.sku, e);
            }
        }
    }

    if let Err(e) = tx.commit().await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response();
    }

    (StatusCode::CREATED, Json(json!({ "success": true, "data": product }))).into_response()
}

#[derive(Deserialize)]
struct UpdateProductPayload {
    category_id: Uuid,
    base_unit_id: Uuid,
    sku: String,
    barcode: String,
    name: String,
    brand_id: Option<Uuid>,
    cost_price: Decimal,
    base_selling_price: Decimal,
    min_stock: Option<Decimal>,
    max_stock: Option<Decimal>,
    tax_percent: Option<Decimal>,
    is_weight_embedded: Option<bool>,
    image_url: Option<String>,
    costing_method: Option<String>,
    auto_adjust_selling_price: Option<bool>,
    markup_percent: Option<Decimal>,
}

async fn update_product_handler(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateProductPayload>,
) -> impl IntoResponse {
    let res = sqlx::query_as::<_, Product>(
        "UPDATE products SET
            category_id = $1,
            base_unit_id = $2,
            sku = $3,
            barcode = $4,
            name = $5,
            brand_id = $6,
            cost_price = $7,
            base_selling_price = $8,
            min_stock = $9,
            max_stock = $10,
            tax_percent = $11,
            is_weight_embedded = $12,
            image_url = $13,
            costing_method = COALESCE($14, costing_method),
            auto_adjust_selling_price = COALESCE($15, auto_adjust_selling_price),
            markup_percent = $16,
            updated_at = CURRENT_TIMESTAMP
        WHERE id = $17
        RETURNING *",
    )
    .bind(payload.category_id)
    .bind(payload.base_unit_id)
    .bind(&payload.sku)
    .bind(&payload.barcode)
    .bind(&payload.name)
    .bind(payload.brand_id)
    .bind(payload.cost_price)
    .bind(payload.base_selling_price)
    .bind(payload.min_stock.unwrap_or(Decimal::ZERO))
    .bind(payload.max_stock.unwrap_or(Decimal::ZERO))
    .bind(payload.tax_percent.unwrap_or_else(|| Decimal::from(11)))
    .bind(payload.is_weight_embedded.unwrap_or(false))
    .bind(payload.image_url)
    .bind(payload.costing_method)
    .bind(payload.auto_adjust_selling_price)
    .bind(payload.markup_percent)
    .bind(id)
    .fetch_one(&pool)
    .await;

    match res {
        Ok(p) => (StatusCode::OK, Json(json!({ "success": true, "data": p }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn delete_product_handler(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let res = sqlx::query("UPDATE products SET is_active = false, updated_at = CURRENT_TIMESTAMP WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await;

    match res {
        Ok(r) if r.rows_affected() > 0 => (StatusCode::OK, Json(json!({ "success": true, "message": "Produk berhasil dinonaktifkan" }))).into_response(),
        Ok(_) => (StatusCode::NOT_FOUND, Json(json!({ "success": false, "message": "Produk tidak ditemukan" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

#[derive(Deserialize)]
struct AddProductUnitPayload {
    unit_id: Uuid,
    conversion_factor: Decimal,
    selling_price: Decimal,
    barcode: Option<String>,
}

async fn add_product_unit_handler(
    State(pool): State<PgPool>,
    Path(product_id): Path<Uuid>,
    Json(payload): Json<AddProductUnitPayload>,
) -> impl IntoResponse {
    let res = sqlx::query(
        "INSERT INTO product_units (product_id, unit_id, conversion_factor, selling_price, barcode)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (product_id, unit_id) DO UPDATE SET
            conversion_factor = EXCLUDED.conversion_factor,
            selling_price = EXCLUDED.selling_price,
            barcode = EXCLUDED.barcode,
            updated_at = CURRENT_TIMESTAMP
         RETURNING id",
    )
    .bind(product_id)
    .bind(payload.unit_id)
    .bind(payload.conversion_factor)
    .bind(payload.selling_price)
    .bind(payload.barcode)
    .fetch_one(&pool)
    .await;

    match res {
        Ok(r) => (StatusCode::CREATED, Json(json!({ "success": true, "id": r.get::<Uuid, _>("id") }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn delete_product_unit_handler(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match sqlx::query("DELETE FROM product_units WHERE id = $1").bind(id).execute(&pool).await {
        Ok(_) => (StatusCode::OK, Json(json!({ "success": true, "message": "Satuan konversi berhasil dihapus" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

#[derive(Deserialize)]
struct AddProductTierPayload {
    min_quantity: Decimal,
    tier_price: Decimal,
}

async fn add_product_tier_handler(
    State(pool): State<PgPool>,
    Path(product_id): Path<Uuid>,
    Json(payload): Json<AddProductTierPayload>,
) -> impl IntoResponse {
    if payload.min_quantity <= Decimal::ONE {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "Kuantitas minimum grosir harus lebih besar dari 1" })),
        ).into_response();
    }
    if payload.tier_price <= Decimal::ZERO {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "Harga tier grosir harus lebih besar dari 0" })),
        ).into_response();
    }

    let res = sqlx::query(
        "INSERT INTO product_price_tiers (product_id, min_quantity, tier_price)
         VALUES ($1, $2, $3)
         RETURNING id",
    )
    .bind(product_id)
    .bind(payload.min_quantity)
    .bind(payload.tier_price)
    .fetch_one(&pool)
    .await;

    match res {
        Ok(r) => (StatusCode::CREATED, Json(json!({ "success": true, "id": r.get::<Uuid, _>("id") }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn delete_product_tier_handler(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match sqlx::query("DELETE FROM product_price_tiers WHERE id = $1").bind(id).execute(&pool).await {
        Ok(_) => (StatusCode::OK, Json(json!({ "success": true, "message": "Tier harga grosir berhasil dihapus" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

#[derive(Deserialize)]
struct SaveProductGroupPricePayload {
    customer_group_id: i32,
    special_price: Decimal,
}

async fn get_product_group_prices_handler(
    State(pool): State<PgPool>,
    Path(product_id): Path<Uuid>,
) -> impl IntoResponse {
    let rows = sqlx::query(
        "SELECT cgp.id, cgp.customer_group_id, cg.name as customer_group_name, cgp.product_id, cgp.special_price
         FROM customer_group_prices cgp
         JOIN customer_groups cg ON cgp.customer_group_id = cg.id
         WHERE cgp.product_id = $1
         ORDER BY cg.id ASC",
    )
    .bind(product_id)
    .fetch_all(&pool)
    .await;

    match rows {
        Ok(r) => {
            let list: Vec<serde_json::Value> = r
                .iter()
                .map(|row| {
                    json!({
                        "id": row.get::<Uuid, _>("id"),
                        "customer_group_id": row.get::<i32, _>("customer_group_id"),
                        "customer_group_name": row.get::<String, _>("customer_group_name"),
                        "product_id": row.get::<Uuid, _>("product_id"),
                        "special_price": row.get::<Decimal, _>("special_price"),
                    })
                })
                .collect();
            (StatusCode::OK, Json(json!({ "success": true, "data": list }))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn save_product_group_price_handler(
    State(pool): State<PgPool>,
    Path(product_id): Path<Uuid>,
    Json(payload): Json<SaveProductGroupPricePayload>,
) -> impl IntoResponse {
    if payload.customer_group_id <= 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "ID kelompok pelanggan tidak valid" })),
        ).into_response();
    }
    if payload.special_price <= Decimal::ZERO {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "Harga khusus kelompok pelanggan harus lebih besar dari 0" })),
        ).into_response();
    }

    let res = sqlx::query(
        "INSERT INTO customer_group_prices (customer_group_id, product_id, special_price)
         VALUES ($1, $2, $3)
         ON CONFLICT (customer_group_id, product_id) DO UPDATE SET
            special_price = EXCLUDED.special_price,
            updated_at = CURRENT_TIMESTAMP
         RETURNING id",
    )
    .bind(payload.customer_group_id)
    .bind(product_id)
    .bind(payload.special_price)
    .fetch_one(&pool)
    .await;

    match res {
        Ok(r) => (StatusCode::OK, Json(json!({ "success": true, "id": r.get::<Uuid, _>("id"), "message": "Harga kelompok pelanggan berhasil disimpan" }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn delete_product_group_price_handler(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match sqlx::query("DELETE FROM customer_group_prices WHERE id = $1").bind(id).execute(&pool).await {
        Ok(_) => (StatusCode::OK, Json(json!({ "success": true, "message": "Harga kelompok pelanggan berhasil dihapus" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

// ----------------------------------------------------------------------------
// Multi-Outlet Pricing Handlers
// ----------------------------------------------------------------------------

#[derive(Deserialize)]
struct SaveProductOutletPricePayload {
    outlet_id: Uuid,
    cost_price: Decimal,
    base_selling_price: Decimal,
}

async fn get_product_outlet_prices_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(product_id): Path<Uuid>,
) -> impl IntoResponse {
    let claims = match extract_claims(&headers) {
        Ok(c) => Some(c),
        Err(_) => None,
    };

    let rows = if let Some(c) = claims {
        if c.role_name == "SUPERADMIN" {
            sqlx::query(
                "SELECT pop.id, pop.outlet_id, o.code as outlet_code, o.name as outlet_name, o.is_warehouse,
                        pop.product_id, pop.cost_price, pop.base_selling_price, pop.last_purchase_price
                 FROM product_outlet_prices pop
                 JOIN outlets o ON pop.outlet_id = o.id
                 WHERE pop.product_id = $1 AND o.is_active = true
                 ORDER BY o.is_warehouse ASC, o.name ASC",
            )
            .bind(product_id)
            .fetch_all(&pool)
            .await
        } else {
            sqlx::query(
                "SELECT pop.id, pop.outlet_id, o.code as outlet_code, o.name as outlet_name, o.is_warehouse,
                        pop.product_id, pop.cost_price, pop.base_selling_price, pop.last_purchase_price
                 FROM product_outlet_prices pop
                 JOIN outlets o ON pop.outlet_id = o.id
                 JOIN user_outlets uo ON o.id = uo.outlet_id
                 WHERE pop.product_id = $1 AND uo.user_id = $2 AND o.is_active = true
                 ORDER BY o.is_warehouse ASC, o.name ASC",
            )
            .bind(product_id)
            .bind(c.sub)
            .fetch_all(&pool)
            .await
        }
    } else {
        sqlx::query(
            "SELECT pop.id, pop.outlet_id, o.code as outlet_code, o.name as outlet_name, o.is_warehouse,
                    pop.product_id, pop.cost_price, pop.base_selling_price, pop.last_purchase_price
             FROM product_outlet_prices pop
             JOIN outlets o ON pop.outlet_id = o.id
             WHERE pop.product_id = $1 AND o.is_active = true
             ORDER BY o.is_warehouse ASC, o.name ASC",
        )
        .bind(product_id)
        .fetch_all(&pool)
        .await
    };

    match rows {
        Ok(r) => {
            let list: Vec<serde_json::Value> = r
                .iter()
                .map(|row| {
                    json!({
                        "id": row.get::<Uuid, _>("id"),
                        "outlet_id": row.get::<Uuid, _>("outlet_id"),
                        "outlet_code": row.get::<String, _>("outlet_code"),
                        "outlet_name": row.get::<String, _>("outlet_name"),
                        "is_warehouse": row.get::<bool, _>("is_warehouse"),
                        "product_id": row.get::<Uuid, _>("product_id"),
                        "cost_price": row.get::<Decimal, _>("cost_price"),
                        "base_selling_price": row.get::<Decimal, _>("base_selling_price"),
                        "last_purchase_price": row.get::<Option<Decimal>, _>("last_purchase_price"),
                    })
                })
                .collect();
            (StatusCode::OK, Json(json!({ "success": true, "data": list }))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn save_product_outlet_price_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(product_id): Path<Uuid>,
    Json(payload): Json<SaveProductOutletPricePayload>,
) -> impl IntoResponse {
    if payload.base_selling_price <= Decimal::ZERO {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "Harga jual outlet harus lebih besar dari 0" })),
        ).into_response();
    }
    if payload.cost_price < Decimal::ZERO {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "HPP tidak boleh bernilai negatif" })),
        ).into_response();
    }

    // 1. Verify user outlet permission (SUPERADMIN has access to all)
    let claims = match extract_claims(&headers) {
        Ok(c) => c,
        Err(resp) => return resp.into_response(),
    };

    if claims.role_name != "SUPERADMIN" {
        let has_outlet_access: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM user_outlets WHERE user_id = $1 AND outlet_id = $2)"
        )
        .bind(claims.sub)
        .bind(payload.outlet_id)
        .fetch_one(&pool)
        .await
        .unwrap_or(false);

        if !has_outlet_access {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "success": false,
                    "message": "Akses ditolak: Anda tidak memiliki wewenang untuk mengatur harga di outlet ini"
                })),
            ).into_response();
        }
    }

    let res = sqlx::query(
        "INSERT INTO product_outlet_prices (outlet_id, product_id, cost_price, base_selling_price)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (outlet_id, product_id) DO UPDATE SET
            cost_price = EXCLUDED.cost_price,
            base_selling_price = EXCLUDED.base_selling_price,
            updated_at = CURRENT_TIMESTAMP
         RETURNING id",
    )
    .bind(payload.outlet_id)
    .bind(product_id)
    .bind(payload.cost_price)
    .bind(payload.base_selling_price)
    .fetch_one(&pool)
    .await;

    match res {
        Ok(r) => (StatusCode::OK, Json(json!({ "success": true, "id": r.get::<Uuid, _>("id"), "message": "Harga khusus outlet berhasil disimpan" }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn delete_product_outlet_price_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    // 1. Verify user outlet permission
    let claims = match extract_claims(&headers) {
        Ok(c) => c,
        Err(resp) => return resp.into_response(),
    };

    if claims.role_name != "SUPERADMIN" {
        let target_outlet_id = sqlx::query_scalar::<_, Uuid>(
            "SELECT outlet_id FROM product_outlet_prices WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&pool)
        .await
        .unwrap_or(None);

        if let Some(oid) = target_outlet_id {
            let has_outlet_access: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM user_outlets WHERE user_id = $1 AND outlet_id = $2)"
            )
            .bind(claims.sub)
            .bind(oid)
            .fetch_one(&pool)
            .await
            .unwrap_or(false);

            if !has_outlet_access {
                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "success": false,
                        "message": "Akses ditolak: Anda tidak memiliki wewenang untuk menghapus harga di outlet ini"
                    })),
                ).into_response();
            }
        }
    }

    match sqlx::query("DELETE FROM product_outlet_prices WHERE id = $1").bind(id).execute(&pool).await {
        Ok(_) => (StatusCode::OK, Json(json!({ "success": true, "message": "Harga khusus outlet berhasil dihapus (kembali ke harga dasar produk)" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

#[derive(Deserialize)]
struct ProductBatchesQuery {
    outlet_id: Option<Uuid>,
}

async fn get_product_batches_handler(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Query(query): Query<ProductBatchesQuery>,
) -> impl IntoResponse {
    match crate::services::costing_service::get_product_stock_batches(&pool, id, query.outlet_id).await {
        Ok(batches) => (StatusCode::OK, Json(json!({ "success": true, "data": batches }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e }))).into_response(),
    }
}

async fn get_batch_sales_usage_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    if let Err(res) = require_any_permission(
        &pool,
        &headers,
        &["inventory:batch_tracking", "inventory:read", "pos:checkout", "sales:read"],
    ).await {
        return res.into_response();
    }

    match crate::services::pos_service::get_batch_sales_usage(&pool, id).await {
        Ok(summary) => (StatusCode::OK, Json(json!({ "success": true, "data": summary }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e }))).into_response(),
    }
}


// ============================================================================
// OUTLET CUSTOMER GROUP PRICES (HARGA MEMBER PER OUTLET)
// ============================================================================

#[derive(Deserialize)]
struct OutletCgpQuery {
    outlet_id: Option<Uuid>,
}

#[derive(Deserialize)]
struct SaveOutletCgpPayload {
    outlet_id: Uuid,
    customer_group_id: i32,
    special_price: Decimal,
}

async fn get_outlet_cgp_handler(
    State(pool): State<PgPool>,
    Path(product_id): Path<Uuid>,
    Query(query): Query<OutletCgpQuery>,
) -> impl IntoResponse {
    let rows = if let Some(oid) = query.outlet_id {
        sqlx::query(
            "SELECT ocgp.id, ocgp.outlet_id, o.code as outlet_code, o.name as outlet_name,
                    ocgp.customer_group_id, cg.name as customer_group_name,
                    ocgp.product_id, ocgp.special_price
             FROM outlet_customer_group_prices ocgp
             JOIN outlets o ON ocgp.outlet_id = o.id
             JOIN customer_groups cg ON ocgp.customer_group_id = cg.id
             WHERE ocgp.product_id = $1 AND ocgp.outlet_id = $2 AND cg.is_active = true
             ORDER BY cg.id ASC",
        )
        .bind(product_id)
        .bind(oid)
        .fetch_all(&pool)
        .await
    } else {
        sqlx::query(
            "SELECT ocgp.id, ocgp.outlet_id, o.code as outlet_code, o.name as outlet_name,
                    ocgp.customer_group_id, cg.name as customer_group_name,
                    ocgp.product_id, ocgp.special_price
             FROM outlet_customer_group_prices ocgp
             JOIN outlets o ON ocgp.outlet_id = o.id
             JOIN customer_groups cg ON ocgp.customer_group_id = cg.id
             WHERE ocgp.product_id = $1 AND cg.is_active = true
             ORDER BY o.name ASC, cg.id ASC",
        )
        .bind(product_id)
        .fetch_all(&pool)
        .await
    };

    match rows {
        Ok(r) => {
            let list: Vec<serde_json::Value> = r
                .iter()
                .map(|row| {
                    json!({
                        "id": row.get::<Uuid, _>("id"),
                        "outlet_id": row.get::<Uuid, _>("outlet_id"),
                        "outlet_code": row.get::<String, _>("outlet_code"),
                        "outlet_name": row.get::<String, _>("outlet_name"),
                        "customer_group_id": row.get::<i32, _>("customer_group_id"),
                        "customer_group_name": row.get::<String, _>("customer_group_name"),
                        "product_id": row.get::<Uuid, _>("product_id"),
                        "special_price": row.get::<Decimal, _>("special_price"),
                    })
                })
                .collect();
            (StatusCode::OK, Json(json!({ "success": true, "data": list }))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn save_outlet_cgp_handler(
    State(pool): State<PgPool>,
    Path(product_id): Path<Uuid>,
    Json(payload): Json<SaveOutletCgpPayload>,
) -> impl IntoResponse {
    if payload.special_price <= Decimal::ZERO {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "Harga khusus member outlet harus lebih besar dari 0" })),
        ).into_response();
    }
    if payload.customer_group_id <= 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "ID kelompok pelanggan tidak valid" })),
        ).into_response();
    }

    let res = sqlx::query(
        "INSERT INTO outlet_customer_group_prices (outlet_id, customer_group_id, product_id, special_price)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (outlet_id, customer_group_id, product_id) DO UPDATE SET
            special_price = EXCLUDED.special_price,
            updated_at = CURRENT_TIMESTAMP
         RETURNING id",
    )
    .bind(payload.outlet_id)
    .bind(payload.customer_group_id)
    .bind(product_id)
    .bind(payload.special_price)
    .fetch_one(&pool)
    .await;

    match res {
        Ok(r) => (StatusCode::OK, Json(json!({
            "success": true,
            "id": r.get::<Uuid, _>("id"),
            "message": "Harga member outlet berhasil disimpan"
        }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn delete_outlet_cgp_handler(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match sqlx::query("DELETE FROM outlet_customer_group_prices WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
    {
        Ok(_) => (StatusCode::OK, Json(json!({ "success": true, "message": "Harga member outlet berhasil dihapus" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

// ============================================================================
// 2. CUSTOMERS & CUSTOMER GROUPS CRUD
// ============================================================================

#[derive(Deserialize)]
struct CustomerQuery {
    search: Option<String>,
    group_id: Option<i32>,
    sort_by: Option<String>,
    sort_dir: Option<String>,
    page: Option<i64>,
    page_size: Option<i64>,
}

async fn list_customers_handler(
    State(pool): State<PgPool>,
    Query(q): Query<CustomerQuery>,
) -> impl IntoResponse {
    let mut filter_sql = " WHERE c.is_active = true".to_string();

    if let Some(gid) = q.group_id {
        filter_sql.push_str(&format!(" AND c.customer_group_id = {}", gid));
    }
    if let Some(s) = &q.search {
        if !s.trim().is_empty() {
            let clean = s.trim().replace('\'', "''");
            filter_sql.push_str(&format!(" AND (c.name ILIKE '%{}%' OR c.code ILIKE '%{}%' OR c.phone ILIKE '%{}%')", clean, clean, clean));
        }
    }

    let count_query = format!("SELECT COUNT(*)::bigint FROM customers c {}", filter_sql);
    let total_items: i64 = sqlx::query_scalar(&count_query)
        .fetch_one(&pool)
        .await
        .unwrap_or(0);

    let sort_col = match q.sort_by.as_deref() {
        Some("code") => "c.code",
        Some("phone") => "c.phone",
        Some("credit_limit") => "c.credit_limit",
        Some("reward_points") => "c.reward_points",
        Some("group_name") => "group_name",
        _ => "c.name",
    };
    let sort_dir = match q.sort_dir.as_deref() {
        Some("desc") | Some("DESC") => "DESC",
        _ => "ASC",
    };

    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(10).max(1);
    let offset = (page - 1) * page_size;

    let sql = format!(
        "SELECT c.*, cg.name as group_name 
         FROM customers c 
         LEFT JOIN customer_groups cg ON c.customer_group_id = cg.id 
         {} 
         ORDER BY {} {} 
         LIMIT {} OFFSET {}",
        filter_sql, sort_col, sort_dir, page_size, offset
    );

    let rows = sqlx::query(&sql).fetch_all(&pool).await;

    match rows {
        Ok(r) => {
            let list: Vec<serde_json::Value> = r.iter().map(|c| json!({
                "id": c.get::<Uuid, _>("id"),
                "customer_group_id": c.get::<Option<i32>, _>("customer_group_id"),
                "code": c.get::<String, _>("code"),
                "name": c.get::<String, _>("name"),
                "phone": c.get::<Option<String>, _>("phone"),
                "email": c.get::<Option<String>, _>("email"),
                "credit_limit": c.get::<Decimal, _>("credit_limit"),
                "reward_points": c.get::<i32, _>("reward_points"),
                "group_name": c.get::<Option<String>, _>("group_name").unwrap_or_else(|| "-".into()),
            })).collect();

            let total_pages = if total_items > 0 {
                ((total_items as f64) / (page_size as f64)).ceil() as i64
            } else {
                1
            };

            (StatusCode::OK, Json(json!({
                "success": true,
                "data": list,
                "total_items": total_items,
                "page": page,
                "page_size": page_size,
                "total_pages": total_pages,
            }))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

#[derive(Deserialize)]
struct CustomerInput {
    customer_group_id: Option<i32>,
    code: String,
    name: String,
    phone: Option<String>,
    email: Option<String>,
    credit_limit: Option<Decimal>,
}

async fn create_customer_handler(
    State(pool): State<PgPool>,
    Json(payload): Json<CustomerInput>,
) -> impl IntoResponse {
    let res = sqlx::query(
        "INSERT INTO customers (customer_group_id, code, name, phone, email, credit_limit, reward_points)
         VALUES ($1, $2, $3, $4, $5, $6, 0)
         RETURNING id, code, name",
    )
    .bind(payload.customer_group_id)
    .bind(&payload.code)
    .bind(&payload.name)
    .bind(payload.phone)
    .bind(payload.email)
    .bind(payload.credit_limit.unwrap_or(Decimal::ZERO))
    .fetch_one(&pool)
    .await;

    match res {
        Ok(r) => (StatusCode::CREATED, Json(json!({
            "success": true,
            "data": {
                "id": r.get::<Uuid, _>("id"),
                "code": r.get::<String, _>("code"),
                "name": r.get::<String, _>("name"),
            }
        }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn update_customer_handler(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Json(payload): Json<CustomerInput>,
) -> impl IntoResponse {
    let res = sqlx::query(
        "UPDATE customers SET
            customer_group_id = $1,
            code = $2,
            name = $3,
            phone = $4,
            email = $5,
            credit_limit = $6,
            updated_at = CURRENT_TIMESTAMP
         WHERE id = $7",
    )
    .bind(payload.customer_group_id)
    .bind(&payload.code)
    .bind(&payload.name)
    .bind(payload.phone)
    .bind(payload.email)
    .bind(payload.credit_limit.unwrap_or(Decimal::ZERO))
    .bind(id)
    .execute(&pool)
    .await;

    match res {
        Ok(r) if r.rows_affected() > 0 => (StatusCode::OK, Json(json!({ "success": true, "message": "Data pelanggan berhasil diperbarui" }))).into_response(),
        Ok(_) => (StatusCode::NOT_FOUND, Json(json!({ "success": false, "message": "Pelanggan tidak ditemukan" }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn delete_customer_handler(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match sqlx::query("UPDATE customers SET is_active = false, updated_at = CURRENT_TIMESTAMP WHERE id = $1").bind(id).execute(&pool).await {
        Ok(_) => (StatusCode::OK, Json(json!({ "success": true, "message": "Pelanggan berhasil dinonaktifkan" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct AllPointLedgersQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub customer_id: Option<Uuid>,
    pub search: Option<String>,
    pub transaction_type: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

async fn list_all_point_ledgers_handler(
    State(pool): State<PgPool>,
    Query(q): Query<AllPointLedgersQuery>,
) -> impl IntoResponse {
    let mut filter_clauses: Vec<String> = Vec::new();

    if let Some(cid) = q.customer_id {
        filter_clauses.push(format!("l.customer_id = '{}'", cid));
    }

    if let Some(tx_type) = &q.transaction_type {
        let clean = tx_type.trim();
        if !clean.is_empty() && clean != "ALL" {
            let sanitized = clean.replace('\'', "''");
            filter_clauses.push(format!("l.transaction_type = '{}'", sanitized));
        }
    }

    if let Some(s) = &q.search {
        let clean = s.trim();
        if !clean.is_empty() {
            let sanitized = clean.replace('\'', "''");
            filter_clauses.push(format!(
                "(c.name ILIKE '%{0}%' OR c.code ILIKE '%{0}%' OR c.phone ILIKE '%{0}%' OR s.invoice_number ILIKE '%{0}%' OR l.notes ILIKE '%{0}%')",
                sanitized
            ));
        }
    }

    if let Some(sd) = &q.start_date {
        let clean = sd.trim();
        if !clean.is_empty() {
            filter_clauses.push(format!("l.created_at >= '{} 00:00:00'", clean.replace('\'', "''")));
        }
    }

    if let Some(ed) = &q.end_date {
        let clean = ed.trim();
        if !clean.is_empty() {
            filter_clauses.push(format!("l.created_at <= '{} 23:59:59'", clean.replace('\'', "''")));
        }
    }

    let where_clause = if filter_clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", filter_clauses.join(" AND "))
    };

    // 1. Hitung total items
    let count_sql = format!(
        "SELECT COUNT(*)::bigint 
         FROM customer_point_ledgers l 
         LEFT JOIN customers c ON l.customer_id = c.id 
         LEFT JOIN sales s ON l.reference_id = s.id 
         {}",
        where_clause
    );

    let total_items: i64 = sqlx::query_scalar(&count_sql)
        .fetch_one(&pool)
        .await
        .unwrap_or(0);

    // 2. Summary stats
    let summary_sql = format!(
        "SELECT 
            COALESCE(SUM(l.points_in), 0)::bigint as total_earned,
            COALESCE(SUM(l.points_out), 0)::bigint as total_redeemed
         FROM customer_point_ledgers l 
         LEFT JOIN customers c ON l.customer_id = c.id 
         LEFT JOIN sales s ON l.reference_id = s.id 
         {}",
        where_clause
    );

    let (total_earned, total_redeemed) = match sqlx::query(&summary_sql).fetch_one(&pool).await {
        Ok(r) => (
            r.get::<i64, _>("total_earned"),
            r.get::<i64, _>("total_redeemed"),
        ),
        Err(_) => (0, 0),
    };

    let total_active_points: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(reward_points), 0)::bigint FROM customers WHERE is_active = true"
    )
    .fetch_one(&pool)
    .await
    .unwrap_or(0);

    // 3. Paging
    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(10).max(1);
    let offset = (page - 1) * page_size;

    let data_sql = format!(
        "SELECT 
            l.id,
            l.customer_id,
            c.code as customer_code,
            c.name as customer_name,
            c.phone as customer_phone,
            cg.name as customer_group_name,
            l.transaction_type,
            l.reference_id,
            s.invoice_number as reference_number,
            l.points_in,
            l.points_out,
            l.balance,
            l.notes,
            l.created_at,
            u.full_name as created_by_name,
            u.username as created_by_username
         FROM customer_point_ledgers l
         LEFT JOIN customers c ON l.customer_id = c.id
         LEFT JOIN customer_groups cg ON c.customer_group_id = cg.id
         LEFT JOIN sales s ON l.reference_id = s.id
         LEFT JOIN users u ON l.created_by = u.id
         {}
         ORDER BY l.created_at DESC, l.id DESC
         LIMIT {} OFFSET {}",
        where_clause, page_size, offset
    );

    let rows = sqlx::query(&data_sql).fetch_all(&pool).await;

    match rows {
        Ok(r) => {
            let ledgers: Vec<serde_json::Value> = r.iter().map(|row| {
                let created_at: DateTime<Utc> = row.get("created_at");
                json!({
                    "id": row.get::<i64, _>("id"),
                    "customer_id": row.get::<Uuid, _>("customer_id"),
                    "customer_code": row.get::<Option<String>, _>("customer_code").unwrap_or_default(),
                    "customer_name": row.get::<Option<String>, _>("customer_name").unwrap_or_default(),
                    "customer_phone": row.get::<Option<String>, _>("customer_phone"),
                    "customer_group_name": row.get::<Option<String>, _>("customer_group_name").unwrap_or_else(|| "-".into()),
                    "transaction_type": row.get::<String, _>("transaction_type"),
                    "reference_id": row.get::<Option<Uuid>, _>("reference_id"),
                    "reference_number": row.get::<Option<String>, _>("reference_number"),
                    "points_in": row.get::<i32, _>("points_in"),
                    "points_out": row.get::<i32, _>("points_out"),
                    "balance": row.get::<i32, _>("balance"),
                    "notes": row.get::<Option<String>, _>("notes"),
                    "created_at": created_at.to_rfc3339(),
                    "created_by_name": row.get::<Option<String>, _>("created_by_name"),
                    "created_by_username": row.get::<Option<String>, _>("created_by_username"),
                })
            }).collect();

            let total_pages = if total_items > 0 {
                ((total_items as f64) / (page_size as f64)).ceil() as i64
            } else {
                1
            };

            (StatusCode::OK, Json(json!({
                "success": true,
                "data": ledgers,
                "summary": {
                    "total_earned": total_earned,
                    "total_redeemed": total_redeemed,
                    "total_active_points": total_active_points,
                },
                "pagination": {
                    "page": page,
                    "page_size": page_size,
                    "total_items": total_items,
                    "total_pages": total_pages,
                }
            }))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct CustomerPointHistoryQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub transaction_type: Option<String>,
}

async fn get_customer_point_history_handler(
    State(pool): State<PgPool>,
    Path(customer_id): Path<Uuid>,
    Query(q): Query<CustomerPointHistoryQuery>,
) -> impl IntoResponse {
    // 1. Ambil data pelanggan & grup
    let customer_row = sqlx::query(
        "SELECT c.id, c.customer_group_id, c.code, c.name, c.phone, c.email, c.credit_limit, c.reward_points, cg.name as group_name
         FROM customers c
         LEFT JOIN customer_groups cg ON c.customer_group_id = cg.id
         WHERE c.id = $1"
    )
    .bind(customer_id)
    .fetch_optional(&pool)
    .await;

    let customer = match customer_row {
        Ok(Some(c)) => json!({
            "id": c.get::<Uuid, _>("id"),
            "customer_group_id": c.get::<Option<i32>, _>("customer_group_id"),
            "code": c.get::<String, _>("code"),
            "name": c.get::<String, _>("name"),
            "phone": c.get::<Option<String>, _>("phone"),
            "email": c.get::<Option<String>, _>("email"),
            "credit_limit": c.get::<Decimal, _>("credit_limit"),
            "reward_points": c.get::<i32, _>("reward_points"),
            "group_name": c.get::<Option<String>, _>("group_name").unwrap_or_else(|| "-".into()),
        }),
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "success": false, "message": "Pelanggan tidak ditemukan" })),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "success": false, "message": e.to_string() })),
            )
                .into_response();
        }
    };

    // 2. Ambil ringkasan total pendapatan dan penukaran poin
    let summary_row = sqlx::query(
        "SELECT 
            COALESCE(SUM(points_in), 0)::bigint as total_earned,
            COALESCE(SUM(points_out), 0)::bigint as total_redeemed
         FROM customer_point_ledgers
         WHERE customer_id = $1"
    )
    .bind(customer_id)
    .fetch_one(&pool)
    .await;

    let (total_earned, total_redeemed) = match summary_row {
        Ok(r) => (
            r.get::<i64, _>("total_earned"),
            r.get::<i64, _>("total_redeemed"),
        ),
        Err(_) => (0, 0),
    };

    // 3. Query ledger dengan pagination & filter tipe transaksi
    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(20).max(1);
    let offset = (page - 1) * page_size;

    let mut count_sql = "SELECT COUNT(*)::bigint FROM customer_point_ledgers WHERE customer_id = $1".to_string();
    let mut data_sql = "
        SELECT 
            l.id,
            l.customer_id,
            l.transaction_type,
            l.reference_id,
            s.invoice_number as reference_number,
            l.points_in,
            l.points_out,
            l.balance,
            l.notes,
            l.created_at,
            u.full_name as created_by_name,
            u.username as created_by_username
        FROM customer_point_ledgers l
        LEFT JOIN sales s ON l.reference_id = s.id
        LEFT JOIN users u ON l.created_by = u.id
        WHERE l.customer_id = $1
    ".to_string();

    if let Some(tx_type) = &q.transaction_type {
        let clean_tx = tx_type.trim();
        if !clean_tx.is_empty() && clean_tx != "ALL" {
            let sanitized = clean_tx.replace('\'', "''");
            count_sql.push_str(&format!(" AND transaction_type = '{}'", sanitized));
            data_sql.push_str(&format!(" AND l.transaction_type = '{}'", sanitized));
        }
    }

    let total_items: i64 = sqlx::query_scalar(&count_sql)
        .bind(customer_id)
        .fetch_one(&pool)
        .await
        .unwrap_or(0);

    data_sql.push_str(&format!(
        " ORDER BY l.created_at DESC, l.id DESC LIMIT {} OFFSET {}",
        page_size, offset
    ));

    let rows = sqlx::query(&data_sql)
        .bind(customer_id)
        .fetch_all(&pool)
        .await;

    match rows {
        Ok(r) => {
            let ledgers: Vec<serde_json::Value> = r.iter().map(|row| {
                let created_at: DateTime<Utc> = row.get("created_at");
                json!({
                    "id": row.get::<i64, _>("id"),
                    "customer_id": row.get::<Uuid, _>("customer_id"),
                    "transaction_type": row.get::<String, _>("transaction_type"),
                    "reference_id": row.get::<Option<Uuid>, _>("reference_id"),
                    "reference_number": row.get::<Option<String>, _>("reference_number"),
                    "points_in": row.get::<i32, _>("points_in"),
                    "points_out": row.get::<i32, _>("points_out"),
                    "balance": row.get::<i32, _>("balance"),
                    "notes": row.get::<Option<String>, _>("notes"),
                    "created_at": created_at.to_rfc3339(),
                    "created_by_name": row.get::<Option<String>, _>("created_by_name"),
                    "created_by_username": row.get::<Option<String>, _>("created_by_username"),
                })
            }).collect();

            let total_pages = if total_items > 0 {
                ((total_items as f64) / (page_size as f64)).ceil() as i64
            } else {
                1
            };

            let current_pts = customer.get("reward_points").and_then(|v| v.as_i64()).unwrap_or(0);

            (StatusCode::OK, Json(json!({
                "success": true,
                "data": {
                    "customer": customer,
                    "summary": {
                        "current_balance": current_pts,
                        "total_earned": total_earned,
                        "total_redeemed": total_redeemed,
                    },
                    "ledgers": ledgers,
                    "pagination": {
                        "page": page,
                        "page_size": page_size,
                        "total_items": total_items,
                        "total_pages": total_pages,
                    }
                }
            }))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct AdjustCustomerPointsRequest {
    pub adjustment_type: String, // "ADD" or "DEDUCT"
    pub points: i32,
    pub notes: String,
}

async fn adjust_customer_points_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(customer_id): Path<Uuid>,
    Json(payload): Json<AdjustCustomerPointsRequest>,
) -> impl IntoResponse {
    let claims = match extract_claims(&headers) {
        Ok(c) => c,
        Err((code, msg)) => return (code, msg).into_response(),
    };

    let adj_type = payload.adjustment_type.trim().to_uppercase();
    if adj_type != "ADD" && adj_type != "DEDUCT" {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "Tipe penyesuaian harus 'ADD' (penambahan) atau 'DEDUCT' (pemotongan)" })),
        )
            .into_response();
    }

    if payload.points <= 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "Jumlah poin penyesuaian harus lebih dari 0" })),
        )
            .into_response();
    }

    let notes = payload.notes.trim();
    if notes.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "Keterangan/alasan penyesuaian poin wajib diisi" })),
        )
            .into_response();
    }

    let mut tx = match pool.begin().await {
        Ok(t) => t,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    };

    let current_pts = match sqlx::query_scalar::<_, i32>(
        "SELECT reward_points FROM customers WHERE id = $1 FOR UPDATE"
    )
    .bind(customer_id)
    .fetch_optional(&mut *tx)
    .await {
        Ok(Some(pts)) => pts,
        Ok(None) => {
            let _ = tx.rollback().await;
            return (StatusCode::NOT_FOUND, Json(json!({ "success": false, "message": "Pelanggan tidak ditemukan" }))).into_response();
        }
        Err(e) => {
            let _ = tx.rollback().await;
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response();
        }
    };

    let (points_in, points_out, new_balance) = if adj_type == "ADD" {
        (payload.points, 0, current_pts + payload.points)
    } else {
        if current_pts < payload.points {
            let _ = tx.rollback().await;
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "message": format!("Saldo poin pelanggan ({}) tidak mencukupi untuk pemotongan ({} poin)", current_pts, payload.points)
                })),
            )
                .into_response();
        }
        (0, payload.points, current_pts - payload.points)
    };

    if let Err(e) = sqlx::query("UPDATE customers SET reward_points = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
        .bind(new_balance)
        .bind(customer_id)
        .execute(&mut *tx)
        .await {
        let _ = tx.rollback().await;
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response();
    }

    let insert_ledger = sqlx::query(
        "INSERT INTO customer_point_ledgers (
            customer_id, transaction_type, points_in, points_out, balance, notes, created_at, created_by
        ) VALUES ($1, 'CORRECTION', $2, $3, $4, $5, CURRENT_TIMESTAMP, $6)
        RETURNING id"
    )
    .bind(customer_id)
    .bind(points_in)
    .bind(points_out)
    .bind(new_balance)
    .bind(format!("Koreksi manual ({}): {}", if adj_type == "ADD" { format!("+{}", payload.points) } else { format!("-{}", payload.points) }, notes))
    .bind(claims.sub)
    .fetch_one(&mut *tx)
    .await;

    if let Err(e) = insert_ledger {
        let _ = tx.rollback().await;
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response();
    }

    if let Err(e) = tx.commit().await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response();
    }

    (StatusCode::OK, Json(json!({
        "success": true,
        "message": format!("Saldo poin berhasil disesuaikan menjadi {} poin", new_balance),
        "data": {
            "previous_balance": current_pts,
            "new_balance": new_balance,
            "adjustment_type": adj_type,
            "points": payload.points,
        }
    }))).into_response()
}

// --- Customer Groups ---

async fn list_customer_groups_handler(State(pool): State<PgPool>) -> impl IntoResponse {
    let rows = sqlx::query("SELECT * FROM customer_groups WHERE is_active = true ORDER BY id ASC").fetch_all(&pool).await;
    match rows {
        Ok(r) => {
            let list: Vec<serde_json::Value> = r.iter().map(|cg| json!({
                "id": cg.get::<i32, _>("id"),
                "name": cg.get::<String, _>("name"),
                "description": cg.get::<Option<String>, _>("description"),
            })).collect();
            (StatusCode::OK, Json(json!({ "success": true, "data": list }))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

#[derive(Deserialize)]
struct CustomerGroupInput {
    name: String,
    description: Option<String>,
}

async fn create_customer_group_handler(
    State(pool): State<PgPool>,
    Json(payload): Json<CustomerGroupInput>,
) -> impl IntoResponse {
    let res = sqlx::query(
        "INSERT INTO customer_groups (name, description) VALUES ($1, $2) RETURNING id, name",
    )
    .bind(&payload.name)
    .bind(payload.description)
    .fetch_one(&pool)
    .await;

    match res {
        Ok(r) => (StatusCode::CREATED, Json(json!({ "success": true, "id": r.get::<i32, _>("id") }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn update_customer_group_handler(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
    Json(payload): Json<CustomerGroupInput>,
) -> impl IntoResponse {
    let res = sqlx::query(
        "UPDATE customer_groups SET name = $1, description = $2, updated_at = CURRENT_TIMESTAMP WHERE id = $3",
    )
    .bind(&payload.name)
    .bind(payload.description)
    .bind(id)
    .execute(&pool)
    .await;

    match res {
        Ok(_) => (StatusCode::OK, Json(json!({ "success": true, "message": "Grup pelanggan diperbarui" }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn delete_customer_group_handler(
    State(pool): State<PgPool>,
    Path(id): Path<i32>,
) -> impl IntoResponse {
    match sqlx::query("UPDATE customer_groups SET is_active = false WHERE id = $1").bind(id).execute(&pool).await {
        Ok(_) => (StatusCode::OK, Json(json!({ "success": true, "message": "Grup pelanggan dinonaktifkan" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

// ============================================================================
// 3. VOUCHERS CRUD
// ============================================================================

async fn list_vouchers_handler(State(pool): State<PgPool>) -> impl IntoResponse {
    // Proaktif auto-expire status voucher jika tanggal masa berlaku telah lewat
    let _ = sqlx::query(
        "UPDATE vouchers 
         SET status = 'EXPIRED', updated_at = CURRENT_TIMESTAMP 
         WHERE status = 'ACTIVE' AND valid_until <= CURRENT_TIMESTAMP",
    )
    .execute(&pool)
    .await;

    let rows = sqlx::query("SELECT * FROM vouchers ORDER BY created_at DESC").fetch_all(&pool).await;
    match rows {
        Ok(r) => {
            let list: Vec<serde_json::Value> = r.iter().map(|v| json!({
                "id": v.get::<Uuid, _>("id"),
                "code": v.get::<String, _>("code"),
                "description": v.get::<Option<String>, _>("description"),
                "discount_type": v.get::<String, _>("discount_type"),
                "discount_value": v.get::<Decimal, _>("discount_value"),
                "min_purchase_amount": v.get::<Decimal, _>("min_purchase_amount"),
                "max_discount_amount": v.get::<Option<Decimal>, _>("max_discount_amount"),
                "valid_from": v.get::<DateTime<Utc>, _>("valid_from"),
                "valid_until": v.get::<DateTime<Utc>, _>("valid_until"),
                "quota": v.get::<Option<i32>, _>("quota").unwrap_or(0),
                "used_count": v.get::<Option<i32>, _>("used_count").unwrap_or(0),
                "status": v.get::<String, _>("status"),
            })).collect();
            (StatusCode::OK, Json(json!({ "success": true, "data": list }))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

#[derive(Deserialize)]
struct VoucherInput {
    code: String,
    description: Option<String>,
    discount_type: String, // PERCENTAGE, FIXED_AMOUNT
    discount_value: Decimal,
    min_purchase_amount: Option<Decimal>,
    max_discount_amount: Option<Decimal>,
    valid_from: DateTime<Utc>,
    valid_until: DateTime<Utc>,
    quota: Option<i32>,
    status: Option<String>,
}

async fn create_voucher_handler(
    State(pool): State<PgPool>,
    Json(payload): Json<VoucherInput>,
) -> impl IntoResponse {
    let clean_code = payload.code.trim().to_uppercase();
    let is_expired = payload.valid_until <= chrono::Utc::now();
    let effective_status = if is_expired {
        "EXPIRED".to_string()
    } else {
        payload.status.unwrap_or_else(|| "ACTIVE".into())
    };

    let res = sqlx::query(
        "INSERT INTO vouchers (code, description, discount_type, discount_value, min_purchase_amount, max_discount_amount, valid_from, valid_until, quota, used_count, status)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 0, $10)
         RETURNING id, code",
    )
    .bind(&clean_code)
    .bind(payload.description)
    .bind(&payload.discount_type)
    .bind(payload.discount_value)
    .bind(payload.min_purchase_amount.unwrap_or(Decimal::ZERO))
    .bind(payload.max_discount_amount)
    .bind(payload.valid_from)
    .bind(payload.valid_until)
    .bind(payload.quota.unwrap_or(0))
    .bind(&effective_status)
    .fetch_one(&pool)
    .await;

    match res {
        Ok(r) => (StatusCode::CREATED, Json(json!({ "success": true, "id": r.get::<Uuid, _>("id") }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn update_voucher_handler(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Json(payload): Json<VoucherInput>,
) -> impl IntoResponse {
    let clean_code = payload.code.trim().to_uppercase();
    let is_expired = payload.valid_until <= chrono::Utc::now();
    let effective_status = if is_expired {
        "EXPIRED".to_string()
    } else {
        payload.status.unwrap_or_else(|| "ACTIVE".into())
    };

    let res = sqlx::query(
        "UPDATE vouchers SET
            code = $1,
            description = $2,
            discount_type = $3,
            discount_value = $4,
            min_purchase_amount = $5,
            max_discount_amount = $6,
            valid_from = $7,
            valid_until = $8,
            quota = $9,
            status = $10,
            updated_at = CURRENT_TIMESTAMP
         WHERE id = $11",
    )
    .bind(&clean_code)
    .bind(payload.description)
    .bind(&payload.discount_type)
    .bind(payload.discount_value)
    .bind(payload.min_purchase_amount.unwrap_or(Decimal::ZERO))
    .bind(payload.max_discount_amount)
    .bind(payload.valid_from)
    .bind(payload.valid_until)
    .bind(payload.quota.unwrap_or(0))
    .bind(&effective_status)
    .bind(id)
    .execute(&pool)
    .await;

    match res {
        Ok(_) => (StatusCode::OK, Json(json!({ "success": true, "message": "Voucher berhasil diperbarui" }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn delete_voucher_handler(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match sqlx::query("UPDATE vouchers SET status = 'EXPIRED', updated_at = CURRENT_TIMESTAMP WHERE id = $1").bind(id).execute(&pool).await {
        Ok(_) => (StatusCode::OK, Json(json!({ "success": true, "message": "Voucher dinonaktifkan / kedaluwarsa" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn validate_voucher_handler(
    State(pool): State<PgPool>,
    Path(code): Path<String>,
) -> impl IntoResponse {
    let clean_code = code.trim().to_uppercase();
    let v = sqlx::query(
        "SELECT * FROM vouchers WHERE code = $1 AND status = 'ACTIVE' AND valid_from <= CURRENT_TIMESTAMP AND valid_until >= CURRENT_TIMESTAMP",
    )
    .bind(&clean_code)
    .fetch_optional(&pool)
    .await;

    match v {
        Ok(Some(v)) => {
            let quota: i32 = v.get::<Option<i32>, _>("quota").unwrap_or(0);
            let used_count: i32 = v.get::<Option<i32>, _>("used_count").unwrap_or(0);

            if quota > 0 && used_count >= quota {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "success": false,
                        "message": format!("Kuota penggunaan voucher telah habis (Kuota: {}, Terpakai: {})", quota, used_count)
                    })),
                ).into_response();
            }

            (StatusCode::OK, Json(json!({
                "success": true,
                "data": {
                    "id": v.get::<Uuid, _>("id"),
                    "code": v.get::<String, _>("code"),
                    "description": v.get::<Option<String>, _>("description"),
                    "discount_type": v.get::<String, _>("discount_type"),
                    "discount_value": v.get::<Decimal, _>("discount_value"),
                    "min_purchase_amount": v.get::<Decimal, _>("min_purchase_amount"),
                    "max_discount_amount": v.get::<Option<Decimal>, _>("max_discount_amount"),
                    "quota": quota,
                    "used_count": used_count,
                }
            }))).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({ "success": false, "message": "Kode voucher tidak ditemukan atau sudah kedaluwarsa" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn get_voucher_claims_handler(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let rows = sqlx::query(
        "SELECT 
            s.id,
            s.invoice_number,
            s.created_at,
            s.subtotal,
            s.discount_amount,
            s.total_amount,
            s.status,
            u.name as cashier_name,
            c.name as customer_name,
            o.name as outlet_name
         FROM sales s
         LEFT JOIN users u ON s.created_by = u.id
         LEFT JOIN customers c ON s.customer_id = c.id
         LEFT JOIN outlets o ON s.outlet_id = o.id
         WHERE s.voucher_id = $1
         ORDER BY s.created_at DESC"
    )
    .bind(id)
    .fetch_all(&pool)
    .await;

    match rows {
        Ok(r) => {
            let claims: Vec<serde_json::Value> = r.iter().map(|row| json!({
                "sale_id": row.get::<Uuid, _>("id"),
                "invoice_number": row.get::<String, _>("invoice_number"),
                "created_at": row.get::<DateTime<Utc>, _>("created_at"),
                "subtotal": row.get::<Decimal, _>("subtotal"),
                "discount_amount": row.get::<Decimal, _>("discount_amount"),
                "total_amount": row.get::<Decimal, _>("total_amount"),
                "status": row.get::<String, _>("status"),
                "cashier_name": row.get::<Option<String>, _>("cashier_name"),
                "customer_name": row.get::<Option<String>, _>("customer_name"),
                "outlet_name": row.get::<Option<String>, _>("outlet_name"),
            })).collect();

            (StatusCode::OK, Json(json!({ "success": true, "data": claims }))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

// ============================================================================
// 4. UNITS (SATUAN) CRUD
// ============================================================================

async fn list_units_handler(State(pool): State<PgPool>) -> impl IntoResponse {
    match sqlx::query_as::<_, Unit>("SELECT * FROM units ORDER BY code ASC").fetch_all(&pool).await {
        Ok(data) => (StatusCode::OK, Json(json!({ "success": true, "data": data }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

#[derive(Deserialize)]
struct UnitInput {
    code: String,
    name: String,
    is_fractional: Option<bool>,
}

async fn create_unit_handler(
    State(pool): State<PgPool>,
    Json(payload): Json<UnitInput>,
) -> impl IntoResponse {
    let res = sqlx::query_as::<_, Unit>(
        "INSERT INTO units (code, name, is_fractional) VALUES ($1, $2, $3) RETURNING *",
    )
    .bind(&payload.code)
    .bind(&payload.name)
    .bind(payload.is_fractional.unwrap_or(false))
    .fetch_one(&pool)
    .await;

    match res {
        Ok(u) => (StatusCode::CREATED, Json(json!({ "success": true, "data": u }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn update_unit_handler(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UnitInput>,
) -> impl IntoResponse {
    let res = sqlx::query_as::<_, Unit>(
        "UPDATE units SET code = $1, name = $2, is_fractional = $3, updated_at = CURRENT_TIMESTAMP WHERE id = $4 RETURNING *",
    )
    .bind(&payload.code)
    .bind(&payload.name)
    .bind(payload.is_fractional.unwrap_or(false))
    .bind(id)
    .fetch_one(&pool)
    .await;

    match res {
        Ok(u) => (StatusCode::OK, Json(json!({ "success": true, "data": u }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn delete_unit_handler(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match sqlx::query("DELETE FROM units WHERE id = $1").bind(id).execute(&pool).await {
        Ok(_) => (StatusCode::OK, Json(json!({ "success": true, "message": "Satuan berhasil dihapus" }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": format!("Satuan tidak bisa dihapus karena sedang digunakan oleh produk: {}", e) }))).into_response(),
    }
}

// ============================================================================
// 5. CATEGORIES (KATEGORI) CRUD
// ============================================================================

#[derive(Deserialize)]
struct CategoryQuery {
    search: Option<String>,
}

async fn list_categories_handler(
    State(pool): State<PgPool>,
    Query(query): Query<CategoryQuery>,
) -> impl IntoResponse {
    let sql = if let Some(s) = query.search {
        if !s.trim().is_empty() {
            let clean = s.trim().replace('\'', "''");
            format!("SELECT * FROM categories WHERE is_active = true AND (name ILIKE '%{}%' OR code ILIKE '%{}%') ORDER BY name ASC", clean, clean)
        } else {
            "SELECT * FROM categories WHERE is_active = true ORDER BY name ASC".to_string()
        }
    } else {
        "SELECT * FROM categories WHERE is_active = true ORDER BY name ASC".to_string()
    };

    match sqlx::query_as::<_, Category>(&sql).fetch_all(&pool).await {
        Ok(data) => (StatusCode::OK, Json(json!({ "success": true, "data": data }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

#[derive(Deserialize)]
struct CategoryInput {
    code: String,
    name: String,
    parent_id: Option<Uuid>,
}

async fn create_category_handler(
    State(pool): State<PgPool>,
    Json(payload): Json<CategoryInput>,
) -> impl IntoResponse {
    let res = sqlx::query_as::<_, Category>(
        "INSERT INTO categories (code, name, parent_id) VALUES ($1, $2, $3) RETURNING *",
    )
    .bind(&payload.code)
    .bind(&payload.name)
    .bind(payload.parent_id)
    .fetch_one(&pool)
    .await;

    match res {
        Ok(c) => (StatusCode::CREATED, Json(json!({ "success": true, "data": c }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn update_category_handler(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Json(payload): Json<CategoryInput>,
) -> impl IntoResponse {
    let res = sqlx::query_as::<_, Category>(
        "UPDATE categories SET code = $1, name = $2, parent_id = $3, updated_at = CURRENT_TIMESTAMP WHERE id = $4 RETURNING *",
    )
    .bind(&payload.code)
    .bind(&payload.name)
    .bind(payload.parent_id)
    .bind(id)
    .fetch_one(&pool)
    .await;

    match res {
        Ok(c) => (StatusCode::OK, Json(json!({ "success": true, "data": c }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn delete_category_handler(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match sqlx::query("UPDATE categories SET is_active = false, updated_at = CURRENT_TIMESTAMP WHERE id = $1").bind(id).execute(&pool).await {
        Ok(_) => (StatusCode::OK, Json(json!({ "success": true, "message": "Kategori berhasil dinonaktifkan" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

// ============================================================================
// 6. BRANDS (MEREK) CRUD
// ============================================================================

async fn list_brands_handler(State(pool): State<PgPool>) -> impl IntoResponse {
    match sqlx::query_as::<_, Brand>("SELECT * FROM brands WHERE is_active = true ORDER BY name ASC").fetch_all(&pool).await {
        Ok(data) => (StatusCode::OK, Json(json!({ "success": true, "data": data }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

#[derive(Deserialize)]
struct BrandInput {
    code: String,
    name: String,
    description: Option<String>,
}

async fn create_brand_handler(
    State(pool): State<PgPool>,
    Json(payload): Json<BrandInput>,
) -> impl IntoResponse {
    let res = sqlx::query_as::<_, Brand>(
        "INSERT INTO brands (code, name, description) VALUES ($1, $2, $3) RETURNING *",
    )
    .bind(&payload.code)
    .bind(&payload.name)
    .bind(payload.description)
    .fetch_one(&pool)
    .await;

    match res {
        Ok(b) => (StatusCode::CREATED, Json(json!({ "success": true, "data": b }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn update_brand_handler(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Json(payload): Json<BrandInput>,
) -> impl IntoResponse {
    let res = sqlx::query_as::<_, Brand>(
        "UPDATE brands SET code = $1, name = $2, description = $3, updated_at = CURRENT_TIMESTAMP WHERE id = $4 RETURNING *",
    )
    .bind(&payload.code)
    .bind(&payload.name)
    .bind(payload.description)
    .bind(id)
    .fetch_one(&pool)
    .await;

    match res {
        Ok(b) => (StatusCode::OK, Json(json!({ "success": true, "data": b }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn delete_brand_handler(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match sqlx::query("UPDATE brands SET is_active = false, updated_at = CURRENT_TIMESTAMP WHERE id = $1").bind(id).execute(&pool).await {
        Ok(_) => (StatusCode::OK, Json(json!({ "success": true, "message": "Merek berhasil dinonaktifkan" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

// ============================================================================
// 7. SUPPLIERS (PEMASOK) CRUD
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct SupplierQuery {
    pub search: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub limit: Option<i64>,
    pub sort_by: Option<String>,
    pub sort_dir: Option<String>,
}

async fn list_suppliers_handler(
    State(pool): State<PgPool>,
    Query(query): Query<SupplierQuery>,
) -> impl IntoResponse {
    let mut filter_sql = " WHERE is_active = true".to_string();
    if let Some(s) = &query.search {
        if !s.trim().is_empty() {
            let clean = s.trim().replace('\'', "''");
            filter_sql.push_str(&format!(
                " AND (name ILIKE '%{0}%' OR code ILIKE '%{0}%' OR contact_person ILIKE '%{0}%' OR phone ILIKE '%{0}%')",
                clean
            ));
        }
    }

    let count_sql = format!("SELECT COUNT(*)::bigint FROM suppliers {}", filter_sql);
    let total_items: i64 = sqlx::query_scalar(&count_sql).fetch_one(&pool).await.unwrap_or(0);

    let sort_col = match query.sort_by.as_deref() {
        Some("code") => "code",
        Some("name") => "name",
        Some("contact_person") => "contact_person",
        Some("phone") => "phone",
        Some("address") => "address",
        Some("term_of_payment_days") => "term_of_payment_days",
        _ => "name",
    };
    let sort_dir = match query.sort_dir.as_deref() {
        Some("desc") | Some("DESC") => "DESC",
        _ => "ASC",
    };

    let mut sql = format!("SELECT * FROM suppliers {} ORDER BY {} {}", filter_sql, sort_col, sort_dir);

    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(query.limit.unwrap_or(10)).max(1);

    if query.page.is_some() || query.page_size.is_some() {
        let offset = (page - 1) * page_size;
        sql.push_str(&format!(" LIMIT {} OFFSET {}", page_size, offset));
    } else if let Some(lim) = query.limit {
        if lim > 0 {
            sql.push_str(&format!(" LIMIT {}", lim));
        }
    }

    let rows = sqlx::query(&sql).fetch_all(&pool).await;
    match rows {
        Ok(r) => {
            let list: Vec<serde_json::Value> = r.iter().map(|s| json!({
                "id": s.get::<Uuid, _>("id"),
                "code": s.get::<String, _>("code"),
                "name": s.get::<String, _>("name"),
                "contact_person": s.get::<Option<String>, _>("contact_person"),
                "phone": s.get::<Option<String>, _>("phone"),
                "address": s.get::<Option<String>, _>("address"),
                "term_of_payment_days": s.get::<i32, _>("term_of_payment_days")
            })).collect();

            let total_pages = if total_items > 0 {
                ((total_items as f64) / (page_size as f64)).ceil() as i64
            } else {
                1
            };

            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "data": list,
                    "total_items": total_items,
                    "page": page,
                    "page_size": page_size,
                    "total_pages": total_pages,
                })),
            ).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

#[derive(Deserialize)]
struct SupplierInput {
    code: String,
    name: String,
    contact_person: Option<String>,
    phone: Option<String>,
    address: Option<String>,
    term_of_payment_days: Option<i32>,
}

async fn create_supplier_handler(
    State(pool): State<PgPool>,
    Json(payload): Json<SupplierInput>,
) -> impl IntoResponse {
    let res = sqlx::query(
        "INSERT INTO suppliers (code, name, contact_person, phone, address, term_of_payment_days)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id, code, name",
    )
    .bind(&payload.code)
    .bind(&payload.name)
    .bind(payload.contact_person)
    .bind(payload.phone)
    .bind(payload.address)
    .bind(payload.term_of_payment_days.unwrap_or(0))
    .fetch_one(&pool)
    .await;

    match res {
        Ok(r) => (StatusCode::CREATED, Json(json!({ "success": true, "id": r.get::<Uuid, _>("id") }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn update_supplier_handler(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Json(payload): Json<SupplierInput>,
) -> impl IntoResponse {
    let res = sqlx::query(
        "UPDATE suppliers SET
            code = $1,
            name = $2,
            contact_person = $3,
            phone = $4,
            address = $5,
            term_of_payment_days = $6,
            updated_at = CURRENT_TIMESTAMP
         WHERE id = $7",
    )
    .bind(&payload.code)
    .bind(&payload.name)
    .bind(payload.contact_person)
    .bind(payload.phone)
    .bind(payload.address)
    .bind(payload.term_of_payment_days.unwrap_or(0))
    .bind(id)
    .execute(&pool)
    .await;

    match res {
        Ok(_) => (StatusCode::OK, Json(json!({ "success": true, "message": "Pemasok berhasil diperbarui" }))).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn delete_supplier_handler(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match sqlx::query("UPDATE suppliers SET is_active = false, updated_at = CURRENT_TIMESTAMP WHERE id = $1").bind(id).execute(&pool).await {
        Ok(_) => (StatusCode::OK, Json(json!({ "success": true, "message": "Pemasok berhasil dinonaktifkan" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

// ============================================================================
// 8. OTHER MASTER (PAYMENT METHODS, REASON CODES, WAREHOUSES, OUTLETS)
// ============================================================================

async fn list_payment_methods_handler(State(pool): State<PgPool>) -> impl IntoResponse {
    let rows = sqlx::query("SELECT * FROM payment_methods WHERE is_active = true ORDER BY category, name ASC").fetch_all(&pool).await;
    match rows {
        Ok(r) => {
            let list: Vec<serde_json::Value> = r.iter().map(|p| json!({
                "id": p.get::<Uuid, _>("id"),
                "code": p.get::<String, _>("code"),
                "name": p.get::<String, _>("name"),
                "category": p.get::<String, _>("category"),
            })).collect();
            (StatusCode::OK, Json(json!({ "success": true, "data": list }))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn list_reason_codes_handler(State(pool): State<PgPool>) -> impl IntoResponse {
    let rows = sqlx::query("SELECT * FROM reason_codes WHERE is_active = true ORDER BY type, code ASC").fetch_all(&pool).await;
    match rows {
        Ok(r) => {
            let list: Vec<serde_json::Value> = r.iter().map(|rc| json!({
                "id": rc.get::<Uuid, _>("id"),
                "type": rc.get::<String, _>("type"),
                "code": rc.get::<String, _>("code"),
                "description": rc.get::<String, _>("description"),
            })).collect();
            (StatusCode::OK, Json(json!({ "success": true, "data": list }))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn list_warehouses_handler(State(pool): State<PgPool>) -> impl IntoResponse {
    let rows = sqlx::query("SELECT id, code, name, is_active FROM outlets WHERE is_warehouse = true AND is_active = true ORDER BY name ASC").fetch_all(&pool).await;
    match rows {
        Ok(r) => {
            let list: Vec<serde_json::Value> = r.iter().map(|w| json!({
                "id": w.get::<Uuid, _>("id"),
                "outlet_id": w.get::<Uuid, _>("id"),
                "code": w.get::<String, _>("code"),
                "name": w.get::<String, _>("name"),
            })).collect();
            (StatusCode::OK, Json(json!({ "success": true, "data": list }))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct OutletPayload {
    pub code: String,
    pub name: String,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub is_active: Option<bool>,
    pub is_warehouse: Option<bool>,
    pub manager_id: Option<Uuid>,
    pub supervisor_ids: Option<Vec<Uuid>>,
}

#[derive(Debug, Deserialize)]
pub struct ListOutletsQuery {
    pub is_warehouse: Option<bool>,
    pub status: Option<String>,
    pub is_active: Option<bool>,
    pub search: Option<String>,
    pub sort_by: Option<String>,
    pub sort_dir: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub limit: Option<i64>,
}

async fn list_outlets_handler(
    State(pool): State<PgPool>,
    Query(query): Query<ListOutletsQuery>,
) -> impl IntoResponse {
    let mut filter_sql = " WHERE 1=1".to_string();

    if let Some(iw) = query.is_warehouse {
        filter_sql.push_str(&format!(" AND o.is_warehouse = {}", iw));
    }
    if let Some(status) = &query.status {
        match status.to_uppercase().as_str() {
            "ACTIVE" => filter_sql.push_str(" AND o.is_active = true"),
            "INACTIVE" => filter_sql.push_str(" AND o.is_active = false"),
            _ => {}
        }
    } else if let Some(ia) = query.is_active {
        filter_sql.push_str(&format!(" AND o.is_active = {}", ia));
    }
    if let Some(s) = &query.search {
        if !s.trim().is_empty() {
            let sanitized = s.trim().replace('\'', "''");
            filter_sql.push_str(&format!(
                " AND (o.name ILIKE '%{0}%' OR o.code ILIKE '%{0}%' OR o.address ILIKE '%{0}%' OR o.phone ILIKE '%{0}%' OR u.full_name ILIKE '%{0}%')",
                sanitized
            ));
        }
    }

    // Hitung total data untuk server-side pagination
    let count_query = format!(
        "SELECT COUNT(*)::bigint FROM outlets o LEFT JOIN users u ON o.manager_id = u.id {}",
        filter_sql
    );
    let total_items: i64 = sqlx::query_scalar(&count_query)
        .fetch_one(&pool)
        .await
        .unwrap_or(0);

    let sort_col = match query.sort_by.as_deref() {
        Some("code") => "o.code",
        Some("name") => "o.name",
        Some("address") => "o.address",
        Some("phone") => "o.phone",
        Some("is_active") => "o.is_active",
        Some("is_warehouse") => "o.is_warehouse",
        Some("manager_name") => "u.full_name",
        Some("created_at") => "o.created_at",
        _ => "o.code",
    };
    let sort_dir = match query.sort_dir.as_deref() {
        Some("desc") | Some("DESC") => "DESC",
        _ => "ASC",
    };

    let mut sql = format!(
        "SELECT 
            o.id, o.code, o.name, o.address, o.phone, o.is_active, o.is_warehouse,
            o.manager_id, o.inventory_account_id, o.rak_account_id,
            u.full_name as manager_name,
            coa_inv.code as inventory_account_code,
            coa_inv.name as inventory_account_name,
            coa_rak.code as rak_account_code,
            coa_rak.name as rak_account_name
         FROM outlets o
         LEFT JOIN users u ON o.manager_id = u.id
         LEFT JOIN chart_of_accounts coa_inv ON o.inventory_account_id = coa_inv.id
         LEFT JOIN chart_of_accounts coa_rak ON o.rak_account_id = coa_rak.id
         {}
         ORDER BY {} {}",
        filter_sql, sort_col, sort_dir
    );

    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(query.limit.unwrap_or(10)).max(1);

    if query.page.is_some() || query.page_size.is_some() {
        let offset = (page - 1) * page_size;
        sql.push_str(&format!(" LIMIT {} OFFSET {}", page_size, offset));
    } else if let Some(lim) = query.limit {
        if lim > 0 {
            sql.push_str(&format!(" LIMIT {}", lim));
        }
    }

    let rows = sqlx::query(&sql)
        .fetch_all(&pool)
        .await;

    match rows {
        Ok(r) => {
            let mut list = Vec::new();
            for o in r {
                let oid: Uuid = o.get("id");

                let spv_rows = sqlx::query(
                    "SELECT u.id, u.username, u.full_name, r.name as role_name
                     FROM outlet_supervisors os
                     JOIN users u ON os.user_id = u.id
                     JOIN roles r ON u.role_id = r.id
                     WHERE os.outlet_id = $1
                     ORDER BY u.full_name ASC",
                )
                .bind(oid)
                .fetch_all(&pool)
                .await
                .unwrap_or_default();

                let supervisors: Vec<serde_json::Value> = spv_rows.iter().map(|s| json!({
                    "id": s.get::<Uuid, _>("id"),
                    "username": s.get::<String, _>("username"),
                    "full_name": s.get::<String, _>("full_name"),
                    "role_name": s.get::<String, _>("role_name"),
                })).collect();

                list.push(json!({
                    "id": oid,
                    "code": o.get::<String, _>("code"),
                    "name": o.get::<String, _>("name"),
                    "address": o.get::<Option<String>, _>("address"),
                    "phone": o.get::<Option<String>, _>("phone"),
                    "is_active": o.get::<bool, _>("is_active"),
                    "is_warehouse": o.get::<bool, _>("is_warehouse"),
                    "manager_id": o.get::<Option<Uuid>, _>("manager_id"),
                    "manager_name": o.get::<Option<String>, _>("manager_name"),
                    "inventory_account_id": o.get::<Option<Uuid>, _>("inventory_account_id"),
                    "inventory_account_code": o.get::<Option<String>, _>("inventory_account_code"),
                    "inventory_account_name": o.get::<Option<String>, _>("inventory_account_name"),
                    "rak_account_id": o.get::<Option<Uuid>, _>("rak_account_id"),
                    "rak_account_code": o.get::<Option<String>, _>("rak_account_code"),
                    "rak_account_name": o.get::<Option<String>, _>("rak_account_name"),
                    "supervisors": supervisors,
                    "supervisors_count": supervisors.len(),
                }));
            }

            let total_pages = if total_items > 0 {
                ((total_items as f64) / (page_size as f64)).ceil() as i64
            } else {
                1
            };

            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "data": list,
                    "total_items": total_items,
                    "page": page,
                    "page_size": page_size,
                    "total_pages": total_pages,
                })),
            ).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn get_outlet_handler(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let outlet = sqlx::query(
        "SELECT 
            o.id, o.code, o.name, o.address, o.phone, o.is_active, o.is_warehouse,
            o.manager_id, o.inventory_account_id, o.rak_account_id,
            u.full_name as manager_name,
            coa_inv.code as inventory_account_code,
            coa_inv.name as inventory_account_name,
            coa_rak.code as rak_account_code,
            coa_rak.name as rak_account_name
         FROM outlets o
         LEFT JOIN users u ON o.manager_id = u.id
         LEFT JOIN chart_of_accounts coa_inv ON o.inventory_account_id = coa_inv.id
         LEFT JOIN chart_of_accounts coa_rak ON o.rak_account_id = coa_rak.id
         WHERE o.id = $1",
    )
    .bind(id)
    .fetch_optional(&pool)
    .await;

    match outlet {
        Ok(Some(o)) => {
            let spv_rows = sqlx::query(
                "SELECT u.id, u.username, u.full_name, r.name as role_name
                 FROM outlet_supervisors os
                 JOIN users u ON os.user_id = u.id
                 JOIN roles r ON u.role_id = r.id
                 WHERE os.outlet_id = $1
                 ORDER BY u.full_name ASC",
            )
            .bind(id)
            .fetch_all(&pool)
            .await
            .unwrap_or_default();

            let supervisors: Vec<serde_json::Value> = spv_rows.iter().map(|s| json!({
                "id": s.get::<Uuid, _>("id"),
                "username": s.get::<String, _>("username"),
                "full_name": s.get::<String, _>("full_name"),
                "role_name": s.get::<String, _>("role_name"),
            })).collect();

            (StatusCode::OK, Json(json!({
                "success": true,
                "data": json!({
                    "id": id,
                    "code": o.get::<String, _>("code"),
                    "name": o.get::<String, _>("name"),
                    "address": o.get::<Option<String>, _>("address"),
                    "phone": o.get::<Option<String>, _>("phone"),
                    "is_active": o.get::<bool, _>("is_active"),
                    "is_warehouse": o.get::<bool, _>("is_warehouse"),
                    "manager_id": o.get::<Option<Uuid>, _>("manager_id"),
                    "manager_name": o.get::<Option<String>, _>("manager_name"),
                    "inventory_account_id": o.get::<Option<Uuid>, _>("inventory_account_id"),
                    "inventory_account_code": o.get::<Option<String>, _>("inventory_account_code"),
                    "inventory_account_name": o.get::<Option<String>, _>("inventory_account_name"),
                    "rak_account_id": o.get::<Option<Uuid>, _>("rak_account_id"),
                    "rak_account_code": o.get::<Option<String>, _>("rak_account_code"),
                    "rak_account_name": o.get::<Option<String>, _>("rak_account_name"),
                    "supervisors": supervisors,
                })
            }))).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({ "success": false, "message": "Outlet tidak ditemukan" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn create_outlet_handler(
    State(pool): State<PgPool>,
    Json(payload): Json<OutletPayload>,
) -> impl IntoResponse {
    let mut tx = match pool.begin().await {
        Ok(t) => t,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    };

    let outlet_id = Uuid::new_v4();
    let is_active = payload.is_active.unwrap_or(true);
    let is_warehouse = payload.is_warehouse.unwrap_or(false);

    let insert_res = sqlx::query(
        "INSERT INTO outlets (id, code, name, address, phone, is_active, is_warehouse, manager_id, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    )
    .bind(outlet_id)
    .bind(payload.code.trim())
    .bind(payload.name.trim())
    .bind(payload.address.as_deref().map(|s| s.trim()))
    .bind(payload.phone.as_deref().map(|s| s.trim()))
    .bind(is_active)
    .bind(is_warehouse)
    .bind(payload.manager_id)
    .execute(&mut *tx)
    .await;

    if let Err(e) = insert_res {
        return (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": format!("Gagal membuat outlet: {}", e) }))).into_response();
    }

    // Assign supervisors
    if let Some(spv_ids) = payload.supervisor_ids {
        for spv_id in spv_ids {
            sqlx::query(
                "INSERT INTO outlet_supervisors (outlet_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            )
            .bind(outlet_id)
            .bind(spv_id)
            .execute(&mut *tx)
            .await
            .ok();
        }
    }

    // Hubungkan ke akun master terpusat Persediaan (1201) dan RAK (1901)
    let master_inv: Option<(Uuid, String, String)> = sqlx::query_as(
        "SELECT id, code, name FROM chart_of_accounts WHERE code = '1201' LIMIT 1"
    )
    .fetch_optional(&mut *tx)
    .await
    .unwrap_or(None);

    let master_rak: Option<(Uuid, String, String)> = sqlx::query_as(
        "SELECT id, code, name FROM chart_of_accounts WHERE code = '1901' LIMIT 1"
    )
    .fetch_optional(&mut *tx)
    .await
    .unwrap_or(None);

    let created_inv_coa_id = master_inv.as_ref().map(|x| x.0);
    let created_inv_coa_code = master_inv.as_ref().map(|x| x.1.clone());
    let created_inv_coa_name = master_inv.as_ref().map(|x| x.2.clone());

    let created_rak_coa_id = master_rak.as_ref().map(|x| x.0);
    let created_rak_coa_code = master_rak.as_ref().map(|x| x.1.clone());
    let created_rak_coa_name = master_rak.as_ref().map(|x| x.2.clone());

    sqlx::query("UPDATE outlets SET inventory_account_id = $1, rak_account_id = $2 WHERE id = $3")
        .bind(created_inv_coa_id)
        .bind(created_rak_coa_id)
        .bind(outlet_id)
        .execute(&mut *tx)
        .await
        .ok();

    if let Err(e) = tx.commit().await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": format!("Gagal commit transaksi: {}", e) }))).into_response();
    }

    (StatusCode::CREATED, Json(json!({
        "success": true,
        "message": if is_warehouse { "Gudang (Warehouse) berhasil dibuat" } else { "Outlet berhasil dibuat" },
        "data": {
            "id": outlet_id,
            "code": payload.code,
            "name": payload.name,
            "is_warehouse": is_warehouse,
            "inventory_account_id": created_inv_coa_id,
            "inventory_account_code": created_inv_coa_code,
            "inventory_account_name": created_inv_coa_name,
            "rak_account_id": created_rak_coa_id,
            "rak_account_code": created_rak_coa_code,
            "rak_account_name": created_rak_coa_name,
        }
    }))).into_response()
}

async fn update_outlet_handler(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Json(payload): Json<OutletPayload>,
) -> impl IntoResponse {
    let mut tx = match pool.begin().await {
        Ok(t) => t,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    };

    let is_active = payload.is_active.unwrap_or(true);
    let is_warehouse = payload.is_warehouse.unwrap_or(false);

    let update_res = sqlx::query(
        "UPDATE outlets 
         SET code = $1, name = $2, address = $3, phone = $4, is_active = $5, 
             is_warehouse = $6, manager_id = $7, updated_at = CURRENT_TIMESTAMP
         WHERE id = $8",
    )
    .bind(payload.code.trim())
    .bind(payload.name.trim())
    .bind(payload.address.as_deref().map(|s| s.trim()))
    .bind(payload.phone.as_deref().map(|s| s.trim()))
    .bind(is_active)
    .bind(is_warehouse)
    .bind(payload.manager_id)
    .bind(id)
    .execute(&mut *tx)
    .await;

    if let Err(e) = update_res {
        return (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": format!("Gagal memperbarui outlet: {}", e) }))).into_response();
    }

    // Update supervisors
    if let Some(spv_ids) = payload.supervisor_ids {
        sqlx::query("DELETE FROM outlet_supervisors WHERE outlet_id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await
            .ok();

        for spv_id in spv_ids {
            sqlx::query(
                "INSERT INTO outlet_supervisors (outlet_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            )
            .bind(id)
            .bind(spv_id)
            .execute(&mut *tx)
            .await
            .ok();
        }
    }

    if let Err(e) = tx.commit().await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": format!("Gagal commit transaksi: {}", e) }))).into_response();
    }

    (StatusCode::OK, Json(json!({ "success": true, "message": "Outlet berhasil diperbarui" }))).into_response()
}

async fn delete_outlet_handler(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    // 1. Periksa riwayat transaksi operasional & akuntansi pada outlet ini
    let sales_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM sales WHERE outlet_id = $1 OR warehouse_id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap_or(0);

    let stock_transfers_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM stock_transfers WHERE from_outlet_id = $1 OR to_outlet_id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap_or(0);

    let purchases_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM purchase_invoices WHERE outlet_id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap_or(0);

    let shifts_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM cashier_shifts WHERE outlet_id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap_or(0);

    let journal_lines_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM journal_lines WHERE outlet_id = $1"
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap_or(0);

    let has_transactions = sales_count > 0 
        || stock_transfers_count > 0 
        || purchases_count > 0 
        || shifts_count > 0 
        || journal_lines_count > 0;

    let mut tx = match pool.begin().await {
        Ok(t) => t,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    };

    // JIKA TELAH MEMILIKI RIWAYAT TRANSAKSI:
    // Terapkan SOFT DELETE otomatis agar data historis laporan dan akuntansi tetap valid tanpa error foreign key
    if has_transactions {
        let upd = sqlx::query("UPDATE outlets SET is_active = FALSE, updated_at = CURRENT_TIMESTAMP WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await;

        if let Err(e) = upd {
            return (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": format!("Gagal menonaktifkan outlet: {}", e) }))).into_response();
        }

        if let Err(e) = tx.commit().await {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": format!("Gagal commit transaksi: {}", e) }))).into_response();
        }

        return (StatusCode::OK, Json(json!({
            "success": true,
            "soft_deleted": true,
            "message": "Outlet/Gudang memiliki riwayat transaksi operasional, sehingga dialihkan ke status Nonaktif (Soft Delete) demi menjaga integritas data riwayat dan akuntansi."
        }))).into_response();
    }

    // JIKA TIDAK MEMILIKI TRANSAKSI SAMA SEKALI (Outlet baru kosong):
    // Periksa apakah masih ada stok fisik barang tersisa
    let has_stocks = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM warehouse_stocks WHERE warehouse_id = $1 AND quantity > 0")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap_or(0);

    if has_stocks > 0 {
        return (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": "Outlet/Gudang tidak dapat dihapus karena masih memiliki sisa stok barang fisik." }))).into_response();
    }

    // Bersihkan stok kuantitas 0
    sqlx::query("DELETE FROM warehouse_stocks WHERE warehouse_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .ok();

    // Hapus relasi supervisor dan user_outlets
    sqlx::query("DELETE FROM outlet_supervisors WHERE outlet_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .ok();

    sqlx::query("DELETE FROM user_outlets WHERE outlet_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .ok();

    // Lepaskan referensi inventory_account_id dan rak_account_id
    sqlx::query("UPDATE outlets SET inventory_account_id = NULL, rak_account_id = NULL WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .ok();

    let del = sqlx::query("DELETE FROM outlets WHERE id = $1").bind(id).execute(&mut *tx).await;
    match del {
        Ok(_) => {
            if let Err(e) = tx.commit().await {
                (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": format!("Gagal commit transaksi: {}", e) }))).into_response()
            } else {
                (StatusCode::OK, Json(json!({ "success": true, "message": "Outlet/Gudang dan akun perkiraan terkait berhasil dihapus permanen" }))).into_response()
            }
        }
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": format!("Gagal menghapus outlet: {}", e) }))).into_response(),
    }
}

// ============================================================================
// 12. RECEIPT & STORE SETTINGS
// ============================================================================

async fn get_receipt_settings_handler(State(pool): State<PgPool>) -> impl IntoResponse {
    let row = sqlx::query_as::<_, ReceiptSettings>(
        "SELECT * FROM receipt_settings ORDER BY created_at ASC LIMIT 1"
    )
    .fetch_optional(&pool)
    .await;

    match row {
        Ok(Some(s)) => (StatusCode::OK, Json(json!({ "success": true, "data": s }))).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({ "success": false, "message": "Pengaturan struk belum ditemukan" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn save_receipt_settings_handler(
    State(pool): State<PgPool>,
    Json(payload): Json<UpdateReceiptSettingsRequest>,
) -> impl IntoResponse {
    let existing_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM receipt_settings ORDER BY created_at ASC LIMIT 1"
    )
    .fetch_optional(&pool)
    .await
    .unwrap_or(None);

    let paper_w = payload.paper_width.unwrap_or_else(|| "58mm".to_string());
    let show_cashier = payload.show_cashier_name.unwrap_or(true);
    let show_cust = payload.show_customer_info.unwrap_or(true);
    let show_points = payload.show_loyalty_points.unwrap_or(true);
    let show_tax = payload.show_tax_info.unwrap_or(true);
    let show_ph = payload.show_phone.unwrap_or(true);
    let show_addr = payload.show_address.unwrap_or(true);
    let show_barc = payload.show_barcode.unwrap_or(true);
    let show_fs = payload.show_flash_sale_discount.unwrap_or(true);
    let show_mem = payload.show_member_discount.unwrap_or(true);
    let show_tax_sum = payload.show_tax_summary.unwrap_or(true);
    let show_sav = payload.show_savings_summary.unwrap_or(true);
    let cash_rounding = payload.cash_rounding_mode.unwrap_or_else(|| "NONE".to_string());
    let max_cash_limit = payload.max_cash_drawer_limit.unwrap_or_else(|| rust_decimal::Decimal::from(2000000));
    let bio_mode = payload.supervisor_biometric_mode.unwrap_or_else(|| "OFF".to_string());
    let bio_policy = payload.supervisor_biometric_policy.unwrap_or_else(|| "OPTIONAL".to_string());
    let face_thresh = payload.face_match_threshold.unwrap_or_else(|| rust_decimal::Decimal::from(75));

    let result = if let Some(id) = existing_id {
        sqlx::query_as::<_, ReceiptSettings>(
            "UPDATE receipt_settings 
             SET store_name = $1, store_tagline = $2, store_address = $3, store_phone = $4,
                 store_website = $5, tax_id = $6, header_note = $7, footer_note_1 = $8,
                 footer_note_2 = $9, social_media = $10, show_cashier_name = $11,
                 show_customer_info = $12, show_loyalty_points = $13, show_tax_info = $14,
                 paper_width = $15, outlet_id = $16, show_phone = $17, show_address = $18,
                 show_barcode = $19, show_flash_sale_discount = $20, show_member_discount = $21,
                 show_tax_summary = $22, show_savings_summary = $23,
                 cash_rounding_mode = $24, max_cash_drawer_limit = $25, store_logo_url = $26,
                 supervisor_biometric_mode = $27, supervisor_biometric_policy = $28, face_match_threshold = $29,
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = $30
             RETURNING *"
        )
        .bind(&payload.store_name)
        .bind(&payload.store_tagline)
        .bind(&payload.store_address)
        .bind(&payload.store_phone)
        .bind(&payload.store_website)
        .bind(&payload.tax_id)
        .bind(&payload.header_note)
        .bind(&payload.footer_note_1)
        .bind(&payload.footer_note_2)
        .bind(&payload.social_media)
        .bind(show_cashier)
        .bind(show_cust)
        .bind(show_points)
        .bind(show_tax)
        .bind(&paper_w)
        .bind(payload.outlet_id)
        .bind(show_ph)
        .bind(show_addr)
        .bind(show_barc)
        .bind(show_fs)
        .bind(show_mem)
        .bind(show_tax_sum)
        .bind(show_sav)
        .bind(&cash_rounding)
        .bind(max_cash_limit)
        .bind(&payload.store_logo_url)
        .bind(&bio_mode)
        .bind(&bio_policy)
        .bind(face_thresh)
        .bind(id)
        .fetch_one(&pool)
        .await
    } else {
        sqlx::query_as::<_, ReceiptSettings>(
            "INSERT INTO receipt_settings (
                store_name, store_tagline, store_address, store_phone,
                store_website, tax_id, header_note, footer_note_1,
                footer_note_2, social_media, show_cashier_name,
                show_customer_info, show_loyalty_points, show_tax_info,
                paper_width, outlet_id, show_phone, show_address,
                show_barcode, show_flash_sale_discount, show_member_discount,
                show_tax_summary, show_savings_summary,
                cash_rounding_mode, max_cash_drawer_limit, store_logo_url,
                supervisor_biometric_mode, supervisor_biometric_policy, face_match_threshold
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26, $27, $28, $29)
             RETURNING *"
        )
        .bind(&payload.store_name)
        .bind(&payload.store_tagline)
        .bind(&payload.store_address)
        .bind(&payload.store_phone)
        .bind(&payload.store_website)
        .bind(&payload.tax_id)
        .bind(&payload.header_note)
        .bind(&payload.footer_note_1)
        .bind(&payload.footer_note_2)
        .bind(&payload.social_media)
        .bind(show_cashier)
        .bind(show_cust)
        .bind(show_points)
        .bind(show_tax)
        .bind(&paper_w)
        .bind(payload.outlet_id)
        .bind(show_ph)
        .bind(show_addr)
        .bind(show_barc)
        .bind(show_fs)
        .bind(show_mem)
        .bind(show_tax_sum)
        .bind(show_sav)
        .bind(&cash_rounding)
        .bind(max_cash_limit)
        .bind(&payload.store_logo_url)
        .bind(&bio_mode)
        .bind(&bio_policy)
        .bind(face_thresh)
        .fetch_one(&pool)
        .await
    };

    match result {
        Ok(s) => (StatusCode::OK, Json(json!({ "success": true, "message": "Pengaturan struk berhasil disimpan ke database", "data": s }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": format!("Gagal menyimpan pengaturan struk: {}", e) }))).into_response(),
    }
}

async fn get_biometric_settings_handler(State(pool): State<PgPool>) -> impl IntoResponse {
    let row = sqlx::query_as::<_, (String, String, rust_decimal::Decimal)>(
        "SELECT supervisor_biometric_mode, supervisor_biometric_policy, face_match_threshold 
         FROM receipt_settings ORDER BY created_at ASC LIMIT 1"
    )
    .fetch_optional(&pool)
    .await;

    match row {
        Ok(Some((mode, policy, threshold))) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "data": {
                    "supervisor_biometric_mode": mode,
                    "supervisor_biometric_policy": policy,
                    "face_match_threshold": threshold
                }
            })),
        ).into_response(),
        Ok(None) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "data": {
                    "supervisor_biometric_mode": "OFF",
                    "supervisor_biometric_policy": "OPTIONAL",
                    "face_match_threshold": 75.0
                }
            })),
        ).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": e.to_string() })),
        ).into_response(),
    }
}

async fn save_biometric_settings_handler(
    State(pool): State<PgPool>,
    Json(payload): Json<UpdateBiometricSettingsRequest>,
) -> impl IntoResponse {
    let mode = payload.supervisor_biometric_mode;
    let policy = payload.supervisor_biometric_policy.unwrap_or_else(|| "OPTIONAL".to_string());
    let threshold = payload.face_match_threshold.unwrap_or_else(|| rust_decimal::Decimal::from(75));

    let res = sqlx::query(
        "UPDATE receipt_settings 
         SET supervisor_biometric_mode = $1, supervisor_biometric_policy = $2, face_match_threshold = $3, updated_at = CURRENT_TIMESTAMP"
    )
    .bind(&mode)
    .bind(&policy)
    .bind(threshold)
    .execute(&pool)
    .await;

    match res {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "message": "Pengaturan biometrik supervisor berhasil diperbarui",
                "data": {
                    "supervisor_biometric_mode": mode,
                    "supervisor_biometric_policy": policy,
                    "face_match_threshold": threshold
                }
            })),
        ).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": format!("Gagal menyimpan pengaturan: {}", e) })),
        ).into_response(),
    }
}

// ============================================================================
// 12B. LOYALTY POINT SETTINGS
// ============================================================================

async fn get_loyalty_settings_handler(State(pool): State<PgPool>) -> impl IntoResponse {
    let row = sqlx::query_as::<_, LoyaltySettings>(
        "SELECT * FROM loyalty_settings ORDER BY created_at ASC LIMIT 1"
    )
    .fetch_optional(&pool)
    .await;

    match row {
        Ok(Some(s)) => (StatusCode::OK, Json(json!({ "success": true, "data": s }))).into_response(),
        Ok(None) => (StatusCode::OK, Json(json!({
            "success": true,
            "data": {
                "is_enabled": true,
                "point_value_rupiah": "100.00",
                "min_spend_per_point": "10000.00",
                "max_point_usage_percent": "50.00",
                "min_redeem_points": 1,
            }
        }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn save_loyalty_settings_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<UpdateLoyaltySettingsRequest>,
) -> impl IntoResponse {
    // 1. Pengecekan Permission di Backend (Fail-Closed)
    if let Err(resp) = require_any_permission(&pool, &headers, &["loyalty:manage", "settings:manage"]).await {
        return resp.into_response();
    }

    // 2. Validasi Sisi Backend & Batasan Nilai (Domain Validation)
    if let Some(val) = payload.point_value_rupiah {
        if val <= Decimal::ZERO || val > Decimal::from(1_000_000) {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "success": false, "message": "Nilai 1 poin dalam rupiah harus lebih besar dari 0 dan maksimal Rp 1.000.000" })),
            ).into_response();
        }
    }

    if let Some(min_s) = payload.min_spend_per_point {
        if min_s < Decimal::ZERO || min_s > Decimal::from(1_000_000_000) {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "success": false, "message": "Minimal belanja per poin tidak boleh negatif dan maksimal Rp 1.000.000.000" })),
            ).into_response();
        }
    }

    if let Some(max_p) = payload.max_point_usage_percent {
        if max_p < Decimal::ZERO || max_p > Decimal::from(100) {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "success": false, "message": "Batas persentase penggunaan poin harus berada di antara 0% dan 100%" })),
            ).into_response();
        }
    }

    if let Some(min_r) = payload.min_redeem_points {
        if min_r < 0 || min_r > 1_000_000 {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "success": false, "message": "Minimal penukaran poin tidak boleh negatif dan maksimal 1.000.000 poin" })),
            ).into_response();
        }
    }

    let existing_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM loyalty_settings ORDER BY created_at ASC LIMIT 1"
    )
    .fetch_optional(&pool)
    .await
    .unwrap_or(None);

    let is_enabled = payload.is_enabled.unwrap_or(true);
    let point_val = payload.point_value_rupiah.unwrap_or_else(|| Decimal::from(100));
    let min_spend = payload.min_spend_per_point.unwrap_or_else(|| Decimal::from(10000));
    let max_percent = payload.max_point_usage_percent.unwrap_or_else(|| Decimal::from(50));
    let min_redeem = payload.min_redeem_points.unwrap_or(1);

    let result = if let Some(id) = existing_id {
        sqlx::query_as::<_, LoyaltySettings>(
            "UPDATE loyalty_settings
             SET is_enabled = $1, point_value_rupiah = $2, min_spend_per_point = $3,
                 max_point_usage_percent = $4, min_redeem_points = $5, updated_at = CURRENT_TIMESTAMP
             WHERE id = $6
             RETURNING *"
        )
        .bind(is_enabled)
        .bind(point_val)
        .bind(min_spend)
        .bind(max_percent)
        .bind(min_redeem)
        .bind(id)
        .fetch_one(&pool)
        .await
    } else {
        sqlx::query_as::<_, LoyaltySettings>(
            "INSERT INTO loyalty_settings (
                is_enabled, point_value_rupiah, min_spend_per_point, max_point_usage_percent, min_redeem_points
             ) VALUES ($1, $2, $3, $4, $5)
             RETURNING *"
        )
        .bind(is_enabled)
        .bind(point_val)
        .bind(min_spend)
        .bind(max_percent)
        .bind(min_redeem)
        .fetch_one(&pool)
        .await
    };

    match result {
        Ok(s) => (StatusCode::OK, Json(json!({ "success": true, "message": "Pengaturan poin reward berhasil disimpan", "data": s }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": format!("Gagal menyimpan pengaturan poin reward: {}", e) }))).into_response(),
    }
}

async fn get_inventory_costing_settings_handler(State(pool): State<PgPool>) -> impl IntoResponse {
    let row = sqlx::query_as::<_, InventoryCostingSettings>(
        "SELECT id, default_costing_method, default_auto_adjust_selling_price, default_markup_percent, created_at, updated_at
         FROM inventory_costing_settings 
         ORDER BY created_at ASC 
         LIMIT 1"
    )
    .fetch_optional(&pool)
    .await;

    match row {
        Ok(Some(s)) => (StatusCode::OK, Json(json!({ "success": true, "data": s }))).into_response(),
        Ok(None) => (StatusCode::OK, Json(json!({
            "success": true,
            "data": {
                "default_costing_method": "WMA",
                "default_auto_adjust_selling_price": false,
                "default_markup_percent": 0
            }
        }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn save_inventory_costing_settings_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<UpdateInventoryCostingSettingsRequest>,
) -> impl IntoResponse {
    // 1. Pengecekan Permission di Backend (Fail-Closed)
    if let Err(resp) = require_any_permission(&pool, &headers, &["settings:manage", "master:write"]).await {
        return resp.into_response();
    }

    // 2. Validasi Sisi Backend (Domain Validation)
    let method = payload.default_costing_method.to_uppercase();
    if method != "WMA" && method != "FIFO" && method != "FEFO" && method != "REPLACEMENT_COST" {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "Metode HPP tidak valid. Pilih: 'WMA', 'FIFO', 'FEFO', atau 'REPLACEMENT_COST'" })),
        ).into_response();
    }

    if let Some(mp) = payload.default_markup_percent {
        if mp < Decimal::ZERO || mp > Decimal::from(10000) {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "success": false, "message": "Markup persen tidak boleh negatif atau lebih dari 10.000%" })),
            ).into_response();
        }
    }

    let existing_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM inventory_costing_settings ORDER BY created_at ASC LIMIT 1"
    )
    .fetch_optional(&pool)
    .await
    .unwrap_or(None);

    let auto_adj = payload.default_auto_adjust_selling_price.unwrap_or(false);

    let result = if let Some(id) = existing_id {
        sqlx::query_as::<_, InventoryCostingSettings>(
            "UPDATE inventory_costing_settings
             SET default_costing_method = $1, default_auto_adjust_selling_price = $2,
                 default_markup_percent = $3, updated_at = CURRENT_TIMESTAMP
             WHERE id = $4
             RETURNING *"
        )
        .bind(method)
        .bind(auto_adj)
        .bind(payload.default_markup_percent)
        .bind(id)
        .fetch_one(&pool)
        .await
    } else {
        sqlx::query_as::<_, InventoryCostingSettings>(
            "INSERT INTO inventory_costing_settings (
                default_costing_method, default_auto_adjust_selling_price, default_markup_percent
             ) VALUES ($1, $2, $3)
             RETURNING *"
        )
        .bind(method)
        .bind(auto_adj)
        .bind(payload.default_markup_percent)
        .fetch_one(&pool)
        .await
    };

    match result {
        Ok(s) => (StatusCode::OK, Json(json!({ "success": true, "data": s, "message": "Pengaturan global metode HPP berhasil disimpan" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct UploadProductPhotoPayload {
    pub filename: Option<String>,
    pub data_base64: String,
}

pub async fn save_product_image_from_base64(data_base64: &str, stem: Option<&str>) -> Result<(String, String, usize), String> {
    use base64::Engine;

    // Strip data URL prefix jika ada (e.g. "data:image/jpeg;base64,...")
    let b64_data = if let Some(comma_pos) = data_base64.find(',') {
        &data_base64[comma_pos + 1..]
    } else {
        data_base64
    };

    let image_bytes = base64::engine::general_purpose::STANDARD
        .decode(b64_data)
        .map_err(|e| format!("Data gambar base64 tidak valid: {}", e))?;

    // Deteksi ekstensi dari MIME type
    let ext = if data_base64.starts_with("data:image/png") {
        "png"
    } else if data_base64.starts_with("data:image/webp") {
        "webp"
    } else if data_base64.starts_with("data:image/gif") {
        "gif"
    } else {
        "jpg"
    };

    // Buat nama file unik dan aman
    let filename_stem = stem
        .and_then(|f| std::path::Path::new(f).file_stem()?.to_str())
        .unwrap_or("product");
    let safe_stem: String = filename_stem
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let unique_id = uuid::Uuid::new_v4().to_string().replace('-', "")[..8].to_string();
    let final_filename = format!("{}-{}.{}", safe_stem, unique_id, ext);

    // Tentukan direktori penyimpanan: ~/.notane/product_images/
    let images_dir = dirs::home_dir()
        .ok_or_else(|| "Tidak dapat menentukan direktori home".to_string())?
        .join(".notane")
        .join("product_images");

    tokio::fs::create_dir_all(&images_dir)
        .await
        .map_err(|e| format!("Gagal membuat direktori gambar: {}", e))?;

    let file_path = images_dir.join(&final_filename);
    tokio::fs::write(&file_path, &image_bytes)
        .await
        .map_err(|e| format!("Gagal menyimpan file gambar: {}", e))?;

    tracing::info!("Foto produk disimpan: {} ({} bytes)", file_path.display(), image_bytes.len());

    let image_url = format!("/uploads/product_images/{}", final_filename);
    Ok((image_url, final_filename, image_bytes.len()))
}

async fn upload_product_photo_handler(
    Json(payload): Json<UploadProductPhotoPayload>,
) -> impl IntoResponse {
    match save_product_image_from_base64(&payload.data_base64, payload.filename.as_deref()).await {
        Ok((image_url, final_filename, size_bytes)) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "image_url": image_url,
                "filename": final_filename,
                "size_bytes": size_bytes
            })),
        ).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": e })),
        ).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct ImportOutletPriceItem {
    pub outlet_id: Option<Uuid>,
    pub outlet_code: Option<String>,
    pub cost_price: Option<Decimal>,
    pub selling_price: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct ImportMemberPriceItem {
    pub customer_group_id: Option<i32>,
    pub customer_group_name: Option<String>,
    pub special_price: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct ImportOutletMemberPriceItem {
    pub outlet_id: Option<Uuid>,
    pub outlet_code: Option<String>,
    pub customer_group_id: Option<i32>,
    pub customer_group_name: Option<String>,
    pub special_price: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct ImportProductItem {
    pub sku: String,
    pub barcode: String,
    pub name: String,
    pub category_name: Option<String>,
    pub category_id: Option<Uuid>,
    pub unit_code: Option<String>,
    pub unit_id: Option<Uuid>,
    pub brand_name: Option<String>,
    pub brand_id: Option<Uuid>,
    pub cost_price: Option<Decimal>,
    pub base_selling_price: Decimal,
    pub min_stock: Option<Decimal>,
    pub max_stock: Option<Decimal>,
    pub tax_percent: Option<Decimal>,
    pub is_weight_embedded: Option<bool>,
    pub image_url: Option<String>,
    pub initial_stock: Option<Decimal>,
    pub warehouse_id: Option<Uuid>,

    pub outlet_prices: Option<Vec<ImportOutletPriceItem>>,
    pub member_prices: Option<Vec<ImportMemberPriceItem>>,
    pub outlet_member_prices: Option<Vec<ImportOutletMemberPriceItem>>,
}

#[derive(Debug, Deserialize)]
pub struct ImportProductsRequest {
    pub update_existing: Option<bool>,
    pub default_warehouse_id: Option<Uuid>,
    pub products: Vec<ImportProductItem>,
}

async fn import_products_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<ImportProductsRequest>,
) -> impl IntoResponse {
    let user_id = extract_claims(&headers).ok().map(|c| c.sub);
    let mut tx = match pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "success": false, "message": format!("Gagal memulai transaksi database: {}", e) })),
            )
                .into_response()
        }
    };

    let update_existing = payload.update_existing.unwrap_or(true);
    let mut inserted_count = 0usize;
    let mut updated_count = 0usize;
    let mut skipped_count = 0usize;
    let mut images_attached_count = 0usize;
    let mut outlet_prices_saved = 0usize;
    let mut member_prices_saved = 0usize;
    let mut errors: Vec<String> = Vec::new();

    // 1. Cache Categories
    let cat_rows = sqlx::query("SELECT id, name, code FROM categories")
        .fetch_all(&mut *tx)
        .await
        .unwrap_or_default();
    let mut cat_map: std::collections::HashMap<String, Uuid> = std::collections::HashMap::new();
    for row in &cat_rows {
        let id: Uuid = row.get("id");
        let name: String = row.get("name");
        let code: String = row.get("code");
        cat_map.insert(name.trim().to_lowercase(), id);
        cat_map.insert(code.trim().to_lowercase(), id);
    }
    let default_cat_id = cat_rows.first().map(|r| r.get::<Uuid, _>("id"));

    // 2. Cache Units
    let unit_rows = sqlx::query("SELECT id, name, code FROM units")
        .fetch_all(&mut *tx)
        .await
        .unwrap_or_default();
    let mut unit_map: std::collections::HashMap<String, Uuid> = std::collections::HashMap::new();
    for row in &unit_rows {
        let id: Uuid = row.get("id");
        let name: String = row.get("name");
        let code: String = row.get("code");
        unit_map.insert(name.trim().to_lowercase(), id);
        unit_map.insert(code.trim().to_lowercase(), id);
    }
    let default_unit_id = unit_rows.first().map(|r| r.get::<Uuid, _>("id"));

    // 3. Cache Outlets
    let outlet_rows = sqlx::query("SELECT id, code, name FROM outlets")
        .fetch_all(&mut *tx)
        .await
        .unwrap_or_default();
    let mut outlet_map: std::collections::HashMap<String, Uuid> = std::collections::HashMap::new();
    for row in &outlet_rows {
        let id: Uuid = row.get("id");
        let code: String = row.get("code");
        let name: String = row.get("name");
        outlet_map.insert(code.trim().to_lowercase(), id);
        outlet_map.insert(name.trim().to_lowercase(), id);
    }

    // 4. Cache Customer Groups
    let group_rows = sqlx::query("SELECT id, name FROM customer_groups")
        .fetch_all(&mut *tx)
        .await
        .unwrap_or_default();
    let mut group_map: std::collections::HashMap<String, i32> = std::collections::HashMap::new();
    for row in &group_rows {
        let id: i32 = row.get("id");
        let name: String = row.get("name");
        group_map.insert(name.trim().to_lowercase(), id);
        group_map.insert(format!("{}", id), id);
    }

    for item in payload.products {
        let sku = item.sku.trim();
        let barcode = item.barcode.trim();
        let name = item.name.trim();

        if sku.is_empty() || barcode.is_empty() || name.is_empty() {
            errors.push(format!("Baris diabaikan: SKU, Barcode, dan Nama wajib diisi (SKU: '{}')", sku));
            continue;
        }

        // Resolving Category (Auto-create if new name provided)
        let resolved_cat_id = if let Some(cid) = item.category_id {
            Some(cid)
        } else if let Some(ref cname) = item.category_name {
            let key = cname.trim().to_lowercase();
            if let Some(&found_id) = cat_map.get(&key) {
                Some(found_id)
            } else if !cname.trim().is_empty() {
                let safe_code = format!("CAT-{}", &Uuid::new_v4().to_string()[..6]).to_uppercase();
                let new_cat_res = sqlx::query(
                    "INSERT INTO categories (name, code) VALUES ($1, $2) RETURNING id"
                )
                .bind(cname.trim())
                .bind(&safe_code)
                .fetch_one(&mut *tx)
                .await;

                if let Ok(crow) = new_cat_res {
                    let new_id: Uuid = crow.get("id");
                    cat_map.insert(cname.trim().to_lowercase(), new_id);
                    cat_map.insert(safe_code.to_lowercase(), new_id);
                    Some(new_id)
                } else {
                    default_cat_id
                }
            } else {
                default_cat_id
            }
        } else {
            default_cat_id
        };

        let final_cat_id = match resolved_cat_id {
            Some(cid) => cid,
            None => {
                errors.push(format!("Produk '{}': Tidak ada kategori yang valid", name));
                continue;
            }
        };

        // Resolving Unit (Auto-create if new unit provided)
        let resolved_unit_id = if let Some(uid) = item.unit_id {
            Some(uid)
        } else if let Some(ref ucode) = item.unit_code {
            let key = ucode.trim().to_lowercase();
            if let Some(&found_id) = unit_map.get(&key) {
                Some(found_id)
            } else if !ucode.trim().is_empty() {
                let code_up = ucode.trim().to_uppercase();
                let new_unit_res = sqlx::query(
                    "INSERT INTO units (code, name) VALUES ($1, $2) RETURNING id"
                )
                .bind(&code_up)
                .bind(ucode.trim())
                .fetch_one(&mut *tx)
                .await;

                if let Ok(urow) = new_unit_res {
                    let new_id: Uuid = urow.get("id");
                    unit_map.insert(ucode.trim().to_lowercase(), new_id);
                    unit_map.insert(code_up.to_lowercase(), new_id);
                    Some(new_id)
                } else {
                    default_unit_id
                }
            } else {
                default_unit_id
            }
        } else {
            default_unit_id
        };

        let final_unit_id = match resolved_unit_id {
            Some(uid) => uid,
            None => {
                errors.push(format!("Produk '{}': Tidak ada satuan dasar yang valid", name));
                continue;
            }
        };

        let cost_price = item.cost_price.unwrap_or(Decimal::ZERO);
        let base_selling_price = item.base_selling_price;
        let min_stock = item.min_stock.unwrap_or(Decimal::ZERO);
        let max_stock = item.max_stock.unwrap_or(Decimal::ZERO);
        let tax_percent = item.tax_percent.unwrap_or_else(|| Decimal::from(11));
        let is_weight = item.is_weight_embedded.unwrap_or(false);
        let has_image = item.image_url.as_ref().map(|s| !s.trim().is_empty()).unwrap_or(false);

        let resolved_image_url: Option<String> = if let Some(ref img) = item.image_url {
            if img.starts_with("data:image/") {
                match save_product_image_from_base64(img, Some(sku)).await {
                    Ok((url, _, _)) => {
                        images_attached_count += 1;
                        Some(url)
                    }
                    Err(e) => {
                        tracing::warn!("Gagal menyimpan foto produk '{}': {}", name, e);
                        None
                    }
                }
            } else if !img.trim().is_empty() {
                images_attached_count += 1;
                Some(img.clone())
            } else {
                None
            }
        } else {
            None
        };

        // Check existing product by barcode or sku
        let existing_res = sqlx::query("SELECT id FROM products WHERE barcode = $1 OR sku = $2")
            .bind(barcode)
            .bind(sku)
            .fetch_optional(&mut *tx)
            .await;

        let existing_opt = match existing_res {
            Ok(opt) => opt,
            Err(e) => {
                errors.push(format!("Gagal memeriksa produk '{}': {}", name, e));
                continue;
            }
        };

        let target_product_id: Uuid;

        if let Some(existing_row) = existing_opt {
            if !update_existing {
                skipped_count += 1;
                continue;
            }
            let existing_id: Uuid = existing_row.get("id");
            let update_res = sqlx::query(
                "UPDATE products SET
                    category_id = $1,
                    base_unit_id = $2,
                    sku = $3,
                    barcode = $4,
                    name = $5,
                    cost_price = $6,
                    base_selling_price = $7,
                    min_stock = $8,
                    max_stock = $9,
                    tax_percent = $10,
                    is_weight_embedded = $11,
                    image_url = CASE WHEN $12::text IS NOT NULL AND $12::text <> '' THEN $12::text ELSE image_url END,
                    updated_at = CURRENT_TIMESTAMP
                WHERE id = $13"
            )
            .bind(final_cat_id)
            .bind(final_unit_id)
            .bind(sku)
            .bind(barcode)
            .bind(name)
            .bind(cost_price)
            .bind(base_selling_price)
            .bind(min_stock)
            .bind(max_stock)
            .bind(tax_percent)
            .bind(is_weight)
            .bind(&resolved_image_url)
            .bind(existing_id)
            .execute(&mut *tx)
            .await;

            if let Err(e) = update_res {
                errors.push(format!("Gagal memperbarui produk '{}': {}", name, e));
                continue;
            }

            target_product_id = existing_id;
            updated_count += 1;
        } else {
            let new_id = Uuid::new_v4();
            let insert_res = sqlx::query(
                "INSERT INTO products (
                    id, category_id, base_unit_id, sku, barcode, name,
                    costing_method, cost_price, base_selling_price, min_stock, max_stock,
                    tax_percent, is_weight_embedded, image_url
                ) VALUES ($1, $2, $3, $4, $5, $6, 'WMA', $7, $8, $9, $10, $11, $12, $13)"
            )
            .bind(new_id)
            .bind(final_cat_id)
            .bind(final_unit_id)
            .bind(sku)
            .bind(barcode)
            .bind(name)
            .bind(cost_price)
            .bind(base_selling_price)
            .bind(min_stock)
            .bind(max_stock)
            .bind(tax_percent)
            .bind(is_weight)
            .bind(&resolved_image_url)
            .execute(&mut *tx)
            .await;

            if let Err(e) = insert_res {
                errors.push(format!("Gagal menambahkan produk baru '{}': {}", name, e));
                continue;
            }

            target_product_id = new_id;
            inserted_count += 1;

            // Handle initial stock
            let wh_target = item.warehouse_id.or(payload.default_warehouse_id);
            if let (Some(init_stock), Some(wid)) = (item.initial_stock, wh_target) {
                if init_stock > Decimal::ZERO {
                    let _ = sqlx::query(
                        "INSERT INTO warehouse_stocks (warehouse_id, product_id, quantity) VALUES ($1, $2, $3)
                         ON CONFLICT (warehouse_id, product_id) DO UPDATE SET quantity = warehouse_stocks.quantity + EXCLUDED.quantity",
                    )
                    .bind(wid)
                    .bind(target_product_id)
                    .bind(init_stock)
                    .execute(&mut *tx)
                    .await;

                    let _ = sqlx::query(
                        "INSERT INTO stock_mutations (
                            warehouse_id, product_id, reference_type, reference_id, qty_in, qty_out, balance, cost_price, notes
                        ) VALUES ($1, $2, 'INITIAL_STOCK', $3, $4, 0, $4, $5, 'Import Saldo Awal')",
                    )
                    .bind(wid)
                    .bind(target_product_id)
                    .bind(target_product_id)
                    .bind(init_stock)
                    .bind(cost_price)
                    .execute(&mut *tx)
                    .await;

                    // Simpan initial batch untuk pelacakan FIFO
                    let _ = sqlx::query(
                        "INSERT INTO product_stock_batches (
                            product_id, outlet_id, batch_number, initial_qty, remaining_qty,
                            cost_price, reference_type, received_at
                        ) VALUES ($1, $2, 'SALDO-AWAL', $3, $3, $4, 'INITIAL_STOCK', CURRENT_TIMESTAMP)",
                    )
                    .bind(target_product_id)
                    .bind(wid)
                    .bind(init_stock)
                    .bind(cost_price)
                    .execute(&mut *tx)
                    .await;

                    // Buat Jurnal Akuntansi Saldo Awal Persediaan
                    if let Err(e) = record_initial_stock_journal(
                        &mut tx,
                        wid,
                        target_product_id,
                        name,
                        sku,
                        init_stock,
                        cost_price,
                        user_id,
                    )
                    .await
                    {
                        tracing::error!("Gagal membuat jurnal saldo awal import produk {}: {}", sku, e);
                    }
                }
            }
        }

        if has_image {
            images_attached_count += 1;
        }

        // 5. Handle Outlet Prices
        if let Some(ref outlet_price_list) = item.outlet_prices {
            for op in outlet_price_list {
                let target_outlet_id = if let Some(oid) = op.outlet_id {
                    Some(oid)
                } else if let Some(ref ocode) = op.outlet_code {
                    outlet_map.get(&ocode.trim().to_lowercase()).copied()
                } else {
                    None
                };

                if let Some(oid) = target_outlet_id {
                    let op_cost = op.cost_price.unwrap_or(cost_price);
                    let _ = sqlx::query(
                        "INSERT INTO product_outlet_prices (outlet_id, product_id, cost_price, base_selling_price)
                         VALUES ($1, $2, $3, $4)
                         ON CONFLICT (outlet_id, product_id) DO UPDATE SET
                            cost_price = EXCLUDED.cost_price,
                            base_selling_price = EXCLUDED.base_selling_price,
                            updated_at = CURRENT_TIMESTAMP"
                    )
                    .bind(oid)
                    .bind(target_product_id)
                    .bind(op_cost)
                    .bind(op.selling_price)
                    .execute(&mut *tx)
                    .await;

                    outlet_prices_saved += 1;
                }
            }
        }

        // 6. Handle Member Prices (Global Customer Group Prices)
        if let Some(ref member_price_list) = item.member_prices {
            for mp in member_price_list {
                let target_group_id = if let Some(gid) = mp.customer_group_id {
                    Some(gid)
                } else if let Some(ref gname) = mp.customer_group_name {
                    group_map.get(&gname.trim().to_lowercase()).copied()
                } else {
                    None
                };

                if let Some(gid) = target_group_id {
                    let _ = sqlx::query(
                        "INSERT INTO customer_group_prices (customer_group_id, product_id, special_price)
                         VALUES ($1, $2, $3)
                         ON CONFLICT (customer_group_id, product_id) DO UPDATE SET
                            special_price = EXCLUDED.special_price,
                            updated_at = CURRENT_TIMESTAMP"
                    )
                    .bind(gid)
                    .bind(target_product_id)
                    .bind(mp.special_price)
                    .execute(&mut *tx)
                    .await;

                    member_prices_saved += 1;
                }
            }
        }

        // 7. Handle Outlet Member Prices (Spesifik per Outlet dan Grup)
        if let Some(ref omp_list) = item.outlet_member_prices {
            for omp in omp_list {
                let target_outlet_id = if let Some(oid) = omp.outlet_id {
                    Some(oid)
                } else if let Some(ref ocode) = omp.outlet_code {
                    outlet_map.get(&ocode.trim().to_lowercase()).copied()
                } else {
                    None
                };

                let target_group_id = if let Some(gid) = omp.customer_group_id {
                    Some(gid)
                } else if let Some(ref gname) = omp.customer_group_name {
                    group_map.get(&gname.trim().to_lowercase()).copied()
                } else {
                    None
                };

                if let (Some(oid), Some(gid)) = (target_outlet_id, target_group_id) {
                    let _ = sqlx::query(
                        "INSERT INTO outlet_customer_group_prices (outlet_id, customer_group_id, product_id, special_price)
                         VALUES ($1, $2, $3, $4)
                         ON CONFLICT (outlet_id, customer_group_id, product_id) DO UPDATE SET
                            special_price = EXCLUDED.special_price,
                            updated_at = CURRENT_TIMESTAMP"
                    )
                    .bind(oid)
                    .bind(gid)
                    .bind(target_product_id)
                    .bind(omp.special_price)
                    .execute(&mut *tx)
                    .await;

                    member_prices_saved += 1;
                }
            }
        }
    }

    if let Err(e) = tx.commit().await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": format!("Gagal menyimpan transaksi import: {}", e) })),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": format!(
                "Import selesai: {} produk baru, {} diperbarui, {} dilewati. {} foto dihubungkan, {} harga outlet, {} harga member.",
                inserted_count, updated_count, skipped_count, images_attached_count, outlet_prices_saved, member_prices_saved
            ),
            "data": {
                "inserted": inserted_count,
                "updated": updated_count,
                "skipped": skipped_count,
                "images_attached": images_attached_count,
                "outlet_prices_saved": outlet_prices_saved,
                "member_prices_saved": member_prices_saved,
                "errors": errors
            }
        })),
    )
        .into_response()
}

// ============================================================================
// MULTI-BARCODE HANDLERS
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct AddProductBarcodePayload {
    pub barcode: String,
    pub description: Option<String>,
    pub set_as_primary: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ScanCheckBarcodePayload {
    pub barcode: String,
}

// 1. GET /products/:id/barcodes
async fn get_product_barcodes_handler(
    State(pool): State<PgPool>,
    Path(product_id): Path<Uuid>,
) -> impl IntoResponse {
    let product_res = sqlx::query("SELECT id, name, sku, barcode FROM products WHERE id = $1")
        .bind(product_id)
        .fetch_optional(&pool)
        .await;

    let product_row = match product_res {
        Ok(Some(r)) => r,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(json!({ "success": false, "message": "Produk tidak ditemukan" }))).into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    };

    let p_name: String = product_row.get("name");
    let p_sku: String = product_row.get("sku");
    let primary_barcode: String = product_row.get("barcode");

    let aliases_res = sqlx::query(
        "SELECT id, barcode, description, created_at FROM product_barcodes WHERE product_id = $1 ORDER BY created_at DESC"
    )
    .bind(product_id)
    .fetch_all(&pool)
    .await;

    match aliases_res {
        Ok(rows) => {
            let aliases: Vec<serde_json::Value> = rows.iter().map(|r| {
                json!({
                    "id": r.get::<Uuid, _>("id"),
                    "barcode": r.get::<String, _>("barcode"),
                    "description": r.get::<Option<String>, _>("description"),
                    "created_at": r.get::<chrono::DateTime<chrono::Utc>, _>("created_at"),
                })
            }).collect();

            (StatusCode::OK, Json(json!({
                "success": true,
                "data": {
                    "product_id": product_id,
                    "product_name": p_name,
                    "product_sku": p_sku,
                    "primary_barcode": primary_barcode,
                    "additional_barcodes": aliases
                }
            }))).into_response()
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

// 2. POST /products/:id/barcodes
async fn add_product_barcode_handler(
    State(pool): State<PgPool>,
    Path(product_id): Path<Uuid>,
    Json(payload): Json<AddProductBarcodePayload>,
) -> impl IntoResponse {
    let new_bc = payload.barcode.trim();
    if new_bc.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": "Kode barcode tidak boleh kosong" }))).into_response();
    }
    if new_bc.len() < 2 || new_bc.len() > 100 {
        return (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": "Panjang kode barcode harus antara 2 hingga 100 karakter" }))).into_response();
    }

    // Periksa apakah barcode sudah dipakai oleh produk lain sebagai barcode utama
    let existing_prod = sqlx::query("SELECT id, name FROM products WHERE barcode = $1 AND id <> $2")
        .bind(new_bc)
        .bind(product_id)
        .fetch_optional(&pool)
        .await;

    if let Ok(Some(row)) = existing_prod {
        let other_name: String = row.get("name");
        return (StatusCode::BAD_REQUEST, Json(json!({
            "success": false,
            "message": format!("Barcode '{}' sudah terdaftar sebagai barcode utama untuk produk '{}'", new_bc, other_name)
        }))).into_response();
    }

    // Periksa apakah barcode sudah dipakai di product_barcodes oleh produk lain
    let existing_alias = sqlx::query(
        "SELECT pb.product_id, p.name FROM product_barcodes pb JOIN products p ON pb.product_id = p.id WHERE pb.barcode = $1 AND pb.product_id <> $2"
    )
    .bind(new_bc)
    .bind(product_id)
    .fetch_optional(&pool)
    .await;

    if let Ok(Some(row)) = existing_alias {
        let other_name: String = row.get("name");
        return (StatusCode::BAD_REQUEST, Json(json!({
            "success": false,
            "message": format!("Barcode '{}' sudah terdaftar sebagai barcode alias untuk produk '{}'", new_bc, other_name)
        }))).into_response();
    }

    // Ambil barcode produk saat ini
    let current_bc_opt = sqlx::query_scalar::<_, String>("SELECT barcode FROM products WHERE id = $1")
        .bind(product_id)
        .fetch_optional(&pool)
        .await
        .unwrap_or(None);

    let current_bc = match current_bc_opt {
        Some(bc) => bc,
        None => return (StatusCode::NOT_FOUND, Json(json!({ "success": false, "message": "Produk tidak ditemukan" }))).into_response(),
    };

    if current_bc == new_bc {
        return (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": "Barcode ini sudah merupakan Barcode Utama dari produk ini" }))).into_response();
    }

    // Periksa apakah sudah terdaftar di barcode alias produk ini sendiri
    let already_alias = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM product_barcodes WHERE product_id = $1 AND barcode = $2)"
    )
    .bind(product_id)
    .bind(new_bc)
    .fetch_one(&pool)
    .await
    .unwrap_or(false);

    if already_alias {
        return (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": "Barcode ini sudah terdaftar dalam daftar barcode tambahan produk ini" }))).into_response();
    }

    let set_primary = payload.set_as_primary.unwrap_or(false);

    if set_primary {
        let mut tx = match pool.begin().await {
            Ok(t) => t,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
        };

        // Simpan barcode lama ke product_barcodes (jika tidak kosong)
        if !current_bc.trim().is_empty() {
            let _ = sqlx::query(
                "INSERT INTO product_barcodes (id, product_id, barcode, description) VALUES ($1, $2, $3, $4) ON CONFLICT (barcode) DO NOTHING"
            )
            .bind(Uuid::new_v4())
            .bind(product_id)
            .bind(&current_bc)
            .bind("Barcode Sebelumnya (Tergantikan)")
            .execute(&mut *tx)
            .await;
        }

        // Update barcode utama di products
        let upd = sqlx::query("UPDATE products SET barcode = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
            .bind(new_bc)
            .bind(product_id)
            .execute(&mut *tx)
            .await;

        if let Err(e) = upd {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response();
        }

        if let Err(e) = tx.commit().await {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response();
        }

        (StatusCode::OK, Json(json!({
            "success": true,
            "message": format!("Barcode '{}' berhasil dijadikan Barcode Utama. Barcode lama '{}' disimpan sebagai barcode alias.", new_bc, current_bc)
        }))).into_response()
    } else {
        // Masukkan sebagai barcode alias biasa
        let desc = payload.description.as_deref().unwrap_or("Barcode Kemasan Baru / Alternatif");
        let ins = sqlx::query(
            "INSERT INTO product_barcodes (id, product_id, barcode, description) VALUES ($1, $2, $3, $4)"
        )
        .bind(Uuid::new_v4())
        .bind(product_id)
        .bind(new_bc)
        .bind(desc)
        .execute(&pool)
        .await;

        match ins {
            Ok(_) => (StatusCode::OK, Json(json!({
                "success": true,
                "message": format!("Barcode alias '{}' berhasil ditambahkan ke produk ini.", new_bc)
            }))).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
        }
    }
}

// 3. DELETE /products/:id/barcodes/:barcode_id
async fn delete_product_barcode_handler(
    State(pool): State<PgPool>,
    Path((product_id, barcode_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let del = sqlx::query("DELETE FROM product_barcodes WHERE id = $1 AND product_id = $2")
        .bind(barcode_id)
        .bind(product_id)
        .execute(&pool)
        .await;

    match del {
        Ok(res) if res.rows_affected() > 0 => (StatusCode::OK, Json(json!({ "success": true, "message": "Barcode tambahan berhasil dihapus" }))).into_response(),
        Ok(_) => (StatusCode::NOT_FOUND, Json(json!({ "success": false, "message": "Barcode tidak ditemukan" }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

// 4. PUT /products/:id/barcodes/:barcode_id/set-primary
async fn set_primary_barcode_handler(
    State(pool): State<PgPool>,
    Path((product_id, barcode_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let mut tx = match pool.begin().await {
        Ok(t) => t,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    };

    // Ambil barcode alias
    let alias_row_opt = sqlx::query("SELECT barcode, description FROM product_barcodes WHERE id = $1 AND product_id = $2")
        .bind(barcode_id)
        .bind(product_id)
        .fetch_optional(&mut *tx)
        .await
        .unwrap_or(None);

    let alias_barcode: String = match alias_row_opt {
        Some(r) => r.get("barcode"),
        None => return (StatusCode::NOT_FOUND, Json(json!({ "success": false, "message": "Barcode alias tidak ditemukan" }))).into_response(),
    };

    // Ambil barcode utama saat ini
    let current_primary_bc: String = match sqlx::query_scalar::<_, String>("SELECT barcode FROM products WHERE id = $1")
        .bind(product_id)
        .fetch_optional(&mut *tx)
        .await
        .unwrap_or(None)
    {
        Some(bc) => bc,
        None => return (StatusCode::NOT_FOUND, Json(json!({ "success": false, "message": "Produk tidak ditemukan" }))).into_response(),
    };

    // Hapus barcode alias ini dari product_barcodes
    let _ = sqlx::query("DELETE FROM product_barcodes WHERE id = $1")
        .bind(barcode_id)
        .execute(&mut *tx)
        .await;

    // Simpan barcode utama lama ke product_barcodes
    if !current_primary_bc.trim().is_empty() {
        let _ = sqlx::query(
            "INSERT INTO product_barcodes (id, product_id, barcode, description) VALUES ($1, $2, $3, $4) ON CONFLICT (barcode) DO NOTHING"
        )
        .bind(Uuid::new_v4())
        .bind(product_id)
        .bind(&current_primary_bc)
        .bind("Barcode Sebelumnya (Tergantikan)")
        .execute(&mut *tx)
        .await;
    }

    // Set barcode baru ke products
    let upd = sqlx::query("UPDATE products SET barcode = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
        .bind(&alias_barcode)
        .bind(product_id)
        .execute(&mut *tx)
        .await;

    if let Err(e) = upd {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response();
    }

    if let Err(e) = tx.commit().await {
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response();
    }

    (StatusCode::OK, Json(json!({
        "success": true,
        "message": format!("Barcode '{}' sekarang menjadi Barcode Utama. Barcode lama '{}' disimpan sebagai alias.", alias_barcode, current_primary_bc)
    }))).into_response()
}

// 5. POST /products/barcode-scan
async fn check_scanned_barcode_handler(
    State(pool): State<PgPool>,
    Json(payload): Json<ScanCheckBarcodePayload>,
) -> impl IntoResponse {
    let clean = payload.barcode.trim();
    if clean.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": "Barcode tidak boleh kosong" }))).into_response();
    }

    // Cek di master products (barcode atau sku)
    let prod_row_opt = sqlx::query(
        "SELECT p.id, p.name, p.sku, p.barcode, p.cost_price, p.base_selling_price, p.image_url,
                c.name as category_name, u.code as unit_code
         FROM products p
         JOIN categories c ON p.category_id = c.id
         JOIN units u ON p.base_unit_id = u.id
         WHERE (p.barcode = $1 OR p.sku = $1) AND p.is_active = true
         LIMIT 1"
    )
    .bind(clean)
    .fetch_optional(&pool)
    .await
    .unwrap_or(None);

    if let Some(r) = prod_row_opt {
        return (StatusCode::OK, Json(json!({
            "success": true,
            "found": true,
            "is_alias": false,
            "matched_barcode": clean,
            "product": {
                "id": r.get::<Uuid, _>("id"),
                "name": r.get::<String, _>("name"),
                "sku": r.get::<String, _>("sku"),
                "barcode": r.get::<String, _>("barcode"),
                "cost_price": r.get::<Decimal, _>("cost_price"),
                "base_selling_price": r.get::<Decimal, _>("base_selling_price"),
                "category_name": r.get::<String, _>("category_name"),
                "unit_code": r.get::<String, _>("unit_code"),
                "image_url": r.get::<Option<String>, _>("image_url"),
            }
        }))).into_response();
    }

    // Cek di product_barcodes (alias)
    let alias_row_opt = sqlx::query(
        "SELECT p.id, p.name, p.sku, p.barcode, p.cost_price, p.base_selling_price, p.image_url,
                c.name as category_name, u.code as unit_code, pb.description as alias_desc
         FROM product_barcodes pb
         JOIN products p ON pb.product_id = p.id
         JOIN categories c ON p.category_id = c.id
         JOIN units u ON p.base_unit_id = u.id
         WHERE pb.barcode = $1 AND p.is_active = true
         LIMIT 1"
    )
    .bind(clean)
    .fetch_optional(&pool)
    .await
    .unwrap_or(None);

    if let Some(r) = alias_row_opt {
        return (StatusCode::OK, Json(json!({
            "success": true,
            "found": true,
            "is_alias": true,
            "alias_description": r.get::<Option<String>, _>("alias_desc"),
            "matched_barcode": clean,
            "product": {
                "id": r.get::<Uuid, _>("id"),
                "name": r.get::<String, _>("name"),
                "sku": r.get::<String, _>("sku"),
                "barcode": r.get::<String, _>("barcode"),
                "cost_price": r.get::<Decimal, _>("cost_price"),
                "base_selling_price": r.get::<Decimal, _>("base_selling_price"),
                "category_name": r.get::<String, _>("category_name"),
                "unit_code": r.get::<String, _>("unit_code"),
                "image_url": r.get::<Option<String>, _>("image_url"),
            }
        }))).into_response();
    }

    // Tidak ditemukan di manapun
    (StatusCode::OK, Json(json!({
        "success": true,
        "found": false,
        "scanned_barcode": clean,
        "message": format!("Barcode '{}' belum terdaftar pada sistem.", clean)
    }))).into_response()
}

