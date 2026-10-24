use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Category {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub code: String,
    pub name: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Unit {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub is_fractional: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Brand {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Product {
    pub id: Uuid,
    pub category_id: Uuid,
    pub base_unit_id: Uuid,
    pub sku: String,
    pub barcode: String,
    pub name: String,
    pub brand_id: Option<Uuid>,
    pub costing_method: String,
    pub cost_price: Decimal,
    pub base_selling_price: Decimal,
    pub min_stock: Decimal,
    pub max_stock: Decimal,
    pub tax_percent: Decimal,
    pub is_weight_embedded: bool,
    pub is_active: bool,
    pub image_url: Option<String>,
    #[sqlx(default)]
    pub auto_adjust_selling_price: bool,
    #[sqlx(default)]
    pub markup_percent: Option<Decimal>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct ProductUnit {
    pub id: Uuid,
    pub product_id: Uuid,
    pub unit_id: Uuid,
    pub conversion_factor: Decimal,
    pub selling_price: Decimal,
    pub barcode: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct ProductPriceTier {
    pub id: Uuid,
    pub product_id: Uuid,
    pub min_quantity: Decimal,
    pub tier_price: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct CustomerGroupPrice {
    pub id: Uuid,
    pub customer_group_id: i32,
    pub product_id: Uuid,
    pub special_price: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CustomerGroupPriceDetail {
    pub id: Uuid,
    pub customer_group_id: i32,
    pub customer_group_name: String,
    pub product_id: Uuid,
    pub special_price: Decimal,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct ProductOutletPrice {
    pub id: Uuid,
    pub outlet_id: Uuid,
    pub product_id: Uuid,
    pub cost_price: Decimal,
    pub base_selling_price: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProductOutletPriceDetail {
    pub id: Uuid,
    pub outlet_id: Uuid,
    pub outlet_code: String,
    pub outlet_name: String,
    pub is_warehouse: bool,
    pub product_id: Uuid,
    pub cost_price: Decimal,
    pub base_selling_price: Decimal,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OutletCustomerGroupPriceDetail {
    pub id: Uuid,
    pub outlet_id: Uuid,
    pub customer_group_id: i32,
    pub customer_group_name: String,
    pub product_id: Uuid,
    pub special_price: Decimal,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProductDetail {
    pub product: Product,
    pub category_name: String,
    pub base_unit_code: String,
    pub base_unit_name: String,
    pub is_fractional: bool,
    pub brand_name: Option<String>,
    pub units: Vec<ProductUnitDetail>,
    pub price_tiers: Vec<ProductPriceTier>,
    pub customer_group_prices: Vec<CustomerGroupPriceDetail>,
    pub outlet_customer_group_prices: Vec<OutletCustomerGroupPriceDetail>,
    pub outlet_prices: Option<Vec<ProductOutletPriceDetail>>,
    pub current_stock: Decimal,
    pub scanned_quantity: Option<Decimal>,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProductUnitDetail {
    pub id: Uuid,
    pub unit_id: Uuid,
    pub unit_code: String,
    pub unit_name: String,
    pub conversion_factor: Decimal,
    pub selling_price: Decimal,
    pub barcode: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProductRequest {
    pub category_id: Uuid,
    pub base_unit_id: Uuid,
    pub sku: String,
    pub barcode: String,
    pub name: String,
    pub brand_id: Option<Uuid>,
    pub costing_method: Option<String>,
    pub cost_price: Decimal,
    pub base_selling_price: Decimal,
    pub min_stock: Option<Decimal>,
    pub max_stock: Option<Decimal>,
    pub tax_percent: Option<Decimal>,
    pub is_weight_embedded: Option<bool>,
    pub image_url: Option<String>,
    pub initial_stock: Option<Decimal>,
    pub warehouse_id: Option<Uuid>,
    pub auto_adjust_selling_price: Option<bool>,
    pub markup_percent: Option<Decimal>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct ProductStockBatch {
    pub id: Uuid,
    pub product_id: Uuid,
    pub outlet_id: Uuid,
    pub batch_number: String,
    pub initial_qty: Decimal,
    pub remaining_qty: Decimal,
    pub cost_price: Decimal,
    pub reference_type: String,
    pub reference_id: Option<Uuid>,
    pub received_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    #[sqlx(default)]
    pub expiry_date: Option<chrono::NaiveDate>,
    #[sqlx(default)]
    pub source_document_number: Option<String>,
    #[sqlx(default)]
    pub outlet_name: Option<String>,
    #[sqlx(default)]
    pub product_name: Option<String>,
}
