<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
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
  Clock,
  RotateCcw,
  CheckCircle2,
  AlertCircle,
  Printer,
  Calendar,
  DollarSign,
  Tag,
  Check,
  ChevronDown,
  Sliders,
  Info,
  Wallet,
  Building2,
  Sparkles,
  FileSpreadsheet,
} from 'lucide-vue-next'
import { Icon } from '@iconify/vue'
import { api } from '@/services/api'
import { useDialog } from '@/composables/useDialog'
import { useLocationStore } from '@/stores/locationStore'
import { useAuthStore } from '@/stores/authStore'
import { useExportStore } from '@/stores/exportStore'
import { formatCurrency, formatDate } from '@/utils/formatters'
import CopyableText from '@/components/common/CopyableText.vue'
import type {
  ExpenseTransaction,
  ExpenseCategory,
  ExpenseSummaryResponse,
  CreateExpenseRequest
} from '@/types'

const locationStore = useLocationStore()
const authStore = useAuthStore()
const exportStore = useExportStore()
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
const expenses = ref<ExpenseTransaction[]>([])
const categories = ref<ExpenseCategory[]>([])
const paymentAccounts = ref<any[]>([])
const expenseAccounts = ref<any[]>([])
const summary = ref<ExpenseSummaryResponse | null>(null)
const isLoading = ref<boolean>(false)
const isSubmitting = ref<boolean>(false)

// Modals
const showCreateModal = ref<boolean>(false)
const showDetailModal = ref<boolean>(false)
const showCancelModal = ref<boolean>(false)
const showCategoryModal = ref<boolean>(false)

// Selected Transaction for Detail / Cancel
const selectedExpense = ref<ExpenseTransaction | null>(null)
const cancelReason = ref<string>('')

// Form Create Expense
const newExpense = ref<CreateExpenseRequest>({
  outlet_id: locationStore.currentOutletId || '',
  category_id: '',
  expense_account_id: '',
  payment_account_id: '',
  amount: 0,
  transaction_date: new Date().toISOString().split('T')[0],
  recipient_name: '',
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
  newExpense.value.amount = (Number(newExpense.value.amount) || 0) + val
}

function resetAmount() {
  newExpense.value.amount = 0
}

const categoryMetaMap: Record<string, { icon: string; badge: string; color: string; desc: string }> = {
  UTILITAS: { icon: '⚡', badge: 'Utilitas', color: 'bg-amber-100 text-amber-900 border-amber-300', desc: 'Listrik PLN, PDAM, Pulsa & WiFi Toko' },
  PERLENGKAPAN: { icon: '🛍️', badge: 'Toko', color: 'bg-blue-100 text-blue-900 border-blue-300', desc: 'Plastik kresek, kertas thermal, lakban' },
  KONSUMSI: { icon: '☕', badge: 'Dapur', color: 'bg-emerald-100 text-emerald-900 border-emerald-300', desc: 'Air galon, kopi, teh, alat tulis kantor' },
  PEMELIHARAAN: { icon: '🔧', badge: 'Servis', color: 'bg-orange-100 text-orange-900 border-orange-300', desc: 'Servis AC, lampu, rak & komputer kasir' },
  KEBERSIHAN: { icon: '🧹', badge: 'Sanitasi', color: 'bg-teal-100 text-teal-900 border-teal-300', desc: 'Sabun cuci, pembersih lantai, alat pel' },
  TRANSPORT: { icon: '🛵', badge: 'Kurir', color: 'bg-sky-100 text-sky-900 border-sky-300', desc: 'Bensin motor kurir, parkir, jemput barang' },
  SEWA: { icon: '🏢', badge: 'Gedung', color: 'bg-purple-100 text-purple-900 border-purple-300', desc: 'Sewa ruko / lapak gerai toko bulanan' },
  GAJI: { icon: '👥', badge: 'SDM', color: 'bg-indigo-100 text-indigo-900 border-indigo-300', desc: 'Upah lembur, uang makan kasir, bonus staf' },
  PROMOSI: { icon: '📢', badge: 'Promo', color: 'bg-rose-100 text-rose-900 border-rose-300', desc: 'Spanduk, banner, brosur promo, iklan medsos' },
  BIAYA_BANK: { icon: '💳', badge: 'Bank', color: 'bg-cyan-100 text-cyan-900 border-cyan-300', desc: 'Biaya admin bank, fee transfer, MDR QRIS' },
  RETRIBUSI: { icon: '🏛️', badge: 'Iuran', color: 'bg-stone-100 text-stone-900 border-stone-300', desc: 'Iuran keamanan, sampah pasar, parkir resmi' },
  PRIVE: { icon: '👤', badge: 'Owner', color: 'bg-violet-100 text-violet-900 border-violet-300', desc: 'Tarik tunai uang toko untuk keperluan pribadi' },
  LAIN_LAIN: { icon: '📦', badge: 'Lainnya', color: 'bg-slate-100 text-slate-900 border-slate-300', desc: 'Pengeluaran operasional umum kasir lainnya' },
}

function getCategoryMeta(code: string) {
  return categoryMetaMap[code] || { icon: '🏷️', badge: 'Biaya', color: 'bg-slate-100 text-slate-800 border-slate-300', desc: 'Pengeluaran operasional toko' }
}

function formatPaymentAccountLabel(acc: any) {
  if (!acc) return ''
  if (acc.code === '1101') return '💵 Kas Tunai Laci Kasir (1101)'
  if (acc.code === '1104') return '🪙 Kas Kecil Toko (Petty Cash 1104)'
  if (acc.code?.startsWith('1102')) return `🏦 ${acc.name} (${acc.code})`
  return `💳 ${acc.name} (${acc.code})`
}

// Searchable options computed from API results
const paymentAccountOptions = computed(() => {
  return paymentAccounts.value.map((acc) => ({
    id: acc.id,
    name: formatPaymentAccountLabel(acc),
    code: acc.code,
    raw_name: acc.name,
  }))
})

const expenseAccountOptions = computed(() => {
  return expenseAccounts.value.map((acc) => ({
    id: acc.id,
    name: `${acc.code} - ${acc.name}`,
    code: acc.code,
  }))
})

const statusOptions = [
  { id: 'COMPLETED', name: 'Selesai (Aktif)' },
  { id: 'CANCELLED', name: 'Dibatalkan' },
]

function handleFilterChange() {
  currentPage.value = 1
  fetchExpenses()
  fetchSummary()
}

function handleOutletFilterChange(val: string) {
  selectedOutletId.value = val || ''
  currentPage.value = 1
  fetchExpenses()
  fetchSummary()
  fetchAccounts()
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

// Set default dates based on preset
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
  fetchExpenses()
  fetchSummary()
}

// Watch outlet changes from store
watch(
  () => locationStore.currentOutletId,
  (newId) => {
    if (newId) {
      selectedOutletId.value = newId
      newExpense.value.outlet_id = newId
      fetchExpenses()
      fetchSummary()
      fetchActiveShift()
    }
  }
)

onMounted(async () => {
  applyDatePreset('month')
  await Promise.all([
    fetchCategories(),
    fetchAccounts(),
    fetchActiveShift(),
  ])
})

async function fetchExpenses() {
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

    const res = await api.get('/expenses', { params })
    if (res.data?.success) {
      expenses.value = res.data.data || []
      totalItems.value = res.data.total || 0
      totalPages.value = res.data.total_pages || 1
    }
  } catch (err: any) {
    dialog.alert(err.response?.data?.message || 'Gagal memuat daftar transaksi pengeluaran.', 'danger')
  } finally {
    isLoading.value = false
  }
}

