<script setup lang="ts">
import { ref } from 'vue'
import { Trash2, Scale, Percent, AlertTriangle, Zap, Gift, Tag } from 'lucide-vue-next'
import { useCartStore } from '@/stores/cartStore'
import { soundService } from '@/services/soundService'
import { formatCurrency } from '@/utils/formatters'
import type { CartItem } from '@/types'

const emit = defineEmits<{
  'discount-item': [CartItem, number]
}>()

const cartStore = useCartStore()

const giftToDismiss = ref<CartItem | null>(null)
const showDismissModal = ref(false)

function handleRemoveItem(item: CartItem, index: number) {
  if (item.is_auto_gift) {
    giftToDismiss.value = item
    showDismissModal.value = true
  } else {
    cartStore.removeItem(index)
  }
}

function confirmDismissGift() {
  if (giftToDismiss.value) {
    const item = giftToDismiss.value
    cartStore.dismissGift(
      item.auto_gift_promo_rule_id || item.product.id,
      item.promo_id || '',
      item.promo_name || 'Promo Gratis',
      item.product.id,
      item.product.name
    )
  }
  showDismissModal.value = false
  giftToDismiss.value = null
}

function handleIncreaseQuantity(index: number, item: any) {
  const maxAllowed = cartStore.getMaxAllowedQuantity(item)
  if (item.quantity >= maxAllowed) {
    soundService.playOutOfStock()
    return
  }
  cartStore.updateQuantity(index, item.quantity + 1)
}

