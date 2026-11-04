<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, watch } from 'vue'
import {
  subscribeCustomerDisplay,
  getCachedCustomerDisplayState,
  type CustomerDisplayState
} from '@/services/customerDisplayChannel'
import { useStoreBrandingStore } from '@/stores/storeBrandingStore'
import { api } from '@/services/api'
import type { CustomerDisplayBanner } from '@/types'
import { formatCurrency, formatQty } from '@/utils/formatters'
import {
  Building2,
  ChevronLeft,
  ChevronRight,
  ShoppingBag,
  CheckCircle2,
  UserCheck,
  Receipt,
  Clock,
  Tag,
  Store,
} from 'lucide-vue-next'
import ThemeToggle from '@/components/common/ThemeToggle.vue'
import { useTheme } from '@/composables/useTheme'

const displayState = ref<CustomerDisplayState>(getCachedCustomerDisplayState())
const storeBrandingStore = useStoreBrandingStore()
const { setTheme, initTheme } = useTheme()
let unsubscribe: (() => void) | null = null

const currentTime = ref<string>('')
let timer: any = null

// Banner Promosi & Brand Partner State
const banners = ref<CustomerDisplayBanner[]>([])
const currentBannerIndex = ref<number>(0)
const imageFailed = ref<boolean>(false)
let bannerInterval: any = null
let pollBannersInterval: any = null

function updateTime() {
  const now = new Date()
  currentTime.value = now.toLocaleTimeString('id-ID', { hour: '2-digit', minute: '2-digit', second: '2-digit' })
}

const isSuccess = computed(() => displayState.value.status === 'PAYMENT_SUCCESS' && !!displayState.value.paymentResult)
const hasItems = computed(() => !isSuccess.value && displayState.value.items.length > 0)

// Banner fallback bawaan berbasis informasi toko nyata
const defaultBanners = computed<CustomerDisplayBanner[]>(() => [
  {
    id: 'default-1',
    title: 'Belanja Lengkap & Hemat Setiap Hari',
    subtitle: `Terima kasih telah berbelanja di ${storeBrandingStore.storeName || 'Toko Kami'}. Nikmati harga grosir dan promo menarik setiap hari.`,
    badge_text: 'PROMO TOKO',
    brand_name: storeBrandingStore.storeName || 'Notane Retail',
    image_url: '',
    is_active: true,
    display_order: 1,
    created_at: '',
    updated_at: '',
  },
  {
    id: 'default-2',
    title: 'Menerima Pembayaran Non-Tunai QRIS',
    subtitle: 'Transaksi lebih mudah, cepat, dan aman menggunakan QRIS seluruh e-wallet & perbankan digital.',
    badge_text: 'PEMBAYARAN DIGITAL',
    brand_name: 'QRIS & Cashless',
    image_url: '',
    is_active: true,
    display_order: 2,
    created_at: '',
    updated_at: '',
  },
  {
    id: 'default-3',
    title: 'Daftar Member & Nikmati Harga Khusus',
    subtitle: 'Kumpulkan poin di setiap transaksi dan dapatkan potongan harga eksklusif untuk pelanggan setia.',
    badge_text: 'MEMBER REWARDS',
    brand_name: 'Loyalitas Pelanggan',
    image_url: '',
    is_active: true,
    display_order: 3,
    created_at: '',
    updated_at: '',
  },
])

const activeBanners = computed(() => {
  if (banners.value.length > 0) return banners.value
  return defaultBanners.value
})

const currentBanner = computed(() => {
  if (activeBanners.value.length === 0) return null
  return activeBanners.value[currentBannerIndex.value % activeBanners.value.length]
})

watch(currentBannerIndex, () => {
  imageFailed.value = false
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
    if (activeBanners.value.length > 1) {
      currentBannerIndex.value = (currentBannerIndex.value + 1) % activeBanners.value.length
    }
  }, 6000)
}

function nextBanner() {
  if (activeBanners.value.length > 1) {
    currentBannerIndex.value = (currentBannerIndex.value + 1) % activeBanners.value.length
  }
}

function prevBanner() {
  if (activeBanners.value.length > 1) {
    currentBannerIndex.value = (currentBannerIndex.value - 1 + activeBanners.value.length) % activeBanners.value.length
  }
}

