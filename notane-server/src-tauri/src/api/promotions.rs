use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::utils::jwt::{require_any_permission, verify_jwt};

pub fn promotion_routes() -> Router<PgPool> {
    Router::new()
        .route("/promotions", get(list_promotions_handler).post(create_promotion_handler))
        .route("/promotions/active", get(get_active_promotions_handler))
        .route("/promotions/evaluate", post(evaluate_promotions_handler))
        .route(
            "/promotions/{id}",
            get(get_promotion_detail_handler)
                .put(update_promotion_handler)
                .delete(delete_promotion_handler),
        )
}

fn get_user_id(headers: &HeaderMap) -> Result<Uuid, (StatusCode, Json<serde_json::Value>)> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| if h.starts_with("Bearer ") { Some(&h[7..]) } else { None })
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({ "success": false, "message": "Autentikasi dibutuhkan. Silakan login." })),
            )
        })?;

    let claims = verify_jwt(auth_header).map_err(|e| {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "success": false, "message": e })),
        )
    })?;

    Ok(claims.sub)
}

#[derive(Debug, Deserialize)]
pub struct PromotionFilter {
    pub search: Option<String>,
    pub promo_type: Option<String>,
    pub outlet_id: Option<Uuid>,
    pub is_active: Option<bool>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct PromotionItemRow {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub promo_type: String,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub is_active: bool,
    pub min_purchase_amount: Decimal,
    pub outlet_id: Option<Uuid>,
    pub outlet_name: Option<String>,
    pub priority: i32,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct PromotionRuleRow {
    pub id: Uuid,
    pub promotion_id: Uuid,
    pub buy_product_id: Option<Uuid>,
    pub buy_product_name: Option<String>,
    pub buy_product_barcode: Option<String>,
    pub buy_quantity: Decimal,
    pub reward_product_id: Option<Uuid>,
    pub reward_product_name: Option<String>,
    pub reward_product_barcode: Option<String>,
    pub reward_quantity: Decimal,
    pub reward_discount_percent: Decimal,
    pub reward_special_price: Option<Decimal>,
    pub bundle_discount_amount: Decimal,
}

fn deserialize_optional_uuid<'de, D>(deserializer: D) -> Result<Option<Uuid>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt = Option::<serde_json::Value>::deserialize(deserializer)?;
    match opt {
        Some(serde_json::Value::String(s)) if !s.trim().is_empty() => {
            Uuid::parse_str(s.trim()).map(Some).map_err(serde::de::Error::custom)
        }
        _ => Ok(None),
    }
}

fn deserialize_flexible_datetime<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
        return Ok(dt.with_timezone(&Utc));
    }
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S") {
        return Ok(DateTime::from_naive_utc_and_offset(dt, Utc));
    }
    if let Ok(d) = chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d") {
        let dt = chrono::NaiveDateTime::new(d, chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap());
        return Ok(DateTime::from_naive_utc_and_offset(dt, Utc));
    }
    Err(serde::de::Error::custom(format!("Format tanggal tidak valid: '{}'", s)))
}

fn deserialize_flexible_datetime_end<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
        return Ok(dt.with_timezone(&Utc));
    }
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&s, "%Y-%m-%dT%H:%M:%S") {
        return Ok(DateTime::from_naive_utc_and_offset(dt, Utc));
    }
    if let Ok(d) = chrono::NaiveDate::parse_from_str(&s, "%Y-%m-%d") {
        let dt = chrono::NaiveDateTime::new(d, chrono::NaiveTime::from_hms_opt(23, 59, 59).unwrap());
        return Ok(DateTime::from_naive_utc_and_offset(dt, Utc));
    }
    Err(serde::de::Error::custom(format!("Format tanggal tidak valid: '{}'", s)))
}

