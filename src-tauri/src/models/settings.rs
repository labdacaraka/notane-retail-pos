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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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
}
