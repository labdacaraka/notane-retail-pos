use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DividendDistribution {
    pub id: Uuid,
    pub distribution_number: String,
    pub closing_id: Uuid,
    pub period_id: Uuid, // Alias kompatibel dengan frontend
    pub closing_number: Option<String>,
    pub period_name: Option<String>,
    pub period_start: Option<NaiveDate>,
    pub period_end: Option<NaiveDate>,
    pub distribution_date: NaiveDate,
    pub total_amount: Decimal,
    pub retained_earnings_account_id: Uuid,
    pub retained_earnings_account_code: Option<String>,
    pub retained_earnings_account_name: Option<String>,
    pub payment_account_id: Uuid,
    pub payment_account_code: Option<String>,
    pub payment_account_name: Option<String>,
    pub journal_entry_id: Option<Uuid>,
    pub journal_number: Option<String>,
    pub recipient_name: String,
    pub notes: Option<String>,
    pub recipients_data: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<Uuid>,
    pub created_by_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDividendRequest {
    pub closing_id: Option<Uuid>,
    pub period_id: Option<Uuid>, // Mendukung period_id atau closing_id dari form
    pub distribution_date: NaiveDate,
    pub total_amount: Decimal,
    pub retained_earnings_account_id: Option<Uuid>,
    pub payment_account_id: Uuid,
    pub recipient_name: String,
    pub notes: Option<String>,
    pub recipients: Option<Vec<DividendRecipientItem>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DividendRecipientItem {
    pub name: String,
    pub share_percentage: Option<Decimal>,
    pub amount: Decimal,
    pub payment_method: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DividendFilter {
    pub closing_id: Option<Uuid>,
    pub period_id: Option<Uuid>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClosedPeriodForDividend {
    pub id: Uuid, // ID Dokumen Tutup Buku (accounting_closings.id)
    pub closing_number: String,
    pub name: String,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub closing_date: NaiveDate,
    pub is_closed: bool,
    pub closed_by_name: Option<String>,
    pub net_income: Decimal,
    pub total_dividends_drawn: Decimal,
    pub remaining_income: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DividendSummary {
    pub total_retained_earnings: Decimal,
    pub total_dividends_paid: Decimal,
    pub closed_periods_count: i64,
    pub eligible_closed_periods: Vec<ClosedPeriodForDividend>,
}
