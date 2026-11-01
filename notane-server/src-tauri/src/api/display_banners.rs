use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::models::display_banner::{
    CreateBannerPayload, CustomerDisplayBanner, ReorderBannersPayload, UpdateBannerPayload,
};
use crate::utils::jwt::{require_license_feature, require_permission};

pub fn display_banner_routes() -> Router<PgPool> {
    Router::new()
        // Endpoint publik untuk Layar Hadap Pelanggan (Customer Display)
        .route("/customer-display/banners", get(get_active_display_banners_handler))
        // Endpoint manajemen admin (CRUD)
        .route("/display-banners", get(list_display_banners_handler).post(create_display_banner_handler))
        .route("/display-banners/reorder", post(reorder_display_banners_handler))
        .route(
            "/display-banners/{id}",
            get(get_display_banner_handler)
                .put(update_display_banner_handler)
                .delete(delete_display_banner_handler),
        )
}

#[derive(Debug, Deserialize)]
pub struct BannerListQuery {
    pub search: Option<String>,
    pub status: Option<String>, // "ALL", "ACTIVE", "INACTIVE"
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    pub limit: Option<i64>,
    pub sort_by: Option<String>,
    pub sort_dir: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct BannerSummaryStats {
    total: i64,
    active: i64,
    inactive: i64,
    brands: i64,
}

fn apply_banner_filters(
    builder: &mut QueryBuilder<Postgres>,
    search: Option<&str>,
    status: Option<&str>,
) {
    if let Some(st) = status {
        match st.to_uppercase().as_str() {
            "ACTIVE" => {
                builder.push(" AND is_active = true");
            }
            "INACTIVE" => {
                builder.push(" AND is_active = false");
            }
            _ => {}
        }
    }

    if let Some(q) = search {
        let q_clean = q.trim();
        if !q_clean.is_empty() {
            let pattern = format!("%{}%", q_clean.to_lowercase());
            builder.push(" AND (LOWER(title) LIKE ");
            builder.push_bind(pattern.clone());
            builder.push(" OR LOWER(COALESCE(brand_name, '')) LIKE ");
            builder.push_bind(pattern.clone());
            builder.push(" OR LOWER(COALESCE(subtitle, '')) LIKE ");
            builder.push_bind(pattern.clone());
            builder.push(" OR LOWER(COALESCE(badge_text, '')) LIKE ");
            builder.push_bind(pattern);
            builder.push(")");
        }
    }
}

/// 1. GET /customer-display/banners
/// Mengambil seluruh banner promosi yang aktif dan dalam masa berlaku untuk ditayangkan pada Customer Display.
async fn get_active_display_banners_handler(State(pool): State<PgPool>) -> impl IntoResponse {
    if !crate::services::license_service::LicenseService::has_feature(&pool, "customer_display").await {
        return (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "data": Vec::<CustomerDisplayBanner>::new()
            })),
        )
            .into_response();
    }

    let banners = sqlx::query_as::<_, CustomerDisplayBanner>(
        "SELECT * FROM customer_display_banners
         WHERE is_active = true
           AND (start_date IS NULL OR start_date <= CURRENT_DATE)
           AND (end_date IS NULL OR end_date >= CURRENT_DATE)
         ORDER BY display_order ASC, created_at DESC",
    )
    .fetch_all(&pool)
    .await;

    match banners {
        Ok(items) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "data": items
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "message": format!("Gagal memuat banner customer display: {}", e)
            })),
        )
            .into_response(),
    }
}

