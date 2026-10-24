use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct ReceiptSettings {
    pub id: Uuid,
    pub outlet_id: Option<Uuid>,
    pub store_name: String,
    pub store_tagline: Option<String>,
    pub store_address: String,
    pub store_phone: String,
    pub store_website: Option<String>,
    pub tax_id: Option<String>,
    pub header_note: Option<String>,
    pub footer_note_1: Option<String>,
    pub footer_note_2: Option<String>,
    pub social_media: Option<String>,
    pub show_cashier_name: bool,
    pub show_customer_info: bool,
    pub show_loyalty_points: bool,
    pub show_tax_info: bool,
    pub paper_width: String,
    #[serde(default)]
    pub show_phone: bool,
    #[serde(default)]
    pub show_address: bool,
    #[serde(default)]
    pub show_barcode: bool,
    #[serde(default)]
    pub show_flash_sale_discount: bool,
    #[serde(default)]
    pub show_member_discount: bool,
    #[serde(default)]
    pub show_tax_summary: bool,
    #[serde(default)]
    pub show_savings_summary: bool,
    #[serde(default = "default_rounding_mode")]
    pub cash_rounding_mode: String,
    #[serde(default = "default_drawer_limit")]
    pub max_cash_drawer_limit: rust_decimal::Decimal,
    #[serde(default)]
    pub store_logo_url: Option<String>,
    #[serde(default = "default_biometric_mode")]
    pub supervisor_biometric_mode: String,
    #[serde(default = "default_biometric_policy")]
    pub supervisor_biometric_policy: String,
    #[serde(default = "default_face_threshold")]
    pub face_match_threshold: rust_decimal::Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

fn default_rounding_mode() -> String {
    "NONE".to_string()
}

fn default_drawer_limit() -> rust_decimal::Decimal {
    rust_decimal::Decimal::from(2000000)
}

fn default_biometric_mode() -> String {
    "OFF".to_string()
}

fn default_biometric_policy() -> String {
    "OPTIONAL".to_string()
}

fn default_face_threshold() -> rust_decimal::Decimal {
    rust_decimal::Decimal::from(75)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpdateReceiptSettingsRequest {
    pub outlet_id: Option<Uuid>,
    pub store_name: String,
    pub store_tagline: Option<String>,
    pub store_address: String,
    pub store_phone: String,
    pub store_website: Option<String>,
    pub tax_id: Option<String>,
    pub header_note: Option<String>,
    pub footer_note_1: Option<String>,
    pub footer_note_2: Option<String>,
    pub social_media: Option<String>,
    pub show_cashier_name: Option<bool>,
    pub show_customer_info: Option<bool>,
    pub show_loyalty_points: Option<bool>,
    pub show_tax_info: Option<bool>,
    pub paper_width: Option<String>,
    pub show_phone: Option<bool>,
    pub show_address: Option<bool>,
    pub show_barcode: Option<bool>,
    pub show_flash_sale_discount: Option<bool>,
    pub show_member_discount: Option<bool>,
    pub show_tax_summary: Option<bool>,
    pub show_savings_summary: Option<bool>,
    pub cash_rounding_mode: Option<String>,
    pub max_cash_drawer_limit: Option<rust_decimal::Decimal>,
    pub store_logo_url: Option<String>,
    pub supervisor_biometric_mode: Option<String>,
    pub supervisor_biometric_policy: Option<String>,
    pub face_match_threshold: Option<rust_decimal::Decimal>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BiometricSettingsResponse {
    pub supervisor_biometric_mode: String,
    pub supervisor_biometric_policy: String,
    pub face_match_threshold: rust_decimal::Decimal,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpdateBiometricSettingsRequest {
    pub supervisor_biometric_mode: String,
    pub supervisor_biometric_policy: Option<String>,
    pub face_match_threshold: Option<rust_decimal::Decimal>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct LoyaltySettings {
    pub id: Uuid,
    pub is_enabled: bool,
    pub point_value_rupiah: rust_decimal::Decimal,
    pub min_spend_per_point: rust_decimal::Decimal,
    pub max_point_usage_percent: rust_decimal::Decimal,
    pub min_redeem_points: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpdateLoyaltySettingsRequest {
    pub is_enabled: Option<bool>,
    pub point_value_rupiah: Option<rust_decimal::Decimal>,
    pub min_spend_per_point: Option<rust_decimal::Decimal>,
    pub max_point_usage_percent: Option<rust_decimal::Decimal>,
    pub min_redeem_points: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct InventoryCostingSettings {
    pub id: Uuid,
    pub default_costing_method: String,
    pub default_auto_adjust_selling_price: bool,
    pub default_markup_percent: Option<rust_decimal::Decimal>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpdateInventoryCostingSettingsRequest {
    pub default_costing_method: String,
    pub default_auto_adjust_selling_price: Option<bool>,
    pub default_markup_percent: Option<rust_decimal::Decimal>,
}
