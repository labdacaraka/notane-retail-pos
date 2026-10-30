use chrono::{Local, NaiveDate};
use rust_decimal::Decimal;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::models::dividend::{
    ClosedPeriodForDividend, CreateDividendRequest, DividendDistribution, DividendFilter,
    DividendSummary,
};

pub async fn list_dividends(
    pool: &PgPool,
    filter: DividendFilter,
) -> Result<(Vec<DividendDistribution>, i64), String> {
    let page = filter.page.unwrap_or(1).max(1);
    let limit = filter.limit.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * limit;

    let mut conditions = Vec::new();
    let mut param_idx = 1;

    let target_closing_id = filter.closing_id.or(filter.period_id);
    if target_closing_id.is_some() {
        conditions.push(format!("dd.closing_id = ${}", param_idx));
        param_idx += 1;
    }
    if filter.start_date.is_some() {
        conditions.push(format!("dd.distribution_date >= ${}", param_idx));
        param_idx += 1;
    }
    if filter.end_date.is_some() {
        conditions.push(format!("dd.distribution_date <= ${}", param_idx));
        param_idx += 1;
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let count_sql = format!(
        "SELECT COUNT(*) as total FROM dividend_distributions dd {}",
        where_clause
    );

    let mut count_query = sqlx::query_scalar::<_, i64>(&count_sql);
    if let Some(cid) = target_closing_id {
        count_query = count_query.bind(cid);
    }
    if let Some(start_date) = filter.start_date {
        count_query = count_query.bind(start_date);
    }
    if let Some(end_date) = filter.end_date {
        count_query = count_query.bind(end_date);
    }

    let total = count_query
        .fetch_one(pool)
        .await
        .map_err(|e| format!("Gagal menghitung total data dividen: {}", e))?;

    let sql = format!(
        r#"
        SELECT 
            dd.id,
            dd.distribution_number,
            dd.closing_id,
            dd.closing_id as period_id,
            ac.closing_number,
            format('%s (%s s/d %s)', ac.closing_number, to_char(ac.period_start, 'DD/MM/YYYY'), to_char(ac.period_end, 'DD/MM/YYYY')) as period_name,
            ac.period_start,
            ac.period_end,
            dd.distribution_date,
            dd.total_amount,
            dd.retained_earnings_account_id,
            coa_re.code as retained_earnings_account_code,
            coa_re.name as retained_earnings_account_name,
            dd.payment_account_id,
            coa_pay.code as payment_account_code,
            coa_pay.name as payment_account_name,
            dd.journal_entry_id,
            je.journal_number,
            dd.recipient_name,
            dd.notes,
            dd.recipients_data,
            dd.created_at,
            dd.created_by,
            u.full_name as created_by_name
        FROM dividend_distributions dd
        LEFT JOIN accounting_closings ac ON dd.closing_id = ac.id
        LEFT JOIN chart_of_accounts coa_re ON dd.retained_earnings_account_id = coa_re.id
        LEFT JOIN chart_of_accounts coa_pay ON dd.payment_account_id = coa_pay.id
        LEFT JOIN journal_entries je ON dd.journal_entry_id = je.id
        LEFT JOIN users u ON dd.created_by = u.id
        {}
        ORDER BY dd.distribution_date DESC, dd.created_at DESC
        LIMIT ${} OFFSET ${}
        "#,
        where_clause, param_idx, param_idx + 1
    );

    let mut query = sqlx::query(&sql);
    if let Some(cid) = target_closing_id {
        query = query.bind(cid);
    }
    if let Some(start_date) = filter.start_date {
        query = query.bind(start_date);
    }
    if let Some(end_date) = filter.end_date {
        query = query.bind(end_date);
    }
    query = query.bind(limit).bind(offset);

    let rows = query
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Gagal mengambil daftar dividen: {}", e))?;

    let items = rows
        .into_iter()
        .map(|r| DividendDistribution {
            id: r.get("id"),
            distribution_number: r.get("distribution_number"),
            closing_id: r.get("closing_id"),
            period_id: r.get("period_id"),
            closing_number: r.get("closing_number"),
            period_name: r.get("period_name"),
            period_start: r.get("period_start"),
            period_end: r.get("period_end"),
            distribution_date: r.get("distribution_date"),
            total_amount: r.get("total_amount"),
            retained_earnings_account_id: r.get("retained_earnings_account_id"),
            retained_earnings_account_code: r.get("retained_earnings_account_code"),
            retained_earnings_account_name: r.get("retained_earnings_account_name"),
            payment_account_id: r.get("payment_account_id"),
            payment_account_code: r.get("payment_account_code"),
            payment_account_name: r.get("payment_account_name"),
            journal_entry_id: r.get("journal_entry_id"),
            journal_number: r.get("journal_number"),
            recipient_name: r.get("recipient_name"),
            notes: r.get("notes"),
            recipients_data: r.get("recipients_data"),
            created_at: r.get("created_at"),
            created_by: r.get("created_by"),
            created_by_name: r.get("created_by_name"),
        })
        .collect();

    Ok((items, total))
}

pub async fn get_dividend_by_id(
    pool: &PgPool,
    id: Uuid,
) -> Result<DividendDistribution, String> {
    let sql = r#"
        SELECT 
            dd.id,
            dd.distribution_number,
            dd.closing_id,
            dd.closing_id as period_id,
            ac.closing_number,
            format('%s (%s s/d %s)', ac.closing_number, to_char(ac.period_start, 'DD/MM/YYYY'), to_char(ac.period_end, 'DD/MM/YYYY')) as period_name,
            ac.period_start,
            ac.period_end,
            dd.distribution_date,
            dd.total_amount,
            dd.retained_earnings_account_id,
            coa_re.code as retained_earnings_account_code,
            coa_re.name as retained_earnings_account_name,
            dd.payment_account_id,
            coa_pay.code as payment_account_code,
            coa_pay.name as payment_account_name,
            dd.journal_entry_id,
            je.journal_number,
            dd.recipient_name,
            dd.notes,
            dd.recipients_data,
            dd.created_at,
            dd.created_by,
            u.full_name as created_by_name
        FROM dividend_distributions dd
        LEFT JOIN accounting_closings ac ON dd.closing_id = ac.id
        LEFT JOIN chart_of_accounts coa_re ON dd.retained_earnings_account_id = coa_re.id
        LEFT JOIN chart_of_accounts coa_pay ON dd.payment_account_id = coa_pay.id
        LEFT JOIN journal_entries je ON dd.journal_entry_id = je.id
        LEFT JOIN users u ON dd.created_by = u.id
        WHERE dd.id = $1
    "#;

    let r = sqlx::query(sql)
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| format!("Gagal mengambil data dividen: {}", e))?
        .ok_or_else(|| "Data dividen tidak ditemukan.".to_string())?;

    Ok(DividendDistribution {
        id: r.get("id"),
        distribution_number: r.get("distribution_number"),
        closing_id: r.get("closing_id"),
        period_id: r.get("period_id"),
        closing_number: r.get("closing_number"),
        period_name: r.get("period_name"),
        period_start: r.get("period_start"),
        period_end: r.get("period_end"),
        distribution_date: r.get("distribution_date"),
        total_amount: r.get("total_amount"),
        retained_earnings_account_id: r.get("retained_earnings_account_id"),
        retained_earnings_account_code: r.get("retained_earnings_account_code"),
        retained_earnings_account_name: r.get("retained_earnings_account_name"),
        payment_account_id: r.get("payment_account_id"),
        payment_account_code: r.get("payment_account_code"),
        payment_account_name: r.get("payment_account_name"),
        journal_entry_id: r.get("journal_entry_id"),
        journal_number: r.get("journal_number"),
        recipient_name: r.get("recipient_name"),
        notes: r.get("notes"),
        recipients_data: r.get("recipients_data"),
        created_at: r.get("created_at"),
        created_by: r.get("created_by"),
        created_by_name: r.get("created_by_name"),
    })
}