/// 2. GET /display-banners
/// Mengambil daftar banner promosi untuk tabel manajemen di Admin Back-Office dengan server-side pagination, sorting, dan filter.
async fn list_display_banners_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(query): Query<BannerListQuery>,
) -> impl IntoResponse {
    if let Err(resp) = require_license_feature(&pool, "customer_display", "Customer Display & Banner Promo").await {
        return resp.into_response();
    }

    if let Err(resp) = require_permission(&pool, &headers, "display_banners:read").await {
        return resp.into_response();
    }

    // 1. Ambil summary statistik global (total, active, inactive, brands)
    let stats = sqlx::query_as::<_, BannerSummaryStats>(
        "SELECT 
            COUNT(*)::bigint AS total,
            COUNT(*) FILTER (WHERE is_active = true)::bigint AS active,
            COUNT(*) FILTER (WHERE is_active = false)::bigint AS inactive,
            COUNT(DISTINCT brand_name) FILTER (WHERE brand_name IS NOT NULL AND TRIM(brand_name) <> '')::bigint AS brands
         FROM customer_display_banners"
    )
    .fetch_one(&pool)
    .await
    .unwrap_or(BannerSummaryStats {
        total: 0,
        active: 0,
        inactive: 0,
        brands: 0,
    });

    // 2. Query Count dengan filter
    let mut count_builder: QueryBuilder<Postgres> =
        QueryBuilder::new("SELECT COUNT(*)::bigint FROM customer_display_banners WHERE 1=1");
    apply_banner_filters(&mut count_builder, query.search.as_deref(), query.status.as_deref());

    let total_items: i64 = count_builder
        .build_query_scalar()
        .fetch_one(&pool)
        .await
        .unwrap_or(0);

    // 3. Query Data dengan filter, sorting, dan pagination
    let mut data_builder: QueryBuilder<Postgres> =
        QueryBuilder::new("SELECT * FROM customer_display_banners WHERE 1=1");
    apply_banner_filters(&mut data_builder, query.search.as_deref(), query.status.as_deref());

    let sort_col = match query.sort_by.as_deref() {
        Some("title") => "title",
        Some("brand_name") => "brand_name",
        Some("start_date") => "start_date",
        Some("end_date") => "end_date",
        Some("is_active") => "is_active",
        Some("created_at") => "created_at",
        _ => "display_order",
    };

    let sort_dir = match query.sort_dir.as_deref() {
        Some("desc") | Some("DESC") => "DESC",
        _ => "ASC",
    };

    data_builder.push(format!(" ORDER BY {} {}", sort_col, sort_dir));
    if sort_col != "display_order" {
        data_builder.push(", display_order ASC");
    }
    data_builder.push(", created_at DESC");

    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(query.limit.unwrap_or(12)).max(1).min(100);
    let offset = (page - 1) * page_size;

    data_builder.push(" LIMIT ");
    data_builder.push_bind(page_size);
    data_builder.push(" OFFSET ");
    data_builder.push_bind(offset);

    let items = data_builder
        .build_query_as::<CustomerDisplayBanner>()
        .fetch_all(&pool)
        .await;

    let total_pages = if total_items > 0 {
        ((total_items as f64) / (page_size as f64)).ceil() as i64
    } else {
        1
    };

    match items {
        Ok(list) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "data": list,
                "total": total_items,
                "total_items": total_items,
                "page": page,
                "page_size": page_size,
                "total_pages": total_pages,
                "summary": {
                    "total": stats.total,
                    "active": stats.active,
                    "inactive": stats.inactive,
                    "brands": stats.brands,
                }
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "message": format!("Database error: {}", e)
            })),
        )
            .into_response(),
    }
}

/// 3. GET /display-banners/{id}
async fn get_display_banner_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(banner_id): Path<Uuid>,
) -> impl IntoResponse {
    if let Err(resp) = require_license_feature(&pool, "customer_display", "Customer Display & Banner Promo").await {
        return resp.into_response();
    }

    if let Err(resp) = require_permission(&pool, &headers, "display_banners:read").await {
        return resp.into_response();
    }

    let banner = sqlx::query_as::<_, CustomerDisplayBanner>(
        "SELECT * FROM customer_display_banners WHERE id = $1",
    )
    .bind(banner_id)
    .fetch_optional(&pool)
    .await;

    match banner {
        Ok(Some(item)) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "data": item
            })),
        )
            .into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "message": "Banner promosi tidak ditemukan"
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "message": format!("Database error: {}", e)
            })),
        )
            .into_response(),
    }
}

/// 4. POST /display-banners
/// Membuat banner promosi baru.
async fn create_display_banner_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<CreateBannerPayload>,
) -> impl IntoResponse {
    if let Err(resp) = require_license_feature(&pool, "customer_display", "Customer Display & Banner Promo").await {
        return resp.into_response();
    }

    let claims = match require_permission(&pool, &headers, "display_banners:create").await {
        Ok(c) => c,
        Err(resp) => return resp.into_response(),
    };

    let title_clean = payload.title.trim();
    if title_clean.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "Judul banner promosi wajib diisi" })),
        )
            .into_response();
    }

    let image_clean = payload.image_url.trim();
    if image_clean.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "Gambar banner promosi wajib disertakan" })),
        )
            .into_response();
    }

    let subtitle_clean = payload.subtitle.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty());
    let brand_clean = payload.brand_name.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty());
    let badge_clean = payload.badge_text.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty()).unwrap_or("PROMO SPESIAL");
    let is_active = payload.is_active.unwrap_or(true);
    let display_order = payload.display_order.unwrap_or(0);

    let inserted = sqlx::query_as::<_, CustomerDisplayBanner>(
        "INSERT INTO customer_display_banners (
            title, subtitle, image_url, brand_name, badge_text, is_active, display_order, start_date, end_date, created_by, updated_by
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $10)
        RETURNING *",
    )
    .bind(title_clean)
    .bind(subtitle_clean)
    .bind(image_clean)
    .bind(brand_clean)
    .bind(badge_clean)
    .bind(is_active)
    .bind(display_order)
    .bind(payload.start_date)
    .bind(payload.end_date)
    .bind(claims.sub)
    .fetch_one(&pool)
    .await;

    match inserted {
        Ok(item) => (
            StatusCode::CREATED,
            Json(json!({
                "success": true,
                "message": "Banner promosi berhasil ditambahkan",
                "data": item
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "message": format!("Gagal menyimpan banner promosi: {}", e)
            })),
        )
            .into_response(),
    }
}

