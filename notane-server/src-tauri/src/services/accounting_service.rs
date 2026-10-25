use crate::models::accounting::{
    AccountSummaryItem, BalanceSheetReport, ChartOfAccount, IncomeStatementReport,
    JournalEntry, JournalEntryDetail, JournalLineDetail, AccountingClosing,
    CreateClosingRequest, ReopenClosingRequest, ManualJournalRequest,
    CreateCoaRequest, UpdateCoaRequest, JournalQueryFilter, PaginatedJournalEntries,
    FundTransferRequest, FundTransferResponse, AccountingPeriod, CreatePeriodRequest,
    ReopenPeriodRequest, HardCloseRequest, PeriodQueryFilter, PaginatedAccountingPeriods,
    ClosingQueryFilter, PaginatedAccountingClosings,
};
use chrono::{Local, NaiveDate};
use rust_decimal::Decimal;
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub async fn list_chart_of_accounts(pool: &PgPool) -> Result<Vec<ChartOfAccount>, String> {
    sqlx::query_as::<_, ChartOfAccount>(
        "SELECT id, account_category_id, parent_id, code, name, description, is_header, is_active, is_system, created_at, updated_at
         FROM chart_of_accounts
         ORDER BY code ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))
}

/// Membuat akun baru di Chart of Accounts.
/// Validasi: kode akun harus unik, category_id harus antara 1-5.
pub async fn create_chart_of_account(
    pool: &PgPool,
    req: CreateCoaRequest,
) -> Result<ChartOfAccount, String> {
    // Validasi category
    if req.account_category_id < 1 || req.account_category_id > 5 {
        return Err("Kategori akun tidak valid. Pilih antara: 1=ASSET, 2=LIABILITY, 3=EQUITY, 4=REVENUE, 5=EXPENSE.".to_string());
    }

    let code = req.code.trim().to_string();
    let name = req.name.trim().to_string();

    if code.is_empty() {
        return Err("Kode akun tidak boleh kosong.".to_string());
    }
    if name.is_empty() {
        return Err("Nama akun tidak boleh kosong.".to_string());
    }

    // Cek duplikasi kode
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM chart_of_accounts WHERE code = $1)",
    )
    .bind(&code)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?;

    if exists {
        return Err(format!(
            "Kode akun '{}' sudah digunakan. Gunakan kode akun yang berbeda.",
            code
        ));
    }

    let is_header = req.is_header.unwrap_or(false);
    let new_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO chart_of_accounts 
         (id, account_category_id, parent_id, code, name, description, is_header, is_active, is_system, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, TRUE, FALSE, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
    )
    .bind(new_id)
    .bind(req.account_category_id)
    .bind(req.parent_id)
    .bind(&code)
    .bind(&name)
    .bind(&req.description)
    .bind(is_header)
    .execute(pool)
    .await
    .map_err(|e| format!("Gagal membuat akun: {}", e))?;

    // Ambil kembali data yang baru dibuat
    sqlx::query_as::<_, ChartOfAccount>(
        "SELECT id, account_category_id, parent_id, code, name, description, is_header, is_active, is_system, created_at, updated_at
         FROM chart_of_accounts
         WHERE id = $1",
    )
    .bind(new_id)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))
}

/// Memperbarui nama, deskripsi, dan status aktif sebuah akun COA.
/// Kode akun tidak dapat diubah untuk menjaga integritas referensial jurnal.
/// Akun sistem (`is_system = true`) TIDAK DAPAT dimodifikasi melalui API ini.
pub async fn update_chart_of_account(
    pool: &PgPool,
    id: Uuid,
    req: UpdateCoaRequest,
) -> Result<ChartOfAccount, String> {
    // === GUARD: Cek apakah ini akun sistem yang dilindungi ===
    let is_system: bool = sqlx::query_scalar(
        "SELECT is_system FROM chart_of_accounts WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?
    .ok_or_else(|| "Akun tidak ditemukan.".to_string())?;

    if is_system {
        return Err(
            "Akun ini adalah akun sistem bawaan yang dilindungi. \
             Akun sistem tidak dapat diubah karena terhubung langsung ke mesin pencatatan jurnal otomatis. \
             Hubungi administrator jika perlu penyesuaian."
            .to_string(),
        );
    }

    let name = req.name.trim().to_string();
    if name.is_empty() {
        return Err("Nama akun tidak boleh kosong.".to_string());
    }

    let affected = sqlx::query(
        "UPDATE chart_of_accounts 
         SET name = $1, description = $2, is_active = $3, updated_at = CURRENT_TIMESTAMP 
         WHERE id = $4 AND is_system = FALSE",
    )
    .bind(&name)
    .bind(&req.description)
    .bind(req.is_active)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| format!("Gagal memperbarui akun: {}", e))?;

    if affected.rows_affected() == 0 {
        return Err("Akun tidak dapat diperbarui. Akun mungkin terlindungi atau tidak ditemukan.".to_string());
    }

    sqlx::query_as::<_, ChartOfAccount>(
        "SELECT id, account_category_id, parent_id, code, name, description, is_header, is_active, is_system, created_at, updated_at
         FROM chart_of_accounts
         WHERE id = $1",
    )
    .bind(id)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))
}


pub async fn list_journal_entries(
    pool: &PgPool,
    filter: JournalQueryFilter,
    owner_id: Option<Uuid>,
) -> Result<PaginatedJournalEntries, String> {
    let mut where_clause = " WHERE 1=1".to_string();

    if let Some(uid) = owner_id {
        where_clause.push_str(&format!(" AND (je.created_by = '{uid}' OR je.updated_by = '{uid}')"));
    }

    if let Some(oid) = filter.outlet_id {
        where_clause.push_str(&format!(
            " AND EXISTS (SELECT 1 FROM journal_lines jl_f WHERE jl_f.journal_id = je.id AND jl_f.outlet_id = '{oid}')"
        ));
    }

    if let Some(s) = filter.start_date {
        where_clause.push_str(&format!(" AND je.journal_date >= '{s}'"));
    }
    if let Some(e) = filter.end_date {
        where_clause.push_str(&format!(" AND je.journal_date <= '{e}'"));
    }

    if let Some(ref q) = filter.search {
        let clean = q.trim().replace('\'', "''");
        if !clean.is_empty() {
            where_clause.push_str(&format!(
                " AND (je.journal_number ILIKE '%{clean}%' OR je.description ILIKE '%{clean}%' OR je.reference_type ILIKE '%{clean}%' OR EXISTS (
                    SELECT 1 FROM journal_lines jl_s
                    JOIN chart_of_accounts coa_s ON jl_s.account_id = coa_s.id
                    LEFT JOIN outlets o_s ON jl_s.outlet_id = o_s.id
                    WHERE jl_s.journal_id = je.id
                      AND (coa_s.code ILIKE '%{clean}%' OR coa_s.name ILIKE '%{clean}%' OR jl_s.memo ILIKE '%{clean}%' OR o_s.name ILIKE '%{clean}%')
                ))"
            ));
        }
    }

    let count_sql = format!(
        "SELECT COUNT(*)::bigint FROM journal_entries je {}",
        where_clause
    );
    let total_items: i64 = sqlx::query_scalar(&count_sql)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let sort_col = match filter.sort_by.as_deref() {
        Some("journal_number") => "je.journal_number",
        Some("reference_type") => "je.reference_type",
        Some("description") => "je.description",
        Some("total_debit") => "je.total_debit",
        Some("total_credit") => "je.total_credit",
        Some("created_at") => "je.created_at",
        _ => "je.journal_date",
    };
    let sort_dir = match filter.sort_dir.as_deref() {
        Some("asc") | Some("ASC") => "ASC",
        _ => "DESC",
    };

    let page = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(filter.limit.unwrap_or(10)).max(1);
    let total_pages = if total_items > 0 {
        ((total_items as f64) / (page_size as f64)).ceil() as i64
    } else {
        1
    };

    let offset = (page - 1) * page_size;

    let sql = format!(
        "SELECT je.* 
         FROM journal_entries je 
         {} 
         ORDER BY {} {}, je.created_at DESC 
         LIMIT {} OFFSET {}",
        where_clause, sort_col, sort_dir, page_size, offset
    );

    let rows = sqlx::query(&sql)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let mut result = Vec::new();

    for r in rows {
        let entry_id: Uuid = r.get("id");
        let entry = JournalEntry {
            id: entry_id,
            journal_number: r.get("journal_number"),
            journal_date: r.get("journal_date"),
            reference_type: r.get("reference_type"),
            reference_id: r.get("reference_id"),
            description: r.get("description"),
            total_debit: r.get("total_debit"),
            total_credit: r.get("total_credit"),
            is_closing_entry: r.get("is_closing_entry"),
            created_at: r.get("created_at"),
        };

        let lines = sqlx::query(
            "SELECT jl.id, jl.account_id, coa.code as account_code, coa.name as account_name,
                   jl.debit, jl.credit, jl.memo, jl.outlet_id, o.name as outlet_name
            FROM journal_lines jl
            JOIN chart_of_accounts coa ON jl.account_id = coa.id
            LEFT JOIN outlets o ON jl.outlet_id = o.id
            WHERE jl.journal_id = $1
            ORDER BY jl.debit DESC",
        )
        .bind(entry_id)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

        let line_details = lines
            .into_iter()
            .map(|l| JournalLineDetail {
                id: l.get("id"),
                account_id: l.get("account_id"),
                account_code: l.get("account_code"),
                account_name: l.get("account_name"),
                debit: l.get("debit"),
                credit: l.get("credit"),
                memo: l.get("memo"),
                outlet_id: l.get("outlet_id"),
                outlet_name: l.get("outlet_name"),
            })
            .collect();

        result.push(JournalEntryDetail {
            entry,
            lines: line_details,
        });
    }

    Ok(PaginatedJournalEntries {
        data: result,
        total_items,
        total_pages,
        page,
        page_size,
    })
}

