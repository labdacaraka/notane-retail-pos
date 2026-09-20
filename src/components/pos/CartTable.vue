<script setup lang="ts">
import { Trash2, Scale, Percent } from 'lucide-vue-next'
import { useCartStore } from '@/stores/cartStore'
import type { CartItem } from '@/types'

const emit = defineEmits<{
  (e: 'discount-item', item: CartItem, index: number): void
}>()

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
  <div class="flex-1 overflow-y-auto bg-slate-100 select-none">
    <table class="w-full text-left text-xs border-collapse">
      <!-- Table Header (Sticky) -->
      <thead class="bg-slate-200/90 backdrop-blur sticky top-0 z-10 border-b border-slate-300 text-slate-700 uppercase tracking-wider font-mono text-[11px] font-bold">
        <tr>
          <th class="py-3 px-3 w-12 text-center">#</th>
          <th class="py-3 px-3">Produk & Barcode</th>
          <th class="py-3 px-3 w-28 text-center">Satuan</th>
          <th class="py-3 px-3 w-36 text-center">Kuantitas</th>
          <th class="py-3 px-3 w-32 text-right">Harga Satuan</th>
          <th class="py-3 px-3 w-28 text-right">Diskon</th>
          <th class="py-3 px-3 w-36 text-right">Subtotal</th>
          <th class="py-3 px-3 w-16 text-center">Aksi</th>
        </tr>
      </thead>

      <!-- Table Body -->
      <tbody class="divide-y divide-slate-200 font-mono">
        <tr
          v-for="(item, index) in cartStore.items"
          :key="item.id"
          class="hover:bg-emerald-50/50 transition-colors group"
          :class="index % 2 === 0 ? 'bg-white' : 'bg-slate-50/60'"
        >
          <!-- Row Number -->
          <td class="py-3 px-3 text-center text-slate-400 font-sans text-xs">
            {{ cartStore.items.length - index }}
          </td>

          <!-- Product Info -->
          <td class="py-3 px-3 font-sans">
            <div class="flex items-center gap-2">
              <span
                v-if="item.product.is_weight_embedded"
                class="bg-amber-100 text-amber-900 border border-amber-300 px-1.5 py-0.5 rounded text-[10px] font-mono flex items-center gap-1 font-bold"
                title="Produk Timbangan"
              >
                <Scale class="w-3 h-3" /> Timbang
              </span>
              <span class="font-bold text-slate-900 text-sm group-hover:text-emerald-800 transition">
                {{ item.product.name }}
              </span>
            </div>
            <div class="flex items-center gap-2 text-[11px] text-slate-500 mt-0.5 font-mono">
              <span>SKU: {{ item.product.sku }}</span>
              <span>•</span>
              <span>Barcode: {{ item.product.barcode }}</span>
              <span v-if="item.product.tax_percent > 0" class="text-sky-700 font-bold">
                (PPN {{ item.product.tax_percent }}%)
              </span>
            </div>
          </td>

          <!-- Multi-Unit Selector -->
          <td class="py-3 px-3 text-center font-sans">
            <span class="inline-block bg-slate-100 border border-slate-300 text-slate-800 text-xs px-2.5 py-1 rounded-md font-bold font-mono">
              {{ item.selected_unit.unit_code }}
            </span>
          </td>

          <!-- Quantity Stepper -->
          <td class="py-3 px-3 text-center">
            <div class="inline-flex items-center bg-white border border-slate-300 rounded-lg overflow-hidden shadow-xs">
              <button
                @click="cartStore.updateQuantity(index, item.quantity - 1)"
                class="px-2.5 py-1 text-slate-700 hover:text-slate-900 hover:bg-slate-100 transition active:scale-95 text-sm font-bold"
              >
                -
              </button>
              <input
                type="number"
                :value="item.quantity"
                :step="item.product.is_weight_embedded ? '0.001' : '1'"
                class="w-16 bg-transparent text-center text-slate-900 font-bold text-sm focus:outline-none"
                @change="(e: any) => cartStore.updateQuantity(index, parseFloat(e.target.value) || 1)"
              />
              <button
                @click="cartStore.updateQuantity(index, item.quantity + 1)"
                class="px-2.5 py-1 text-slate-700 hover:text-slate-900 hover:bg-slate-100 transition active:scale-95 text-sm font-bold"
              >
                +
              </button>
            </div>
          </td>

          <!-- Unit Price -->
          <td class="py-3 px-3 text-right text-slate-700 font-bold">
            {{ formatCurrency(item.unit_price) }}
          </td>

          <!-- Item Discount -->
          <td class="py-3 px-3 text-right">
            <button
              @click="emit('discount-item', item, index)"
              class="inline-flex items-center gap-1 text-xs text-amber-800 hover:text-amber-900 font-bold hover:underline"
            >
              <Percent class="w-3 h-3" />
              <span>{{ item.discount_amount > 0 ? formatCurrency(item.discount_amount) : '-' }}</span>
            </button>
          </td>

          <!-- Subtotal -->
          <td class="py-3 px-3 text-right text-emerald-700 font-extrabold text-sm">
            {{ formatCurrency(item.subtotal) }}
          </td>

          <!-- Action (Hapus Item Langsung) -->
          <td class="py-3 px-3 text-center">
            <button
              @click="cartStore.removeItem(index)"
              class="text-slate-400 hover:text-rose-600 p-1.5 rounded-lg hover:bg-rose-50 transition active:scale-90 cursor-pointer"
              title="Hapus Barang dari Keranjang"
            >
              <Trash2 class="w-4 h-4" />
            </button>
          </td>
        </tr>

        <!-- Empty Cart State -->
        <tr v-if="cartStore.items.length === 0">
          <td colspan="8" class="py-20 text-center text-slate-500 font-sans">
            <div class="flex flex-col items-center justify-center gap-2">
              <div class="w-16 h-16 rounded-full bg-slate-200 border border-slate-300 flex items-center justify-center text-3xl">
                🛒
              </div>
              <p class="text-base font-bold text-slate-800">Keranjang Belanja Masih Kosong</p>
              <p class="text-xs text-slate-500">Pindai barcode barang atau gunakan tombol Cari (F1) untuk memulai transaksi.</p>
            </div>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>
