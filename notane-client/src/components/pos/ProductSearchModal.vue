<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from 'vue'
import { Search, X, Package, Plus, AlertCircle } from 'lucide-vue-next'
import { api } from '@/services/api'
import { useCartStore } from '@/stores/cartStore'
import { useShiftStore } from '@/stores/shiftStore'
import { soundService } from '@/services/soundService'

const props = defineProps<{
  show: boolean
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

const cartStore = useCartStore()
const shiftStore = useShiftStore()
const searchQuery = ref<string>('')
const products = ref<any[]>([])
const isLoading = ref<boolean>(false)
const searchInputRef = ref<HTMLInputElement | null>(null)

async function fetchProducts() {
  if (!shiftStore.isShiftOpen || !cartStore.currentOutletId) {
    products.value = []
    return
  }

  isLoading.value = true
  try {
    const params: any = { search: searchQuery.value || undefined, limit: 25 }

    const res = await api.get('/products', { params })
    if (res.data.success) {
      products.value = res.data.data
    }
  } catch (err) {
    console.error(err)
  } finally {
    isLoading.value = false
  }
}

watch(
  () => props.show,
  (val) => {
    if (val) {
      searchQuery.value = ''
      if (shiftStore.isShiftOpen) {
        fetchProducts()
        setTimeout(() => searchInputRef.value?.focus(), 50)
      } else {
        products.value = []
      }
    }
  }
)

async function handleSelectProduct(prod: any) {
  if (!shiftStore.isShiftOpen) {
    alert('Shift kasir belum dibuka. Silakan buka shift kasir terlebih dahulu.')
    emit('close')
    shiftStore.openShiftModal('OPEN')
    return
  }

  if (Number(prod.stock_qty ?? 0) <= 0) {
    soundService.playOutOfStock()
    alert(`Stok produk '${prod.name}' habis (0 ${prod.unit_code}). Tidak dapat ditambahkan ke kasir.`)
    return
  }
  try {
    await cartStore.addProduct(prod, 1)
    emit('close')
  } catch (err: any) {
    alert(err.message || 'Gagal menambahkan produk')
  }
}

function handleKey(e: KeyboardEvent) {
  if (e.key === 'Escape' && props.show) {
    emit('close')
  }
}

onMounted(() => {
  window.addEventListener('keydown', handleKey)
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
    <div class="bg-white border border-slate-300 w-full max-w-2xl rounded-2xl shadow-2xl overflow-hidden flex flex-col max-h-[85vh] text-slate-900">
      <!-- Header with Search Input -->
      <div class="bg-slate-100 p-4 px-6 border-b border-slate-300 flex items-center gap-3">
        <div class="relative flex-1">
          <Search class="w-4 h-4 text-slate-400 absolute left-3.5 top-1/2 -translate-y-1/2" />
          <input
            ref="searchInputRef"
            v-model="searchQuery"
            type="text"
            :disabled="!shiftStore.isShiftOpen"
            :placeholder="!shiftStore.isShiftOpen ? 'Shift belum dibuka — Buka shift kasir terlebih dahulu' : 'Ketik nama barang, SKU, atau barcode... (Enter)'"
            class="w-full bg-white border border-slate-300 rounded-xl pl-10 pr-4 py-2.5 text-sm text-slate-900 focus:outline-none focus:border-emerald-600 font-sans shadow-xs disabled:bg-slate-100 disabled:text-slate-400 disabled:cursor-not-allowed"
            @keyup.enter="fetchProducts"
          />
        </div>
        <button @click="emit('close')" class="text-slate-400 hover:text-slate-700 p-1.5 rounded-lg hover:bg-slate-200">
          <X class="w-5 h-5" />
        </button>
      </div>

      <!-- Locked State if Shift Not Open -->
      <div v-if="!shiftStore.isShiftOpen" class="flex-1 flex flex-col items-center justify-center p-8 py-16 text-center bg-slate-50">
        <div class="w-16 h-16 rounded-2xl bg-amber-50 border border-amber-200 text-amber-600 flex items-center justify-center mb-4 shadow-xs">
          <AlertCircle class="w-8 h-8" />
        </div>
        <h3 class="text-base font-bold text-slate-800 mb-1">Shift Kasir Belum Dibuka</h3>
        <p class="text-xs text-slate-500 max-w-md mb-6 leading-relaxed">
          Katalog barang, harga, dan ketersediaan stok cabang toko tidak dapat ditampilkan sebelum shift kasir dibuka karena sistem belum menentukan cabang toko yang aktif.
        </p>
        <button
          @click="emit('close'); shiftStore.openShiftModal('OPEN')"
          class="px-5 py-2.5 bg-emerald-600 hover:bg-emerald-700 text-white text-xs font-bold rounded-xl transition shadow-sm flex items-center gap-2 cursor-pointer active:scale-95"
        >
          Buka Shift Kasir Sekarang
        </button>
      </div>

      <!-- Results List -->
      <div v-else class="flex-1 overflow-y-auto p-4 space-y-2 bg-slate-50/50">
        <div
          v-for="prod in products"
          :key="prod.id"
          @click="handleSelectProduct(prod)"
          class="border p-3 rounded-xl flex items-center justify-between transition group shadow-xs"
          :class="Number(prod.stock_qty ?? 0) <= 0 ? 'bg-slate-50/80 border-slate-200 opacity-60 cursor-not-allowed' : 'bg-white border-slate-300 hover:border-emerald-600 cursor-pointer'"
        >
          <div class="flex items-center gap-3">
            <div class="w-11 h-11 rounded-lg bg-slate-100 border border-slate-200 flex items-center justify-center text-slate-500 group-hover:text-emerald-700 overflow-hidden shrink-0 shadow-2xs">
              <img
                v-if="prod.image_url"
                :src="prod.image_url"
                :alt="prod.name"
                class="w-full h-full object-cover transition-transform group-hover:scale-105"
              />
              <Package v-else class="w-5 h-5" />
            </div>
            <div>
              <div class="font-bold text-slate-900 text-sm group-hover:text-emerald-800">
                {{ prod.name }}
              </div>
              <div class="flex items-center gap-2 text-xs text-slate-500 font-mono mt-0.5">
                <span>SKU: {{ prod.sku }}</span>
                <span>•</span>
                <span>Barcode: {{ prod.barcode }}</span>
                <span>•</span>
                <span
                  class="font-bold"
                  :class="Number(prod.stock_qty ?? 0) <= 0 ? 'text-rose-600 bg-rose-50 px-1.5 py-0.5 rounded border border-rose-200' : 'text-slate-700'"
                >
                  {{ Number(prod.stock_qty ?? 0) <= 0 ? 'Stok Habis (0)' : `Stok: ${prod.stock_qty} ${prod.unit_code}` }}
                </span>
              </div>
            </div>
          </div>

          <div class="flex items-center gap-3">
            <div class="text-right">
              <div class="font-bold text-emerald-700 text-sm font-mono">
                Rp {{ prod.base_selling_price.toLocaleString() }}
              </div>
              <div class="text-[10px] text-slate-500">per {{ prod.unit_code }}</div>
            </div>
            <button
              :disabled="Number(prod.stock_qty ?? 0) <= 0"
              class="p-2 rounded-lg transition active:scale-95 border"
              :class="Number(prod.stock_qty ?? 0) <= 0 ? 'bg-slate-100 text-slate-400 border-slate-200 cursor-not-allowed' : 'bg-emerald-50 text-emerald-700 group-hover:bg-emerald-600 group-hover:text-white border-emerald-300'"
            >
              <Plus class="w-4 h-4" />
            </button>
          </div>
        </div>

        <div v-if="products.length === 0 && !isLoading" class="py-16 text-center text-slate-400 text-xs font-medium">
          Produk tidak ditemukan untuk cabang toko ini.
        </div>
      </div>
    </div>
  </div>
</template>
