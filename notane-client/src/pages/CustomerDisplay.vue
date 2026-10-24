<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import {
  subscribeCustomerDisplay,
  getCachedCustomerDisplayState,
  type CustomerDisplayState
} from '@/services/customerDisplayChannel'
import { Icon } from '@iconify/vue'
import { useStoreBrandingStore } from '@/stores/storeBrandingStore'

const displayState = ref<CustomerDisplayState>(getCachedCustomerDisplayState())
const storeBrandingStore = useStoreBrandingStore()
let unsubscribe: (() => void) | null = null

const currentTime = ref<string>('')
let timer: any = null

function updateTime() {
  const now = new Date()
  currentTime.value = now.toLocaleTimeString('id-ID', { hour: '2-digit', minute: '2-digit', second: '2-digit' })
}

function formatCurrency(val: number): string {
  return new Intl.NumberFormat('id-ID', {
    style: 'currency',
    currency: 'IDR',
    minimumFractionDigits: 0,
    maximumFractionDigits: 0,
  }).format(val || 0)
}

const isSuccess = computed(() => displayState.value.status === 'PAYMENT_SUCCESS' && !!displayState.value.paymentResult)
const hasItems = computed(() => !isSuccess.value && displayState.value.items.length > 0)

onMounted(() => {
  updateTime()
  timer = setInterval(updateTime, 1000)
  storeBrandingStore.fetchBranding()

  unsubscribe = subscribeCustomerDisplay((newState) => {
    displayState.value = newState
  })
})

onUnmounted(() => {
  if (unsubscribe) unsubscribe()
  if (timer) clearInterval(timer)
})
</script>

