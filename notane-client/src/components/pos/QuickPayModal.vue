<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { X, Check, QrCode, CreditCard, Banknote, Coins, Sparkles, Camera, FileText, Trash2 } from 'lucide-vue-next'
import { useCartStore } from '@/stores/cartStore'
import { useShiftStore } from '@/stores/shiftStore'
import { useAuthStore } from '@/stores/authStore'
import { usePrinterStore } from '@/stores/printerStore'
import { api } from '@/services/api'
import { broadcastPaymentSuccess } from '@/services/customerDisplayChannel'
import { soundService } from '@/services/soundService'
import { Icon } from '@iconify/vue'
import { formatCurrency } from '@/utils/formatters'
import type { PaymentEntry, PaymentMethod, CheckoutResult, LoyaltySettings } from '@/types'

const props = defineProps<{
  show: boolean
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'success', result: CheckoutResult): void
}>()

const cartStore = useCartStore()
const shiftStore = useShiftStore()
const authStore = useAuthStore()
const printerStore = usePrinterStore()

const paymentMethods = ref<PaymentMethod[]>([
  { id: 'f0000000-0000-0000-0000-000000000001', code: 'CASH', name: 'Uang Tunai (Cash)', category: 'CASH' },
  { id: 'f0000000-0000-0000-0000-000000000002', code: 'QRIS', name: 'QRIS Dinamis', category: 'QRIS' },
  { id: 'f0000000-0000-0000-0000-000000000003', code: 'EDC_BCA', name: 'Kartu Debit/Kredit EDC', category: 'EDC' },
  { id: 'f0000000-0000-0000-0000-000000000004', code: 'POINTS', name: 'Poin Reward Member', category: 'POINTS' },
])

const activeMethod = ref<string>('CASH')
const cashAmount = ref<number>(0)
const edcApprovalCode = ref<string>('')
const qrisRefNumber = ref<string>('')
const qrisProofImage = ref<string | null>(null)
const qrisProofFileName = ref<string>('')
const qrisProofInputRef = ref<HTMLInputElement | null>(null)
const isUploadingProof = ref<boolean>(false)
const isProcessing = ref<boolean>(false)
const errorMsg = ref<string>('')
const receiptData = ref<CheckoutResult | null>(null)
const showReceipt = ref<boolean>(false)
const printStatus = ref<string>('')
const isPrinting = ref<boolean>(false)

// QRIS Proof of Payment Upload Handlers
function handleQrisProofSelect(e: Event) {
  const target = e.target as HTMLInputElement
  const file = target.files?.[0]
  if (!file) return

  if (file.size > 5 * 1024 * 1024) {
    errorMsg.value = 'Ukuran foto struk maksimal 5MB'
    return
  }

  qrisProofFileName.value = file.name
  const reader = new FileReader()
  reader.onload = (event) => {
    qrisProofImage.value = event.target?.result as string
  }
  reader.readAsDataURL(file)
}

function clearQrisProof() {
  qrisProofImage.value = null
  qrisProofFileName.value = ''
  if (qrisProofInputRef.value) {
    qrisProofInputRef.value.value = ''
  }
}

// Loyalty Point Redemption State
const loyaltySettings = ref<LoyaltySettings | null>(null)
const appliedPoints = ref<number>(0)
const customPointsInput = ref<number>(0)

async function loadLoyaltySettings() {
  try {
    const res = await api.get('/settings/loyalty')
    if (res.data?.success && res.data.data) {
      loyaltySettings.value = {
        id: res.data.data.id,
        is_enabled: res.data.data.is_enabled,
        point_value_rupiah: parseFloat(res.data.data.point_value_rupiah) || 100,
        min_spend_per_point: parseFloat(res.data.data.min_spend_per_point) || 10000,
        max_point_usage_percent: parseFloat(res.data.data.max_point_usage_percent) || 50,
        min_redeem_points: res.data.data.min_redeem_points || 1,
      }
    }
  } catch (err) {
    console.error('Gagal mengambil pengaturan loyalty di POS:', err)
  }
}

const customerPoints = computed(() => cartStore.selectedCustomer?.reward_points || 0)
const pointValueRupiah = computed(() => loyaltySettings.value?.point_value_rupiah || 100)
const customerPointsRupiah = computed(() => customerPoints.value * pointValueRupiah.value)

const maxPointUsagePercent = computed(() => loyaltySettings.value?.max_point_usage_percent ?? 50)
const maxAllowedPointsRupiah = computed(() => {
  if (!loyaltySettings.value?.is_enabled) return 0
  const pct = maxPointUsagePercent.value
  return Math.floor((cartStore.grandTotal * pct) / 100)
})

const maxAllowedPointsCount = computed(() => {
  if (pointValueRupiah.value <= 0) return 0
  const maxFromCap = Math.floor(maxAllowedPointsRupiah.value / pointValueRupiah.value)
  return Math.min(maxFromCap, customerPoints.value)
})

const appliedPointsRupiah = computed(() => appliedPoints.value * pointValueRupiah.value)
const remainingBill = computed(() => Math.max(0, cartStore.grandTotal - appliedPointsRupiah.value))

function applyPoints(pts: number) {
  errorMsg.value = ''
  if (pts <= 0) {
    removePoints()
    return
  }
  const minRedeem = loyaltySettings.value?.min_redeem_points || 1
  if (pts < minRedeem) {
    errorMsg.value = `Minimal penukaran adalah ${minRedeem} poin`
    return
  }
  const target = Math.max(0, Math.min(pts, maxAllowedPointsCount.value))
  appliedPoints.value = target
  customPointsInput.value = target
  setExactCash()
}

function applyPointsPercent(pct: number) {
  if (cartStore.grandTotal <= 0 || pointValueRupiah.value <= 0) return
  const allowedPct = Math.min(pct, maxPointUsagePercent.value)
  const targetRupiah = (cartStore.grandTotal * allowedPct) / 100
  const pts = Math.min(customerPoints.value, Math.floor(targetRupiah / pointValueRupiah.value))
  applyPoints(pts)
}

function removePoints() {
  appliedPoints.value = 0
  customPointsInput.value = 0
  errorMsg.value = ''
  setExactCash()
}

const isDonationApplied = ref<boolean>(false)

const roundingMode = computed(() => {
  return printerStore.receiptSettings?.cash_rounding_mode || 'NONE'
})

const cashRoundingAdjustment = computed(() => {
  if (activeMethod.value !== 'CASH') return 0
  const bill = Math.round(remainingBill.value * 100) / 100
  if (roundingMode.value === 'ROUND_100_DOWN') {
    return bill % 100
  } else if (roundingMode.value === 'ROUND_500_DOWN') {
    return bill % 500
  } else {
    // Mode NONE: jika ada pecahan koma desimal uang tunai (misal Rp 15.234,50), bulatkan ke rupiah utuh
    const decimalFraction = bill - Math.floor(bill)
    if (decimalFraction > 0) {
      return Number(decimalFraction.toFixed(2))
    }
  }
  return 0
})

