use std::collections::HashMap;
use chrono::{Datelike, Duration, Local, NaiveDate};
use rust_decimal::Decimal;
use sqlx::{PgPool, Row};

use crate::models::analytics::{
    AnalyticsOverviewResponse, AnalyticsPaginationQuery, AnalyticsQuery, CategoryAnalyticsResponse,
    CategoryDistributionItem, DailyLedgerAnalyticsResponse, DailyTrendPoint, ExpensesSummaryKpi,
    NetProfitSummaryKpi, OtherIncomeSummaryKpi, PaymentMethodDistributionItem,
    ProductsAnalyticsResponse, PurchasesSummaryKpi, SalesSummaryKpi, TopProductItem,
};

fn get_day_name_id(date: NaiveDate) -> &'static str {
    match date.weekday() {
        chrono::Weekday::Mon => "Sen",
        chrono::Weekday::Tue => "Sel",
        chrono::Weekday::Wed => "Rab",
        chrono::Weekday::Thu => "Kam",
        chrono::Weekday::Fri => "Jum",
        chrono::Weekday::Sat => "Sab",
        chrono::Weekday::Sun => "Min",
    }
}

fn map_payment_method_label(code: &str) -> &'static str {
    match code.to_uppercase().as_str() {
        "CASH" => "Tunai (Cash)",
        "QRIS" => "QRIS",
        "TRANSFER" | "BANK_TRANSFER" => "Transfer Bank",
        "DEBIT" | "DEBIT_CARD" => "Kartu Debit (EDC)",
        "CREDIT" | "CREDIT_CARD" => "Kartu Kredit (EDC)",
        "RECEIVABLE" | "DEBT" | "PIUTANG" => "Piutang Member / Kasbon",
        "POINT" | "POINTS" => "Poin Member",
        "VOUCHER" => "Voucher Promo",
        _ => "Metode Lain",
    }
}

