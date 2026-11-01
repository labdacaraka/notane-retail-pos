<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import AdminLayout from '@/layouts/AdminLayout.vue'
import SearchableSelect from '@/components/common/SearchableSelect.vue'
import {
  RefreshCw,
  FileSpreadsheet,
  Printer,
  TrendingUp,
  TrendingDown,
  ShoppingBag,
  CreditCard,
  Building2,
  DollarSign,
  PieChart,
  BarChart3,
  Receipt,
  Layers,
  Search,
  ArrowUpDown,
  ChevronLeft,
  ChevronRight,
  Lock,
} from 'lucide-vue-next'
import { api } from '@/services/api'
import { useLocationStore } from '@/stores/locationStore'
import { useLicenseStore } from '@/stores/licenseStore'
import { formatCurrency } from '@/utils/formatters'
import * as XLSX from 'xlsx'
import type {
  AnalyticsOverviewResponse,
  CategoryAnalyticsResponse,
  DailyLedgerAnalyticsResponse,
  DailyTrendPoint,
  ProductsAnalyticsResponse,
} from '@/types'

const locationStore = useLocationStore()
const licenseStore = useLicenseStore()

// State Filters Global
const selectedOutletId = ref<string>(locationStore.currentOutletId || '')
const datePreset = ref<'today' | '7days' | '30days' | 'month' | 'last_month' | 'year' | 'custom'>('30days')
const startDate = ref<string>('')
const endDate = ref<string>('')
const isLicenseUnlocked = computed(() => licenseStore.hasFeature('analytics'))

// Tab State
const activeTab = ref<'overview' | 'sales' | 'expenses' | 'daily_table'>('overview')

// Tab 1: Overview State
const analyticsData = ref<AnalyticsOverviewResponse | null>(null)
const isOverviewLoading = ref<boolean>(false)

// Chart Toggle Series
const showSeriesSales = ref<boolean>(true)
const showSeriesPurchases = ref<boolean>(true)
const showSeriesExpenses = ref<boolean>(true)
const showSeriesOtherIncome = ref<boolean>(true)
const showSeriesNetProfit = ref<boolean>(true)
const hoveredPointIndex = ref<number | null>(null)

// Tab 2: Sales & Top Products State (Server-Side)
const productsData = ref<ProductsAnalyticsResponse | null>(null)
const isProductsLoading = ref<boolean>(false)
const productsSearch = ref<string>('')
const productsSortBy = ref<'sales' | 'qty' | 'profit' | 'margin' | 'name'>('sales')
const productsSortOrder = ref<'asc' | 'desc'>('desc')
const productsPage = ref<number>(1)
const productsPerPage = ref<number>(10)

// Tab 3: Expenses & Other Income Breakdown State (Server-Side)
const expensesData = ref<CategoryAnalyticsResponse | null>(null)
const isExpensesLoading = ref<boolean>(false)
const expensesSearch = ref<string>('')
const expensesSortBy = ref<'amount' | 'name' | 'count'>('amount')
const expensesSortOrder = ref<'asc' | 'desc'>('desc')
const expensesPage = ref<number>(1)
const expensesPerPage = ref<number>(10)

const otherIncomeData = ref<CategoryAnalyticsResponse | null>(null)
const isOtherIncomeLoading = ref<boolean>(false)
const otherIncomeSearch = ref<string>('')
const otherIncomeSortBy = ref<'amount' | 'name' | 'count'>('amount')
const otherIncomeSortOrder = ref<'asc' | 'desc'>('desc')
const otherIncomePage = ref<number>(1)
const otherIncomePerPage = ref<number>(10)

// Tab 4: Daily Ledger Table State (Server-Side)
const dailyData = ref<DailyLedgerAnalyticsResponse | null>(null)
const isDailyLoading = ref<boolean>(false)
const dailySearch = ref<string>('')
const dailySortBy = ref<string>('date')
const dailySortOrder = ref<'asc' | 'desc'>('desc')
const dailyPage = ref<number>(1)
const dailyPerPage = ref<number>(15)

// Debounce timer
let searchDebounceTimeout: any = null

function applyDatePreset(preset: 'today' | '7days' | '30days' | 'month' | 'last_month' | 'year' | 'custom') {
  datePreset.value = preset
  const today = new Date()
  const todayStr = today.toISOString().split('T')[0]

  if (preset === 'today') {
    startDate.value = todayStr
    endDate.value = todayStr
  } else if (preset === '7days') {
    const past7 = new Date()
    past7.setDate(today.getDate() - 6)
    startDate.value = past7.toISOString().split('T')[0]
    endDate.value = todayStr
  } else if (preset === '30days') {
    const past30 = new Date()
    past30.setDate(today.getDate() - 29)
    startDate.value = past30.toISOString().split('T')[0]
    endDate.value = todayStr
  } else if (preset === 'month') {
    const firstDay = new Date(today.getFullYear(), today.getMonth(), 1)
    startDate.value = firstDay.toISOString().split('T')[0]
    endDate.value = todayStr
  } else if (preset === 'last_month') {
    const firstDayLastMonth = new Date(today.getFullYear(), today.getMonth() - 1, 1)
    const lastDayLastMonth = new Date(today.getFullYear(), today.getMonth(), 0)
    startDate.value = firstDayLastMonth.toISOString().split('T')[0]
    endDate.value = lastDayLastMonth.toISOString().split('T')[0]
  } else if (preset === 'year') {
    const firstDayYear = new Date(today.getFullYear(), 0, 1)
    startDate.value = firstDayYear.toISOString().split('T')[0]
    endDate.value = todayStr
  }

  if (preset !== 'custom') {
    refreshAll()
  }
}

// 1. Fetch Overview (Tab 1)
async function fetchOverview() {
  if (!isLicenseUnlocked.value) return
  isOverviewLoading.value = true
  try {
    const params: Record<string, any> = {
      start_date: startDate.value,
      end_date: endDate.value,
    }
    if (selectedOutletId.value) {
      params.outlet_id = selectedOutletId.value
    }

    const res = await api.get('/analytics/overview', { params })
    if (res.data?.success) {
      analyticsData.value = res.data.data
    }
  } catch (err) {
    console.error('Gagal memuat analitik ikhtisar:', err)
  } finally {
    isOverviewLoading.value = false
  }
}

// 2. Fetch Products Performance (Tab 2 Server-Side)
async function fetchProducts() {
  if (!isLicenseUnlocked.value) return
  isProductsLoading.value = true
  try {
    const params: Record<string, any> = {
      start_date: startDate.value,
      end_date: endDate.value,
      page: productsPage.value,
      per_page: productsPerPage.value,
      sort_by: productsSortBy.value,
      sort_order: productsSortOrder.value,
    }
    if (selectedOutletId.value) {
      params.outlet_id = selectedOutletId.value
    }
    if (productsSearch.value.trim()) {
      params.search = productsSearch.value.trim()
    }

    const res = await api.get('/analytics/products', { params })
    if (res.data?.success) {
      productsData.value = res.data.data
    }
  } catch (err) {
    console.error('Gagal memuat analitik produk:', err)
  } finally {
    isProductsLoading.value = false
  }
}

// 3. Fetch Expenses Breakdown (Tab 3 Server-Side)
async function fetchExpenses() {
  if (!isLicenseUnlocked.value) return
  isExpensesLoading.value = true
  try {
    const params: Record<string, any> = {
      start_date: startDate.value,
      end_date: endDate.value,
      page: expensesPage.value,
      per_page: expensesPerPage.value,
      sort_by: expensesSortBy.value,
      sort_order: expensesSortOrder.value,
    }
    if (selectedOutletId.value) {
      params.outlet_id = selectedOutletId.value
    }
    if (expensesSearch.value.trim()) {
      params.search = expensesSearch.value.trim()
    }

    const res = await api.get('/analytics/expenses', { params })
    if (res.data?.success) {
      expensesData.value = res.data.data
    }
  } catch (err) {
    console.error('Gagal memuat kategori beban:', err)
  } finally {
    isExpensesLoading.value = false
  }
}

// 4. Fetch Other Income Breakdown (Tab 3 Server-Side)
async function fetchOtherIncome() {
  if (!isLicenseUnlocked.value) return
  isOtherIncomeLoading.value = true
  try {
    const params: Record<string, any> = {
      start_date: startDate.value,
      end_date: endDate.value,
      page: otherIncomePage.value,
      per_page: otherIncomePerPage.value,
      sort_by: otherIncomeSortBy.value,
      sort_order: otherIncomeSortOrder.value,
    }
    if (selectedOutletId.value) {
      params.outlet_id = selectedOutletId.value
    }
    if (otherIncomeSearch.value.trim()) {
      params.search = otherIncomeSearch.value.trim()
    }

    const res = await api.get('/analytics/other-income', { params })
    if (res.data?.success) {
      otherIncomeData.value = res.data.data
    }
  } catch (err) {
    console.error('Gagal memuat kategori pendapatan lain:', err)
  } finally {
    isOtherIncomeLoading.value = false
  }
}

