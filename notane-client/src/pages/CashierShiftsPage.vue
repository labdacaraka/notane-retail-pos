<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import AdminLayout from '@/layouts/AdminLayout.vue'
import PaginationBar from '@/components/common/PaginationBar.vue'
import {
  Search,
  RefreshCw,
  Printer,
  Clock,
  User,
  AlertTriangle,
  CheckCircle2,
  TrendingUp,
  AlertCircle,
  X,
} from 'lucide-vue-next'
import { Icon } from '@iconify/vue'
import { useShiftStore } from '@/stores/shiftStore'
import { useLocationStore } from '@/stores/locationStore'
import { usePrinterStore } from '@/stores/printerStore'
import { formatCurrency, formatDate } from '@/utils/formatters'

const shiftStore = useShiftStore()
const locationStore = useLocationStore()
const printerStore = usePrinterStore()

// State
const shifts = ref<any[]>([])
const totalShifts = ref<number>(0)
const stats = ref<any>({
  total_shifts: 0,
  open_shifts: 0,
  closed_shifts: 0,
  total_opening_cash: '0',
  total_actual_cash: '0',
  total_discrepancy: '0',
  total_shift_sales: '0',
})
const isLoading = ref<boolean>(false)
const isLoadingStats = ref<boolean>(false)
const errorMsg = ref<string>('')

// Filter states
const searchQuery = ref<string>('')
const statusFilter = ref<string>('ALL')
const selectedOutletId = ref<string>('')

// Server-side Pagination
const currentPage = ref<number>(1)
const pageSize = ref<number>(15)

// Modal Detail & Z-Report State
const showDetailModal = ref<boolean>(false)
const selectedZReport = ref<any>(null)
const isLoadingDetail = ref<boolean>(false)
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
      totalShifts.value = res.total_items || 0
    } else {
      errorMsg.value = res.message || 'Gagal memuat daftar shift kasir'
    }
  } catch (err: any) {
    errorMsg.value = err.response?.data?.message || err.message || 'Gagal memuat daftar shift kasir'
  } finally {
    isLoading.value = false
  }
}

async function loadStats() {
  isLoadingStats.value = true
  try {
    const res = await shiftStore.fetchShiftStats(selectedOutletId.value || undefined)
    if (res.success && res.data) {
      stats.value = res.data
    }
  } catch {
    // Non-blocking for page functionality
  } finally {
    isLoadingStats.value = false
  }
}

let searchTimeout: any = null
function onSearchInput() {
  clearTimeout(searchTimeout)
  searchTimeout = setTimeout(() => {
    currentPage.value = 1
    loadShifts()
  }, 400)
}

watch([statusFilter, selectedOutletId], () => {
  currentPage.value = 1
  loadShifts()
  loadStats()
})

async function openZReportModal(shiftId: string) {
  isLoadingDetail.value = true
  printStatusMsg.value = ''
  showDetailModal.value = true
  selectedZReport.value = null
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
    isLoadingDetail.value = false
  }
}

async function handlePrintThermal() {
  if (!selectedZReport.value) return
  if (printerStore.isConnected && printerStore.connectionType !== 'VIRTUAL') {
    try {
      isPrinting.value = true
      printStatusMsg.value = 'Mencetak ke printer thermal...'
      const res = await printerStore.printZReport(selectedZReport.value)
      if (res.success) {
        printStatusMsg.value = 'Berhasil dicetak ke printer thermal!'
        setTimeout(() => {
          printStatusMsg.value = ''
        }, 3000)
      } else {
        printStatusMsg.value = 'Gagal mencetak: ' + (res.message || 'Koneksi error')
      }
    } catch (e: any) {
      printStatusMsg.value = 'Error mencetak: ' + (e.message || 'Printer offline')
    } finally {
      isPrinting.value = false
    }
  } else {
    // Fallback: Browser Print
    window.print()
  }
}

function handlePrintSystem() {
  window.print()
}

function paymentIcon(category: string): string {
  switch (category?.toUpperCase()) {
    case 'CASH':
      return '💵'
    case 'QRIS':
      return '📱'
    case 'EDC':
    case 'CARD':
      return '💳'
    case 'TRANSFER':
      return '🏦'
    case 'POINTS':
      return '⭐'
    default:
      return '💰'
  }
}

onMounted(() => {
  loadShifts()
  loadStats()
})
</script>

