<script setup lang="ts">
import { ref, onMounted } from 'vue'
import PosLayout from '@/layouts/PosLayout.vue'
import BarcodeBar from '@/components/pos/BarcodeBar.vue'
import CartTable from '@/components/pos/CartTable.vue'
import QuickPayModal from '@/components/pos/QuickPayModal.vue'
import SupervisorPinModal from '@/components/pos/SupervisorPinModal.vue'
import HoldRecallModal from '@/components/pos/HoldRecallModal.vue'
import ProductSearchModal from '@/components/pos/ProductSearchModal.vue'
import ItemDiscountModal from '@/components/pos/ItemDiscountModal.vue'
import { useCartStore } from '@/stores/cartStore'
import { useShiftStore } from '@/stores/shiftStore'
import { useAuthStore } from '@/stores/authStore'
import { useLocationStore } from '@/stores/locationStore'
import { useDialog } from '@/composables/useDialog'
import { api } from '@/services/api'
import { Tag, User, ShoppingCart } from 'lucide-vue-next'
import type { CartItem, CheckoutResult } from '@/types'

const cartStore = useCartStore()
const shiftStore = useShiftStore()
const authStore = useAuthStore()
const locationStore = useLocationStore()
const dialog = useDialog()

onMounted(async () => {
  await shiftStore.checkCurrentShift()
  if (shiftStore.isShiftOpen) {
    await cartStore.initOutlet()
  } else {
    cartStore.setOutlet('')
    cartStore.clearCart()
  }
})

const barcodeBarRef = ref<any>(null)
const showQuickPay = ref<boolean>(false)
const showSupervisorPin = ref<boolean>(false)
const supervisorAction = ref<string>('')
const supervisorDesc = ref<string>('')

const showDiscountModal = ref<boolean>(false)
const selectedDiscountItem = ref<CartItem | null>(null)
const selectedDiscountIndex = ref<number>(-1)

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
  selectedDiscountItem.value = item
  selectedDiscountIndex.value = index
  showDiscountModal.value = true
}

function handleSaveItemDiscount(payload: { index: number; discountAmount: number; discountPercent: number }) {
  cartStore.updateItemDiscount(payload.index, payload.discountAmount, payload.discountPercent)
  showDiscountModal.value = false
  barcodeBarRef.value?.focusInput()
}

function handleOpenDrawer() {
  if (authStore.hasPermission(['sales:open_drawer', 'pos:open_drawer'])) {
    dialog.toast('Cash Drawer Berhasil Dibuka!', 'success')
  } else {
    supervisorAction.value = 'OPEN_DRAWER'
    supervisorDesc.value = 'Otorisasi pembukaan Cash Drawer manual tanpa transaksi'
    showSupervisorPin.value = true
  }
}

function handleClearCart() {
  if (cartStore.items.length === 0) return
  if (authStore.hasPermission(['sales:void', 'pos:void'])) {
    cartStore.clearCart()
    dialog.toast('Keranjang belanja berhasil dikosongkan', 'info')
  } else {
    supervisorAction.value = 'CLEAR_CART'
    supervisorDesc.value = 'Otorisasi supervisor untuk mengosongkan seluruh keranjang belanja kasir'
    showSupervisorPin.value = true
  }
}

function handleSupervisorApproved(supervisor: { id: string; name: string }) {
  showSupervisorPin.value = false

  if (supervisorAction.value === 'OPEN_DRAWER') {
    dialog.toast(`Cash Drawer Berhasil Dibuka! Diotorisasi oleh: ${supervisor.name}`, 'success')
  } else if (supervisorAction.value === 'CLEAR_CART') {
    cartStore.clearCart()
    dialog.toast(`Keranjang belanja berhasil dikosongkan! Diotorisasi oleh: ${supervisor.name}`, 'success')
  }

  barcodeBarRef.value?.focusInput()
}

function handleOpenProductSearch() {
  if (!shiftStore.isShiftOpen) {
    dialog.toast('Shift kasir belum dibuka. Silakan buka shift kasir terlebih dahulu.', 'warning')
    shiftStore.openShiftModal('OPEN')
    return
  }
  showProductSearch.value = true
}

