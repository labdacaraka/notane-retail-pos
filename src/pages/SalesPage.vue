<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import AdminLayout from '@/layouts/AdminLayout.vue'
import PaginationBar from '@/components/common/PaginationBar.vue'
import {
  Search,
  RefreshCw,
  Printer,
  Eye,
  XCircle,
  X,
  AlertCircle,
  CheckCircle2,
  Calendar,
  CreditCard,
  Receipt,
  RotateCcw,
} from 'lucide-vue-next'
import { Icon } from '@iconify/vue'
import { api } from '@/services/api'
import { usePrinterStore } from '@/stores/printerStore'

const router = useRouter()
const printerStore = usePrinterStore()

// State
const sales = ref<any[]>([])
const stats = ref<any>({
  total_revenue: '0',
  total_transactions: 0,
  total_items_sold: '0',
  total_discount: '0',
  avg_basket_size: '0',
})
const isLoading = ref<boolean>(false)
const searchQuery = ref<string>('')
const statusFilter = ref<string>('ALL')

// Pagination
const currentPage = ref<number>(1)
const pageSize = ref<number>(10)

// Modal Detail State
const showDetailModal = ref<boolean>(false)
const selectedSale = ref<any>(null)
const isLoadingDetail = ref<boolean>(false)

// Modal Void State
const showVoidModal = ref<boolean>(false)
const saleToVoid = ref<any>(null)
const supervisorPin = ref<string>('123456')
const voidReason = ref<string>('Kesalahan Input Kasir')
const isVoiding = ref<boolean>(false)
const voidError = ref<string>('')

// Alert toast
const toastMessage = ref<string>('')
const toastType = ref<'success' | 'error'>('success')

function showToast(msg: string, type: 'success' | 'error' = 'success') {
  toastMessage.value = msg
  toastType.value = type
  setTimeout(() => {
    toastMessage.value = ''
  }, 4000)
}

// Load Data
async function loadSales() {
  try {
    isLoading.value = true
    const [salesRes, statsRes] = await Promise.all([
      api.get('/sales'),
      api.get('/sales/stats'),
    ])

    if (salesRes.data?.success) {
      sales.value = salesRes.data.data
    }
    if (statsRes.data?.success) {
      stats.value = statsRes.data.data
    }
  } catch (err: any) {
    console.error('Gagal memuat data penjualan:', err)
    showToast('Gagal memuat data penjualan', 'error')
  } finally {
    isLoading.value = false
  }
}

onMounted(() => {
  loadSales()
})

// Filter & Pagination
const filteredSales = computed(() => {
  let list = sales.value

  if (statusFilter.value !== 'ALL') {
    list = list.filter((s) => s.status === statusFilter.value)
  }

  if (searchQuery.value.trim()) {
    const q = searchQuery.value.toLowerCase().trim()
    list = list.filter((s) => {
      const inv = (s.invoice_number || '').toLowerCase()
      const cashier = (s.cashier_name || '').toLowerCase()
      const customer = (s.customer_name || '').toLowerCase()
      const notes = (s.notes || '').toLowerCase()
      return inv.includes(q) || cashier.includes(q) || customer.includes(q) || notes.includes(q)
    })
  }

  return list
})

const paginatedSales = computed(() => {
  const start = (currentPage.value - 1) * pageSize.value
  return filteredSales.value.slice(start, start + pageSize.value)
})

// Format Helpers
function formatRupiah(val: any): string {
  const num = Number(val) || 0
  return new Intl.NumberFormat('id-ID', {
    style: 'currency',
    currency: 'IDR',
    minimumFractionDigits: 0,
    maximumFractionDigits: 0,
  }).format(num)
}

function formatQty(val: any): string {
  const num = Number(val) || 0
  return Number.isInteger(num) ? num.toString() : num.toFixed(2)
}

function formatDate(dateStr: string): string {
  if (!dateStr) return '-'
  try {
    const d = new Date(dateStr)
    return new Intl.DateTimeFormat('id-ID', {
      day: '2-digit',
      month: 'short',
      year: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    }).format(d)
  } catch {
    return dateStr
  }
}

// Actions
async function viewDetail(saleId: string) {
  try {
    isLoadingDetail.value = true
    showDetailModal.value = true
    const res = await api.get(`/sales/${saleId}`)
    if (res.data?.success) {
      selectedSale.value = res.data.data
    } else {
      showToast(res.data?.message || 'Gagal memuat detail', 'error')
      showDetailModal.value = false
    }
  } catch (err: any) {
    showToast(err.response?.data?.message || 'Gagal memuat detail transaksi', 'error')
    showDetailModal.value = false
  } finally {
    isLoadingDetail.value = false
  }
}

