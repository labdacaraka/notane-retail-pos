<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import AdminLayout from '@/layouts/AdminLayout.vue'
import PaginationBar from '@/components/common/PaginationBar.vue'
import SearchableSelect from '@/components/common/SearchableSelect.vue'
import {
  Search,
  RefreshCw,
  Plus,
  Eye,
  X,
  FileText,
  RotateCcw,
  CheckCircle2,
  AlertCircle,
  Calendar,
  Tag,
  ChevronDown,
  Sliders,
  Wallet,
  TrendingUp,
  Receipt,
  ArrowUpDown,
} from 'lucide-vue-next'
import { api } from '@/services/api'
import { useDialog } from '@/composables/useDialog'
import { useLocationStore } from '@/stores/locationStore'
import { useAuthStore } from '@/stores/authStore'
import { formatCurrency, formatDate } from '@/utils/formatters'
import CopyableText from '@/components/common/CopyableText.vue'
import type {
  OtherIncomeTransaction,
  OtherIncomeCategory,
  OtherIncomeSummaryResponse,
  CreateOtherIncomeRequest,
} from '@/types'

const locationStore = useLocationStore()
const authStore = useAuthStore()
const dialog = useDialog()

// Tab state: 'transactions' | 'categories'
const activeTab = ref<'transactions' | 'categories'>('transactions')

// Filters & Pagination
const selectedOutletId = ref<string>(locationStore.currentOutletId || '')
const searchQuery = ref<string>('')
const selectedCategoryId = ref<string>('')
const selectedStatus = ref<string>('')
const datePreset = ref<'all' | 'today' | '7days' | 'month'>('month')
const startDate = ref<string>('')
const endDate = ref<string>('')
const currentPage = ref<number>(1)
const pageSize = ref<number>(20)
const totalItems = ref<number>(0)
const totalPages = ref<number>(1)

// Data states
const otherIncomes = ref<OtherIncomeTransaction[]>([])
const categories = ref<OtherIncomeCategory[]>([])
const paymentAccounts = ref<any[]>([])
const incomeAccounts = ref<any[]>([])
const summary = ref<OtherIncomeSummaryResponse | null>(null)
const isLoading = ref<boolean>(false)
const isSubmitting = ref<boolean>(false)

// Modals
const showCreateModal = ref<boolean>(false)
const showDetailModal = ref<boolean>(false)
const showCancelModal = ref<boolean>(false)
const showCategoryModal = ref<boolean>(false)

// Selected Transaction for Detail / Cancel
const selectedIncome = ref<OtherIncomeTransaction | null>(null)
const cancelReason = ref<string>('')

// Form Create Other Income
const newIncome = ref<CreateOtherIncomeRequest>({
  outlet_id: locationStore.currentOutletId || '',
  category_id: '',
  income_account_id: '',
  payment_account_id: '',
  amount: 0,
  transaction_date: new Date().toISOString().split('T')[0],
  payer_name: '',
  notes: '',
  reference_no: '',
  attachment_url: '',
  shift_id: null,
})
const linkActiveShift = ref<boolean>(false)
const activeShiftInfo = ref<any>(null)
const showAdvancedAccountingOption = ref<boolean>(false)
const quickAmounts = [10000, 20000, 50000, 100000, 200000, 500000]

function addQuickAmount(val: number) {
  newIncome.value.amount = (Number(newIncome.value.amount) || 0) + val
}

function resetAmount() {
  newIncome.value.amount = 0
}

const categoryMetaMap: Record<string, { icon: string; badge: string; color: string; desc: string }> = {
  SEWA_PROMOSI: { icon: '📢', badge: 'Sewa & Promo', color: 'bg-emerald-100 text-emerald-900 border-emerald-300', desc: 'Sewa space booth, banner promosi brand toko' },
  KOMISI: { icon: '🤝', badge: 'Komisi & Jastip', color: 'bg-blue-100 text-blue-900 border-blue-300', desc: 'Bagi hasil konsinyasi UMKM, komisi jasa titip' },
  LIMBAH: { icon: '📦', badge: 'Limbah & Kardus', color: 'bg-amber-100 text-amber-900 border-amber-300', desc: 'Penjualan kardus bekas, karung packing, limbah toko' },
  BUNGA_BANK: { icon: '🏦', badge: 'Bunga Bank', color: 'bg-cyan-100 text-cyan-900 border-cyan-300', desc: 'Bunga tabungan giro operasional bank toko' },
  LAIN_LAIN: { icon: '✨', badge: 'Lain-lain', color: 'bg-teal-100 text-teal-900 border-teal-300', desc: 'Pendapatan non-operasional toko lainnya' },
}

function getCategoryMeta(code: string) {
  return categoryMetaMap[code] || { icon: '💰', badge: 'Pendapatan', color: 'bg-emerald-100 text-emerald-800 border-emerald-300', desc: 'Pendapatan lain-lain toko' }
}

function handleFilterChange() {
  currentPage.value = 1
  fetchOtherIncomes()
  fetchSummary()
}

const statusOptions = [
  { id: 'COMPLETED', name: 'Selesai (Aktif)' },
  { id: 'CANCELLED', name: 'Dibatalkan (Void)' },
]

function onCategorySelected(_catId: any, item: any) {
  if (item && item.account_id) {
    newIncome.value.income_account_id = item.account_id
  }
}

// Form Category (Create / Edit)
const isEditingCategory = ref<boolean>(false)
const categoryForm = ref({
  id: '',
  code: '',
  name: '',
  account_id: '',
  description: '',
  is_active: true,
})

// Categories Filter, Search, Sorting, & Pagination
const categorySearch = ref<string>('')
const categorySortBy = ref<string>('name')
const categorySortOrder = ref<'asc' | 'desc'>('asc')
const categoryActiveOnly = ref<boolean | ''>('')
const categoryCurrentPage = ref<number>(1)
const categoryPageSize = ref<number>(12)
const categoryTotalItems = ref<number>(0)
const isCategoriesLoading = ref<boolean>(false)
let categorySearchDebounce: ReturnType<typeof setTimeout> | null = null

function onCategorySearchInput() {
  if (categorySearchDebounce) clearTimeout(categorySearchDebounce)
  categorySearchDebounce = setTimeout(() => {
    categoryCurrentPage.value = 1
    fetchCategories()
  }, 350)
}

function handleCategoryFilterChange() {
  categoryCurrentPage.value = 1
  fetchCategories()
}

function handleCategoryPageSizeChange(newSize: number) {
  categoryPageSize.value = newSize
  categoryCurrentPage.value = 1
  fetchCategories()
}

function toggleCategorySort(column: string) {
  if (categorySortBy.value === column) {
    categorySortOrder.value = categorySortOrder.value === 'asc' ? 'desc' : 'asc'
  } else {
    categorySortBy.value = column
    categorySortOrder.value = 'asc'
  }
  categoryCurrentPage.value = 1
  fetchCategories()
}

function applyDatePreset(preset: 'all' | 'today' | '7days' | 'month') {
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
  } else if (preset === 'month') {
    const firstDay = new Date(today.getFullYear(), today.getMonth(), 1)
    startDate.value = firstDay.toISOString().split('T')[0]
    endDate.value = todayStr
  } else {
    startDate.value = ''
    endDate.value = ''
  }
  currentPage.value = 1
  fetchOtherIncomes()
  fetchSummary()
}

watch(
  () => locationStore.currentOutletId,
  (newId) => {
    if (newId) {
      selectedOutletId.value = newId
      newIncome.value.outlet_id = newId
      fetchOtherIncomes()
      fetchSummary()
      fetchAccounts()
    }
  }
)

// Sinkronkan akun pendapatan otomatis saat kategori dipilih
watch(
  () => newIncome.value.category_id,
  (catId) => {
    if (!catId) return
    const cat = categories.value.find((c) => c.id === catId)
    if (cat && cat.account_id) {
      newIncome.value.income_account_id = cat.account_id
    }
  }
)

// Fetch Data
async function fetchOtherIncomes() {
  isLoading.value = true
  try {
    const params: Record<string, any> = {
      page: currentPage.value,
      limit: pageSize.value,
    }
    if (selectedOutletId.value) params.outlet_id = selectedOutletId.value
    if (selectedCategoryId.value) params.category_id = selectedCategoryId.value
    if (selectedStatus.value) params.status = selectedStatus.value
    if (startDate.value) params.start_date = startDate.value
    if (endDate.value) params.end_date = endDate.value
    if (searchQuery.value.trim()) params.search = searchQuery.value.trim()

    const res = await api.get('/other-income', { params })
    if (res.data?.success) {
      otherIncomes.value = res.data.data || []
      totalItems.value = res.data.total || 0
      totalPages.value = res.data.total_pages || 1
    }
  } catch (err: any) {
    console.error('Gagal memuat pendapatan lain-lain:', err)
  } finally {
    isLoading.value = false
  }
}