/// 5. PUT /display-banners/{id}
/// Memperbarui informasi banner promosi.
async fn update_display_banner_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(banner_id): Path<Uuid>,
    Json(payload): Json<UpdateBannerPayload>,
) -> impl IntoResponse {
    if let Err(resp) = require_license_feature(&pool, "customer_display", "Customer Display & Banner Promo").await {
        return resp.into_response();
    }

    let claims = match require_permission(&pool, &headers, "display_banners:update").await {
        Ok(c) => c,
        Err(resp) => return resp.into_response(),
    };

    // Ambil data banner saat ini
    let current = sqlx::query_as::<_, CustomerDisplayBanner>(
        "SELECT * FROM customer_display_banners WHERE id = $1",
    )
    .bind(banner_id)
    .fetch_optional(&pool)
    .await;

    let existing = match current {
        Ok(Some(item)) => item,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "success": false, "message": "Banner promosi tidak ditemukan" })),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "success": false, "message": format!("Database error: {}", e) })),
            )
                .into_response();
        }
    };

    let new_title = payload.title.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty()).unwrap_or(&existing.title);
    let new_subtitle = payload.subtitle.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty()).or(existing.subtitle.as_deref());
    let new_image = payload.image_url.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty()).unwrap_or(&existing.image_url);
    let new_brand = payload.brand_name.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty()).or(existing.brand_name.as_deref());
    let new_badge = payload.badge_text.as_deref().map(|s| s.trim()).filter(|s| !s.is_empty()).or(existing.badge_text.as_deref());
    let new_is_active = payload.is_active.unwrap_or(existing.is_active);
    let new_order = payload.display_order.unwrap_or(existing.display_order);
    let new_start = payload.start_date.or(existing.start_date);
    let new_end = payload.end_date.or(existing.end_date);

    let updated = sqlx::query_as::<_, CustomerDisplayBanner>(
        "UPDATE customer_display_banners SET
            title = $1,
            subtitle = $2,
            image_url = $3,
            brand_name = $4,
            badge_text = $5,
            is_active = $6,
            display_order = $7,
            start_date = $8,
            end_date = $9,
            updated_by = $10,
            updated_at = CURRENT_TIMESTAMP
         WHERE id = $11
         RETURNING *",
    )
    .bind(new_title)
    .bind(new_subtitle)
    .bind(new_image)
    .bind(new_brand)
    .bind(new_badge)
    .bind(new_is_active)
    .bind(new_order)
    .bind(new_start)
    .bind(new_end)
    .bind(claims.sub)
    .bind(banner_id)
    .fetch_one(&pool)
    .await;

    match updated {
        Ok(item) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "message": "Banner promosi berhasil diperbarui",
                "data": item
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "message": format!("Gagal memperbarui banner promosi: {}", e)
            })),
        )
            .into_response(),
    }
}

/// 6. DELETE /display-banners/{id}
async fn delete_display_banner_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(banner_id): Path<Uuid>,
) -> impl IntoResponse {
    if let Err(resp) = require_license_feature(&pool, "customer_display", "Customer Display & Banner Promo").await {
        return resp.into_response();
    }

    if let Err(resp) = require_permission(&pool, &headers, "display_banners:delete").await {
        return resp.into_response();
    }

    let deleted = sqlx::query("DELETE FROM customer_display_banners WHERE id = $1")
        .bind(banner_id)
        .execute(&pool)
        .await;

    match deleted {
        Ok(res) if res.rows_affected() > 0 => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "message": "Banner promosi berhasil dihapus"
            })),
        )
            .into_response(),
        Ok(_) => (
            StatusCode::NOT_FOUND,
            Json(json!({
                "success": false,
                "message": "Banner promosi tidak ditemukan"
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "success": false,
                "message": format!("Gagal menghapus banner: {}", e)
            })),
        )
            .into_response(),
    }
}

/// 7. POST /display-banners/reorder
/// Memperbarui urutan tayang sejumlah banner sekaligus.
async fn reorder_display_banners_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<ReorderBannersPayload>,
) -> impl IntoResponse {
    if let Err(resp) = require_license_feature(&pool, "customer_display", "Customer Display & Banner Promo").await {
        return resp.into_response();
    }

    if let Err(resp) = require_permission(&pool, &headers, "display_banners:update").await {
        return resp.into_response();
    }

    let mut tx = match pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "success": false, "message": format!("Transaction error: {}", e) })),
            )
                .into_response();
        }
    };

    for item in payload.items {
        let res = sqlx::query(
            "UPDATE customer_display_banners SET display_order = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2",
        )
        .bind(item.display_order)
        .bind(item.id)
        .execute(&mut *tx)
        .await;

        if let Err(e) = res {
            let _ = tx.rollback().await;
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "success": false, "message": format!("Gagal memperbarui urutan banner: {}", e) })),
            )
                .into_response();
        }
    }

    if let Err(e) = tx.commit().await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": format!("Commit error: {}", e) })),
        )
            .into_response();
    }

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "message": "Urutan banner promosi berhasil disimpan"
        })),
    )
        .into_response()
}

