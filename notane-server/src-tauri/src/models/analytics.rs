use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct AnalyticsQuery {
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub outlet_id: Option<Uuid>,
}

#[derive(Debug, Serialize, Clone)]
pub struct SalesSummaryKpi {
    pub gross_sales: Decimal,
    pub total_discounts: Decimal,
    pub total_tax: Decimal,
    pub net_sales: Decimal,
    pub total_cogs: Decimal,
    pub gross_profit: Decimal,
    pub gross_margin_pct: Decimal,
    pub transaction_count: i64,
    pub average_order_value: Decimal,
    pub total_items_sold: Decimal,
}

#[derive(Debug, Serialize, Clone)]
pub struct PurchasesSummaryKpi {
    pub total_purchases: Decimal,
    pub purchase_count: i64,
    pub paid_purchases: Decimal,
    pub unpaid_purchases: Decimal,
}

#[derive(Debug, Serialize, Clone)]
pub struct ExpensesSummaryKpi {
    pub total_expenses: Decimal,
    pub expense_count: i64,
}

#[derive(Debug, Serialize, Clone)]
pub struct OtherIncomeSummaryKpi {
    pub total_other_income: Decimal,
    pub other_income_count: i64,
}

#[derive(Debug, Serialize, Clone)]
pub struct NetProfitSummaryKpi {
    pub net_operating_profit: Decimal,
    pub net_margin_pct: Decimal,
}

#[derive(Debug, Serialize, Clone)]
pub struct DailyTrendPoint {
    pub date: String,
    pub day_name: String,
    pub net_sales: Decimal,
    pub cogs: Decimal,
    pub gross_profit: Decimal,
    pub purchases: Decimal,
    pub expenses: Decimal,
    pub other_income: Decimal,
    pub net_profit: Decimal,
    pub transaction_count: i64,
}

#[derive(Debug, Serialize, Clone)]
pub struct CategoryDistributionItem {
    pub name: String,
    pub amount: Decimal,
    pub count: i64,
    pub percentage: Decimal,
}

#[derive(Debug, Serialize, Clone)]
pub struct PaymentMethodDistributionItem {
    pub method: String,
    pub label: String,
    pub amount: Decimal,
    pub count: i64,
    pub percentage: Decimal,
}

#[derive(Debug, Serialize, Clone)]
pub struct TopProductItem {
    pub product_id: Option<Uuid>,
    pub product_name: String,
    pub sku: String,
    pub total_qty: Decimal,
    pub total_sales: Decimal,
    pub total_profit: Decimal,
    pub margin_pct: Decimal,
}

#[derive(Debug, Serialize, Clone)]
pub struct AnalyticsOverviewResponse {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub outlet_id: Option<Uuid>,
    pub outlet_name: Option<String>,
    pub sales: SalesSummaryKpi,
    pub purchases: PurchasesSummaryKpi,
    pub expenses: ExpensesSummaryKpi,
    pub other_income: OtherIncomeSummaryKpi,
    pub profit: NetProfitSummaryKpi,
    pub trends: Vec<DailyTrendPoint>,
    pub expense_categories: Vec<CategoryDistributionItem>,
    pub other_income_categories: Vec<CategoryDistributionItem>,
    pub payment_methods: Vec<PaymentMethodDistributionItem>,
    pub top_products: Vec<TopProductItem>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AnalyticsPaginationQuery {
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub outlet_id: Option<Uuid>,
    pub search: Option<String>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ProductsAnalyticsResponse {
    pub items: Vec<TopProductItem>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
    pub summary_qty: Decimal,
    pub summary_sales: Decimal,
    pub summary_profit: Decimal,
}

#[derive(Debug, Serialize, Clone)]
pub struct CategoryAnalyticsResponse {
    pub items: Vec<CategoryDistributionItem>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
    pub total_amount: Decimal,
}

#[derive(Debug, Serialize, Clone)]
pub struct DailyLedgerAnalyticsResponse {
    pub items: Vec<DailyTrendPoint>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
    pub summary_sales: Decimal,
    pub summary_cogs: Decimal,
    pub summary_gross_profit: Decimal,
    pub summary_purchases: Decimal,
    pub summary_expenses: Decimal,
    pub summary_other_income: Decimal,
    pub summary_net_profit: Decimal,
    pub summary_transactions: i64,
}

