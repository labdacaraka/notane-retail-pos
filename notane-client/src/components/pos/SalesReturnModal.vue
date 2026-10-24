<script setup lang="ts">
import { ref, computed } from 'vue'
import { api } from '@/services/api'
import type { LookupSaleDetail, LookupSaleItem } from '@/types'
import { Icon } from '@iconify/vue'
import SupervisorAuthSection from '@/components/pos/SupervisorAuthSection.vue'
import { useLocationStore } from '@/stores/locationStore'

const locationStore = useLocationStore()

defineProps<{
  modelValue: boolean
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', val: boolean): void
  (e: 'return-completed', result: any): void
}>()

const invoiceInput = ref('')
const isLoadingLookup = ref(false)
const lookupError = ref('')
const saleData = ref<LookupSaleDetail | null>(null)

// Item selection
interface SelectedReturnItem {
  sale_item_id: string
  product_id: string
  product_name: string
  unit_code: string
  max_qty: number
  return_qty: number
  unit_price: number
  discount_per_unit: number
  net_price: number
  condition: 'GOOD' | 'DAMAGED'
  restock: boolean
}

const selectedItems = ref<Record<string, SelectedReturnItem>>({})
const refundMethod = ref<'CASH' | 'STORE_CREDIT'>('CASH')
const reasonCode = ref('RET_DEFECT')
const notes = ref('')

// Supervisor Auth
const supervisors = ref<any[]>([])
const selectedSupervisorId = ref<string>('')
const supervisorPin = ref('')
const supervisorAuthToken = ref('')
const isSubmitting = ref(false)
const submitError = ref('')
const submitSuccess = ref('')

async function lookupInvoice() {
  const inv = invoiceInput.value.trim()
  if (!inv) return

  isLoadingLookup.value = true
  lookupError.value = ''
  saleData.value = null
  selectedItems.value = {}

  try {
    const res = await api.get(`/sales/lookup/${encodeURIComponent(inv)}`)
    if (res.data?.success && res.data.data) {
      saleData.value = res.data.data
      await loadSupervisors()
    } else {
      lookupError.value = res.data?.message || 'Faktur penjualan tidak ditemukan'
    }
  } catch (err: any) {
    lookupError.value = err.response?.data?.message || 'Faktur penjualan tidak ditemukan atau terjadi kesalahan server'
  } finally {
    isLoadingLookup.value = false
  }
}

async function loadSupervisors() {
  try {
    const res = await api.get('/auth/supervisors')
    if (res.data?.success && Array.isArray(res.data.data)) {
      supervisors.value = res.data.data
      if (supervisors.value.length > 0) {
        selectedSupervisorId.value = supervisors.value[0].id
      }
    }
  } catch {
    // Fallback: supervisor list empty
  }
}

function toggleSelectItem(item: LookupSaleItem) {
  if (selectedItems.value[item.id]) {
    delete selectedItems.value[item.id]
  } else {
    const discPerUnit = item.quantity > 0 ? item.discount_amount / item.quantity : 0
    const netPrice = Math.max(0, item.unit_price - discPerUnit)
    selectedItems.value[item.id] = {
      sale_item_id: item.id,
      product_id: item.product_id,
      product_name: item.product_name,
      unit_code: item.unit_code,
      max_qty: item.remaining_returnable_qty,
      return_qty: Math.min(1, item.remaining_returnable_qty),
      unit_price: item.unit_price,
      discount_per_unit: discPerUnit,
      net_price: netPrice,
      condition: 'GOOD',
      restock: true,
    }
  }
}

const totalRefundAmount = computed(() => {
  return Object.values(selectedItems.value).reduce((sum, it) => {
    return sum + (it.return_qty * it.net_price)
  }, 0)
})

const hasSelectedItems = computed(() => {
  return Object.keys(selectedItems.value).length > 0
})

function formatRupiah(num: number): string {
  return new Intl.NumberFormat('id-ID', {
    style: 'currency',
    currency: 'IDR',
    maximumFractionDigits: 0,
  }).format(num)
}

