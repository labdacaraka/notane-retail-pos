use crate::models::accounting::{
    CreateClosingRequest, ManualJournalRequest, ReopenClosingRequest,
    CreateCoaRequest, UpdateCoaRequest, JournalQueryFilter,
    FundTransferRequest, CreatePeriodRequest,
    ReopenPeriodRequest, HardCloseRequest,
    ClosingQueryFilter, PeriodQueryFilter,
};
use crate::services::accounting_service::{
    create_manual_journal_entry, get_balance_sheet, get_income_statement,
    list_accounting_closings, list_chart_of_accounts, list_journal_entries,
    perform_accounting_closing, reopen_accounting_closing,
    create_chart_of_account, update_chart_of_account, create_fund_transfer,
    get_account_balance, list_accounting_periods, create_accounting_period,
    soft_close_accounting_period, reopen_accounting_period, perform_hard_close,
};
use crate::utils::jwt::{check_permission, extract_claims, require_any_permission, require_permission};
use axum::{
    extract::{Path, Query, Request, State},
    http::{HeaderMap, Method, StatusCode},
    middleware::{from_fn_with_state, Next},
    response::{IntoResponse, Response},
    routing::{get, post, put},
    Json, Router,
};
use chrono::{Local, NaiveDate};
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

async fn reports_auth_middleware(
    State(pool): State<PgPool>,
    req: Request,
    next: Next,
) -> Result<Response, (StatusCode, Json<serde_json::Value>)> {
    let method = req.method();
    if method == Method::OPTIONS {
        return Ok(next.run(req).await);
    }

    let path = req.uri().path();
    let is_write = method == Method::POST || method == Method::PUT || method == Method::DELETE;

    if path.contains("/accounting") {
        crate::utils::jwt::require_license_feature(&pool, "accounting", "Akuntansi & Pembukuan").await?;
    }

    if path.contains("/accounting/periods") {
        if path.contains("/reopen") {
            require_any_permission(&pool, req.headers(), &["accounting:periods:reopen", "accounting:write"]).await?;
        } else if path.contains("/close") || is_write {
            require_any_permission(&pool, req.headers(), &["accounting:periods:close", "accounting:write"]).await?;
        } else {
            require_any_permission(&pool, req.headers(), &["accounting:periods:read", "accounting:read"]).await?;
        }
    } else if path.contains("/accounting/hard-close") {
        require_any_permission(&pool, req.headers(), &["accounting:hard_close", "accounting:write"]).await?;
    } else if path.contains("/accounting/closings") {
        if path.contains("/reopen") {
            require_any_permission(&pool, req.headers(), &["accounting:reopen_period", "accounting:write"]).await?;
        } else if is_write {
            require_any_permission(&pool, req.headers(), &["accounting:close_period", "accounting:write"]).await?;
        } else {
            require_permission(&pool, req.headers(), "accounting:read").await?;
        }
    } else if path.contains("/accounting/fund-transfers") {
        if is_write {
            require_any_permission(&pool, req.headers(), &["accounting:fund_transfer", "accounting:create", "accounting:write"]).await?;
        } else {
            require_permission(&pool, req.headers(), "accounting:read").await?;
        }
    } else if path.contains("/accounting/") {
        if is_write {
            require_any_permission(&pool, req.headers(), &["accounting:create", "accounting:update", "accounting:write"]).await?;
        } else {
            require_permission(&pool, req.headers(), "accounting:read").await?;
        }
    } else {
        // /reports/*
        require_any_permission(&pool, req.headers(), &["report:read", "report:view"]).await?;
    }

    Ok(next.run(req).await)
}

