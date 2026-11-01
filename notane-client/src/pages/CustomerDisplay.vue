<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import {
  subscribeCustomerDisplay,
  getCachedCustomerDisplayState,
  type CustomerDisplayState
} from '@/services/customerDisplayChannel'
import { Icon } from '@iconify/vue'
import { useStoreBrandingStore } from '@/stores/storeBrandingStore'
import { api } from '@/services/api'
import type { CustomerDisplayBanner } from '@/types'
import { formatCurrency, formatQty } from '@/utils/formatters'
import { Building2, ChevronLeft, ChevronRight } from 'lucide-vue-next'

const displayState = ref<CustomerDisplayState>(getCachedCustomerDisplayState())
const storeBrandingStore = useStoreBrandingStore()
let unsubscribe: (() => void) | null = null

const currentTime = ref<string>('')
let timer: any = null

// Banner Promosi & Brand Partner State
const banners = ref<CustomerDisplayBanner[]>([])
const currentBannerIndex = ref<number>(0)
let bannerInterval: any = null
let pollBannersInterval: any = null

function updateTime() {
  const now = new Date()
  currentTime.value = now.toLocaleTimeString('id-ID', { hour: '2-digit', minute: '2-digit', second: '2-digit' })
}

const isSuccess = computed(() => displayState.value.status === 'PAYMENT_SUCCESS' && !!displayState.value.paymentResult)
const hasItems = computed(() => !isSuccess.value && displayState.value.items.length > 0)

const currentBanner = computed(() => {
  if (banners.value.length === 0) return null
  return banners.value[currentBannerIndex.value] || banners.value[0]
})

async function fetchBanners() {
  try {
    const res = await api.get('/customer-display/banners')
    if (res.data?.success && Array.isArray(res.data.data)) {
      banners.value = res.data.data
      if (currentBannerIndex.value >= banners.value.length) {
        currentBannerIndex.value = 0
      }
    }
  } catch (err) {
    console.warn('Customer Display: Belum dapat memuat banner promosi:', err)
  }
}

function startBannerRotation() {
  if (bannerInterval) clearInterval(bannerInterval)
  bannerInterval = setInterval(() => {
    if (banners.value.length > 1) {
      currentBannerIndex.value = (currentBannerIndex.value + 1) % banners.value.length
    }
  }, 6000)
}

function nextBanner() {
  if (banners.value.length > 1) {
    currentBannerIndex.value = (currentBannerIndex.value + 1) % banners.value.length
  }
}

function prevBanner() {
  if (banners.value.length > 1) {
    currentBannerIndex.value = (currentBannerIndex.value - 1 + banners.value.length) % banners.value.length
  }
}

onMounted(() => {
  updateTime()
  timer = setInterval(updateTime, 1000)
  storeBrandingStore.fetchBranding()
  fetchBanners()
  startBannerRotation()

  // Polling banner setiap 60 detik untuk auto-sync jika admin mengupdate banner di back-office
  pollBannersInterval = setInterval(fetchBanners, 60000)

  unsubscribe = subscribeCustomerDisplay((newState) => {
    displayState.value = newState
  })
})

