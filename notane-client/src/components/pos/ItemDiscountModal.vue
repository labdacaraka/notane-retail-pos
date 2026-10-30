<script setup lang="ts">
import { ref, computed, watch, nextTick } from 'vue'
import { Percent, Tag, X, Check, Zap } from 'lucide-vue-next'
import { formatCurrency, formatQty } from '@/utils/formatters'
import type { CartItem } from '@/types'

const props = defineProps<{
  show: boolean
  item: CartItem | null
  index: number
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'save', payload: { index: number; discountAmount: number; discountPercent: number }): void
}>()

const inputRef = ref<HTMLInputElement | null>(null)
const discountMode = ref<'nominal' | 'percent'>('nominal')
const inputValue = ref<number>(0)

watch(
  () => props.show,
  (isOpen) => {
    if (isOpen && props.item) {
      if (props.item.discount_percent > 0) {
        discountMode.value = 'percent'
        inputValue.value = props.item.discount_percent
      } else {
        discountMode.value = 'nominal'
        inputValue.value = props.item.discount_amount || 0
      }
      nextTick(() => {
        setTimeout(() => {
          inputRef.value?.focus()
          inputRef.value?.select()
        }, 50)
      })
    }
  },
  { immediate: true }
)

const itemGross = computed(() => {
  if (!props.item) return 0
  return props.item.unit_price * props.item.quantity
})

const calculatedDiscountAmount = computed(() => {
  if (!props.item) return 0
  const gross = itemGross.value
  if (discountMode.value === 'percent') {
    const pct = Math.min(100, Math.max(0, inputValue.value || 0))
    return Math.round((gross * pct) / 100)
  }
  return Math.min(gross, Math.max(0, inputValue.value || 0))
})

const finalSubtotalPreview = computed(() => {
  if (!props.item) return 0
  const taxable = Math.max(0, itemGross.value - calculatedDiscountAmount.value)
  const tax = (taxable * (props.item.product.tax_percent || 0)) / 100
  return taxable + tax
})

function handleSave() {
  if (!props.item) return
  const amount = calculatedDiscountAmount.value
  const percent = discountMode.value === 'percent' ? inputValue.value : 0
  emit('save', {
    index: props.index,
    discountAmount: amount,
    discountPercent: percent,
  })
  emit('close')
}

function handleReset() {
  inputValue.value = 0
  handleSave()
}

