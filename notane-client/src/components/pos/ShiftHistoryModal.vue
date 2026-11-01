<script setup lang="ts">
import { ref, watch } from 'vue'
import {
  X,
  Search,
  RefreshCw,
  Printer,
  ChevronLeft,
  ChevronRight,
  Clock,
  User,
  AlertTriangle,
  CheckCircle2,
} from 'lucide-vue-next'
import { useShiftStore } from '@/stores/shiftStore'
import { useLocationStore } from '@/stores/locationStore'
import { usePrinterStore } from '@/stores/printerStore'
import { useAuthStore } from '@/stores/authStore'
import { formatCurrency } from '@/utils/formatters'

const props = defineProps<{
  show: boolean
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

const shiftStore = useShiftStore()
const locationStore = useLocationStore()
const printerStore = usePrinterStore()
const authStore = useAuthStore()

const shifts = ref<any[]>([])
const isLoading = ref<boolean>(false)
const errorMsg = ref<string>('')

// Filter states
const statusFilter = ref<string>('ALL')
const selectedOutletId = ref<string>('')
const searchQuery = ref<string>('')
const currentPage = ref<number>(1)
const pageSize = ref<number>(10)
const totalItems = ref<number>(0)
const totalPages = ref<number>(1)

// Z-Report Detail modal
const selectedZReport = ref<any>(null)
const isLoadingZReport = ref<boolean>(false)
const isPrinting = ref<boolean>(false)
const printStatusMsg = ref<string>('')

async function loadShifts() {
  isLoading.value = true
  errorMsg.value = ''
  try {
    const res = await shiftStore.fetchShifts({
      status: statusFilter.value,
      outlet_id: selectedOutletId.value || undefined,
      search: searchQuery.value || undefined,
      page: currentPage.value,
      page_size: pageSize.value,
    })
    if (res.success) {
      shifts.value = res.data || []
      totalItems.value = res.total_items || 0
      totalPages.value = res.total_pages || 1
    } else {
      errorMsg.value = res.message || 'Gagal memuat daftar shift'
    }
  } catch (err: any) {
    errorMsg.value = err.response?.data?.message || err.message || 'Gagal memuat daftar shift'
  } finally {
    isLoading.value = false
  }
}

async function viewZReport(shiftId: string) {
  isLoadingZReport.value = true
  printStatusMsg.value = ''
  try {
    const res = await shiftStore.fetchShiftDetail(shiftId)
    if (res.success) {
      selectedZReport.value = res.data
    } else {
      errorMsg.value = res.message || 'Gagal memuat data Z-Report'
    }
  } catch (err: any) {
    errorMsg.value = err.response?.data?.message || err.message || 'Gagal memuat data Z-Report'
  } finally {
    isLoadingZReport.value = false
  }
}

async function handlePrintZReport() {
  if (!selectedZReport.value) return

  if (printerStore.isConnected && printerStore.connectionType !== 'VIRTUAL') {
    try {
      isPrinting.value = true
      printStatusMsg.value = 'Mencetak ke printer thermal...'
      const res = await printerStore.printZReport(selectedZReport.value)
      printStatusMsg.value = res.message
      return
    } catch (err: any) {
      console.warn('Gagal cetak thermal:', err)
      printStatusMsg.value = `Gagal cetak thermal (${err.message}). Beralih ke dialog sistem...`
    } finally {
      isPrinting.value = false
    }
  }

  window.print()
}

function openBrowserPrint() {
  window.print()
}

function paymentIcon(category: string): string {
  const cat = (category || '').toUpperCase()
  if (cat.includes('CASH') || cat.includes('TUNAI')) return '💵'
  if (cat.includes('QRIS') || cat.includes('QR')) return '📱'
  if (cat.includes('EDC') || cat.includes('CARD') || cat.includes('DEBIT') || cat.includes('KREDIT')) return '💳'
  if (cat.includes('POINT') || cat.includes('POIN') || cat.includes('REWARD')) return '⭐'
  if (cat.includes('TRANSFER') || cat.includes('BANK')) return '🏦'
  return '💰'
}

watch(
  () => props.show,
  (val) => {
    if (val) {
      currentPage.value = 1
      loadShifts()
    } else {
      selectedZReport.value = null
    }
  }
)

watch([statusFilter, selectedOutletId], () => {
  currentPage.value = 1
  loadShifts()
})

let searchTimeout: any = null
function onSearchInput() {
  clearTimeout(searchTimeout)
  searchTimeout = setTimeout(() => {
    currentPage.value = 1
    loadShifts()
  }, 400)
}

function prevPage() {
  if (currentPage.value > 1) {
    currentPage.value--
    loadShifts()
  }
}

function nextPage() {
  if (currentPage.value < totalPages.value) {
    currentPage.value++
    loadShifts()
  }
}
</script>

<template>
  <div
    v-if="show"
    class="fixed inset-0 z-50 bg-slate-900/60 backdrop-blur-xs flex items-center justify-center p-4 select-none animate-fadeIn print:static print:inset-auto print:p-0 print:bg-transparent print:overflow-visible"
    @click.self="emit('close')"
  >
    <!-- 1. MODAL DAFTAR SELURUH SHIFT KASIR -->
    <div
      v-if="!selectedZReport"
      class="bg-white border border-slate-300 w-full max-w-5xl rounded-2xl shadow-2xl flex flex-col text-slate-900 max-h-[90vh] overflow-hidden"
    >
      <!-- Header -->
      <div class="bg-slate-900 px-6 py-4 flex items-center justify-between text-white shrink-0">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-xl bg-emerald-500/20 text-emerald-400 border border-emerald-500/30 flex items-center justify-center font-bold">
            <Clock class="w-5 h-5" />
          </div>
          <div>
            <h3 class="font-extrabold text-base leading-tight">
              {{ authStore.isSupervisor ? 'Riwayat Seluruh Shift Kasir' : 'Riwayat Shift Saya' }}
            </h3>
            <p class="text-xs text-slate-400 mt-0.5">
              {{ authStore.isSupervisor
                ? 'Pantau shift aktif, rekap penutupan laci kasir, selisih kas, & cetak ulang Z-Report.'
                : 'Daftar riwayat sesi shift kasir Anda, rincian kas, dan cetak ulang laporan Z-Report.' }}
            </p>
          </div>
        </div>
        <button
          type="button"
          @click="emit('close')"
          class="w-8 h-8 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800 flex items-center justify-center transition cursor-pointer"
        >
          <X class="w-5 h-5" />
        </button>
      </div>

      <!-- Filter Bar -->
      <div class="p-4 bg-slate-50 border-b border-slate-200 flex flex-wrap items-center justify-between gap-3 shrink-0">
        <div class="flex flex-wrap items-center gap-2">
          <!-- Filter Status -->
          <div class="inline-flex rounded-lg border border-slate-300 p-0.5 bg-white text-xs font-semibold">
            <button
              type="button"
              @click="statusFilter = 'ALL'"
              class="px-3 py-1.5 rounded-md transition cursor-pointer"
              :class="statusFilter === 'ALL' ? 'bg-slate-800 text-white shadow-xs' : 'text-slate-600 hover:text-slate-900'"
            >
              Semua Status
            </button>
            <button
              type="button"
              @click="statusFilter = 'OPEN'"
              class="px-3 py-1.5 rounded-md transition cursor-pointer flex items-center gap-1.5"
              :class="statusFilter === 'OPEN' ? 'bg-emerald-600 text-white shadow-xs' : 'text-slate-600 hover:text-slate-900'"
            >
              <span class="w-2 h-2 rounded-full bg-emerald-400 animate-pulse"></span>
              <span>Sedang Dibuka</span>
            </button>
            <button
              type="button"
              @click="statusFilter = 'CLOSED'"
              class="px-3 py-1.5 rounded-md transition cursor-pointer"
              :class="statusFilter === 'CLOSED' ? 'bg-slate-800 text-white shadow-xs' : 'text-slate-600 hover:text-slate-900'"
            >
              Sudah Ditutup
            </button>
          </div>

          <!-- Filter Cabang Outlet (Hanya untuk Supervisor/Admin jika ada multi-outlet) -->
          <select
            v-if="authStore.isSupervisor && locationStore.accessibleOutlets && locationStore.accessibleOutlets.length > 1"
            v-model="selectedOutletId"
            class="text-xs font-medium border border-slate-300 rounded-lg px-2.5 py-1.5 bg-white text-slate-700 outline-none focus:border-emerald-500"
          >
            <option value="">Semua Cabang Outlet</option>
            <option v-for="o in locationStore.accessibleOutlets" :key="o.id" :value="o.id">
              {{ o.name }} ({{ o.code }})
            </option>
          </select>
        </div>

        <div class="flex items-center gap-2">
          <!-- Search input -->
          <div class="relative">
            <Search class="w-3.5 h-3.5 text-slate-400 absolute left-2.5 top-1/2 -translate-y-1/2" />
            <input
              type="text"
              v-model="searchQuery"
              @input="onSearchInput"
              :placeholder="authStore.isSupervisor ? 'Cari kasir / terminal...' : 'Cari terminal / shift...'"
              class="text-xs pl-8 pr-3 py-1.5 rounded-lg border border-slate-300 bg-white text-slate-800 placeholder-slate-400 focus:outline-none focus:border-emerald-500 w-48 transition"
            />
          </div>

          <button
            type="button"
            @click="loadShifts"
            :disabled="isLoading"
            class="p-2 rounded-lg border border-slate-300 bg-white hover:bg-slate-100 text-slate-700 transition cursor-pointer"
            title="Muat ulang daftar shift"
          >
            <RefreshCw class="w-4 h-4" :class="{ 'animate-spin': isLoading }" />
          </button>
        </div>
      </div>

      <!-- Error Message -->
      <div v-if="errorMsg" class="mx-6 mt-4 p-3 rounded-xl bg-rose-50 border border-rose-200 text-rose-700 text-xs flex items-center justify-between">
        <span>{{ errorMsg }}</span>
        <button type="button" @click="errorMsg = ''" class="text-rose-500 hover:text-rose-700">
          <X class="w-4 h-4" />
        </button>
      </div>

      <!-- Table Content (Scrollable) -->
      <div class="flex-1 overflow-auto min-h-0">
        <table class="w-full text-left text-xs text-slate-700 min-w-[850px]">
          <thead class="bg-slate-100/80 border-b border-slate-200 text-[11px] font-extrabold text-slate-600 uppercase tracking-wider sticky top-0 z-10">
            <tr>
              <th class="px-4 py-3">Waktu Shift</th>
              <th class="px-4 py-3">Kasir & Terminal</th>
              <th class="px-4 py-3">Cabang Outlet</th>
              <th class="px-4 py-3 text-right">Modal Awal</th>
              <th class="px-4 py-3 text-right">Penjualan Shift</th>
              <th class="px-4 py-3 text-right">Kas Sistem vs Fisik</th>
              <th class="px-4 py-3 text-center">Selisih Kas</th>
              <th class="px-4 py-3 text-center">Status</th>
              <th class="px-4 py-3 text-center">Aksi</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-slate-200">
            <!-- Loading Row -->
            <tr v-if="isLoading">
              <td colspan="9" class="py-12 text-center text-slate-400">
                <RefreshCw class="w-6 h-6 mx-auto mb-2 text-emerald-600 animate-spin" />
                <p class="font-bold text-xs">Memuat data shift kasir...</p>
              </td>
            </tr>

            <!-- Empty Row -->
            <tr v-else-if="shifts.length === 0">
              <td colspan="9" class="py-12 text-center text-slate-400">
                <Clock class="w-8 h-8 mx-auto mb-2 text-slate-300" />
                <p class="font-bold text-slate-600">Tidak ada riwayat shift ditemukan</p>
                <p class="text-[11px] text-slate-400 mt-0.5">Sesuaikan filter atau lakukan pencarian lain.</p>
              </td>
            </tr>

            <!-- Shift Rows -->
            <tr
              v-else
              v-for="s in shifts"
              :key="s.id"
              class="hover:bg-slate-50/80 transition"
              :class="{ 'bg-emerald-50/30': s.status === 'OPEN' }"
            >
              <!-- 1. Waktu Shift -->
              <td class="px-4 py-3 whitespace-nowrap">
                <div class="font-bold text-slate-900">
                  {{ new Date(s.opened_at).toLocaleDateString('id-ID', { day: '2-digit', month: 'short', year: 'numeric' }) }}
                </div>
                <div class="text-[10px] text-slate-500 font-mono mt-0.5">
                  Buka: {{ new Date(s.opened_at).toLocaleTimeString('id-ID', { hour: '2-digit', minute: '2-digit' }) }}
                </div>
                <div v-if="s.closed_at" class="text-[10px] text-slate-600 font-mono">
                  Tutup: {{ new Date(s.closed_at).toLocaleTimeString('id-ID', { hour: '2-digit', minute: '2-digit' }) }}
                </div>
                <div v-else class="text-[10px] font-bold text-emerald-600">
                  ● Sedang Berlangsung
                </div>
              </td>

              <!-- 2. Kasir & Terminal -->
              <td class="px-4 py-3">
                <div class="font-bold text-slate-900 flex items-center gap-1.5">
                  <User class="w-3.5 h-3.5 text-slate-400 shrink-0" />
                  <span>{{ s.cashier_name || s.cashier_username }}</span>
                </div>
                <div class="text-[10px] text-slate-500 font-mono mt-0.5">
                  Terminal: {{ s.terminal_id || 'Kasir' }}
                </div>
                <div v-if="s.supervisor_name" class="text-[10px] text-slate-400">
                  Otorisasi: {{ s.supervisor_name }}
                </div>
              </td>

              <!-- 3. Cabang Outlet -->
              <td class="px-4 py-3 whitespace-nowrap">
                <div class="font-medium text-slate-800">{{ s.outlet_name || 'Outlet Utama' }}</div>
                <div v-if="s.outlet_code" class="text-[10px] font-mono text-slate-400">{{ s.outlet_code }}</div>
              </td>

              <!-- 4. Modal Awal -->
              <td class="px-4 py-3 text-right font-mono font-medium whitespace-nowrap">
                {{ formatCurrency(s.opening_cash) }}
              </td>

              <!-- 5. Penjualan Shift -->
              <td class="px-4 py-3 text-right font-mono whitespace-nowrap">
                <div class="font-bold text-slate-900">{{ formatCurrency(s.total_net_sales) }}</div>
                <div class="text-[10px] text-slate-500">{{ s.total_sales_count }} transaksi</div>
              </td>

              <!-- 6. Kas Sistem vs Fisik -->
              <td class="px-4 py-3 text-right font-mono whitespace-nowrap">
                <div v-if="s.status === 'CLOSED'">
                  <div class="text-slate-500 text-[10px]">Sistem: {{ formatCurrency(s.expected_cash || 0) }}</div>
                  <div class="font-bold text-slate-900">Fisik: {{ formatCurrency(s.actual_cash || 0) }}</div>
                </div>
                <div v-else class="text-emerald-700 font-bold text-[11px]">
                  Aktif di Laci: {{ formatCurrency((Number(s.opening_cash) + Number(s.total_cash_sales) - Number(s.total_cash_drops) - Number(s.total_expenses) - Number(s.total_sales_returns))) }}
                </div>
              </td>

              <!-- 7. Selisih Kas -->
              <td class="px-4 py-3 text-center whitespace-nowrap">
                <template v-if="s.status === 'CLOSED'">
                  <span
                    v-if="Number(s.discrepancy || 0) === 0"
                    class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-bold bg-emerald-100 text-emerald-800 border border-emerald-300"
                  >
                    <CheckCircle2 class="w-3 h-3" />
                    <span>SESUAI</span>
                  </span>
                  <span
                    v-else-if="Number(s.discrepancy || 0) > 0"
                    class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-bold bg-green-100 text-green-800 border border-green-300"
                  >
                    <span>+{{ formatCurrency(s.discrepancy) }} (Lebih)</span>
                  </span>
                  <span
                    v-else
                    class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-bold bg-rose-100 text-rose-800 border border-rose-300"
                  >
                    <AlertTriangle class="w-3 h-3" />
                    <span>{{ formatCurrency(s.discrepancy) }} (Kurang)</span>
                  </span>
                </template>
                <span v-else class="text-slate-400 text-[10px] italic">
                  Belum settlement
                </span>
              </td>

              <!-- 8. Status -->
              <td class="px-4 py-3 text-center whitespace-nowrap">
                <span
                  v-if="s.status === 'OPEN'"
                  class="inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-[10px] font-extrabold bg-emerald-100 text-emerald-800 border border-emerald-300 shadow-2xs"
                >
                  <span class="w-1.5 h-1.5 rounded-full bg-emerald-600 animate-ping"></span>
                  <span>DIBUKA</span>
                </span>
                <span
                  v-else
                  class="px-2.5 py-0.5 rounded-full text-[10px] font-bold bg-slate-100 text-slate-700 border border-slate-300"
                >
                  DITUTUP
                </span>
              </td>

              <!-- 9. Aksi -->
              <td class="px-4 py-3 text-center whitespace-nowrap">
                <button
                  type="button"
                  @click="viewZReport(s.id)"
                  class="inline-flex items-center gap-1 px-2.5 py-1 rounded-lg text-xs font-bold text-slate-700 bg-white hover:bg-slate-100 border border-slate-300 shadow-2xs transition active:scale-95 cursor-pointer"
                  title="Lihat rincian penerimaan dan cetak Z-Report"
                >
                  <Printer class="w-3.5 h-3.5 text-slate-500" />
                  <span>Z-Report</span>
                </button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <!-- Pagination Footer -->
      <div class="px-6 py-3.5 bg-slate-50 border-t border-slate-200 flex items-center justify-between text-xs shrink-0">
        <div class="text-slate-500">
          Menampilkan <span class="font-bold text-slate-800">{{ shifts.length }}</span> dari
          <span class="font-bold text-slate-800">{{ totalItems }}</span> shift kasir
        </div>

        <div class="flex items-center gap-2">
          <button
            type="button"
            @click="prevPage"
            :disabled="currentPage <= 1 || isLoading"
            class="px-3 py-1.5 rounded-lg border border-slate-300 bg-white hover:bg-slate-100 disabled:opacity-40 disabled:cursor-not-allowed font-medium text-slate-700 transition flex items-center gap-1 cursor-pointer"
          >
            <ChevronLeft class="w-4 h-4" />
            <span>Sebelumnya</span>
          </button>

          <span class="font-mono text-slate-700 px-2 font-bold">
            {{ currentPage }} / {{ totalPages || 1 }}
          </span>

          <button
            type="button"
            @click="nextPage"
            :disabled="currentPage >= totalPages || isLoading"
            class="px-3 py-1.5 rounded-lg border border-slate-300 bg-white hover:bg-slate-100 disabled:opacity-40 disabled:cursor-not-allowed font-medium text-slate-700 transition flex items-center gap-1 cursor-pointer"
          >
            <span>Selanjutnya</span>
            <ChevronRight class="w-4 h-4" />
          </button>
        </div>
      </div>
    </div>

    <!-- 2. MODAL PREVIEW & CETAK ULANG Z-REPORT -->
    <div
      v-else
      id="printable-z-report"
      class="bg-white border border-slate-300 w-full max-w-lg rounded-2xl shadow-2xl flex flex-col text-slate-900 overflow-hidden print:max-w-none print:shadow-none print:border-none print:w-[80mm] print:m-0 print:p-0"
    >
      <!-- Header -->
      <div class="bg-slate-900 p-4 text-center text-white relative">
        <button
          type="button"
          @click="selectedZReport = null"
          class="absolute left-3 top-3.5 text-slate-400 hover:text-white p-1 rounded-lg hover:bg-slate-800 transition cursor-pointer print:hidden"
          title="Kembali ke daftar shift"
        >
          <ChevronLeft class="w-5 h-5" />
        </button>
        <h4 class="font-extrabold text-sm tracking-wide">📋 LAPORAN PENUTUPAN SHIFT</h4>
        <p class="text-[10px] text-slate-400 mt-0.5">Z-REPORT — NOTANE RETAIL POS</p>
      </div>

      <!-- Scrollable content -->
      <div class="overflow-y-auto max-h-[70vh] p-4 space-y-3 print:max-h-none print:overflow-visible print:p-1">
        <!-- Shift Info -->
        <div class="bg-slate-50 border border-slate-200 rounded-xl p-3 text-[11px] space-y-1">
          <div class="flex justify-between text-slate-700">
            <span class="font-semibold">Kasir</span>
            <span class="text-slate-900 font-bold">{{ selectedZReport.cashier_name }}</span>
          </div>
          <div class="flex justify-between text-slate-700">
            <span class="font-semibold">Terminal</span>
            <span class="text-slate-900">{{ selectedZReport.shift?.terminal_id }}</span>
          </div>
          <div class="flex justify-between text-slate-700">
            <span class="font-semibold">Outlet</span>
            <span class="text-slate-900">{{ selectedZReport.shift?.outlet_name || selectedZReport.outlet_name || '-' }}</span>
          </div>
          <div class="flex justify-between text-slate-700 pt-1 border-t border-slate-200">
            <span class="font-semibold">Mulai Shift</span>
            <span class="text-slate-900">{{ new Date(selectedZReport.shift?.opened_at).toLocaleString('id-ID', { day:'2-digit', month:'short', year:'numeric', hour:'2-digit', minute:'2-digit' }) }}</span>
          </div>
          <div class="flex justify-between text-slate-700">
            <span class="font-semibold">Tutup Shift</span>
            <span class="text-slate-900">
              {{ selectedZReport.shift?.closed_at ? new Date(selectedZReport.shift.closed_at).toLocaleString('id-ID', { day:'2-digit', month:'short', year:'numeric', hour:'2-digit', minute:'2-digit' }) : '(Belum Ditutup)' }}
            </span>
          </div>
          <div class="flex justify-between text-slate-700">
            <span class="font-semibold">Jumlah Transaksi</span>
            <span class="text-slate-900 font-bold">{{ selectedZReport.total_sales_count ?? selectedZReport.transaction_count ?? '-' }} nota</span>
          </div>
        </div>

        <!-- Payment Method Breakdown -->
        <div>
          <h5 class="text-[10px] font-bold text-slate-500 uppercase tracking-widest mb-2">Rekapan Penerimaan per Metode Bayar</h5>
          <div class="border border-slate-200 rounded-xl overflow-hidden">
            <template v-if="selectedZReport.payment_breakdowns && selectedZReport.payment_breakdowns.length > 0">
              <div
                v-for="(pm, idx) in selectedZReport.payment_breakdowns"
                :key="pm.category || pm.payment_method_id || idx"
                class="flex items-center justify-between px-3 py-2 text-[11px]"
                :class="idx % 2 === 0 ? 'bg-white' : 'bg-slate-50'"
              >
                <div class="flex items-center gap-2">
                  <span class="text-base">{{ paymentIcon(pm.category) }}</span>
                  <span class="text-slate-700 font-medium">{{ pm.payment_method_name }}</span>
                  <span v-if="pm.transaction_count" class="text-slate-400 text-[10px]">({{ pm.transaction_count }}x)</span>
                </div>
                <span class="font-bold text-slate-900">{{ formatCurrency(pm.total_amount) }}</span>
              </div>
            </template>
            <template v-else>
              <div class="flex items-center justify-between px-3 py-2 text-[11px] bg-white">
                <div class="flex items-center gap-2">
                  <span class="text-base">💵</span>
                  <span class="text-slate-700 font-medium">Uang Tunai (Cash)</span>
                </div>
                <span class="font-bold text-slate-900">{{ formatCurrency(selectedZReport.total_cash_received ?? 0) }}</span>
              </div>
              <div v-if="Number(selectedZReport.total_qris_received ?? 0) > 0" class="flex items-center justify-between px-3 py-2 text-[11px] bg-slate-50">
                <div class="flex items-center gap-2">
                  <span class="text-base">📱</span>
                  <span class="text-slate-700 font-medium">QRIS</span>
                </div>
                <span class="font-bold text-slate-900">{{ formatCurrency(selectedZReport.total_qris_received ?? 0) }}</span>
              </div>
              <div v-if="Number(selectedZReport.total_card_received ?? 0) > 0" class="flex items-center justify-between px-3 py-2 text-[11px] bg-white">
                <div class="flex items-center gap-2">
                  <span class="text-base">💳</span>
                  <span class="text-slate-700 font-medium">Debit / Kredit EDC</span>
                </div>
                <span class="font-bold text-slate-900">{{ formatCurrency(selectedZReport.total_card_received ?? 0) }}</span>
              </div>
            </template>

            <!-- Grand Total row -->
            <div class="flex items-center justify-between px-3 py-2.5 text-[12px] bg-slate-800 text-white">
              <span class="font-bold">Total Penjualan Bersih (Net Sales)</span>
              <span class="font-bold">{{ formatCurrency(selectedZReport.total_net_sales ?? 0) }}</span>
            </div>
          </div>
        </div>

        <!-- Cash Drawer Reconciliation -->
        <div>
          <h5 class="text-[10px] font-bold text-slate-500 uppercase tracking-widest mb-2">Rekonsiliasi Laci Kasir (Cash Drawer)</h5>
          <div class="border border-slate-200 rounded-xl overflow-hidden">
            <div class="flex items-center justify-between px-3 py-2 text-[11px] bg-white">
              <span class="text-slate-600">Modal Awal Shift (Opening Cash)</span>
              <span class="font-semibold text-slate-800">{{ formatCurrency(selectedZReport.shift?.opening_cash ?? 0) }}</span>
            </div>
            <div class="flex items-center justify-between px-3 py-2 text-[11px] bg-slate-50">
              <span class="text-slate-600">+ Penerimaan Tunai Shift</span>
              <span class="font-semibold text-emerald-700">+{{ formatCurrency(selectedZReport.total_cash_received ?? 0) }}</span>
            </div>
            <div v-if="Number(selectedZReport.total_expenses ?? 0) > 0" class="flex items-center justify-between px-3 py-2 text-[11px] bg-white">
              <span class="text-slate-600">- Pengeluaran Laci Kasir</span>
              <span class="font-semibold text-rose-600">-{{ formatCurrency(selectedZReport.total_expenses) }}</span>
            </div>
            <div class="flex items-center justify-between px-3 py-2.5 text-[11px] bg-amber-50 border-t border-amber-200">
              <span class="text-amber-800 font-bold">= Seharusnya di Laci</span>
              <span class="font-bold text-amber-800">{{ formatCurrency(selectedZReport.expected_cash_in_drawer ?? 0) }}</span>
            </div>
            <div class="flex items-center justify-between px-3 py-2 text-[11px] bg-white">
              <span class="text-slate-600">Hitungan Fisik Kasir</span>
              <span class="font-semibold text-slate-800">{{ formatCurrency(selectedZReport.actual_cash_counted ?? 0) }}</span>
            </div>
            <div
              class="flex items-center justify-between px-3 py-2.5 text-[12px] font-bold border-t-2"
              :class="(selectedZReport.discrepancy ?? 0) < 0
                ? 'bg-red-50 border-red-300 text-red-700'
                : (selectedZReport.discrepancy ?? 0) > 0
                ? 'bg-green-50 border-green-300 text-green-700'
                : 'bg-emerald-50 border-emerald-300 text-emerald-700'"
            >
              <span>Selisih Kas</span>
              <span>
                {{ formatCurrency(Math.abs(selectedZReport.discrepancy ?? 0)) }}
                <span class="text-[10px] ml-1">
                  {{ (selectedZReport.discrepancy ?? 0) < 0 ? '⚠️ KURANG KAS' : (selectedZReport.discrepancy ?? 0) > 0 ? '✅ LEBIH KAS' : '✅ SESUAI' }}
                </span>
              </span>
            </div>
          </div>
        </div>

        <p class="text-[10px] text-slate-400 text-center">
          Pencatatan shift telah dibukukan otomatis ke General Ledger (SHIFT_CLOSING).
        </p>
      </div>

      <!-- Footer actions -->
      <div class="p-4 border-t border-slate-200 flex flex-col gap-2 bg-slate-50 print:hidden">
        <div v-if="printStatusMsg" class="text-xs text-center font-medium text-slate-600">
          {{ printStatusMsg }}
        </div>
        <div class="flex items-center gap-2">
          <button
            type="button"
            @click="handlePrintZReport"
            :disabled="isPrinting"
            class="flex-1 bg-emerald-600 hover:bg-emerald-700 disabled:opacity-50 text-white py-2.5 rounded-xl text-xs font-bold flex items-center justify-center gap-1.5 transition active:scale-95 shadow-sm border border-emerald-700 cursor-pointer"
          >
            <Printer class="w-4 h-4" />
            <span>{{ isPrinting ? 'Mencetak...' : 'Cetak Z-Report' }}</span>
          </button>
          <button
            type="button"
            @click="openBrowserPrint"
            title="Cetak via dialog browser / simpan ke PDF"
            class="bg-white hover:bg-slate-100 border border-slate-300 text-slate-700 px-3 py-2.5 rounded-xl text-xs font-bold transition active:scale-95 flex items-center gap-1 cursor-pointer"
          >
            <span>PDF / Dialog</span>
          </button>
          <button
            type="button"
            @click="selectedZReport = null"
            class="bg-slate-100 hover:bg-slate-200 border border-slate-300 text-slate-800 px-4 py-2.5 rounded-xl text-xs font-bold transition active:scale-95 cursor-pointer"
          >
            Kembali
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
