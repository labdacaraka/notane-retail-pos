<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { X, Check, QrCode, CreditCard, Banknote, Award } from 'lucide-vue-next'
import { useCartStore } from '@/stores/cartStore'
import { useShiftStore } from '@/stores/shiftStore'
import { useAuthStore } from '@/stores/authStore'
import { usePrinterStore } from '@/stores/printerStore'
import { api } from '@/services/api'
import type { PaymentEntry, PaymentMethod, CheckoutResult } from '@/types'

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
const paymentsList = ref<PaymentEntry[]>([])
const isProcessing = ref<boolean>(false)
const errorMsg = ref<string>('')
const receiptData = ref<CheckoutResult | null>(null)
const showReceipt = ref<boolean>(false)
const printStatus = ref<string>('')
const isPrinting = ref<boolean>(false)

const totalPaid = computed(() => {
  const enteredCash = activeMethod.value === 'CASH' ? cashAmount.value : 0
  const otherPaid = paymentsList.value.reduce((sum, p) => sum + p.amount, 0)
  return enteredCash + otherPaid
})

const changeAmount = computed(() => {
  return Math.max(0, totalPaid.value - cartStore.grandTotal)
})

function formatCurrency(val: number): string {
  return new Intl.NumberFormat('id-ID', {
    style: 'currency',
    currency: 'IDR',
    minimumFractionDigits: 0,
    maximumFractionDigits: 0,
  }).format(val)
}

function setExactCash() {
  cashAmount.value = cartStore.grandTotal
}

function addCash(denomination: number) {
  cashAmount.value = (cashAmount.value || 0) + denomination
}

function addPaymentEntry(method: PaymentMethod, amount: number) {
  if (amount <= 0) return
  paymentsList.value.push({
    payment_method_id: method.id,
    method_name: method.name,
    category: method.category,
    amount,
  })
}