async function handleReprint(sale: any) {
  try {
    // If sale doesn't have joined items, fetch full detail
    let detail = sale
    if (!detail.items || detail.items.length === 0) {
      const res = await api.get(`/sales/${sale.id}`)
      if (res.data?.success) {
        detail = res.data.data
      }
    }

    const printPayload = {
      invoice_number: detail.invoice_number,
      created_at: detail.created_at,
      cashier_name: detail.cashier_name || 'Kasir',
      customer_name: detail.customer_name || 'Pelanggan Umum',
      items: (detail.items || []).map((it: any) => ({
        product_name: it.product_name,
        quantity: it.quantity,
        unit_price: it.unit_selling_price,
        discount_amount: it.discount_amount,
        subtotal: it.subtotal,
      })),
      subtotal: detail.subtotal,
      discount_amount: detail.discount_amount,
      tax_amount: detail.tax_amount,
      total_amount: detail.total_amount,
      paid_amount: detail.paid_amount,
      change_amount: detail.change_amount,
      payments: (detail.payments || []).map((p: any) => ({
        payment_method_name: p.payment_method_name,
        amount: p.amount,
      })),
    }

    const pRes = await printerStore.printReceipt(printPayload)
    showToast(pRes.message || 'Struk berhasil dicetak ulang!', 'success')
  } catch (err: any) {
    showToast(`Gagal mencetak struk: ${err.message}`, 'error')
  }
}

function openVoidModal(sale: any) {
  saleToVoid.value = sale
  supervisorPin.value = '123456'
  voidReason.value = 'Kesalahan Input Kasir'
  voidError.value = ''
  showVoidModal.value = true
}

async function confirmVoid() {
  if (!saleToVoid.value) return
  if (!supervisorPin.value) {
    voidError.value = 'PIN Supervisor wajib diisi'
    return
  }

  try {
    isVoiding.value = true
    voidError.value = ''
    const res = await api.post(`/sales/${saleToVoid.value.id}/void`, {
      supervisor_pin: supervisorPin.value,
      notes: voidReason.value,
    })

    if (res.data?.success) {
      showToast(res.data.message || 'Transaksi berhasil di-void', 'success')
      showVoidModal.value = false
      if (showDetailModal.value && selectedSale.value?.id === saleToVoid.value.id) {
        selectedSale.value.status = 'VOID'
      }
      loadSales()
    } else {
      voidError.value = res.data?.message || 'Gagal membatalkan transaksi'
    }
  } catch (err: any) {
    voidError.value = err.response?.data?.message || err.message || 'Gagal membatalkan transaksi'
  } finally {
    isVoiding.value = false
  }
}
</script>

