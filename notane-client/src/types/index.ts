export interface UserProfile {
  id: string
  role_id: number
  role_name: string
  username: string
  full_name: string
  email?: string
  phone?: string
  permissions: string[]
  outlet_ids?: string[]
  has_biometric?: boolean
  has_face_recognition?: boolean
  face_photo_preview?: string | null
  biometric_credential_id?: string | null
}

export type SupervisorBiometricMode = 'OFF' | 'FINGERPRINT' | 'FACE_RECOGNITION' | 'ANY'
export type SupervisorBiometricPolicy = 'OPTIONAL' | 'STRICT'

export interface BiometricSettings {
  supervisor_biometric_mode: SupervisorBiometricMode
  supervisor_biometric_policy: SupervisorBiometricPolicy
  face_match_threshold: number
}

export interface ProductUnitDetail {
  id: string
  unit_id: string
  unit_code: string
  unit_name: string
  conversion_factor: number
  selling_price: number
  barcode?: string
}

export interface ProductPriceTier {
  id: string
  min_quantity: number
  tier_price: number
}

export interface Product {
  id: string
  category_id: string
  base_unit_id: string
  sku: string
  barcode: string
  name: string
  brand_id?: string
  costing_method: string
  cost_price: number
  base_selling_price: number
  min_stock: number
  max_stock: number
  tax_percent: number
  is_weight_embedded: boolean
  is_active: boolean
  auto_adjust_selling_price?: boolean
  markup_percent?: number | null
}

export interface CustomerGroupPriceDetail {
  id: string
  customer_group_id: number
  customer_group_name: string
  product_id: string
  special_price: number
}

export interface ProductOutletPriceDetail {
  id: string
  outlet_id: string
  outlet_code: string
  outlet_name: string
  is_warehouse: boolean
  product_id: string
  cost_price: number
  base_selling_price: number
  last_purchase_price?: number | null
}

export interface ProductStockBatch {
  id: string
  product_id: string
  outlet_id: string
  batch_number: string
  initial_qty: number
  remaining_qty: number
  cost_price: number
  reference_type: string
  reference_id?: string | null
  received_at: string
  created_at: string
  outlet_name?: string
  product_name?: string
}

export interface OutletCustomerGroupPriceDetail {
  id: string
  outlet_id: string
  customer_group_id: number
  customer_group_name: string
  product_id: string
  special_price: number
}

export interface ProductDetail {
  product: Product
  category_name: string
  base_unit_code: string
  base_unit_name: string
  is_fractional: boolean
  brand_name?: string
  units: ProductUnitDetail[]
  price_tiers: ProductPriceTier[]
  customer_group_prices?: CustomerGroupPriceDetail[]
  outlet_customer_group_prices?: OutletCustomerGroupPriceDetail[]
  outlet_prices?: ProductOutletPriceDetail[]
  current_stock: number
  scanned_quantity?: number | string
}

export interface CartItem {
  id: string // local cart unique ID
  product: Product
  selected_unit: ProductUnitDetail
  quantity: number
  unit_price: number
  discount_percent: number
  discount_amount: number
  tax_amount: number
  subtotal: number
  product_detail?: ProductDetail
  current_stock?: number
  flash_sale_id?: string
  flash_sale_name?: string
  original_price?: number
  flash_sale_discount?: number
  member_discount?: number
  is_flash_sale?: boolean
  promo_discount?: number
  promo_name?: string
  promo_id?: string
  is_free_item?: boolean
  is_auto_gift?: boolean
  auto_gift_promo_rule_id?: string
}

export interface EligibleGift {
  promo_id: string
  promo_name: string
  reward_product_id: string
  reward_product_name: string
  reward_product_barcode?: string
  quantity: number
  discount_percent: number
  special_price?: number | null
}

export interface OutOfStockGift {
  promo_id: string
  promo_name: string
  reward_product_id: string
  reward_product_name: string
  reward_product_barcode?: string
  required_quantity: number
  available_stock: number
}

export interface DismissedGiftInfo {
  rule_key: string
  promo_id: string
  promo_name: string
  product_id: string
  product_name: string
}

export interface ActivePromoProduct {
  flash_sale_id: string
  flash_sale_name: string
  product_id: string
  discount_type: 'FIXED_PRICE' | 'PERCENTAGE' | 'NOMINAL_CUT'
  discount_value: number
  promo_price: number
  is_stackable_with_member: boolean
  max_qty_per_transaction?: number | null
  remaining_quota?: number | null
}