async function fetchSummary() {
  try {
    const params: Record<string, any> = {}
    if (selectedOutletId.value) params.outlet_id = selectedOutletId.value
    if (startDate.value) params.start_date = startDate.value
    if (endDate.value) params.end_date = endDate.value

    const res = await api.get('/other-income/summary', { params })
    if (res.data?.success) {
      summary.value = res.data.data
    }
  } catch (err) {
    console.error('Gagal memuat ringkasan pendapatan:', err)
  }
}

async function fetchCategories() {
  isCategoriesLoading.value = true
  try {
    const params: Record<string, any> = {
      page: categoryCurrentPage.value,
      limit: categoryPageSize.value,
      sort_by: categorySortBy.value,
      sort_order: categorySortOrder.value,
    }
    if (categorySearch.value.trim()) {
      params.search = categorySearch.value.trim()
    }
    if (categoryActiveOnly.value !== '') {
      params.active_only = categoryActiveOnly.value
    }

    const res = await api.get('/other-income/categories', { params })
    if (res.data?.success) {
      categories.value = res.data.data || []
      categoryTotalItems.value = res.data.total_items ?? res.data.total ?? categories.value.length
    }
  } catch (err) {
    console.error('Gagal memuat kategori pendapatan:', err)
  } finally {
    isCategoriesLoading.value = false
  }
}

async function fetchAccounts() {
  try {
    const res = await api.get('/other-income/accounts')
    if (res.data?.success) {
      paymentAccounts.value = res.data.data.payment_accounts || []
      incomeAccounts.value = res.data.data.income_accounts || []

      // Set default payment account jika belum ada
      if (!newIncome.value.payment_account_id && paymentAccounts.value.length > 0) {
        const cashAcc = paymentAccounts.value.find((a) => a.code === '1101') || paymentAccounts.value[0]
        newIncome.value.payment_account_id = cashAcc.id
      }
    }
  } catch (err) {
    console.error('Gagal memuat akun akuntansi:', err)
  }
}

async function checkActiveShift() {
  try {
    const res = await api.get('/pos/shifts/current')
    if (res.data?.success && res.data.data) {
      activeShiftInfo.value = res.data.data
      linkActiveShift.value = true
      newIncome.value.shift_id = res.data.data.id
    } else {
      activeShiftInfo.value = null
      linkActiveShift.value = false
      newIncome.value.shift_id = null
    }
  } catch {
    activeShiftInfo.value = null
    linkActiveShift.value = false
    newIncome.value.shift_id = null
  }
}

function openCreateModal() {
  newIncome.value = {
    outlet_id: selectedOutletId.value || locationStore.currentOutletId || '',
    category_id: categories.value.length > 0 ? categories.value[0].id : '',
    income_account_id: '',
    payment_account_id: '',
    amount: 0,
    transaction_date: new Date().toISOString().split('T')[0],
    payer_name: '',
    notes: '',
    reference_no: '',
    attachment_url: '',
    shift_id: null,
  }

  // Pilih payment account default (1101 Kasir atau pertama)
  if (paymentAccounts.value.length > 0) {
    const cashAcc = paymentAccounts.value.find((a) => a.code === '1101') || paymentAccounts.value[0]
    newIncome.value.payment_account_id = cashAcc.id
  }

  // Sinkronkan akun pendapatan default dari kategori pertama
  if (newIncome.value.category_id) {
    const cat = categories.value.find((c) => c.id === newIncome.value.category_id)
    if (cat?.account_id) {
      newIncome.value.income_account_id = cat.account_id
    } else if (incomeAccounts.value.length > 0) {
      newIncome.value.income_account_id = incomeAccounts.value[0].id
    }
  }

  showAdvancedAccountingOption.value = false
  checkActiveShift()
  showCreateModal.value = true
}

async function handleCreateIncome() {
  if (Number(newIncome.value.amount) <= 0) {
    dialog.alert('Nominal pendapatan harus lebih besar dari Rp 0.', 'danger')
    return
  }

  if (!newIncome.value.outlet_id) {
    dialog.alert('Silakan pilih cabang / outlet yang menerima pendapatan.', 'danger')
    return
  }

  if (!newIncome.value.payment_account_id) {
    dialog.alert('Pilih rekening atau kas tempat dana diterima.', 'danger')
    return
  }

  if (!newIncome.value.income_account_id) {
    dialog.alert('Pilih akun pendapatan akuntansi (CoA) untuk transaksi ini.', 'danger')
    return
  }

  isSubmitting.value = true
  try {
    const payload = {
      ...newIncome.value,
      amount: Number(newIncome.value.amount),
      shift_id: linkActiveShift.value ? activeShiftInfo.value?.id : null,
    }

    const res = await api.post('/other-income', payload)
    if (res.data?.success) {
      showCreateModal.value = false
      dialog.alert(`Transaksi #${res.data.data.transaction_no} sebesar ${formatCurrency(res.data.data.amount)} berhasil dibukukan ke jurnal umum.`, 'success')
      fetchOtherIncomes()
      fetchSummary()
    }
  } catch (err: any) {
    const msg = err.response?.data?.message || err.message || 'Gagal menyimpan transaksi pendapatan'
    dialog.alert(msg, 'danger')
  } finally {
    isSubmitting.value = false
  }
}

function openDetailModal(item: OtherIncomeTransaction) {
  selectedIncome.value = item
  showDetailModal.value = true
}

function openCancelModal(item: OtherIncomeTransaction) {
  selectedIncome.value = item
  cancelReason.value = ''
  showCancelModal.value = true
}

async function handleCancelIncome() {
  if (!selectedIncome.value) return
  if (!cancelReason.value.trim() || cancelReason.value.trim().length < 3) {
    dialog.alert('Mohon cantumkan alasan pembatalan minimal 3 karakter untuk kebutuhan audit akuntansi.', 'danger')
    return
  }

  const txNo = selectedIncome.value.transaction_no
  isSubmitting.value = true
  try {
    const res = await api.post(`/other-income/${selectedIncome.value.id}/cancel`, {
      reason: cancelReason.value.trim(),
    })
    if (res.data?.success) {
      showCancelModal.value = false
      if (showDetailModal.value && selectedIncome.value?.id === res.data.data.id) {
        selectedIncome.value = res.data.data
      }
      dialog.alert(`Transaksi #${txNo} berhasil dibatalkan dan Jurnal Pembalik (Reversal Entry) telah diterbitkan.`, 'success')
      fetchOtherIncomes()
      fetchSummary()
    }
  } catch (err: any) {
    const msg = err.response?.data?.message || err.message || 'Gagal membatalkan transaksi pendapatan'
    dialog.alert(msg, 'danger')
  } finally {
    isSubmitting.value = false
  }
}

// Category Management Handlers
function openCreateCategoryModal() {
  isEditingCategory.value = false
  categoryForm.value = {
    id: '',
    code: '',
    name: '',
    account_id: incomeAccounts.value.length > 0 ? incomeAccounts.value[0].id : '',
    description: '',
    is_active: true,
  }
  showCategoryModal.value = true
}

function openEditCategoryModal(cat: OtherIncomeCategory) {
  isEditingCategory.value = true
  categoryForm.value = {
    id: cat.id,
    code: cat.code,
    name: cat.name,
    account_id: cat.account_id || '',
    description: cat.description || '',
    is_active: cat.is_active,
  }
  showCategoryModal.value = true
}

async function handleSaveCategory() {
  if (!categoryForm.value.name.trim()) {
    dialog.alert('Nama kategori tidak boleh kosong.', 'danger')
    return
  }

  isSubmitting.value = true
  try {
    if (isEditingCategory.value) {
      const res = await api.put(`/other-income/categories/${categoryForm.value.id}`, {
        name: categoryForm.value.name,
        account_id: categoryForm.value.account_id || null,
        description: categoryForm.value.description || null,
        is_active: categoryForm.value.is_active,
      })
      if (res.data?.success) {
        showCategoryModal.value = false
        fetchCategories()
      }
    } else {
      const res = await api.post('/other-income/categories', {
        code: categoryForm.value.code,
        name: categoryForm.value.name,
        account_id: categoryForm.value.account_id || null,
        description: categoryForm.value.description || null,
      })
      if (res.data?.success) {
        showCategoryModal.value = false
        fetchCategories()
      }
    }
  } catch (err: any) {
    const msg = err.response?.data?.message || err.message || 'Gagal menyimpan kategori'
    dialog.alert(msg, 'danger')
  } finally {
    isSubmitting.value = false
  }
}

onMounted(async () => {
  applyDatePreset('month')
  await Promise.all([fetchCategories(), fetchAccounts()])
})
</script>