pub async fn get_dividend_summary(pool: &PgPool) -> Result<DividendSummary, String> {
    // 1. Saldo akun 3201 (Laba Ditahan) saat ini dari buku besar
    let re_sql = r#"
        SELECT COALESCE(SUM(jl.credit - jl.debit), 0) as balance
        FROM journal_lines jl
        JOIN chart_of_accounts coa ON jl.account_id = coa.id
        WHERE coa.code = '3201'
    "#;
    let total_retained_earnings: Decimal = sqlx::query_scalar(re_sql)
        .fetch_one(pool)
        .await
        .unwrap_or(Decimal::ZERO);

    // 2. Total dividen yang telah dibagikan
    let div_paid_sql = "SELECT COALESCE(SUM(total_amount), 0) FROM dividend_distributions";
    let total_dividends_paid: Decimal = sqlx::query_scalar(div_paid_sql)
        .fetch_one(pool)
        .await
        .unwrap_or(Decimal::ZERO);

    // 3. Ambil daftar DOKUMEN TUTUP BUKU (accounting_closings) yang berstatus CLOSED
    // Penutupan laba/rugi ke Laba Ditahan (3201) terjadi secara sah pada dokumen Tutup Buku ini
    let closings_sql = r#"
        SELECT 
            ac.id,
            ac.closing_number,
            ac.period_start,
            ac.period_end,
            ac.closing_date,
            ac.net_income,
            ac.status,
            u.full_name as closed_by_name,
            -- Ambil total dividen yang sudah ditarik untuk dokumen tutup buku ini
            COALESCE((
                SELECT SUM(dd.total_amount)
                FROM dividend_distributions dd
                WHERE dd.closing_id = ac.id
            ), 0) as total_dividends_drawn
        FROM accounting_closings ac
        LEFT JOIN users u ON ac.created_by = u.id
        WHERE ac.status = 'CLOSED'
        ORDER BY ac.period_end DESC, ac.closing_date DESC
    "#;

    let closing_rows = sqlx::query(closings_sql)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Gagal memuat dokumen tutup buku: {}", e))?;

    let closed_periods_count = closing_rows.len() as i64;
    let mut eligible_closed_periods = Vec::new();

    for r in closing_rows {
        let closing_number: String = r.get("closing_number");
        let start_date: NaiveDate = r.get("period_start");
        let end_date: NaiveDate = r.get("period_end");
        let net_income: Decimal = r.get("net_income");
        let total_drawn: Decimal = r.get("total_dividends_drawn");
        let remaining = (net_income - total_drawn).max(Decimal::ZERO);

        let name = format!(
            "{} ({} s/d {})",
            closing_number,
            start_date.format("%d/%m/%Y"),
            end_date.format("%d/%m/%Y")
        );

        eligible_closed_periods.push(ClosedPeriodForDividend {
            id: r.get("id"),
            closing_number,
            name,
            period_start: start_date,
            period_end: end_date,
            closing_date: r.get("closing_date"),
            is_closed: true,
            closed_by_name: r.get("closed_by_name"),
            net_income,
            total_dividends_drawn: total_drawn,
            remaining_income: remaining,
        });
    }

    Ok(DividendSummary {
        total_retained_earnings,
        total_dividends_paid,
        closed_periods_count,
        eligible_closed_periods,
    })
}