const effectivePayableBill = computed(() => {
  return Math.max(0, Math.round(remainingBill.value - cashRoundingAdjustment.value))
})

const rawChangeAmount = computed(() => {
  if (activeMethod.value === 'CASH') {
    return Math.max(0, (cashAmount.value || 0) - effectivePayableBill.value)
  }
  return 0
})

const donationEligibleCoins = computed(() => {
  if (roundingMode.value !== 'DONATION_ROUND' || rawChangeAmount.value <= 0) return 0
  const remainder = rawChangeAmount.value % 1000
  return remainder > 0 ? remainder : (rawChangeAmount.value < 1000 ? rawChangeAmount.value : 0)
})

const donationAmount = computed(() => {
  if (isDonationApplied.value && donationEligibleCoins.value > 0) {
    return donationEligibleCoins.value
  }
  return 0
})

const changeAmount = computed(() => {
  return Math.max(0, rawChangeAmount.value - donationAmount.value)
})

const totalPaid = computed(() => {
  const ptsPaid = appliedPointsRupiah.value
  if (remainingBill.value === 0) {
    return ptsPaid
  }
  if (activeMethod.value === 'CASH') {
    return ptsPaid + (cashAmount.value || 0)
  } else if (activeMethod.value === 'QRIS' || activeMethod.value === 'EDC') {
    return ptsPaid + remainingBill.value
  }
  return ptsPaid
})

const receiptConfig = computed(() => {
  return printerStore.receiptSettings || {
    store_name: 'NOTANE RETAIL ENTERPRISE',
    store_tagline: 'Solusi Ritel Pintar & Modern',
    store_address: 'Jl. Surya Kencana No. 88, Bogor Tengah, Jawa Barat',
    store_phone: '0812-3456-7890',
    store_website: 'www.notaneretail.id',
    tax_id: '01.234.567.8-901.000',
    header_note: 'Selamat Datang & Selamat Berbelanja',
    footer_note_1: 'Barang yang sudah dibeli tidak dapat ditukar/dikembalikan',
    footer_note_2: 'Kritik & Saran WA: 0812-3456-7890 | Terima Kasih',
    social_media: 'IG & TikTok: @notane.retail',
    paper_width: '58mm',
    show_phone: true,
    show_address: true,
    show_cashier_name: true,
    show_barcode: true,
    show_tax_summary: true,
    show_flash_sale_discount: true,
    show_member_discount: true,
    show_savings_summary: true,
    cash_rounding_mode: 'NONE',
    max_cash_drawer_limit: 2000000,
  }
})

function itemUnitCatalogPrice(item: any): number {
  const orig = Number(item.original_price || 0)
  const selling = Number(item.unit_selling_price || item.unit_price || 0)
  return orig > selling ? orig : selling
}

function itemGrossTotal(item: any): number {
  const qty = Number(item.quantity || 1)
  return itemUnitCatalogPrice(item) * qty
}

function getItemOtherDiscount(item: any): number {
  const totalDisc = Number(item.discount_amount || 0)
  const fsDisc = Number(item.flash_sale_discount || 0)
  const memDisc = Number(item.member_discount || 0)
  const diff = totalDisc - fsDisc - memDisc
  return diff > 0 ? diff : 0
}

function getSaleOtherDiscount(): number {
  const sale: any = receiptData.value?.sale || receiptData.value
  const totalDisc = Number(sale?.discount_amount || 0)
  const fsDisc = Number(sale?.total_flash_sale_discount || 0)
  const memDisc = Number(sale?.total_member_discount || 0)
  const diff = totalDisc - fsDisc - memDisc
  return diff > 0 ? diff : 0
}

function setExactCash() {
  cashAmount.value = Math.round(effectivePayableBill.value)
}

function addCash(denomination: number) {
  cashAmount.value = (cashAmount.value || 0) + denomination
}