// 5. Fetch Daily Ledger (Tab 4 Server-Side)
async function fetchDailyLedger() {
  if (!isLicenseUnlocked.value) return
  isDailyLoading.value = true
  try {
    const params: Record<string, any> = {
      start_date: startDate.value,
      end_date: endDate.value,
      page: dailyPage.value,
      per_page: dailyPerPage.value,
      sort_by: dailySortBy.value,
      sort_order: dailySortOrder.value,
    }
    if (selectedOutletId.value) {
      params.outlet_id = selectedOutletId.value
    }
    if (dailySearch.value.trim()) {
      params.search = dailySearch.value.trim()
    }

    const res = await api.get('/analytics/daily-ledger', { params })
    if (res.data?.success) {
      dailyData.value = res.data.data
    }
  } catch (err) {
    console.error('Gagal memuat buku harian finansial:', err)
  } finally {
    isDailyLoading.value = false
  }
}

// Load data based on active tab
function loadActiveTab() {
  if (activeTab.value === 'overview') {
    fetchOverview()
  } else if (activeTab.value === 'sales') {
    fetchProducts()
    if (!analyticsData.value) fetchOverview() // for payment method breakdown
  } else if (activeTab.value === 'expenses') {
    fetchExpenses()
    fetchOtherIncome()
    if (!analyticsData.value) fetchOverview() // for total summary
  } else if (activeTab.value === 'daily_table') {
    fetchDailyLedger()
  }
}

function refreshAll() {
  productsPage.value = 1
  expensesPage.value = 1
  otherIncomePage.value = 1
  dailyPage.value = 1
  loadActiveTab()
}

// Tab Switching
watch(activeTab, () => {
  loadActiveTab()
})

// Search Watchers with Debounce
watch(productsSearch, () => {
  clearTimeout(searchDebounceTimeout)
  searchDebounceTimeout = setTimeout(() => {
    productsPage.value = 1
    fetchProducts()
  }, 350)
})

watch(expensesSearch, () => {
  clearTimeout(searchDebounceTimeout)
  searchDebounceTimeout = setTimeout(() => {
    expensesPage.value = 1
    fetchExpenses()
  }, 350)
})

watch(otherIncomeSearch, () => {
  clearTimeout(searchDebounceTimeout)
  searchDebounceTimeout = setTimeout(() => {
    otherIncomePage.value = 1
    fetchOtherIncome()
  }, 350)
})

watch(dailySearch, () => {
  clearTimeout(searchDebounceTimeout)
  searchDebounceTimeout = setTimeout(() => {
    dailyPage.value = 1
    fetchDailyLedger()
  }, 350)
})

// Sorting Toggles
function toggleProductsSort(field: 'sales' | 'qty' | 'profit' | 'margin' | 'name') {
  if (productsSortBy.value === field) {
    productsSortOrder.value = productsSortOrder.value === 'desc' ? 'asc' : 'desc'
  } else {
    productsSortBy.value = field
    productsSortOrder.value = 'desc'
  }
  productsPage.value = 1
  fetchProducts()
}

function toggleDailySort(field: string) {
  if (dailySortBy.value === field) {
    dailySortOrder.value = dailySortOrder.value === 'desc' ? 'asc' : 'desc'
  } else {
    dailySortBy.value = field
    dailySortOrder.value = 'desc'
  }
  dailyPage.value = 1
  fetchDailyLedger()
}

// Chart Calculations
const trends = computed<DailyTrendPoint[]>(() => analyticsData.value?.trends || [])

const maxChartValue = computed(() => {
  if (trends.value.length === 0) return 1000000
  let max = 0
  for (const t of trends.value) {
    if (showSeriesSales.value && t.net_sales > max) max = t.net_sales
    if (showSeriesPurchases.value && t.purchases > max) max = t.purchases
    if (showSeriesExpenses.value && t.expenses > max) max = t.expenses
    if (showSeriesOtherIncome.value && t.other_income > max) max = t.other_income
    if (showSeriesNetProfit.value && t.net_profit > max) max = t.net_profit
  }
  return max > 0 ? max * 1.15 : 1000000
})

const chartWidth = 900
const chartHeight = 280
const chartPadding = { top: 20, right: 30, bottom: 40, left: 70 }

const chartInnerWidth = computed(() => chartWidth - chartPadding.left - chartPadding.right)
const chartInnerHeight = computed(() => chartHeight - chartPadding.top - chartPadding.bottom)

function getX(index: number): number {
  if (trends.value.length <= 1) return chartPadding.left + chartInnerWidth.value / 2
  return chartPadding.left + (index / (trends.value.length - 1)) * chartInnerWidth.value
}

function getY(value: number): number {
  const safeVal = Math.max(0, value)
  const ratio = safeVal / maxChartValue.value
  return chartPadding.top + chartInnerHeight.value - ratio * chartInnerHeight.value
}

function createPath(accessor: (d: DailyTrendPoint) => number): string {
  if (trends.value.length === 0) return ''
  return trends.value.reduce((path, d, i) => {
    const x = getX(i)
    const y = getY(accessor(d))
    return i === 0 ? `M ${x} ${y}` : `${path} L ${x} ${y}`
  }, '')
}

function createAreaPath(accessor: (d: DailyTrendPoint) => number): string {
  if (trends.value.length === 0) return ''
  const line = createPath(accessor)
  const startX = getX(0)
  const endX = getX(trends.value.length - 1)
  const bottomY = chartPadding.top + chartInnerHeight.value
  return `${line} L ${endX} ${bottomY} L ${startX} ${bottomY} Z`
}

const salesPath = computed(() => createPath((d) => d.net_sales))
const salesAreaPath = computed(() => createAreaPath((d) => d.net_sales))
const purchasesPath = computed(() => createPath((d) => d.purchases))
const expensesPath = computed(() => createPath((d) => d.expenses))
const otherIncomePath = computed(() => createPath((d) => d.other_income))
const netProfitPath = computed(() => createPath((d) => d.net_profit))

const yAxisTicks = computed(() => {
  const steps = 4
  const ticks = []
  for (let i = 0; i <= steps; i++) {
    const val = (maxChartValue.value / steps) * i
    const y = getY(val)
    ticks.push({ val, y })
  }
  return ticks
})

const xAxisTicks = computed(() => {
  if (trends.value.length === 0) return []
  const maxLabels = 7
  const step = Math.max(1, Math.floor(trends.value.length / maxLabels))
  const ticks = []
  for (let i = 0; i < trends.value.length; i += step) {
    ticks.push({
      index: i,
      x: getX(i),
      label: trends.value[i].date.substring(5),
    })
  }
  return ticks
})

// Ekspor Excel (.xlsx) Lengkap
async function exportToExcel() {
  try {
    // Ambil deret lengkap jika perlu
    let dataToExport = dailyData.value?.items || []
    if (dataToExport.length === 0 && trends.value.length > 0) {
      dataToExport = trends.value
    }

    if (dataToExport.length === 0) {
      return
    }

    const rows = dataToExport.map((t, idx) => ({
      No: idx + 1,
      Tanggal: t.date,
      Hari: t.day_name,
      'Penjualan Bersih (Rp)': t.net_sales,
      'HPP Modal (Rp)': t.cogs,
      'Laba Kotor (Rp)': t.gross_profit,
      'Pembelian Stok (Rp)': t.purchases,
      'Beban Kasir (Rp)': t.expenses,
      'Pendapatan Lain (Rp)': t.other_income,
      'Laba Bersih (Rp)': t.net_profit,
      'Total Transaksi': t.transaction_count,
    }))

    const ws = XLSX.utils.json_to_sheet(rows)
    const wb = XLSX.utils.book_new()
    XLSX.utils.book_append_sheet(wb, ws, 'Buku Harian Finansial')

    const dateRangeLabel = `${startDate.value}_sd_${endDate.value}`
    XLSX.writeFile(wb, `Laporan_Analitik_Finansial_${dateRangeLabel}.xlsx`)
  } catch (e) {
    console.error('Gagal mengekspor file Excel:', e)
  }
}

function handlePrint() {
  window.print()
}

onMounted(() => {
  applyDatePreset('30days')
})
</script>

