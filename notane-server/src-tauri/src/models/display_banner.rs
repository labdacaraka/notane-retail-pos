use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CustomerDisplayBanner {
    pub id: Uuid,
    pub title: String,
    pub subtitle: Option<String>,
    pub image_url: String,
    pub brand_name: Option<String>,
    pub badge_text: Option<String>,
    pub is_active: bool,
    pub display_order: i32,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<Uuid>,
    pub updated_by: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct CreateBannerPayload {
    pub title: String,
    pub subtitle: Option<String>,
    pub image_url: String,
    pub brand_name: Option<String>,
    pub badge_text: Option<String>,
    pub is_active: Option<bool>,
    pub display_order: Option<i32>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBannerPayload {
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub image_url: Option<String>,
    pub brand_name: Option<String>,
    pub badge_text: Option<String>,
    pub is_active: Option<bool>,
    pub display_order: Option<i32>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
}

#[derive(Debug, Deserialize)]
pub struct ReorderBannersPayload {
    pub items: Vec<BannerOrderItem>,
}

#[derive(Debug, Deserialize)]
pub struct BannerOrderItem {
    pub id: Uuid,
    pub display_order: i32,
}

