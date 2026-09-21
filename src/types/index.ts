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
}

export interface CashierShift {
  id: string
  terminal_id: string
  user_id: string
  outlet_id?: string
  outlet_name?: string
  outlet_code?: string
  default_warehouse_id?: string
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
  reference_number?: string
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
    created_at: string
  }
  items?: CheckoutResultItem[]
  payments?: CheckoutResultPayment[]
  customer_name?: string
  items_count: number
  change_amount: number
  earned_points: number
  invoice_number: string
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