async function handleCompletePayment() {
  if (!shiftStore.currentShift) {
    errorMsg.value = 'Shift kasir tidak aktif. Silakan buka shift terlebih dahulu.'
    return
  }

  const finalPayments: PaymentEntry[] = []

  // 1. Masukkan pembayaran Poin Reward jika ada
  if (appliedPoints.value > 0 && appliedPointsRupiah.value > 0) {
    const pointsMethod = paymentMethods.value.find((m) => m.category === 'POINTS') || {
      id: 'f0000000-0000-0000-0000-000000000004',
      code: 'POINTS',
      name: 'Poin Reward Member',
      category: 'POINTS',
    }
    finalPayments.push({
      payment_method_id: pointsMethod.id,
      method_name: `Poin Reward (${appliedPoints.value} Pts)`,
      category: 'POINTS',
      amount: appliedPointsRupiah.value,
      points: appliedPoints.value,
    })
  }

  // 2. Masukkan metode pembayaran kedua (Tunai, QRIS, atau EDC) untuk sisa tagihan
  if (remainingBill.value === 0 && appliedPointsRupiah.value >= cartStore.grandTotal) {
    // Sudah lunas 100% dengan poin
  } else if (activeMethod.value === 'CASH') {
    if (cashAmount.value > 0) {
      const cashMethod = paymentMethods.value.find((m) => m.category === 'CASH')!
      finalPayments.push({
        payment_method_id: cashMethod.id,
        method_name: cashMethod.name,
        category: 'CASH',
        amount: cashAmount.value,
      })
    }
  } else if (activeMethod.value === 'QRIS') {
    let qrisProofUrl: string | undefined = undefined
    if (qrisProofImage.value) {
      try {
        isUploadingProof.value = true
        const uploadRes = await api.post('/sales/upload-proof', {
          filename: qrisProofFileName.value || 'qris_proof.jpg',
          data_base64: qrisProofImage.value,
        })
        if (uploadRes.data?.success && uploadRes.data?.proof_url) {
          qrisProofUrl = uploadRes.data.proof_url
        }
      } catch (uploadErr: any) {
        console.warn('Gagal mengupload bukti QRIS, melanjutkan checkout:', uploadErr)
      } finally {
        isUploadingProof.value = false
      }
    }

    const qrisMethod = paymentMethods.value.find((m) => m.category === 'QRIS')!
    finalPayments.push({
      payment_method_id: qrisMethod.id,
      method_name: qrisMethod.name,
      category: 'QRIS',
      amount: remainingBill.value,
      reference_number: qrisRefNumber.value.trim() || undefined,
      payment_proof_url: qrisProofUrl,
    })
  } else if (activeMethod.value === 'EDC') {
    const edcMethod = paymentMethods.value.find((m) => m.category === 'EDC')!
    finalPayments.push({
      payment_method_id: edcMethod.id,
      method_name: edcMethod.name,
      category: 'EDC',
      amount: remainingBill.value,
      reference_number: edcApprovalCode.value.trim() || undefined,
    })
  }

  const payableTarget = activeMethod.value === 'CASH'
    ? Math.round(effectivePayableBill.value) + appliedPointsRupiah.value
    : cartStore.grandTotal
  const total = finalPayments.reduce((sum, p) => sum + p.amount, 0)
  if (total < payableTarget) {
    errorMsg.value = `Total pembayaran masih kurang dari total belanja (Kurang: ${formatCurrency(payableTarget - total)})`
    return
  }

  // Pre-flight Stock Validation: Cegah checkout jika ada item yang melebihi stok tersedia
  for (const it of cartStore.items) {
    const maxStock = cartStore.getMaxAllowedQuantity(it)
    if (it.quantity > maxStock) {
      soundService.playOutOfStock()
      errorMsg.value = `Jumlah '${it.product.name}' (${it.quantity} ${it.selected_unit.unit_code}) melebihi stok yang tersedia (${maxStock} ${it.selected_unit.unit_code}). Silakan sesuaikan keranjang terlebih dahulu.`
      return
    }
  }

  isProcessing.value = true
  errorMsg.value = ''

  try {
    const payload = {
      customer_id: cartStore.selectedCustomer?.id,
      voucher_code: cartStore.voucherCode || undefined,
      items: cartStore.items.map((it) => ({
        product_id: it.product.id,
        unit_id: it.selected_unit.unit_id,
        quantity: it.quantity,
        unit_selling_price: it.unit_price,
        discount_percent: it.discount_percent,
        discount_amount: Number(((it.discount_amount || 0) + (it.promo_discount || 0)).toFixed(2)),
        flash_sale_id: it.flash_sale_id,
        original_price: it.original_price,
        flash_sale_discount: it.flash_sale_discount,
        member_discount: it.member_discount,
      })),
      payments: finalPayments.map((p) => ({
        payment_method_id: p.payment_method_id,
        amount: p.category === 'POINTS' ? 0 : p.amount,
        points: p.category === 'POINTS' ? (p.points ?? appliedPoints.value) : undefined,
        reference_number: p.reference_number,
        payment_proof_url: p.payment_proof_url,
      })),
      notes: `Kasir: ${authStore.user?.full_name}`,
      rounding_amount: activeMethod.value === 'CASH' && cashRoundingAdjustment.value > 0 ? Number(cashRoundingAdjustment.value.toFixed(2)) : undefined,
    }

    const res = await api.post('/sales/checkout', payload)
    if (res.data.success) {
      soundService.playPaymentSuccess()
      receiptData.value = res.data.data
      showReceipt.value = true

      const matchedMethod = paymentMethods.value.find((m) => m.code === activeMethod.value)
      broadcastPaymentSuccess({
        tendered: totalPaid.value,
        change: changeAmount.value,
        paymentMethod: matchedMethod?.name || 'Tunai',
        invoiceNumber: res.data.data?.invoice_number || 'TRX-SUCCESS',
        grandTotal: cartStore.grandTotal,
        customerName: cartStore.selectedCustomer?.name,
      })

      cartStore.clearCart()
      emit('success', res.data.data)

      // Cetak otomatis ke printer thermal yang terhubung tanpa memunculkan dialog browser
      if (printerStore.autoPrint && printerStore.isConnected) {
        try {
          isPrinting.value = true
          const pRes = await printerStore.printReceipt(res.data.data)
          printStatus.value = pRes.message
        } catch (pErr: any) {
          console.warn('Gagal cetak otomatis:', pErr)
          printStatus.value = `Gagal cetak otomatis: ${pErr.message}`
        } finally {
          isPrinting.value = false
        }
      }
    } else {
      soundService.playError()
      errorMsg.value = res.data.message || 'Gagal memproses transaksi'
    }
  } catch (err: any) {
    soundService.playError()
    errorMsg.value = err.response?.data?.message || err.message || 'Gagal memproses pembayaran'
  } finally {
    isProcessing.value = false
  }
}

async function printReceipt() {
  if (printerStore.isConnected && receiptData.value) {
    try {
      isPrinting.value = true
      const pRes = await printerStore.printReceipt(receiptData.value)
      printStatus.value = pRes.message
    } catch (err: any) {
      printStatus.value = `Gagal cetak: ${err.message}`
    } finally {
      isPrinting.value = false
    }
  } else {
    // Fallback jika printer fisik/simulator tidak terhubung
    window.print()
  }
}

function handleKey(e: KeyboardEvent) {
  if (e.key === 'Escape' && props.show) {
    if (showReceipt.value) {
      showReceipt.value = false
      emit('close')
    } else {
      emit('close')
    }
  }
}

watch(
  () => props.show,
  (newVal) => {
    if (newVal) {
      loadLoyaltySettings()
      appliedPoints.value = 0
      customPointsInput.value = 0
      qrisRefNumber.value = ''
      clearQrisProof()
      edcApprovalCode.value = ''
      errorMsg.value = ''
      setExactCash()
    }
  }
)

onMounted(() => {
  window.addEventListener('keydown', handleKey)
  loadLoyaltySettings()
  setExactCash()
  if (!printerStore.receiptSettings) {
    printerStore.fetchReceiptSettings()
  }
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKey)
})
</script>

