<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from 'vue'
import { Search, X, Package, Plus } from 'lucide-vue-next'
import { api } from '@/services/api'
import { useCartStore } from '@/stores/cartStore'

const props = defineProps<{
  show: boolean
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

const cartStore = useCartStore()
const searchQuery = ref<string>('')
const products = ref<any[]>([])
const isLoading = ref<boolean>(false)
const searchInputRef = ref<HTMLInputElement | null>(null)

async function fetchProducts() {
  isLoading.value = true
  try {
    const res = await api.get('/products', {
      params: { search: searchQuery.value || undefined },
    })
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
      fetchProducts()
      setTimeout(() => searchInputRef.value?.focus(), 50)
    }
  }
)

async function handleSelectProduct(prod: any) {
  try {
    await cartStore.addProduct(prod, 1)
    emit('close')
  } catch (err) {
    console.error(err)
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
            placeholder="Ketik nama barang, SKU, atau barcode... (Enter)"
            class="w-full bg-white border border-slate-300 rounded-xl pl-10 pr-4 py-2.5 text-sm text-slate-900 focus:outline-none focus:border-emerald-600 font-sans shadow-xs"
            @keyup.enter="fetchProducts"
          />
        </div>
        <button @click="emit('close')" class="text-slate-400 hover:text-slate-700 p-1.5 rounded-lg hover:bg-slate-200">
          <X class="w-5 h-5" />
        </button>
      </div>

      <!-- Results List -->
      <div class="flex-1 overflow-y-auto p-4 space-y-2 bg-slate-50/50">
        <div
          v-for="prod in products"
          :key="prod.id"
          @click="handleSelectProduct(prod)"
          class="bg-white border border-slate-300 hover:border-emerald-600 p-3 rounded-xl flex items-center justify-between cursor-pointer transition group shadow-xs"
        >
          <div class="flex items-center gap-3">
            <div class="w-10 h-10 rounded-lg bg-slate-100 border border-slate-200 flex items-center justify-center text-slate-500 group-hover:text-emerald-700">
              <Package class="w-5 h-5" />
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
                <span class="text-amber-700 font-bold">Stok: {{ prod.stock_qty }} {{ prod.unit_code }}</span>
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
            <button class="bg-emerald-50 text-emerald-700 group-hover:bg-emerald-600 group-hover:text-white p-2 rounded-lg transition active:scale-95 border border-emerald-300">
              <Plus class="w-4 h-4" />
            </button>
          </div>
        </div>

        <div v-if="products.length === 0 && !isLoading" class="py-16 text-center text-slate-400 text-xs font-medium">
          Produk tidak ditemukan.
        </div>
      </div>
    </div>
  </div>
</template>