pub async fn get_income_statement(
    pool: &PgPool,
    start_date: NaiveDate,
    end_date: NaiveDate,
    outlet_id: Option<Uuid>,
) -> Result<IncomeStatementReport, String> {
    let outlet_name: Option<String> = if let Some(oid) = outlet_id {
        sqlx::query_scalar("SELECT name FROM outlets WHERE id = $1")
            .bind(oid)
            .fetch_optional(pool)
            .await
            .unwrap_or(None)
    } else {
        None
    };

    let outlet_filter_sql = if let Some(oid) = outlet_id {
        format!(" AND jl.outlet_id = '{oid}'")
    } else {
        "".to_string()
    };

    let revenue_query = format!(
        "SELECT coa.code, coa.name,
               COALESCE(SUM(jl.credit - jl.debit), 0) as amount
        FROM journal_lines jl
        JOIN journal_entries je ON jl.journal_id = je.id
        JOIN chart_of_accounts coa ON jl.account_id = coa.id
        JOIN account_categories ac ON coa.account_category_id = ac.id
        WHERE ac.name = 'REVENUE' 
          AND je.journal_date >= $1 
          AND je.journal_date <= $2
          AND je.is_closing_entry = FALSE
          {}
        GROUP BY coa.code, coa.name
        ORDER BY coa.code ASC",
        outlet_filter_sql
    );

    let revenue_rows = sqlx::query(&revenue_query)
        .bind(start_date)
        .bind(end_date)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Gagal memuat pendapatan: {}", e))?;

    let mut revenue_details = Vec::new();
    let mut total_revenue = Decimal::ZERO;
    for r in revenue_rows {
        let code: String = r.get("code");
        let name: String = r.get("name");
        let amt: Decimal = r.get("amount");
        total_revenue += amt;
        revenue_details.push(AccountSummaryItem {
            code,
            name,
            amount: amt,
        });
    }

    let expense_query = format!(
        "SELECT coa.code, coa.name,
               COALESCE(SUM(jl.debit - jl.credit), 0) as amount
        FROM journal_lines jl
        JOIN journal_entries je ON jl.journal_id = je.id
        JOIN chart_of_accounts coa ON jl.account_id = coa.id
        JOIN account_categories ac ON coa.account_category_id = ac.id
        WHERE ac.name = 'EXPENSE' 
          AND je.journal_date >= $1 
          AND je.journal_date <= $2
          AND je.is_closing_entry = FALSE
          {}
        GROUP BY coa.code, coa.name
        ORDER BY coa.code ASC",
        outlet_filter_sql
    );

    let expense_rows = sqlx::query(&expense_query)
        .bind(start_date)
        .bind(end_date)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Gagal memuat beban: {}", e))?;

    let mut expense_details = Vec::new();
    let mut total_cogs = Decimal::ZERO;
    let mut total_operating_expenses = Decimal::ZERO;

    for exp in expense_rows {
        let code: String = exp.get("code");
        let name: String = exp.get("name");
        let amt: Decimal = exp.get("amount");
        if code.starts_with("51") {
            total_cogs += amt;
        } else {
            total_operating_expenses += amt;
        }
        expense_details.push(AccountSummaryItem {
            code,
            name,
            amount: amt,
        });
    }

    let gross_profit = total_revenue - total_cogs;
    let net_profit = gross_profit - total_operating_expenses;

    Ok(IncomeStatementReport {
        start_date,
        end_date,
        outlet_id,
        outlet_name,
        total_revenue,
        total_cogs,
        gross_profit,
        total_operating_expenses,
        net_profit,
        revenue_details,
        expense_details,
    })
}

pub async fn get_balance_sheet(
    pool: &PgPool,
    as_of_date: NaiveDate,
    outlet_id: Option<Uuid>,
) -> Result<BalanceSheetReport, String> {
    let outlet_name: Option<String> = if let Some(oid) = outlet_id {
        sqlx::query_scalar("SELECT name FROM outlets WHERE id = $1")
            .bind(oid)
            .fetch_optional(pool)
            .await
            .unwrap_or(None)
    } else {
        None
    };

    let outlet_filter_sql = if let Some(oid) = outlet_id {
        format!(" AND jl.outlet_id = '{oid}'")
    } else {
        "".to_string()
    };

    // 1. Assets (Category 1: ASSET, Normal Balance: DEBIT)
    let asset_query = format!(
        "SELECT coa.code, coa.name,
               COALESCE(SUM(jl.debit - jl.credit), 0) as amount
        FROM journal_lines jl
        JOIN journal_entries je ON jl.journal_id = je.id
        JOIN chart_of_accounts coa ON jl.account_id = coa.id
        JOIN account_categories ac ON coa.account_category_id = ac.id
        WHERE ac.name = 'ASSET' 
          AND je.journal_date <= $1
          {}
        GROUP BY coa.code, coa.name
        HAVING COALESCE(SUM(jl.debit - jl.credit), 0) != 0
        ORDER BY coa.code ASC",
        outlet_filter_sql
    );

    let asset_rows = sqlx::query(&asset_query)
        .bind(as_of_date)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Gagal memuat posisi aset: {}", e))?;

    let mut asset_details = Vec::new();
    let mut current_assets = Decimal::ZERO;
    let mut non_current_assets = Decimal::ZERO;
    let mut rak_equity_items = Vec::new();

    for r in asset_rows {
        let code: String = r.get("code");
        let name: String = r.get("name");
        let amt: Decimal = r.get("amount");

        // Perlakuan Rekening Antar-Kantor (RAK / 19xx)
        let is_rak = code.starts_with("19");

        if is_rak && amt < Decimal::ZERO {
            // Dalam akuntansi kantor pusat & cabang, saldo kredit pada Rekening Antar-Kantor (RAK)
            // merupakan modal kerja / ekuitas penempatan dari Kantor Pusat ke cabang penerima.
            // Nilai ini tidak boleh mengurangi total aset sebagai saldo minus, melainkan
            // diakui dan disajikan pada sisi Ekuitas (Pasiva) cabang.
            let display_name = if name.contains("RAK") || name.contains("Rekening Antar-Kantor") {
                "Rekening Antar-Kantor (Modal / Penempatan dari Kantor Pusat)".to_string()
            } else {
                format!("{} (Modal dari Kantor Pusat)", name)
            };
            rak_equity_items.push(AccountSummaryItem {
                code,
                name: display_name,
                amount: amt.abs(),
            });
        } else {
            if code.starts_with("11") || code.starts_with("12") {
                current_assets += amt;
            } else {
                non_current_assets += amt;
            }

            asset_details.push(AccountSummaryItem {
                code,
                name,
                amount: amt,
            });
        }
    }

    let total_assets = current_assets + non_current_assets;

    // 2. Liabilities (Category 2: LIABILITY, Normal Balance: CREDIT)
    let liability_query = format!(
        "SELECT coa.code, coa.name,
               COALESCE(SUM(jl.credit - jl.debit), 0) as amount
        FROM journal_lines jl
        JOIN journal_entries je ON jl.journal_id = je.id
        JOIN chart_of_accounts coa ON jl.account_id = coa.id
        JOIN account_categories ac ON coa.account_category_id = ac.id
        WHERE ac.name = 'LIABILITY' 
          AND je.journal_date <= $1
          {}
        GROUP BY coa.code, coa.name
        HAVING COALESCE(SUM(jl.credit - jl.debit), 0) != 0
        ORDER BY coa.code ASC",
        outlet_filter_sql
    );

    let liability_rows = sqlx::query(&liability_query)
        .bind(as_of_date)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Gagal memuat posisi kewajiban: {}", e))?;

    let mut liability_details = Vec::new();
    let mut total_liabilities = Decimal::ZERO;

    for r in liability_rows {
        let code: String = r.get("code");
        let name: String = r.get("name");
        let amt: Decimal = r.get("amount");
        total_liabilities += amt;
        liability_details.push(AccountSummaryItem {
            code,
            name,
            amount: amt,
        });
    }

    // 3. Equity (Category 3: EQUITY, Normal Balance: CREDIT)
    let equity_query = format!(
        "SELECT coa.code, coa.name,
               COALESCE(SUM(jl.credit - jl.debit), 0) as amount
        FROM journal_lines jl
        JOIN journal_entries je ON jl.journal_id = je.id
        JOIN chart_of_accounts coa ON jl.account_id = coa.id
        JOIN account_categories ac ON coa.account_category_id = ac.id
        WHERE ac.name = 'EQUITY' 
          AND je.journal_date <= $1
          {}
        GROUP BY coa.code, coa.name
        HAVING COALESCE(SUM(jl.credit - jl.debit), 0) != 0
        ORDER BY coa.code ASC",
        outlet_filter_sql
    );

    let equity_rows = sqlx::query(&equity_query)
        .bind(as_of_date)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Gagal memuat posisi ekuitas: {}", e))?;

    let mut equity_details = Vec::new();
    let mut recorded_equity = Decimal::ZERO;

    for r in equity_rows {
        let code: String = r.get("code");
        let name: String = r.get("name");
        let amt: Decimal = r.get("amount");
        recorded_equity += amt;
        equity_details.push(AccountSummaryItem {
            code,
            name,
            amount: amt,
        });
    }

    // Masukkan ekuitas RAK dari penempatan kantor pusat ke cabang
    for rak_item in rak_equity_items {
        recorded_equity += rak_item.amount;
        equity_details.push(rak_item);
    }

    // 4. Hitung Laba/Rugi Belum Ditutup (Unclosed Current Earnings to date)
    // Sesuai standar neraca: Saldo pendapatan - saldo beban yang belum ditutup
    let unclosed_rev_query = format!(
        "SELECT COALESCE(SUM(jl.credit - jl.debit), 0) as amount
         FROM journal_lines jl
         JOIN journal_entries je ON jl.journal_id = je.id
         JOIN chart_of_accounts coa ON jl.account_id = coa.id
         JOIN account_categories ac ON coa.account_category_id = ac.id
         WHERE ac.name = 'REVENUE' 
           AND je.journal_date <= $1
           AND je.is_closing_entry = FALSE
           {}",
        outlet_filter_sql
    );

    let unclosed_rev: Decimal = sqlx::query_scalar(&unclosed_rev_query)
        .bind(as_of_date)
        .fetch_one(pool)
        .await
        .unwrap_or(Decimal::ZERO);

    let unclosed_exp_query = format!(
        "SELECT COALESCE(SUM(jl.debit - jl.credit), 0) as amount
         FROM journal_lines jl
         JOIN journal_entries je ON jl.journal_id = je.id
         JOIN chart_of_accounts coa ON jl.account_id = coa.id
         JOIN account_categories ac ON coa.account_category_id = ac.id
         WHERE ac.name = 'EXPENSE' 
           AND je.journal_date <= $1
           AND je.is_closing_entry = FALSE
           {}",
        outlet_filter_sql
    );

    let unclosed_exp: Decimal = sqlx::query_scalar(&unclosed_exp_query)
        .bind(as_of_date)
        .fetch_one(pool)
        .await
        .unwrap_or(Decimal::ZERO);

    let current_period_earnings = unclosed_rev - unclosed_exp;

    if current_period_earnings != Decimal::ZERO {
        equity_details.push(AccountSummaryItem {
            code: "3301".to_string(),
            name: "Laba (Rugi) Periode Berjalan".to_string(),
            amount: current_period_earnings,
        });
    }

    let total_equity = recorded_equity + current_period_earnings;
    let total_liabilities_and_equity = total_liabilities + total_equity;
    let is_balanced = total_assets == total_liabilities_and_equity;

    Ok(BalanceSheetReport {
        as_of_date,
        outlet_id,
        outlet_name,
        total_assets,
        current_assets,
        non_current_assets,
        total_liabilities,
        total_equity,
        total_liabilities_and_equity,
        is_balanced,
        asset_details,
        liability_details,
        equity_details,
    })
}

