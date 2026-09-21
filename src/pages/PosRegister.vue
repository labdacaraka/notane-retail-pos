<script setup lang="ts">
import { ref, onMounted } from 'vue'
import PosLayout from '@/layouts/PosLayout.vue'
import BarcodeBar from '@/components/pos/BarcodeBar.vue'
import CartTable from '@/components/pos/CartTable.vue'
import QuickPayModal from '@/components/pos/QuickPayModal.vue'
import SupervisorPinModal from '@/components/pos/SupervisorPinModal.vue'
import HoldRecallModal from '@/components/pos/HoldRecallModal.vue'
import ProductSearchModal from '@/components/pos/ProductSearchModal.vue'
import { useCartStore } from '@/stores/cartStore'
import { api } from '@/services/api'
import { Tag, User, ShoppingCart } from 'lucide-vue-next'
import type { CartItem, CheckoutResult } from '@/types'

const cartStore = useCartStore()

onMounted(() => {
  cartStore.initOutlet()
})

const barcodeBarRef = ref<any>(null)
const showQuickPay = ref<boolean>(false)
const showSupervisorPin = ref<boolean>(false)
const supervisorAction = ref<string>('')
const supervisorDesc = ref<string>('')

const showHoldRecall = ref<boolean>(false)
const showProductSearch = ref<boolean>(false)
const showCustomerModal = ref<boolean>(false)
const customerPhoneInput = ref<string>('')
const showVoucherModal = ref<boolean>(false)
const voucherInput = ref<string>('')
const voucherError = ref<string>('')

function formatCurrency(val: number): string {
  return new Intl.NumberFormat('id-ID', {
    style: 'currency',
    currency: 'IDR',
    minimumFractionDigits: 0,
    maximumFractionDigits: 0,
  }).format(val)
}

function handleItemDiscount(item: CartItem, index: number) {
  const input = prompt(`Masukkan diskon nominal untuk "${item.product.name}" (Rp):`, String(item.discount_amount || '0'))
  if (input !== null) {
    const val = parseFloat(input) || 0
    cartStore.updateItemDiscount(index, val)
  }
}

function handleOpenDrawer() {
  supervisorAction.value = 'OPEN_DRAWER'
  supervisorDesc.value = 'Otorisasi pembukaan Cash Drawer manual tanpa transaksi'
  showSupervisorPin.value = true
}

function handleSupervisorApproved(supervisor: { id: string; name: string }) {
  showSupervisorPin.value = false

  if (supervisorAction.value === 'OPEN_DRAWER') {
    alert(`Cash Drawer Berhasil Dibuka! Diotorisasi oleh: ${supervisor.name}`)
  }

  barcodeBarRef.value?.focusInput()
}

function handleHoldOrder() {
  if (cartStore.items.length === 0) return
  const held = cartStore.holdCurrentOrder()
  if (held) {
    alert(`Transaksi antrean berhasil ditahan (${held.id})`)
  }
}

function handleRecallOrder() {
  showHoldRecall.value = true
}

function handleCheckout() {
  if (cartStore.items.length === 0) return
  showQuickPay.value = true
}

function handlePaymentSuccess(result: CheckoutResult) {
  console.log('Transaksi berhasil:', result)
}

async function handleApplyCustomer() {
  const query = customerPhoneInput.value.trim()
  if (query) {
    try {
      const res = await api.get('/customers', { params: { search: query } })
      if (res.data.success && res.data.data && res.data.data.length > 0) {
        const cust = res.data.data[0]
        cartStore.applyCustomerToCart(cust)
      } else {
        alert('Pelanggan / Member tidak ditemukan dengan nomor/kode tersebut')
      }
    } catch (e: any) {
      alert('Gagal mencari data pelanggan: ' + (e.response?.data?.message || e.message))
    }
  }
  showCustomerModal.value = false
  customerPhoneInput.value = ''
  barcodeBarRef.value?.focusInput()
}

async function handleApplyVoucher() {
  voucherError.value = ''
  try {
    await cartStore.applyVoucher(voucherInput.value)
    showVoucherModal.value = false
    voucherInput.value = ''
    barcodeBarRef.value?.focusInput()
  } catch (err: any) {
    voucherError.value = err.message || 'Gagal menerapkan voucher'
  }
}
</script>

