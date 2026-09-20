use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Sale {
    pub id: Uuid,
    pub invoice_number: String,
    pub shift_id: Uuid,
    pub customer_id: Option<Uuid>,
    pub warehouse_id: Uuid,
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
    pub shift_id: Uuid,
    pub warehouse_id: Uuid,
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
}

#[derive(Debug, Deserialize, Clone)]
pub struct CheckoutPaymentInput {
    pub payment_method_id: Uuid,
    pub amount: Decimal,
    pub card_number_masked: Option<String>,
    pub reference_number: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CheckoutResponse {
    pub sale: Sale,
    pub items_count: usize,
    pub change_amount: Decimal,
    pub earned_points: i32,
    pub invoice_number: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct VoidSaleRequest {
    pub supervisor_pin: String,
    pub reason_id: Uuid,
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
