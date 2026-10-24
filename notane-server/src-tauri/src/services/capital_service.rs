use crate::models::capital::{
    CancelCapitalDepositRequest, CapitalDepositDetail, CapitalDepositFilter, CapitalSummaryResponse,
    CreateCapitalDepositRequest,
};
use crate::services::accounting_service::check_transaction_date_allowed;
use chrono::{Datelike, Local, NaiveDate};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct AccountOption {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub category_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CapitalEligibleAccountsResponse {
    pub payment_accounts: Vec<AccountOption>,
    pub equity_accounts: Vec<AccountOption>,
}

pub async fn generate_capital_deposit_no(
    pool: &PgPool,
    deposit_date: NaiveDate,
) -> Result<String, String> {
    let year = deposit_date.year();
    let month = deposit_date.month();
    let prefix = format!("CAP-{:04}{:02}-", year, month);

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM capital_deposits WHERE deposit_no LIKE $1",
    )
    .bind(format!("{}%", prefix))
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let next_seq = count + 1;
    Ok(format!("{}{:04}", prefix, next_seq))
}

pub async fn get_capital_eligible_accounts(
    pool: &PgPool,
) -> Result<CapitalEligibleAccountsResponse, String> {
    let payment_accounts: Vec<AccountOption> = sqlx::query_as(
        "SELECT coa.id, coa.code, coa.name, ac.name as category_name
         FROM chart_of_accounts coa
         JOIN account_categories ac ON coa.account_category_id = ac.id
         WHERE ac.name = 'ASSET' AND coa.is_active = TRUE
         ORDER BY coa.code ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Gagal mengambil daftar akun kas/bank: {}", e))?;

    let equity_accounts: Vec<AccountOption> = sqlx::query_as(
        "SELECT coa.id, coa.code, coa.name, ac.name as category_name
         FROM chart_of_accounts coa
         JOIN account_categories ac ON coa.account_category_id = ac.id
         WHERE ac.name = 'EQUITY' AND coa.is_active = TRUE
         ORDER BY coa.code ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Gagal mengambil daftar akun modal: {}", e))?;

    Ok(CapitalEligibleAccountsResponse {
        payment_accounts,
        equity_accounts,
    })
}

pub async fn create_capital_deposit(
    pool: &PgPool,
    req: CreateCapitalDepositRequest,
    user_id: Option<Uuid>,
) -> Result<CapitalDepositDetail, String> {
    if req.amount <= Decimal::ZERO {
        return Err("Nominal setoran modal harus lebih besar dari 0.".to_string());
    }

    if req.investor_name.trim().is_empty() {
        return Err("Nama penyetor modal / pemilik wajib diisi.".to_string());
    }

    let tx_date = req.deposit_date.unwrap_or_else(|| Local::now().date_naive());

    // 1. Validasi proteksi tutup buku periode akuntansi
    check_transaction_date_allowed(pool, tx_date, req.outlet_id).await?;

    // 2. Validasi Akun Pembayaran (Harus akun kategori ASSET / 1, misal Kas atau Bank)
    let is_payment_acc: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1 FROM chart_of_accounts coa
            JOIN account_categories ac ON coa.account_category_id = ac.id
            WHERE coa.id = $1 AND ac.name = 'ASSET'
        )",
    )
    .bind(req.payment_account_id)
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    if !is_payment_acc {
        return Err("Akun kas/bank penerima setoran harus bertipe Aset / Kas / Bank (ASSET).".to_string());
    }

    // 3. Validasi Akun Ekuitas (Harus akun kategori EQUITY / 3)
    let is_equity_acc: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1 FROM chart_of_accounts coa
            JOIN account_categories ac ON coa.account_category_id = ac.id
            WHERE coa.id = $1 AND ac.name = 'EQUITY'
        )",
    )
    .bind(req.equity_account_id)
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    if !is_equity_acc {
        return Err("Akun modal harus bertipe Ekuitas / Modal (EQUITY).".to_string());
    }

    let deposit_no = generate_capital_deposit_no(pool, tx_date).await?;
    let deposit_id = Uuid::new_v4();
    let journal_id = Uuid::new_v4();
    let journal_number = format!("JRN-CAP-{}", Local::now().format("%Y%m%d%H%M%S%f"));

    let desc = match &req.notes {
        Some(n) if !n.trim().is_empty() => {
            format!("Setoran Modal: {} - {}", req.investor_name.trim(), n.trim())
        }
        _ => format!("Setoran Modal: {}", req.investor_name.trim()),
    };

    let mut tx = pool.begin().await.map_err(|e| format!("Database error: {}", e))?;

    // 4. Buat Jurnal Akuntansi Berpasangan (Double-Entry)
    // Debet: Kas / Bank (Payment Account) - Aset bertambah
    // Kredit: Modal Disetor (Equity Account) - Ekuitas bertambah
    sqlx::query(
        "INSERT INTO journal_entries (
            id, journal_number, journal_date, reference_type, reference_id, description,
            total_debit, total_credit, is_closing_entry, created_at, updated_at, created_by, updated_by
        ) VALUES (
            $1, $2, $3, 'CAPITAL_DEPOSIT', $4, $5, $6, $6, FALSE, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $7, $7
        )",
    )
    .bind(journal_id)
    .bind(&journal_number)
    .bind(tx_date)
    .bind(deposit_id)
    .bind(&desc)
    .bind(req.amount)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal mencatat header jurnal setoran modal: {}", e))?;

    // Line 1: Debet Akun Kas/Bank (Asset bertambah, dengan dimensi outlet_id)
    sqlx::query(
        "INSERT INTO journal_lines (
            id, journal_id, account_id, debit, credit, memo, outlet_id, created_at, updated_at, created_by, updated_by
        ) VALUES (
            gen_random_uuid(), $1, $2, $3, 0, $4, $5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $6, $6
        )",
    )
    .bind(journal_id)
    .bind(req.payment_account_id)
    .bind(req.amount)
    .bind(&desc)
    .bind(req.outlet_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal mencatat baris debet kas jurnal setoran modal: {}", e))?;

    // Line 2: Kredit Akun Ekuitas / Modal (Equity bertambah, dengan dimensi outlet_id)
    sqlx::query(
        "INSERT INTO journal_lines (
            id, journal_id, account_id, debit, credit, memo, outlet_id, created_at, updated_at, created_by, updated_by
        ) VALUES (
            gen_random_uuid(), $1, $2, 0, $3, $4, $5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $6, $6
        )",
    )
    .bind(journal_id)
    .bind(req.equity_account_id)
    .bind(req.amount)
    .bind(&desc)
    .bind(req.outlet_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal mencatat baris kredit modal jurnal setoran modal: {}", e))?;

    // 5. Simpan transaksi setoran modal
    sqlx::query(
        "INSERT INTO capital_deposits (
            id, deposit_no, outlet_id, deposit_date, investor_name, amount,
            payment_account_id, equity_account_id, reference_no, notes,
            attachment_url, journal_entry_id, created_by, status,
            created_at, updated_at
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, 'COMPLETED',
            CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
        )",
    )
    .bind(deposit_id)
    .bind(&deposit_no)
    .bind(req.outlet_id)
    .bind(tx_date)
    .bind(req.investor_name.trim())
    .bind(req.amount)
    .bind(req.payment_account_id)
    .bind(req.equity_account_id)
    .bind(req.reference_no.as_deref().map(|s| s.trim()))
    .bind(req.notes.as_deref().map(|s| s.trim()))
    .bind(req.attachment_url)
    .bind(journal_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal menyimpan transaksi setoran modal: {}", e))?;

    tx.commit().await.map_err(|e| format!("Commit error: {}", e))?;

    get_capital_deposit_by_id(pool, deposit_id)
        .await?
        .ok_or_else(|| "Gagal mengambil data setoran modal yang baru dibuat.".to_string())
}

