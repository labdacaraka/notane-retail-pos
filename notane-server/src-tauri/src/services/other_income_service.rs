use crate::models::other_income::{
    CancelOtherIncomeRequest, CreateOtherIncomeCategoryRequest, CreateOtherIncomeRequest,
    OtherIncomeCategory, OtherIncomeCategoryBreakdown, OtherIncomeCategoryFilterQuery,
    OtherIncomeFilterQuery, OtherIncomeSummaryResponse, OtherIncomeTransaction,
    OtherIncomeTransactionDetail, UpdateOtherIncomeCategoryRequest,
};
use crate::services::accounting_service::check_transaction_date_allowed;
use chrono::{Datelike, Local, NaiveDate};
use rust_decimal::Decimal;
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub async fn generate_other_income_transaction_no(
    pool: &PgPool,
    transaction_date: NaiveDate,
) -> Result<String, String> {
    let year = transaction_date.year();
    let month = transaction_date.month();
    let prefix = format!("INC-{:04}{:02}-", year, month);

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM other_income_transactions WHERE transaction_no LIKE $1",
    )
    .bind(format!("{}%", prefix))
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let next_seq = count + 1;
    Ok(format!("{}{:04}", prefix, next_seq))
}

pub async fn create_other_income(
    pool: &PgPool,
    req: CreateOtherIncomeRequest,
    user_id: Option<Uuid>,
) -> Result<OtherIncomeTransactionDetail, String> {
    if req.amount <= Decimal::ZERO {
        return Err("Nominal pendapatan harus lebih besar dari Rp 0.".to_string());
    }

    let tx_date = req.transaction_date.unwrap_or_else(|| Local::now().date_naive());

    // 1. Validasi proteksi tutup buku periode akuntansi
    check_transaction_date_allowed(pool, tx_date, Some(req.outlet_id)).await?;

    // 2. Validasi Akun Pendapatan (Harus akun kategori REVENUE / 4)
    let is_revenue_acc: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1 FROM chart_of_accounts coa
            JOIN account_categories ac ON coa.account_category_id = ac.id
            WHERE coa.id = $1 AND ac.name = 'REVENUE'
        )",
    )
    .bind(req.income_account_id)
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    if !is_revenue_acc {
        return Err("Akun pendapatan harus bertipe Pendapatan (REVENUE).".to_string());
    }

    // 3. Validasi Akun Penerimaan Kas/Bank (Harus akun kategori ASSET / 1)
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
        return Err("Akun kas/bank penerima dana harus bertipe Aset (ASSET).".to_string());
    }

    // 4. Jika terkait shift kasir, pastikan shift masih OPEN
    if let Some(sid) = req.shift_id {
        let is_shift_open: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM cashier_shifts
                WHERE id = $1 AND status = 'OPEN' AND (outlet_id = $2 OR outlet_id IS NULL)
            )",
        )
        .bind(sid)
        .bind(req.outlet_id)
        .fetch_one(pool)
        .await
        .unwrap_or(false);

        if !is_shift_open {
            return Err("Shift kasir yang dipilih tidak ditemukan, sudah ditutup, atau berbeda outlet.".to_string());
        }
    }

    // Ambil nama kategori untuk deskripsi jurnal
    let category_name: String = if let Some(cid) = req.category_id {
        sqlx::query_scalar("SELECT name FROM other_income_categories WHERE id = $1")
            .bind(cid)
            .fetch_optional(pool)
            .await
            .unwrap_or(None)
            .unwrap_or_else(|| "Pendapatan Lain-lain".to_string())
    } else {
        "Pendapatan Lain-lain".to_string()
    };

    let tx_no = generate_other_income_transaction_no(pool, tx_date).await?;
    let income_id = Uuid::new_v4();
    let journal_id = Uuid::new_v4();
    let journal_number = format!("JRN-INC-{}", Local::now().format("%Y%m%d%H%M%S%f"));

    let desc = match (&req.payer_name, &req.notes) {
        (Some(p), Some(n)) if !p.trim().is_empty() && !n.trim().is_empty() => {
            format!("Pendapatan: {} ({}) - {}", category_name, p.trim(), n.trim())
        }
        (Some(p), _) if !p.trim().is_empty() => {
            format!("Pendapatan: {} ({})", category_name, p.trim())
        }
        (_, Some(n)) if !n.trim().is_empty() => {
            format!("Pendapatan: {} - {}", category_name, n.trim())
        }
        _ => format!("Pendapatan: {}", category_name),
    };

    let mut tx = pool.begin().await.map_err(|e| format!("Database error: {}", e))?;

    // 5. Buat Jurnal Akuntansi Berpasangan (Double-Entry)
    // Debet: Kas / Bank (Payment Account bertambah)
    // Kredit: Pendapatan Lain-lain (Income Account bertambah)
    sqlx::query(
        "INSERT INTO journal_entries (
            id, journal_number, journal_date, reference_type, reference_id, description,
            total_debit, total_credit, is_closing_entry, created_at, updated_at, created_by, updated_by
        ) VALUES (
            $1, $2, $3, 'OTHER_INCOME', $4, $5, $6, $6, FALSE, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $7, $7
        )",
    )
    .bind(journal_id)
    .bind(&journal_number)
    .bind(tx_date)
    .bind(income_id)
    .bind(&desc)
    .bind(req.amount)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal mencatat header jurnal pendapatan lain: {}", e))?;

    // Line 1: Debet Akun Kas/Bank (uang kas masuk bertambah)
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
    .map_err(|e| format!("Gagal mencatat baris debet kas jurnal pendapatan: {}", e))?;

    // Line 2: Kredit Akun Pendapatan (saldo pendapatan bertambah di kredit normal)
    sqlx::query(
        "INSERT INTO journal_lines (
            id, journal_id, account_id, debit, credit, memo, outlet_id, created_at, updated_at, created_by, updated_by
        ) VALUES (
            gen_random_uuid(), $1, $2, 0, $3, $4, $5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $6, $6
        )",
    )
    .bind(journal_id)
    .bind(req.income_account_id)
    .bind(req.amount)
    .bind(&desc)
    .bind(req.outlet_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal mencatat baris kredit akun pendapatan: {}", e))?;

    // 6. Simpan Transaksi Pendapatan Lain-lain
    sqlx::query(
        "INSERT INTO other_income_transactions (
            id, transaction_no, outlet_id, category_id, income_account_id, payment_account_id,
            amount, transaction_date, payer_name, notes, reference_no, attachment_url,
            shift_id, journal_entry_id, created_by, status, created_at, updated_at
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, 'COMPLETED', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
        )",
    )
    .bind(income_id)
    .bind(&tx_no)
    .bind(req.outlet_id)
    .bind(req.category_id)
    .bind(req.income_account_id)
    .bind(req.payment_account_id)
    .bind(req.amount)
    .bind(tx_date)
    .bind(req.payer_name)
    .bind(req.notes)
    .bind(req.reference_no)
    .bind(req.attachment_url)
    .bind(req.shift_id)
    .bind(journal_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal menyimpan transaksi pendapatan lain-lain: {}", e))?;

    tx.commit().await.map_err(|e| format!("Commit error: {}", e))?;

    get_other_income_by_id(pool, income_id, None).await
}

