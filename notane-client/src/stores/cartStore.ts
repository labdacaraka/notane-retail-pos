import { defineStore } from 'pinia'
import { ref, computed, watch } from 'vue'
import { api } from '@/services/api'
import { broadcastCustomerDisplayState, listenForSyncRequests, type CustomerDisplayState, type CustomerDisplayItem } from '@/services/customerDisplayChannel'
import { soundService } from '@/services/soundService'
import type { CartItem, Customer, HeldOrder, ProductDetail, ProductUnitDetail, ActivePromoProduct, ActivePromotion, EligibleGift, OutOfStockGift, DismissedGiftInfo } from '@/types'
import { useShiftStore } from './shiftStore'

export const useCartStore = defineStore('cart', () => {
  const items = ref<CartItem[]>([])
  const selectedCustomer = ref<Customer | null>(null)
  const voucherCode = ref<string>('')
  const voucherDiscount = ref<number>(0)
  const activeFlashSales = ref<ActivePromoProduct[]>([])
  const activePromotions = ref<ActivePromotion[]>([])
  const eligibleGifts = ref<EligibleGift[]>([])
  const outOfStockGifts = ref<OutOfStockGift[]>([])
  const dismissedGifts = ref<DismissedGiftInfo[]>([])
  const isLoadingPromos = ref<boolean>(false)
  const dismissedGiftRuleIds = ref<Set<string>>(new Set())
  const productDetailCache = new Map<string, ProductDetail>()
  const heldOrders = ref<HeldOrder[]>(
    localStorage.getItem('notane_held_orders')
      ? JSON.parse(localStorage.getItem('notane_held_orders')!)
      : []
  )
  const currentOutletId = ref<string>('')

  async function fetchProductForGift(productId: string, barcode?: string): Promise<ProductDetail | null> {
    if (productDetailCache.has(productId)) {
      return productDetailCache.get(productId)!
    }

    const params: Record<string, any> = {}
    if (currentOutletId.value) {
      params.outlet_id = currentOutletId.value
      params.warehouse_id = currentOutletId.value
    }

    // 1. Coba pencarian barcode langsung
    if (barcode && barcode.trim()) {
      try {
        const res = await api.get(`/products/barcode/${barcode.trim()}`, { params })
        if (res.data?.success && res.data.data) {
          productDetailCache.set(productId, res.data.data)
          return res.data.data
        }
      } catch {
        // Lanjutkan fallback
      }
    }

    // 2. Coba pencarian ID via lookup barcode
    try {
      const res = await api.get(`/products/barcode/${productId}`, { params })
      if (res.data?.success && res.data.data) {
        productDetailCache.set(productId, res.data.data)
        return res.data.data
      }
    } catch {
      // Lanjutkan fallback
    }

    // 3. Fallback pencarian katalog produk
    try {
      const res = await api.get('/products', { params: { limit: 50, ...params } })
      if (res.data?.success && Array.isArray(res.data.data)) {
        const found = res.data.data.find((p: any) => p.id === productId)
        if (found) {
          const code = found.barcode || found.sku
          if (code) {
            const bRes = await api.get(`/products/barcode/${code}`, { params })
            if (bRes.data?.success && bRes.data.data) {
              productDetailCache.set(productId, bRes.data.data)
              return bRes.data.data
            }
          }
        }
      }
    } catch {
      // Abaikan
    }

    return null
  }

  function isPosRoute(): boolean {
    if (typeof window === 'undefined') return true
    const p = window.location.pathname
    return p === '/' || p.startsWith('/pos') || p === ''
  }

  async function fetchActivePromos() {
    // Hanya panggil promosi aktif jika berada di rute antarmuka kasir POS
    if (!isPosRoute()) {
      return
    }

    try {
      isLoadingPromos.value = true
      const params: Record<string, any> = {}
      if (currentOutletId.value) {
        params.outlet_id = currentOutletId.value
      }
      const [fsRes, promoRes] = await Promise.allSettled([
        api.get('/flash-sales/active-promos', { params }),
        api.get('/promotions/active', { params }),
      ])

      if (fsRes.status === 'fulfilled' && fsRes.value.data?.success && Array.isArray(fsRes.value.data.data)) {
        activeFlashSales.value = fsRes.value.data.data
      }
      if (promoRes.status === 'fulfilled' && promoRes.value.data?.success && Array.isArray(promoRes.value.data.data)) {
        activePromotions.value = promoRes.value.data.data

        // Pre-cache item hadiah di background agar saat kasir scan produk syarat, hadiah instan muncul
        for (const promo of activePromotions.value) {
          for (const rule of promo.rules || []) {
            if (rule.reward_product_id) {
              fetchProductForGift(rule.reward_product_id, rule.reward_product_barcode || undefined).catch(() => {})
            }
          }
        }
      }

      // Recalculate item-item yang ada di keranjang
      for (const it of items.value) {
        recalculateTiers(it)
      }
      evaluatePromotions()
    } catch (err) {
      console.warn('Gagal memuat promosi aktif:', err)
    } finally {
      isLoadingPromos.value = false
    }
  }

  async function setOutlet(outletId: string) {
    currentOutletId.value = outletId
    if (outletId) {
      localStorage.setItem('notane_pos_outlet_id', outletId)
      localStorage.setItem('notane_active_outlet_id', outletId)
      if (isPosRoute()) {
        await fetchActivePromos()
      }
    } else {
      localStorage.removeItem('notane_pos_outlet_id')
      localStorage.removeItem('notane_active_outlet_id')
      activePromotions.value = []
      activeFlashSales.value = []
    }
  }

  async function initOutlet() {
    const shiftStore = useShiftStore()
    if (shiftStore.isShiftOpen && shiftStore.currentShift?.outlet_id) {
      await setOutlet(shiftStore.currentShift.outlet_id)
      return
    }
    // Jika belum ada shift yang terbuka, reset outlet dan jangan load promo/produk
    currentOutletId.value = ''
    activePromotions.value = []
    activeFlashSales.value = []
  }

  // Subtotal kotor sebelum diskon apapun (harga katalog normal * quantity)
  const grossSubtotal = computed(() => {
    return items.value.reduce((sum, item) => {
      const normalPrice = Number(item.original_price || item.unit_price || 0)
      return sum + normalPrice * item.quantity
    }, 0)
  })

  const subtotal = computed(() => {
    return items.value.reduce((sum, item) => sum + item.unit_price * item.quantity, 0)
  })

  const itemDiscounts = computed(() => {
    return items.value.reduce((sum, item) => sum + item.discount_amount, 0)
  })

  const totalFlashSaleSavings = computed(() => {
    return items.value.reduce((sum, item) => sum + (item.flash_sale_discount || 0), 0)
  })

  const totalMemberSavings = computed(() => {
    return items.value.reduce((sum, item) => sum + (item.member_discount || 0), 0)
  })

  // Evaluasi diskon promosi otomatis (Buy X Get Y, Min Belanja, PWP)
  const minPurchaseDiscount = computed(() => {
    let savings = 0
    if (!activePromotions.value || activePromotions.value.length === 0) return 0
    for (const promo of activePromotions.value) {
      for (const rule of promo.rules || []) {
        const ruleType = rule.rule_type || promo.promo_type
        if (ruleType === 'MIN_PURCHASE_DISCOUNT') {
          const minSpend = Number(rule.min_transaction_amount) || 0
          if (subtotal.value >= minSpend) {
            if (Number(rule.reward_discount_percent) > 0) {
              savings = Math.max(savings, (subtotal.value * Number(rule.reward_discount_percent)) / 100)
            } else if (Number(rule.reward_price) > 0) {
              savings = Math.max(savings, Number(rule.reward_price))
            }
          }
        }
      }
    }
    return Math.round(savings)
  })

  const totalItemPromoSavings = computed(() => {
    return items.value.reduce((sum, item) => sum + (item.promo_discount || 0), 0)
  })

  const totalPromoSavings = computed(() => {
    return totalItemPromoSavings.value + minPurchaseDiscount.value
  })

  const totalDiscount = computed(() => {
    return itemDiscounts.value + voucherDiscount.value + totalPromoSavings.value
  })

  // Akumulasi total yang dihemat oleh pelanggan (Flash sale + diskon member + voucher + diskon promo otomatis + diskon manual)
  const totalSavings = computed(() => {
    return totalFlashSaleSavings.value + totalMemberSavings.value + totalDiscount.value
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
    const shiftStore = useShiftStore()
    if (!shiftStore.isShiftOpen || !shiftStore.currentShift) {
      throw new Error('Shift kasir belum dibuka. Silakan buka shift kasir terlebih dahulu.')
    }
    const outletId = shiftStore.currentShift.outlet_id || currentOutletId.value
    if (!outletId) {
      throw new Error('Shift kasir belum memiliki cabang toko yang dipilih.')
    }

    const trimmed = barcode.trim()
    if (!trimmed) return

    const params: Record<string, any> = {
      outlet_id: outletId,
      warehouse_id: outletId,
    }

    const res = await api.get(`/products/barcode/${trimmed}`, { params })

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

  function getBasePriceForCustomer(detail: ProductDetail, unit: ProductUnitDetail): number {
    if (selectedCustomer.value && selectedCustomer.value.customer_group_id) {
      const groupId = selectedCustomer.value.customer_group_id

      // Prioritas 1: Harga member spesifik outlet (outlet_customer_group_prices)
      const outletGroupPrices = detail.outlet_customer_group_prices || []
      const foundOutletGroupPrice = outletGroupPrices.find(
        (ogp) => ogp.customer_group_id === groupId
      )
      if (foundOutletGroupPrice && Number(foundOutletGroupPrice.special_price) > 0) {
        const factor = Number(unit.conversion_factor) || 1
        return Number(foundOutletGroupPrice.special_price) * factor
      }

      // Prioritas 2: Harga member global (customer_group_prices)
      const groupPrices = detail.customer_group_prices || []
      const foundGroupPrice = groupPrices.find(
        (gp) => gp.customer_group_id === groupId
      )
      if (foundGroupPrice && Number(foundGroupPrice.special_price) > 0) {
        const factor = Number(unit.conversion_factor) || 1
        return Number(foundGroupPrice.special_price) * factor
      }
    }
    // Prioritas 3: Harga jual unit (sudah mencakup harga outlet jika ada)
    return Number(unit.selling_price || detail.product.base_selling_price || 0)
  }

  function getMaxAllowedQuantity(item: CartItem): number {
    const rawStock = Number(
      item.product_detail?.current_stock ??
      item.current_stock ??
      (item.product as any).stock_qty ??
      0
    )
    let otherAllocatedStock = 0
    if (!item.is_auto_gift) {
      // Hitung stok yang sudah digunakan oleh hadiah promosi otomatis untuk produk yang sama
      const giftItems = items.value.filter(
        (it) => it.is_auto_gift && it.product.id === item.product.id
      )
      for (const g of giftItems) {
        const gFactor = Number(g.selected_unit?.conversion_factor) || 1
        otherAllocatedStock += Number(g.quantity || 0) * gFactor
      }
    }
    const availableStock = Math.max(0, rawStock - otherAllocatedStock)
    const factor = Number(item.selected_unit?.conversion_factor) || 1
    if (factor <= 0) return availableStock
    const maxQty = availableStock / factor
    return item.product.is_weight_embedded
      ? Number(Math.max(0, maxQty).toFixed(3))
      : Math.max(0, Math.floor(maxQty))
  }

  function addProductToCart(detail: ProductDetail, quantity = 1) {
    const shiftStore = useShiftStore()
    if (!shiftStore.isShiftOpen || !shiftStore.currentShift) {
      throw new Error('Shift kasir belum dibuka. Silakan buka shift kasir terlebih dahulu.')
    }
    const baseUnit: ProductUnitDetail = detail.units[0] || {
      id: '',
      unit_id: detail.product.base_unit_id,
      unit_code: detail.base_unit_code,
      unit_name: detail.base_unit_name,
      conversion_factor: 1,
      selling_price: detail.product.base_selling_price,
    }

    const rawStock = Number(detail.current_stock ?? 0)
    // Hitung stok yang sudah dialokasikan untuk hadiah promosi gratis dari produk yang sama
    const allocatedGiftQty = items.value
      .filter((it) => it.is_auto_gift && it.product.id === detail.product.id)
      .reduce((sum, g) => {
        const gFactor = Number(g.selected_unit?.conversion_factor) || 1
        return sum + Number(g.quantity || 0) * gFactor
      }, 0)
    const effectiveStock = Math.max(0, rawStock - allocatedGiftQty)

    const factor = Number(baseUnit.conversion_factor) || 1
    const maxAllowedForUnit = factor > 0
      ? (detail.product.is_weight_embedded ? Number((effectiveStock / factor).toFixed(3)) : Math.floor(effectiveStock / factor))
      : effectiveStock

    if (rawStock <= 0) {
      soundService.playOutOfStock()
      throw new Error(`Stok produk '${detail.product.name}' habis (0 ${detail.base_unit_code})`)
    }

    if (effectiveStock <= 0) {
      soundService.playOutOfStock()
      throw new Error(
        `Sisa stok produk '${detail.product.name}' tidak mencukupi untuk pembelian reguler (total stok ${rawStock} ${detail.base_unit_code}, ${allocatedGiftQty} ${detail.base_unit_code} sudah dialokasikan sebagai hadiah promosi gratis)`
      )
    }

    const qtyToAdd = Number(quantity) > 0 ? Number(quantity) : 1

    // Check if item already exists in cart with same product & unit (HANYA ITEM REGULER NON-HADIAH)
    const existingIndex = items.value.findIndex(
      (it) => !it.is_auto_gift && it.product.id === detail.product.id && (
        !it.selected_unit?.unit_id || !baseUnit.unit_id || it.selected_unit.unit_id === baseUnit.unit_id
      )
    )

    if (existingIndex >= 0) {
      const currentQty = Number(items.value[existingIndex].quantity) || 0
      const newQty = Number((currentQty + qtyToAdd).toFixed(3))
      items.value[existingIndex].product_detail = detail
      items.value[existingIndex].current_stock = rawStock

      if (newQty > maxAllowedForUnit) {
        items.value[existingIndex].quantity = maxAllowedForUnit
        recalculateTiers(items.value[existingIndex])
        soundService.playOutOfStock()
        throw new Error(
          `Stok '${detail.product.name}' tidak mencukupi untuk menambah ${qtyToAdd} ${baseUnit.unit_code}. Maksimal stok: ${maxAllowedForUnit} ${baseUnit.unit_code} (sudah di keranjang: ${currentQty} ${baseUnit.unit_code})`
        )
      }

      updateQuantity(existingIndex, newQty)
      recalculateTiers(items.value[existingIndex])
      soundService.playScanSuccess()
    } else {
      if (qtyToAdd > maxAllowedForUnit) {
        soundService.playOutOfStock()
        throw new Error(
          `Stok '${detail.product.name}' tidak mencukupi. Stok tersedia untuk dibeli: ${maxAllowedForUnit} ${baseUnit.unit_code}, diminta: ${qtyToAdd} ${baseUnit.unit_code}`
        )
      }

      const price = getBasePriceForCustomer(detail, baseUnit)
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
        product_detail: detail,
        current_stock: rawStock,
        is_auto_gift: false,
      }

      items.value.unshift(newItem)
      recalculateTiers(items.value[0])
      evaluatePromotions()
      soundService.playScanSuccess()
    }
  }

  async function addProduct(prod: any, quantity = 1) {
    const shiftStore = useShiftStore()
    if (!shiftStore.isShiftOpen || !shiftStore.currentShift) {
      throw new Error('Shift kasir belum dibuka. Silakan buka shift kasir terlebih dahulu.')
    }
    const qtyToAdd = Number(quantity) > 0 ? Number(quantity) : 1
    const code = prod.barcode || prod.sku
    if (code) {
      try {
        await scanBarcode(code)
        return
      } catch (err: any) {
        if (err.message && (err.message.includes('Stok') || err.message.includes('stok'))) {
          throw err
        }
        console.warn('Scan barcode failed for product add, using direct fallback:', err)
      }
    }

    // Direct add fallback
    const rawStock = Number(prod.stock_qty ?? 0)
    const allocatedGiftQty = items.value
      .filter((it) => it.is_auto_gift && it.product.id === prod.id)
      .reduce((sum, g) => {
        const gFactor = Number(g.selected_unit?.conversion_factor) || 1
        return sum + Number(g.quantity || 0) * gFactor
      }, 0)
    const effectiveStock = Math.max(0, rawStock - allocatedGiftQty)

    if (rawStock <= 0) {
      soundService.playOutOfStock()
      throw new Error(`Stok produk '${prod.name}' habis (0 ${prod.unit_code || 'PCS'})`)
    }

    if (effectiveStock <= 0) {
      soundService.playOutOfStock()
      throw new Error(
        `Sisa stok produk '${prod.name}' tidak mencukupi untuk pembelian reguler (total stok ${rawStock} ${prod.unit_code || 'PCS'}, ${allocatedGiftQty} sudah dialokasikan sebagai hadiah promosi gratis)`
      )
    }

    const existingIndex = items.value.findIndex((it) => !it.is_auto_gift && it.product.id === prod.id)
    if (existingIndex >= 0) {
      const currentQty = Number(items.value[existingIndex].quantity) || 0
      const newQty = Number((currentQty + qtyToAdd).toFixed(3))
      const res = updateQuantity(existingIndex, newQty)
      if (!res.success && res.message) {
        throw new Error(res.message)
      }
      soundService.playScanSuccess()
      return
    }

    if (qtyToAdd > effectiveStock) {
      soundService.playOutOfStock()
      throw new Error(`Stok produk '${prod.name}' tidak mencukupi (Tersedia untuk dibeli: ${effectiveStock}, diminta: ${qtyToAdd})`)
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
      current_stock: rawStock,
      is_auto_gift: false,
    }

    items.value.unshift(newItem)
    recalculateTiers(items.value[0])
    evaluatePromotions()
    soundService.playScanSuccess()
  }

  function recalculateTiers(item: CartItem) {
    if (item.is_auto_gift) return
    const normalBasePrice = Number(
      item.selected_unit.selling_price ||
      item.product_detail?.product.base_selling_price ||
      item.product.base_selling_price ||
      0
    )
    item.original_price = normalBasePrice

    // 1. Evaluasi harga Member / Customer Group
    const memberPrice = item.product_detail
      ? getBasePriceForCustomer(item.product_detail, item.selected_unit)
      : normalBasePrice
    const hasMemberDiscount = Boolean(selectedCustomer.value && selectedCustomer.value.customer_group_id && memberPrice < normalBasePrice)

    // 2. Evaluasi promo Flash Sale aktif
    const promo = activeFlashSales.value.find((p) => p.product_id === item.product.id)

    let finalUnitPrice = normalBasePrice
    item.flash_sale_discount = 0
    item.member_discount = 0
    item.is_flash_sale = false
    item.flash_sale_id = undefined
    item.flash_sale_name = undefined

    if (promo) {
      const factor = Number(item.selected_unit.conversion_factor) || 1
      const promoUnitPrice = Number(promo.promo_price) * factor
      const fsSavingsPerUnit = Math.max(0, normalBasePrice - promoUnitPrice)

      // Cek batas kuantitas per transaksi dan sisa kuota promo
      const maxPerTrx = promo.max_qty_per_transaction ? Number(promo.max_qty_per_transaction) : Infinity
      const remQuota = promo.remaining_quota !== null && promo.remaining_quota !== undefined ? Number(promo.remaining_quota) : Infinity

      // Syarat Mutlak Promo Flash Sale:
      // Jika kuantitas transaksi melebihi batas maksimal per nota (max_qty_per_transaction)
      // atau melebihi sisa kuota promo yang tersedia (remaining_quota),
      // maka produk ini TIDAK BERHAK atas harga promo flash sale sama sekali (kembali ke harga normal / member).
      const isEligible = item.quantity <= maxPerTrx && item.quantity <= remQuota && remQuota > 0 && fsSavingsPerUnit > 0

      if (isEligible) {
        if (hasMemberDiscount) {
          if (promo.is_stackable_with_member) {
            // Akumulasi ganda (Stackable): Flash sale dipotong lagi dengan diskon member
            const memberSavingsPerUnit = Math.max(0, normalBasePrice - memberPrice)
            finalUnitPrice = Math.max(0, promoUnitPrice - memberSavingsPerUnit)
            item.flash_sale_discount = fsSavingsPerUnit * item.quantity
            item.member_discount = memberSavingsPerUnit * item.quantity
            item.is_flash_sale = true
            item.flash_sale_id = promo.flash_sale_id
            item.flash_sale_name = promo.flash_sale_name
          } else {
            // Standar Retail Modern (Best Value Rule): Pilih harga yang paling menguntungkan pelanggan
            if (promoUnitPrice <= memberPrice) {
              finalUnitPrice = promoUnitPrice
              item.flash_sale_discount = fsSavingsPerUnit * item.quantity
              item.member_discount = 0
              item.is_flash_sale = true
              item.flash_sale_id = promo.flash_sale_id
              item.flash_sale_name = promo.flash_sale_name
            } else {
              finalUnitPrice = memberPrice
              item.flash_sale_discount = 0
              item.member_discount = (normalBasePrice - memberPrice) * item.quantity
              item.is_flash_sale = false
            }
          }
        } else {
          // Pelanggan reguler (non-member)
          finalUnitPrice = promoUnitPrice
          item.flash_sale_discount = fsSavingsPerUnit * item.quantity
          item.member_discount = 0
          item.is_flash_sale = true
          item.flash_sale_id = promo.flash_sale_id
          item.flash_sale_name = promo.flash_sale_name
        }
      } else if (hasMemberDiscount) {
        finalUnitPrice = memberPrice
        item.member_discount = (normalBasePrice - memberPrice) * item.quantity
      }
    } else if (hasMemberDiscount) {
      finalUnitPrice = memberPrice
      item.member_discount = (normalBasePrice - memberPrice) * item.quantity
    }

    // 3. Jika bukan Flash Sale, periksa Tier Harga Grosir (Quantity Break)
    if (!item.is_flash_sale) {
      const priceTiers = item.product_detail?.price_tiers
      if (priceTiers && priceTiers.length > 0) {
        for (const tier of priceTiers) {
          if (item.quantity >= Number(tier.min_quantity)) {
            finalUnitPrice = Number(tier.tier_price)
          }
        }
      }
    }

    item.unit_price = finalUnitPrice
    recalculateItem(item)
  }

  function applyCustomerToCart(customer: Customer | null) {
    selectedCustomer.value = customer
    // Refresh unit_price semua item di keranjang dengan harga grup baru
    for (const item of items.value) {
      recalculateTiers(item)
    }
    evaluatePromotions()
  }

  function recalculateItem(item: CartItem) {
    const gross = item.unit_price * item.quantity
    let manualDisc = item.discount_amount || 0
    if (item.discount_percent > 0) {
      manualDisc = (gross * item.discount_percent) / 100
      item.discount_amount = manualDisc
    }
    const promoDisc = item.promo_discount || 0
    const totalItemDisc = manualDisc + promoDisc
    const taxable = Math.max(0, gross - totalItemDisc)
    item.tax_amount = (taxable * item.product.tax_percent) / 100
    item.subtotal = taxable + item.tax_amount
  }

  let isEvaluatingPromotions = false
  let pendingPromoEvalRequest = false

  async function evaluateCartPromotions() {
    if (isEvaluatingPromotions) {
      pendingPromoEvalRequest = true
      return
    }
    isEvaluatingPromotions = true

    try {
      const regularItems = items.value.filter((it) => !it.is_auto_gift)

      // Jika keranjang reguler kosong, bersihkan semua hadiah otomatis
      if (regularItems.length === 0) {
        items.value = items.value.filter((it) => !it.is_auto_gift)
        outOfStockGifts.value = []
        eligibleGifts.value = []
        return
      }

      const payload = {
        outlet_id: currentOutletId.value || undefined,
        customer_id: selectedCustomer.value?.id || undefined,
        items: regularItems.map((it) => ({
          product_id: it.product.id,
          unit_id: it.selected_unit?.unit_id || undefined,
          quantity: it.quantity,
          unit_selling_price: it.unit_price,
        })),
        dismissed_rule_ids: Array.from(dismissedGiftRuleIds.value),
      }

      const res = await api.post('/promotions/evaluate', payload)
      if (res.data?.success && res.data.data) {
        const { item_adjustments, auto_gifts, out_of_stock_gifts: oosGifts } = res.data.data

        // 1. Sinkronkan penyesuaian promo item reguler dari backend
        for (const it of items.value) {
          if (!it.is_auto_gift) {
            const adj = (item_adjustments || []).find((a: any) => a.product_id === it.product.id)
            if (adj) {
              it.promo_discount = Number(adj.discount_amount || 0)
              it.promo_name = adj.promo_name
              it.promo_id = adj.promo_id
              it.is_free_item = Boolean(adj.is_free_item)
            } else {
              it.promo_discount = 0
              it.promo_name = undefined
              it.promo_id = undefined
              it.is_free_item = false
            }
            recalculateItem(it)
          }
        }

        // 2. Sinkronkan daftar hadiah habis (Out of Stock) dari database backend
        outOfStockGifts.value = (oosGifts || []).map((g: any) => ({
          promo_id: g.promo_id,
          promo_name: g.promo_name,
          reward_product_id: g.reward_product_id,
          reward_product_name: g.reward_product_name,
          reward_product_barcode: g.reward_product_barcode,
          required_quantity: Number(g.required_quantity || 1),
          available_stock: Number(g.available_stock || 0),
        }))

        // 3. Sinkronkan hadiah otomatis (Auto-Gifts) resmi dari backend
        const backendGiftList = auto_gifts || []

        // A. Hapus hadiah lokal yang sudah tidak valid menurut backend
        items.value = items.value.filter((it) => {
          if (!it.is_auto_gift) return true
          const stillGranted = backendGiftList.some(
            (bg: any) => bg.product_id === it.product.id && bg.rule_id === it.auto_gift_promo_rule_id
          )
          return stillGranted
        })

        // B. Perbarui atau tambahkan hadiah baru yang di-approve oleh backend
        for (const bg of backendGiftList) {
          const existing = items.value.find(
            (it) => it.is_auto_gift && it.product.id === bg.product_id && it.auto_gift_promo_rule_id === bg.rule_id
          )

          const grantQty = Number(bg.quantity || 1)
          const unitPrice = Number(bg.unit_price || 0)

          if (existing) {
            existing.quantity = grantQty
            existing.promo_discount = unitPrice * grantQty
            existing.subtotal = 0
            existing.tax_amount = 0
            existing.unit_price = unitPrice
            existing.original_price = unitPrice
            existing.current_stock = Number(bg.available_stock || 0)
          } else {
            const giftItem: CartItem = {
              id: `gift-${bg.rule_id}-${Date.now()}`,
              product: {
                id: bg.product_id,
                name: bg.product_name,
                sku: bg.sku,
                barcode: bg.barcode,
                base_unit_id: bg.unit_id,
                base_selling_price: unitPrice,
                tax_percent: 0,
                cost_price: 0,
                is_active: true,
                is_weight_embedded: false,
                category_id: '',
              } as any,
              selected_unit: {
                id: '',
                unit_id: bg.unit_id,
                unit_code: bg.unit_code,
                unit_name: bg.unit_name,
                conversion_factor: 1,
                selling_price: unitPrice,
              },
              quantity: grantQty,
              unit_price: unitPrice,
              original_price: unitPrice,
              discount_percent: 0,
              discount_amount: 0,
              promo_discount: unitPrice * grantQty,
              tax_amount: 0,
              subtotal: 0,
              current_stock: Number(bg.available_stock || 0),
              is_free_item: true,
              is_auto_gift: true,
              promo_name: bg.promo_name,
              promo_id: bg.promo_id,
              auto_gift_promo_rule_id: bg.rule_id,
            }

            items.value.push(giftItem)
            soundService.playScanSuccess()
          }
        }
      }
    } catch (err) {
      console.warn('[CartStore] Gagal mengevaluasi promosi di backend API:', err)
    } finally {
      isEvaluatingPromotions = false
      if (pendingPromoEvalRequest) {
        pendingPromoEvalRequest = false
        evaluateCartPromotions()
      }
    }
  }

  function evaluatePromotions() {
    evaluateCartPromotions()
  }

  function dismissGift(ruleKey: string, promoId: string, promoName: string, productId: string, productName: string) {
    dismissedGiftRuleIds.value.add(ruleKey)
    if (!dismissedGifts.value.some((d) => d.rule_key === ruleKey)) {
      dismissedGifts.value.push({
        rule_key: ruleKey,
        promo_id: promoId,
        promo_name: promoName,
        product_id: productId,
        product_name: productName,
      })
    }
    const idx = items.value.findIndex(
      (i) => i.is_auto_gift && (i.auto_gift_promo_rule_id === ruleKey || i.product.id === productId)
    )
    if (idx >= 0) {
      items.value.splice(idx, 1)
    }
    evaluatePromotions()
    soundService.playRemoveItem()
  }

  function restoreGift(ruleKey: string) {
    dismissedGiftRuleIds.value.delete(ruleKey)
    dismissedGifts.value = dismissedGifts.value.filter((d) => d.rule_key !== ruleKey)
    evaluatePromotions()
    soundService.playScanSuccess()
  }

  async function claimFreeGift(gift: EligibleGift) {
    try {
      if (gift.reward_product_barcode) {
        try {
          await scanBarcode(gift.reward_product_barcode)
          return
        } catch {
          // fallback
        }
      }
      if (gift.reward_product_id) {
        try {
          await scanBarcode(gift.reward_product_id)
          return
        } catch {
          // fallback
        }
      }
      const res = await api.get('/products', {
        params: { search: gift.reward_product_name, limit: 1 }
      })
      if (res.data?.success && Array.isArray(res.data.data) && res.data.data.length > 0) {
        await addProduct(res.data.data[0], gift.quantity)
        return
      }
      throw new Error(`Produk '${gift.reward_product_name}' tidak ditemukan di katalog`)
    } catch (err: any) {
      soundService.playError()
      throw err
    }
  }

  function updateQuantity(index: number, newQty: number): { success: boolean; message?: string } {
    if (index < 0 || index >= items.value.length) return { success: false }

    if (newQty <= 0) {
      removeItem(index)
      return { success: true }
    }

    const item = items.value[index]
    const maxAllowed = getMaxAllowedQuantity(item)

    if (newQty > maxAllowed) {
      item.quantity = maxAllowed
      recalculateItem(item)
      recalculateTiers(item)
      evaluatePromotions()
      soundService.playOutOfStock()
      return {
        success: false,
        message: `Jumlah tidak dapat melebihi stok yang tersedia (${maxAllowed} ${item.selected_unit.unit_code})`,
      }
    }

    item.quantity = Number(Number(newQty).toFixed(3))
    recalculateItem(item)
    recalculateTiers(item)
    evaluatePromotions()
    return { success: true }
  }

  function updateItemDiscount(index: number, discAmount: number, discPercent = 0) {
    if (index < 0 || index >= items.value.length) return
    const item = items.value[index]
    item.discount_percent = discPercent
    item.discount_amount = discAmount
    recalculateItem(item)
    evaluatePromotions()
  }

  function changeUnit(index: number, newUnit: ProductUnitDetail) {
    if (index < 0 || index >= items.value.length) return
    const item = items.value[index]
    item.selected_unit = newUnit
    item.unit_price = newUnit.selling_price

    const maxAllowed = getMaxAllowedQuantity(item)
    if (item.quantity > maxAllowed) {
      item.quantity = maxAllowed
    }

    recalculateItem(item)
    recalculateTiers(item)
    evaluatePromotions()
  }

  function removeItem(index: number) {
    if (index >= 0 && index < items.value.length) {
      const target = items.value[index]
      if (target.is_auto_gift && target.auto_gift_promo_rule_id) {
        dismissedGiftRuleIds.value.add(target.auto_gift_promo_rule_id)
      }
      items.value.splice(index, 1)
      evaluatePromotions()
      soundService.playRemoveItem()
    }
  }

  function clearCart() {
    items.value = []
    selectedCustomer.value = null
    voucherCode.value = ''
    voucherDiscount.value = 0
    eligibleGifts.value = []
    outOfStockGifts.value = []
    dismissedGiftRuleIds.value.clear()
    dismissedGifts.value = []
    soundService.playRemoveItem()
  }

  async function applyVoucher(code: string) {
    const trimmed = code.trim().toUpperCase()
    if (!trimmed) return

    const res = await api.get(`/vouchers/validate/${trimmed}`)
    if (res.data.success && res.data.data) {
      const v = res.data.data
      if (v.quota > 0 && v.used_count >= v.quota) {
        throw new Error(`Kuota penggunaan kupon voucher '${v.code}' telah habis (${v.used_count}/${v.quota})`)
      }
      if (subtotal.value < v.min_purchase_amount) {
        throw new Error(`Minimal belanja untuk voucher ini adalah Rp ${v.min_purchase_amount.toLocaleString('id-ID')}`)
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
    soundService.playHoldRecall()
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
      soundService.playHoldRecall()
      return order
    }
    return null
  }

  function getCustomerDisplayState(status: 'IDLE' | 'ACTIVE' | 'PAYMENT_PENDING' | 'PAYMENT_SUCCESS' = 'ACTIVE'): CustomerDisplayState {
    const displayItems: CustomerDisplayItem[] = items.value.map((it) => ({
      id: it.id,
      name: it.product.name,
      sku: it.product.sku,
      barcode: it.product.barcode,
      quantity: it.quantity,
      unit_code: it.selected_unit.unit_code,
      unit_price: it.original_price || it.unit_price,
      discount_amount: (it.discount_amount || 0) + (it.flash_sale_discount || 0) + (it.promo_discount || 0) + (it.member_discount || 0),
      subtotal: it.subtotal,
    }))

    return {
      status: items.value.length === 0 ? 'IDLE' : status,
      items: displayItems,
      selectedCustomer: selectedCustomer.value
        ? {
            id: selectedCustomer.value.id,
            name: selectedCustomer.value.name,
            phone: selectedCustomer.value.phone,
            customer_group_name: selectedCustomer.value.group_name,
          }
        : null,
      subtotal: subtotal.value,
      itemDiscounts: itemDiscounts.value,
      voucherCode: voucherCode.value,
      voucherDiscount: voucherDiscount.value,
      totalDiscount: totalDiscount.value,
      totalTax: totalTax.value,
      grandTotal: grandTotal.value,
      totalItemsCount: totalItemsCount.value,
      lastScannedItem: displayItems.length > 0 ? displayItems[0] : null,
      updatedAt: Date.now(),
    }
  }

  function syncToCustomerDisplay() {
    broadcastCustomerDisplayState(getCustomerDisplayState())
  }

  // Realtime reactivity: sinkronkan ke Customer Display setiap ada perubahan
  watch(
    [items, selectedCustomer, voucherCode, voucherDiscount],
    () => {
      syncToCustomerDisplay()
    },
    { deep: true }
  )

  // Tanggapi permintaan sinkronisasi dari Customer Display jika baru dibuka
  listenForSyncRequests(() => getCustomerDisplayState())

  return {
    items,
    selectedCustomer,
    voucherCode,
    activeFlashSales,
    activePromotions,
    eligibleGifts,
    outOfStockGifts,
    dismissedGifts,
    isLoadingPromos,
    fetchActivePromos,
    evaluatePromotions,
    claimFreeGift,
    dismissGift,
    restoreGift,
    totalFlashSaleSavings,
    totalMemberSavings,
    totalPromoSavings,
    totalSavings,
    heldOrders,
    currentOutletId,
    setOutlet,
    initOutlet,
    grossSubtotal,
    subtotal,
    itemDiscounts,
    voucherDiscount,
    totalDiscount,
    totalTax,
    grandTotal,
    totalItemsCount,
    scanBarcode,
    addProduct,
    addProductToCart,
    getMaxAllowedQuantity,
    updateQuantity,
    updateItemDiscount,
    changeUnit,
    removeItem,
    clearCart,
    applyVoucher,
    removeVoucher,
    applyCustomerToCart,
    holdCurrentOrder,
    recallOrder,
    getCustomerDisplayState,
    syncToCustomerDisplay,
  }
})