async function handleSubmitReturn() {
  if (!saleData.value) return
  if (!hasSelectedItems.value) {
    submitError.value = 'Pilih minimal satu barang yang akan diretur'
    return
  }

  if (!supervisorPin.value.trim()) {
    submitError.value = 'PIN Supervisor wajib diisi untuk otorisasi retur penjualan'
    return
  }

  const itemsPayload = Object.values(selectedItems.value).map((it) => ({
    sale_item_id: it.sale_item_id,
    product_id: it.product_id,
    quantity: it.return_qty,
    condition: it.condition,
    restock: it.condition === 'GOOD' ? it.restock : false,
  }))

  isSubmitting.value = true
  submitError.value = ''
  submitSuccess.value = ''

  if (!supervisorPin.value.trim() && !supervisorAuthToken.value.trim()) {
    submitError.value = 'Otorisasi Supervisor (Biometrik atau PIN) wajib dilakukan'
    return
  }

  try {
    const res = await api.post('/sales/returns', {
      sale_id: saleData.value.id,
      refund_method: refundMethod.value,
      reason_code: reasonCode.value,
      notes: notes.value || undefined,
      supervisor_id: selectedSupervisorId.value || undefined,
      supervisor_pin: supervisorPin.value.trim() || undefined,
      supervisor_auth_token: supervisorAuthToken.value || undefined,
      items: itemsPayload,
    })

    if (res.data?.success) {
      submitSuccess.value = res.data.message || 'Retur penjualan berhasil diproses!'
      emit('return-completed', res.data.data)
      setTimeout(() => {
        closeModal()
      }, 1500)
    } else {
      submitError.value = res.data?.message || 'Gagal memproses retur'
    }
  } catch (err: any) {
    submitError.value = err.response?.data?.message || 'Gagal memproses retur. Periksa PIN Supervisor Anda.'
  } finally {
    isSubmitting.value = false
  }
}

function closeModal() {
  emit('update:modelValue', false)
  invoiceInput.value = ''
  saleData.value = null
  selectedItems.value = {}
  supervisorPin.value = ''
  submitError.value = ''
  submitSuccess.value = ''
  lookupError.value = ''
}
</script>

