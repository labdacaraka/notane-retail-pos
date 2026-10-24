use crate::models::expense::{
    CancelExpenseRequest, CreateExpenseCategoryRequest, CreateExpenseRequest,
    ExpenseCategory, ExpenseCategoryBreakdown, ExpenseFilterQuery, ExpenseSummaryResponse,
    ExpenseTransaction, ExpenseTransactionDetail, UpdateExpenseCategoryRequest,
};
use crate::services::accounting_service::check_transaction_date_allowed;
use chrono::{Datelike, Local, NaiveDate};
use rust_decimal::Decimal;
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub async fn generate_expense_transaction_no(
    pool: &PgPool,
    transaction_date: NaiveDate,
) -> Result<String, String> {
    let year = transaction_date.year();
    let month = transaction_date.month();
    let prefix = format!("EXP-{:04}{:02}-", year, month);

    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*)::bigint FROM expense_transactions WHERE transaction_no LIKE $1",
    )
    .bind(format!("{}%", prefix))
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let next_seq = count + 1;
    Ok(format!("{}{:04}", prefix, next_seq))
}

pub async fn create_expense(
    pool: &PgPool,
    req: CreateExpenseRequest,
    user_id: Option<Uuid>,
) -> Result<ExpenseTransactionDetail, String> {
    if req.amount <= Decimal::ZERO {
        return Err("Nominal pengeluaran harus lebih besar dari 0.".to_string());
    }

    let tx_date = req.transaction_date.unwrap_or_else(|| Local::now().date_naive());

    // 1. Validasi proteksi tutup buku periode akuntansi
    check_transaction_date_allowed(pool, tx_date, Some(req.outlet_id)).await?;

    // 2. Validasi Akun Beban atau Penarikan Prive Pemilik (EXPENSE / EQUITY 3103)
    let is_expense_acc: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1 FROM chart_of_accounts coa
            JOIN account_categories ac ON coa.account_category_id = ac.id
            WHERE coa.id = $1 AND (ac.name = 'EXPENSE' OR (ac.name = 'EQUITY' AND coa.code = '3103'))
        )",
    )
    .bind(req.expense_account_id)
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    if !is_expense_acc {
        return Err("Akun pengeluaran harus bertipe Beban (EXPENSE) atau Penarikan Ekuitas Prive (3103).".to_string());
    }

    // 3. Validasi Akun Pembayaran (Harus akun kategori ASSET / 1, misal Kas atau Bank)
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
        return Err("Akun sumber dana pembayaran harus bertipe Aset / Kas / Bank (ASSET).".to_string());
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
        sqlx::query_scalar("SELECT name FROM expense_categories WHERE id = $1")
            .bind(cid)
            .fetch_optional(pool)
            .await
            .unwrap_or(None)
            .unwrap_or_else(|| "Biaya Operasional".to_string())
    } else {
        "Biaya Operasional".to_string()
    };

    let tx_no = generate_expense_transaction_no(pool, tx_date).await?;
    let expense_id = Uuid::new_v4();
    let journal_id = Uuid::new_v4();
    let journal_number = format!("JRN-EXP-{}", Local::now().format("%Y%m%d%H%M%S%f"));

    let desc = match (&req.recipient_name, &req.notes) {
        (Some(r), Some(n)) if !r.trim().is_empty() && !n.trim().is_empty() => {
            format!("Pengeluaran: {} ({}) - {}", category_name, r.trim(), n.trim())
        }
        (Some(r), _) if !r.trim().is_empty() => {
            format!("Pengeluaran: {} ({})", category_name, r.trim())
        }
        (_, Some(n)) if !n.trim().is_empty() => {
            format!("Pengeluaran: {} - {}", category_name, n.trim())
        }
        _ => format!("Pengeluaran: {}", category_name),
    };

    let mut tx = pool.begin().await.map_err(|e| format!("Database error: {}", e))?;

    // 5. Buat Jurnal Akuntansi Berpasangan (Double-Entry)
    // Debet: Beban (Expense Account)
    // Kredit: Kas / Bank (Payment Account)
    sqlx::query(
        "INSERT INTO journal_entries (
            id, journal_number, journal_date, reference_type, reference_id, description,
            total_debit, total_credit, is_closing_entry, created_at, updated_at, created_by, updated_by
        ) VALUES (
            $1, $2, $3, 'EXPENSE', $4, $5, $6, $6, FALSE, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $7, $7
        )",
    )
    .bind(journal_id)
    .bind(&journal_number)
    .bind(tx_date)
    .bind(expense_id)
    .bind(&desc)
    .bind(req.amount)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal mencatat header jurnal pengeluaran: {}", e))?;

    // Line 1: Debet Akun Beban (dengan dimensi outlet_id)
    sqlx::query(
        "INSERT INTO journal_lines (
            id, journal_id, account_id, debit, credit, memo, outlet_id, created_at, updated_at, created_by, updated_by
        ) VALUES (
            gen_random_uuid(), $1, $2, $3, 0, $4, $5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $6, $6
        )",
    )
    .bind(journal_id)
    .bind(req.expense_account_id)
    .bind(req.amount)
    .bind(&desc)
    .bind(req.outlet_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal mencatat baris debet jurnal pengeluaran: {}", e))?;

    // Line 2: Kredit Akun Kas/Bank (dengan dimensi outlet_id)
    sqlx::query(
        "INSERT INTO journal_lines (
            id, journal_id, account_id, debit, credit, memo, outlet_id, created_at, updated_at, created_by, updated_by
        ) VALUES (
            gen_random_uuid(), $1, $2, 0, $3, $4, $5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $6, $6
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
    .map_err(|e| format!("Gagal mencatat baris kredit jurnal pengeluaran: {}", e))?;

    // 6. Simpan Transaksi Pengeluaran
    sqlx::query(
        "INSERT INTO expense_transactions (
            id, transaction_no, outlet_id, category_id, expense_account_id, payment_account_id,
            amount, transaction_date, recipient_name, notes, reference_no, attachment_url,
            shift_id, journal_entry_id, created_by, status, created_at, updated_at
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, 'COMPLETED', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
        )",
    )
    .bind(expense_id)
    .bind(&tx_no)
    .bind(req.outlet_id)
    .bind(req.category_id)
    .bind(req.expense_account_id)
    .bind(req.payment_account_id)
    .bind(req.amount)
    .bind(tx_date)
    .bind(req.recipient_name)
    .bind(req.notes)
    .bind(req.reference_no)
    .bind(req.attachment_url)
    .bind(req.shift_id)
    .bind(journal_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal menyimpan transaksi pengeluaran: {}", e))?;

    tx.commit().await.map_err(|e| format!("Commit error: {}", e))?;

    get_expense_by_id(pool, expense_id, None).await
}

pub async fn cancel_expense(
    pool: &PgPool,
    id: Uuid,
    req: CancelExpenseRequest,
    user_id: Option<Uuid>,
) -> Result<ExpenseTransactionDetail, String> {
    let expense = sqlx::query_as::<_, ExpenseTransaction>(
        "SELECT * FROM expense_transactions WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?
    .ok_or_else(|| "Transaksi pengeluaran tidak ditemukan.".to_string())?;

    if expense.status == "CANCELLED" {
        return Err("Transaksi pengeluaran ini sudah pernah dibatalkan sebelumnya.".to_string());
    }

    // Validasi tanggal transaksi terhadap proteksi tutup buku
    let today = Local::now().date_naive();
    check_transaction_date_allowed(pool, today, Some(expense.outlet_id)).await?;

    let mut tx = pool.begin().await.map_err(|e| format!("Database error: {}", e))?;

    // Buat Jurnal Pembalik (Reversal Journal Entry)
    let rev_journal_id = Uuid::new_v4();
    let rev_journal_number = format!("JRN-REV-EXP-{}", Local::now().format("%Y%m%d%H%M%S%f"));
    let rev_desc = format!(
        "Pembatalan Pengeluaran #{}: {}",
        expense.transaction_no,
        req.reason.trim()
    );

    sqlx::query(
        "INSERT INTO journal_entries (
            id, journal_number, journal_date, reference_type, reference_id, description,
            total_debit, total_credit, is_closing_entry, created_at, updated_at, created_by, updated_by
        ) VALUES (
            $1, $2, $3, 'EXPENSE_VOID', $4, $5, $6, $6, FALSE, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $7, $7
        )",
    )
    .bind(rev_journal_id)
    .bind(&rev_journal_number)
    .bind(today)
    .bind(expense.id)
    .bind(&rev_desc)
    .bind(expense.amount)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal mencatat jurnal pembalik: {}", e))?;

    // Debet Kas/Bank (Uang kembali ke kas pada outlet_id)
    sqlx::query(
        "INSERT INTO journal_lines (
            id, journal_id, account_id, debit, credit, memo, outlet_id, created_at, updated_at, created_by, updated_by
        ) VALUES (
            gen_random_uuid(), $1, $2, $3, 0, $4, $5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $6, $6
        )",
    )
    .bind(rev_journal_id)
    .bind(expense.payment_account_id)
    .bind(expense.amount)
    .bind(&rev_desc)
    .bind(expense.outlet_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal mencatat baris kas jurnal pembalik: {}", e))?;

    // Kredit Beban (Mengurangi beban yang tercatat pada outlet_id)
    sqlx::query(
        "INSERT INTO journal_lines (
            id, journal_id, account_id, debit, credit, memo, outlet_id, created_at, updated_at, created_by, updated_by
        ) VALUES (
            gen_random_uuid(), $1, $2, 0, $3, $4, $5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $6, $6
        )",
    )
    .bind(rev_journal_id)
    .bind(expense.expense_account_id)
    .bind(expense.amount)
    .bind(&rev_desc)
    .bind(expense.outlet_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal mencatat baris beban jurnal pembalik: {}", e))?;

    // Update status transaksi pengeluaran
    sqlx::query(
        "UPDATE expense_transactions SET
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
    .map_err(|e| format!("Gagal membatalkan transaksi pengeluaran: {}", e))?;

    tx.commit().await.map_err(|e| format!("Commit error: {}", e))?;

    get_expense_by_id(pool, id, None).await
}

pub async fn get_expense_by_id(pool: &PgPool, id: Uuid, owner_id: Option<Uuid>) -> Result<ExpenseTransactionDetail, String> {
    let sql = "SELECT et.*,
                      o.name as outlet_name,
                      ec.name as category_name,
                      exp_coa.code as expense_account_code,
                      exp_coa.name as expense_account_name,
                      pay_coa.code as payment_account_code,
                      pay_coa.name as payment_account_name,
                      u_cr.full_name as created_by_name,
                      u_cn.full_name as cancelled_by_name
               FROM expense_transactions et
               JOIN outlets o ON et.outlet_id = o.id
               LEFT JOIN expense_categories ec ON et.category_id = ec.id
               JOIN chart_of_accounts exp_coa ON et.expense_account_id = exp_coa.id
               JOIN chart_of_accounts pay_coa ON et.payment_account_id = pay_coa.id
               LEFT JOIN users u_cr ON et.created_by = u_cr.id
               LEFT JOIN users u_cn ON et.cancelled_by = u_cn.id
               WHERE et.id = $1";

    let row = sqlx::query(sql)
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or_else(|| "Transaksi pengeluaran tidak ditemukan.".to_string())?;

    if let Some(uid) = owner_id {
        let created_by = row.try_get::<Option<Uuid>, _>("created_by").unwrap_or(None);
        let cancelled_by = row.try_get::<Option<Uuid>, _>("cancelled_by").unwrap_or(None);
        let is_owner = created_by == Some(uid) || cancelled_by == Some(uid);
        if !is_owner {
            return Err("Akses ditolak: Anda tidak memiliki hak akses untuk melihat data transaksi pengeluaran staf lain.".to_string());
        }
    }

    Ok(ExpenseTransactionDetail {
        id: row.get("id"),
        transaction_no: row.get("transaction_no"),
        outlet_id: row.get("outlet_id"),
        outlet_name: row.get("outlet_name"),
        category_id: row.get("category_id"),
        category_name: row.get("category_name"),
        expense_account_id: row.get("expense_account_id"),
        expense_account_code: row.get("expense_account_code"),
        expense_account_name: row.get("expense_account_name"),
        payment_account_id: row.get("payment_account_id"),
        payment_account_code: row.get("payment_account_code"),
        payment_account_name: row.get("payment_account_name"),
        amount: row.get("amount"),
        transaction_date: row.get("transaction_date"),
        recipient_name: row.get("recipient_name"),
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

pub async fn list_expenses(
    pool: &PgPool,
    query: ExpenseFilterQuery,
    owner_id: Option<Uuid>,
) -> Result<(Vec<ExpenseTransactionDetail>, i64), String> {
    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let page = query.page.unwrap_or(1).max(1);
    let offset = (page - 1) * limit;

    let mut where_clauses = vec!["1=1".to_string()];

    if let Some(uid) = owner_id {
        where_clauses.push(format!("(et.created_by = '{uid}' OR et.cancelled_by = '{uid}')"));
    }

    if let Some(oid) = query.outlet_id {
        where_clauses.push(format!("et.outlet_id = '{oid}'"));
    }
    if let Some(cid) = query.category_id {
        where_clauses.push(format!("et.category_id = '{cid}'"));
    }
    if let Some(s) = query.start_date {
        where_clauses.push(format!("et.transaction_date >= '{s}'"));
    }
    if let Some(e) = query.end_date {
        where_clauses.push(format!("et.transaction_date <= '{e}'"));
    }
    if let Some(ref st) = query.status {
        if !st.trim().is_empty() {
            where_clauses.push(format!("et.status = '{}'", st.trim().to_uppercase()));
        }
    }
    if let Some(sid) = query.shift_id {
        where_clauses.push(format!("et.shift_id = '{sid}'"));
    }
    if let Some(ref search) = query.search {
        let s = search.trim();
        if !s.is_empty() {
            let escaped = s.replace('\'', "''");
            where_clauses.push(format!(
                "(et.transaction_no ILIKE '%{escaped}%' OR et.recipient_name ILIKE '%{escaped}%' OR et.notes ILIKE '%{escaped}%' OR et.reference_no ILIKE '%{escaped}%')"
            ));
        }
    }

    let where_sql = where_clauses.join(" AND ");

    let count_sql = format!("SELECT COUNT(*)::bigint FROM expense_transactions et WHERE {}", where_sql);
    let total: i64 = sqlx::query_scalar(&count_sql)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let sql = format!(
        "SELECT et.*,
                o.name as outlet_name,
                ec.name as category_name,
                exp_coa.code as expense_account_code,
                exp_coa.name as expense_account_name,
                pay_coa.code as payment_account_code,
                pay_coa.name as payment_account_name,
                u_cr.full_name as created_by_name,
                u_cn.full_name as cancelled_by_name
         FROM expense_transactions et
         JOIN outlets o ON et.outlet_id = o.id
         LEFT JOIN expense_categories ec ON et.category_id = ec.id
         JOIN chart_of_accounts exp_coa ON et.expense_account_id = exp_coa.id
         JOIN chart_of_accounts pay_coa ON et.payment_account_id = pay_coa.id
         LEFT JOIN users u_cr ON et.created_by = u_cr.id
         LEFT JOIN users u_cn ON et.cancelled_by = u_cn.id
         WHERE {}
         ORDER BY et.transaction_date DESC, et.created_at DESC
         LIMIT {} OFFSET {}",
        where_sql, limit, offset
    );

    let rows = sqlx::query(&sql)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let list = rows
        .into_iter()
        .map(|row| ExpenseTransactionDetail {
            id: row.get("id"),
            transaction_no: row.get("transaction_no"),
            outlet_id: row.get("outlet_id"),
            outlet_name: row.get("outlet_name"),
            category_id: row.get("category_id"),
            category_name: row.get("category_name"),
            expense_account_id: row.get("expense_account_id"),
            expense_account_code: row.get("expense_account_code"),
            expense_account_name: row.get("expense_account_name"),
            payment_account_id: row.get("payment_account_id"),
            payment_account_code: row.get("payment_account_code"),
            payment_account_name: row.get("payment_account_name"),
            amount: row.get("amount"),
            transaction_date: row.get("transaction_date"),
            recipient_name: row.get("recipient_name"),
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
        .collect();

    Ok((list, total))
}

pub async fn get_expense_summary(
    pool: &PgPool,
    outlet_id: Option<Uuid>,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
    owner_id: Option<Uuid>,
) -> Result<ExpenseSummaryResponse, String> {
    let today = Local::now().date_naive();
    let month_start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today);

    let mut outlet_filter = "".to_string();
    if let Some(oid) = outlet_id {
        outlet_filter = format!(" AND et.outlet_id = '{oid}'");
    }

    let mut date_filter = "".to_string();
    if let Some(s) = start_date {
        date_filter.push_str(&format!(" AND et.transaction_date >= '{s}'"));
    }
    if let Some(e) = end_date {
        date_filter.push_str(&format!(" AND et.transaction_date <= '{e}'"));
    }

    let owner_filter = if let Some(uid) = owner_id {
        format!(" AND (et.created_by = '{uid}' OR et.cancelled_by = '{uid}')")
    } else {
        "".to_string()
    };

    let overall_sql = format!(
        "SELECT COALESCE(SUM(amount), 0) as total_amount, COUNT(*)::bigint as total_count
         FROM expense_transactions et
         WHERE status = 'COMPLETED' {}{}{}",
        outlet_filter, date_filter, owner_filter
    );

    let row = sqlx::query(&overall_sql)
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let total_amount: Decimal = row.get("total_amount");
    let total_count: i64 = row.get("total_count");

    // Pengeluaran hari ini
    let today_sql = format!(
        "SELECT COALESCE(SUM(amount), 0) as today_amount
         FROM expense_transactions et
         WHERE status = 'COMPLETED' AND transaction_date = '{}'{}{}",
        today, outlet_filter, owner_filter
    );
    let today_amount: Decimal = sqlx::query_scalar(&today_sql)
        .fetch_one(pool)
        .await
        .unwrap_or(Decimal::ZERO);

    // Pengeluaran bulan ini
    let month_sql = format!(
        "SELECT COALESCE(SUM(amount), 0) as month_amount
         FROM expense_transactions et
         WHERE status = 'COMPLETED' AND transaction_date >= '{}' AND transaction_date <= '{}'{}{}",
        month_start, today, outlet_filter, owner_filter
    );
    let this_month_amount: Decimal = sqlx::query_scalar(&month_sql)
        .fetch_one(pool)
        .await
        .unwrap_or(Decimal::ZERO);

    // Breakdown per kategori
    let breakdown_sql = format!(
        "SELECT ec.id as category_id,
                COALESCE(ec.name, 'Lain-lain / Tanpa Kategori') as category_name,
                COALESCE(SUM(et.amount), 0) as total_amount,
                COUNT(et.id)::bigint as transaction_count
         FROM expense_transactions et
         LEFT JOIN expense_categories ec ON et.category_id = ec.id
         WHERE et.status = 'COMPLETED' {}{}{}
         GROUP BY ec.id, ec.name
         ORDER BY total_amount DESC",
        outlet_filter, date_filter, owner_filter
    );

    let breakdown_rows = sqlx::query(&breakdown_sql)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let mut categories_breakdown = Vec::new();
    for r in breakdown_rows {
        let cat_amount: Decimal = r.get("total_amount");
        let pct = if total_amount > Decimal::ZERO {
            ((cat_amount / total_amount) * Decimal::from(100)).round_dp(1)
        } else {
            Decimal::ZERO
        };
        categories_breakdown.push(ExpenseCategoryBreakdown {
            category_id: r.get("category_id"),
            category_name: r.get("category_name"),
            total_amount: cat_amount,
            transaction_count: r.get("transaction_count"),
            percentage: pct,
        });
    }

    Ok(ExpenseSummaryResponse {
        total_amount,
        total_count,
        today_amount,
        this_month_amount,
        categories_breakdown,
    })
}

// ----------------------------------------------------------------------------
// KATEGORI PENGELUARAN
// ----------------------------------------------------------------------------

pub async fn list_expense_categories(
    pool: &PgPool,
    active_only: bool,
) -> Result<Vec<ExpenseCategory>, String> {
    let mut sql = "SELECT ec.*, coa.name as account_name, coa.code as account_code
                   FROM expense_categories ec
                   LEFT JOIN chart_of_accounts coa ON ec.account_id = coa.id
                   WHERE 1=1".to_string();
    if active_only {
        sql.push_str(" AND ec.is_active = TRUE");
    }
    sql.push_str(" ORDER BY ec.name ASC");

    sqlx::query_as::<_, ExpenseCategory>(&sql)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Database error: {}", e))
}

pub async fn create_expense_category(
    pool: &PgPool,
    req: CreateExpenseCategoryRequest,
) -> Result<ExpenseCategory, String> {
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM expense_categories WHERE code = $1)",
    )
    .bind(&req.code)
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    if exists {
        return Err(format!("Kode kategori '{}' sudah digunakan.", req.code));
    }

    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO expense_categories (id, code, name, account_id, description, is_active, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, TRUE, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    )
    .bind(id)
    .bind(&req.code.to_uppercase())
    .bind(&req.name)
    .bind(req.account_id)
    .bind(&req.description)
    .execute(pool)
    .await
    .map_err(|e| format!("Gagal menyimpan kategori pengeluaran: {}", e))?;

    let cat = sqlx::query_as::<_, ExpenseCategory>(
        "SELECT ec.*, coa.name as account_name, coa.code as account_code
         FROM expense_categories ec
         LEFT JOIN chart_of_accounts coa ON ec.account_id = coa.id
         WHERE ec.id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?;

    Ok(cat)
}

pub async fn update_expense_category(
    pool: &PgPool,
    id: Uuid,
    req: UpdateExpenseCategoryRequest,
) -> Result<ExpenseCategory, String> {
    sqlx::query(
        "UPDATE expense_categories SET
            name = $1,
            account_id = $2,
            description = $3,
            is_active = COALESCE($4, is_active),
            updated_at = CURRENT_TIMESTAMP
         WHERE id = $5",
    )
    .bind(&req.name)
    .bind(req.account_id)
    .bind(&req.description)
    .bind(req.is_active)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| format!("Gagal memperbarui kategori pengeluaran: {}", e))?;

    let cat = sqlx::query_as::<_, ExpenseCategory>(
        "SELECT ec.*, coa.name as account_name, coa.code as account_code
         FROM expense_categories ec
         LEFT JOIN chart_of_accounts coa ON ec.account_id = coa.id
         WHERE ec.id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?;

    Ok(cat)
}
