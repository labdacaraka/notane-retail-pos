use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::services::auth_service::verify_supervisor_auth;
use crate::utils::jwt::{require_any_permission, require_permission};

#[derive(Debug, Deserialize)]
pub struct CreateCashDropRequest {
    pub shift_id: Uuid,
    pub amount: Decimal,
    pub supervisor_id: Option<Uuid>,
    pub supervisor_pin: Option<String>,
    pub supervisor_auth_token: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CashDropItem {
    pub id: Uuid,
    pub drop_number: String,
    pub shift_id: Uuid,
    pub outlet_id: Option<Uuid>,
    pub cashier_id: Uuid,
    pub cashier_name: Option<String>,
    pub supervisor_id: Uuid,
    pub supervisor_name: Option<String>,
    pub amount: Decimal,
    pub notes: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CashDropFilter {
    pub shift_id: Option<Uuid>,
    pub outlet_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct CashDropStatusSummary {
    pub current_drawer_cash: Decimal,
    pub max_drawer_limit: Decimal,
    pub should_drop: bool,
    pub total_dropped: Decimal,
    pub drops: Vec<CashDropItem>,
}

#[derive(Debug, sqlx::FromRow)]
struct ShiftCheckRow {
    id: Uuid,
    outlet_id: Option<Uuid>,
    opening_cash: Decimal,
    status: String,
}

#[derive(Debug, sqlx::FromRow)]
struct ActiveShiftRow {
    id: Uuid,
    opening_cash: Decimal,
    outlet_id: Option<Uuid>,
}

pub fn cash_drop_routes() -> Router<PgPool> {
    Router::new()
        .route("/pos/cash-drops", post(create_cash_drop_handler))
        .route("/pos/cash-drops", get(list_cash_drops_handler))
        .route("/pos/cash-drops/current-shift", get(get_current_shift_cash_drops_handler))
}

async fn create_cash_drop_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Json(payload): Json<CreateCashDropRequest>,
) -> impl IntoResponse {
    let claims = match require_permission(&pool, &headers, "cash_drops:create").await {
        Ok(c) => c,
        Err(resp) => return resp.into_response(),
    };

    if payload.amount <= Decimal::ZERO {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "success": false, "message": "Nominal setor kas (cash drop) harus lebih besar dari 0" })),
        )
            .into_response();
    }

    // Verifikasi otorisasi supervisor (Biometrik atau PIN)
    let supervisor_id = match verify_supervisor_auth(
        &pool,
        payload.supervisor_id,
        payload.supervisor_pin.as_deref(),
        payload.supervisor_auth_token.as_deref(),
        "CASH_DROP",
    )
    .await
    {
        Ok(spv_id) => spv_id,
        Err(e) => {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({ "success": false, "message": format!("Otorisasi Supervisor gagal: {}", e) })),
            )
                .into_response();
        }
    };

    let cashier_id = claims.sub;

    // Pastikan shift masih OPEN
    let shift_check = sqlx::query_as::<_, ShiftCheckRow>(
        "SELECT id, outlet_id, opening_cash, status FROM cashier_shifts WHERE id = $1",
    )
    .bind(payload.shift_id)
    .fetch_optional(&pool)
    .await;

    let shift = match shift_check {
        Ok(Some(s)) => {
            if s.status != "OPEN" {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "success": false, "message": "Shift kasir sudah ditutup. Tidak dapat melakukan cash drop." })),
                )
                    .into_response();
            }
            s
        }
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "success": false, "message": "Data shift kasir tidak ditemukan" })),
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

    let mut tx = match pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "success": false, "message": format!("Gagal memulai transaksi: {}", e) })),
            )
                .into_response();
        }
    };

    // Generate drop number: DROP-YYYYMMDD-XXXX
    let drop_number = format!(
        "DROP-{}-{}",
        chrono::Utc::now().format("%Y%m%d"),
        &Uuid::new_v4().to_string()[..6].to_uppercase()
    );

    let inserted = sqlx::query_as::<_, CashDropItem>(
        r#"
        INSERT INTO cash_drops (
            drop_number, shift_id, outlet_id, cashier_id, supervisor_id, amount, notes
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7
        )
        RETURNING id, drop_number, shift_id, outlet_id, cashier_id, NULL::text as cashier_name, supervisor_id, NULL::text as supervisor_name, amount, notes, created_at
        "#
    )
    .bind(&drop_number)
    .bind(shift.id)
    .bind(shift.outlet_id)
    .bind(cashier_id)
    .bind(supervisor_id)
    .bind(payload.amount)
    .bind(&payload.notes)
    .fetch_one(&mut *tx)
    .await;

    let drop_record = match inserted {
        Ok(r) => r,
        Err(e) => {
            let _ = tx.rollback().await;
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "success": false, "message": format!("Gagal menyimpan cash drop: {}", e) })),
            )
                .into_response();
        }
    };

    // Jurnal Akuntansi Kas Masuk ke Brankas / Kas Pengawas:
    // Debit: Kas Besar / Brankas (1102 jika ada atau Kas Utama 1101), Kredit: Kas Laci Kasir (1101)
    // Cek apakah akun 1102 ada
    let coa_brankas = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM chart_of_accounts WHERE code = '1102' AND is_active = true LIMIT 1"
    )
    .fetch_optional(&mut *tx)
    .await
    .unwrap_or(None);

    let coa_kas_laci = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM chart_of_accounts WHERE code = '1101' AND is_active = true LIMIT 1"
    )
    .fetch_optional(&mut *tx)
    .await
    .unwrap_or(None);

    if let (Some(debit_acc), Some(credit_acc)) = (coa_brankas, coa_kas_laci) {
        let journal_id = Uuid::new_v4();
        let journal_number = format!("JRN-CD-{}", chrono::Utc::now().format("%Y%m%d%H%M%S"));
        let journal_res = sqlx::query(
            r#"
            INSERT INTO general_journals (
                id, journal_number, reference_type, reference_id, transaction_date, description, total_amount, created_by
            ) VALUES (
                $1, $2, 'CASH_DROP', $3, CURRENT_TIMESTAMP, $4, $5, $6
            )
            "#,
        )
        .bind(journal_id)
        .bind(&journal_number)
        .bind(drop_record.id)
        .bind(format!("Setor Kas Laci (Cash Drop) {} oleh Kasir", drop_number))
        .bind(payload.amount)
        .bind(cashier_id)
        .execute(&mut *tx)
        .await;

        if journal_res.is_ok() {
            let _ = sqlx::query(
                r#"
                INSERT INTO journal_entries (
                    journal_id, coa_id, debit_amount, credit_amount, description
                ) VALUES
                ($1, $2, $3, 0, 'Setor Kas ke Brankas/Supervisor'),
                ($1, $4, 0, $3, 'Pengurangan Kas Laci Kasir')
                "#,
            )
            .bind(journal_id)
            .bind(debit_acc)
            .bind(payload.amount)
            .bind(credit_acc)
            .execute(&mut *tx)
            .await;
        }
    }

    if let Err(e) = tx.commit().await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": format!("Gagal menyelesaikan transaksi: {}", e) })),
        )
            .into_response();
    }

    let spv_name = sqlx::query_scalar::<_, String>("SELECT full_name FROM users WHERE id = $1")
        .bind(supervisor_id)
        .fetch_optional(&pool)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| "Supervisor".to_string());

    (
        StatusCode::CREATED,
        Json(json!({
            "success": true,
            "message": format!("Cash Drop sejumlah Rp {} berhasil dicatat dengan No. {}", payload.amount, drop_number),
            "data": {
                "id": drop_record.id,
                "drop_number": drop_record.drop_number,
                "amount": drop_record.amount,
                "supervisor_name": spv_name,
                "created_at": drop_record.created_at,
                "notes": drop_record.notes
            }
        })),
    )
        .into_response()
}

