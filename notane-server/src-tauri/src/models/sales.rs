use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Sale {
    pub id: Uuid,
    pub invoice_number: String,
    pub outlet_id: Uuid,
    pub shift_id: Uuid,
    pub customer_id: Option<Uuid>,
    pub warehouse_id: Option<Uuid>,
    pub voucher_id: Option<Uuid>,
    pub subtotal: Decimal,
    pub discount_amount: Decimal,
    pub tax_amount: Decimal,
    pub total_amount: Decimal,
    pub paid_amount: Decimal,
    pub change_amount: Decimal,
    pub total_cost_amount: Decimal,
    pub status: String, // COMPLETED, VOID, RETURNED, ON_HOLD
    pub notes: Option<String>,
    #[serde(default)]
    pub total_flash_sale_discount: Decimal,
    #[serde(default)]
    pub total_member_discount: Decimal,
    #[serde(default)]
    pub total_savings_amount: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct SaleItem {
    pub id: Uuid,
    pub sale_id: Uuid,
    pub product_id: Uuid,
    pub unit_id: Uuid,
    pub quantity: Decimal,
    pub unit_cost_price: Decimal,
    pub unit_selling_price: Decimal,
    pub discount_percent: Decimal,
    pub discount_amount: Decimal,
    pub tax_amount: Decimal,
    pub subtotal: Decimal,
    pub is_voided: bool,
    pub void_reason_id: Option<Uuid>,
    pub void_authorized_by: Option<Uuid>,
    pub flash_sale_id: Option<Uuid>,
    pub original_price: Option<Decimal>,
    #[serde(default)]
    pub flash_sale_discount: Decimal,
    #[serde(default)]
    pub member_discount: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct SalePayment {
    pub id: Uuid,
    pub sale_id: Uuid,
    pub payment_method_id: Uuid,
    pub amount: Decimal,
    pub card_number_masked: Option<String>,
    pub reference_number: Option<String>,
    pub payment_proof_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CheckoutRequest {
    pub shift_id: Option<Uuid>,
    pub outlet_id: Option<Uuid>,
    pub warehouse_id: Option<Uuid>,
    pub customer_id: Option<Uuid>,
    pub voucher_code: Option<String>,
    pub items: Vec<CheckoutItemInput>,
    pub payments: Vec<CheckoutPaymentInput>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CheckoutItemInput {
    pub product_id: Uuid,
    pub unit_id: Uuid,
    pub quantity: Decimal,
    pub unit_selling_price: Decimal,
    pub discount_percent: Option<Decimal>,
    pub discount_amount: Option<Decimal>,
    pub flash_sale_id: Option<Uuid>,
    pub original_price: Option<Decimal>,
    pub flash_sale_discount: Option<Decimal>,
    pub member_discount: Option<Decimal>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CheckoutPaymentInput {
    pub payment_method_id: Uuid,
    #[serde(default)]
    pub amount: Decimal,
    pub points: Option<i32>,
    pub card_number_masked: Option<String>,
    pub reference_number: Option<String>,
    pub payment_proof_url: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct CheckoutItemResponse {
    pub product_id: Uuid,
    pub product_name: String,
    pub unit_id: Uuid,
    pub unit_code: String,
    pub quantity: Decimal,
    pub unit_selling_price: Decimal,
    pub discount_percent: Decimal,
    pub discount_amount: Decimal,
    pub tax_amount: Decimal,
    pub subtotal: Decimal,
    pub original_price: Option<Decimal>,
    pub flash_sale_discount: Decimal,
    pub member_discount: Decimal,
}

#[derive(Debug, Serialize, Clone)]
pub struct CheckoutPaymentResponse {
    pub payment_method_id: Uuid,
    pub payment_method_name: String,
    pub category: String,
    pub amount: Decimal,
}

#[derive(Debug, Serialize)]
pub struct CheckoutResponse {
    pub sale: Sale,
    pub items: Vec<CheckoutItemResponse>,
    pub payments: Vec<CheckoutPaymentResponse>,
    pub customer_name: Option<String>,
    pub items_count: usize,
    pub change_amount: Decimal,
    pub earned_points: i32,
    pub invoice_number: String,
    pub total_savings_amount: Decimal,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct VoidSaleRequest {
    pub supervisor_id: Option<Uuid>,
    pub supervisor_pin: Option<String>,
    pub supervisor_auth_token: Option<String>,
    pub reason_id: Option<Uuid>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct HoldSaleRequest {
    pub shift_id: Uuid,
    pub customer_id: Option<Uuid>,
    pub customer_name: Option<String>,
    pub items: Vec<CheckoutItemInput>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct SaleItemStockBatch {
    pub id: Uuid,
    pub sale_id: Uuid,
    pub sale_item_id: Uuid,
    pub product_id: Uuid,
    pub outlet_id: Uuid,
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
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct SaleItemStockBatchAuditView {
    pub id: Uuid,
    pub sale_id: Uuid,
    pub sale_item_id: Uuid,
    pub product_id: Uuid,
    pub product_name: String,
    pub product_sku: String,
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
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct BatchHeaderRow {
    pub id: Uuid,
    pub batch_number: String,
    pub product_id: Uuid,
    pub product_name: String,
    pub outlet_id: Uuid,
    pub outlet_name: String,
    pub initial_qty: Decimal,
    pub remaining_qty: Decimal,
    pub cost_price: Decimal,
    pub expiry_date: Option<chrono::NaiveDate>,
    pub source_document_number: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct BatchSaleUsageItem {
    pub id: Uuid,
    pub sale_id: Uuid,
    pub sale_item_id: Uuid,
    pub invoice_number: String,
    pub sale_status: String,
    pub sale_created_at: DateTime<Utc>,
    pub outlet_id: Uuid,
    pub outlet_name: String,
    pub customer_name: Option<String>,
    pub cashier_name: Option<String>,
    pub quantity: Decimal,
    pub cost_price: Decimal,
    pub subtotal_cost: Decimal,
    pub allocation_method: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BatchSaleUsageSummary {
    pub batch_id: Uuid,
    pub batch_number: String,
    pub product_id: Uuid,
    pub product_name: String,
    pub outlet_id: Uuid,
    pub outlet_name: String,
    pub initial_qty: Decimal,
    pub remaining_qty: Decimal,
    pub cost_price: Decimal,
    pub expiry_date: Option<chrono::NaiveDate>,
    pub source_document_number: Option<String>,
    pub total_consumed_qty: Decimal,
    pub total_consumed_cost: Decimal,
    pub sales: Vec<BatchSaleUsageItem>,
}