pub fn report_routes(pool: PgPool) -> Router<PgPool> {
    Router::new()
        .route("/reports/sales-summary", get(sales_summary_handler))
        .route("/reports/income-statement", get(income_statement_handler))
        .route("/reports/balance-sheet", get(balance_sheet_handler))
        .route("/reports/shift-z-report/{shift_id}", get(shift_z_report_handler))
        .route("/accounting/chart-of-accounts", get(chart_of_accounts_handler).post(create_coa_handler))
        .route("/accounting/chart-of-accounts/{id}", put(update_coa_handler))
        .route(
            "/accounting/journal-entries",
            get(journal_entries_handler).post(create_manual_journal_handler),
        )
        .route(
            "/accounting/fund-transfers",
            post(create_fund_transfer_handler),
        )
        .route(
            "/accounting/accounts/{id}/balance",
            get(get_account_balance_handler),
        )
        .route(
            "/accounting/closings",
            get(list_closings_handler).post(create_closing_handler),
        )
        .route(
            "/accounting/closings/{id}/reopen",
            post(reopen_closing_handler),
        )
        .route(
            "/accounting/periods",
            get(list_periods_handler).post(create_period_handler),
        )
        .route(
            "/accounting/periods/{id}/close",
            post(soft_close_period_handler),
        )
        .route(
            "/accounting/periods/{id}/reopen",
            post(reopen_period_handler),
        )
        .route(
            "/accounting/hard-close",
            post(hard_close_handler),
        )
        .layer(from_fn_with_state(pool, reports_auth_middleware))
}

fn get_user_id(headers: &HeaderMap) -> Option<Uuid> {
    extract_claims(headers).map(|c| c.sub).ok()
}

#[derive(Deserialize)]
struct DateRangeQuery {
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
}

