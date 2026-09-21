use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Warehouse {
    pub id: Uuid,
    pub outlet_id: Option<Uuid>,
    pub code: String,
    pub name: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct WarehouseStock {
    pub id: Uuid,
    pub warehouse_id: Uuid,
    pub product_id: Uuid,
    pub quantity: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct StockMutation {
    pub id: i64,
    pub warehouse_id: Uuid,
    pub product_id: Uuid,
    pub reference_type: String,
    pub reference_id: Uuid,
    pub qty_in: Decimal,
    pub qty_out: Decimal,
    pub balance: Decimal,
    pub cost_price: Decimal,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Supplier {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub contact_person: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub term_of_payment_days: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct StockAdjustmentRequest {
    pub warehouse_id: Uuid,
    pub reason_id: Uuid,
    pub notes: Option<String>,
    pub items: Vec<StockAdjustmentItemInput>,
}

#[derive(Debug, Deserialize)]
pub struct StockAdjustmentItemInput {
    pub product_id: Uuid,
    pub actual_qty: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct GoodsReceiptNoteRequest {
    pub po_id: Option<Uuid>,
    pub supplier_id: Uuid,
    pub outlet_id: Option<Uuid>,
    pub warehouse_id: Option<Uuid>,
    pub supplier_invoice_number: String,
    pub invoice_date: NaiveDate,
    pub due_date: NaiveDate,
    pub notes: Option<String>,
    pub items: Vec<GoodsReceiptItemInput>,
}

#[derive(Debug, Deserialize)]
pub struct GoodsReceiptItemInput {
    pub product_id: Uuid,
    pub received_qty: Decimal,
    pub unit_price: Decimal,
    pub discount_amount: Option<Decimal>,
}