pub async fn create_dividend(
    pool: &PgPool,
    req: CreateDividendRequest,
    user_id: Option<Uuid>,
) -> Result<DividendDistribution, String> {
    if req.total_amount <= Decimal::ZERO {
        return Err("Nominal dividen harus lebih besar dari 0.".to_string());
    }

    if req.recipient_name.trim().is_empty() {
        return Err("Nama penerima dividen / pemegang saham wajib diisi.".to_string());
    }

    let target_closing_id = req
        .closing_id
        .or(req.period_id)
        .ok_or_else(|| "Dokumen Tutup Buku (closing_id) wajib dipilih.".to_string())?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("Gagal memulai transaksi: {}", e))?;

    // 1. Verifikasi Dokumen Tutup Buku: WAJIB TERHUBUNG KE DOKUMEN TUTUP BUKU DENGAN STATUS CLOSED
    let closing_row = sqlx::query(
        "SELECT id, closing_number, period_start, period_end, net_income, status 
         FROM accounting_closings WHERE id = $1",
    )
    .bind(target_closing_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| format!("Gagal memeriksa dokumen tutup buku: {}", e))?
    .ok_or_else(|| "Dokumen Tutup Buku yang dipilih tidak ditemukan dalam sistem.".to_string())?;

    let status: String = closing_row.get("status");
    let closing_number: String = closing_row.get("closing_number");
    let closing_net_income: Decimal = closing_row.get("net_income");

    if status != "CLOSED" {
        return Err(format!(
            "Akses Ditolak: Dokumen Tutup Buku '{}' belum berstatus CLOSED atau telah dibuka kembali (Status: {}). Sesuai kaidah akuntansi, pembagian dividen hanya dapat diproses dari Tutup Buku yang berstatus CLOSED!",
            closing_number, status
        ));
    }

    // 2. Verifikasi Batas Maksimal Dividen untuk Dokumen Tutup Buku Ini
    let total_drawn_closing: Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(total_amount), 0) FROM dividend_distributions WHERE closing_id = $1",
    )
    .bind(target_closing_id)
    .fetch_one(&mut *tx)
    .await
    .unwrap_or(Decimal::ZERO);

    let remaining_closing_income = (closing_net_income - total_drawn_closing).max(Decimal::ZERO);
    if req.total_amount > remaining_closing_income {
        return Err(format!(
            "Nominal dividen (Rp {}) melebihi sisa laba bersih dari Dokumen Tutup Buku {} (Laba bersih: Rp {}, telah ditarik: Rp {}, sisa yang dapat dibagikan: Rp {}).",
            req.total_amount, closing_number, closing_net_income, total_drawn_closing, remaining_closing_income
        ));
    }

    // 3. Tentukan Akun Laba Ditahan (Retained Earnings - 3201)
    let retained_earnings_acc_id = if let Some(acc_id) = req.retained_earnings_account_id {
        acc_id
    } else {
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM chart_of_accounts WHERE code = '3201' LIMIT 1")
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| "Akun 3201 (Laba Ditahan) tidak ditemukan dalam bagan akun CoA")?
    };

    // 4. Verifikasi Saldo Akun Laba Ditahan (3201) dari buku besar
    let current_re_balance: Decimal = sqlx::query_scalar(
        r#"
        SELECT COALESCE(SUM(jl.credit - jl.debit), 0)
        FROM journal_lines jl
        WHERE jl.account_id = $1
        "#,
    )
    .bind(retained_earnings_acc_id)
    .fetch_one(&mut *tx)
    .await
    .unwrap_or(Decimal::ZERO);

    if current_re_balance < req.total_amount {
        return Err(format!(
            "Saldo akun Laba Ditahan (3201) tidak mencukupi untuk pembagian dividen. Saldo buku besar saat ini: Rp {}, Permintaan dividen: Rp {}.",
            current_re_balance, req.total_amount
        ));
    }

    // 5. Verifikasi Akun Kas/Bank Pembayaran
    let pay_acc = sqlx::query(
        "SELECT id, code, name, is_active FROM chart_of_accounts WHERE id = $1",
    )
    .bind(req.payment_account_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| format!("Gagal memverifikasi akun kas/bank pembayaran: {}", e))?
    .ok_or_else(|| "Akun kas/bank pembayaran tidak ditemukan.".to_string())?;

    let is_pay_active: bool = pay_acc.get("is_active");
    if !is_pay_active {
        return Err("Akun kas/bank yang dipilih dalam status nonaktif.".to_string());
    }

    let pay_code: String = pay_acc.get("code");
    let pay_name: String = pay_acc.get("name");

    // 6. Generate Nomor Dividen
    let today = Local::now().format("%Y%m%d").to_string();
    let count_today: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM dividend_distributions WHERE distribution_number LIKE $1",
    )
    .bind(format!("DIV-{}%", today))
    .fetch_one(&mut *tx)
    .await
    .unwrap_or(0);

    let dist_number = format!("DIV-{}-{:04}", today, count_today + 1);

    // 7. Simpan Dokumen Dividen ke dividend_distributions
    let dist_id = Uuid::new_v4();
    let recipients_json = req.recipients.map(|r| serde_json::to_value(r).unwrap_or(serde_json::Value::Null));

    sqlx::query(
        r#"
        INSERT INTO dividend_distributions (
            id, distribution_number, closing_id, distribution_date, total_amount,
            retained_earnings_account_id, payment_account_id, recipient_name,
            notes, recipients_data, created_by, updated_by
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $11
        )
        "#,
    )
    .bind(dist_id)
    .bind(&dist_number)
    .bind(target_closing_id)
    .bind(req.distribution_date)
    .bind(req.total_amount)
    .bind(retained_earnings_acc_id)
    .bind(req.payment_account_id)
    .bind(req.recipient_name.trim())
    .bind(req.notes.as_deref())
    .bind(recipients_json)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal menyimpan pembagian dividen: {}", e))?;

    // 8. Buat Jurnal Pembagian Dividen:
    // Debet: Laba Ditahan (3201) -> Mengurangi Ekuitas
    // Kredit: Rekening Kas/Bank Pembayaran -> Mengurangi Aset Lancar
    let journal_id = Uuid::new_v4();
    let journal_number = format!("JRN-DIV-{}", dist_number);
    let journal_notes = format!(
        "Pembagian dividen {} kepada {} atas Tutup Buku {}",
        dist_number, req.recipient_name.trim(), closing_number
    );

    sqlx::query(
        r#"
        INSERT INTO journal_entries (
            id, journal_number, journal_date, reference_type, reference_id,
            description, total_debit, total_credit, is_closing_entry, created_by
        ) VALUES (
            $1, $2, $3, 'DIVIDEND_DISTRIBUTION', $4,
            $5, $6, $6, FALSE, $7
        )
        "#,
    )
    .bind(journal_id)
    .bind(&journal_number)
    .bind(req.distribution_date)
    .bind(dist_id)
    .bind(&journal_notes)
    .bind(req.total_amount)
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal membuat header jurnal dividen: {}", e))?;

    // Baris 1: Debet 3201 Laba Ditahan
    let jl_id_debit = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO journal_lines (
            id, journal_id, account_id, debit, credit, memo, created_by
        ) VALUES (
            $1, $2, $3, $4, 0, $5, $6
        )
        "#,
    )
    .bind(jl_id_debit)
    .bind(journal_id)
    .bind(retained_earnings_acc_id)
    .bind(req.total_amount)
    .bind(format!("Pembagian dividen {} - Pengurangan Laba Ditahan", dist_number))
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal mencatat baris debit jurnal dividen: {}", e))?;

    // Baris 2: Kredit Rekening Kas/Bank Pencairan
    let jl_id_credit = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO journal_lines (
            id, journal_id, account_id, debit, credit, memo, created_by
        ) VALUES (
            $1, $2, $3, 0, $4, $5, $6
        )
        "#,
    )
    .bind(jl_id_credit)
    .bind(journal_id)
    .bind(req.payment_account_id)
    .bind(req.total_amount)
    .bind(format!("Pencairan dividen {} via {} ({})", dist_number, pay_name, pay_code))
    .bind(user_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| format!("Gagal mencatat baris kredit jurnal dividen: {}", e))?;

    // 9. Update journal_entry_id pada dividend_distributions
    sqlx::query("UPDATE dividend_distributions SET journal_entry_id = $1 WHERE id = $2")
        .bind(journal_id)
        .bind(dist_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal mengaitkan jurnal ke dokumen dividen: {}", e))?;

    // 10. Audit log
    let audit_desc = format!(
        "Pembagian dividen {} sebesar Rp {} kepada {} atas Tutup Buku {}",
        dist_number, req.total_amount, req.recipient_name.trim(), closing_number
    );
    let _ = sqlx::query(
        r#"
        INSERT INTO audit_logs (
            user_id, action, entity_name, entity_id, new_values, created_at
        ) VALUES (
            $1, 'DIVIDEND_DISTRIBUTION', 'dividend_distributions', $2, $3, CURRENT_TIMESTAMP
        )
        "#,
    )
    .bind(user_id)
    .bind(dist_id.to_string())
    .bind(serde_json::json!({
        "distribution_number": dist_number,
        "amount": req.total_amount,
        "recipient": req.recipient_name.trim(),
        "notes": audit_desc
    }))
    .execute(&mut *tx)
    .await;

    tx.commit()
        .await
        .map_err(|e| format!("Gagal menyelesaikan transaksi: {}", e))?;

    get_dividend_by_id(pool, dist_id).await
}

