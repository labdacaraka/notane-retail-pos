use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct FlashSale {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub is_all_day: bool,
    pub start_time: Option<NaiveTime>,
    pub end_time: Option<NaiveTime>,
    pub is_active: bool,
    pub is_stackable_with_member: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<Uuid>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct FlashSaleItem {
    pub id: Uuid,
    pub flash_sale_id: Uuid,
    pub product_id: Uuid,
    pub discount_type: String, // FIXED_PRICE, PERCENTAGE, NOMINAL_CUT
    pub discount_value: Decimal,
    pub max_qty_per_transaction: Option<Decimal>,
    pub promo_quota: Option<Decimal>,
    pub sold_qty: Decimal,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FlashSaleItemDetail {
    pub id: Uuid,
    pub flash_sale_id: Uuid,
    pub product_id: Uuid,
    pub product_sku: String,
    pub product_name: String,
    pub product_barcode: Option<String>,
    pub unit_code: String,
    pub normal_price: Decimal,
    pub discount_type: String,
    pub discount_value: Decimal,
    pub promo_price: Decimal,
    pub max_qty_per_transaction: Option<Decimal>,
    pub promo_quota: Option<Decimal>,
    pub sold_qty: Decimal,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FlashSaleOutletDetail {
    pub outlet_id: Uuid,
    pub outlet_code: String,
    pub outlet_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FlashSaleDetailResponse {
    pub flash_sale: FlashSale,
    pub items: Vec<FlashSaleItemDetail>,
    pub outlets: Vec<FlashSaleOutletDetail>,
    pub total_products: usize,
    pub is_currently_running: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CreateFlashSaleItemInput {
    pub product_id: Uuid,
    pub discount_type: String, // FIXED_PRICE, PERCENTAGE, NOMINAL_CUT
    pub discount_value: Decimal,
    pub max_qty_per_transaction: Option<Decimal>,
    pub promo_quota: Option<Decimal>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CreateFlashSaleRequest {
    pub name: String,
    pub description: Option<String>,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub is_all_day: bool,
    pub start_time: Option<NaiveTime>,
    pub end_time: Option<NaiveTime>,
    pub is_active: Option<bool>,
    pub is_stackable_with_member: Option<bool>,
    pub outlet_ids: Vec<Uuid>, // Jika kosong, berarti berlaku di semua outlet
    pub items: Vec<CreateFlashSaleItemInput>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ActivePromoProduct {
    pub flash_sale_id: Uuid,
    pub flash_sale_name: String,
    pub product_id: Uuid,
    pub discount_type: String,
    pub discount_value: Decimal,
    pub promo_price: Decimal,
    pub is_stackable_with_member: bool,
    pub max_qty_per_transaction: Option<Decimal>,
    pub remaining_quota: Option<Decimal>,
}
