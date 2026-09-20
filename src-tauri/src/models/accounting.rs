use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct AccountCategory {
    pub id: i32,
    pub name: String,
    pub normal_balance: String, // DEBIT, CREDIT
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct ChartOfAccount {
    pub id: Uuid,
    pub account_category_id: i32,
    pub parent_id: Option<Uuid>,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub is_header: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct JournalEntry {
    pub id: Uuid,
    pub journal_number: String,
    pub journal_date: NaiveDate,
    pub reference_type: String, // SALES, PURCHASE, INVENTORY_DISCREPANCY, PAYMENT, EXPENSE, VOID
    pub reference_id: Option<Uuid>,
    pub description: String,
    pub total_debit: Decimal,
    pub total_credit: Decimal,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct JournalLine {
    pub id: Uuid,
    pub journal_id: Uuid,
    pub account_id: Uuid,
    pub debit: Decimal,
    pub credit: Decimal,
    pub memo: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JournalEntryDetail {
    pub entry: JournalEntry,
    pub lines: Vec<JournalLineDetail>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JournalLineDetail {
    pub id: Uuid,
    pub account_id: Uuid,
    pub account_code: String,
    pub account_name: String,
    pub debit: Decimal,
    pub credit: Decimal,
    pub memo: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct IncomeStatementReport {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub total_revenue: Decimal,
    pub total_cogs: Decimal,
    pub gross_profit: Decimal,
    pub total_operating_expenses: Decimal,
    pub net_profit: Decimal,
    pub revenue_details: Vec<AccountSummaryItem>,
    pub expense_details: Vec<AccountSummaryItem>,
}

#[derive(Debug, Serialize)]
pub struct AccountSummaryItem {
    pub code: String,
    pub name: String,
    pub amount: Decimal,
}
