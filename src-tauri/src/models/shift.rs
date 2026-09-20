use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct CashierShift {
    pub id: Uuid,
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
}

#[derive(Debug, Deserialize)]
pub struct OpenShiftRequest {
    pub terminal_id: String,
    pub opening_cash: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct CloseShiftRequest {
    pub shift_id: Uuid,
    pub actual_cash: Decimal, // Blind count oleh kasir
    pub supervisor_pin: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ShiftSummaryResponse {
    pub shift: CashierShift,
    pub cashier_name: String,
    pub total_sales_count: i64,
    pub total_gross_sales: Decimal,
    pub total_discounts: Decimal,
    pub total_tax: Decimal,
    pub total_net_sales: Decimal,
    pub total_cash_received: Decimal,
    pub total_qris_received: Decimal,
    pub total_card_received: Decimal,
    pub total_points_redeemed: Decimal,
    pub expected_cash_in_drawer: Decimal,
    pub actual_cash_counted: Option<Decimal>,
    pub discrepancy: Option<Decimal>,
}