#[derive(Debug, Deserialize)]
pub struct CreatePromotionRulePayload {
    #[serde(default, deserialize_with = "deserialize_optional_uuid")]
    pub buy_product_id: Option<Uuid>,
    pub buy_quantity: Option<Decimal>,
    #[serde(default, deserialize_with = "deserialize_optional_uuid")]
    pub reward_product_id: Option<Uuid>,
    pub reward_quantity: Option<Decimal>,
    pub reward_discount_percent: Option<Decimal>,
    pub reward_special_price: Option<Decimal>,
    pub reward_price: Option<Decimal>,
    pub bundle_discount_amount: Option<Decimal>,
    pub min_transaction_amount: Option<Decimal>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePromotionPayload {
    pub code: Option<String>,
    pub name: String,
    pub promo_type: String,
    #[serde(deserialize_with = "deserialize_flexible_datetime")]
    pub start_date: DateTime<Utc>,
    #[serde(deserialize_with = "deserialize_flexible_datetime_end")]
    pub end_date: DateTime<Utc>,
    pub is_active: Option<bool>,
    pub min_purchase_amount: Option<Decimal>,
    pub outlet_id: Option<Uuid>,
    pub priority: Option<i32>,
    pub description: Option<String>,
    #[serde(default)]
    pub rules: Vec<CreatePromotionRulePayload>,
}

pub async fn list_promotions_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(filter): Query<PromotionFilter>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    require_any_permission(&pool, &headers, &["promotions:read", "super_admin"]).await?;

    // Proaktif auto-expire promosi yang telah melewati tanggal end_date
    let _ = sqlx::query(
        "UPDATE promotions 
         SET is_active = false, updated_at = CURRENT_TIMESTAMP 
         WHERE is_active = true AND end_date <= CURRENT_TIMESTAMP",
    )
    .execute(&pool)
    .await;

    let page = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(15).max(1);
    let offset = (page - 1) * page_size;

    let search_pattern = filter.search.as_ref().map(|s| format!("%{}%", s.trim().to_lowercase()));

    let query_str = r#"
        SELECT 
            p.id, p.code, p.name, p.promo_type, p.start_date, p.end_date,
            p.is_active, p.min_purchase_amount, p.outlet_id,
            o.name as outlet_name,
            p.priority, p.description, p.created_at, p.updated_at
        FROM promotions p
        LEFT JOIN outlets o ON p.outlet_id = o.id
        WHERE ($1::text IS NULL OR LOWER(p.code) LIKE $1 OR LOWER(p.name) LIKE $1)
          AND ($2::text IS NULL OR p.promo_type = $2)
          AND ($3::uuid IS NULL OR p.outlet_id = $3 OR p.outlet_id IS NULL)
          AND ($4::boolean IS NULL OR p.is_active = $4)
        ORDER BY p.priority DESC, p.created_at DESC
        LIMIT $5 OFFSET $6
    "#;

    let count_query_str = r#"
        SELECT COUNT(*)
        FROM promotions p
        WHERE ($1::text IS NULL OR LOWER(p.code) LIKE $1 OR LOWER(p.name) LIKE $1)
          AND ($2::text IS NULL OR p.promo_type = $2)
          AND ($3::uuid IS NULL OR p.outlet_id = $3 OR p.outlet_id IS NULL)
          AND ($4::boolean IS NULL OR p.is_active = $4)
    "#;