function handleStorage(e: StorageEvent) {
  if (e.key === 'notane_theme' && e.newValue) {
    setTheme(e.newValue as 'light' | 'dark')
  }
}

onMounted(() => {
  initTheme()
  window.addEventListener('storage', handleStorage)

  updateTime()
  timer = setInterval(updateTime, 1000)
  storeBrandingStore.fetchBranding()
  fetchBanners()
  startBannerRotation()

  // Polling banner setiap 60 detik untuk auto-sync jika admin mengupdate banner di back-office
  pollBannersInterval = setInterval(fetchBanners, 60000)

  unsubscribe = subscribeCustomerDisplay((newState) => {
    displayState.value = newState
    if (newState.theme) {
      setTheme(newState.theme)
    }
  })
})

onUnmounted(() => {
  window.removeEventListener('storage', handleStorage)
  if (unsubscribe) unsubscribe()
  if (timer) clearInterval(timer)
  if (bannerInterval) clearInterval(bannerInterval)
  if (pollBannersInterval) clearInterval(pollBannersInterval)
})
</script>

<template>
  <div class="h-screen w-screen bg-slate-100 dark:bg-slate-950 text-slate-800 dark:text-slate-100 flex flex-col justify-between select-none overflow-hidden font-sans transition-colors duration-200">
    <!-- 1. Top Header Bar: Clean 3-Zone Layout with Centered Brand -->
    <header class="bg-white dark:bg-slate-900 border-b border-slate-200 dark:border-slate-800 px-6 py-2.5 flex items-center justify-between shrink-0 z-20 transition-colors duration-200">
      
      <!-- Sisi Kiri: Status Kasir & Member Badge -->
      <div class="flex items-center gap-3 w-1/3 justify-start">
        <div class="flex items-center gap-2 text-xs text-emerald-800 dark:text-emerald-300 bg-emerald-50 dark:bg-emerald-950/60 border border-emerald-200 dark:border-emerald-800 px-3 py-1.5 rounded-lg font-semibold">
          <span class="w-2 h-2 rounded-full bg-emerald-600 dark:bg-emerald-400"></span>
          <span class="hidden sm:inline">Kasir Siap Melayani</span>
        </div>

        <div v-if="displayState.selectedCustomer" class="flex items-center gap-2 bg-indigo-50 dark:bg-indigo-950/70 border border-indigo-200 dark:border-indigo-800 px-3 py-1.5 rounded-lg">
          <UserCheck class="w-4 h-4 text-indigo-600 dark:text-indigo-400 shrink-0" />
          <div class="leading-tight">
            <span class="text-[9px] text-indigo-600 dark:text-indigo-300 uppercase tracking-wider font-bold block">Member</span>
            <span class="font-bold text-slate-900 dark:text-white text-xs truncate max-w-[140px] block">{{ displayState.selectedCustomer.name }}</span>
          </div>
        </div>
      </div>

      <!-- Sisi Tengah: Logo & Nama Toko (Center-Aligned) -->
      <div class="flex items-center justify-center gap-3 w-1/3 text-center">
        <div class="w-10 h-10 rounded-xl bg-slate-50 dark:bg-slate-800 border border-slate-200 dark:border-slate-700 p-1 flex items-center justify-center shrink-0 overflow-hidden">
          <img
            v-if="storeBrandingStore.storeLogoUrl"
            :src="storeBrandingStore.storeLogoUrl"
            alt="Logo Toko"
            class="w-full h-full object-contain"
          />
          <div
            v-else
            class="w-full h-full rounded-lg bg-emerald-600 flex items-center justify-center text-white font-bold text-lg"
          >
            {{ (storeBrandingStore.storeName || 'N').charAt(0).toUpperCase() }}
          </div>
        </div>

        <div class="text-left">
          <h1 class="text-base lg:text-lg font-extrabold tracking-tight text-slate-900 dark:text-white uppercase leading-tight truncate max-w-[200px] sm:max-w-[280px]">
            {{ storeBrandingStore.storeName || 'Notane POS' }}
          </h1>
          <p v-if="storeBrandingStore.storeTagline" class="text-xs text-slate-500 dark:text-slate-400 truncate max-w-[240px] sm:max-w-[320px]">
            {{ storeBrandingStore.storeTagline }}
          </p>
        </div>
      </div>

      <!-- Sisi Kanan: Jam Realtime, Powered by, Theme Toggle -->
      <div class="flex items-center gap-2.5 w-1/3 justify-end">
        <div class="hidden md:flex items-center gap-1.5 text-xs font-mono font-semibold text-slate-700 dark:text-slate-300 px-3 py-1.5 rounded-lg bg-slate-50 dark:bg-slate-800 border border-slate-200 dark:border-slate-700">
          <Clock class="w-3.5 h-3.5 text-slate-400" />
          <span>{{ currentTime }}</span>
        </div>

        <div
          v-if="!storeBrandingStore.removeBranding"
          class="hidden lg:flex items-center gap-1.5 bg-slate-50 dark:bg-slate-800/80 border border-slate-200 dark:border-slate-700/80 px-2.5 py-1.5 rounded-lg text-slate-500 dark:text-slate-400 text-xs font-medium"
        >
          <img src="/logo.png" alt="Notane Logo" class="w-3.5 h-3.5 object-contain rounded" />
          <span>Notane</span>
        </div>

        <ThemeToggle />
      </div>
    </header>

    <!-- 2. Main Body Area: 50% / 50% Full Height Grid -->
    <main class="flex-1 h-full min-h-0 overflow-hidden p-4 lg:p-6 grid grid-cols-2 gap-5 lg:gap-6">
      
      <!-- ================= KOLOM KIRI (50% & FULL HEIGHT): RINCIAN HARGA & FLOATING TOTAL ================= -->
      <section class="h-full min-h-0 flex flex-col justify-between overflow-hidden bg-white dark:bg-slate-900 rounded-2xl border border-slate-200 dark:border-slate-800 p-4 lg:p-5 relative">
        
        <!-- Header Rincian Belanja -->
        <div class="flex items-center justify-between pb-3 text-xs text-slate-600 dark:text-slate-400 font-bold uppercase tracking-wider border-b border-slate-200 dark:border-slate-800 px-1 shrink-0">
          <div class="flex items-center gap-2">
            <ShoppingBag class="w-4 h-4 text-emerald-600 dark:text-emerald-400" />
            <span>Rincian Belanja</span>
            <span v-if="hasItems" class="px-2 py-0.5 rounded-md text-[10px] font-bold bg-slate-100 dark:bg-slate-800 text-slate-800 dark:text-slate-200 border border-slate-200 dark:border-slate-700">
              {{ displayState.items.length }} Item
            </span>
          </div>
          <span>Harga & Subtotal</span>
        </div>

        <!-- Area Scroll Rincian Item / Status Transaksi -->
        <div class="flex-1 overflow-y-auto space-y-2.5 py-3 pr-1 min-h-0">
          
          <!-- Kasus 1: Pembayaran Berhasil -->
          <div
            v-if="isSuccess && displayState.paymentResult"
            class="h-full flex flex-col items-center justify-center p-4 text-center"
          >
            <div class="w-full bg-slate-50 dark:bg-slate-950 border border-emerald-300 dark:border-emerald-800 rounded-2xl p-6 relative overflow-hidden">
              <div class="w-12 h-12 bg-emerald-100 dark:bg-emerald-500/20 border border-emerald-300 dark:border-emerald-500/40 rounded-full flex items-center justify-center mx-auto mb-3 text-emerald-600 dark:text-emerald-400">
                <CheckCircle2 class="w-8 h-8" />
              </div>

              <span class="text-xs uppercase font-bold tracking-wider text-emerald-800 dark:text-emerald-300 bg-emerald-100/70 dark:bg-emerald-950/80 px-3 py-1 rounded-full border border-emerald-300 dark:border-emerald-700">
                Pembayaran Berhasil
              </span>

              <h2 class="text-2xl font-black text-slate-900 dark:text-white mt-2">Terima Kasih Atas Kunjungan Anda</h2>
              <p class="text-slate-500 dark:text-slate-400 text-xs mt-1 font-mono">No. Faktur: {{ displayState.paymentResult.invoiceNumber }}</p>

              <div class="grid grid-cols-2 gap-3 mt-5 text-left">
                <div class="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 p-3.5 rounded-xl">
                  <span class="text-[11px] text-slate-500 dark:text-slate-400 font-semibold uppercase tracking-wider">Total Belanja</span>
                  <div class="text-xl font-bold font-mono text-slate-900 dark:text-white mt-0.5">
                    {{ formatCurrency(displayState.grandTotal) }}
                  </div>
                </div>

                <div class="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 p-3.5 rounded-xl">
                  <span class="text-[11px] text-slate-500 dark:text-slate-400 font-semibold uppercase tracking-wider">Metode Bayar</span>
                  <div class="text-lg font-bold font-mono text-slate-800 dark:text-slate-200 mt-0.5">
                    {{ formatCurrency(displayState.paymentResult.tendered) }}
                    <span class="text-xs text-emerald-600 dark:text-emerald-400 block font-sans font-medium">
                      ({{ displayState.paymentResult.paymentMethod }})
                    </span>
                  </div>
                </div>
              </div>

              <!-- Kembalian Card -->
              <div class="mt-4 bg-emerald-100/70 dark:bg-emerald-950/70 border border-emerald-300 dark:border-emerald-700 p-4 rounded-xl flex items-center justify-between">
                <div class="text-left">
                  <span class="text-xs uppercase font-extrabold text-emerald-900 dark:text-emerald-300 tracking-wider">Kembalian Anda</span>
                  <div class="text-xs text-emerald-800/80 dark:text-emerald-400/80 mt-0.5">Silakan ambil uang kembalian Anda</div>
                </div>
                <div class="text-3xl font-black font-mono text-emerald-700 dark:text-emerald-300">
                  {{ formatCurrency(displayState.paymentResult.change) }}
                </div>
              </div>
            </div>
          </div>

          <!-- Kasus 2: Ada Barang di Keranjang -->
          <template v-else-if="hasItems">
            <div
              v-for="(item, idx) in displayState.items"
              :key="item.id"
              class="p-3.5 lg:p-4 rounded-xl border transition-colors flex items-center justify-between"
              :class="idx === 0
                ? 'bg-emerald-50/60 dark:bg-emerald-950/30 border-emerald-300 dark:border-emerald-800'
                : 'bg-slate-50/70 dark:bg-slate-950/50 border-slate-200 dark:border-slate-800 hover:border-slate-300 dark:hover:border-slate-700'"
            >
              <div class="flex items-center gap-3.5 min-w-0 pr-3">
                <div
                  class="w-10 h-10 rounded-lg flex items-center justify-center shrink-0 border"
                  :class="idx === 0
                    ? 'bg-emerald-100 dark:bg-emerald-500/20 border-emerald-300 dark:border-emerald-400/40 text-emerald-700 dark:text-emerald-400'
                    : 'bg-white dark:bg-slate-800 border-slate-200 dark:border-slate-700 text-slate-500 dark:text-slate-400'"
                >
                  <ShoppingBag class="w-5 h-5" />
                </div>

                <div class="min-w-0">
                  <div class="flex items-center gap-2 flex-wrap">
                    <h3 class="text-sm lg:text-base font-bold text-slate-900 dark:text-white tracking-tight leading-snug truncate max-w-[200px] sm:max-w-[280px]">
                      {{ item.name }}
                    </h3>
                    <span
                      v-if="idx === 0"
                      class="bg-emerald-600 text-white font-bold text-[9px] px-2 py-0.5 rounded uppercase tracking-wider shrink-0"
                    >
                      Item Terakhir
                    </span>
                  </div>

                  <div class="flex items-center gap-2 text-xs text-slate-600 dark:text-slate-400 font-mono mt-0.5 flex-wrap">
                    <span class="text-slate-900 dark:text-slate-200 font-bold">{{ formatQty(item.quantity) }} {{ item.unit_code }}</span>
                    <span>×</span>
                    <span>{{ formatCurrency(item.unit_price) }}</span>
                    <span v-if="item.discount_amount > 0" class="text-rose-600 dark:text-rose-400 font-semibold ml-1">
                      (Diskon: -{{ formatCurrency(item.discount_amount) }})
                    </span>
                  </div>
                </div>
              </div>

              <div class="text-right shrink-0">
                <div class="text-xl lg:text-2xl font-black font-mono text-emerald-700 dark:text-emerald-400">
                  {{ formatCurrency(item.subtotal) }}
                </div>
              </div>
            </div>
          </template>

          <!-- Kasus 3: Keranjang Kosong (Idle) -->
          <div
            v-else
            class="h-full min-h-[260px] flex flex-col items-center justify-center p-6 text-center rounded-xl border border-dashed border-slate-300 dark:border-slate-800 bg-slate-50/50 dark:bg-slate-950/40"
          >
            <div class="w-14 h-14 rounded-xl bg-slate-100 dark:bg-slate-800 border border-slate-200 dark:border-slate-700 flex items-center justify-center text-slate-600 dark:text-slate-400 mb-3">
              <Receipt class="w-7 h-7" />
            </div>

            <h3 class="text-base font-bold text-slate-800 dark:text-slate-200">
              Keranjang Belanja Kosong
            </h3>
            <p class="text-xs text-slate-500 dark:text-slate-400 max-w-sm mt-1 leading-relaxed">
              Silakan letakkan barang belanjaan di meja kasir. Rincian produk dan total harga akan tampil di sini saat kasir memindai barang.
            </p>
          </div>

        </div>

        <!-- Bagian Paling Bawah: Floating Total Card -->
        <div class="shrink-0 mt-2 p-4 lg:p-5 rounded-xl border border-slate-200 dark:border-slate-800 bg-slate-50/90 dark:bg-slate-950/90 shadow-md">
          <div class="flex items-center justify-between gap-4">
            <div>
              <span class="text-[11px] font-bold text-slate-500 dark:text-slate-400 uppercase tracking-wider block">Total Tagihan Belanja</span>
              <div class="flex items-center gap-2 text-xs text-slate-600 dark:text-slate-400 mt-1 font-mono flex-wrap">
                <span class="bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 px-2 py-0.5 rounded text-slate-800 dark:text-slate-200 font-bold">
                  {{ displayState.totalItemsCount || displayState.items.length }} Barang
                </span>
                <span v-if="displayState.totalDiscount > 0" class="text-rose-600 dark:text-rose-400 font-bold">
                  Hemat: {{ formatCurrency(displayState.totalDiscount) }}
                </span>
                <span v-if="displayState.totalTax > 0" class="text-slate-500 dark:text-slate-400">
                  (Termasuk PPN: {{ formatCurrency(displayState.totalTax) }})
                </span>
              </div>
            </div>

            <div class="text-right">
              <div class="text-3xl lg:text-4xl xl:text-5xl font-black font-mono text-emerald-700 dark:text-emerald-400 tracking-tight">
                {{ formatCurrency(displayState.grandTotal) }}
              </div>
            </div>
          </div>
        </div>

      </section>

      <!-- ================= KOLOM KANAN (50% & FULL HEIGHT): BANNER PROMOSI ================= -->
      <section class="h-full min-h-0 flex flex-col justify-between overflow-hidden bg-white dark:bg-slate-900 rounded-2xl border border-slate-200 dark:border-slate-800 relative">
        <div
          v-if="currentBanner"
          class="h-full min-h-0 flex flex-col justify-between overflow-hidden relative"
        >
          <!-- Header Banner: Badge Promo & Brand Partner -->
          <div class="p-4 pb-2 flex items-center justify-between gap-2 shrink-0">
            <span class="px-3 py-1 rounded-md text-[11px] font-bold uppercase tracking-wider bg-emerald-600 text-white flex items-center gap-1.5">
              <Tag class="w-3.5 h-3.5" />
              <span>{{ currentBanner.badge_text || 'PROMO' }}</span>
            </span>
            <span
              v-if="currentBanner.brand_name"
              class="text-xs font-semibold text-slate-700 dark:text-slate-300 flex items-center gap-1.5 bg-slate-100 dark:bg-slate-800 px-3 py-1 rounded-md border border-slate-200 dark:border-slate-700 truncate max-w-[200px]"
            >
              <Building2 class="w-3.5 h-3.5 text-slate-500 dark:text-slate-400 shrink-0" />
              <span class="truncate">{{ currentBanner.brand_name }}</span>
            </span>
          </div>

          <!-- Banner Visual & Captions (Full Height Container) -->
          <div class="flex-1 min-h-0 p-4 pt-1 flex flex-col justify-between overflow-hidden">
            <div class="relative flex-1 w-full h-full min-h-0 rounded-xl overflow-hidden border border-slate-200 dark:border-slate-800 bg-slate-900">
              <!-- Jika gambar ada dan valid -->
              <img
                v-if="currentBanner.image_url && !imageFailed"
                :src="currentBanner.image_url"
                :alt="currentBanner.title"
                class="w-full h-full object-cover transition-transform duration-500"
                @error="imageFailed = true"
              />

              <!-- Fallback visual jika offline / tanpa gambar -->
              <div
                v-else
                class="w-full h-full flex flex-col items-center justify-center bg-slate-950 p-6 text-center text-white relative"
              >
                <div class="w-14 h-14 rounded-xl bg-white/10 border border-white/20 flex items-center justify-center mb-3 text-emerald-400">
                  <Store class="w-7 h-7" />
                </div>
                <span class="text-xs uppercase font-extrabold tracking-widest text-emerald-400">
                  {{ currentBanner.badge_text || 'PROMO KHUSUS' }}
                </span>
                <h3 class="text-xl lg:text-2xl font-black mt-2 max-w-sm leading-snug">
                  {{ currentBanner.title }}
                </h3>
                <p v-if="currentBanner.subtitle" class="text-xs text-slate-300 mt-2 max-w-md leading-relaxed">
                  {{ currentBanner.subtitle }}
                </p>
              </div>

              <!-- Gradient & Info overlay jika memakai gambar -->
              <template v-if="currentBanner.image_url && !imageFailed">
                <div class="absolute inset-0 bg-gradient-to-t from-slate-950/90 via-slate-950/30 to-transparent pointer-events-none"></div>

                <div class="absolute bottom-5 inset-x-5 text-white">
                  <h3 class="text-lg lg:text-2xl font-extrabold leading-snug text-white">
                    {{ currentBanner.title }}
                  </h3>
                  <p v-if="currentBanner.subtitle" class="text-xs lg:text-sm text-slate-200 mt-1.5 line-clamp-3 leading-relaxed">
                    {{ currentBanner.subtitle }}
                  </p>
                </div>
              </template>
            </div>
          </div>

          <!-- Bottom Carousel Controls & Indicators -->
          <div
            v-if="activeBanners.length > 1"
            class="px-5 py-3 border-t border-slate-100 dark:border-slate-800 flex items-center justify-between shrink-0"
          >
            <button
              type="button"
              @click="prevBanner"
              aria-label="Lihat banner sebelumnya"
              class="p-1.5 rounded-lg text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white hover:bg-slate-100 dark:hover:bg-slate-800 focus-visible:ring-2 focus-visible:ring-emerald-500 focus-visible:outline-none transition cursor-pointer"
            >
              <ChevronLeft class="w-5 h-5" />
            </button>

            <!-- Indicators Dots -->
            <div class="flex items-center gap-1.5">
              <button
                v-for="(_, bIdx) in activeBanners"
                :key="bIdx"
                @click="currentBannerIndex = bIdx"
                :aria-label="'Buka slide banner ke-' + (bIdx + 1)"
                class="h-2 rounded-full transition-all duration-300 cursor-pointer focus-visible:ring-2 focus-visible:ring-emerald-500 focus-visible:outline-none"
                :class="bIdx === (currentBannerIndex % activeBanners.length) ? 'w-6 bg-emerald-600 dark:bg-emerald-500' : 'w-2 bg-slate-300 dark:bg-slate-700 hover:bg-slate-400 dark:hover:bg-slate-600'"
              ></button>
            </div>

            <button
              type="button"
              @click="nextBanner"
              aria-label="Lihat banner berikutnya"
              class="p-1.5 rounded-lg text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white hover:bg-slate-100 dark:hover:bg-slate-800 focus-visible:ring-2 focus-visible:ring-emerald-500 focus-visible:outline-none transition cursor-pointer"
            >
              <ChevronRight class="w-5 h-5" />
            </button>
          </div>
        </div>
      </section>

    </main>
  </div>
</template>