function handleHoldOrder() {
  if (!shiftStore.isShiftOpen) {
    dialog.toast('Shift kasir belum dibuka. Silakan buka shift kasir terlebih dahulu.', 'warning')
    shiftStore.openShiftModal('OPEN')
    return
  }
  if (cartStore.items.length === 0) return
  const held = cartStore.holdCurrentOrder()
  if (held) {
    alert(`Transaksi antrean berhasil ditahan (${held.id})`)
  }
}

function handleRecallOrder() {
  if (!shiftStore.isShiftOpen) {
    dialog.toast('Shift kasir belum dibuka. Silakan buka shift kasir terlebih dahulu.', 'warning')
    shiftStore.openShiftModal('OPEN')
    return
  }
  showHoldRecall.value = true
}

function handleCheckout() {
  if (!shiftStore.isShiftOpen) {
    dialog.toast('Shift kasir belum dibuka. Silakan buka shift kasir terlebih dahulu.', 'warning')
    shiftStore.openShiftModal('OPEN')
    return
  }
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

async function handleClaimGift(gift: any) {
  try {
    await cartStore.claimFreeGift(gift)
    dialog.toast(`Hadiah "${gift.reward_product_name}" berhasil dimasukkan ke keranjang!`, 'success')
  } catch (e: any) {
    dialog.toast(e.message || 'Gagal menambahkan hadiah', 'danger')
  }
}
</script>

<template>
  <PosLayout>
    <!-- Barcode & Shortcut Ribbon -->
    <BarcodeBar
      ref="barcodeBarRef"
      @search="handleOpenProductSearch"
      @customer="showCustomerModal = true"
      @voucher="showVoucherModal = true"
      @open-drawer="handleOpenDrawer"
      @hold="handleHoldOrder"
      @recall="handleRecallOrder"
      @checkout="handleCheckout"
    />

    <!-- Warning Banner: Shift Kasir Belum Dibuka -->
    <div
      v-if="!shiftStore.isShiftOpen"
      class="bg-amber-50 border-b border-amber-300 px-5 py-3 flex flex-col sm:flex-row items-center justify-between gap-3 text-xs animate-fadeIn select-none shadow-2xs shrink-0"
    >
      <div class="flex items-center gap-3">
        <div class="w-8 h-8 rounded-xl bg-amber-100 border border-amber-300 text-amber-700 flex items-center justify-center shrink-0 font-bold text-sm shadow-2xs">
          ⚠️
        </div>
        <div>
          <div class="font-extrabold text-amber-950 text-sm tracking-tight flex items-center gap-2">
            <span>Shift Kasir Belum Dibuka</span>
            <span class="bg-amber-200 text-amber-900 text-[10px] font-mono px-2 py-0.5 rounded-full font-bold">Transaksi Terkunci</span>
          </div>
          <p class="text-amber-800 text-xs mt-0.5">
            Katalog barang, harga cabang, dan pemindaian barcode dinonaktifkan sampai kasir membuka shift dan memilih cabang toko.
          </p>
        </div>
      </div>
      <button
        @click="shiftStore.openShiftModal('OPEN')"
        class="bg-amber-600 hover:bg-amber-700 text-white font-bold px-4 py-2 rounded-xl text-xs shadow-sm transition active:scale-95 cursor-pointer flex items-center gap-1.5 shrink-0"
      >
        🔓 Buka Shift Kasir Sekarang
      </button>
    </div>

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

        <!-- Auto Gift Notification Banner -->
        <div
          v-if="cartStore.items.some(i => i.is_auto_gift)"
          class="bg-gradient-to-r from-emerald-50 to-teal-50 border-b border-emerald-300 px-4 py-2 flex items-center justify-between gap-2 text-xs"
        >
          <div class="flex items-center gap-2">
            <span class="text-base">🎉</span>
            <div>
              <span class="font-extrabold text-emerald-900 uppercase tracking-wide">Promo Gratis Otomatis:</span>
              <div class="text-slate-700 font-medium">
                <span
                  v-for="(giftItem, idx) in cartStore.items.filter(i => i.is_auto_gift)"
                  :key="giftItem.id"
                >
                  <span class="font-bold text-emerald-800">{{ giftItem.quantity }}x "{{ giftItem.product.name }}"</span>
                  <span v-if="idx < cartStore.items.filter(i => i.is_auto_gift).length - 1">, </span>
                </span>
                telah otomatis ditambahkan ke keranjang belanja (Rp 0)!
              </div>
            </div>
          </div>
          <span class="bg-emerald-600 text-white font-bold px-2.5 py-0.5 rounded-full text-[11px] font-mono shadow-2xs">
            ✓ Hadiah Terpasang
          </span>
        </div>

        <!-- Out of Stock Promo Gift Alert Banner -->
        <div
          v-if="cartStore.outOfStockGifts.length > 0"
          class="bg-amber-50 border-b border-amber-300 px-4 py-2 flex flex-wrap items-center justify-between gap-2 text-xs"
        >
          <div class="flex items-center gap-2">
            <span class="text-base">⚠️</span>
            <div>
              <span class="font-extrabold text-amber-900 uppercase tracking-wide">Hadiah Promo Habis di Toko:</span>
              <div class="text-amber-800 font-medium">
                Pelanggan berhak atas promo
                <span v-for="(oos, oIdx) in cartStore.outOfStockGifts" :key="oIdx">
                  "{{ oos.promo_name }}", namun stok fisik hadiah <strong>"{{ oos.reward_product_name }}"</strong> sedang kosong (Sisa: {{ oos.available_stock }}).
                  <span v-if="oIdx < cartStore.outOfStockGifts.length - 1">, </span>
                </span>
                <span class="text-slate-600 block text-[11px] mt-0.5">
                  💡 Transaksi tetap dapat dilanjutkan tanpa kendala. Mohon informasikan kepada pelanggan bahwa bonus hadiah sedang kosong di cabang ini.
                </span>
              </div>
            </div>
          </div>
          <span class="bg-amber-200 text-amber-900 font-bold px-2.5 py-1 rounded-lg text-[11px] font-mono border border-amber-300">
            Stok Hadiah Habis
          </span>
        </div>

        <!-- Dismissed Gift Banner (Opsi Pasang Kembali jika Kasir/Pelanggan Berubah Pikiran) -->
        <div
          v-if="cartStore.dismissedGifts.length > 0"
          class="bg-slate-100 border-b border-slate-300 px-4 py-2 flex flex-wrap items-center justify-between gap-2 text-xs"
        >
          <div class="flex items-center gap-2">
            <span class="text-base">ℹ️</span>
            <div>
              <span class="font-bold text-slate-800">Hadiah Promo Dilepas:</span>
              <span class="text-slate-600 ml-1">
                <span v-for="(dis, dIdx) in cartStore.dismissedGifts" :key="dis.rule_key">
                  "{{ dis.product_name }}" dilepas dari transaksi saat ini.
                  <span v-if="dIdx < cartStore.dismissedGifts.length - 1">, </span>
                </span>
              </span>
            </div>
          </div>
          <div class="flex items-center gap-2">
            <button
              v-for="dis in cartStore.dismissedGifts"
              :key="dis.rule_key"
              @click="cartStore.restoreGift(dis.rule_key)"
              class="bg-emerald-600 hover:bg-emerald-700 text-white font-bold px-3 py-1 rounded-lg text-xs shadow-xs transition active:scale-95 cursor-pointer flex items-center gap-1"
            >
              <span>+ Pasang Kembali "{{ dis.product_name }}"</span>
            </button>
          </div>
        </div>

        <!-- Eligible Promo Free Gift Banner (Fallback / PWP Tebus Murah) -->
        <div
          v-if="cartStore.eligibleGifts.length > 0"
          class="bg-gradient-to-r from-amber-50 to-emerald-50 border-b border-amber-300 px-4 py-2 flex flex-wrap items-center justify-between gap-2 text-xs"
        >
          <div class="flex items-center gap-2">
            <span class="text-base">🎁</span>
            <div>
              <span class="font-extrabold text-amber-900 uppercase tracking-wide">Promo Aktif Terpenuhi:</span>
              <div class="text-slate-700 font-medium">
                Pelanggan berhak mendapatkan
                <span
                  v-for="(gift, gIdx) in cartStore.eligibleGifts"
                  :key="gIdx"
                  class="font-bold text-emerald-800"
                >
                  {{ gift.quantity }}x "{{ gift.reward_product_name }}"
                  <span v-if="gIdx < cartStore.eligibleGifts.length - 1">, </span>
                </span>
                !
              </div>
            </div>
          </div>
          <div class="flex items-center gap-2">
            <button
              v-for="(gift, gIdx) in cartStore.eligibleGifts"
              :key="gIdx"
              @click="handleClaimGift(gift)"
              class="bg-emerald-600 hover:bg-emerald-700 text-white font-bold px-3 py-1 rounded-lg transition active:scale-95 shadow-xs flex items-center gap-1.5 cursor-pointer text-xs"
            >
              <span>+ Klaim Hadiah ({{ gift.quantity }}x)</span>
            </button>
          </div>
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

          <!-- Flash Sale Savings -->
          <div v-if="cartStore.totalFlashSaleSavings > 0" class="flex justify-between text-amber-700 font-medium">
            <span class="flex items-center gap-1">
              <span>⚡ Hemat Flash Sale:</span>
            </span>
            <span class="font-mono font-bold">-{{ formatCurrency(cartStore.totalFlashSaleSavings) }}</span>
          </div>

          <!-- Member Discount Savings -->
          <div v-if="cartStore.totalMemberSavings > 0" class="flex justify-between text-purple-700 font-medium">
            <span class="flex items-center gap-1">
              <span>★ Hemat Member:</span>
            </span>
            <span class="font-mono font-bold">-{{ formatCurrency(cartStore.totalMemberSavings) }}</span>
          </div>

          <!-- Voucher Savings -->
          <div v-if="cartStore.voucherDiscount > 0" class="flex justify-between text-indigo-700 font-medium">
            <span class="flex items-center gap-1">
              <span>🎟 Diskon Kupon/Voucher:</span>
            </span>
            <span class="font-mono font-bold">-{{ formatCurrency(cartStore.voucherDiscount) }}</span>
          </div>

          <!-- Promo Otomatis Toko (Buy X Get Y / Min Belanja) -->
          <div v-if="cartStore.totalPromoSavings > 0" class="flex justify-between text-rose-700 font-medium">
            <span class="flex items-center gap-1">
              <span>🎁 Promo Otomatis Toko:</span>
            </span>
            <span class="font-mono font-bold">-{{ formatCurrency(cartStore.totalPromoSavings) }}</span>
          </div>

          <!-- Manual / Item Discount -->
          <div v-if="cartStore.itemDiscounts > 0" class="flex justify-between text-slate-700 font-medium">
            <span>Diskon Item / Kasir:</span>
            <span class="font-mono font-bold">-{{ formatCurrency(cartStore.itemDiscounts) }}</span>
          </div>

          <!-- Highlight Total Savings Badge -->
          <div v-if="cartStore.totalSavings > 0" class="bg-emerald-50 border border-emerald-200 rounded-lg p-2 text-center text-xs font-bold text-emerald-800">
            <span>🎉 Total Penghematan: {{ formatCurrency(cartStore.totalSavings) }}</span>
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
            @click="handleClearCart"
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
      :outlet-id="locationStore.currentOutletId"
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

    <ItemDiscountModal
      :show="showDiscountModal"
      :item="selectedDiscountItem"
      :index="selectedDiscountIndex"
      @close="showDiscountModal = false; barcodeBarRef?.focusInput()"
      @save="handleSaveItemDiscount"
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