    let total: i64 = sqlx::query_scalar(count_query_str)
        .bind(&search_pattern)
        .bind(&filter.promo_type)
        .bind(filter.outlet_id)
        .bind(filter.is_active)
        .fetch_one(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))))?;

    let rows: Vec<PromotionItemRow> = sqlx::query_as(query_str)
        .bind(&search_pattern)
        .bind(&filter.promo_type)
        .bind(filter.outlet_id)
        .bind(filter.is_active)
        .bind(page_size)
        .bind(offset)
        .fetch_all(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))))?;

    // Dapatkan rules untuk setiap promotion
    let mut results = Vec::new();
    for row in rows {
        let rules: Vec<PromotionRuleRow> = sqlx::query_as(
            r#"
            SELECT 
                r.id, r.promotion_id, r.buy_product_id, bp.name as buy_product_name, bp.barcode as buy_product_barcode,
                r.buy_quantity, r.reward_product_id, rp.name as reward_product_name, rp.barcode as reward_product_barcode,
                r.reward_quantity, r.reward_discount_percent, r.reward_special_price, r.bundle_discount_amount
            FROM promotion_rules r
            LEFT JOIN products bp ON r.buy_product_id = bp.id
            LEFT JOIN products rp ON r.reward_product_id = rp.id
            WHERE r.promotion_id = $1
            "#,
        )
        .bind(row.id)
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

        results.push(json!({
            "id": row.id,
            "code": row.code,
            "name": row.name,
            "promo_type": row.promo_type,
            "start_date": row.start_date,
            "end_date": row.end_date,
            "is_active": row.is_active,
            "min_purchase_amount": row.min_purchase_amount,
            "outlet_id": row.outlet_id,
            "outlet_name": row.outlet_name,
            "priority": row.priority,
            "description": row.description,
            "created_at": row.created_at,
            "updated_at": row.updated_at,
            "rules": rules
        }));
    }

    Ok(Json(json!({
        "success": true,
        "data": results,
        "meta": {
            "page": page,
            "page_size": page_size,
            "total": total,
            "total_pages": (total as f64 / page_size as f64).ceil() as i64
        }
    })))
}

/// Endpoint untuk Kasir POS mendapatkan daftar promo aktif secara real-time
pub async fn get_active_promotions_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(filter): Query<PromotionFilter>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let effective_outlet_id = crate::utils::jwt::resolve_effective_outlet_id(&pool, &headers, filter.outlet_id).await;
    let now = Utc::now();
    let query_str = r#"
        SELECT 
            p.id, p.code, p.name, p.promo_type, p.start_date, p.end_date,
            p.is_active, p.min_purchase_amount, p.outlet_id,
            o.name as outlet_name,
            p.priority, p.description, p.created_at, p.updated_at
        FROM promotions p
        LEFT JOIN outlets o ON p.outlet_id = o.id
        WHERE p.is_active = TRUE
          AND p.start_date <= $1 AND p.end_date >= $1
          AND ($2::uuid IS NULL OR p.outlet_id = $2 OR p.outlet_id IS NULL)
        ORDER BY p.priority DESC, p.created_at DESC
    "#;

    let rows: Vec<PromotionItemRow> = sqlx::query_as(query_str)
        .bind(now)
        .bind(effective_outlet_id)
        .fetch_all(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))))?;

    let mut results = Vec::new();
    for row in rows {
        let rules: Vec<PromotionRuleRow> = sqlx::query_as(
            r#"
            SELECT 
                r.id, r.promotion_id, r.buy_product_id, bp.name as buy_product_name, bp.barcode as buy_product_barcode,
                r.buy_quantity, r.reward_product_id, rp.name as reward_product_name, rp.barcode as reward_product_barcode,
                r.reward_quantity, r.reward_discount_percent, r.reward_special_price, r.bundle_discount_amount
            FROM promotion_rules r
            LEFT JOIN products bp ON r.buy_product_id = bp.id
            LEFT JOIN products rp ON r.reward_product_id = rp.id
            WHERE r.promotion_id = $1
            "#,
        )
        .bind(row.id)
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

        results.push(json!({
            "id": row.id,
            "code": row.code,
            "name": row.name,
            "promo_type": row.promo_type,
            "start_date": row.start_date,
            "end_date": row.end_date,
            "is_active": row.is_active,
            "min_purchase_amount": row.min_purchase_amount,
            "outlet_id": row.outlet_id,
            "outlet_name": row.outlet_name,
            "priority": row.priority,
            "description": row.description,
            "rules": rules
        }));
    }

    Ok(Json(json!({
        "success": true,
        "data": results
    })))
}