<template>
  <div v-if="modelValue" class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/60 backdrop-blur-sm animate-fade-in">
    <div class="bg-white rounded-2xl shadow-2xl border border-slate-200 w-full max-w-4xl max-h-[92vh] flex flex-col overflow-hidden">
      <!-- Header -->
      <div class="px-6 py-4 bg-gradient-to-r from-rose-600 via-rose-500 to-amber-600 text-white flex items-center justify-between">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-xl bg-white/20 flex items-center justify-center backdrop-blur-sm">
            <Icon icon="fluent:arrow-undo-24-filled" class="text-2xl" />
          </div>
          <div>
            <h2 class="text-lg font-black tracking-wide">Retur Penjualan Konsumen</h2>
            <p class="text-xs text-rose-100">Klaim barang cacat / tukar barang dengan persetujuan Supervisor</p>
          </div>
        </div>
        <button @click="closeModal" class="p-2 rounded-lg hover:bg-white/20 transition-colors text-white">
          <Icon icon="fluent:dismiss-24-regular" class="text-xl" />
        </button>
      </div>

      <!-- Body Container -->
      <div class="p-6 overflow-y-auto flex-1 space-y-5">
        <!-- Cari Faktur Form -->
        <div class="bg-slate-50 border border-slate-200 rounded-xl p-4">
          <label class="block text-xs font-bold text-slate-700 uppercase mb-2">No. Struk / Faktur Asli</label>
          <div class="flex gap-2">
            <div class="relative flex-1">
              <Icon icon="fluent:receipt-search-24-regular" class="absolute left-3 top-1/2 -translate-y-1/2 text-slate-400 text-lg" />
              <input
                v-model="invoiceInput"
                @keyup.enter="lookupInvoice"
                type="text"
                placeholder="Contoh: INV-20261008-0001 atau scan barcode struk..."
                class="w-full pl-10 pr-4 py-2.5 text-sm bg-white border border-slate-300 rounded-xl font-mono uppercase focus:ring-2 focus:ring-rose-500 focus:outline-none"
              />
            </div>
            <button
              @click="lookupInvoice"
              :disabled="isLoadingLookup || !invoiceInput.trim()"
              class="px-5 py-2.5 bg-rose-600 hover:bg-rose-700 disabled:opacity-50 text-white font-bold text-sm rounded-xl flex items-center gap-2 transition shadow-sm"
            >
              <Icon v-if="isLoadingLookup" icon="fluent:spinner-ios-20-regular" class="animate-spin text-lg" />
              <Icon v-else icon="fluent:search-20-filled" class="text-lg" />
              <span>Cari Faktur</span>
            </button>
          </div>

          <div v-if="lookupError" class="mt-2 text-xs font-bold text-rose-600 flex items-center gap-1.5">
            <Icon icon="fluent:error-circle-20-filled" />
            <span>{{ lookupError }}</span>
          </div>
        </div>

        <!-- Detail Faktur Ditemukan -->
        <div v-if="saleData" class="space-y-4 animate-fade-in">
          <!-- Banner Ringkasan Transaksi Asli -->
          <div class="grid grid-cols-2 sm:grid-cols-4 gap-3 bg-slate-100 p-3.5 rounded-xl text-xs">
            <div>
              <span class="text-slate-500 block">No. Faktur:</span>
              <span class="font-bold font-mono text-slate-800">{{ saleData.invoice_number }}</span>
            </div>
            <div>
              <span class="text-slate-500 block">Waktu Belanja:</span>
              <span class="font-bold text-slate-800">{{ new Date(saleData.created_at).toLocaleString('id-ID') }}</span>
            </div>
            <div>
              <span class="text-slate-500 block">Pelanggan:</span>
              <span class="font-bold text-slate-800">{{ saleData.customer_name || 'Pelanggan Umum' }}</span>
            </div>
            <div>
              <span class="text-slate-500 block">Total Belanja:</span>
              <span class="font-black text-rose-600">{{ formatRupiah(saleData.total_amount) }}</span>
            </div>
          </div>

          <!-- Tabel Pilihan Item Retur -->
          <div>
            <div class="flex items-center justify-between mb-2">
              <h3 class="text-xs font-bold uppercase tracking-wider text-slate-700">Pilih Barang yang Dikembalikan</h3>
              <span class="text-[11px] text-slate-500">Centang item dan tentukan jumlah yang diklaim konsumen</span>
            </div>

            <div class="border border-slate-200 rounded-xl overflow-hidden divide-y divide-slate-100">
              <div
                v-for="item in saleData.items"
                :key="item.id"
                class="p-3 transition-colors"
                :class="selectedItems[item.id] ? 'bg-rose-50/60' : 'bg-white hover:bg-slate-50'"
              >
                <div class="flex items-start gap-3">
                  <input
                    type="checkbox"
                    :id="`check-${item.id}`"
                    :checked="!!selectedItems[item.id]"
                    :disabled="item.remaining_returnable_qty <= 0"
                    @change="toggleSelectItem(item)"
                    class="mt-1 w-4 h-4 text-rose-600 rounded border-slate-300 focus:ring-rose-500 cursor-pointer disabled:opacity-40"
                  />

                  <div class="flex-1 min-w-0">
                    <div class="flex items-baseline justify-between">
                      <label :for="`check-${item.id}`" class="text-sm font-bold text-slate-800 cursor-pointer truncate">
                        {{ item.product_name }}
                      </label>
                      <span class="text-xs font-bold text-slate-700">
                        {{ formatRupiah(item.unit_price) }} / {{ item.unit_code }}
                      </span>
                    </div>

                    <div class="flex items-center gap-3 text-xs text-slate-500 mt-0.5">
                      <span>Beli Asli: <b>{{ item.quantity }}</b> {{ item.unit_code }}</span>
                      <span v-if="item.returned_quantity > 0" class="text-amber-600 font-medium">
                        Sudah diretur: {{ item.returned_quantity }}
                      </span>
                      <span :class="item.remaining_returnable_qty > 0 ? 'text-emerald-700 font-bold' : 'text-slate-400'">
                        Sisa bisa retur: {{ item.remaining_returnable_qty }} {{ item.unit_code }}
                      </span>
                    </div>

                    <!-- Input Kuantitas & Kondisi Retur Jika Dicentang -->
                    <div v-if="selectedItems[item.id]" class="mt-3 p-3 bg-white border border-rose-200 rounded-xl grid grid-cols-1 sm:grid-cols-3 gap-3 text-xs animate-fade-in">
                      <div>
                        <label class="block font-bold text-slate-700 mb-1">Jumlah Diretur:</label>
                        <div class="flex items-center gap-2">
                          <input
                            v-model.number="selectedItems[item.id].return_qty"
                            type="number"
                            min="1"
                            :max="selectedItems[item.id].max_qty"
                            class="w-20 px-2 py-1.5 border border-slate-300 rounded-lg text-center font-bold focus:ring-2 focus:ring-rose-500 focus:outline-none"
                          />
                          <span class="text-slate-500">{{ item.unit_code }}</span>
                        </div>
                      </div>

                      <div>
                        <label class="block font-bold text-slate-700 mb-1">Kondisi Fisik:</label>
                        <select
                          v-model="selectedItems[item.id].condition"
                          class="w-full px-2 py-1.5 border border-slate-300 rounded-lg font-medium focus:ring-2 focus:ring-rose-500 focus:outline-none"
                        >
                          <option value="GOOD">Bagus / Utuh (Bisa Dijual Lagi)</option>
                          <option value="DAMAGED">Rusak / Cacat / Basi</option>
                        </select>
                      </div>

                      <div>
                        <label class="block font-bold text-slate-700 mb-1">Kembalikan ke Stok:</label>
                        <label class="flex items-center gap-2 mt-2 cursor-pointer">
                          <input
                            type="checkbox"
                            v-model="selectedItems[item.id].restock"
                            :disabled="selectedItems[item.id].condition === 'DAMAGED'"
                            class="w-4 h-4 text-rose-600 rounded border-slate-300"
                          />
                          <span class="text-xs" :class="selectedItems[item.id].condition === 'DAMAGED' ? 'text-slate-400 italic' : 'text-slate-700 font-semibold'">
                            {{ selectedItems[item.id].condition === 'DAMAGED' ? 'Otomatis Masuk Waste (Tidak Restock)' : 'Restock ke Rak Toko' }}
                          </span>
                        </label>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- Opsi Pengembalian Dana & Alasan -->
          <div class="grid grid-cols-1 sm:grid-cols-2 gap-4 bg-slate-50 p-4 rounded-xl border border-slate-200">
            <div>
              <label class="block text-xs font-bold text-slate-700 uppercase mb-1.5">Alasan Retur</label>
              <select v-model="reasonCode" class="w-full px-3 py-2 text-xs border border-slate-300 rounded-xl bg-white font-medium focus:ring-2 focus:ring-rose-500">
                <option value="RET_DEFECT">Barang Cacat / Rusak / Expired</option>
                <option value="RET_EXCHANGE">Konsumen Tukar Barang / Salah Beli</option>
              </select>

              <label class="block text-xs font-bold text-slate-700 uppercase mt-3 mb-1.5">Catatan Tambahan (Opsional)</label>
              <input
                v-model="notes"
                type="text"
                placeholder="Misal: Kemasan sobek saat dibuka pelanggan..."
                class="w-full px-3 py-2 text-xs border border-slate-300 rounded-xl bg-white focus:ring-2 focus:ring-rose-500"
              />
            </div>

            <div>
              <label class="block text-xs font-bold text-slate-700 uppercase mb-1.5">Metode Pengembalian Dana (Refund)</label>
              <div class="grid grid-cols-2 gap-2">
                <button
                  type="button"
                  @click="refundMethod = 'CASH'"
                  class="p-2.5 rounded-xl border text-xs font-bold flex flex-col items-center gap-1 transition"
                  :class="refundMethod === 'CASH' ? 'border-rose-600 bg-rose-50 text-rose-700 ring-2 ring-rose-500' : 'border-slate-300 bg-white text-slate-700 hover:bg-slate-100'"
                >
                  <Icon icon="fluent:money-24-regular" class="text-xl" />
                  <span>Tunai (Kas Laci)</span>
                </button>
                <button
                  type="button"
                  @click="refundMethod = 'STORE_CREDIT'"
                  class="p-2.5 rounded-xl border text-xs font-bold flex flex-col items-center gap-1 transition"
                  :class="refundMethod === 'STORE_CREDIT' ? 'border-rose-600 bg-rose-50 text-rose-700 ring-2 ring-rose-500' : 'border-slate-300 bg-white text-slate-700 hover:bg-slate-100'"
                >
                  <Icon icon="fluent:wallet-credit-card-24-regular" class="text-xl" />
                  <span>Kredit Toko / Deposit</span>
                </button>
              </div>

              <!-- Total Refund -->
              <div class="mt-4 p-3 bg-rose-100/70 border border-rose-200 rounded-xl flex items-center justify-between">
                <span class="text-xs font-bold text-rose-950 uppercase">Total Uang Kembali:</span>
                <span class="text-lg font-black text-rose-700">{{ formatRupiah(totalRefundAmount) }}</span>
              </div>
            </div>
          </div>

          <!-- Otorisasi Supervisor (Mandatory) -->
          <div class="p-3 bg-amber-50/70 border border-amber-300 rounded-xl">
            <SupervisorAuthSection
              v-model:model-value-supervisor-id="selectedSupervisorId"
              v-model:model-value-pin="supervisorPin"
              v-model:model-value-auth-token="supervisorAuthToken"
              :outlet-id="saleData?.outlet_id || locationStore.currentOutletId"
              action-type="SALES_RETURN"
              label="Otorisasi Supervisor Kasir (Retur Penjualan)"
            />
          </div>

          <!-- Alerts -->
          <div v-if="submitError" class="p-3 bg-rose-50 border border-rose-200 rounded-xl text-xs font-bold text-rose-700 flex items-center gap-2">
            <Icon icon="fluent:error-circle-24-filled" class="text-lg" />
            <span>{{ submitError }}</span>
          </div>

          <div v-if="submitSuccess" class="p-3 bg-emerald-50 border border-emerald-200 rounded-xl text-xs font-bold text-emerald-700 flex items-center gap-2">
            <Icon icon="fluent:checkmark-circle-24-filled" class="text-lg" />
            <span>{{ submitSuccess }}</span>
          </div>
        </div>
      </div>

      <!-- Footer Buttons -->
      <div class="px-6 py-4 bg-slate-50 border-t border-slate-200 flex items-center justify-between">
        <button
          type="button"
          @click="closeModal"
          class="px-4 py-2 border border-slate-300 text-slate-700 hover:bg-slate-100 font-bold text-xs rounded-xl transition"
        >
          Batal / Keluar
        </button>

        <button
          v-if="saleData"
          type="button"
          @click="handleSubmitReturn"
          :disabled="isSubmitting || !hasSelectedItems || !supervisorPin.trim()"
          class="px-6 py-2.5 bg-rose-600 hover:bg-rose-700 disabled:opacity-50 text-white font-black text-sm rounded-xl flex items-center gap-2 shadow-lg shadow-rose-600/30 transition"
        >
          <Icon v-if="isSubmitting" icon="fluent:spinner-ios-20-regular" class="animate-spin text-lg" />
          <Icon v-else icon="fluent:checkmark-square-24-filled" class="text-lg" />
          <span>Setujui & Cetak Struk Retur ({{ formatRupiah(totalRefundAmount) }})</span>
        </button>
      </div>
    </div>
  </div>
</template>