<template>
  <PosLayout>
    <!-- Barcode & Shortcut Ribbon -->
    <BarcodeBar
      ref="barcodeBarRef"
      @search="showProductSearch = true"
      @customer="showCustomerModal = true"
      @voucher="showVoucherModal = true"
      @open-drawer="handleOpenDrawer"
      @hold="handleHoldOrder"
      @recall="handleRecallOrder"
      @checkout="handleCheckout"
    />

    <!-- Main Register Grid -->
    <div class="flex-1 flex flex-col lg:flex-row overflow-hidden">
      <!-- Left: Cart Table View -->
      <div class="flex-1 flex flex-col overflow-hidden">
        <!-- Customer Banner if selected -->
        <div
          v-if="cartStore.selectedCustomer"
          class="bg-sky-50 border-b border-sky-200 px-4 py-2 flex items-center justify-between text-xs"
        >
          <div class="flex items-center gap-2">
            <User class="w-4 h-4 text-sky-700" />
            <span class="font-bold text-sky-900">{{ cartStore.selectedCustomer.name }}</span>
            <span class="text-sky-700 font-mono">({{ cartStore.selectedCustomer.group_name }})</span>
            <span class="text-slate-600">• Poin: <strong class="text-amber-700">{{ cartStore.selectedCustomer.reward_points }}</strong></span>
          </div>
          <button @click="cartStore.applyCustomerToCart(null)" class="text-slate-500 hover:text-rose-700 font-bold">
            Lepas (X)
          </button>
        </div>

        <!-- Cart Table -->
        <CartTable
          @discount-item="handleItemDiscount"
        />
      </div>

      <!-- Right: Grand Total & Live Summary Panel -->
      <div class="w-full lg:w-96 bg-white border-t lg:border-t-0 lg:border-l border-slate-300 flex flex-col justify-between p-5 space-y-4 shadow-sm">
        <!-- Grand Total Giant Display -->
        <div class="bg-slate-50 border border-slate-300 rounded-xl p-5 shadow-xs">
          <div class="flex items-center justify-between text-xs font-bold text-slate-600 uppercase tracking-wider">
            <span>Total Tagihan</span>
            <span class="font-mono text-slate-500">{{ cartStore.totalItemsCount }} Qty</span>
          </div>
          <div class="text-4xl lg:text-5xl font-extrabold text-emerald-700 font-mono tracking-tight mt-1 text-right truncate">
            {{ formatCurrency(cartStore.grandTotal) }}
          </div>
        </div>

        <!-- Breakdown List -->
        <div class="space-y-2.5 text-xs">
          <div class="flex justify-between text-slate-600">
            <span>Subtotal Bruto:</span>
            <span class="font-mono font-bold text-slate-900">{{ formatCurrency(cartStore.subtotal) }}</span>
          </div>

          <div v-if="cartStore.totalDiscount > 0" class="flex justify-between text-amber-800 font-medium">
            <span>Potongan Diskon & Promo:</span>
            <span class="font-mono font-bold">-{{ formatCurrency(cartStore.totalDiscount) }}</span>
          </div>

          <div class="flex justify-between text-slate-600">
            <span>Pajak (PPN):</span>
            <span class="font-mono font-bold text-sky-800">{{ formatCurrency(cartStore.totalTax) }}</span>
          </div>

          <div class="pt-2.5 border-t border-slate-200 flex justify-between text-sm font-bold text-slate-900">
            <span>Total Bersih:</span>
            <span class="font-mono text-emerald-700 text-base font-extrabold">{{ formatCurrency(cartStore.grandTotal) }}</span>
          </div>
        </div>

        <!-- Actions -->
        <div class="space-y-2 pt-2">
          <button
            @click="handleCheckout"
            :disabled="cartStore.items.length === 0"
            class="w-full bg-emerald-600 hover:bg-emerald-700 disabled:opacity-40 disabled:pointer-events-none text-white py-4 rounded-xl text-base font-extrabold flex items-center justify-center gap-2 shadow-sm border border-emerald-700 transition active:scale-95"
          >
            <ShoppingCart class="w-5 h-5" />
            <span>BAYAR (F12)</span>
          </button>

          <button
            @click="cartStore.clearCart()"
            :disabled="cartStore.items.length === 0"
            class="w-full bg-slate-100 hover:bg-slate-200 disabled:opacity-40 text-slate-700 hover:text-slate-900 border border-slate-300 py-2 rounded-xl text-xs font-bold transition active:scale-95"
          >
            Kosongkan Keranjang
          </button>
        </div>
      </div>
    </div>

    <!-- Modals -->
    <QuickPayModal
      :show="showQuickPay"
      @close="showQuickPay = false; barcodeBarRef?.focusInput()"
      @success="handlePaymentSuccess"
    />

    <SupervisorPinModal
      :show="showSupervisorPin"
      :action-type="supervisorAction"
      :description="supervisorDesc"
      @close="showSupervisorPin = false; barcodeBarRef?.focusInput()"
      @approved="handleSupervisorApproved"
    />

    <HoldRecallModal
      :show="showHoldRecall"
      @close="showHoldRecall = false; barcodeBarRef?.focusInput()"
    />

    <ProductSearchModal
      :show="showProductSearch"
      @close="showProductSearch = false; barcodeBarRef?.focusInput()"
    />

    <!-- Customer Member Select Modal -->
    <div
      v-if="showCustomerModal"
      class="fixed inset-0 z-50 bg-slate-900/50 backdrop-blur-xs flex items-center justify-center p-4 select-none"
    >
      <div class="bg-white border border-slate-300 w-full max-w-sm rounded-xl p-5 space-y-4 shadow-2xl text-slate-900">
        <h3 class="font-bold text-sm text-slate-900 border-b border-slate-200 pb-2">Input Member / Pelanggan</h3>
        <input
          v-model="customerPhoneInput"
          placeholder="No. HP / Kode Kartu Member"
          class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-sm text-slate-900 focus:bg-white focus:border-emerald-600 focus:outline-none"
          @keyup.enter="handleApplyCustomer"
        />
        <div class="flex justify-end gap-2">
          <button @click="showCustomerModal = false" class="bg-slate-100 hover:bg-slate-200 border border-slate-300 text-xs text-slate-700 font-bold px-3 py-1.5 rounded-lg">Batal</button>
          <button @click="handleApplyCustomer" class="bg-emerald-600 hover:bg-emerald-700 text-white text-xs font-bold px-4 py-1.5 rounded-lg shadow-sm border border-emerald-700">Terapkan</button>
        </div>
      </div>
    </div>

    <!-- Voucher Promo Modal -->
    <div
      v-if="showVoucherModal"
      class="fixed inset-0 z-50 bg-slate-900/50 backdrop-blur-xs flex items-center justify-center p-4 select-none"
    >
      <div class="bg-white border border-slate-300 w-full max-w-sm rounded-xl p-5 space-y-4 shadow-2xl text-slate-900">
        <h3 class="font-bold text-sm text-slate-900 flex items-center gap-2 border-b border-slate-200 pb-2">
          <Tag class="w-4 h-4 text-amber-600" />
          Kupon Diskon & Promo
        </h3>
        <input
          v-model="voucherInput"
          placeholder="Ketik Kode Voucher (contoh: HEMAT10K)"
          class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-sm text-slate-900 font-mono uppercase focus:bg-white focus:border-emerald-600 focus:outline-none"
          @keyup.enter="handleApplyVoucher"
        />
        <p v-if="voucherError" class="text-xs text-rose-600 font-semibold">⚠️ {{ voucherError }}</p>
        <div class="flex justify-end gap-2">
          <button @click="showVoucherModal = false" class="bg-slate-100 hover:bg-slate-200 border border-slate-300 text-xs text-slate-700 font-bold px-3 py-1.5 rounded-lg">Batal</button>
          <button @click="handleApplyVoucher" class="bg-amber-600 hover:bg-amber-700 text-white text-xs font-bold px-4 py-1.5 rounded-lg shadow-sm border border-amber-700">Gunakan</button>
        </div>
      </div>
    </div>
  </PosLayout>
</template>