function handleQuantityChange(index: number, val: number) {
  const item = cartStore.items[index]
  if (!item) return
  const maxAllowed = cartStore.getMaxAllowedQuantity(item)
  if (val > maxAllowed) {
    soundService.playOutOfStock()
    cartStore.updateQuantity(index, maxAllowed)
    alert(`Jumlah '${item.product.name}' tidak dapat melebihi stok yang tersedia (${maxAllowed} ${item.selected_unit.unit_code}). Otomatis disesuaikan ke jumlah maksimal.`)
  } else {
    cartStore.updateQuantity(index, val)
  }
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
          :class="item.is_auto_gift ? 'bg-emerald-50/80 border-l-4 border-l-emerald-500' : (index % 2 === 0 ? 'bg-white' : 'bg-slate-50/60')"
        >
          <!-- Row Number -->
          <td class="py-3 px-3 text-center text-slate-400 font-sans text-xs">
            {{ cartStore.items.length - index }}
          </td>

          <!-- Product Info -->
          <td class="py-3 px-3 font-sans">
            <div class="flex items-center gap-2">
              <span
                v-if="item.is_flash_sale"
                class="bg-amber-600 text-white px-2 py-0.5 rounded text-[10px] font-mono flex items-center gap-1 font-bold shadow-2xs"
                title="Produk sedang masuk program Flash Sale"
              >
                <Zap class="w-3 h-3" /> Flash Sale
              </span>
              <span
                v-if="item.product.is_weight_embedded"
                class="bg-amber-100 text-amber-900 border border-amber-300 px-1.5 py-0.5 rounded text-[10px] font-mono flex items-center gap-1 font-bold"
                title="Produk Timbangan"
              >
                <Scale class="w-3 h-3" /> Timbang
              </span>
              <span
                v-if="item.is_auto_gift"
                class="bg-emerald-600 text-white px-2 py-0.5 rounded text-[10px] font-mono flex items-center gap-1 font-bold shadow-2xs"
                :title="item.promo_name || 'Hadiah Promo Otomatis'"
              >
                <Gift class="w-3 h-3" /> Hadiah Promo
              </span>
              <span
                v-else-if="item.is_free_item || (item.promo_discount || 0) > 0"
                class="bg-emerald-600 text-white px-2 py-0.5 rounded text-[10px] font-mono flex items-center gap-1 font-bold shadow-2xs"
                :title="item.promo_name || 'Program Promosi Ritel'"
              >
                <Gift v-if="item.is_free_item" class="w-3 h-3" />
                <Tag v-else class="w-3 h-3" />
                <span>{{ item.is_free_item ? 'Gratis' : 'Promo' }}</span>
              </span>
              <span class="font-bold text-slate-900 text-sm group-hover:text-emerald-800 transition">
                {{ item.product.name }}
              </span>
            </div>
            <div class="flex items-center gap-2 text-[11px] text-slate-500 mt-0.5 font-mono">
              <span>SKU: {{ item.product.sku }}</span>
              <span>•</span>
              <span>Barcode: {{ item.product.barcode }}</span>
              <span>•</span>
              <span
                class="font-bold"
                :class="item.quantity >= cartStore.getMaxAllowedQuantity(item) ? 'text-rose-600' : 'text-slate-600'"
              >
                Sisa Stok: {{ cartStore.getMaxAllowedQuantity(item) }} {{ item.selected_unit.unit_code }}
              </span>
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
            <div v-if="item.is_auto_gift" class="inline-flex items-center px-2.5 py-1 bg-emerald-100 border border-emerald-300 rounded-lg text-emerald-900 font-bold text-xs font-mono shadow-2xs" title="Kuantitas diatur otomatis oleh sistem promosi">
              <span>🎁 {{ item.quantity }} {{ item.selected_unit.unit_code }}</span>
            </div>
            <div v-else class="inline-flex items-center bg-white border border-slate-300 rounded-lg overflow-hidden shadow-xs">
              <button
                @click="cartStore.updateQuantity(index, item.quantity - 1)"
                class="px-2.5 py-1 text-slate-700 hover:text-slate-900 hover:bg-slate-100 transition active:scale-95 text-sm font-bold"
              >
                -
              </button>
              <input
                type="number"
                :value="item.quantity"
                :min="1"
                :max="cartStore.getMaxAllowedQuantity(item)"
                :step="item.product.is_weight_embedded ? '0.001' : '1'"
                class="w-16 bg-transparent text-center text-slate-900 font-bold text-sm focus:outline-none"
                @change="(e: any) => handleQuantityChange(index, parseFloat(e.target.value) || 1)"
              />
              <button
                @click="handleIncreaseQuantity(index, item)"
                :disabled="item.quantity >= cartStore.getMaxAllowedQuantity(item)"
                class="px-2.5 py-1 transition active:scale-95 text-sm font-bold"
                :class="item.quantity >= cartStore.getMaxAllowedQuantity(item) ? 'text-slate-300 bg-slate-100 cursor-not-allowed' : 'text-slate-700 hover:text-slate-900 hover:bg-slate-100'"
                :title="item.quantity >= cartStore.getMaxAllowedQuantity(item) ? 'Maksimal stok tercapai' : 'Tambah Kuantitas'"
              >
                +
              </button>
            </div>
            <div v-if="!item.is_auto_gift && item.quantity >= cartStore.getMaxAllowedQuantity(item)" class="text-[10px] text-rose-600 font-bold mt-0.5">
              Maksimal Stok
            </div>
          </td>

          <!-- Unit Price -->
          <td class="py-3 px-3 text-right font-bold">
            <span v-if="item.original_price && item.original_price > item.unit_price" class="line-through text-slate-400 text-[10px] block font-mono">
              {{ formatCurrency(item.original_price) }}
            </span>
            <span :class="item.is_flash_sale ? 'text-amber-700 font-extrabold' : 'text-slate-700'">
              {{ formatCurrency(item.unit_price) }}
            </span>
          </td>

          <!-- Item Discount -->
          <td class="py-3 px-3 text-right">
            <button
              @click="emit('discount-item', item, index)"
              class="inline-flex flex-col items-end gap-0.5 text-xs font-bold hover:underline"
              :class="item.discount_amount > 0 || (item.promo_discount || 0) > 0 || (item.flash_sale_discount || 0) > 0 ? 'text-amber-800' : 'text-slate-400'"
            >
              <div class="flex items-center gap-1">
                <Percent class="w-3 h-3" />
                <span>{{ (item.discount_amount + (item.promo_discount || 0) + (item.flash_sale_discount || 0) + (item.member_discount || 0)) > 0 ? formatCurrency(item.discount_amount + (item.promo_discount || 0) + (item.flash_sale_discount || 0) + (item.member_discount || 0)) : '-' }}</span>
              </div>
              <span v-if="(item.promo_discount || 0) > 0" class="text-[9px] text-emerald-700 font-mono font-bold">
                🎁 Promo -{{ formatCurrency(item.promo_discount || 0) }}
              </span>
              <span v-if="(item.flash_sale_discount || 0) > 0" class="text-[9px] text-amber-600 font-mono">
                ⚡ Flash -{{ formatCurrency(item.flash_sale_discount || 0) }}
              </span>
              <span v-if="(item.member_discount || 0) > 0" class="text-[9px] text-purple-600 font-mono">
                ★ Mbr -{{ formatCurrency(item.member_discount || 0) }}
              </span>
            </button>
          </td>

          <!-- Subtotal -->
          <td class="py-3 px-3 text-right text-emerald-700 font-extrabold text-sm">
            {{ formatCurrency(item.subtotal) }}
          </td>

          <!-- Action (Hapus Item / Lepas Hadiah) -->
          <td class="py-3 px-3 text-center">
            <button
              @click="handleRemoveItem(item, index)"
              class="text-slate-400 hover:text-rose-600 p-1.5 rounded-lg hover:bg-rose-50 transition active:scale-90 cursor-pointer"
              :title="item.is_auto_gift ? 'Lepas Hadiah Promo dari Keranjang' : 'Hapus Barang dari Keranjang'"
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

    <!-- Modal Konfirmasi Pelepasan Hadiah Promo -->
    <div
      v-if="showDismissModal && giftToDismiss"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-xs p-4 select-none font-sans"
    >
      <div class="bg-white rounded-2xl max-w-md w-full shadow-2xl overflow-hidden border border-slate-200 animate-in fade-in zoom-in-95 duration-150">
        <div class="p-6">
          <div class="flex items-center gap-3 text-amber-600 mb-3">
            <div class="w-10 h-10 rounded-full bg-amber-100 flex items-center justify-center shrink-0">
              <AlertTriangle class="w-5 h-5" />
            </div>
            <div>
              <h3 class="text-base font-black text-slate-900 leading-tight">Lepas Hadiah Promo?</h3>
              <p class="text-xs text-slate-500 font-mono mt-0.5">{{ giftToDismiss.promo_name || 'Program Promo Ritel' }}</p>
            </div>
          </div>
          
          <div class="bg-amber-50/70 border border-amber-200 rounded-xl p-3 text-xs text-amber-900 space-y-2">
            <p>
              Produk <strong class="font-bold">"{{ giftToDismiss.product.name }}"</strong> merupakan hadiah gratis otomatis (100% Gratis Rp 0).
            </p>
            <p class="text-[11px] text-amber-800">
              Apakah Anda ingin melepas hadiah ini dari transaksi saat ini?
              <br/>
              <span class="text-slate-500 italic">• Biasa dilakukan jika pelanggan menolak hadiah gratis atau fisik barang tidak tersedia di rak toko.</span>
            </p>
          </div>
        </div>

        <div class="bg-slate-50 px-6 py-3.5 border-t border-slate-200 flex items-center justify-end gap-2.5">
          <button
            @click="showDismissModal = false; giftToDismiss = null"
            class="px-4 py-2 rounded-xl text-xs font-bold text-slate-700 hover:bg-slate-200 transition cursor-pointer"
          >
            Batal (Tetap Ambil Hadiah)
          </button>
          <button
            @click="confirmDismissGift"
            class="px-4 py-2 rounded-xl text-xs font-extrabold bg-rose-600 hover:bg-rose-700 text-white shadow-xs transition active:scale-95 cursor-pointer"
          >
            Lepas Hadiah Promo
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
