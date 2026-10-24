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

use crate::models::expense::{
    CancelExpenseRequest, CreateExpenseCategoryRequest, CreateExpenseRequest,
    ExpenseFilterQuery, UpdateExpenseCategoryRequest,
};
use crate::services::expense_service::{
    cancel_expense, create_expense, create_expense_category, get_expense_by_id,
    get_expense_summary, list_expense_categories, list_expenses, update_expense_category,
};
use crate::utils::jwt::require_any_permission;

pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/expenses", get(list_expenses_handler).post(create_expense_handler))
        .route("/expenses/summary", get(get_expense_summary_handler))
        .route("/expenses/accounts", get(get_expense_accounts_handler))
        .route("/expenses/categories", get(list_expense_categories_handler).post(create_expense_category_handler))
        .route("/expenses/categories/{id}", put(update_expense_category_handler))
        .route("/expenses/{id}", get(get_expense_handler))
        .route("/expenses/{id}/cancel", post(cancel_expense_handler))
}

async fn list_expenses_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(query): Query<ExpenseFilterQuery>,
) -> impl IntoResponse {
    let claims = match require_any_permission(&pool, &headers, &["expenses:read", "accounting:read", "master:read"]).await {
        Ok(c) => c,
        Err(resp) => return resp.into_response(),
    };

    let has_all_ownership = crate::utils::jwt::check_permission(&pool, &claims, "expenses:all_ownership").await;
    let owner_id = if has_all_ownership { None } else { Some(claims.sub) };

    let page = query.page.unwrap_or(1);
    let limit = query.limit.unwrap_or(20);

    match list_expenses(&pool, query, owner_id).await {
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

async fn get_expense_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let claims = match require_any_permission(&pool, &headers, &["expenses:read", "accounting:read", "master:read"]).await {
        Ok(c) => c,
        Err(resp) => return resp.into_response(),
    };

    let has_all_ownership = crate::utils::jwt::check_permission(&pool, &claims, "expenses:all_ownership").await;
    let owner_id = if has_all_ownership { None } else { Some(claims.sub) };

    match get_expense_by_id(&pool, id, owner_id).await {
        Ok(expense) => (
            StatusCode::OK,
            Json(json!({ "success": true, "data": expense })),
        )
            .into_response(),
        Err(e) => {
            let status = if e.contains("Akses ditolak") {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::NOT_FOUND
            };
            (
                status,
                Json(json!({ "success": false, "message": e })),
            )
                .into_response()
        }
    }
}

async fn create_expense_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<CreateExpenseRequest>,
) -> impl IntoResponse {
    let claims = match require_any_permission(&pool, &headers, &["expenses:create", "pos:checkout", "accounting:write"]).await {
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

    let user_id = Some(claims.sub);

    match create_expense(&pool, payload, user_id).await {
        Ok(expense) => (
            StatusCode::CREATED,
            Json(json!({
                "success": true,
                "message": format!("Transaksi pengeluaran #{} berhasil dicatat.", expense.transaction_no),
                "data": expense,
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

async fn cancel_expense_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<CancelExpenseRequest>,
) -> impl IntoResponse {
    let claims = match require_any_permission(&pool, &headers, &["expenses:delete", "accounting:write"]).await {
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

    let user_id = Some(claims.sub);

    match cancel_expense(&pool, id, payload, user_id).await {
        Ok(expense) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "message": format!("Transaksi pengeluaran #{} berhasil dibatalkan.", expense.transaction_no),
                "data": expense,
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
    pub outlet_id: Option<Uuid>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
}

async fn get_expense_summary_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(query): Query<SummaryQuery>,
) -> impl IntoResponse {
    let claims = match require_any_permission(&pool, &headers, &["expenses:read", "accounting:read", "report:view"]).await {
        Ok(c) => c,
        Err(resp) => return resp.into_response(),
    };

    let has_all_ownership = crate::utils::jwt::check_permission(&pool, &claims, "expenses:all_ownership").await;
    let owner_id = if has_all_ownership { None } else { Some(claims.sub) };

    match get_expense_summary(&pool, query.outlet_id, query.start_date, query.end_date, owner_id).await {
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

#[derive(serde::Deserialize)]
struct ActiveOnlyQuery {
    pub active_only: Option<bool>,
}

async fn list_expense_categories_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(query): Query<ActiveOnlyQuery>,
) -> impl IntoResponse {
    if let Err(resp) = require_any_permission(&pool, &headers, &["expenses:read", "master:read", "pos:checkout"]).await {
        return resp.into_response();
    }

    let active_only = query.active_only.unwrap_or(false);

    match list_expense_categories(&pool, active_only).await {
        Ok(cats) => (
            StatusCode::OK,
            Json(json!({ "success": true, "data": cats })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": e })),
        )
            .into_response(),
    }
}

async fn create_expense_category_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<CreateExpenseCategoryRequest>,
) -> impl IntoResponse {
    if let Err(resp) = require_any_permission(&pool, &headers, &["expenses:categories", "master:write"]).await {
        return resp.into_response();
    }

    if let Err(e) = payload.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": format!("Validasi gagal: {}", e) })),
        )
            .into_response();
    }

    match create_expense_category(&pool, payload).await {
        Ok(cat) => (
            StatusCode::CREATED,
            Json(json!({ "success": true, "data": cat, "message": "Kategori pengeluaran berhasil dibuat." })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": e })),
        )
            .into_response(),
    }
}

async fn update_expense_category_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateExpenseCategoryRequest>,
) -> impl IntoResponse {
    if let Err(resp) = require_any_permission(&pool, &headers, &["expenses:categories", "master:write"]).await {
        return resp.into_response();
    }

    if let Err(e) = payload.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": format!("Validasi gagal: {}", e) })),
        )
            .into_response();
    }

    match update_expense_category(&pool, id, payload).await {
        Ok(cat) => (
            StatusCode::OK,
            Json(json!({ "success": true, "data": cat, "message": "Kategori pengeluaran berhasil diperbarui." })),
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
struct AccountOptionQuery {
    pub outlet_id: Option<Uuid>,
}

async fn get_expense_accounts_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(_query): Query<AccountOptionQuery>,
) -> impl IntoResponse {
    if let Err(resp) = require_any_permission(&pool, &headers, &["expenses:read", "expenses:create", "accounting:read"]).await {
        return resp.into_response();
    }

    // 1. Akun Pembayaran (ASSET / Kas & Bank - Category 1)
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

    // 2. Akun Beban (EXPENSE - Category 5 atau Prive Pemilik 3103)
    let expense_accounts_res = sqlx::query(
        "SELECT coa.id, coa.code, coa.name
         FROM chart_of_accounts coa
         JOIN account_categories ac ON coa.account_category_id = ac.id
         WHERE (ac.name = 'EXPENSE' OR coa.code = '3103') AND coa.is_active = TRUE AND coa.is_header = FALSE
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

    let expense_accounts: Vec<serde_json::Value> = match expense_accounts_res {
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
                Json(json!({ "success": false, "message": format!("Gagal memuat akun beban: {}", e) })),
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
                "expense_accounts": expense_accounts,
            }
        })),
    )
        .into_response()
}
