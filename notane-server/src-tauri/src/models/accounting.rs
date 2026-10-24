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
    /// Akun sistem bawaan yang tidak boleh diubah atau dinonaktifkan oleh user.
    /// Di-set `true` oleh migrasi untuk akun yang terhubung ke logika bisnis inti.
    pub is_system: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct JournalEntry {
    pub id: Uuid,
    pub journal_number: String,
    pub journal_date: NaiveDate,
    pub reference_type: String, // SALES, PURCHASE, INVENTORY_DISCREPANCY, PAYMENT, EXPENSE, VOID, CLOSING_ENTRY, FUND_TRANSFER
    pub reference_id: Option<Uuid>,
    pub description: String,
    pub total_debit: Decimal,
    pub total_credit: Decimal,
    pub is_closing_entry: bool,
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
    pub outlet_id: Option<Uuid>,
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
    pub outlet_id: Option<Uuid>,
    pub outlet_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct JournalQueryFilter {
    pub limit: Option<i64>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub sort_by: Option<String>,
    pub sort_dir: Option<String>,
    pub search: Option<String>,
    pub outlet_id: Option<Uuid>,
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PaginatedJournalEntries {
    pub data: Vec<JournalEntryDetail>,
    pub total_items: i64,
    pub total_pages: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Serialize)]
pub struct IncomeStatementReport {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub outlet_id: Option<Uuid>,
    pub outlet_name: Option<String>,
    pub total_revenue: Decimal,
    pub total_cogs: Decimal,
    pub gross_profit: Decimal,
    pub total_operating_expenses: Decimal,
    pub net_profit: Decimal,
    pub revenue_details: Vec<AccountSummaryItem>,
    pub expense_details: Vec<AccountSummaryItem>,
}

#[derive(Debug, Serialize)]
pub struct BalanceSheetReport {
    pub as_of_date: NaiveDate,
    pub outlet_id: Option<Uuid>,
    pub outlet_name: Option<String>,
    pub total_assets: Decimal,
    pub current_assets: Decimal,
    pub non_current_assets: Decimal,
    pub total_liabilities: Decimal,
    pub total_equity: Decimal,
    pub total_liabilities_and_equity: Decimal,
    pub is_balanced: bool,
    pub asset_details: Vec<AccountSummaryItem>,
    pub liability_details: Vec<AccountSummaryItem>,
    pub equity_details: Vec<AccountSummaryItem>,
}

#[derive(Debug, Serialize)]
pub struct AccountSummaryItem {
    pub code: String,
    pub name: String,
    pub amount: Decimal,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct AccountingClosing {
    pub id: Uuid,
    pub closing_number: String,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub closing_date: NaiveDate,
    pub outlet_id: Option<Uuid>,
    pub outlet_name: Option<String>,
    pub total_revenue: Decimal,
    pub total_expense: Decimal,
    pub net_income: Decimal,
    pub journal_entry_id: Option<Uuid>,
    pub journal_number: Option<String>,
    pub status: String,
    pub notes: Option<String>,
    pub reopened_at: Option<DateTime<Utc>>,
    pub reopened_by: Option<Uuid>,
    pub reopen_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<Uuid>,
    pub creator_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateClosingRequest {
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub outlet_id: Option<Uuid>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ReopenClosingRequest {
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct ManualJournalRequest {
    pub journal_date: NaiveDate,
    pub description: String,
    pub reference_type: Option<String>,
    pub outlet_id: Option<Uuid>,
    pub lines: Vec<ManualJournalLineRequest>,
}

#[derive(Debug, Deserialize)]
pub struct ManualJournalLineRequest {
    pub account_id: Uuid,
    pub debit: Decimal,
    pub credit: Decimal,
    pub memo: Option<String>,
    pub outlet_id: Option<Uuid>,
}

/// Request body untuk membuat akun baru di Chart of Accounts (Master Data Global)
#[derive(Debug, Deserialize)]
pub struct CreateCoaRequest {
    /// Kategori akun: 1=ASSET, 2=LIABILITY, 3=EQUITY, 4=REVENUE, 5=EXPENSE
    pub account_category_id: i32,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    /// Jika true, akun ini adalah induk / tidak dapat diposting langsung
    pub is_header: Option<bool>,
    pub parent_id: Option<Uuid>,
}

/// Request body untuk mengubah akun COA yang sudah ada
#[derive(Debug, Deserialize)]
pub struct UpdateCoaRequest {
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
}

/// Request body untuk transfer dana / kas antar cabang melalui Rekening Antar-Kantor (RAK)
#[derive(Debug, Deserialize)]
pub struct FundTransferRequest {
    pub transfer_date: NaiveDate,
    pub from_outlet_id: Uuid,
    pub to_outlet_id: Uuid,
    pub from_account_id: Uuid,
    pub to_account_id: Uuid,
    pub amount: Decimal,
    pub notes: Option<String>,
}

/// Response payload hasil eksekusi transfer dana antar cabang
#[derive(Debug, Serialize)]
pub struct FundTransferResponse {
    pub transfer_date: NaiveDate,
    pub from_outlet_id: Uuid,
    pub to_outlet_id: Uuid,
    pub from_outlet_name: String,
    pub to_outlet_name: String,
    pub from_journal_id: Uuid,
    pub to_journal_id: Uuid,
    pub from_journal_number: String,
    pub to_journal_number: String,
    pub amount: Decimal,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct AccountingPeriod {
    pub id: Uuid,
    pub name: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub is_closed: bool,
    pub closed_at: Option<DateTime<Utc>>,
    pub closed_by: Option<Uuid>,
    pub closed_by_name: Option<String>,
    pub reopened_at: Option<DateTime<Utc>>,
    pub reopened_by: Option<Uuid>,
    pub reopened_by_name: Option<String>,
    pub reopen_reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePeriodRequest {
    pub name: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

#[derive(Debug, Deserialize)]
pub struct ReopenPeriodRequest {
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct HardCloseRequest {
    pub fiscal_year: i32,
    pub description: Option<String>,
}

