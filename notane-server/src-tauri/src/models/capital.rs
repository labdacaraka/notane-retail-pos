use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct CapitalDeposit {
    pub id: Uuid,
    pub deposit_no: String,
    pub outlet_id: Option<Uuid>,
    pub deposit_date: NaiveDate,
    pub investor_name: String,
    pub amount: Decimal,
    pub payment_account_id: Uuid,
    pub equity_account_id: Uuid,
    pub reference_no: Option<String>,
    pub notes: Option<String>,
    pub attachment_url: Option<String>,
    pub journal_entry_id: Option<Uuid>,
    pub created_by: Option<Uuid>,
    pub cancelled_by: Option<Uuid>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub cancellation_reason: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CapitalDepositDetail {
    pub id: Uuid,
    pub deposit_no: String,
    pub outlet_id: Option<Uuid>,
    pub outlet_name: Option<String>,
    pub deposit_date: NaiveDate,
    pub investor_name: String,
    pub amount: Decimal,
    pub payment_account_id: Uuid,
    pub payment_account_code: String,
    pub payment_account_name: String,
    pub equity_account_id: Uuid,
    pub equity_account_code: String,
    pub equity_account_name: String,
    pub reference_no: Option<String>,
    pub notes: Option<String>,
    pub attachment_url: Option<String>,
    pub journal_entry_id: Option<Uuid>,
    pub created_by: Option<Uuid>,
    pub created_by_name: Option<String>,
    pub cancelled_by: Option<Uuid>,
    pub cancelled_by_name: Option<String>,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub cancellation_reason: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateCapitalDepositRequest {
    pub outlet_id: Option<Uuid>,
    pub deposit_date: Option<NaiveDate>,
    pub investor_name: String,
    pub amount: Decimal,
    pub payment_account_id: Uuid,
    pub equity_account_id: Uuid,
    pub reference_no: Option<String>,
    pub notes: Option<String>,
    pub attachment_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CancelCapitalDepositRequest {
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct CapitalDepositFilter {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub outlet_id: Option<Uuid>,
    pub status: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub search: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CapitalSummaryResponse {
    pub total_amount: Decimal,
    pub total_count: i64,
    pub this_month_amount: Decimal,
    pub this_year_amount: Decimal,
}
