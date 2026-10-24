use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct CashierShift {
    pub id: Uuid,
    pub outlet_id: Option<Uuid>,
    pub terminal_id: String,
    pub user_id: Uuid,
    pub opened_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub opening_cash: Decimal,
    pub expected_cash: Option<Decimal>,
    pub actual_cash: Option<Decimal>,
    pub discrepancy: Option<Decimal>,
    pub status: String, // OPEN, CLOSED
    pub supervisor_approval_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[sqlx(default)]
    pub outlet_name: Option<String>,
    #[sqlx(default)]
    pub outlet_code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OpenShiftRequest {
    pub outlet_id: Option<Uuid>,
    pub terminal_id: String,
    pub opening_cash: Decimal,
    pub supervisor_id: Option<Uuid>,
    pub supervisor_pin: Option<String>,
    pub supervisor_auth_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CloseShiftRequest {
    pub shift_id: Uuid,
    pub actual_cash: Decimal, // Blind count oleh kasir
    pub supervisor_id: Option<Uuid>,
    pub supervisor_pin: Option<String>,
    pub supervisor_auth_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShiftPaymentBreakdown {
    pub payment_method_id: Uuid,
    pub payment_method_name: String,
    pub category: String, // CASH, QRIS, EDC, POINTS, etc.
    pub total_amount: Decimal,
    pub transaction_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShiftSummaryResponse {
    pub shift: CashierShift,
    pub cashier_name: String,
    pub outlet_name: Option<String>,
    pub opened_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    
    // Ringkasan Penjualan Shift
    pub total_sales_count: i64,
    pub total_gross_sales: Decimal,
    pub total_discounts: Decimal,
    pub total_tax: Decimal,
    pub total_net_sales: Decimal,

    // Rekapan Uang di Laci Kasir (Cash Drawer Recapitulation)
    pub opening_cash: Decimal,
    pub total_cash_tendered: Decimal,
    pub total_cash_change: Decimal,
    pub net_cash_sales: Decimal,
    pub total_expenses: Decimal,
    pub expected_cash_in_drawer: Decimal,
    pub actual_cash_counted: Option<Decimal>,
    pub discrepancy: Option<Decimal>,

    // Akumulasi Pembayaran Utama
    pub total_cash_received: Decimal,
    pub total_qris_received: Decimal,
    pub total_card_received: Decimal,
    pub total_points_redeemed: Decimal,
    pub total_points_count: i64,
    pub total_voucher_discount: Decimal,

    // Rincian Lengkap per Metode Pembayaran (Dynamic Breakdown)
    pub payment_breakdowns: Vec<ShiftPaymentBreakdown>,
}