pub async fn get_capital_deposit_by_id(
    pool: &PgPool,
    id: Uuid,
) -> Result<Option<CapitalDepositDetail>, String> {
    let row = sqlx::query(
        "SELECT 
            cd.id, cd.deposit_no, cd.outlet_id, o.name as outlet_name,
            cd.deposit_date, cd.investor_name, cd.amount,
            cd.payment_account_id, pay.code as payment_account_code, pay.name as payment_account_name,
            cd.equity_account_id, eq.code as equity_account_code, eq.name as equity_account_name,
            cd.reference_no, cd.notes, cd.attachment_url, cd.journal_entry_id,
            cd.created_by, u_cr.full_name as created_by_name,
            cd.cancelled_by, u_cx.full_name as cancelled_by_name,
            cd.cancelled_at, cd.cancellation_reason, cd.status,
            cd.created_at, cd.updated_at
        FROM capital_deposits cd
        LEFT JOIN outlets o ON cd.outlet_id = o.id
        JOIN chart_of_accounts pay ON cd.payment_account_id = pay.id
        JOIN chart_of_accounts eq ON cd.equity_account_id = eq.id
        LEFT JOIN users u_cr ON cd.created_by = u_cr.id
        LEFT JOIN users u_cx ON cd.cancelled_by = u_cx.id
        WHERE cd.id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Gagal mengambil data setoran modal: {}", e))?;

    let res = row.map(|r| CapitalDepositDetail {
        id: r.get("id"),
        deposit_no: r.get("deposit_no"),
        outlet_id: r.get("outlet_id"),
        outlet_name: r.get("outlet_name"),
        deposit_date: r.get("deposit_date"),
        investor_name: r.get("investor_name"),
        amount: r.get("amount"),
        payment_account_id: r.get("payment_account_id"),
        payment_account_code: r.get("payment_account_code"),
        payment_account_name: r.get("payment_account_name"),
        equity_account_id: r.get("equity_account_id"),
        equity_account_code: r.get("equity_account_code"),
        equity_account_name: r.get("equity_account_name"),
        reference_no: r.get("reference_no"),
        notes: r.get("notes"),
        attachment_url: r.get("attachment_url"),
        journal_entry_id: r.get("journal_entry_id"),
        created_by: r.get("created_by"),
        created_by_name: r.get("created_by_name"),
        cancelled_by: r.get("cancelled_by"),
        cancelled_by_name: r.get("cancelled_by_name"),
        cancelled_at: r.get("cancelled_at"),
        cancellation_reason: r.get("cancellation_reason"),
        status: r.get("status"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    });

    Ok(res)
}

pub async fn list_capital_deposits(
    pool: &PgPool,
    filter: CapitalDepositFilter,
) -> Result<(Vec<CapitalDepositDetail>, i64), String> {
    let page = filter.page.unwrap_or(1).max(1);
    let limit = filter.limit.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * limit;

    let mut conditions = Vec::new();
    let mut param_index = 1;

    if filter.outlet_id.is_some() {
        conditions.push(format!("(cd.outlet_id = ${} OR cd.outlet_id IS NULL)", param_index));
        param_index += 1;
    }

    if let Some(ref st) = filter.status {
        if !st.trim().is_empty() && st.trim() != "ALL" {
            conditions.push(format!("cd.status = ${}", param_index));
            param_index += 1;
        }
    }

    if filter.start_date.is_some() {
        conditions.push(format!("cd.deposit_date >= ${}", param_index));
        param_index += 1;
    }

    if filter.end_date.is_some() {
        conditions.push(format!("cd.deposit_date <= ${}", param_index));
        param_index += 1;
    }

    if let Some(ref q) = filter.search {
        if !q.trim().is_empty() {
            conditions.push(format!(
                "(cd.deposit_no ILIKE ${param} OR cd.investor_name ILIKE ${param} OR cd.reference_no ILIKE ${param} OR cd.notes ILIKE ${param})",
                param = param_index
            ));
            param_index += 1;
        }
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let count_sql = format!(
        "SELECT COUNT(*)::bigint FROM capital_deposits cd {}",
        where_clause
    );

    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);

    if let Some(oid) = filter.outlet_id {
        count_query = count_query.bind(oid);
    }
    if let Some(ref st) = filter.status {
        if !st.trim().is_empty() && st.trim() != "ALL" {
            count_query = count_query.bind(st.trim());
        }
    }
    if let Some(sd) = filter.start_date {
        count_query = count_query.bind(sd);
    }
    if let Some(ed) = filter.end_date {
        count_query = count_query.bind(ed);
    }
    if let Some(ref q) = filter.search {
        if !q.trim().is_empty() {
            let term = format!("%{}%", q.trim());
            count_query = count_query.bind(term);
        }
    }

    let total: i64 = count_query
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Gagal menghitung jumlah transaksi setoran modal: {}", e))?;

    let select_sql = format!(
        "SELECT 
            cd.id, cd.deposit_no, cd.outlet_id, o.name as outlet_name,
            cd.deposit_date, cd.investor_name, cd.amount,
            cd.payment_account_id, pay.code as payment_account_code, pay.name as payment_account_name,
            cd.equity_account_id, eq.code as equity_account_code, eq.name as equity_account_name,
            cd.reference_no, cd.notes, cd.attachment_url, cd.journal_entry_id,
            cd.created_by, u_cr.full_name as created_by_name,
            cd.cancelled_by, u_cx.full_name as cancelled_by_name,
            cd.cancelled_at, cd.cancellation_reason, cd.status,
            cd.created_at, cd.updated_at
        FROM capital_deposits cd
        LEFT JOIN outlets o ON cd.outlet_id = o.id
        JOIN chart_of_accounts pay ON cd.payment_account_id = pay.id
        JOIN chart_of_accounts eq ON cd.equity_account_id = eq.id
        LEFT JOIN users u_cr ON cd.created_by = u_cr.id
        LEFT JOIN users u_cx ON cd.cancelled_by = u_cx.id
        {}
        ORDER BY cd.deposit_date DESC, cd.created_at DESC
        LIMIT ${} OFFSET ${}",
        where_clause,
        param_index,
        param_index + 1
    );

    let mut select_query = sqlx::query(&select_sql);

    if let Some(oid) = filter.outlet_id {
        select_query = select_query.bind(oid);
    }
    if let Some(ref st) = filter.status {
        if !st.trim().is_empty() && st.trim() != "ALL" {
            select_query = select_query.bind(st.trim());
        }
    }
    if let Some(sd) = filter.start_date {
        select_query = select_query.bind(sd);
    }
    if let Some(ed) = filter.end_date {
        select_query = select_query.bind(ed);
    }
    if let Some(ref q) = filter.search {
        if !q.trim().is_empty() {
            let term = format!("%{}%", q.trim());
            select_query = select_query.bind(term);
        }
    }

    select_query = select_query.bind(limit).bind(offset);

    let rows = select_query
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Gagal mengambil daftar setoran modal: {}", e))?;

    let items = rows
        .into_iter()
        .map(|r| CapitalDepositDetail {
            id: r.get("id"),
            deposit_no: r.get("deposit_no"),
            outlet_id: r.get("outlet_id"),
            outlet_name: r.get("outlet_name"),
            deposit_date: r.get("deposit_date"),
            investor_name: r.get("investor_name"),
            amount: r.get("amount"),
            payment_account_id: r.get("payment_account_id"),
            payment_account_code: r.get("payment_account_code"),
            payment_account_name: r.get("payment_account_name"),
            equity_account_id: r.get("equity_account_id"),
            equity_account_code: r.get("equity_account_code"),
            equity_account_name: r.get("equity_account_name"),
            reference_no: r.get("reference_no"),
            notes: r.get("notes"),
            attachment_url: r.get("attachment_url"),
            journal_entry_id: r.get("journal_entry_id"),
            created_by: r.get("created_by"),
            created_by_name: r.get("created_by_name"),
            cancelled_by: r.get("cancelled_by"),
            cancelled_by_name: r.get("cancelled_by_name"),
            cancelled_at: r.get("cancelled_at"),
            cancellation_reason: r.get("cancellation_reason"),
            status: r.get("status"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        })
        .collect();

    Ok((items, total))
}

pub async fn cancel_capital_deposit(
    pool: &PgPool,
    id: Uuid,
    req: CancelCapitalDepositRequest,
    user_id: Option<Uuid>,
) -> Result<CapitalDepositDetail, String> {
    if req.reason.trim().len() < 3 {
        return Err("Alasan pembatalan setoran modal minimal 3 karakter.".to_string());
    }

    let deposit = get_capital_deposit_by_id(pool, id)
        .await?
        .ok_or_else(|| "Transaksi setoran modal tidak ditemukan.".to_string())?;

    if deposit.status == "CANCELLED" {
        return Err("Transaksi setoran modal ini sudah pernah dibatalkan sebelumnya.".to_string());
    }

    let now_date = Local::now().date_naive();
    // Validasi penutupan buku pada tanggal pembatalan
    check_transaction_date_allowed(pool, now_date, deposit.outlet_id).await?;

    let mut tx = pool.begin().await.map_err(|e| format!("Database error: {}", e))?;

    // Buat Jurnal Pembalik (Reversal Journal Entry)
    // Debet: Akun Ekuitas/Modal (mengurangi modal)
    // Kredit: Akun Kas/Bank (mengurangi kas)
    let rev_journal_id = Uuid::new_v4();
    let rev_journal_number = format!("JRN-CAP-REV-{}", Local::now().format("%Y%m%d%H%M%S%f"));
    let rev_desc = format!(
        "Pembatalan Setoran Modal {}: {}",
        deposit.deposit_no,
        req.reason.trim()
    );

    sqlx::query(
        "INSERT INTO journal_entries (
            id, journal_number, journal_date, reference_type, reference_id, description,
            total_debit, total_credit, is_closing_entry, created_at, updated_at, created_by, updated_by
        ) VALUES (
            $1, $2, $3, 'CAPITAL_DEPOSIT_REVERSAL', $4, $5, $6, $6, FALSE, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $7, $7
        )",
    )
    .bind(rev_journal_id)
    .bind(&rev_journal_number)
    .bind(now_date)
    .bind(deposit.id)
    .bind(&rev_desc)
    .bind(deposit.amount)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal mencatat header jurnal pembalik setoran modal: {}", e))?;

    // Line 1: Debet Akun Ekuitas (mengurangi modal, dengan dimensi outlet_id)
    sqlx::query(
        "INSERT INTO journal_lines (
            id, journal_id, account_id, debit, credit, memo, outlet_id, created_at, updated_at, created_by, updated_by
        ) VALUES (
            gen_random_uuid(), $1, $2, $3, 0, $4, $5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $6, $6
        )",
    )
    .bind(rev_journal_id)
    .bind(deposit.equity_account_id)
    .bind(deposit.amount)
    .bind(&rev_desc)
    .bind(deposit.outlet_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal mencatat baris debet pembalik modal: {}", e))?;

    // Line 2: Kredit Akun Kas/Bank (mengurangi kas, dengan dimensi outlet_id)
    sqlx::query(
        "INSERT INTO journal_lines (
            id, journal_id, account_id, debit, credit, memo, outlet_id, created_at, updated_at, created_by, updated_by
        ) VALUES (
            gen_random_uuid(), $1, $2, 0, $3, $4, $5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $6, $6
        )",
    )
    .bind(rev_journal_id)
    .bind(deposit.payment_account_id)
    .bind(deposit.amount)
    .bind(&rev_desc)
    .bind(deposit.outlet_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal mencatat baris kredit pembalik kas: {}", e))?;

    // Update status transaksi setoran modal
    sqlx::query(
        "UPDATE capital_deposits
         SET status = 'CANCELLED',
             cancelled_by = $1,
             cancelled_at = CURRENT_TIMESTAMP,
             cancellation_reason = $2,
             updated_at = CURRENT_TIMESTAMP
         WHERE id = $3",
    )
    .bind(user_id)
    .bind(req.reason.trim())
    .bind(id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal mengupdate status transaksi setoran modal: {}", e))?;

    tx.commit().await.map_err(|e| format!("Commit error: {}", e))?;

    get_capital_deposit_by_id(pool, id)
        .await?
        .ok_or_else(|| "Gagal mengambil data setoran modal setelah dibatalkan.".to_string())
}

pub async fn get_capital_summary(
    pool: &PgPool,
    outlet_id: Option<Uuid>,
) -> Result<CapitalSummaryResponse, String> {
    let now = Local::now().date_naive();
    let first_day_month = NaiveDate::from_ymd_opt(now.year(), now.month(), 1).unwrap_or(now);
    let first_day_year = NaiveDate::from_ymd_opt(now.year(), 1, 1).unwrap_or(now);

    let row = sqlx::query(
        "SELECT 
            COALESCE(SUM(amount) FILTER (WHERE status = 'COMPLETED'), 0)::numeric as total_amount,
            COUNT(*) FILTER (WHERE status = 'COMPLETED')::bigint as total_count,
            COALESCE(SUM(amount) FILTER (WHERE status = 'COMPLETED' AND deposit_date >= $2), 0)::numeric as this_month_amount,
            COALESCE(SUM(amount) FILTER (WHERE status = 'COMPLETED' AND deposit_date >= $3), 0)::numeric as this_year_amount
         FROM capital_deposits
         WHERE ($1::uuid IS NULL OR outlet_id = $1 OR outlet_id IS NULL)",
    )
    .bind(outlet_id)
    .bind(first_day_month)
    .bind(first_day_year)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Gagal mengambil ringkasan setoran modal: {}", e))?;

    let total_amount: Decimal = row.get("total_amount");
    let total_count: i64 = row.get("total_count");
    let this_month_amount: Decimal = row.get("this_month_amount");
    let this_year_amount: Decimal = row.get("this_year_amount");

    Ok(CapitalSummaryResponse {
        total_amount,
        total_count,
        this_month_amount,
        this_year_amount,
    })
}
