use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct OtherIncomeCategory {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub account_id: Option<Uuid>,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[sqlx(default)]
    pub account_name: Option<String>,
    #[sqlx(default)]
    pub account_code: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct OtherIncomeTransaction {
    pub id: Uuid,
    pub transaction_no: String,
    pub outlet_id: Uuid,
    pub category_id: Option<Uuid>,
    pub income_account_id: Uuid,
    pub payment_account_id: Uuid,
    pub amount: Decimal,
    pub transaction_date: NaiveDate,
    pub payer_name: Option<String>,
    pub notes: Option<String>,
    pub reference_no: Option<String>,
    pub attachment_url: Option<String>,
    pub shift_id: Option<Uuid>,
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
pub struct OtherIncomeTransactionDetail {
    pub id: Uuid,
    pub transaction_no: String,
    pub outlet_id: Uuid,
    pub outlet_name: String,
    pub category_id: Option<Uuid>,
    pub category_name: Option<String>,
    pub income_account_id: Uuid,
    pub income_account_code: String,
    pub income_account_name: String,
    pub payment_account_id: Uuid,
    pub payment_account_code: String,
    pub payment_account_name: String,
    pub amount: Decimal,
    pub transaction_date: NaiveDate,
    pub payer_name: Option<String>,
    pub notes: Option<String>,
    pub reference_no: Option<String>,
    pub attachment_url: Option<String>,
    pub shift_id: Option<Uuid>,
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

#[derive(Debug, Deserialize)]
pub struct CreateOtherIncomeRequest {
    pub outlet_id: Uuid,
    pub category_id: Option<Uuid>,
    pub income_account_id: Uuid,
    pub payment_account_id: Uuid,
    pub amount: Decimal,
    pub transaction_date: Option<NaiveDate>,
    pub payer_name: Option<String>,
    pub notes: Option<String>,
    pub reference_no: Option<String>,
    pub attachment_url: Option<String>,
    pub shift_id: Option<Uuid>,
}

impl CreateOtherIncomeRequest {
    pub fn validate(&self) -> Result<(), String> {
        if self.amount <= Decimal::ZERO {
            return Err("Nominal pendapatan harus lebih besar dari Rp 0.".to_string());
        }
        if let Some(ref p) = self.payer_name {
            if p.trim().len() > 150 {
                return Err("Nama pembayar / penyetor maksimal 150 karakter.".to_string());
            }
        }
        if let Some(ref r) = self.reference_no {
            if r.trim().len() > 100 {
                return Err("Nomor referensi / bukti nota maksimal 100 karakter.".to_string());
            }
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct CancelOtherIncomeRequest {
    pub reason: String,
}

impl CancelOtherIncomeRequest {
    pub fn validate(&self) -> Result<(), String> {
        let r = self.reason.trim();
        if r.len() < 3 {
            return Err("Alasan pembatalan minimal 3 karakter.".to_string());
        }
        if r.len() > 500 {
            return Err("Alasan pembatalan maksimal 500 karakter.".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateOtherIncomeCategoryRequest {
    pub code: String,
    pub name: String,
    pub account_id: Option<Uuid>,
    pub description: Option<String>,
}

impl CreateOtherIncomeCategoryRequest {
    pub fn validate(&self) -> Result<(), String> {
        let c = self.code.trim();
        if c.len() < 2 || c.len() > 50 {
            return Err("Kode kategori harus antara 2 hingga 50 karakter.".to_string());
        }
        let n = self.name.trim();
        if n.len() < 2 || n.len() > 150 {
            return Err("Nama kategori harus antara 2 hingga 150 karakter.".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateOtherIncomeCategoryRequest {
    pub name: String,
    pub account_id: Option<Uuid>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
}

impl UpdateOtherIncomeCategoryRequest {
    pub fn validate(&self) -> Result<(), String> {
        let n = self.name.trim();
        if n.len() < 2 || n.len() > 150 {
            return Err("Nama kategori harus antara 2 hingga 150 karakter.".to_string());
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct OtherIncomeCategoryFilterQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub search: Option<String>,
    pub active_only: Option<bool>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OtherIncomeFilterQuery {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub outlet_id: Option<Uuid>,
    pub category_id: Option<Uuid>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub status: Option<String>,
    pub shift_id: Option<Uuid>,
    pub search: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct OtherIncomeCategoryBreakdown {
    pub category_id: Option<Uuid>,
    pub category_name: String,
    pub total_amount: Decimal,
    pub transaction_count: i64,
    pub percentage: Decimal,
}

#[derive(Debug, Serialize, Clone)]
pub struct OtherIncomeSummaryResponse {
    pub total_amount: Decimal,
    pub total_count: i64,
    pub today_amount: Decimal,
    pub this_month_amount: Decimal,
    pub categories_breakdown: Vec<OtherIncomeCategoryBreakdown>,
}