<template>
  <AdminLayout>
    <div class="p-6 space-y-4 text-slate-800 dark:text-slate-100 select-none pb-12">
      <!-- HEADER WORKSPACE & TOOLBAR DESKTOP -->
      <div class="bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 rounded-xl p-3.5 shadow-xs flex flex-wrap items-center justify-between gap-3">
        <!-- Judul & Breadcrumb -->
        <div class="flex items-center gap-2.5">
          <div class="w-8 h-8 rounded-lg bg-emerald-600/10 text-emerald-600 dark:bg-emerald-500/20 dark:text-emerald-400 flex items-center justify-center">
            <BarChart3 class="w-4 h-4" />
          </div>
          <div>
            <h1 class="text-sm font-bold tracking-tight text-slate-900 dark:text-white flex items-center gap-2">
              <span>Statistik & Analisis Bisnis</span>
              <span class="text-[10px] font-mono font-semibold px-2 py-0.5 rounded bg-emerald-100 text-emerald-800 dark:bg-emerald-950/80 dark:text-emerald-300 border border-emerald-300 dark:border-emerald-700">
                ERP Analytics
              </span>
            </h1>
            <p class="text-[11px] text-slate-500 dark:text-slate-400">
              Konsolidasi performa finansial, margin laba, arus beban operasional, dan buku harian ritel.
            </p>
          </div>
        </div>

        <!-- Filter Presets & Controls -->
        <div class="flex flex-wrap items-center gap-2">
          <!-- Outlet Selector -->
          <div class="w-48">
            <SearchableSelect
              v-model="selectedOutletId"
              :options="[
                { value: '', label: 'Semua Cabang (Konsolidasi)' },
                ...locationStore.accessibleOutlets.map(o => ({ value: o.id, label: `${o.code} - ${o.name}` }))
              ]"
              placeholder="Pilih Cabang..."
              @change="refreshAll"
            />
          </div>

          <!-- Quick Date Range Presets -->
          <div class="inline-flex rounded-lg border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-900/60 p-0.5 text-xs font-semibold text-slate-600 dark:text-slate-300">
            <button
              type="button"
              @click="applyDatePreset('today')"
              class="px-2 py-1 rounded transition"
              :class="datePreset === 'today' ? 'bg-white dark:bg-slate-700 text-emerald-600 dark:text-emerald-400 shadow-xs font-bold' : 'hover:text-slate-900'"
            >
              Hari Ini
            </button>
            <button
              type="button"
              @click="applyDatePreset('7days')"
              class="px-2 py-1 rounded transition"
              :class="datePreset === '7days' ? 'bg-white dark:bg-slate-700 text-emerald-600 dark:text-emerald-400 shadow-xs font-bold' : 'hover:text-slate-900'"
            >
              7 Hari
            </button>
            <button
              type="button"
              @click="applyDatePreset('30days')"
              class="px-2 py-1 rounded transition"
              :class="datePreset === '30days' ? 'bg-white dark:bg-slate-700 text-emerald-600 dark:text-emerald-400 shadow-xs font-bold' : 'hover:text-slate-900'"
            >
              30 Hari
            </button>
            <button
              type="button"
              @click="applyDatePreset('month')"
              class="px-2 py-1 rounded transition"
              :class="datePreset === 'month' ? 'bg-white dark:bg-slate-700 text-emerald-600 dark:text-emerald-400 shadow-xs font-bold' : 'hover:text-slate-900'"
            >
              Bulan Ini
            </button>
            <button
              type="button"
              @click="applyDatePreset('year')"
              class="px-2 py-1 rounded transition"
              :class="datePreset === 'year' ? 'bg-white dark:bg-slate-700 text-emerald-600 dark:text-emerald-400 shadow-xs font-bold' : 'hover:text-slate-900'"
            >
              Tahun Ini
            </button>
          </div>

          <!-- Datepicker Rentang Kustom -->
          <div class="flex items-center gap-1.5 text-xs">
            <input
              type="date"
              v-model="startDate"
              @change="applyDatePreset('custom')"
              class="px-2 py-1 border border-slate-200 dark:border-slate-700 rounded-lg bg-white dark:bg-slate-800 text-slate-800 dark:text-slate-100 font-mono text-xs focus:ring-1 focus:ring-emerald-500"
            />
            <span class="text-slate-400 font-bold">s/d</span>
            <input
              type="date"
              v-model="endDate"
              @change="applyDatePreset('custom')"
              class="px-2 py-1 border border-slate-200 dark:border-slate-700 rounded-lg bg-white dark:bg-slate-800 text-slate-800 dark:text-slate-100 font-mono text-xs focus:ring-1 focus:ring-emerald-500"
            />
          </div>

          <!-- Tombol Refresh & Cetak -->
          <div class="flex items-center gap-1 border-l border-slate-200 dark:border-slate-700 pl-2">
            <button
              type="button"
              @click="refreshAll"
              title="Perbarui Data"
              class="p-1.5 rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 text-slate-600 dark:text-slate-300 hover:bg-slate-50 dark:hover:bg-slate-700 transition cursor-pointer"
            >
              <RefreshCw class="w-3.5 h-3.5" :class="{ 'animate-spin': isOverviewLoading || isProductsLoading || isDailyLoading }" />
            </button>
            <button
              type="button"
              @click="exportToExcel"
              title="Ekspor Spreadsheet (.xlsx)"
              class="p-1.5 rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 text-emerald-600 hover:bg-emerald-50 dark:hover:bg-slate-700 transition cursor-pointer"
            >
              <FileSpreadsheet class="w-3.5 h-3.5" />
            </button>
            <button
              type="button"
              @click="handlePrint"
              title="Cetak Laporan / PDF"
              class="p-1.5 rounded-lg border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 text-slate-600 dark:text-slate-300 hover:bg-slate-50 dark:hover:bg-slate-700 transition cursor-pointer"
            >
              <Printer class="w-3.5 h-3.5" />
            </button>
          </div>
        </div>
      </div>

      <!-- BANNER JIKA LISENSI BELUM DIAKTIFKAN -->
      <div
        v-if="!isLicenseUnlocked"
        class="bg-amber-50 dark:bg-amber-950/40 border border-amber-300 dark:border-amber-700 p-4 rounded-xl flex items-start gap-3"
      >
        <Lock class="w-5 h-5 text-amber-600 shrink-0 mt-0.5" />
        <div>
          <h2 class="text-xs font-bold text-amber-900 dark:text-amber-200">
            Modul Statistik & Analisis Bisnis Memerlukan Aktivasi Lisensi
          </h2>
          <p class="text-xs text-amber-800/90 dark:text-amber-300/80 mt-0.5">
            Fitur modul "Statistik & Analisis Bisnis" (analytics) saat ini belum diaktifkan pada lisensi server retail Anda.
            Silakan hubungi Provider Notane Retail POS melalui Notane Manager untuk menambahkan modul ini ke paket lisensi toko Anda.
          </p>
        </div>
      </div>

      <!-- KARTU METRIK UTAMA / KPI SCORECARD -->
      <div v-if="isLicenseUnlocked" class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-3 print:grid-cols-3">
        <!-- 1. Net Sales -->
        <div class="bg-white dark:bg-slate-800 rounded-xl p-3 border border-slate-200 dark:border-slate-700 shadow-xs flex flex-col justify-between">
          <div class="flex items-center justify-between text-slate-500 dark:text-slate-400">
            <span class="text-[11px] font-bold uppercase tracking-wider">Omzet Bersih</span>
            <TrendingUp class="w-3.5 h-3.5 text-emerald-600" />
          </div>
          <div class="mt-2">
            <div class="text-base font-extrabold font-mono text-slate-900 dark:text-white truncate">
              {{ formatCurrency(analyticsData?.sales.net_sales || 0) }}
            </div>
            <div class="text-[10px] text-slate-400 font-mono mt-0.5">
              {{ analyticsData?.sales.transaction_count || 0 }} nota • AOV {{ formatCurrency(analyticsData?.sales.average_order_value || 0) }}
            </div>
          </div>
        </div>

        <!-- 2. Gross Profit -->
        <div class="bg-white dark:bg-slate-800 rounded-xl p-3 border border-slate-200 dark:border-slate-700 shadow-xs flex flex-col justify-between">
          <div class="flex items-center justify-between text-slate-500 dark:text-slate-400">
            <span class="text-[11px] font-bold uppercase tracking-wider">Laba Kotor</span>
            <DollarSign class="w-3.5 h-3.5 text-teal-600" />
          </div>
          <div class="mt-2">
            <div class="text-base font-extrabold font-mono text-teal-700 dark:text-teal-400 truncate">
              {{ formatCurrency(analyticsData?.sales.gross_profit || 0) }}
            </div>
            <div class="text-[10px] text-slate-400 font-mono mt-0.5">
              Margin: <span class="font-bold text-slate-700 dark:text-slate-300">{{ analyticsData?.sales.gross_margin_pct || 0 }}%</span>
            </div>
          </div>
        </div>

        <!-- 3. Expenses -->
        <div class="bg-white dark:bg-slate-800 rounded-xl p-3 border border-slate-200 dark:border-slate-700 shadow-xs flex flex-col justify-between">
          <div class="flex items-center justify-between text-slate-500 dark:text-slate-400">
            <span class="text-[11px] font-bold uppercase tracking-wider">Beban Kasir</span>
            <TrendingDown class="w-3.5 h-3.5 text-rose-600" />
          </div>
          <div class="mt-2">
            <div class="text-base font-extrabold font-mono text-rose-700 dark:text-rose-400 truncate">
              {{ formatCurrency(analyticsData?.expenses.total_expenses || 0) }}
            </div>
            <div class="text-[10px] text-slate-400 font-mono mt-0.5">
              {{ analyticsData?.expenses.expense_count || 0 }} transaksi bukti kasir
            </div>
          </div>
        </div>

        <!-- 4. Purchases -->
        <div class="bg-white dark:bg-slate-800 rounded-xl p-3 border border-slate-200 dark:border-slate-700 shadow-xs flex flex-col justify-between">
          <div class="flex items-center justify-between text-slate-500 dark:text-slate-400">
            <span class="text-[11px] font-bold uppercase tracking-wider">Beli Stok</span>
            <Building2 class="w-3.5 h-3.5 text-blue-600" />
          </div>
          <div class="mt-2">
            <div class="text-base font-extrabold font-mono text-blue-700 dark:text-blue-400 truncate">
              {{ formatCurrency(analyticsData?.purchases.total_purchases || 0) }}
            </div>
            <div class="text-[10px] text-slate-400 font-mono mt-0.5">
              {{ analyticsData?.purchases.purchase_count || 0 }} faktur pembelian
            </div>
          </div>
        </div>

        <!-- 5. Other Income -->
        <div class="bg-white dark:bg-slate-800 rounded-xl p-3 border border-slate-200 dark:border-slate-700 shadow-xs flex flex-col justify-between">
          <div class="flex items-center justify-between text-slate-500 dark:text-slate-400">
            <span class="text-[11px] font-bold uppercase tracking-wider">Pendapatan Lain</span>
            <Layers class="w-3.5 h-3.5 text-emerald-600" />
          </div>
          <div class="mt-2">
            <div class="text-base font-extrabold font-mono text-emerald-700 dark:text-emerald-400 truncate">
              {{ formatCurrency(analyticsData?.other_income.total_other_income || 0) }}
            </div>
            <div class="text-[10px] text-slate-400 font-mono mt-0.5">
              {{ analyticsData?.other_income.other_income_count || 0 }} transaksi non-sales
            </div>
          </div>
        </div>

        <!-- 6. Net Profit -->
        <div class="bg-white dark:bg-slate-800 rounded-xl p-3 border border-slate-200 dark:border-slate-700 shadow-xs flex flex-col justify-between bg-emerald-50/30 dark:bg-emerald-950/20">
          <div class="flex items-center justify-between text-slate-500 dark:text-slate-400">
            <span class="text-[11px] font-bold uppercase tracking-wider">Laba Bersih</span>
            <BarChart3 class="w-3.5 h-3.5 text-indigo-600" />
          </div>
          <div class="mt-2">
            <div
              class="text-base font-extrabold font-mono truncate"
              :class="(analyticsData?.profit.net_operating_profit || 0) >= 0 ? 'text-indigo-700 dark:text-indigo-400' : 'text-rose-600'"
            >
              {{ formatCurrency(analyticsData?.profit.net_operating_profit || 0) }}
            </div>
            <div class="text-[10px] text-slate-400 font-mono mt-0.5">
              Net Margin: <span class="font-bold text-slate-700 dark:text-slate-300">{{ analyticsData?.profit.net_margin_pct || 0 }}%</span>
            </div>
          </div>
        </div>
      </div>

      <!-- TABS NAVIGATION (DESKTOP WORKSPACE) -->
      <div v-if="isLicenseUnlocked" class="border-b border-slate-200 dark:border-slate-700 print:hidden">
        <nav class="flex gap-4 -mb-px">
          <button
            type="button"
            @click="activeTab = 'overview'"
            class="py-2.5 px-1 text-xs font-bold border-b-2 transition flex items-center gap-1.5 cursor-pointer"
            :class="[
              activeTab === 'overview'
                ? 'border-emerald-600 text-emerald-600 dark:text-emerald-400 dark:border-emerald-400'
                : 'border-transparent text-slate-500 hover:text-slate-700 dark:hover:text-slate-300'
            ]"
          >
            <BarChart3 class="w-3.5 h-3.5" />
            <span>Ikhtisar Tren & Grafik Finansial</span>
          </button>

          <button
            type="button"
            @click="activeTab = 'sales'"
            class="py-2.5 px-1 text-xs font-bold border-b-2 transition flex items-center gap-1.5 cursor-pointer"
            :class="[
              activeTab === 'sales'
                ? 'border-emerald-600 text-emerald-600 dark:text-emerald-400 dark:border-emerald-400'
                : 'border-transparent text-slate-500 hover:text-slate-700 dark:hover:text-slate-300'
            ]"
          >
            <ShoppingBag class="w-3.5 h-3.5" />
            <span>Metode Bayar & Produk Terlaris</span>
          </button>

          <button
            type="button"
            @click="activeTab = 'expenses'"
            class="py-2.5 px-1 text-xs font-bold border-b-2 transition flex items-center gap-1.5 cursor-pointer"
            :class="[
              activeTab === 'expenses'
                ? 'border-emerald-600 text-emerald-600 dark:text-emerald-400 dark:border-emerald-400'
                : 'border-transparent text-slate-500 hover:text-slate-700 dark:hover:text-slate-300'
            ]"
          >
            <PieChart class="w-3.5 h-3.5" />
            <span>Komposisi Beban & Pendapatan Lain</span>
          </button>

          <button
            type="button"
            @click="activeTab = 'daily_table'"
            class="py-2.5 px-1 text-xs font-bold border-b-2 transition flex items-center gap-1.5 cursor-pointer"
            :class="[
              activeTab === 'daily_table'
                ? 'border-emerald-600 text-emerald-600 dark:text-emerald-400 dark:border-emerald-400'
                : 'border-transparent text-slate-500 hover:text-slate-700 dark:hover:text-slate-300'
            ]"
          >
            <Receipt class="w-3.5 h-3.5" />
            <span>Buku Harian Finansial (Tabel Server-side)</span>
          </button>
        </nav>
      </div>

      <!-- TAB 1: OVERVIEW & MULTI-SERIES CHART -->
      <div v-if="isLicenseUnlocked && activeTab === 'overview'" class="space-y-4">
        <!-- Interactive Multi-series SVG Chart Card -->
        <div class="bg-white dark:bg-slate-800 rounded-xl p-4 border border-slate-200 dark:border-slate-700 shadow-xs space-y-4">
          <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
            <div>
              <h3 class="text-sm font-bold text-slate-900 dark:text-white flex items-center gap-1.5">
                <span>Grafik Komparasi Finansial Multi-Deret</span>
              </h3>
              <p class="text-[11px] text-slate-500">
                Arahkan kursor pada grafik untuk melihat rincian per tanggal secara detail.
              </p>
            </div>

            <!-- Series Toggle Pills -->
            <div class="flex flex-wrap items-center gap-1.5 text-xs font-semibold">
              <button
                type="button"
                @click="showSeriesSales = !showSeriesSales"
                class="px-2.5 py-1 rounded-full border transition flex items-center gap-1.5 cursor-pointer"
                :class="showSeriesSales ? 'bg-emerald-50 text-emerald-700 border-emerald-300 dark:bg-emerald-950/60 dark:text-emerald-400' : 'bg-slate-100 text-slate-400 border-transparent dark:bg-slate-700'"
              >
                <span class="w-2 h-2 rounded-full bg-emerald-500"></span>
                <span>Penjualan</span>
              </button>

              <button
                type="button"
                @click="showSeriesNetProfit = !showSeriesNetProfit"
                class="px-2.5 py-1 rounded-full border transition flex items-center gap-1.5 cursor-pointer"
                :class="showSeriesNetProfit ? 'bg-indigo-50 text-indigo-700 border-indigo-300 dark:bg-indigo-950/60 dark:text-indigo-400' : 'bg-slate-100 text-slate-400 border-transparent dark:bg-slate-700'"
              >
                <span class="w-2 h-2 rounded-full bg-indigo-500"></span>
                <span>Laba Bersih</span>
              </button>

              <button
                type="button"
                @click="showSeriesPurchases = !showSeriesPurchases"
                class="px-2.5 py-1 rounded-full border transition flex items-center gap-1.5 cursor-pointer"
                :class="showSeriesPurchases ? 'bg-blue-50 text-blue-700 border-blue-300 dark:bg-blue-950/60 dark:text-blue-400' : 'bg-slate-100 text-slate-400 border-transparent dark:bg-slate-700'"
              >
                <span class="w-2 h-2 rounded-full bg-blue-500"></span>
                <span>Pembelian</span>
              </button>

              <button
                type="button"
                @click="showSeriesExpenses = !showSeriesExpenses"
                class="px-2.5 py-1 rounded-full border transition flex items-center gap-1.5 cursor-pointer"
                :class="showSeriesExpenses ? 'bg-rose-50 text-rose-700 border-rose-300 dark:bg-rose-950/60 dark:text-rose-400' : 'bg-slate-100 text-slate-400 border-transparent dark:bg-slate-700'"
              >
                <span class="w-2 h-2 rounded-full bg-rose-500"></span>
                <span>Beban Kasir</span>
              </button>

              <button
                type="button"
                @click="showSeriesOtherIncome = !showSeriesOtherIncome"
                class="px-2.5 py-1 rounded-full border transition flex items-center gap-1.5 cursor-pointer"
                :class="showSeriesOtherIncome ? 'bg-teal-50 text-teal-700 border-teal-300 dark:bg-teal-950/60 dark:text-teal-400' : 'bg-slate-100 text-slate-400 border-transparent dark:bg-slate-700'"
              >
                <span class="w-2 h-2 rounded-full bg-teal-500"></span>
                <span>Pendapatan Lain</span>
              </button>
            </div>
          </div>

          <!-- SVG Chart Area -->
          <div class="relative w-full overflow-x-auto">
            <svg
              :viewBox="`0 0 ${chartWidth} ${chartHeight}`"
              class="w-full h-auto max-h-72 select-none"
              @mouseleave="hoveredPointIndex = null"
            >
              <defs>
                <linearGradient id="salesGradient" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stop-color="#10b981" stop-opacity="0.25" />
                  <stop offset="100%" stop-color="#10b981" stop-opacity="0.0" />
                </linearGradient>
              </defs>

              <!-- Grid Lines Horisontal -->
              <g class="stroke-slate-100 dark:stroke-slate-700/60 stroke-[1]">
                <line
                  v-for="tick in yAxisTicks"
                  :key="tick.y"
                  :x1="chartPadding.left"
                  :y1="tick.y"
                  :x2="chartWidth - chartPadding.right"
                  :y2="tick.y"
                />
              </g>

              <!-- Y-Axis Labels -->
              <g class="fill-slate-400 text-[10px] font-mono">
                <text
                  v-for="tick in yAxisTicks"
                  :key="tick.val"
                  :x="chartPadding.left - 8"
                  :y="tick.y + 3"
                  text-anchor="end"
                >
                  {{ formatCurrency(tick.val) }}
                </text>
              </g>

              <!-- X-Axis Labels -->
              <g class="fill-slate-400 text-[10px] font-mono">
                <text
                  v-for="tick in xAxisTicks"
                  :key="tick.index"
                  :x="tick.x"
                  :y="chartHeight - chartPadding.bottom + 16"
                  text-anchor="middle"
                >
                  {{ tick.label }}
                </text>
              </g>

              <!-- Area Fill untuk Sales -->
              <path
                v-if="showSeriesSales && salesAreaPath"
                :d="salesAreaPath"
                fill="url(#salesGradient)"
              />

              <!-- Trend Lines -->
              <path
                v-if="showSeriesPurchases && purchasesPath"
                :d="purchasesPath"
                fill="none"
                stroke="#3b82f6"
                stroke-width="1.8"
                stroke-dasharray="4 3"
              />
              <path
                v-if="showSeriesExpenses && expensesPath"
                :d="expensesPath"
                fill="none"
                stroke="#f43f5e"
                stroke-width="1.8"
              />
              <path
                v-if="showSeriesOtherIncome && otherIncomePath"
                :d="otherIncomePath"
                fill="none"
                stroke="#14b8a6"
                stroke-width="1.8"
              />
              <path
                v-if="showSeriesNetProfit && netProfitPath"
                :d="netProfitPath"
                fill="none"
                stroke="#6366f1"
                stroke-width="2.2"
              />
              <path
                v-if="showSeriesSales && salesPath"
                :d="salesPath"
                fill="none"
                stroke="#10b981"
                stroke-width="2.5"
              />

              <!-- Interactive Hover Vertical Guide & Overlay -->
              <g v-if="hoveredPointIndex !== null && trends[hoveredPointIndex]">
                <line
                  :x1="getX(hoveredPointIndex)"
                  :y1="chartPadding.top"
                  :x2="getX(hoveredPointIndex)"
                  :y2="chartHeight - chartPadding.bottom"
                  stroke="#94a3b8"
                  stroke-width="1"
                  stroke-dasharray="3 3"
                />

                <!-- Point Circles -->
                <circle
                  v-if="showSeriesSales"
                  :cx="getX(hoveredPointIndex)"
                  :cy="getY(trends[hoveredPointIndex].net_sales)"
                  r="4"
                  fill="#10b981"
                  stroke="#ffffff"
                  stroke-width="1.5"
                />
                <circle
                  v-if="showSeriesNetProfit"
                  :cx="getX(hoveredPointIndex)"
                  :cy="getY(trends[hoveredPointIndex].net_profit)"
                  r="4"
                  fill="#6366f1"
                  stroke="#ffffff"
                  stroke-width="1.5"
                />
              </g>

              <!-- Transparent Mouse Target Rectangles -->
              <rect
                v-for="(t, i) in trends"
                :key="t.date"
                :x="getX(i) - (chartInnerWidth / Math.max(1, trends.length)) / 2"
                :y="chartPadding.top"
                :width="chartInnerWidth / Math.max(1, trends.length)"
                :height="chartInnerHeight"
                fill="transparent"
                class="cursor-crosshair"
                @mouseenter="hoveredPointIndex = i"
              />
            </svg>

            <!-- Floating Tooltip Box -->
            <div
              v-if="hoveredPointIndex !== null && trends[hoveredPointIndex]"
              class="absolute z-20 top-2 pointer-events-none bg-slate-900/95 text-white p-3 rounded-lg shadow-lg border border-slate-700 text-xs font-mono space-y-1.5 transition-all"
              :style="{
                left: `${Math.min(chartWidth - 200, Math.max(chartPadding.left + 10, getX(hoveredPointIndex) - 90))}px`,
              }"
            >
              <div class="font-bold font-sans text-slate-200 border-b border-slate-700/80 pb-1 flex justify-between gap-4">
                <span>{{ trends[hoveredPointIndex].day_name }}, {{ trends[hoveredPointIndex].date }}</span>
                <span class="text-emerald-400 font-mono">{{ trends[hoveredPointIndex].transaction_count }} Nota</span>
              </div>
              <div v-if="showSeriesSales" class="flex justify-between gap-4 text-emerald-300">
                <span>Penjualan:</span>
                <span class="font-bold">{{ formatCurrency(trends[hoveredPointIndex].net_sales) }}</span>
              </div>
              <div v-if="showSeriesNetProfit" class="flex justify-between gap-4 text-indigo-300">
                <span>Laba Bersih:</span>
                <span class="font-bold">{{ formatCurrency(trends[hoveredPointIndex].net_profit) }}</span>
              </div>
              <div v-if="showSeriesPurchases" class="flex justify-between gap-4 text-blue-300">
                <span>Pembelian:</span>
                <span>{{ formatCurrency(trends[hoveredPointIndex].purchases) }}</span>
              </div>
              <div v-if="showSeriesExpenses" class="flex justify-between gap-4 text-rose-300">
                <span>Beban:</span>
                <span>{{ formatCurrency(trends[hoveredPointIndex].expenses) }}</span>
              </div>
              <div v-if="showSeriesOtherIncome" class="flex justify-between gap-4 text-teal-300">
                <span>Pendapatan Lain:</span>
                <span>{{ formatCurrency(trends[hoveredPointIndex].other_income) }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- TAB 2: SALES PERFORMANCE & TOP PRODUCTS (SERVER-SIDE) -->
      <div v-if="isLicenseUnlocked && activeTab === 'sales'" class="grid grid-cols-1 lg:grid-cols-12 gap-4">
        <!-- Metode Pembayaran Kasir -->
        <div class="lg:col-span-4 bg-white dark:bg-slate-800 rounded-xl p-4 border border-slate-200 dark:border-slate-700 shadow-xs space-y-3">
          <h3 class="text-xs font-bold uppercase tracking-wider text-slate-500 dark:text-slate-400 flex items-center gap-1.5">
            <CreditCard class="w-4 h-4 text-emerald-600" />
            <span>Metode Pembayaran Kasir</span>
          </h3>

          <div v-if="analyticsData?.payment_methods.length === 0" class="py-8 text-center text-xs text-slate-400">
            Belum ada transaksi pada periode ini
          </div>

          <div v-else class="space-y-3 pt-1">
            <div
              v-for="pm in analyticsData?.payment_methods"
              :key="pm.method"
              class="space-y-1"
            >
              <div class="flex items-center justify-between text-xs">
                <span class="font-semibold text-slate-800 dark:text-slate-200">{{ pm.label }}</span>
                <div class="text-right font-mono">
                  <span class="font-bold text-slate-900 dark:text-white">{{ formatCurrency(pm.amount) }}</span>
                  <span class="text-[10px] text-slate-400 ml-1.5">({{ pm.percentage }}%)</span>
                </div>
              </div>
              <!-- Progress Bar -->
              <div class="w-full h-2 rounded-full bg-slate-100 dark:bg-slate-700 overflow-hidden">
                <div
                  class="h-full rounded-full bg-emerald-500 transition-all duration-300"
                  :style="{ width: `${pm.percentage}%` }"
                ></div>
              </div>
              <div class="text-[10px] text-slate-400 font-mono">
                {{ pm.count }} kali transaksi
              </div>
            </div>
          </div>
        </div>

        <!-- Tabel Kinerja Produk Terjual (Server-Side Paginated) -->
        <div class="lg:col-span-8 bg-white dark:bg-slate-800 rounded-xl p-4 border border-slate-200 dark:border-slate-700 shadow-xs space-y-3">
          <!-- Toolbar Search & Sorting -->
          <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-2.5">
            <div class="flex items-center gap-2">
              <ShoppingBag class="w-4 h-4 text-emerald-600" />
              <h3 class="text-xs font-bold uppercase tracking-wider text-slate-700 dark:text-slate-300">
                Kinerja Penjualan Produk
              </h3>
              <span class="text-[10px] font-mono text-slate-400">
                ({{ productsData?.total || 0 }} SKU Ditemukan)
              </span>
            </div>

            <div class="flex items-center gap-2">
              <!-- Search Bar -->
              <div class="relative w-56">
                <Search class="w-3.5 h-3.5 text-slate-400 absolute left-2.5 top-2.5 pointer-events-none" />
                <input
                  type="text"
                  v-model="productsSearch"
                  placeholder="Cari nama / barcode / SKU..."
                  class="w-full pl-8 pr-2.5 py-1 text-xs border border-slate-200 dark:border-slate-700 rounded-lg bg-slate-50 dark:bg-slate-900 text-slate-800 dark:text-slate-200 focus:ring-1 focus:ring-emerald-500"
                />
              </div>

              <!-- Page Size Selector -->
              <select
                v-model="productsPerPage"
                @change="productsPage = 1; fetchProducts()"
                class="px-2 py-1 text-xs border border-slate-200 dark:border-slate-700 rounded-lg bg-white dark:bg-slate-800 font-mono"
              >
                <option :value="10">10 / hal</option>
                <option :value="25">25 / hal</option>
                <option :value="50">50 / hal</option>
              </select>
            </div>
          </div>

          <!-- Summary Badges -->
          <div class="grid grid-cols-3 gap-2 bg-slate-50 dark:bg-slate-900/50 p-2.5 rounded-lg text-xs font-mono">
            <div>
              <span class="text-slate-400 block text-[10px] uppercase font-sans">Total Qty Terjual</span>
              <span class="font-bold text-slate-800 dark:text-slate-200">{{ productsData?.summary_qty || 0 }} unit</span>
            </div>
            <div>
              <span class="text-slate-400 block text-[10px] uppercase font-sans">Total Omzet Produk</span>
              <span class="font-bold text-emerald-700 dark:text-emerald-400">{{ formatCurrency(productsData?.summary_sales || 0) }}</span>
            </div>
            <div>
              <span class="text-slate-400 block text-[10px] uppercase font-sans">Kontribusi Laba Kotor</span>
              <span class="font-bold text-teal-600">{{ formatCurrency(productsData?.summary_profit || 0) }}</span>
            </div>
          </div>

          <!-- Table Container -->
          <div class="overflow-x-auto border border-slate-200 dark:border-slate-700 rounded-lg">
            <table class="w-full text-left text-xs">
              <thead class="bg-slate-50 dark:bg-slate-900/70 text-slate-500 dark:text-slate-400 font-bold uppercase text-[10px] border-b border-slate-200 dark:border-slate-700 select-none">
                <tr>
                  <th class="py-2 px-3 w-10 text-center">#</th>
                  <th
                    @click="toggleProductsSort('name')"
                    class="py-2 px-3 cursor-pointer hover:text-slate-900 dark:hover:text-white"
                  >
                    <div class="flex items-center gap-1">
                      <span>Nama Produk / SKU</span>
                      <ArrowUpDown class="w-3 h-3 text-slate-400" />
                    </div>
                  </th>
                  <th
                    @click="toggleProductsSort('qty')"
                    class="py-2 px-3 text-right cursor-pointer hover:text-slate-900 dark:hover:text-white"
                  >
                    <div class="flex items-center justify-end gap-1">
                      <span>Qty Terjual</span>
                      <ArrowUpDown class="w-3 h-3 text-slate-400" />
                    </div>
                  </th>
                  <th
                    @click="toggleProductsSort('sales')"
                    class="py-2 px-3 text-right cursor-pointer hover:text-slate-900 dark:hover:text-white"
                  >
                    <div class="flex items-center justify-end gap-1">
                      <span>Total Omzet</span>
                      <ArrowUpDown class="w-3 h-3 text-slate-400" />
                    </div>
                  </th>
                  <th
                    @click="toggleProductsSort('profit')"
                    class="py-2 px-3 text-right cursor-pointer hover:text-slate-900 dark:hover:text-white"
                  >
                    <div class="flex items-center justify-end gap-1">
                      <span>Laba Kotor</span>
                      <ArrowUpDown class="w-3 h-3 text-slate-400" />
                    </div>
                  </th>
                  <th
                    @click="toggleProductsSort('margin')"
                    class="py-2 px-3 text-right cursor-pointer hover:text-slate-900 dark:hover:text-white"
                  >
                    <div class="flex items-center justify-end gap-1">
                      <span>Margin %</span>
                      <ArrowUpDown class="w-3 h-3 text-slate-400" />
                    </div>
                  </th>
                </tr>
              </thead>
              <tbody class="divide-y divide-slate-100 dark:divide-slate-700/60 font-mono">
                <tr v-if="isProductsLoading">
                  <td colspan="6" class="py-10 text-center text-slate-400 font-sans">
                    <RefreshCw class="w-4 h-4 animate-spin mx-auto mb-1 text-emerald-600" />
                    Memuat data penjualan produk server-side...
                  </td>
                </tr>
                <tr v-else-if="productsData?.items.length === 0">
                  <td colspan="6" class="py-8 text-center text-slate-400 font-sans">
                    Tidak ditemukan data produk sesuai kriteria filter atau pencarian.
                  </td>
                </tr>
                <tr
                  v-for="(prod, idx) in productsData?.items"
                  :key="prod.sku || idx"
                  class="hover:bg-slate-50 dark:hover:bg-slate-750 transition"
                >
                  <td class="py-2 px-3 text-center text-slate-400 font-bold">
                    {{ (productsPage - 1) * productsPerPage + idx + 1 }}
                  </td>
                  <td class="py-2 px-3 font-sans">
                    <div class="font-bold text-slate-800 dark:text-slate-200">{{ prod.product_name }}</div>
                    <div class="text-[10px] text-slate-400 font-mono">{{ prod.sku }}</div>
                  </td>
                  <td class="py-2 px-3 text-right font-bold text-slate-700 dark:text-slate-300">
                    {{ prod.total_qty }}
                  </td>
                  <td class="py-2 px-3 text-right font-bold text-emerald-700 dark:text-emerald-400">
                    {{ formatCurrency(prod.total_sales) }}
                  </td>
                  <td class="py-2 px-3 text-right text-slate-600 dark:text-slate-300">
                    {{ formatCurrency(prod.total_profit) }}
                  </td>
                  <td class="py-2 px-3 text-right font-bold" :class="prod.margin_pct >= 20 ? 'text-emerald-600' : 'text-amber-600'">
                    {{ prod.margin_pct }}%
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <!-- Pagination Footer -->
          <div class="flex items-center justify-between text-xs text-slate-500 pt-1">
            <span class="font-mono text-[11px]">
              Halaman {{ productsPage }} dari {{ productsData?.total_pages || 1 }}
            </span>
            <div class="flex items-center gap-1.5">
              <button
                type="button"
                :disabled="productsPage <= 1 || isProductsLoading"
                @click="productsPage--; fetchProducts()"
                class="px-2.5 py-1 rounded border border-slate-200 dark:border-slate-700 disabled:opacity-40 hover:bg-slate-50 dark:hover:bg-slate-700 flex items-center gap-1 cursor-pointer"
              >
                <ChevronLeft class="w-3.5 h-3.5" />
                <span>Sebelumnya</span>
              </button>
              <button
                type="button"
                :disabled="productsPage >= (productsData?.total_pages || 1) || isProductsLoading"
                @click="productsPage++; fetchProducts()"
                class="px-2.5 py-1 rounded border border-slate-200 dark:border-slate-700 disabled:opacity-40 hover:bg-slate-50 dark:hover:bg-slate-700 flex items-center gap-1 cursor-pointer"
              >
                <span>Selanjutnya</span>
                <ChevronRight class="w-3.5 h-3.5" />
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- TAB 3: EXPENSES & OTHER INCOME COMPOSITION (SERVER-SIDE) -->
      <div v-if="isLicenseUnlocked && activeTab === 'expenses'" class="grid grid-cols-1 lg:grid-cols-2 gap-4">
        <!-- 1. Komposisi Beban Operasional -->
        <div class="bg-white dark:bg-slate-800 rounded-xl p-4 border border-slate-200 dark:border-slate-700 shadow-xs space-y-3">
          <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-2">
            <div>
              <h3 class="text-xs font-bold uppercase tracking-wider text-slate-700 dark:text-slate-300 flex items-center gap-1.5">
                <TrendingDown class="w-4 h-4 text-rose-600" />
                <span>Komposisi Pengeluaran Beban Toko</span>
              </h3>
              <p class="text-[10px] text-slate-400">
                Server-side breakdown kategori beban operasional
              </p>
            </div>
            <span class="text-xs font-mono font-bold text-rose-700 dark:text-rose-400">
              Total: {{ formatCurrency(expensesData?.total_amount || 0) }}
            </span>
          </div>

          <!-- Search & Controls -->
          <div class="flex items-center gap-2">
            <div class="relative flex-1">
              <Search class="w-3.5 h-3.5 text-slate-400 absolute left-2.5 top-2.5 pointer-events-none" />
              <input
                type="text"
                v-model="expensesSearch"
                placeholder="Cari kategori pengeluaran..."
                class="w-full pl-8 pr-2.5 py-1 text-xs border border-slate-200 dark:border-slate-700 rounded-lg bg-slate-50 dark:bg-slate-900 text-slate-800 dark:text-slate-200 focus:ring-1 focus:ring-rose-500"
              />
            </div>
            <select
              v-model="expensesSortOrder"
              @change="fetchExpenses"
              class="px-2 py-1 text-xs border border-slate-200 dark:border-slate-700 rounded-lg bg-white dark:bg-slate-800 font-mono"
            >
              <option value="desc">Nominal Tertinggi</option>
              <option value="asc">Nominal Terendah</option>
            </select>
          </div>

          <!-- List Items -->
          <div v-if="isExpensesLoading" class="py-8 text-center text-xs text-slate-400">
            <RefreshCw class="w-4 h-4 animate-spin mx-auto mb-1 text-rose-600" />
            Memuat kategori pengeluaran...
          </div>
          <div v-else-if="expensesData?.items.length === 0" class="py-8 text-center text-xs text-slate-400">
            Tidak ada pengeluaran beban pada filter ini
          </div>
          <div v-else class="space-y-3 pt-1">
            <div
              v-for="cat in expensesData?.items"
              :key="cat.name"
              class="space-y-1"
            >
              <div class="flex items-center justify-between text-xs">
                <span class="font-semibold text-slate-800 dark:text-slate-200">{{ cat.name }}</span>
                <div class="text-right font-mono">
                  <span class="font-bold text-slate-900 dark:text-white">{{ formatCurrency(cat.amount) }}</span>
                  <span class="text-[10px] text-slate-400 ml-1.5">({{ cat.percentage }}%)</span>
                </div>
              </div>
              <div class="w-full h-2 rounded-full bg-slate-100 dark:bg-slate-700 overflow-hidden">
                <div
                  class="h-full rounded-full bg-rose-500 transition-all duration-300"
                  :style="{ width: `${cat.percentage}%` }"
                ></div>
              </div>
              <div class="text-[10px] text-slate-400 font-mono">
                {{ cat.count }} transaksi bukti pengeluaran
              </div>
            </div>
          </div>

          <!-- Pagination -->
          <div class="flex items-center justify-between text-xs text-slate-500 pt-2 border-t border-slate-100 dark:border-slate-700/60">
            <span class="font-mono text-[11px]">Hal {{ expensesPage }} / {{ expensesData?.total_pages || 1 }}</span>
            <div class="flex items-center gap-1">
              <button
                type="button"
                :disabled="expensesPage <= 1"
                @click="expensesPage--; fetchExpenses()"
                class="px-2 py-0.5 rounded border border-slate-200 dark:border-slate-700 disabled:opacity-40"
              >
                &larr;
              </button>
              <button
                type="button"
                :disabled="expensesPage >= (expensesData?.total_pages || 1)"
                @click="expensesPage++; fetchExpenses()"
                class="px-2 py-0.5 rounded border border-slate-200 dark:border-slate-700 disabled:opacity-40"
              >
                &rarr;
              </button>
            </div>
          </div>
        </div>

        <!-- 2. Komposisi Pendapatan Lain-lain -->
        <div class="bg-white dark:bg-slate-800 rounded-xl p-4 border border-slate-200 dark:border-slate-700 shadow-xs space-y-3">
          <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-2">
            <div>
              <h3 class="text-xs font-bold uppercase tracking-wider text-slate-700 dark:text-slate-300 flex items-center gap-1.5">
                <DollarSign class="w-4 h-4 text-emerald-600" />
                <span>Komposisi Pendapatan Lain-lain</span>
              </h3>
              <p class="text-[10px] text-slate-400">
                Server-side breakdown pendapatan kas non-penjualan
              </p>
            </div>
            <span class="text-xs font-mono font-bold text-emerald-700 dark:text-emerald-400">
              Total: {{ formatCurrency(otherIncomeData?.total_amount || 0) }}
            </span>
          </div>

          <!-- Search & Controls -->
          <div class="flex items-center gap-2">
            <div class="relative flex-1">
              <Search class="w-3.5 h-3.5 text-slate-400 absolute left-2.5 top-2.5 pointer-events-none" />
              <input
                type="text"
                v-model="otherIncomeSearch"
                placeholder="Cari kategori pendapatan..."
                class="w-full pl-8 pr-2.5 py-1 text-xs border border-slate-200 dark:border-slate-700 rounded-lg bg-slate-50 dark:bg-slate-900 text-slate-800 dark:text-slate-200 focus:ring-1 focus:ring-emerald-500"
              />
            </div>
            <select
              v-model="otherIncomeSortOrder"
              @change="fetchOtherIncome"
              class="px-2 py-1 text-xs border border-slate-200 dark:border-slate-700 rounded-lg bg-white dark:bg-slate-800 font-mono"
            >
              <option value="desc">Nominal Tertinggi</option>
              <option value="asc">Nominal Terendah</option>
            </select>
          </div>

          <!-- List Items -->
          <div v-if="isOtherIncomeLoading" class="py-8 text-center text-xs text-slate-400">
            <RefreshCw class="w-4 h-4 animate-spin mx-auto mb-1 text-emerald-600" />
            Memuat kategori pendapatan lain...
          </div>
          <div v-else-if="otherIncomeData?.items.length === 0" class="py-8 text-center text-xs text-slate-400">
            Tidak ada pendapatan lain pada filter ini
          </div>
          <div v-else class="space-y-3 pt-1">
            <div
              v-for="cat in otherIncomeData?.items"
              :key="cat.name"
              class="space-y-1"
            >
              <div class="flex items-center justify-between text-xs">
                <span class="font-semibold text-slate-800 dark:text-slate-200">{{ cat.name }}</span>
                <div class="text-right font-mono">
                  <span class="font-bold text-slate-900 dark:text-white">{{ formatCurrency(cat.amount) }}</span>
                  <span class="text-[10px] text-slate-400 ml-1.5">({{ cat.percentage }}%)</span>
                </div>
              </div>
              <div class="w-full h-2 rounded-full bg-slate-100 dark:bg-slate-700 overflow-hidden">
                <div
                  class="h-full rounded-full bg-emerald-500 transition-all duration-300"
                  :style="{ width: `${cat.percentage}%` }"
                ></div>
              </div>
              <div class="text-[10px] text-slate-400 font-mono">
                {{ cat.count }} transaksi penerimaan kas
              </div>
            </div>
          </div>

          <!-- Pagination -->
          <div class="flex items-center justify-between text-xs text-slate-500 pt-2 border-t border-slate-100 dark:border-slate-700/60">
            <span class="font-mono text-[11px]">Hal {{ otherIncomePage }} / {{ otherIncomeData?.total_pages || 1 }}</span>
            <div class="flex items-center gap-1">
              <button
                type="button"
                :disabled="otherIncomePage <= 1"
                @click="otherIncomePage--; fetchOtherIncome()"
                class="px-2 py-0.5 rounded border border-slate-200 dark:border-slate-700 disabled:opacity-40"
              >
                &larr;
              </button>
              <button
                type="button"
                :disabled="otherIncomePage >= (otherIncomeData?.total_pages || 1)"
                @click="otherIncomePage++; fetchOtherIncome()"
                class="px-2 py-0.5 rounded border border-slate-200 dark:border-slate-700 disabled:opacity-40"
              >
                &rarr;
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- TAB 4: DAILY TABULAR BREAKDOWN (SERVER-SIDE PAGINATED) -->
      <div v-if="isLicenseUnlocked && activeTab === 'daily_table'" class="bg-white dark:bg-slate-800 rounded-xl border border-slate-200 dark:border-slate-700 shadow-xs overflow-hidden space-y-3 p-4">
        <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-2.5 pb-2 border-b border-slate-200 dark:border-slate-700">
          <div>
            <h3 class="text-xs font-bold uppercase tracking-wider text-slate-700 dark:text-slate-300 flex items-center gap-1.5">
              <Receipt class="w-4 h-4 text-emerald-600" />
              <span>Tabel Rincian Harian Finansial (Server-side)</span>
            </h3>
            <p class="text-[11px] text-slate-400">
              Arsip data harian untuk audit dan pencocokan buku besar kasir.
            </p>
          </div>

          <!-- Search & Page Size Toolbar -->
          <div class="flex items-center gap-2">
            <div class="relative w-48">
              <Search class="w-3.5 h-3.5 text-slate-400 absolute left-2.5 top-2.5 pointer-events-none" />
              <input
                type="text"
                v-model="dailySearch"
                placeholder="Cari tanggal / hari..."
                class="w-full pl-8 pr-2.5 py-1 text-xs border border-slate-200 dark:border-slate-700 rounded-lg bg-slate-50 dark:bg-slate-900 text-slate-800 dark:text-slate-200 focus:ring-1 focus:ring-emerald-500"
              />
            </div>

            <select
              v-model="dailyPerPage"
              @change="dailyPage = 1; fetchDailyLedger()"
              class="px-2 py-1 text-xs border border-slate-200 dark:border-slate-700 rounded-lg bg-white dark:bg-slate-800 font-mono"
            >
              <option :value="10">10 baris</option>
              <option :value="15">15 baris</option>
              <option :value="31">31 baris</option>
            </select>

            <button
              type="button"
              @click="exportToExcel"
              class="text-xs font-bold text-emerald-600 hover:text-emerald-700 flex items-center gap-1 px-2.5 py-1 rounded-lg border border-emerald-300 dark:border-emerald-700 bg-emerald-50 dark:bg-emerald-950/40"
            >
              <FileSpreadsheet class="w-3.5 h-3.5" />
              <span>Unduh Excel</span>
            </button>
          </div>
        </div>

        <div class="overflow-x-auto border border-slate-200 dark:border-slate-700 rounded-lg">
          <table class="min-w-full divide-y divide-slate-200 dark:divide-slate-700 text-xs">
            <thead class="bg-slate-50 dark:bg-slate-900/60 text-slate-500 font-bold uppercase text-[10px] select-none">
              <tr>
                <th
                  @click="toggleDailySort('date')"
                  class="py-2.5 px-3 text-left cursor-pointer hover:text-slate-900 dark:hover:text-white"
                >
                  <div class="flex items-center gap-1">
                    <span>Tanggal</span>
                    <ArrowUpDown class="w-3 h-3 text-slate-400" />
                  </div>
                </th>
                <th class="py-2.5 px-3 text-center">Hari</th>
                <th
                  @click="toggleDailySort('net_sales')"
                  class="py-2.5 px-3 text-right cursor-pointer hover:text-slate-900 dark:hover:text-white"
                >
                  <div class="flex items-center justify-end gap-1">
                    <span>Penjualan Bersih</span>
                    <ArrowUpDown class="w-3 h-3 text-slate-400" />
                  </div>
                </th>
                <th
                  @click="toggleDailySort('cogs')"
                  class="py-2.5 px-3 text-right cursor-pointer hover:text-slate-900 dark:hover:text-white"
                >
                  <div class="flex items-center justify-end gap-1">
                    <span>HPP Modal</span>
                    <ArrowUpDown class="w-3 h-3 text-slate-400" />
                  </div>
                </th>
                <th
                  @click="toggleDailySort('gross_profit')"
                  class="py-2.5 px-3 text-right cursor-pointer hover:text-slate-900 dark:hover:text-white"
                >
                  <div class="flex items-center justify-end gap-1">
                    <span>Laba Kotor</span>
                    <ArrowUpDown class="w-3 h-3 text-slate-400" />
                  </div>
                </th>
                <th
                  @click="toggleDailySort('purchases')"
                  class="py-2.5 px-3 text-right cursor-pointer hover:text-slate-900 dark:hover:text-white"
                >
                  <div class="flex items-center justify-end gap-1">
                    <span>Pembelian</span>
                    <ArrowUpDown class="w-3 h-3 text-slate-400" />
                  </div>
                </th>
                <th
                  @click="toggleDailySort('expenses')"
                  class="py-2.5 px-3 text-right cursor-pointer hover:text-slate-900 dark:hover:text-white"
                >
                  <div class="flex items-center justify-end gap-1">
                    <span>Beban Kasir</span>
                    <ArrowUpDown class="w-3 h-3 text-slate-400" />
                  </div>
                </th>
                <th
                  @click="toggleDailySort('other_income')"
                  class="py-2.5 px-3 text-right cursor-pointer hover:text-slate-900 dark:hover:text-white"
                >
                  <div class="flex items-center justify-end gap-1">
                    <span>Pendapatan Lain</span>
                    <ArrowUpDown class="w-3 h-3 text-slate-400" />
                  </div>
                </th>
                <th
                  @click="toggleDailySort('net_profit')"
                  class="py-2.5 px-3 text-right cursor-pointer hover:text-slate-900 dark:hover:text-white"
                >
                  <div class="flex items-center justify-end gap-1">
                    <span>Laba Bersih</span>
                    <ArrowUpDown class="w-3 h-3 text-slate-400" />
                  </div>
                </th>
                <th
                  @click="toggleDailySort('transaction_count')"
                  class="py-2.5 px-3 text-center cursor-pointer hover:text-slate-900 dark:hover:text-white"
                >
                  <div class="flex items-center justify-center gap-1">
                    <span>Nota</span>
                    <ArrowUpDown class="w-3 h-3 text-slate-400" />
                  </div>
                </th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-100 dark:divide-slate-700/60 font-mono">
              <tr v-if="isDailyLoading">
                <td colspan="10" class="py-10 text-center text-slate-400 font-sans">
                  <RefreshCw class="w-4 h-4 animate-spin mx-auto mb-1 text-emerald-600" />
                  Memuat data buku harian finansial...
                </td>
              </tr>
              <tr v-else-if="dailyData?.items.length === 0">
                <td colspan="10" class="py-8 text-center text-slate-400 font-sans">
                  Tidak ada catatan finansial pada tanggal yang dipilih.
                </td>
              </tr>
              <tr
                v-for="t in dailyData?.items"
                :key="t.date"
                class="hover:bg-slate-50 dark:hover:bg-slate-750 transition"
              >
                <td class="py-2 px-3 text-slate-800 dark:text-slate-200 font-bold">{{ t.date }}</td>
                <td class="py-2 px-3 text-center text-slate-400 font-sans">{{ t.day_name }}</td>
                <td class="py-2 px-3 text-right font-bold text-emerald-700 dark:text-emerald-400">
                  {{ formatCurrency(t.net_sales) }}
                </td>
                <td class="py-2 px-3 text-right text-slate-500">
                  {{ formatCurrency(t.cogs) }}
                </td>
                <td class="py-2 px-3 text-right font-bold text-teal-600">
                  {{ formatCurrency(t.gross_profit) }}
                </td>
                <td class="py-2 px-3 text-right text-blue-600">
                  {{ formatCurrency(t.purchases) }}
                </td>
                <td class="py-2 px-3 text-right text-rose-600">
                  {{ formatCurrency(t.expenses) }}
                </td>
                <td class="py-2 px-3 text-right text-teal-600">
                  {{ formatCurrency(t.other_income) }}
                </td>
                <td
                  class="py-2 px-3 text-right font-extrabold"
                  :class="t.net_profit >= 0 ? 'text-indigo-600 dark:text-indigo-400' : 'text-rose-600'"
                >
                  {{ formatCurrency(t.net_profit) }}
                </td>
                <td class="py-2 px-3 text-center font-bold text-slate-700 dark:text-slate-300">
                  {{ t.transaction_count }}
                </td>
              </tr>
            </tbody>

            <!-- Total Agregat Summary Row -->
            <tfoot class="bg-slate-100 dark:bg-slate-900 text-slate-800 dark:text-slate-200 font-mono font-bold text-xs border-t-2 border-slate-300 dark:border-slate-600">
              <tr>
                <td colspan="2" class="py-2.5 px-3 text-left font-sans uppercase text-[10px] tracking-wider text-slate-500">
                  Total Seluruh Periode
                </td>
                <td class="py-2.5 px-3 text-right text-emerald-700 dark:text-emerald-400">
                  {{ formatCurrency(dailyData?.summary_sales || 0) }}
                </td>
                <td class="py-2.5 px-3 text-right text-slate-600 dark:text-slate-300">
                  {{ formatCurrency(dailyData?.summary_cogs || 0) }}
                </td>
                <td class="py-2.5 px-3 text-right text-teal-700 dark:text-teal-400">
                  {{ formatCurrency(dailyData?.summary_gross_profit || 0) }}
                </td>
                <td class="py-2.5 px-3 text-right text-blue-700 dark:text-blue-400">
                  {{ formatCurrency(dailyData?.summary_purchases || 0) }}
                </td>
                <td class="py-2.5 px-3 text-right text-rose-700 dark:text-rose-400">
                  {{ formatCurrency(dailyData?.summary_expenses || 0) }}
                </td>
                <td class="py-2.5 px-3 text-right text-teal-700 dark:text-teal-400">
                  {{ formatCurrency(dailyData?.summary_other_income || 0) }}
                </td>
                <td class="py-2.5 px-3 text-right text-indigo-700 dark:text-indigo-400">
                  {{ formatCurrency(dailyData?.summary_net_profit || 0) }}
                </td>
                <td class="py-2.5 px-3 text-center">
                  {{ dailyData?.summary_transactions || 0 }}
                </td>
              </tr>
            </tfoot>
          </table>
        </div>

        <!-- Pagination Controls -->
        <div class="flex items-center justify-between text-xs text-slate-500 pt-1">
          <span class="font-mono text-[11px]">
            Halaman {{ dailyPage }} dari {{ dailyData?.total_pages || 1 }} (Total {{ dailyData?.total || 0 }} Hari)
          </span>
          <div class="flex items-center gap-1.5">
            <button
              type="button"
              :disabled="dailyPage <= 1 || isDailyLoading"
              @click="dailyPage--; fetchDailyLedger()"
              class="px-2.5 py-1 rounded border border-slate-200 dark:border-slate-700 disabled:opacity-40 hover:bg-slate-50 dark:hover:bg-slate-700 flex items-center gap-1 cursor-pointer"
            >
              <ChevronLeft class="w-3.5 h-3.5" />
              <span>Sebelumnya</span>
            </button>
            <button
              type="button"
              :disabled="dailyPage >= (dailyData?.total_pages || 1) || isDailyLoading"
              @click="dailyPage++; fetchDailyLedger()"
              class="px-2.5 py-1 rounded border border-slate-200 dark:border-slate-700 disabled:opacity-40 hover:bg-slate-50 dark:hover:bg-slate-700 flex items-center gap-1 cursor-pointer"
            >
              <span>Selanjutnya</span>
              <ChevronRight class="w-3.5 h-3.5" />
            </button>
          </div>
        </div>
      </div>
    </div>
  </AdminLayout>
</template>