<template>
  <AdminLayout>
    <div class="space-y-6 max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6">
      <!-- HEADER & ACTIONS -->
      <div class="flex flex-col md:flex-row md:items-center md:justify-between gap-4">
        <div>
          <div class="flex items-center gap-2">
            <h1 class="text-2xl font-bold tracking-tight text-slate-900 dark:text-white">
              Pendapatan Lain-lain
            </h1>
            <span class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-semibold bg-emerald-100 text-emerald-800 dark:bg-emerald-900/40 dark:text-emerald-300 border border-emerald-300 dark:border-emerald-700">
              Double-Entry SAK
            </span>
          </div>
          <p class="text-sm text-slate-500 dark:text-slate-400 mt-1">
            Pencatatan pendapatan di luar transaksi penjualan kasir (sewa booth promo brand, bagi hasil komisi, penjualan kardus/limbah, bunga bank, dll).
          </p>
        </div>

        <div class="flex items-center gap-3">
          <button
            @click="fetchOtherIncomes(); fetchSummary()"
            :disabled="isLoading"
            class="inline-flex items-center gap-1.5 px-3 py-2 text-sm font-medium text-slate-700 dark:text-slate-200 bg-white dark:bg-slate-800 border border-slate-300 dark:border-slate-700 rounded-lg hover:bg-slate-50 dark:hover:bg-slate-700/60 shadow-sm transition-all"
          >
            <RefreshCw class="w-4 h-4" :class="{ 'animate-spin': isLoading }" />
            <span>Muat Ulang</span>
          </button>

          <button
            v-if="authStore.hasPermission(['other_income:create'])"
            @click="openCreateModal"
            class="inline-flex items-center gap-2 px-4 py-2 text-sm font-semibold text-white bg-emerald-600 hover:bg-emerald-700 rounded-lg shadow-sm hover:shadow transition-all"
          >
            <Plus class="w-4 h-4" />
            <span>Catat Pendapatan</span>
          </button>
        </div>
      </div>

      <!-- METRIC SUMMARY CARDS -->
      <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <!-- Total Pendapatan Sesuai Filter -->
        <div class="bg-white dark:bg-slate-800 rounded-xl p-5 border border-slate-200 dark:border-slate-700 shadow-sm">
          <div class="flex items-center justify-between">
            <span class="text-xs font-medium uppercase tracking-wider text-slate-500 dark:text-slate-400">Total Pendapatan</span>
            <div class="w-9 h-9 rounded-lg bg-emerald-100 dark:bg-emerald-950/60 flex items-center justify-center text-emerald-600 dark:text-emerald-400">
              <TrendingUp class="w-5 h-5" />
            </div>
          </div>
          <div class="mt-3">
            <div class="text-2xl font-bold text-slate-900 dark:text-white">
              {{ formatCurrency(summary?.total_amount || 0) }}
            </div>
            <p class="text-xs text-slate-500 dark:text-slate-400 mt-1">
              Dari <span class="font-semibold text-slate-700 dark:text-slate-200">{{ summary?.total_count || 0 }}</span> transaksi aktif
            </p>
          </div>
        </div>

        <!-- Pendapatan Hari Ini -->
        <div class="bg-white dark:bg-slate-800 rounded-xl p-5 border border-slate-200 dark:border-slate-700 shadow-sm">
          <div class="flex items-center justify-between">
            <span class="text-xs font-medium uppercase tracking-wider text-slate-500 dark:text-slate-400">Hari Ini</span>
            <div class="w-9 h-9 rounded-lg bg-teal-100 dark:bg-teal-950/60 flex items-center justify-center text-teal-600 dark:text-teal-400">
              <Calendar class="w-5 h-5" />
            </div>
          </div>
          <div class="mt-3">
            <div class="text-2xl font-bold text-slate-900 dark:text-white">
              {{ formatCurrency(summary?.today_amount || 0) }}
            </div>
            <p class="text-xs text-slate-500 dark:text-slate-400 mt-1">
              Penerimaan kas/bank hari ini
            </p>
          </div>
        </div>

        <!-- Pendapatan Bulan Ini -->
        <div class="bg-white dark:bg-slate-800 rounded-xl p-5 border border-slate-200 dark:border-slate-700 shadow-sm">
          <div class="flex items-center justify-between">
            <span class="text-xs font-medium uppercase tracking-wider text-slate-500 dark:text-slate-400">Bulan Ini</span>
            <div class="w-9 h-9 rounded-lg bg-blue-100 dark:bg-blue-950/60 flex items-center justify-center text-blue-600 dark:text-blue-400">
              <Receipt class="w-5 h-5" />
            </div>
          </div>
          <div class="mt-3">
            <div class="text-2xl font-bold text-slate-900 dark:text-white">
              {{ formatCurrency(summary?.this_month_amount || 0) }}
            </div>
            <p class="text-xs text-slate-500 dark:text-slate-400 mt-1">
              Akumulasi pendapatan bulan berjalan
            </p>
          </div>
        </div>

        <!-- Master Kategori -->
        <div class="bg-white dark:bg-slate-800 rounded-xl p-5 border border-slate-200 dark:border-slate-700 shadow-sm">
          <div class="flex items-center justify-between">
            <span class="text-xs font-medium uppercase tracking-wider text-slate-500 dark:text-slate-400">Kategori Akun</span>
            <div class="w-9 h-9 rounded-lg bg-purple-100 dark:bg-purple-950/60 flex items-center justify-center text-purple-600 dark:text-purple-400">
              <Tag class="w-5 h-5" />
            </div>
          </div>
          <div class="mt-3">
            <div class="text-2xl font-bold text-slate-900 dark:text-white">
              {{ categoryTotalItems }}
            </div>
            <p class="text-xs text-slate-500 dark:text-slate-400 mt-1">
              Kategori terhubung ke Chart of Accounts
            </p>
          </div>
        </div>
      </div>

      <!-- TABS NAVIGATION -->
      <div class="border-b border-slate-200 dark:border-slate-700">
        <nav class="flex gap-6 -mb-px">
          <button
            @click="activeTab = 'transactions'"
            class="py-3 px-1 text-sm font-semibold border-b-2 transition-all flex items-center gap-2"
            :class="[
              activeTab === 'transactions'
                ? 'border-emerald-600 text-emerald-600 dark:text-emerald-400 dark:border-emerald-400'
                : 'border-transparent text-slate-500 dark:text-slate-400 hover:text-slate-700 dark:hover:text-slate-300'
            ]"
          >
            <TrendingUp class="w-4 h-4" />
            <span>Riwayat Transaksi Pendapatan</span>
            <span class="ml-1.5 px-2 py-0.5 text-xs rounded-full bg-slate-100 dark:bg-slate-700 text-slate-700 dark:text-slate-300">
              {{ totalItems }}
            </span>
          </button>

          <button
            @click="activeTab = 'categories'"
            class="py-3 px-1 text-sm font-semibold border-b-2 transition-all flex items-center gap-2"
            :class="[
              activeTab === 'categories'
                ? 'border-emerald-600 text-emerald-600 dark:text-emerald-400 dark:border-emerald-400'
                : 'border-transparent text-slate-500 dark:text-slate-400 hover:text-slate-700 dark:hover:text-slate-300'
            ]"
          >
            <Tag class="w-4 h-4" />
            <span>Kategori Pendapatan & CoA</span>
            <span class="ml-1.5 px-2 py-0.5 text-xs rounded-full bg-slate-100 dark:bg-slate-700 text-slate-700 dark:text-slate-300">
              {{ categoryTotalItems }}
            </span>
          </button>
        </nav>
      </div>

      <!-- TAB 1: TRANSACTIONS -->
      <div v-if="activeTab === 'transactions'" class="space-y-4">
        <!-- FILTER BAR -->
        <div class="bg-white dark:bg-slate-800 p-4 rounded-xl border border-slate-200 dark:border-slate-700 shadow-sm space-y-4">
          <div class="grid grid-cols-1 md:grid-cols-12 gap-3">
            <!-- Outlet Filter -->
            <div class="md:col-span-3">
              <label class="block text-xs font-medium text-slate-500 dark:text-slate-400 mb-1">Outlet / Toko</label>
              <SearchableSelect
                v-model="selectedOutletId"
                api-endpoint="/outlets"
                :extra-params="{ limit: 100 }"
                value-key="id"
                label-key="name"
                secondary-key="code"
                placeholder="Semua Outlet"
                search-placeholder="Cari outlet..."
                :allow-clear="true"
                @change="handleFilterChange"
              />
            </div>

            <!-- Kategori Filter -->
            <div class="md:col-span-3">
              <label class="block text-xs font-medium text-slate-500 dark:text-slate-400 mb-1">Kategori</label>
              <SearchableSelect
                v-model="selectedCategoryId"
                api-endpoint="/other-income/categories"
                value-key="id"
                label-key="name"
                secondary-key="code"
                placeholder="Semua Kategori"
                search-placeholder="Cari kategori..."
                :allow-clear="true"
                @change="handleFilterChange"
              />
            </div>

            <!-- Status Filter -->
            <div class="md:col-span-2">
              <label class="block text-xs font-medium text-slate-500 dark:text-slate-400 mb-1">Status</label>
              <SearchableSelect
                v-model="selectedStatus"
                :options="statusOptions"
                value-key="id"
                label-key="name"
                placeholder="Semua Status"
                search-placeholder="Cari status..."
                :allow-clear="true"
                @change="handleFilterChange"
              />
            </div>

            <!-- Search -->
            <div class="md:col-span-4">
              <label class="block text-xs font-medium text-slate-500 dark:text-slate-400 mb-1">Cari Transaksi</label>
              <div class="relative">
                <Search class="w-4 h-4 absolute left-3 top-2.5 text-slate-400" />
                <input
                  type="text"
                  v-model="searchQuery"
                  @keyup.enter="handleFilterChange"
                  placeholder="No transaksi, penyetor, catatan..."
                  class="w-full pl-9 pr-3 py-2 text-sm rounded-lg border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 text-slate-800 dark:text-slate-200 placeholder-slate-400 focus:ring-emerald-500 focus:border-emerald-500"
                />
              </div>
            </div>
          </div>

          <!-- Date Preset Buttons -->
          <div class="flex flex-wrap items-center gap-2 pt-2 border-t border-slate-100 dark:border-slate-700/60">
            <span class="text-xs font-medium text-slate-500 dark:text-slate-400 mr-1">Periode:</span>
            <button
              v-for="preset in [
                { id: 'today', label: 'Hari Ini' },
                { id: '7days', label: '7 Hari Terakhir' },
                { id: 'month', label: 'Bulan Ini' },
                { id: 'all', label: 'Semua Waktu' }
              ]"
              :key="preset.id"
              @click="applyDatePreset(preset.id as any)"
              class="px-2.5 py-1 text-xs rounded-md font-medium transition-all"
              :class="[
                datePreset === preset.id
                  ? 'bg-emerald-600 text-white shadow-sm'
                  : 'bg-slate-100 dark:bg-slate-700 text-slate-600 dark:text-slate-300 hover:bg-slate-200 dark:hover:bg-slate-600'
              ]"
            >
              {{ preset.label }}
            </button>

            <!-- Custom date range input -->
            <div class="ml-auto flex items-center gap-2 text-xs">
              <input
                type="date"
                v-model="startDate"
                @change="handleFilterChange"
                class="px-2 py-1 text-xs rounded border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 text-slate-800 dark:text-slate-200"
              />
              <span class="text-slate-400">s/d</span>
              <input
                type="date"
                v-model="endDate"
                @change="handleFilterChange"
                class="px-2 py-1 text-xs rounded border border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 text-slate-800 dark:text-slate-200"
              />
            </div>
          </div>
        </div>

        <!-- TRANSACTIONS TABLE -->
        <div class="bg-white dark:bg-slate-800 rounded-xl border border-slate-200 dark:border-slate-700 overflow-hidden shadow-sm">
          <div class="overflow-x-auto">
            <table class="min-w-full divide-y divide-slate-200 dark:divide-slate-700 text-sm">
              <thead class="bg-slate-50 dark:bg-slate-900/50 text-slate-600 dark:text-slate-400">
                <tr>
                  <th class="py-3 px-4 text-left font-semibold">No. Transaksi</th>
                  <th class="py-3 px-4 text-left font-semibold">Tanggal</th>
                  <th class="py-3 px-4 text-left font-semibold">Kategori</th>
                  <th class="py-3 px-4 text-left font-semibold">Pembayar / Pihak Ketiga</th>
                  <th class="py-3 px-4 text-left font-semibold">Akun Kas (Debet)</th>
                  <th class="py-3 px-right font-semibold text-right">Nominal</th>
                  <th class="py-3 px-4 text-center font-semibold">Status</th>
                  <th class="py-3 px-4 text-center font-semibold">Aksi</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-slate-200 dark:divide-slate-700">
                <tr v-if="isLoading" class="text-center">
                  <td colspan="8" class="py-12 text-slate-500">
                    <div class="flex items-center justify-center gap-2">
                      <RefreshCw class="w-5 h-5 animate-spin text-emerald-600" />
                      <span>Memuat data transaksi pendapatan...</span>
                    </div>
                  </td>
                </tr>

                <tr v-else-if="otherIncomes.length === 0" class="text-center">
                  <td colspan="8" class="py-12">
                    <div class="flex flex-col items-center justify-center">
                      <div class="w-12 h-12 rounded-full bg-emerald-50 dark:bg-emerald-950/40 flex items-center justify-center text-emerald-600 dark:text-emerald-400 mb-3">
                        <TrendingUp class="w-6 h-6" />
                      </div>
                      <p class="text-base font-medium text-slate-800 dark:text-slate-200">Belum ada transaksi pendapatan</p>
                      <p class="text-xs text-slate-500 dark:text-slate-400 mt-1 max-w-sm">
                        Catat penerimaan kas baru di luar penjualan retail melalui tombol "Catat Pendapatan" di atas.
                      </p>
                    </div>
                  </td>
                </tr>

                <tr
                  v-else
                  v-for="item in otherIncomes"
                  :key="item.id"
                  class="hover:bg-slate-50 dark:hover:bg-slate-750 transition-colors"
                  :class="{ 'opacity-60 bg-slate-50/50 dark:bg-slate-900/30': item.status === 'CANCELLED' }"
                >
                  <!-- No Transaksi -->
                  <td class="py-3.5 px-4 font-mono font-medium text-slate-900 dark:text-white">
                    <div class="flex items-center gap-1.5">
                      <CopyableText :text="item.transaction_no" class="hover:text-emerald-600" />
                    </div>
                    <div class="text-xs text-slate-400 font-sans mt-0.5">
                      {{ item.outlet_name }}
                    </div>
                  </td>

                  <!-- Tanggal -->
                  <td class="py-3.5 px-4 text-slate-700 dark:text-slate-300">
                    <div>{{ formatDate(item.transaction_date) }}</div>
                    <div v-if="item.created_by_name" class="text-xs text-slate-400 mt-0.5">
                      Oleh: {{ item.created_by_name }}
                    </div>
                  </td>

                  <!-- Kategori -->
                  <td class="py-3.5 px-4">
                    <span class="inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-xs font-medium border" :class="getCategoryMeta(item.category_name || '').color">
                      <span>{{ getCategoryMeta(item.category_name || '').icon }}</span>
                      <span>{{ item.category_name || 'Lain-lain' }}</span>
                    </span>
                    <div v-if="item.notes" class="text-xs text-slate-500 dark:text-slate-400 mt-1 line-clamp-1 max-w-xs" :title="item.notes">
                      {{ item.notes }}
                    </div>
                  </td>

                  <!-- Pembayar -->
                  <td class="py-3.5 px-4 text-slate-700 dark:text-slate-300">
                    <div class="font-medium text-slate-900 dark:text-white">
                      {{ item.payer_name || '-' }}
                    </div>
                    <div v-if="item.reference_no" class="text-xs text-slate-400 font-mono mt-0.5">
                      Ref: {{ item.reference_no }}
                    </div>
                  </td>

                  <!-- Akun Kas Masuk -->
                  <td class="py-3.5 px-4 text-slate-600 dark:text-slate-300">
                    <div class="font-medium flex items-center gap-1.5">
                      <Wallet class="w-3.5 h-3.5 text-emerald-600" />
                      <span>{{ item.payment_account_name }}</span>
                    </div>
                    <div class="text-xs text-slate-400 font-mono mt-0.5">
                      CoA: {{ item.payment_account_code }}
                    </div>
                  </td>

                  <!-- Nominal -->
                  <td class="py-3.5 px-4 text-right font-semibold text-emerald-600 dark:text-emerald-400" :class="{ 'line-through text-slate-400': item.status === 'CANCELLED' }">
                    {{ formatCurrency(item.amount) }}
                  </td>

                  <!-- Status -->
                  <td class="py-3.5 px-4 text-center">
                    <span
                      v-if="item.status === 'COMPLETED'"
                      class="inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-xs font-semibold bg-emerald-50 text-emerald-700 dark:bg-emerald-950/40 dark:text-emerald-300 border border-emerald-200 dark:border-emerald-800"
                    >
                      <CheckCircle2 class="w-3 h-3" />
                      <span>Selesai</span>
                    </span>
                    <span
                      v-else
                      class="inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-xs font-semibold bg-rose-50 text-rose-700 dark:bg-rose-950/40 dark:text-rose-300 border border-rose-200 dark:border-rose-800"
                      :title="item.cancellation_reason || 'Dibatalkan'"
                    >
                      <RotateCcw class="w-3 h-3" />
                      <span>Dibatalkan</span>
                    </span>
                  </td>

                  <!-- Aksi -->
                  <td class="py-3.5 px-4 text-center">
                    <div class="flex items-center justify-center gap-1.5">
                      <button
                        @click="openDetailModal(item)"
                        class="p-1.5 text-slate-600 hover:text-emerald-600 dark:text-slate-400 dark:hover:text-emerald-400 hover:bg-slate-100 dark:hover:bg-slate-700 rounded-lg transition-colors"
                        title="Lihat Rincian Transaksi & Jurnal"
                      >
                        <Eye class="w-4 h-4" />
                      </button>

                      <button
                        v-if="item.status === 'COMPLETED' && authStore.hasPermission(['other_income:delete'])"
                        @click="openCancelModal(item)"
                        class="p-1.5 text-slate-400 hover:text-rose-600 dark:hover:text-rose-400 hover:bg-rose-50 dark:hover:bg-rose-950/40 rounded-lg transition-colors"
                        title="Batalkan / Void Transaksi"
                      >
                        <RotateCcw class="w-4 h-4" />
                      </button>
                    </div>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <!-- Pagination Bar -->
          <div v-if="totalItems > 0" class="p-4 border-t border-slate-200 dark:border-slate-700">
            <PaginationBar
              v-model:currentPage="currentPage"
              :totalItems="totalItems"
              :pageSize="pageSize"
              @update:currentPage="fetchOtherIncomes"
            />
          </div>
        </div>
      </div>

      <!-- TAB 2: CATEGORIES -->
      <div v-if="activeTab === 'categories'" class="space-y-4">
        <!-- FILTER & ACTION BAR -->
        <div class="bg-white dark:bg-slate-800 p-4 rounded-xl border border-slate-200 dark:border-slate-700 shadow-sm space-y-4">
          <div class="flex flex-col md:flex-row md:items-center md:justify-between gap-4">
            <div>
              <h3 class="text-base font-semibold text-slate-900 dark:text-white">Master Kategori Pendapatan Lain</h3>
              <p class="text-xs text-slate-500 dark:text-slate-400">
                Setiap kategori secara otomatis memetakan transaksi ke Akun Pendapatan (REVENUE) di Chart of Accounts.
              </p>
            </div>

            <div class="flex items-center gap-2">
              <button
                @click="fetchCategories"
                :disabled="isCategoriesLoading"
                class="inline-flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium text-slate-700 dark:text-slate-200 bg-white dark:bg-slate-800 border border-slate-300 dark:border-slate-700 rounded-lg hover:bg-slate-50 dark:hover:bg-slate-700/60 shadow-sm transition-all"
              >
                <RefreshCw class="w-3.5 h-3.5" :class="{ 'animate-spin': isCategoriesLoading }" />
                <span>Muat Ulang</span>
              </button>

              <button
                v-if="authStore.hasPermission(['other_income:categories'])"
                @click="openCreateCategoryModal"
                class="inline-flex items-center gap-2 px-3 py-1.5 text-xs font-semibold text-white bg-emerald-600 hover:bg-emerald-700 rounded-lg shadow-sm transition-all"
              >
                <Plus class="w-3.5 h-3.5" />
                <span>Tambah Kategori</span>
              </button>
            </div>
          </div>

          <!-- SEARCH, SORT, & FILTER CONTROLS -->
          <div class="grid grid-cols-1 sm:grid-cols-12 gap-3 pt-3 border-t border-slate-100 dark:border-slate-700/60">
            <!-- Search Bar -->
            <div class="sm:col-span-5">
              <div class="relative">
                <Search class="w-4 h-4 absolute left-3 top-2.5 text-slate-400" />
                <input
                  type="text"
                  v-model="categorySearch"
                  @input="onCategorySearchInput"
                  placeholder="Cari nama, kode kategori, atau akun CoA..."
                  class="w-full pl-9 pr-8 py-2 text-xs rounded-lg border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 text-slate-800 dark:text-slate-200 placeholder-slate-400 focus:ring-emerald-500 focus:border-emerald-500"
                />
                <button
                  v-if="categorySearch"
                  @click="categorySearch = ''; handleCategoryFilterChange()"
                  class="absolute right-2.5 top-2.5 text-slate-400 hover:text-slate-600 dark:hover:text-slate-200"
                >
                  <X class="w-3.5 h-3.5" />
                </button>
              </div>
            </div>

            <!-- Sort By Field -->
            <div class="sm:col-span-3">
              <select
                v-model="categorySortBy"
                @change="handleCategoryFilterChange"
                class="w-full text-xs rounded-lg border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 text-slate-800 dark:text-slate-200 focus:ring-emerald-500 focus:border-emerald-500 py-2"
              >
                <option value="name">Urut: Nama Kategori</option>
                <option value="code">Urut: Kode Kategori</option>
                <option value="account_code">Urut: Kode Akun CoA</option>
                <option value="created_at">Urut: Waktu Ditambahkan</option>
                <option value="is_active">Urut: Status Aktif</option>
              </select>
            </div>

            <!-- Sort Direction Toggle -->
            <div class="sm:col-span-2">
              <button
                type="button"
                @click="toggleCategorySort(categorySortBy)"
                class="w-full py-2 px-3 text-xs font-medium border border-slate-300 dark:border-slate-700 rounded-lg bg-white dark:bg-slate-900 text-slate-700 dark:text-slate-200 hover:bg-slate-50 dark:hover:bg-slate-800 flex items-center justify-center gap-1.5 shadow-xs transition-colors"
                :title="categorySortOrder === 'asc' ? 'Urutan Naik (A-Z / Terlama)' : 'Urutan Turun (Z-A / Terbaru)'"
              >
                <ArrowUpDown class="w-3.5 h-3.5 text-emerald-600" />
                <span>{{ categorySortOrder === 'asc' ? 'A-Z (Naik)' : 'Z-A (Turun)' }}</span>
              </button>
            </div>

            <!-- Status Filter -->
            <div class="sm:col-span-2">
              <select
                v-model="categoryActiveOnly"
                @change="handleCategoryFilterChange"
                class="w-full text-xs rounded-lg border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 text-slate-800 dark:text-slate-200 focus:ring-emerald-500 focus:border-emerald-500 py-2"
              >
                <option :value="''">Semua Status</option>
                <option :value="true">Hanya Aktif</option>
              </select>
            </div>
          </div>
        </div>

        <!-- CATEGORIES LIST / CARDS -->
        <div v-if="isCategoriesLoading" class="py-12 text-center text-slate-500 bg-white dark:bg-slate-800 rounded-xl border border-slate-200 dark:border-slate-700">
          <div class="flex items-center justify-center gap-2">
            <RefreshCw class="w-5 h-5 animate-spin text-emerald-600" />
            <span class="text-xs">Memuat daftar kategori pendapatan...</span>
          </div>
        </div>

        <div v-else-if="categories.length === 0" class="bg-white dark:bg-slate-800 rounded-xl p-8 border border-slate-200 dark:border-slate-700 text-center">
          <Tag class="w-10 h-10 text-slate-400 mx-auto mb-2 opacity-60" />
          <h4 class="text-sm font-semibold text-slate-800 dark:text-slate-200">Tidak ada kategori ditemukan</h4>
          <p class="text-xs text-slate-500 dark:text-slate-400 mt-1">Coba sesuaikan kata kunci pencarian atau reset filter.</p>
          <button
            v-if="categorySearch || categoryActiveOnly !== ''"
            @click="categorySearch = ''; categoryActiveOnly = ''; handleCategoryFilterChange()"
            class="mt-3 px-3 py-1.5 text-xs text-emerald-600 hover:underline font-semibold"
          >
            Reset Filter & Pencarian
          </button>
        </div>

        <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          <div
            v-for="cat in categories"
            :key="cat.id"
            class="bg-white dark:bg-slate-800 rounded-xl p-4 border border-slate-200 dark:border-slate-700 shadow-sm relative group hover:border-emerald-500/50 transition-all flex flex-col justify-between"
            :class="{ 'opacity-70 bg-slate-50/50': !cat.is_active }"
          >
            <div>
              <div class="flex items-start justify-between">
                <div class="flex items-center gap-2.5">
                  <div class="text-2xl">{{ getCategoryMeta(cat.code).icon }}</div>
                  <div>
                    <h4 class="text-sm font-semibold text-slate-900 dark:text-white">{{ cat.name }}</h4>
                    <span class="font-mono text-xs text-slate-400">{{ cat.code }}</span>
                  </div>
                </div>

                <div class="flex items-center gap-1.5">
                  <span
                    class="text-[10px] font-semibold px-2 py-0.5 rounded-full"
                    :class="cat.is_active ? 'bg-emerald-100 text-emerald-800 dark:bg-emerald-950 dark:text-emerald-300' : 'bg-slate-100 text-slate-500 dark:bg-slate-700 dark:text-slate-400'"
                  >
                    {{ cat.is_active ? 'Aktif' : 'Nonaktif' }}
                  </span>

                  <button
                    v-if="authStore.hasPermission(['other_income:categories'])"
                    @click="openEditCategoryModal(cat)"
                    class="opacity-0 group-hover:opacity-100 text-xs text-emerald-600 hover:underline transition-opacity font-medium ml-1"
                  >
                    Ubah
                  </button>
                </div>
              </div>

              <p class="text-xs text-slate-500 dark:text-slate-400 mt-2.5 line-clamp-2">
                {{ cat.description || getCategoryMeta(cat.code).desc || '-' }}
              </p>
            </div>

            <div class="mt-4 pt-3 border-t border-slate-100 dark:border-slate-700/60 flex items-center justify-between text-xs">
              <span class="text-slate-500 dark:text-slate-400">Akun CoA:</span>
              <span class="font-mono font-medium text-emerald-700 dark:text-emerald-400 truncate max-w-[200px]" :title="cat.account_name || ''">
                {{ cat.account_code || '-' }} {{ cat.account_name ? `(${cat.account_name})` : '' }}
              </span>
            </div>
          </div>
        </div>

        <!-- Pagination Bar for Categories -->
        <div v-if="categoryTotalItems > 0" class="bg-white dark:bg-slate-800 rounded-xl p-3 border border-slate-200 dark:border-slate-700">
          <PaginationBar
            v-model:currentPage="categoryCurrentPage"
            v-model:pageSize="categoryPageSize"
            :totalItems="categoryTotalItems"
            :pageSizeOptions="[6, 12, 24, 48]"
            @update:currentPage="fetchCategories"
            @update:pageSize="handleCategoryPageSizeChange"
          />
        </div>
      </div>
    </div>

    <!-- MODAL 1: CATAT PENDAPATAN LAIN -->
    <Teleport to="body">
      <div
        v-if="showCreateModal"
        class="fixed inset-0 z-50 overflow-y-auto"
      >
        <!-- Backdrop -->
        <div
          class="fixed inset-0 bg-slate-900/60 backdrop-blur-xs transition-opacity"
          @click="showCreateModal = false"
        ></div>

        <!-- Scrollable wrapper -->
        <div class="flex min-h-full items-center justify-center p-4 text-center pointer-events-none">
          <div class="relative bg-white dark:bg-slate-800 rounded-2xl max-w-xl w-full border border-slate-200 dark:border-slate-700 shadow-xl overflow-hidden my-8 text-left pointer-events-auto">
            <div class="flex items-center justify-between px-6 py-4 border-b border-slate-200 dark:border-slate-700 bg-slate-50/50 dark:bg-slate-900/50">
              <div class="flex items-center gap-2.5">
                <div class="w-8 h-8 rounded-lg bg-emerald-100 dark:bg-emerald-950/60 flex items-center justify-center text-emerald-600 dark:text-emerald-400">
                  <TrendingUp class="w-4 h-4" />
                </div>
                <div>
                  <h3 class="text-base font-bold text-slate-900 dark:text-white">Catat Pendapatan Lain-lain</h3>
                  <p class="text-xs text-slate-500 dark:text-slate-400">Penerimaan kas/bank di luar transaksi kasir</p>
                </div>
              </div>
              <button @click="showCreateModal = false" class="text-slate-400 hover:text-slate-600 dark:hover:text-slate-200">
                <X class="w-5 h-5" />
              </button>
            </div>

        <form @submit.prevent="handleCreateIncome" class="p-6 space-y-4">
          <!-- Outlet & Tanggal -->
          <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <div>
              <label class="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">Outlet Penerima *</label>
              <SearchableSelect
                v-model="newIncome.outlet_id"
                api-endpoint="/outlets"
                :extra-params="{ limit: 100 }"
                value-key="id"
                label-key="name"
                secondary-key="code"
                placeholder="Pilih Outlet Penerima *"
                search-placeholder="Cari outlet..."
                :allow-clear="false"
                :required="true"
              />
            </div>

            <div>
              <label class="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">Tanggal Transaksi *</label>
              <input
                type="date"
                v-model="newIncome.transaction_date"
                required
                class="w-full text-sm rounded-lg border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 text-slate-800 dark:text-slate-200 focus:ring-emerald-500 focus:border-emerald-500"
              />
            </div>
          </div>

          <!-- Kategori Pendapatan -->
          <div>
            <label class="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">Kategori Pendapatan *</label>
            <SearchableSelect
              v-model="newIncome.category_id"
              api-endpoint="/other-income/categories"
              value-key="id"
              label-key="name"
              secondary-key="code"
              placeholder="Pilih Kategori Pendapatan *"
              search-placeholder="Cari kategori..."
              :allow-clear="false"
              :required="true"
              @change="onCategorySelected"
            />
          </div>

          <!-- Akun Kas / Bank Penerima -->
          <div>
            <label class="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">
              Diterima ke Rekening / Kas *
            </label>
            <SearchableSelect
              v-model="newIncome.payment_account_id"
              api-endpoint="/other-income/payment-accounts"
              value-key="id"
              label-key="name"
              secondary-key="code"
              placeholder="Pilih Rekening / Kas Penerima *"
              search-placeholder="Ketik cari kas / bank..."
              :allow-clear="false"
              :required="true"
            />
          </div>

          <!-- Nominal -->
          <div>
            <div class="flex items-center justify-between mb-1">
              <label class="block text-xs font-semibold text-slate-700 dark:text-slate-300">Nominal Pendapatan (Rp) *</label>
              <button
                type="button"
                @click="resetAmount"
                class="text-xs text-slate-400 hover:text-rose-500 transition-colors"
              >
                Reset
              </button>
            </div>
            <div class="relative">
              <span class="absolute left-3 top-2.5 text-sm font-bold text-slate-400">Rp</span>
              <input
                type="number"
                v-model="newIncome.amount"
                min="1"
                step="any"
                required
                placeholder="0"
                class="w-full pl-10 pr-3 py-2 text-lg font-bold text-emerald-700 dark:text-emerald-400 rounded-lg border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 focus:ring-emerald-500 focus:border-emerald-500 font-mono"
              />
            </div>

            <!-- Quick Amounts -->
            <div class="flex flex-wrap gap-1.5 mt-2">
              <button
                v-for="amt in quickAmounts"
                :key="amt"
                type="button"
                @click="addQuickAmount(amt)"
                class="px-2 py-1 text-xs font-medium rounded bg-slate-100 hover:bg-emerald-50 text-slate-700 hover:text-emerald-700 dark:bg-slate-700 dark:hover:bg-emerald-950/40 dark:text-slate-200 dark:hover:text-emerald-300 border border-slate-200 dark:border-slate-600 transition-colors"
              >
                +{{ formatCurrency(amt) }}
              </button>
            </div>
          </div>

          <!-- Pembayar & No Ref -->
          <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <div>
              <label class="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">Pembayar / Brand / Pihak Ketiga</label>
              <input
                type="text"
                v-model="newIncome.payer_name"
                placeholder="Misal: PT Unilever, Bu Siti Jastip, Pak Budi"
                class="w-full text-sm rounded-lg border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 text-slate-800 dark:text-slate-200 focus:ring-emerald-500 focus:border-emerald-500"
              />
            </div>

            <div>
              <label class="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">No. Referensi / Bukti Transfer</label>
              <input
                type="text"
                v-model="newIncome.reference_no"
                placeholder="Misal: KWT-001, TRF-BCA-987"
                class="w-full text-sm rounded-lg border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 text-slate-800 dark:text-slate-200 focus:ring-emerald-500 focus:border-emerald-500 font-mono"
              />
            </div>
          </div>

          <!-- Catatan -->
          <div>
            <label class="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">Keterangan / Catatan Tambahan</label>
            <textarea
              v-model="newIncome.notes"
              rows="2"
              placeholder="Uraian pendapatan atau rincian perjanjian promosi/kerjasama..."
              class="w-full text-sm rounded-lg border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 text-slate-800 dark:text-slate-200 focus:ring-emerald-500 focus:border-emerald-500"
            ></textarea>
          </div>

          <!-- Opsi Hubungkan ke Shift Kasir -->
          <div v-if="activeShiftInfo" class="p-3 bg-emerald-50/50 dark:bg-emerald-950/20 rounded-xl border border-emerald-200 dark:border-emerald-800/50">
            <label class="flex items-start gap-2.5 cursor-pointer">
              <input
                type="checkbox"
                v-model="linkActiveShift"
                class="mt-0.5 rounded text-emerald-600 focus:ring-emerald-500"
              />
              <div class="text-xs">
                <span class="font-semibold text-slate-800 dark:text-slate-200">Hubungkan ke Shift Kasir Aktif</span>
                <p class="text-slate-500 dark:text-slate-400 mt-0.5">
                  Shift aktif: <span class="font-mono font-medium">{{ activeShiftInfo.id.slice(0, 8) }}</span> (Kasir: {{ activeShiftInfo.cashier_name }}). Jika kas diterima langsung di laci kasir, saldo fisik saat tutup shift akan disesuaikan otomatis.
                </p>
              </div>
            </label>
          </div>

          <!-- Advanced Accounting Option (Ganti Akun CoA) -->
          <div>
            <button
              type="button"
              @click="showAdvancedAccountingOption = !showAdvancedAccountingOption"
              class="text-xs text-slate-500 dark:text-slate-400 hover:text-emerald-600 flex items-center gap-1 transition-colors"
            >
              <Sliders class="w-3.5 h-3.5" />
              <span>{{ showAdvancedAccountingOption ? 'Sembunyikan Opsi Akun Akuntansi' : 'Opsi Akun Akuntansi Lanjutan' }}</span>
              <ChevronDown class="w-3 h-3 transition-transform" :class="{ 'rotate-180': showAdvancedAccountingOption }" />
            </button>

            <div v-if="showAdvancedAccountingOption" class="mt-2.5 p-3 bg-slate-50 dark:bg-slate-900 rounded-lg border border-slate-200 dark:border-slate-700">
              <label class="block text-xs font-medium text-slate-600 dark:text-slate-300 mb-1">
                Akun Pendapatan Akuntansi (REVENUE CoA):
              </label>
              <SearchableSelect
                v-model="newIncome.income_account_id"
                api-endpoint="/other-income/income-accounts"
                value-key="id"
                label-key="name"
                secondary-key="code"
                placeholder="Pilih Akun Pendapatan (CoA)"
                search-placeholder="Ketik cari kode atau nama akun..."
                :allow-clear="false"
              />
            </div>
          </div>

          <!-- ACTIONS -->
          <div class="flex items-center justify-end gap-3 pt-4 border-t border-slate-200 dark:border-slate-700">
            <button
              type="button"
              @click="showCreateModal = false"
              class="px-4 py-2 text-sm font-medium text-slate-700 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700 rounded-lg transition-colors"
            >
              Batal
            </button>
            <button
              type="submit"
              :disabled="isSubmitting"
              class="inline-flex items-center gap-2 px-5 py-2 text-sm font-semibold text-white bg-emerald-600 hover:bg-emerald-700 rounded-lg shadow-sm transition-all disabled:opacity-50"
            >
              <RefreshCw v-if="isSubmitting" class="w-4 h-4 animate-spin" />
              <span>{{ isSubmitting ? 'Menyimpan...' : 'Simpan & Bukukan' }}</span>
            </button>
          </div>
        </form>
      </div>
    </div>
  </div>