async fn list_cash_drops_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
    Query(filter): Query<CashDropFilter>,
) -> impl IntoResponse {
    let _claims = match require_any_permission(&pool, &headers, &["cash_drops:read", "cash_drops:view"]).await {
        Ok(c) => c,
        Err(resp) => return resp.into_response(),
    };

    let mut query = String::from(
        r#"
        SELECT 
            cd.id, cd.drop_number, cd.shift_id, cd.outlet_id,
            cd.cashier_id, u_cashier.name as cashier_name,
            cd.supervisor_id, u_spv.name as supervisor_name,
            cd.amount, cd.notes, cd.created_at
        FROM cash_drops cd
        LEFT JOIN users u_cashier ON cd.cashier_id = u_cashier.id
        LEFT JOIN users u_spv ON cd.supervisor_id = u_spv.id
        WHERE 1=1
        "#
    );

    if filter.shift_id.is_some() {
        query.push_str(" AND cd.shift_id = $1");
    }

    query.push_str(" ORDER BY cd.created_at DESC LIMIT 100");

    let rows = if let Some(sid) = filter.shift_id {
        sqlx::query_as::<_, CashDropItem>(&query)
            .bind(sid)
            .fetch_all(&pool)
            .await
    } else {
        sqlx::query_as::<_, CashDropItem>(&query)
            .fetch_all(&pool)
            .await
    };

    match rows {
        Ok(drops) => (StatusCode::OK, Json(json!({ "success": true, "data": drops }))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "success": false, "message": format!("Database error: {}", e) })),
        )
            .into_response(),
    }
}