export interface FlashSaleItem {
  id: string
  flash_sale_id: string
  product_id: string
  product_sku: string
  product_name: string
  product_barcode?: string
  unit_code: string
  normal_price: number
  discount_type: 'FIXED_PRICE' | 'PERCENTAGE' | 'NOMINAL_CUT'
  discount_value: number
  promo_price: number
  max_qty_per_transaction?: number | null
  promo_quota?: number | null
  sold_qty: number
}

export interface FlashSaleOutlet {
  outlet_id: string
  outlet_code: string
  outlet_name: string
}

export interface FlashSale {
  id: string
  name: string
  description?: string
  start_date: string
  end_date: string
  is_all_day: boolean
  start_time?: string
  end_time?: string
  is_active: boolean
  is_stackable_with_member: boolean
  created_at: string
  total_items?: number
  items_count?: number
  total_outlets?: number
  outlets_count?: number
  is_running?: boolean
  status?: string
  status_label?: string
}

export interface FlashSaleDetailResponse {
  flash_sale: FlashSale
  items: FlashSaleItem[]
  outlets: FlashSaleOutlet[]
  total_products: number
  is_currently_running: boolean
}

export interface CashierShift {
  id: string
  terminal_id: string
  user_id: string
  outlet_id?: string
  outlet_name?: string
  outlet_code?: string
  opened_at: string
  closed_at?: string
  opening_cash: number
  expected_cash?: number
  actual_cash?: number
  discrepancy?: number
  status: 'OPEN' | 'CLOSED'
}

export interface Customer {
  id: string
  customer_group_id?: number
  code: string
  name: string
  phone?: string
  email?: string
  credit_limit: number
  reward_points: number
  group_name?: string
}

export interface CustomerPointLedger {
  id: number
  customer_id: string
  transaction_type: 'EARNED' | 'REDEEMED' | 'CORRECTION' | 'EXPIRED' | string
  reference_id?: string
  reference_number?: string
  points_in: number
  points_out: number
  balance: number
  notes?: string
  created_at: string
  created_by_name?: string
  created_by_username?: string
}

export interface CustomerPointSummary {
  current_balance: number
  total_earned: number
  total_redeemed: number
}

export interface CustomerPointHistoryResponse {
  customer: Customer
  summary: CustomerPointSummary
  ledgers: CustomerPointLedger[]
  pagination: {
    page: number
    page_size: number
    total_items: number
    total_pages: number
  }
}

export interface LoyaltySettings {
  id?: string
  is_enabled: boolean
  point_value_rupiah: number
  min_spend_per_point: number
  max_point_usage_percent: number
  min_redeem_points: number
}

export interface PaymentMethod {
  id: string
  code: string
  name: string
  category: 'CASH' | 'QRIS' | 'EDC' | 'POINTS' | 'TRANSFER'
}

export interface PaymentEntry {
  payment_method_id: string
  method_name: string
  category: string
  amount: number
  points?: number
  reference_number?: string
  payment_proof_url?: string
}

export interface CheckoutResultItem {
  product_id: string
  product_name: string
  unit_id: string
  unit_code: string
  quantity: number
  unit_selling_price: number
  discount_percent: number
  discount_amount: number
  tax_amount: number
  subtotal: number
  original_price?: number
  flash_sale_discount?: number
  member_discount?: number
}

export interface CheckoutResultPayment {
  payment_method_id: string
  payment_method_name: string
  category: string
  amount: number
}

export interface CheckoutResult {
  sale: {
    id: string
    invoice_number: string
    subtotal: number
    discount_amount: number
    tax_amount: number
    total_amount: number
    paid_amount: number
    change_amount: number
    total_flash_sale_discount?: number
    total_member_discount?: number
    total_savings_amount?: number
    created_at: string
  }
  items?: CheckoutResultItem[]
  payments?: CheckoutResultPayment[]
  customer_name?: string
  items_count: number
  change_amount: number
  earned_points: number
  invoice_number: string
  total_savings_amount?: number
  message: string
}

export interface HeldOrder {
  id: string
  timestamp: string
  customer?: Customer
  customer_name: string
  items: CartItem[]
  total_amount: number
}

export interface Outlet {
  id: string
  code: string
  name: string
  address?: string
  phone?: string
  is_warehouse?: boolean
  is_active: boolean
}

export interface InventoryCostingSettings {
  id: string
  default_costing_method: 'WMA' | 'FIFO' | 'FEFO' | 'REPLACEMENT_COST'
  default_auto_adjust_selling_price: boolean
  default_markup_percent: number | null
  created_at?: string
  updated_at?: string
}

export interface ExpenseCategory {
  id: string
  code: string
  name: string
  account_id?: string | null
  account_code?: string | null
  account_name?: string | null
  description?: string | null
  is_active: boolean
  created_at?: string
  updated_at?: string
}