// ============================================================================
// SISTEM TUTUP BUKU & PROTEKSI PERIODE TERTUTUP (PERIOD CLOSING PROTECTION)
// ============================================================================

pub async fn check_transaction_date_allowed(
    pool: &PgPool,
    tx_date: NaiveDate,
    outlet_id: Option<Uuid>,
) -> Result<(), String> {
    // 1. Cek periode penguncian Soft Close (accounting_periods)
    let closed_period = sqlx::query(
        "SELECT name, start_date, end_date
         FROM accounting_periods
         WHERE is_closed = TRUE
           AND $1 >= start_date AND $1 <= end_date
         LIMIT 1",
    )
    .bind(tx_date)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Gagal memvalidasi status periode akuntansi: {}", e))?;

    if let Some(row) = closed_period {
        let name: String = row.get("name");
        let s_date: NaiveDate = row.get("start_date");
        let e_date: NaiveDate = row.get("end_date");
        return Err(format!(
            "Transaksi ditolak: Tanggal transaksi ({}) berada dalam periode akuntansi yang terkunci / ditutup ({} - {} s/d {}). Sesuai kaidah akuntansi standar, transaksi pada periode yang telah dikunci tidak dapat ditambah, diubah, atau dihapus.",
            tx_date.format("%d-%m-%Y"),
            name,
            s_date.format("%d-%m-%Y"),
            e_date.format("%d-%m-%Y")
        ));
    }

    // 2. Cek histori penutupan buku (accounting_closings)
    let closed_record = sqlx::query(
        "SELECT closing_number, period_end 
         FROM accounting_closings 
         WHERE status = 'CLOSED' 
           AND period_end >= $1 
           AND ($2::UUID IS NULL OR outlet_id IS NULL OR outlet_id = $2)
         ORDER BY period_end DESC 
         LIMIT 1",
    )
    .bind(tx_date)
    .bind(outlet_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Gagal memvalidasi status tutup buku: {}", e))?;

    if let Some(row) = closed_record {
        let closing_num: String = row.get("closing_number");
        let p_end: NaiveDate = row.get("period_end");
        return Err(format!(
            "Transaksi ditolak: Tanggal transaksi ({}) berada dalam periode akuntansi yang telah DITUTUP BUKU ({}, batas akhir: {}). Sesuai kaidah akuntansi standar, transaksi pada periode yang telah ditutup tidak dapat dibuat, diubah, atau dihapus.",
            tx_date.format("%d-%m-%Y"),
            closing_num,
            p_end.format("%d-%m-%Y")
        ));
    }

    Ok(())
}

pub async fn perform_accounting_closing(
    pool: &PgPool,
    req: CreateClosingRequest,
    user_id: Option<Uuid>,
) -> Result<AccountingClosing, String> {
    if req.period_start > req.period_end {
        return Err("Tanggal awal periode tidak boleh lebih besar dari tanggal akhir periode.".to_string());
    }

    // 1. Cek apakah ada periode tutup buku CLOSED yang bertabrakan (overlap)
    let overlap_closed = sqlx::query(
        "SELECT closing_number, period_start, period_end 
         FROM accounting_closings 
         WHERE status = 'CLOSED' 
           AND ($1 <= period_end AND $2 >= period_start)
           AND ($3::UUID IS NULL OR outlet_id IS NULL OR outlet_id = $3)
         LIMIT 1",
    )
    .bind(req.period_start)
    .bind(req.period_end)
    .bind(req.outlet_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Gagal memeriksa status penutupan buku: {}", e))?;

    if let Some(r) = overlap_closed {
        let num: String = r.get("closing_number");
        let pstart: NaiveDate = r.get("period_start");
        let pend: NaiveDate = r.get("period_end");
        return Err(format!(
            "Periode yang dipilih bertabrakan dengan periode penutupan buku {} s/d {} (Dokumen: {}).",
            pstart.format("%d-%m-%Y"),
            pend.format("%d-%m-%Y"),
            num
        ));
    }

    // 2. Cek apakah ada periode di masa sesudahnya yang sudah ditutup (tutup buku harus berurutan kronologis)
    let future_closed = sqlx::query(
        "SELECT closing_number, period_start 
         FROM accounting_closings 
         WHERE status = 'CLOSED' 
           AND period_start > $1
           AND ($2::UUID IS NULL OR outlet_id IS NULL OR outlet_id = $2)
         ORDER BY period_start ASC
         LIMIT 1",
    )
    .bind(req.period_end)
    .bind(req.outlet_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Gagal memeriksa urutan penutupan buku: {}", e))?;

    if let Some(r) = future_closed {
        let num: String = r.get("closing_number");
        let pstart: NaiveDate = r.get("period_start");
        return Err(format!(
            "Tidak dapat menutup periode hingga {} karena periode sesudahnya (mulai {}) telah ditutup buku (Dokumen: {}).",
            req.period_end.format("%d-%m-%Y"),
            pstart.format("%d-%m-%Y"),
            num
        ));
    }

    let mut tx = pool.begin().await.map_err(|e| format!("Gagal memulai transaksi DB: {}", e))?;

    let outlet_filter_sql = if let Some(oid) = req.outlet_id {
        format!(" AND jl.outlet_id = '{oid}'")
    } else {
        "".to_string()
    };

    // 1. Ambil seluruh akun Pendapatan yang memiliki saldo kredit / debet
    let rev_query = format!(
        "SELECT coa.id, coa.code, coa.name, COALESCE(SUM(jl.credit - jl.debit), 0) as balance
         FROM journal_lines jl
         JOIN journal_entries je ON jl.journal_id = je.id
         JOIN chart_of_accounts coa ON jl.account_id = coa.id
         JOIN account_categories ac ON coa.account_category_id = ac.id
         WHERE ac.name = 'REVENUE' 
           AND je.journal_date >= $1 AND je.journal_date <= $2
           AND je.reference_type != 'CLOSING_ENTRY'
           AND je.is_closing_entry = FALSE
           {}
         GROUP BY coa.id, coa.code, coa.name
         HAVING COALESCE(SUM(jl.credit - jl.debit), 0) != 0",
        outlet_filter_sql
    );

    let rev_rows = sqlx::query(&rev_query)
        .bind(req.period_start)
        .bind(req.period_end)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| format!("Gagal menghitung saldo pendapatan penutupan: {}", e))?;

    // 2. Ambil seluruh akun Beban yang memiliki saldo debet / kredit
    let exp_query = format!(
        "SELECT coa.id, coa.code, coa.name, COALESCE(SUM(jl.debit - jl.credit), 0) as balance
         FROM journal_lines jl
         JOIN journal_entries je ON jl.journal_id = je.id
         JOIN chart_of_accounts coa ON jl.account_id = coa.id
         JOIN account_categories ac ON coa.account_category_id = ac.id
         WHERE ac.name = 'EXPENSE' 
           AND je.journal_date >= $1 AND je.journal_date <= $2
           AND je.reference_type != 'CLOSING_ENTRY'
           AND je.is_closing_entry = FALSE
           {}
         GROUP BY coa.id, coa.code, coa.name
         HAVING COALESCE(SUM(jl.debit - jl.credit), 0) != 0",
        outlet_filter_sql
    );

    let exp_rows = sqlx::query(&exp_query)
        .bind(req.period_start)
        .bind(req.period_end)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| format!("Gagal menghitung saldo beban penutupan: {}", e))?;

    let mut total_revenue = Decimal::ZERO;
    for r in &rev_rows {
        total_revenue += r.get::<Decimal, _>("balance");
    }

    let mut total_expense = Decimal::ZERO;
    for r in &exp_rows {
        total_expense += r.get::<Decimal, _>("balance");
    }

    let net_income = total_revenue - total_expense;

    // Cari akun penutup 3901 (Ikhtisar Laba Rugi) dan 3201 (Laba Ditahan)
    let acc_income_summary_id: Uuid = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '3901'")
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| "Akun 3901 (Ikhtisar Laba Rugi) tidak ditemukan dalam sistem CoA")?;

    let acc_retained_earnings_id: Uuid = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '3201'")
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| "Akun 3201 (Laba Ditahan) tidak ditemukan dalam sistem CoA")?;

    let closing_id = Uuid::new_v4();
    let closing_number = format!("CLS-{}", Local::now().format("%Y%m%d%H%M%S"));
    let journal_id = Uuid::new_v4();
    let journal_number = format!("JRN-CLS-{}", Local::now().format("%Y%m%d%H%M%S"));

    // Buat Jurnal Penutup (Closing Entries)
    let mut raw_lines: Vec<(Uuid, Decimal, Decimal, String)> = Vec::new();

    // A. Menutup Akun Pendapatan ke Ikhtisar L/R
    for r in &rev_rows {
        let acc_id: Uuid = r.get("id");
        let bal: Decimal = r.get("balance");
        if bal > Decimal::ZERO {
            // Debet akun Pendapatan untuk menutup saldo kredit normal
            raw_lines.push((acc_id, bal, Decimal::ZERO, "Penutupan Akun Pendapatan ke Ikhtisar L/R".to_string()));
        } else if bal < Decimal::ZERO {
            // Kredit akun Kontra Pendapatan (misal potongan diskon) untuk menutup saldo debet
            raw_lines.push((acc_id, Decimal::ZERO, bal.abs(), "Penutupan Kontra Pendapatan ke Ikhtisar L/R".to_string()));
        }
    }
    if total_revenue > Decimal::ZERO {
        // Kredit Ikhtisar L/R atas pendapatan bersih
        raw_lines.push((acc_income_summary_id, Decimal::ZERO, total_revenue, "Kredit Total Pendapatan ke Ikhtisar L/R".to_string()));
    } else if total_revenue < Decimal::ZERO {
        // Debet Ikhtisar L/R jika pendapatan bersih defisit
        raw_lines.push((acc_income_summary_id, total_revenue.abs(), Decimal::ZERO, "Debet Total Pengurang Pendapatan ke Ikhtisar L/R".to_string()));
    }

    // B. Menutup Akun Beban ke Ikhtisar L/R
    if total_expense > Decimal::ZERO {
        // Debet Ikhtisar L/R atas total beban normal
        raw_lines.push((acc_income_summary_id, total_expense, Decimal::ZERO, "Debet Total Beban ke Ikhtisar L/R".to_string()));
    } else if total_expense < Decimal::ZERO {
        // Kredit Ikhtisar L/R jika beban bernilai negatif bersih (surplus penyesuaian)
        raw_lines.push((acc_income_summary_id, Decimal::ZERO, total_expense.abs(), "Kredit Surplus Beban ke Ikhtisar L/R".to_string()));
    }
    for r in &exp_rows {
        let acc_id: Uuid = r.get("id");
        let bal: Decimal = r.get("balance");
        if bal > Decimal::ZERO {
            // Kredit akun Beban untuk menutup saldo debet normal
            raw_lines.push((acc_id, Decimal::ZERO, bal, "Penutupan Akun Beban ke Ikhtisar L/R".to_string()));
        } else if bal < Decimal::ZERO {
            // Debet akun Beban jika saldo abnormal kredit (misal surplus persediaan)
            raw_lines.push((acc_id, bal.abs(), Decimal::ZERO, "Penutupan Saldo Kredit Beban ke Ikhtisar L/R".to_string()));
        }
    }

    // C. Menutup Ikhtisar L/R ke Laba Ditahan (Retained Earnings)
    if net_income > Decimal::ZERO {
        // Laba bersih: Debet Ikhtisar L/R, Kredit Laba Ditahan
        raw_lines.push((acc_income_summary_id, net_income, Decimal::ZERO, "Pemindahan Laba Bersih ke Laba Ditahan".to_string()));
        raw_lines.push((acc_retained_earnings_id, Decimal::ZERO, net_income, "Penambahan Laba Ditahan dari Laba Periode".to_string()));
    } else if net_income < Decimal::ZERO {
        // Rugi bersih: Debet Laba Ditahan, Kredit Ikhtisar L/R
        raw_lines.push((acc_retained_earnings_id, net_income.abs(), Decimal::ZERO, "Pembebanan Rugi Periode ke Laba Ditahan".to_string()));
        raw_lines.push((acc_income_summary_id, Decimal::ZERO, net_income.abs(), "Penutupan Saldo Rugi Ikhtisar L/R".to_string()));
    }

    // Validasi & filter ketat:
    // Pastikan setiap baris mematuhi check constraint: (debit > 0 AND credit == 0) OR (debit == 0 AND credit > 0)
    let mut journal_lines_to_insert: Vec<(Uuid, Decimal, Decimal, String)> = Vec::new();
    let mut tot_debit = Decimal::ZERO;
    let mut tot_credit = Decimal::ZERO;

    for (acc, d, c, memo) in raw_lines {
        let d_val = d.round_dp(4);
        let c_val = c.round_dp(4);
        if d_val > Decimal::ZERO && c_val == Decimal::ZERO {
            tot_debit += d_val;
            journal_lines_to_insert.push((acc, d_val, Decimal::ZERO, memo));
        } else if c_val > Decimal::ZERO && d_val == Decimal::ZERO {
            tot_credit += c_val;
            journal_lines_to_insert.push((acc, Decimal::ZERO, c_val, memo));
        }
    }

    // Simpan Jurnal Penutup jika ada akun nominal yang ditutup dan seimbang
    let final_journal_id = if tot_debit > Decimal::ZERO && tot_debit == tot_credit {
        sqlx::query(
            "INSERT INTO journal_entries (
                id, journal_number, journal_date, reference_type, reference_id, description,
                total_debit, total_credit, is_closing_entry, created_at, updated_at, created_by, updated_by
            ) VALUES (
                $1, $2, $3, 'CLOSING_ENTRY', $4, $5, $6, $7, TRUE, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $8, $8
            )",
        )
        .bind(journal_id)
        .bind(&journal_number)
        .bind(req.period_end)
        .bind(closing_id)
        .bind(format!("Jurnal Penutup Buku Periode {} s/d {}", req.period_start, req.period_end))
        .bind(tot_debit)
        .bind(tot_credit)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal membuat header jurnal penutup: {}", e))?;

        for (acc, d, c, memo) in journal_lines_to_insert {
            sqlx::query(
                "INSERT INTO journal_lines (
                    id, journal_id, account_id, debit, credit, memo, outlet_id, created_at, updated_at, created_by, updated_by
                ) VALUES (
                    gen_random_uuid(), $1, $2, $3, $4, $5, $6, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $7, $7
                )",
            )
            .bind(journal_id)
            .bind(acc)
            .bind(d)
            .bind(c)
            .bind(memo)
            .bind(req.outlet_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Gagal mencatat baris jurnal penutup: {}", e))?;
        }

        Some(journal_id)
    } else {
        None
    };

    let today = Local::now().date_naive();

    sqlx::query(
        "INSERT INTO accounting_closings (
            id, closing_number, period_start, period_end, closing_date, outlet_id,
            total_revenue, total_expense, net_income, journal_entry_id, status, notes, created_by
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'CLOSED', $11, $12
        )",
    )
    .bind(closing_id)
    .bind(&closing_number)
    .bind(req.period_start)
    .bind(req.period_end)
    .bind(today)
    .bind(req.outlet_id)
    .bind(total_revenue)
    .bind(total_expense)
    .bind(net_income)
    .bind(final_journal_id)
    .bind(&req.notes)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal menyimpan riwayat tutup buku: {}", e))?;

    tx.commit().await.map_err(|e| format!("Gagal commit penutupan buku: {}", e))?;

    let created_row = sqlx::query(
        "SELECT ac.*, o.name as outlet_name, u.full_name as creator_name, je.journal_number 
         FROM accounting_closings ac 
         LEFT JOIN outlets o ON ac.outlet_id = o.id 
         LEFT JOIN users u ON ac.created_by = u.id 
         LEFT JOIN journal_entries je ON ac.journal_entry_id = je.id 
         WHERE ac.id = $1",
    )
    .bind(closing_id)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Gagal mengambil data tutup buku: {}", e))?;

    Ok(AccountingClosing {
        id: created_row.get("id"),
        closing_number: created_row.get("closing_number"),
        period_start: created_row.get("period_start"),
        period_end: created_row.get("period_end"),
        closing_date: created_row.get("closing_date"),
        outlet_id: created_row.get("outlet_id"),
        outlet_name: created_row.get("outlet_name"),
        total_revenue: created_row.get("total_revenue"),
        total_expense: created_row.get("total_expense"),
        net_income: created_row.get("net_income"),
        journal_entry_id: created_row.get("journal_entry_id"),
        journal_number: created_row.get("journal_number"),
        status: created_row.get("status"),
        notes: created_row.get("notes"),
        reopened_at: created_row.get("reopened_at"),
        reopened_by: created_row.get("reopened_by"),
        reopen_reason: created_row.get("reopen_reason"),
        created_at: created_row.get("created_at"),
        created_by: created_row.get("created_by"),
        creator_name: created_row.get("creator_name"),
    })
}