async fn sales_summary_handler(
    State(pool): State<PgPool>,
    Query(query): Query<DateRangeQuery>,
) -> impl IntoResponse {
    let today = Local::now().date_naive();
    let start = query.start_date.unwrap_or(today);
    let end = query.end_date.unwrap_or(today);

    let summary = sqlx::query(
        "SELECT 
            COUNT(*)::bigint as transaction_count,
            COALESCE(SUM(subtotal), 0) as gross_sales,
            COALESCE(SUM(discount_amount), 0) as total_discounts,
            COALESCE(SUM(tax_amount), 0) as total_tax,
            COALESCE(SUM(total_amount), 0) as net_sales,
            COALESCE(SUM(total_cost_amount), 0) as total_cogs
        FROM sales 
        WHERE status = 'COMPLETED' 
          AND DATE(created_at) >= $1 
          AND DATE(created_at) <= $2",
    )
    .bind(start)
    .bind(end)
    .fetch_one(&pool)
    .await;

    match summary {
        Ok(s) => {
            let tx_count: i64 = s.get("transaction_count");
            let gross: Decimal = s.get("gross_sales");
            let disc: Decimal = s.get("total_discounts");
            let tax: Decimal = s.get("total_tax");
            let net: Decimal = s.get("net_sales");
            let cogs: Decimal = s.get("total_cogs");
            let profit = net - cogs;

            (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "data": {
                        "start_date": start,
                        "end_date": end,
                        "transaction_count": tx_count,
                        "gross_sales": gross,
                        "total_discounts": disc,
                        "total_tax": tax,
                        "net_sales": net,
                        "total_cogs": cogs,
                        "gross_profit": profit,
                    }
                })),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
struct IncomeStatementQuery {
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
    outlet_id: Option<Uuid>,
}

async fn income_statement_handler(
    State(pool): State<PgPool>,
    Query(query): Query<IncomeStatementQuery>,
) -> impl IntoResponse {
    let today = Local::now().date_naive();
    let start = query.start_date.unwrap_or(today);
    let end = query.end_date.unwrap_or(today);

    match get_income_statement(&pool, start, end, query.outlet_id).await {
        Ok(report) => (StatusCode::OK, Json(json!({ "success": true, "data": report }))).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
struct BalanceSheetQuery {
    as_of_date: Option<NaiveDate>,
    outlet_id: Option<Uuid>,
}

async fn balance_sheet_handler(
    State(pool): State<PgPool>,
    Query(query): Query<BalanceSheetQuery>,
) -> impl IntoResponse {
    let as_of = query.as_of_date.unwrap_or_else(|| Local::now().date_naive());

    match get_balance_sheet(&pool, as_of, query.outlet_id).await {
        Ok(report) => (StatusCode::OK, Json(json!({ "success": true, "data": report }))).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": err })),
        )
            .into_response(),
    }
}

async fn shift_z_report_handler(
    State(pool): State<PgPool>,
    Path(shift_id): Path<Uuid>,
) -> impl IntoResponse {
    let shift_opt = sqlx::query(
        "SELECT cs.*, u.full_name as cashier_name 
         FROM cashier_shifts cs 
         JOIN users u ON cs.user_id = u.id 
         WHERE cs.id = $1",
    )
    .bind(shift_id)
    .fetch_optional(&pool)
    .await;

    let shift = match shift_opt {
        Ok(Some(s)) => s,
        Ok(None) => return (StatusCode::NOT_FOUND, Json(json!({ "success": false, "message": "Shift tidak ditemukan" }))).into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    };

    let sales_stats = sqlx::query(
        "SELECT 
            COUNT(*)::bigint as total_count,
            COALESCE(SUM(subtotal), 0) as gross_sales,
            COALESCE(SUM(discount_amount), 0) as discounts,
            COALESCE(SUM(tax_amount), 0) as tax,
            COALESCE(SUM(total_amount), 0) as net_sales
        FROM sales 
        WHERE shift_id = $1 AND status = 'COMPLETED'",
    )
    .bind(shift_id)
    .fetch_one(&pool)
    .await;

    let (tot_count, gross, disc, tax, net) = match sales_stats {
        Ok(st) => (
            st.get::<i64, _>("total_count"),
            st.get::<Decimal, _>("gross_sales"),
            st.get::<Decimal, _>("discounts"),
            st.get::<Decimal, _>("tax"),
            st.get::<Decimal, _>("net_sales"),
        ),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e.to_string() }))).into_response(),
    };

    let payment_rows = sqlx::query(
        "SELECT 
            payment_method, 
            COUNT(*)::bigint as count,
            COALESCE(SUM(amount), 0) as total_amount 
        FROM sale_payments sp
        JOIN sales s ON sp.sale_id = s.id
        WHERE s.shift_id = $1 AND s.status = 'COMPLETED'
        GROUP BY payment_method",
    )
    .bind(shift_id)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    let pay_list: Vec<serde_json::Value> = payment_rows
        .into_iter()
        .map(|r| {
            json!({
                "method": r.get::<String, _>("payment_method"),
                "count": r.get::<i64, _>("count"),
                "total": r.get::<Decimal, _>("total_amount"),
            })
        })
        .collect();

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "data": {
                "shift_id": shift.get::<Uuid, _>("id"),
                "terminal_id": shift.get::<String, _>("terminal_id"),
                "cashier_name": shift.get::<String, _>("cashier_name"),
                "opened_at": shift.get::<chrono::DateTime<chrono::Utc>, _>("opened_at"),
                "closed_at": shift.get::<Option<chrono::DateTime<chrono::Utc>>, _>("closed_at"),
                "opening_cash": shift.get::<Decimal, _>("opening_cash"),
                "expected_cash": shift.get::<Option<Decimal>, _>("expected_cash"),
                "actual_cash": shift.get::<Option<Decimal>, _>("actual_cash"),
                "discrepancy": shift.get::<Option<Decimal>, _>("discrepancy"),
                "status": shift.get::<String, _>("status"),
                "sales_summary": {
                    "total_transactions": tot_count,
                    "gross_sales": gross,
                    "discounts": disc,
                    "tax": tax,
                    "net_sales": net,
                },
                "payment_breakdown": pay_list
            }
        })),
    )
        .into_response()
}