onUnmounted(() => {
  if (unsubscribe) unsubscribe()
  if (timer) clearInterval(timer)
  if (bannerInterval) clearInterval(bannerInterval)
  if (pollBannersInterval) clearInterval(pollBannersInterval)
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
            {{ storeBrandingStore.storeTagline || 'Daftar Belanja & Total Pembayaran' }}
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
        <div class="w-full max-w-2xl bg-slate-850 border border-slate-700 rounded-3xl p-8 shadow-2xl text-center relative overflow-hidden">
          <div class="w-16 h-16 bg-emerald-500/10 border border-emerald-500/30 rounded-full flex items-center justify-center mx-auto mb-4 text-emerald-400">
            <Icon icon="fluent-color:checkmark-circle-24" class="w-10 h-10" />
          </div>

          <span class="text-xs uppercase font-extrabold tracking-wider text-emerald-400 bg-emerald-950/80 px-3 py-1 rounded-full border border-emerald-500/40">
            Pembayaran Selesai
          </span>

          <h2 class="text-3xl font-black text-white mt-3">Terima Kasih Atas Kunjungan Anda</h2>
          <p class="text-slate-400 text-xs mt-1 font-mono">No. Faktur: {{ displayState.paymentResult.invoiceNumber }}</p>

          <div class="grid grid-cols-2 gap-4 mt-6 text-left">
            <div class="bg-slate-900 border border-slate-700/80 p-4 rounded-2xl">
              <span class="text-xs text-slate-400 font-bold uppercase tracking-wider">Total Belanja</span>
              <div class="text-2xl font-black font-mono text-white mt-1">
                {{ formatCurrency(displayState.grandTotal) }}
              </div>
            </div>

            <div class="bg-slate-900 border border-slate-700/80 p-4 rounded-2xl">
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
          <div class="mt-4 bg-emerald-950/70 border border-emerald-500/50 p-5 rounded-2xl flex items-center justify-between shadow-inner">
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

      <!-- STATE B: ITEMS IN CART (Daftar Barang Belanja + Banner Promosi Side by Side) -->
      <div v-else-if="hasItems" class="flex-1 flex overflow-hidden gap-6">
        <!-- Kolom Kiri: Daftar Barang Terpindai (Tidak terganggu oleh banner) -->
        <div class="flex-1 flex flex-col overflow-hidden">
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
                      class="bg-emerald-500 text-slate-950 font-bold text-[9px] px-1.5 py-0.5 rounded uppercase tracking-wider"
                    >
                      Baru Dipindai
                    </span>
                  </div>

                  <div class="flex items-center gap-2 text-xs text-slate-400 font-mono mt-0.5">
                    <span class="text-white font-bold">{{ formatQty(item.quantity) }} {{ item.unit_code }}</span>
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

        <!-- Kolom Kanan: Panel Banner Promosi Toko & Sponsor Brand (Berdiri sendiri tanpa mengganggu list) -->
        <div
          v-if="currentBanner"
          class="w-80 lg:w-96 shrink-0 flex flex-col justify-between bg-slate-850/90 rounded-3xl border border-slate-700/80 p-4 shadow-xl overflow-hidden relative animate-in fade-in"
        >
          <div>
            <!-- Header Banner Card: Badge & Brand Sponsor -->
            <div class="flex items-center justify-between gap-2 mb-3">
              <span class="px-2.5 py-0.5 rounded-full text-[10px] font-black uppercase tracking-wider bg-emerald-500/20 text-emerald-300 border border-emerald-500/40">
                {{ currentBanner.badge_text || 'PROMO SPESIAL' }}
              </span>
              <span
                v-if="currentBanner.brand_name"
                class="text-[10px] font-bold text-indigo-300 flex items-center gap-1 bg-indigo-950/80 px-2.5 py-0.5 rounded-full border border-indigo-500/40 truncate max-w-[180px]"
              >
                <Building2 class="w-3 h-3 text-indigo-400 shrink-0" />
                <span class="truncate">{{ currentBanner.brand_name }}</span>
              </span>
            </div>

            <!-- Banner Visual -->
            <div class="relative aspect-16/9 rounded-2xl overflow-hidden border border-slate-700/80 bg-slate-900 shadow-md">
              <img
                :src="currentBanner.image_url"
                :alt="currentBanner.title"
                class="w-full h-full object-cover transition-all duration-500"
              />
              <div class="absolute inset-0 bg-gradient-to-t from-slate-950/70 via-transparent to-transparent pointer-events-none"></div>
            </div>

            <!-- Title & Subtitle -->
            <div class="mt-3.5 space-y-1">
              <h4 class="text-sm font-extrabold text-white leading-snug line-clamp-2">
                {{ currentBanner.title }}
              </h4>
              <p v-if="currentBanner.subtitle" class="text-xs text-slate-300 line-clamp-2 leading-relaxed opacity-90">
                {{ currentBanner.subtitle }}
              </p>
            </div>
          </div>

          <!-- Carousel Controls & Indicators -->
          <div v-if="banners.length > 1" class="flex items-center justify-between pt-3 mt-3 border-t border-slate-800">
            <button
              type="button"
              @click="prevBanner"
              class="p-1 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800 transition cursor-pointer"
            >
              <ChevronLeft class="w-4 h-4" />
            </button>

            <!-- Indicators Dots -->
            <div class="flex items-center gap-1.5">
              <button
                v-for="(_, bIdx) in banners"
                :key="bIdx"
                @click="currentBannerIndex = bIdx"
                class="h-1.5 rounded-full transition-all duration-300 cursor-pointer"
                :class="bIdx === currentBannerIndex ? 'w-5 bg-emerald-400' : 'w-1.5 bg-slate-700 hover:bg-slate-500'"
              ></button>
            </div>

            <button
              type="button"
              @click="nextBanner"
              class="p-1 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800 transition cursor-pointer"
            >
              <ChevronRight class="w-4 h-4" />
            </button>
          </div>
        </div>
      </div>

      <!-- STATE C: IDLE (Keranjang Kosong / Menunggu Kasir - Hero Showcase Banner Promosi) -->
      <div
        v-else
        class="flex-1 flex flex-col lg:flex-row items-center justify-center gap-8 py-6 px-4 animate-fadeIn overflow-hidden"
      >
        <!-- Sisi Kiri: Ucapan Selamat Datang & Identitas Toko -->
        <div class="max-w-md text-center lg:text-left space-y-4">
          <div class="w-24 h-24 rounded-3xl bg-slate-800/80 border-2 border-slate-700 p-2.5 inline-flex items-center justify-center shadow-2xl overflow-hidden">
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
            <h2 class="text-3xl font-black text-white tracking-tight uppercase leading-tight">
              Selamat Datang di {{ storeBrandingStore.storeName }}
            </h2>
            <p class="text-slate-400 text-xs sm:text-sm mt-2 leading-relaxed">
              {{ storeBrandingStore.storeTagline || 'Silakan letakkan barang belanjaan di meja kasir. Rincian belanja dan total harga akan tampil di layar ini.' }}
            </p>
          </div>

          <div class="flex items-center gap-2.5 justify-center lg:justify-start">
            <div class="flex items-center gap-2 text-xs text-emerald-400 bg-emerald-950/60 border border-emerald-500/30 px-3.5 py-1.5 rounded-full font-bold shadow-xs">
              <span class="w-2 h-2 rounded-full bg-emerald-400"></span>
              <span>Kasir Siap Melayani Transaksi</span>
            </div>

            <div
              v-if="!storeBrandingStore.removeBranding"
              class="flex items-center gap-1.5 text-[11px] text-slate-400 bg-slate-800/80 border border-slate-700/70 px-3 py-1.5 rounded-full font-mono"
            >
              <img src="/logo.png" alt="Notane POS" class="w-3.5 h-3.5 object-contain rounded" />
              <span>Notane POS</span>
            </div>
          </div>
        </div>

        <!-- Sisi Kanan: Hero Showcase Banner Carousel (Jika ada banner) -->
        <div
          v-if="currentBanner"
          class="w-full max-w-xl bg-slate-850/90 border border-slate-700/80 rounded-3xl p-5 shadow-2xl overflow-hidden flex flex-col justify-between"
        >
          <!-- Badge Header -->
          <div class="flex items-center justify-between gap-2 mb-3">
            <span class="px-3 py-1 rounded-full text-xs font-black uppercase tracking-wider bg-emerald-500 text-slate-950 shadow-xs">
              {{ currentBanner.badge_text || 'PROMO SPESIAL' }}
            </span>
            <span
              v-if="currentBanner.brand_name"
              class="text-xs font-bold text-indigo-300 flex items-center gap-1.5 bg-indigo-950/80 px-3 py-1 rounded-full border border-indigo-500/40"
            >
              <Building2 class="w-3.5 h-3.5 text-indigo-400" />
              <span>{{ currentBanner.brand_name }}</span>
            </span>
          </div>

          <!-- Hero Image -->
          <div class="relative aspect-16/9 rounded-2xl overflow-hidden border border-slate-700 bg-slate-950 shadow-inner group">
            <img
              :src="currentBanner.image_url"
              :alt="currentBanner.title"
              class="w-full h-full object-cover transition-all duration-700"
            />
            <div class="absolute inset-0 bg-gradient-to-t from-slate-950/80 via-transparent to-transparent pointer-events-none"></div>

            <div class="absolute bottom-4 inset-x-4 text-white">
              <h3 class="text-lg font-black leading-tight drop-shadow-md">
                {{ currentBanner.title }}
              </h3>
              <p v-if="currentBanner.subtitle" class="text-xs text-slate-200 mt-1 line-clamp-2 drop-shadow-sm opacity-90">
                {{ currentBanner.subtitle }}
              </p>
            </div>
          </div>

          <!-- Carousel Controls & Indicators -->
          <div v-if="banners.length > 1" class="flex items-center justify-between pt-3 mt-3 border-t border-slate-800">
            <button
              type="button"
              @click="prevBanner"
              class="p-1.5 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800 transition cursor-pointer"
            >
              <ChevronLeft class="w-5 h-5" />
            </button>

            <!-- Indicators Dots -->
            <div class="flex items-center gap-2">
              <button
                v-for="(_, bIdx) in banners"
                :key="bIdx"
                @click="currentBannerIndex = bIdx"
                class="h-2 rounded-full transition-all duration-300 cursor-pointer"
                :class="bIdx === currentBannerIndex ? 'w-6 bg-emerald-400' : 'w-2 bg-slate-700 hover:bg-slate-500'"
              ></button>
            </div>

            <button
              type="button"
              @click="nextBanner"
              class="p-1.5 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800 transition cursor-pointer"
            >
              <ChevronRight class="w-5 h-5" />
            </button>
          </div>
        </div>
      </div>
    </main>

    <!-- 3. Bottom Giant Total Footer (Selalu 100% Lebar & Sangat Jelas Terlihat) -->
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