pub async fn reopen_accounting_closing(
    pool: &PgPool,
    closing_id: Uuid,
    req: ReopenClosingRequest,
    user_id: Option<Uuid>,
) -> Result<AccountingClosing, String> {
    if req.reason.trim().is_empty() {
        return Err("Alasan pembukaan kembali buku (reopen reason) wajib diisi untuk jejak audit standar akuntansi.".to_string());
    }

    let existing = sqlx::query(
        "SELECT * FROM accounting_closings WHERE id = $1",
    )
    .bind(closing_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?
    .ok_or_else(|| "Dokumen penutupan buku tidak ditemukan.".to_string())?;

    let status: String = existing.get("status");
    if status == "REOPENED" {
        return Err("Periode pembukuan ini sudah dalam status terbuka (REOPENED).".to_string());
    }

    let mut tx = pool.begin().await.map_err(|e| format!("Gagal memulai transaksi DB: {}", e))?;

    // Jika ada jurnal penutup terkait, hapus/batalkan jurnal tersebut
    let jrn_id: Option<Uuid> = existing.get("journal_entry_id");
    if let Some(jid) = jrn_id {
        sqlx::query("DELETE FROM journal_entries WHERE id = $1")
            .bind(jid)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Gagal membatalkan jurnal penutup: {}", e))?;
    }

    sqlx::query(
        "UPDATE accounting_closings 
         SET status = 'REOPENED', 
             reopened_at = CURRENT_TIMESTAMP, 
             reopened_by = $1, 
             reopen_reason = $2,
             updated_at = CURRENT_TIMESTAMP
         WHERE id = $3",
    )
    .bind(user_id)
    .bind(&req.reason)
    .bind(closing_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal memperbarui status buka kembali: {}", e))?;

    tx.commit().await.map_err(|e| format!("Gagal commit reopen: {}", e))?;

    let updated_row = sqlx::query(
        "SELECT ac.*, o.name as outlet_name, u.full_name as creator_name 
         FROM accounting_closings ac 
         LEFT JOIN outlets o ON ac.outlet_id = o.id 
         LEFT JOIN users u ON ac.created_by = u.id 
         WHERE ac.id = $1",
    )
    .bind(closing_id)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Gagal mengambil data tutup buku: {}", e))?;

    Ok(AccountingClosing {
        id: updated_row.get("id"),
        closing_number: updated_row.get("closing_number"),
        period_start: updated_row.get("period_start"),
        period_end: updated_row.get("period_end"),
        closing_date: updated_row.get("closing_date"),
        outlet_id: updated_row.get("outlet_id"),
        outlet_name: updated_row.get("outlet_name"),
        total_revenue: updated_row.get("total_revenue"),
        total_expense: updated_row.get("total_expense"),
        net_income: updated_row.get("net_income"),
        journal_entry_id: None,
        journal_number: None,
        status: updated_row.get("status"),
        notes: updated_row.get("notes"),
        reopened_at: updated_row.get("reopened_at"),
        reopened_by: updated_row.get("reopened_by"),
        reopen_reason: updated_row.get("reopen_reason"),
        created_at: updated_row.get("created_at"),
        created_by: updated_row.get("created_by"),
        creator_name: updated_row.get("creator_name"),
    })
}

pub async fn list_accounting_closings(
    pool: &PgPool,
    filter: ClosingQueryFilter,
) -> Result<PaginatedAccountingClosings, String> {
    let mut where_clause = " WHERE 1=1".to_string();

    if let Some(oid) = filter.outlet_id {
        where_clause.push_str(&format!(" AND (ac.outlet_id = '{oid}' OR ac.outlet_id IS NULL)"));
    }

    if let Some(ref st) = filter.status {
        let st_clean = st.trim().to_uppercase();
        if !st_clean.is_empty() && st_clean != "ALL" {
            let clean = st_clean.replace('\'', "''");
            where_clause.push_str(&format!(" AND ac.status = '{clean}'"));
        }
    }

    if let Some(ref q) = filter.search {
        let clean = q.trim().replace('\'', "''");
        if !clean.is_empty() {
            where_clause.push_str(&format!(
                " AND (ac.closing_number ILIKE '%{clean}%' OR ac.notes ILIKE '%{clean}%' OR ac.reopen_reason ILIKE '%{clean}%' OR je.journal_number ILIKE '%{clean}%' OR o.name ILIKE '%{clean}%' OR u.full_name ILIKE '%{clean}%')"
            ));
        }
    }

    let count_sql = format!(
        "SELECT COUNT(*)::bigint 
         FROM accounting_closings ac 
         LEFT JOIN outlets o ON ac.outlet_id = o.id 
         LEFT JOIN users u ON ac.created_by = u.id 
         LEFT JOIN journal_entries je ON ac.journal_entry_id = je.id 
         {}",
        where_clause
    );
    let total_items: i64 = sqlx::query_scalar(&count_sql)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let sort_col = match filter.sort_by.as_deref() {
        Some("closing_number") => "ac.closing_number",
        Some("period_start") => "ac.period_start",
        Some("period_end") => "ac.period_end",
        Some("closing_date") => "ac.closing_date",
        Some("net_income") => "ac.net_income",
        Some("status") => "ac.status",
        Some("created_at") => "ac.created_at",
        _ => "ac.period_end",
    };
    let sort_dir = match filter.sort_dir.as_deref() {
        Some("asc") | Some("ASC") => "ASC",
        _ => "DESC",
    };

    let page = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(filter.limit.unwrap_or(10)).max(1);
    let total_pages = if total_items > 0 {
        ((total_items as f64) / (page_size as f64)).ceil() as i64
    } else {
        1
    };
    let offset = (page - 1) * page_size;

    let sql = format!(
        "SELECT ac.*, o.name as outlet_name, u.full_name as creator_name, je.journal_number 
         FROM accounting_closings ac 
         LEFT JOIN outlets o ON ac.outlet_id = o.id 
         LEFT JOIN users u ON ac.created_by = u.id 
         LEFT JOIN journal_entries je ON ac.journal_entry_id = je.id 
         {} 
         ORDER BY {} {}, ac.created_at DESC 
         LIMIT {} OFFSET {}",
        where_clause, sort_col, sort_dir, page_size, offset
    );

    let rows = sqlx::query(&sql)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Database error: {}", e))?;

    let list = rows
        .into_iter()
        .map(|r| AccountingClosing {
            id: r.get("id"),
            closing_number: r.get("closing_number"),
            period_start: r.get("period_start"),
            period_end: r.get("period_end"),
            closing_date: r.get("closing_date"),
            outlet_id: r.get("outlet_id"),
            outlet_name: r.get("outlet_name"),
            total_revenue: r.get("total_revenue"),
            total_expense: r.get("total_expense"),
            net_income: r.get("net_income"),
            journal_entry_id: r.get("journal_entry_id"),
            journal_number: r.get("journal_number"),
            status: r.get("status"),
            notes: r.get("notes"),
            reopened_at: r.get("reopened_at"),
            reopened_by: r.get("reopened_by"),
            reopen_reason: r.get("reopen_reason"),
            created_at: r.get("created_at"),
            created_by: r.get("created_by"),
            creator_name: r.get("creator_name"),
        })
        .collect();

    Ok(PaginatedAccountingClosings {
        data: list,
        total_items,
        total_pages,
        page,
        page_size,
    })
}

pub async fn create_manual_journal_entry(
    pool: &PgPool,
    req: ManualJournalRequest,
    user_id: Option<Uuid>,
) -> Result<JournalEntryDetail, String> {
    if req.lines.is_empty() {
        return Err("Jurnal harus memiliki setidaknya 2 baris debet/kredit.".to_string());
    }

    // 1. Validasi proteksi tutup buku
    check_transaction_date_allowed(pool, req.journal_date, req.outlet_id).await?;

    let mut total_debit = Decimal::ZERO;
    let mut total_credit = Decimal::ZERO;

    for l in &req.lines {
        total_debit += l.debit;
        total_credit += l.credit;
        if (l.debit > Decimal::ZERO && l.credit > Decimal::ZERO) || (l.debit == Decimal::ZERO && l.credit == Decimal::ZERO) {
            return Err("Setiap baris akun hanya boleh memiliki nilai debet ATAU kredit (tidak boleh keduanya atau keduanya nol).".to_string());
        }
    }

    if total_debit != total_credit {
        return Err(format!(
            "Jurnal tidak seimbang (Unbalanced): Total Debet ({}) tidak sama dengan Total Kredit ({}).",
            total_debit, total_credit
        ));
    }

    let mut tx = pool.begin().await.map_err(|e| format!("Database error: {}", e))?;

    let journal_id = Uuid::new_v4();
    let journal_number = format!("JRN-MAN-{}", Local::now().format("%Y%m%d%H%M%S"));
    let ref_type = req.reference_type.unwrap_or_else(|| "MANUAL_ADJUSTMENT".to_string());

    sqlx::query(
        "INSERT INTO journal_entries (
            id, journal_number, journal_date, reference_type, reference_id, description,
            total_debit, total_credit, is_closing_entry, created_at, updated_at, created_by, updated_by
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, FALSE, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $9, $9
        )",
    )
    .bind(journal_id)
    .bind(&journal_number)
    .bind(req.journal_date)
    .bind(ref_type)
    .bind(Option::<Uuid>::None)
    .bind(&req.description)
    .bind(total_debit)
    .bind(total_credit)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal menyimpan header jurnal: {}", e))?;

    for line in &req.lines {
        let line_outlet_id = line.outlet_id.or(req.outlet_id);
        sqlx::query(
            "INSERT INTO journal_lines (
                id, journal_id, account_id, debit, credit, memo, outlet_id, created_at, updated_at, created_by, updated_by
            ) VALUES (
                gen_random_uuid(), $1, $2, $3, $4, $5, $6, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $7, $7
            )",
        )
        .bind(journal_id)
        .bind(line.account_id)
        .bind(line.debit)
        .bind(line.credit)
        .bind(&line.memo)
        .bind(line_outlet_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal menyimpan baris jurnal: {}", e))?;
    }

    tx.commit().await.map_err(|e| format!("Gagal commit transaksi jurnal: {}", e))?;

    let details = list_journal_entries(
        pool,
        JournalQueryFilter {
            limit: Some(5),
            outlet_id: req.outlet_id,
            start_date: Some(req.journal_date),
            end_date: Some(req.journal_date),
            ..Default::default()
        },
        None,
    ).await?;
    details.data.into_iter().find(|j| j.entry.id == journal_id).ok_or_else(|| "Jurnal berhasil dibuat".to_string())
}

/// Mengambil saldo berjalan (current balance) suatu akun perkiraan pada cabang tertentu atau global.
pub async fn get_account_balance(
    pool: &PgPool,
    account_id: Uuid,
    outlet_id: Option<Uuid>,
) -> Result<Decimal, String> {
    let account = sqlx::query(
        "SELECT coa.code, coa.name, ac.name as category_name, ac.normal_balance
         FROM chart_of_accounts coa
         JOIN account_categories ac ON coa.account_category_id = ac.id
         WHERE coa.id = $1"
    )
    .bind(account_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?
    .ok_or_else(|| "Akun perkiraan tidak ditemukan.".to_string())?;

    let normal_balance: String = account.get("normal_balance");

    let balance: Decimal = if let Some(oid) = outlet_id {
        if normal_balance == "DEBIT" {
            sqlx::query_scalar(
                r#"
                SELECT COALESCE(SUM(jl.debit - jl.credit), 0)
                FROM journal_lines jl
                WHERE jl.account_id = $1
                  AND jl.outlet_id = $2
                "#
            )
            .bind(account_id)
            .bind(oid)
            .fetch_one(pool)
            .await
            .map_err(|e| format!("Database error: {}", e))?
        } else {
            sqlx::query_scalar(
                r#"
                SELECT COALESCE(SUM(jl.credit - jl.debit), 0)
                FROM journal_lines jl
                WHERE jl.account_id = $1
                  AND jl.outlet_id = $2
                "#
            )
            .bind(account_id)
            .bind(oid)
            .fetch_one(pool)
            .await
            .map_err(|e| format!("Database error: {}", e))?
        }
    } else {
        if normal_balance == "DEBIT" {
            sqlx::query_scalar(
                "SELECT COALESCE(SUM(debit - credit), 0) FROM journal_lines WHERE account_id = $1"
            )
            .bind(account_id)
            .fetch_one(pool)
            .await
            .map_err(|e| format!("Database error: {}", e))?
        } else {
            sqlx::query_scalar(
                "SELECT COALESCE(SUM(credit - debit), 0) FROM journal_lines WHERE account_id = $1"
            )
            .bind(account_id)
            .fetch_one(pool)
            .await
            .map_err(|e| format!("Database error: {}", e))?
        }
    };

    Ok(balance)
}

/// Melakukan transfer dana / kas antar cabang dengan mekanisme Rekening Antar-Kantor (RAK).
/// Menghasilkan 2 jurnal double-entry berpasangan yang seimbang pada masing-masing cabang.
pub async fn create_fund_transfer(
    pool: &PgPool,
    req: FundTransferRequest,
    user_id: Option<Uuid>,
) -> Result<FundTransferResponse, String> {
    if req.amount <= Decimal::ZERO {
        return Err("Nominal transfer dana harus lebih besar dari 0.".to_string());
    }

    if req.from_account_id == req.to_account_id {
        return Err("Rekening kas/bank asal dan tujuan tidak boleh sama.".to_string());
    }

    // 1. Validasi Hak Akses Pengguna terhadap Outlet Pengirim (Zero-Trust & Fail-Closed)
    if let Some(uid) = user_id {
        let is_superadmin_or_all: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM users u
                JOIN roles r ON u.role_id = r.id
                LEFT JOIN role_permissions rp ON r.id = rp.role_id
                LEFT JOIN permissions p ON rp.permission_id = p.id
                WHERE u.id = $1 AND (r.name = 'SUPERADMIN' OR p.code = 'accounting:all_ownership')
            )
            "#
        )
        .bind(uid)
        .fetch_one(pool)
        .await
        .unwrap_or(false);

        if !is_superadmin_or_all {
            let has_outlet_access: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM user_outlets WHERE user_id = $1 AND outlet_id = $2)"
            )
            .bind(uid)
            .bind(req.from_outlet_id)
            .fetch_one(pool)
            .await
            .unwrap_or(false);

            if !has_outlet_access {
                return Err("Akses ditolak: Anda tidak memiliki wewenang pada cabang pengirim tersebut.".to_string());
            }
        }
    } else {
        return Err("Akses ditolak: Pengguna tidak terautentikasi.".to_string());
    }

    // 2. Validasi Ketersediaan Saldo Rekening Pengirim
    let sender_balance = get_account_balance(pool, req.from_account_id, Some(req.from_outlet_id)).await?;
    if sender_balance <= Decimal::ZERO {
        return Err(format!(
            "Saldo rekening kas/bank asal tidak mencukupi (Saldo saat ini: Rp {}). Silakan lakukan setoran kas/modal terlebih dahulu.",
            sender_balance
        ));
    }
    if req.amount > sender_balance {
        return Err(format!(
            "Nominal transfer (Rp {}) melebihi saldo rekening kas/bank asal yang tersedia (Saldo saat ini: Rp {}).",
            req.amount, sender_balance
        ));
    }

    // 3. Validasi proteksi periode tutup buku pada kedua cabang
    check_transaction_date_allowed(pool, req.transfer_date, Some(req.from_outlet_id)).await?;
    if req.from_outlet_id != req.to_outlet_id {
        check_transaction_date_allowed(pool, req.transfer_date, Some(req.to_outlet_id)).await?;
    }

    let mut tx = pool.begin().await.map_err(|e| format!("Database error: {}", e))?;

    // 2. Cek validitas nama outlet
    let from_outlet_name: String = sqlx::query_scalar("SELECT name FROM outlets WHERE id = $1")
        .bind(req.from_outlet_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or_else(|| "Outlet asal tidak ditemukan.".to_string())?;

    let to_outlet_name: String = if req.from_outlet_id == req.to_outlet_id {
        from_outlet_name.clone()
    } else {
        sqlx::query_scalar("SELECT name FROM outlets WHERE id = $1")
            .bind(req.to_outlet_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|e| format!("Database error: {}", e))?
            .ok_or_else(|| "Outlet tujuan tidak ditemukan.".to_string())?
    };

    // 3. Cek validitas akun kas/bank asal dan tujuan
    let from_acc_name: String = sqlx::query_scalar("SELECT name FROM chart_of_accounts WHERE id = $1 AND is_active = TRUE")
        .bind(req.from_account_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or_else(|| "Akun sumber dana tidak ditemukan atau nonaktif.".to_string())?;

    let to_acc_name: String = sqlx::query_scalar("SELECT name FROM chart_of_accounts WHERE id = $1 AND is_active = TRUE")
        .bind(req.to_account_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| format!("Database error: {}", e))?
        .ok_or_else(|| "Akun tujuan penerimaan dana tidak ditemukan atau nonaktif.".to_string())?;

    let now_str = chrono::Utc::now().format("%Y%m%d%H%M%S").to_string();
    let note_text = req.notes.as_deref().unwrap_or("Transfer dana kas/bank");

    // JIKA TRANSFER INTERNAL DALAM SATU CABANG YANG SAMA (e.g. Kas Kasir -> Bank BCA)
    if req.from_outlet_id == req.to_outlet_id {
        let journal_id = Uuid::new_v4();
        let journal_number = format!("JRN-FND-INT-{}-{}", now_str, &journal_id.to_string()[..4].to_uppercase());

        sqlx::query(
            "INSERT INTO journal_entries (
                id, journal_number, journal_date, reference_type, reference_id, description,
                total_debit, total_credit, is_closing_entry, created_at, updated_at, created_by, updated_by
            ) VALUES (
                $1, $2, $3, 'FUND_TRANSFER', NULL, $4, $5, $5, FALSE, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $6, $6
            )",
        )
        .bind(journal_id)
        .bind(&journal_number)
        .bind(req.transfer_date)
        .bind(format!("Transfer Kas/Bank Internal dari {} ke {} - {}", from_acc_name, to_acc_name, note_text))
        .bind(req.amount)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal mencatat jurnal transfer internal: {}", e))?;

        // DEBIT Akun Tujuan (Dana Masuk)
        sqlx::query(
            "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, $3, 0, $4, $5)"
        )
        .bind(journal_id)
        .bind(req.to_account_id)
        .bind(req.amount)
        .bind(format!("Penerimaan transfer dana dari {}", from_acc_name))
        .bind(Some(req.from_outlet_id))
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal mencatat debit akun tujuan: {}", e))?;

        // KREDIT Akun Asal (Dana Keluar)
        sqlx::query(
            "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, 0, $3, $4, $5)"
        )
        .bind(journal_id)
        .bind(req.from_account_id)
        .bind(req.amount)
        .bind(format!("Pengeluaran transfer dana ke {}", to_acc_name))
        .bind(Some(req.from_outlet_id))
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal mencatat kredit akun asal: {}", e))?;

        tx.commit().await.map_err(|e| format!("Commit error: {}", e))?;

        return Ok(FundTransferResponse {
            transfer_date: req.transfer_date,
            from_outlet_id: req.from_outlet_id,
            to_outlet_id: req.to_outlet_id,
            from_outlet_name: from_outlet_name.clone(),
            to_outlet_name: to_outlet_name.clone(),
            from_journal_id: journal_id,
            to_journal_id: journal_id,
            from_journal_number: journal_number.clone(),
            to_journal_number: journal_number,
            amount: req.amount,
            message: format!(
                "Transfer kas/bank Rp {} dari '{}' ke '{}' di '{}' berhasil dibukukan.",
                req.amount, from_acc_name, to_acc_name, from_outlet_name
            ),
        });
    }

    // JIKA TRANSFER ANTAR CABANG (INTER-BRANCH VIA RAK)
    // 4. Ambil akun master Rekening Antar-Kantor (RAK - 1901)
    let rak_account_id: Uuid = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '1901' LIMIT 1")
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| "Akun Rekening Antar-Kantor (1901) tidak ditemukan. Pastikan migrasi akun RAK telah dijalankan.".to_string())?;

    let now_str = chrono::Utc::now().format("%Y%m%d%H%M%S").to_string();
    let note_text = req.notes.as_deref().unwrap_or("Transfer dana antar-cabang");

    // 5. JURNAL 1: Cabang Pengirim (Outlet Asal)
    // DEBIT: Rekening Antar-Kantor (RAK) - Piutang klaim antar-kantor (Dimensi Cabang Pengirim)
    // KREDIT: Akun Sumber Kas/Bank Cabang Pengirim - Dana berkurang (Dimensi Cabang Pengirim)
    let from_journal_id = Uuid::new_v4();
    let from_journal_number = format!("JRN-FND-OUT-{}-{}", now_str, &from_journal_id.to_string()[..4].to_uppercase());

    sqlx::query(
        "INSERT INTO journal_entries (
            id, journal_number, journal_date, reference_type, reference_id, description,
            total_debit, total_credit, is_closing_entry, created_at, updated_at, created_by, updated_by
        ) VALUES (
            $1, $2, $3, 'FUND_TRANSFER', NULL, $4, $5, $5, FALSE, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $6, $6
        )",
    )
    .bind(from_journal_id)
    .bind(&from_journal_number)
    .bind(req.transfer_date)
    .bind(format!("Transfer Dana ke {} ({}) - {}", to_outlet_name, to_acc_name, note_text))
    .bind(req.amount)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal mencatat jurnal pengirim: {}", e))?;

    sqlx::query(
        "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, $3, 0, $4, $5)"
    )
    .bind(from_journal_id)
    .bind(rak_account_id)
    .bind(req.amount)
    .bind(format!("Piutang RAK transfer dana ke {}", to_outlet_name))
    .bind(Some(req.from_outlet_id))
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal mencatat debit RAK pengirim: {}", e))?;

    sqlx::query(
        "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, 0, $3, $4, $5)"
    )
    .bind(from_journal_id)
    .bind(req.from_account_id)
    .bind(req.amount)
    .bind(format!("Pengeluaran dana kas/bank dari {} ({})", from_outlet_name, from_acc_name))
    .bind(Some(req.from_outlet_id))
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal mencatat kredit kas pengirim: {}", e))?;

    // 6. JURNAL 2: Cabang Penerima (Outlet Tujuan)
    // DEBIT: Akun Tujuan Kas/Bank Cabang Penerima - Dana bertambah (Dimensi Cabang Penerima)
    // KREDIT: Rekening Antar-Kantor (RAK) - Kewajiban antar-kantor (Dimensi Cabang Penerima)
    let to_journal_id = Uuid::new_v4();
    let to_journal_number = format!("JRN-FND-IN-{}-{}", now_str, &to_journal_id.to_string()[..4].to_uppercase());

    sqlx::query(
        "INSERT INTO journal_entries (
            id, journal_number, journal_date, reference_type, reference_id, description,
            total_debit, total_credit, is_closing_entry, created_at, updated_at, created_by, updated_by
        ) VALUES (
            $1, $2, $3, 'FUND_TRANSFER', NULL, $4, $5, $5, FALSE, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $6, $6
        )",
    )
    .bind(to_journal_id)
    .bind(&to_journal_number)
    .bind(req.transfer_date)
    .bind(format!("Penerimaan Transfer Dana dari {} ({}) - {}", from_outlet_name, from_acc_name, note_text))
    .bind(req.amount)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal mencatat jurnal penerima: {}", e))?;

    sqlx::query(
        "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, $3, 0, $4, $5)"
    )
    .bind(to_journal_id)
    .bind(req.to_account_id)
    .bind(req.amount)
    .bind(format!("Penerimaan dana kas/bank di {} ({})", to_outlet_name, to_acc_name))
    .bind(Some(req.to_outlet_id))
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal mencatat debit kas penerima: {}", e))?;

    sqlx::query(
        "INSERT INTO journal_lines (journal_id, account_id, debit, credit, memo, outlet_id) VALUES ($1, $2, 0, $3, $4, $5)"
    )
    .bind(to_journal_id)
    .bind(rak_account_id)
    .bind(req.amount)
    .bind(format!("Kewajiban RAK transfer dana dari {}", from_outlet_name))
    .bind(Some(req.to_outlet_id))
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal mencatat kredit RAK penerima: {}", e))?;

    tx.commit().await.map_err(|e| format!("Commit error: {}", e))?;

    Ok(FundTransferResponse {
        transfer_date: req.transfer_date,
        from_outlet_id: req.from_outlet_id,
        to_outlet_id: req.to_outlet_id,
        from_outlet_name: from_outlet_name.clone(),
        to_outlet_name: to_outlet_name.clone(),
        from_journal_id,
        to_journal_id,
        from_journal_number,
        to_journal_number,
        amount: req.amount,
        message: format!(
            "Transfer dana Rp {} dari '{}' ke '{}' berhasil dibukukan dengan Rekening Antar-Kantor (RAK)",
            req.amount, from_outlet_name, to_outlet_name
        ),
    })
}