function exportExpensesToExcel() {
  const currentParams: Record<string, any> = {}
  if (selectedOutletId.value) currentParams.outlet_id = selectedOutletId.value
  if (selectedCategoryId.value) currentParams.category_id = selectedCategoryId.value
  if (selectedStatus.value) currentParams.status = selectedStatus.value
  if (startDate.value) currentParams.start_date = startDate.value
  if (endDate.value) currentParams.end_date = endDate.value
  if (searchQuery.value.trim()) currentParams.search = searchQuery.value.trim()

  exportStore.queueServerExport({
    title: 'Transaksi Pengeluaran Operasional',
    module: 'expenses',
    filterParams: currentParams,
    outletId: selectedOutletId.value || undefined,
  })
}

async function fetchSummary() {
  try {
    const params: Record<string, any> = {}
    if (selectedOutletId.value) params.outlet_id = selectedOutletId.value
    if (startDate.value) params.start_date = startDate.value
    if (endDate.value) params.end_date = endDate.value

    const res = await api.get('/expenses/summary', { params })
    if (res.data?.success) {
      summary.value = res.data.data
    }
  } catch (_) {
    // Fail gracefully
  }
}

async function fetchCategories() {
  try {
    const res = await api.get('/expenses/categories')
    if (res.data?.success) {
      categories.value = res.data.data || []
    }
  } catch (_) {}
}

async function fetchAccounts() {
  try {
    const params = selectedOutletId.value ? { outlet_id: selectedOutletId.value } : {}
    const res = await api.get('/expenses/accounts', { params })
    if (res.data?.success) {
      paymentAccounts.value = res.data.data.payment_accounts || []
      expenseAccounts.value = res.data.data.expense_accounts || []
    }
  } catch (_) {}
}

async function fetchActiveShift() {
  try {
    const res = await api.get('/shifts/current')
    if (res.data?.success && res.data?.data && res.data.data.status === 'OPEN') {
      activeShiftInfo.value = res.data.data
    } else {
      activeShiftInfo.value = null
    }
  } catch (_) {
    activeShiftInfo.value = null
  }
}

// When user picks category in Create Modal, auto-select linked expense account
function handleCategoryChange(catId: string) {
  newExpense.value.category_id = catId
  const found = categories.value.find((c) => c.id === catId)
  if (found && found.account_id) {
    newExpense.value.expense_account_id = found.account_id
  }
}

function openCreateModal() {
  if (!authStore.hasPermission('expenses:create')) {
    dialog.alert('Akses ditolak: Anda tidak memiliki izin mencatat transaksi pengeluaran.', 'warning')
    return
  }

  // Set default outlet
  const outletId = locationStore.currentOutletId || (locationStore.accessibleOutlets[0]?.id ?? '')
  
  // Set default payment account (prefer Kas Laci 1101 or Kas Kecil 1104)
  const defaultPay = paymentAccounts.value.find(a => a.code === '1101' || a.code === '1104') || paymentAccounts.value[0]
  const defaultCat = categories.value[0]

  newExpense.value = {
    outlet_id: outletId,
    category_id: defaultCat?.id || '',
    expense_account_id: defaultCat?.account_id || expenseAccounts.value[0]?.id || '',
    payment_account_id: defaultPay?.id || '',
    amount: 0,
    transaction_date: new Date().toISOString().split('T')[0],
    recipient_name: '',
    notes: '',
    reference_no: '',
    attachment_url: '',
    shift_id: null,
  }

  // If there's an active shift, link it by default if payment is Kas Laci
  if (activeShiftInfo.value?.id) {
    linkActiveShift.value = true
    newExpense.value.shift_id = activeShiftInfo.value.id
  } else {
    linkActiveShift.value = false
    newExpense.value.shift_id = null
  }

  showCreateModal.value = true
}

async function handleSaveExpense() {
  if (newExpense.value.amount <= 0) {
    dialog.alert('Nominal pengeluaran harus lebih besar dari Rp 0.', 'warning')
    return
  }
  if (!newExpense.value.expense_account_id) {
    dialog.alert('Silakan pilih Akun Beban Pengeluaran.', 'warning')
    return
  }
  if (!newExpense.value.payment_account_id) {
    dialog.alert('Silakan pilih Sumber Dana Pembayaran.', 'warning')
    return
  }

  // Set shift_id
  if (linkActiveShift.value && activeShiftInfo.value?.id) {
    newExpense.value.shift_id = activeShiftInfo.value.id
  } else {
    newExpense.value.shift_id = null
  }

  isSubmitting.value = true
  try {
    const res = await api.post('/expenses', newExpense.value)
    if (res.data?.success) {
      dialog.toast(res.data.message || 'Transaksi pengeluaran berhasil dicatat.', 'success')
      showCreateModal.value = false
      fetchExpenses()
      fetchSummary()
    }
  } catch (err: any) {
    dialog.alert(err.response?.data?.message || 'Gagal menyimpan transaksi pengeluaran.', 'danger')
  } finally {
    isSubmitting.value = false
  }
}

function openDetailModal(expense: ExpenseTransaction) {
  selectedExpense.value = expense
  showDetailModal.value = true
}

function openCancelModal(expense: ExpenseTransaction) {
  if (!authStore.hasPermission('expenses:delete')) {
    dialog.alert('Akses ditolak: Anda tidak memiliki izin membatalkan transaksi pengeluaran.', 'warning')
    return
  }
  selectedExpense.value = expense
  cancelReason.value = ''
  showCancelModal.value = true
}

async function handleExecuteCancel() {
  if (!selectedExpense.value) return
  if (!cancelReason.value.trim() || cancelReason.value.trim().length < 3) {
    dialog.alert('Harap isi alasan pembatalan pengeluaran (minimal 3 karakter).', 'warning')
    return
  }

  isSubmitting.value = true
  try {
    const res = await api.post(`/expenses/${selectedExpense.value.id}/cancel`, {
      reason: cancelReason.value.trim(),
    })
    if (res.data?.success) {
      dialog.toast(res.data.message || 'Transaksi pengeluaran berhasil dibatalkan.', 'success')
      showCancelModal.value = false
      selectedExpense.value = null
      fetchExpenses()
      fetchSummary()
    }
  } catch (err: any) {
    dialog.alert(err.response?.data?.message || 'Gagal membatalkan pengeluaran.', 'danger')
  } finally {
    isSubmitting.value = false
  }
}

function printExpenseVoucher() {
  window.print()
}

// Kategori Modal
function openAddCategoryModal() {
  isEditingCategory.value = false
  categoryForm.value = {
    id: '',
    code: '',
    name: '',
    account_id: expenseAccounts.value[0]?.id || '',
    description: '',
    is_active: true,
  }
  showCategoryModal.value = true
}

function openEditCategoryModal(cat: ExpenseCategory) {
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
    dialog.alert('Nama kategori tidak boleh kosong.', 'warning')
    return
  }

  isSubmitting.value = true
  try {
    if (isEditingCategory.value) {
      const res = await api.put(`/expenses/categories/${categoryForm.value.id}`, {
        name: categoryForm.value.name.trim(),
        account_id: categoryForm.value.account_id || null,
        description: categoryForm.value.description.trim() || null,
        is_active: categoryForm.value.is_active,
      })
      if (res.data?.success) {
        dialog.toast('Kategori berhasil diperbarui.', 'success')
        showCategoryModal.value = false
        fetchCategories()
      }
    } else {
      if (!categoryForm.value.code.trim()) {
        dialog.alert('Kode kategori tidak boleh kosong.', 'warning')
        return
      }
      const res = await api.post('/expenses/categories', {
        code: categoryForm.value.code.trim().toUpperCase(),
        name: categoryForm.value.name.trim(),
        account_id: categoryForm.value.account_id || null,
        description: categoryForm.value.description.trim() || null,
      })
      if (res.data?.success) {
        dialog.toast('Kategori pengeluaran baru berhasil dibuat.', 'success')
        showCategoryModal.value = false
        fetchCategories()
      }
    }
  } catch (err: any) {
    dialog.alert(err.response?.data?.message || 'Gagal menyimpan kategori pengeluaran.', 'danger')
  } finally {
    isSubmitting.value = false
  }
}
</script>

