use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use chrono::NaiveDate;
use serde_json::json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::models::other_income::{
    CancelOtherIncomeRequest, CreateOtherIncomeCategoryRequest, CreateOtherIncomeRequest,
    OtherIncomeCategoryFilterQuery, OtherIncomeFilterQuery, UpdateOtherIncomeCategoryRequest,
};
use crate::services::other_income_service::{
    cancel_other_income, create_other_income, create_other_income_category,
    get_other_income_by_id, get_other_income_summary, list_other_income_categories,
    list_other_incomes, update_other_income_category,
};
use crate::utils::jwt::{require_any_permission, require_license_feature};

pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/other-income", get(list_other_incomes_handler).post(create_other_income_handler))
        .route("/other-income/summary", get(get_other_income_summary_handler))
        .route("/other-income/accounts", get(get_other_income_accounts_handler))
        .route("/other-income/payment-accounts", get(get_payment_accounts_handler))
        .route("/other-income/income-accounts", get(get_income_accounts_handler))
        .route("/other-income/categories", get(list_other_income_categories_handler).post(create_other_income_category_handler))
        .route("/other-income/categories/{id}", put(update_other_income_category_handler))
        .route("/other-income/{id}", get(get_other_income_handler))
        .route("/other-income/{id}/cancel", post(cancel_other_income_handler))
}

async fn list_other_incomes_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(query): Query<OtherIncomeFilterQuery>,
) -> impl IntoResponse {
    if let Err(resp) = require_license_feature(&pool, "other_income", "Pendapatan Lain-lain").await {
        return resp.into_response();
    }

    let claims = match require_any_permission(&pool, &headers, &["other_income:read", "accounting:read", "master:read"]).await {
        Ok(c) => c,
        Err(resp) => return resp.into_response(),
    };

    let has_all_ownership = crate::utils::jwt::check_permission(&pool, &claims, "other_income:all_ownership").await
        || crate::utils::jwt::check_permission(&pool, &claims, "sales:all_ownership").await;
    let owner_id = if has_all_ownership { None } else { Some(claims.sub) };

    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(20);

    match list_other_incomes(&pool, query, owner_id).await {
        Ok((items, total)) => {
            let total_pages = if total == 0 { 0 } else { (total + limit - 1) / limit };
            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "data": items,
                    "total": total,
                    "page": page,
                    "limit": limit,
                    "total_pages": total_pages,
                })),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": e })),
        )
            .into_response(),
    }
}

async fn get_other_income_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    if let Err(resp) = require_license_feature(&pool, "other_income", "Pendapatan Lain-lain").await {
        return resp.into_response();
    }

    let claims = match require_any_permission(&pool, &headers, &["other_income:read", "accounting:read", "master:read"]).await {
        Ok(c) => c,
        Err(resp) => return resp.into_response(),
    };

    let has_all_ownership = crate::utils::jwt::check_permission(&pool, &claims, "other_income:all_ownership").await
        || crate::utils::jwt::check_permission(&pool, &claims, "sales:all_ownership").await;
    let owner_id = if has_all_ownership { None } else { Some(claims.sub) };

    match get_other_income_by_id(&pool, id, owner_id).await {
        Ok(income) => (
            StatusCode::OK,
            Json(json!({ "success": true, "data": income })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "success": false, "message": e })),
        )
            .into_response(),
    }
}

async fn create_other_income_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<CreateOtherIncomeRequest>,
) -> impl IntoResponse {
    if let Err(resp) = require_license_feature(&pool, "other_income", "Pendapatan Lain-lain").await {
        return resp.into_response();
    }

    let claims = match require_any_permission(&pool, &headers, &["other_income:create"]).await {
        Ok(c) => c,
        Err(resp) => return resp.into_response(),
    };

    if let Err(e) = payload.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": format!("Validasi gagal: {}", e) })),
        )
            .into_response();
    }

    match create_other_income(&pool, payload, Some(claims.sub)).await {
        Ok(detail) => (
            StatusCode::CREATED,
            Json(json!({
                "success": true,
                "data": detail,
                "message": "Penerimaan pendapatan lain-lain berhasil dicatat."
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": e })),
        )
            .into_response(),
    }
}

async fn cancel_other_income_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<CancelOtherIncomeRequest>,
) -> impl IntoResponse {
    if let Err(resp) = require_license_feature(&pool, "other_income", "Pendapatan Lain-lain").await {
        return resp.into_response();
    }

    let claims = match require_any_permission(&pool, &headers, &["other_income:delete"]).await {
        Ok(c) => c,
        Err(resp) => return resp.into_response(),
    };

    if let Err(e) = payload.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": format!("Validasi gagal: {}", e) })),
        )
            .into_response();
    }

    match cancel_other_income(&pool, id, payload, Some(claims.sub)).await {
        Ok(detail) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "data": detail,
                "message": "Transaksi pendapatan lain-lain berhasil dibatalkan dan jurnal pembalik telah dibukukan."
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": e })),
        )
            .into_response(),
    }
}