<template>
  <div class="h-screen w-screen bg-slate-900 text-slate-100 flex flex-col justify-between select-none overflow-hidden font-sans">
    <!-- 1. Top Header Bar (Eyecatching Store Identity & Co-branding) -->
    <header class="bg-slate-850/95 backdrop-blur border-b border-slate-700 px-6 py-3.5 flex items-center justify-between shrink-0 shadow-lg">
      <div class="flex items-center gap-3.5">
        <!-- Logo Toko Utama (Eyecatching) -->
        <div class="w-12 h-12 rounded-2xl bg-white/10 backdrop-blur border border-white/20 p-1.5 flex items-center justify-center shadow-lg shrink-0 overflow-hidden">
          <img
            v-if="storeBrandingStore.storeLogoUrl"
            :src="storeBrandingStore.storeLogoUrl"
            alt="Logo Toko"
            class="w-full h-full object-contain"
          />
          <div
            v-else
            class="w-full h-full rounded-xl bg-gradient-to-br from-emerald-500 to-teal-700 flex items-center justify-center text-white font-black text-xl"
          >
            {{ (storeBrandingStore.storeName || 'N').charAt(0).toUpperCase() }}
          </div>
        </div>

        <div>
          <div class="flex items-center gap-2">
            <h1 class="text-xl font-black tracking-tight text-white uppercase drop-shadow-xs">
              {{ storeBrandingStore.storeName }}
            </h1>
            <span class="bg-emerald-500/20 text-emerald-300 border border-emerald-500/40 text-[10px] px-2.5 py-0.5 rounded-full font-bold uppercase tracking-wider">
              Layar Pelanggan
            </span>
          </div>
          <p class="text-xs text-slate-400 font-medium mt-0.5">
            {{ storeBrandingStore.storeTagline || 'Transparansi Belanja Realtime' }}
          </p>
        </div>
      </div>

      <!-- Right Header: Member Badge, Time, and Co-branding -->
      <div class="flex items-center gap-3">
        <!-- Member Customer Badge -->
        <div v-if="displayState.selectedCustomer" class="flex items-center gap-2.5 bg-indigo-950/70 border border-indigo-500/40 px-3.5 py-1.5 rounded-xl shadow-inner">
          <Icon icon="fluent-color:person-starburst-24" class="w-6 h-6 shrink-0" />
          <div class="text-right">
            <span class="text-[10px] text-indigo-300 uppercase tracking-wider font-bold">Pelanggan Member</span>
            <div class="font-black text-white text-sm leading-tight">{{ displayState.selectedCustomer.name }}</div>
          </div>
        </div>

        <!-- Co-branding: Powered by Notane POS -->
        <div
          v-if="!storeBrandingStore.removeBranding"
          class="flex items-center gap-1.5 bg-slate-800/80 border border-slate-700/80 px-2.5 py-1.5 rounded-xl text-slate-300 shadow-inner"
        >
          <img src="/logo.png" alt="Notane Logo" class="w-4 h-4 object-contain rounded" />
          <span class="font-mono text-[10px] font-semibold text-slate-400 uppercase tracking-tight">Powered by Notane POS</span>
        </div>

        <div class="font-mono text-xs font-bold text-slate-400 px-3 py-1.5 rounded-lg bg-slate-800 border border-slate-700">
          {{ currentTime }}
        </div>
      </div>
    </header>

    <!-- 2. Main Body Area -->
    <main class="flex-1 overflow-hidden p-6 relative flex flex-col">
      <!-- STATE A: PAYMENT SUCCESS (Kembalian & Ucapan Terima Kasih) -->
      <div
        v-if="isSuccess && displayState.paymentResult"
        class="flex-1 flex flex-col items-center justify-center animate-fadeIn"
      >
        <div class="w-full max-w-2xl bg-gradient-to-b from-slate-800 to-slate-850 border-2 border-emerald-500/60 rounded-3xl p-8 shadow-2xl text-center relative overflow-hidden">
          <!-- Background Glow Effect -->
          <div class="absolute -top-24 -left-24 w-60 h-60 bg-emerald-500/10 rounded-full blur-3xl pointer-events-none"></div>
          <div class="absolute -bottom-24 -right-24 w-60 h-60 bg-teal-500/10 rounded-full blur-3xl pointer-events-none"></div>

          <div class="w-20 h-20 bg-emerald-500/20 border-2 border-emerald-400 rounded-full flex items-center justify-center mx-auto mb-4 text-emerald-400 shadow-lg animate-bounce">
            <Icon icon="fluent-color:checkmark-circle-24" class="w-12 h-12" />
          </div>

          <span class="text-xs uppercase font-extrabold tracking-widest text-emerald-400 bg-emerald-950/80 px-3 py-1 rounded-full border border-emerald-500/40">
            Transaksi Selesai & Lunas
          </span>

          <h2 class="text-3xl font-black text-white mt-3">Terima Kasih Telah Berbelanja!</h2>
          <p class="text-slate-400 text-xs mt-1 font-mono">No. Faktur: {{ displayState.paymentResult.invoiceNumber }}</p>

          <div class="grid grid-cols-2 gap-4 mt-6 text-left">
            <div class="bg-slate-900/80 border border-slate-700/80 p-4 rounded-2xl">
              <span class="text-xs text-slate-400 font-bold uppercase tracking-wider">Total Belanja</span>
              <div class="text-2xl font-black font-mono text-white mt-1">
                {{ formatCurrency(displayState.grandTotal) }}
              </div>
            </div>

            <div class="bg-slate-900/80 border border-slate-700/80 p-4 rounded-2xl">
              <span class="text-xs text-slate-400 font-bold uppercase tracking-wider">Metode & Bayar</span>
              <div class="text-xl font-bold font-mono text-slate-200 mt-1">
                {{ formatCurrency(displayState.paymentResult.tendered) }}
                <span class="text-xs text-emerald-400 block font-sans font-medium mt-0.5">
                  ({{ displayState.paymentResult.paymentMethod }})
                </span>
              </div>
            </div>
          </div>

          <!-- Kembalian Giant Card -->
          <div class="mt-4 bg-emerald-950/70 border-2 border-emerald-500/50 p-5 rounded-2xl flex items-center justify-between shadow-inner">
            <div class="text-left">
              <span class="text-xs uppercase font-extrabold text-emerald-300 tracking-wider">Kembalian Anda</span>
              <div class="text-xs text-emerald-400/80 mt-0.5">Silakan ambil uang kembalian Anda di kasir</div>
            </div>
            <div class="text-4xl font-black font-mono text-emerald-300">
              {{ formatCurrency(displayState.paymentResult.change) }}
            </div>
          </div>
        </div>
      </div>

      <!-- STATE B: ITEMS IN CART (Daftar Barang Belanja) -->
      <div v-else-if="hasItems" class="flex-1 flex flex-col overflow-hidden">
        <div class="flex items-center justify-between pb-3 text-xs text-slate-400 font-bold uppercase tracking-wider border-b border-slate-800 px-2">
          <span>Daftar Barang Terpindai ({{ displayState.items.length }} Item)</span>
          <span>Harga & Subtotal</span>
        </div>

        <div class="flex-1 overflow-y-auto space-y-2.5 py-3 pr-1">
          <div
            v-for="(item, idx) in displayState.items"
            :key="item.id"
            class="p-4 rounded-2xl border transition-all duration-300 flex items-center justify-between shadow-md"
            :class="idx === 0
              ? 'bg-slate-800/90 border-emerald-500/80 shadow-emerald-500/10 ring-1 ring-emerald-500/30'
              : 'bg-slate-800/50 border-slate-700/60 hover:bg-slate-800/70'"
          >
            <div class="flex items-center gap-3.5">
              <div
                class="w-10 h-10 rounded-xl flex items-center justify-center shrink-0 border"
                :class="idx === 0
                  ? 'bg-emerald-500/20 border-emerald-400/40 text-emerald-400'
                  : 'bg-slate-700/50 border-slate-600/50 text-slate-400'"
              >
                <Icon icon="streamline-ultimate-color:shopping-bag-check" class="w-5 h-5" />
              </div>

              <div>
                <div class="flex items-center gap-2">
                  <h3 class="text-base font-extrabold text-white tracking-tight leading-snug">
                    {{ item.name }}
                  </h3>
                  <span
                    v-if="idx === 0"
                    class="bg-emerald-500 text-slate-950 font-black text-[9px] px-1.5 py-0.5 rounded uppercase tracking-wider animate-pulse"
                  >
                    Baru Dipindai
                  </span>
                </div>

                <div class="flex items-center gap-2 text-xs text-slate-400 font-mono mt-0.5">
                  <span class="text-white font-bold">{{ item.quantity }} {{ item.unit_code }}</span>
                  <span>×</span>
                  <span>{{ formatCurrency(item.unit_price) }}</span>
                  <span v-if="item.discount_amount > 0" class="text-rose-400 font-bold ml-1">
                    (Diskon: -{{ formatCurrency(item.discount_amount) }})
                  </span>
                </div>
              </div>
            </div>

            <div class="text-right shrink-0">
              <div class="text-2xl font-black font-mono text-emerald-400">
                {{ formatCurrency(item.subtotal) }}
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- STATE C: IDLE (Keranjang Kosong / Menunggu) -->
      <div
        v-else
        class="flex-1 flex flex-col items-center justify-center text-center space-y-5 py-12 animate-fadeIn"
      >
        <div class="w-28 h-28 rounded-3xl bg-slate-800/80 border-2 border-slate-700 p-3 flex items-center justify-center shadow-2xl overflow-hidden">
          <img
            v-if="storeBrandingStore.storeLogoUrl"
            :src="storeBrandingStore.storeLogoUrl"
            alt="Logo Toko"
            class="w-full h-full object-contain filter drop-shadow-md"
          />
          <div
            v-else
            class="w-full h-full rounded-2xl bg-gradient-to-br from-emerald-500 to-teal-700 flex items-center justify-center text-white font-black text-3xl shadow-inner"
          >
            {{ (storeBrandingStore.storeName || 'N').charAt(0).toUpperCase() }}
          </div>
        </div>

        <div>
          <h2 class="text-3xl font-black text-white tracking-tight uppercase">
            Selamat Datang di {{ storeBrandingStore.storeName }}
          </h2>
          <p class="text-slate-400 text-sm mt-1.5 max-w-md mx-auto">
            {{ storeBrandingStore.storeTagline || 'Silakan letakkan barang belanjaan Anda di meja kasir. Detail dan harga akan tampil secara realtime di layar ini.' }}
          </p>
        </div>

        <div class="flex items-center gap-3">
          <div class="flex items-center gap-2 text-xs text-emerald-400 bg-emerald-950/60 border border-emerald-500/30 px-3.5 py-1.5 rounded-full font-bold shadow-xs">
            <span class="w-2 h-2 rounded-full bg-emerald-400 animate-ping"></span>
            <span>Kasir Siap Melayani Transaksi Anda</span>
          </div>

          <div
            v-if="!storeBrandingStore.removeBranding"
            class="flex items-center gap-1.5 text-[11px] text-slate-400 bg-slate-800/80 border border-slate-700/70 px-3 py-1.5 rounded-full font-mono"
          >
            <img src="/logo.png" alt="Notane POS" class="w-3.5 h-3.5 object-contain rounded" />
            <span>Powered by <strong class="text-emerald-400">Notane POS</strong></span>
          </div>
        </div>
      </div>
    </main>

    <!-- 3. Bottom Giant Total Footer -->
    <footer class="bg-slate-850 border-t-2 border-slate-700 px-8 py-5 flex items-center justify-between shrink-0 shadow-2xl">
      <div class="flex items-center gap-6">
        <div>
          <span class="text-xs font-bold text-slate-400 uppercase tracking-widest block">Total Tagihan Belanja</span>
          <div class="flex items-center gap-2 text-xs text-slate-400 mt-1 font-mono">
            <span class="bg-slate-700/80 px-2 py-0.5 rounded text-white font-bold">
              {{ displayState.totalItemsCount || displayState.items.length }} Barang
            </span>
            <span v-if="displayState.totalDiscount > 0" class="text-rose-400 font-bold">
              Hemat: {{ formatCurrency(displayState.totalDiscount) }}
            </span>
            <span v-if="displayState.totalTax > 0" class="text-slate-400">
              (Termasuk PPN: {{ formatCurrency(displayState.totalTax) }})
            </span>
          </div>
        </div>
      </div>

      <div class="text-right">
        <div class="text-5xl lg:text-6xl font-black font-mono text-emerald-400 tracking-tight drop-shadow-md">
          {{ formatCurrency(displayState.grandTotal) }}
        </div>
      </div>
    </footer>
  </div>
</template>