async fn get_current_shift_cash_drops_handler(
    State(pool): State<PgPool>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let claims = match require_any_permission(&pool, &headers, &["cash_drops:read", "cash_drops:view", "pos:checkout"]).await {
        Ok(c) => c,
        Err(resp) => return resp.into_response(),
    };

    let user_id = claims.sub;

    // Cari shift aktif user
    let active_shift = match sqlx::query_as::<_, ActiveShiftRow>(
        "SELECT id, opening_cash, outlet_id FROM cashier_shifts WHERE user_id = $1 AND status = 'OPEN' ORDER BY opened_at DESC LIMIT 1",
    )
    .bind(user_id)
    .fetch_optional(&pool)
    .await
    {
        Ok(Some(s)) => s,
        Ok(None) => {
            return (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "data": CashDropStatusSummary {
                        current_drawer_cash: Decimal::ZERO,
                        max_drawer_limit: Decimal::from(2000000),
                        should_drop: false,
                        total_dropped: Decimal::ZERO,
                        drops: vec![]
                    }
                })),
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

    // Ambil receipt settings untuk max_cash_drawer_limit
    let max_drawer_limit = sqlx::query_scalar::<_, Decimal>(
        "SELECT COALESCE(max_cash_drawer_limit, 2000000) FROM receipt_settings WHERE outlet_id = $1 OR outlet_id IS NULL ORDER BY outlet_id NULLS LAST LIMIT 1"
    )
    .bind(active_shift.outlet_id)
    .fetch_optional(&pool)
    .await
    .unwrap_or(None)
    .unwrap_or(Decimal::from(2000000));

    // Hitung total penerimaan tunai dari sales
    let cash_sales = sqlx::query_scalar::<_, Decimal>(
        r#"
        SELECT COALESCE(SUM(sp.amount), 0)
        FROM sale_payments sp
        JOIN sales s ON sp.sale_id = s.id
        JOIN payment_methods pm ON sp.payment_method_id = pm.id
        WHERE s.shift_id = $1 AND s.status = 'COMPLETED' AND pm.category = 'CASH'
        "#
    )
    .bind(active_shift.id)
    .fetch_one(&pool)
    .await
    .unwrap_or(Decimal::ZERO);

    // Hitung pengeluaran operasional laci
    let cash_expenses = sqlx::query_scalar::<_, Decimal>(
        "SELECT COALESCE(SUM(amount), 0) FROM expense_transactions WHERE shift_id = $1 AND status = 'COMPLETED'"
    )
    .bind(active_shift.id)
    .fetch_one(&pool)
    .await
    .unwrap_or(Decimal::ZERO);

    // Hitung retur tunai konsumen
    let cash_returns = sqlx::query_scalar::<_, Decimal>(
        "SELECT COALESCE(SUM(refund_amount), 0) FROM sales_returns WHERE shift_id = $1 AND refund_method = 'CASH'"
    )
    .bind(active_shift.id)
    .fetch_one(&pool)
    .await
    .unwrap_or(Decimal::ZERO);

    // Hitung riwayat cash drop
    let drops = sqlx::query_as::<_, CashDropItem>(
        r#"
        SELECT 
            cd.id, cd.drop_number, cd.shift_id, cd.outlet_id,
            cd.cashier_id, u_cashier.name as cashier_name,
            cd.supervisor_id, u_spv.name as supervisor_name,
            cd.amount, cd.notes, cd.created_at
        FROM cash_drops cd
        LEFT JOIN users u_cashier ON cd.cashier_id = u_cashier.id
        LEFT JOIN users u_spv ON cd.supervisor_id = u_spv.id
        WHERE cd.shift_id = $1
        ORDER BY cd.created_at DESC
        "#
    )
    .bind(active_shift.id)
    .fetch_all(&pool)
    .await
    .unwrap_or_default();

    let total_dropped: Decimal = drops.iter().map(|d| d.amount).sum();

    // Sisa fisik kas tunai di laci = opening + cash_sales - cash_expenses - cash_returns - total_dropped
    let current_drawer_cash = active_shift.opening_cash + cash_sales - cash_expenses - cash_returns - total_dropped;
    let should_drop = current_drawer_cash >= max_drawer_limit;

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "data": CashDropStatusSummary {
                current_drawer_cash,
                max_drawer_limit,
                should_drop,
                total_dropped,
                drops
            }
        })),
    )
        .into_response()
}