async fn chart_of_accounts_handler(State(pool): State<PgPool>) -> impl IntoResponse {
    match list_chart_of_accounts(&pool).await {
        Ok(accounts) => (StatusCode::OK, Json(json!({ "success": true, "data": accounts }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e }))).into_response(),
    }
}

async fn create_coa_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<CreateCoaRequest>,
) -> impl IntoResponse {
    if let Err(resp) = require_any_permission(
        &pool,
        &headers,
        &["accounting:write", "master:write"],
    )
    .await
    {
        return resp.into_response();
    }

    match create_chart_of_account(&pool, payload).await {
        Ok(coa) => (
            StatusCode::CREATED,
            Json(json!({ "success": true, "message": "Akun berhasil ditambahkan.", "data": coa })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": e })),
        )
            .into_response(),
    }
}

async fn update_coa_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateCoaRequest>,
) -> impl IntoResponse {
    if let Err(resp) = require_any_permission(
        &pool,
        &headers,
        &["accounting:write", "master:write"],
    )
    .await
    {
        return resp.into_response();
    }

    match update_chart_of_account(&pool, id, payload).await {
        Ok(coa) => (
            StatusCode::OK,
            Json(json!({ "success": true, "message": "Akun berhasil diperbarui.", "data": coa })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": e })),
        )
            .into_response(),
    }
}


async fn journal_entries_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(query): Query<JournalQueryFilter>,
) -> impl IntoResponse {
    let claims = match require_any_permission(&pool, &headers, &["accounting:read", "report:view"]).await {
        Ok(c) => c,
        Err(resp) => return resp.into_response(),
    };

    let has_all_ownership = check_permission(&pool, &claims, "accounting:all_ownership").await;
    let owner_id = if has_all_ownership { None } else { Some(claims.sub) };

    match list_journal_entries(&pool, query, owner_id).await {
        Ok(res) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "data": res.data,
                "total_items": res.total_items,
                "total_pages": res.total_pages,
                "page": res.page,
                "page_size": res.page_size
            }))
        ).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e }))).into_response(),
    }
}

async fn create_manual_journal_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<ManualJournalRequest>,
) -> impl IntoResponse {
    let user_id = get_user_id(&headers);

    if payload.lines.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "Baris entri jurnal manual tidak boleh kosong" })),
        )
            .into_response();
    }

    match create_manual_journal_entry(&pool, payload, user_id).await {
        Ok(detail) => (
            StatusCode::CREATED,
            Json(json!({ "success": true, "message": "Jurnal penyesuaian berhasil dibukukan", "data": detail })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": e })),
        )
            .into_response(),
    }
}

async fn list_closings_handler(
    State(pool): State<PgPool>,
    Query(query): Query<ClosingQueryFilter>,
) -> impl IntoResponse {
    match list_accounting_closings(&pool, query).await {
        Ok(res) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "data": res.data,
                "total_items": res.total_items,
                "total_pages": res.total_pages,
                "page": res.page,
                "page_size": res.page_size
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": e })),
        )
            .into_response(),
    }
}

