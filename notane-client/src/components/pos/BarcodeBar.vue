<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from 'vue'
import { api } from '@/services/api'
import { useCartStore } from '@/stores/cartStore'
import { useShiftStore } from '@/stores/shiftStore'
import { soundService } from '@/services/soundService'

const emit = defineEmits<{
  (e: 'search'): void
  (e: 'customer'): void
  (e: 'voucher'): void
  (e: 'open-drawer'): void
  (e: 'hold'): void
  (e: 'recall'): void
  (e: 'checkout'): void
}>()

const cartStore = useCartStore()
const shiftStore = useShiftStore()
const barcodeInput = ref<string>('')
const inputRef = ref<HTMLInputElement | null>(null)
const containerRef = ref<HTMLElement | null>(null)
const isScanning = ref<boolean>(false)
const errorMessage = ref<string>('')

// Autocomplete Dropdown State
const searchResults = ref<any[]>([])
const showDropdown = ref<boolean>(false)
const selectedIndex = ref<number>(-1)
const isSearching = ref<boolean>(false)
let debounceTimer: any = null

function focusInput() {
  if (shiftStore.isShiftOpen) {
    inputRef.value?.focus()
  }
}

function formatCurrency(val: number): string {
  return new Intl.NumberFormat('id-ID', {
    style: 'currency',
    currency: 'IDR',
    minimumFractionDigits: 0,
    maximumFractionDigits: 0,
  }).format(val)
}

async function searchProducts(query: string) {
  if (!shiftStore.isShiftOpen || !cartStore.currentOutletId) {
    searchResults.value = []
    showDropdown.value = false
    selectedIndex.value = -1
    return
  }

  if (!query || query.trim().length === 0) {
    searchResults.value = []
    showDropdown.value = false
    selectedIndex.value = -1
    return
  }

  isSearching.value = true
  try {
    const params: any = { search: query.trim(), limit: 12 }

    const res = await api.get('/products', { params })
    if (res.data.success && Array.isArray(res.data.data)) {
      searchResults.value = res.data.data
      showDropdown.value = searchResults.value.length > 0
      selectedIndex.value = -1
    }
  } catch (err) {
    console.error('Gagal memuat dropdown barang:', err)
  } finally {
    isSearching.value = false
  }
}

watch(barcodeInput, (newVal) => {
  if (debounceTimer) clearTimeout(debounceTimer)
  if (!shiftStore.isShiftOpen || !newVal || newVal.trim().length === 0) {
    searchResults.value = []
    showDropdown.value = false
    selectedIndex.value = -1
    return
  }
  debounceTimer = setTimeout(() => {
    searchProducts(newVal)
  }, 120)
})

async function handleFocus() {
  if (!shiftStore.isShiftOpen) {
    searchResults.value = []
    showDropdown.value = false
    return
  }

  if (barcodeInput.value.trim().length > 0) {
    if (searchResults.value.length > 0) {
      showDropdown.value = true
    } else {
      searchProducts(barcodeInput.value)
    }
  } else {
    // Tampilkan beberapa barang cepat saat input diklik
    try {
      const params: any = { limit: 8 }
      const res = await api.get('/products', { params })
      if (res.data.success && Array.isArray(res.data.data) && res.data.data.length > 0) {
        searchResults.value = res.data.data
        showDropdown.value = true
        selectedIndex.value = -1
      }
    } catch {
      // Abaikan error background fetch
    }
  }
}

async function selectProduct(prod: any) {
  if (!shiftStore.isShiftOpen) {
    shiftStore.openShiftModal('OPEN')
    return
  }

  showDropdown.value = false
  errorMessage.value = ''
  barcodeInput.value = ''
  selectedIndex.value = -1

  if (Number(prod.stock_qty ?? 0) <= 0) {
    soundService.playOutOfStock()
    errorMessage.value = `Stok barang '${prod.name}' habis (0 ${prod.unit_code || 'PCS'})`
    setTimeout(() => {
      errorMessage.value = ''
    }, 3500)
    return
  }

  isScanning.value = true

  try {
    await cartStore.addProduct(prod, 1)
  } catch (err: any) {
    errorMessage.value = err.message || 'Gagal menambahkan barang'
    setTimeout(() => {
      errorMessage.value = ''
    }, 3500)
  } finally {
    isScanning.value = false
    focusInput()
  }
}