pub async fn cancel_dividend(
    pool: &PgPool,
    id: Uuid,
    user_id: Option<Uuid>,
) -> Result<bool, String> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| format!("Gagal memulai transaksi: {}", e))?;

    let div_row = sqlx::query(
        "SELECT distribution_number, journal_entry_id, total_amount, recipient_name FROM dividend_distributions WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| format!("Gagal memeriksa data dividen: {}", e))?
    .ok_or_else(|| "Data pembagian dividen tidak ditemukan.".to_string())?;

    let dist_num: String = div_row.get("distribution_number");
    let journal_entry_id: Option<Uuid> = div_row.get("journal_entry_id");
    let total_amount: Decimal = div_row.get("total_amount");
    let recipient: String = div_row.get("recipient_name");

    // 1. Hapus dokumen dividen
    sqlx::query("DELETE FROM dividend_distributions WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|e| format!("Gagal menghapus data dividen: {}", e))?;

    // 2. Hapus jurnal akuntansi terkait
    if let Some(jid) = journal_entry_id {
        sqlx::query("DELETE FROM journal_lines WHERE journal_id = $1")
            .bind(jid)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Gagal menghapus baris jurnal dividen: {}", e))?;

        sqlx::query("DELETE FROM journal_entries WHERE id = $1")
            .bind(jid)
            .execute(&mut *tx)
            .await
            .map_err(|e| format!("Gagal menghapus header jurnal dividen: {}", e))?;
    }

    // 3. Catat audit log pembatalan
    let audit_desc = format!(
        "Pembatalan pembagian dividen {} sebesar Rp {} (Penerima: {})",
        dist_num, total_amount, recipient
    );
    let _ = sqlx::query(
        r#"
        INSERT INTO audit_logs (
            user_id, action, entity_name, entity_id, new_values, created_at
        ) VALUES (
            $1, 'DIVIDEND_CANCELLED', 'dividend_distributions', $2, $3, CURRENT_TIMESTAMP
        )
        "#,
    )
    .bind(user_id)
    .bind(id.to_string())
    .bind(serde_json::json!({
        "distribution_number": dist_num,
        "amount": total_amount,
        "recipient": recipient,
        "notes": audit_desc
    }))
    .execute(&mut *tx)
    .await;

    tx.commit()
        .await
        .map_err(|e| format!("Gagal menyelesaikan pembatalan dividen: {}", e))?;

    Ok(true)
}