// ============================================================================
// SOFT CLOSE & HARD CLOSE (DIMENSIONAL ACCOUNTING CLOSING LIFECYCLE)
// ============================================================================

pub async fn get_accounting_period_by_id(pool: &PgPool, id: Uuid) -> Result<AccountingPeriod, String> {
    let r = sqlx::query(
        "SELECT ap.*, 
                u1.full_name as closed_by_name,
                u2.full_name as reopened_by_name
         FROM accounting_periods ap
         LEFT JOIN users u1 ON ap.closed_by = u1.id
         LEFT JOIN users u2 ON ap.reopened_by = u2.id
         WHERE ap.id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?
    .ok_or_else(|| "Periode akuntansi tidak ditemukan".to_string())?;

    Ok(AccountingPeriod {
        id: r.get("id"),
        name: r.get("name"),
        start_date: r.get("start_date"),
        end_date: r.get("end_date"),
        is_closed: r.get("is_closed"),
        closed_at: r.get("closed_at"),
        closed_by: r.get("closed_by"),
        closed_by_name: r.get("closed_by_name"),
        reopened_at: r.get("reopened_at"),
        reopened_by: r.get("reopened_by"),
        reopened_by_name: r.get("reopened_by_name"),
        reopen_reason: r.get("reopen_reason"),
        created_at: r.get("created_at"),
        updated_at: r.get("updated_at"),
    })
}

pub async fn list_accounting_periods(
    pool: &PgPool,
    filter: PeriodQueryFilter,
) -> Result<PaginatedAccountingPeriods, String> {
    let mut where_clause = " WHERE 1=1".to_string();

    if let Some(ref st) = filter.status {
        let st_clean = st.trim().to_uppercase();
        if st_clean == "OPEN" {
            where_clause.push_str(" AND ap.is_closed = FALSE");
        } else if st_clean == "CLOSED" {
            where_clause.push_str(" AND ap.is_closed = TRUE");
        }
    }

    if let Some(ref q) = filter.search {
        let clean = q.trim().replace('\'', "''");
        if !clean.is_empty() {
            where_clause.push_str(&format!(
                " AND (ap.name ILIKE '%{clean}%' OR ap.reopen_reason ILIKE '%{clean}%' OR u1.full_name ILIKE '%{clean}%' OR u2.full_name ILIKE '%{clean}%')"
            ));
        }
    }

    let count_sql = format!(
        "SELECT COUNT(*)::bigint FROM accounting_periods ap
         LEFT JOIN users u1 ON ap.closed_by = u1.id
         LEFT JOIN users u2 ON ap.reopened_by = u2.id
         {}",
        where_clause
    );
    let total_items: i64 = sqlx::query_scalar(&count_sql)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let sort_col = match filter.sort_by.as_deref() {
        Some("name") => "ap.name",
        Some("end_date") => "ap.end_date",
        Some("is_closed") => "ap.is_closed",
        Some("created_at") => "ap.created_at",
        _ => "ap.start_date",
    };
    let sort_dir = match filter.sort_dir.as_deref() {
        Some("asc") | Some("ASC") => "ASC",
        _ => "DESC",
    };

    let page = filter.page.unwrap_or(1).max(1);
    let page_size = filter.page_size.unwrap_or(filter.limit.unwrap_or(10)).max(1);
    let total_pages = if total_items > 0 {
        ((total_items as f64) / (page_size as f64)).ceil() as i64
    } else {
        1
    };
    let offset = (page - 1) * page_size;

    let sql = format!(
        "SELECT ap.*, 
                u1.full_name as closed_by_name,
                u2.full_name as reopened_by_name
         FROM accounting_periods ap
         LEFT JOIN users u1 ON ap.closed_by = u1.id
         LEFT JOIN users u2 ON ap.reopened_by = u2.id
         {}
         ORDER BY {} {}, ap.created_at DESC
         LIMIT {} OFFSET {}",
        where_clause, sort_col, sort_dir, page_size, offset
    );

    let rows = sqlx::query(&sql)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Gagal mengambil daftar periode akuntansi: {}", e))?;

    let list = rows
        .into_iter()
        .map(|r| AccountingPeriod {
            id: r.get("id"),
            name: r.get("name"),
            start_date: r.get("start_date"),
            end_date: r.get("end_date"),
            is_closed: r.get("is_closed"),
            closed_at: r.get("closed_at"),
            closed_by: r.get("closed_by"),
            closed_by_name: r.get("closed_by_name"),
            reopened_at: r.get("reopened_at"),
            reopened_by: r.get("reopened_by"),
            reopened_by_name: r.get("reopened_by_name"),
            reopen_reason: r.get("reopen_reason"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        })
        .collect();

    Ok(PaginatedAccountingPeriods {
        data: list,
        total_items,
        total_pages,
        page,
        page_size,
    })
}

pub async fn create_accounting_period(
    pool: &PgPool,
    req: CreatePeriodRequest,
    _user_id: Option<Uuid>,
) -> Result<AccountingPeriod, String> {
    if req.start_date > req.end_date {
        return Err("Tanggal awal periode tidak boleh lebih besar dari tanggal akhir periode.".to_string());
    }

    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO accounting_periods (id, name, start_date, end_date, is_closed)
         VALUES ($1, $2, $3, $4, FALSE)",
    )
    .bind(id)
    .bind(&req.name)
    .bind(req.start_date)
    .bind(req.end_date)
    .execute(pool)
    .await
    .map_err(|e| format!("Gagal membuat periode akuntansi: {}", e))?;

    get_accounting_period_by_id(pool, id).await
}

pub async fn soft_close_accounting_period(
    pool: &PgPool,
    id: Uuid,
    user_id: Option<Uuid>,
) -> Result<AccountingPeriod, String> {
    let res = sqlx::query(
        "UPDATE accounting_periods
         SET is_closed = TRUE,
             closed_at = CURRENT_TIMESTAMP,
             closed_by = $1,
             updated_at = CURRENT_TIMESTAMP
         WHERE id = $2 AND is_closed = FALSE",
    )
    .bind(user_id)
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| format!("Gagal melakukan soft close periode: {}", e))?;

    if res.rows_affected() == 0 {
        return Err("Periode tidak ditemukan atau sudah dalam status terkunci (CLOSED).".to_string());
    }

    get_accounting_period_by_id(pool, id).await
}

