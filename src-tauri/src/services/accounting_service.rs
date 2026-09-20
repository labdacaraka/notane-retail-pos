use crate::models::accounting::{
    AccountSummaryItem, ChartOfAccount, IncomeStatementReport, JournalEntry, JournalEntryDetail,
    JournalLineDetail,
};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqlx::{PgPool, Row};

pub async fn list_chart_of_accounts(pool: &PgPool) -> Result<Vec<ChartOfAccount>, String> {
    sqlx::query_as::<_, ChartOfAccount>(
        "SELECT * FROM chart_of_accounts ORDER BY code ASC",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))
}

pub async fn list_journal_entries(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<JournalEntryDetail>, String> {
    let entries = sqlx::query_as::<_, JournalEntry>(
        "SELECT * FROM journal_entries ORDER BY created_at DESC LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?;

    let mut result = Vec::new();

    for entry in entries {
        let lines = sqlx::query(
            "SELECT jl.id, jl.account_id, coa.code as account_code, coa.name as account_name,
                   jl.debit, jl.credit, jl.memo
            FROM journal_lines jl
            JOIN chart_of_accounts coa ON jl.account_id = coa.id
            WHERE jl.journal_id = $1
            ORDER BY jl.debit DESC",
        )
        .bind(entry.id)
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
            })
            .collect();

        result.push(JournalEntryDetail {
            entry,
            lines: line_details,
        });
    }

    Ok(result)
}

pub async fn get_income_statement(
    pool: &PgPool,
    start_date: NaiveDate,
    end_date: NaiveDate,
) -> Result<IncomeStatementReport, String> {
    let revenue_rows = sqlx::query(
        "SELECT coa.code, coa.name,
               COALESCE(SUM(jl.credit - jl.debit), 0) as amount
        FROM journal_lines jl
        JOIN journal_entries je ON jl.journal_id = je.id
        JOIN chart_of_accounts coa ON jl.account_id = coa.id
        JOIN account_categories ac ON coa.account_category_id = ac.id
        WHERE ac.name = 'REVENUE' AND je.journal_date >= $1 AND je.journal_date <= $2
        GROUP BY coa.code, coa.name",
    )
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

    let expense_rows = sqlx::query(
        "SELECT coa.code, coa.name,
               COALESCE(SUM(jl.debit - jl.credit), 0) as amount
        FROM journal_lines jl
        JOIN journal_entries je ON jl.journal_id = je.id
        JOIN chart_of_accounts coa ON jl.account_id = coa.id
        JOIN account_categories ac ON coa.account_category_id = ac.id
        WHERE ac.name = 'EXPENSE' AND je.journal_date >= $1 AND je.journal_date <= $2
        GROUP BY coa.code, coa.name",
    )
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
        total_revenue,
        total_cogs,
        gross_profit,
        total_operating_expenses,
        net_profit,
        revenue_details,
        expense_details,
    })
}