pub async fn get_promotion_detail_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    require_any_permission(&pool, &headers, &["promotions:read", "super_admin"]).await?;

    let row: Option<PromotionItemRow> = sqlx::query_as(
        r#"
        SELECT 
            p.id, p.code, p.name, p.promo_type, p.start_date, p.end_date,
            p.is_active, p.min_purchase_amount, p.outlet_id,
            o.name as outlet_name,
            p.priority, p.description, p.created_at, p.updated_at
        FROM promotions p
        LEFT JOIN outlets o ON p.outlet_id = o.id
        WHERE p.id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))))?;

    let row = match row {
        Some(r) => r,
        None => return Err((StatusCode::NOT_FOUND, Json(json!({ "success": false, "message": "Promo tidak ditemukan" })))),
    };

    let rules: Vec<PromotionRuleRow> = sqlx::query_as(
        r#"
        SELECT 
            r.id, r.promotion_id, r.buy_product_id, bp.name as buy_product_name, bp.barcode as buy_product_barcode,
            r.buy_quantity, r.reward_product_id, rp.name as reward_product_name, rp.barcode as reward_product_barcode,
            r.reward_quantity, r.reward_discount_percent, r.reward_special_price, r.bundle_discount_amount
        FROM promotion_rules r
        LEFT JOIN products bp ON r.buy_product_id = bp.id
        LEFT JOIN products rp ON r.reward_product_id = rp.id
        WHERE r.promotion_id = $1
        "#,
    )
    .bind(row.id)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    Ok(Json(json!({
        "success": true,
        "data": {
            "id": row.id,
            "code": row.code,
            "name": row.name,
            "promo_type": row.promo_type,
            "start_date": row.start_date,
            "end_date": row.end_date,
            "is_active": row.is_active,
            "min_purchase_amount": row.min_purchase_amount,
            "outlet_id": row.outlet_id,
            "outlet_name": row.outlet_name,
            "priority": row.priority,
            "description": row.description,
            "created_at": row.created_at,
            "updated_at": row.updated_at,
            "rules": rules
        }
    })))
}

pub async fn create_promotion_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<CreatePromotionPayload>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_user_id(&headers)?;
    require_any_permission(&pool, &headers, &["promotions:create", "promotions:write", "master:write", "super_admin"]).await?;

    let mut tx = pool.begin().await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() })))
    })?;

    let promo_id = Uuid::new_v4();
    let min_purchase = payload
        .min_purchase_amount
        .or_else(|| payload.rules.iter().find_map(|r| r.min_transaction_amount))
        .unwrap_or(Decimal::ZERO);
    let priority = payload.priority.unwrap_or(1);
    let is_active = if payload.end_date <= chrono::Utc::now() {
        false
    } else {
        payload.is_active.unwrap_or(true)
    };

    let code = payload
        .code
        .filter(|c| !c.trim().is_empty())
        .unwrap_or_else(|| {
            let slug: String = payload
                .name
                .to_uppercase()
                .chars()
                .filter(|c| c.is_alphanumeric())
                .take(6)
                .collect();
            let short_id = &promo_id.to_string()[..6].to_uppercase();
            if slug.is_empty() {
                format!("PROMO-{}", short_id)
            } else {
                format!("{}-{}", slug, short_id)
            }
        });

    sqlx::query(
        r#"
        INSERT INTO promotions (
            id, code, name, promo_type, start_date, end_date,
            is_active, min_purchase_amount, outlet_id, priority, description,
            created_by, updated_by
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $12)
        "#,
    )
    .bind(promo_id)
    .bind(code.trim().to_uppercase())
    .bind(payload.name.trim())
    .bind(&payload.promo_type)
    .bind(payload.start_date)
    .bind(payload.end_date)
    .bind(is_active)
    .bind(min_purchase)
    .bind(payload.outlet_id)
    .bind(priority)
    .bind(&payload.description)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": format!("Gagal membuat promo: {}", e) }))))?;

    for rule in payload.rules {
        let buy_qty = rule.buy_quantity.unwrap_or(Decimal::ONE);
        let reward_qty = rule.reward_quantity.unwrap_or(Decimal::ONE);
        let disc_pct = rule.reward_discount_percent.unwrap_or(Decimal::from(100));
        let reward_price = rule.reward_special_price.or(rule.reward_price);
        let bundle_disc = rule.bundle_discount_amount.unwrap_or(Decimal::ZERO);

        sqlx::query(
            r#"
            INSERT INTO promotion_rules (
                promotion_id, buy_product_id, buy_quantity,
                reward_product_id, reward_quantity, reward_discount_percent,
                reward_special_price, bundle_discount_amount
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(promo_id)
        .bind(rule.buy_product_id)
        .bind(buy_qty)
        .bind(rule.reward_product_id)
        .bind(reward_qty)
        .bind(disc_pct)
        .bind(reward_price)
        .bind(bundle_disc)
        .execute(&mut *tx)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": format!("Gagal menyimpan aturan promo: {}", e) }))))?;
    }

    tx.commit().await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() })))
    })?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "success": true,
            "message": "Program promo berhasil dibuat",
            "data": { "id": promo_id }
        })),
    ))
}