pub async fn cancel_other_income(
    pool: &PgPool,
    id: Uuid,
    req: CancelOtherIncomeRequest,
    user_id: Option<Uuid>,
) -> Result<OtherIncomeTransactionDetail, String> {
    let income = sqlx::query_as::<_, OtherIncomeTransaction>(
        "SELECT * FROM other_income_transactions WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?
    .ok_or_else(|| "Transaksi pendapatan lain-lain tidak ditemukan.".to_string())?;

    if income.status == "CANCELLED" {
        return Err("Transaksi pendapatan ini sudah pernah dibatalkan sebelumnya.".to_string());
    }

    // Validasi tanggal transaksi terhadap proteksi tutup buku
    let rev_date = Local::now().date_naive().max(income.transaction_date);
    check_transaction_date_allowed(pool, rev_date, Some(income.outlet_id)).await?;

    let mut tx = pool.begin().await.map_err(|e| format!("Database error: {}", e))?;

    // Buat Jurnal Pembalik (Reversal Journal Entry)
    // SAK: Balikkan posisi jurnal asli secara presisi
    // Debet: Pendapatan Lain-lain (mengurangi pendapatan)
    // Kredit: Kas / Bank (mengeluarkan dana yang batal diterima)
    let rev_journal_id = Uuid::new_v4();
    let rev_journal_number = format!("JRN-REV-INC-{}", Local::now().format("%Y%m%d%H%M%S%f"));
    let rev_desc = format!(
        "Pembatalan Pendapatan #{}: {}",
        income.transaction_no,
        req.reason.trim()
    );

    sqlx::query(
        "INSERT INTO journal_entries (
            id, journal_number, journal_date, reference_type, reference_id, description,
            total_debit, total_credit, is_closing_entry, created_at, updated_at, created_by, updated_by
        ) VALUES (
            $1, $2, $3, 'REVERSAL_OTHER_INCOME', $4, $5, $6, $6, FALSE, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $7, $7
        )",
    )
    .bind(rev_journal_id)
    .bind(&rev_journal_number)
    .bind(rev_date)
    .bind(income.id)
    .bind(&rev_desc)
    .bind(income.amount)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal mencatat jurnal pembalik pendapatan: {}", e))?;

    // Debet Akun Pendapatan (Menghapus pengakuan pendapatan)
    sqlx::query(
        "INSERT INTO journal_lines (
            id, journal_id, account_id, debit, credit, memo, outlet_id, created_at, updated_at, created_by, updated_by
        ) VALUES (
            gen_random_uuid(), $1, $2, $3, 0, $4, $5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $6, $6
        )",
    )
    .bind(rev_journal_id)
    .bind(income.income_account_id)
    .bind(income.amount)
    .bind(&rev_desc)
    .bind(income.outlet_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal mencatat baris debet jurnal pembalik: {}", e))?;

    // Kredit Akun Kas/Bank (Mengurangi saldo kas/bank akibat pembatalan)
    sqlx::query(
        "INSERT INTO journal_lines (
            id, journal_id, account_id, debit, credit, memo, outlet_id, created_at, updated_at, created_by, updated_by
        ) VALUES (
            gen_random_uuid(), $1, $2, 0, $3, $4, $5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $6, $6
        )",
    )
    .bind(rev_journal_id)
    .bind(income.payment_account_id)
    .bind(income.amount)
    .bind(&rev_desc)
    .bind(income.outlet_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal mencatat baris kredit kas jurnal pembalik: {}", e))?;

    // Update status transaksi pendapatan
    sqlx::query(
        "UPDATE other_income_transactions SET
            status = 'CANCELLED',
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
    .map_err(|e| format!("Gagal membatalkan transaksi pendapatan lain: {}", e))?;

    tx.commit().await.map_err(|e| format!("Commit error: {}", e))?;

    get_other_income_by_id(pool, id, None).await
}

pub async fn get_other_income_by_id(
    pool: &PgPool,
    id: Uuid,
    owner_id: Option<Uuid>,
) -> Result<OtherIncomeTransactionDetail, String> {
    let sql = "SELECT oit.*,
                      o.name as outlet_name,
                      oic.name as category_name,
                      inc_coa.code as income_account_code,
                      inc_coa.name as income_account_name,
                      pay_coa.code as payment_account_code,
                      pay_coa.name as payment_account_name,
                      u_cr.full_name as created_by_name,
                      u_cn.full_name as cancelled_by_name
               FROM other_income_transactions oit
               JOIN outlets o ON oit.outlet_id = o.id
               LEFT JOIN other_income_categories oic ON oit.category_id = oic.id
               JOIN chart_of_accounts inc_coa ON oit.income_account_id = inc_coa.id
               JOIN chart_of_accounts pay_coa ON oit.payment_account_id = pay_coa.id
               LEFT JOIN users u_cr ON oit.created_by = u_cr.id
               LEFT JOIN users u_cn ON oit.cancelled_by = u_cn.id
               WHERE oit.id = $1";

    let row = sqlx::query(sql)
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or_else(|| "Transaksi pendapatan lain-lain tidak ditemukan.".to_string())?;

    if let Some(uid) = owner_id {
        let created_by: Option<Uuid> = row.get("created_by");
        if created_by != Some(uid) {
            return Err("Akses ditolak: Anda tidak memiliki izin melihat transaksi ini.".to_string());
        }
    }

    Ok(OtherIncomeTransactionDetail {
        id: row.get("id"),
        transaction_no: row.get("transaction_no"),
        outlet_id: row.get("outlet_id"),
        outlet_name: row.get("outlet_name"),
        category_id: row.get("category_id"),
        category_name: row.get("category_name"),
        income_account_id: row.get("income_account_id"),
        income_account_code: row.get("income_account_code"),
        income_account_name: row.get("income_account_name"),
        payment_account_id: row.get("payment_account_id"),
        payment_account_code: row.get("payment_account_code"),
        payment_account_name: row.get("payment_account_name"),
        amount: row.get("amount"),
        transaction_date: row.get("transaction_date"),
        payer_name: row.get("payer_name"),
        notes: row.get("notes"),
        reference_no: row.get("reference_no"),
        attachment_url: row.get("attachment_url"),
        shift_id: row.get("shift_id"),
        journal_entry_id: row.get("journal_entry_id"),
        created_by: row.get("created_by"),
        created_by_name: row.get("created_by_name"),
        cancelled_by: row.get("cancelled_by"),
        cancelled_by_name: row.get("cancelled_by_name"),
        cancelled_at: row.get("cancelled_at"),
        cancellation_reason: row.get("cancellation_reason"),
        status: row.get("status"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

pub async fn list_other_incomes(
    pool: &PgPool,
    query: OtherIncomeFilterQuery,
    owner_id: Option<Uuid>,
) -> Result<(Vec<OtherIncomeTransactionDetail>, i64), String> {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).max(1);
    let offset = (page - 1) * limit;

    let mut conditions = Vec::new();
    let mut count_conditions = Vec::new();

    if let Some(oid) = query.outlet_id {
        conditions.push(format!("oit.outlet_id = '{oid}'"));
        count_conditions.push(format!("outlet_id = '{oid}'"));
    }

    if let Some(cid) = query.category_id {
        conditions.push(format!("oit.category_id = '{cid}'"));
        count_conditions.push(format!("category_id = '{cid}'"));
    }

    if let Some(ref st) = query.status {
        if !st.trim().is_empty() && st != "ALL" {
            let clean = st.replace('\'', "''");
            conditions.push(format!("oit.status = '{clean}'"));
            count_conditions.push(format!("status = '{clean}'"));
        }
    }

    if let Some(sid) = query.shift_id {
        conditions.push(format!("oit.shift_id = '{sid}'"));
        count_conditions.push(format!("shift_id = '{sid}'"));
    }

    if let Some(sdate) = query.start_date {
        conditions.push(format!("oit.transaction_date >= '{sdate}'"));
        count_conditions.push(format!("transaction_date >= '{sdate}'"));
    }

    if let Some(edate) = query.end_date {
        conditions.push(format!("oit.transaction_date <= '{edate}'"));
        count_conditions.push(format!("transaction_date <= '{edate}'"));
    }

    if let Some(uid) = owner_id {
        conditions.push(format!("oit.created_by = '{uid}'"));
        count_conditions.push(format!("created_by = '{uid}'"));
    }

    if let Some(ref q) = query.search {
        let clean = q.trim().replace('\'', "''");
        if !clean.is_empty() {
            let search_clause = format!(
                "(oit.transaction_no ILIKE '%{clean}%' OR \
                  oit.payer_name ILIKE '%{clean}%' OR \
                  oit.notes ILIKE '%{clean}%' OR \
                  oit.reference_no ILIKE '%{clean}%' OR \
                  oic.name ILIKE '%{clean}%')"
            );
            conditions.push(search_clause);

            let count_search = format!(
                "(transaction_no ILIKE '%{clean}%' OR \
                  payer_name ILIKE '%{clean}%' OR \
                  notes ILIKE '%{clean}%' OR \
                  reference_no ILIKE '%{clean}%')"
            );
            count_conditions.push(count_search);
        }
    }

    let where_clause = if conditions.is_empty() {
        "".to_string()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let count_where_clause = if count_conditions.is_empty() {
        "".to_string()
    } else {
        format!("WHERE {}", count_conditions.join(" AND "))
    };

    let count_sql = format!(
        "SELECT COUNT(*)::bigint FROM other_income_transactions {}",
        count_where_clause
    );

    let total: i64 = sqlx::query_scalar(&count_sql)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let select_sql = format!(
        "SELECT oit.*,
                o.name as outlet_name,
                oic.name as category_name,
                inc_coa.code as income_account_code,
                inc_coa.name as income_account_name,
                pay_coa.code as payment_account_code,
                pay_coa.name as payment_account_name,
                u_cr.full_name as created_by_name,
                u_cn.full_name as cancelled_by_name
         FROM other_income_transactions oit
         JOIN outlets o ON oit.outlet_id = o.id
         LEFT JOIN other_income_categories oic ON oit.category_id = oic.id
         JOIN chart_of_accounts inc_coa ON oit.income_account_id = inc_coa.id
         JOIN chart_of_accounts pay_coa ON oit.payment_account_id = pay_coa.id
         LEFT JOIN users u_cr ON oit.created_by = u_cr.id
         LEFT JOIN users u_cn ON oit.cancelled_by = u_cn.id
         {}
         ORDER BY oit.transaction_date DESC, oit.created_at DESC
         LIMIT {} OFFSET {}",
        where_clause, limit, offset
    );

    let rows = sqlx::query(&select_sql)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Gagal memuat daftar pendapatan lain: {}", e))?;

    let items = rows
        .into_iter()
        .map(|r| OtherIncomeTransactionDetail {
            id: r.get("id"),
            transaction_no: r.get("transaction_no"),
            outlet_id: r.get("outlet_id"),
            outlet_name: r.get("outlet_name"),
            category_id: r.get("category_id"),
            category_name: r.get("category_name"),
            income_account_id: r.get("income_account_id"),
            income_account_code: r.get("income_account_code"),
            income_account_name: r.get("income_account_name"),
            payment_account_id: r.get("payment_account_id"),
            payment_account_code: r.get("payment_account_code"),
            payment_account_name: r.get("payment_account_name"),
            amount: r.get("amount"),
            transaction_date: r.get("transaction_date"),
            payer_name: r.get("payer_name"),
            notes: r.get("notes"),
            reference_no: r.get("reference_no"),
            attachment_url: r.get("attachment_url"),
            shift_id: r.get("shift_id"),
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

pub async fn get_other_income_summary(
    pool: &PgPool,
    outlet_id: Option<Uuid>,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
) -> Result<OtherIncomeSummaryResponse, String> {
    let today = Local::now().date_naive();
    let first_day_of_month = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();

    let mut conditions = vec!["status = 'COMPLETED'".to_string()];
    if let Some(oid) = outlet_id {
        conditions.push(format!("outlet_id = '{oid}'"));
    }
    if let Some(sd) = start_date {
        conditions.push(format!("transaction_date >= '{sd}'"));
    }
    if let Some(ed) = end_date {
        conditions.push(format!("transaction_date <= '{ed}'"));
    }

    let filter_clause = format!("WHERE {}", conditions.join(" AND "));

    let total_query = format!(
        "SELECT COALESCE(SUM(amount), 0) as total_amount, COUNT(*)::bigint as total_count
         FROM other_income_transactions {}",
        filter_clause
    );

    let total_row = sqlx::query(&total_query)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Gagal memuat ringkasan pendapatan: {}", e))?;

    let total_amount: Decimal = total_row.get("total_amount");
    let total_count: i64 = total_row.get("total_count");

    // Hari Ini
    let outlet_filter = if let Some(oid) = outlet_id {
        format!(" AND outlet_id = '{oid}'")
    } else {
        "".to_string()
    };

    let today_query = format!(
        "SELECT COALESCE(SUM(amount), 0) FROM other_income_transactions
         WHERE status = 'COMPLETED' AND transaction_date = $1{}",
        outlet_filter
    );
    let today_amount: Decimal = sqlx::query_scalar(&today_query)
        .bind(today)
        .fetch_one(pool)
        .await
        .unwrap_or(Decimal::ZERO);

    // Bulan Ini
    let month_query = format!(
        "SELECT COALESCE(SUM(amount), 0) FROM other_income_transactions
         WHERE status = 'COMPLETED' AND transaction_date >= $1 AND transaction_date <= $2{}",
        outlet_filter
    );
    let this_month_amount: Decimal = sqlx::query_scalar(&month_query)
        .bind(first_day_of_month)
        .bind(today)
        .fetch_one(pool)
        .await
        .unwrap_or(Decimal::ZERO);

    // Breakdown per kategori
    let breakdown_query = format!(
        "SELECT oit.category_id,
                COALESCE(oic.name, 'Lain-lain / Tanpa Kategori') as category_name,
                COALESCE(SUM(oit.amount), 0) as cat_amount,
                COUNT(*)::bigint as cat_count
         FROM other_income_transactions oit
         LEFT JOIN other_income_categories oic ON oit.category_id = oic.id
         {}
         GROUP BY oit.category_id, oic.name
         ORDER BY cat_amount DESC",
        filter_clause
    );

    let breakdown_rows = sqlx::query(&breakdown_query)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

    let mut categories_breakdown = Vec::new();
    for r in breakdown_rows {
        let cat_amount: Decimal = r.get("cat_amount");
        let percentage = if total_amount > Decimal::ZERO {
            (cat_amount / total_amount * Decimal::from(100)).round_dp(2)
        } else {
            Decimal::ZERO
        };

        categories_breakdown.push(OtherIncomeCategoryBreakdown {
            category_id: r.get("category_id"),
            category_name: r.get("category_name"),
            total_amount: cat_amount,
            transaction_count: r.get("cat_count"),
            percentage,
        });
    }

    Ok(OtherIncomeSummaryResponse {
        total_amount,
        total_count,
        today_amount,
        this_month_amount,
        categories_breakdown,
    })
}

// ==========================================
// KATEGORI PENDAPATAN LAIN-LAIN
// ==========================================

pub async fn list_other_income_categories(
    pool: &PgPool,
    query: &OtherIncomeCategoryFilterQuery,
) -> Result<(Vec<OtherIncomeCategory>, i64), String> {
    let mut conditions = Vec::new();
    if query.active_only.unwrap_or(false) {
        conditions.push("oic.is_active = TRUE".to_string());
    }
    if let Some(ref q) = query.search {
        let clean = q.trim().replace('\'', "''");
        if !clean.is_empty() {
            conditions.push(format!(
                "(oic.name ILIKE '%{clean}%' OR oic.code ILIKE '%{clean}%' OR COALESCE(oic.description, '') ILIKE '%{clean}%' OR COALESCE(coa.name, '') ILIKE '%{clean}%' OR COALESCE(coa.code, '') ILIKE '%{clean}%')"
            ));
        }
    }
    let where_clause = if conditions.is_empty() {
        "".to_string()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let count_sql = format!(
        "SELECT COUNT(*)::bigint FROM other_income_categories oic
         LEFT JOIN chart_of_accounts coa ON oic.account_id = coa.id
         {}",
        where_clause
    );
    let total: i64 = sqlx::query_scalar(&count_sql)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Gagal menghitung total kategori pendapatan: {}", e))?;

    let sort_col = match query.sort_by.as_deref() {
        Some("code") => "oic.code",
        Some("account_code") => "coa.code",
        Some("account_name") => "coa.name",
        Some("created_at") => "oic.created_at",
        Some("is_active") => "oic.is_active",
        _ => "oic.name",
    };
    let sort_dir = match query.sort_order.as_deref() {
        Some(s) if s.eq_ignore_ascii_case("desc") => "DESC",
        _ => "ASC",
    };

    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).max(1);
    let offset = (page - 1) * limit;

    let sql = format!(
        "SELECT oic.*, coa.name as account_name, coa.code as account_code
         FROM other_income_categories oic
         LEFT JOIN chart_of_accounts coa ON oic.account_id = coa.id
         {}
         ORDER BY {} {} NULLS LAST
         LIMIT {} OFFSET {}",
        where_clause, sort_col, sort_dir, limit, offset
    );

    let rows = sqlx::query_as::<_, OtherIncomeCategory>(&sql)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Gagal memuat kategori pendapatan: {}", e))?;

    Ok((rows, total))
}

pub async fn create_other_income_category(
    pool: &PgPool,
    req: CreateOtherIncomeCategoryRequest,
) -> Result<OtherIncomeCategory, String> {
    let clean_code = req.code.trim().to_uppercase();
    let clean_name = req.name.trim();

    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM other_income_categories WHERE code = $1)",
    )
    .bind(&clean_code)
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    if exists {
        return Err(format!("Kode kategori '{}' sudah digunakan.", clean_code));
    }

    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO other_income_categories (id, code, name, account_id, description, is_active)
         VALUES ($1, $2, $3, $4, $5, TRUE)",
    )
    .bind(id)
    .bind(&clean_code)
    .bind(clean_name)
    .bind(req.account_id)
    .bind(req.description.as_deref().map(|s| s.trim()))
    .execute(pool)
    .await
    .map_err(|e| format!("Gagal membuat kategori pendapatan: {}", e))?;

    let sql = "SELECT oic.*, coa.name as account_name, coa.code as account_code
               FROM other_income_categories oic
               LEFT JOIN chart_of_accounts coa ON oic.account_id = coa.id
               WHERE oic.id = $1";

    sqlx::query_as::<_, OtherIncomeCategory>(sql)
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Database error: {}", e))
}

pub async fn update_other_income_category(
    pool: &PgPool,
    id: Uuid,
    req: UpdateOtherIncomeCategoryRequest,
) -> Result<OtherIncomeCategory, String> {
    let clean_name = req.name.trim();

    let res = sqlx::query(
        "UPDATE other_income_categories SET
            name = $1,
            account_id = $2,
            description = $3,
            is_active = COALESCE($4, is_active),
            updated_at = CURRENT_TIMESTAMP
        WHERE id = $5",
    )
    .bind(clean_name)
    .bind(req.account_id)
    .bind(req.description.as_deref().map(|s| s.trim()))
    .bind(req.is_active)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| format!("Gagal memperbarui kategori pendapatan: {}", e))?;

    if res.rows_affected() == 0 {
        return Err("Kategori pendapatan lain-lain tidak ditemukan.".to_string());
    }

    let sql = "SELECT oic.*, coa.name as account_name, coa.code as account_code
               FROM other_income_categories oic
               LEFT JOIN chart_of_accounts coa ON oic.account_id = coa.id
               WHERE oic.id = $1";

    sqlx::query_as::<_, OtherIncomeCategory>(sql)
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Database error: {}", e))
}

