use crate::services::accounting_service::{
    get_income_statement, list_chart_of_accounts, list_journal_entries,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use chrono::{Local, NaiveDate};
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::json;
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub fn report_routes() -> Router<PgPool> {
    Router::new()
        .route("/reports/sales-summary", get(sales_summary_handler))
        .route("/reports/income-statement", get(income_statement_handler))
        .route("/reports/shift-z-report/{shift_id}", get(shift_z_report_handler))
        .route("/accounting/chart-of-accounts", get(chart_of_accounts_handler))
        .route("/accounting/journal-entries", get(journal_entries_handler))
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

async fn income_statement_handler(
    State(pool): State<PgPool>,
    Query(query): Query<DateRangeQuery>,
) -> impl IntoResponse {
    let today = Local::now().date_naive();
    let start = query.start_date.unwrap_or(today);
    let end = query.end_date.unwrap_or(today);

    match get_income_statement(&pool, start, end).await {
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
        Err(_) => (0, Decimal::ZERO, Decimal::ZERO, Decimal::ZERO, Decimal::ZERO),
    };

    let payments = sqlx::query(
        "SELECT pm.name as method_name, pm.category, COALESCE(SUM(sp.amount), 0) as total_amount
        FROM sale_payments sp
        JOIN sales s ON sp.sale_id = s.id
        JOIN payment_methods pm ON sp.payment_method_id = pm.id
        WHERE s.shift_id = $1 AND s.status = 'COMPLETED'
        GROUP BY pm.name, pm.category",
    )
    .bind(shift_id)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    let pay_list: Vec<serde_json::Value> = payments
        .iter()
        .map(|p| {
            json!({
                "method_name": p.get::<String, _>("method_name"),
                "category": p.get::<String, _>("category"),
                "total_amount": p.get::<Decimal, _>("total_amount"),
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

#[derive(Deserialize)]
struct JournalQuery {
    limit: Option<i64>,
}

async fn journal_entries_handler(
    State(pool): State<PgPool>,
    Query(query): Query<JournalQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(50);
    match list_journal_entries(&pool, limit).await {
        Ok(entries) => (StatusCode::OK, Json(json!({ "success": true, "data": entries }))).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "success": false, "message": e }))).into_response(),
    }
}
