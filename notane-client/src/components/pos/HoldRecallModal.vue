<script setup lang="ts">
import { X, PlayCircle, Clock, ShoppingBag } from 'lucide-vue-next'
import { useCartStore } from '@/stores/cartStore'

defineProps<{
  show: boolean
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

const cartStore = useCartStore()

function handleRecall(orderId: string) {
  cartStore.recallOrder(orderId)
  emit('close')
}

function formatCurrency(val: number): string {
  return new Intl.NumberFormat('id-ID', {
    style: 'currency',
    currency: 'IDR',
    minimumFractionDigits: 0,
    maximumFractionDigits: 0,
  }).format(val)
}
</script>

<template>
  <div
    v-if="show"
    class="fixed inset-0 z-50 bg-slate-900/50 backdrop-blur-xs flex items-center justify-center p-4 select-none animate-fadeIn"
  >
    <div class="bg-white border border-slate-300 w-full max-w-lg rounded-2xl shadow-2xl overflow-hidden flex flex-col text-slate-900">
      <!-- Header -->
      <div class="bg-slate-100 p-4 px-6 border-b border-slate-300 flex items-center justify-between">
        <div class="flex items-center gap-2 text-teal-700">
          <Clock class="w-5 h-5" />
          <h3 class="font-bold text-sm text-slate-900">Daftar Transaksi Tertunda (Hold & Recall)</h3>
        </div>
        <button @click="emit('close')" class="text-slate-400 hover:text-slate-700 p-1">
          <X class="w-4 h-4" />
        </button>
      </div>

      <!-- List -->
      <div class="p-6 overflow-y-auto max-h-[60vh] space-y-3">
        <div
          v-for="order in cartStore.heldOrders"
          :key="order.id"
          class="bg-slate-50 border border-slate-300 hover:border-teal-600 p-4 rounded-xl flex items-center justify-between transition group shadow-xs"
        >
          <div class="space-y-1">
            <div class="flex items-center gap-2">
              <span class="font-bold text-slate-900 text-sm">{{ order.customer_name }}</span>
              <span class="text-[11px] text-slate-500 font-mono">({{ order.timestamp }})</span>
            </div>
            <div class="text-xs text-slate-600 flex items-center gap-2">
              <ShoppingBag class="w-3.5 h-3.5 text-slate-400" />
              <span>{{ order.items.length }} Item</span>
              <span>•</span>
              <span class="font-mono text-emerald-700 font-bold">{{ formatCurrency(order.total_amount) }}</span>
            </div>
          </div>

          <button
            @click="handleRecall(order.id)"
            class="bg-teal-600 hover:bg-teal-700 text-white px-3.5 py-2 rounded-lg text-xs font-bold flex items-center gap-1.5 transition active:scale-95 shadow-xs border border-teal-700"
          >
            <PlayCircle class="w-4 h-4" />
            <span>Panggil</span>
          </button>
        </div>

        <div v-if="cartStore.heldOrders.length === 0" class="py-12 text-center text-slate-400 text-xs font-medium">
          Tidak ada transaksi yang sedang ditahan.
        </div>
      </div>
    </div>
  </div>
</template>
