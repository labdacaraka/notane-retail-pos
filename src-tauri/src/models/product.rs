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
    pub initial_stock: Option<Decimal>,
    pub warehouse_id: Option<Uuid>,
}