pub async fn get_analytics_overview(
    pool: &PgPool,
    query: AnalyticsQuery,
) -> Result<AnalyticsOverviewResponse, String> {
    let today = Local::now().date_naive();
    let end_date = query.end_date.unwrap_or(today);
    let start_date = query.start_date.unwrap_or_else(|| end_date - Duration::days(29));

    let (start_date, end_date) = if start_date > end_date {
        (end_date, start_date)
    } else {
        (start_date, end_date)
    };

    // Ambil nama outlet jika spesifik
    let outlet_name: Option<String> = if let Some(oid) = query.outlet_id {
        sqlx::query_scalar("SELECT name FROM outlets WHERE id = $1")
            .bind(oid)
            .fetch_optional(pool)
            .await
            .unwrap_or(None)
    } else {
        None
    };

    // 1. SALES KPI SUMMARY
    let sales_row = sqlx::query(
        "SELECT 
            COUNT(*)::bigint as tx_count,
            COALESCE(SUM(subtotal), 0) as gross_sales,
            COALESCE(SUM(discount_amount), 0) as total_discounts,
            COALESCE(SUM(tax_amount), 0) as total_tax,
            COALESCE(SUM(total_amount), 0) as net_sales,
            COALESCE(SUM(total_cost_amount), 0) as total_cogs
         FROM sales s
         WHERE s.status = 'COMPLETED'
           AND DATE(s.created_at) >= $1
           AND DATE(s.created_at) <= $2
           AND ($3::uuid IS NULL OR s.outlet_id = $3)",
    )
    .bind(start_date)
    .bind(end_date)
    .bind(query.outlet_id)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Gagal memuat ringkasan penjualan: {}", e))?;

    let tx_count: i64 = sales_row.get("tx_count");
    let gross_sales: Decimal = sales_row.get("gross_sales");
    let total_discounts: Decimal = sales_row.get("total_discounts");
    let total_tax: Decimal = sales_row.get("total_tax");
    let net_sales: Decimal = sales_row.get("net_sales");
    let total_cogs: Decimal = sales_row.get("total_cogs");
    let gross_profit = net_sales - total_cogs;
    let gross_margin_pct = if net_sales > Decimal::ZERO {
        (gross_profit / net_sales * Decimal::from(100)).round_dp(2)
    } else {
        Decimal::ZERO
    };
    let aov = if tx_count > 0 {
        (net_sales / Decimal::from(tx_count)).round_dp(2)
    } else {
        Decimal::ZERO
    };

    let total_items_sold: Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(si.quantity), 0)
         FROM sale_items si
         JOIN sales s ON si.sale_id = s.id
         WHERE s.status = 'COMPLETED'
           AND DATE(s.created_at) >= $1
           AND DATE(s.created_at) <= $2
           AND ($3::uuid IS NULL OR s.outlet_id = $3)",
    )
    .bind(start_date)
    .bind(end_date)
    .bind(query.outlet_id)
    .fetch_one(pool)
    .await
    .unwrap_or(Decimal::ZERO);

    let sales_kpi = SalesSummaryKpi {
        gross_sales,
        total_discounts,
        total_tax,
        net_sales,
        total_cogs,
        gross_profit,
        gross_margin_pct,
        transaction_count: tx_count,
        average_order_value: aov,
        total_items_sold,
    };

    // 2. PURCHASES KPI SUMMARY
    let purchases_row = sqlx::query(
        "SELECT
            COUNT(*)::bigint as purchase_count,
            COALESCE(SUM(total_amount), 0) as total_purchases,
            COALESCE(SUM(paid_amount), 0) as paid_purchases
         FROM purchase_invoices pi
         WHERE pi.payment_status != 'CANCELLED'
           AND pi.invoice_date >= $1
           AND pi.invoice_date <= $2
           AND ($3::uuid IS NULL OR pi.outlet_id = $3)",
    )
    .bind(start_date)
    .bind(end_date)
    .bind(query.outlet_id)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Gagal memuat ringkasan pembelian: {}", e))?;

    let purchase_count: i64 = purchases_row.get("purchase_count");
    let total_purchases: Decimal = purchases_row.get("total_purchases");
    let paid_purchases: Decimal = purchases_row.get("paid_purchases");
    let unpaid_purchases = (total_purchases - paid_purchases).max(Decimal::ZERO);

    let purchases_kpi = PurchasesSummaryKpi {
        total_purchases,
        purchase_count,
        paid_purchases,
        unpaid_purchases,
    };

    // 3. EXPENSES KPI SUMMARY
    let expenses_row = sqlx::query(
        "SELECT
            COUNT(*)::bigint as expense_count,
            COALESCE(SUM(amount), 0) as total_expenses
         FROM expense_transactions e
         WHERE e.status = 'COMPLETED'
           AND e.transaction_date >= $1
           AND e.transaction_date <= $2
           AND ($3::uuid IS NULL OR e.outlet_id = $3)",
    )
    .bind(start_date)
    .bind(end_date)
    .bind(query.outlet_id)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Gagal memuat ringkasan beban: {}", e))?;

    let expense_count: i64 = expenses_row.get("expense_count");
    let total_expenses: Decimal = expenses_row.get("total_expenses");

    let expenses_kpi = ExpensesSummaryKpi {
        total_expenses,
        expense_count,
    };

    // 4. OTHER INCOME KPI SUMMARY
    let other_income_row = sqlx::query(
        "SELECT
            COUNT(*)::bigint as other_income_count,
            COALESCE(SUM(amount), 0) as total_other_income
         FROM other_income_transactions oit
         WHERE oit.status = 'COMPLETED'
           AND oit.transaction_date >= $1
           AND oit.transaction_date <= $2
           AND ($3::uuid IS NULL OR oit.outlet_id = $3)",
    )
    .bind(start_date)
    .bind(end_date)
    .bind(query.outlet_id)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Gagal memuat ringkasan pendapatan lain: {}", e))?;

    let other_income_count: i64 = other_income_row.get("other_income_count");
    let total_other_income: Decimal = other_income_row.get("total_other_income");

    let other_income_kpi = OtherIncomeSummaryKpi {
        total_other_income,
        other_income_count,
    };

    // 5. NET PROFIT KPI
    let net_operating_profit = gross_profit + total_other_income - total_expenses;
    let total_revenue = net_sales + total_other_income;
    let net_margin_pct = if total_revenue > Decimal::ZERO {
        (net_operating_profit / total_revenue * Decimal::from(100)).round_dp(2)
    } else {
        Decimal::ZERO
    };

    let profit_kpi = NetProfitSummaryKpi {
        net_operating_profit,
        net_margin_pct,
    };

    // 6. DAILY TIMESERIES FOR CHARTS
    let (trends, _) = build_daily_trends(pool, start_date, end_date, query.outlet_id).await?;

    // 7. EXPENSE CATEGORIES BREAKDOWN
    let expense_cat_rows = sqlx::query(
        "SELECT COALESCE(ec.name, 'Lain-lain') as name,
                COALESCE(SUM(e.amount), 0) as amount,
                COUNT(e.id)::bigint as count
         FROM expense_transactions e
         LEFT JOIN expense_categories ec ON e.category_id = ec.id
         WHERE e.status = 'COMPLETED'
           AND e.transaction_date >= $1
           AND e.transaction_date <= $2
           AND ($3::uuid IS NULL OR e.outlet_id = $3)
         GROUP BY ec.name
         ORDER BY amount DESC
         LIMIT 10",
    )
    .bind(start_date)
    .bind(end_date)
    .bind(query.outlet_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut expense_categories = Vec::new();
    for r in expense_cat_rows {
        let amt: Decimal = r.get("amount");
        let pct = if total_expenses > Decimal::ZERO {
            (amt / total_expenses * Decimal::from(100)).round_dp(2)
        } else {
            Decimal::ZERO
        };
        expense_categories.push(CategoryDistributionItem {
            name: r.get("name"),
            amount: amt,
            count: r.get("count"),
            percentage: pct,
        });
    }

    // 8. OTHER INCOME CATEGORIES BREAKDOWN
    let other_income_cat_rows = sqlx::query(
        "SELECT COALESCE(oic.name, 'Lain-lain') as name,
                COALESCE(SUM(oit.amount), 0) as amount,
                COUNT(oit.id)::bigint as count
         FROM other_income_transactions oit
         LEFT JOIN other_income_categories oic ON oit.category_id = oic.id
         WHERE oit.status = 'COMPLETED'
           AND oit.transaction_date >= $1
           AND oit.transaction_date <= $2
           AND ($3::uuid IS NULL OR oit.outlet_id = $3)
         GROUP BY oic.name
         ORDER BY amount DESC
         LIMIT 10",
    )
    .bind(start_date)
    .bind(end_date)
    .bind(query.outlet_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut other_income_categories = Vec::new();
    for r in other_income_cat_rows {
        let amt: Decimal = r.get("amount");
        let pct = if total_other_income > Decimal::ZERO {
            (amt / total_other_income * Decimal::from(100)).round_dp(2)
        } else {
            Decimal::ZERO
        };
        other_income_categories.push(CategoryDistributionItem {
            name: r.get("name"),
            amount: amt,
            count: r.get("count"),
            percentage: pct,
        });
    }

    // 9. PAYMENT METHOD DISTRIBUTION
    let payment_rows = sqlx::query(
        "SELECT COALESCE(payment_method, 'CASH') as method,
                COUNT(*)::bigint as tx_count,
                COALESCE(SUM(total_amount), 0) as total_amount
         FROM sales
         WHERE status = 'COMPLETED'
           AND DATE(created_at) >= $1
           AND DATE(created_at) <= $2
           AND ($3::uuid IS NULL OR outlet_id = $3)
         GROUP BY payment_method
         ORDER BY total_amount DESC",
    )
    .bind(start_date)
    .bind(end_date)
    .bind(query.outlet_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut payment_methods = Vec::new();
    for r in payment_rows {
        let method: String = r.get("method");
        let label = map_payment_method_label(&method).to_string();
        let amt: Decimal = r.get("total_amount");
        let pct = if net_sales > Decimal::ZERO {
            (amt / net_sales * Decimal::from(100)).round_dp(2)
        } else {
            Decimal::ZERO
        };
        payment_methods.push(PaymentMethodDistributionItem {
            method,
            label,
            amount: amt,
            count: r.get("tx_count"),
            percentage: pct,
        });
    }

    // 10. TOP SELLING PRODUCTS
    let top_prod_rows = sqlx::query(
        "SELECT si.product_id,
                COALESCE(p.name, si.product_name, 'Produk') as product_name,
                COALESCE(p.sku, '-') as sku,
                COALESCE(SUM(si.quantity), 0) as total_qty,
                COALESCE(SUM(si.subtotal), 0) as total_sales,
                COALESCE(SUM(si.subtotal - (COALESCE(si.cost_price, 0) * si.quantity)), 0) as total_profit
         FROM sale_items si
         JOIN sales s ON si.sale_id = s.id
         LEFT JOIN products p ON si.product_id = p.id
         WHERE s.status = 'COMPLETED'
           AND DATE(s.created_at) >= $1
           AND DATE(s.created_at) <= $2
           AND ($3::uuid IS NULL OR s.outlet_id = $3)
         GROUP BY si.product_id, p.name, si.product_name, p.sku
         ORDER BY total_sales DESC
         LIMIT 10",
    )
    .bind(start_date)
    .bind(end_date)
    .bind(query.outlet_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut top_products = Vec::new();
    for r in top_prod_rows {
        let total_sales: Decimal = r.get("total_sales");
        let total_profit: Decimal = r.get("total_profit");
        let margin_pct = if total_sales > Decimal::ZERO {
            (total_profit / total_sales * Decimal::from(100)).round_dp(2)
        } else {
            Decimal::ZERO
        };

        top_products.push(TopProductItem {
            product_id: r.get("product_id"),
            product_name: r.get("product_name"),
            sku: r.get("sku"),
            total_qty: r.get("total_qty"),
            total_sales,
            total_profit,
            margin_pct,
        });
    }

    Ok(AnalyticsOverviewResponse {
        start_date,
        end_date,
        outlet_id: query.outlet_id,
        outlet_name,
        sales: sales_kpi,
        purchases: purchases_kpi,
        expenses: expenses_kpi,
        other_income: other_income_kpi,
        profit: profit_kpi,
        trends,
        expense_categories,
        other_income_categories,
        payment_methods,
        top_products,
    })
}

/// Helper untuk menghasilkan deret harian finansial lengkap
pub async fn build_daily_trends(
    pool: &PgPool,
    start_date: NaiveDate,
    end_date: NaiveDate,
    outlet_id: Option<uuid::Uuid>,
) -> Result<(Vec<DailyTrendPoint>, (Decimal, Decimal, Decimal, Decimal, Decimal, Decimal, Decimal, i64)), String> {
    // 1. Sales daily map
    let sales_daily_rows = sqlx::query(
        "SELECT DATE(created_at) as tx_date,
                COALESCE(SUM(total_amount), 0) as net_sales,
                COALESCE(SUM(total_cost_amount), 0) as cogs,
                COUNT(*)::bigint as count
         FROM sales
         WHERE status = 'COMPLETED'
           AND DATE(created_at) >= $1
           AND DATE(created_at) <= $2
           AND ($3::uuid IS NULL OR outlet_id = $3)
         GROUP BY DATE(created_at)",
    )
    .bind(start_date)
    .bind(end_date)
    .bind(outlet_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut sales_map: HashMap<NaiveDate, (Decimal, Decimal, i64)> = HashMap::new();
    for r in sales_daily_rows {
        let d: NaiveDate = r.get("tx_date");
        sales_map.insert(d, (r.get("net_sales"), r.get("cogs"), r.get("count")));
    }

    // 2. Purchases daily map
    let purchases_daily_rows = sqlx::query(
        "SELECT invoice_date as tx_date,
                COALESCE(SUM(total_amount), 0) as total_purchases
         FROM purchase_invoices
         WHERE payment_status != 'CANCELLED'
           AND invoice_date >= $1
           AND invoice_date <= $2
           AND ($3::uuid IS NULL OR outlet_id = $3)
         GROUP BY invoice_date",
    )
    .bind(start_date)
    .bind(end_date)
    .bind(outlet_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut purchases_map: HashMap<NaiveDate, Decimal> = HashMap::new();
    for r in purchases_daily_rows {
        let d: NaiveDate = r.get("tx_date");
        purchases_map.insert(d, r.get("total_purchases"));
    }

    // 3. Expenses daily map
    let expenses_daily_rows = sqlx::query(
        "SELECT transaction_date as tx_date,
                COALESCE(SUM(amount), 0) as total_expenses
         FROM expense_transactions
         WHERE status = 'COMPLETED'
           AND transaction_date >= $1
           AND transaction_date <= $2
           AND ($3::uuid IS NULL OR outlet_id = $3)
         GROUP BY transaction_date",
    )
    .bind(start_date)
    .bind(end_date)
    .bind(outlet_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut expenses_map: HashMap<NaiveDate, Decimal> = HashMap::new();
    for r in expenses_daily_rows {
        let d: NaiveDate = r.get("tx_date");
        expenses_map.insert(d, r.get("total_expenses"));
    }

    // 4. Other income daily map
    let other_income_daily_rows = sqlx::query(
        "SELECT transaction_date as tx_date,
                COALESCE(SUM(amount), 0) as total_other_income
         FROM other_income_transactions
         WHERE status = 'COMPLETED'
           AND transaction_date >= $1
           AND transaction_date <= $2
           AND ($3::uuid IS NULL OR outlet_id = $3)
         GROUP BY transaction_date",
    )
    .bind(start_date)
    .bind(end_date)
    .bind(outlet_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    let mut other_income_map: HashMap<NaiveDate, Decimal> = HashMap::new();
    for r in other_income_daily_rows {
        let d: NaiveDate = r.get("tx_date");
        other_income_map.insert(d, r.get("total_other_income"));
    }

    // Generate unbroken sequence of dates from start_date to end_date
    let mut trends = Vec::new();
    let mut curr = start_date;
    let mut sum_sales = Decimal::ZERO;
    let mut sum_cogs = Decimal::ZERO;
    let mut sum_gross_profit = Decimal::ZERO;
    let mut sum_purchases = Decimal::ZERO;
    let mut sum_expenses = Decimal::ZERO;
    let mut sum_other_income = Decimal::ZERO;
    let mut sum_net_profit = Decimal::ZERO;
    let mut sum_tx_count = 0i64;

    while curr <= end_date {
        let (d_net_sales, d_cogs, d_tx_count) = sales_map.get(&curr).cloned().unwrap_or((Decimal::ZERO, Decimal::ZERO, 0));
        let d_gross_profit = d_net_sales - d_cogs;
        let d_purchases = purchases_map.get(&curr).cloned().unwrap_or(Decimal::ZERO);
        let d_expenses = expenses_map.get(&curr).cloned().unwrap_or(Decimal::ZERO);
        let d_other_income = other_income_map.get(&curr).cloned().unwrap_or(Decimal::ZERO);
        let d_net_profit = d_gross_profit + d_other_income - d_expenses;

        sum_sales += d_net_sales;
        sum_cogs += d_cogs;
        sum_gross_profit += d_gross_profit;
        sum_purchases += d_purchases;
        sum_expenses += d_expenses;
        sum_other_income += d_other_income;
        sum_net_profit += d_net_profit;
        sum_tx_count += d_tx_count;

        trends.push(DailyTrendPoint {
            date: curr.format("%Y-%m-%d").to_string(),
            day_name: get_day_name_id(curr).to_string(),
            net_sales: d_net_sales,
            cogs: d_cogs,
            gross_profit: d_gross_profit,
            purchases: d_purchases,
            expenses: d_expenses,
            other_income: d_other_income,
            net_profit: d_net_profit,
            transaction_count: d_tx_count,
        });

        curr += Duration::days(1);
    }

    Ok((
        trends,
        (
            sum_sales,
            sum_cogs,
            sum_gross_profit,
            sum_purchases,
            sum_expenses,
            sum_other_income,
            sum_net_profit,
            sum_tx_count,
        ),
    ))
}

/// Endpoint Tab Penjualan Produk: Server-side pagination, sorting, dan filtering
pub async fn get_products_analytics(
    pool: &PgPool,
    query: AnalyticsPaginationQuery,
) -> Result<ProductsAnalyticsResponse, String> {
    let today = Local::now().date_naive();
    let end_date = query.end_date.unwrap_or(today);
    let start_date = query.start_date.unwrap_or_else(|| end_date - Duration::days(29));
    let (start_date, end_date) = if start_date > end_date {
        (end_date, start_date)
    } else {
        (start_date, end_date)
    };

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(10).clamp(1, 100);
    let offset = (page - 1) * per_page;

    let search_clean = query
        .search
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| format!("%{}%", s.to_lowercase()));

    let sort_col = match query.sort_by.as_deref().unwrap_or("sales") {
        "qty" => "total_qty",
        "profit" | "gross_profit" => "total_profit",
        "margin" | "margin_pct" => "margin_pct",
        "name" | "product_name" => "product_name",
        _ => "total_sales",
    };
    let sort_dir = match query.sort_order.as_deref().unwrap_or("desc").to_lowercase().as_str() {
        "asc" => "ASC",
        _ => "DESC",
    };

    // 1. Hitung total records unik dan summary agregat produk
    let count_row = sqlx::query(
        "WITH aggregated AS (
            SELECT 
                si.product_id,
                COALESCE(p.name, 'Item') as product_name,
                COALESCE(p.sku, COALESCE(p.barcode, '-')) as sku,
                COALESCE(SUM(si.quantity), 0) as total_qty,
                COALESCE(SUM(si.subtotal), 0) as total_sales,
                COALESCE(SUM(si.subtotal - (COALESCE(si.unit_cost_price, 0) * si.quantity)), 0) as total_profit
            FROM sale_items si
            JOIN sales s ON si.sale_id = s.id
            LEFT JOIN products p ON si.product_id = p.id
            WHERE s.status = 'COMPLETED'
              AND DATE(s.created_at) >= $1
              AND DATE(s.created_at) <= $2
              AND ($3::uuid IS NULL OR s.outlet_id = $3)
              AND ($4::text IS NULL OR LOWER(COALESCE(p.name, '')) LIKE $4 OR LOWER(COALESCE(p.sku, '')) LIKE $4 OR LOWER(COALESCE(p.barcode, '')) LIKE $4)
            GROUP BY si.product_id, p.name, p.sku, p.barcode
        )
        SELECT 
            COUNT(*)::bigint as total_count,
            COALESCE(SUM(total_qty), 0) as sum_qty,
            COALESCE(SUM(total_sales), 0) as sum_sales,
            COALESCE(SUM(total_profit), 0) as sum_profit
        FROM aggregated",
    )
    .bind(start_date)
    .bind(end_date)
    .bind(query.outlet_id)
    .bind(&search_clean)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Gagal menghitung statistik produk: {}", e))?;

    let total: i64 = count_row.get("total_count");
    let summary_qty: Decimal = count_row.get("sum_qty");
    let summary_sales: Decimal = count_row.get("sum_sales");
    let summary_profit: Decimal = count_row.get("sum_profit");
    let total_pages = if total == 0 { 1 } else { ((total as f64) / (per_page as f64)).ceil() as i64 };

    // 2. Query paginated data
    let data_query = format!(
        "SELECT 
            si.product_id,
            COALESCE(p.name, 'Item') as product_name,
            COALESCE(p.sku, COALESCE(p.barcode, '-')) as sku,
            COALESCE(SUM(si.quantity), 0) as total_qty,
            COALESCE(SUM(si.subtotal), 0) as total_sales,
            COALESCE(SUM(si.subtotal - (COALESCE(si.unit_cost_price, 0) * si.quantity)), 0) as total_profit,
            CASE 
                WHEN COALESCE(SUM(si.subtotal), 0) > 0 
                THEN ROUND((COALESCE(SUM(si.subtotal - (COALESCE(si.unit_cost_price, 0) * si.quantity)), 0) / SUM(si.subtotal) * 100)::numeric, 2)
                ELSE 0 
            END as margin_pct
         FROM sale_items si
         JOIN sales s ON si.sale_id = s.id
         LEFT JOIN products p ON si.product_id = p.id
         WHERE s.status = 'COMPLETED'
           AND DATE(s.created_at) >= $1
           AND DATE(s.created_at) <= $2
           AND ($3::uuid IS NULL OR s.outlet_id = $3)
           AND ($4::text IS NULL OR LOWER(COALESCE(p.name, '')) LIKE $4 OR LOWER(COALESCE(p.sku, '')) LIKE $4 OR LOWER(COALESCE(p.barcode, '')) LIKE $4)
         GROUP BY si.product_id, p.name, p.sku, p.barcode
         ORDER BY {sort_col} {sort_dir}
         LIMIT $5 OFFSET $6"
    );

    let rows = sqlx::query(&data_query)
        .bind(start_date)
        .bind(end_date)
        .bind(query.outlet_id)
        .bind(&search_clean)
        .bind(per_page)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Gagal memuat daftar produk: {}", e))?;

    let mut items = Vec::new();
    for r in rows {
        items.push(TopProductItem {
            product_id: r.get("product_id"),
            product_name: r.get("product_name"),
            sku: r.get("sku"),
            total_qty: r.get("total_qty"),
            total_sales: r.get("total_sales"),
            total_profit: r.get("total_profit"),
            margin_pct: r.get("margin_pct"),
        });
    }

    Ok(ProductsAnalyticsResponse {
        items,
        total,
        page,
        per_page,
        total_pages,
        summary_qty,
        summary_sales,
        summary_profit,
    })
}

/// Endpoint Tab Beban Operasional: Server-side pagination, sorting, dan filtering
pub async fn get_expenses_analytics(
    pool: &PgPool,
    query: AnalyticsPaginationQuery,
) -> Result<CategoryAnalyticsResponse, String> {
    let today = Local::now().date_naive();
    let end_date = query.end_date.unwrap_or(today);
    let start_date = query.start_date.unwrap_or_else(|| end_date - Duration::days(29));
    let (start_date, end_date) = if start_date > end_date {
        (end_date, start_date)
    } else {
        (start_date, end_date)
    };

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(10).clamp(1, 100);
    let offset = (page - 1) * per_page;

    let search_clean = query
        .search
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| format!("%{}%", s.to_lowercase()));

    let sort_col = match query.sort_by.as_deref().unwrap_or("amount") {
        "count" => "count",
        "name" => "name",
        _ => "amount",
    };
    let sort_dir = match query.sort_order.as_deref().unwrap_or("desc").to_lowercase().as_str() {
        "asc" => "ASC",
        _ => "DESC",
    };

    let sum_row = sqlx::query(
        "WITH aggregated AS (
            SELECT 
                COALESCE(ec.name, 'Lain-lain') as name,
                COALESCE(SUM(e.amount), 0) as amount,
                COUNT(e.id)::bigint as count
            FROM expense_transactions e
            LEFT JOIN expense_categories ec ON e.category_id = ec.id
            WHERE e.status = 'COMPLETED'
              AND e.transaction_date >= $1
              AND e.transaction_date <= $2
              AND ($3::uuid IS NULL OR e.outlet_id = $3)
              AND ($4::text IS NULL OR LOWER(COALESCE(ec.name, 'Lain-lain')) LIKE $4)
            GROUP BY ec.name
        )
        SELECT 
            COUNT(*)::bigint as total_count,
            COALESCE(SUM(amount), 0) as total_amount
        FROM aggregated",
    )
    .bind(start_date)
    .bind(end_date)
    .bind(query.outlet_id)
    .bind(&search_clean)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Gagal menghitung statistik beban: {}", e))?;

    let total: i64 = sum_row.get("total_count");
    let total_amount: Decimal = sum_row.get("total_amount");
    let total_pages = if total == 0 { 1 } else { ((total as f64) / (per_page as f64)).ceil() as i64 };

    let data_query = format!(
        "SELECT 
            COALESCE(ec.name, 'Lain-lain') as name,
            COALESCE(SUM(e.amount), 0) as amount,
            COUNT(e.id)::bigint as count
         FROM expense_transactions e
         LEFT JOIN expense_categories ec ON e.category_id = ec.id
         WHERE e.status = 'COMPLETED'
           AND e.transaction_date >= $1
           AND e.transaction_date <= $2
           AND ($3::uuid IS NULL OR e.outlet_id = $3)
           AND ($4::text IS NULL OR LOWER(COALESCE(ec.name, 'Lain-lain')) LIKE $4)
         GROUP BY ec.name
         ORDER BY {sort_col} {sort_dir}
         LIMIT $5 OFFSET $6"
    );

    let rows = sqlx::query(&data_query)
        .bind(start_date)
        .bind(end_date)
        .bind(query.outlet_id)
        .bind(&search_clean)
        .bind(per_page)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Gagal memuat kategori beban: {}", e))?;

    let mut items = Vec::new();
    for r in rows {
        let amt: Decimal = r.get("amount");
        let pct = if total_amount > Decimal::ZERO {
            (amt / total_amount * Decimal::from(100)).round_dp(2)
        } else {
            Decimal::ZERO
        };
        items.push(CategoryDistributionItem {
            name: r.get("name"),
            amount: amt,
            count: r.get("count"),
            percentage: pct,
        });
    }

    Ok(CategoryAnalyticsResponse {
        items,
        total,
        page,
        per_page,
        total_pages,
        total_amount,
    })
}

/// Endpoint Tab Pendapatan Lain-lain: Server-side pagination, sorting, dan filtering
pub async fn get_other_income_analytics(
    pool: &PgPool,
    query: AnalyticsPaginationQuery,
) -> Result<CategoryAnalyticsResponse, String> {
    let today = Local::now().date_naive();
    let end_date = query.end_date.unwrap_or(today);
    let start_date = query.start_date.unwrap_or_else(|| end_date - Duration::days(29));
    let (start_date, end_date) = if start_date > end_date {
        (end_date, start_date)
    } else {
        (start_date, end_date)
    };

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(10).clamp(1, 100);
    let offset = (page - 1) * per_page;

    let search_clean = query
        .search
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| format!("%{}%", s.to_lowercase()));

    let sort_col = match query.sort_by.as_deref().unwrap_or("amount") {
        "count" => "count",
        "name" => "name",
        _ => "amount",
    };
    let sort_dir = match query.sort_order.as_deref().unwrap_or("desc").to_lowercase().as_str() {
        "asc" => "ASC",
        _ => "DESC",
    };

    let sum_row = sqlx::query(
        "WITH aggregated AS (
            SELECT 
                COALESCE(oic.name, 'Lain-lain') as name,
                COALESCE(SUM(oit.amount), 0) as amount,
                COUNT(oit.id)::bigint as count
            FROM other_income_transactions oit
            LEFT JOIN other_income_categories oic ON oit.category_id = oic.id
            WHERE oit.status = 'COMPLETED'
              AND oit.transaction_date >= $1
              AND oit.transaction_date <= $2
              AND ($3::uuid IS NULL OR oit.outlet_id = $3)
              AND ($4::text IS NULL OR LOWER(COALESCE(oic.name, 'Lain-lain')) LIKE $4)
            GROUP BY oic.name
        )
        SELECT 
            COUNT(*)::bigint as total_count,
            COALESCE(SUM(amount), 0) as total_amount
        FROM aggregated",
    )
    .bind(start_date)
    .bind(end_date)
    .bind(query.outlet_id)
    .bind(&search_clean)
    .fetch_one(pool)
    .await
    .map_err(|e| format!("Gagal menghitung statistik pendapatan lain: {}", e))?;

    let total: i64 = sum_row.get("total_count");
    let total_amount: Decimal = sum_row.get("total_amount");
    let total_pages = if total == 0 { 1 } else { ((total as f64) / (per_page as f64)).ceil() as i64 };

    let data_query = format!(
        "SELECT 
            COALESCE(oic.name, 'Lain-lain') as name,
            COALESCE(SUM(oit.amount), 0) as amount,
            COUNT(oit.id)::bigint as count
         FROM other_income_transactions oit
         LEFT JOIN other_income_categories oic ON oit.category_id = oic.id
         WHERE oit.status = 'COMPLETED'
           AND oit.transaction_date >= $1
           AND oit.transaction_date <= $2
           AND ($3::uuid IS NULL OR oit.outlet_id = $3)
           AND ($4::text IS NULL OR LOWER(COALESCE(oic.name, 'Lain-lain')) LIKE $4)
         GROUP BY oic.name
         ORDER BY {sort_col} {sort_dir}
         LIMIT $5 OFFSET $6"
    );

    let rows = sqlx::query(&data_query)
        .bind(start_date)
        .bind(end_date)
        .bind(query.outlet_id)
        .bind(&search_clean)
        .bind(per_page)
        .bind(offset)
        .fetch_all(pool)
        .await
        .map_err(|e| format!("Gagal memuat kategori pendapatan lain: {}", e))?;

    let mut items = Vec::new();
    for r in rows {
        let amt: Decimal = r.get("amount");
        let pct = if total_amount > Decimal::ZERO {
            (amt / total_amount * Decimal::from(100)).round_dp(2)
        } else {
            Decimal::ZERO
        };
        items.push(CategoryDistributionItem {
            name: r.get("name"),
            amount: amt,
            count: r.get("count"),
            percentage: pct,
        });
    }

    Ok(CategoryAnalyticsResponse {
        items,
        total,
        page,
        per_page,
        total_pages,
        total_amount,
    })
}

/// Endpoint Tab Buku Harian Finansial: Server-side pagination, sorting, dan filtering
pub async fn get_daily_ledger_analytics(
    pool: &PgPool,
    query: AnalyticsPaginationQuery,
) -> Result<DailyLedgerAnalyticsResponse, String> {
    let today = Local::now().date_naive();
    let end_date = query.end_date.unwrap_or(today);
    let start_date = query.start_date.unwrap_or_else(|| end_date - Duration::days(29));
    let (start_date, end_date) = if start_date > end_date {
        (end_date, start_date)
    } else {
        (start_date, end_date)
    };

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(10).clamp(1, 100);

    // Ambil data deret harian lengkap beserta ringkasan periodenya
    let (trends, summary) = build_daily_trends(pool, start_date, end_date, query.outlet_id).await?;

    let search_clean = query
        .search
        .as_deref()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty());

    // Filter server-side
    let mut filtered_trends: Vec<DailyTrendPoint> = trends
        .into_iter()
        .filter(|t| {
            if let Some(ref q) = search_clean {
                t.date.to_lowercase().contains(q) || t.day_name.to_lowercase().contains(q)
            } else {
                true
            }
        })
        .collect();

    // Sort server-side
    let sort_col = query.sort_by.as_deref().unwrap_or("date");
    let is_asc = query
        .sort_order
        .as_deref()
        .unwrap_or("desc")
        .eq_ignore_ascii_case("asc");

    filtered_trends.sort_by(|a, b| {
        let ord = match sort_col {
            "net_sales" => a.net_sales.cmp(&b.net_sales),
            "cogs" => a.cogs.cmp(&b.cogs),
            "gross_profit" => a.gross_profit.cmp(&b.gross_profit),
            "purchases" => a.purchases.cmp(&b.purchases),
            "expenses" => a.expenses.cmp(&b.expenses),
            "other_income" => a.other_income.cmp(&b.other_income),
            "net_profit" => a.net_profit.cmp(&b.net_profit),
            "transaction_count" | "transactions" => a.transaction_count.cmp(&b.transaction_count),
            _ => a.date.cmp(&b.date),
        };
        if is_asc {
            ord
        } else {
            ord.reverse()
        }
    });

    let total = filtered_trends.len() as i64;
    let total_pages = if total == 0 {
        1
    } else {
        ((total as f64) / (per_page as f64)).ceil() as i64
    };

    let offset = ((page - 1) * per_page) as usize;
    let items = if offset < filtered_trends.len() {
        filtered_trends
            .into_iter()
            .skip(offset)
            .take(per_page as usize)
            .collect()
    } else {
        Vec::new()
    };

    Ok(DailyLedgerAnalyticsResponse {
        items,
        total,
        page,
        per_page,
        total_pages,
        summary_sales: summary.0,
        summary_cogs: summary.1,
        summary_gross_profit: summary.2,
        summary_purchases: summary.3,
        summary_expenses: summary.4,
        summary_other_income: summary.5,
        summary_net_profit: summary.6,
        summary_transactions: summary.7,
    })
}