#[derive(serde::Deserialize)]
struct SummaryQuery {
    outlet_id: Option<Uuid>,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
}

async fn get_other_income_summary_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(query): Query<SummaryQuery>,
) -> impl IntoResponse {
    if let Err(resp) = require_license_feature(&pool, "other_income", "Pendapatan Lain-lain").await {
        return resp.into_response();
    }

    if let Err(resp) = require_any_permission(&pool, &headers, &["other_income:read", "accounting:read"]).await {
        return resp.into_response();
    }

    match get_other_income_summary(&pool, query.outlet_id, query.start_date, query.end_date).await {
        Ok(summary) => (
            StatusCode::OK,
            Json(json!({ "success": true, "data": summary })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": e })),
        )
            .into_response(),
    }
}

async fn list_other_income_categories_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(query): Query<OtherIncomeCategoryFilterQuery>,
) -> impl IntoResponse {
    if let Err(resp) = require_license_feature(&pool, "other_income", "Pendapatan Lain-lain").await {
        return resp.into_response();
    }

    if let Err(resp) = require_any_permission(&pool, &headers, &["other_income:read", "other_income:create", "other_income:categories"]).await {
        return resp.into_response();
    }

    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).max(1);

    match list_other_income_categories(&pool, &query).await {
        Ok((cats, total)) => {
            let total_pages = if total == 0 {
                1
            } else {
                ((total as f64) / (limit as f64)).ceil() as i64
            };
            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "data": cats,
                    "total": total,
                    "total_items": total,
                    "page": page,
                    "limit": limit,
                    "page_size": limit,
                    "total_pages": total_pages,
                })),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": e })),
        )
            .into_response(),
    }
}

async fn create_other_income_category_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<CreateOtherIncomeCategoryRequest>,
) -> impl IntoResponse {
    if let Err(resp) = require_license_feature(&pool, "other_income", "Pendapatan Lain-lain").await {
        return resp.into_response();
    }

    if let Err(resp) = require_any_permission(&pool, &headers, &["other_income:categories"]).await {
        return resp.into_response();
    }

    if let Err(e) = payload.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": format!("Validasi gagal: {}", e) })),
        )
            .into_response();
    }

    match create_other_income_category(&pool, payload).await {
        Ok(cat) => (
            StatusCode::CREATED,
            Json(json!({ "success": true, "data": cat, "message": "Kategori pendapatan berhasil dibuat." })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": e })),
        )
            .into_response(),
    }
}

async fn update_other_income_category_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateOtherIncomeCategoryRequest>,
) -> impl IntoResponse {
    if let Err(resp) = require_license_feature(&pool, "other_income", "Pendapatan Lain-lain").await {
        return resp.into_response();
    }

    if let Err(resp) = require_any_permission(&pool, &headers, &["other_income:categories"]).await {
        return resp.into_response();
    }

    if let Err(e) = payload.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": format!("Validasi gagal: {}", e) })),
        )
            .into_response();
    }

    match update_other_income_category(&pool, id, payload).await {
        Ok(cat) => (
            StatusCode::OK,
            Json(json!({ "success": true, "data": cat, "message": "Kategori pendapatan berhasil diperbarui." })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": e })),
        )
            .into_response(),
    }
}

async fn get_other_income_accounts_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(resp) = require_license_feature(&pool, "other_income", "Pendapatan Lain-lain").await {
        return resp.into_response();
    }

    if let Err(resp) = require_any_permission(&pool, &headers, &["other_income:read", "other_income:create", "accounting:read"]).await {
        return resp.into_response();
    }

    // 1. Akun Penerimaan Kas/Bank (ASSET / Kas & Bank - Category 1)
    let payment_accounts_res = sqlx::query(
        "SELECT coa.id, coa.code, coa.name
         FROM chart_of_accounts coa
         JOIN account_categories ac ON coa.account_category_id = ac.id
         WHERE ac.name = 'ASSET' AND coa.is_active = TRUE AND coa.is_header = FALSE
           AND (coa.code LIKE '11%' OR coa.code LIKE '12%')
         ORDER BY coa.code ASC",
    )
    .fetch_all(&pool)
    .await;

    // 2. Akun Pendapatan (REVENUE - Category 4)
    let income_accounts_res = sqlx::query(
        "SELECT coa.id, coa.code, coa.name
         FROM chart_of_accounts coa
         JOIN account_categories ac ON coa.account_category_id = ac.id
         WHERE ac.name = 'REVENUE' AND coa.is_active = TRUE AND coa.is_header = FALSE
           AND coa.code NOT IN ('4102', '4103', '4104', '4105', '4106', '4107')
         ORDER BY coa.code ASC",
    )
    .fetch_all(&pool)
    .await;

    let payment_accounts: Vec<serde_json::Value> = match payment_accounts_res {
        Ok(rows) => rows
            .iter()
            .map(|r| {
                json!({
                    "id": r.get::<Uuid, _>("id"),
                    "code": r.get::<String, _>("code"),
                    "name": r.get::<String, _>("name"),
                })
            })
            .collect(),
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "success": false, "message": format!("Gagal memuat akun kas: {}", e) })),
            )
                .into_response();
        }
    };

    let income_accounts: Vec<serde_json::Value> = match income_accounts_res {
        Ok(rows) => rows
            .iter()
            .map(|r| {
                json!({
                    "id": r.get::<Uuid, _>("id"),
                    "code": r.get::<String, _>("code"),
                    "name": r.get::<String, _>("name"),
                })
            })
            .collect(),
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "success": false, "message": format!("Gagal memuat akun pendapatan: {}", e) })),
            )
                .into_response();
        }
    };

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "data": {
                "payment_accounts": payment_accounts,
                "income_accounts": income_accounts,
            }
        })),
    )
        .into_response()
}