<template>
  <AdminLayout>
    <div class="h-full flex flex-col bg-slate-100 overflow-hidden">
      <!-- Toast Notification -->
      <transition
        enter-active-class="transition duration-300 ease-out"
        enter-from-class="transform -translate-y-4 opacity-0"
        enter-to-class="transform translate-y-0 opacity-100"
        leave-active-class="transition duration-200 ease-in"
        leave-from-class="transform translate-y-0 opacity-100"
        leave-to-class="transform -translate-y-4 opacity-0"
      >
        <div
          v-if="toastMessage"
          class="fixed top-20 right-6 z-50 flex items-center gap-2.5 px-4 py-3 rounded-xl shadow-xl text-xs font-bold border backdrop-blur-md"
          :class="toastType === 'success'
            ? 'bg-emerald-500/90 text-white border-emerald-400'
            : 'bg-rose-500/90 text-white border-rose-400'"
        >
          <CheckCircle2 v-if="toastType === 'success'" class="w-4 h-4 shrink-0" />
          <AlertCircle v-else class="w-4 h-4 shrink-0" />
          <span>{{ toastMessage }}</span>
        </div>
      </transition>

      <!-- 1. Header Toolbar -->
      <div class="bg-white border-b border-slate-300 px-6 py-4 shrink-0 flex items-center justify-between shadow-xs">
        <div class="flex items-center gap-3.5">
          <div class="w-11 h-11 rounded-xl bg-amber-50 border border-amber-300 flex items-center justify-center shadow-xs">
            <Icon icon="fluent-color:receipt-24" class="w-7 h-7" />
          </div>
          <div>
            <h1 class="text-lg font-black text-slate-900 tracking-tight flex items-center gap-2">
              <span>Transaksi Penjualan POS</span>
              <span class="text-[10px] bg-amber-100 text-amber-900 font-mono px-2 py-0.5 rounded-full border border-amber-300 font-bold">
                SALES AUDIT
              </span>
            </h1>
            <p class="text-xs text-slate-500 mt-0.5">
              Rekapitulasi penjualan kasir, riwayat faktur POS, cetak ulang struk thermal langsung & void pembatalan
            </p>
          </div>
        </div>

        <div class="flex items-center gap-2.5">
          <button
            type="button"
            @click="loadSales"
            :disabled="isLoading"
            class="px-3.5 py-2 rounded-lg border border-slate-300 bg-white hover:bg-slate-50 text-slate-700 font-bold text-xs flex items-center gap-2 transition active:scale-95 cursor-pointer shadow-xs"
            title="Muat Ulang Data"
          >
            <RefreshCw class="w-3.5 h-3.5" :class="{ 'animate-spin': isLoading }" />
            <span>Segarkan</span>
          </button>

          <button
            type="button"
            @click="router.push('/')"
            class="px-4 py-2 rounded-lg bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-xs flex items-center gap-2 transition active:scale-95 cursor-pointer shadow-xs border border-emerald-700"
          >
            <Icon icon="streamline-ultimate-color:shopping-cart-full" class="w-4 h-4" />
            <span>Buka Kasir POS (F1)</span>
          </button>
        </div>
      </div>

      <!-- 2. KPI Summary Cards Bar -->
      <div class="px-6 pt-4 pb-2 shrink-0">
        <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3.5">
          <!-- Card 1: Total Omzet -->
          <div class="bg-white border border-slate-200 rounded-xl p-3.5 shadow-2xs flex items-center gap-3.5 hover:border-amber-300 transition">
            <div class="w-12 h-12 rounded-xl bg-emerald-50 border border-emerald-200 flex items-center justify-center shrink-0">
              <Icon icon="streamline-ultimate-color:cash-payment-bill" class="w-7 h-7" />
            </div>
            <div class="min-w-0">
              <div class="text-[10px] font-bold text-slate-400 uppercase tracking-wider">Total Omzet Penjualan</div>
              <div class="text-base font-black text-slate-900 truncate mt-0.5">
                {{ formatRupiah(stats.total_revenue) }}
              </div>
              <div class="text-[10px] text-emerald-600 font-bold mt-0.5">
                {{ stats.total_transactions }} Transaksi Tercatat
              </div>
            </div>
          </div>

          <!-- Card 2: Total Item Terjual -->
          <div class="bg-white border border-slate-200 rounded-xl p-3.5 shadow-2xs flex items-center gap-3.5 hover:border-blue-300 transition">
            <div class="w-12 h-12 rounded-xl bg-blue-50 border border-blue-200 flex items-center justify-center shrink-0">
              <Icon icon="streamline-ultimate-color:shopping-cart-full" class="w-7 h-7" />
            </div>
            <div class="min-w-0">
              <div class="text-[10px] font-bold text-slate-400 uppercase tracking-wider">Total Item Terjual</div>
              <div class="text-base font-black text-slate-900 truncate mt-0.5">
                {{ formatQty(stats.total_items_sold) }} <span class="text-xs font-normal text-slate-500">Unit/Kg</span>
              </div>
              <div class="text-[10px] text-blue-600 font-bold mt-0.5">Produk Ritel Keluar</div>
            </div>
          </div>

          <!-- Card 3: Rata-Rata Keranjang (Basket Size) -->
          <div class="bg-white border border-slate-200 rounded-xl p-3.5 shadow-2xs flex items-center gap-3.5 hover:border-purple-300 transition">
            <div class="w-12 h-12 rounded-xl bg-purple-50 border border-purple-200 flex items-center justify-center shrink-0">
              <Icon icon="streamline-ultimate-color:accounting-calculator-1" class="w-7 h-7" />
            </div>
            <div class="min-w-0">
              <div class="text-[10px] font-bold text-slate-400 uppercase tracking-wider">Rata-Rata Belanja (ATV)</div>
              <div class="text-base font-black text-slate-900 truncate mt-0.5">
                {{ formatRupiah(stats.avg_basket_size) }}
              </div>
              <div class="text-[10px] text-purple-600 font-bold mt-0.5">Nilai per Transaksi</div>
            </div>
          </div>

          <!-- Card 4: Total Diskon Diberikan -->
          <div class="bg-white border border-slate-200 rounded-xl p-3.5 shadow-2xs flex items-center gap-3.5 hover:border-rose-300 transition">
            <div class="w-12 h-12 rounded-xl bg-rose-50 border border-rose-200 flex items-center justify-center shrink-0">
              <Icon icon="fluent-color:gift-card-24" class="w-7 h-7" />
            </div>
            <div class="min-w-0">
              <div class="text-[10px] font-bold text-slate-400 uppercase tracking-wider">Total Potongan Diskon</div>
              <div class="text-base font-black text-slate-900 truncate mt-0.5">
                {{ formatRupiah(stats.total_discount) }}
              </div>
              <div class="text-[10px] text-rose-600 font-bold mt-0.5">Voucher & Diskon Kasir</div>
            </div>
          </div>
        </div>
      </div>

      <!-- 3. Filter & Table Content -->
      <div class="flex-1 px-6 py-3 min-h-0 flex flex-col">
        <div class="bg-white border border-slate-300 rounded-xl flex flex-col h-full shadow-xs overflow-hidden">
          <!-- Filter Controls -->
          <div class="px-4 py-3 border-b border-slate-200 flex flex-wrap items-center justify-between gap-3 bg-slate-50/70 shrink-0">
            <!-- Search -->
            <div class="relative flex-1 min-w-[240px] max-w-md">
              <Search class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-slate-400" />
              <input
                v-model="searchQuery"
                type="text"
                placeholder="Cari no. faktur, nama kasir, pelanggan..."
                class="w-full pl-9 pr-3.5 py-1.5 rounded-lg border border-slate-300 bg-white text-xs text-slate-800 placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-amber-500 focus:border-transparent transition"
              />
            </div>

            <!-- Status Filter -->
            <div class="flex items-center gap-2">
              <span class="text-xs font-bold text-slate-500">Status:</span>
              <div class="inline-flex rounded-lg border border-slate-300 bg-slate-100 p-0.5 text-xs">
                <button
                  type="button"
                  @click="statusFilter = 'ALL'"
                  class="px-3 py-1 rounded-md font-bold transition cursor-pointer"
                  :class="statusFilter === 'ALL' ? 'bg-white text-slate-900 shadow-2xs' : 'text-slate-600 hover:text-slate-900'"
                >
                  Semua ({{ sales.length }})
                </button>
                <button
                  type="button"
                  @click="statusFilter = 'COMPLETED'"
                  class="px-3 py-1 rounded-md font-bold transition cursor-pointer"
                  :class="statusFilter === 'COMPLETED' ? 'bg-white text-emerald-800 shadow-2xs' : 'text-slate-600 hover:text-slate-900'"
                >
                  Sukses
                </button>
                <button
                  type="button"
                  @click="statusFilter = 'VOID'"
                  class="px-3 py-1 rounded-md font-bold transition cursor-pointer"
                  :class="statusFilter === 'VOID' ? 'bg-white text-rose-800 shadow-2xs' : 'text-slate-600 hover:text-slate-900'"
                >
                  Void
                </button>
              </div>
            </div>
          </div>

          <!-- Table -->
          <div class="flex-1 overflow-auto min-h-0">
            <table class="w-full text-left text-xs text-slate-700">
              <thead class="bg-slate-50 border-b border-slate-200 text-[11px] font-bold text-slate-500 uppercase tracking-wider sticky top-0 z-10">
                <tr>
                  <th class="px-4 py-2.5">No. Faktur</th>
                  <th class="px-4 py-2.5">Waktu Transaksi</th>
                  <th class="px-4 py-2.5">Kasir / Terminal</th>
                  <th class="px-4 py-2.5">Pelanggan</th>
                  <th class="px-4 py-2.5 text-right">Subtotal</th>
                  <th class="px-4 py-2.5 text-right">Total Tagihan</th>
                  <th class="px-4 py-2.5 text-center">Status</th>
                  <th class="px-4 py-2.5 text-center">Aksi</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-slate-200">
                <tr
                  v-for="s in paginatedSales"
                  :key="s.id"
                  class="hover:bg-amber-50/40 transition group"
                  :class="{ 'bg-rose-50/30 opacity-75': s.status === 'VOID' }"
                >
                  <!-- No Faktur -->
                  <td class="px-4 py-3 font-mono font-bold text-slate-900 whitespace-nowrap">
                    <div class="flex items-center gap-1.5">
                      <Receipt class="w-3.5 h-3.5 text-amber-600 shrink-0" />
                      <span>{{ s.invoice_number }}</span>
                    </div>
                  </td>

                  <!-- Waktu Transaksi -->
                  <td class="px-4 py-3 text-slate-600 whitespace-nowrap">
                    <div class="flex items-center gap-1.5">
                      <Calendar class="w-3.5 h-3.5 text-slate-400" />
                      <span>{{ formatDate(s.created_at) }}</span>
                    </div>
                  </td>

                  <!-- Kasir & Terminal -->
                  <td class="px-4 py-3">
                    <div class="font-bold text-slate-800">{{ s.cashier_name || 'Kasir Default' }}</div>
                    <div class="text-[10px] text-slate-500 font-mono">{{ s.terminal_id || 'TERMINAL-POS' }}</div>
                  </td>

                  <!-- Pelanggan -->
                  <td class="px-4 py-3 text-slate-600">
                    <span v-if="s.customer_name" class="font-bold text-indigo-700">
                      {{ s.customer_name }}
                    </span>
                    <span v-else class="text-slate-400 italic">Umum (Non-Member)</span>
                  </td>

                  <!-- Subtotal -->
                  <td class="px-4 py-3 text-right font-mono text-slate-600 whitespace-nowrap">
                    {{ formatRupiah(s.subtotal) }}
                  </td>

                  <!-- Total Tagihan -->
                  <td class="px-4 py-3 text-right font-mono font-extrabold text-slate-900 whitespace-nowrap">
                    {{ formatRupiah(s.total_amount) }}
                  </td>

                  <!-- Status -->
                  <td class="px-4 py-3 text-center whitespace-nowrap">
                    <span
                      v-if="s.status === 'COMPLETED'"
                      class="px-2.5 py-0.5 rounded-full text-[10px] font-extrabold bg-emerald-100 text-emerald-800 border border-emerald-300"
                    >
                      SELESAI
                    </span>
                    <span
                      v-else-if="s.status === 'VOID'"
                      class="px-2.5 py-0.5 rounded-full text-[10px] font-extrabold bg-rose-100 text-rose-800 border border-rose-300 line-through"
                    >
                      DIBATALKAN (VOID)
                    </span>
                    <span
                      v-else
                      class="px-2.5 py-0.5 rounded-full text-[10px] font-extrabold bg-slate-100 text-slate-800 border border-slate-300"
                    >
                      {{ s.status }}
                    </span>
                  </td>

                  <!-- Aksi -->
                  <td class="px-4 py-3 text-center whitespace-nowrap">
                    <div class="flex items-center justify-center gap-1.5">
                      <!-- Detail -->
                      <button
                        type="button"
                        @click="viewDetail(s.id)"
                        class="p-1.5 rounded-lg border border-slate-300 bg-white hover:bg-slate-100 text-slate-700 transition cursor-pointer shadow-2xs"
                        title="Lihat Rincian Faktur"
                      >
                        <Eye class="w-3.5 h-3.5 text-blue-600" />
                      </button>

                      <!-- Cetak Ulang Struk -->
                      <button
                        type="button"
                        @click="handleReprint(s)"
                        class="p-1.5 rounded-lg border border-slate-300 bg-white hover:bg-emerald-50 text-emerald-700 transition cursor-pointer shadow-2xs hover:border-emerald-300"
                        title="Cetak Ulang Struk Thermal"
                      >
                        <Printer class="w-3.5 h-3.5" />
                      </button>

                      <!-- Void Transaksi -->
                      <button
                        v-if="s.status !== 'VOID'"
                        type="button"
                        @click="openVoidModal(s)"
                        class="p-1.5 rounded-lg border border-slate-300 bg-white hover:bg-rose-50 text-rose-600 transition cursor-pointer shadow-2xs hover:border-rose-300"
                        title="Void / Batalkan Transaksi"
                      >
                        <XCircle class="w-3.5 h-3.5" />
                      </button>
                    </div>
                  </td>
                </tr>

                <!-- Empty State -->
                <tr v-if="paginatedSales.length === 0 && !isLoading">
                  <td colspan="8" class="px-6 py-12 text-center text-slate-400">
                    <Icon icon="fluent-color:receipt-24" class="w-12 h-12 mx-auto mb-2 opacity-60" />
                    <p class="font-bold text-sm text-slate-600">Tidak ada riwayat penjualan ditemukan</p>
                    <p class="text-xs text-slate-400 mt-0.5">Silakan lakukan transaksi di kasir POS terlebih dahulu</p>
                  </td>
                </tr>

                <!-- Loading State -->
                <tr v-if="isLoading">
                  <td colspan="8" class="px-6 py-12 text-center text-slate-400">
                    <RefreshCw class="w-8 h-8 mx-auto mb-2 text-amber-500 animate-spin" />
                    <p class="font-bold text-xs text-slate-600">Memuat data faktur...</p>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <!-- Pagination Bar -->
          <div class="px-4 py-2.5 border-t border-slate-200 bg-slate-50 shrink-0">
            <PaginationBar
              v-model:currentPage="currentPage"
              v-model:pageSize="pageSize"
              :totalItems="filteredSales.length"
            />
          </div>
        </div>
      </div>

      <!-- 4. Modal Detail Faktur -->
      <div
        v-if="showDetailModal"
        class="fixed inset-0 z-50 bg-slate-900/60 backdrop-blur-xs flex items-center justify-center p-4"
        @click.self="showDetailModal = false"
      >
        <div class="bg-white rounded-2xl shadow-2xl border border-slate-300 w-full max-w-3xl overflow-hidden flex flex-col max-h-[90vh] animate-in fade-in zoom-in-95 duration-150">
          <!-- Modal Header -->
          <div class="px-6 py-4 bg-slate-900 text-white flex items-center justify-between shrink-0">
            <div class="flex items-center gap-3">
              <div class="w-9 h-9 rounded-xl bg-amber-500/20 border border-amber-400/40 flex items-center justify-center">
                <Receipt class="w-5 h-5 text-amber-400" />
              </div>
              <div>
                <div class="text-xs font-bold text-slate-400 uppercase tracking-wider">Rincian Faktur Penjualan</div>
                <div class="text-base font-black tracking-tight flex items-center gap-2">
                  <span>{{ selectedSale?.invoice_number }}</span>
                  <span
                    v-if="selectedSale?.status === 'COMPLETED'"
                    class="text-[10px] bg-emerald-500/20 text-emerald-300 px-2 py-0.5 rounded border border-emerald-400/30 font-bold"
                  >
                    COMPLETED
                  </span>
                  <span
                    v-else-if="selectedSale?.status === 'VOID'"
                    class="text-[10px] bg-rose-500/20 text-rose-300 px-2 py-0.5 rounded border border-rose-400/30 font-bold line-through"
                  >
                    VOIDED
                  </span>
                </div>
              </div>
            </div>
            <button
              type="button"
              @click="showDetailModal = false"
              class="text-slate-400 hover:text-white p-1 rounded-lg hover:bg-slate-800 transition cursor-pointer"
            >
              <X class="w-5 h-5" />
            </button>
          </div>

          <!-- Modal Body (Scrollable) -->
          <div class="p-6 overflow-y-auto space-y-5 flex-1 min-h-0 text-xs">
            <!-- Loading -->
            <div v-if="isLoadingDetail" class="py-12 text-center text-slate-400">
              <RefreshCw class="w-8 h-8 mx-auto mb-2 text-amber-500 animate-spin" />
              <p class="font-bold">Memuat rincian item faktur...</p>
            </div>

            <div v-else-if="selectedSale" class="space-y-5">
              <!-- Meta Info Grid -->
              <div class="grid grid-cols-2 sm:grid-cols-4 gap-3 bg-slate-50 p-3.5 rounded-xl border border-slate-200">
                <div>
                  <div class="text-[10px] font-bold text-slate-400 uppercase">Waktu Transaksi</div>
                  <div class="font-bold text-slate-800 mt-0.5">{{ formatDate(selectedSale.created_at) }}</div>
                </div>
                <div>
                  <div class="text-[10px] font-bold text-slate-400 uppercase">Kasir Pelaksana</div>
                  <div class="font-bold text-slate-800 mt-0.5">{{ selectedSale.cashier_name || 'Kasir Default' }}</div>
                </div>
                <div>
                  <div class="text-[10px] font-bold text-slate-400 uppercase">Terminal Register</div>
                  <div class="font-mono font-bold text-slate-800 mt-0.5">{{ selectedSale.terminal_id || 'POS-01' }}</div>
                </div>
                <div>
                  <div class="text-[10px] font-bold text-slate-400 uppercase">Pelanggan</div>
                  <div class="font-bold text-indigo-700 mt-0.5">
                    {{ selectedSale.customer_name || 'Pelanggan Umum' }}
                  </div>
                </div>
              </div>

              <!-- Item List Table -->
              <div>
                <div class="text-xs font-bold text-slate-800 mb-2 uppercase tracking-wide flex items-center justify-between">
                  <span>Daftar Produk yang Dibeli</span>
                  <span class="text-slate-400 font-normal">({{ (selectedSale.items || []).length }} item baris)</span>
                </div>
                <div class="border border-slate-200 rounded-xl overflow-hidden">
                  <table class="w-full text-left text-xs text-slate-700">
                    <thead class="bg-slate-100 text-[10px] font-bold text-slate-500 uppercase border-b border-slate-200">
                      <tr>
                        <th class="px-3 py-2">Produk</th>
                        <th class="px-3 py-2">Barcode / SKU</th>
                        <th class="px-3 py-2 text-right">Qty</th>
                        <th class="px-3 py-2 text-right">Harga Satuan</th>
                        <th class="px-3 py-2 text-right">Diskon</th>
                        <th class="px-3 py-2 text-right">Subtotal</th>
                      </tr>
                    </thead>
                    <tbody class="divide-y divide-slate-100 font-mono">
                      <tr v-for="(it, idx) in selectedSale.items" :key="idx" class="hover:bg-slate-50/80">
                        <td class="px-3 py-2.5 font-sans font-bold text-slate-900">
                          {{ it.product_name }}
                        </td>
                        <td class="px-3 py-2.5 text-[11px] text-slate-500">
                          {{ it.product_barcode || it.product_sku || '-' }}
                        </td>
                        <td class="px-3 py-2.5 text-right font-bold text-slate-800">
                          {{ formatQty(it.quantity) }} {{ it.unit_code }}
                        </td>
                        <td class="px-3 py-2.5 text-right text-slate-600">
                          {{ formatRupiah(it.unit_selling_price) }}
                        </td>
                        <td class="px-3 py-2.5 text-right text-rose-600">
                          {{ Number(it.discount_amount) > 0 ? `-${formatRupiah(it.discount_amount)}` : '-' }}
                        </td>
                        <td class="px-3 py-2.5 text-right font-black text-slate-900">
                          {{ formatRupiah(it.subtotal) }}
                        </td>
                      </tr>
                    </tbody>
                  </table>
                </div>
              </div>

              <!-- Financial Breakdown & Payment Methods -->
              <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
                <!-- Payment Methods -->
                <div class="bg-slate-50 p-4 rounded-xl border border-slate-200">
                  <div class="text-xs font-bold text-slate-800 mb-2.5 flex items-center gap-1.5 uppercase">
                    <CreditCard class="w-4 h-4 text-emerald-600" />
                    <span>Metode Pembayaran</span>
                  </div>
                  <div class="space-y-2">
                    <div
                      v-for="(pm, pidx) in selectedSale.payments"
                      :key="pidx"
                      class="flex items-center justify-between p-2 rounded-lg bg-white border border-slate-200 text-xs"
                    >
                      <div class="font-bold text-slate-800">{{ pm.payment_method_name }}</div>
                      <div class="font-mono font-bold text-emerald-700">{{ formatRupiah(pm.amount) }}</div>
                    </div>
                  </div>
                </div>

                <!-- Financial Summary -->
                <div class="bg-amber-50/50 p-4 rounded-xl border border-amber-200 space-y-2 font-mono">
                  <div class="flex justify-between text-slate-600">
                    <span>Subtotal Barang:</span>
                    <span class="font-bold">{{ formatRupiah(selectedSale.subtotal) }}</span>
                  </div>
                  <div class="flex justify-between text-rose-600" v-if="Number(selectedSale.discount_amount) > 0">
                    <span>Diskon / Potongan:</span>
                    <span class="font-bold">-{{ formatRupiah(selectedSale.discount_amount) }}</span>
                  </div>
                  <div class="flex justify-between text-slate-600" v-if="Number(selectedSale.tax_amount) > 0">
                    <span>PPN (Pajak):</span>
                    <span class="font-bold">{{ formatRupiah(selectedSale.tax_amount) }}</span>
                  </div>
                  <div class="h-px bg-amber-200 my-1"></div>
                  <div class="flex justify-between text-slate-900 font-extrabold text-sm">
                    <span class="font-sans">Total Tagihan:</span>
                    <span>{{ formatRupiah(selectedSale.total_amount) }}</span>
                  </div>
                  <div class="flex justify-between text-emerald-700 font-bold">
                    <span>Jumlah Dibayar:</span>
                    <span>{{ formatRupiah(selectedSale.paid_amount) }}</span>
                  </div>
                  <div class="flex justify-between text-slate-700 font-bold" v-if="Number(selectedSale.change_amount) > 0">
                    <span>Uang Kembalian:</span>
                    <span>{{ formatRupiah(selectedSale.change_amount) }}</span>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- Modal Footer -->
          <div class="px-6 py-3.5 bg-slate-50 border-t border-slate-200 flex items-center justify-between shrink-0">
            <button
              v-if="selectedSale && selectedSale.status !== 'VOID'"
              type="button"
              @click="openVoidModal(selectedSale)"
              class="px-3.5 py-2 rounded-lg border border-rose-300 bg-white hover:bg-rose-50 text-rose-700 font-bold text-xs flex items-center gap-2 transition active:scale-95 cursor-pointer shadow-2xs"
            >
              <XCircle class="w-4 h-4" />
              <span>Void Transaksi Ini</span>
            </button>
            <div v-else></div>

            <div class="flex items-center gap-2.5">
              <button
                type="button"
                @click="showDetailModal = false"
                class="px-4 py-2 rounded-lg border border-slate-300 bg-white hover:bg-slate-100 text-slate-700 font-bold text-xs transition cursor-pointer"
              >
                Tutup
              </button>

              <button
                v-if="selectedSale"
                type="button"
                @click="handleReprint(selectedSale)"
                class="px-4 py-2 rounded-lg bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-xs flex items-center gap-2 shadow-xs border border-emerald-700 transition active:scale-95 cursor-pointer"
              >
                <Printer class="w-4 h-4" />
                <span>Cetak Ulang Struk Thermal</span>
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- 5. Modal Konfirmasi Void Transaksi -->
      <div
        v-if="showVoidModal"
        class="fixed inset-0 z-50 bg-slate-900/60 backdrop-blur-xs flex items-center justify-center p-4"
        @click.self="showVoidModal = false"
      >
        <div class="bg-white rounded-2xl shadow-2xl border border-slate-300 w-full max-w-md overflow-hidden animate-in fade-in zoom-in-95 duration-150">
          <div class="p-6">
            <div class="flex items-center gap-3.5 mb-4">
              <div class="w-11 h-11 rounded-xl bg-rose-50 border border-rose-300 flex items-center justify-center shrink-0">
                <RotateCcw class="w-6 h-6 text-rose-600" />
              </div>
              <div>
                <h3 class="text-base font-black text-slate-900">Batalkan Transaksi (Void)</h3>
                <p class="text-xs text-slate-500 font-mono">{{ saleToVoid?.invoice_number }}</p>
              </div>
            </div>

            <div class="p-3 bg-amber-50 border border-amber-200 rounded-xl text-xs text-amber-900 mb-4 flex gap-2">
              <AlertCircle class="w-4 h-4 text-amber-600 shrink-0 mt-0.5" />
              <p>
                Void akan membatalkan status penjualan, mengembalikan stok barang ke gudang, dan mencatat pembalikan jurnal akuntansi secara otomatis.
              </p>
            </div>

            <div v-if="voidError" class="p-3 bg-rose-50 border border-rose-200 rounded-xl text-xs text-rose-700 mb-4">
              {{ voidError }}
            </div>

            <div class="space-y-3 text-xs">
              <div>
                <label class="block font-bold text-slate-700 mb-1">PIN Otorisasi Supervisor *</label>
                <input
                  v-model="supervisorPin"
                  type="password"
                  placeholder="Masukkan 6-digit PIN supervisor (default: 123456)"
                  class="w-full px-3.5 py-2 rounded-lg border border-slate-300 font-mono text-sm tracking-widest focus:ring-2 focus:ring-rose-500 focus:outline-none"
                />
              </div>

              <div>
                <label class="block font-bold text-slate-700 mb-1">Alasan Pembatalan / Void *</label>
                <select
                  v-model="voidReason"
                  class="w-full px-3.5 py-2 rounded-lg border border-slate-300 bg-white font-sans text-xs focus:ring-2 focus:ring-rose-500 focus:outline-none"
                >
                  <option value="Kesalahan Input Kasir">Kesalahan Input Kasir</option>
                  <option value="Pelanggan Membatalkan Transaksi">Pelanggan Membatalkan Transaksi</option>
                  <option value="Barang Rusak / Tidak Sesuai">Barang Rusak / Tidak Sesuai</option>
                  <option value="Uji Coba Sistem / Training">Uji Coba Sistem / Training</option>
                  <option value="Lainnya">Lainnya</option>
                </select>
              </div>
            </div>
          </div>

          <div class="px-6 py-3.5 bg-slate-50 border-t border-slate-200 flex items-center justify-end gap-2.5">
            <button
              type="button"
              @click="showVoidModal = false"
              :disabled="isVoiding"
              class="px-4 py-2 rounded-lg border border-slate-300 bg-white hover:bg-slate-100 text-slate-700 font-bold text-xs transition cursor-pointer"
            >
              Batal
            </button>
            <button
              type="button"
              @click="confirmVoid"
              :disabled="isVoiding"
              class="px-4 py-2 rounded-lg bg-rose-600 hover:bg-rose-700 text-white font-bold text-xs flex items-center gap-2 shadow-xs transition active:scale-95 cursor-pointer disabled:opacity-50"
            >
              <RefreshCw v-if="isVoiding" class="w-4 h-4 animate-spin" />
              <XCircle v-else class="w-4 h-4" />
              <span>Konfirmasi Void</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  </AdminLayout>
</template>