<template>
  <AdminLayout>
    <div class="h-full flex flex-col overflow-hidden bg-slate-100 print:bg-white print:overflow-visible">
      <!-- 1. Header Toolbar -->
      <div class="bg-white border-b border-slate-200 px-6 py-4 flex flex-wrap items-center justify-between gap-4 shrink-0 shadow-2xs print:hidden">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-xl bg-blue-50 border border-blue-200 flex items-center justify-center text-blue-600 shadow-2xs">
            <Icon icon="fluent-color:clock-24" class="w-6 h-6" />
          </div>
          <div>
            <h1 class="text-lg font-black text-slate-900 tracking-tight flex items-center gap-2">
              <span>Laporan & Riwayat Shift Kasir</span>
              <span class="text-xs font-semibold px-2 py-0.5 rounded-full bg-blue-100 text-blue-800 border border-blue-200">
                Backoffice
              </span>
            </h1>
            <p class="text-xs text-slate-500 mt-0.5">
              Monitoring seluruh sesi kasir, audit rekonsiliasi kas, pelacakan selisih laci, & cetak ulang Z-Report.
            </p>
          </div>
        </div>

        <div class="flex items-center gap-2.5">
          <button
            type="button"
            @click="() => { loadShifts(); loadStats() }"
            :disabled="isLoading || isLoadingStats"
            class="inline-flex items-center gap-2 px-3.5 py-2 rounded-xl text-xs font-bold text-slate-700 bg-white hover:bg-slate-50 border border-slate-200 shadow-2xs transition active:scale-95 disabled:opacity-50 cursor-pointer"
          >
            <RefreshCw class="w-3.5 h-3.5" :class="{ 'animate-spin': isLoading }" />
            <span>Segarkan</span>
          </button>
        </div>
      </div>

      <!-- 2. KPI Summary Cards -->
      <div class="px-6 pt-4 pb-1 grid grid-cols-2 md:grid-cols-5 gap-3 shrink-0 print:hidden">
        <!-- Card 1: Total Shift -->
        <div class="bg-white border border-slate-200 rounded-xl p-3.5 shadow-2xs">
          <div class="flex items-center justify-between text-slate-500 mb-1">
            <span class="text-xs font-semibold uppercase tracking-wider">Total Shift</span>
            <div class="p-1 rounded-md bg-slate-100 text-slate-600">
              <Clock class="w-4 h-4" />
            </div>
          </div>
          <div class="text-xl font-black text-slate-900 tracking-tight">
            {{ stats.total_shifts }}
          </div>
          <div class="text-[11px] text-slate-500 mt-0.5">Sesi kasir terdaftar</div>
        </div>

        <!-- Card 2: Shift Aktif -->
        <div class="bg-white border border-emerald-200 rounded-xl p-3.5 shadow-2xs bg-emerald-50/20">
          <div class="flex items-center justify-between text-emerald-700 mb-1">
            <span class="text-xs font-bold uppercase tracking-wider">Sedang Dibuka</span>
            <div class="p-1 rounded-md bg-emerald-100 text-emerald-600">
              <span class="w-2.5 h-2.5 rounded-full bg-emerald-500 inline-block animate-ping"></span>
            </div>
          </div>
          <div class="text-xl font-black text-emerald-800 tracking-tight">
            {{ stats.open_shifts }}
          </div>
          <div class="text-[11px] text-emerald-600 font-medium mt-0.5">Kasir aktif di POS</div>
        </div>

        <!-- Card 3: Shift Ditutup -->
        <div class="bg-white border border-slate-200 rounded-xl p-3.5 shadow-2xs">
          <div class="flex items-center justify-between text-slate-500 mb-1">
            <span class="text-xs font-semibold uppercase tracking-wider">Sudah Ditutup</span>
            <div class="p-1 rounded-md bg-slate-100 text-slate-600">
              <CheckCircle2 class="w-4 h-4" />
            </div>
          </div>
          <div class="text-xl font-black text-slate-900 tracking-tight">
            {{ stats.closed_shifts }}
          </div>
          <div class="text-[11px] text-slate-500 mt-0.5">Telah settlement laci</div>
        </div>

        <!-- Card 4: Total Penjualan -->
        <div class="bg-white border border-slate-200 rounded-xl p-3.5 shadow-2xs">
          <div class="flex items-center justify-between text-slate-500 mb-1">
            <span class="text-xs font-semibold uppercase tracking-wider">Total Omzet Shift</span>
            <div class="p-1 rounded-md bg-indigo-50 text-indigo-600">
              <TrendingUp class="w-4 h-4" />
            </div>
          </div>
          <div class="text-xl font-black text-slate-900 tracking-tight font-mono">
            {{ formatCurrency(stats.total_shift_sales) }}
          </div>
          <div class="text-[11px] text-slate-500 mt-0.5">Penjualan bersih shift</div>
        </div>

        <!-- Card 5: Selisih Kas -->
        <div
          class="bg-white border rounded-xl p-3.5 shadow-2xs"
          :class="Number(stats.total_discrepancy || 0) === 0 ? 'border-slate-200' : Number(stats.total_discrepancy || 0) < 0 ? 'border-rose-300 bg-rose-50/20' : 'border-amber-300 bg-amber-50/20'"
        >
          <div class="flex items-center justify-between text-slate-500 mb-1">
            <span class="text-xs font-semibold uppercase tracking-wider">Total Selisih Kas</span>
            <div class="p-1 rounded-md bg-slate-100 text-slate-600">
              <AlertCircle class="w-4 h-4" />
            </div>
          </div>
          <div
            class="text-xl font-black tracking-tight font-mono"
            :class="Number(stats.total_discrepancy || 0) === 0 ? 'text-slate-900' : Number(stats.total_discrepancy || 0) < 0 ? 'text-rose-700' : 'text-amber-700'"
          >
            {{ formatCurrency(stats.total_discrepancy) }}
          </div>
          <div class="text-[11px] text-slate-500 mt-0.5">
            {{ Number(stats.total_discrepancy || 0) === 0 ? 'Kas seimbang 100%' : Number(stats.total_discrepancy || 0) < 0 ? 'Kas kurang dari sistem' : 'Kas berlebih dari sistem' }}
          </div>
        </div>
      </div>

      <!-- 3. Filter Bar & Controls -->
      <div class="px-6 py-3 flex flex-wrap items-center justify-between gap-3 shrink-0 print:hidden">
        <div class="flex flex-wrap items-center gap-2">
          <!-- Status Segments -->
          <div class="inline-flex rounded-xl border border-slate-300 p-1 bg-white shadow-2xs text-xs font-bold">
            <button
              type="button"
              @click="statusFilter = 'ALL'"
              class="px-3 py-1.5 rounded-lg transition cursor-pointer"
              :class="statusFilter === 'ALL' ? 'bg-slate-900 text-white shadow-xs' : 'text-slate-600 hover:text-slate-900'"
            >
              Semua
            </button>
            <button
              type="button"
              @click="statusFilter = 'OPEN'"
              class="px-3 py-1.5 rounded-lg transition cursor-pointer flex items-center gap-1.5"
              :class="statusFilter === 'OPEN' ? 'bg-emerald-600 text-white shadow-xs' : 'text-slate-600 hover:text-slate-900'"
            >
              <span class="w-2 h-2 rounded-full bg-emerald-400"></span>
              <span>Sedang Dibuka</span>
            </button>
            <button
              type="button"
              @click="statusFilter = 'CLOSED'"
              class="px-3 py-1.5 rounded-lg transition cursor-pointer"
              :class="statusFilter === 'CLOSED' ? 'bg-slate-900 text-white shadow-xs' : 'text-slate-600 hover:text-slate-900'"
            >
              Sudah Ditutup
            </button>
          </div>

          <!-- Outlet Filter Dropdown -->
          <select
            v-if="locationStore.accessibleOutlets && locationStore.accessibleOutlets.length > 1"
            v-model="selectedOutletId"
            class="text-xs font-semibold border border-slate-300 rounded-xl px-3 py-2 bg-white text-slate-700 shadow-2xs outline-none focus:border-blue-500"
          >
            <option value="">Semua Cabang Outlet</option>
            <option v-for="o in locationStore.accessibleOutlets" :key="o.id" :value="o.id">
              {{ o.name }} ({{ o.code }})
            </option>
          </select>
        </div>

        <div class="flex items-center gap-2">
          <!-- Search Input -->
          <div class="relative">
            <Search class="w-4 h-4 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2" />
            <input
              type="text"
              v-model="searchQuery"
              @input="onSearchInput"
              placeholder="Cari kasir / terminal..."
              class="text-xs pl-9 pr-3 py-2 rounded-xl border border-slate-300 bg-white text-slate-800 placeholder-slate-400 shadow-2xs focus:outline-none focus:border-blue-500 w-56 transition"
            />
          </div>
        </div>
      </div>

      <!-- Error Message Banner -->
      <div v-if="errorMsg" class="mx-6 mb-3 p-3 rounded-xl bg-rose-50 border border-rose-200 text-rose-700 text-xs flex items-center justify-between shrink-0">
        <div class="flex items-center gap-2">
          <AlertCircle class="w-4 h-4 text-rose-500" />
          <span>{{ errorMsg }}</span>
        </div>
        <button type="button" @click="errorMsg = ''" class="text-rose-500 hover:text-rose-700 cursor-pointer">
          <X class="w-4 h-4" />
        </button>
      </div>

      <!-- 4. Table Container -->
      <div class="flex-1 px-6 pb-4 overflow-hidden flex flex-col min-h-0 print:hidden">
        <div class="bg-white border border-slate-300 rounded-2xl shadow-2xs flex-1 flex flex-col overflow-hidden">
          <div class="flex-1 overflow-auto">
            <table class="w-full text-left text-xs border-collapse">
              <thead class="bg-slate-50 text-slate-600 font-bold uppercase tracking-wider border-b border-slate-200 sticky top-0 z-10">
                <tr>
                  <th class="px-4 py-3 whitespace-nowrap">Waktu Mulai / Selesai</th>
                  <th class="px-4 py-3 whitespace-nowrap">Kasir & Terminal</th>
                  <th class="px-4 py-3 whitespace-nowrap">Cabang Toko</th>
                  <th class="px-4 py-3 text-right whitespace-nowrap">Modal Awal</th>
                  <th class="px-4 py-3 text-right whitespace-nowrap">Penjualan Shift</th>
                  <th class="px-4 py-3 text-right whitespace-nowrap">Kas Sistem vs Fisik</th>
                  <th class="px-4 py-3 text-center whitespace-nowrap">Selisih Kas</th>
                  <th class="px-4 py-3 text-center whitespace-nowrap">Supervisor Otorisasi</th>
                  <th class="px-4 py-3 text-center whitespace-nowrap">Status</th>
                  <th class="px-4 py-3 text-center whitespace-nowrap">Aksi</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-slate-200">
                <tr v-if="isLoading" class="bg-white">
                  <td colspan="10" class="px-4 py-12 text-center text-slate-400">
                    <div class="flex flex-col items-center justify-center gap-2">
                      <RefreshCw class="w-6 h-6 animate-spin text-blue-600" />
                      <span class="font-medium text-slate-600">Memuat data shift kasir...</span>
                    </div>
                  </td>
                </tr>

                <tr v-else-if="shifts.length === 0" class="bg-white">
                  <td colspan="10" class="px-4 py-12 text-center text-slate-400">
                    <div class="flex flex-col items-center justify-center gap-2">
                      <Clock class="w-8 h-8 text-slate-300" />
                      <span class="font-bold text-slate-600">Tidak ada sesi shift yang sesuai kriteria</span>
                      <span class="text-xs text-slate-400">Coba ubah filter atau lakukan pencarian lain</span>
                    </div>
                  </td>
                </tr>

                <tr
                  v-for="s in shifts"
                  :key="s.id"
                  class="hover:bg-slate-50/80 transition"
                  :class="s.status === 'OPEN' ? 'bg-emerald-50/20' : ''"
                >
                  <!-- 1. Waktu Mulai & Tutup -->
                  <td class="px-4 py-3 whitespace-nowrap">
                    <div class="font-bold text-slate-900">{{ formatDate(s.opened_at) }}</div>
                    <div class="text-[11px] text-slate-500 font-mono">
                      Mulai: {{ new Date(s.opened_at).toLocaleTimeString('id-ID', { hour: '2-digit', minute: '2-digit' }) }}
                    </div>
                    <div v-if="s.closed_at" class="text-[11px] text-slate-500 font-mono">
                      Tutup: {{ new Date(s.closed_at).toLocaleTimeString('id-ID', { hour: '2-digit', minute: '2-digit' }) }}
                    </div>
                    <div v-else class="text-[10px] text-emerald-600 font-bold uppercase tracking-wider">
                      (Sesi Aktif)
                    </div>
                  </td>

                  <!-- 2. Kasir & Terminal -->
                  <td class="px-4 py-3 whitespace-nowrap">
                    <div class="flex items-center gap-2">
                      <div class="w-7 h-7 rounded-lg bg-blue-100 text-blue-800 flex items-center justify-center font-bold text-xs shrink-0">
                        {{ (s.cashier_name || 'K').charAt(0).toUpperCase() }}
                      </div>
                      <div>
                        <div class="font-bold text-slate-900 leading-tight">{{ s.cashier_name || '-' }}</div>
                        <div class="flex items-center gap-1.5 mt-0.5">
                          <span class="text-[10px] font-mono text-slate-500">@{{ s.cashier_username }}</span>
                          <span class="text-[10px] px-1.5 py-0.2 rounded-md bg-slate-100 text-slate-600 font-mono border border-slate-200">
                            {{ s.terminal_id }}
                          </span>
                        </div>
                      </div>
                    </div>
                  </td>

                  <!-- 3. Cabang Outlet -->
                  <td class="px-4 py-3 whitespace-nowrap">
                    <div class="font-semibold text-slate-800">{{ s.outlet_name || 'Outlet Utama' }}</div>
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
                        class="inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-[10px] font-bold bg-emerald-100 text-emerald-800 border border-emerald-300"
                      >
                        <CheckCircle2 class="w-3 h-3" />
                        <span>SESUAI</span>
                      </span>
                      <span
                        v-else-if="Number(s.discrepancy || 0) > 0"
                        class="inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-[10px] font-bold bg-amber-100 text-amber-800 border border-amber-300"
                      >
                        <span>+{{ formatCurrency(s.discrepancy) }} (Lebih)</span>
                      </span>
                      <span
                        v-else
                        class="inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-[10px] font-bold bg-rose-100 text-rose-800 border border-rose-300"
                      >
                        <AlertTriangle class="w-3 h-3" />
                        <span>{{ formatCurrency(s.discrepancy) }} (Kurang)</span>
                      </span>
                    </template>
                    <span v-else class="text-slate-400 text-[10px] italic">
                      Belum settlement
                    </span>
                  </td>

                  <!-- 8. Supervisor Otorisasi -->
                  <td class="px-4 py-3 text-center whitespace-nowrap">
                    <span v-if="s.supervisor_name" class="inline-flex items-center gap-1 px-2 py-0.5 rounded-md bg-purple-50 text-purple-700 border border-purple-200 text-[10px] font-bold">
                      <User class="w-3 h-3" />
                      <span>{{ s.supervisor_name }}</span>
                    </span>
                    <span v-else class="text-slate-400 text-[10px]">-</span>
                  </td>

                  <!-- 9. Status -->
                  <td class="px-4 py-3 text-center whitespace-nowrap">
                    <span
                      v-if="s.status === 'OPEN'"
                      class="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-[10px] font-extrabold bg-emerald-100 text-emerald-800 border border-emerald-300 shadow-2xs"
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

                  <!-- 10. Aksi -->
                  <td class="px-4 py-3 text-center whitespace-nowrap">
                    <button
                      type="button"
                      @click="openZReportModal(s.id)"
                      class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-bold text-slate-700 bg-white hover:bg-slate-100 border border-slate-300 shadow-2xs transition active:scale-95 cursor-pointer"
                      title="Lihat rincian penerimaan dan cetak Z-Report"
                    >
                      <Printer class="w-3.5 h-3.5 text-blue-600" />
                      <span>Z-Report</span>
                    </button>
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
              :totalItems="totalShifts"
              @update:currentPage="loadShifts"
              @update:pageSize="() => { currentPage = 1; loadShifts() }"
            />
          </div>
        </div>
      </div>

      <!-- 5. Modal Z-Report & Audit Settlement -->
      <Teleport to="body">
        <div
          v-if="showDetailModal"
          class="fixed inset-0 z-50 overflow-y-auto print:static print:inset-auto print:p-0 print:bg-transparent"
        >
          <!-- Dedicated Fixed Backdrop -->
          <div
            class="fixed inset-0 bg-slate-900/60 backdrop-blur-xs transition-opacity print:hidden"
            @click="showDetailModal = false"
          ></div>

          <!-- Scrollable Centered Wrapper -->
          <div class="flex min-h-full items-center justify-center p-4 text-center pointer-events-none print:p-0 print:block">
            <div
              id="printable-z-report"
              class="relative bg-white border border-slate-300 w-full max-w-xl rounded-2xl shadow-2xl flex flex-col text-slate-900 overflow-hidden max-h-[92vh] text-left pointer-events-auto my-8 print:max-w-none print:shadow-none print:border-none print:w-[80mm] print:m-0 print:p-0"
            >
              <!-- Header Modal -->
              <div class="bg-slate-50 dark:bg-slate-900 px-6 py-4 flex items-center justify-between text-slate-800 dark:text-white border-b border-slate-200 dark:border-slate-800 shrink-0 print:hidden">
                <div class="flex items-center gap-2.5">
                  <div class="w-8 h-8 rounded-lg bg-emerald-500/20 text-emerald-600 dark:text-emerald-400 border border-emerald-500/30 flex items-center justify-center font-bold">
                    <Printer class="w-4 h-4" />
                  </div>
                  <div>
                    <h4 class="font-extrabold text-sm leading-tight text-slate-900 dark:text-white">Laporan Penutupan Shift (Z-Report)</h4>
                    <p class="text-[10px] text-slate-500 dark:text-slate-400 mt-0.5">Audit rekonsiliasi kasir & breakdown penerimaan</p>
                  </div>
                </div>
                <button
                  type="button"
                  @click="showDetailModal = false"
                  class="w-8 h-8 rounded-lg text-slate-400 hover:text-slate-700 dark:hover:text-white hover:bg-slate-100 dark:hover:bg-slate-800 flex items-center justify-center transition cursor-pointer"
                >
                  <X class="w-5 h-5" />
                </button>
              </div>

          <!-- Modal Body (Scrollable) -->
          <div class="flex-1 overflow-y-auto p-6 space-y-4 print:p-1 print:max-h-none print:overflow-visible">
            <div v-if="isLoadingDetail" class="py-12 flex flex-col items-center justify-center gap-2 text-slate-400">
              <RefreshCw class="w-6 h-6 animate-spin text-blue-600" />
              <span class="text-xs font-semibold text-slate-600">Memuat rincian Z-Report...</span>
            </div>

            <template v-else-if="selectedZReport">
              <!-- Printable Header for Thermal Paper -->
              <div class="hidden print:block text-center pb-2 border-b border-dashed border-black mb-2 text-black">
                <div class="font-black text-sm uppercase">NOTANE RETAIL POS</div>
                <div class="font-bold text-xs uppercase mt-0.5">LAPORAN PENUTUPAN SHIFT (Z-REPORT)</div>
                <div class="text-[10px] mt-0.5">{{ selectedZReport.shift?.outlet_name || 'Outlet Utama' }}</div>
              </div>

              <!-- Shift Overview Card -->
              <div class="bg-slate-50 border border-slate-200 rounded-xl p-3.5 text-xs space-y-1.5 print:bg-transparent print:border-none print:p-0">
                <div class="flex justify-between text-slate-700">
                  <span class="font-semibold">Nama Kasir</span>
                  <span class="text-slate-900 font-bold">{{ selectedZReport.cashier_name }}</span>
                </div>
                <div class="flex justify-between text-slate-700">
                  <span class="font-semibold">Terminal POS</span>
                  <span class="text-slate-900 font-mono">{{ selectedZReport.shift?.terminal_id }}</span>
                </div>
                <div class="flex justify-between text-slate-700">
                  <span class="font-semibold">Cabang Toko</span>
                  <span class="text-slate-900">{{ selectedZReport.shift?.outlet_name || selectedZReport.outlet_name || '-' }}</span>
                </div>
                <div class="flex justify-between text-slate-700 pt-1.5 border-t border-slate-200">
                  <span class="font-semibold">Waktu Buka</span>
                  <span class="text-slate-900 font-mono">{{ new Date(selectedZReport.shift?.opened_at).toLocaleString('id-ID', { day:'2-digit', month:'short', year:'numeric', hour:'2-digit', minute:'2-digit' }) }}</span>
                </div>
                <div class="flex justify-between text-slate-700">
                  <span class="font-semibold">Waktu Tutup</span>
                  <span class="text-slate-900 font-mono">
                    {{ selectedZReport.shift?.closed_at ? new Date(selectedZReport.shift.closed_at).toLocaleString('id-ID', { day:'2-digit', month:'short', year:'numeric', hour:'2-digit', minute:'2-digit' }) : '(Belum Ditutup)' }}
                  </span>
                </div>
                <div class="flex justify-between text-slate-700">
                  <span class="font-semibold">Jumlah Transaksi Selesai</span>
                  <span class="text-slate-900 font-bold font-mono">{{ selectedZReport.total_sales_count ?? selectedZReport.transaction_count ?? 0 }} nota</span>
                </div>
              </div>

              <!-- Payment Method Breakdown -->
              <div>
                <h5 class="text-[11px] font-extrabold text-slate-600 uppercase tracking-wider mb-2">Rekapitulasi Cara Bayar</h5>
                <div class="border border-slate-200 rounded-xl overflow-hidden print:border-none">
                  <template v-if="selectedZReport.payment_breakdowns && selectedZReport.payment_breakdowns.length > 0">
                    <div
                      v-for="(pm, idx) in selectedZReport.payment_breakdowns"
                      :key="pm.category || pm.payment_method_id || idx"
                      class="flex items-center justify-between px-3.5 py-2 text-xs"
                      :class="idx % 2 === 0 ? 'bg-white' : 'bg-slate-50'"
                    >
                      <div class="flex items-center gap-2">
                        <span class="text-sm">{{ paymentIcon(pm.category) }}</span>
                        <span class="text-slate-700 font-semibold">{{ pm.payment_method_name }}</span>
                        <span v-if="pm.transaction_count" class="text-slate-400 text-[10px]">({{ pm.transaction_count }}x)</span>
                      </div>
                      <span class="font-bold text-slate-900 font-mono">{{ formatCurrency(pm.total_amount) }}</span>
                    </div>
                  </template>
                  <template v-else>
                    <div class="flex items-center justify-between px-3.5 py-2 text-xs bg-white">
                      <div class="flex items-center gap-2">
                        <span>💵</span>
                        <span class="text-slate-700 font-medium">Uang Tunai (Cash)</span>
                      </div>
                      <span class="font-bold text-slate-900 font-mono">{{ formatCurrency(selectedZReport.total_cash_received ?? 0) }}</span>
                    </div>
                  </template>
                </div>
              </div>

              <!-- Drawer Cash Reconciliation (Audit Laci) -->
              <div>
                <h5 class="text-[11px] font-extrabold text-slate-600 uppercase tracking-wider mb-2">Rekonsiliasi Kas Laci (Drawer Audit)</h5>
                <div class="border border-slate-200 rounded-xl divide-y divide-slate-100 text-xs bg-white print:border-none">
                  <div class="flex justify-between px-3.5 py-2 text-slate-700">
                    <span>Modal Awal Kasir</span>
                    <span class="font-mono font-semibold">{{ formatCurrency(selectedZReport.shift?.opening_cash || selectedZReport.opening_cash || 0) }}</span>
                  </div>
                  <div class="flex justify-between px-3.5 py-2 text-slate-700">
                    <span>Penjualan Tunai Bersih (+)</span>
                    <span class="font-mono font-semibold text-emerald-700">{{ formatCurrency(selectedZReport.total_cash_received ?? selectedZReport.cash_sales ?? 0) }}</span>
                  </div>
                  <div v-if="Number(selectedZReport.shift_cash_drops || selectedZReport.cash_drops || 0) > 0" class="flex justify-between px-3.5 py-2 text-slate-700">
                    <span>Setor Kas ke Brankas (-)</span>
                    <span class="font-mono font-semibold text-rose-600">-{{ formatCurrency(selectedZReport.shift_cash_drops || selectedZReport.cash_drops || 0) }}</span>
                  </div>
                  <div v-if="Number(selectedZReport.shift_expenses || selectedZReport.expenses || 0) > 0" class="flex justify-between px-3.5 py-2 text-slate-700">
                    <span>Pengeluaran Kas Operasional (-)</span>
                    <span class="font-mono font-semibold text-rose-600">-{{ formatCurrency(selectedZReport.shift_expenses || selectedZReport.expenses || 0) }}</span>
                  </div>
                  <div v-if="Number(selectedZReport.shift_sales_returns || selectedZReport.sales_returns || 0) > 0" class="flex justify-between px-3.5 py-2 text-slate-700">
                    <span>Pengembalian Retur Kas (-)</span>
                    <span class="font-mono font-semibold text-rose-600">-{{ formatCurrency(selectedZReport.shift_sales_returns || selectedZReport.sales_returns || 0) }}</span>
                  </div>
                  <div class="flex justify-between px-3.5 py-2 bg-slate-50 text-slate-900 font-bold border-t border-slate-200">
                    <span>Kas Diharapkan Sistem</span>
                    <span class="font-mono">{{ formatCurrency(selectedZReport.shift?.expected_cash ?? selectedZReport.expected_cash ?? 0) }}</span>
                  </div>
                  <div class="flex justify-between px-3.5 py-2 bg-slate-50 text-slate-900 font-bold">
                    <span>Uang Fisik Dihitung (Actual)</span>
                    <span class="font-mono">{{ formatCurrency(selectedZReport.shift?.actual_cash ?? selectedZReport.actual_cash ?? 0) }}</span>
                  </div>
                  <div
                    class="flex justify-between px-3.5 py-2 font-extrabold text-sm"
                    :class="Number(selectedZReport.shift?.discrepancy ?? selectedZReport.discrepancy ?? 0) === 0 ? 'bg-emerald-50 text-emerald-800' : Number(selectedZReport.shift?.discrepancy ?? selectedZReport.discrepancy ?? 0) < 0 ? 'bg-rose-50 text-rose-800' : 'bg-amber-50 text-amber-800'"
                  >
                    <span>Selisih Laci Kas</span>
                    <span class="font-mono">
                      {{ Number(selectedZReport.shift?.discrepancy ?? selectedZReport.discrepancy ?? 0) > 0 ? '+' : '' }}{{ formatCurrency(selectedZReport.shift?.discrepancy ?? selectedZReport.discrepancy ?? 0) }}
                    </span>
                  </div>
                </div>
              </div>

              <!-- Supervisor Approval Notice -->
              <div v-if="selectedZReport.supervisor_name" class="p-3 bg-purple-50 border border-purple-200 rounded-xl text-purple-900 text-xs flex items-center justify-between">
                <div>
                  <div class="font-bold">Otorisasi Supervisor</div>
                  <div class="text-[11px] text-purple-700 mt-0.5">Ditutup atas izin: {{ selectedZReport.supervisor_name }}</div>
                </div>
                <div class="p-1 rounded-md bg-purple-200 text-purple-800">
                  <CheckCircle2 class="w-4 h-4" />
                </div>
              </div>

              <!-- Thermal Print Status -->
              <div v-if="printStatusMsg" class="p-3 rounded-xl text-xs font-semibold text-center" :class="printStatusMsg.includes('Berhasil') ? 'bg-emerald-50 text-emerald-700 border border-emerald-200' : 'bg-amber-50 text-amber-700 border border-amber-200'">
                {{ printStatusMsg }}
              </div>
            </template>
          </div>

          <!-- Footer Actions -->
          <div class="bg-slate-100 p-4 px-6 border-t border-slate-300 flex items-center justify-between gap-3 shrink-0 print:hidden">
            <button
              type="button"
              @click="showDetailModal = false"
              class="bg-slate-200 hover:bg-slate-300 text-slate-700 font-bold px-4 py-2 rounded-xl text-xs border border-slate-300 transition active:scale-95 cursor-pointer"
            >
              Tutup
            </button>

            <div class="flex items-center gap-2">
              <button
                type="button"
                @click="handlePrintSystem"
                class="bg-white hover:bg-slate-50 text-slate-700 font-bold px-4 py-2 rounded-xl text-xs border border-slate-300 transition active:scale-95 cursor-pointer flex items-center gap-1.5 shadow-2xs"
              >
                <Printer class="w-3.5 h-3.5" />
                <span>Cetak Sistem (A4/PDF)</span>
              </button>

              <button
                type="button"
                @click="handlePrintThermal"
                :disabled="isPrinting || !selectedZReport"
                class="bg-blue-600 hover:bg-blue-700 text-white font-bold px-4 py-2 rounded-xl text-xs transition active:scale-95 cursor-pointer flex items-center gap-1.5 shadow-xs disabled:opacity-50"
              >
                <Printer class="w-3.5 h-3.5" />
                <span>{{ isPrinting ? 'Mencetak...' : 'Cetak Thermal POS (Z-Report)' }}</span>
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
    </div>
  </AdminLayout>
</template>