<template>
  <AdminLayout>
    <div class="p-6 space-y-6 max-w-7xl mx-auto">
      <!-- 1. HEADER & ACTION TOOLBAR -->
      <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 bg-white p-5 rounded-2xl border border-slate-200 shadow-xs">
        <div>
          <div class="flex items-center gap-2.5">
            <div class="p-2.5 bg-amber-100 text-amber-900 rounded-xl">
              <Icon icon="streamline-ultimate-color:money-bag-dollar" class="w-7 h-7" />
            </div>
            <div>
              <h1 class="text-xl font-black text-slate-900 tracking-tight">Biaya & Transaksi Pengeluaran</h1>
              <p class="text-xs text-slate-500">Pencatatan kas keluar operasional, petty cash kasir, dan beban toko terintegrasi jurnal akuntansi.</p>
            </div>
          </div>
        </div>

        <div class="flex items-center gap-2 flex-wrap">
          <!-- Tab Switcher -->
          <div class="bg-slate-100 p-1 rounded-xl flex items-center gap-1 border border-slate-200">
            <button
              type="button"
              @click="activeTab = 'transactions'"
              class="px-3 py-1.5 rounded-lg text-xs font-bold transition cursor-pointer"
              :class="activeTab === 'transactions' ? 'bg-white text-slate-900 shadow-xs font-black' : 'text-slate-600 hover:text-slate-900'"
            >
              <span class="flex items-center gap-1.5">
                <FileText class="w-3.5 h-3.5" />
                <span>Transaksi Kas Keluar</span>
              </span>
            </button>
            <button
              type="button"
              @click="activeTab = 'categories'"
              class="px-3 py-1.5 rounded-lg text-xs font-bold transition cursor-pointer"
              :class="activeTab === 'categories' ? 'bg-white text-slate-900 shadow-xs font-black' : 'text-slate-600 hover:text-slate-900'"
            >
              <span class="flex items-center gap-1.5">
                <Tag class="w-3.5 h-3.5" />
                <span>Kategori Beban</span>
              </span>
            </button>
          </div>

          <!-- Action Buttons -->
          <button
            v-if="activeTab === 'transactions'"
            type="button"
            @click="openCreateModal"
            class="bg-amber-600 hover:bg-amber-700 text-white font-bold text-xs px-4 py-2.5 rounded-xl shadow-xs transition flex items-center gap-1.5 cursor-pointer active:scale-95"
          >
            <Plus class="w-4 h-4" />
            <span>+ Catat Pengeluaran</span>
          </button>

          <button
            v-if="activeTab === 'categories'"
            type="button"
            @click="openAddCategoryModal"
            class="bg-indigo-600 hover:bg-indigo-700 text-white font-bold text-xs px-4 py-2.5 rounded-xl shadow-xs transition flex items-center gap-1.5 cursor-pointer active:scale-95"
          >
            <Plus class="w-4 h-4" />
            <span>+ Tambah Kategori</span>
          </button>
        </div>
      </div>

      <!-- 2. KPI SUMMARY METRIC CARDS (HANYA TAB TRANSAKSI) -->
      <div v-if="activeTab === 'transactions'" class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <!-- Pengeluaran Bulan Ini -->
        <div class="bg-gradient-to-br from-rose-50 to-white p-4 rounded-2xl border border-rose-200/80 shadow-xs flex items-center gap-3.5">
          <div class="w-12 h-12 rounded-xl bg-rose-500/10 border border-rose-200 flex items-center justify-center text-rose-600 shrink-0">
            <DollarSign class="w-6 h-6" />
          </div>
          <div>
            <div class="text-[11px] font-bold text-slate-500 uppercase tracking-wider">Pengeluaran Bulan Ini</div>
            <div class="text-lg font-black text-rose-900 leading-tight">
              {{ formatCurrency(summary?.this_month_amount || 0) }}
            </div>
            <div class="text-[10px] text-slate-400 mt-0.5">Akumulasi bulan berjalan</div>
          </div>
        </div>

        <!-- Pengeluaran Hari Ini -->
        <div class="bg-gradient-to-br from-amber-50 to-white p-4 rounded-2xl border border-amber-200/80 shadow-xs flex items-center gap-3.5">
          <div class="w-12 h-12 rounded-xl bg-amber-500/10 border border-amber-200 flex items-center justify-center text-amber-600 shrink-0">
            <Clock class="w-6 h-6" />
          </div>
          <div>
            <div class="text-[11px] font-bold text-slate-500 uppercase tracking-wider">Kas Keluar Hari Ini</div>
            <div class="text-lg font-black text-amber-900 leading-tight">
              {{ formatCurrency(summary?.today_amount || 0) }}
            </div>
            <div class="text-[10px] text-slate-400 mt-0.5">Transaksi tanggal hari ini</div>
          </div>
        </div>

        <!-- Total Periode Terpilih -->
        <div class="bg-gradient-to-br from-blue-50 to-white p-4 rounded-2xl border border-blue-200/80 shadow-xs flex items-center gap-3.5">
          <div class="w-12 h-12 rounded-xl bg-blue-500/10 border border-blue-200 flex items-center justify-center text-blue-600 shrink-0">
            <Calendar class="w-6 h-6" />
          </div>
          <div>
            <div class="text-[11px] font-bold text-slate-500 uppercase tracking-wider">Total Filter Periode</div>
            <div class="text-lg font-black text-blue-900 leading-tight">
              {{ formatCurrency(summary?.total_amount || 0) }}
            </div>
            <div class="text-[10px] text-slate-500 mt-0.5 font-medium">{{ summary?.total_count || 0 }} nota transaksi</div>
          </div>
        </div>

        <!-- Status Shift Kasir Aktif -->
        <div class="bg-gradient-to-br from-emerald-50 to-white p-4 rounded-2xl border border-emerald-200/80 shadow-xs flex items-center gap-3.5">
          <div class="w-12 h-12 rounded-xl bg-emerald-500/10 border border-emerald-200 flex items-center justify-center text-emerald-600 shrink-0">
            <Icon icon="streamline-ultimate-color:cash-payment-bill" class="w-6 h-6 drop-shadow-xs" />
          </div>
          <div>
            <div class="text-[11px] font-bold text-slate-500 uppercase tracking-wider">Laci Shift Kasir</div>
            <div class="text-xs font-black" :class="activeShiftInfo ? 'text-emerald-700' : 'text-slate-500'">
              {{ activeShiftInfo ? 'SHIFT AKTIF TERBUKA' : 'TIDAK ADA SHIFT TERBUKA' }}
            </div>
            <div class="text-[10px] text-slate-400 mt-0.5 truncate max-w-[160px]">
              {{ activeShiftInfo ? `Kasir: ${activeShiftInfo.cashier_name || 'Aktif'}` : 'Petty cash non-shift' }}
            </div>
          </div>
        </div>
      </div>

      <!-- ==================================================================== -->
      <!-- TAB 1: TRANSAKSI PENGELUARAN                                         -->
      <!-- ==================================================================== -->
      <div v-if="activeTab === 'transactions'" class="space-y-4">
        <!-- FILTER BAR (SEARCHABLE & DIPANGGIL DARI API) -->
        <div class="bg-white p-4 rounded-2xl border border-slate-200 shadow-xs flex flex-col md:flex-row gap-3 items-stretch md:items-center justify-between">
          <div class="flex items-center gap-2 flex-wrap flex-1">
            <!-- Search Text -->
            <div class="relative min-w-[200px] flex-1 max-w-xs">
              <Search class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-slate-400" />
              <input
                v-model="searchQuery"
                @keyup.enter="handleFilterChange"
                type="text"
                placeholder="Cari no. nota, penerima, catatan..."
                class="w-full pl-9 pr-3 py-2 bg-slate-50 border border-slate-300 rounded-xl text-xs focus:ring-2 focus:ring-amber-500 focus:bg-white transition"
              />
            </div>

            <!-- Filter Lokasi Outlet (Searchable dari API) -->
            <div class="w-52">
              <SearchableSelect
                v-model="selectedOutletId"
                api-endpoint="/outlets"
                :extra-params="{ limit: 100 }"
                value-key="id"
                label-key="name"
                secondary-key="code"
                placeholder="Semua Lokasi Gerai"
                search-placeholder="Cari lokasi gerai..."
                :allow-clear="true"
                @change="handleOutletFilterChange"
              />
            </div>

            <!-- Filter Kategori (Searchable dari API) -->
            <div class="w-52">
              <SearchableSelect
                v-model="selectedCategoryId"
                api-endpoint="/expenses/categories"
                value-key="id"
                label-key="name"
                secondary-key="code"
                placeholder="Semua Kategori"
                search-placeholder="Cari kategori beban..."
                :allow-clear="true"
                @change="handleFilterChange"
              />
            </div>

            <!-- Filter Status (Searchable) -->
            <div class="w-40">
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
          </div>

          <!-- Date Preset Buttons -->
          <div class="flex items-center gap-1.5 self-end md:self-auto">
            <button
              type="button"
              @click="applyDatePreset('today')"
              class="px-2.5 py-1.5 rounded-lg text-xs font-bold transition cursor-pointer"
              :class="datePreset === 'today' ? 'bg-amber-600 text-white shadow-2xs' : 'bg-slate-100 text-slate-600 hover:bg-slate-200'"
            >
              Hari Ini
            </button>
            <button
              type="button"
              @click="applyDatePreset('7days')"
              class="px-2.5 py-1.5 rounded-lg text-xs font-bold transition cursor-pointer"
              :class="datePreset === '7days' ? 'bg-amber-600 text-white shadow-2xs' : 'bg-slate-100 text-slate-600 hover:bg-slate-200'"
            >
              7 Hari
            </button>
            <button
              type="button"
              @click="applyDatePreset('month')"
              class="px-2.5 py-1.5 rounded-lg text-xs font-bold transition cursor-pointer"
              :class="datePreset === 'month' ? 'bg-amber-600 text-white shadow-2xs' : 'bg-slate-100 text-slate-600 hover:bg-slate-200'"
            >
              Bulan Ini
            </button>
            <button
              type="button"
              @click="applyDatePreset('all')"
              class="px-2.5 py-1.5 rounded-lg text-xs font-bold transition cursor-pointer"
              :class="datePreset === 'all' ? 'bg-amber-600 text-white shadow-2xs' : 'bg-slate-100 text-slate-600 hover:bg-slate-200'"
            >
              Semua
            </button>
            <button
              type="button"
              @click="fetchExpenses(); fetchSummary()"
              class="p-2 text-slate-600 hover:text-amber-800 hover:bg-amber-50 rounded-lg transition border border-slate-200 cursor-pointer"
              title="Segarkan Data"
            >
              <RefreshCw class="w-3.5 h-3.5" :class="{ 'animate-spin': isLoading }" />
            </button>

            <!-- Tombol Ekspor Excel Pengeluaran -->
            <button
              v-if="authStore.hasPermission(['expenses:export', 'expenses:read', 'expenses:view'])"
              type="button"
              @click="exportExpensesToExcel"
              class="px-3.5 py-1.5 rounded-lg bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-xs flex items-center gap-1.5 transition active:scale-95 cursor-pointer shadow-xs border border-emerald-700"
              title="Ekspor seluruh data pengeluaran terfilter ke Excel di latar belakang"
            >
              <FileSpreadsheet class="w-4 h-4" />
              <span>Ekspor Excel</span>
            </button>
          </div>
        </div>

        <!-- TABLE TRANSAKSI PENGELUARAN -->
        <div class="bg-white rounded-2xl border border-slate-200 shadow-xs overflow-hidden">
          <div class="overflow-x-auto">
            <table class="w-full text-left text-xs">
              <thead class="bg-slate-50/90 text-slate-600 font-extrabold uppercase tracking-wider border-b border-slate-200">
                <tr>
                  <th class="py-3 px-4">No. Transaksi</th>
                  <th class="py-3 px-4">Tanggal</th>
                  <th class="py-3 px-4">Gerai / Lokasi</th>
                  <th class="py-3 px-4">Kategori & Beban</th>
                  <th class="py-3 px-4">Keterangan / Penerima</th>
                  <th class="py-3 px-4">Sumber Pembayaran</th>
                  <th class="py-3 px-4 text-right">Nominal (Rp)</th>
                  <th class="py-3 px-4 text-center">Status</th>
                  <th class="py-3 px-4 text-center">Aksi</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-slate-100">
                <tr v-if="isLoading" class="text-center py-10">
                  <td colspan="9" class="py-10 text-slate-400">
                    <div class="flex flex-col items-center gap-2">
                      <RefreshCw class="w-5 h-5 animate-spin text-amber-500" />
                      <span>Memuat data pengeluaran...</span>
                    </div>
                  </td>
                </tr>

                <tr v-else-if="expenses.length === 0" class="text-center py-10">
                  <td colspan="9" class="py-12 text-slate-400">
                    <div class="flex flex-col items-center gap-2">
                      <Icon icon="fluent:document-error-24-regular" class="w-8 h-8 text-slate-300" />
                      <span class="font-bold text-slate-500">Belum ada transaksi pengeluaran pada periode ini</span>
                      <span class="text-[11px] text-slate-400">Klik tombol "+ Catat Pengeluaran" di atas untuk mencatat kas keluar.</span>
                    </div>
                  </td>
                </tr>

                <tr
                  v-for="item in expenses"
                  :key="item.id"
                  class="hover:bg-slate-50/80 transition"
                  :class="{ 'opacity-60 bg-rose-50/20': item.status === 'CANCELLED' }"
                >
                  <!-- No. Transaksi & Shift Tag -->
                  <td class="py-3 px-4">
                    <div class="flex items-center gap-1.5">
                      <CopyableText :value="item.transaction_no" label="No. Transaksi" />
                      <span
                        v-if="item.shift_id"
                        class="text-[9px] bg-emerald-100 text-emerald-800 font-bold px-1.5 py-0.5 rounded border border-emerald-200"
                        title="Dikeluarkan dari kas laci kasir saat shift aktif"
                      >
                        Laci Kasir
                      </span>
                    </div>
                    <div v-if="item.reference_no" class="text-[10px] text-slate-400 font-normal mt-0.5">
                      Ref: <CopyableText :value="item.reference_no" label="No. Referensi" :bold="false" />
                    </div>
                  </td>

                  <!-- Tanggal -->
                  <td class="py-3 px-4 text-slate-600 whitespace-nowrap">
                    {{ formatDate(item.transaction_date) }}
                  </td>

                  <!-- Gerai -->
                  <td class="py-3 px-4 text-slate-700 font-semibold">
                    {{ item.outlet_name || 'Gerai Pusat' }}
                  </td>

                  <!-- Kategori & Beban -->
                  <td class="py-3 px-4">
                    <div class="font-bold text-slate-800">
                      {{ item.category_name || 'Operasional' }}
                    </div>
                    <div class="text-[10px] text-slate-400 font-mono">
                      {{ item.expense_account_code }} - {{ item.expense_account_name }}
                    </div>
                  </td>

                  <!-- Keterangan & Penerima -->
                  <td class="py-3 px-4 max-w-xs">
                    <div class="font-medium text-slate-800 truncate" :title="item.notes || '-'">
                      {{ item.notes || '-' }}
                    </div>
                    <div v-if="item.recipient_name" class="text-[10px] text-amber-800 font-semibold truncate">
                      Penerima: {{ item.recipient_name }}
                    </div>
                  </td>

                  <!-- Sumber Pembayaran -->
                  <td class="py-3 px-4">
                    <div class="text-slate-700 font-medium flex items-center gap-1">
                      <Icon icon="fluent:wallet-credit-card-16-regular" class="w-3.5 h-3.5 text-slate-400" />
                      <span>{{ item.payment_account_name }}</span>
                    </div>
                    <div class="text-[10px] text-slate-400 font-mono">{{ item.payment_account_code }}</div>
                  </td>

                  <!-- Nominal -->
                  <td class="py-3 px-4 text-right font-black" :class="item.status === 'CANCELLED' ? 'text-slate-400 line-through' : 'text-rose-600'">
                    {{ formatCurrency(item.amount) }}
                  </td>

                  <!-- Status -->
                  <td class="py-3 px-4 text-center">
                    <span
                      v-if="item.status === 'COMPLETED'"
                      class="inline-flex items-center gap-1 text-[10px] font-bold px-2 py-0.5 rounded-full bg-emerald-100 text-emerald-800 border border-emerald-200"
                    >
                      <CheckCircle2 class="w-2.5 h-2.5" />
                      <span>Selesai</span>
                    </span>
                    <span
                      v-else
                      class="inline-flex items-center gap-1 text-[10px] font-bold px-2 py-0.5 rounded-full bg-rose-100 text-rose-800 border border-rose-200"
                      :title="item.cancellation_reason || 'Dibatalkan'"
                    >
                      <X class="w-2.5 h-2.5" />
                      <span>Dibatalkan</span>
                    </span>
                  </td>

                  <!-- Aksi -->
                  <td class="py-3 px-4 text-center whitespace-nowrap">
                    <div class="flex items-center justify-center gap-1">
                      <button
                        type="button"
                        @click="openDetailModal(item)"
                        class="p-1.5 text-slate-600 hover:text-blue-600 hover:bg-blue-50 rounded-lg transition cursor-pointer"
                        title="Lihat Rincian & Cetak Bukti"
                      >
                        <Eye class="w-3.5 h-3.5" />
                      </button>
                      <button
                        v-if="item.status === 'COMPLETED' && authStore.hasPermission('expenses:delete')"
                        type="button"
                        @click="openCancelModal(item)"
                        class="p-1.5 text-slate-400 hover:text-rose-600 hover:bg-rose-50 rounded-lg transition cursor-pointer"
                        title="Batalkan Pengeluaran Ini"
                      >
                        <RotateCcw class="w-3.5 h-3.5" />
                      </button>
                    </div>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <!-- Pagination Bar -->
          <div class="p-3 border-t border-slate-200 bg-slate-50/50">
            <PaginationBar
              :current-page="currentPage"
              :total-pages="totalPages"
              :total-items="totalItems"
              :page-size="pageSize"
              @page-change="(p: number) => { currentPage = p; fetchExpenses(); }"
            />
          </div>
        </div>
      </div>

      <!-- ==================================================================== -->
      <!-- TAB 2: MASTER KATEGORI PENGELUARAN                                   -->
      <!-- ==================================================================== -->
      <div v-if="activeTab === 'categories'" class="bg-white rounded-2xl border border-slate-200 shadow-xs overflow-hidden">
        <div class="p-4 border-b border-slate-200 bg-slate-50/60 flex items-center justify-between">
          <div>
            <h2 class="text-sm font-black text-slate-900">Daftar Kategori Beban Pengeluaran</h2>
            <p class="text-[11px] text-slate-500">Kategori mempermudah kasir & staf mengelompokkan pengeluaran dan menghubungkannya ke akun akuntansi yang sesuai.</p>
          </div>
          <button
            type="button"
            @click="openAddCategoryModal"
            class="bg-indigo-600 hover:bg-indigo-700 text-white font-bold text-xs px-3.5 py-2 rounded-xl transition flex items-center gap-1.5 cursor-pointer shadow-xs"
          >
            <Plus class="w-3.5 h-3.5" />
            <span>Tambah Kategori Baru</span>
          </button>
        </div>

        <div class="overflow-x-auto">
          <table class="w-full text-left text-xs">
            <thead class="bg-slate-50 text-slate-600 font-extrabold uppercase tracking-wider border-b border-slate-200">
              <tr>
                <th class="py-3 px-4">Kode</th>
                <th class="py-3 px-4">Nama Kategori</th>
                <th class="py-3 px-4">Akun COA Beban Terhubung</th>
                <th class="py-3 px-4">Deskripsi / Contoh</th>
                <th class="py-3 px-4 text-center">Status</th>
                <th class="py-3 px-4 text-center">Aksi</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-100">
              <tr v-for="cat in categories" :key="cat.id" class="hover:bg-slate-50 transition">
                <td class="py-3 px-4 font-mono font-bold text-indigo-900">{{ cat.code }}</td>
                <td class="py-3 px-4 font-bold text-slate-800">{{ cat.name }}</td>
                <td class="py-3 px-4">
                  <div v-if="cat.account_name" class="flex items-center gap-1 font-medium text-slate-700">
                    <span class="font-mono text-slate-400 font-bold">{{ cat.account_code }}</span>
                    <span>- {{ cat.account_name }}</span>
                  </div>
                  <span v-else class="text-slate-400 italic">Belum dihubungkan</span>
                </td>
                <td class="py-3 px-4 text-slate-500 max-w-xs truncate">{{ cat.description || '-' }}</td>
                <td class="py-3 px-4 text-center">
                  <span
                    class="text-[10px] font-bold px-2 py-0.5 rounded-full"
                    :class="cat.is_active ? 'bg-emerald-100 text-emerald-800' : 'bg-slate-100 text-slate-500'"
                  >
                    {{ cat.is_active ? 'Aktif' : 'Nonaktif' }}
                  </span>
                </td>
                <td class="py-3 px-4 text-center">
                  <button
                    type="button"
                    @click="openEditCategoryModal(cat)"
                    class="px-2.5 py-1 text-slate-600 hover:text-indigo-600 hover:bg-indigo-50 rounded-lg transition font-bold text-[11px] cursor-pointer"
                  >
                    Edit
                  </button>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <!-- ==================================================================== -->
      <!-- MODAL: CATAT PENGELUARAN BARU                                        -->
      <!-- ==================================================================== -->
      <!-- ==================================================================== -->
      <!-- MODAL: CATAT PENGELUARAN BARU (RAMAH PENGGUNA AWAM RETAIL)            -->
      <!-- ==================================================================== -->
      <div
        v-if="showCreateModal"
        class="fixed inset-0 z-50 flex items-center justify-center p-3 sm:p-4 bg-slate-900/60 backdrop-blur-xs animate-fadeIn"
      >
        <div class="bg-white rounded-2xl shadow-2xl border border-slate-200 w-full max-w-2xl overflow-hidden flex flex-col max-h-[92vh]">
          <!-- Modal Header -->
          <div class="px-6 py-4 bg-slate-900 text-white flex items-center justify-between shrink-0">
            <div class="flex items-center gap-3">
              <div class="p-2.5 bg-slate-800 rounded-xl border border-slate-700">
                <Icon icon="streamline:money-cash-bag-dollar-bag-payment-cash-money-finance" class="w-5 h-5 text-amber-400" />
              </div>
              <div>
                <h3 class="text-base font-bold tracking-tight">Catat Pengeluaran Operasional</h3>
                <p class="text-xs text-slate-400">Catat kas keluar dan beban operasional toko.</p>
              </div>
            </div>
            <button
              type="button"
              @click="showCreateModal = false"
              class="text-white/80 hover:text-white p-1.5 rounded-xl hover:bg-white/20 transition cursor-pointer"
            >
              <X class="w-5 h-5" />
            </button>
          </div>

          <!-- Modal Body -->
          <form @submit.prevent="handleSaveExpense" class="p-5 sm:p-6 overflow-y-auto space-y-4 text-xs">
            <!-- Lokasi & Tanggal Pengeluaran -->
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-3.5 bg-slate-50 p-3.5 rounded-2xl border border-slate-200">
              <div>
                <label class="block font-bold text-slate-700 mb-1 flex items-center gap-1.5">
                  <Building2 class="w-3.5 h-3.5 text-slate-500" />
                  <span>Lokasi Toko / Gerai *</span>
                </label>
                <SearchableSelect
                  v-model="newExpense.outlet_id"
                  api-endpoint="/outlets"
                  :extra-params="{ limit: 100 }"
                  value-key="id"
                  label-key="name"
                  secondary-key="code"
                  placeholder="Pilih Lokasi Gerai *"
                  search-placeholder="Cari lokasi gerai..."
                  :allow-clear="false"
                  :required="true"
                />
              </div>

              <div>
                <label class="block font-bold text-slate-700 mb-1 flex items-center gap-1.5">
                  <Calendar class="w-3.5 h-3.5 text-slate-500" />
                  <span>Tanggal Pengeluaran *</span>
                </label>
                <input
                  v-model="newExpense.transaction_date"
                  type="date"
                  class="w-full px-3 py-2 bg-white border border-slate-300 rounded-xl focus:ring-2 focus:ring-amber-500 font-bold text-slate-800 text-xs shadow-2xs"
                  required
                />
              </div>
            </div>

            <!-- Kategori Pengeluaran (Visual Grid Cards) -->
            <div>
              <div class="flex items-center justify-between mb-2">
                <label class="font-bold text-slate-800 flex items-center gap-1.5">
                  <Sparkles class="w-3.5 h-3.5 text-amber-600" />
                  <span>Kategori Pengeluaran Toko *</span>
                </label>
                <span class="text-[11px] text-slate-500 font-medium">Pilih jenis keperluan belanja/biaya</span>
              </div>

              <div class="grid grid-cols-2 sm:grid-cols-3 gap-2">
                <button
                  type="button"
                  v-for="c in categories"
                  :key="c.id"
                  @click="handleCategoryChange(c.id)"
                  :class="[
                    'p-2.5 rounded-xl border text-left transition-all duration-150 relative flex flex-col justify-between cursor-pointer',
                    newExpense.category_id === c.id
                      ? 'border-amber-500 bg-amber-50/80 ring-2 ring-amber-400/40 shadow-xs'
                      : 'border-slate-200 bg-white hover:border-slate-300 hover:bg-slate-50'
                  ]"
                >
                  <div class="flex items-start justify-between gap-1 w-full">
                    <span class="text-lg leading-none">{{ getCategoryMeta(c.code).icon }}</span>
                    <span
                      v-if="newExpense.category_id === c.id"
                      class="w-4 h-4 rounded-full bg-amber-600 text-white flex items-center justify-center shrink-0"
                    >
                      <Check class="w-2.5 h-2.5 stroke-[3]" />
                    </span>
                  </div>
                  <div class="mt-1.5">
                    <div class="font-bold text-[11px] text-slate-800 leading-tight">{{ c.name }}</div>
                    <div class="text-[9.5px] text-slate-500 font-medium mt-0.5 line-clamp-1">
                      {{ getCategoryMeta(c.code).badge }}
                    </div>
                  </div>
                </button>
              </div>
            </div>

            <!-- Nominal Pengeluaran & Tombol Cepat -->
            <div class="bg-amber-50/60 border border-amber-200/80 rounded-2xl p-4 space-y-2.5">
              <div class="flex items-center justify-between">
                <label class="font-bold text-amber-950 text-xs flex items-center gap-1.5">
                  <Wallet class="w-3.5 h-3.5 text-amber-700" />
                  <span>Nominal Pengeluaran *</span>
                </label>
                <div v-if="newExpense.amount > 0" class="text-xs font-black text-rose-600">
                  {{ formatCurrency(newExpense.amount) }}
                </div>
              </div>

              <div class="relative">
                <span class="absolute left-3.5 top-1/2 -translate-y-1/2 font-black text-slate-400 text-sm">Rp</span>
                <input
                  v-model.number="newExpense.amount"
                  type="number"
                  min="1"
                  step="1"
                  placeholder="0"
                  class="w-full pl-11 pr-4 py-2.5 bg-white border border-amber-300 rounded-xl text-base font-black text-slate-900 focus:ring-2 focus:ring-amber-500 shadow-2xs"
                  required
                />
              </div>

              <!-- Quick Nominal Buttons -->
              <div>
                <div class="flex items-center justify-between mb-1.5">
                  <span class="text-[10.5px] font-bold text-slate-500">Nominal Cepat:</span>
                  <button
                    v-if="newExpense.amount > 0"
                    type="button"
                    @click="resetAmount"
                    class="text-[10px] text-rose-600 hover:text-rose-700 font-bold hover:underline cursor-pointer"
                  >
                    Reset ke 0
                  </button>
                </div>
                <div class="flex flex-wrap gap-1.5">
                  <button
                    v-for="amt in quickAmounts"
                    :key="amt"
                    type="button"
                    @click="addQuickAmount(amt)"
                    class="px-2.5 py-1 bg-white hover:bg-amber-100 hover:text-amber-900 border border-amber-200/80 rounded-lg text-[11px] font-bold text-slate-700 transition cursor-pointer shadow-2xs active:scale-95"
                  >
                    +{{ (amt / 1000).toLocaleString('id-ID') }}rb
                  </button>
                </div>
              </div>
            </div>

            <!-- Sumber Pembayaran (Kas Toko vs Rekening Bank) -->
            <div>
              <label class="block font-bold text-slate-800 mb-1 flex items-center gap-1.5">
                <Wallet class="w-3.5 h-3.5 text-slate-500" />
                <span>Dibayar Menggunakan (Sumber Uang) *</span>
              </label>
              <SearchableSelect
                v-model="newExpense.payment_account_id"
                :options="paymentAccountOptions"
                value-key="id"
                label-key="name"
                secondary-key="code"
                placeholder="-- Pilih Kas Tunai atau Rekening Bank --"
                search-placeholder="Ketik cari kas / bank..."
                :allow-clear="false"
                :required="true"
              />
              <p class="text-[10px] text-slate-500 mt-1">
                Pilih apakah dibayarkan dengan uang tunai laci/brankas toko atau transfer rekening bank.
              </p>
            </div>

            <!-- Opsi Kaitan Shift Kasir -->
            <div v-if="activeShiftInfo" class="bg-emerald-50 border border-emerald-200 rounded-xl p-3 flex items-start gap-2.5">
              <input
                id="linkShift"
                v-model="linkActiveShift"
                type="checkbox"
                class="w-4 h-4 text-emerald-600 rounded mt-0.5 cursor-pointer"
              />
              <label for="linkShift" class="cursor-pointer text-[11px] text-emerald-950">
                <span class="font-bold">Potong Langsung dari Kas Laci Kasir (Shift Aktif)</span>
                <p class="text-emerald-700 text-[10px] mt-0.5">
                  Uang tunai diambil langsung dari laci kasir shift ini. Pengeluaran otomatis mengurangi saldo fisik laci saat kasir tutup shift.
                </p>
              </label>
            </div>

            <!-- Keperluan / Keterangan -->
            <div>
              <label class="block font-bold text-slate-800 mb-1">Keperluan / Keterangan Pengeluaran *</label>
              <textarea
                v-model="newExpense.notes"
                rows="2"
                placeholder="Contoh: Beli air mineral galon & tisu kasir, isi token listrik, dsb..."
                class="w-full px-3 py-2 bg-slate-50 border border-slate-300 rounded-xl focus:ring-2 focus:ring-amber-500 text-slate-800 text-xs"
                required
              ></textarea>
            </div>

            <!-- Penerima & No. Bukti Fisik (Opsional) -->
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-3.5">
              <div>
                <label class="block font-bold text-slate-700 mb-1">Penerima Uang (Opsional)</label>
                <input
                  v-model="newExpense.recipient_name"
                  type="text"
                  placeholder="Contoh: Toko Sebelah / Tukang AC"
                  class="w-full px-3 py-2 bg-slate-50 border border-slate-300 rounded-xl focus:ring-2 focus:ring-amber-500 text-xs"
                />
              </div>

              <div>
                <label class="block font-bold text-slate-700 mb-1">No. Bukti / Struk Fisik (Opsional)</label>
                <input
                  v-model="newExpense.reference_no"
                  type="text"
                  placeholder="Contoh: NOTA-128 / KWIT-01"
                  class="w-full px-3 py-2 bg-slate-50 border border-slate-300 rounded-xl focus:ring-2 focus:ring-amber-500 text-xs"
                />
              </div>
            </div>

            <!-- OPSI AKUNTANSI LANJUTAN (COLLAPSIBLE / ACCORDION) -->
            <div class="pt-1">
              <button
                type="button"
                @click="showAdvancedAccountingOption = !showAdvancedAccountingOption"
                class="w-full flex items-center justify-between px-3.5 py-2.5 rounded-xl border border-dashed border-slate-300 bg-slate-50/70 hover:bg-slate-100 text-slate-600 transition cursor-pointer text-left"
              >
                <div class="flex items-center gap-2">
                  <Sliders class="w-3.5 h-3.5 text-slate-500" />
                  <span class="text-[11px] font-bold text-slate-700">Opsi Akuntansi Lanjutan (Pilih Akun Beban Manual)</span>
                </div>
                <div class="flex items-center gap-1.5 text-[10px] text-slate-500 font-medium">
                  <span>{{ showAdvancedAccountingOption ? 'Sembunyikan' : 'Buka Pengaturan' }}</span>
                  <ChevronDown
                    :class="['w-3.5 h-3.5 transition-transform duration-200', showAdvancedAccountingOption ? 'rotate-180' : '']"
                  />
                </div>
              </button>

              <div
                v-if="showAdvancedAccountingOption"
                class="mt-2.5 p-3.5 bg-slate-50 border border-slate-200 rounded-xl space-y-2 animate-fadeIn"
              >
                <div class="flex items-start gap-2 text-slate-500 text-[10px]">
                  <Info class="w-3.5 h-3.5 text-amber-600 shrink-0 mt-0.5" />
                  <span>
                    Secara default, akun beban sudah dipetakan otomatis sesuai kategori di atas. Gunakan pilihan di bawah jika ingin mengalihkan ke kode akun COA khusus.
                  </span>
                </div>
                <div>
                  <label class="block font-bold text-slate-700 mb-1">Akun Beban / Ekuitas (Debet) *</label>
                  <SearchableSelect
                    v-model="newExpense.expense_account_id"
                    :options="expenseAccountOptions"
                    value-key="id"
                    label-key="name"
                    secondary-key="code"
                    placeholder="-- Pilih Akun COA --"
                    search-placeholder="Ketik cari kode atau nama akun..."
                    :allow-clear="false"
                    :required="true"
                  />
                </div>
              </div>
            </div>

            <!-- Modal Footer -->
            <div class="pt-3 border-t border-slate-200 flex items-center justify-end gap-2.5">
              <button
                type="button"
                @click="showCreateModal = false"
                class="px-4 py-2 text-slate-600 hover:bg-slate-100 rounded-xl font-bold transition cursor-pointer"
              >
                Batal
              </button>
              <button
                type="submit"
                :disabled="isSubmitting"
                class="bg-gradient-to-r from-amber-600 to-amber-700 hover:from-amber-700 hover:to-amber-800 text-white font-bold px-5 py-2.5 rounded-xl transition flex items-center gap-2 cursor-pointer shadow-sm hover:shadow disabled:opacity-50"
              >
                <RefreshCw v-if="isSubmitting" class="w-3.5 h-3.5 animate-spin" />
                <span>{{ isSubmitting ? 'Menyimpan...' : 'Simpan Transaksi Pengeluaran' }}</span>
              </button>
            </div>
          </form>
        </div>
      </div>

      <!-- ==================================================================== -->
      <!-- MODAL: DETAIL TRANSAKSI & CETAK BUKTI KAS KELUAR                     -->
      <!-- ==================================================================== -->
      <div
        v-if="showDetailModal && selectedExpense"
        class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/60 backdrop-blur-xs animate-fadeIn"
      >
        <div class="bg-white rounded-3xl shadow-2xl border border-slate-200 w-full max-w-lg overflow-hidden flex flex-col">
          <!-- Header -->
          <div class="px-6 py-4 bg-slate-800 text-white flex items-center justify-between print:hidden">
            <div class="flex items-center gap-2">
              <Icon icon="fluent:receipt-24-filled" class="w-5 h-5 text-amber-400" />
              <span class="font-black text-sm">Bukti Kas Keluar (Voucher Pengeluaran)</span>
            </div>
            <div class="flex items-center gap-2">
              <button
                type="button"
                @click="printExpenseVoucher"
                class="bg-white/20 hover:bg-white/30 text-white px-2.5 py-1 rounded-lg text-xs font-bold transition flex items-center gap-1 cursor-pointer"
              >
                <Printer class="w-3.5 h-3.5" />
                <span>Cetak</span>
              </button>
              <button
                type="button"
                @click="showDetailModal = false"
                class="text-slate-400 hover:text-white p-1 rounded-lg transition cursor-pointer"
              >
                <X class="w-5 h-5" />
              </button>
            </div>
          </div>

          <!-- Printable Voucher Body -->
          <div class="p-6 space-y-4 text-xs text-slate-800" id="printable-voucher">
            <!-- Header Voucher -->
            <div class="text-center border-b pb-3 border-slate-200">
              <h2 class="text-base font-black tracking-wider text-slate-900 uppercase">BUKTI PENGELUARAN KAS</h2>
              <div class="text-[11px] font-bold text-slate-600">{{ selectedExpense.outlet_name || 'NOTANE RETAIL' }}</div>
              <div class="font-mono text-xs font-extrabold text-amber-800 mt-1">#{{ selectedExpense.transaction_no }}</div>
            </div>

            <!-- Detail Grid -->
            <div class="grid grid-cols-2 gap-2 text-[11px]">
              <div>
                <span class="text-slate-400">Tanggal:</span>
                <div class="font-bold">{{ formatDate(selectedExpense.transaction_date) }}</div>
              </div>
              <div>
                <span class="text-slate-400">Status:</span>
                <div class="font-bold" :class="selectedExpense.status === 'COMPLETED' ? 'text-emerald-700' : 'text-rose-700'">
                  {{ selectedExpense.status === 'COMPLETED' ? 'LUNAS (SELESAI)' : 'DIBATALKAN' }}
                </div>
              </div>
              <div>
                <span class="text-slate-400">Kategori Beban:</span>
                <div class="font-bold">{{ selectedExpense.category_name || '-' }}</div>
                <div class="text-[10px] text-slate-500 font-mono">{{ selectedExpense.expense_account_code }} - {{ selectedExpense.expense_account_name }}</div>
              </div>
              <div>
                <span class="text-slate-400">Sumber Pembayaran:</span>
                <div class="font-bold">{{ selectedExpense.payment_account_name }}</div>
                <div class="text-[10px] text-slate-500 font-mono">{{ selectedExpense.payment_account_code }}</div>
              </div>
              <div v-if="selectedExpense.recipient_name" class="col-span-2">
                <span class="text-slate-400">Dibayarkan Kepada:</span>
                <div class="font-bold text-slate-900">{{ selectedExpense.recipient_name }}</div>
              </div>
              <div v-if="selectedExpense.reference_no" class="col-span-2">
                <span class="text-slate-400">No. Bukti / Struk Fisik:</span>
                <div class="font-mono font-bold">{{ selectedExpense.reference_no }}</div>
              </div>
              <div class="col-span-2">
                <span class="text-slate-400">Keterangan / Keperluan:</span>
                <div class="font-medium bg-slate-50 p-2 rounded-lg border border-slate-200 mt-0.5 text-slate-700">
                  {{ selectedExpense.notes || '-' }}
                </div>
              </div>
            </div>

            <!-- Total Amount -->
            <div class="bg-rose-50 border border-rose-200 p-3 rounded-xl flex items-center justify-between">
              <span class="font-bold text-rose-900">TOTAL PENGELUARAN</span>
              <span class="text-lg font-black text-rose-700">{{ formatCurrency(selectedExpense.amount) }}</span>
            </div>

            <!-- Tanda Tangan -->
            <div class="pt-6 grid grid-cols-2 text-center text-[10px] text-slate-600 gap-4">
              <div>
                <div>Dibuat / Dikeluarkan oleh,</div>
                <div class="h-12"></div>
                <div class="font-bold border-t border-slate-300 pt-1">{{ selectedExpense.created_by_name || 'Petugas Kasir' }}</div>
              </div>
              <div>
                <div>Penerima / Vendor,</div>
                <div class="h-12"></div>
                <div class="font-bold border-t border-slate-300 pt-1">{{ selectedExpense.recipient_name || '(................................)' }}</div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- ==================================================================== -->
      <!-- MODAL: BATALKAN PENGELUARAN                                          -->
      <!-- ==================================================================== -->
      <div
        v-if="showCancelModal && selectedExpense"
        class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/60 backdrop-blur-xs animate-fadeIn"
      >
        <div class="bg-white rounded-3xl shadow-2xl border border-slate-200 w-full max-w-md overflow-hidden p-6 space-y-4">
          <div class="flex items-center gap-3 text-rose-600">
            <div class="p-2.5 bg-rose-100 rounded-2xl">
              <AlertCircle class="w-6 h-6" />
            </div>
            <div>
              <h3 class="text-base font-black text-slate-900">Batalkan Pengeluaran?</h3>
              <p class="text-xs text-slate-500">Nomor: <span class="font-mono font-bold text-slate-800">#{{ selectedExpense.transaction_no }}</span></p>
            </div>
          </div>

          <div class="text-xs text-slate-600 bg-slate-50 p-3 rounded-xl border border-slate-200 space-y-1">
            <div class="flex justify-between">
              <span>Nominal:</span>
              <strong class="text-rose-600">{{ formatCurrency(selectedExpense.amount) }}</strong>
            </div>
            <div class="flex justify-between">
              <span>Keperluan:</span>
              <span class="truncate max-w-[200px]">{{ selectedExpense.notes }}</span>
            </div>
            <p class="text-[10px] text-slate-400 pt-1 border-t border-slate-200">
              * Pembatalan akan otomatis membuat <strong>jurnal pembalik (reversal entry)</strong> untuk mengembalikan saldo kas dan membatalkan pencatatan beban.
            </p>
          </div>

          <div>
            <label class="block font-bold text-slate-700 text-xs mb-1">Alasan Pembatalan *</label>
            <textarea
              v-model="cancelReason"
              rows="3"
              placeholder="Contoh: Salah input nominal / transaksi dibatalkan oleh supplier"
              class="w-full px-3 py-2 bg-slate-50 border border-slate-300 rounded-xl text-xs focus:ring-2 focus:ring-rose-500"
              required
            ></textarea>
          </div>

          <div class="flex items-center justify-end gap-2 pt-2">
            <button
              type="button"
              @click="showCancelModal = false"
              class="px-4 py-2 text-slate-600 hover:bg-slate-100 rounded-xl text-xs font-bold transition cursor-pointer"
            >
              Kembali
            </button>
            <button
              type="button"
              @click="handleExecuteCancel"
              :disabled="isSubmitting"
              class="bg-rose-600 hover:bg-rose-700 text-white font-bold text-xs px-4 py-2 rounded-xl transition flex items-center gap-1.5 cursor-pointer shadow-xs disabled:opacity-50"
            >
              <RefreshCw v-if="isSubmitting" class="w-3.5 h-3.5 animate-spin" />
              <span>{{ isSubmitting ? 'Memproses...' : 'Ya, Batalkan Pengeluaran' }}</span>
            </button>
          </div>
        </div>
      </div>

      <!-- ==================================================================== -->
      <!-- MODAL: KATEGORI PENGELUARAN (TAMBAH / EDIT)                         -->
      <!-- ==================================================================== -->
      <div
        v-if="showCategoryModal"
        class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/60 backdrop-blur-xs animate-fadeIn"
      >
        <div class="bg-white rounded-3xl shadow-2xl border border-slate-200 w-full max-w-md overflow-hidden p-6 space-y-4">
          <div class="flex items-center justify-between border-b pb-3 border-slate-200">
            <h3 class="text-sm font-black text-slate-900">
              {{ isEditingCategory ? 'Edit Kategori Pengeluaran' : 'Tambah Kategori Pengeluaran' }}
            </h3>
            <button type="button" @click="showCategoryModal = false" class="text-slate-400 hover:text-slate-700">
              <X class="w-4 h-4" />
            </button>
          </div>

          <form @submit.prevent="handleSaveCategory" class="space-y-3 text-xs">
            <div v-if="!isEditingCategory">
              <label class="block font-bold text-slate-700 mb-1">Kode Kategori *</label>
              <input
                v-model="categoryForm.code"
                type="text"
                placeholder="Contoh: OPS-KONSUMSI"
                class="w-full px-3 py-2 bg-slate-50 border border-slate-300 rounded-xl font-mono uppercase focus:ring-2 focus:ring-indigo-500 font-bold"
                required
              />
            </div>

            <div>
              <label class="block font-bold text-slate-700 mb-1">Nama Kategori *</label>
              <input
                v-model="categoryForm.name"
                type="text"
                placeholder="Contoh: Konsumsi & Makan Siang Kasir"
                class="w-full px-3 py-2 bg-slate-50 border border-slate-300 rounded-xl focus:ring-2 focus:ring-indigo-500 font-bold"
                required
              />
            </div>

            <div>
              <label class="block font-bold text-slate-700 mb-1">Akun Beban Akuntansi (COA)</label>
              <SearchableSelect
                v-model="categoryForm.account_id"
                :options="expenseAccountOptions"
                value-key="id"
                label-key="name"
                secondary-key="code"
                placeholder="-- Pilih Akun Beban Terkait --"
                search-placeholder="Ketik cari akun beban..."
                :allow-clear="true"
              />
            </div>

            <div>
              <label class="block font-bold text-slate-700 mb-1">Deskripsi / Contoh Pemakaian</label>
              <input
                v-model="categoryForm.description"
                type="text"
                placeholder="Contoh: Galon air minum, kopi, snack staf toko"
                class="w-full px-3 py-2 bg-slate-50 border border-slate-300 rounded-xl focus:ring-2 focus:ring-indigo-500"
              />
            </div>

            <div v-if="isEditingCategory" class="flex items-center gap-2 pt-1">
              <input id="catActive" v-model="categoryForm.is_active" type="checkbox" class="w-4 h-4 text-indigo-600 rounded" />
              <label for="catActive" class="font-bold text-slate-700 cursor-pointer">Kategori Aktif</label>
            </div>

            <div class="flex items-center justify-end gap-2 pt-3 border-t border-slate-200">
              <button
                type="button"
                @click="showCategoryModal = false"
                class="px-4 py-2 text-slate-600 hover:bg-slate-100 rounded-xl font-bold transition cursor-pointer"
              >
                Batal
              </button>
              <button
                type="submit"
                :disabled="isSubmitting"
                class="bg-indigo-600 hover:bg-indigo-700 text-white font-bold px-4 py-2 rounded-xl transition flex items-center gap-1.5 cursor-pointer shadow-xs disabled:opacity-50"
              >
                <RefreshCw v-if="isSubmitting" class="w-3.5 h-3.5 animate-spin" />
                <span>{{ isSubmitting ? 'Menyimpan...' : 'Simpan Kategori' }}</span>
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>
  </AdminLayout>
</template>

<style scoped>
@media print {
  body * {
    visibility: hidden;
  }
  #printable-voucher, #printable-voucher * {
    visibility: visible;
  }
  #printable-voucher {
    position: absolute;
    left: 0;
    top: 0;
    width: 100%;
  }
}
</style>