async function handleBarcodeSubmit() {
  if (!shiftStore.isShiftOpen) {
    errorMessage.value = 'Shift kasir belum dibuka. Silakan buka shift kasir terlebih dahulu.'
    shiftStore.openShiftModal('OPEN')
    setTimeout(() => {
      errorMessage.value = ''
    }, 3500)
    return
  }
  const code = barcodeInput.value.trim()
  if (!code) return

  showDropdown.value = false
  isScanning.value = true
  errorMessage.value = ''

  try {
    // Jika barcode input cocok dengan item hasil pencarian yang tepat
    if (searchResults.value.length > 0 && selectedIndex.value >= 0 && selectedIndex.value < searchResults.value.length) {
      const selected = searchResults.value[selectedIndex.value]
      if (Number(selected.stock_qty ?? 0) <= 0) {
        soundService.playOutOfStock()
        errorMessage.value = `Stok barang '${selected.name}' habis (0 ${selected.unit_code || 'PCS'})`
        setTimeout(() => { errorMessage.value = '' }, 3500)
        isScanning.value = false
        return
      }
      await cartStore.addProduct(selected, 1)
    } else {
      await cartStore.scanBarcode(code)
    }
    barcodeInput.value = ''
    selectedIndex.value = -1
  } catch (err: any) {
    // Coba fallback ke pencarian nama jika barcode gagal
    if (searchResults.value.length > 0) {
      try {
        const fallback = searchResults.value[0]
        if (Number(fallback.stock_qty ?? 0) > 0) {
          await cartStore.addProduct(fallback, 1)
          barcodeInput.value = ''
          selectedIndex.value = -1
          errorMessage.value = ''
          return
        }
      } catch {
        // Fallthrough ke pesan error
      }
    }
    soundService.playError()
    errorMessage.value = err.message || 'Barang tidak ditemukan'
    setTimeout(() => {
      errorMessage.value = ''
    }, 3000)
  } finally {
    isScanning.value = false
    focusInput()
  }
}

function handleInputKeyDown(e: KeyboardEvent) {
  if (e.key === 'ArrowDown') {
    if (showDropdown.value && searchResults.value.length > 0) {
      e.preventDefault()
      selectedIndex.value = (selectedIndex.value + 1) % searchResults.value.length
    } else if (!showDropdown.value) {
      handleFocus()
    }
  } else if (e.key === 'ArrowUp') {
    if (showDropdown.value && searchResults.value.length > 0) {
      e.preventDefault()
      selectedIndex.value = selectedIndex.value <= 0 ? searchResults.value.length - 1 : selectedIndex.value - 1
    }
  } else if (e.key === 'Escape') {
    if (showDropdown.value) {
      e.preventDefault()
      showDropdown.value = false
      selectedIndex.value = -1
    }
  } else if (e.key === 'Enter') {
    e.preventDefault()
    if (showDropdown.value && selectedIndex.value >= 0 && selectedIndex.value < searchResults.value.length) {
      selectProduct(searchResults.value[selectedIndex.value])
    } else {
      handleBarcodeSubmit()
    }
  }
}

function handleClickOutside(e: MouseEvent) {
  if (containerRef.value && !containerRef.value.contains(e.target as Node)) {
    showDropdown.value = false
    selectedIndex.value = -1
  }
}