async fn create_closing_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<CreateClosingRequest>,
) -> impl IntoResponse {
    let user_id = get_user_id(&headers);
    match perform_accounting_closing(&pool, payload, user_id).await {
        Ok(closing) => (
            StatusCode::CREATED,
            Json(json!({ "success": true, "message": "Tutup buku periode berhasil dilaksanakan dan jurnal penutup telah dibukukan.", "data": closing })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": e })),
        )
            .into_response(),
    }
}

async fn reopen_closing_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<ReopenClosingRequest>,
) -> impl IntoResponse {
    let user_id = get_user_id(&headers);
    match reopen_accounting_closing(&pool, id, payload, user_id).await {
        Ok(closing) => (
            StatusCode::OK,
            Json(json!({ "success": true, "message": "Buku periode berhasil dibuka kembali (Reopened). Transaksi penyesuaian dapat dilakukan.", "data": closing })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": e })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct AccountBalanceQuery {
    pub outlet_id: Option<Uuid>,
}

async fn get_account_balance_handler(
    State(pool): State<PgPool>,
    Path(account_id): Path<Uuid>,
    Query(query): Query<AccountBalanceQuery>,
) -> impl IntoResponse {
    match get_account_balance(&pool, account_id, query.outlet_id).await {
        Ok(balance) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "data": {
                    "account_id": account_id,
                    "outlet_id": query.outlet_id,
                    "balance": balance,
                }
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

async fn create_fund_transfer_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<FundTransferRequest>,
) -> impl IntoResponse {
    let claims = match extract_claims(&headers) {
        Ok(c) => c,
        Err(resp) => return resp.into_response(),
    };

    if claims.role_name != "SUPERADMIN" {
        let has_all_ownership = check_permission(&pool, &claims, "accounting:all_ownership").await;
        if !has_all_ownership {
            let has_outlet_access: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM user_outlets WHERE user_id = $1 AND outlet_id = $2)"
            )
            .bind(claims.sub)
            .bind(payload.from_outlet_id)
            .fetch_one(&pool)
            .await
            .unwrap_or(false);

            if !has_outlet_access {
                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({
                        "success": false,
                        "message": "Akses ditolak: Anda tidak memiliki hak akses pada cabang pengirim tersebut."
                    })),
                ).into_response();
            }
        }
    }

    match create_fund_transfer(&pool, payload, Some(claims.sub)).await {
        Ok(res) => (
            StatusCode::CREATED,
            Json(json!({ "success": true, "message": "Transfer dana antar cabang berhasil dibukukan.", "data": res })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": e })),
        )
            .into_response(),
    }
}

async fn list_periods_handler(
    State(pool): State<PgPool>,
    Query(query): Query<PeriodQueryFilter>,
) -> impl IntoResponse {
    match list_accounting_periods(&pool, query).await {
        Ok(res) => (
            StatusCode::OK,
            Json(json!({
                "success": true,
                "data": res.data,
                "total_items": res.total_items,
                "total_pages": res.total_pages,
                "page": res.page,
                "page_size": res.page_size
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": e })),
        )
            .into_response(),
    }
}

async fn create_period_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<CreatePeriodRequest>,
) -> impl IntoResponse {
    let user_id = get_user_id(&headers);
    match create_accounting_period(&pool, payload, user_id).await {
        Ok(period) => (
            StatusCode::CREATED,
            Json(json!({ "success": true, "message": "Periode akuntansi berhasil dibuat.", "data": period })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": e })),
        )
            .into_response(),
    }
}

async fn soft_close_period_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let user_id = get_user_id(&headers);
    match soft_close_accounting_period(&pool, id, user_id).await {
        Ok(period) => (
            StatusCode::OK,
            Json(json!({ "success": true, "message": "Periode akuntansi berhasil dikunci (Soft Close). Transaksi dalam periode ini sekarang diproteksi.", "data": period })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": e })),
        )
            .into_response(),
    }
}

async fn reopen_period_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(payload): Json<ReopenPeriodRequest>,
) -> impl IntoResponse {
    let user_id = get_user_id(&headers);
    match reopen_accounting_period(&pool, id, payload, user_id).await {
        Ok(period) => (
            StatusCode::OK,
            Json(json!({ "success": true, "message": "Periode akuntansi berhasil dibuka kembali (Reopened).", "data": period })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": e })),
        )
            .into_response(),
    }
}

async fn hard_close_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<HardCloseRequest>,
) -> impl IntoResponse {
    let user_id = get_user_id(&headers);
    match perform_hard_close(&pool, payload, user_id).await {
        Ok(detail) => (
            StatusCode::CREATED,
            Json(json!({
                "success": true,
                "message": "Tutup buku akhir tahun (Hard Close) berhasil dibukukan ke Laba Ditahan.",
                "data": detail
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

