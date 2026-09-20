use crate::models::product::{Brand, Category, CreateProductRequest, Product, Unit};
use crate::models::settings::{ReceiptSettings, UpdateReceiptSettingsRequest};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub fn master_routes() -> Router<PgPool> {
    Router::new()
        // Products
        .route("/products", get(list_products_handler).post(create_product_handler))
        .route("/products/{id}", put(update_product_handler).delete(delete_product_handler))
        .route("/products/{id}/units", post(add_product_unit_handler))
        .route("/products/units/{id}", delete(delete_product_unit_handler))
        .route("/products/{id}/price-tiers", post(add_product_tier_handler))
        .route("/products/price-tiers/{id}", delete(delete_product_tier_handler))

        // Customers & Customer Groups
        .route("/customers", get(list_customers_handler).post(create_customer_handler))
        .route("/customers/{id}", put(update_customer_handler).delete(delete_customer_handler))
        .route("/customer-groups", get(list_customer_groups_handler).post(create_customer_group_handler))
        .route("/customer-groups/{id}", put(update_customer_group_handler).delete(delete_customer_group_handler))

        // Vouchers & Promotions
        .route("/vouchers", get(list_vouchers_handler).post(create_voucher_handler))
        .route("/vouchers/{id}", put(update_voucher_handler).delete(delete_voucher_handler))
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
        .route("/outlets", get(list_outlets_handler))

        // Receipt & Store Settings
        .route("/settings/receipt", get(get_receipt_settings_handler).post(save_receipt_settings_handler).put(save_receipt_settings_handler))
}

// ============================================================================
// 1. PRODUCTS CRUD
// ============================================================================

#[derive(Deserialize)]
struct ProductQuery {
    category_id: Option<Uuid>,
    search: Option<String>,
}

async fn list_products_handler(
    State(pool): State<PgPool>,
    Query(query): Query<ProductQuery>,
) -> impl IntoResponse {
    let mut sql = "SELECT p.*, c.name as category_name, u.code as unit_code, COALESCE(SUM(ws.quantity), 0) as stock_qty 
                   FROM products p 
                   JOIN categories c ON p.category_id = c.id 
                   JOIN units u ON p.base_unit_id = u.id 
                   LEFT JOIN warehouse_stocks ws ON p.id = ws.product_id 
                   WHERE p.is_active = true".to_string();

    if let Some(cat) = query.category_id {
        sql.push_str(&format!(" AND p.category_id = '{}'", cat));
    }
    if let Some(s) = query.search {
        sql.push_str(&format!(" AND (p.name ILIKE '%{}%' OR p.barcode ILIKE '%{}%' OR p.sku ILIKE '%{}%')", s, s, s));
    }
    sql.push_str(" GROUP BY p.id, c.name, u.code ORDER BY p.name ASC");

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
                        "category_name": row.get::<String, _>("category_name"),
                        "unit_code": row.get::<String, _>("unit_code"),
                        "cost_price": row.get::<Decimal, _>("cost_price"),
                        "base_selling_price": row.get::<Decimal, _>("base_selling_price"),
                        "min_stock": row.get::<Decimal, _>("min_stock"),
                        "max_stock": row.get::<Decimal, _>("max_stock"),
                        "tax_percent": row.get::<Decimal, _>("tax_percent"),
                        "is_weight_embedded": row.get::<bool, _>("is_weight_embedded"),
                        "stock_qty": row.get::<Decimal, _>("stock_qty"),
                    })
                })
                .collect();
            (StatusCode::OK, Json(json!({ "success": true, "data": list }))).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": e.to_string() })),
        )
            .into_response(),
    }
}

