<script setup lang="ts">
import { useCartStore } from '@/stores/cartStore'

const cartStore = useCartStore()

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
  <div class="h-screen w-screen bg-slate-100 text-slate-900 flex flex-col justify-between p-6 select-none">
    <!-- Top Header -->
    <div class="flex items-center justify-between border-b border-slate-300 pb-4">
      <div class="flex items-center gap-3">
        <img src="/logo.png" alt="Notane Logo" class="w-12 h-12 rounded-xl shadow-sm border border-slate-200 object-contain" />
        <div>
          <h1 class="text-xl font-black text-slate-900">NOTANE <span class="text-notane-gradient">RETAIL STORE</span></h1>
          <p class="text-xs text-slate-500 font-medium">Selamat Berbelanja • Transparansi Kasir</p>
        </div>
      </div>
      <div v-if="cartStore.selectedCustomer" class="text-right">
        <span class="text-xs text-slate-500">Pelanggan Member:</span>
        <div class="font-bold text-sky-700 text-sm">{{ cartStore.selectedCustomer.name }}</div>
      </div>
    </div>

    <!-- Center: Scanned Items Display -->
    <div class="flex-1 overflow-y-auto py-4 space-y-3">
      <div
        v-for="item in cartStore.items.slice(0, 5)"
        :key="item.id"
        class="bg-white border border-slate-300 p-4 rounded-2xl flex items-center justify-between shadow-xs animate-fadeIn"
      >
        <div>
          <h3 class="text-lg font-bold text-slate-900">{{ item.product.name }}</h3>
          <p class="text-sm text-slate-500 font-mono">
            {{ item.quantity }} {{ item.selected_unit.unit_code }} × {{ formatCurrency(item.unit_price) }}
          </p>
        </div>
        <div class="text-2xl font-bold font-mono text-emerald-700">
          {{ formatCurrency(item.subtotal) }}
        </div>
      </div>

      <div v-if="cartStore.items.length === 0" class="h-full flex flex-col items-center justify-center text-center space-y-3 py-16">
        <div class="text-6xl">🛍️</div>
        <h2 class="text-2xl font-bold text-slate-800">Selamat Datang di Notane Retail</h2>
        <p class="text-sm text-slate-500">Silakan letakkan barang belanjaan Anda di meja kasir.</p>
      </div>
    </div>

    <!-- Bottom Giant Total Footer -->
    <div class="bg-white border border-slate-300 rounded-3xl p-6 flex items-center justify-between shadow-md">
      <div>
        <span class="text-sm font-bold text-slate-500 uppercase tracking-wider">Total Pembayaran</span>
        <div class="text-xs text-slate-400 mt-0.5 font-medium">{{ cartStore.items.length }} Item Terpindai</div>
      </div>
      <div class="text-5xl lg:text-6xl font-black text-emerald-700 font-mono">
        {{ formatCurrency(cartStore.grandTotal) }}
      </div>
    </div>
  </div>
</template>