export interface ExpenseTransaction {
  id: string
  transaction_no: string
  outlet_id: string
  outlet_name?: string
  category_id?: string | null
  category_name?: string | null
  expense_account_id: string
  expense_account_code?: string
  expense_account_name?: string
  payment_account_id: string
  payment_account_code?: string
  payment_account_name?: string
  amount: number | string
  transaction_date: string
  recipient_name?: string | null
  notes?: string | null
  reference_no?: string | null
  attachment_url?: string | null
  shift_id?: string | null
  journal_entry_id?: string | null
  created_by?: string | null
  created_by_name?: string | null
  cancelled_by?: string | null
  cancelled_by_name?: string | null
  cancelled_at?: string | null
  cancellation_reason?: string | null
  status: 'COMPLETED' | 'CANCELLED'
  created_at?: string
  updated_at?: string
}

export interface ExpenseCategoryBreakdown {
  category_id?: string | null
  category_name: string
  total_amount: number | string
  transaction_count: number
  percentage: number | string
}

export interface ExpenseSummaryResponse {
  total_amount: number | string
  total_count: number
  today_amount: number | string
  this_month_amount: number | string
  categories_breakdown: ExpenseCategoryBreakdown[]
}

export interface CreateExpenseRequest {
  outlet_id: string
  category_id?: string | null
  expense_account_id: string
  payment_account_id: string
  amount: number
  transaction_date?: string
  recipient_name?: string
  notes?: string
  reference_no?: string
  attachment_url?: string
  shift_id?: string | null
}

export interface CapitalDepositDetail {
  id: string
  deposit_no: string
  outlet_id?: string | null
  outlet_name?: string | null
  deposit_date: string
  investor_name: string
  amount: number | string
  payment_account_id: string
  payment_account_code: string
  payment_account_name: string
  equity_account_id: string
  equity_account_code: string
  equity_account_name: string
  reference_no?: string | null
  notes?: string | null
  attachment_url?: string | null
  journal_entry_id?: string | null
  created_by?: string | null
  created_by_name?: string | null
  cancelled_by?: string | null
  cancelled_by_name?: string | null
  cancelled_at?: string | null
  cancellation_reason?: string | null
  status: 'COMPLETED' | 'CANCELLED'
  created_at?: string
  updated_at?: string
}

export interface CreateCapitalDepositRequest {
  outlet_id?: string | null
  deposit_date?: string
  investor_name: string
  amount: number
  payment_account_id: string
  equity_account_id: string
  reference_no?: string
  notes?: string
  attachment_url?: string
}

export interface CapitalSummaryResponse {
  total_amount: number | string
  total_count: number
  this_month_amount: number | string
  this_year_amount: number | string
}

export interface BankAccount {
  id: string
  bank_name: string
  account_number: string
  account_holder: string
  branch?: string | null
  outlet_id?: string | null
  outlet_name?: string | null
  coa_id?: string | null
  coa_code?: string | null
  coa_name?: string | null
  is_active: boolean
  is_default: boolean
  notes?: string | null
  created_at: string
  updated_at: string
}

export interface CreateBankAccountRequest {
  bank_name: string
  account_number: string
  account_holder: string
  branch?: string
  outlet_id?: string | null
  is_default?: boolean
  notes?: string
}

export interface UpdateBankAccountRequest {
  bank_name: string
  account_number: string
  account_holder: string
  branch?: string
  outlet_id?: string | null
  is_active: boolean
  is_default?: boolean
  notes?: string
}

// ------------------- RETAIL ENTERPRISE P1 & P2 -------------------

export interface PromotionRule {
  id?: string
  promotion_id?: string
  rule_type: string
  buy_product_id?: string | null
  buy_product_name?: string | null
  buy_quantity: number
  reward_product_id?: string | null
  reward_product_name?: string | null
  reward_quantity: number
  reward_discount_percent: number
  reward_price?: number | null
  min_transaction_amount?: number | null
  max_usage_per_transaction?: number | null
}

export interface Promotion {
  id: string
  code?: string | null
  name: string
  promo_type: 'BUY_X_GET_Y' | 'MIN_PURCHASE_DISCOUNT' | 'BUNDLE_PRICE' | 'PURCHASE_WITH_PURCHASE'
  start_date: string
  end_date: string
  is_active: boolean
  banner_url?: string | null
  description?: string | null
  outlet_ids?: string[] | null
  rules: PromotionRule[]
  created_at?: string
}

