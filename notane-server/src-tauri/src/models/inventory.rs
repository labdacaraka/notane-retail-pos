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
    #[serde(default)]
    pub warehouse_id: Option<Uuid>,
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
pub struct CreatePurchaseOrderRequest {
    pub supplier_id: Uuid,
    pub outlet_id: Option<Uuid>,
    pub order_date: NaiveDate,
    pub expected_delivery: Option<NaiveDate>,
    pub notes: Option<String>,
    pub items: Vec<CreatePurchaseOrderItemInput>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePurchaseOrderItemInput {
    pub product_id: Uuid,
    pub ordered_qty: Decimal,
    pub unit_price: Decimal,
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
    pub payment_status: Option<String>,
    pub notes: Option<String>,
    pub items: Vec<GoodsReceiptItemInput>,
}

#[derive(Debug, Deserialize)]
pub struct GoodsReceiptItemInput {
    pub product_id: Uuid,
    pub ordered_qty: Option<Decimal>,
    pub received_qty: Decimal,
    pub unit_price: Decimal,
    pub discount_amount: Option<Decimal>,
    pub new_barcode: Option<String>,
    pub new_barcode_desc: Option<String>,
    pub expiry_date: Option<NaiveDate>,
    pub batch_number: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateGoodsReceiptRequest {
    pub po_id: Uuid,
    pub delivery_order_number: String,
    pub receive_date: NaiveDate,
    pub notes: Option<String>,
    pub items: Vec<CreateGoodsReceiptItemInput>,
}

#[derive(Debug, Deserialize)]
pub struct CreateGoodsReceiptItemInput {
    pub po_item_id: Option<Uuid>,
    pub product_id: Uuid,
    pub received_qty: Decimal,
    pub expiry_date: Option<NaiveDate>,
    pub batch_number: Option<String>,
    pub new_barcode: Option<String>,
    pub new_barcode_desc: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct GoodsReceiptQueryFilter {
    pub search: Option<String>,
    pub po_id: Option<Uuid>,
    pub supplier_id: Option<Uuid>,
    pub outlet_id: Option<Uuid>,
    pub status: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub sort_by: Option<String>,
    pub sort_dir: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Default, Deserialize)]
pub struct PurchaseOrderQueryFilter {
    pub search: Option<String>,
    pub supplier_id: Option<Uuid>,
    pub outlet_id: Option<Uuid>,
    pub status: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub sort_by: Option<String>,
    pub sort_dir: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePurchaseInvoiceFromGrnRequest {
    pub goods_receipt_id: Uuid,
    pub supplier_invoice_number: String,
    pub invoice_date: NaiveDate,
    pub due_date: NaiveDate,
    pub payment_status: Option<String>,
    pub discount_amount: Option<Decimal>,
    pub tax_amount: Option<Decimal>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateStockTransferRequest {
    pub from_outlet_id: Uuid,
    pub to_outlet_id: Uuid,
    pub transfer_date: NaiveDate,
    pub notes: Option<String>,
    pub items: Vec<CreateStockTransferItemInput>,
    pub is_immediate: Option<bool>, // Jika true: langsung COMPLETED. Jika false (default): IN_TRANSIT (alur 2 tahap).
}

#[derive(Debug, Deserialize)]
pub struct CreateStockTransferItemInput {
    pub product_id: Uuid,
    pub quantity: Decimal,
}

#[derive(Debug, Default, Deserialize)]
pub struct StockTransferQueryFilter {
    pub search: Option<String>,
    pub from_outlet_id: Option<Uuid>,
    pub to_outlet_id: Option<Uuid>,
    pub status: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub sort_by: Option<String>,
    pub sort_dir: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ReceiveStockTransferRequest {
    pub receive_date: NaiveDate,
    pub notes: Option<String>,
    pub items: Option<Vec<ReceiveStockTransferItemInput>>,
}

#[derive(Debug, Deserialize)]
pub struct ReceiveStockTransferItemInput {
    pub product_id: Uuid,
    pub received_quantity: Decimal,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CancelStockTransferRequest {
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePurchaseReturnRequest {
    pub supplier_id: Uuid,
    pub outlet_id: Option<Uuid>,
    pub invoice_id: Option<Uuid>,
    pub goods_receipt_id: Option<Uuid>,
    pub return_date: NaiveDate,
    pub refund_method: Option<String>, // "DEBT_REDUCTION", "CASH_REFUND", "REPLACEMENT"
    pub reason: String,
    pub notes: Option<String>,
    pub items: Vec<CreatePurchaseReturnItemInput>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePurchaseReturnItemInput {
    pub product_id: Uuid,
    pub returned_qty: Decimal,
    pub unit_price: Decimal,
    pub notes: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct PurchaseReturnQueryFilter {
    pub search: Option<String>,
    pub supplier_id: Option<Uuid>,
    pub outlet_id: Option<Uuid>,
    pub invoice_id: Option<Uuid>,
    pub status: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub sort_by: Option<String>,
    pub sort_dir: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}
