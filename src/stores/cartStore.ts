import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { api } from '@/services/api'
import type { CartItem, Customer, HeldOrder, ProductDetail, ProductUnitDetail } from '@/types'

export const useCartStore = defineStore('cart', () => {
  const items = ref<CartItem[]>([])
  const selectedCustomer = ref<Customer | null>(null)
  const voucherCode = ref<string>('')
  const voucherDiscount = ref<number>(0)
  const heldOrders = ref<HeldOrder[]>(
    localStorage.getItem('notane_held_orders')
      ? JSON.parse(localStorage.getItem('notane_held_orders')!)
      : []
  )
  const defaultWarehouseId = ref<string>('b0000000-0000-0000-0000-000000000002')

  const subtotal = computed(() => {
    return items.value.reduce((sum, item) => sum + item.unit_price * item.quantity, 0)
  })

  const itemDiscounts = computed(() => {
    return items.value.reduce((sum, item) => sum + item.discount_amount, 0)
  })

  const totalDiscount = computed(() => {
    return itemDiscounts.value + voucherDiscount.value
  })

  const totalTax = computed(() => {
    return items.value.reduce((sum, item) => sum + item.tax_amount, 0)
  })

  const grandTotal = computed(() => {
    const net = subtotal.value - totalDiscount.value
    return Math.max(0, net + totalTax.value)
  })

  const totalItemsCount = computed(() => {
    return items.value.reduce((sum, item) => sum + Number(item.quantity), 0)
  })

  async function scanBarcode(barcode: string) {
    const trimmed = barcode.trim()
    if (!trimmed) return

    const res = await api.get(`/products/barcode/${trimmed}`, {
      params: { warehouse_id: defaultWarehouseId.value }
    })

    if (res.data.success && res.data.data) {
      const detail: ProductDetail = res.data.data
      
      // Determine quantity: if barcode was weight-embedded, parse weight from barcode
      let qty = 1
      if (detail.scanned_quantity && Number(detail.scanned_quantity) > 0) {
        qty = Number(detail.scanned_quantity)
      } else if (detail.product.is_weight_embedded && trimmed.startsWith('2') && trimmed.length >= 12) {
        const weightPart = trimmed.length === 13 && (trimmed.startsWith('20') || trimmed.startsWith('02'))
          ? trimmed.substring(7, 12)
          : trimmed.substring(6, 11)
        const grams = parseInt(weightPart, 10)
        if (!isNaN(grams) && grams > 0) {
          qty = grams / 1000.0 // e.g. 1.250 kg
        } else {
          qty = 1 // Default jika scan barcode master tanpa bobot embedded (e.g. 200001000000)
        }
      }

      addProductToCart(detail, qty)
      return detail
    } else {
      throw new Error(res.data.message || 'Barang tidak ditemukan')
    }
  }

  function addProductToCart(detail: ProductDetail, quantity = 1) {
    const baseUnit: ProductUnitDetail = detail.units[0] || {
      id: '',
      unit_id: detail.product.base_unit_id,
      unit_code: detail.base_unit_code,
      unit_name: detail.base_unit_name,
      conversion_factor: 1,
      selling_price: detail.product.base_selling_price,
    }

    const qtyToAdd = Number(quantity) > 0 ? Number(quantity) : 1

    // Check if item already exists in cart with same product & unit
    const existingIndex = items.value.findIndex(
      (it) => it.product.id === detail.product.id && (
        !it.selected_unit?.unit_id || !baseUnit.unit_id || it.selected_unit.unit_id === baseUnit.unit_id
      )
    )

    if (existingIndex >= 0) {
      const currentQty = Number(items.value[existingIndex].quantity) || 0
      const newQty = Number((currentQty + qtyToAdd).toFixed(3))
      updateQuantity(existingIndex, newQty)
      recalculateTiers(items.value[existingIndex], detail.price_tiers)
    } else {
      const price = Number(baseUnit.selling_price)
      const gross = price * qtyToAdd
      const taxPercent = Number(detail.product.tax_percent) || 0
      const tax = (gross * taxPercent) / 100

      const newItem: CartItem = {
        id: `${detail.product.id}-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`,
        product: detail.product,
        selected_unit: baseUnit,
        quantity: qtyToAdd,
        unit_price: price,
        discount_percent: 0,
        discount_amount: 0,
        tax_amount: tax,
        subtotal: gross + tax,
      }

      items.value.unshift(newItem)
      recalculateTiers(items.value[0], detail.price_tiers)
    }
  }

  async function addProduct(prod: any, quantity = 1) {
    const qtyToAdd = Number(quantity) > 0 ? Number(quantity) : 1
    const code = prod.barcode || prod.sku
    if (code) {
      try {
        await scanBarcode(code)
        return
      } catch (err) {
        console.warn('Scan barcode failed for product add, using direct fallback:', err)
      }
    }

    // Direct add fallback
    const existingIndex = items.value.findIndex((it) => it.product.id === prod.id)
    if (existingIndex >= 0) {
      const currentQty = Number(items.value[existingIndex].quantity) || 0
      updateQuantity(existingIndex, Number((currentQty + qtyToAdd).toFixed(3)))
      return
    }

    const price = Number(prod.base_selling_price || prod.selling_price || 0)
    const gross = price * qtyToAdd
    const taxPercent = Number(prod.tax_percent) || 0
    const tax = (gross * taxPercent) / 100

    const unit: ProductUnitDetail = {
      id: '',
      unit_id: prod.base_unit_id || prod.unit_id || '',
      unit_code: prod.base_unit_code || prod.unit_code || 'PCS',
      unit_name: prod.base_unit_name || prod.unit_name || 'Pieces',
      conversion_factor: 1,
      selling_price: price,
    }

    const newItem: CartItem = {
      id: `${prod.id}-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`,
      product: prod,
      selected_unit: unit,
      quantity: qtyToAdd,
      unit_price: price,
      discount_percent: 0,
      discount_amount: 0,
      tax_amount: tax,
      subtotal: gross + tax,
    }

    items.value.unshift(newItem)
  }

  function recalculateTiers(item: CartItem, priceTiers?: { min_quantity: number; tier_price: number }[]) {
    if (!priceTiers || priceTiers.length === 0) return

    // Find highest tier matching quantity
    let matchedTierPrice = item.selected_unit.selling_price
    for (const tier of priceTiers) {
      if (item.quantity >= tier.min_quantity) {
        matchedTierPrice = tier.tier_price
      }
    }

    item.unit_price = matchedTierPrice
    recalculateItem(item)
  }

  function recalculateItem(item: CartItem) {
    const gross = item.unit_price * item.quantity
    let disc = item.discount_amount
    if (item.discount_percent > 0) {
      disc = (gross * item.discount_percent) / 100
      item.discount_amount = disc
    }
    const taxable = Math.max(0, gross - disc)
    item.tax_amount = (taxable * item.product.tax_percent) / 100
    item.subtotal = taxable + item.tax_amount
  }

  function updateQuantity(index: number, newQty: number) {
    if (index < 0 || index >= items.value.length) return

    if (newQty <= 0) {
      removeItem(index)
      return
    }

    const item = items.value[index]
    item.quantity = Number(Number(newQty).toFixed(3))
    recalculateItem(item)
  }

  function updateItemDiscount(index: number, discAmount: number, discPercent = 0) {
    if (index < 0 || index >= items.value.length) return
    const item = items.value[index]
    item.discount_percent = discPercent
    item.discount_amount = discAmount
    recalculateItem(item)
  }

  function changeUnit(index: number, newUnit: ProductUnitDetail) {
    if (index < 0 || index >= items.value.length) return
    const item = items.value[index]
    item.selected_unit = newUnit
    item.unit_price = newUnit.selling_price
    recalculateItem(item)
  }

  function removeItem(index: number) {
    if (index >= 0 && index < items.value.length) {
      items.value.splice(index, 1)
    }
  }

  function clearCart() {
    items.value = []
    selectedCustomer.value = null
    voucherCode.value = ''
    voucherDiscount.value = 0
  }

  async function applyVoucher(code: string) {
    const trimmed = code.trim().toUpperCase()
    if (!trimmed) return

    const res = await api.get(`/vouchers/validate/${trimmed}`)
    if (res.data.success && res.data.data) {
      const v = res.data.data
      if (subtotal.value < v.min_purchase_amount) {
        throw new Error(`Minimal belanja untuk voucher ini adalah Rp ${v.min_purchase_amount.toLocaleString()}`)
      }

      voucherCode.value = v.code
      if (v.discount_type === 'PERCENTAGE') {
        const calc = (subtotal.value * v.discount_value) / 100
        voucherDiscount.value = v.max_discount_amount ? Math.min(calc, v.max_discount_amount) : calc
      } else {
        voucherDiscount.value = v.discount_value
      }
      return v
    }
  }

  function removeVoucher() {
    voucherCode.value = ''
    voucherDiscount.value = 0
  }

  function holdCurrentOrder(notes = ''): HeldOrder | null {
    if (items.value.length === 0) return null

    const held: HeldOrder = {
      id: `HOLD-${Date.now()}`,
      timestamp: new Date().toLocaleTimeString(),
      customer: selectedCustomer.value || undefined,
      customer_name: selectedCustomer.value?.name || notes || `Antrean #${heldOrders.value.length + 1}`,
      items: [...items.value],
      total_amount: grandTotal.value,
    }

    heldOrders.value.push(held)
    localStorage.setItem('notane_held_orders', JSON.stringify(heldOrders.value))
    clearCart()
    return held
  }

  function recallOrder(orderId: string) {
    const idx = heldOrders.value.findIndex((o) => o.id === orderId)
    if (idx >= 0) {
      const order = heldOrders.value[idx]
      items.value = order.items
      selectedCustomer.value = order.customer || null
      heldOrders.value.splice(idx, 1)
      localStorage.setItem('notane_held_orders', JSON.stringify(heldOrders.value))
      return order
    }
    return null
  }

  return {
    items,
    selectedCustomer,
    voucherCode,
    voucherDiscount,
    heldOrders,
    defaultWarehouseId,
    subtotal,
    itemDiscounts,
    totalDiscount,
    totalTax,
    grandTotal,
    totalItemsCount,
    scanBarcode,
    addProduct,
    addProductToCart,
    updateQuantity,
    updateItemDiscount,
    changeUnit,
    removeItem,
    clearCart,
    applyVoucher,
    removeVoucher,
    holdCurrentOrder,
    recallOrder,
  }
})