async function handleCompletePayment() {
  if (!shiftStore.currentShift) {
    errorMsg.value = 'Shift kasir tidak aktif. Silakan buka shift terlebih dahulu.'
    return
  }

  // Combine cash input into payment list
  const finalPayments = [...paymentsList.value]
  if (cashAmount.value > 0) {
    const cashMethod = paymentMethods.value.find((m) => m.category === 'CASH')!
    finalPayments.push({
      payment_method_id: cashMethod.id,
      method_name: cashMethod.name,
      category: 'CASH',
      amount: cashAmount.value,
    })
  }

  const total = finalPayments.reduce((sum, p) => sum + p.amount, 0)
  if (total < cartStore.grandTotal) {
    errorMsg.value = 'Total pembayaran masih kurang dari total belanja'
    return
  }

  isProcessing.value = true
  errorMsg.value = ''

  try {
    const payload = {
      shift_id: shiftStore.currentShift.id,
      warehouse_id: cartStore.defaultWarehouseId,
      customer_id: cartStore.selectedCustomer?.id,
      voucher_code: cartStore.voucherCode || undefined,
      items: cartStore.items.map((it) => ({
        product_id: it.product.id,
        unit_id: it.selected_unit.unit_id,
        quantity: it.quantity,
        unit_selling_price: it.unit_price,
        discount_percent: it.discount_percent,
        discount_amount: it.discount_amount,
      })),
      payments: finalPayments.map((p) => ({
        payment_method_id: p.payment_method_id,
        amount: p.amount,
      })),
      notes: `Kasir: ${authStore.user?.full_name}`,
    }

    const res = await api.post('/sales/checkout', payload)
    if (res.data.success) {
      receiptData.value = res.data.data
      showReceipt.value = true
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
      errorMsg.value = res.data.message || 'Gagal memproses transaksi'
    }
  } catch (err: any) {
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

onMounted(() => {
  window.addEventListener('keydown', handleKey)
  setExactCash()
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
      <div class="p-6 space-y-5 flex-1">
        <!-- Total Belanja Banner -->
        <div class="bg-slate-50 border border-slate-300 rounded-xl p-4 flex items-center justify-between">
          <div>
            <span class="text-xs font-bold text-slate-500 uppercase tracking-wider">Total Tagihan</span>
            <div class="text-3xl font-extrabold text-emerald-700 font-mono mt-0.5">
              {{ formatCurrency(cartStore.grandTotal) }}
            </div>
          </div>
          <div class="text-right">
            <span class="text-xs font-bold text-slate-500 uppercase tracking-wider">Kembalian</span>
            <div class="text-2xl font-extrabold font-mono mt-0.5" :class="changeAmount > 0 ? 'text-amber-800' : 'text-slate-400'">
              {{ formatCurrency(changeAmount) }}
            </div>
          </div>
        </div>

        <!-- Payment Method Tabs -->
        <div class="grid grid-cols-4 gap-2">
          <button
            @click="activeMethod = 'CASH'"
            class="flex flex-col items-center justify-center p-3 rounded-xl border text-xs font-bold transition active:scale-95 shadow-xs"
            :class="activeMethod === 'CASH' ? 'bg-emerald-50 border-emerald-600 text-emerald-800' : 'bg-slate-50 border-slate-300 text-slate-700 hover:bg-slate-100'"
          >
            <Banknote class="w-5 h-5 mb-1" />
            <span>Tunai</span>
          </button>

          <button
            @click="activeMethod = 'QRIS'"
            class="flex flex-col items-center justify-center p-3 rounded-xl border text-xs font-bold transition active:scale-95 shadow-xs"
            :class="activeMethod === 'QRIS' ? 'bg-sky-50 border-sky-600 text-sky-800' : 'bg-slate-50 border-slate-300 text-slate-700 hover:bg-slate-100'"
          >
            <QrCode class="w-5 h-5 mb-1" />
            <span>QRIS</span>
          </button>

          <button
            @click="activeMethod = 'EDC'"
            class="flex flex-col items-center justify-center p-3 rounded-xl border text-xs font-bold transition active:scale-95 shadow-xs"
            :class="activeMethod === 'EDC' ? 'bg-indigo-50 border-indigo-600 text-indigo-800' : 'bg-slate-50 border-slate-300 text-slate-700 hover:bg-slate-100'"
          >
            <CreditCard class="w-5 h-5 mb-1" />
            <span>Debit / EDC</span>
          </button>

          <button
            @click="activeMethod = 'POINTS'"
            class="flex flex-col items-center justify-center p-3 rounded-xl border text-xs font-bold transition active:scale-95 shadow-xs"
            :class="activeMethod === 'POINTS' ? 'bg-amber-50 border-amber-600 text-amber-800' : 'bg-slate-50 border-slate-300 text-slate-700 hover:bg-slate-100'"
          >
            <Award class="w-5 h-5 mb-1" />
            <span>Poin Member</span>
          </button>
        </div>

        <!-- Cash Details & Quick Denomination Buttons -->
        <div v-if="activeMethod === 'CASH'" class="space-y-3">
          <label class="block text-xs font-bold text-slate-700">Uang Tunai Diterima (Rp)</label>
          <input
            v-model.number="cashAmount"
            type="number"
            class="w-full bg-slate-50 border-2 border-slate-300 rounded-xl px-4 py-3 text-2xl font-mono text-slate-900 text-right focus:bg-white focus:outline-none focus:border-emerald-600 focus:ring-2 focus:ring-emerald-500/20 font-bold"
          />

          <!-- Quick Denominations -->
          <div class="grid grid-cols-5 gap-2 pt-1">
            <button
              @click="setExactCash"
              class="bg-slate-100 hover:bg-slate-200 text-slate-800 font-mono text-xs font-bold py-2.5 rounded-lg border border-slate-300 transition active:scale-95 shadow-xs"
            >
              Uang Pas
            </button>
            <button
              @click="addCash(10000)"
              class="bg-slate-100 hover:bg-slate-200 text-slate-800 font-mono text-xs font-bold py-2.5 rounded-lg border border-slate-300 transition active:scale-95 shadow-xs"
            >
              +10.000
            </button>
            <button
              @click="addCash(20000)"
              class="bg-slate-100 hover:bg-slate-200 text-slate-800 font-mono text-xs font-bold py-2.5 rounded-lg border border-slate-300 transition active:scale-95 shadow-xs"
            >
              +20.000
            </button>
            <button
              @click="addCash(50000)"
              class="bg-slate-100 hover:bg-slate-200 text-slate-800 font-mono text-xs font-bold py-2.5 rounded-lg border border-slate-300 transition active:scale-95 shadow-xs"
            >
              +50.000
            </button>
            <button
              @click="addCash(100000)"
              class="bg-slate-100 hover:bg-slate-200 text-slate-800 font-mono text-xs font-bold py-2.5 rounded-lg border border-slate-300 transition active:scale-95 shadow-xs"
            >
              +100.000
            </button>
          </div>
        </div>

        <!-- QRIS Simulation -->
        <div v-else-if="activeMethod === 'QRIS'" class="bg-slate-50 p-4 rounded-xl border border-slate-300 flex flex-col items-center justify-center text-center space-y-2">
          <div class="bg-white p-3 rounded-lg shadow-sm border border-slate-200">
            <QrCode class="w-32 h-32 text-slate-900" />
          </div>
          <p class="text-xs font-bold text-slate-900">QRIS Dinamis Otomatis</p>
          <p class="text-[11px] text-slate-500 font-medium">Total Rp {{ cartStore.grandTotal.toLocaleString() }} • Tampilkan ke konsumen</p>
          <button
            @click="addPaymentEntry(paymentMethods[1], cartStore.grandTotal); cashAmount = 0"
            class="bg-sky-600 hover:bg-sky-700 text-white text-xs font-bold px-4 py-2 rounded-lg transition active:scale-95 mt-2 shadow-sm border border-sky-700"
          >
            Konfirmasi Pembayaran QRIS Masuk
          </button>
        </div>

        <!-- EDC Card Simulation -->
        <div v-else-if="activeMethod === 'EDC'" class="bg-slate-50 p-4 rounded-xl border border-slate-300 space-y-3">
          <p class="text-xs font-bold text-slate-900">Mesin EDC Bank</p>
          <input
            placeholder="Nomor Approval Code / Trace EDC"
            class="w-full bg-white border border-slate-300 rounded-lg px-3 py-2 text-xs text-slate-900 focus:outline-none focus:border-indigo-600"
          />
          <button
            @click="addPaymentEntry(paymentMethods[2], cartStore.grandTotal); cashAmount = 0"
            class="w-full bg-indigo-600 hover:bg-indigo-700 text-white text-xs font-bold py-2.5 rounded-lg transition active:scale-95 shadow-sm border border-indigo-700"
          >
            Simpan Pembayaran Kartu EDC
          </button>
        </div>

        <!-- Member Points -->
        <div v-else-if="activeMethod === 'POINTS'" class="bg-slate-50 p-4 rounded-xl border border-slate-300 space-y-2">
          <div class="flex items-center justify-between text-xs">
            <span class="text-slate-600">Member:</span>
            <span class="font-bold text-slate-900">{{ cartStore.selectedCustomer?.name || 'Bukan Member' }}</span>
          </div>
          <div class="flex items-center justify-between text-xs">
            <span class="text-slate-600">Saldo Poin:</span>
            <span class="font-bold text-amber-700 font-mono">{{ cartStore.selectedCustomer?.reward_points || 0 }} Poin</span>
          </div>
          <p class="text-[11px] text-slate-500">1 Poin = Rp 100 potongan belanja</p>
        </div>

        <!-- Error Warning -->
        <div v-if="errorMsg" class="bg-rose-50 border border-rose-300 p-3 rounded-xl text-rose-700 text-xs font-semibold">
          ⚠️ {{ errorMsg }}
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
      <!-- Receipt Paper (Printable) -->
      <div id="thermal-receipt" class="bg-white text-black font-mono text-xs p-4 rounded border border-slate-200 shadow-sm w-full">
        <div class="text-center pb-2 border-b border-dashed border-gray-400">
          <h4 class="font-bold text-sm">NOTANE RETAIL STORE</h4>
          <p class="text-[10px]">Jl. Sudirman No. 123, Jakarta</p>
          <p class="text-[10px]">Telp: 021-5551234</p>
        </div>

        <div class="py-2 border-b border-dashed border-gray-400 text-[10px] space-y-0.5">
          <div class="flex justify-between">
            <span>Nota: {{ receiptData?.invoice_number }}</span>
          </div>
          <div class="flex justify-between">
            <span>Kasir: {{ authStore.user?.full_name }}</span>
            <span>{{ new Date().toLocaleTimeString('id-ID') }}</span>
          </div>
        </div>

        <div class="py-2 border-b border-dashed border-gray-400 space-y-1">
          <div class="text-[11px] font-bold text-center">TRANSAKSI SUKSES</div>
          <div class="flex justify-between text-xs font-bold pt-1">
            <span>TOTAL:</span>
            <span>Rp {{ receiptData?.sale.total_amount.toLocaleString() }}</span>
          </div>
          <div class="flex justify-between text-[11px]">
            <span>BAYAR:</span>
            <span>Rp {{ receiptData?.sale.paid_amount.toLocaleString() }}</span>
          </div>
          <div class="flex justify-between text-[11px]">
            <span>KEMBALIAN:</span>
            <span>Rp {{ receiptData?.change_amount.toLocaleString() }}</span>
          </div>
          <div v-if="receiptData?.earned_points" class="text-[10px] text-gray-600 text-center pt-1">
            +{{ receiptData.earned_points }} Poin Member Diperoleh
          </div>
        </div>

        <div class="text-center pt-2 text-[10px] text-gray-600">
          <p>Terima Kasih Atas Kunjungan Anda</p>
          <p>Barang yang sudah dibeli tidak dapat ditukar</p>
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