pub async fn reopen_accounting_period(
    pool: &PgPool,
    id: Uuid,
    req: ReopenPeriodRequest,
    user_id: Option<Uuid>,
) -> Result<AccountingPeriod, String> {
    if req.reason.trim().is_empty() {
        return Err("Alasan pembukaan kembali periode wajib dicantumkan.".to_string());
    }

    let res = sqlx::query(
        "UPDATE accounting_periods
         SET is_closed = FALSE,
             reopened_at = CURRENT_TIMESTAMP,
             reopened_by = $1,
             reopen_reason = $2,
             updated_at = CURRENT_TIMESTAMP
         WHERE id = $3 AND is_closed = TRUE",
    )
    .bind(user_id)
    .bind(req.reason.trim())
    .bind(id)
    .execute(pool)
    .await
    .map_err(|e| format!("Gagal membuka kembali periode akuntansi: {}", e))?;

    if res.rows_affected() == 0 {
        return Err("Periode tidak ditemukan atau memang belum berstatus terkunci.".to_string());
    }

    get_accounting_period_by_id(pool, id).await
}

pub async fn perform_hard_close(
    pool: &PgPool,
    req: HardCloseRequest,
    user_id: Option<Uuid>,
) -> Result<JournalEntryDetail, String> {
    // Fase 3 (Hard Close Penutupan Akhir Tahun):
    // 1. Membaca total saldo semua akun Pendapatan (Revenue) dan Beban (Expense) pada rentang tahun tersebut.
    // 2. Buat satu journal_entries baru (dengan is_closing_entry = TRUE)
    // 3. Otomatis mendebit semua Pendapatan, mengkredit semua Beban, dan memasukkan selisihnya ke akun Laba Ditahan (3201) pada kelompok Ekuitas.
    if req.fiscal_year < 2000 || req.fiscal_year > 2100 {
        return Err("Tahun buku tidak valid.".to_string());
    }

    let period_start = NaiveDate::from_ymd_opt(req.fiscal_year, 1, 1).unwrap();
    let period_end = NaiveDate::from_ymd_opt(req.fiscal_year, 12, 31).unwrap();

    let mut tx = pool.begin().await.map_err(|e| format!("Gagal memulai transaksi: {}", e))?;

    // 1. Ambil saldo seluruh akun Pendapatan (REVENUE)
    let rev_query = r#"
        SELECT coa.id, coa.code, coa.name, COALESCE(SUM(jl.credit - jl.debit), 0) as balance
        FROM journal_lines jl
        JOIN journal_entries je ON jl.journal_id = je.id
        JOIN chart_of_accounts coa ON jl.account_id = coa.id
        JOIN account_categories ac ON coa.account_category_id = ac.id
        WHERE ac.name = 'REVENUE'
          AND je.journal_date >= $1 AND je.journal_date <= $2
          AND je.is_closing_entry = FALSE
        GROUP BY coa.id, coa.code, coa.name
        HAVING COALESCE(SUM(jl.credit - jl.debit), 0) != 0
    "#;

    let rev_rows = sqlx::query(rev_query)
        .bind(period_start)
        .bind(period_end)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| format!("Gagal menghitung saldo pendapatan: {}", e))?;

    // 2. Ambil saldo seluruh akun Beban (EXPENSE)
    let exp_query = r#"
        SELECT coa.id, coa.code, coa.name, COALESCE(SUM(jl.debit - jl.credit), 0) as balance
        FROM journal_lines jl
        JOIN journal_entries je ON jl.journal_id = je.id
        JOIN chart_of_accounts coa ON jl.account_id = coa.id
        JOIN account_categories ac ON coa.account_category_id = ac.id
        WHERE ac.name = 'EXPENSE'
          AND je.journal_date >= $1 AND je.journal_date <= $2
          AND je.is_closing_entry = FALSE
        GROUP BY coa.id, coa.code, coa.name
        HAVING COALESCE(SUM(jl.debit - jl.credit), 0) != 0
    "#;

    let exp_rows = sqlx::query(exp_query)
        .bind(period_start)
        .bind(period_end)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| format!("Gagal menghitung saldo beban: {}", e))?;

    let mut total_revenue = Decimal::ZERO;
    for r in &rev_rows {
        total_revenue += r.get::<Decimal, _>("balance");
    }

    let mut total_expense = Decimal::ZERO;
    for r in &exp_rows {
        total_expense += r.get::<Decimal, _>("balance");
    }

    let net_income = total_revenue - total_expense;

    if rev_rows.is_empty() && exp_rows.is_empty() {
        return Err(format!("Tidak ditemukan mutasi akun nominal (Pendapatan/Beban) untuk tahun buku {}.", req.fiscal_year));
    }

    // Akun Laba Ditahan (Retained Earnings - 3201)
    let acc_retained_earnings_id: Uuid = sqlx::query_scalar("SELECT id FROM chart_of_accounts WHERE code = '3201' LIMIT 1")
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| "Akun 3201 (Laba Ditahan) tidak ditemukan dalam sistem CoA")?;

    let journal_id = Uuid::new_v4();
    let journal_number = format!("JRN-HRDCLS-{}", Local::now().format("%Y%m%d%H%M%S"));

    let mut closing_lines: Vec<(Uuid, Decimal, Decimal, String)> = Vec::new();

    for r in &rev_rows {
        let acc_id: Uuid = r.get("id");
        let bal: Decimal = r.get("balance");
        if bal > Decimal::ZERO {
            // Debit akun pendapatan untuk menolkan saldo kredit
            closing_lines.push((acc_id, bal, Decimal::ZERO, "Tutup Buku Akhir Tahun: Penutupan Saldo Pendapatan".to_string()));
        } else if bal < Decimal::ZERO {
            // Kredit jika kontra pendapatan
            closing_lines.push((acc_id, Decimal::ZERO, bal.abs(), "Tutup Buku Akhir Tahun: Penutupan Kontra Pendapatan".to_string()));
        }
    }

    for r in &exp_rows {
        let acc_id: Uuid = r.get("id");
        let bal: Decimal = r.get("balance");
        if bal > Decimal::ZERO {
            // Kredit akun beban untuk menolkan saldo debet
            closing_lines.push((acc_id, Decimal::ZERO, bal, "Tutup Buku Akhir Tahun: Penutupan Saldo Beban".to_string()));
        } else if bal < Decimal::ZERO {
            // Debit jika saldo kontra beban
            closing_lines.push((acc_id, bal.abs(), Decimal::ZERO, "Tutup Buku Akhir Tahun: Penutupan Kontra Beban".to_string()));
        }
    }

    // Selisih laba/rugi ke Laba Ditahan:
    // Jika Laba Bersih (net_income > 0) -> Kredit Laba Ditahan (Ekuitas bertambah)
    // Jika Rugi Bersih (net_income < 0) -> Debit Laba Ditahan (Ekuitas berkurang)
    if net_income > Decimal::ZERO {
        closing_lines.push((acc_retained_earnings_id, Decimal::ZERO, net_income, format!("Tutup Buku Akhir Tahun {}: Pemindahan Laba Bersih ke Laba Ditahan", req.fiscal_year)));
    } else if net_income < Decimal::ZERO {
        closing_lines.push((acc_retained_earnings_id, net_income.abs(), Decimal::ZERO, format!("Tutup Buku Akhir Tahun {}: Pembebanan Rugi Bersih ke Laba Ditahan", req.fiscal_year)));
    }

    let mut tot_debit = Decimal::ZERO;
    let mut tot_credit = Decimal::ZERO;
    for (_, d, c, _) in &closing_lines {
        tot_debit += *d;
        tot_credit += *c;
    }

    if (tot_debit - tot_credit).abs() > Decimal::new(1, 2) {
        return Err(format!("Jurnal penutup tahunan tidak seimbang (Debet: {}, Kredit: {})", tot_debit, tot_credit));
    }

    let desc = req.description.unwrap_or_else(|| format!("Hard Close Tutup Buku Tahunan Tahun {}", req.fiscal_year));

    sqlx::query(
        "INSERT INTO journal_entries (
            id, journal_number, journal_date, reference_type, reference_id, description,
            total_debit, total_credit, is_closing_entry, created_at, updated_at, created_by, updated_by
        ) VALUES (
            $1, $2, $3, 'CLOSING_ENTRY', NULL, $4, $5, $6, TRUE, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $7, $7
        )",
    )
    .bind(journal_id)
    .bind(&journal_number)
    .bind(period_end)
    .bind(&desc)
    .bind(tot_debit)
    .bind(tot_credit)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal mencatat header jurnal penutup tahunan: {}", e))?;

    for (acc, d, c, memo) in closing_lines {
        sqlx::query(
            "INSERT INTO journal_lines (
                id, journal_id, account_id, debit, credit, memo, outlet_id, created_at, updated_at, created_by, updated_by
            ) VALUES (
                gen_random_uuid(), $1, $2, $3, $4, $5, NULL, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, $6, $6
            )",
        )
        .bind(journal_id)
        .bind(acc)
        .bind(d)
        .bind(c)
        .bind(memo)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal mencatat baris jurnal penutup: {}", e))?;
    }

    // Kunci juga periode akuntansi tahunan ini jika ada di accounting_periods
    sqlx::query(
        "UPDATE accounting_periods
         SET is_closed = TRUE,
             closed_at = CURRENT_TIMESTAMP,
             closed_by = $1,
             updated_at = CURRENT_TIMESTAMP
         WHERE start_date <= $2 AND end_date >= $3",
    )
    .bind(user_id)
    .bind(period_start)
    .bind(period_end)
    .execute(&mut *tx)
    .await
    .ok();

    tx.commit().await.map_err(|e| format!("Commit error: {}", e))?;

    let details = list_journal_entries(
        pool,
        JournalQueryFilter {
            limit: Some(1),
            start_date: Some(period_end),
            end_date: Some(period_end),
            ..Default::default()
        },
        None,
    ).await?;

    details.data.into_iter().find(|j| j.entry.id == journal_id).ok_or_else(|| "Tutup buku tahunan (Hard Close) berhasil diproses".to_string())
}