pub async fn update_promotion_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<CreatePromotionPayload>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let user_id = get_user_id(&headers)?;
    require_any_permission(&pool, &headers, &["promotions:update", "promotions:write", "master:write", "super_admin"]).await?;

    let mut tx = pool.begin().await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() })))
    })?;

    let min_purchase = payload
        .min_purchase_amount
        .or_else(|| payload.rules.iter().find_map(|r| r.min_transaction_amount))
        .unwrap_or(Decimal::ZERO);
    let priority = payload.priority.unwrap_or(1);
    let is_active = if payload.end_date <= chrono::Utc::now() {
        false
    } else {
        payload.is_active.unwrap_or(true)
    };

    let code = payload
        .code
        .filter(|c| !c.trim().is_empty())
        .unwrap_or_else(|| {
            let short_id = &id.to_string()[..6].to_uppercase();
            format!("PROMO-{}", short_id)
        });

    sqlx::query(
        r#"
        UPDATE promotions SET
            code = $1, name = $2, promo_type = $3, start_date = $4, end_date = $5,
            is_active = $6, min_purchase_amount = $7, outlet_id = $8, priority = $9,
            description = $10, updated_by = $11, updated_at = CURRENT_TIMESTAMP
        WHERE id = $12
        "#,
    )
    .bind(code.trim().to_uppercase())
    .bind(payload.name.trim())
    .bind(&payload.promo_type)
    .bind(payload.start_date)
    .bind(payload.end_date)
    .bind(is_active)
    .bind(min_purchase)
    .bind(payload.outlet_id)
    .bind(priority)
    .bind(&payload.description)
    .bind(user_id)
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": format!("Gagal mengubah promo: {}", e) }))))?;

    // Ganti rules lama
    sqlx::query("DELETE FROM promotion_rules WHERE promotion_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))))?;

    for rule in payload.rules {
        let buy_qty = rule.buy_quantity.unwrap_or(Decimal::ONE);
        let reward_qty = rule.reward_quantity.unwrap_or(Decimal::ONE);
        let disc_pct = rule.reward_discount_percent.unwrap_or(Decimal::from(100));
        let reward_price = rule.reward_special_price.or(rule.reward_price);
        let bundle_disc = rule.bundle_discount_amount.unwrap_or(Decimal::ZERO);

        sqlx::query(
            r#"
            INSERT INTO promotion_rules (
                promotion_id, buy_product_id, buy_quantity,
                reward_product_id, reward_quantity, reward_discount_percent,
                reward_special_price, bundle_discount_amount
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(id)
        .bind(rule.buy_product_id)
        .bind(buy_qty)
        .bind(rule.reward_product_id)
        .bind(reward_qty)
        .bind(disc_pct)
        .bind(reward_price)
        .bind(bundle_disc)
        .execute(&mut *tx)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({ "success": false, "message": format!("Gagal menyimpan aturan promo: {}", e) }))))?;
    }

    tx.commit().await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() })))
    })?;

    Ok(Json(json!({
        "success": true,
        "message": "Program promo berhasil diperbarui"
    })))
}