export interface ActivePromotionRule {
  id?: string
  rule_type: string
  buy_product_id?: string | null
  buy_product_name?: string | null
  buy_product_barcode?: string | null
  buy_quantity: number
  reward_product_id?: string | null
  reward_product_name?: string | null
  reward_product_barcode?: string | null
  reward_quantity: number
  reward_discount_percent: number
  reward_price?: number | null
  reward_special_price?: number | null
  bundle_discount_amount?: number | null
  min_transaction_amount?: number | null
  max_usage_per_transaction?: number | null
}

export interface ActivePromotion {
  id: string
  name: string
  promo_type: string
  start_date: string
  end_date: string
  description?: string | null
  rules: ActivePromotionRule[]
}

export interface LookupSaleItem {
  id: string
  sale_id: string
  product_id: string
  product_name: string
  sku: string
  barcode?: string | null
  unit_code: string
  quantity: number
  returned_quantity: number
  remaining_returnable_qty: number
  unit_price: number
  discount_amount: number
  subtotal: number
}

export interface LookupSaleDetail {
  id: string
  invoice_number: string
  shift_id: string
  outlet_id?: string | null
  outlet_name?: string | null
  customer_name?: string | null
  total_amount: number
  paid_amount: number
  payment_method: string
  status: string
  created_at: string
  items: LookupSaleItem[]
}

export interface CreateSalesReturnItem {
  sale_item_id: string
  product_id: string
  quantity: number
  condition: 'GOOD' | 'DAMAGED'
  restock: boolean
}

export interface CreateSalesReturnRequest {
  sale_id: string
  refund_method: 'CASH' | 'STORE_CREDIT'
  reason_code: string
  notes?: string
  supervisor_id?: string | null
  supervisor_pin: string
  items: CreateSalesReturnItem[]
}

export interface CashDropItem {
  id: string
  drop_number: string
  shift_id: string
  outlet_id?: string | null
  cashier_id: string
  cashier_name?: string | null
  supervisor_id: string
  supervisor_name?: string | null
  amount: number
  notes?: string | null
  created_at: string
}

export interface CashDropStatusSummary {
  current_drawer_cash: number
  max_drawer_limit: number
  should_drop: boolean
  total_dropped: number
  drops: CashDropItem[]
}

export interface FefoAlertItem {
  grn_item_id: string
  grn_number: string
  product_id: string
  product_name: string
  product_sku: string
  product_barcode?: string | null
  unit_code?: string | null
  batch_number?: string | null
  expiry_date: string
  received_qty: number
  days_remaining: number
  urgency: 'EXPIRED' | 'CRITICAL' | 'WARNING' | 'NOTICE'
  outlet_id?: string | null
  outlet_name?: string | null
  supplier_name?: string | null
}

export interface ReorderSuggestionItem {
  product_id: string
  product_sku: string
  product_barcode?: string | null
  product_name: string
  supplier_id?: string | null
  supplier_name?: string | null
  unit_code?: string | null
  current_stock: number
  min_stock: number
  purchase_price: number
  suggested_qty: number
  estimated_subtotal: number
}

// ------------------- DIVIDEND DISTRIBUTION -------------------

export interface DividendRecipientItem {
  name: string
  share_percentage?: number | null
  amount: number
  payment_method?: string | null
  notes?: string | null
}

export interface DividendDistribution {
  id: string
  distribution_number: string
  period_id: string
  period_name?: string | null
  period_start?: string | null
  period_end?: string | null
  distribution_date: string
  total_amount: number
  retained_earnings_account_id: string
  retained_earnings_account_code?: string | null
  retained_earnings_account_name?: string | null
  payment_account_id: string
  payment_account_code?: string | null
  payment_account_name?: string | null
  journal_entry_id?: string | null
  journal_number?: string | null
  recipient_name: string
  notes?: string | null
  recipients_data?: DividendRecipientItem[] | null
  created_at: string
  created_by?: string | null
  created_by_name?: string | null
}

export interface CreateDividendRequest {
  period_id: string
  distribution_date: string
  total_amount: number
  retained_earnings_account_id?: string | null
  payment_account_id: string
  recipient_name: string
  notes?: string | null
  recipients?: DividendRecipientItem[]
}

export interface ClosedPeriodForDividend {
  id: string
  closing_number: string
  name: string
  period_start: string
  period_end: string
  closing_date?: string
  start_date?: string
  end_date?: string
  is_closed: boolean
  closed_at?: string | null
  closed_by_name?: string | null
  net_income: number
  total_dividends_drawn: number
  remaining_income: number
}

export interface DividendSummary {
  total_retained_earnings: number
  total_dividends_paid: number
  closed_periods_count: number
  eligible_closed_periods: ClosedPeriodForDividend[]
}