function handleKeyDown(e: KeyboardEvent) {
  if (!props.show) return
  if (e.key === 'Escape') {
    e.preventDefault()
    e.stopPropagation()
    emit('close')
  } else if (e.key === 'Enter') {
    e.preventDefault()
    e.stopPropagation()
    handleSave()
  }
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="show && item"
      class="fixed inset-0 z-[9999] flex items-center justify-center p-4 bg-slate-900/60 backdrop-blur-xs select-none animate-in fade-in duration-150"
      @keydown="handleKeyDown"
      @click.self="emit('close')"
    >
      <div
        class="bg-white border-2 border-slate-300 shadow-2xl rounded-2xl w-full max-w-md overflow-hidden flex flex-col font-sans"
        role="dialog"
        aria-modal="true"
      >
        <!-- Modal Header -->
        <div class="px-5 py-3.5 bg-amber-700 flex items-center justify-between text-white shadow-xs">
          <div class="flex items-center gap-2 font-bold text-sm">
            <Tag class="w-4 h-4 text-amber-200" />
            <span>Atur Diskon Kasir / Item</span>
          </div>
          <button
            type="button"
            @click="emit('close')"
            class="text-white/80 hover:text-white hover:bg-white/20 p-1 rounded-lg transition cursor-pointer"
            title="Tutup (Esc)"
          >
            <X class="w-4 h-4" />
          </button>
        </div>

        <!-- Modal Body -->
        <div class="p-5 space-y-4 bg-slate-50">
          <!-- Product Information Card -->
          <div class="bg-white p-3.5 rounded-xl border border-slate-200 shadow-2xs space-y-2">
            <div class="flex items-start justify-between gap-2">
              <div>
                <h4 class="font-bold text-slate-900 text-sm leading-snug">{{ item.product.name }}</h4>
                <div class="text-[11px] text-slate-500 font-mono mt-0.5">
                  SKU: {{ item.product.sku }} • {{ formatQty(item.quantity) }} {{ item.selected_unit.unit_code }} @ {{ formatCurrency(item.unit_price) }}
                </div>
              </div>
              <span class="text-xs font-bold font-mono text-slate-700 bg-slate-100 px-2 py-1 rounded-md shrink-0">
                Gross: {{ formatCurrency(itemGross) }}
              </span>
            </div>

            <!-- Flash Sale Program Info Alert -->
            <div
              v-if="item.is_flash_sale || (item.flash_sale_discount || 0) > 0"
              class="bg-amber-50 border border-amber-300 rounded-lg p-2.5 flex items-start gap-2 text-xs"
            >
              <Zap class="w-4 h-4 text-amber-600 shrink-0 mt-0.5" />
              <div class="flex-1 min-w-0">
                <div class="font-bold text-amber-900 flex items-center gap-1.5">
                  <span>Program Flash Sale Aktif</span>
                  <span class="bg-amber-600 text-white text-[10px] px-1.5 py-0.2 rounded font-mono font-bold">
                    -{{ formatCurrency(item.flash_sale_discount || 0) }}
                  </span>
                </div>
                <div class="text-[11px] text-amber-800 mt-0.5">
                  {{ item.flash_sale_name || 'Promo Flash Sale' }} sudah otomatis memotong harga dasar barang ini.
                </div>
              </div>
            </div>

            <!-- Member Discount Info Alert -->
            <div
              v-if="(item.member_discount || 0) > 0"
              class="bg-purple-50 border border-purple-200 rounded-lg p-2.5 flex items-start gap-2 text-xs"
            >
              <span class="text-purple-600 font-bold">★</span>
              <div class="flex-1 min-w-0">
                <div class="font-bold text-purple-900 flex items-center gap-1.5">
                  <span>Potongan Harga Member</span>
                  <span class="bg-purple-600 text-white text-[10px] px-1.5 py-0.2 rounded font-mono font-bold">
                    -{{ formatCurrency(item.member_discount || 0) }}
                  </span>
                </div>
              </div>
            </div>
          </div>

          <!-- Discount Mode Selector -->
          <div class="space-y-2">
            <label class="block text-xs font-bold text-slate-700">Tipe Diskon Tambahan Kasir:</label>
            <div class="grid grid-cols-2 gap-2 bg-slate-200/70 p-1 rounded-xl">
              <button
                type="button"
                @click="discountMode = 'nominal'"
                class="py-1.5 text-xs font-bold rounded-lg transition cursor-pointer flex items-center justify-center gap-1.5"
                :class="discountMode === 'nominal' ? 'bg-white text-slate-900 shadow-xs' : 'text-slate-600 hover:text-slate-900'"
              >
                <span>Nominal (Rp)</span>
              </button>
              <button
                type="button"
                @click="discountMode = 'percent'"
                class="py-1.5 text-xs font-bold rounded-lg transition cursor-pointer flex items-center justify-center gap-1.5"
                :class="discountMode === 'percent' ? 'bg-white text-slate-900 shadow-xs' : 'text-slate-600 hover:text-slate-900'"
              >
                <Percent class="w-3.5 h-3.5" />
                <span>Persentase (%)</span>
              </button>
            </div>
          </div>

          <!-- Discount Input Box -->
          <div class="space-y-1.5">
            <div class="relative">
              <span
                class="absolute left-3.5 top-1/2 -translate-y-1/2 text-sm font-bold font-mono"
                :class="discountMode === 'nominal' ? 'text-slate-500' : 'text-purple-600'"
              >
                {{ discountMode === 'nominal' ? 'Rp' : '%' }}
              </span>
              <input
                ref="inputRef"
                type="number"
                v-model.number="inputValue"
                :min="0"
                :max="discountMode === 'percent' ? 100 : itemGross"
                class="w-full pl-11 pr-4 py-2.5 bg-white border-2 border-slate-300 focus:border-amber-600 rounded-xl text-lg font-black font-mono text-slate-900 focus:outline-none transition shadow-2xs"
                placeholder="0"
                @keyup.enter="handleSave"
              />
            </div>

            <!-- Quick Preset Chips -->
            <div class="flex flex-wrap gap-1.5 pt-1">
              <template v-if="discountMode === 'percent'">
                <button
                  v-for="p in [5, 10, 15, 20, 50]"
                  :key="p"
                  type="button"
                  @click="inputValue = p"
                  class="px-2.5 py-1 text-xs font-bold bg-white border border-slate-300 hover:border-amber-500 hover:bg-amber-50 text-slate-700 rounded-lg transition cursor-pointer"
                >
                  {{ p }}%
                </button>
              </template>
              <template v-else>
                <button
                  v-for="nom in [1000, 2000, 5000, 10000]"
                  :key="nom"
                  type="button"
                  @click="inputValue = nom"
                  class="px-2 py-1 text-[11px] font-bold bg-white border border-slate-300 hover:border-amber-500 hover:bg-amber-50 text-slate-700 rounded-lg transition cursor-pointer"
                >
                  {{ formatCurrency(nom) }}
                </button>
              </template>
              <button
                type="button"
                @click="inputValue = 0"
                class="px-2.5 py-1 text-xs font-bold bg-slate-100 hover:bg-slate-200 border border-slate-300 text-slate-600 rounded-lg transition cursor-pointer ml-auto"
              >
                Reset (0)
              </button>
            </div>
          </div>

          <!-- Live Preview Summary -->
          <div class="bg-amber-50/60 border border-amber-200/80 rounded-xl p-3 text-xs space-y-1.5 font-mono">
            <div class="flex justify-between text-slate-600">
              <span>Potongan Tambahan Kasir:</span>
              <span class="font-bold text-amber-700">-{{ formatCurrency(calculatedDiscountAmount) }}</span>
            </div>
            <div class="flex justify-between text-slate-900 font-bold pt-1 border-t border-amber-200 text-sm">
              <span>Subtotal Item Akhir:</span>
              <span class="text-emerald-700">{{ formatCurrency(finalSubtotalPreview) }}</span>
            </div>
          </div>
        </div>

        <!-- Modal Footer Actions -->
        <div class="px-5 py-3.5 bg-white border-t border-slate-200 flex items-center justify-between">
          <button
            type="button"
            @click="handleReset"
            class="text-xs font-bold text-rose-600 hover:text-rose-700 hover:underline cursor-pointer"
          >
            Hapus Diskon Kasir
          </button>
          <div class="flex items-center gap-2">
            <button
              type="button"
              @click="emit('close')"
              class="px-3.5 py-2 text-xs font-bold text-slate-600 hover:bg-slate-100 border border-slate-300 rounded-xl transition cursor-pointer"
            >
              Batal (Esc)
            </button>
            <button
              type="button"
              @click="handleSave"
              class="px-4 py-2 text-xs font-bold text-white bg-amber-600 hover:bg-amber-700 border border-amber-700 rounded-xl shadow-xs transition flex items-center gap-1.5 cursor-pointer active:scale-95"
            >
              <Check class="w-3.5 h-3.5" />
              <span>Terapkan (Enter)</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>