</Teleport>

    <!-- MODAL 2: DETAIL TRANSAKSI & JURNAL -->
    <Teleport to="body">
      <div
        v-if="showDetailModal && selectedIncome"
        class="fixed inset-0 z-50 overflow-y-auto"
      >
        <!-- Backdrop -->
        <div
          class="fixed inset-0 bg-slate-900/60 backdrop-blur-xs transition-opacity"
          @click="showDetailModal = false"
        ></div>

        <!-- Scrollable wrapper -->
        <div class="flex min-h-full items-center justify-center p-4 text-center pointer-events-none">
          <div class="relative bg-white dark:bg-slate-800 rounded-2xl max-w-xl w-full border border-slate-200 dark:border-slate-700 shadow-xl overflow-hidden my-8 text-left pointer-events-auto">
            <div class="flex items-center justify-between px-6 py-4 border-b border-slate-200 dark:border-slate-700 bg-slate-50/50 dark:bg-slate-900/50">
          <div>
            <h3 class="text-base font-bold text-slate-900 dark:text-white">Rincian Pendapatan Lain-lain</h3>
            <p class="text-xs font-mono text-slate-500 dark:text-slate-400 mt-0.5">
              #{{ selectedIncome.transaction_no }}
            </p>
          </div>
          <button @click="showDetailModal = false" class="text-slate-400 hover:text-slate-600 dark:hover:text-slate-200">
            <X class="w-5 h-5" />
          </button>
        </div>

        <div class="p-6 space-y-4">
          <!-- Status Banner -->
          <div
            class="p-3 rounded-xl flex items-center justify-between text-xs font-medium"
            :class="[
              selectedIncome.status === 'COMPLETED'
                ? 'bg-emerald-50 text-emerald-800 border border-emerald-200 dark:bg-emerald-950/40 dark:text-emerald-300 dark:border-emerald-800'
                : 'bg-rose-50 text-rose-800 border border-rose-200 dark:bg-rose-950/40 dark:text-rose-300 dark:border-rose-800'
            ]"
          >
            <div class="flex items-center gap-2">
              <CheckCircle2 v-if="selectedIncome.status === 'COMPLETED'" class="w-4 h-4 text-emerald-600" />
              <RotateCcw v-else class="w-4 h-4 text-rose-600" />
              <span>Status: {{ selectedIncome.status === 'COMPLETED' ? 'Selesai & Aktif dalam Pembukuan' : 'Transaksi Dibatalkan (Void)' }}</span>
            </div>
            <span class="font-bold">{{ formatCurrency(selectedIncome.amount) }}</span>
          </div>

          <!-- Info Table -->
          <div class="grid grid-cols-2 gap-3 text-xs bg-slate-50 dark:bg-slate-900/50 p-3 rounded-xl border border-slate-200 dark:border-slate-700">
            <div>
              <span class="text-slate-400 block">Tanggal Transaksi:</span>
              <span class="font-semibold text-slate-800 dark:text-slate-200">{{ formatDate(selectedIncome.transaction_date) }}</span>
            </div>
            <div>
              <span class="text-slate-400 block">Outlet / Toko:</span>
              <span class="font-semibold text-slate-800 dark:text-slate-200">{{ selectedIncome.outlet_name }}</span>
            </div>
            <div>
              <span class="text-slate-400 block">Kategori:</span>
              <span class="font-semibold text-slate-800 dark:text-slate-200">{{ selectedIncome.category_name || '-' }}</span>
            </div>
            <div>
              <span class="text-slate-400 block">Pembayar / Pihak Ketiga:</span>
              <span class="font-semibold text-slate-800 dark:text-slate-200">{{ selectedIncome.payer_name || '-' }}</span>
            </div>
            <div>
              <span class="text-slate-400 block">No. Bukti / Referensi:</span>
              <span class="font-mono font-semibold text-slate-800 dark:text-slate-200">{{ selectedIncome.reference_no || '-' }}</span>
            </div>
            <div>
              <span class="text-slate-400 block">Dibuat Oleh:</span>
              <span class="font-semibold text-slate-800 dark:text-slate-200">{{ selectedIncome.created_by_name || '-' }}</span>
            </div>
          </div>

          <!-- Catatan -->
          <div v-if="selectedIncome.notes" class="text-xs p-3 bg-slate-50 dark:bg-slate-900/50 rounded-xl border border-slate-200 dark:border-slate-700">
            <span class="text-slate-400 block mb-1 font-semibold">Catatan / Keterangan:</span>
            <p class="text-slate-700 dark:text-slate-300 whitespace-pre-wrap">{{ selectedIncome.notes }}</p>
          </div>

          <!-- JURNAL AKUNTANSI SAK -->
          <div class="p-3 bg-slate-50 dark:bg-slate-900/50 rounded-xl border border-slate-200 dark:border-slate-700 space-y-2">
            <div class="flex items-center justify-between text-xs font-semibold text-slate-700 dark:text-slate-300">
              <span class="flex items-center gap-1.5">
                <FileText class="w-4 h-4 text-emerald-600" />
                <span>Pencatatan Jurnal Berpasangan (Double-Entry SAK)</span>
              </span>
              <span v-if="selectedIncome.journal_entry_id" class="font-mono text-slate-400">
                Terhubung Jurnal
              </span>
            </div>

            <div class="bg-white dark:bg-slate-800 rounded-lg p-2.5 border border-slate-200 dark:border-slate-700 text-xs font-mono space-y-1.5">
              <!-- Debet -->
              <div class="flex items-center justify-between text-slate-800 dark:text-slate-200">
                <span class="text-emerald-700 dark:text-emerald-400">
                  (D) {{ selectedIncome.payment_account_code }} - {{ selectedIncome.payment_account_name }}
                </span>
                <span class="font-bold">{{ formatCurrency(selectedIncome.amount) }}</span>
              </div>
              <!-- Kredit -->
              <div class="flex items-center justify-between text-slate-600 dark:text-slate-400 pl-4">
                <span>
                  (K) {{ selectedIncome.income_account_code }} - {{ selectedIncome.income_account_name }}
                </span>
                <span>{{ formatCurrency(selectedIncome.amount) }}</span>
              </div>
            </div>
          </div>

          <!-- Info Pembatalan (Jika Dibatalkan) -->
          <div v-if="selectedIncome.status === 'CANCELLED'" class="p-3 bg-rose-50/50 dark:bg-rose-950/20 rounded-xl border border-rose-200 dark:border-rose-800/60 text-xs space-y-1">
            <div class="font-semibold text-rose-800 dark:text-rose-300">Informasi Pembatalan Transaksi:</div>
            <div class="text-slate-600 dark:text-slate-400">
              Dibatalkan oleh <span class="font-semibold">{{ selectedIncome.cancelled_by_name || 'Sistem' }}</span> pada {{ formatDate(selectedIncome.cancelled_at) }}
            </div>
            <div class="text-rose-700 dark:text-rose-400 italic">
              "{{ selectedIncome.cancellation_reason }}"
            </div>
          </div>

          <!-- ACTIONS -->
          <div class="flex items-center justify-between pt-4 border-t border-slate-200 dark:border-slate-700">
            <button
              v-if="selectedIncome.status === 'COMPLETED' && authStore.hasPermission(['other_income:delete'])"
              @click="showDetailModal = false; openCancelModal(selectedIncome)"
              class="px-3 py-1.5 text-xs font-semibold text-rose-600 hover:text-white hover:bg-rose-600 border border-rose-300 dark:border-rose-800 rounded-lg transition-all"
            >
              Batalkan Transaksi (Void)
            </button>
            <div v-else></div>

            <button
              @click="showDetailModal = false"
              class="px-4 py-2 text-sm font-medium text-slate-700 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700 rounded-lg transition-colors"
            >
              Tutup
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</Teleport>

    <!-- MODAL 3: BATALKAN / VOID TRANSAKSI -->
    <Teleport to="body">
      <div
        v-if="showCancelModal && selectedIncome"
        class="fixed inset-0 z-50 overflow-y-auto"
      >
        <!-- Backdrop -->
        <div
          class="fixed inset-0 bg-slate-900/60 backdrop-blur-xs transition-opacity"
          @click="showCancelModal = false"
        ></div>

        <!-- Scrollable wrapper -->
        <div class="flex min-h-full items-center justify-center p-4 text-center pointer-events-none">
          <div class="relative bg-white dark:bg-slate-800 rounded-2xl max-w-md w-full border border-slate-200 dark:border-slate-700 shadow-xl overflow-hidden my-8 text-left pointer-events-auto">
            <div class="p-6 space-y-4">
          <div class="w-12 h-12 rounded-full bg-rose-100 dark:bg-rose-950/60 flex items-center justify-center text-rose-600 dark:text-rose-400 mx-auto">
            <AlertCircle class="w-6 h-6" />
          </div>

          <div class="text-center">
            <h3 class="text-lg font-bold text-slate-900 dark:text-white">Batalkan Transaksi Pendapatan</h3>
            <p class="text-xs text-slate-500 dark:text-slate-400 mt-1">
              Nomor: <span class="font-mono font-semibold text-slate-700 dark:text-slate-300">#{{ selectedIncome.transaction_no }}</span>
              ({{ formatCurrency(selectedIncome.amount) }})
            </p>
          </div>

          <div class="p-3 bg-rose-50 dark:bg-rose-950/30 rounded-xl border border-rose-200 dark:border-rose-800/50 text-xs text-rose-700 dark:text-rose-300">
            <strong>Perhatian:</strong> Sesuai standar akuntansi (SAK), pembatalan transaksi tidak akan menghapus data riil secara fisik, melainkan menerbitkan <strong>Jurnal Pembalik (Reversal Entry)</strong> yang membalik posisi Kas dan Pendapatan ke posisi netral.
          </div>

          <div>
            <label class="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">
              Alasan Pembatalan (Wajib diisi) *
            </label>
            <textarea
              v-model="cancelReason"
              rows="3"
              required
              placeholder="Contoh: Salah input nominal kasir, transfer batal oleh sponsor, salah pilih rekening kas..."
              class="w-full text-sm rounded-lg border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 text-slate-800 dark:text-slate-200 focus:ring-rose-500 focus:border-rose-500"
            ></textarea>
          </div>

          <div class="flex items-center justify-end gap-3 pt-2">
            <button
              type="button"
              @click="showCancelModal = false"
              class="px-4 py-2 text-sm font-medium text-slate-700 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700 rounded-lg transition-colors"
            >
              Kembali
            </button>
            <button
              type="button"
              @click="handleCancelIncome"
              :disabled="isSubmitting || !cancelReason.trim()"
              class="inline-flex items-center gap-2 px-4 py-2 text-sm font-semibold text-white bg-rose-600 hover:bg-rose-700 rounded-lg shadow-sm transition-all disabled:opacity-50"
            >
              <RefreshCw v-if="isSubmitting" class="w-4 h-4 animate-spin" />
              <span>{{ isSubmitting ? 'Memproses...' : 'Konfirmasi Batal (Void)' }}</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</Teleport>

    <!-- MODAL 4: KELOLA KATEGORI PENDAPATAN -->
    <Teleport to="body">
      <div
        v-if="showCategoryModal"
        class="fixed inset-0 z-50 overflow-y-auto"
      >
        <!-- Backdrop -->
        <div
          class="fixed inset-0 bg-slate-900/60 backdrop-blur-xs transition-opacity"
          @click="showCategoryModal = false"
        ></div>

        <!-- Scrollable wrapper -->
        <div class="flex min-h-full items-center justify-center p-4 text-center pointer-events-none">
          <div class="relative bg-white dark:bg-slate-800 rounded-2xl max-w-md w-full border border-slate-200 dark:border-slate-700 shadow-xl overflow-hidden my-8 text-left pointer-events-auto">
            <div class="flex items-center justify-between px-6 py-4 border-b border-slate-200 dark:border-slate-700 bg-slate-50/50 dark:bg-slate-900/50">
              <h3 class="text-base font-bold text-slate-900 dark:text-white">
                {{ isEditingCategory ? 'Ubah Kategori Pendapatan' : 'Tambah Kategori Pendapatan' }}
              </h3>
              <button @click="showCategoryModal = false" class="text-slate-400 hover:text-slate-600 dark:hover:text-slate-200">
                <X class="w-5 h-5" />
              </button>
            </div>

        <form @submit.prevent="handleSaveCategory" class="p-6 space-y-4">
          <div>
            <label class="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">Kode Kategori *</label>
            <input
              type="text"
              v-model="categoryForm.code"
              :disabled="isEditingCategory"
              required
              placeholder="Misal: SEWA_PROMOSI"
              class="w-full text-sm font-mono rounded-lg border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 text-slate-800 dark:text-slate-200 focus:ring-emerald-500 focus:border-emerald-500 disabled:opacity-50 uppercase"
            />
          </div>

          <div>
            <label class="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">Nama Kategori *</label>
            <input
              type="text"
              v-model="categoryForm.name"
              required
              placeholder="Misal: Sewa Tempat & Banner Promosi"
              class="w-full text-sm rounded-lg border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 text-slate-800 dark:text-slate-200 focus:ring-emerald-500 focus:border-emerald-500"
            />
          </div>

          <div>
            <label class="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">
              Akun Pendapatan Akuntansi (REVENUE CoA)
            </label>
            <SearchableSelect
              v-model="categoryForm.account_id"
              api-endpoint="/other-income/income-accounts"
              value-key="id"
              label-key="name"
              secondary-key="code"
              placeholder="-- Pilih Akun Pendapatan (CoA) --"
              search-placeholder="Ketik cari kode atau nama akun..."
              :allow-clear="true"
            />
          </div>

          <div>
            <label class="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">Deskripsi</label>
            <textarea
              v-model="categoryForm.description"
              rows="2"
              placeholder="Keterangan kategori pendapatan..."
              class="w-full text-sm rounded-lg border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-900 text-slate-800 dark:text-slate-200 focus:ring-emerald-500 focus:border-emerald-500"
            ></textarea>
          </div>

          <div v-if="isEditingCategory">
            <label class="flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                v-model="categoryForm.is_active"
                class="rounded text-emerald-600 focus:ring-emerald-500"
              />
              <span class="text-xs font-semibold text-slate-700 dark:text-slate-300">Status Kategori Aktif</span>
            </label>
          </div>

          <div class="flex items-center justify-end gap-3 pt-4 border-t border-slate-200 dark:border-slate-700">
            <button
              type="button"
              @click="showCategoryModal = false"
              class="px-4 py-2 text-sm font-medium text-slate-700 dark:text-slate-300 hover:bg-slate-100 dark:hover:bg-slate-700 rounded-lg transition-colors"
            >
              Batal
            </button>
            <button
              type="submit"
              :disabled="isSubmitting"
              class="inline-flex items-center gap-2 px-5 py-2 text-sm font-semibold text-white bg-emerald-600 hover:bg-emerald-700 rounded-lg shadow-sm transition-all disabled:opacity-50"
            >
              <RefreshCw v-if="isSubmitting" class="w-4 h-4 animate-spin" />
              <span>Simpan Kategori</span>
            </button>
          </div>
        </form>
      </div>
    </div>
  </div>
</Teleport>
  </AdminLayout>
</template>