pub async fn delete_promotion_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    require_any_permission(&pool, &headers, &["promotions:delete", "promotions:write", "master:write", "super_admin"]).await?;

    sqlx::query("DELETE FROM promotions WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))))?;

    Ok(Json(json!({
        "success": true,
        "message": "Program promo berhasil dihapus"
    })))
}

#[derive(Debug, Deserialize, Clone)]
pub struct EvaluatePromotionItemInput {
    pub product_id: Uuid,
    pub unit_id: Option<Uuid>,
    pub quantity: Decimal,
    pub unit_selling_price: Decimal,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EvaluatePromotionsRequest {
    pub outlet_id: Option<Uuid>,
    pub customer_id: Option<Uuid>,
    pub items: Vec<EvaluatePromotionItemInput>,
    #[serde(default)]
    pub dismissed_rule_ids: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ItemPromotionAdjustment {
    pub product_id: Uuid,
    pub promo_id: Uuid,
    pub promo_name: String,
    pub discount_amount: Decimal,
    pub is_free_item: bool,
    pub free_quantity: Decimal,
}

#[derive(Debug, Serialize, Clone)]
pub struct AutoGiftProduct {
    pub promo_id: Uuid,
    pub promo_name: String,
    pub rule_id: String,
    pub product_id: Uuid,
    pub product_name: String,
    pub barcode: Option<String>,
    pub sku: String,
    pub unit_id: Uuid,
    pub unit_code: String,
    pub unit_name: String,
    pub unit_price: Decimal,
    pub quantity: Decimal,
    pub promo_discount: Decimal,
    pub available_stock: Decimal,
    pub is_free_item: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct OutOfStockGiftDetail {
    pub promo_id: Uuid,
    pub promo_name: String,
    pub rule_id: String,
    pub reward_product_id: Uuid,
    pub reward_product_name: String,
    pub reward_product_barcode: Option<String>,
    pub required_quantity: Decimal,
    pub available_stock: Decimal,
}

#[derive(Debug, Serialize, Clone, Default)]
pub struct PromotionEvaluationResult {
    pub item_adjustments: Vec<ItemPromotionAdjustment>,
    pub auto_gifts: Vec<AutoGiftProduct>,
    pub out_of_stock_gifts: Vec<OutOfStockGiftDetail>,
    pub total_promo_discount: Decimal,
}

pub async fn evaluate_promotions_logic(
    pool: &PgPool,
    req: &EvaluatePromotionsRequest,
) -> Result<PromotionEvaluationResult, String> {
    let now = Utc::now();
    let mut result = PromotionEvaluationResult::default();

    if req.items.is_empty() {
        return Ok(result);
    }

    // 1. Ambil promo aktif untuk outlet ini
    let promotions: Vec<PromotionItemRow> = sqlx::query_as(
        r#"
        SELECT 
            p.id, p.code, p.name, p.promo_type, p.start_date, p.end_date,
            p.is_active, p.min_purchase_amount, p.outlet_id,
            o.name as outlet_name,
            p.priority, p.description, p.created_at, p.updated_at
        FROM promotions p
        LEFT JOIN outlets o ON p.outlet_id = o.id
        WHERE p.is_active = TRUE
          AND p.start_date <= $1 AND p.end_date >= $1
          AND ($2::uuid IS NULL OR p.outlet_id = $2 OR p.outlet_id IS NULL)
        ORDER BY p.priority DESC, p.created_at DESC
        "#,
    )
    .bind(now)
    .bind(req.outlet_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?;

    if promotions.is_empty() {
        return Ok(result);
    }

    // Buat map akumulasi kuantitas produk dalam cart
    let mut buy_qty_map: std::collections::HashMap<Uuid, Decimal> = std::collections::HashMap::new();
    let mut item_price_map: std::collections::HashMap<Uuid, Decimal> = std::collections::HashMap::new();

    for it in &req.items {
        *buy_qty_map.entry(it.product_id).or_insert(Decimal::ZERO) += it.quantity;
        item_price_map.insert(it.product_id, it.unit_selling_price);
    }

    let dismissed_set: std::collections::HashSet<String> = req.dismissed_rule_ids.iter().cloned().collect();

    // Loop promo & rules
    for promo in promotions {
        let rules: Vec<PromotionRuleRow> = sqlx::query_as(
            r#"
            SELECT 
                r.id, r.promotion_id, r.buy_product_id, bp.name as buy_product_name, bp.barcode as buy_product_barcode,
                r.buy_quantity, r.reward_product_id, rp.name as reward_product_name, rp.barcode as reward_product_barcode,
                r.reward_quantity, r.reward_discount_percent, r.reward_special_price, r.bundle_discount_amount
            FROM promotion_rules r
            LEFT JOIN products bp ON r.buy_product_id = bp.id
            LEFT JOIN products rp ON r.reward_product_id = rp.id
            WHERE r.promotion_id = $1
            "#,
        )
        .bind(promo.id)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        for rule in rules {
            if promo.promo_type == "BUY_X_GET_Y" {
                if let Some(buy_prod_id) = rule.buy_product_id {
                    let total_buy_qty = *buy_qty_map.get(&buy_prod_id).unwrap_or(&Decimal::ZERO);
                    let buy_req = if rule.buy_quantity > Decimal::ZERO { rule.buy_quantity } else { Decimal::ONE };

                    let reward_prod_id = rule.reward_product_id.unwrap_or(buy_prod_id);
                    let rule_key = rule.id.to_string();
                    let fallback_rule_key = format!("{}-{}", promo.id, reward_prod_id);

                    if dismissed_set.contains(&rule_key) || dismissed_set.contains(&fallback_rule_key) {
                        continue;
                    }

                    if buy_prod_id == reward_prod_id {
                        // Beli X Gratis X (Produk sama)
                        let reward_req = if rule.reward_quantity > Decimal::ZERO { rule.reward_quantity } else { Decimal::ONE };
                        let set_size = buy_req + reward_req;
                        let full_sets = (total_buy_qty / set_size).floor();
                        let free_qty = full_sets * reward_req;

                        if free_qty > Decimal::ZERO {
                            let unit_price = *item_price_map.get(&buy_prod_id).unwrap_or(&Decimal::ZERO);
                            let disc_pct = rule.reward_discount_percent;
                            let disc_per_unit = (unit_price * disc_pct) / Decimal::from(100);
                            let disc_amount = free_qty * disc_per_unit;

                            result.total_promo_discount += disc_amount;
                            result.item_adjustments.push(ItemPromotionAdjustment {
                                product_id: buy_prod_id,
                                promo_id: promo.id,
                                promo_name: promo.name.clone(),
                                discount_amount: disc_amount,
                                is_free_item: disc_pct >= Decimal::from(100),
                                free_quantity: free_qty,
                            });
                        }
                    } else {
                        // Beli X Gratis Y (Produk beda)
                        if total_buy_qty >= buy_req {
                            let eligible_sets = (total_buy_qty / buy_req).floor();
                            let reward_req = if rule.reward_quantity > Decimal::ZERO { rule.reward_quantity } else { Decimal::ONE };
                            let target_reward_qty = eligible_sets * reward_req;

                            if target_reward_qty > Decimal::ZERO {
                                // Cek stok riil reward di database untuk outlet ini
                                let target_wh = req.outlet_id;
                                let available_stock: Decimal = if let Some(wh_id) = target_wh {
                                    sqlx::query_scalar(
                                        "SELECT quantity FROM warehouse_stocks WHERE warehouse_id = $1 AND product_id = $2"
                                    )
                                    .bind(wh_id)
                                    .bind(reward_prod_id)
                                    .fetch_optional(pool)
                                    .await
                                    .unwrap_or(None)
                                    .unwrap_or(Decimal::ZERO)
                                } else {
                                    sqlx::query_scalar(
                                        "SELECT COALESCE(SUM(quantity), 0) FROM warehouse_stocks WHERE product_id = $1"
                                    )
                                    .bind(reward_prod_id)
                                    .fetch_one(pool)
                                    .await
                                    .unwrap_or(Decimal::ZERO)
                                };

                                // Ambil info produk reward
                                let prod_row = sqlx::query_as::<_, (String, Option<String>, String, Uuid, Decimal, String, String)>(
                                    r#"
                                    SELECT 
                                        p.name, p.barcode, p.sku, p.base_unit_id, p.base_selling_price,
                                        u.code as unit_code, u.name as unit_name
                                    FROM products p
                                    JOIN units u ON p.base_unit_id = u.id
                                    WHERE p.id = $1
                                    "#,
                                )
                                .bind(reward_prod_id)
                                .fetch_optional(pool)
                                .await
                                .unwrap_or(None);

                                if let Some((pname, pbarcode, psku, pbase_unit, pbase_price, ucode, uname)) = prod_row {
                                    if available_stock <= Decimal::ZERO {
                                        // Stok kosong di toko/outlet
                                        result.out_of_stock_gifts.push(OutOfStockGiftDetail {
                                            promo_id: promo.id,
                                            promo_name: promo.name.clone(),
                                            rule_id: rule.id.to_string(),
                                            reward_product_id: reward_prod_id,
                                            reward_product_name: pname,
                                            reward_product_barcode: pbarcode,
                                            required_quantity: target_reward_qty,
                                            available_stock: Decimal::ZERO,
                                        });
                                    } else {
                                        let grant_qty = std::cmp::min(target_reward_qty, available_stock);
                                        let disc_amount = pbase_price * grant_qty;

                                        result.total_promo_discount += disc_amount;
                                        result.auto_gifts.push(AutoGiftProduct {
                                            promo_id: promo.id,
                                            promo_name: promo.name.clone(),
                                            rule_id: rule.id.to_string(),
                                            product_id: reward_prod_id,
                                            product_name: pname.clone(),
                                            barcode: pbarcode.clone(),
                                            sku: psku,
                                            unit_id: pbase_unit,
                                            unit_code: ucode,
                                            unit_name: uname,
                                            unit_price: pbase_price,
                                            quantity: grant_qty,
                                            promo_discount: disc_amount,
                                            available_stock,
                                            is_free_item: true,
                                        });

                                        if grant_qty < target_reward_qty {
                                            result.out_of_stock_gifts.push(OutOfStockGiftDetail {
                                                promo_id: promo.id,
                                                promo_name: promo.name.clone(),
                                                rule_id: rule.id.to_string(),
                                                reward_product_id: reward_prod_id,
                                                reward_product_name: pname,
                                                reward_product_barcode: pbarcode,
                                                required_quantity: target_reward_qty - grant_qty,
                                                available_stock,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(result)
}

pub async fn evaluate_promotions_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<EvaluatePromotionsRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    require_any_permission(&pool, &headers, &["pos:checkout", "promotions:read", "master:read"]).await?;

    let result = evaluate_promotions_logic(&pool, &payload).await.map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e })))
    })?;

    Ok(Json(json!({
        "success": true,
        "data": result
    })))
}