async fn create_product_handler(
    State(pool): State<PgPool>,
    Json(payload): Json<CreateProductRequest>,
) -> impl IntoResponse {
    let mut tx = match pool.begin().await {
        Ok(t) => t,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    };

    let prod_id = Uuid::new_v4();

    let res = sqlx::query_as::<_, Product>(
        "INSERT INTO products (
            id, category_id, base_unit_id, sku, barcode, name, brand_id, costing_method,
            cost_price, base_selling_price, min_stock, max_stock, tax_percent, is_weight_embedded
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        RETURNING *",
    )
    .bind(prod_id)
    .bind(payload.category_id)
    .bind(payload.base_unit_id)
    .bind(&payload.sku)
    .bind(&payload.barcode)
    .bind(&payload.name)
    .bind(payload.brand_id)
    .bind(payload.costing_method.unwrap_or_else(|| "WMA".into()))
    .bind(payload.cost_price)
    .bind(payload.base_selling_price)
    .bind(payload.min_stock.unwrap_or(Decimal::ZERO))
    .bind(payload.max_stock.unwrap_or(Decimal::ZERO))
    .bind(payload.tax_percent.unwrap_or_else(|| Decimal::from(11)))
    .bind(payload.is_weight_embedded.unwrap_or(false))
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
            updated_at = CURRENT_TIMESTAMP
        WHERE id = $13
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

// ============================================================================
// 2. CUSTOMERS & CUSTOMER GROUPS CRUD
// ============================================================================

#[derive(Deserialize)]
struct CustomerQuery {
    search: Option<String>,
    group_id: Option<i32>,
}

async fn list_customers_handler(
    State(pool): State<PgPool>,
    Query(q): Query<CustomerQuery>,
) -> impl IntoResponse {
    let mut sql = "SELECT c.*, cg.name as group_name 
                   FROM customers c 
                   LEFT JOIN customer_groups cg ON c.customer_group_id = cg.id 
                   WHERE c.is_active = true".to_string();

    if let Some(gid) = q.group_id {
        sql.push_str(&format!(" AND c.customer_group_id = {}", gid));
    }
    if let Some(s) = q.search {
        sql.push_str(&format!(" AND (c.name ILIKE '%{}%' OR c.code ILIKE '%{}%' OR c.phone ILIKE '%{}%')", s, s, s));
    }
    sql.push_str(" ORDER BY c.name ASC");

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
            (StatusCode::OK, Json(json!({ "success": true, "data": list }))).into_response()
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
    status: Option<String>,
}

async fn create_voucher_handler(
    State(pool): State<PgPool>,
    Json(payload): Json<VoucherInput>,
) -> impl IntoResponse {
    let res = sqlx::query(
        "INSERT INTO vouchers (code, description, discount_type, discount_value, min_purchase_amount, max_discount_amount, valid_from, valid_until, status)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
         RETURNING id, code",
    )
    .bind(&payload.code)
    .bind(payload.description)
    .bind(&payload.discount_type)
    .bind(payload.discount_value)
    .bind(payload.min_purchase_amount.unwrap_or(Decimal::ZERO))
    .bind(payload.max_discount_amount)
    .bind(payload.valid_from)
    .bind(payload.valid_until)
    .bind(payload.status.unwrap_or_else(|| "ACTIVE".into()))
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
            status = $9,
            updated_at = CURRENT_TIMESTAMP
         WHERE id = $10",
    )
    .bind(&payload.code)
    .bind(payload.description)
    .bind(&payload.discount_type)
    .bind(payload.discount_value)
    .bind(payload.min_purchase_amount.unwrap_or(Decimal::ZERO))
    .bind(payload.max_discount_amount)
    .bind(payload.valid_from)
    .bind(payload.valid_until)
    .bind(payload.status.unwrap_or_else(|| "ACTIVE".into()))
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
    let v = sqlx::query(
        "SELECT * FROM vouchers WHERE code = $1 AND status = 'ACTIVE' AND valid_from <= CURRENT_TIMESTAMP AND valid_until >= CURRENT_TIMESTAMP",
    )
    .bind(code)
    .fetch_optional(&pool)
    .await;

    match v {
        Ok(Some(v)) => (StatusCode::OK, Json(json!({
            "success": true,
            "data": {
                "id": v.get::<Uuid, _>("id"),
                "code": v.get::<String, _>("code"),
                "description": v.get::<Option<String>, _>("description"),
                "discount_type": v.get::<String, _>("discount_type"),
                "discount_value": v.get::<Decimal, _>("discount_value"),
                "min_purchase_amount": v.get::<Decimal, _>("min_purchase_amount"),
                "max_discount_amount": v.get::<Option<Decimal>, _>("max_discount_amount"),
            }
        }))).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({ "success": false, "message": "Kode voucher tidak ditemukan atau sudah kedaluwarsa" }))).into_response(),
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

async fn list_categories_handler(State(pool): State<PgPool>) -> impl IntoResponse {
    match sqlx::query_as::<_, Category>("SELECT * FROM categories WHERE is_active = true ORDER BY name ASC").fetch_all(&pool).await {
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

async fn list_suppliers_handler(State(pool): State<PgPool>) -> impl IntoResponse {
    let rows = sqlx::query("SELECT * FROM suppliers WHERE is_active = true ORDER BY name ASC").fetch_all(&pool).await;
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
            (StatusCode::OK, Json(json!({ "success": true, "data": list }))).into_response()
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
    let rows = sqlx::query("SELECT * FROM warehouses WHERE is_active = true ORDER BY name ASC").fetch_all(&pool).await;
    match rows {
        Ok(r) => {
            let list: Vec<serde_json::Value> = r.iter().map(|w| json!({
                "id": w.get::<Uuid, _>("id"),
                "outlet_id": w.get::<Uuid, _>("outlet_id"),
                "code": w.get::<String, _>("code"),
                "name": w.get::<String, _>("name"),
            })).collect();
            (StatusCode::OK, Json(json!({ "success": true, "data": list }))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    }
}

async fn list_outlets_handler(State(pool): State<PgPool>) -> impl IntoResponse {
    let rows = sqlx::query("SELECT * FROM outlets WHERE is_active = true ORDER BY name ASC").fetch_all(&pool).await;
    match rows {
        Ok(r) => {
            let list: Vec<serde_json::Value> = r.iter().map(|o| json!({
                "id": o.get::<Uuid, _>("id"),
                "code": o.get::<String, _>("code"),
                "name": o.get::<String, _>("name"),
                "address": o.get::<Option<String>, _>("address"),
                "phone": o.get::<Option<String>, _>("phone"),
            })).collect();
            (StatusCode::OK, Json(json!({ "success": true, "data": list }))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
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

    let result = if let Some(id) = existing_id {
        sqlx::query_as::<_, ReceiptSettings>(
            "UPDATE receipt_settings 
             SET store_name = $1, store_tagline = $2, store_address = $3, store_phone = $4,
                 store_website = $5, tax_id = $6, header_note = $7, footer_note_1 = $8,
                 footer_note_2 = $9, social_media = $10, show_cashier_name = $11,
                 show_customer_info = $12, show_loyalty_points = $13, show_tax_info = $14,
                 paper_width = $15, outlet_id = $16, updated_at = CURRENT_TIMESTAMP
             WHERE id = $17
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
                paper_width, outlet_id
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
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
        .fetch_one(&pool)
        .await
    };

    match result {
        Ok(s) => (StatusCode::OK, Json(json!({ "success": true, "message": "Pengaturan struk berhasil disimpan ke database", "data": s }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": format!("Gagal menyimpan pengaturan struk: {}", e) }))).into_response(),
    }
}