<template>
  <div
    v-if="show"
    class="fixed inset-0 z-50 bg-slate-900/50 backdrop-blur-xs flex items-center justify-center p-4 select-none animate-fadeIn"
  >
    <!-- Main Payment Modal -->
    <div
      v-if="!showReceipt"
      class="bg-white border border-slate-300 w-full max-w-2xl rounded-2xl shadow-2xl overflow-hidden flex flex-col text-slate-900"
    >
      <!-- Header -->
      <div class="bg-slate-100 p-4 px-6 border-b border-slate-300 flex items-center justify-between">
        <div>
          <h3 class="text-base font-bold text-slate-900 flex items-center gap-2">
            <Banknote class="w-5 h-5 text-emerald-600" />
            Pembayaran Transaksi POS
          </h3>
          <p class="text-xs text-slate-500">Pilih metode pembayaran dan masukkan nominal yang diterima.</p>
        </div>
        <button
          @click="emit('close')"
          class="text-slate-400 hover:text-slate-700 p-1.5 rounded-lg hover:bg-slate-200 transition"
        >
          <X class="w-5 h-5" />
        </button>
      </div>

      <!-- Content -->
      <div class="p-6 space-y-4 flex-1 overflow-y-auto max-h-[75vh]">
        <!-- Total Belanja & Status Tagihan Banner -->
        <div class="bg-slate-50 border border-slate-300 rounded-xl p-4">
          <div class="grid grid-cols-2 md:grid-cols-3 gap-4 items-center">
            <div>
              <span class="text-[11px] font-bold text-slate-500 uppercase tracking-wider">Total Tagihan</span>
              <div class="text-2xl font-extrabold text-slate-900 font-mono mt-0.5">
                {{ formatCurrency(cartStore.grandTotal) }}
              </div>
            </div>

            <div v-if="appliedPoints > 0" class="bg-amber-100/60 border border-amber-300/80 px-3 py-1.5 rounded-xl">
              <span class="text-[10px] font-bold text-amber-800 uppercase tracking-wider flex items-center gap-1">
                <Coins class="w-3 h-3 text-amber-600" />
                Potongan Poin ({{ appliedPoints }} Pts)
              </span>
              <div class="text-lg font-extrabold text-amber-700 font-mono">
                -{{ formatCurrency(appliedPointsRupiah) }}
              </div>
            </div>

            <div :class="appliedPoints > 0 ? '' : 'col-span-1'">
              <span class="text-[11px] font-bold text-slate-500 uppercase tracking-wider">
                {{ appliedPoints > 0 ? 'Sisa Tagihan' : 'Kembalian' }}
              </span>
              <div
                class="text-2xl font-extrabold font-mono mt-0.5"
                :class="appliedPoints > 0 ? 'text-emerald-700' : (changeAmount > 0 ? 'text-amber-800' : 'text-slate-400')"
              >
                {{ appliedPoints > 0 ? formatCurrency(remainingBill) : formatCurrency(changeAmount) }}
              </div>
            </div>
          </div>

          <!-- Kembalian Bar jika ada sisa tagihan & kembalian tunai -->
          <div v-if="appliedPoints > 0 && activeMethod === 'CASH' && changeAmount > 0" class="mt-2.5 pt-2 border-t border-slate-200 flex items-center justify-between text-xs">
            <span class="font-bold text-slate-600">Uang Kembalian Tunai:</span>
            <span class="font-extrabold text-amber-800 font-mono text-sm bg-amber-100 px-2 py-0.5 rounded">
              {{ formatCurrency(changeAmount) }}
            </span>
          </div>

          <!-- Total Hemat Bar -->
          <div v-if="cartStore.totalSavings > 0" class="w-full mt-2.5 pt-2 border-t border-slate-200 flex items-center justify-between text-xs">
            <span class="text-emerald-700 font-semibold flex items-center gap-1">
              <Icon icon="fluent-color:savings-24" class="w-4 h-4" />
              Total Anda Hemat:
            </span>
            <span class="font-bold text-emerald-800 font-mono bg-emerald-100 px-2 py-0.5 rounded">
              -{{ formatCurrency(cartStore.totalSavings) }}
            </span>
          </div>
        </div>

        <!-- LOYALTY REWARD POINTS SECTION (Bisa Digunakan Bersamaan) -->
        <div class="bg-amber-50/50 border border-amber-200 rounded-xl p-3.5 space-y-3">
          <!-- Member Info Header -->
          <div class="flex flex-wrap items-center justify-between gap-2">
            <div class="flex items-center gap-2">
              <div class="w-7 h-7 rounded-lg bg-amber-100 border border-amber-300 flex items-center justify-center text-amber-700">
                <Coins class="w-4 h-4" />
              </div>
              <div>
                <div class="text-xs font-bold text-slate-900 flex items-center gap-1.5">
                  <span>{{ cartStore.selectedCustomer?.name || 'Pelanggan Umum (Bukan Member)' }}</span>
                  <span v-if="cartStore.selectedCustomer" class="px-1.5 py-0.2 bg-amber-200 text-amber-900 text-[10px] rounded font-mono font-bold">
                    MEMBER
                  </span>
                </div>
                <div v-if="cartStore.selectedCustomer" class="text-[11px] text-amber-800 font-medium">
                  Saldo: <strong>{{ customerPoints.toLocaleString() }} Poin</strong> (Rp {{ customerPointsRupiah.toLocaleString() }})
                </div>
                <div v-else class="text-[11px] text-slate-500">
                  Pilih member di kasir untuk menggunakan saldo poin belanja.
                </div>
              </div>
            </div>

            <!-- Limit Badge -->
            <div v-if="cartStore.selectedCustomer && loyaltySettings?.is_enabled" class="text-right">
              <span class="text-[10px] font-bold text-slate-500 uppercase tracking-wider block">Batas Maks. Poin</span>
              <span class="text-xs font-mono font-bold text-indigo-700 bg-indigo-50 border border-indigo-200 px-2 py-0.5 rounded-md inline-block">
                {{ maxPointUsagePercent }}% (Maks. {{ maxAllowedPointsCount }} Pts)
              </span>
            </div>
          </div>

          <!-- Controls when Member is Selected -->
          <div v-if="cartStore.selectedCustomer && loyaltySettings?.is_enabled" class="pt-2 border-t border-amber-200/80 space-y-2">
            <!-- If Points Applied -->
            <div v-if="appliedPoints > 0" class="flex items-center justify-between bg-white border border-amber-300 p-2.5 rounded-lg shadow-xs">
              <div class="flex items-center gap-2 text-xs">
                <Sparkles class="w-4 h-4 text-amber-600 shrink-0" />
                <span class="text-slate-800">
                  Poin Diterapkan: <strong class="text-amber-800 font-mono">{{ appliedPoints }} Pts</strong>
                  (<strong class="text-emerald-700 font-mono">-{{ formatCurrency(appliedPointsRupiah) }}</strong>)
                </span>
              </div>
              <button
                @click="removePoints"
                class="text-xs font-bold text-rose-600 hover:text-rose-800 hover:bg-rose-50 px-2.5 py-1 rounded-md transition border border-rose-200 active:scale-95 cursor-pointer flex items-center gap-1"
              >
                <X class="w-3.5 h-3.5" />
                <span>Batal Poin</span>
              </button>
            </div>

            <!-- If Points Not Yet Applied -->
            <div v-else-if="maxAllowedPointsCount > 0" class="space-y-2">
              <div class="flex items-center gap-2">
                <span class="text-[11px] font-bold text-slate-600">Pilihan Cepat:</span>
                <button
                  type="button"
                  @click="applyPointsPercent(25)"
                  class="bg-white hover:bg-amber-100 text-amber-900 border border-amber-300 text-[11px] font-bold px-2.5 py-1 rounded-md transition active:scale-95 shadow-2xs"
                >
                  25% Tagihan
                </button>
                <button
                  type="button"
                  @click="applyPointsPercent(50)"
                  class="bg-white hover:bg-amber-100 text-amber-900 border border-amber-300 text-[11px] font-bold px-2.5 py-1 rounded-md transition active:scale-95 shadow-2xs"
                >
                  50% Tagihan
                </button>
                <button
                  type="button"
                  @click="applyPoints(maxAllowedPointsCount)"
                  class="bg-amber-600 hover:bg-amber-700 text-white border border-amber-700 text-[11px] font-bold px-3 py-1 rounded-md transition active:scale-95 shadow-2xs flex items-center gap-1"
                >
                  <Sparkles class="w-3 h-3" />
                  <span>Gunakan Maksimal ({{ maxAllowedPointsCount }} Pts)</span>
                </button>
              </div>

              <!-- Manual Points Input -->
              <div class="flex items-center gap-2">
                <div class="relative flex-1">
                  <input
                    v-model.number="customPointsInput"
                    type="number"
                    min="1"
                    :max="maxAllowedPointsCount"
                    placeholder="Masukkan jumlah poin..."
                    class="w-full bg-white border border-slate-300 rounded-lg px-3 py-1.5 text-xs font-mono font-bold text-slate-900 focus:outline-none focus:border-amber-600"
                  />
                  <span class="absolute right-3 top-1/2 -translate-y-1/2 text-[10px] text-slate-400 font-bold">Poin</span>
                </div>
                <button
                  type="button"
                  @click="applyPoints(customPointsInput)"
                  :disabled="customPointsInput <= 0"
                  class="bg-slate-800 hover:bg-slate-900 disabled:opacity-50 text-white font-bold text-xs px-3.5 py-1.5 rounded-lg transition active:scale-95 cursor-pointer"
                >
                  Terapkan Poin
                </button>
              </div>
            </div>

            <div v-else class="text-[11px] text-slate-500 italic">
              Saldo poin member tidak mencukupi untuk penukaran transaksi ini.
            </div>
          </div>
        </div>

        <!-- PAYMENT METHOD SELECTION UNTUK SISA TAGIHAN -->
        <div v-if="remainingBill > 0" class="space-y-3">
          <div class="flex items-center justify-between">
            <span class="text-xs font-bold text-slate-700 uppercase tracking-wider">
              Metode Pembayaran Sisa Tagihan ({{ formatCurrency(remainingBill) }})
            </span>
          </div>

          <!-- Payment Method Tabs -->
          <div class="grid grid-cols-3 gap-2">
            <button
              @click="activeMethod = 'CASH'; setExactCash()"
              class="flex flex-col items-center justify-center p-3 rounded-xl border text-xs font-bold transition active:scale-95 shadow-xs"
              :class="activeMethod === 'CASH' ? 'bg-emerald-50 border-emerald-600 text-emerald-800' : 'bg-slate-50 border-slate-300 text-slate-700 hover:bg-slate-100'"
            >
              <Banknote class="w-5 h-5 mb-1" />
              <span>Tunai (Cash)</span>
            </button>

            <button
              @click="activeMethod = 'QRIS'"
              class="flex flex-col items-center justify-center p-3 rounded-xl border text-xs font-bold transition active:scale-95 shadow-xs"
              :class="activeMethod === 'QRIS' ? 'bg-sky-50 border-sky-600 text-sky-800' : 'bg-slate-50 border-slate-300 text-slate-700 hover:bg-slate-100'"
            >
              <QrCode class="w-5 h-5 mb-1" />
              <span>QRIS Dinamis</span>
            </button>

            <button
              @click="activeMethod = 'EDC'"
              class="flex flex-col items-center justify-center p-3 rounded-xl border text-xs font-bold transition active:scale-95 shadow-xs"
              :class="activeMethod === 'EDC' ? 'bg-indigo-50 border-indigo-600 text-indigo-800' : 'bg-slate-50 border-slate-300 text-slate-700 hover:bg-slate-100'"
            >
              <CreditCard class="w-5 h-5 mb-1" />
              <span>Debit / Kartu EDC</span>
            </button>
          </div>

          <!-- Cash Details & Quick Denomination Buttons -->
          <div v-if="activeMethod === 'CASH'" class="space-y-3 pt-1">
            <div class="flex items-center justify-between">
              <label class="block text-xs font-bold text-slate-700">Uang Tunai Diterima (Rp)</label>
              <span class="text-[11px] text-slate-500 font-medium">Wajib: {{ formatCurrency(remainingBill) }}</span>
            </div>
            <input
              v-model.number="cashAmount"
              type="number"
              class="w-full bg-slate-50 border-2 border-slate-300 rounded-xl px-4 py-2.5 text-2xl font-mono text-slate-900 text-right focus:bg-white focus:outline-none focus:border-emerald-600 focus:ring-2 focus:ring-emerald-500/20 font-bold"
            />

            <!-- Quick Denominations -->
            <div class="grid grid-cols-5 gap-2 pt-1">
              <button
                @click="setExactCash"
                class="bg-slate-100 hover:bg-slate-200 text-slate-800 font-mono text-xs font-bold py-2 rounded-lg border border-slate-300 transition active:scale-95 shadow-xs"
              >
                Uang Pas
              </button>
              <button
                @click="addCash(10000)"
                class="bg-slate-100 hover:bg-slate-200 text-slate-800 font-mono text-xs font-bold py-2 rounded-lg border border-slate-300 transition active:scale-95 shadow-xs"
              >
                +10.000
              </button>
              <button
                @click="addCash(20000)"
                class="bg-slate-100 hover:bg-slate-200 text-slate-800 font-mono text-xs font-bold py-2 rounded-lg border border-slate-300 transition active:scale-95 shadow-xs"
              >
                +20.000
              </button>
              <button
                @click="addCash(50000)"
                class="bg-slate-100 hover:bg-slate-200 text-slate-800 font-mono text-xs font-bold py-2 rounded-lg border border-slate-300 transition active:scale-95 shadow-xs"
              >
                +50.000
              </button>
              <button
                @click="addCash(100000)"
                class="bg-slate-100 hover:bg-slate-200 text-slate-800 font-mono text-xs font-bold py-2 rounded-lg border border-slate-300 transition active:scale-95 shadow-xs"
              >
                +100.000
              </button>
            </div>

            <!-- Cash Rounding Policy Badge -->
            <div v-if="cashRoundingAdjustment > 0" class="p-2.5 bg-purple-50 border border-purple-200 rounded-xl flex items-center justify-between text-xs">
              <div class="flex items-center gap-1.5 text-purple-900 font-semibold">
                <Icon icon="fluent-color:money-calculator-24" class="w-4 h-4 shrink-0" />
                <span>Pembulatan Kebawah:</span>
              </div>
              <div class="text-right">
                <span class="text-purple-600 line-through text-[11px] font-mono mr-1.5">{{ formatCurrency(remainingBill) }}</span>
                <span class="font-extrabold text-purple-900 font-mono">{{ formatCurrency(effectivePayableBill) }}</span>
                <span class="text-[10px] text-purple-600 block">Potongan kasir Rp {{ cashRoundingAdjustment }}</span>
              </div>
            </div>

            <!-- Donation Option Toggle (Indomaret Style) -->
            <div v-if="donationEligibleCoins > 0" class="p-3 bg-amber-50/80 border border-amber-300 rounded-xl flex items-center justify-between text-xs">
              <div class="space-y-0.5">
                <div class="font-bold text-amber-900 flex items-center gap-1.5">
                  <Icon icon="fluent-color:heart-24" class="w-4 h-4 shrink-0" />
                  <span>Donasi Kembalian Koin Receh</span>
                </div>
                <div class="text-[11px] text-slate-600">
                  Salurkan koin <strong class="font-mono text-amber-800">{{ formatCurrency(donationEligibleCoins) }}</strong> ke Kotak Amal / Donasi?
                </div>
              </div>
              <label class="flex items-center gap-2 cursor-pointer select-none">
                <input
                  type="checkbox"
                  v-model="isDonationApplied"
                  class="w-4 h-4 text-amber-600 rounded focus:ring-amber-500 cursor-pointer"
                />
                <span class="font-bold text-amber-900 text-xs">{{ isDonationApplied ? 'Disetujui' : 'Tidak' }}</span>
              </label>
            </div>
          </div>

          <!-- QRIS Dinamis Simulation & Manual Proof Input -->
          <div v-else-if="activeMethod === 'QRIS'" class="bg-slate-50 p-4 rounded-xl border border-slate-300 space-y-3">
            <div class="flex items-center justify-between border-b border-slate-200 pb-2">
              <div class="flex items-center gap-2">
                <div class="p-1.5 rounded-lg bg-sky-100 text-sky-700">
                  <QrCode class="w-4 h-4" />
                </div>
                <div>
                  <h5 class="text-xs font-bold text-slate-900">QRIS Dinamis</h5>
                  <p class="text-[10px] text-slate-500">Scan via Layar Pelanggan atau EDC / Mobile POS</p>
                </div>
              </div>
              <div class="text-right">
                <span class="text-[10px] text-slate-400 block font-medium">Nominal QRIS</span>
                <span class="text-sm font-mono font-black text-sky-700">{{ formatCurrency(remainingBill) }}</span>
              </div>
            </div>

            <!-- Notice EDC / Manual QRIS -->
            <div class="bg-sky-50/70 border border-sky-200 rounded-lg p-2.5 text-[11px] text-sky-950 flex items-start gap-2">
              <Icon icon="fluent:info-16-regular" class="w-4 h-4 text-sky-600 shrink-0 mt-0.5" />
              <div class="leading-relaxed">
                Jika QRIS dibuat langsung dari mesin EDC atau aplikasi Mobile POS bank, Anda dapat mengisi <strong>Nomor Bukti / RRN</strong> dan melampirkan <strong>Foto Struk</strong> di bawah ini (opsional).
              </div>
            </div>

            <!-- Form Manual Input Bukti Bayar QRIS (Opsional) -->
            <div class="space-y-2.5">
              <div>
                <label class="block text-[11px] font-bold text-slate-700 mb-1 flex items-center justify-between">
                  <span>Nomor Bukti Transaksi / RRN / Trace No</span>
                  <span class="text-[10px] text-slate-400 font-normal">Opsional</span>
                </label>
                <div class="relative">
                  <FileText class="w-4 h-4 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2" />
                  <input
                    v-model="qrisRefNumber"
                    type="text"
                    placeholder="Contoh: RRN-123456789012 atau ID Transaksi"
                    class="w-full bg-white border border-slate-300 rounded-lg pl-9 pr-3 py-2 text-xs font-mono text-slate-900 placeholder:font-sans focus:outline-none focus:border-sky-600 focus:ring-1 focus:ring-sky-600"
                  />
                </div>
              </div>

              <!-- Upload Bukti Struk EDC (Opsional) -->
              <div>
                <label class="block text-[11px] font-bold text-slate-700 mb-1 flex items-center justify-between">
                  <span>Unggah Foto Bukti Struk EDC / Bukti Pembayaran</span>
                  <span class="text-[10px] text-slate-400 font-normal">Opsional</span>
                </label>

                <!-- Hidden file input -->
                <input
                  ref="qrisProofInputRef"
                  type="file"
                  accept="image/*"
                  capture="environment"
                  class="hidden"
                  @change="handleQrisProofSelect"
                />

                <!-- Image Selected Preview -->
                <div
                  v-if="qrisProofImage"
                  class="p-2 bg-white rounded-lg border border-slate-200 flex items-center justify-between gap-3"
                >
                  <div class="flex items-center gap-2.5 overflow-hidden">
                    <img
                      :src="qrisProofImage"
                      alt="Bukti Struk"
                      class="w-12 h-12 object-cover rounded-md border border-slate-200 shrink-0"
                    />
                    <div class="overflow-hidden">
                      <div class="text-xs font-bold text-slate-800 truncate">
                        {{ qrisProofFileName || 'Foto Bukti Terpilih' }}
                      </div>
                      <div class="text-[10px] text-emerald-600 font-medium flex items-center gap-1">
                        <Check class="w-3 h-3" />
                        <span>Siap disimpan ke arsip transaksi</span>
                      </div>
                    </div>
                  </div>

                  <div class="flex items-center gap-1.5 shrink-0">
                    <button
                      type="button"
                      @click="qrisProofInputRef?.click()"
                      class="p-1.5 text-xs text-sky-700 hover:bg-sky-50 rounded-md border border-sky-200 font-semibold"
                      title="Ganti Foto"
                    >
                      Ganti
                    </button>
                    <button
                      type="button"
                      @click="clearQrisProof"
                      class="p-1.5 text-xs text-rose-600 hover:bg-rose-50 rounded-md border border-rose-200"
                      title="Hapus Bukti"
                    >
                      <Trash2 class="w-3.5 h-3.5" />
                    </button>
                  </div>
                </div>

                <!-- Button to Select/Capture Image -->
                <div v-else>
                  <button
                    type="button"
                    @click="qrisProofInputRef?.click()"
                    class="w-full py-2.5 px-3 bg-white hover:bg-slate-100 border border-dashed border-slate-300 hover:border-sky-400 rounded-lg text-slate-600 hover:text-sky-700 text-xs font-medium flex items-center justify-center gap-2 transition cursor-pointer"
                  >
                    <Camera class="w-4 h-4 text-slate-500" />
                    <span>Ambil Foto atau Pilih File Struk (JPG/PNG)</span>
                  </button>
                </div>
              </div>
            </div>
          </div>

          <!-- EDC Card Simulation -->
          <div v-else-if="activeMethod === 'EDC'" class="bg-slate-50 p-4 rounded-xl border border-slate-300 space-y-2.5">
            <div class="flex justify-between items-center text-xs">
              <span class="font-bold text-slate-900">Mesin EDC Bank</span>
              <span class="font-mono font-bold text-indigo-700">{{ formatCurrency(remainingBill) }}</span>
            </div>
            <input
              v-model="edcApprovalCode"
              placeholder="Nomor Approval Code / Trace EDC (Opsional)"
              class="w-full bg-white border border-slate-300 rounded-lg px-3 py-2 text-xs text-slate-900 focus:outline-none focus:border-indigo-600"
            />
            <p class="text-[10px] text-slate-500">Gesek atau tap kartu di mesin EDC sesuai nominal sisa tagihan.</p>
          </div>
        </div>

        <!-- JIKA LUNAS PENUH DENGAN POIN -->
        <div v-else class="bg-emerald-50 border border-emerald-300 rounded-xl p-4 text-center space-y-1.5">
          <div class="inline-flex p-2 rounded-full bg-emerald-100 text-emerald-700 mb-1">
            <Check class="w-6 h-6" />
          </div>
          <h4 class="font-bold text-emerald-900 text-sm">Tagihan Lunas 100% Menggunakan Poin Reward!</h4>
          <p class="text-xs text-emerald-700">
            Seluruh tagihan senilai <strong>{{ formatCurrency(cartStore.grandTotal) }}</strong> telah ditutup dengan <strong>{{ appliedPoints }} Poin</strong>. Tidak memerlukan pembayaran uang tunai atau digital lainnya.
          </p>
        </div>

        <!-- Error Warning -->
        <div v-if="errorMsg" class="bg-rose-50 border border-rose-300 p-3 rounded-xl text-rose-700 text-xs font-semibold flex items-center gap-2">
          <span>⚠️</span>
          <span>{{ errorMsg }}</span>
        </div>
      </div>

      <!-- Modal Footer -->
      <div class="bg-slate-100 p-4 px-6 border-t border-slate-300 flex items-center justify-between">
        <button
          @click="emit('close')"
          class="bg-slate-200 hover:bg-slate-300 text-slate-700 font-bold px-4 py-2.5 rounded-xl text-xs transition active:scale-95 border border-slate-300"
        >
          Batal (Esc)
        </button>

        <button
          @click="handleCompletePayment"
          :disabled="isProcessing || totalPaid < cartStore.grandTotal"
          class="bg-emerald-600 hover:bg-emerald-700 disabled:opacity-50 disabled:pointer-events-none text-white px-6 py-2.5 rounded-xl text-sm font-bold flex items-center gap-2 shadow-sm border border-emerald-700 transition active:scale-95"
        >
          <Check class="w-4 h-4" />
          <span>{{ isProcessing ? 'Memproses...' : 'SELESAIKAN PEMBAYARAN' }}</span>
        </button>
      </div>
    </div>

    <!-- Thermal Receipt Preview Modal -->
    <div
      v-else
      class="bg-white border border-slate-300 w-full max-w-sm rounded-2xl shadow-2xl p-4 flex flex-col items-center space-y-4 text-slate-900"
    >
      <!-- Realistic Thermal Receipt Container -->
      <div class="w-full max-h-[70vh] overflow-y-auto flex justify-center p-2 bg-slate-100 rounded-xl border border-slate-200 shadow-inner">
        <!-- The Paper Slip -->
        <div
          id="thermal-receipt"
          class="bg-white text-slate-900 shadow-lg font-mono text-[11px] leading-tight p-4 select-none relative transition-all duration-300 w-full"
          :class="receiptConfig.paper_width === '80mm' ? 'max-w-[340px]' : 'max-w-[280px]'"
          style="filter: drop-shadow(0 4px 6px rgba(0,0,0,0.12));"
        >
          <!-- Top Jagged Paper Edge -->
          <div class="absolute -top-1.5 left-0 right-0 h-2 bg-repeat-x" style="background: radial-gradient(circle, transparent, transparent 50%, #ffffff 50%, #ffffff 100%); background-size: 8px 8px;"></div>

          <!-- STORE HEADER -->
          <div class="text-center pb-2 border-b border-dashed border-slate-400">
            <div class="font-extrabold text-sm tracking-wide uppercase">{{ receiptConfig.store_name || 'NOTANE RETAIL' }}</div>
            <div v-if="receiptConfig.store_tagline" class="text-[10px] text-slate-600 italic mt-0.5">{{ receiptConfig.store_tagline }}</div>
            <div v-if="receiptConfig.show_address && receiptConfig.store_address" class="text-[10px] text-slate-700 mt-1 leading-snug">{{ receiptConfig.store_address }}</div>
            <div v-if="receiptConfig.show_phone && receiptConfig.store_phone" class="text-[10px] text-slate-700 mt-0.5">Telp: {{ receiptConfig.store_phone }}</div>
            <div v-if="receiptConfig.store_website" class="text-[10px] text-slate-600">{{ receiptConfig.store_website }}</div>
            <div v-if="receiptConfig.tax_id" class="text-[9px] text-slate-500 font-mono mt-0.5">
              {{ (receiptConfig.tax_id || '').toUpperCase().startsWith('NPWP') ? receiptConfig.tax_id : 'NPWP: ' + receiptConfig.tax_id }}
            </div>
            <div v-if="receiptConfig.header_note" class="text-[10px] font-semibold text-slate-800 mt-1 border-t border-dotted border-slate-300 pt-1">
              {{ receiptConfig.header_note }}
            </div>
          </div>

          <!-- TRANSACTION METADATA -->
          <div class="py-2 text-[10px] space-y-0.5 border-b border-dashed border-slate-400">
            <div class="flex justify-between">
              <span>No: {{ receiptData?.invoice_number || receiptData?.sale?.invoice_number }}</span>
            </div>
            <div class="flex justify-between">
              <span>Tgl: {{ receiptData?.sale?.created_at ? new Date(receiptData.sale.created_at).toLocaleString('id-ID') : new Date().toLocaleString('id-ID') }}</span>
            </div>
            <div v-if="receiptConfig.show_cashier_name" class="flex justify-between">
              <span>Kasir: {{ authStore.user?.full_name || 'Kasir' }}</span>
            </div>
            <div v-if="receiptData?.customer_name" class="flex justify-between">
              <span>Plg: {{ receiptData.customer_name }}</span>
            </div>
          </div>

          <!-- ITEMS LIST -->
          <div class="py-2 border-b border-dashed border-slate-400 space-y-1.5">
            <div v-for="(item, idx) in (receiptData?.items || [])" :key="idx" class="text-[10px]">
              <div class="font-semibold text-slate-900 truncate">{{ item.product_name }}</div>
              <div class="flex justify-between text-slate-700">
                <span>{{ item.quantity }} {{ item.unit_code }} x {{ formatCurrency(itemUnitCatalogPrice(item)) }}</span>
                <span class="font-bold">{{ formatCurrency(itemGrossTotal(item)) }}</span>
              </div>
              <div v-if="receiptConfig.show_flash_sale_discount !== false && Number(item.flash_sale_discount || 0) > 0" class="flex justify-between text-[9px] text-amber-700 font-medium">
                <span>  (⚡ Flash Sale)</span>
                <span>-{{ formatCurrency(item.flash_sale_discount || 0) }}</span>
              </div>
              <div v-if="receiptConfig.show_member_discount !== false && Number(item.member_discount || 0) > 0" class="flex justify-between text-[9px] text-purple-700 font-medium">
                <span>  (★ Diskon Member)</span>
                <span>-{{ formatCurrency(item.member_discount || 0) }}</span>
              </div>
              <div v-if="getItemOtherDiscount(item) > 0" class="flex justify-between text-[9px] text-emerald-700">
                <span>  {{ Number(item.subtotal || 0) === 0 ? '(🎁 Hadiah Promo Gratis)' : '(Diskon Promo/Kasir)' }}</span>
                <span>-{{ formatCurrency(getItemOtherDiscount(item)) }}</span>
              </div>
            </div>
          </div>

          <!-- TOTALS & PAYMENT -->
          <div class="py-2 border-b border-dashed border-slate-400 text-[10px] space-y-0.5">
            <div class="flex justify-between">
              <span>Subtotal:</span>
              <span>{{ formatCurrency(receiptData?.sale?.subtotal || 0) }}</span>
            </div>
            <div v-if="receiptConfig.show_flash_sale_discount !== false && Number(receiptData?.sale?.total_flash_sale_discount || 0) > 0" class="flex justify-between text-amber-700 font-medium">
              <span>Potongan Flash Sale:</span>
              <span>-{{ formatCurrency(receiptData?.sale?.total_flash_sale_discount || 0) }}</span>
            </div>
            <div v-if="receiptConfig.show_member_discount !== false && Number(receiptData?.sale?.total_member_discount || 0) > 0" class="flex justify-between text-purple-700 font-medium">
              <span>Potongan Member:</span>
              <span>-{{ formatCurrency(receiptData?.sale?.total_member_discount || 0) }}</span>
            </div>
            <div v-if="getSaleOtherDiscount() > 0" class="flex justify-between text-emerald-700">
              <span>Diskon Kupon/Kasir:</span>
              <span>-{{ formatCurrency(getSaleOtherDiscount()) }}</span>
            </div>
            <div v-if="receiptConfig.show_tax_summary !== false && (receiptData?.sale?.tax_amount || 0) > 0" class="flex justify-between text-slate-600">
              <span>Pajak (PPN):</span>
              <span>{{ formatCurrency(receiptData?.sale?.tax_amount || 0) }}</span>
            </div>
            <div class="flex justify-between font-bold text-xs pt-1 border-t border-dotted border-slate-300">
              <span>TOTAL:</span>
              <span>{{ formatCurrency(receiptData?.sale?.total_amount || 0) }}</span>
            </div>

            <!-- Anda Telah Hemat Signature -->
            <div
              v-if="receiptConfig.show_savings_summary !== false && (Number(receiptData?.total_savings_amount || receiptData?.sale?.total_savings_amount || 0) > 0)"
              class="flex justify-between text-[9.5px] font-bold text-emerald-700 py-0.5 border-y border-dotted border-emerald-300 my-1 font-mono"
            >
              <span>* ANDA TELAH HEMAT</span>
              <span>{{ formatCurrency(receiptData?.total_savings_amount || receiptData?.sale?.total_savings_amount || 0) }} *</span>
            </div>

            <!-- Payment Breakdowns -->
            <div v-if="receiptData?.payments && receiptData.payments.length > 0" class="pt-1 space-y-0.5">
              <div v-for="(pay, pIdx) in receiptData.payments" :key="pIdx" class="flex justify-between">
                <span>BAYAR ({{ pay.payment_method_name }}):</span>
                <span>{{ formatCurrency(pay.amount) }}</span>
              </div>
            </div>
            <div v-else class="flex justify-between pt-1">
              <span>BAYAR:</span>
              <span>{{ formatCurrency(receiptData?.sale?.paid_amount || 0) }}</span>
            </div>

            <div class="flex justify-between font-semibold">
              <span>KEMBALIAN:</span>
              <span>{{ formatCurrency(receiptData?.change_amount ?? receiptData?.sale?.change_amount ?? 0) }}</span>
            </div>

            <div v-if="receiptData?.earned_points" class="text-[10px] text-emerald-700 font-semibold text-center pt-1">
              +{{ receiptData.earned_points }} Poin Member Diperoleh
            </div>
          </div>

          <!-- FOOTER & NOTES -->
          <div class="text-center pt-2.5 pb-1 space-y-1 text-[10px]">
            <div v-if="receiptConfig.footer_note_1" class="text-slate-700 leading-snug">
              {{ receiptConfig.footer_note_1 }}
            </div>
            <div v-if="receiptConfig.footer_note_2" class="text-slate-700 leading-snug font-medium">
              {{ receiptConfig.footer_note_2 }}
            </div>
            <div v-if="receiptConfig.social_media" class="text-[9px] text-slate-600 italic">
              {{ receiptConfig.social_media }}
            </div>

            <!-- Barcode Simulation -->
            <div v-if="receiptConfig.show_barcode" class="pt-2 flex flex-col items-center">
              <div class="tracking-widest font-black text-sm text-slate-800 scale-y-125 select-none font-mono">
                ||| | | |||| | ||| || |||
              </div>
              <div class="text-[8px] tracking-wider text-slate-500 font-mono mt-0.5">
                {{ receiptData?.invoice_number || receiptData?.sale?.invoice_number }}
              </div>
            </div>

            <div class="text-[8px] text-slate-400 font-mono pt-1">
              *** NOTANE POS CLOUD & LAN ***
            </div>
          </div>

          <!-- Bottom Jagged Paper Cut Edge -->
          <div class="absolute -bottom-1.5 left-0 right-0 h-2 bg-repeat-x" style="background: radial-gradient(circle, transparent, transparent 50%, #ffffff 50%, #ffffff 100%); background-size: 8px 8px; transform: rotate(180deg);"></div>
        </div>
      </div>

      <!-- Silent Print Notification Badge -->
      <div v-if="printStatus" class="w-full bg-emerald-50 border border-emerald-300 text-emerald-900 text-xs px-3 py-2 rounded-xl text-center font-medium flex items-center justify-center gap-1.5 shadow-xs">
        <Icon icon="streamline-ultimate-color:check-badge" class="w-4 h-4 shrink-0" />
        <span class="truncate">{{ printStatus }}</span>
      </div>

      <!-- Action Buttons -->
      <div class="flex items-center gap-2 w-full">
        <button
          @click="printReceipt"
          :disabled="isPrinting"
          class="flex-1 bg-emerald-600 hover:bg-emerald-700 text-white py-2.5 rounded-xl text-xs font-bold flex items-center justify-center gap-1.5 transition active:scale-95 shadow-sm border border-emerald-700 cursor-pointer disabled:opacity-50"
        >
          <Icon icon="streamline-ultimate-color:print-text" class="w-4 h-4" />
          <span>{{ isPrinting ? 'Mencetak...' : (printerStore.isConnected ? 'Cetak Ulang Struk' : 'Cetak Struk') }}</span>
        </button>

        <button
          @click="showReceipt = false; emit('close')"
          class="bg-slate-100 hover:bg-slate-200 border border-slate-300 text-slate-800 px-4 py-2.5 rounded-xl text-xs font-bold transition active:scale-95 cursor-pointer"
        >
          Selesai (Esc)
        </button>
      </div>
    </div>
  </div>
</template>
