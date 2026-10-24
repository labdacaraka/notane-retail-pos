use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct BankAccount {
    pub id: Uuid,
    pub bank_name: String,
    pub account_number: String,
    pub account_holder: String,
    pub branch: Option<String>,
    pub outlet_id: Option<Uuid>,
    pub outlet_name: Option<String>,
    pub coa_id: Option<Uuid>,
    pub coa_code: Option<String>,
    pub coa_name: Option<String>,
    pub is_active: bool,
    pub is_default: bool,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateBankAccountRequest {
    pub bank_name: String,
    pub account_number: String,
    pub account_holder: String,
    pub branch: Option<String>,
    pub outlet_id: Option<Uuid>,
    pub is_default: Option<bool>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBankAccountRequest {
    pub bank_name: String,
    pub account_number: String,
    pub account_holder: String,
    pub branch: Option<String>,
    pub outlet_id: Option<Uuid>,
    pub is_active: bool,
    pub is_default: Option<bool>,
    pub notes: Option<String>,
}