#[derive(serde::Deserialize)]
struct AccountSearchQuery {
    pub search: Option<String>,
}

async fn get_payment_accounts_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(query): Query<AccountSearchQuery>,
) -> impl IntoResponse {
    if let Err(resp) = require_license_feature(&pool, "other_income", "Pendapatan Lain-lain").await {
        return resp.into_response();
    }

    if let Err(resp) = require_any_permission(&pool, &headers, &["other_income:read", "other_income:create", "accounting:read"]).await {
        return resp.into_response();
    }

    let search_filter = if let Some(ref q) = query.search {
        let clean = q.trim().replace('\'', "''");
        if !clean.is_empty() {
            format!(" AND (coa.name ILIKE '%{clean}%' OR coa.code ILIKE '%{clean}%')")
        } else {
            "".to_string()
        }
    } else {
        "".to_string()
    };

    let sql = format!(
        "SELECT coa.id, coa.code, coa.name
         FROM chart_of_accounts coa
         JOIN account_categories ac ON coa.account_category_id = ac.id
         WHERE ac.name = 'ASSET' AND coa.is_active = TRUE AND coa.is_header = FALSE
           AND (coa.code LIKE '11%' OR coa.code LIKE '12%')
           {}
         ORDER BY coa.code ASC",
        search_filter
    );

    match sqlx::query(&sql).fetch_all(&pool).await {
        Ok(rows) => {
            let list: Vec<serde_json::Value> = rows
                .iter()
                .map(|r| {
                    json!({
                        "id": r.get::<Uuid, _>("id"),
                        "code": r.get::<String, _>("code"),
                        "name": r.get::<String, _>("name"),
                    })
                })
                .collect();
            (StatusCode::OK, Json(json!({ "success": true, "data": list }))).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": format!("Gagal memuat akun kas: {}", e) })),
        )
            .into_response(),
    }
}

async fn get_income_accounts_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(query): Query<AccountSearchQuery>,
) -> impl IntoResponse {
    if let Err(resp) = require_license_feature(&pool, "other_income", "Pendapatan Lain-lain").await {
        return resp.into_response();
    }

    if let Err(resp) = require_any_permission(&pool, &headers, &["other_income:read", "other_income:create", "accounting:read"]).await {
        return resp.into_response();
    }

    let search_filter = if let Some(ref q) = query.search {
        let clean = q.trim().replace('\'', "''");
        if !clean.is_empty() {
            format!(" AND (coa.name ILIKE '%{clean}%' OR coa.code ILIKE '%{clean}%')")
        } else {
            "".to_string()
        }
    } else {
        "".to_string()
    };

    let sql = format!(
        "SELECT coa.id, coa.code, coa.name
         FROM chart_of_accounts coa
         JOIN account_categories ac ON coa.account_category_id = ac.id
         WHERE ac.name = 'REVENUE' AND coa.is_active = TRUE AND coa.is_header = FALSE
           AND coa.code NOT IN ('4102', '4103', '4104', '4105', '4106', '4107')
           {}
         ORDER BY coa.code ASC",
        search_filter
    );

    match sqlx::query(&sql).fetch_all(&pool).await {
        Ok(rows) => {
            let list: Vec<serde_json::Value> = rows
                .iter()
                .map(|r| {
                    json!({
                        "id": r.get::<Uuid, _>("id"),
                        "code": r.get::<String, _>("code"),
                        "name": r.get::<String, _>("name"),
                    })
                })
                .collect();
            (StatusCode::OK, Json(json!({ "success": true, "data": list }))).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": format!("Gagal memuat akun pendapatan: {}", e) })),
        )
            .into_response(),
    }
}