// Global hotkey handler
function handleKeyDown(e: KeyboardEvent) {
  if (e.key === 'F1') {
    e.preventDefault()
    if (!shiftStore.isShiftOpen) {
      errorMessage.value = 'Shift kasir belum dibuka. Silakan buka shift kasir terlebih dahulu.'
      shiftStore.openShiftModal('OPEN')
      setTimeout(() => { errorMessage.value = '' }, 3500)
      return
    }
    emit('search')
  } else if (e.key === 'F4') {
    e.preventDefault()
    emit('open-drawer')
  } else if (e.key === 'F6') {
    e.preventDefault()
    emit('hold')
  } else if (e.key === 'F7') {
    e.preventDefault()
    emit('recall')
  } else if (e.key === 'F12') {
    e.preventDefault()
    emit('checkout')
  }
}

onMounted(() => {
  window.addEventListener('keydown', handleKeyDown)
  document.addEventListener('mousedown', handleClickOutside)
  focusInput()
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeyDown)
  document.removeEventListener('mousedown', handleClickOutside)
  if (debounceTimer) clearTimeout(debounceTimer)
})

defineExpose({
  focusInput,
})
</script>

<template>
  <div class="bg-white border-b border-slate-300 p-3 flex flex-col md:flex-row items-center gap-3 shadow-xs select-none">
    <!-- Barcode Scanner Input with Autocomplete Dropdown -->
    <div ref="containerRef" class="relative flex-1 w-full">
      <div class="absolute inset-y-0 left-0 pl-3.5 flex items-center pointer-events-none">
        <Icon icon="streamline-ultimate-color:barcode" class="w-5 h-5 drop-shadow-xs" />
      </div>
      <input
        ref="inputRef"
        v-model="barcodeInput"
        type="text"
        :placeholder="!shiftStore.isShiftOpen ? '⚠️ Shift Kasir Belum Dibuka — Silakan Buka Shift Kasir Terlebih Dahulu' : 'Pindai Barcode / Ketik Nama Barang / SKU... (Enter)'"
        class="w-full rounded-xl pl-11 pr-28 py-3 text-lg font-mono transition"
        :class="!shiftStore.isShiftOpen
          ? 'bg-amber-50/70 border-2 border-amber-300 text-amber-900 placeholder-amber-700/70 cursor-not-allowed select-none'
          : 'bg-slate-50 border-2 border-emerald-600/70 text-slate-900 placeholder-slate-400 focus:bg-white focus:outline-none focus:ring-4 focus:ring-emerald-500/20 focus:border-emerald-600'"
        :disabled="!shiftStore.isShiftOpen || isScanning"
        @focus="handleFocus"
        @keydown="handleInputKeyDown"
      />
      <div class="absolute inset-y-0 right-2 flex items-center gap-1.5">
        <span v-if="isScanning || isSearching" class="text-xs text-emerald-700 animate-pulse px-2 py-1 font-mono font-bold">
          {{ isSearching ? 'Mencari...' : 'Memproses...' }}
        </span>
        <button
          v-if="!shiftStore.isShiftOpen"
          @click="shiftStore.openShiftModal('OPEN')"
          class="bg-amber-500 hover:bg-amber-600 text-white text-xs font-bold px-3 py-1.5 rounded-lg border border-amber-600 transition active:scale-95 shadow-xs cursor-pointer flex items-center gap-1"
          title="Buka Shift Kasir untuk mulai transaksi"
        >
          Buka Shift
        </button>
        <button
          v-else
          @click="handleBarcodeSubmit"
          class="bg-emerald-600 hover:bg-emerald-700 text-white text-xs font-bold px-3 py-1.5 rounded-lg border border-emerald-700 transition active:scale-95 shadow-xs cursor-pointer"
        >
          Enter ↵
        </button>
      </div>

      <!-- Floating Error Alert -->
      <div
        v-if="errorMessage"
        class="absolute top-full mt-1.5 left-0 z-40 bg-rose-50 border border-rose-300 text-rose-700 text-xs px-3 py-1.5 rounded-lg shadow-md flex items-center gap-1.5 animate-bounce font-medium"
      >
        <span>⚠️ {{ errorMessage }}</span>
      </div>

      <!-- Dropdown Daftar Barang (Searchable Autocomplete Popover) -->
      <div
        v-if="showDropdown && searchResults.length > 0"
        class="absolute left-0 right-0 top-full mt-1.5 z-50 bg-white border-2 border-emerald-600 rounded-xl shadow-2xl max-h-80 overflow-y-auto divide-y divide-slate-100"
      >
        <div class="px-3 py-1.5 bg-emerald-50 border-b border-emerald-200 flex items-center justify-between text-[11px] font-mono text-emerald-900 font-bold">
          <span>Daftar Barang Ditemukan (Gunakan ↑ ↓ lalu Enter untuk memilih)</span>
          <span class="bg-emerald-200 text-emerald-900 px-2 py-0.5 rounded-full text-[10px]">{{ searchResults.length }} Opsi</span>
        </div>
        <div
          v-for="(prod, idx) in searchResults"
          :key="prod.id"
          @mousedown.prevent="selectProduct(prod)"
          @mouseenter="selectedIndex = idx"
          class="px-4 py-2.5 flex items-center justify-between cursor-pointer transition-colors"
          :class="selectedIndex === idx ? 'bg-amber-100 text-amber-950 font-medium' : 'hover:bg-emerald-50/60 text-slate-800'"
        >
          <div class="flex items-center gap-3">
            <div class="w-10 h-10 rounded-lg bg-slate-50 border border-slate-200 flex items-center justify-center shrink-0 shadow-xs overflow-hidden">
              <img
                v-if="prod.image_url"
                :src="prod.image_url"
                :alt="prod.name"
                class="w-full h-full object-cover"
              />
              <Icon v-else icon="streamline-ultimate-color:warehouse-cart-packages-2" class="w-5 h-5" />
            </div>
            <div>
              <div class="text-sm font-bold leading-tight" :class="selectedIndex === idx ? 'text-amber-950' : 'text-slate-900'">
                {{ prod.name }}
              </div>
              <div class="flex items-center gap-2 text-[11px] font-mono text-slate-500 mt-0.5">
                <span class="bg-slate-100 px-1.5 py-0.5 rounded border border-slate-200 text-slate-700 font-bold">SKU: {{ prod.sku }}</span>
                <span v-if="prod.barcode" class="text-slate-600 font-mono">Barcode: {{ prod.barcode }}</span>
                <span class="text-emerald-700 font-bold">• {{ prod.category_name || 'Retail' }}</span>
                <span>• Satuan: {{ prod.unit_code || 'PCS' }}</span>
              </div>
            </div>
          </div>
          <div class="text-right shrink-0">
            <div class="text-sm font-black font-mono text-emerald-700">
              {{ formatCurrency(Number(prod.base_selling_price || 0)) }}
            </div>
            <div class="text-[10px] font-mono">
              <span v-if="Number(prod.stock_qty ?? 0) <= 0" class="text-rose-600 bg-rose-50 px-1.5 py-0.5 rounded border border-rose-200 font-bold">
                Stok Habis (0)
              </span>
              <span v-else class="text-slate-500">
                Stok: <strong class="text-slate-800 font-bold">{{ prod.stock_qty }} {{ prod.unit_code || 'PCS' }}</strong>
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- Rapid Hotkey Toolbar -->
    <div class="flex items-center gap-2 overflow-x-auto w-full md:w-auto">
      <button
        @click="emit('search')"
        class="flex items-center gap-1.5 bg-slate-100 hover:bg-slate-200 text-slate-800 px-3 py-2 rounded-xl text-xs font-bold border border-slate-300 shadow-xs transition active:scale-95 whitespace-nowrap cursor-pointer"
        title="Pencarian Produk Manual (F1)"
      >
        <Icon icon="streamline-ultimate-color:search-circle" class="w-4 h-4" />
        <span>Cari</span>
        <kbd class="bg-white border border-slate-300 px-1.5 py-0.5 rounded text-[10px] text-slate-700 font-mono shadow-xs">F1</kbd>
      </button>

      <button
        @click="emit('customer')"
        class="flex items-center gap-1.5 bg-slate-100 hover:bg-slate-200 text-slate-800 px-3 py-2 rounded-xl text-xs font-bold border border-slate-300 shadow-xs transition active:scale-95 whitespace-nowrap cursor-pointer"
        title="Pilih Pelanggan / Member"
      >
        <Icon icon="streamline-ultimate-color:multiple-users-1" class="w-4 h-4" />
        <span class="max-w-[80px] truncate">{{ cartStore.selectedCustomer?.name || 'Member' }}</span>
      </button>

      <button
        @click="emit('voucher')"
        class="flex items-center gap-1.5 bg-slate-100 hover:bg-slate-200 text-slate-800 px-3 py-2 rounded-xl text-xs font-bold border border-slate-300 shadow-xs transition active:scale-95 whitespace-nowrap cursor-pointer"
        title="Voucher Promo"
      >
        <Icon icon="streamline-ultimate-color:ticket-1" class="w-4 h-4" />
        <span>{{ cartStore.voucherCode ? cartStore.voucherCode : 'Voucher' }}</span>
      </button>

      <button
        @click="emit('open-drawer')"
        class="flex items-center gap-1.5 bg-slate-100 hover:bg-slate-200 text-slate-800 px-3 py-2 rounded-xl text-xs font-bold border border-slate-300 shadow-xs transition active:scale-95 whitespace-nowrap cursor-pointer"
        title="Buka Cash Drawer (F4)"
      >
        <Icon icon="streamline-ultimate-color:office-drawer" class="w-4 h-4" />
        <span>Laci</span>
        <kbd class="bg-white border border-slate-300 px-1.5 py-0.5 rounded text-[10px] text-slate-700 font-mono shadow-xs">F4</kbd>
      </button>

      <button
        @click="emit('hold')"
        class="flex items-center gap-1.5 bg-slate-100 hover:bg-slate-200 text-slate-800 px-3 py-2 rounded-xl text-xs font-bold border border-slate-300 shadow-xs transition active:scale-95 whitespace-nowrap cursor-pointer"
        title="Tahan Transaksi (F6)"
      >
        <Icon icon="streamline-ultimate-color:controls-pause" class="w-4 h-4" />
        <span>Tahan</span>
        <kbd class="bg-white border border-slate-300 px-1.5 py-0.5 rounded text-[10px] text-slate-700 font-mono shadow-xs">F6</kbd>
      </button>

      <button
        @click="emit('recall')"
        class="flex items-center gap-1.5 bg-slate-100 hover:bg-slate-200 text-slate-800 px-3 py-2 rounded-xl text-xs font-bold border border-slate-300 shadow-xs transition active:scale-95 whitespace-nowrap relative cursor-pointer"
        title="Panggil Transaksi Tertunda (F7)"
      >
        <Icon icon="streamline-ultimate-color:button-play-1" class="w-4 h-4" />
        <span>Panggil</span>
        <span
          v-if="cartStore.heldOrders.length > 0"
          class="bg-rose-500 text-white text-[10px] font-bold w-4 h-4 rounded-full flex items-center justify-center -top-1 -right-1 absolute animate-pulse"
        >
          {{ cartStore.heldOrders.length }}
        </span>
        <kbd class="bg-white border border-slate-300 px-1.5 py-0.5 rounded text-[10px] text-slate-700 font-mono shadow-xs">F7</kbd>
      </button>

      <button
        @click="emit('checkout')"
        :disabled="cartStore.items.length === 0"
        class="flex items-center gap-1.5 bg-emerald-600 hover:bg-emerald-700 disabled:opacity-50 disabled:pointer-events-none text-white px-4 py-2 rounded-xl text-xs font-bold border border-emerald-700 shadow-xs transition active:scale-95 whitespace-nowrap cursor-pointer"
        title="Bayar / Checkout (F12)"
      >
        <Icon icon="streamline-ultimate-color:business-deal-cash-2" class="w-4 h-4" />
        <span>BAYAR</span>
        <kbd class="bg-emerald-800 px-1.5 py-0.5 rounded text-[10px] text-emerald-100 font-mono">F12</kbd>
      </button>
    </div>
  </div>
</template>
