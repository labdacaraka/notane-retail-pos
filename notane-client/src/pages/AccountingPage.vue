<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue'
import AdminLayout from '@/layouts/AdminLayout.vue'
import PaginationBar from '@/components/common/PaginationBar.vue'
import SearchableSelect from '@/components/common/SearchableSelect.vue'
import CopyableText from '@/components/common/CopyableText.vue'
import {
  TrendingUp,
  Building2,
  Calendar,
  Lock,
  Unlock,
  Plus,
  RefreshCw,
  FileText,
  CheckCircle2,
  AlertTriangle,
  Scale,
  BookOpen,
  X,
  Trash2,
  Coins,
  Printer,
  RotateCcw,
  Search,
  PenLine,
  Landmark,
  Copy,
  Check,
  ArrowLeftRight,
  Eye,
  Users,
  PiggyBank,
  Sparkles,
  Sliders,
  Info,
  ArrowUpDown,
  ArrowUp,
  ArrowDown,
} from 'lucide-vue-next'
import { Icon } from '@iconify/vue'
import { api } from '@/services/api'
import { useAuthStore } from '@/stores/authStore'
import { useStoreBrandingStore } from '@/stores/storeBrandingStore'
import { formatCurrency, formatDate } from '@/utils/formatters'
import type {
  CapitalDepositDetail,
  CapitalSummaryResponse,
  BankAccount,
  DividendDistribution,
  DividendSummary,
  CreateDividendRequest,
} from '@/types'

const authStore = useAuthStore()
const storeBrandingStore = useStoreBrandingStore()

// State Tabs
const activeTab = ref<'income' | 'balance_sheet' | 'journal' | 'closing' | 'dividend' | 'capital' | 'bank' | 'coa'>('income')

// Global Filter State (Outlet & Date)
const outletsList = ref<any[]>([])
const selectedOutletId = ref<string>('') // empty = konsolidasi seluruh outlet
const selectedOutletName = computed(() => {
  if (!selectedOutletId.value) return 'Semua Outlet (Konsolidasian Seluruh Cabang)'
  const match = outletsList.value.find(o => o.id === selectedOutletId.value)
  return match ? `${match.name} (${match.code || 'Cabang'})` : 'Cabang Terpilih'
})

const authenticatedUserSignature = computed(() => {
  if (authStore.user?.full_name?.trim()) return authStore.user.full_name.trim()
  if (authStore.user?.username?.trim()) return authStore.user.username.trim()
  try {
    const raw = localStorage.getItem('notane_user')
    if (raw) {
      const parsed = JSON.parse(raw)
      if (parsed?.full_name?.trim()) return parsed.full_name.trim()
      if (parsed?.username?.trim()) return parsed.username.trim()
    }
  } catch {}
  return 'Administrator'
})

const authenticatedUserRole = computed(() => {
  if (authStore.user?.role_name?.trim()) return authStore.user.role_name.trim()
  try {
    const raw = localStorage.getItem('notane_user')
    if (raw) {
      const parsed = JSON.parse(raw)
      if (parsed?.role_name?.trim()) return parsed.role_name.trim()
    }
  } catch {}
  return 'Staff Keuangan'
})

// Date filters for Income Statement & Journal
const todayStr = new Date().toISOString().slice(0, 10)
const firstDayOfMonth = new Date(new Date().getFullYear(), new Date().getMonth(), 1).toISOString().slice(0, 10)
const startDate = ref<string>(firstDayOfMonth)
const endDate = ref<string>(todayStr)

// Date filter for Balance Sheet (Posisi Aset As-Of Date)
const asOfDate = ref<string>(todayStr)

// Data States
const incomeStatement = ref<any>(null)
const balanceSheet = ref<any>(null)
const journalEntries = ref<any[]>([])
const chartOfAccounts = ref<any[]>([])
const closingList = ref<any[]>([])
const isLoading = ref<boolean>(false)

const currentAssetDetails = computed(() => {
  if (!balanceSheet.value?.asset_details) return []
  return balanceSheet.value.asset_details.filter(
    (ast: any) => ast.code.startsWith('11') || ast.code.startsWith('12')
  )
})

const nonCurrentAssetDetails = computed(() => {
  if (!balanceSheet.value?.asset_details) return []
  return balanceSheet.value.asset_details.filter(
    (ast: any) => !(ast.code.startsWith('11') || ast.code.startsWith('12'))
  )
})

const printCogsDetails = computed(() => {
  if (!incomeStatement.value?.expense_details) return []
  return incomeStatement.value.expense_details.filter((item: any) => String(item.code).startsWith('51'))
})

const printOperatingExpenseDetails = computed(() => {
  if (!incomeStatement.value?.expense_details) return []
  return incomeStatement.value.expense_details.filter((item: any) => !String(item.code).startsWith('51'))
})

// COA CRUD State
const showCoaModal = ref<boolean>(false)
const coaModalMode = ref<'create' | 'edit'>('create')
const isSubmittingCoa = ref<boolean>(false)
const selectedCoaToEdit = ref<any>(null)
const coaSearch = ref<string>('')
const coaForm = ref({
  account_category_id: 1,
  code: '',
  name: '',
  description: '',
  is_header: false,
  // for edit only
  is_active: true,
})

const ACCOUNT_CATEGORIES = [
  { id: 1, name: 'ASSET', label: 'Aktiva / Aset (1xxx)', badge: 'text-emerald-700 bg-emerald-50 border-emerald-200' },
  { id: 2, name: 'LIABILITY', label: 'Kewajiban / Hutang (2xxx)', badge: 'text-amber-700 bg-amber-50 border-amber-200' },
  { id: 3, name: 'EQUITY', label: 'Modal / Ekuitas (3xxx)', badge: 'text-purple-700 bg-purple-50 border-purple-200' },
  { id: 4, name: 'REVENUE', label: 'Pendapatan (4xxx)', badge: 'text-sky-700 bg-sky-50 border-sky-200' },
  { id: 5, name: 'EXPENSE', label: 'Beban / Biaya (5xxx)', badge: 'text-rose-700 bg-rose-50 border-rose-200' },
]

const filteredCOA = computed(() => {
  if (!coaSearch.value.trim()) return chartOfAccounts.value
  const q = coaSearch.value.toLowerCase()
  return chartOfAccounts.value.filter(
    (c: any) =>
      c.code.toLowerCase().includes(q) ||
      c.name.toLowerCase().includes(q)
  )
})

const paginatedFilteredCOA = computed(() => {
  const start = (coaPage.value - 1) * coaPageSize.value
  return filteredCOA.value.slice(start, start + coaPageSize.value)
})

// Akun COA yang dapat diposting (bukan header dan aktif)
const postableCOA = computed(() => {
  return chartOfAccounts.value.filter((c: any) => !c.is_header && c.is_active)
})

// Toast State
const toastMessage = ref<string>('')
const toastType = ref<'success' | 'error'>('success')
function showToast(msg: string, type: 'success' | 'error' = 'success') {
  toastMessage.value = msg
  toastType.value = type
  setTimeout(() => {
    toastMessage.value = ''
  }, 4500)
}

// Pagination & Sorting states
const jrnPage = ref<number>(1)
const jrnPageSize = ref<number>(10)
const jrnTotalItems = ref<number>(0)
const jrnSortBy = ref<string>('journal_date')
const jrnSortDir = ref<'asc' | 'desc'>('desc')
const jrnSearch = ref<string>('')

const coaPage = ref<number>(1)
const coaPageSize = ref<number>(10)


// Capital (Setoran Modal) State
const capitalDeposits = ref<CapitalDepositDetail[]>([])
const capitalTotal = ref<number>(0)
const capitalPage = ref<number>(1)
const capitalPageSize = ref<number>(10)
const capitalSummary = ref<CapitalSummaryResponse>({
  total_amount: 0,
  total_count: 0,
  this_month_amount: 0,
  this_year_amount: 0,
})
const capitalPaymentAccounts = ref<any[]>([])
const capitalEquityAccounts = ref<any[]>([])
const capitalStatusFilter = ref<string>('ALL')
const capitalSearch = ref<string>('')
const bankAccounts = ref<BankAccount[]>([])

// Modal: Setoran Modal
const showCapitalModal = ref<boolean>(false)
const isSubmittingCapital = ref<boolean>(false)
const showAdvancedEquityOption = ref<boolean>(false)
const capitalForm = ref({
  outlet_id: '',
  deposit_date: todayStr,
  investor_name: '',
  amount: 0,
  payment_account_id: '',
  equity_account_id: '',
  reference_no: '',
  notes: '',
})

// Pilihan Rekening Bisnis Gabungan (Kas Operasional Laci Kasir + Rekening Bank Aktif Toko)
const businessPaymentAccountOptions = computed(() => {
  const options: Array<{
    id: string
    name: string
    code: string
    account_number?: string
    bank_name?: string
    account_holder?: string
    type: 'cash' | 'bank'
  }> = []

  // 1. Kas Operasional Laci Kasir (1101)
  const cashAcc = chartOfAccounts.value.find(
    (a: any) => a.code === '1101' || (!a.is_header && a.is_active && /kas.*(operasional|kasir|utama)/i.test(a.name))
  ) || capitalPaymentAccounts.value.find((a: any) => a.code === '1101')

  if (cashAcc) {
    options.push({
      id: cashAcc.id,
      name: `💵 Kas Tunai (Laci Kasir Toko)`,
      code: cashAcc.code || '1101',
      type: 'cash',
    })
  }

  // 2. Rekening Bank dari bankAccounts
  bankAccounts.value
    .filter(b => b.is_active)
    .forEach(b => {
      let targetCoaId = b.coa_id
      let targetCoaCode = b.coa_code
      if (!targetCoaId && b.coa_code) {
        const match = chartOfAccounts.value.find((a: any) => a.code === b.coa_code)
        if (match) targetCoaId = match.id
      }
      if (!targetCoaId) {
        const parentBank = chartOfAccounts.value.find((a: any) => a.code === '1102')
        if (parentBank) {
          targetCoaId = parentBank.id
          targetCoaCode = parentBank.code
        }
      }

      if (targetCoaId) {
        options.push({
          id: targetCoaId,
          name: `🏦 ${b.bank_name} - ${b.account_number} (a/n ${b.account_holder})`,
          code: targetCoaCode || '1102',
          account_number: b.account_number,
          bank_name: b.bank_name,
          account_holder: b.account_holder,
          type: 'bank',
        })
      }
    })

  // 3. Fallback jika bankAccounts kosong atau belum dimuat
  if (options.length === 0) {
    if (capitalPaymentAccounts.value.length > 0) {
      return capitalPaymentAccounts.value.map((a: any) => ({
        id: a.id,
        name: `${a.code} - ${a.name}`,
        code: a.code,
        type: a.code === '1101' ? 'cash' : 'bank',
      }))
    }
    return chartOfAccounts.value
      .filter((a: any) => !a.is_header && a.is_active && (a.code?.startsWith('11') || (a.name && /kas|bank/i.test(a.name))))
      .map((a: any) => ({
        id: a.id,
        name: `${a.code} - ${a.name}`,
        code: a.code,
        type: a.code === '1101' ? 'cash' : 'bank',
      }))
  }

  return options
})

const currentCapitalEquityName = computed(() => {
  const found = capitalEquityAccounts.value.find((a: any) => a.id === capitalForm.value.equity_account_id) ||
    chartOfAccounts.value.find((a: any) => a.id === capitalForm.value.equity_account_id)
  return found ? `${found.code} - ${found.name}` : '3101 - Modal Awal Pemilik'
})

// Modal: Batalkan Setoran Modal
const showCancelCapitalModal = ref<boolean>(false)
const selectedCapitalToCancel = ref<CapitalDepositDetail | null>(null)
const cancelCapitalReason = ref<string>('')
const isSubmittingCancelCapital = ref<boolean>(false)

// Modal: Voucher Bukti Kas Masuk / Modal
const showCapitalVoucherModal = ref<boolean>(false)
const selectedCapitalForVoucher = ref<CapitalDepositDetail | null>(null)

// Modal: Tutup Buku
const showClosingModal = ref<boolean>(false)
const isSubmittingClosing = ref<boolean>(false)
const closingForm = ref({
  outlet_id: '',
  period_start: firstDayOfMonth,
  period_end: todayStr,
  notes: '',
})

// Sub-tab untuk Tab Tutup Buku Periode: 'periods' (Soft Close) atau 'closings' (Hard Close / Histori Tutup Buku)
const closingSubTab = ref<'periods' | 'closings'>('periods')

// State Server-Side Pagination & Sorting: Accounting Periods (Soft Close)
const accountingPeriods = ref<any[]>([])
const periodPage = ref<number>(1)
const periodPageSize = ref<number>(10)
const periodTotalItems = ref<number>(0)
const periodTotalPages = ref<number>(1)
const periodSortBy = ref<string>('start_date')
const periodSortDir = ref<'asc' | 'desc'>('desc')
const periodSearch = ref<string>('')
const periodStatus = ref<string>('')
const isPeriodLoading = ref<boolean>(false)

// State Server-Side Pagination & Sorting: Accounting Closings (Hard Close)
const closingPage = ref<number>(1)
const closingPageSize = ref<number>(10)
const closingTotalItems = ref<number>(0)
const closingTotalPages = ref<number>(1)
const closingSortBy = ref<string>('period_end')
const closingSortDir = ref<'asc' | 'desc'>('desc')
const closingSearch = ref<string>('')
const closingStatus = ref<string>('')
const isClosingLoading = ref<boolean>(false)

// Modal: Buka Kembali Buku (Reopen)
const showReopenModal = ref<boolean>(false)
const selectedClosingToReopen = ref<any>(null)
const reopenReason = ref<string>('')
const isSubmittingReopen = ref<boolean>(false)

// State & Modal: Accounting Periods (Soft Close)
const showPeriodModal = ref<boolean>(false)
const isSubmittingPeriod = ref<boolean>(false)
const periodForm = ref({
  name: '',
  start_date: firstDayOfMonth,
  end_date: todayStr,
})

const showReopenPeriodModal = ref<boolean>(false)
const isReopeningPeriod = ref<boolean>(false)
const selectedPeriodToReopen = ref<any>(null)
const periodReopenReason = ref<string>('')

// Modal: Hard Close (Tutup Buku Akhir Tahun)
const showHardCloseModal = ref<boolean>(false)
const isSubmittingHardClose = ref<boolean>(false)
const hardCloseForm = ref({
  fiscal_year: new Date().getFullYear(),
  description: '',
})

// Modal: Jurnal Penyesuaian (Mode Peruntukan vs Manual)
export type AdjustmentCategory =
  | 'CASH_DIFF'
  | 'DEPRECIATION'
  | 'PREPAID_AMORTIZATION'
  | 'ACCRUED_EXPENSE'
  | 'BAD_DEBT'
  | 'BANK_RECON'
  | 'PRIOR_CORRECTION'

const showManualJournalModal = ref<boolean>(false)
const isSubmittingManualJournal = ref<boolean>(false)
const adjustmentMode = ref<'purpose' | 'manual'>('purpose')

const purposeForm = ref({
  journal_date: todayStr,
  outlet_id: '',
  category: 'CASH_DIFF' as AdjustmentCategory,
  cash_diff_type: 'SHORTAGE' as 'SHORTAGE' | 'SURPLUS',
  bank_recon_type: 'INTEREST' as 'INTEREST' | 'ADMIN',
  accrued_type: 'UTILITY' as 'UTILITY' | 'SALARY',
  payment_account_id: '',
  amount: 0,
  description: 'Penyesuaian selisih kurang fisik kas di laci kasir saat closing shift',
})

const manualJournalForm = ref({
  journal_date: todayStr,
  outlet_id: '',
  description: '',
  reference_type: 'MANUAL_ADJUSTMENT',
  lines: [
    { account_id: '', debit: 0, credit: 0, memo: '' },
    { account_id: '', debit: 0, credit: 0, memo: '' },
  ],
})

const adjustmentCategories: Array<{
  id: AdjustmentCategory
  name: string
  badge: string
  icon: string
  description: string
}> = [
  {
    id: 'CASH_DIFF',
    name: 'Selisih Kas Laci Kasir',
    badge: 'Kasir',
    icon: '💵',
    description: 'Selisih fisik uang di laci kasir saat closing shift (kurang atau lebih fisik)',
  },
  {
    id: 'DEPRECIATION',
    name: 'Penyusutan Aset & Peralatan',
    badge: 'Bulanan',
    icon: '📉',
    description: 'Penyusutan bulanan rak display, chiller, AC, etalase, dan komputer POS',
  },
  {
    id: 'PREPAID_AMORTIZATION',
    name: 'Amortisasi Sewa Ruko Dibayar Dimuka',
    badge: 'Non-Kas',
    icon: '🏢',
    description: 'Pengakuan beban sewa ruko/lapak bulanan dari uang muka sewa tahunan',
  },
  {
    id: 'ACCRUED_EXPENSE',
    name: 'Beban Akrual (Belum Dibayar)',
    badge: 'Akrual',
    icon: '⏳',
    description: 'Mencatat biaya listrik/gaji periode berjalan yang belum dibayar saat cut-off',
  },
  {
    id: 'BAD_DEBT',
    name: 'Penghapusan Piutang Macet',
    badge: 'Piutang',
    icon: '❌',
    description: 'Menghapus saldo piutang bon pelanggan/member retail yang tidak tertagih',
  },
  {
    id: 'BANK_RECON',
    name: 'Rekonsiliasi Bank (Bunga & Biaya)',
    badge: 'Perbankan',
    icon: '🏦',
    description: 'Pencatatan bunga jasa giro bank koran atau potongan biaya administrasi bank',
  },
  {
    id: 'PRIOR_CORRECTION',
    name: 'Koreksi Saldo Periode Lalu',
    badge: 'Koreksi',
    icon: '🔄',
    description: 'Koreksi pembukuan atas periode akuntansi masa lalu yang sudah ditutup',
  },
]

const purposeJournalPreview = computed(() => {
  const cat = purposeForm.value.category

  // Find selected payment account
  const payAcc = businessPaymentAccountOptions.value.find(
    (a) => a.id === purposeForm.value.payment_account_id
  ) || businessPaymentAccountOptions.value[0] || { id: '', name: 'Kas Tunai Laci Kasir', code: '1101' }

  const findCoa = (code: string, fallbackName: string) => {
    const acc = chartOfAccounts.value.find((a: any) => a.code === code)
    return acc
      ? { id: acc.id, name: `${acc.code} - ${acc.name}`, code: acc.code }
      : { id: '', name: fallbackName, code }
  }

  if (cat === 'CASH_DIFF') {
    if (purposeForm.value.cash_diff_type === 'SHORTAGE') {
      const debitAcc = findCoa('5201', '5201 - Beban Selisih Kasir (Kurang Kas)')
      return {
        debitAccount: debitAcc,
        creditAccount: payAcc,
        needsPaymentAccount: true,
        explanation: 'Fisik kas kasir kurang: Dicatat sebagai Beban Selisih Kasir (Debet) dan mengurangi saldo Kas/Bank (Kredit).',
      }
    } else {
      const creditAcc = findCoa('4201', '4201 - Pendapatan Selisih Kasir (Lebih Kas)')
      return {
        debitAccount: payAcc,
        creditAccount: creditAcc,
        needsPaymentAccount: true,
        explanation: 'Fisik kas kasir lebih: Menambah saldo Kas/Bank (Debet) dan dicatat sebagai Pendapatan Selisih Kasir (Kredit).',
      }
    }
  } else if (cat === 'DEPRECIATION') {
    const debitAcc = findCoa('5410', '5410 - Beban Penyusutan Aset Tetap Toko')
    const creditAcc = findCoa('1302', '1302 - Akumulasi Penyusutan Peralatan Toko')
    return {
      debitAccount: debitAcc,
      creditAccount: creditAcc,
      needsPaymentAccount: false,
      explanation: 'Penyusutan bulanan nilai aset peralatan toko dicatat pada Beban Penyusutan (Debet) dan Akumulasi Penyusutan (Kredit). Non-kas.',
    }
  } else if (cat === 'PREPAID_AMORTIZATION') {
    const debitAcc = findCoa('5403', '5403 - Beban Sewa Gedung & Tempat')
    const creditAcc = findCoa('1401', '1401 - Sewa & Beban Dibayar Dimuka')
    return {
      debitAccount: debitAcc,
      creditAccount: creditAcc,
      needsPaymentAccount: false,
      explanation: 'Amortisasi sewa ruko/lapak gerai bulanan diakui sebagai Beban Sewa (Debet) dan mengurangi aset Sewa Dibayar Dimuka (Kredit). Non-kas.',
    }
  } else if (cat === 'ACCRUED_EXPENSE') {
    const debitAcc = purposeForm.value.accrued_type === 'SALARY'
      ? findCoa('5401', '5401 - Beban Gaji & Upah Karyawan')
      : findCoa('5402', '5402 - Beban Listrik, Air & Internet')
    const creditAcc = findCoa('2105', '2105 - Beban Akrual & Hutang Biaya Operasional')
    return {
      debitAccount: debitAcc,
      creditAccount: creditAcc,
      needsPaymentAccount: false,
      explanation: 'Biaya periode berjalan yang belum dibayar diakui sebagai Beban (Debet) dan Hutang Biaya Akrual (Kredit). Non-kas.',
    }
  } else if (cat === 'BAD_DEBT') {
    const debitAcc = findCoa('5499', '5499 - Beban Operasional Lain-lain (Kerugian Piutang)')
    const creditAcc = findCoa('1103', '1103 - Piutang Usaha Pelanggan')
    return {
      debitAccount: debitAcc,
      creditAccount: creditAcc,
      needsPaymentAccount: false,
      explanation: 'Penghapusan saldo piutang pelanggan macet dicatat pada Beban Kerugian Piutang (Debet) dan menghapus saldo Piutang (Kredit). Non-kas.',
    }
  } else if (cat === 'BANK_RECON') {
    if (purposeForm.value.bank_recon_type === 'INTEREST') {
      const creditAcc = findCoa('4301', '4301 - Pendapatan Bunga Bank & Jasa Giro')
      return {
        debitAccount: payAcc,
        creditAccount: creditAcc,
        needsPaymentAccount: true,
        explanation: 'Bunga tabungan/jasa giro bank menambah saldo rekening bank (Debet) dan diakui sebagai Pendapatan Bunga (Kredit).',
      }
    } else {
      const debitAcc = findCoa('5501', '5501 - Beban Administrasi Bank, EDC & QRIS')
      return {
        debitAccount: debitAcc,
        creditAccount: payAcc,
        needsPaymentAccount: true,
        explanation: 'Potongan biaya administrasi bank bulanan dicatat pada Beban Administrasi Bank (Debet) dan memotong saldo rekening bank (Kredit).',
      }
    }
  } else {
    // PRIOR_CORRECTION
    const debitAcc = findCoa('3201', '3201 - Laba Ditahan (Retained Earnings)')
    return {
      debitAccount: debitAcc,
      creditAccount: payAcc,
      needsPaymentAccount: true,
      explanation: 'Koreksi penyesuaian atas pembukuan periode lalu dibukukan langsung ke akun Laba Ditahan (Debet) dan rekening Kas/Bank lawan (Kredit).',
    }
  }
})

function onPurposeCategoryChange(cat: AdjustmentCategory) {
  purposeForm.value.category = cat
  if (cat === 'CASH_DIFF') {
    purposeForm.value.description =
      purposeForm.value.cash_diff_type === 'SHORTAGE'
        ? 'Penyesuaian selisih kurang fisik kas di laci kasir saat closing shift'
        : 'Penyesuaian selisih lebih fisik kas di laci kasir saat closing shift'
  } else if (cat === 'DEPRECIATION') {
    purposeForm.value.description = 'Penyusutan bulanan peralatan toko, rak display, chiller & komputer POS'
  } else if (cat === 'PREPAID_AMORTIZATION') {
    purposeForm.value.description = 'Amortisasi beban sewa ruko / lapak gerai toko periode berjalan'
  } else if (cat === 'ACCRUED_EXPENSE') {
    purposeForm.value.description =
      purposeForm.value.accrued_type === 'SALARY'
        ? 'Akrual hutang gaji & upah lembur karyawan belum dibayar akhir periode'
        : 'Akrual tagihan listrik, air & internet toko belum dibayar akhir periode'
  } else if (cat === 'BAD_DEBT') {
    purposeForm.value.description = 'Penghapusan saldo piutang pelanggan / member macet tidak tertagih'
  } else if (cat === 'BANK_RECON') {
    purposeForm.value.description =
      purposeForm.value.bank_recon_type === 'INTEREST'
        ? 'Penerimaan bunga tabungan / jasa giro bank berdasarkan rekening koran'
        : 'Pencatatan biaya administrasi bank / potongan buku bulanan'
  } else {
    purposeForm.value.description = 'Koreksi pembukuan atas transaksi periode akuntansi lampau yang telah ditutup'
  }
}

function onCashDiffTypeChange(type: 'SHORTAGE' | 'SURPLUS') {
  purposeForm.value.cash_diff_type = type
  if (purposeForm.value.category === 'CASH_DIFF') {
    purposeForm.value.description =
      type === 'SHORTAGE'
        ? 'Penyesuaian selisih kurang fisik kas di laci kasir saat closing shift'
        : 'Penyesuaian selisih lebih fisik kas di laci kasir saat closing shift'
  }
}

function onBankReconTypeChange(type: 'INTEREST' | 'ADMIN') {
  purposeForm.value.bank_recon_type = type
  if (purposeForm.value.category === 'BANK_RECON') {
    purposeForm.value.description =
      type === 'INTEREST'
        ? 'Penerimaan bunga tabungan / jasa giro bank berdasarkan rekening koran'
        : 'Pencatatan biaya administrasi bank / potongan buku bulanan'
  }
}

function onAccruedTypeChange(type: 'UTILITY' | 'SALARY') {
  purposeForm.value.accrued_type = type
  if (purposeForm.value.category === 'ACCRUED_EXPENSE') {
    purposeForm.value.description =
      type === 'SALARY'
        ? 'Akrual hutang gaji & upah lembur karyawan belum dibayar akhir periode'
        : 'Akrual tagihan listrik, air & internet toko belum dibayar akhir periode'
  }
}

// Modal: Transfer Dana / Kas Antar Cabang (Rekening Antar-Kantor)
const showFundTransferModal = ref<boolean>(false)
const isSubmittingFundTransfer = ref<boolean>(false)
const fundTransferForm = ref({
  transfer_date: todayStr,
  from_outlet_id: '',
  to_outlet_id: '',
  from_account_id: '',
  to_account_id: '',
  amount: 0,
  reference_no: '',
  notes: '',
})

const fromOutletPaymentAccounts = computed(() => {
  return chartOfAccounts.value.filter((a: any) =>
    a.account_category_id === 1 &&
    !a.is_header &&
    a.is_active &&
    !a.code.startsWith('19') &&
    (!a.outlet_id || a.outlet_id === fundTransferForm.value.from_outlet_id)
  )
})

const toOutletPaymentAccounts = computed(() => {
  return chartOfAccounts.value.filter((a: any) =>
    a.account_category_id === 1 &&
    !a.is_header &&
    a.is_active &&
    !a.code.startsWith('19') &&
    (!a.outlet_id || a.outlet_id === fundTransferForm.value.to_outlet_id)
  )
})

const userAccessibleOutlets = ref<any[]>([])
const fromAccountBalance = ref<number | null>(null)
const isLoadingBalance = ref<boolean>(false)

async function fetchFromAccountBalance() {
  if (!fundTransferForm.value.from_account_id) {
    fromAccountBalance.value = null
    return
  }
  isLoadingBalance.value = true
  try {
    const params: Record<string, any> = {}
    if (fundTransferForm.value.from_outlet_id) {
      params.outlet_id = fundTransferForm.value.from_outlet_id
    }
    const res = await api.get(`/accounting/accounts/${fundTransferForm.value.from_account_id}/balance`, { params })
    if (res.data?.success && res.data?.data) {
      fromAccountBalance.value = Number(res.data.data.balance || 0)
    } else {
      fromAccountBalance.value = null
    }
  } catch (err) {
    console.error('Error fetching account balance:', err)
    fromAccountBalance.value = null
  } finally {
    isLoadingBalance.value = false
  }
}

watch(
  () => [fundTransferForm.value.from_outlet_id, fundTransferForm.value.from_account_id],
  () => {
    fetchFromAccountBalance()
  }
)

// Quick Date Range Presets
function setDatePreset(preset: 'today' | 'this_month' | 'last_month' | 'this_year') {
  const now = new Date()
  if (preset === 'today') {
    startDate.value = todayStr
    endDate.value = todayStr
    asOfDate.value = todayStr
  } else if (preset === 'this_month') {
    startDate.value = new Date(now.getFullYear(), now.getMonth(), 1).toISOString().slice(0, 10)
    endDate.value = todayStr
    asOfDate.value = todayStr
  } else if (preset === 'last_month') {
    const firstDayLastMonth = new Date(now.getFullYear(), now.getMonth() - 1, 1).toISOString().slice(0, 10)
    const lastDayLastMonth = new Date(now.getFullYear(), now.getMonth(), 0).toISOString().slice(0, 10)
    startDate.value = firstDayLastMonth
    endDate.value = lastDayLastMonth
    asOfDate.value = lastDayLastMonth
  } else if (preset === 'this_year') {
    startDate.value = new Date(now.getFullYear(), 0, 1).toISOString().slice(0, 10)
    endDate.value = todayStr
    asOfDate.value = todayStr
  }
}

// Fetch Outlets List
async function fetchOutlets() {
  try {
    const res = await api.get('/outlets')
    const raw = res.data?.success ? res.data.data : (Array.isArray(res.data) ? res.data : [])
    const uniqueMap = new Map()
    for (const o of raw) {
      if (!uniqueMap.has(o.id)) {
        uniqueMap.set(o.id, o)
      }
    }
    outletsList.value = Array.from(uniqueMap.values())
  } catch (e) {
    console.error('Error fetching outlets:', e)
  }
}

watch(selectedOutletId, () => {
  onFilterChange()
})

// Tab-level Cache and Lazy-Loading Engine
const loadedTabs = ref<Record<string, boolean>>({
  income: false,
  balance_sheet: false,
  journal: false,
  closing: false,
  dividend: false,
  capital: false,
  bank: false,
  coa: false,
})

// Fetch Data spesifik hanya untuk tab yang diminta (On-Demand / Lazy-load)
async function fetchDataForTab(tab: 'income' | 'balance_sheet' | 'journal' | 'closing' | 'dividend' | 'capital' | 'bank' | 'coa', force = false) {
  if (loadedTabs.value[tab] && !force) {
    return
  }

  isLoading.value = true
  const params: Record<string, any> = {}
  if (selectedOutletId.value) params.outlet_id = selectedOutletId.value

  try {
    if (tab === 'income') {
      const res = await api.get('/reports/income-statement', {
        params: {
          ...params,
          start_date: startDate.value,
          end_date: endDate.value,
        },
      })
      if (res.data?.success) incomeStatement.value = res.data.data
    } else if (tab === 'balance_sheet') {
      const res = await api.get('/reports/balance-sheet', {
        params: {
          ...params,
          as_of_date: asOfDate.value,
        },
      })
      if (res.data?.success) balanceSheet.value = res.data.data
    } else if (tab === 'journal') {
      await fetchJournalEntries()
    } else if (tab === 'closing') {
      if (closingSubTab.value === 'periods') {
        await fetchAccountingPeriods()
      } else {
        await fetchClosingList()
      }
    } else if (tab === 'dividend') {
      await fetchDividendSummary()
      await fetchDividendList()
      if (chartOfAccounts.value.length === 0) {
        const res = await api.get('/accounting/chart-of-accounts')
        if (res.data?.success) chartOfAccounts.value = res.data.data
      }
    } else if (tab === 'capital') {
      await fetchCapitalAccounts()
      await fetchCapitalSummary()
      await fetchCapitalDeposits()
    } else if (tab === 'bank') {
      await fetchBankAccounts()
    } else if (tab === 'coa') {
      const res = await api.get('/accounting/chart-of-accounts')
      if (res.data?.success) chartOfAccounts.value = res.data.data
    }
    loadedTabs.value[tab] = true
  } catch (err: any) {
    console.error(`Error loading accounting tab ${tab}:`, err)
    showToast(err.response?.data?.message || `Gagal memuat data ${tab}`, 'error')
  } finally {
    isLoading.value = false
  }
}

// Fetch Journal Entries Server-side dengan pagination, filter, dan sorting
async function fetchJournalEntries() {
  isLoading.value = true
  try {
    const params: Record<string, any> = {
      page: jrnPage.value,
      page_size: jrnPageSize.value,
      sort_by: jrnSortBy.value,
      sort_dir: jrnSortDir.value,
      start_date: startDate.value,
      end_date: endDate.value,
    }
    if (selectedOutletId.value) params.outlet_id = selectedOutletId.value
    if (jrnSearch.value.trim()) params.search = jrnSearch.value.trim()

    const res = await api.get('/accounting/journal-entries', { params })
    if (res.data?.success) {
      journalEntries.value = res.data.data
      jrnTotalItems.value = res.data.total_items ?? res.data.data.length
    }
  } catch (err: any) {
    console.error('Error loading journal entries:', err)
    showToast(err.response?.data?.message || 'Gagal memuat jurnal umum', 'error')
  } finally {
    isLoading.value = false
  }
}

function handleJrnSort(column: string) {
  if (jrnSortBy.value === column) {
    jrnSortDir.value = jrnSortDir.value === 'asc' ? 'desc' : 'asc'
  } else {
    jrnSortBy.value = column
    jrnSortDir.value = 'desc'
  }
  jrnPage.value = 1
  fetchJournalEntries()
}

function handleJrnSearch() {
  jrnPage.value = 1
  fetchJournalEntries()
}

// Grid Navigasi Menu Kotak-Kotak (Hub Menu Akuntansi) - Bergaya Command Ribbon Menu Utama
const accountingMenuTiles = computed(() => [
  {
    id: 'income' as const,
    title: 'Laba Rugi',
    subtitle: 'Pendapatan, HPP, beban operasional & profit bersih',
    tag: 'Income Statement',
    icon: TrendingUp,
    iconName: 'fluent-color:data-bar-vertical-ascending-24',
    badge: incomeStatement.value ? formatCurrency(incomeStatement.value.net_profit) : 'Live',
    badgeColor: incomeStatement.value && Number(incomeStatement.value.net_profit) >= 0 ? 'bg-emerald-100 text-emerald-800' : 'bg-rose-100 text-rose-800',
  },
  {
    id: 'balance_sheet' as const,
    title: 'Posisi Aset & Neraca',
    subtitle: 'Kekayaan aset, liabilitas hutang & ekuitas modal',
    tag: 'Balance Sheet',
    icon: Scale,
    iconName: 'fluent-color:chart-multiple-24',
    badge: balanceSheet.value ? formatCurrency(balanceSheet.value.total_assets) : 'Live',
    badgeColor: 'bg-sky-100 text-sky-800',
  },
  {
    id: 'journal' as const,
    title: 'Buku Jurnal Umum',
    subtitle: 'Log transaksi mutasi debet kredit double-entry',
    tag: 'General Ledger',
    icon: BookOpen,
    iconName: 'fluent-color:receipt-24',
    badge: `${jrnTotalItems.value || 0}`,
    badgeColor: 'bg-indigo-100 text-indigo-800',
  },
  {
    id: 'closing' as const,
    title: 'Tutup Buku Periode',
    subtitle: 'Kunci transaksi berkala & simpan laba ditahan',
    tag: 'Period Closing',
    icon: Lock,
    iconName: 'fluent-color:lock-closed-24',
    badge: `${closingSubTab.value === 'periods' ? (periodTotalItems.value || 0) : (closingTotalItems.value || 0)}`,
    badgeColor: 'bg-amber-100 text-amber-800',
  },
  {
    id: 'dividend' as const,
    title: 'Pembagian Dividen',
    subtitle: 'Distribusi laba usaha kepada pemegang saham',
    tag: 'Dividends',
    icon: Coins,
    iconName: 'fluent-color:coin-multiple-24',
    badge: `${dividendTotalItems.value || 0}`,
    badgeColor: 'bg-purple-100 text-purple-800',
  },
  {
    id: 'capital' as const,
    title: 'Setoran Modal Usaha',
    subtitle: 'Pencatatan penambahan ekuitas & modal pemilik',
    tag: 'Equity Capital',
    icon: PiggyBank,
    iconName: 'fluent-color:vault-24',
    badge: `${capitalTotal.value || 0}`,
    badgeColor: 'bg-teal-100 text-teal-800',
  },
  {
    id: 'bank' as const,
    title: 'Rekening Bank & Kas',
    subtitle: 'Akun penampung EDC, QRIS, & transfer bank',
    tag: 'Cash & Banks',
    icon: Landmark,
    iconName: 'fluent-color:building-government-24',
    badge: `${bankAccounts.value?.length || 0}`,
    badgeColor: 'bg-blue-100 text-blue-800',
  },
  {
    id: 'coa' as const,
    title: 'Bagan Akun (COA)',
    subtitle: 'Master nomor rekening & klasifikasi akuntansi',
    tag: 'Chart of Accounts',
    icon: FileText,
    iconName: 'fluent-color:database-24',
    badge: `${chartOfAccounts.value?.length || 0}`,
    badgeColor: 'bg-slate-100 text-slate-800',
  },
])

// =========================================================================
// UNIVERSAL PRINT ACCOUNTING REPORT STATE & HANDLERS
// =========================================================================
const showPrintReportModal = ref<boolean>(false)
const reportToPrint = ref<'income' | 'balance_sheet' | 'journal' | 'closing' | 'dividend' | 'capital' | 'bank' | 'coa'>('income')
const isPreparingPrintData = ref<boolean>(false)

const REPORT_METADATA: Record<string, { title: string; subtitle: string; periodLabel: string }> = {
  income: {
    title: 'LAPORAN LABA RUGI',
    subtitle: 'INCOME STATEMENT / PROFIT & LOSS REPORT',
    periodLabel: 'Periode Berjalan',
  },
  balance_sheet: {
    title: 'LAPORAN POSISI KEUANGAN / NERACA',
    subtitle: 'STATEMENT OF FINANCIAL POSITION / BALANCE SHEET',
    periodLabel: 'Posisi Per Tanggal',
  },
  journal: {
    title: 'BUKU JURNAL UMUM',
    subtitle: 'GENERAL LEDGER & GENERAL JOURNAL TRANSACTIONS',
    periodLabel: 'Periode Transaksi',
  },
  closing: {
    title: 'LAPORAN RIWAYAT PENUTUPAN BUKU PERIODE',
    subtitle: 'ACCOUNTING PERIOD CLOSINGS AUDIT LOG',
    periodLabel: 'Periode Tutup Buku',
  },
  dividend: {
    title: 'LAPORAN REKAPITULASI PEMBAGIAN DIVIDEN',
    subtitle: 'SHAREHOLDER DIVIDEND DISTRIBUTION REPORT',
    periodLabel: 'Periode Distribusi',
  },
  capital: {
    title: 'LAPORAN REKAPITULASI SETORAN MODAL USAHA',
    subtitle: 'EQUITY & CAPITAL DEPOSITS REPORT',
    periodLabel: 'Periode Setoran Modal',
  },
  bank: {
    title: 'LAPORAN REKAPITULASI REKENING BANK & KAS',
    subtitle: 'BANK & OPERATIONAL CASH ACCOUNTS DIRECTORY',
    periodLabel: 'Posisi Data Master',
  },
  coa: {
    title: 'BAGAN AKUN STANDAR (CHART OF ACCOUNTS)',
    subtitle: 'STANDARDIZED CHART OF ACCOUNTS & CLASSIFICATIONS',
    periodLabel: 'Daftar Akun Master',
  },
}

function checkTabHasData(tab: string): boolean {
  switch (tab) {
    case 'income': return incomeStatement.value !== null
    case 'balance_sheet': return balanceSheet.value !== null
    case 'journal': return journalEntries.value.length > 0
    case 'closing': return closingList.value.length > 0
    case 'dividend': return dividendList.value.length > 0
    case 'capital': return capitalDeposits.value.length > 0
    case 'bank': return bankAccounts.value.length > 0
    case 'coa': return chartOfAccounts.value.length > 0
    default: return false
  }
}

function openPrintReport(tab?: 'income' | 'balance_sheet' | 'journal' | 'closing' | 'dividend' | 'capital' | 'bank' | 'coa') {
  if (!authStore.user) {
    authStore.fetchCurrentUser().catch(() => {})
  }
  const targetTab = (typeof tab === 'string' && tab) ? tab : activeTab.value
  reportToPrint.value = targetTab
  showPrintReportModal.value = true
  showToast(`Membuka Dokumen Laporan ${REPORT_METADATA[targetTab]?.title || ''}...`, 'success')
  
  if (!checkTabHasData(targetTab)) {
    isPreparingPrintData.value = true
    fetchDataForTab(targetTab)
      .then(() => {
        if (targetTab === 'journal' && journalEntries.value.length === 0) {
          return fetchJournalEntries()
        }
      })
      .catch((err) => {
        console.warn('Gagal memuat sebagian data cetak:', err)
      })
      .finally(() => {
        isPreparingPrintData.value = false
      })
  }
}

function handleSwitchPrintReport(newTab: 'income' | 'balance_sheet' | 'journal' | 'closing' | 'dividend' | 'capital' | 'bank' | 'coa') {
  reportToPrint.value = newTab
  if (!checkTabHasData(newTab)) {
    isPreparingPrintData.value = true
    fetchDataForTab(newTab)
      .then(() => {
        if (newTab === 'journal' && journalEntries.value.length === 0) {
          return fetchJournalEntries()
        }
      })
      .catch((err) => {
        console.warn('Gagal memuat data laporan:', err)
      })
      .finally(() => {
        isPreparingPrintData.value = false
      })
  }
}

function triggerPrintReport() {
  window.print()
}

// Navigasi Tab: selalu panggil API server-side on-demand saat berpindah tab
function switchTab(tab: 'income' | 'balance_sheet' | 'journal' | 'closing' | 'dividend' | 'capital' | 'bank' | 'coa') {
  activeTab.value = tab
  fetchDataForTab(tab, true)
}

// Segarkan data tab yang sedang aktif
function refreshActiveTab() {
  fetchDataForTab(activeTab.value, true)
}

// Reset flag cache saat filter berubah, dan langsung reload tab yang sedang dibuka
function onFilterChange() {
  loadedTabs.value = {
    income: false,
    balance_sheet: false,
    journal: false,
    closing: false,
    dividend: false,
    capital: false,
    bank: false,
    coa: false,
  }
  jrnPage.value = 1
  fetchDataForTab(activeTab.value, true)
}

// Handler: Tutup Buku
function openClosingModal() {
  closingForm.value = {
    outlet_id: selectedOutletId.value,
    period_start: startDate.value,
    period_end: endDate.value,
    notes: '',
  }
  showClosingModal.value = true
}

async function handleExecuteClosing() {
  if (!authStore.hasPermission(['accounting:close_period', 'accounting:write'])) {
    showToast('Akses ditolak: Anda tidak memiliki wewenang melakukan tutup buku (accounting:close_period)', 'error')
    return
  }
  if (!closingForm.value.period_start || !closingForm.value.period_end) {
    showToast('Tentukan tanggal awal dan akhir periode tutup buku', 'error')
    return
  }

  isSubmittingClosing.value = true
  try {
    const payload: Record<string, any> = {
      period_start: closingForm.value.period_start,
      period_end: closingForm.value.period_end,
      notes: closingForm.value.notes || undefined,
    }
    if (closingForm.value.outlet_id) {
      payload.outlet_id = closingForm.value.outlet_id
    }

    const res = await api.post('/accounting/closings', payload)
    if (res.data?.success) {
      showToast(res.data.message || 'Tutup buku periode berhasil dibukukan', 'success')
      showClosingModal.value = false
      onFilterChange()
    }
  } catch (err: any) {
    showToast(err.response?.data?.message || err.message || 'Gagal melakukan tutup buku', 'error')
  } finally {
    isSubmittingClosing.value = false
  }
}

// Handler: Buka Kembali Buku (Reopen)
function viewClosingJournal(journalNum: string) {
  activeTab.value = 'journal'
  jrnSearch.value = journalNum
  jrnPage.value = 1
  fetchJournalEntries()
}

function openReopenModal(closing: any) {
  selectedClosingToReopen.value = closing
  reopenReason.value = ''
  showReopenModal.value = true
}

async function handleExecuteReopen() {
  if (!authStore.hasPermission(['accounting:reopen_period', 'accounting:write'])) {
    showToast('Akses ditolak: Anda tidak memiliki wewenang membuka kembali tutup buku (accounting:reopen_period)', 'error')
    return
  }
  if (!reopenReason.value.trim()) {
    showToast('Wajib mencantumkan alasan pembukaan kembali untuk jejak audit standar', 'error')
    return
  }

  isSubmittingReopen.value = true
  try {
    const res = await api.post(`/accounting/closings/${selectedClosingToReopen.value.id}/reopen`, {
      reason: reopenReason.value.trim(),
    })
    if (res.data?.success) {
      showToast(res.data.message || 'Buku periode berhasil dibuka kembali', 'success')
      showReopenModal.value = false
      onFilterChange()
    }
  } catch (err: any) {
    showToast(err.response?.data?.message || err.message || 'Gagal membuka kembali buku', 'error')
  } finally {
    isSubmittingReopen.value = false
  }
}

// =========================================================================
// HANDLERS: SOFT CLOSE (PERIODE AKUNTANSI) & HARD CLOSE (TAHUNAN)
// =========================================================================
async function fetchAccountingPeriods() {
  isPeriodLoading.value = true
  try {
    const params: any = {
      page: periodPage.value,
      page_size: periodPageSize.value,
      sort_by: periodSortBy.value,
      sort_dir: periodSortDir.value,
    }
    if (periodSearch.value.trim()) {
      params.search = periodSearch.value.trim()
    }
    if (periodStatus.value) {
      params.status = periodStatus.value
    }
    const res = await api.get('/accounting/periods', { params })
    if (res.data?.success) {
      accountingPeriods.value = res.data.data
      periodTotalItems.value = res.data.total_items ?? res.data.data.length
      periodTotalPages.value = res.data.total_pages ?? 1
    }
  } catch (err: any) {
    console.error('Gagal mengambil daftar periode akuntansi:', err)
  } finally {
    isPeriodLoading.value = false
  }
}

async function fetchClosingList() {
  isClosingLoading.value = true
  try {
    const params: any = {
      page: closingPage.value,
      page_size: closingPageSize.value,
      sort_by: closingSortBy.value,
      sort_dir: closingSortDir.value,
    }
    if (selectedOutletId.value) {
      params.outlet_id = selectedOutletId.value
    }
    if (closingSearch.value.trim()) {
      params.search = closingSearch.value.trim()
    }
    if (closingStatus.value) {
      params.status = closingStatus.value
    }
    const res = await api.get('/accounting/closings', { params })
    if (res.data?.success) {
      closingList.value = res.data.data
      closingTotalItems.value = res.data.total_items ?? res.data.data.length
      closingTotalPages.value = res.data.total_pages ?? 1
    }
  } catch (err: any) {
    console.error('Gagal mengambil riwayat tutup buku:', err)
  } finally {
    isClosingLoading.value = false
  }
}

async function switchClosingSubTab(tab: 'periods' | 'closings') {
  closingSubTab.value = tab
  if (tab === 'periods') {
    await fetchAccountingPeriods()
  } else {
    await fetchClosingList()
  }
}

function togglePeriodSort(col: string) {
  if (periodSortBy.value === col) {
    periodSortDir.value = periodSortDir.value === 'asc' ? 'desc' : 'asc'
  } else {
    periodSortBy.value = col
    periodSortDir.value = 'desc'
  }
  periodPage.value = 1
  fetchAccountingPeriods()
}

function toggleClosingSort(col: string) {
  if (closingSortBy.value === col) {
    closingSortDir.value = closingSortDir.value === 'asc' ? 'desc' : 'asc'
  } else {
    closingSortBy.value = col
    closingSortDir.value = 'desc'
  }
  closingPage.value = 1
  fetchClosingList()
}

let periodSearchTimeout: any = null
function onPeriodSearchInput() {
  clearTimeout(periodSearchTimeout)
  periodSearchTimeout = setTimeout(() => {
    periodPage.value = 1
    fetchAccountingPeriods()
  }, 350)
}

let closingSearchTimeout: any = null
function onClosingSearchInput() {
  clearTimeout(closingSearchTimeout)
  closingSearchTimeout = setTimeout(() => {
    closingPage.value = 1
    fetchClosingList()
  }, 350)
}

function openCreatePeriodModal() {
  const now = new Date()
  const monthName = now.toLocaleString('id-ID', { month: 'long', year: 'numeric' })
  periodForm.value = {
    name: `Periode ${monthName}`,
    start_date: firstDayOfMonth,
    end_date: todayStr,
  }
  showPeriodModal.value = true
}

async function handleSavePeriod() {
  if (!periodForm.value.name.trim() || !periodForm.value.start_date || !periodForm.value.end_date) {
    showToast('Nama periode, tanggal mulai, dan tanggal selesai wajib diisi.', 'error')
    return
  }
  isSubmittingPeriod.value = true
  try {
    const res = await api.post('/accounting/periods', {
      name: periodForm.value.name.trim(),
      start_date: periodForm.value.start_date,
      end_date: periodForm.value.end_date,
    })
    if (res.data?.success) {
      showToast(`Periode akuntansi '${res.data.data.name}' berhasil dibuat.`, 'success')
      showPeriodModal.value = false
      await fetchAccountingPeriods()
    }
  } catch (err: any) {
    showToast(err.response?.data?.message || err.message || 'Gagal membuat periode akuntansi', 'error')
  } finally {
    isSubmittingPeriod.value = false
  }
}

async function handleSoftClosePeriod(period: any) {
  if (!authStore.hasPermission(['accounting:periods:close', 'accounting:close_period', 'accounting:write'])) {
    showToast('Akses ditolak: Anda tidak memiliki wewenang untuk mengunci periode.', 'error')
    return
  }
  const confirmed = confirm(
    `Apakah Anda yakin ingin MENGUNCI periode "${period.name}" (${formatDate(period.start_date)} s/d ${formatDate(period.end_date)})?\n\nSetelah dikunci, seluruh mutasi kas, penjualan POS, pembelian, dan penyesuaian pada rentang tanggal tersebut akan ditolak oleh sistem (Soft Close).`
  )
  if (!confirmed) return

  try {
    const res = await api.post(`/accounting/periods/${period.id}/close`)
    if (res.data?.success) {
      showToast(`Periode '${period.name}' berhasil dikunci (Soft Close aktif).`, 'success')
      await fetchAccountingPeriods()
    }
  } catch (err: any) {
    showToast(err.response?.data?.message || err.message || 'Gagal mengunci periode akuntansi', 'error')
  }
}

function openReopenPeriodModal(period: any) {
  selectedPeriodToReopen.value = period
  periodReopenReason.value = ''
  showReopenPeriodModal.value = true
}

async function handleExecuteReopenPeriod() {
  if (!authStore.hasPermission(['accounting:periods:reopen', 'accounting:reopen_period', 'accounting:write'])) {
    showToast('Akses ditolak: Anda tidak memiliki wewenang untuk membuka kembali periode.', 'error')
    return
  }
  if (!periodReopenReason.value.trim()) {
    showToast('Alasan pembukaan kunci periode wajib dicantumkan untuk audit log.', 'error')
    return
  }
  isReopeningPeriod.value = true
  try {
    const res = await api.post(`/accounting/periods/${selectedPeriodToReopen.value.id}/reopen`, {
      reason: periodReopenReason.value.trim(),
    })
    if (res.data?.success) {
      showToast(`Kunci periode '${selectedPeriodToReopen.value.name}' berhasil dibuka kembali.`, 'success')
      showReopenPeriodModal.value = false
      await fetchAccountingPeriods()
    }
  } catch (err: any) {
    showToast(err.response?.data?.message || err.message || 'Gagal membuka kembali periode', 'error')
  } finally {
    isReopeningPeriod.value = false
  }
}

function openHardCloseModal() {
  hardCloseForm.value = {
    fiscal_year: new Date().getFullYear(),
    description: '',
  }
  showHardCloseModal.value = true
}

async function handleExecuteHardClose() {
  if (!authStore.hasPermission(['accounting:hard_close', 'accounting:close_period', 'accounting:write'])) {
    showToast('Akses ditolak: Anda tidak memiliki izin untuk melakukan Tutup Buku Akhir Tahun (accounting:hard_close).', 'error')
    return
  }
  if (hardCloseForm.value.fiscal_year < 2000 || hardCloseForm.value.fiscal_year > 2100) {
    showToast('Tahun buku tidak valid.', 'error')
    return
  }

  isSubmittingHardClose.value = true
  try {
    const res = await api.post('/accounting/hard-close', {
      fiscal_year: Number(hardCloseForm.value.fiscal_year),
      description: hardCloseForm.value.description.trim() || undefined,
    })
    if (res.data?.success) {
      showToast(res.data.message || 'Hard close tahun buku berhasil diposting ke Laba Ditahan (3201)', 'success')
      showHardCloseModal.value = false
      onFilterChange()
      await fetchAccountingPeriods()
    }
  } catch (err: any) {
    showToast(err.response?.data?.message || err.message || 'Gagal melakukan hard close akhir tahun', 'error')
  } finally {
    isSubmittingHardClose.value = false
  }
}

// Handler: Jurnal Manual / Penyesuaian
async function openManualJournalModal() {
  await fetchBankAccounts()
  if (chartOfAccounts.value.length === 0) {
    try {
      const res = await api.get('/accounting/chart-of-accounts')
      if (res.data?.success) chartOfAccounts.value = res.data.data
    } catch (e) {
      console.error(e)
    }
  }

  adjustmentMode.value = 'purpose'
  purposeForm.value = {
    journal_date: todayStr,
    outlet_id: selectedOutletId.value || '',
    category: 'CASH_DIFF',
    cash_diff_type: 'SHORTAGE',
    bank_recon_type: 'ADMIN',
    accrued_type: 'UTILITY',
    payment_account_id: businessPaymentAccountOptions.value[0]?.id || '',
    amount: 0,
    description: 'Penyesuaian selisih kurang fisik kas di laci kasir saat closing shift',
  }

  manualJournalForm.value = {
    journal_date: todayStr,
    outlet_id: selectedOutletId.value,
    description: '',
    reference_type: 'MANUAL_ADJUSTMENT',
    lines: [
      { account_id: '', debit: 0, credit: 0, memo: '' },
      { account_id: '', debit: 0, credit: 0, memo: '' },
    ],
  }
  showManualJournalModal.value = true
}

function addManualJournalLine() {
  manualJournalForm.value.lines.push({ account_id: '', debit: 0, credit: 0, memo: '' })
}

function removeManualJournalLine(index: number) {
  if (manualJournalForm.value.lines.length > 2) {
    manualJournalForm.value.lines.splice(index, 1)
  }
}

const manualJournalTotalDebit = computed(() => {
  return manualJournalForm.value.lines.reduce((sum, l) => sum + Number(l.debit || 0), 0)
})

const manualJournalTotalCredit = computed(() => {
  return manualJournalForm.value.lines.reduce((sum, l) => sum + Number(l.credit || 0), 0)
})

const isManualJournalBalanced = computed(() => {
  return (
    manualJournalTotalDebit.value > 0 &&
    manualJournalTotalDebit.value === manualJournalTotalCredit.value
  )
})

async function handleSaveManualJournal() {
  if (!authStore.hasPermission('accounting:write')) {
    showToast('Akses ditolak: Anda tidak memiliki wewenang membuat jurnal manual (accounting:write)', 'error')
    return
  }
  if (!manualJournalForm.value.description.trim()) {
    showToast('Keterangan jurnal wajib diisi', 'error')
    return
  }

  if (!isManualJournalBalanced.value) {
    showToast('Total Debet dan Total Kredit harus sama besar (seimbang)', 'error')
    return
  }

  // Cek apakah ada baris yang belum memilih akun
  for (const l of manualJournalForm.value.lines) {
    if (!l.account_id) {
      showToast('Pilih akun CoA untuk setiap baris jurnal', 'error')
      return
    }
  }

  isSubmittingManualJournal.value = true
  try {
    const payload: Record<string, any> = {
      journal_date: manualJournalForm.value.journal_date,
      description: manualJournalForm.value.description.trim(),
      reference_type: manualJournalForm.value.reference_type,
      lines: manualJournalForm.value.lines.map((l) => ({
        account_id: l.account_id,
        debit: Number(l.debit || 0),
        credit: Number(l.credit || 0),
        memo: l.memo || undefined,
      })),
    }
    if (manualJournalForm.value.outlet_id) {
      payload.outlet_id = manualJournalForm.value.outlet_id
    }

    const res = await api.post('/accounting/journal-entries', payload)
    if (res.data?.success) {
      showToast('Jurnal penyesuaian berhasil dibukukan', 'success')
      showManualJournalModal.value = false
      onFilterChange()
    }
  } catch (err: any) {
    showToast(err.response?.data?.message || err.message || 'Gagal menyimpan jurnal', 'error')
  } finally {
    isSubmittingManualJournal.value = false
  }
}

async function handleSavePurposeAdjustment() {
  if (!authStore.hasPermission('accounting:write')) {
    showToast('Akses ditolak: Anda tidak memiliki wewenang membuat jurnal (accounting:write)', 'error')
    return
  }

  const amt = Number(purposeForm.value.amount || 0)
  if (amt <= 0) {
    showToast('Nominal penyesuaian harus lebih besar dari Rp 0', 'error')
    return
  }

  const preview = purposeJournalPreview.value
  if (!preview.debitAccount?.id || !preview.creditAccount?.id) {
    showToast('Akun jurnal debit atau kredit tidak ditemukan di master Chart of Accounts', 'error')
    return
  }

  const desc = purposeForm.value.description.trim()
  if (!desc) {
    showToast('Keterangan penyesuaian wajib diisi', 'error')
    return
  }

  isSubmittingManualJournal.value = true
  try {
    const payload: Record<string, any> = {
      journal_date: purposeForm.value.journal_date,
      description: desc,
      reference_type: 'MANUAL_ADJUSTMENT',
      lines: [
        {
          account_id: preview.debitAccount.id,
          debit: amt,
          credit: 0,
          memo: `[Debet] ${desc}`,
        },
        {
          account_id: preview.creditAccount.id,
          debit: 0,
          credit: amt,
          memo: `[Kredit] ${desc}`,
        },
      ],
    }

    if (purposeForm.value.outlet_id) {
      payload.outlet_id = purposeForm.value.outlet_id
    }

    const res = await api.post('/accounting/journal-entries', payload)
    if (res.data?.success) {
      showToast('Jurnal penyesuaian berhasil dibukukan secara otomatis!', 'success')
      showManualJournalModal.value = false
      onFilterChange()
    }
  } catch (err: any) {
    showToast(err.response?.data?.message || err.message || 'Gagal menyimpan jurnal penyesuaian', 'error')
  } finally {
    isSubmittingManualJournal.value = false
  }
}

// =========================================================================
// HANDLER: TRANSFER DANA / KAS ANTAR CABANG (REKENING ANTAR-KANTOR)
// =========================================================================
async function openFundTransferModal() {
  if (chartOfAccounts.value.length === 0) {
    fetchDataForTab('coa', true)
  }
  try {
    const res = await api.get('/me/outlets')
    if (res.data?.success && Array.isArray(res.data.data)) {
      userAccessibleOutlets.value = res.data.data
    }
  } catch (err) {
    console.error('Gagal memuat outlet user:', err)
  }

  const defaultFrom = userAccessibleOutlets.value.find((o) => o.id === selectedOutletId.value)?.id
    || userAccessibleOutlets.value[0]?.id
    || ''
  const defaultTo = outletsList.value.find((o) => o.id !== defaultFrom)?.id || defaultFrom

  fundTransferForm.value = {
    transfer_date: todayStr,
    from_outlet_id: defaultFrom,
    to_outlet_id: defaultTo,
    from_account_id: '',
    to_account_id: '',
    amount: 0,
    reference_no: '',
    notes: '',
  }
  fromAccountBalance.value = null
  showFundTransferModal.value = true
}

async function handleSaveFundTransfer() {
  if (!authStore.hasPermission(['accounting:fund_transfer', 'accounting:write'])) {
    showToast('Akses ditolak: Anda tidak memiliki wewenang transfer kas antar cabang', 'error')
    return
  }
  if (!fundTransferForm.value.from_outlet_id || !fundTransferForm.value.to_outlet_id) {
    showToast('Pilih cabang asal dan cabang tujuan', 'error')
    return
  }
  if (!fundTransferForm.value.from_account_id || !fundTransferForm.value.to_account_id) {
    showToast('Pilih rekening kas/bank asal dan rekening tujuan', 'error')
    return
  }
  if (fundTransferForm.value.from_account_id === fundTransferForm.value.to_account_id) {
    showToast('Rekening kas/bank asal dan rekening tujuan tidak boleh sama', 'error')
    return
  }
  if (Number(fundTransferForm.value.amount || 0) <= 0) {
    showToast('Nominal transfer harus lebih besar dari 0', 'error')
    return
  }
  if (fromAccountBalance.value !== null && fromAccountBalance.value <= 0) {
    showToast(
      `Saldo rekening kas/bank asal tidak mencukupi (${formatCurrency(fromAccountBalance.value)}). Silakan lakukan setoran kas terlebih dahulu.`,
      'error'
    )
    return
  }
  if (fromAccountBalance.value !== null && Number(fundTransferForm.value.amount) > fromAccountBalance.value) {
    showToast(
      `Nominal transfer (${formatCurrency(fundTransferForm.value.amount)}) melebihi saldo kas/bank asal (${formatCurrency(fromAccountBalance.value)})`,
      'error'
    )
    return
  }

  isSubmittingFundTransfer.value = true
  try {
    const payload = {
      from_outlet_id: fundTransferForm.value.from_outlet_id,
      to_outlet_id: fundTransferForm.value.to_outlet_id,
      from_account_id: fundTransferForm.value.from_account_id,
      to_account_id: fundTransferForm.value.to_account_id,
      amount: Number(fundTransferForm.value.amount),
      transfer_date: fundTransferForm.value.transfer_date,
      reference_no: fundTransferForm.value.reference_no.trim() || undefined,
      notes: fundTransferForm.value.notes.trim() || undefined,
    }

    const res = await api.post('/accounting/fund-transfers', payload)
    if (res.data?.success) {
      showToast(res.data.message || 'Transfer dana kas/bank berhasil dibukukan', 'success')
      showFundTransferModal.value = false
      onFilterChange()
    }
  } catch (err: any) {
    showToast(err.response?.data?.message || err.message || 'Gagal memproses transfer dana', 'error')
  } finally {
    isSubmittingFundTransfer.value = false
  }
}

// =========================================================================
// HANDLER: SETORAN MODAL USAHA (CAPITAL DEPOSIT)
// =========================================================================
async function fetchCapitalAccounts() {
  try {
    const res = await api.get('/capital/accounts')
    if (res.data?.success) {
      capitalPaymentAccounts.value = res.data.data.payment_accounts || []
      capitalEquityAccounts.value = res.data.data.equity_accounts || []
      if (!capitalForm.value.payment_account_id && capitalPaymentAccounts.value.length > 0) {
        capitalForm.value.payment_account_id = capitalPaymentAccounts.value[0].id
      }
      if (!capitalForm.value.equity_account_id && capitalEquityAccounts.value.length > 0) {
        capitalForm.value.equity_account_id = capitalEquityAccounts.value[0].id
      }
    }
  } catch (err) {
    console.error('Error fetching capital accounts:', err)
  }
}

async function fetchCapitalDeposits() {
  try {
    const params: Record<string, any> = {
      page: capitalPage.value,
      limit: capitalPageSize.value,
    }
    if (selectedOutletId.value) params.outlet_id = selectedOutletId.value
    if (capitalStatusFilter.value !== 'ALL') params.status = capitalStatusFilter.value
    if (capitalSearch.value.trim()) params.search = capitalSearch.value.trim()
    if (startDate.value) params.start_date = startDate.value
    if (endDate.value) params.end_date = endDate.value

    const res = await api.get('/capital', { params })
    if (res.data?.success) {
      capitalDeposits.value = res.data.data || []
      capitalTotal.value = res.data.total || 0
    }
  } catch (err: any) {
    console.error('Error fetching capital deposits:', err)
  }
}

async function fetchCapitalSummary() {
  try {
    const params: Record<string, any> = {}
    if (selectedOutletId.value) params.outlet_id = selectedOutletId.value

    const res = await api.get('/capital/summary', { params })
    if (res.data?.success) {
      capitalSummary.value = res.data.data
    }
  } catch (err: any) {
    console.error('Error fetching capital summary:', err)
  }
}

async function openCapitalModal() {
  await Promise.all([
    fetchCapitalAccounts(),
    fetchBankAccounts(),
  ])
  if (chartOfAccounts.value.length === 0) {
    try {
      const res = await api.get('/accounting/chart-of-accounts')
      if (res.data?.success) chartOfAccounts.value = res.data.data
    } catch (e) {
      console.error(e)
    }
  }

  const defaultEquity = capitalEquityAccounts.value.find((a: any) => a.code === '3101') ||
    chartOfAccounts.value.find((a: any) => a.code === '3101') ||
    capitalEquityAccounts.value[0]

  capitalForm.value = {
    outlet_id: selectedOutletId.value || '',
    deposit_date: todayStr,
    investor_name: '',
    amount: 0,
    payment_account_id: businessPaymentAccountOptions.value[0]?.id || capitalPaymentAccounts.value[0]?.id || '',
    equity_account_id: defaultEquity?.id || '',
    reference_no: '',
    notes: '',
  }
  showAdvancedEquityOption.value = false
  showCapitalModal.value = true
}

async function handleSaveCapital() {
  if (!capitalForm.value.investor_name.trim()) {
    showToast('Nama pemilik / penyetor modal wajib diisi.', 'error')
    return
  }
  if (!capitalForm.value.amount || Number(capitalForm.value.amount) <= 0) {
    showToast('Nominal setoran modal harus lebih dari Rp 0.', 'error')
    return
  }
  if (!capitalForm.value.payment_account_id) {
    showToast('Pilih akun kas/bank penerima dana.', 'error')
    return
  }
  if (!capitalForm.value.equity_account_id) {
    showToast('Pilih akun modal/ekuitas.', 'error')
    return
  }

  isSubmittingCapital.value = true
  try {
    const payload = {
      outlet_id: capitalForm.value.outlet_id || null,
      deposit_date: capitalForm.value.deposit_date || todayStr,
      investor_name: capitalForm.value.investor_name.trim(),
      amount: Number(capitalForm.value.amount),
      payment_account_id: capitalForm.value.payment_account_id,
      equity_account_id: capitalForm.value.equity_account_id,
      reference_no: capitalForm.value.reference_no.trim() || undefined,
      notes: capitalForm.value.notes.trim() || undefined,
    }

    const res = await api.post('/capital', payload)
    if (res.data?.success) {
      showToast('Setoran modal usaha berhasil dibukukan.', 'success')
      showCapitalModal.value = false
      await fetchCapitalDeposits()
      await fetchCapitalSummary()
      loadedTabs.value.balance_sheet = false
      loadedTabs.value.journal = false
    }
  } catch (err: any) {
    showToast(err.response?.data?.message || err.message || 'Gagal menyimpan setoran modal.', 'error')
  } finally {
    isSubmittingCapital.value = false
  }
}

function openCancelCapitalModal(deposit: CapitalDepositDetail) {
  selectedCapitalToCancel.value = deposit
  cancelCapitalReason.value = ''
  showCancelCapitalModal.value = true
}

async function handleExecuteCancelCapital() {
  if (!selectedCapitalToCancel.value) return
  if (!cancelCapitalReason.value.trim() || cancelCapitalReason.value.trim().length < 3) {
    showToast('Harap cantumkan alasan pembatalan (minimal 3 karakter).', 'error')
    return
  }

  isSubmittingCancelCapital.value = true
  try {
    const res = await api.post(`/capital/${selectedCapitalToCancel.value.id}/cancel`, {
      reason: cancelCapitalReason.value.trim(),
    })
    if (res.data?.success) {
      showToast('Setoran modal berhasil dibatalkan dan jurnal pembalik dibukukan.', 'success')
      showCancelCapitalModal.value = false
      await fetchCapitalDeposits()
      await fetchCapitalSummary()
      loadedTabs.value.balance_sheet = false
      loadedTabs.value.journal = false
    }
  } catch (err: any) {
    showToast(err.response?.data?.message || err.message || 'Gagal membatalkan setoran modal.', 'error')
  } finally {
    isSubmittingCancelCapital.value = false
  }
}

function openCapitalVoucher(deposit: CapitalDepositDetail) {
  selectedCapitalForVoucher.value = deposit
  showCapitalVoucherModal.value = true
}

function printCapitalVoucher() {
  window.print()
}

// =========================================================================
// PEMBAGIAN DIVIDEN (LABA DITAHAN) STATE & HANDLERS
// =========================================================================
const dividendList = ref<DividendDistribution[]>([])
const dividendSummary = ref<DividendSummary>({
  total_retained_earnings: 0,
  total_dividends_paid: 0,
  closed_periods_count: 0,
  eligible_closed_periods: [],
})
const dividendTotalItems = ref<number>(0)
const dividendPage = ref<number>(1)
const dividendPageSize = ref<number>(10)
const dividendSearch = ref<string>('')
const dividendFilterPeriodId = ref<string>('')
const isDividendLoading = ref<boolean>(false)
const showCreateDividendModal = ref<boolean>(false)
const isSubmittingDividend = ref<boolean>(false)

interface DividendRecipientFormItem {
  name: string
  share_percentage: number
  amount: number
  notes: string
}

const dividendForm = ref<{
  period_id: string
  distribution_date: string
  total_amount: number
  payment_account_id: string
  notes: string
  recipients: DividendRecipientFormItem[]
}>({
  period_id: '',
  distribution_date: todayStr,
  total_amount: 0,
  payment_account_id: '',
  notes: '',
  recipients: [
    {
      name: 'Pemegang Saham Utama',
      share_percentage: 100,
      amount: 0,
      notes: '',
    },
  ],
})

const showDetailDividendModal = ref<boolean>(false)
const selectedDividendDetail = ref<DividendDistribution | null>(null)
const showCancelDividendModal = ref<boolean>(false)
const dividendToCancel = ref<DividendDistribution | null>(null)
const isCancellingDividend = ref<boolean>(false)

const dividendPaymentAccounts = computed(() => {
  return chartOfAccounts.value
    .filter(
      (a: any) =>
        a.account_category_id === 1 &&
        !a.is_header &&
        a.is_active &&
        (a.code.startsWith('11') || a.code.startsWith('10') || (a.name && /kas|bank/i.test(a.name)))
    )
    .map((a: any) => ({
      id: a.id,
      name: `${a.code} - ${a.name}`,
      code: a.code,
    }))
})

const dividendPeriodOptions = computed(() => {
  return (dividendSummary.value?.eligible_closed_periods || []).map((p) => ({
    id: p.id,
    name: `${p.closing_number} (${formatDate(p.period_start)} s/d ${formatDate(p.period_end)})`,
    code: `Laba: ${formatCurrency(p.net_income)}`,
    net_income: p.net_income,
    total_dividends_drawn: p.total_dividends_drawn,
    remaining_income: p.remaining_income,
  }))
})

const selectedClosedPeriod = computed(() => {
  if (!dividendForm.value.period_id) return null
  return dividendSummary.value.eligible_closed_periods.find((p) => p.id === dividendForm.value.period_id) || null
})

const dividendRecipientsTotalAmount = computed(() => {
  return dividendForm.value.recipients.reduce((sum, r) => sum + Number(r.amount || 0), 0)
})

const dividendRecipientsTotalPercentage = computed(() => {
  return dividendForm.value.recipients.reduce((sum, r) => sum + Number(r.share_percentage || 0), 0)
})

const isDividendOverBalance = computed(() => {
  const currentAvailable = dividendSummary.value.total_retained_earnings || 0
  return dividendForm.value.total_amount > currentAvailable
})

async function fetchDividendSummary() {
  try {
    const res = await api.get('/dividends/summary')
    if (res.data?.success) {
      dividendSummary.value = res.data.data
    }
  } catch (err: any) {
    console.error('Error fetching dividend summary:', err)
  }
}

async function fetchDividendList() {
  isDividendLoading.value = true
  try {
    const params: Record<string, any> = {
      page: dividendPage.value,
      page_size: dividendPageSize.value,
    }
    if (dividendSearch.value.trim()) params.search = dividendSearch.value.trim()
    if (dividendFilterPeriodId.value) params.period_id = dividendFilterPeriodId.value

    const res = await api.get('/dividends', { params })
    if (res.data?.success) {
      dividendList.value = res.data.data
      dividendTotalItems.value = res.data.total_items ?? res.data.data.length
    }
  } catch (err: any) {
    console.error('Error fetching dividend list:', err)
    showToast(err.response?.data?.message || 'Gagal memuat daftar dividen', 'error')
  } finally {
    isDividendLoading.value = false
  }
}

function handleDividendSearch() {
  dividendPage.value = 1
  fetchDividendList()
}

async function openCreateDividendModal(defaultPeriodId?: string) {
  // Selalu muat data terbaru dari API saat modal dibuka
  await Promise.all([
    fetchDividendSummary(),
    fetchBankAccounts(),
  ])
  try {
    if (chartOfAccounts.value.length === 0) {
      const res = await api.get('/accounting/chart-of-accounts')
      if (res.data?.success) chartOfAccounts.value = res.data.data
    }
  } catch (err) {
    console.error('Error fetching chart of accounts:', err)
  }

  // Pick target period
  let periodId = defaultPeriodId || ''
  if (!periodId && dividendSummary.value.eligible_closed_periods.length > 0) {
    periodId = dividendSummary.value.eligible_closed_periods[0].id
  }

  // Pick default payment account (Kas Tunai atau Rekening Bank)
  const defaultPayAcc = businessPaymentAccountOptions.value[0]?.id || dividendPaymentAccounts.value[0]?.id || ''

  dividendForm.value = {
    period_id: periodId,
    distribution_date: todayStr,
    total_amount: 0,
    payment_account_id: defaultPayAcc,
    notes: '',
    recipients: [
      {
        name: 'Pemegang Saham Utama',
        share_percentage: 100,
        amount: 0,
        notes: 'Alokasi dividen laba periode',
      },
    ],
  }
  showCreateDividendModal.value = true
}

function addDividendRecipient() {
  dividendForm.value.recipients.push({
    name: '',
    share_percentage: 0,
    amount: 0,
    notes: '',
  })
}

function removeDividendRecipient(index: number) {
  if (dividendForm.value.recipients.length > 1) {
    dividendForm.value.recipients.splice(index, 1)
  }
}

function onDividendTotalAmountChange() {
  const total = Number(dividendForm.value.total_amount) || 0
  if (dividendForm.value.recipients.length === 1) {
    dividendForm.value.recipients[0].amount = total
    dividendForm.value.recipients[0].share_percentage = 100
    return
  }
  // Recalculate each recipient's amount from share_percentage
  for (const r of dividendForm.value.recipients) {
    const pct = Number(r.share_percentage) || 0
    r.amount = Math.round((total * pct) / 100)
  }
}

function onRecipientPercentageChange(idx: number) {
  const total = Number(dividendForm.value.total_amount) || 0
  const r = dividendForm.value.recipients[idx]
  if (r) {
    const pct = Number(r.share_percentage) || 0
    r.amount = Math.round((total * pct) / 100)
  }
}

function onRecipientAmountChange(idx: number) {
  const total = Number(dividendForm.value.total_amount) || 0
  const r = dividendForm.value.recipients[idx]
  if (r && total > 0) {
    const amt = Number(r.amount) || 0
    r.share_percentage = Number(((amt / total) * 100).toFixed(2))
  }
}

async function handleSaveDividend() {
  if (!authStore.hasPermission(['accounting:dividend:create', 'accounting:write'])) {
    showToast('Akses ditolak: Anda tidak memiliki wewenang membuat pembagian dividen', 'error')
    return
  }
  if (!dividendForm.value.period_id) {
    showToast('Pilih periode akuntansi yang telah ditutup buku', 'error')
    return
  }
  if (!dividendForm.value.distribution_date) {
    showToast('Tentukan tanggal pembagian dividen', 'error')
    return
  }
  if (!dividendForm.value.payment_account_id) {
    showToast('Pilih rekening kas/bank untuk pencairan dividen', 'error')
    return
  }
  const total = Number(dividendForm.value.total_amount) || 0
  if (total <= 0) {
    showToast('Nominal total dividen harus lebih besar dari Rp 0', 'error')
    return
  }
  const currentRetained = dividendSummary.value.total_retained_earnings || 0
  if (total > currentRetained) {
    showToast(
      `Saldo Laba Ditahan (3201) tidak mencukupi! Tersedia: ${formatCurrency(currentRetained)}, Diajukan: ${formatCurrency(total)}`,
      'error'
    )
    return
  }

  // Validate recipients
  for (let i = 0; i < dividendForm.value.recipients.length; i++) {
    const r = dividendForm.value.recipients[i]
    if (!r.name.trim()) {
      showToast(`Nama penerima/pemegang saham pada baris #${i + 1} wajib diisi`, 'error')
      return
    }
    if (Number(r.amount) <= 0) {
      showToast(`Nominal dividen untuk ${r.name} harus lebih besar dari Rp 0`, 'error')
      return
    }
  }

  // Check sum of recipient amounts matches total_amount
  const sumAmounts = dividendRecipientsTotalAmount.value
  if (Math.abs(sumAmounts - total) > 1) {
    showToast(
      `Total rincian penerima (${formatCurrency(sumAmounts)}) harus sama dengan total dividen (${formatCurrency(total)})`,
      'error'
    )
    return
  }

  isSubmittingDividend.value = true
  try {
    const primaryName = dividendForm.value.recipients[0]?.name.trim() || 'Pemegang Saham'
    const payload: CreateDividendRequest = {
      period_id: dividendForm.value.period_id,
      distribution_date: dividendForm.value.distribution_date,
      total_amount: total,
      payment_account_id: dividendForm.value.payment_account_id,
      recipient_name: primaryName,
      notes: dividendForm.value.notes.trim() || undefined,
      recipients: dividendForm.value.recipients.map((r) => ({
        name: r.name.trim(),
        share_percentage: Number(r.share_percentage) || 0,
        amount: Number(r.amount) || 0,
        notes: r.notes?.trim() || undefined,
      })),
    }

    const res = await api.post('/dividends', payload)
    if (res.data?.success) {
      showToast(res.data.message || 'Pembagian dividen berhasil dibukukan dan diposting ke jurnal', 'success')
      showCreateDividendModal.value = false
      await fetchDividendSummary()
      await fetchDividendList()
      // Invalidate balance sheet and journal caches
      loadedTabs.value.balance_sheet = false
      loadedTabs.value.journal = false
    }
  } catch (err: any) {
    showToast(err.response?.data?.message || err.message || 'Gagal memproses pembagian dividen', 'error')
  } finally {
    isSubmittingDividend.value = false
  }
}

function openDividendDetail(d: DividendDistribution) {
  selectedDividendDetail.value = d
  showDetailDividendModal.value = true
}

function openCancelDividendModal(d: DividendDistribution) {
  dividendToCancel.value = d
  showCancelDividendModal.value = true
}

async function handleConfirmCancelDividend() {
  if (!dividendToCancel.value) return
  if (!authStore.hasPermission(['accounting:dividend:delete', 'accounting:write'])) {
    showToast('Akses ditolak: Anda tidak memiliki wewenang membatalkan pembagian dividen', 'error')
    return
  }

  isCancellingDividend.value = true
  try {
    const res = await api.post(`/dividends/${dividendToCancel.value.id}/cancel`)
    if (res.data?.success) {
      showToast('Pembagian dividen berhasil dibatalkan dan jurnal telah dibalik', 'success')
      showCancelDividendModal.value = false
      dividendToCancel.value = null
      await fetchDividendSummary()
      await fetchDividendList()
      loadedTabs.value.balance_sheet = false
      loadedTabs.value.journal = false
    }
  } catch (err: any) {
    showToast(err.response?.data?.message || err.message || 'Gagal membatalkan pembagian dividen', 'error')
  } finally {
    isCancellingDividend.value = false
  }
}

function viewDividendJournal(journalEntryId?: string | null) {
  if (!journalEntryId) return
  showDetailDividendModal.value = false
  activeTab.value = 'journal'
  fetchDataForTab('journal', true)
}

function handlePrintVoucher() {
  window.print()
}

// =========================================================================
// REKENING BANK PENERIMAAN STATE & HANDLERS
// =========================================================================
const isBankLoading = ref<boolean>(false)
const bankSearch = ref<string>('')
const bankOutletFilter = ref<string>('')
const showBankModal = ref<boolean>(false)
const bankModalMode = ref<'create' | 'edit'>('create')
const editingBankId = ref<string | null>(null)
const isSubmittingBank = ref<boolean>(false)
const copiedBankId = ref<string | null>(null)

const popularBanks = [
  'BCA',
  'Bank Mandiri',
  'BRI',
  'BNI',
  'BSI',
  'CIMB Niaga',
  'Permata',
  'Danamon',
  'Bank Jago',
]

const bankForm = ref({
  bank_name: 'BCA',
  account_number: '',
  account_holder: '',
  branch: '',
  outlet_id: '',
  is_default: false,
  is_active: true,
  notes: '',
})

async function fetchBankAccounts() {
  isBankLoading.value = true
  try {
    const params: Record<string, any> = {}
    if (selectedOutletId.value) params.outlet_id = selectedOutletId.value
    const res = await api.get('/banks', { params })
    if (res.data?.success) {
      bankAccounts.value = res.data.data
    }
  } catch (err: any) {
    console.error('Error fetching bank accounts:', err)
    showToast(err.response?.data?.message || 'Gagal memuat daftar rekening bank', 'error')
  } finally {
    isBankLoading.value = false
  }
}

const filteredBankAccounts = computed(() => {
  return bankAccounts.value.filter(b => {
    const q = bankSearch.value.trim().toLowerCase()
    const matchSearch =
      !q ||
      b.bank_name.toLowerCase().includes(q) ||
      b.account_number.toLowerCase().includes(q) ||
      b.account_holder.toLowerCase().includes(q) ||
      (b.branch && b.branch.toLowerCase().includes(q)) ||
      (b.coa_code && b.coa_code.toLowerCase().includes(q))
    const matchOutlet = !bankOutletFilter.value || b.outlet_id === bankOutletFilter.value
    return matchSearch && matchOutlet
  })
})

function openCreateBankModal() {
  bankModalMode.value = 'create'
  editingBankId.value = null
  bankForm.value = {
    bank_name: 'BCA',
    account_number: '',
    account_holder: '',
    branch: '',
    outlet_id: selectedOutletId.value || '',
    is_default: bankAccounts.value.length === 0,
    is_active: true,
    notes: '',
  }
  showBankModal.value = true
}

function openEditBankModal(bank: BankAccount) {
  bankModalMode.value = 'edit'
  editingBankId.value = bank.id
  bankForm.value = {
    bank_name: bank.bank_name,
    account_number: bank.account_number,
    account_holder: bank.account_holder,
    branch: bank.branch || '',
    outlet_id: bank.outlet_id || '',
    is_default: bank.is_default,
    is_active: bank.is_active,
    notes: bank.notes || '',
  }
  showBankModal.value = true
}

async function saveBankAccount() {
  if (!bankForm.value.bank_name.trim()) {
    showToast('Nama bank wajib diisi', 'error')
    return
  }
  if (!bankForm.value.account_number.trim()) {
    showToast('Nomor rekening wajib diisi', 'error')
    return
  }
  if (!bankForm.value.account_holder.trim()) {
    showToast('Nama pemilik rekening (atas nama) wajib diisi', 'error')
    return
  }

  isSubmittingBank.value = true
  try {
    const payload = {
      bank_name: bankForm.value.bank_name.trim(),
      account_number: bankForm.value.account_number.trim(),
      account_holder: bankForm.value.account_holder.trim(),
      branch: bankForm.value.branch.trim() || undefined,
      outlet_id: bankForm.value.outlet_id || null,
      is_default: bankForm.value.is_default,
      is_active: bankForm.value.is_active,
      notes: bankForm.value.notes.trim() || undefined,
    }

    if (bankModalMode.value === 'create') {
      const res = await api.post('/banks', payload)
      if (res.data?.success) {
        showToast('Rekening bank penerimaan berhasil didaftarkan', 'success')
      }
    } else if (editingBankId.value) {
      const res = await api.put(`/banks/${editingBankId.value}`, payload)
      if (res.data?.success) {
        showToast('Rekening bank penerimaan berhasil diperbarui', 'success')
      }
    }

    showBankModal.value = false
    await fetchBankAccounts()
    loadedTabs.value.coa = false
    loadedTabs.value.capital = false
  } catch (err: any) {
    console.error('Error saving bank account:', err)
    showToast(err.response?.data?.message || 'Gagal menyimpan rekening bank', 'error')
  } finally {
    isSubmittingBank.value = false
  }
}

async function deleteBankAccount(bank: BankAccount) {
  const confirmMsg = `Hapus rekening bank ${bank.bank_name} (${bank.account_number}) - a.n. ${bank.account_holder}?`
  if (!confirm(confirmMsg)) return

  try {
    const res = await api.delete(`/banks/${bank.id}`)
    if (res.data?.success) {
      showToast('Rekening bank penerimaan berhasil dihapus', 'success')
      await fetchBankAccounts()
      loadedTabs.value.coa = false
      loadedTabs.value.capital = false
    }
  } catch (err: any) {
    console.error('Error deleting bank account:', err)
    showToast(
      err.response?.data?.message || 'Gagal menghapus rekening bank. Rekening mungkin sudah terhubung transaksi.',
      'error'
    )
  }
}

function copyAccountNumber(bank: BankAccount) {
  navigator.clipboard.writeText(bank.account_number)
  copiedBankId.value = bank.id
  showToast(`Nomor rekening ${bank.account_number} disalin ke clipboard`, 'success')
  setTimeout(() => {
    if (copiedBankId.value === bank.id) {
      copiedBankId.value = null
    }
  }, 2000)
}

// =========================================================================
// HANDLER: CHART OF ACCOUNTS (COA) CRUD
// =========================================================================
function openCreateCoaModal(prefillCategory?: 1 | 2 | 3 | 4 | 5) {
  coaModalMode.value = 'create'
  selectedCoaToEdit.value = null
  coaForm.value = {
    account_category_id: prefillCategory || 1,
    code: '',
    name: '',
    description: '',
    is_header: false,
    is_active: true,
  }
  showCoaModal.value = true
}

function openEditCoaModal(coa: any) {
  // === GUARD: Akun sistem tidak dapat diedit ===
  if (coa.is_system) {
    showToast(
      `"${coa.name}" adalah akun sistem bawaan yang terlindungi dan tidak dapat diubah.`,
      'error'
    )
    return
  }
  coaModalMode.value = 'edit'
  selectedCoaToEdit.value = coa
  coaForm.value = {
    account_category_id: coa.account_category_id,
    code: coa.code,
    name: coa.name,
    description: coa.description || '',
    is_header: coa.is_header,
    is_active: coa.is_active,
  }
  showCoaModal.value = true
}


async function handleSaveCoa() {
  if (!authStore.hasPermission(['accounting:write', 'master:write'])) {
    showToast('Akses ditolak: Anda tidak memiliki izin untuk mengelola akun keuangan.', 'error')
    return
  }
  if (!coaForm.value.code.trim()) {
    showToast('Kode akun wajib diisi.', 'error')
    return
  }
  if (!coaForm.value.name.trim()) {
    showToast('Nama akun wajib diisi.', 'error')
    return
  }

  isSubmittingCoa.value = true
  try {
    if (coaModalMode.value === 'create') {
      const payload = {
        account_category_id: Number(coaForm.value.account_category_id),
        code: coaForm.value.code.trim(),
        name: coaForm.value.name.trim(),
        description: coaForm.value.description.trim() || null,
        is_header: coaForm.value.is_header,
      }
      const res = await api.post('/accounting/chart-of-accounts', payload)
      if (res.data?.success) {
        showToast(`Akun '${res.data.data.code} - ${res.data.data.name}' berhasil ditambahkan.`, 'success')
        // Tambahkan ke list lokal tanpa reload penuh
        chartOfAccounts.value = [...chartOfAccounts.value, res.data.data].sort(
          (a: any, b: any) => a.code.localeCompare(b.code)
        )
        showCoaModal.value = false
        // Refresh capital accounts agar dropdown ikut terupdate
        await fetchCapitalAccounts()
      }
    } else {
      const payload = {
        name: coaForm.value.name.trim(),
        description: coaForm.value.description.trim() || null,
        is_active: coaForm.value.is_active,
      }
      const res = await api.put(
        `/accounting/chart-of-accounts/${selectedCoaToEdit.value.id}`,
        payload
      )
      if (res.data?.success) {
        showToast('Akun berhasil diperbarui.', 'success')
        // Update lokal
        const idx = chartOfAccounts.value.findIndex((c: any) => c.id === selectedCoaToEdit.value.id)
        if (idx !== -1) chartOfAccounts.value[idx] = res.data.data
        showCoaModal.value = false
        await fetchCapitalAccounts()
      }
    }
  } catch (err: any) {
    showToast(err.response?.data?.message || err.message || 'Gagal menyimpan akun.', 'error')
  } finally {
    isSubmittingCoa.value = false
  }
}


// Watch filters: jika filter tanggal / outlet berubah, update tab yang sedang dibuka
watch([selectedOutletId, startDate, endDate, asOfDate], () => {
  onFilterChange()
})

onMounted(async () => {
  await authStore.fetchCurrentUser().catch(() => {})
  storeBrandingStore.fetchBranding().catch(() => {})
  await fetchOutlets()
  await fetchDataForTab('income', true)
  // Pre-load essential accounting master data so print previews are instantaneous
  fetchDataForTab('balance_sheet').catch(() => {})
  fetchDataForTab('coa').catch(() => {})
})
</script>

<template>
  <AdminLayout>
    <div class="p-6 md:p-8 space-y-6 max-w-7xl mx-auto select-none">
      <!-- Toast Alert Notification -->
      <div
        v-if="toastMessage"
        class="fixed top-5 right-5 z-[99] px-4 py-3 rounded-xl shadow-2xl flex items-center gap-3 border text-xs font-bold animate-in fade-in slide-in-from-top-3 duration-200"
        :class="toastType === 'success' ? 'bg-emerald-900/90 text-white border-emerald-500' : 'bg-rose-900/90 text-white border-rose-500'"
      >
        <CheckCircle2 v-if="toastType === 'success'" class="w-4 h-4 text-emerald-400 shrink-0" />
        <AlertTriangle v-else class="w-4 h-4 text-rose-400 shrink-0" />
        <span>{{ toastMessage }}</span>
      </div>

      <!-- Header Banner & Title -->
      <div class="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h2 class="text-xl font-black text-slate-900 flex items-center gap-2.5">
            <Icon icon="streamline-ultimate-color:accounting-calculator-1" class="w-7 h-7 drop-shadow-xs" />
            <span>Pembukuan Akuntansi & Laporan Posisi Keuangan</span>
          </h2>
          <p class="text-xs text-slate-600 mt-1">
            Pencatatan akuntansi double-entry otomatis, laporan neraca & laba rugi per cabang, serta proteksi penutupan buku standar akuntansi.
          </p>
        </div>

        <!-- Global Action Buttons -->
        <div class="flex items-center gap-2.5">
          <button
            v-if="authStore.hasPermission(['capital:create', 'accounting:write'])"
            @click="openCapitalModal"
            class="inline-flex items-center gap-1.5 px-3.5 py-2 rounded-xl bg-emerald-700 hover:bg-emerald-600 text-white text-xs font-bold shadow-md transition active:scale-95 cursor-pointer"
          >
            <Coins class="w-3.5 h-3.5 text-amber-300" />
            <span>+ Setor Modal Usaha</span>
          </button>
          <button
            v-if="authStore.hasPermission('accounting:write')"
            @click="openManualJournalModal"
            class="inline-flex items-center gap-1.5 px-3.5 py-2 rounded-xl bg-white hover:bg-slate-50 text-slate-800 border border-slate-300 text-xs font-bold shadow-xs transition active:scale-95 cursor-pointer"
          >
            <Plus class="w-3.5 h-3.5 text-emerald-600" />
            <span>+ Jurnal Penyesuaian Manual</span>
          </button>
          <button
            v-if="authStore.hasPermission(['accounting:fund_transfer', 'accounting:write'])"
            @click="openFundTransferModal"
            class="inline-flex items-center gap-1.5 px-3.5 py-2 rounded-xl bg-white hover:bg-slate-50 text-slate-800 border border-slate-300 text-xs font-bold shadow-xs transition active:scale-95 cursor-pointer"
          >
            <ArrowLeftRight class="w-3.5 h-3.5 text-blue-600" />
            <span>+ Transfer Kas / Bank</span>
          </button>
          <button
            v-if="authStore.hasPermission(['accounting:close_period', 'accounting:write'])"
            @click="openClosingModal"
            class="inline-flex items-center gap-1.5 px-3.5 py-2 rounded-xl bg-slate-900 hover:bg-slate-800 text-white text-xs font-bold shadow-md transition active:scale-95 cursor-pointer"
          >
            <Lock class="w-3.5 h-3.5 text-amber-400" />
            <span>Tutup Buku Periode</span>
          </button>
          <button
            id="btn-cetak-laporan-utama"
            type="button"
            @click="openPrintReport()"
            class="inline-flex items-center gap-1.5 px-3.5 py-2 rounded-xl bg-slate-900 hover:bg-slate-800 text-white text-xs font-bold shadow-md transition active:scale-95 cursor-pointer"
            title="Cetak Seluruh Laporan Keuangan ke PDF Resmi"
          >
            <Printer class="w-3.5 h-3.5 text-emerald-400" />
            <span>Cetak Seluruh Laporan (PDF)</span>
          </button>
        </div>
      </div>

      <!-- Filter Bar: Outlet & Date Range Controls -->
      <div class="bg-white p-4 rounded-2xl border border-slate-200/90 shadow-xs flex flex-wrap items-center justify-between gap-4">
        <div class="flex flex-wrap items-center gap-3">
          <!-- Outlet Filter -->
          <div class="flex items-center gap-2 bg-slate-50 border border-slate-300 rounded-xl px-3 py-1.5 shadow-2xs min-w-[280px]">
            <Building2 class="w-4 h-4 text-emerald-700 shrink-0" />
            <span class="text-xs font-bold text-slate-700 shrink-0">Outlet / Cabang:</span>
            <div class="flex-1 min-w-[200px]">
              <SearchableSelect
                v-model="selectedOutletId"
                api-endpoint="/outlets"
                secondary-key="code"
                placeholder="Semua Outlet (Konsolidasian)"
                search-placeholder="Cari cabang..."
                :allow-clear="true"
                @change="onFilterChange"
              />
            </div>
          </div>

          <!-- Periode Mulai & Selesai (Untuk Laba Rugi, Jurnal, Tutup Buku) -->
          <div v-if="activeTab !== 'balance_sheet'" class="flex items-center gap-2 bg-slate-50 border border-slate-300 rounded-xl px-3 py-1.5 shadow-2xs text-xs">
            <Calendar class="w-3.5 h-3.5 text-slate-500 shrink-0" />
            <span class="font-bold text-slate-700">Periode:</span>
            <input
              type="date"
              v-model="startDate"
              class="bg-transparent font-mono font-bold text-slate-900 focus:outline-none"
            />
            <span class="text-slate-400 font-bold">s/d</span>
            <input
              type="date"
              v-model="endDate"
              class="bg-transparent font-mono font-bold text-slate-900 focus:outline-none"
            />
          </div>

          <!-- As-Of Date (Untuk Posisi Aset / Neraca) -->
          <div v-else class="flex items-center gap-2 bg-slate-50 border border-slate-300 rounded-xl px-3 py-1.5 shadow-2xs text-xs">
            <Scale class="w-3.5 h-3.5 text-emerald-600 shrink-0" />
            <span class="font-bold text-slate-700">Posisi Per Tanggal (Cut-off):</span>
            <input
              type="date"
              v-model="asOfDate"
              class="bg-transparent font-mono font-bold text-slate-900 focus:outline-none"
            />
          </div>

          <!-- Quick Date Presets -->
          <div class="flex items-center gap-1 text-[11px] font-bold text-slate-600">
            <button
              @click="setDatePreset('today')"
              class="px-2 py-1 rounded-lg bg-slate-100 hover:bg-slate-200 transition cursor-pointer"
            >
              Hari Ini
            </button>
            <button
              @click="setDatePreset('this_month')"
              class="px-2 py-1 rounded-lg bg-slate-100 hover:bg-slate-200 transition cursor-pointer"
            >
              Bulan Ini
            </button>
            <button
              @click="setDatePreset('last_month')"
              class="px-2 py-1 rounded-lg bg-slate-100 hover:bg-slate-200 transition cursor-pointer"
            >
              Bulan Lalu
            </button>
            <button
              @click="setDatePreset('this_year')"
              class="px-2 py-1 rounded-lg bg-slate-100 hover:bg-slate-200 transition cursor-pointer"
            >
              Tahun Ini
            </button>
          </div>
        </div>

        <div class="flex items-center gap-2">
          <button
            id="btn-cetak-pdf-toolbar"
            type="button"
            @click="openPrintReport()"
            class="inline-flex items-center gap-1.5 px-3.5 py-2 rounded-xl bg-slate-900 hover:bg-slate-800 text-white text-xs font-bold shadow-xs transition active:scale-95 cursor-pointer shrink-0"
            title="Cetak Laporan PDF Resmi"
          >
            <Printer class="w-3.5 h-3.5 text-emerald-400" />
            <span>Cetak PDF</span>
          </button>

          <button
            @click="refreshActiveTab"
            class="p-2 rounded-xl text-slate-600 hover:text-slate-900 hover:bg-slate-100 border border-slate-200 transition cursor-pointer"
            title="Segarkan Data Tab Aktif"
          >
            <RefreshCw class="w-4 h-4" :class="isLoading ? 'animate-spin text-emerald-600' : ''" />
          </button>
        </div>
      </div>

      <!-- Navigasi Modul Akuntansi (Kotak-Kotak Kompak Bergaya Menu Utama dengan Icon Berwarna) -->
      <div class="bg-slate-200/90 border border-slate-300 p-2 rounded-2xl shadow-inner">
        <div class="grid grid-cols-2 sm:grid-cols-4 lg:grid-cols-8 gap-2">
          <button
            v-for="tile in accountingMenuTiles"
            :key="tile.id"
            type="button"
            @click="switchTab(tile.id)"
            class="group flex flex-col items-center justify-center p-2 rounded-xl transition-all duration-150 relative border select-none h-[68px] text-center gap-1 cursor-pointer"
            :class="activeTab === tile.id
              ? 'bg-emerald-100 text-emerald-950 border-emerald-500 shadow-sm ring-2 ring-emerald-400/50 scale-[1.02]'
              : 'bg-white hover:bg-slate-50 text-slate-700 hover:text-slate-900 border-slate-300 shadow-xs hover:border-emerald-300 active:scale-95'"
            :title="tile.subtitle"
          >
            <!-- Badge count / status di pojok kanan atas -->
            <span
              v-if="tile.badge"
              class="absolute top-1 right-1 text-[8.5px] font-mono font-bold px-1 rounded-full border leading-none shadow-2xs"
              :class="activeTab === tile.id ? 'bg-emerald-800 text-emerald-100 border-emerald-600' : 'bg-slate-100 text-slate-600 border-slate-200'"
            >
              {{ tile.badge }}
            </span>

            <!-- Icon Berwarna (Streamline / Fluent Color) -->
            <Icon
              :icon="tile.iconName"
              class="w-6 h-6 transition-transform duration-150 group-hover:scale-110 drop-shadow-xs"
            />

            <!-- Judul Modul -->
            <span
              class="text-[10px] leading-tight tracking-tight font-bold whitespace-nowrap"
              :class="activeTab === tile.id ? 'text-emerald-950 font-black' : 'text-slate-700 group-hover:text-slate-900'"
            >
              {{ tile.title }}
            </span>

            <!-- Active Indicator Underline -->
            <div
              v-if="activeTab === tile.id"
              class="w-4 h-0.5 bg-emerald-600 rounded-full"
            ></div>
          </button>
        </div>
      </div>

      <!-- Loading State Saat Fetch On-Demand Tab Aktif -->
      <div v-if="isLoading" class="p-16 text-center text-slate-400 flex flex-col items-center gap-3 bg-white rounded-2xl border border-slate-200 shadow-xs">
        <div class="w-8 h-8 border-3 border-emerald-700 border-t-transparent rounded-full animate-spin"></div>
        <span class="text-xs font-bold text-slate-700">Memuat data akuntansi dari server...</span>
        <span class="text-[11px] text-slate-400">Hanya mengambil data yang diperlukan untuk tab aktif</span>
      </div>

      <template v-else>
        <!-- ===================================================================== -->
        <!-- TAB 1: LABA RUGI (INCOME STATEMENT) -->
        <!-- ===================================================================== -->
        <div v-if="activeTab === 'income'" class="space-y-5">
        <div v-if="incomeStatement" class="space-y-5">
          <!-- Summary Cards Grid -->
          <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
            <div class="bg-white border border-slate-200 p-5 rounded-2xl shadow-xs">
              <span class="text-xs font-bold text-slate-500 uppercase font-mono">Total Pendapatan</span>
              <div class="text-2xl font-black text-slate-900 font-mono mt-1">
                {{ formatCurrency(incomeStatement.total_revenue) }}
              </div>
              <div class="text-[10.5px] text-slate-500 mt-1">
                {{ incomeStatement.outlet_name || 'Semua Cabang' }}
              </div>
            </div>

            <div class="bg-white border border-slate-200 p-5 rounded-2xl shadow-xs">
              <span class="text-xs font-bold text-slate-500 uppercase font-mono">Beban Pokok Penjualan (HPP)</span>
              <div class="text-2xl font-black text-rose-700 font-mono mt-1">
                {{ formatCurrency(incomeStatement.total_cogs) }}
              </div>
              <div class="text-[10.5px] text-slate-500 mt-1">Biaya persediaan terjual</div>
            </div>

            <div class="bg-white border border-slate-200 p-5 rounded-2xl shadow-xs">
              <span class="text-xs font-bold text-slate-500 uppercase font-mono">Laba Kotor (Gross Profit)</span>
              <div class="text-2xl font-black text-sky-700 font-mono mt-1">
                {{ formatCurrency(incomeStatement.gross_profit) }}
              </div>
              <div class="text-[10.5px] text-slate-500 mt-1">Pendapatan dikurangi HPP</div>
            </div>

            <div class="bg-white border border-slate-200 p-5 rounded-2xl shadow-xs">
              <span class="text-xs font-bold text-slate-500 uppercase font-mono">Laba Bersih (Net Income)</span>
              <div
                class="text-2xl font-black font-mono mt-1"
                :class="Number(incomeStatement.net_profit) >= 0 ? 'text-emerald-700' : 'text-rose-700'"
              >
                {{ formatCurrency(incomeStatement.net_profit) }}
              </div>
              <div class="text-[10.5px] text-slate-500 mt-1">
                Status: {{ Number(incomeStatement.net_profit) >= 0 ? 'Surplus / Laba' : 'Defisit / Rugi' }}
              </div>
            </div>
          </div>

          <!-- Income Statement Breakdown Report -->
          <div class="bg-white border border-slate-200 rounded-2xl p-6 shadow-xs space-y-6">
            <div class="flex flex-wrap items-center justify-between border-b border-slate-200 pb-3 gap-3">
              <h3 class="font-black text-sm text-slate-900 flex items-center gap-2">
                <TrendingUp class="w-4 h-4 text-emerald-600" />
                <span>Rincian Laporan Laba Rugi</span>
              </h3>
              <div class="flex items-center gap-3">
                <div class="text-xs text-slate-500 font-mono">
                  Periode: {{ formatDate(incomeStatement.start_date) }} s/d {{ formatDate(incomeStatement.end_date) }}
                </div>
                <button
                  @click="openPrintReport('income')"
                  class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-slate-900 hover:bg-slate-800 text-white text-xs font-bold transition shadow-xs cursor-pointer"
                >
                  <Printer class="w-3.5 h-3.5 text-emerald-400" />
                  <span>Cetak Laporan (PDF)</span>
                </button>
              </div>
            </div>

            <!-- Revenue Section -->
            <div class="space-y-2">
              <div class="text-xs font-bold text-slate-800 uppercase tracking-wider font-mono flex items-center justify-between">
                <span>I. PENDAPATAN OPERASIONAL & PENJUALAN</span>
                <span class="text-emerald-800">{{ formatCurrency(incomeStatement.total_revenue) }}</span>
              </div>
              <div v-if="incomeStatement.revenue_details.length === 0" class="text-xs text-slate-400 italic py-2">
                Belum ada transaksi pendapatan pada periode ini.
              </div>
              <div
                v-for="rev in incomeStatement.revenue_details"
                :key="rev.code"
                class="flex justify-between text-xs py-2 px-3 bg-slate-50 border border-slate-200 rounded-xl font-mono hover:bg-slate-100 transition"
              >
                <span class="text-slate-800 font-sans font-medium">{{ rev.code }} - {{ rev.name }}</span>
                <span :class="Number(rev.amount) < 0 ? 'text-rose-600 font-bold' : 'text-emerald-700 font-bold'">
                  {{ formatCurrency(rev.amount) }}
                </span>
              </div>
            </div>

            <!-- Expenses Section -->
            <div class="space-y-2">
              <div class="text-xs font-bold text-slate-800 uppercase tracking-wider font-mono flex items-center justify-between">
                <span>II. BEBAN OPERASIONAL & HARGA POKOK PENJUALAN</span>
                <span class="text-rose-700">{{ formatCurrency(Number(incomeStatement.total_cogs) + Number(incomeStatement.total_operating_expenses)) }}</span>
              </div>
              <div v-if="incomeStatement.expense_details.length === 0" class="text-xs text-slate-400 italic py-2">
                Belum ada beban tercatat pada periode ini.
              </div>
              <div
                v-for="exp in incomeStatement.expense_details"
                :key="exp.code"
                class="flex justify-between text-xs py-2 px-3 bg-slate-50 border border-slate-200 rounded-xl font-mono hover:bg-slate-100 transition"
              >
                <span class="text-slate-800 font-sans font-medium">{{ exp.code }} - {{ exp.name }}</span>
                <span class="text-rose-700 font-bold">{{ formatCurrency(exp.amount) }}</span>
              </div>
            </div>

            <!-- Net Result Summary Footer -->
            <div class="pt-4 border-t-2 border-slate-200 flex items-center justify-between font-mono font-bold text-sm bg-emerald-50/50 p-4 rounded-xl border">
              <span class="text-slate-900 uppercase">LABA (RUGI) BERSIH TAHUN BERJALAN:</span>
              <span :class="Number(incomeStatement.net_profit) >= 0 ? 'text-emerald-800 text-lg' : 'text-rose-800 text-lg'">
                {{ formatCurrency(incomeStatement.net_profit) }}
              </span>
            </div>
          </div>
        </div>
      </div>

      <!-- ===================================================================== -->
      <!-- TAB 2: POSISI ASET & NERACA (BALANCE SHEET) -->
      <!-- ===================================================================== -->
      <div v-else-if="activeTab === 'balance_sheet'" class="space-y-5">
        <div v-if="balanceSheet" class="space-y-5">
          <!-- Summary Cards Grid -->
          <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
            <div class="bg-white border border-slate-200 p-5 rounded-2xl shadow-xs">
              <div class="flex items-center justify-between">
                <span class="text-xs font-bold text-slate-500 uppercase font-mono">Total Aset / Aktiva</span>
                <Icon icon="fluent-color:arrow-sync-24" class="w-5 h-5 text-emerald-600" />
              </div>
              <div class="text-2xl font-black text-emerald-800 font-mono mt-1">
                {{ formatCurrency(balanceSheet.total_assets) }}
              </div>
              <div class="text-[10.5px] text-slate-500 mt-1">
                Lancar: {{ formatCurrency(balanceSheet.current_assets) }}
                <span v-if="Number(balanceSheet.non_current_assets) !== 0">
                  | Tidak Lancar / Antar-Kantor: {{ formatCurrency(balanceSheet.non_current_assets) }}
                </span>
              </div>
            </div>

            <div class="bg-white border border-slate-200 p-5 rounded-2xl shadow-xs">
              <div class="flex items-center justify-between">
                <span class="text-xs font-bold text-slate-500 uppercase font-mono">Total Kewajiban</span>
                <Icon icon="streamline-ultimate-color:task-list-approve" class="w-5 h-5" />
              </div>
              <div class="text-2xl font-black text-amber-800 font-mono mt-1">
                {{ formatCurrency(balanceSheet.total_liabilities) }}
              </div>
              <div class="text-[10.5px] text-slate-500 mt-1">Hutang usaha & pajak</div>
            </div>

            <div class="bg-white border border-slate-200 p-5 rounded-2xl shadow-xs">
              <div class="flex items-center justify-between">
                <span class="text-xs font-bold text-slate-500 uppercase font-mono">Total Ekuitas / Modal</span>
                <Scale class="w-5 h-5 text-purple-600" />
              </div>
              <div class="text-2xl font-black text-purple-800 font-mono mt-1">
                {{ formatCurrency(balanceSheet.total_equity) }}
              </div>
              <div class="text-[10.5px] text-slate-500 mt-1">Termasuk laba periode & penempatan cabang</div>
            </div>

            <div class="bg-white border border-slate-200 p-5 rounded-2xl shadow-xs">
              <div class="flex items-center justify-between">
                <span class="text-xs font-bold text-slate-500 uppercase font-mono">Status Keseimbangan</span>
                <span
                  class="text-[10px] font-mono px-2 py-0.5 rounded-full font-bold"
                  :class="balanceSheet.is_balanced ? 'bg-emerald-100 text-emerald-800 border border-emerald-300' : 'bg-rose-100 text-rose-800 border border-rose-300'"
                >
                  {{ balanceSheet.is_balanced ? 'Balanced ✓' : 'Unbalanced !' }}
                </span>
              </div>
              <div class="text-xs font-bold text-slate-700 mt-2">
                Pasiva: <span class="font-mono text-slate-900">{{ formatCurrency(balanceSheet.total_liabilities_and_equity) }}</span>
              </div>
              <div class="text-[10.5px] text-slate-500 mt-0.5">
                Persamaan Akuntansi: Aktiva = Kewajiban + Ekuitas
              </div>
            </div>
          </div>

          <!-- Action Bar: Print & Cut-off info -->
          <div class="bg-white border border-slate-200 rounded-2xl p-4 shadow-xs flex flex-wrap items-center justify-between gap-3">
            <div class="flex items-center gap-2">
              <Scale class="w-4 h-4 text-emerald-600" />
              <span class="text-xs font-bold text-slate-800">Laporan Neraca & Posisi Aset (Balance Sheet)</span>
              <span class="text-xs text-slate-400 font-mono">Cut-off: {{ formatDate(asOfDate) }}</span>
            </div>
            <button
              @click="openPrintReport('balance_sheet')"
              class="inline-flex items-center gap-1.5 px-3.5 py-2 rounded-xl bg-slate-900 hover:bg-slate-800 text-white text-xs font-bold transition shadow-xs cursor-pointer"
            >
              <Printer class="w-3.5 h-3.5 text-emerald-400" />
              <span>Cetak Laporan Neraca (PDF)</span>
            </button>
          </div>

          <!-- Two-Column Balance Sheet (Aktiva vs Pasiva) -->
          <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
            <!-- Left Column: AKTIVA (ASSETS) -->
            <div class="bg-white border border-slate-200 rounded-2xl p-6 shadow-xs space-y-4">
              <div class="flex items-center justify-between border-b border-slate-200 pb-3">
                <h3 class="font-black text-sm text-slate-900 flex items-center gap-2">
                  <Icon icon="fluent-color:arrow-sync-24" class="w-4 h-4" />
                  <span>AKTIVA / POSISI ASET</span>
                </h3>
                <span class="text-xs font-mono font-bold text-emerald-800">
                  Total: {{ formatCurrency(balanceSheet.total_assets) }}
                </span>
              </div>

              <!-- Current Assets -->
              <div class="space-y-2">
                <div class="text-xs font-bold text-slate-700 uppercase font-mono tracking-wider flex items-center justify-between">
                  <span>A. ASET LANCAR (CURRENT ASSETS)</span>
                  <span class="text-emerald-700">{{ formatCurrency(balanceSheet.current_assets) }}</span>
                </div>
                <div v-if="currentAssetDetails.length === 0" class="text-xs text-slate-400 italic py-2">
                  Tidak ada saldo akun aset lancar.
                </div>
                <div
                  v-for="ast in currentAssetDetails"
                  :key="ast.code"
                  class="flex justify-between text-xs py-2 px-3 bg-slate-50 border border-slate-200 rounded-xl font-mono hover:bg-slate-100 transition"
                >
                  <span class="text-slate-800 font-sans font-medium">{{ ast.code }} - {{ ast.name }}</span>
                  <span class="font-bold text-emerald-800">{{ formatCurrency(ast.amount) }}</span>
                </div>
              </div>

              <!-- Non-Current & Inter-Branch Assets (if any) -->
              <div v-if="nonCurrentAssetDetails.length > 0 || Number(balanceSheet.non_current_assets) !== 0" class="space-y-2 pt-2 border-t border-slate-100">
                <div class="text-xs font-bold text-slate-700 uppercase font-mono tracking-wider flex items-center justify-between">
                  <span>B. ASET TIDAK LANCAR & ANTAR-KANTOR</span>
                  <span class="text-emerald-700">{{ formatCurrency(balanceSheet.non_current_assets) }}</span>
                </div>
                <div
                  v-for="ast in nonCurrentAssetDetails"
                  :key="ast.code"
                  class="flex justify-between text-xs py-2 px-3 bg-slate-50 border border-slate-200 rounded-xl font-mono hover:bg-slate-100 transition"
                >
                  <span class="text-slate-800 font-sans font-medium">{{ ast.code }} - {{ ast.name }}</span>
                  <span class="font-bold text-emerald-800">{{ formatCurrency(ast.amount) }}</span>
                </div>
              </div>

              <div class="pt-3 border-t border-slate-200 flex justify-between font-mono font-bold text-xs bg-slate-100 p-3 rounded-xl">
                <span>TOTAL AKTIVA / ASET:</span>
                <span class="text-emerald-800 text-sm">{{ formatCurrency(balanceSheet.total_assets) }}</span>
              </div>
            </div>

            <!-- Right Column: PASIVA (LIABILITIES & EQUITY) -->
            <div class="bg-white border border-slate-200 rounded-2xl p-6 shadow-xs space-y-4">
              <div class="flex items-center justify-between border-b border-slate-200 pb-3">
                <h3 class="font-black text-sm text-slate-900 flex items-center gap-2">
                  <Scale class="w-4 h-4 text-purple-600" />
                  <span>PASIVA (KEWAJIBAN & EKUITAS)</span>
                </h3>
                <span class="text-xs font-mono font-bold text-purple-800">
                  Total: {{ formatCurrency(balanceSheet.total_liabilities_and_equity) }}
                </span>
              </div>

              <!-- Liabilities Section -->
              <div class="space-y-2">
                <div class="text-xs font-bold text-slate-700 uppercase font-mono tracking-wider flex items-center justify-between">
                  <span>A. KEWAJIBAN / HUTANG (LIABILITIES)</span>
                  <span class="text-amber-700">{{ formatCurrency(balanceSheet.total_liabilities) }}</span>
                </div>
                <div v-if="balanceSheet.liability_details.length === 0" class="text-xs text-slate-400 italic py-1">
                  Tidak ada kewajiban / hutang aktif.
                </div>
                <div
                  v-for="lia in balanceSheet.liability_details"
                  :key="lia.code"
                  class="flex justify-between text-xs py-2 px-3 bg-slate-50 border border-slate-200 rounded-xl font-mono hover:bg-slate-100 transition"
                >
                  <span class="text-slate-800 font-sans font-medium">{{ lia.code }} - {{ lia.name }}</span>
                  <span class="font-bold text-amber-800">{{ formatCurrency(lia.amount) }}</span>
                </div>
              </div>

              <!-- Equity Section -->
              <div class="space-y-2 pt-2">
                <div class="text-xs font-bold text-slate-700 uppercase font-mono tracking-wider flex items-center justify-between">
                  <span>B. EKUITAS / MODAL (EQUITY)</span>
                  <span class="text-purple-700">{{ formatCurrency(balanceSheet.total_equity) }}</span>
                </div>
                <div v-if="balanceSheet.equity_details.length === 0" class="text-xs text-slate-400 italic py-1">
                  Tidak ada modal / ekuitas tercatat.
                </div>
                <div
                  v-for="eq in balanceSheet.equity_details"
                  :key="eq.code"
                  class="flex justify-between text-xs py-2 px-3 bg-slate-50 border border-slate-200 rounded-xl font-mono hover:bg-slate-100 transition"
                >
                  <span class="text-slate-800 font-sans font-medium">{{ eq.code }} - {{ eq.name }}</span>
                  <span :class="Number(eq.amount) >= 0 ? 'text-purple-800 font-bold' : 'text-rose-700 font-bold'">
                    {{ formatCurrency(eq.amount) }}
                  </span>
                </div>
              </div>

              <div class="pt-3 border-t border-slate-200 flex justify-between font-mono font-bold text-xs bg-slate-100 p-3 rounded-xl">
                <span>TOTAL PASIVA (KEWAJIBAN + EKUITAS):</span>
                <span class="text-purple-800 text-sm">{{ formatCurrency(balanceSheet.total_liabilities_and_equity) }}</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- ===================================================================== -->
      <!-- TAB 3: BUKU JURNAL UMUM (LEDGER) -->
      <!-- ===================================================================== -->
      <div v-else-if="activeTab === 'journal'" class="space-y-4">
        <!-- Search and Action Bar -->
        <div class="bg-white border border-slate-200 rounded-2xl p-4 shadow-xs flex flex-wrap items-center justify-between gap-3">
          <div class="flex items-center gap-2 flex-1 min-w-[260px] max-w-md">
            <div class="relative w-full">
              <Search class="w-4 h-4 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2" />
              <input
                v-model="jrnSearch"
                type="text"
                placeholder="Cari no. jurnal, deskripsi, referensi, atau cabang..."
                class="w-full pl-9 pr-8 py-2 rounded-xl border border-slate-200 text-xs focus:ring-2 focus:ring-emerald-700/20 focus:border-emerald-700 focus:outline-none"
                @keyup.enter="handleJrnSearch"
              />
              <button
                v-if="jrnSearch"
                @click="jrnSearch = ''; handleJrnSearch()"
                class="absolute right-2.5 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-600"
              >
                <X class="w-3.5 h-3.5" />
              </button>
            </div>
            <button
              @click="handleJrnSearch"
              class="px-3.5 py-2 bg-slate-100 hover:bg-slate-200 text-slate-700 rounded-xl text-xs font-bold transition cursor-pointer flex items-center gap-1.5 shrink-0"
            >
              <Search class="w-3.5 h-3.5" />
              <span>Cari</span>
            </button>
          </div>

          <div class="flex items-center gap-2.5">
            <span class="text-xs text-slate-500 font-mono">
              Total: <strong class="text-slate-800">{{ jrnTotalItems }}</strong> transaksi
            </span>
            <button
              @click="openPrintReport('journal')"
              class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-slate-900 hover:bg-slate-800 text-white text-xs font-bold transition shadow-xs cursor-pointer"
            >
              <Printer class="w-3.5 h-3.5 text-emerald-400" />
              <span>Cetak Jurnal (PDF)</span>
            </button>
            <button
              @click="fetchJournalEntries"
              class="p-2 rounded-xl border border-slate-200 hover:bg-slate-100 text-slate-600 transition cursor-pointer"
              title="Segarkan data jurnal"
            >
              <RefreshCw class="w-4 h-4" />
            </button>
          </div>
        </div>

        <div class="bg-white border border-slate-200 rounded-2xl overflow-hidden shadow-xs">
          <table class="w-full text-left text-xs border-collapse">
            <thead class="bg-slate-100 border-b border-slate-200 text-slate-700 font-mono text-[11px] uppercase tracking-wider select-none">
              <tr>
                <th
                  @click="handleJrnSort('journal_date')"
                  class="py-3.5 px-4 cursor-pointer hover:bg-slate-200/70 transition"
                  title="Klik untuk mengurutkan tanggal transaksi"
                >
                  <div class="flex items-center gap-1.5">
                    <span>No. Jurnal & Waktu</span>
                    <span v-if="jrnSortBy === 'journal_date'" class="text-emerald-700 font-bold">
                      {{ jrnSortDir === 'asc' ? '↑' : '↓' }}
                    </span>
                  </div>
                </th>
                <th
                  @click="handleJrnSort('reference_type')"
                  class="py-3.5 px-4 cursor-pointer hover:bg-slate-200/70 transition"
                  title="Klik untuk mengurutkan referensi transaksi"
                >
                  <div class="flex items-center gap-1.5">
                    <span>Referensi & Outlet</span>
                    <span v-if="jrnSortBy === 'reference_type'" class="text-emerald-700 font-bold">
                      {{ jrnSortDir === 'asc' ? '↑' : '↓' }}
                    </span>
                  </div>
                </th>
                <th
                  @click="handleJrnSort('description')"
                  class="py-3.5 px-4 cursor-pointer hover:bg-slate-200/70 transition"
                  title="Klik untuk mengurutkan deskripsi transaksi"
                >
                  <div class="flex items-center gap-1.5">
                    <span>Deskripsi Transaksi</span>
                    <span v-if="jrnSortBy === 'description'" class="text-emerald-700 font-bold">
                      {{ jrnSortDir === 'asc' ? '↑' : '↓' }}
                    </span>
                  </div>
                </th>
                <th
                  @click="handleJrnSort('total_debit')"
                  class="py-3.5 px-4 text-right cursor-pointer hover:bg-slate-200/70 transition"
                  title="Klik untuk mengurutkan nilai total debet"
                >
                  <div class="flex items-center justify-end gap-1.5">
                    <span>Total Debet</span>
                    <span v-if="jrnSortBy === 'total_debit'" class="text-emerald-700 font-bold">
                      {{ jrnSortDir === 'asc' ? '↑' : '↓' }}
                    </span>
                  </div>
                </th>
                <th
                  @click="handleJrnSort('total_credit')"
                  class="py-3.5 px-4 text-right cursor-pointer hover:bg-slate-200/70 transition"
                  title="Klik untuk mengurutkan nilai total kredit"
                >
                  <div class="flex items-center justify-end gap-1.5">
                    <span>Total Kredit</span>
                    <span v-if="jrnSortBy === 'total_credit'" class="text-emerald-700 font-bold">
                      {{ jrnSortDir === 'asc' ? '↑' : '↓' }}
                    </span>
                  </div>
                </th>
                <th class="py-3.5 px-4 text-center">Status</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-200">
              <tr v-if="journalEntries.length === 0">
                <td colspan="6" class="py-12 text-center text-slate-400">
                  Tidak ada transaksi jurnal pada filter yang dipilih.
                </td>
              </tr>
              <template v-for="jrn in journalEntries" :key="jrn.entry.id">
                <tr class="bg-slate-50/70 font-mono font-bold hover:bg-slate-100/60 transition">
                  <td class="py-3 px-4 text-slate-900">
                    <div class="font-extrabold text-sky-800">
                      <CopyableText :text="jrn.entry.journal_number" label="Nomor Jurnal" />
                    </div>
                    <div class="text-[10px] text-slate-500 font-normal">{{ formatDate(jrn.entry.journal_date) }}</div>
                  </td>
                  <td class="py-3 px-4">
                    <div class="flex items-center gap-1.5 flex-wrap">
                      <span
                        class="px-2 py-0.5 rounded-md text-[10px] font-bold border"
                        :class="jrn.entry.reference_type === 'CLOSING_ENTRY' ? 'bg-amber-100 text-amber-900 border-amber-300' : 'bg-sky-50 text-sky-800 border-sky-200'"
                      >
                        {{ jrn.entry.reference_type }}
                      </span>
                      <span
                        v-if="jrn.entry.is_closing_entry"
                        class="px-2 py-0.5 rounded-md text-[10px] font-bold border bg-purple-100 text-purple-900 border-purple-300"
                      >
                        Tutup Buku
                      </span>
                    </div>
                  </td>
                  <td class="py-3 px-4 font-sans text-slate-800">{{ jrn.entry.description }}</td>
                  <td class="py-3 px-4 text-right text-emerald-700 font-bold">{{ formatCurrency(jrn.entry.total_debit) }}</td>
                  <td class="py-3 px-4 text-right text-emerald-700 font-bold">{{ formatCurrency(jrn.entry.total_credit) }}</td>
                  <td class="py-3 px-4 text-center">
                    <span class="bg-emerald-50 text-emerald-800 text-[10px] px-2 py-0.5 rounded-full border border-emerald-300 font-bold">
                      Balanced ✓
                    </span>
                  </td>
                </tr>

                <!-- Nested Journal Lines with Dimensional Branch Indicator -->
                <tr v-for="line in jrn.lines" :key="line.id" class="bg-white text-[11px] font-mono text-slate-600 hover:bg-slate-50/40">
                  <td class="py-1.5 px-4 pl-8" colspan="2">
                    <div class="flex items-center gap-2 flex-wrap">
                      <span class="text-slate-800 font-medium">{{ line.account_code }} - {{ line.account_name }}</span>
                      <span v-if="line.outlet_name" class="inline-flex items-center gap-1 text-[10px] bg-slate-100 border border-slate-300 text-slate-700 px-1.5 py-0.5 rounded font-sans font-medium">
                        <Building2 class="w-2.5 h-2.5 text-slate-500" />
                        {{ line.outlet_name }}
                      </span>
                    </div>
                  </td>
                  <td class="py-1.5 px-4 font-sans italic text-slate-500">{{ line.memo || '-' }}</td>
                  <td class="py-1.5 px-4 text-right text-slate-700">{{ Number(line.debit) > 0 ? formatCurrency(line.debit) : '-' }}</td>
                  <td class="py-1.5 px-4 text-right text-slate-700">{{ Number(line.credit) > 0 ? formatCurrency(line.credit) : '-' }}</td>
                  <td></td>
                </tr>
              </template>
            </tbody>
          </table>
          <PaginationBar
            v-model:currentPage="jrnPage"
            v-model:pageSize="jrnPageSize"
            :totalItems="jrnTotalItems"
            @update:currentPage="fetchJournalEntries"
            @update:pageSize="() => { jrnPage = 1; fetchJournalEntries(); }"
          />
        </div>
      </div>

      <!-- ===================================================================== -->
      <!-- TAB 4: SISTEM TUTUP BUKU (PERIOD CLOSING) -->
      <!-- ===================================================================== -->
      <div v-else-if="activeTab === 'closing'" class="space-y-6">
        <!-- Accounting Standard Guidelines Card -->
        <div class="bg-gradient-to-r from-amber-50 to-orange-50 border border-amber-200 rounded-2xl p-5 text-amber-950 flex items-start gap-4 shadow-2xs">
          <div class="p-2.5 rounded-xl bg-amber-500 text-white shrink-0 mt-0.5">
            <Lock class="w-5 h-5" />
          </div>
          <div class="space-y-1.5 text-xs">
            <h4 class="font-extrabold text-sm text-amber-900">
              Kaidah Akuntansi Standar Penutupan Buku (Period End Closing) & Proteksi Periode Tertutup
            </h4>
            <ul class="list-disc list-inside space-y-1 text-slate-700 leading-relaxed">
              <li>
                <strong>Penutupan Akun Nominal:</strong> Seluruh saldo akun Pendapatan (Revenue) dan Beban (Expenses/HPP) ditutup ke akun <em>Ikhtisar Laba Rugi</em>, dan selisih laba/rugi bersih otomatis dipindahkan ke akun <em>Laba Ditahan (Retained Earnings, 3201)</em> melalui jurnal penutup resmi.
              </li>
              <li>
                <strong>Penguncian Transaksi:</strong> Setelah periode ditutup buku, transaksi pada periode tersebut <strong>dikunci permanen</strong> dan tidak dapat diubah atau dihapus agar laporan keuangan tetap akurat dan konsisten.
              </li>
              <li>
                <strong>Penyesuaian Standar:</strong> Sesuai PSAK/SAK, penyesuaian untuk masa lampau yang sudah tutup buku wajib dicatat pada <em>periode berjalan terbuka</em> via <strong>Jurnal Penyesuaian Manual</strong> (Prior Period Adjustment), atau jika sangat mendesak dapat dilakukan <em>Buka Kembali Buku (Reopen)</em> dengan alasan audit tertulis.
              </li>
            </ul>
          </div>
        </div>

        <!-- Sub-Tab Switcher & Quick Actions -->
        <div class="flex flex-wrap items-center justify-between gap-3 p-2 bg-slate-100/90 rounded-2xl border border-slate-200/90">
          <div class="flex items-center gap-1.5 p-1 bg-slate-200/70 rounded-xl">
            <button
              type="button"
              @click="switchClosingSubTab('periods')"
              :class="[
                'flex items-center gap-2 px-4 py-2 rounded-lg text-xs font-bold transition-all cursor-pointer shadow-xs',
                closingSubTab === 'periods'
                  ? 'bg-white text-emerald-900 shadow-sm ring-1 ring-emerald-500/20'
                  : 'text-slate-600 hover:text-slate-900 hover:bg-white/50'
              ]"
            >
              <Lock class="w-4 h-4 text-emerald-600" />
              <span>Periode Akuntansi (Soft Close)</span>
              <span
                class="px-2 py-0.5 rounded-full text-[10px] font-bold"
                :class="closingSubTab === 'periods' ? 'bg-emerald-100 text-emerald-800' : 'bg-slate-200 text-slate-600'"
              >
                {{ periodTotalItems }}
              </span>
            </button>
            <button
              type="button"
              @click="switchClosingSubTab('closings')"
              :class="[
                'flex items-center gap-2 px-4 py-2 rounded-lg text-xs font-bold transition-all cursor-pointer shadow-xs',
                closingSubTab === 'closings'
                  ? 'bg-white text-indigo-900 shadow-sm ring-1 ring-indigo-500/20'
                  : 'text-slate-600 hover:text-slate-900 hover:bg-white/50'
              ]"
            >
              <FileText class="w-4 h-4 text-indigo-600" />
              <span>Histori Tutup Buku (Hard Close)</span>
              <span
                class="px-2 py-0.5 rounded-full text-[10px] font-bold"
                :class="closingSubTab === 'closings' ? 'bg-indigo-100 text-indigo-900' : 'bg-slate-200 text-slate-600'"
              >
                {{ closingTotalItems }}
              </span>
            </button>
          </div>

          <!-- Header Quick Actions Sesuai Sub-Tab Aktif -->
          <div class="flex items-center gap-2 flex-wrap">
            <template v-if="closingSubTab === 'periods'">
              <button
                @click="openPrintReport('closing')"
                class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-slate-900 hover:bg-slate-800 text-white text-xs font-bold transition shadow-xs cursor-pointer"
              >
                <Printer class="w-3.5 h-3.5 text-emerald-400" />
                <span>Cetak Riwayat (PDF)</span>
              </button>
              <button
                v-if="authStore.hasPermission(['accounting:periods:close', 'accounting:close_period', 'accounting:write'])"
                @click="openCreatePeriodModal"
                class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-emerald-600 hover:bg-emerald-700 text-white text-xs font-bold transition shadow-xs cursor-pointer"
              >
                <Plus class="w-3.5 h-3.5" />
                <span>Tambah Periode Baru</span>
              </button>
              <button
                v-if="authStore.hasPermission(['accounting:hard_close', 'accounting:close_period', 'accounting:write'])"
                @click="openHardCloseModal"
                class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-purple-700 hover:bg-purple-800 text-white text-xs font-bold transition shadow-xs cursor-pointer"
                title="Tutup Buku Akhir Tahun: Memindahkan saldo nominal ke Laba Ditahan (3201)"
              >
                <Scale class="w-3.5 h-3.5" />
                <span>Tutup Buku Akhir Tahun</span>
              </button>
            </template>
            <template v-else>
              <button
                @click="openPrintReport('closing')"
                class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-slate-900 hover:bg-slate-800 text-white text-xs font-bold transition shadow-xs cursor-pointer"
              >
                <Printer class="w-3.5 h-3.5 text-indigo-400" />
                <span>Cetak Riwayat (PDF)</span>
              </button>
              <button
                v-if="authStore.hasPermission(['accounting:close_period', 'accounting:write'])"
                @click="openClosingModal"
                class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-indigo-600 hover:bg-indigo-700 text-white text-xs font-bold transition shadow-xs cursor-pointer"
              >
                <Plus class="w-3.5 h-3.5" />
                <span>Tutup Buku Baru</span>
              </button>
            </template>
          </div>
        </div>

        <!-- ================================================================= -->
        <!-- SUB-TAB 1: PERIODE AKUNTANSI & KONTROL SOFT CLOSE -->
        <!-- ================================================================= -->
        <div v-if="closingSubTab === 'periods'" class="bg-white border border-slate-200 rounded-2xl overflow-hidden shadow-xs">
          <!-- Toolbar Filter Server-Side -->
          <div class="p-4 bg-slate-50 border-b border-slate-200 flex flex-wrap items-center justify-between gap-3">
            <div class="flex items-center gap-2.5">
              <div class="p-1.5 rounded-lg bg-emerald-100 text-emerald-800">
                <Lock class="w-4 h-4" />
              </div>
              <div>
                <h3 class="font-bold text-xs uppercase tracking-wider text-slate-800 font-mono">
                  Daftar Periode Akuntansi (Soft Close Control)
                </h3>
                <p class="text-[11px] text-slate-500 font-sans">
                  Kunci periode mencegah pencatatan transaksi mutasi susulan (back-dated) di seluruh modul toko.
                </p>
              </div>
            </div>

            <!-- Search, Status Filter & Refresh -->
            <div class="flex items-center gap-2 flex-wrap">
              <div class="relative">
                <Search class="w-3.5 h-3.5 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2" />
                <input
                  v-model="periodSearch"
                  type="text"
                  placeholder="Cari nama periode/alasan..."
                  @input="onPeriodSearchInput"
                  @keyup.enter="periodPage = 1; fetchAccountingPeriods()"
                  class="pl-8 pr-3 py-1.5 text-xs border border-slate-300 rounded-lg bg-white text-slate-800 placeholder-slate-400 focus:outline-none focus:ring-1 focus:ring-emerald-500 w-56"
                />
              </div>

              <select
                v-model="periodStatus"
                @change="periodPage = 1; fetchAccountingPeriods()"
                class="text-xs border border-slate-300 rounded-lg bg-white text-slate-700 px-3 py-1.5 focus:outline-none focus:ring-1 focus:ring-emerald-500 cursor-pointer"
              >
                <option value="">Semua Status</option>
                <option value="OPEN">Terbuka (Open)</option>
                <option value="CLOSED">Terkunci (Closed)</option>
              </select>

              <button
                type="button"
                @click="periodSearch = ''; periodStatus = ''; periodPage = 1; fetchAccountingPeriods()"
                class="p-1.5 text-slate-500 hover:text-slate-800 hover:bg-slate-200/70 rounded-lg transition cursor-pointer"
                title="Reset Filter"
              >
                <RefreshCw class="w-4 h-4" :class="{ 'animate-spin': isPeriodLoading }" />
              </button>
            </div>
          </div>

          <!-- Loading State -->
          <div v-if="isPeriodLoading" class="py-12 text-center text-slate-500 font-sans flex flex-col items-center justify-center gap-2">
            <RefreshCw class="w-6 h-6 animate-spin text-emerald-600" />
            <span class="text-xs font-medium">Memuat data periode akuntansi...</span>
          </div>

          <!-- Table Content -->
          <div v-else class="overflow-x-auto">
            <table class="w-full text-left text-xs border-collapse">
              <thead class="bg-slate-100 border-b border-slate-200 text-slate-700 font-mono text-[11px] uppercase tracking-wider">
                <tr>
                  <th
                    @click="togglePeriodSort('name')"
                    class="py-3 px-4 cursor-pointer hover:bg-slate-200/70 select-none transition"
                  >
                    <div class="flex items-center gap-1.5">
                      <span>Nama Periode</span>
                      <ArrowUp v-if="periodSortBy === 'name' && periodSortDir === 'asc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowDown v-else-if="periodSortBy === 'name' && periodSortDir === 'desc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th
                    @click="togglePeriodSort('start_date')"
                    class="py-3 px-4 cursor-pointer hover:bg-slate-200/70 select-none transition"
                  >
                    <div class="flex items-center gap-1.5">
                      <span>Rentang Tanggal</span>
                      <ArrowUp v-if="periodSortBy === 'start_date' && periodSortDir === 'asc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowDown v-else-if="periodSortBy === 'start_date' && periodSortDir === 'desc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th
                    @click="togglePeriodSort('is_closed')"
                    class="py-3 px-4 text-center cursor-pointer hover:bg-slate-200/70 select-none transition"
                  >
                    <div class="flex items-center justify-center gap-1.5">
                      <span>Status Periode</span>
                      <ArrowUp v-if="periodSortBy === 'is_closed' && periodSortDir === 'asc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowDown v-else-if="periodSortBy === 'is_closed' && periodSortDir === 'desc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th class="py-3 px-4">Catatan Penguncian</th>
                  <th class="py-3 px-4 text-center w-36">Aksi</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-slate-200 font-mono">
                <tr v-if="accountingPeriods.length === 0">
                  <td colspan="5" class="py-12 text-center text-slate-400 font-sans">
                    <p class="font-medium text-slate-600">Tidak ada periode akuntansi yang sesuai.</p>
                    <p class="text-xs text-slate-400 mt-1">Coba sesuaikan kata kunci pencarian atau tambah periode baru.</p>
                  </td>
                </tr>
                <tr v-for="p in accountingPeriods" :key="p.id" class="hover:bg-slate-50 transition">
                  <td class="py-3.5 px-4 font-bold text-slate-900 font-sans">
                    {{ p.name }}
                  </td>
                  <td class="py-3.5 px-4 font-bold text-sky-800">
                    {{ formatDate(p.start_date) }} s/d {{ formatDate(p.end_date) }}
                  </td>
                  <td class="py-3.5 px-4 text-center">
                    <span
                      v-if="p.is_closed"
                      class="inline-flex items-center gap-1 px-2.5 py-1 rounded-full text-[10px] font-bold bg-rose-100 text-rose-900 border border-rose-300"
                    >
                      <Lock class="w-3 h-3 text-rose-700" />
                      <span>TERKUNCI (SOFT CLOSED)</span>
                    </span>
                    <span
                      v-else
                      class="inline-flex items-center gap-1 px-2.5 py-1 rounded-full text-[10px] font-bold bg-emerald-100 text-emerald-800 border border-emerald-300"
                    >
                      <Unlock class="w-3 h-3 text-emerald-700" />
                      <span>TERBUKA (OPEN)</span>
                    </span>
                  </td>
                  <td class="py-3.5 px-4 text-[11px] font-sans text-slate-600">
                    <div v-if="p.is_closed && p.closed_at">
                      <span class="font-semibold text-rose-800">Dikunci:</span> {{ formatDate(p.closed_at) }}
                      <span v-if="p.closed_by_name" class="text-slate-500">oleh {{ p.closed_by_name }}</span>
                    </div>
                    <div v-else-if="p.reopened_at">
                      <span class="font-semibold text-amber-700">Dibuka Kembali:</span> {{ formatDate(p.reopened_at) }}
                      <span v-if="p.reopened_by_name" class="text-slate-500">oleh {{ p.reopened_by_name }}</span>
                      <p v-if="p.reopen_reason" class="text-amber-900 italic mt-0.5 font-sans">Alasan: {{ p.reopen_reason }}</p>
                    </div>
                    <div v-else class="text-slate-400 italic">
                      Periode aktif berjalan (mutasi diperbolehkan)
                    </div>
                  </td>
                  <td class="py-3.5 px-4 text-center">
                    <button
                      v-if="!p.is_closed && authStore.hasPermission(['accounting:periods:close', 'accounting:close_period', 'accounting:write'])"
                      type="button"
                      @click="handleSoftClosePeriod(p)"
                      class="px-2.5 py-1 rounded-lg bg-rose-50 hover:bg-rose-100 text-rose-700 border border-rose-200 text-[11px] font-bold transition cursor-pointer flex items-center justify-center gap-1 mx-auto"
                    >
                      <Lock class="w-3 h-3" />
                      <span>Kunci Periode</span>
                    </button>
                    <div v-else-if="p.is_closed" class="flex flex-col gap-1 items-center justify-center">
                      <button
                        v-if="authStore.hasPermission(['accounting:periods:reopen', 'accounting:reopen_period', 'accounting:write'])"
                        type="button"
                        @click="openReopenPeriodModal(p)"
                        class="px-2 py-0.5 rounded-lg bg-amber-50 hover:bg-amber-100 text-amber-800 border border-amber-200 text-[10px] font-bold transition cursor-pointer flex items-center justify-center gap-1 w-full"
                      >
                        <Unlock class="w-2.5 h-2.5" />
                        <span>Buka Kunci</span>
                      </button>
                    </div>
                    <span v-else class="text-slate-400 text-[11px] italic">-</span>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <!-- Pagination Bar -->
          <PaginationBar
            v-model:currentPage="periodPage"
            v-model:pageSize="periodPageSize"
            :totalItems="periodTotalItems"
            @update:currentPage="fetchAccountingPeriods"
            @update:pageSize="() => { periodPage = 1; fetchAccountingPeriods(); }"
          />
        </div>

        <!-- ================================================================= -->
        <!-- SUB-TAB 2: HISTORI DOKUMEN TUTUP BUKU (ACCOUNTING CLOSINGS) -->
        <!-- ================================================================= -->
        <div v-else-if="closingSubTab === 'closings'" class="bg-white border border-slate-200 rounded-2xl overflow-hidden shadow-xs">
          <!-- Toolbar Filter Server-Side -->
          <div class="p-4 bg-slate-50 border-b border-slate-200 flex flex-wrap items-center justify-between gap-3">
            <div class="flex items-center gap-2">
              <div class="p-1.5 rounded-lg bg-indigo-100 text-indigo-800">
                <FileText class="w-4 h-4 text-indigo-700" />
              </div>
              <div>
                <h3 class="font-bold text-xs uppercase tracking-wider text-slate-800 font-mono">
                  Riwayat Dokumen Penutupan Buku Periode (Accounting Closings)
                </h3>
                <p class="text-[11px] text-slate-500 font-sans">
                  Daftar dokumen penutupan buku resmi dengan jurnal penutup otomatis ke laba ditahan.
                </p>
              </div>
            </div>

            <!-- Search, Status Filter & Refresh -->
            <div class="flex items-center gap-2 flex-wrap">
              <div class="relative">
                <Search class="w-3.5 h-3.5 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2" />
                <input
                  v-model="closingSearch"
                  type="text"
                  placeholder="Cari no. closing/jurnal..."
                  @input="onClosingSearchInput"
                  @keyup.enter="closingPage = 1; fetchClosingList()"
                  class="pl-8 pr-3 py-1.5 text-xs border border-slate-300 rounded-lg bg-white text-slate-800 placeholder-slate-400 focus:outline-none focus:ring-1 focus:ring-indigo-500 w-56"
                />
              </div>

              <select
                v-model="closingStatus"
                @change="closingPage = 1; fetchClosingList()"
                class="text-xs border border-slate-300 rounded-lg bg-white text-slate-700 px-3 py-1.5 focus:outline-none focus:ring-1 focus:ring-indigo-500 cursor-pointer"
              >
                <option value="">Semua Status</option>
                <option value="CLOSED">Terkunci (Closed)</option>
                <option value="REOPENED">Dibuka Kembali</option>
              </select>

              <button
                type="button"
                @click="closingSearch = ''; closingStatus = ''; closingPage = 1; fetchClosingList()"
                class="p-1.5 text-slate-500 hover:text-slate-800 hover:bg-slate-200/70 rounded-lg transition cursor-pointer"
                title="Reset Filter"
              >
                <RefreshCw class="w-4 h-4" :class="{ 'animate-spin': isClosingLoading }" />
              </button>
            </div>
          </div>

          <!-- Loading State -->
          <div v-if="isClosingLoading" class="py-12 text-center text-slate-500 font-sans flex flex-col items-center justify-center gap-2">
            <RefreshCw class="w-6 h-6 animate-spin text-indigo-600" />
            <span class="text-xs font-medium">Memuat data histori tutup buku...</span>
          </div>

          <!-- Table Content -->
          <div v-else class="overflow-x-auto">
            <table class="w-full text-left text-xs border-collapse">
              <thead class="bg-slate-100 border-b border-slate-200 text-slate-700 font-mono text-[11px] uppercase tracking-wider">
                <tr>
                  <th
                    @click="toggleClosingSort('closing_number')"
                    class="py-3 px-4 cursor-pointer hover:bg-slate-200/70 select-none transition"
                  >
                    <div class="flex items-center gap-1.5">
                      <span>No. Tutup Buku</span>
                      <ArrowUp v-if="closingSortBy === 'closing_number' && closingSortDir === 'asc'" class="w-3.5 h-3.5 text-indigo-600" />
                      <ArrowDown v-else-if="closingSortBy === 'closing_number' && closingSortDir === 'desc'" class="w-3.5 h-3.5 text-indigo-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th
                    @click="toggleClosingSort('period_end')"
                    class="py-3 px-4 cursor-pointer hover:bg-slate-200/70 select-none transition"
                  >
                    <div class="flex items-center gap-1.5">
                      <span>Periode Penutupan</span>
                      <ArrowUp v-if="closingSortBy === 'period_end' && closingSortDir === 'asc'" class="w-3.5 h-3.5 text-indigo-600" />
                      <ArrowDown v-else-if="closingSortBy === 'period_end' && closingSortDir === 'desc'" class="w-3.5 h-3.5 text-indigo-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th class="py-3 px-4">Lingkup Cabang</th>
                  <th
                    @click="toggleClosingSort('net_income')"
                    class="py-3 px-4 text-right cursor-pointer hover:bg-slate-200/70 select-none transition"
                  >
                    <div class="flex items-center justify-end gap-1.5">
                      <span>Laba/Rugi Bersih</span>
                      <ArrowUp v-if="closingSortBy === 'net_income' && closingSortDir === 'asc'" class="w-3.5 h-3.5 text-indigo-600" />
                      <ArrowDown v-else-if="closingSortBy === 'net_income' && closingSortDir === 'desc'" class="w-3.5 h-3.5 text-indigo-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th
                    @click="toggleClosingSort('status')"
                    class="py-3 px-4 text-center cursor-pointer hover:bg-slate-200/70 select-none transition"
                  >
                    <div class="flex items-center justify-center gap-1.5">
                      <span>Status</span>
                      <ArrowUp v-if="closingSortBy === 'status' && closingSortDir === 'asc'" class="w-3.5 h-3.5 text-indigo-600" />
                      <ArrowDown v-else-if="closingSortBy === 'status' && closingSortDir === 'desc'" class="w-3.5 h-3.5 text-indigo-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th class="py-3 px-4">Catatan / Alasan Reopen</th>
                  <th class="py-3 px-4 text-center w-28">Aksi</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-slate-200 font-mono">
                <tr v-if="closingList.length === 0">
                  <td colspan="7" class="py-12 text-center text-slate-400 font-sans">
                    <p class="font-medium text-slate-600">Belum ada dokumen penutupan buku yang sesuai.</p>
                    <p class="text-xs text-slate-400 mt-1">Gunakan tombol "Tutup Buku Baru" untuk memulai proses tutup periode.</p>
                  </td>
                </tr>
                <tr v-for="c in closingList" :key="c.id" class="hover:bg-slate-50 transition">
                  <td class="py-3.5 px-4 font-bold text-slate-900">
                    <div>{{ c.closing_number }}</div>
                    <div class="text-[10px] text-slate-400 font-normal">Dibuat: {{ formatDate(c.closing_date) }}</div>
                    <div v-if="c.journal_number" class="mt-1">
                      <button
                        type="button"
                        @click="viewClosingJournal(c.journal_number)"
                        class="inline-flex items-center gap-1 text-[11px] font-mono font-medium text-sky-700 bg-sky-50 hover:bg-sky-100 hover:text-sky-900 px-2 py-0.5 rounded border border-sky-200 transition cursor-pointer"
                        title="Lihat Rincian Jurnal Penutup di Tab Jurnal"
                      >
                        <BookOpen class="w-3 h-3 text-sky-600" />
                        <span>{{ c.journal_number }}</span>
                      </button>
                    </div>
                  </td>
                  <td class="py-3.5 px-4 font-bold text-sky-800">
                    {{ formatDate(c.period_start) }} s/d {{ formatDate(c.period_end) }}
                  </td>
                  <td class="py-3.5 px-4 font-sans">
                    <span v-if="c.outlet_name" class="font-bold text-slate-800">{{ c.outlet_name }}</span>
                    <span v-else class="text-emerald-800 font-bold bg-emerald-50 px-2 py-0.5 rounded border border-emerald-200 text-[10px]">
                      Konsolidasi Seluruh Outlet
                    </span>
                  </td>
                  <td class="py-3.5 px-4 text-right font-bold" :class="Number(c.net_income) >= 0 ? 'text-emerald-700' : 'text-rose-700'">
                    {{ formatCurrency(c.net_income) }}
                  </td>
                  <td class="py-3.5 px-4 text-center">
                    <span
                      v-if="c.status === 'CLOSED'"
                      class="inline-flex items-center gap-1 px-2.5 py-1 rounded-full text-[10px] font-bold bg-amber-100 text-amber-900 border border-amber-300"
                    >
                      <Lock class="w-3 h-3 text-amber-700" />
                      <span>TERKUNCI (CLOSED)</span>
                    </span>
                    <span
                      v-else
                      class="inline-flex items-center gap-1 px-2.5 py-1 rounded-full text-[10px] font-bold bg-slate-100 text-slate-700 border border-slate-300"
                    >
                      <Unlock class="w-3 h-3 text-slate-500" />
                      <span>DIBUKA KEMBALI</span>
                    </span>
                  </td>
                  <td class="py-3.5 px-4 font-sans text-slate-600 max-w-xs truncate">
                    <span v-if="c.status === 'REOPENED' && c.reopen_reason" class="text-rose-700 font-medium">
                      Alasan Reopen: {{ c.reopen_reason }}
                    </span>
                    <span v-else>{{ c.notes || '-' }}</span>
                  </td>
                  <td class="py-3.5 px-4 text-center">
                    <div class="flex flex-col gap-1 items-center justify-center">
                      <button
                        v-if="c.status === 'CLOSED' && Number(c.net_income) > 0 && authStore.hasPermission(['accounting:dividend:create', 'accounting:write'])"
                        type="button"
                        @click="openCreateDividendModal(c.id)"
                        class="px-2 py-0.5 rounded-lg bg-emerald-50 hover:bg-emerald-100 text-emerald-800 border border-emerald-300 text-[10px] font-bold transition cursor-pointer flex items-center justify-center gap-1 w-full"
                        title="Bagi dividen dari hasil penutupan buku ini"
                      >
                        <Coins class="w-2.5 h-2.5 text-emerald-600" />
                        <span>Bagi Dividen</span>
                      </button>
                      <button
                        v-if="c.status === 'CLOSED' && authStore.hasPermission(['accounting:reopen_period', 'accounting:write'])"
                        type="button"
                        @click="openReopenModal(c)"
                        class="px-2.5 py-1 rounded-lg bg-rose-50 hover:bg-rose-100 text-rose-700 border border-rose-200 text-[11px] font-bold transition cursor-pointer w-full"
                      >
                        Buka Kembali
                      </button>
                      <span v-else-if="c.status === 'CLOSED'" class="text-slate-400 text-[11px] italic">Terkunci</span>
                      <span v-else class="text-slate-400 text-[11px] italic">Terbuka</span>
                    </div>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <!-- Pagination Bar -->
          <PaginationBar
            v-model:currentPage="closingPage"
            v-model:pageSize="closingPageSize"
            :totalItems="closingTotalItems"
            @update:currentPage="fetchClosingList"
            @update:pageSize="() => { closingPage = 1; fetchClosingList(); }"
          />
        </div>
      </div>

      <!-- ===================================================================== -->
      <!-- TAB: PEMBAGIAN DIVIDEN (LABA DITAHAN) -->
      <!-- ===================================================================== -->
      <div v-else-if="activeTab === 'dividend'" class="space-y-6">
        <!-- Guidelines Banner -->
        <div class="bg-gradient-to-r from-amber-50 to-orange-50 border border-amber-200 rounded-2xl p-5 text-amber-950 flex items-start gap-4 shadow-2xs">
          <div class="p-2.5 rounded-xl bg-amber-500 text-white shrink-0 mt-0.5">
            <Coins class="w-5 h-5" />
          </div>
          <div class="space-y-1.5 text-xs">
            <h4 class="font-extrabold text-sm text-amber-900">
              Kaidah Akuntansi Pembagian Dividen dari Laba Ditahan (Retained Earnings, Akun 3201)
            </h4>
            <ul class="list-disc list-inside space-y-1 text-slate-700 leading-relaxed">
              <li>
                <strong>Koneksi Langsung dengan Tutup Buku:</strong> Dividen dialokasikan langsung dari akumulasi akun nominal laba bersih yang telah diposting ke <em>Laba Ditahan (3201)</em> melalui proses <strong>Tutup Buku (Accounting Closings)</strong>. Soft close (kunci periode) hanya mengunci transaksi tanpa melakukan penjurnalan pemindahan laba.
              </li>
              <li>
                <strong>Otomatisasi Penjurnalan Berpasangan:</strong> Setiap pembagian dividen langsung memicu jurnal debit ke akun <em>3201 - Laba Ditahan</em> (mengurangi saldo modal laba) dan kredit ke <em>Rekening Kas/Bank</em> pencairan secara seimbang (balanced).
              </li>
              <li>
                <strong>Alokasi Multi Pemegang Saham:</strong> Sistem mencatat rincian proporsi dividen setiap pemegang saham/owner dengan nominal dan persentase kepemilikan yang tersimpan rapi untuk dokumentasi RUPS / pembagian hasil usaha.
              </li>
            </ul>
          </div>
        </div>

        <!-- KPI Summary Cards -->
        <div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
          <!-- Saldo Laba Ditahan -->
          <div class="bg-white p-4 rounded-2xl border border-slate-200 shadow-xs flex items-center justify-between">
            <div>
              <div class="flex items-center gap-1.5">
                <p class="text-[11px] font-bold text-slate-500 uppercase tracking-wider">Saldo Laba Ditahan (3201)</p>
                <span
                  class="px-1.5 py-0.5 rounded text-[9px] font-bold"
                  :class="dividendSummary.total_retained_earnings > 0 ? 'bg-emerald-100 text-emerald-800' : 'bg-rose-100 text-rose-800'"
                >
                  {{ dividendSummary.total_retained_earnings > 0 ? 'Tersedia' : 'Nol / Defisit' }}
                </span>
              </div>
              <h3
                class="text-lg font-black mt-1"
                :class="dividendSummary.total_retained_earnings > 0 ? 'text-emerald-700' : 'text-slate-800'"
              >
                {{ formatCurrency(dividendSummary.total_retained_earnings) }}
              </h3>
              <p class="text-[10px] text-slate-400 mt-0.5">Batas maksimal dividen yang dapat dibagikan</p>
            </div>
            <div class="w-10 h-10 rounded-xl bg-emerald-50 text-emerald-700 flex items-center justify-center">
              <Scale class="w-5 h-5" />
            </div>
          </div>

          <!-- Total Dividen Dibagikan -->
          <div class="bg-white p-4 rounded-2xl border border-slate-200 shadow-xs flex items-center justify-between">
            <div>
              <p class="text-[11px] font-bold text-slate-500 uppercase tracking-wider">Total Dividen Terbagikan</p>
              <h3 class="text-lg font-black text-slate-900 mt-1">
                {{ formatCurrency(dividendSummary.total_dividends_paid) }}
              </h3>
              <p class="text-[10px] text-slate-400 mt-0.5">
                {{ dividendTotalItems }} kali distribusi pembagian laba
              </p>
            </div>
            <div class="w-10 h-10 rounded-xl bg-amber-50 text-amber-700 flex items-center justify-center">
              <Coins class="w-5 h-5" />
            </div>
          </div>

          <!-- Periode Tutup Buku Tersedia -->
          <div class="bg-white p-4 rounded-2xl border border-slate-200 shadow-xs flex items-center justify-between">
            <div>
              <p class="text-[11px] font-bold text-slate-500 uppercase tracking-wider">Periode Tutup Buku Siap</p>
              <h3 class="text-lg font-black text-sky-800 mt-1">
                {{ dividendSummary.eligible_closed_periods.length }} Periode
              </h3>
              <p class="text-[10px] text-slate-400 mt-0.5">Periode akuntansi terkunci (Soft Closed)</p>
            </div>
            <div class="w-10 h-10 rounded-xl bg-sky-50 text-sky-700 flex items-center justify-center">
              <Lock class="w-5 h-5" />
            </div>
          </div>
        </div>

        <!-- Filter & Action Bar -->
        <div class="bg-white p-4 rounded-2xl border border-slate-200 shadow-xs flex flex-wrap items-center justify-between gap-3">
          <div class="flex items-center gap-3 flex-wrap flex-1">
            <!-- Filter Dokumen Tutup Buku -->
            <div class="min-w-64">
              <select
                v-model="dividendFilterPeriodId"
                @change="handleDividendSearch"
                class="w-full text-xs font-semibold px-3 py-2 bg-slate-50 border border-slate-300 rounded-xl focus:ring-2 focus:ring-emerald-500/20 focus:border-emerald-600 transition"
              >
                <option value="">Semua Dokumen Tutup Buku</option>
                <option v-for="cp in dividendSummary.eligible_closed_periods" :key="cp.id" :value="cp.id">
                  {{ cp.closing_number }} ({{ formatDate(cp.period_start) }} s/d {{ formatDate(cp.period_end) }}) - Laba: {{ formatCurrency(cp.net_income) }}
                </option>
              </select>
            </div>

            <!-- Search input -->
            <div class="relative flex-1 min-w-52">
              <Search class="w-3.5 h-3.5 absolute left-3 top-1/2 -translate-y-1/2 text-slate-400" />
              <input
                v-model="dividendSearch"
                @keyup.enter="handleDividendSearch"
                type="text"
                placeholder="Cari no. dividen, penerima, atau catatan..."
                class="w-full pl-9 pr-3 py-2 text-xs bg-slate-50 border border-slate-300 rounded-xl focus:ring-2 focus:ring-emerald-500/20 focus:border-emerald-600 transition"
              />
            </div>

            <button
              @click="handleDividendSearch"
              class="px-3 py-2 bg-slate-100 hover:bg-slate-200 text-slate-700 text-xs font-bold rounded-xl transition cursor-pointer"
            >
              Cari
            </button>
            <button
              @click="refreshActiveTab"
              class="p-2 bg-slate-100 hover:bg-slate-200 text-slate-600 rounded-xl transition cursor-pointer"
              title="Muat ulang data"
            >
              <RefreshCw class="w-3.5 h-3.5" />
            </button>
          </div>

          <!-- Create Dividend & Print Buttons -->
          <div class="flex items-center gap-2">
            <button
              @click="openPrintReport('dividend')"
              class="inline-flex items-center gap-1.5 px-3.5 py-2 rounded-xl bg-slate-900 hover:bg-slate-800 text-white text-xs font-bold transition shadow-xs cursor-pointer"
            >
              <Printer class="w-3.5 h-3.5 text-emerald-400" />
              <span>Cetak Rekap Dividen (PDF)</span>
            </button>
            <button
              v-if="authStore.hasPermission(['accounting:dividend:create', 'accounting:write'])"
              @click="openCreateDividendModal()"
              :disabled="dividendSummary.total_retained_earnings <= 0"
              class="inline-flex items-center gap-1.5 px-4 py-2 rounded-xl text-xs font-bold text-white transition shadow-xs cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed bg-emerald-600 hover:bg-emerald-700"
              :title="dividendSummary.total_retained_earnings <= 0 ? 'Saldo Laba Ditahan (3201) tidak mencukupi untuk dibagikan' : 'Bagikan Dividen dari Laba Periode Tutup Buku'"
            >
              <Plus class="w-3.5 h-3.5" />
              <span>Pembagian Dividen Baru</span>
            </button>
          </div>
        </div>

        <!-- Table of Dividend Distributions -->
        <div class="bg-white border border-slate-200 rounded-2xl overflow-hidden shadow-xs">
          <table class="w-full text-left text-xs border-collapse">
            <thead class="bg-slate-100 border-b border-slate-200 text-slate-700 font-mono text-[11px] uppercase tracking-wider">
              <tr>
                <th class="py-3 px-4">No. Distribusi / Tanggal</th>
                <th class="py-3 px-4">Periode Tutup Buku</th>
                <th class="py-3 px-4">Rekening Pencairan</th>
                <th class="py-3 px-4 text-right">Total Dividen</th>
                <th class="py-3 px-4">Alokasi Penerima</th>
                <th class="py-3 px-4 text-center">No. Jurnal</th>
                <th class="py-3 px-4 text-center w-36">Aksi</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-200 font-mono">
              <tr v-if="dividendList.length === 0">
                <td colspan="7" class="py-12 text-center text-slate-400 font-sans">
                  <div class="flex flex-col items-center gap-2">
                    <Coins class="w-8 h-8 text-slate-300" />
                    <p class="font-bold text-slate-600">Belum ada transaksi pembagian dividen</p>
                    <p class="text-xs text-slate-400 max-w-md">
                      Dividen dapat dibagikan setelah Anda melakukan proses <strong>Tutup Buku Periode</strong> dan saldo akun Laba Ditahan (3201) tersedia.
                    </p>
                  </div>
                </td>
              </tr>
              <tr v-for="d in dividendList" :key="d.id" class="hover:bg-slate-50 transition">
                <td class="py-3.5 px-4">
                  <div class="font-extrabold text-sky-800">{{ d.distribution_number }}</div>
                  <div class="text-[10px] text-slate-500 font-sans font-normal">{{ formatDate(d.distribution_date) }}</div>
                </td>
                <td class="py-3.5 px-4 font-sans">
                  <div class="font-bold text-slate-800">{{ d.period_name || 'Periode #' + d.period_id }}</div>
                  <div class="text-[10px] text-slate-500">
                    <span v-if="d.period_start && d.period_end">
                      {{ formatDate(d.period_start) }} - {{ formatDate(d.period_end) }}
                    </span>
                    <span class="inline-flex items-center gap-0.5 ml-1 text-rose-700 font-bold">
                      <Lock class="w-2.5 h-2.5" /> Terkunci
                    </span>
                  </div>
                </td>
                <td class="py-3.5 px-4 font-sans">
                  <div class="font-semibold text-slate-700">
                    <span class="font-mono text-slate-500 text-[11px] mr-1">{{ d.payment_account_code }}</span>
                    {{ d.payment_account_name }}
                  </div>
                </td>
                <td class="py-3.5 px-4 text-right font-bold text-emerald-700 text-sm">
                  {{ formatCurrency(d.total_amount) }}
                </td>
                <td class="py-3.5 px-4 font-sans">
                  <div v-if="d.recipients_data && d.recipients_data.length > 0">
                    <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-bold bg-amber-100 text-amber-900 border border-amber-300">
                      <Users class="w-3 h-3 text-amber-700" />
                      {{ d.recipients_data.length }} Penerima
                    </span>
                    <div class="text-[10px] text-slate-500 truncate max-w-44 mt-0.5">
                      {{ d.recipients_data.map(r => r.name).join(', ') }}
                    </div>
                  </div>
                  <div v-else class="text-slate-700 text-xs">
                    {{ d.recipient_name || 'Pemegang Saham' }}
                  </div>
                </td>
                <td class="py-3.5 px-4 text-center font-mono">
                  <button
                    v-if="d.journal_number"
                    @click="viewDividendJournal(d.journal_entry_id)"
                    class="inline-flex items-center gap-1 px-2 py-0.5 rounded-md text-[10px] font-bold bg-sky-50 hover:bg-sky-100 text-sky-800 border border-sky-300 transition cursor-pointer"
                    title="Lihat Jurnal Umum"
                  >
                    <BookOpen class="w-3 h-3 text-sky-600" />
                    <span>{{ d.journal_number }}</span>
                  </button>
                  <span v-else class="text-slate-400 text-[10px] italic">-</span>
                </td>
                <td class="py-3.5 px-4 text-center">
                  <div class="flex items-center justify-center gap-1.5">
                    <button
                      type="button"
                      @click="openDividendDetail(d)"
                      class="p-1.5 rounded-lg bg-slate-100 hover:bg-slate-200 text-slate-700 transition cursor-pointer"
                      title="Lihat Detail & Bukti Dividen"
                    >
                      <Eye class="w-3.5 h-3.5" />
                    </button>
                    <button
                      v-if="authStore.hasPermission(['accounting:dividend:delete', 'accounting:write'])"
                      type="button"
                      @click="openCancelDividendModal(d)"
                      class="p-1.5 rounded-lg bg-rose-50 hover:bg-rose-100 text-rose-700 border border-rose-200 transition cursor-pointer"
                      title="Batalkan & Reversal Jurnal Dividen"
                    >
                      <RotateCcw class="w-3.5 h-3.5" />
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>

          <!-- Pagination Bar -->
          <div class="p-3 bg-slate-50 border-t border-slate-200 flex items-center justify-between">
            <span class="text-xs text-slate-500">
              Menampilkan {{ dividendList.length }} dari {{ dividendTotalItems }} transaksi dividen
            </span>
            <PaginationBar
              :total-items="dividendTotalItems"
              :current-page="dividendPage"
              :page-size="dividendPageSize"
              @page-change="(p: number) => { dividendPage = p; fetchDividendList() }"
            />
          </div>
        </div>
      </div>

      <!-- ===================================================================== -->
      <!-- TAB: SETORAN MODAL (CAPITAL DEPOSITS) -->
      <!-- ===================================================================== -->
      <div v-else-if="activeTab === 'capital'" class="space-y-4">
        <!-- KPI Summary Cards -->
        <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
          <div class="bg-white p-4 rounded-2xl border border-slate-200 shadow-xs flex items-center justify-between">
            <div>
              <p class="text-[11px] font-bold text-slate-500 uppercase tracking-wider">Total Modal Masuk</p>
              <h3 class="text-lg font-black text-slate-900 mt-1">{{ formatCurrency(capitalSummary.total_amount) }}</h3>
              <p class="text-[10px] text-slate-400 mt-0.5">Semua waktu (status aktif)</p>
            </div>
            <div class="w-10 h-10 rounded-xl bg-emerald-50 text-emerald-700 flex items-center justify-center">
              <Coins class="w-5 h-5" />
            </div>
          </div>

          <div class="bg-white p-4 rounded-2xl border border-slate-200 shadow-xs flex items-center justify-between">
            <div>
              <p class="text-[11px] font-bold text-slate-500 uppercase tracking-wider">Setoran Bulan Ini</p>
              <h3 class="text-lg font-black text-emerald-700 mt-1">{{ formatCurrency(capitalSummary.this_month_amount) }}</h3>
              <p class="text-[10px] text-slate-400 mt-0.5">Penambahan modal kas</p>
            </div>
            <div class="w-10 h-10 rounded-xl bg-sky-50 text-sky-700 flex items-center justify-center">
              <TrendingUp class="w-5 h-5" />
            </div>
          </div>

          <div class="bg-white p-4 rounded-2xl border border-slate-200 shadow-xs flex items-center justify-between">
            <div>
              <p class="text-[11px] font-bold text-slate-500 uppercase tracking-wider">Setoran Tahun Ini</p>
              <h3 class="text-lg font-black text-purple-700 mt-1">{{ formatCurrency(capitalSummary.this_year_amount) }}</h3>
              <p class="text-[10px] text-slate-400 mt-0.5">Tahun buku berjalan</p>
            </div>
            <div class="w-10 h-10 rounded-xl bg-purple-50 text-purple-700 flex items-center justify-center">
              <Calendar class="w-5 h-5" />
            </div>
          </div>

          <div class="bg-white p-4 rounded-2xl border border-slate-200 shadow-xs flex items-center justify-between">
            <div>
              <p class="text-[11px] font-bold text-slate-500 uppercase tracking-wider">Frekuensi Transaksi</p>
              <h3 class="text-lg font-black text-slate-900 mt-1">{{ capitalSummary.total_count }} Kali</h3>
              <p class="text-[10px] text-slate-400 mt-0.5">Riwayat setoran modal</p>
            </div>
            <div class="w-10 h-10 rounded-xl bg-amber-50 text-amber-700 flex items-center justify-center">
              <Scale class="w-5 h-5" />
            </div>
          </div>
        </div>

        <!-- Filter & Search Toolbar -->
        <div class="bg-white p-3.5 rounded-2xl border border-slate-200 shadow-xs flex flex-wrap items-center justify-between gap-3">
          <div class="flex flex-wrap items-center gap-2.5">
            <div class="relative w-64 sm:w-80">
              <Search class="w-4 h-4 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2" />
              <input
                v-model="capitalSearch"
                @keyup.enter="fetchCapitalDeposits"
                placeholder="Cari pemilik, no. bukti, referensi..."
                class="w-full bg-slate-50 border border-slate-200 rounded-xl pl-9 pr-3 py-1.5 text-xs text-slate-800 placeholder-slate-400 focus:outline-none focus:border-emerald-600"
              />
            </div>

            <select
              v-model="capitalStatusFilter"
              @change="fetchCapitalDeposits"
              class="bg-slate-50 border border-slate-200 rounded-xl px-3 py-1.5 text-xs font-bold text-slate-700 focus:outline-none cursor-pointer"
            >
              <option value="ALL">Semua Status</option>
              <option value="COMPLETED">Selesai (Aktif)</option>
              <option value="CANCELLED">Dibatalkan</option>
            </select>

            <button
              @click="fetchCapitalDeposits"
              class="px-3 py-1.5 rounded-xl bg-slate-100 hover:bg-slate-200 text-slate-700 text-xs font-bold transition cursor-pointer"
            >
              Terapkan
            </button>
          </div>

          <div class="flex items-center gap-2 ml-auto">
            <button
              @click="openPrintReport('capital')"
              class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-slate-900 hover:bg-slate-800 text-white text-xs font-bold transition shadow-xs cursor-pointer"
            >
              <Printer class="w-3.5 h-3.5 text-emerald-400" />
              <span>Cetak Rekap Modal (PDF)</span>
            </button>
            <button
              v-if="authStore.hasPermission(['capital:create', 'accounting:write'])"
              @click="openCapitalModal"
              class="inline-flex items-center gap-1.5 px-3.5 py-1.5 rounded-xl bg-emerald-700 hover:bg-emerald-600 text-white text-xs font-bold shadow-xs transition active:scale-95 cursor-pointer"
            >
              <Coins class="w-3.5 h-3.5 text-amber-300" />
              <span>+ Setor Modal Usaha</span>
            </button>
          </div>
        </div>

        <!-- Capital Deposits Table -->
        <div class="bg-white border border-slate-200 rounded-2xl overflow-hidden shadow-xs">
          <table class="w-full text-left text-xs border-collapse">
            <thead class="bg-slate-100 border-b border-slate-200 text-slate-700 font-mono text-[11px] uppercase tracking-wider">
              <tr>
                <th class="py-3 px-4">No. Setoran & Tanggal</th>
                <th class="py-3 px-4">Penyetor / Pemilik</th>
                <th class="py-3 px-4">Outlet / Lokasi</th>
                <th class="py-3 px-4">Kas / Bank Penerima</th>
                <th class="py-3 px-4">Akun Modal (Ekuitas)</th>
                <th class="py-3 px-4 text-right">Nominal (Rp)</th>
                <th class="py-3 px-4 text-center">Status</th>
                <th class="py-3 px-4 text-center w-28">Aksi</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-200 font-mono">
              <tr v-if="capitalDeposits.length === 0">
                <td colspan="8" class="py-12 text-center text-slate-400 font-sans">
                  Belum ada catatan setoran modal usaha. Klik "+ Setor Modal Usaha" untuk membukukan setoran dana modal pemilik.
                </td>
              </tr>
              <tr v-for="dep in capitalDeposits" :key="dep.id" class="hover:bg-slate-50 transition">
                <td class="py-3.5 px-4 font-bold text-slate-900">
                  <div class="text-sky-800">{{ dep.deposit_no }}</div>
                  <div class="text-[10px] text-slate-400 font-normal">{{ formatDate(dep.deposit_date) }}</div>
                </td>
                <td class="py-3.5 px-4 font-sans">
                  <div class="font-bold text-slate-800">{{ dep.investor_name }}</div>
                  <div v-if="dep.reference_no" class="text-[10px] text-slate-400">Ref: {{ dep.reference_no }}</div>
                </td>
                <td class="py-3.5 px-4 font-sans">
                  <span v-if="dep.outlet_name" class="font-bold text-slate-800">{{ dep.outlet_name }}</span>
                  <span v-else class="text-emerald-800 font-bold bg-emerald-50 px-2 py-0.5 rounded border border-emerald-200 text-[10px]">
                    Kantor Pusat / Konsolidasi
                  </span>
                </td>
                <td class="py-3.5 px-4">
                  <div class="font-bold text-emerald-800">{{ dep.payment_account_code }}</div>
                  <div class="text-[10px] text-slate-500 font-sans truncate max-w-[140px]">{{ dep.payment_account_name }}</div>
                </td>
                <td class="py-3.5 px-4">
                  <div class="font-bold text-purple-800">{{ dep.equity_account_code }}</div>
                  <div class="text-[10px] text-slate-500 font-sans truncate max-w-[140px]">{{ dep.equity_account_name }}</div>
                </td>
                <td class="py-3.5 px-4 text-right font-bold text-slate-900">
                  <div class="text-emerald-700 font-black">{{ formatCurrency(dep.amount) }}</div>
                  <div v-if="dep.notes" class="text-[10px] text-slate-400 font-sans font-normal truncate max-w-[130px] ml-auto">
                    {{ dep.notes }}
                  </div>
                </td>
                <td class="py-3.5 px-4 text-center">
                  <span
                    v-if="dep.status === 'COMPLETED'"
                    class="bg-emerald-50 text-emerald-800 border border-emerald-300 text-[10px] px-2.5 py-0.5 rounded-full font-bold"
                  >
                    Terjurnal
                  </span>
                  <div v-else class="flex flex-col items-center">
                    <span class="bg-rose-50 text-rose-800 border border-rose-300 text-[10px] px-2 py-0.5 rounded-full font-bold">
                      Dibatalkan
                    </span>
                    <span v-if="dep.cancellation_reason" class="text-[9px] text-rose-500 font-sans mt-0.5 max-w-[110px] truncate" :title="dep.cancellation_reason">
                      {{ dep.cancellation_reason }}
                    </span>
                  </div>
                </td>
                <td class="py-3.5 px-4 text-center">
                  <div class="flex items-center justify-center gap-1.5 font-sans">
                    <button
                      @click="openCapitalVoucher(dep)"
                      class="p-1.5 rounded-lg text-slate-600 hover:text-emerald-700 hover:bg-emerald-50 border border-slate-200 transition cursor-pointer"
                      title="Cetak Bukti Kas Masuk / Setoran Modal"
                    >
                      <Printer class="w-3.5 h-3.5" />
                    </button>
                    <button
                      v-if="dep.status === 'COMPLETED' && authStore.hasPermission(['capital:delete', 'accounting:write'])"
                      @click="openCancelCapitalModal(dep)"
                      class="p-1.5 rounded-lg text-rose-600 hover:text-rose-800 hover:bg-rose-50 border border-rose-200 transition cursor-pointer"
                      title="Batalkan Setoran Modal (Jurnal Pembalik)"
                    >
                      <RotateCcw class="w-3.5 h-3.5" />
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
          <PaginationBar
            v-model:currentPage="capitalPage"
            v-model:pageSize="capitalPageSize"
            :totalItems="capitalTotal"
            @update:currentPage="fetchCapitalDeposits"
            @update:pageSize="fetchCapitalDeposits"
          />
        </div>
      </div>

      <!-- ===================================================================== -->
      <!-- TAB: REKENING BANK PENERIMAAN -->
      <!-- ===================================================================== -->
      <div v-else-if="activeTab === 'bank'" class="space-y-4">
        <!-- Info Banner -->
        <div class="p-4 bg-emerald-50/70 border border-emerald-200 rounded-2xl flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3 text-xs text-emerald-950">
          <div class="flex items-start gap-3">
            <div class="p-2 bg-emerald-100 border border-emerald-300 rounded-xl text-emerald-800 shrink-0 mt-0.5">
              <Landmark class="w-4 h-4" />
            </div>
            <div>
              <h4 class="font-bold text-emerald-900">Rekening Bank Penerimaan Operasional & Modal</h4>
              <p class="text-[11px] text-emerald-700 mt-0.5 leading-relaxed">
                Kelola daftar rekening bank tujuan transfer transaksi pelanggan, pembayaran tagihan, dan setoran modal usaha. Setiap rekening akan langsung terhubung ke pencatatan kas & bank.
              </p>
            </div>
          </div>
          <button
            v-if="authStore.hasPermission(['accounting:write', 'master:write'])"
            @click="openCreateBankModal()"
            class="shrink-0 flex items-center gap-2 px-4 py-2 bg-emerald-700 hover:bg-emerald-800 text-white font-bold rounded-xl transition cursor-pointer shadow-xs text-xs"
          >
            <Plus class="w-3.5 h-3.5" />
            <span>Tambah Rekening Bank</span>
          </button>
        </div>

        <!-- Toolbar -->
        <div class="flex flex-col sm:flex-row gap-3 items-start sm:items-center justify-between">
          <!-- Search & Filter -->
          <div class="flex flex-wrap items-center gap-3 flex-1 max-w-2xl">
            <div class="relative flex-1 min-w-[220px]">
              <Search class="absolute left-3 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-slate-400 pointer-events-none" />
              <input
                v-model="bankSearch"
                type="text"
                placeholder="Cari bank, nomor rekening, atau nama pemilik..."
                class="w-full pl-8 pr-3 py-2 text-xs bg-white border border-slate-300 rounded-xl focus:outline-none focus:border-emerald-600 text-slate-800 placeholder-slate-400 shadow-2xs"
              />
            </div>
            <div class="w-64 shrink-0">
              <SearchableSelect
                v-model="bankOutletFilter"
                api-endpoint="/outlets"
                secondary-key="code"
                placeholder="Semua Cabang / Outlet"
                search-placeholder="Cari cabang/outlet..."
                :allow-clear="true"
              />
            </div>
          </div>

          <div class="flex items-center gap-3">
            <span class="text-xs text-slate-500 font-medium">
              Total: <strong class="text-slate-800">{{ filteredBankAccounts.length }}</strong> rekening
            </span>
            <button
              @click="openPrintReport('bank')"
              class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-slate-900 hover:bg-slate-800 text-white text-xs font-bold transition shadow-xs cursor-pointer"
            >
              <Printer class="w-3.5 h-3.5 text-emerald-400" />
              <span>Cetak Rekap Bank (PDF)</span>
            </button>
          </div>
        </div>

        <!-- Cards / Table -->
        <div v-if="filteredBankAccounts.length > 0" class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-5">
          <div
            v-for="bank in filteredBankAccounts"
            :key="bank.id"
            class="bg-white border rounded-2xl p-5 md:p-6 transition-all duration-200 flex flex-col justify-between shadow-xs hover:shadow-md relative overflow-hidden group"
            :class="bank.is_active ? 'border-slate-200 hover:border-emerald-500/50' : 'border-slate-200 opacity-60 bg-slate-50'"
          >
            <!-- Top Badges -->
            <div class="flex items-start justify-between gap-3 mb-4">
              <div class="flex items-center gap-3">
                <div class="w-11 h-11 rounded-xl bg-gradient-to-br from-emerald-600 to-teal-800 text-white flex items-center justify-center font-black text-xs shadow-xs shrink-0 tracking-wider">
                  {{ bank.bank_name.slice(0, 3).toUpperCase() }}
                </div>
                <div>
                  <h3 class="font-bold text-slate-900 text-sm flex items-center gap-1.5">
                    {{ bank.bank_name }}
                    <span
                      v-if="bank.is_default"
                      class="px-2 py-0.5 rounded-full text-[10px] font-bold bg-amber-100 text-amber-900 border border-amber-300 shadow-2xs"
                    >
                      Utama
                    </span>
                  </h3>
                  <span v-if="bank.branch" class="text-[11px] text-slate-500 font-medium">Cabang {{ bank.branch }}</span>
                </div>
              </div>

              <!-- Status Badge -->
              <span
                class="px-2.5 py-0.5 rounded-full text-[10px] font-bold shrink-0"
                :class="bank.is_active ? 'bg-emerald-50 text-emerald-800 border border-emerald-200' : 'bg-slate-100 text-slate-500 border border-slate-200'"
              >
                {{ bank.is_active ? 'Aktif' : 'Nonaktif' }}
              </span>
            </div>

            <!-- Account Details -->
            <div class="bg-slate-50 border border-slate-200/80 rounded-xl p-3.5 my-2 space-y-2">
              <div class="flex items-center justify-between text-xs">
                <span class="text-slate-500 text-[11px] font-medium">Nomor Rekening:</span>
                <button
                  type="button"
                  @click="copyAccountNumber(bank)"
                  class="font-mono font-bold text-slate-900 flex items-center gap-1.5 hover:text-emerald-700 transition cursor-pointer px-1.5 py-0.5 rounded-md hover:bg-white"
                  title="Klik untuk menyalin"
                >
                  <span>{{ bank.account_number }}</span>
                  <Check v-if="copiedBankId === bank.id" class="w-3.5 h-3.5 text-emerald-600" />
                  <Copy v-else class="w-3.5 h-3.5 text-slate-400" />
                </button>
              </div>
              <div class="flex items-center justify-between text-xs">
                <span class="text-slate-500 text-[11px] font-medium">Atas Nama:</span>
                <span class="font-bold text-slate-800 uppercase tracking-wide text-[11px]">{{ bank.account_holder }}</span>
              </div>
              <div v-if="bank.coa_code" class="flex items-center justify-between text-[11px] pt-1.5 border-t border-slate-200/80">
                <span class="text-slate-400">Kode COA:</span>
                <span class="font-mono font-bold text-emerald-700 bg-emerald-50/80 px-1.5 py-0.5 rounded border border-emerald-200">{{ bank.coa_code }}</span>
              </div>
            </div>

            <!-- Outlet Scope & Notes -->
            <div class="flex items-center justify-between text-[11px] text-slate-500 my-2 px-0.5">
              <span class="flex items-center gap-1.5 font-medium">
                <Building2 class="w-3.5 h-3.5 text-slate-400" />
                {{ bank.outlet_name || 'Semua Outlet' }}
              </span>
              <span v-if="bank.notes" class="italic truncate max-w-[150px] text-slate-400" :title="bank.notes">
                {{ bank.notes }}
              </span>
            </div>

            <!-- Actions -->
            <div
              v-if="authStore.hasPermission(['accounting:write', 'master:write'])"
              class="pt-3.5 border-t border-slate-100 flex items-center justify-end gap-2"
            >
              <button
                @click="openEditBankModal(bank)"
                class="px-2.5 py-1.5 rounded-lg text-xs font-semibold text-slate-700 hover:text-slate-900 hover:bg-slate-100 border border-slate-200 transition flex items-center gap-1 cursor-pointer"
              >
                <PenLine class="w-3 h-3" />
                <span>Edit</span>
              </button>
              <button
                @click="deleteBankAccount(bank)"
                class="px-2.5 py-1.5 rounded-lg text-xs font-semibold text-rose-600 hover:text-rose-800 hover:bg-rose-50 border border-rose-200 transition flex items-center gap-1 cursor-pointer"
              >
                <Trash2 class="w-3 h-3" />
                <span>Hapus</span>
              </button>
            </div>
          </div>
        </div>

        <!-- Empty State -->
        <div
          v-else
          class="bg-white border border-slate-200 rounded-2xl p-12 text-center flex flex-col items-center justify-center gap-3 shadow-xs"
        >
          <div class="w-14 h-14 rounded-2xl bg-emerald-50 text-emerald-700 flex items-center justify-center border border-emerald-200">
            <Landmark class="w-7 h-7" />
          </div>
          <h3 class="font-bold text-slate-800 text-sm">Belum Ada Rekening Bank Penerimaan</h3>
          <p class="text-xs text-slate-500 max-w-md">
            Daftarkan rekening bank operasional usaha Anda (seperti BCA, Mandiri, BRI, dll) agar dapat menerima transfer dari pelanggan dan membukukan setoran modal.
          </p>
          <button
            v-if="authStore.hasPermission(['accounting:write', 'master:write'])"
            @click="openCreateBankModal()"
            class="mt-2 px-4 py-2 bg-emerald-700 hover:bg-emerald-800 text-white text-xs font-bold rounded-xl transition cursor-pointer shadow-sm flex items-center gap-2"
          >
            <Plus class="w-3.5 h-3.5" />
            <span>Tambah Rekening Pertama</span>
          </button>
        </div>
      </div>

      <!-- ===================================================================== -->
      <!-- TAB 5: CHART OF ACCOUNTS (COA) -->
      <!-- ===================================================================== -->
      <div v-else-if="activeTab === 'coa'" class="space-y-4">
        <!-- Informational banner about locked system accounts -->
        <div class="p-3.5 bg-amber-50 border border-amber-200 rounded-xl text-xs text-amber-900 flex flex-col sm:flex-row sm:items-center justify-between gap-3 shadow-xs">
          <div class="flex items-start sm:items-center gap-2.5">
            <Lock class="w-4 h-4 text-amber-700 shrink-0 mt-0.5 sm:mt-0" />
            <span>
              Akun bertanda <span class="font-bold text-amber-950">🔒 Sistem</span> adalah Bagan Akun bawaan terlindungi yang terhubung langsung dengan sistem POS, inventaris, dan jurnal otomatis.
            </span>
          </div>
          <button
            @click="switchTab('bank')"
            class="shrink-0 px-3 py-1.5 bg-amber-700 hover:bg-amber-800 text-white font-bold rounded-lg transition text-[11px] cursor-pointer flex items-center gap-1.5"
          >
            <Landmark class="w-3.5 h-3.5" />
            <span>Kelola Rekening Bank &rarr;</span>
          </button>
        </div>
        <!-- Toolbar -->
        <div class="flex flex-col sm:flex-row gap-3 items-start sm:items-center justify-between">
          <!-- Search -->
          <div class="relative flex-1 max-w-sm">
            <Search class="absolute left-3 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-slate-400 pointer-events-none" />
            <input
              v-model="coaSearch"
              type="text"
              placeholder="Cari kode / nama akun..."
              class="w-full pl-8 pr-3 py-2 text-xs bg-white border border-slate-300 rounded-xl focus:outline-none focus:border-emerald-600 text-slate-800 placeholder-slate-400"
            />
          </div>
          <!-- Action buttons -->
          <div class="flex items-center gap-2">
            <button
              @click="openPrintReport('coa')"
              class="flex items-center gap-1.5 px-3.5 py-2 bg-slate-900 hover:bg-slate-800 text-white text-xs font-bold rounded-xl transition cursor-pointer shadow-xs"
            >
              <Printer class="w-3.5 h-3.5 text-emerald-400" />
              <span>Cetak Bagan Akun (PDF)</span>
            </button>
            <button
              v-if="authStore.hasPermission(['accounting:write', 'master:write'])"
              @click="openCreateCoaModal()"
              class="flex items-center gap-2 px-4 py-2 bg-emerald-600 hover:bg-emerald-700 text-white text-xs font-bold rounded-xl transition cursor-pointer shadow-sm"
            >
              <Plus class="w-3.5 h-3.5" />
              <span>Tambah Akun</span>
            </button>
          </div>
        </div>

        <!-- Table -->
        <div class="bg-white border border-slate-200 rounded-2xl overflow-hidden shadow-xs">
          <table class="w-full text-left text-xs border-collapse">
            <thead class="bg-slate-100 border-b border-slate-200 text-slate-700 font-mono text-[11px] uppercase tracking-wider">
              <tr>
                <th class="py-3.5 px-4">Kode Akun</th>
                <th class="py-3.5 px-4">Nama Akun</th>
                <th class="py-3.5 px-4">Kategori Akun</th>
                <th class="py-3.5 px-4">Cakupan Akun</th>
                <th class="py-3.5 px-4 text-center">Status</th>
                <th class="py-3.5 px-4 text-center">Aksi</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-200 font-mono">
              <tr
                v-for="coa in paginatedFilteredCOA"
                :key="coa.id"
                class="hover:bg-emerald-50/40 transition"
                :class="{ 'opacity-50': !coa.is_active }"
              >
                <td class="py-3 px-4 font-bold text-sky-700">
                  <CopyableText :text="coa.code" label="Kode Akun" />
                </td>
                <td class="py-3 px-4 font-sans font-semibold text-slate-900">
                  <div class="flex items-center gap-2 flex-wrap">
                    <span>{{ coa.name }}</span>
                    <!-- System account lock badge -->
                    <span
                      v-if="coa.is_system"
                      class="inline-flex items-center gap-1 text-[11px] bg-amber-100 text-amber-900 border border-amber-300 font-semibold px-2 py-0.5 rounded shadow-xs"
                      title="Akun sistem bawaan — terlindungi dari perubahan"
                    >
                      <Lock class="w-3 h-3 text-amber-700" />
                      Sistem
                    </span>
                    <span v-if="coa.is_header" class="text-[10px] bg-slate-100 border border-slate-300 text-slate-600 px-1.5 py-0.5 rounded font-mono font-normal">
                      Header/Induk
                    </span>
                  </div>
                  <p v-if="coa.description" class="text-[11px] font-normal text-slate-400 font-sans mt-0.5">{{ coa.description }}</p>
                </td>
                <td class="py-3 px-4">
                  <span v-if="coa.account_category_id === 1" class="inline-flex items-center gap-1 text-emerald-700 font-bold text-[11px] bg-emerald-50 px-2 py-0.5 rounded-full border border-emerald-200">ASSET</span>
                  <span v-else-if="coa.account_category_id === 2" class="inline-flex items-center gap-1 text-amber-700 font-bold text-[11px] bg-amber-50 px-2 py-0.5 rounded-full border border-amber-200">LIABILITY</span>
                  <span v-else-if="coa.account_category_id === 3" class="inline-flex items-center gap-1 text-purple-700 font-bold text-[11px] bg-purple-50 px-2 py-0.5 rounded-full border border-purple-200">EQUITY</span>
                  <span v-else-if="coa.account_category_id === 4" class="inline-flex items-center gap-1 text-sky-700 font-bold text-[11px] bg-sky-50 px-2 py-0.5 rounded-full border border-sky-200">REVENUE</span>
                  <span v-else-if="coa.account_category_id === 5" class="inline-flex items-center gap-1 text-rose-700 font-bold text-[11px] bg-rose-50 px-2 py-0.5 rounded-full border border-rose-200">EXPENSE</span>
                </td>
                <td class="py-3 px-4">
                  <span class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-lg bg-slate-100 border border-slate-200 text-slate-700 font-sans font-medium text-[11px]">
                    Universal (Semua Cabang)
                  </span>
                </td>
                <td class="py-3 px-4 text-center">
                  <span v-if="coa.is_active" class="bg-emerald-50 text-emerald-800 border border-emerald-300 text-[10px] px-2.5 py-0.5 rounded-full font-bold">Aktif</span>
                  <span v-else class="bg-slate-100 text-slate-500 border border-slate-300 text-[10px] px-2.5 py-0.5 rounded-full font-bold">Nonaktif</span>
                </td>
                <td class="py-3 px-4 text-center">
                  <!-- System accounts: show lock indicator instead of edit button -->
                  <div
                    v-if="coa.is_system"
                    class="inline-flex items-center gap-1.5 px-2.5 py-1.5 text-[11px] font-semibold text-slate-600 bg-slate-100 border border-slate-300 rounded-lg cursor-not-allowed select-none shadow-xs"
                    title="Akun sistem bawaan — terlindungi dari perubahan"
                  >
                    <Lock class="w-3.5 h-3.5 text-amber-600" />
                    Terkunci
                  </div>
                  <!-- User-created accounts: show edit button -->
                  <button
                    v-else
                    @click="openEditCoaModal(coa)"
                    class="inline-flex items-center gap-1.5 px-2.5 py-1.5 text-[11px] font-bold text-slate-600 bg-slate-100 hover:bg-slate-200 border border-slate-300 rounded-lg transition cursor-pointer"
                    title="Edit akun"
                  >
                    <PenLine class="w-3 h-3" />
                    Edit
                  </button>
                </td>
              </tr>
              <tr v-if="paginatedFilteredCOA.length === 0">
                <td colspan="6" class="py-10 text-center text-slate-400 font-sans text-xs italic">
                  <BookOpen class="w-8 h-8 mx-auto mb-2 text-slate-300" />
                  Tidak ada akun ditemukan
                </td>
              </tr>
            </tbody>
          </table>
          <PaginationBar
            v-model:currentPage="coaPage"
            v-model:pageSize="coaPageSize"
            :totalItems="filteredCOA.length"
          />
        </div>
      </div>
      </template>

      <!-- ===================================================================== -->
      <!-- MODAL 1: FORMULIR TUTUP BUKU PERIODE -->
      <!-- ===================================================================== -->
      <div
        v-if="showClosingModal"
        class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/70 backdrop-blur-xs overflow-y-auto"
      >
        <div class="bg-white rounded-2xl border border-slate-200 shadow-2xl w-full max-w-lg overflow-hidden animate-scaleIn">
          <div class="px-6 py-4 bg-slate-900 text-white flex items-center justify-between">
            <div class="flex items-center gap-2.5">
              <Lock class="w-5 h-5 text-amber-400" />
              <div>
                <h3 class="font-extrabold text-sm">Formulir Penutupan Buku Periode</h3>
                <p class="text-[11px] text-slate-400">Menutup akun nominal dan memindahkan laba/rugi ke Laba Ditahan</p>
              </div>
            </div>
            <button @click="showClosingModal = false" class="text-slate-400 hover:text-white transition cursor-pointer">
              <X class="w-5 h-5" />
            </button>
          </div>

          <div class="p-6 space-y-4 text-xs">
            <!-- Lingkup Outlet -->
            <div>
              <label class="block font-bold text-slate-700 mb-1">Lingkup Outlet / Cabang</label>
              <SearchableSelect
                v-model="closingForm.outlet_id"
                api-endpoint="/outlets"
                secondary-key="code"
                placeholder="Semua Outlet (Konsolidasian)"
                search-placeholder="Cari nama atau kode cabang..."
                :allow-clear="true"
              />
            </div>

            <!-- Rentang Periode -->
            <div class="grid grid-cols-2 gap-3">
              <div>
                <label class="block font-bold text-slate-700 mb-1">Periode Awal *</label>
                <input
                  type="date"
                  v-model="closingForm.period_start"
                  class="w-full bg-white border border-slate-300 rounded-xl px-3 py-2 text-xs font-mono font-bold text-slate-800 focus:outline-none focus:border-emerald-600"
                />
              </div>
              <div>
                <label class="block font-bold text-slate-700 mb-1">Periode Akhir (Cut-off) *</label>
                <input
                  type="date"
                  v-model="closingForm.period_end"
                  class="w-full bg-white border border-slate-300 rounded-xl px-3 py-2 text-xs font-mono font-bold text-slate-800 focus:outline-none focus:border-emerald-600"
                />
              </div>
            </div>

            <!-- Catatan -->
            <div>
              <label class="block font-bold text-slate-700 mb-1">Catatan Penutupan Buku</label>
              <textarea
                v-model="closingForm.notes"
                rows="2"
                placeholder="Misal: Penutupan buku akhir bulan Maret 2026 setelah rekonsiliasi kas dan stok fisik..."
                class="w-full bg-white border border-slate-300 rounded-xl px-3 py-2 text-xs text-slate-800 focus:outline-none focus:border-emerald-600"
              ></textarea>
            </div>

            <div class="p-3 bg-amber-50 rounded-xl border border-amber-200 text-amber-900 text-[11px] leading-relaxed">
              <strong>Peringatan Standar Akuntansi:</strong> Seluruh transaksi (POS, pembelian, transfer stok, mutasi) yang bertanggal $\le$ <strong>{{ formatDate(closingForm.period_end) }}</strong> akan otomatis dikunci dan tidak dapat diubah kembali.
            </div>
          </div>

          <div class="px-6 py-3 bg-slate-100 border-t border-slate-200 flex items-center justify-between">
            <button
              type="button"
              @click="showClosingModal = false"
              class="px-4 py-2 rounded-xl text-slate-700 hover:bg-slate-200 text-xs font-bold transition cursor-pointer"
            >
              Batal
            </button>
            <button
              type="button"
              @click="handleExecuteClosing"
              :disabled="isSubmittingClosing"
              class="px-5 py-2 rounded-xl bg-amber-600 hover:bg-amber-700 text-white text-xs font-bold shadow-md transition active:scale-95 disabled:opacity-50 cursor-pointer flex items-center gap-1.5"
            >
              <Lock class="w-3.5 h-3.5" />
              <span>{{ isSubmittingClosing ? 'Memproses Penutupan...' : 'Konfirmasi & Kunci Periode' }}</span>
            </button>
          </div>
        </div>
      </div>

      <!-- ===================================================================== -->
      <!-- MODAL 2: FORMULIR BUKA KEMBALI BUKU (REOPEN) -->
      <!-- ===================================================================== -->
      <div
        v-if="showReopenModal"
        class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/70 backdrop-blur-xs overflow-y-auto"
      >
        <div class="bg-white rounded-2xl border border-slate-200 shadow-2xl w-full max-w-md overflow-hidden animate-scaleIn">
          <div class="px-6 py-4 bg-rose-900 text-white flex items-center justify-between">
            <div class="flex items-center gap-2">
              <Unlock class="w-5 h-5 text-rose-300" />
              <h3 class="font-extrabold text-sm">Buka Kembali Buku Periode (Reopen)</h3>
            </div>
            <button @click="showReopenModal = false" class="text-white/80 hover:text-white transition cursor-pointer">
              <X class="w-5 h-5" />
            </button>
          </div>

          <div class="p-6 space-y-4 text-xs">
            <p class="text-slate-600 leading-relaxed">
              Anda akan membuka kembali penutupan buku <strong>{{ selectedClosingToReopen?.closing_number }}</strong> (Periode: {{ formatDate(selectedClosingToReopen?.period_start) }} s/d {{ formatDate(selectedClosingToReopen?.period_end) }}).
            </p>

            <div class="p-3 bg-rose-50 rounded-xl border border-rose-200 text-rose-900 text-[11px] leading-relaxed">
              Jurnal penutup terkait akan dibatalkan sehingga penyesuaian transaksi dapat dilakukan. Sesuai standar audit, alasan pembukaan kembali wajib dicatat.
            </div>

            <div>
              <label class="block font-bold text-slate-700 mb-1">Alasan Pembukaan Kembali (Audit Trail) *</label>
              <textarea
                v-model="reopenReason"
                rows="3"
                placeholder="Misal: Penyesuaian audit pajak atau koreksi salah akun persediaan..."
                class="w-full bg-white border border-slate-300 rounded-xl px-3 py-2 text-xs text-slate-800 focus:outline-none focus:border-rose-600"
              ></textarea>
            </div>
          </div>

          <div class="px-6 py-3 bg-slate-100 border-t border-slate-200 flex items-center justify-between">
            <button
              type="button"
              @click="showReopenModal = false"
              class="px-4 py-2 rounded-xl text-slate-700 hover:bg-slate-200 text-xs font-bold transition cursor-pointer"
            >
              Batal
            </button>
            <button
              type="button"
              @click="handleExecuteReopen"
              :disabled="isSubmittingReopen"
              class="px-5 py-2 rounded-xl bg-rose-600 hover:bg-rose-700 text-white text-xs font-bold shadow-md transition active:scale-95 disabled:opacity-50 cursor-pointer flex items-center gap-1.5"
            >
              <Unlock class="w-3.5 h-3.5" />
              <span>{{ isSubmittingReopen ? 'Membuka...' : 'Buka Kembali Periode' }}</span>
            </button>
          </div>
        </div>
      </div>

      <!-- ===================================================================== -->
      <!-- MODAL: TAMBAH PERIODE AKUNTANSI BARU -->
      <!-- ===================================================================== -->
      <Teleport to="body">
      <div
        v-if="showPeriodModal"
        class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/70 backdrop-blur-xs overflow-y-auto"
        @click.self="showPeriodModal = false"
      >
        <div class="bg-white rounded-2xl border border-slate-200 shadow-2xl w-full max-w-md overflow-hidden animate-scaleIn">
          <div class="px-6 py-4 bg-emerald-800 text-white flex items-center justify-between">
            <div class="flex items-center gap-2.5">
              <Calendar class="w-5 h-5 text-emerald-300" />
              <div>
                <h3 class="font-extrabold text-sm">Tambah Periode Akuntansi Baru</h3>
                <p class="text-[11px] text-emerald-200">Daftarkan rentang periode fiskal pembukuan</p>
              </div>
            </div>
            <button @click="showPeriodModal = false" class="text-white/80 hover:text-white transition cursor-pointer">
              <X class="w-5 h-5" />
            </button>
          </div>

          <div class="p-6 space-y-4 text-xs">
            <div>
              <label class="block font-bold text-slate-700 mb-1">Nama Periode *</label>
              <input
                type="text"
                v-model="periodForm.name"
                placeholder="Contoh: Periode Maret 2026 atau Q1 2026"
                class="w-full bg-white border border-slate-300 rounded-xl px-3 py-2 text-xs font-bold text-slate-800 focus:outline-none focus:border-emerald-600"
              />
            </div>

            <div class="grid grid-cols-2 gap-3">
              <div>
                <label class="block font-bold text-slate-700 mb-1">Tanggal Mulai *</label>
                <input
                  type="date"
                  v-model="periodForm.start_date"
                  class="w-full bg-white border border-slate-300 rounded-xl px-3 py-2 text-xs font-mono font-bold text-slate-800 focus:outline-none focus:border-emerald-600"
                />
              </div>
              <div>
                <label class="block font-bold text-slate-700 mb-1">Tanggal Selesai *</label>
                <input
                  type="date"
                  v-model="periodForm.end_date"
                  class="w-full bg-white border border-slate-300 rounded-xl px-3 py-2 text-xs font-mono font-bold text-slate-800 focus:outline-none focus:border-emerald-600"
                />
              </div>
            </div>

            <div class="p-3 bg-emerald-50 rounded-xl border border-emerald-200 text-emerald-900 text-[11px] leading-relaxed">
              Periode akan dibuat dengan status <strong>TERBUKA</strong>. Anda dapat mengunci periode (Soft Close) sewaktu-waktu ketika proses rekonsiliasi telah selesai.
            </div>
          </div>

          <div class="px-6 py-3 bg-slate-100 border-t border-slate-200 flex items-center justify-between">
            <button
              type="button"
              @click="showPeriodModal = false"
              class="px-4 py-2 rounded-xl text-slate-700 hover:bg-slate-200 text-xs font-bold transition cursor-pointer"
            >
              Batal
            </button>
            <button
              type="button"
              @click="handleSavePeriod"
              :disabled="isSubmittingPeriod"
              class="px-5 py-2 rounded-xl bg-emerald-600 hover:bg-emerald-700 text-white text-xs font-bold shadow-md transition active:scale-95 disabled:opacity-50 cursor-pointer flex items-center gap-1.5"
            >
              <Plus class="w-3.5 h-3.5" />
              <span>{{ isSubmittingPeriod ? 'Menyimpan...' : 'Simpan Periode' }}</span>
            </button>
          </div>
        </div>
      </div>
      </Teleport>

      <!-- ===================================================================== -->
      <!-- MODAL: BUKA KUNCI PERIODE AKUNTANSI -->
      <!-- ===================================================================== -->
      <Teleport to="body">
      <div
        v-if="showReopenPeriodModal"
        class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/70 backdrop-blur-xs overflow-y-auto"
        @click.self="showReopenPeriodModal = false"
      >
        <div class="bg-white rounded-2xl border border-slate-200 shadow-2xl w-full max-w-md overflow-hidden animate-scaleIn">
          <div class="px-6 py-4 bg-amber-800 text-white flex items-center justify-between">
            <div class="flex items-center gap-2">
              <Unlock class="w-5 h-5 text-amber-300" />
              <h3 class="font-extrabold text-sm">Buka Kunci Periode Akuntansi</h3>
            </div>
            <button @click="showReopenPeriodModal = false" class="text-white/80 hover:text-white transition cursor-pointer">
              <X class="w-5 h-5" />
            </button>
          </div>

          <div class="p-6 space-y-4 text-xs">
            <p class="text-slate-600 leading-relaxed">
              Anda akan membuka kunci periode <strong>{{ selectedPeriodToReopen?.name }}</strong> ({{ formatDate(selectedPeriodToReopen?.start_date) }} s/d {{ formatDate(selectedPeriodToReopen?.end_date) }}).
            </p>

            <div class="p-3 bg-amber-50 rounded-xl border border-amber-200 text-amber-900 text-[11px] leading-relaxed">
              Setelah dibuka, transaksi mutasi pada rentang tanggal ini akan diizinkan kembali untuk dicatat. Wajib cantumkan alasan untuk jejak audit.
            </div>

            <div>
              <label class="block font-bold text-slate-700 mb-1">Alasan Pembukaan Kunci Periode *</label>
              <textarea
                v-model="periodReopenReason"
                rows="3"
                placeholder="Misal: Penyesuaian stok opname fisik susulan atau revisi invoice vendor..."
                class="w-full bg-white border border-slate-300 rounded-xl px-3 py-2 text-xs text-slate-800 focus:outline-none focus:border-amber-600"
              ></textarea>
            </div>
          </div>

          <div class="px-6 py-3 bg-slate-100 border-t border-slate-200 flex items-center justify-between">
            <button
              type="button"
              @click="showReopenPeriodModal = false"
              class="px-4 py-2 rounded-xl text-slate-700 hover:bg-slate-200 text-xs font-bold transition cursor-pointer"
            >
              Batal
            </button>
            <button
              type="button"
              @click="handleExecuteReopenPeriod"
              :disabled="isReopeningPeriod"
              class="px-5 py-2 rounded-xl bg-amber-600 hover:bg-amber-700 text-white text-xs font-bold shadow-md transition active:scale-95 disabled:opacity-50 cursor-pointer flex items-center gap-1.5"
            >
              <Unlock class="w-3.5 h-3.5" />
              <span>{{ isReopeningPeriod ? 'Membuka...' : 'Buka Kunci Periode' }}</span>
            </button>
          </div>
        </div>
      </div>
      </Teleport>

      <!-- ===================================================================== -->
      <!-- MODAL: TUTUP BUKU AKHIR TAHUN (HARD CLOSE) -->
      <!-- ===================================================================== -->
      <Teleport to="body">
      <div
        v-if="showHardCloseModal"
        class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/70 backdrop-blur-xs overflow-y-auto"
        @click.self="showHardCloseModal = false"
      >
        <div class="bg-white rounded-2xl border border-slate-200 shadow-2xl w-full max-w-lg overflow-hidden animate-scaleIn">
          <div class="px-6 py-4 bg-purple-900 text-white flex items-center justify-between">
            <div class="flex items-center gap-2.5">
              <Scale class="w-5 h-5 text-purple-300" />
              <div>
                <h3 class="font-extrabold text-sm">Tutup Buku Akhir Tahun (Hard Close)</h3>
                <p class="text-[11px] text-purple-200">Penutupan akun nominal tahunan ke Laba Ditahan (3201)</p>
              </div>
            </div>
            <button @click="showHardCloseModal = false" class="text-white/80 hover:text-white transition cursor-pointer">
              <X class="w-5 h-5" />
            </button>
          </div>

          <div class="p-6 space-y-4 text-xs">
            <div>
              <label class="block font-bold text-slate-700 mb-1">Tahun Buku (Fiscal Year) *</label>
              <input
                type="number"
                v-model.number="hardCloseForm.fiscal_year"
                min="2000"
                max="2100"
                class="w-full bg-white border border-slate-300 rounded-xl px-3 py-2 text-sm font-mono font-bold text-purple-950 focus:outline-none focus:border-purple-600"
              />
            </div>

            <div>
              <label class="block font-bold text-slate-700 mb-1">Keterangan / Catatan Transaksi</label>
              <input
                type="text"
                v-model="hardCloseForm.description"
                placeholder="Contoh: Hard Close Tutup Buku Tahunan Tahun 2026"
                class="w-full bg-white border border-slate-300 rounded-xl px-3 py-2 text-xs text-slate-800 focus:outline-none focus:border-purple-600"
              />
            </div>

            <div class="p-4 bg-purple-50 rounded-xl border border-purple-200 text-purple-950 space-y-2 text-[11px] leading-relaxed">
              <div class="font-bold flex items-center gap-1.5 text-purple-900 text-xs">
                <CheckCircle2 class="w-4 h-4 text-purple-700" />
                Mekanisme Akuntansi Penutupan Tahunan:
              </div>
              <ul class="list-disc list-inside space-y-1 text-slate-700">
                <li>Sistem membaca total saldo seluruh akun <strong>Pendapatan (Revenue)</strong> dan <strong>Beban (Expense)</strong> selama 1 Januari s/d 31 Desember tahun {{ hardCloseForm.fiscal_year }}.</li>
                <li>Membuat satu transaksi jurnal resmi dengan flag <code>is_closing_entry = true</code>.</li>
                <li>Mendebit akun Pendapatan dan mengkredit akun Beban sehingga saldo nominal menjadi 0.</li>
                <li>Selisih Laba/Rugi Bersih otomatis dipindahkan ke akun <strong>Laba Ditahan (3201)</strong> pada kelompok Ekuitas.</li>
              </ul>
            </div>
          </div>

          <div class="px-6 py-3 bg-slate-100 border-t border-slate-200 flex items-center justify-between">
            <button
              type="button"
              @click="showHardCloseModal = false"
              class="px-4 py-2 rounded-xl text-slate-700 hover:bg-slate-200 text-xs font-bold transition cursor-pointer"
            >
              Batal
            </button>
            <button
              type="button"
              @click="handleExecuteHardClose"
              :disabled="isSubmittingHardClose"
              class="px-5 py-2 rounded-xl bg-purple-700 hover:bg-purple-800 text-white text-xs font-bold shadow-md transition active:scale-95 disabled:opacity-50 cursor-pointer flex items-center gap-1.5"
            >
              <Scale class="w-3.5 h-3.5" />
              <span>{{ isSubmittingHardClose ? 'Memproses...' : 'Eksekusi Tutup Buku Tahunan' }}</span>
            </button>
          </div>
        </div>
      </div>
      </Teleport>

      <!-- ===================================================================== -->
      <!-- MODAL 3: FORMULIR JURNAL PENYESUAIAN (MODE PERUNTUKAN & MANUAL) -->
      <!-- ===================================================================== -->
      <div
        v-if="showManualJournalModal"
        class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/70 backdrop-blur-xs overflow-y-auto"
      >
        <div class="bg-white rounded-3xl border border-slate-200 shadow-2xl w-full max-w-3xl overflow-hidden flex flex-col max-h-[92vh] animate-scaleIn">
          <!-- Header with Mode Switcher -->
          <div class="px-6 py-4 bg-slate-900 text-white flex flex-wrap items-center justify-between gap-3 shrink-0">
            <div class="flex items-center gap-2.5">
              <div class="w-8 h-8 rounded-xl bg-emerald-700 flex items-center justify-center text-emerald-300">
                <Sparkles class="w-4 h-4" />
              </div>
              <div>
                <h3 class="font-extrabold text-sm">Pencatatan Jurnal Penyesuaian Toko</h3>
                <p class="text-[11px] text-slate-400">Penyesuaian kasir, stok opname, beban toko, atau koreksi pembukuan</p>
              </div>
            </div>

            <!-- Mode Selector Switch -->
            <div class="flex items-center gap-3">
              <div class="inline-flex p-1 bg-slate-800 rounded-xl border border-slate-700">
                <button
                  type="button"
                  @click="adjustmentMode = 'purpose'"
                  class="px-3 py-1.5 rounded-lg text-xs font-bold transition flex items-center gap-1.5 cursor-pointer"
                  :class="adjustmentMode === 'purpose' ? 'bg-emerald-600 text-white shadow-xs' : 'text-slate-400 hover:text-white'"
                >
                  <Sparkles class="w-3.5 h-3.5 text-amber-300" />
                  <span>Sesuai Peruntukan (Mudah)</span>
                </button>
                <button
                  type="button"
                  @click="adjustmentMode = 'manual'"
                  class="px-3 py-1.5 rounded-lg text-xs font-bold transition flex items-center gap-1.5 cursor-pointer"
                  :class="adjustmentMode === 'manual' ? 'bg-emerald-600 text-white shadow-xs' : 'text-slate-400 hover:text-white'"
                >
                  <Sliders class="w-3.5 h-3.5 text-sky-300" />
                  <span>Mode Manual (Bebas)</span>
                </button>
              </div>

              <button @click="showManualJournalModal = false" class="text-slate-400 hover:text-white transition cursor-pointer p-1 rounded-lg hover:bg-slate-800">
                <X class="w-5 h-5" />
              </button>
            </div>
          </div>

          <!-- Body Container -->
          <div class="p-6 overflow-y-auto space-y-4 flex-1 text-xs">

            <!-- MODE 1: SESUAI PERUNTUKAN (MUDAH UNTUK PENGGUNA RETAIL) -->
            <div v-if="adjustmentMode === 'purpose'" class="space-y-4">
              <!-- Info Box: Pemisahan dengan Menu Pengeluaran & Stock Opname -->
              <div class="p-3 bg-amber-50/90 border border-amber-200 rounded-xl text-amber-950 text-xs flex items-start gap-2.5">
                <Info class="w-4 h-4 text-amber-700 shrink-0 mt-0.5" />
                <p class="leading-relaxed text-[11px]">
                  <strong>Panduan Toko:</strong> Pengeluaran kas harian (listrik, belanja kantong plastik, servis, konsumsi, bensin) dicatat di menu <strong>Biaya &amp; Pengeluaran</strong>. Penyesuaian barang rusak / hilang fisik dicatat di menu <strong>Stok &amp; Opname</strong>. Formulir penyesuaian di bawah ini khusus untuk selisih fisik kasir dan jurnal akuntansi penutupan periode (penyusutan aset, amortisasi sewa, akrual, rekonsiliasi bank).
                </p>
              </div>

              <!-- Pilihan Kategori Peruntukan (Clickable Cards) -->
              <div>
                <label class="block font-bold text-slate-800 mb-2 text-xs">
                  Pilih Kebutuhan Penyesuaian Toko:
                </label>
                <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 gap-2.5">
                  <button
                    v-for="cat in adjustmentCategories"
                    :key="cat.id"
                    type="button"
                    @click="onPurposeCategoryChange(cat.id)"
                    class="p-2.5 rounded-xl border text-left transition flex flex-col justify-between cursor-pointer relative"
                    :class="purposeForm.category === cat.id
                      ? 'bg-emerald-50/80 border-emerald-600 ring-2 ring-emerald-500/20 text-emerald-950 shadow-2xs'
                      : 'bg-white border-slate-200 hover:border-slate-300 hover:bg-slate-50/50 text-slate-700'"
                  >
                    <div>
                      <div class="flex items-center justify-between mb-1.5">
                        <span class="text-sm">{{ cat.icon }}</span>
                        <div class="flex items-center gap-1">
                          <span
                            class="px-1.5 py-0.5 rounded-md text-[10px] font-bold"
                            :class="purposeForm.category === cat.id ? 'bg-emerald-200 text-emerald-900' : 'bg-slate-100 text-slate-600'"
                          >
                            {{ cat.badge }}
                          </span>
                          <Check v-if="purposeForm.category === cat.id" class="w-3.5 h-3.5 text-emerald-700" />
                        </div>
                      </div>
                      <p class="font-bold text-xs leading-tight mb-1" :class="purposeForm.category === cat.id ? 'text-emerald-950' : 'text-slate-800'">
                        {{ cat.name }}
                      </p>
                    </div>
                    <p class="text-[10px] line-clamp-2 text-slate-500 leading-snug">
                      {{ cat.description }}
                    </p>
                  </button>
                </div>
              </div>

              <!-- Tipe Selisih Kas (Jika Kategori CASH_DIFF) -->
              <div v-if="purposeForm.category === 'CASH_DIFF'" class="p-3 bg-amber-50/80 border border-amber-200 rounded-xl space-y-2">
                <span class="block text-xs font-bold text-amber-950">Kondisi Fisik Kas Laci Kasir:</span>
                <div class="grid grid-cols-1 sm:grid-cols-2 gap-2">
                  <button
                    type="button"
                    @click="onCashDiffTypeChange('SHORTAGE')"
                    class="p-2.5 rounded-xl border text-left flex items-start gap-2.5 transition cursor-pointer"
                    :class="purposeForm.cash_diff_type === 'SHORTAGE'
                      ? 'bg-white border-rose-500 ring-2 ring-rose-300 text-rose-950 shadow-xs'
                      : 'bg-amber-100/40 border-amber-200 text-slate-700 hover:bg-white'"
                  >
                    <span class="text-base">🔻</span>
                    <div>
                      <p class="font-bold text-xs">Uang Kasir Kurang (Shortage)</p>
                      <p class="text-[10px] text-slate-500">Fisik di laci lebih sedikit dari sistem -> Dicatat Beban Selisih Kasir</p>
                    </div>
                  </button>

                  <button
                    type="button"
                    @click="onCashDiffTypeChange('SURPLUS')"
                    class="p-2.5 rounded-xl border text-left flex items-start gap-2.5 transition cursor-pointer"
                    :class="purposeForm.cash_diff_type === 'SURPLUS'
                      ? 'bg-white border-emerald-500 ring-2 ring-emerald-300 text-emerald-950 shadow-xs'
                      : 'bg-amber-100/40 border-amber-200 text-slate-700 hover:bg-white'"
                  >
                    <span class="text-base">🔺</span>
                    <div>
                      <p class="font-bold text-xs">Uang Kasir Lebih (Surplus)</p>
                      <p class="text-[10px] text-slate-500">Fisik di laci lebih banyak dari sistem -> Dicatat Pendapatan Selisih Kasir</p>
                    </div>
                  </button>
                </div>
              </div>

              <!-- Tipe Rekonsiliasi Bank (Jika Kategori BANK_RECON) -->
              <div v-if="purposeForm.category === 'BANK_RECON'" class="p-3 bg-sky-50/80 border border-sky-200 rounded-xl space-y-2">
                <span class="block text-xs font-bold text-sky-950">Jenis Transaksi Rekonsiliasi Bank:</span>
                <div class="grid grid-cols-1 sm:grid-cols-2 gap-2">
                  <button
                    type="button"
                    @click="onBankReconTypeChange('INTEREST')"
                    class="p-2.5 rounded-xl border text-left flex items-start gap-2.5 transition cursor-pointer"
                    :class="purposeForm.bank_recon_type === 'INTEREST'
                      ? 'bg-white border-emerald-600 ring-2 ring-emerald-300 text-emerald-950 shadow-xs'
                      : 'bg-sky-100/40 border-sky-200 text-slate-700 hover:bg-white'"
                  >
                    <span class="text-base">💰</span>
                    <div>
                      <p class="font-bold text-xs">Bunga Tabungan / Jasa Giro (+ Saldo Bank)</p>
                      <p class="text-[10px] text-slate-500">Menambah saldo rekening bank, dicatat Pendapatan Bunga Bank</p>
                    </div>
                  </button>

                  <button
                    type="button"
                    @click="onBankReconTypeChange('ADMIN')"
                    class="p-2.5 rounded-xl border text-left flex items-start gap-2.5 transition cursor-pointer"
                    :class="purposeForm.bank_recon_type === 'ADMIN'
                      ? 'bg-white border-rose-600 ring-2 ring-rose-300 text-rose-950 shadow-xs'
                      : 'bg-sky-100/40 border-sky-200 text-slate-700 hover:bg-white'"
                  >
                    <span class="text-base">🏦</span>
                    <div>
                      <p class="font-bold text-xs">Biaya Administrasi Bank (- Saldo Bank)</p>
                      <p class="text-[10px] text-slate-500">Memotong saldo rekening bank, dicatat Beban Administrasi Bank</p>
                    </div>
                  </button>
                </div>
              </div>

              <!-- Tipe Beban Akrual (Jika Kategori ACCRUED_EXPENSE) -->
              <div v-if="purposeForm.category === 'ACCRUED_EXPENSE'" class="p-3 bg-purple-50/80 border border-purple-200 rounded-xl space-y-2">
                <span class="block text-xs font-bold text-purple-950">Jenis Beban Akrual (Belum Dibayar):</span>
                <div class="grid grid-cols-1 sm:grid-cols-2 gap-2">
                  <button
                    type="button"
                    @click="onAccruedTypeChange('UTILITY')"
                    class="p-2.5 rounded-xl border text-left flex items-start gap-2.5 transition cursor-pointer"
                    :class="purposeForm.accrued_type === 'UTILITY'
                      ? 'bg-white border-purple-600 ring-2 ring-purple-300 text-purple-950 shadow-xs'
                      : 'bg-purple-100/40 border-purple-200 text-slate-700 hover:bg-white'"
                  >
                    <span class="text-base">⚡</span>
                    <div>
                      <p class="font-bold text-xs">Tagihan Listrik / Air / Internet</p>
                      <p class="text-[10px] text-slate-500">Biaya PLN/PDAM/WiFi bulan ini yang baru dibayar awal bulan depan</p>
                    </div>
                  </button>

                  <button
                    type="button"
                    @click="onAccruedTypeChange('SALARY')"
                    class="p-2.5 rounded-xl border text-left flex items-start gap-2.5 transition cursor-pointer"
                    :class="purposeForm.accrued_type === 'SALARY'
                      ? 'bg-white border-purple-600 ring-2 ring-purple-300 text-purple-950 shadow-xs'
                      : 'bg-purple-100/40 border-purple-200 text-slate-700 hover:bg-white'"
                  >
                    <span class="text-base">👥</span>
                    <div>
                      <p class="font-bold text-xs">Hutang Gaji / Lembur Karyawan</p>
                      <p class="text-[10px] text-slate-500">Gaji/lembur staf bulan berjalan yang baru ditransfer pada tanggal gajian</p>
                    </div>
                  </button>
                </div>
              </div>

              <!-- Input Detail Penyesuaian -->
              <div class="grid grid-cols-1 md:grid-cols-2 gap-4 bg-slate-50 p-4 rounded-2xl border border-slate-200">
                <div>
                  <label class="block font-bold text-slate-700 mb-1">Tanggal Transaksi / Penyesuaian *</label>
                  <input
                    type="date"
                    v-model="purposeForm.journal_date"
                    class="w-full bg-white border border-slate-300 rounded-xl px-3 py-2 font-mono font-bold text-slate-900 focus:outline-none focus:border-emerald-600"
                  />
                </div>

                <div>
                  <label class="block font-bold text-slate-700 mb-1">Cabang / Outlet Terkait</label>
                  <SearchableSelect
                    v-model="purposeForm.outlet_id"
                    api-endpoint="/outlets"
                    secondary-key="code"
                    placeholder="Semua Outlet (Konsolidasian)"
                    search-placeholder="Cari cabang..."
                    :allow-clear="true"
                  />
                </div>

                <!-- Rekening Kas / Bank (Jika Kategori Memerlukan Kas/Bank) -->
                <div v-if="purposeJournalPreview.needsPaymentAccount">
                  <label class="block font-bold text-slate-700 mb-1">
                    Rekening Kas / Bank Terkait *
                  </label>
                  <SearchableSelect
                    v-model="purposeForm.payment_account_id"
                    :options="businessPaymentAccountOptions"
                    secondary-key="code"
                    placeholder="-- Pilih Rekening Kas / Bank --"
                    search-placeholder="Cari kas atau bank..."
                    :allow-clear="false"
                  />
                  <p class="mt-1 text-[10px] text-slate-400">
                    Kas operasional laci kasir atau rekening bank tempat uang dicairkan/disetor
                  </p>
                </div>

                <div :class="!purposeJournalPreview.needsPaymentAccount ? 'md:col-span-2' : ''">
                  <label class="block font-bold text-slate-700 mb-1">Nominal Penyesuaian (Rp) *</label>
                  <div class="relative">
                    <span class="absolute left-3 top-1/2 -translate-y-1/2 font-bold text-xs text-slate-400">Rp</span>
                    <input
                      type="number"
                      v-model.number="purposeForm.amount"
                      min="1"
                      placeholder="0"
                      class="w-full bg-white border border-slate-300 rounded-xl pl-9 pr-3 py-2 text-sm font-mono font-black text-emerald-700 focus:outline-none focus:border-emerald-600"
                    />
                  </div>
                </div>

                <div class="md:col-span-2">
                  <label class="block font-bold text-slate-700 mb-1">Keterangan / Alasan Penyesuaian *</label>
                  <input
                    type="text"
                    v-model="purposeForm.description"
                    placeholder="Tuliskan keterangan detail transaksi..."
                    class="w-full bg-white border border-slate-300 rounded-xl px-3 py-2 text-slate-800 focus:outline-none focus:border-emerald-600"
                  />
                </div>
              </div>

              <!-- Live Auto-Journal Preview Card -->
              <div class="p-4 rounded-2xl bg-gradient-to-br from-slate-900 to-slate-800 text-white shadow-md space-y-3">
                <div class="flex items-center justify-between border-b border-slate-700 pb-2.5">
                  <div class="flex items-center gap-2">
                    <BookOpen class="w-4 h-4 text-emerald-400" />
                    <span class="font-extrabold text-xs tracking-wide uppercase text-slate-200">
                      Pratinjau Jurnal Otomatis (Double-Entry Balanced)
                    </span>
                  </div>
                  <span class="px-2 py-0.5 rounded-full text-[10px] font-mono font-bold bg-emerald-500/20 text-emerald-300 border border-emerald-500/30">
                    Otomatis Seimbang ✓
                  </span>
                </div>

                <div class="grid grid-cols-1 sm:grid-cols-2 gap-3 text-xs">
                  <!-- Debet -->
                  <div class="bg-slate-800/90 border border-emerald-500/30 rounded-xl p-3">
                    <div class="flex items-center justify-between mb-1">
                      <span class="text-[10px] font-bold uppercase tracking-wider text-emerald-400">🟢 DEBET (Bertambah)</span>
                      <span class="font-mono font-bold text-emerald-300">{{ formatCurrency(purposeForm.amount || 0) }}</span>
                    </div>
                    <p class="font-bold text-white text-xs truncate">
                      {{ purposeJournalPreview.debitAccount.name }}
                    </p>
                  </div>

                  <!-- Kredit -->
                  <div class="bg-slate-800/90 border border-blue-500/30 rounded-xl p-3">
                    <div class="flex items-center justify-between mb-1">
                      <span class="text-[10px] font-bold uppercase tracking-wider text-sky-400">🔵 KREDIT (Berkurang / Lawan)</span>
                      <span class="font-mono font-bold text-sky-300">{{ formatCurrency(purposeForm.amount || 0) }}</span>
                    </div>
                    <p class="font-bold text-white text-xs truncate">
                      {{ purposeJournalPreview.creditAccount.name }}
                    </p>
                  </div>
                </div>

                <p class="text-[11px] text-slate-300 leading-relaxed italic bg-slate-950/40 p-2.5 rounded-lg border border-slate-700/50">
                  💡 {{ purposeJournalPreview.explanation }}
                </p>
              </div>
            </div>

            <!-- MODE 2: MODE MANUAL / AKUNTAN (DEBET & KREDIT BEBAS) -->
            <div v-else class="space-y-4">
              <div class="grid grid-cols-1 md:grid-cols-3 gap-3">
                <div>
                  <label class="block font-bold text-slate-700 mb-1">Tanggal Jurnal *</label>
                  <input
                    type="date"
                    v-model="manualJournalForm.journal_date"
                    class="w-full bg-white border border-slate-300 rounded-xl px-3 py-2 font-mono font-bold text-slate-800 focus:outline-none focus:border-emerald-600"
                  />
                </div>

                <div>
                  <label class="block font-bold text-slate-700 mb-1">Outlet Terkait (Opsional)</label>
                  <SearchableSelect
                    v-model="manualJournalForm.outlet_id"
                    api-endpoint="/outlets"
                    secondary-key="code"
                    placeholder="Semua Outlet / Umum"
                    search-placeholder="Cari cabang..."
                    :allow-clear="true"
                  />
                </div>

                <div>
                  <label class="block font-bold text-slate-700 mb-1">Jenis Jurnal</label>
                  <select
                    v-model="manualJournalForm.reference_type"
                    class="w-full bg-white border border-slate-300 rounded-xl px-3 py-2 font-bold text-slate-800 focus:outline-none focus:border-emerald-600"
                  >
                    <option value="MANUAL_ADJUSTMENT">Jurnal Penyesuaian (Adjustment)</option>
                    <option value="PRIOR_PERIOD_CORRECTION">Koreksi Periode Lalu (Prior Period)</option>
                    <option value="OPENING_BALANCE">Saldo Awal / Modal</option>
                  </select>
                </div>

                <div class="md:col-span-3">
                  <label class="block font-bold text-slate-700 mb-1">Keterangan / Alasan Penyesuaian *</label>
                  <input
                    v-model="manualJournalForm.description"
                    placeholder="Misal: Penyesuaian saldo piutang tak tertagih atau koreksi laba ditahan..."
                    class="w-full bg-white border border-slate-300 rounded-xl px-3 py-2 text-slate-800 focus:outline-none focus:border-emerald-600"
                  />
                </div>
              </div>

              <!-- Table Lines -->
              <div class="space-y-2 pt-2">
                <div class="flex items-center justify-between">
                  <span class="font-bold text-slate-700 uppercase font-mono text-[11px]">Rincian Akun Debet & Kredit:</span>
                  <button
                    type="button"
                    @click="addManualJournalLine"
                    class="px-2.5 py-1 rounded-lg bg-emerald-50 hover:bg-emerald-100 text-emerald-800 border border-emerald-200 font-bold text-xs flex items-center gap-1 transition cursor-pointer"
                  >
                    <Plus class="w-3.5 h-3.5" />
                    <span>Tambah Baris</span>
                  </button>
                </div>

                <div class="border border-slate-300 rounded-xl overflow-hidden bg-white shadow-2xs">
                  <table class="w-full text-left text-xs border-collapse">
                    <thead class="bg-slate-100 border-b border-slate-300 text-slate-700 font-mono text-[11px] uppercase">
                      <tr>
                        <th class="py-2.5 px-3 w-64">Pilih Akun (COA) *</th>
                        <th class="py-2.5 px-3 text-right w-36">Debet (Rp)</th>
                        <th class="py-2.5 px-3 text-right w-36">Kredit (Rp)</th>
                        <th class="py-2.5 px-3">Memo Baris</th>
                        <th class="py-2.5 px-3 text-center w-12">Hapus</th>
                      </tr>
                    </thead>
                    <tbody class="divide-y divide-slate-200">
                      <tr v-for="(line, idx) in manualJournalForm.lines" :key="idx" class="hover:bg-slate-50 transition">
                        <td class="py-2 px-3">
                          <SearchableSelect
                            v-model="line.account_id"
                            :options="postableCOA"
                            secondary-key="code"
                            placeholder="-- Pilih Akun COA --"
                            search-placeholder="Cari kode atau nama akun..."
                            :allow-clear="false"
                          />
                        </td>
                        <td class="py-2 px-3 text-right">
                          <input
                            type="number"
                            min="0"
                            v-model.number="line.debit"
                            placeholder="0"
                            class="w-full text-right bg-white border border-slate-300 rounded-lg px-2 py-1 font-mono text-xs font-bold text-slate-900 focus:outline-none focus:border-emerald-600"
                          />
                        </td>
                        <td class="py-2 px-3 text-right">
                          <input
                            type="number"
                            min="0"
                            v-model.number="line.credit"
                            placeholder="0"
                            class="w-full text-right bg-white border border-slate-300 rounded-lg px-2 py-1 font-mono text-xs font-bold text-slate-900 focus:outline-none focus:border-emerald-600"
                          />
                        </td>
                        <td class="py-2 px-3">
                          <input
                            v-model="line.memo"
                            placeholder="Keterangan baris..."
                            class="w-full bg-white border border-slate-300 rounded-lg px-2 py-1 text-xs text-slate-800 focus:outline-none focus:border-emerald-600"
                          />
                        </td>
                        <td class="py-2 px-3 text-center">
                          <button
                            type="button"
                            @click="removeManualJournalLine(idx)"
                            :disabled="manualJournalForm.lines.length <= 2"
                            class="p-1 text-slate-400 hover:text-rose-600 transition disabled:opacity-30 cursor-pointer"
                          >
                            <Trash2 class="w-4 h-4" />
                          </button>
                        </td>
                      </tr>
                    </tbody>
                    <tfoot class="bg-slate-50 border-t border-slate-300 font-bold font-mono">
                      <tr>
                        <td class="py-2.5 px-3 text-right uppercase text-[11px] text-slate-600">Total Keseimbangan:</td>
                        <td class="py-2.5 px-3 text-right text-emerald-800 text-xs">{{ formatCurrency(manualJournalTotalDebit) }}</td>
                        <td class="py-2.5 px-3 text-right text-emerald-800 text-xs">{{ formatCurrency(manualJournalTotalCredit) }}</td>
                        <td colspan="2" class="py-2.5 px-3 text-center">
                          <span
                            class="px-2 py-0.5 rounded-full text-[10px] font-bold border"
                            :class="isManualJournalBalanced ? 'bg-emerald-100 text-emerald-800 border-emerald-300' : 'bg-rose-100 text-rose-800 border-rose-300'"
                          >
                            {{ isManualJournalBalanced ? 'Seimbang (Balanced) ✓' : 'Belum Seimbang !' }}
                          </span>
                        </td>
                      </tr>
                    </tfoot>
                  </table>
                </div>
              </div>
            </div>
          </div>

          <!-- Footer Actions -->
          <div class="px-6 py-3.5 bg-slate-100 border-t border-slate-200 flex items-center justify-between shrink-0">
            <button
              type="button"
              @click="showManualJournalModal = false"
              class="px-4 py-2 rounded-xl text-slate-700 hover:bg-slate-200 text-xs font-bold transition cursor-pointer"
            >
              Batal
            </button>

            <!-- Mode Purpose Submit -->
            <button
              v-if="adjustmentMode === 'purpose'"
              type="button"
              @click="handleSavePurposeAdjustment"
              :disabled="isSubmittingManualJournal || !purposeForm.amount || purposeForm.amount <= 0"
              class="px-5 py-2.5 rounded-xl bg-emerald-600 hover:bg-emerald-700 text-white text-xs font-bold shadow-md transition active:scale-95 disabled:opacity-50 cursor-pointer flex items-center gap-2"
            >
              <CheckCircle2 class="w-4 h-4" />
              <span>{{ isSubmittingManualJournal ? 'Menyimpan...' : 'Simpan & Bukukan Penyesuaian' }}</span>
            </button>

            <!-- Mode Manual Submit -->
            <button
              v-else
              type="button"
              @click="handleSaveManualJournal"
              :disabled="isSubmittingManualJournal || !isManualJournalBalanced"
              class="px-5 py-2.5 rounded-xl bg-emerald-600 hover:bg-emerald-700 text-white text-xs font-bold shadow-md transition active:scale-95 disabled:opacity-50 cursor-pointer flex items-center gap-2"
            >
              <CheckCircle2 class="w-4 h-4" />
              <span>{{ isSubmittingManualJournal ? 'Menyimpan...' : 'Simpan & Bukukan Jurnal Manual' }}</span>
            </button>
          </div>
        </div>
      </div>

      <!-- ===================================================================== -->
      <!-- MODAL: TRANSFER KAS ANTAR CABANG (REKENING ANTAR-KANTOR) -->
      <!-- ===================================================================== -->
      <div
        v-if="showFundTransferModal"
        class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/70 backdrop-blur-xs overflow-y-auto"
      >
        <div class="bg-white rounded-2xl border border-slate-200 shadow-2xl w-full max-w-2xl overflow-hidden flex flex-col max-h-[90vh] animate-scaleIn">
          <div class="px-6 py-4 bg-gradient-to-r from-blue-900 to-indigo-900 text-white flex items-center justify-between shrink-0">
            <div class="flex items-center gap-2.5">
              <ArrowLeftRight class="w-5 h-5 text-sky-400" />
              <div>
                <h3 class="font-extrabold text-sm">Transfer Kas / Bank &amp; Antar-Cabang</h3>
                <p class="text-[11px] text-blue-200">Mutasi kas/bank internal atau kliring antar-cabang via RAK</p>
              </div>
            </div>
            <button @click="showFundTransferModal = false" class="text-blue-200 hover:text-white transition cursor-pointer">
              <X class="w-5 h-5" />
            </button>
          </div>

          <div class="p-6 overflow-y-auto space-y-4 flex-1 text-xs">
            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div>
                <label class="block font-bold text-slate-700 mb-1">Tanggal Transfer *</label>
                <input
                  type="date"
                  v-model="fundTransferForm.transfer_date"
                  class="w-full bg-white border border-slate-300 rounded-xl px-3 py-2 font-mono font-bold text-slate-800 focus:outline-none focus:border-blue-600"
                />
              </div>

              <div>
                <label class="block font-bold text-slate-700 mb-1">No. Referensi / Bukti Transfer (Opsional)</label>
                <input
                  type="text"
                  v-model="fundTransferForm.reference_no"
                  placeholder="Misal: TRF-BCA-20261013-01"
                  class="w-full bg-white border border-slate-300 rounded-xl px-3 py-2 font-mono text-slate-800 focus:outline-none focus:border-blue-600"
                />
              </div>
            </div>

            <!-- Outlet Asal & Tujuan Card -->
            <div class="p-4 bg-slate-50 border border-slate-200 rounded-2xl grid grid-cols-1 md:grid-cols-2 gap-4 relative">
              <!-- From Outlet -->
              <div class="space-y-3">
                <div class="flex items-center gap-1.5 text-xs font-bold text-rose-700">
                  <span class="w-2 h-2 rounded-full bg-rose-500"></span>
                  <span>Cabang Pengirim (Sumber Dana)</span>
                </div>
                <div>
                  <label class="block font-semibold text-slate-600 mb-1">Cabang Asal *</label>
                  <SearchableSelect
                    v-model="fundTransferForm.from_outlet_id"
                    api-endpoint="/me/outlets"
                    secondary-key="code"
                    placeholder="-- Pilih Cabang Asal --"
                    search-placeholder="Cari cabang pengirim..."
                    :allow-clear="false"
                  />
                </div>
                <div>
                  <div class="flex items-center justify-between mb-1">
                    <label class="block font-semibold text-slate-600">Rekening Kas/Bank Asal *</label>
                    <div v-if="fundTransferForm.from_account_id" class="text-xs">
                      <span v-if="isLoadingBalance" class="text-slate-400 italic">Memeriksa saldo...</span>
                      <span
                        v-else-if="fromAccountBalance !== null"
                        :class="fromAccountBalance > 0 ? 'text-emerald-700 font-bold' : 'text-rose-600 font-bold'"
                      >
                        Saldo: {{ formatCurrency(fromAccountBalance) }}
                      </span>
                    </div>
                  </div>
                  <SearchableSelect
                    v-model="fundTransferForm.from_account_id"
                    :options="fromOutletPaymentAccounts"
                    secondary-key="code"
                    placeholder="-- Pilih Rekening Kas/Bank --"
                    search-placeholder="Cari kode atau nama kas/bank..."
                    :allow-clear="false"
                  />
                  <!-- Warning if insufficient balance -->
                  <p
                    v-if="!isLoadingBalance && fromAccountBalance !== null && fromAccountBalance <= 0 && fundTransferForm.from_account_id"
                    class="text-[11px] text-rose-600 font-medium mt-1 flex items-center gap-1"
                  >
                    <AlertTriangle class="w-3.5 h-3.5 shrink-0" />
                    <span>Saldo kas/bank asal kosong atau tidak mencukupi ({{ formatCurrency(fromAccountBalance) }}).</span>
                  </p>
                  <p
                    v-else-if="!isLoadingBalance && fromAccountBalance !== null && fundTransferForm.amount > fromAccountBalance"
                    class="text-[11px] text-amber-600 font-medium mt-1 flex items-center gap-1"
                  >
                    <AlertTriangle class="w-3.5 h-3.5 shrink-0" />
                    <span>Nominal transfer melebihi saldo kas/bank asal (Maks: {{ formatCurrency(fromAccountBalance) }}).</span>
                  </p>
                </div>
              </div>

              <!-- To Outlet -->
              <div class="space-y-3">
                <div class="flex items-center gap-1.5 text-xs font-bold text-emerald-700">
                  <span class="w-2 h-2 rounded-full bg-emerald-500"></span>
                  <span>Cabang Penerima (Tujuan Dana)</span>
                </div>
                <div>
                  <label class="block font-semibold text-slate-600 mb-1">Cabang Tujuan *</label>
                  <SearchableSelect
                    v-model="fundTransferForm.to_outlet_id"
                    api-endpoint="/outlets"
                    secondary-key="code"
                    placeholder="-- Pilih Cabang Tujuan --"
                    search-placeholder="Cari cabang penerima..."
                    :allow-clear="false"
                  />
                </div>
                <div>
                  <label class="block font-semibold text-slate-600 mb-1">Rekening Kas/Bank Tujuan *</label>
                  <SearchableSelect
                    v-model="fundTransferForm.to_account_id"
                    :options="toOutletPaymentAccounts"
                    secondary-key="code"
                    placeholder="-- Pilih Rekening Kas/Bank --"
                    search-placeholder="Cari kode atau nama kas/bank..."
                    :allow-clear="false"
                  />
                </div>
              </div>
            </div>

            <!-- Nominal Transfer -->
            <div>
              <label class="block font-bold text-slate-700 mb-1">Nominal Transfer (Rp) *</label>
              <div class="relative">
                <span class="absolute left-3 top-2.5 font-bold text-slate-400">Rp</span>
                <input
                  type="number"
                  min="1"
                  step="1"
                  v-model.number="fundTransferForm.amount"
                  placeholder="0"
                  class="w-full bg-white border border-slate-300 rounded-xl pl-10 pr-3 py-2 font-mono font-bold text-slate-900 text-sm focus:outline-none focus:border-blue-600"
                />
              </div>
              <p class="text-[11px] text-slate-500 mt-1">
                Terbilang: <span class="font-semibold text-slate-700">{{ formatCurrency(fundTransferForm.amount || 0) }}</span>
              </p>
            </div>

            <!-- Catatan / Memo -->
            <div>
              <label class="block font-bold text-slate-700 mb-1">Catatan / Keterangan (Opsional)</label>
              <textarea
                v-model="fundTransferForm.notes"
                rows="2"
                placeholder="Contoh: Tambahan kas operasional cabang Sudirman"
                class="w-full bg-white border border-slate-300 rounded-xl px-3 py-2 text-slate-800 focus:outline-none focus:border-blue-600 resize-none"
              ></textarea>
            </div>

            <!-- Accounting RAK or Internal Info Banner -->
            <div
              v-if="fundTransferForm.from_outlet_id && fundTransferForm.to_outlet_id && fundTransferForm.from_outlet_id === fundTransferForm.to_outlet_id"
              class="p-3 bg-emerald-50 border border-emerald-200 rounded-xl text-[11px] text-emerald-950 flex items-start gap-2.5"
            >
              <Scale class="w-4 h-4 shrink-0 text-emerald-600 mt-0.5" />
              <div>
                <p class="font-bold mb-0.5">Transfer Kas / Bank Internal (Cabang Sama)</p>
                <p>
                  Sistem otomatis membukukan jurnal mutasi kas langsung: Akun Kas/Bank Tujuan didebit (bertambah) dan Akun Kas/Bank Asal dikredit (berkurang) dalam cabang yang sama.
                </p>
              </div>
            </div>
            <div
              v-else
              class="p-3 bg-blue-50 border border-blue-200 rounded-xl text-[11px] text-blue-900 flex items-start gap-2.5"
            >
              <Scale class="w-4 h-4 shrink-0 text-blue-600 mt-0.5" />
              <div>
                <p class="font-bold mb-0.5">Mekanisme Rekening Antar-Kantor (RAK)</p>
                <p>
                  Sistem otomatis mencatat 2 jurnal berpasangan: Cabang Pengirim mendebit RAK dan mengkredit Kas, sedangkan Cabang Penerima mendebit Kas dan mengkredit RAK. Pembukuan masing-masing cabang tetap seimbang (*balanced*), dan otomatis tereliminasi pada laporan konsolidasian.
                </p>
              </div>
            </div>
          </div>

          <div class="px-6 py-3 bg-slate-100 border-t border-slate-200 flex items-center justify-between shrink-0">
            <button
              type="button"
              @click="showFundTransferModal = false"
              class="px-4 py-2 rounded-xl text-slate-700 hover:bg-slate-200 text-xs font-bold transition cursor-pointer"
            >
              Batal
            </button>
            <button
              type="button"
              @click="handleSaveFundTransfer"
              :disabled="
                isSubmittingFundTransfer ||
                !fundTransferForm.amount ||
                !fundTransferForm.from_account_id ||
                !fundTransferForm.to_account_id ||
                fundTransferForm.from_account_id === fundTransferForm.to_account_id ||
                (fromAccountBalance !== null && (fundTransferForm.amount > fromAccountBalance || fromAccountBalance <= 0))
              "
              class="px-5 py-2 rounded-xl bg-blue-700 hover:bg-blue-800 text-white text-xs font-bold shadow-md transition active:scale-95 disabled:opacity-50 cursor-pointer"
            >
              {{ isSubmittingFundTransfer ? 'Memproses...' : 'Bukukan Transfer Kas / Dana' }}
            </button>
          </div>
        </div>
      </div>

      <!-- ===================================================================== -->
      <!-- MODAL: FORMULIR SETORAN MODAL USAHA (CAPITAL DEPOSIT) -->
      <!-- ===================================================================== -->
      <div
        v-if="showCapitalModal"
        class="fixed inset-0 z-50 bg-slate-900/60 backdrop-blur-xs flex items-center justify-center p-4 overflow-y-auto"
      >
        <div class="bg-white rounded-3xl border border-slate-200 shadow-2xl max-w-xl w-full overflow-hidden animate-in fade-in zoom-in-95 duration-150">
          <div class="px-6 py-4 bg-slate-900 text-white flex items-center justify-between">
            <div class="flex items-center gap-2.5">
              <div class="w-8 h-8 rounded-xl bg-emerald-700 flex items-center justify-center text-amber-300">
                <Coins class="w-4 h-4" />
              </div>
              <div>
                <h3 class="font-bold text-sm">Pencatatan Setoran Modal Usaha</h3>
                <p class="text-[11px] text-slate-400">Penerimaan dana investasi kas/bank dari pemilik atau investor</p>
              </div>
            </div>
            <button @click="showCapitalModal = false" class="p-1 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800 transition cursor-pointer">
              <X class="w-5 h-5" />
            </button>
          </div>

          <form @submit.prevent="handleSaveCapital" class="p-6 space-y-4">
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <!-- Tanggal Setor -->
              <div>
                <label class="block text-xs font-bold text-slate-700 mb-1">Tanggal Setor *</label>
                <input
                  type="date"
                  v-model="capitalForm.deposit_date"
                  required
                  class="w-full bg-slate-50 border border-slate-300 rounded-xl px-3 py-2 text-xs font-mono font-bold text-slate-900 focus:outline-none focus:border-emerald-600"
                />
              </div>

              <!-- Outlet / Cabang -->
              <div>
                <label class="block text-xs font-bold text-slate-700 mb-1">Gerai / Cabang Penerima</label>
                <SearchableSelect
                  v-model="capitalForm.outlet_id"
                  api-endpoint="/outlets"
                  secondary-key="code"
                  placeholder="Kantor Pusat (Umum/Konsolidasi)"
                  search-placeholder="Cari cabang..."
                  :allow-clear="true"
                />
              </div>
            </div>

            <!-- Nama Pemilik / Penyetor -->
            <div>
              <label class="block text-xs font-bold text-slate-700 mb-1">Nama Pemilik / Investor Penyetor *</label>
              <input
                type="text"
                v-model="capitalForm.investor_name"
                required
                placeholder="Contoh: Bpk. Hendra Gunawan (Pendiri) / Ibu Siti"
                class="w-full bg-slate-50 border border-slate-300 rounded-xl px-3 py-2 text-xs font-bold text-slate-900 placeholder-slate-400 focus:outline-none focus:border-emerald-600"
              />
            </div>

            <!-- Nominal Setoran -->
            <div>
              <label class="block text-xs font-bold text-slate-700 mb-1">Nominal Modal Disetor (Rp) *</label>
              <div class="relative">
                <span class="absolute left-3 top-1/2 -translate-y-1/2 font-bold text-xs text-slate-400">Rp</span>
                <input
                  type="number"
                  v-model.number="capitalForm.amount"
                  min="1"
                  required
                  placeholder="0"
                  class="w-full bg-slate-50 border border-slate-300 rounded-xl pl-9 pr-3 py-2 text-sm font-mono font-black text-emerald-700 focus:outline-none focus:border-emerald-600"
                />
              </div>
            </div>

            <!-- Rekening Kas / Bank Penerima Setoran Modal -->
            <div class="space-y-3">
              <div>
                <label class="block text-xs font-bold text-slate-700 mb-1">
                  Disetor Ke Rekening Bank / Kas Toko *
                </label>
                <SearchableSelect
                  v-model="capitalForm.payment_account_id"
                  :options="businessPaymentAccountOptions"
                  secondary-key="code"
                  placeholder="-- Pilih Rekening Kas atau Bank Penerima --"
                  search-placeholder="Cari rekening bank atau kas..."
                  :allow-clear="false"
                />
                <!-- Quick-add link -->
                <button
                  type="button"
                  @click="showCapitalModal = false; openCreateBankModal()"
                  class="mt-1.5 flex items-center gap-1 text-[11px] text-emerald-700 hover:text-emerald-900 font-semibold transition cursor-pointer"
                >
                  <Landmark class="w-3 h-3" />
                  <span>+ Daftarkan rekening bank baru</span>
                </button>
              </div>

              <!-- Pos Ekuitas / Modal (Otomatis 3101 Modal Pemilik dengan Opsi Lanjutan) -->
              <div class="p-3 bg-slate-50 border border-slate-200 rounded-xl space-y-2">
                <div class="flex items-center justify-between">
                  <div class="flex items-center gap-2">
                    <div class="w-6 h-6 rounded-lg bg-purple-100 text-purple-700 flex items-center justify-center shrink-0">
                      <PiggyBank class="w-3.5 h-3.5" />
                    </div>
                    <div>
                      <p class="text-xs font-bold text-slate-800">
                        Pos Ekuitas: <span class="text-purple-700">{{ currentCapitalEquityName }}</span>
                      </p>
                      <p class="text-[10px] text-slate-500">
                        Otomatis dialokasikan ke modal disetor pemilik toko
                      </p>
                    </div>
                  </div>
                  <button
                    type="button"
                    @click="showAdvancedEquityOption = !showAdvancedEquityOption"
                    class="text-[11px] font-bold text-purple-700 hover:text-purple-900 underline transition cursor-pointer"
                  >
                    {{ showAdvancedEquityOption ? 'Sembunyikan' : 'Ganti Akun' }}
                  </button>
                </div>

                <!-- Dropdown ganti akun ekuitas (hanya tampil jika diklik) -->
                <div v-if="showAdvancedEquityOption" class="pt-2 border-t border-slate-200">
                  <label class="block text-[11px] font-bold text-slate-600 mb-1">Pilih Akun Ekuitas / Modal Alternatif:</label>
                  <SearchableSelect
                    v-model="capitalForm.equity_account_id"
                    :options="capitalEquityAccounts"
                    secondary-key="code"
                    placeholder="Pilih Akun Ekuitas"
                    search-placeholder="Cari akun modal..."
                    :allow-clear="false"
                  />
                </div>
              </div>
            </div>

            <!-- No Referensi Transfer & Catatan -->
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <div>
                <label class="block text-xs font-bold text-slate-700 mb-1">No. Referensi / Bukti Transfer</label>
                <input
                  type="text"
                  v-model="capitalForm.reference_no"
                  placeholder="Contoh: TRF-BCA-982173"
                  class="w-full bg-slate-50 border border-slate-300 rounded-xl px-3 py-2 text-xs font-mono text-slate-800 placeholder-slate-400 focus:outline-none focus:border-emerald-600"
                />
              </div>

              <div>
                <label class="block text-xs font-bold text-slate-700 mb-1">Catatan / Peruntukan</label>
                <input
                  type="text"
                  v-model="capitalForm.notes"
                  placeholder="Contoh: Modal awal operasional gerai"
                  class="w-full bg-slate-50 border border-slate-300 rounded-xl px-3 py-2 text-xs text-slate-800 placeholder-slate-400 focus:outline-none focus:border-emerald-600"
                />
              </div>
            </div>

            <!-- Preview Jurnal Double Entry Otomatis -->
            <div class="p-3.5 bg-slate-50 rounded-2xl border border-slate-200 text-xs space-y-1.5 font-mono">
              <div class="flex items-center gap-1.5 text-slate-700 font-sans font-bold text-[11px] mb-1">
                <CheckCircle2 class="w-3.5 h-3.5 text-emerald-600 shrink-0" />
                <span>Simulasi Penjurnalan Otomatis (Double Entry):</span>
              </div>
              <div class="flex justify-between items-center text-emerald-800">
                <span>[DR] Kas/Bank Penerima</span>
                <span class="font-bold">{{ formatCurrency(capitalForm.amount || 0) }}</span>
              </div>
              <div class="flex justify-between items-center text-purple-800 pl-4">
                <span>[CR] Modal Disetor Pemilik</span>
                <span class="font-bold">{{ formatCurrency(capitalForm.amount || 0) }}</span>
              </div>
            </div>

            <div class="pt-2 flex items-center justify-end gap-2.5">
              <button
                type="button"
                @click="showCapitalModal = false"
                class="px-4 py-2 rounded-xl text-slate-600 hover:bg-slate-100 text-xs font-bold transition cursor-pointer"
              >
                Batal
              </button>
              <button
                type="submit"
                :disabled="isSubmittingCapital"
                class="px-5 py-2 rounded-xl bg-emerald-700 hover:bg-emerald-600 text-white text-xs font-bold shadow-md transition active:scale-95 disabled:opacity-50 cursor-pointer"
              >
                {{ isSubmittingCapital ? 'Menyimpan...' : 'Simpan & Bukukan Modal' }}
              </button>
            </div>
          </form>
        </div>
      </div>

      <!-- ===================================================================== -->
      <!-- MODAL: PEMBATALAN SETORAN MODAL (CANCEL / REVERSAL) -->
      <!-- ===================================================================== -->
      <div
        v-if="showCancelCapitalModal"
        class="fixed inset-0 z-50 bg-slate-900/60 backdrop-blur-xs flex items-center justify-center p-4 overflow-y-auto"
      >
        <div class="bg-white rounded-3xl border border-slate-200 shadow-2xl max-w-md w-full overflow-hidden animate-in fade-in zoom-in-95 duration-150">
          <div class="px-6 py-4 bg-rose-900 text-white flex items-center justify-between">
            <div class="flex items-center gap-2">
              <AlertTriangle class="w-5 h-5 text-rose-300" />
              <h3 class="font-bold text-sm">Batalkan Setoran Modal</h3>
            </div>
            <button @click="showCancelCapitalModal = false" class="p-1 rounded-lg text-slate-300 hover:text-white hover:bg-rose-800 transition cursor-pointer">
              <X class="w-5 h-5" />
            </button>
          </div>

          <div class="p-6 space-y-4">
            <div class="p-3 bg-rose-50 rounded-2xl border border-rose-200 text-rose-900 text-xs space-y-1">
              <p class="font-bold">Apakah Anda yakin ingin membatalkan transaksi ini?</p>
              <p class="text-[11px] text-rose-700">
                Sistem akan secara otomatis membuat <span class="font-bold font-mono">Jurnal Pembalik (Reversing Entry)</span> untuk mengembalikan saldo kas dan ekuitas ke kondisi sebelum setoran modal.
              </p>
            </div>

            <div v-if="selectedCapitalToCancel" class="bg-slate-50 p-3 rounded-xl border border-slate-200 font-mono text-xs space-y-1">
              <div class="flex justify-between">
                <span class="text-slate-500 font-sans">No. Bukti:</span>
                <span class="font-bold text-slate-800">{{ selectedCapitalToCancel.deposit_no }}</span>
              </div>
              <div class="flex justify-between">
                <span class="text-slate-500 font-sans">Penyetor:</span>
                <span class="font-bold text-slate-800">{{ selectedCapitalToCancel.investor_name }}</span>
              </div>
              <div class="flex justify-between">
                <span class="text-slate-500 font-sans">Nominal:</span>
                <span class="font-bold text-emerald-700">{{ formatCurrency(selectedCapitalToCancel.amount) }}</span>
              </div>
            </div>

            <div>
              <label class="block text-xs font-bold text-slate-700 mb-1">Alasan Pembatalan *</label>
              <textarea
                v-model="cancelCapitalReason"
                rows="3"
                required
                placeholder="Contoh: Kesalahan input nominal transfer / dibatalkan atas permintaan investor"
                class="w-full bg-slate-50 border border-slate-300 rounded-xl px-3 py-2 text-xs text-slate-800 placeholder-slate-400 focus:outline-none focus:border-rose-600"
              ></textarea>
            </div>

            <div class="pt-2 flex items-center justify-end gap-2.5">
              <button
                type="button"
                @click="showCancelCapitalModal = false"
                class="px-4 py-2 rounded-xl text-slate-600 hover:bg-slate-100 text-xs font-bold transition cursor-pointer"
              >
                Batal
              </button>
              <button
                type="button"
                @click="handleExecuteCancelCapital"
                :disabled="isSubmittingCancelCapital"
                class="px-5 py-2 rounded-xl bg-rose-700 hover:bg-rose-800 text-white text-xs font-bold shadow-md transition active:scale-95 disabled:opacity-50 cursor-pointer"
              >
                {{ isSubmittingCancelCapital ? 'Memproses...' : 'Ya, Batalkan Transaksi' }}
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- ===================================================================== -->
      <!-- MODAL: BUKTI KAS MASUK SETORAN MODAL (PRINT VOUCHER) -->
      <!-- ===================================================================== -->
      <div
        v-if="showCapitalVoucherModal && selectedCapitalForVoucher"
        class="fixed inset-0 z-50 bg-slate-900/60 backdrop-blur-xs flex items-center justify-center p-4 overflow-y-auto"
      >
        <div class="bg-white rounded-3xl border border-slate-200 shadow-2xl max-w-lg w-full overflow-hidden animate-in fade-in zoom-in-95 duration-150">
          <div class="px-6 py-4 bg-slate-900 text-white flex items-center justify-between print:hidden">
            <div class="flex items-center gap-2">
              <Printer class="w-4 h-4 text-emerald-400" />
              <h3 class="font-bold text-sm">Bukti Kas Masuk (Setoran Modal)</h3>
            </div>
            <div class="flex items-center gap-2">
              <button
                @click="printCapitalVoucher"
                class="px-3 py-1.5 rounded-lg bg-emerald-700 hover:bg-emerald-600 text-white text-xs font-bold flex items-center gap-1.5 transition cursor-pointer"
              >
                <Printer class="w-3.5 h-3.5" />
                <span>Cetak / Print</span>
              </button>
              <button @click="showCapitalVoucherModal = false" class="p-1 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800 transition cursor-pointer">
                <X class="w-5 h-5" />
              </button>
            </div>
          </div>

          <!-- Printable Voucher Content -->
          <div id="printable-capital-voucher" class="p-8 space-y-6 text-slate-900 font-sans print:p-0">
            <!-- Header -->
            <div class="text-center border-b-2 border-slate-900 pb-4">
              <h2 class="text-lg font-black uppercase tracking-wider">BUKTI KAS MASUK - SETORAN MODAL</h2>
              <p class="text-xs text-slate-600 font-bold mt-0.5">
                {{ selectedCapitalForVoucher.outlet_name || 'KANTOR PUSAT & OPERASIONAL BISNIS' }}
              </p>
              <p class="text-[11px] font-mono text-slate-500 mt-1">
                No. Bukti: <span class="font-bold text-slate-900">{{ selectedCapitalForVoucher.deposit_no }}</span>
              </p>
            </div>

            <!-- Details List -->
            <div class="space-y-2.5 text-xs font-medium">
              <div class="flex justify-between py-1 border-b border-slate-100">
                <span class="text-slate-500">Tanggal Transaksi</span>
                <span class="font-bold font-mono">{{ formatDate(selectedCapitalForVoucher.deposit_date) }}</span>
              </div>
              <div class="flex justify-between py-1 border-b border-slate-100">
                <span class="text-slate-500">Diterima Dari (Penyetor)</span>
                <span class="font-bold text-slate-900">{{ selectedCapitalForVoucher.investor_name }}</span>
              </div>
              <div class="flex justify-between py-1 border-b border-slate-100">
                <span class="text-slate-500">Rekening Kas / Bank (Debit)</span>
                <span class="font-bold">{{ selectedCapitalForVoucher.payment_account_code }} - {{ selectedCapitalForVoucher.payment_account_name }}</span>
              </div>
              <div class="flex justify-between py-1 border-b border-slate-100">
                <span class="text-slate-500">Akun Ekuitas (Kredit)</span>
                <span class="font-bold">{{ selectedCapitalForVoucher.equity_account_code }} - {{ selectedCapitalForVoucher.equity_account_name }}</span>
              </div>
              <div v-if="selectedCapitalForVoucher.reference_no" class="flex justify-between py-1 border-b border-slate-100">
                <span class="text-slate-500">No. Referensi / Bukti Transfer</span>
                <span class="font-mono">{{ selectedCapitalForVoucher.reference_no }}</span>
              </div>
              <div v-if="selectedCapitalForVoucher.notes" class="flex justify-between py-1 border-b border-slate-100">
                <span class="text-slate-500">Keterangan / Keperluan</span>
                <span class="italic text-slate-700">{{ selectedCapitalForVoucher.notes }}</span>
              </div>
              <div class="flex justify-between py-1 border-b border-slate-100">
                <span class="text-slate-500">Status Pembukuan</span>
                <span
                  class="font-bold font-mono"
                  :class="selectedCapitalForVoucher.status === 'COMPLETED' ? 'text-emerald-700' : 'text-rose-700'"
                >
                  {{ selectedCapitalForVoucher.status === 'COMPLETED' ? 'TERJURNAL & SELESAI' : 'DIBATALKAN' }}
                </span>
              </div>
            </div>

            <!-- Amount Highlight Box -->
            <div class="p-4 bg-emerald-50 rounded-2xl border border-emerald-200 text-center">
              <span class="text-[11px] font-bold text-emerald-800 uppercase tracking-wider block mb-1">Jumlah Diterima:</span>
              <span class="text-2xl font-black font-mono text-emerald-950">{{ formatCurrency(selectedCapitalForVoucher.amount) }}</span>
            </div>

            <!-- Signatures -->
            <div class="grid grid-cols-2 gap-8 pt-6 text-center text-xs">
              <div>
                <p class="text-slate-500 mb-14 font-medium">Diserahkan Oleh (Penyetor),</p>
                <div class="border-t border-slate-400 font-bold pt-1 text-slate-800">
                  {{ selectedCapitalForVoucher.investor_name }}
                </div>
              </div>
              <div>
                <p class="text-slate-500 mb-14 font-medium">Diterima & Dibukukan Oleh,</p>
                <div class="border-t border-slate-400 font-bold pt-1 text-slate-800">
                  Kasir / Manajer Keuangan
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>


      <!-- ===================================================================== -->
      <!-- MODAL: REKENING BANK PENERIMAAN (TAMBAH / EDIT) -->
      <!-- ===================================================================== -->
      <Teleport to="body">
      <div
        v-if="showBankModal"
        class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/60 backdrop-blur-xs overflow-y-auto"
      >
        <div class="bg-white rounded-2xl w-full max-w-lg shadow-2xl border border-slate-200 overflow-hidden my-8">
          <!-- Header -->
          <div class="px-6 py-4 bg-slate-900 text-white flex items-center justify-between">
            <div class="flex items-center gap-2">
              <Landmark class="w-4 h-4 text-emerald-400" />
              <h3 class="font-bold text-sm">
                {{ bankModalMode === 'create' ? 'Tambah Rekening Bank Penerimaan' : 'Edit Rekening Bank' }}
              </h3>
            </div>
            <button
              @click="showBankModal = false"
              class="p-1 rounded-lg text-slate-400 hover:text-white hover:bg-slate-800 transition cursor-pointer"
            >
              <X class="w-5 h-5" />
            </button>
          </div>

          <!-- Body -->
          <div class="p-6 space-y-4 text-xs font-sans">
            <!-- Bank Name with Quick Selector -->
            <div>
              <label class="block font-bold text-slate-700 mb-1">
                Nama Bank <span class="text-rose-500">*</span>
              </label>
              <input
                v-model="bankForm.bank_name"
                type="text"
                placeholder="cth: BCA, Bank Mandiri, BRI"
                class="w-full bg-slate-50 border border-slate-300 rounded-xl px-3 py-2 text-xs text-slate-800 focus:outline-none focus:border-emerald-600 font-semibold"
              />
              <!-- Quick Select Buttons -->
              <div class="flex items-center gap-1.5 flex-wrap mt-1.5">
                <span class="text-[10px] text-slate-400">Pilihan cepat:</span>
                <button
                  v-for="b in popularBanks"
                  :key="b"
                  type="button"
                  @click="bankForm.bank_name = b"
                  class="px-2 py-0.5 rounded-lg border text-[10px] font-medium transition cursor-pointer"
                  :class="bankForm.bank_name === b ? 'bg-emerald-50 text-emerald-800 border-emerald-300 font-bold' : 'bg-slate-50 text-slate-600 border-slate-200 hover:bg-slate-100'"
                >
                  {{ b }}
                </button>
              </div>
            </div>

            <!-- Account Number & Account Holder -->
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
              <div>
                <label class="block font-bold text-slate-700 mb-1">
                  Nomor Rekening <span class="text-rose-500">*</span>
                </label>
                <input
                  v-model="bankForm.account_number"
                  type="text"
                  placeholder="cth: 1234567890"
                  class="w-full bg-slate-50 border border-slate-300 rounded-xl px-3 py-2 text-xs text-slate-800 focus:outline-none focus:border-emerald-600 font-mono font-bold"
                />
              </div>

              <div>
                <label class="block font-bold text-slate-700 mb-1">
                  Atas Nama (Pemilik) <span class="text-rose-500">*</span>
                </label>
                <input
                  v-model="bankForm.account_holder"
                  type="text"
                  placeholder="cth: PT Maju Bersama / John Doe"
                  class="w-full bg-slate-50 border border-slate-300 rounded-xl px-3 py-2 text-xs text-slate-800 focus:outline-none focus:border-emerald-600 uppercase font-semibold"
                />
              </div>
            </div>

            <!-- Branch & Outlet Scope -->
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
              <div>
                <label class="block font-bold text-slate-700 mb-1">
                  Cabang / KCP <span class="font-normal text-slate-400">(opsional)</span>
                </label>
                <input
                  v-model="bankForm.branch"
                  type="text"
                  placeholder="cth: KCP Sudirman Jakarta"
                  class="w-full bg-slate-50 border border-slate-300 rounded-xl px-3 py-2 text-xs text-slate-800 focus:outline-none focus:border-emerald-600"
                />
              </div>

              <div>
                <label class="block font-bold text-slate-700 mb-1">
                  Khusus Outlet <span class="font-normal text-slate-400">(opsional)</span>
                </label>
                <SearchableSelect
                  v-model="bankForm.outlet_id"
                  api-endpoint="/outlets"
                  secondary-key="code"
                  placeholder="Semua Outlet (Nasional)"
                  search-placeholder="Cari cabang..."
                  :allow-clear="true"
                />
              </div>
            </div>

            <!-- Notes -->
            <div>
              <label class="block font-bold text-slate-700 mb-1">
                Catatan / Keterangan <span class="font-normal text-slate-400">(opsional)</span>
              </label>
              <input
                v-model="bankForm.notes"
                type="text"
                placeholder="cth: Rekening utama penerimaan transfer QRIS / Debit"
                class="w-full bg-slate-50 border border-slate-300 rounded-xl px-3 py-2 text-xs text-slate-800 focus:outline-none focus:border-emerald-600"
              />
            </div>

            <!-- Flags: is_default & is_active -->
            <div class="pt-2 border-t border-slate-100 space-y-2">
              <label class="flex items-center gap-2.5 cursor-pointer">
                <input
                  type="checkbox"
                  v-model="bankForm.is_default"
                  class="w-4 h-4 rounded accent-emerald-600 cursor-pointer"
                />
                <div>
                  <span class="font-bold text-slate-700 block">Jadikan Rekening Utama</span>
                  <span class="text-[11px] text-slate-400">Otomatis dipilih saat transaksi pembayaran non-tunai</span>
                </div>
              </label>

              <label v-if="bankModalMode === 'edit'" class="flex items-center gap-2.5 cursor-pointer">
                <input
                  type="checkbox"
                  v-model="bankForm.is_active"
                  class="w-4 h-4 rounded accent-emerald-600 cursor-pointer"
                />
                <div>
                  <span class="font-bold text-slate-700 block">Rekening Aktif</span>
                  <span class="text-[11px] text-slate-400">Rekening nonaktif tidak dapat dipilih saat transaksi</span>
                </div>
              </label>
            </div>

            <!-- Automatic COA Creation Notice -->
            <div class="p-3 bg-emerald-50 border border-emerald-200 rounded-xl text-[11px] text-emerald-900 flex gap-2">
              <Landmark class="w-4 h-4 shrink-0 text-emerald-600 mt-0.5" />
              <div>
                <p class="font-bold mb-0.5">Integrasi Otomatis Bagan Akun (COA)</p>
                <p>
                  Sistem akan otomatis menghubungkan rekening ini ke akun Kas &amp; Bank (<span class="font-mono font-bold">111x</span>). Anda tidak perlu menambah akun manual di Bagan Akun.
                </p>
              </div>
            </div>
          </div>

          <!-- Footer Actions -->
          <div class="px-6 pb-5 flex gap-3 justify-end">
            <button
              @click="showBankModal = false"
              class="px-4 py-2 text-xs font-bold text-slate-600 bg-slate-100 hover:bg-slate-200 rounded-xl transition cursor-pointer"
            >
              Batal
            </button>
            <button
              @click="saveBankAccount"
              :disabled="isSubmittingBank"
              class="px-5 py-2 text-xs font-bold text-white rounded-xl transition cursor-pointer disabled:opacity-60 disabled:cursor-not-allowed bg-emerald-700 hover:bg-emerald-800 shadow-xs"
            >
              {{ isSubmittingBank ? 'Menyimpan...' : (bankModalMode === 'create' ? 'Daftarkan Rekening' : 'Simpan Perbarui') }}
            </button>
          </div>
        </div>
      </div>
      </Teleport>

      <!-- ===================================================================== -->
      <!-- MODAL COA: TAMBAH / EDIT AKUN -->
      <!-- ===================================================================== -->
      <Teleport to="body">
      <div
        v-if="showCoaModal"
        class="fixed inset-0 z-[60] flex items-center justify-center p-4 bg-slate-950/70 backdrop-blur-sm overflow-y-auto"
        @click.self="showCoaModal = false"
      >
        <div class="bg-white rounded-2xl border border-slate-200 shadow-2xl w-full max-w-lg overflow-hidden">
          <!-- Header -->
          <div class="px-6 py-4 flex items-center justify-between"
            :class="coaModalMode === 'create' ? 'bg-emerald-700' : 'bg-slate-800'"
          >
            <div class="flex items-center gap-2.5 text-white">
              <Landmark class="w-5 h-5 opacity-80" />
              <div>
                <h3 class="font-extrabold text-sm">
                  {{ coaModalMode === 'create' ? 'Tambah Akun Baru' : 'Edit Akun' }}
                </h3>
                <p class="text-[11px] opacity-70">
                  {{ coaModalMode === 'create' ? 'Daftarkan rekening baru ke Chart of Accounts' : `Mengedit: ${coaForm.code} - ${coaForm.name}` }}
                </p>
              </div>
            </div>
            <button @click="showCoaModal = false" class="text-white/70 hover:text-white transition cursor-pointer">
              <X class="w-5 h-5" />
            </button>
          </div>

          <!-- Form -->
          <div class="p-6 space-y-4 text-xs">

            <!-- Kategori Akun (Create only) -->
            <div v-if="coaModalMode === 'create'">
              <label class="block font-bold text-slate-700 mb-1.5">Kategori Akun *</label>
              <div class="grid grid-cols-1 sm:grid-cols-2 gap-2">
                <label
                  v-for="cat in ACCOUNT_CATEGORIES"
                  :key="cat.id"
                  class="flex items-center gap-2.5 p-3 rounded-xl border-2 cursor-pointer transition"
                  :class="coaForm.account_category_id === cat.id
                    ? 'border-emerald-500 bg-emerald-50'
                    : 'border-slate-200 hover:border-slate-300 bg-white'"
                >
                  <input
                    type="radio"
                    :value="cat.id"
                    v-model="coaForm.account_category_id"
                    class="accent-emerald-600"
                  />
                  <span class="font-bold text-slate-800">{{ cat.label }}</span>
                </label>
              </div>
            </div>

            <!-- Kategori info (Edit mode) -->
            <div v-else>
              <label class="block font-bold text-slate-700 mb-1">Kategori Akun</label>
              <div class="px-3 py-2 bg-slate-100 rounded-xl border border-slate-200 text-slate-600 font-mono">
                <span v-if="coaForm.account_category_id === 1" class="text-emerald-700 font-bold">1 - ASSET (Aktiva)</span>
                <span v-else-if="coaForm.account_category_id === 2" class="text-amber-700 font-bold">2 - LIABILITY (Kewajiban)</span>
                <span v-else-if="coaForm.account_category_id === 3" class="text-purple-700 font-bold">3 - EQUITY (Modal)</span>
                <span v-else-if="coaForm.account_category_id === 4" class="text-sky-700 font-bold">4 - REVENUE (Pendapatan)</span>
                <span v-else-if="coaForm.account_category_id === 5" class="text-rose-700 font-bold">5 - EXPENSE (Beban)</span>
              </div>
              <p class="mt-1 text-[11px] text-slate-400">Kategori akun ini sudah terkunci dan tidak dapat diubah.</p>
            </div>

            <!-- Kode Akun + Nama -->
            <div class="grid grid-cols-5 gap-3">
              <div class="col-span-2">
                <label class="block font-bold text-slate-700 mb-1">Kode Akun *</label>
                <input
                  v-model="coaForm.code"
                  type="text"
                  :disabled="coaModalMode === 'edit'"
                  placeholder="Contoh: 1103"
                  maxlength="10"
                  class="w-full bg-slate-50 border border-slate-300 rounded-xl px-3 py-2 text-xs font-mono text-slate-800 placeholder-slate-400 focus:outline-none focus:border-emerald-600 disabled:bg-slate-100 disabled:text-slate-500 disabled:cursor-not-allowed"
                />
                <p v-if="coaModalMode === 'edit'" class="mt-1 text-[11px] text-slate-400">Kode tidak dapat diubah.</p>
              </div>
              <div class="col-span-3">
                <label class="block font-bold text-slate-700 mb-1">Nama Akun *</label>
                <input
                  v-model="coaForm.name"
                  type="text"
                  placeholder="Contoh: Bank BCA Bisnis"
                  maxlength="100"
                  class="w-full bg-slate-50 border border-slate-300 rounded-xl px-3 py-2 text-xs text-slate-800 placeholder-slate-400 focus:outline-none focus:border-emerald-600"
                />
              </div>
            </div>

            <!-- Deskripsi -->
            <div>
              <label class="block font-bold text-slate-700 mb-1">Deskripsi / Keterangan <span class="font-normal text-slate-400">(opsional)</span></label>
              <input
                v-model="coaForm.description"
                type="text"
                placeholder="Contoh: Rekening operasional BCA cabang Sudirman"
                maxlength="200"
                class="w-full bg-slate-50 border border-slate-300 rounded-xl px-3 py-2 text-xs text-slate-800 placeholder-slate-400 focus:outline-none focus:border-emerald-600"
              />
            </div>

            <!-- Master Universal Scope Note & Is Header (Create only) -->
            <div v-if="coaModalMode === 'create'" class="space-y-3">
              <div class="p-3 bg-slate-50 rounded-xl border border-slate-200 flex items-center justify-between">
                <div>
                  <span class="font-bold text-slate-700 block">Cakupan Akun: Universal</span>
                  <span class="text-[11px] text-slate-400">Master CoA berlaku terpusat untuk semua cabang & konsolidasi</span>
                </div>
                <label class="flex items-center gap-2.5 cursor-pointer">
                  <input
                    type="checkbox"
                    v-model="coaForm.is_header"
                    class="w-4 h-4 rounded accent-slate-700 cursor-pointer"
                  />
                  <div>
                    <span class="font-bold text-slate-700 block">Akun Induk (Header)</span>
                    <span class="text-[11px] text-slate-400">Tidak dapat diposting langsung</span>
                  </div>
                </label>
              </div>
            </div>

            <!-- Status Aktif (Edit only) -->
            <div v-if="coaModalMode === 'edit'">
              <label class="block font-bold text-slate-700 mb-1">Status Akun</label>
              <div class="flex gap-3">
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="radio" :value="true" v-model="coaForm.is_active" class="accent-emerald-600" />
                  <span class="font-semibold text-emerald-700">Aktif</span>
                </label>
                <label class="flex items-center gap-2 cursor-pointer">
                  <input type="radio" :value="false" v-model="coaForm.is_active" class="accent-slate-500" />
                  <span class="font-semibold text-slate-500">Nonaktif</span>
                </label>
              </div>
              <p class="mt-1 text-[11px] text-slate-400">Akun nonaktif tidak akan muncul di dropdown pemilihan.</p>
            </div>

            <!-- Info tip untuk kas/bank -->
            <div
              v-if="coaForm.account_category_id === 1 && coaModalMode === 'create'"
              class="p-3 bg-sky-50 border border-sky-200 rounded-xl text-[11px] text-sky-800 flex gap-2"
            >
              <Landmark class="w-4 h-4 shrink-0 text-sky-500 mt-0.5" />
              <div>
                <p class="font-bold mb-0.5">Tips Kode Akun ASSET</p>
                <p>Gunakan kode <span class="font-mono font-bold">11xx</span> untuk Kas &amp; Bank (misal: <span class="font-mono">1103</span> untuk BCA Bisnis), <span class="font-mono font-bold">12xx</span> untuk Piutang.</p>
              </div>
            </div>
            <div
              v-if="coaForm.account_category_id === 3 && coaModalMode === 'create'"
              class="p-3 bg-purple-50 border border-purple-200 rounded-xl text-[11px] text-purple-900 flex gap-2"
            >
              <Plus class="w-4 h-4 shrink-0 text-purple-500 mt-0.5" />
              <div>
                <p class="font-bold mb-0.5">Tips Kode Akun EQUITY</p>
                <p>Gunakan kode <span class="font-mono font-bold">31xx</span> untuk Modal (misal: <span class="font-mono">3104</span> untuk Modal Pemegang Saham Baru).</p>
              </div>
            </div>
          </div>

          <!-- Footer Actions -->
          <div class="px-6 pb-5 flex gap-3 justify-end">
            <button
              @click="showCoaModal = false"
              class="px-4 py-2 text-xs font-bold text-slate-600 bg-slate-100 hover:bg-slate-200 rounded-xl transition cursor-pointer"
            >
              Batal
            </button>
            <button
              @click="handleSaveCoa"
              :disabled="isSubmittingCoa"
              class="px-5 py-2 text-xs font-bold text-white rounded-xl transition cursor-pointer disabled:opacity-60 disabled:cursor-not-allowed"
              :class="coaModalMode === 'create' ? 'bg-emerald-600 hover:bg-emerald-700' : 'bg-slate-700 hover:bg-slate-800'"
            >
              {{ isSubmittingCoa ? 'Menyimpan...' : (coaModalMode === 'create' ? 'Simpan Akun' : 'Perbarui Akun') }}
            </button>
          </div>
        </div>
      </div>
      </Teleport>

      <!-- ===================================================================== -->
      <!-- MODAL: PEMBAGIAN DIVIDEN BARU (LABA DITAHAN) -->
      <!-- ===================================================================== -->
      <Teleport to="body">
      <div
        v-if="showCreateDividendModal"
        class="fixed inset-0 z-[60] flex items-center justify-center p-4 bg-slate-950/70 backdrop-blur-sm overflow-y-auto"
        @click.self="showCreateDividendModal = false"
      >
        <div class="bg-white rounded-2xl border border-slate-200 shadow-2xl w-full max-w-2xl overflow-hidden my-6 max-h-[90vh] flex flex-col">
          <!-- Modal Header -->
          <div class="px-6 py-4 bg-emerald-700 text-white flex items-center justify-between shrink-0">
            <div class="flex items-center gap-2.5">
              <div class="p-2 bg-emerald-800/80 rounded-xl">
                <Coins class="w-5 h-5 text-amber-300" />
              </div>
              <div>
                <h3 class="font-extrabold text-sm">Pembagian Dividen Baru</h3>
                <p class="text-[11px] text-emerald-100">
                  Alokasikan dividen dari saldo Laba Ditahan (3201) periode tutup buku
                </p>
              </div>
            </div>
            <button
              @click="showCreateDividendModal = false"
              class="text-white/70 hover:text-white transition cursor-pointer"
            >
              <X class="w-5 h-5" />
            </button>
          </div>

          <!-- Modal Body (Scrollable) -->
          <div class="p-6 space-y-5 overflow-y-auto flex-1 text-xs">
            <!-- Alert Info Akun Laba Ditahan (3201) -->
            <div class="p-3.5 bg-slate-50 border border-slate-200 rounded-xl flex items-center justify-between gap-3">
              <div>
                <div class="text-[11px] text-slate-500 font-bold uppercase tracking-wider">
                  Saldo Akun Laba Ditahan (3201) Tersedia
                </div>
                <div class="text-base font-black text-slate-900 mt-0.5">
                  {{ formatCurrency(dividendSummary.total_retained_earnings) }}
                </div>
              </div>
              <div class="text-right">
                <span
                  class="px-2 py-0.5 rounded-full text-[10px] font-bold"
                  :class="dividendSummary.total_retained_earnings > 0 ? 'bg-emerald-100 text-emerald-800' : 'bg-rose-100 text-rose-800'"
                >
                  {{ dividendSummary.total_retained_earnings > 0 ? 'Saldo Tersedia' : 'Saldo Tidak Cukup' }}
                </span>
                <div class="text-[10px] text-slate-400 mt-0.5">Maksimal penarikan</div>
              </div>
            </div>

            <!-- Form Grid: Periode Tutup Buku & Tanggal -->
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <div>
                <label class="block font-bold text-slate-700 mb-1">
                  Dokumen Tutup Buku (Accounting Closings) *
                </label>
                <SearchableSelect
                  v-model="dividendForm.period_id"
                  :options="dividendPeriodOptions"
                  value-key="id"
                  label-key="name"
                  secondary-key="code"
                  placeholder="-- Pilih Dokumen Tutup Buku --"
                  search-placeholder="Cari no. tutup buku atau rentang tanggal..."
                  :allow-clear="false"
                />
                <p v-if="selectedClosedPeriod" class="mt-1 text-[11px] text-emerald-700 font-sans">
                  Laba bersih tutup buku ini: <strong>{{ formatCurrency(selectedClosedPeriod.net_income) }}</strong>
                  <span v-if="selectedClosedPeriod.total_dividends_drawn > 0" class="text-slate-500 block">
                    (Sudah pernah dibagikan dividen: {{ formatCurrency(selectedClosedPeriod.total_dividends_drawn) }}, Sisa yang dapat dibagikan: <strong>{{ formatCurrency(selectedClosedPeriod.remaining_income) }}</strong>)
                  </span>
                </p>
                <p v-else-if="dividendSummary.eligible_closed_periods.length === 0" class="mt-1 text-[11px] text-rose-600 font-bold">
                  Belum ada dokumen penutupan buku (Tutup Buku) yang berstatus CLOSED. Lakukan Tutup Buku terlebih dahulu di tab 'Tutup Buku' untuk membukukan laba periode ke Laba Ditahan (3201).
                </p>
              </div>

              <div>
                <label class="block font-bold text-slate-700 mb-1">
                  Tanggal Pembagian Dividen *
                </label>
                <input
                  v-model="dividendForm.distribution_date"
                  type="date"
                  class="w-full bg-slate-50 border border-slate-300 rounded-xl px-3 py-2 text-xs font-semibold text-slate-800 focus:outline-none focus:border-emerald-600"
                />
              </div>
            </div>

            <!-- Total Nominal & Rekening Pencairan -->
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <div>
                <label class="block font-bold text-slate-700 mb-1">
                  Total Nominal Dividen (Rp) *
                </label>
                <div class="relative">
                  <span class="absolute left-3 top-1/2 -translate-y-1/2 text-slate-400 font-bold">Rp</span>
                  <input
                    v-model.number="dividendForm.total_amount"
                    @input="onDividendTotalAmountChange"
                    type="number"
                    min="1"
                    placeholder="0"
                    class="w-full bg-slate-50 border rounded-xl pl-9 pr-3 py-2 text-xs font-bold font-mono focus:outline-none"
                    :class="isDividendOverBalance ? 'border-rose-400 bg-rose-50 text-rose-900 focus:border-rose-600' : 'border-slate-300 text-slate-900 focus:border-emerald-600'"
                  />
                </div>
                <p v-if="isDividendOverBalance" class="mt-1 text-[11px] text-rose-600 font-bold">
                  Nominal dividen melebihi saldo Laba Ditahan yang tersedia ({{ formatCurrency(dividendSummary.total_retained_earnings) }})!
                </p>
                <p v-else-if="dividendForm.total_amount > 0" class="mt-1 text-[11px] text-emerald-700 font-semibold font-mono">
                  {{ formatCurrency(dividendForm.total_amount) }}
                </p>
              </div>

              <div>
                <label class="block font-bold text-slate-700 mb-1">
                  Rekening Sumber Pencairan (Kas / Bank) *
                </label>
                <SearchableSelect
                  v-model="dividendForm.payment_account_id"
                  :options="businessPaymentAccountOptions"
                  value-key="id"
                  label-key="name"
                  secondary-key="code"
                  placeholder="-- Pilih Rekening Kas / Bank --"
                  search-placeholder="Cari kas atau rekening bank pencairan..."
                  :allow-clear="false"
                />
                <p class="mt-1 text-[11px] text-slate-400">
                  Uang tunai kasir atau rekening bank toko yang digunakan untuk mencairkan dividen.
                </p>
              </div>
            </div>

            <!-- Rincian Alokasi Pemegang Saham -->
            <div class="space-y-2 border-t border-slate-200 pt-4">
              <div class="flex items-center justify-between">
                <div>
                  <h4 class="font-bold text-slate-800 flex items-center gap-1.5">
                    <Users class="w-4 h-4 text-emerald-700" />
                    <span>Daftar Alokasi Pemegang Saham / Penerima</span>
                  </h4>
                  <p class="text-[11px] text-slate-400">
                    Bagi dividen sesuai persentase kepemilikan atau nominal langsung
                  </p>
                </div>
                <button
                  type="button"
                  @click="addDividendRecipient"
                  class="inline-flex items-center gap-1 px-2.5 py-1 rounded-lg bg-emerald-50 hover:bg-emerald-100 text-emerald-800 border border-emerald-300 text-[11px] font-bold transition cursor-pointer"
                >
                  <Plus class="w-3 h-3" />
                  <span>Tambah Penerima</span>
                </button>
              </div>

              <div class="border border-slate-200 rounded-xl overflow-hidden bg-slate-50/50">
                <table class="w-full text-left text-xs border-collapse">
                  <thead class="bg-slate-100 text-slate-700 font-mono text-[10px] uppercase">
                    <tr>
                      <th class="py-2 px-3">Nama Penerima / Owner *</th>
                      <th class="py-2 px-3 w-24 text-center">Porsi (%)</th>
                      <th class="py-2 px-3 w-40 text-right">Nominal (Rp) *</th>
                      <th class="py-2 px-3">Keterangan / No. Rek</th>
                      <th class="py-2 px-2 text-center w-10"></th>
                    </tr>
                  </thead>
                  <tbody class="divide-y divide-slate-200 bg-white">
                    <tr v-for="(rec, idx) in dividendForm.recipients" :key="idx">
                      <td class="py-2 px-3">
                        <input
                          v-model="rec.name"
                          type="text"
                          placeholder="Nama Pemegang Saham"
                          class="w-full bg-slate-50 border border-slate-300 rounded-lg px-2.5 py-1 text-xs text-slate-800 focus:outline-none focus:border-emerald-600"
                        />
                      </td>
                      <td class="py-2 px-3 text-center">
                        <div class="relative">
                          <input
                            v-model.number="rec.share_percentage"
                            @input="onRecipientPercentageChange(idx)"
                            type="number"
                            min="0"
                            max="100"
                            step="0.01"
                            placeholder="100"
                            class="w-full bg-slate-50 border border-slate-300 rounded-lg px-2 py-1 text-xs text-center font-mono focus:outline-none focus:border-emerald-600"
                          />
                        </div>
                      </td>
                      <td class="py-2 px-3 text-right">
                        <input
                          v-model.number="rec.amount"
                          @input="onRecipientAmountChange(idx)"
                          type="number"
                          min="1"
                          placeholder="0"
                          class="w-full bg-slate-50 border border-slate-300 rounded-lg px-2.5 py-1 text-xs text-right font-mono font-bold text-emerald-800 focus:outline-none focus:border-emerald-600"
                        />
                      </td>
                      <td class="py-2 px-3">
                        <input
                          v-model="rec.notes"
                          type="text"
                          placeholder="Rekening / Catatan..."
                          class="w-full bg-slate-50 border border-slate-300 rounded-lg px-2.5 py-1 text-xs text-slate-700 focus:outline-none focus:border-emerald-600"
                        />
                      </td>
                      <td class="py-2 px-2 text-center">
                        <button
                          type="button"
                          @click="removeDividendRecipient(idx)"
                          :disabled="dividendForm.recipients.length <= 1"
                          class="p-1 rounded text-slate-400 hover:text-rose-600 transition disabled:opacity-30 disabled:cursor-not-allowed cursor-pointer"
                          title="Hapus baris"
                        >
                          <Trash2 class="w-3.5 h-3.5" />
                        </button>
                      </td>
                    </tr>
                  </tbody>
                </table>

                <!-- Summary Bar Penerima -->
                <div class="p-2.5 bg-slate-100/80 border-t border-slate-200 flex items-center justify-between font-mono text-[11px]">
                  <div class="text-slate-600">
                    Total Porsi:
                    <span
                      class="font-bold"
                      :class="Math.round(dividendRecipientsTotalPercentage) === 100 ? 'text-emerald-700' : 'text-amber-700'"
                    >
                      {{ dividendRecipientsTotalPercentage.toFixed(1) }}%
                    </span>
                  </div>
                  <div class="text-right">
                    <span class="text-slate-600 mr-2">Total Alokasi:</span>
                    <span
                      class="font-extrabold text-xs"
                      :class="dividendRecipientsTotalAmount === dividendForm.total_amount ? 'text-emerald-700' : 'text-rose-600'"
                    >
                      {{ formatCurrency(dividendRecipientsTotalAmount) }}
                    </span>
                    <span v-if="dividendRecipientsTotalAmount !== dividendForm.total_amount" class="text-rose-600 text-[10px] block font-sans">
                      (Selisih: {{ formatCurrency(dividendForm.total_amount - dividendRecipientsTotalAmount) }})
                    </span>
                  </div>
                </div>
              </div>
            </div>

            <!-- Pratinjau Jurnal Otomatis -->
            <div class="border border-emerald-200 bg-emerald-50/50 rounded-xl p-3.5 space-y-2">
              <div class="flex items-center justify-between">
                <span class="font-bold text-emerald-950 text-xs flex items-center gap-1.5">
                  <BookOpen class="w-3.5 h-3.5 text-emerald-700" />
                  <span>Pratinjau Otomatis Jurnal Penyesuaian Dividen</span>
                </span>
                <span class="px-2 py-0.5 rounded-full text-[10px] font-bold bg-emerald-100 text-emerald-800 border border-emerald-300">
                  Balanced (Seimbang) ✓
                </span>
              </div>
              <div class="text-[11px] font-mono space-y-1 bg-white p-2.5 rounded-lg border border-emerald-200">
                <div class="flex justify-between items-center text-slate-800">
                  <div>
                    <span class="font-bold text-sky-800">Debet (Dr):</span> 3201 - Laba Ditahan (Retained Earnings)
                  </div>
                  <div class="font-bold text-emerald-700">
                    {{ formatCurrency(dividendForm.total_amount || 0) }}
                  </div>
                </div>
                <div class="flex justify-between items-center text-slate-800 pl-4 border-t border-slate-100 pt-1">
                  <div>
                    <span class="font-bold text-rose-800">Kredit (Cr):</span>
                    {{ businessPaymentAccountOptions.find(a => a.id === dividendForm.payment_account_id)?.name || 'Rekening Kas / Bank Terpilih' }}
                  </div>
                  <div class="font-bold text-emerald-700">
                    {{ formatCurrency(dividendForm.total_amount || 0) }}
                  </div>
                </div>
              </div>
            </div>

            <!-- Catatan Notulen / RUPS -->
            <div>
              <label class="block font-bold text-slate-700 mb-1">
                Catatan / Keputusan RUPS / Memo
              </label>
              <textarea
                v-model="dividendForm.notes"
                rows="2"
                placeholder="Contoh: Berdasarkan Notulen RUPS Tahunan tanggal ... untuk pembagian laba bersih tahun buku ..."
                class="w-full bg-slate-50 border border-slate-300 rounded-xl px-3 py-2 text-xs text-slate-800 placeholder-slate-400 focus:outline-none focus:border-emerald-600"
              ></textarea>
            </div>
          </div>

          <!-- Footer Actions -->
          <div class="px-6 py-4 bg-slate-50 border-t border-slate-200 flex items-center justify-between shrink-0">
            <button
              type="button"
              @click="showCreateDividendModal = false"
              class="px-4 py-2 text-xs font-bold text-slate-600 bg-white hover:bg-slate-100 border border-slate-300 rounded-xl transition cursor-pointer"
            >
              Batal
            </button>
            <button
              type="button"
              @click="handleSaveDividend"
              :disabled="isSubmittingDividend || isDividendOverBalance || dividendForm.total_amount <= 0 || !dividendForm.period_id"
              class="px-5 py-2 text-xs font-bold text-white bg-emerald-600 hover:bg-emerald-700 rounded-xl transition shadow-xs cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed flex items-center gap-1.5"
            >
              <Coins class="w-3.5 h-3.5" />
              <span>{{ isSubmittingDividend ? 'Memproses Penjurnalan...' : 'Proses & Bukukan Dividen' }}</span>
            </button>
          </div>
        </div>
      </div>
      </Teleport>

      <!-- ===================================================================== -->
      <!-- MODAL: DETAIL BUKTI PEMBAGIAN DIVIDEN -->
      <!-- ===================================================================== -->
      <Teleport to="body">
      <div
        v-if="showDetailDividendModal && selectedDividendDetail"
        class="fixed inset-0 z-[60] flex items-center justify-center p-4 bg-slate-950/70 backdrop-blur-sm overflow-y-auto"
        @click.self="showDetailDividendModal = false"
      >
        <div class="bg-white rounded-2xl border border-slate-200 shadow-2xl w-full max-w-xl overflow-hidden my-6 max-h-[90vh] flex flex-col">
          <div class="px-6 py-4 bg-slate-900 text-white flex items-center justify-between shrink-0">
            <div class="flex items-center gap-2">
              <Coins class="w-5 h-5 text-amber-300" />
              <div>
                <h3 class="font-extrabold text-sm">Bukti Pembagian Dividen Laba</h3>
                <p class="text-[11px] text-slate-400 font-mono">{{ selectedDividendDetail.distribution_number }}</p>
              </div>
            </div>
            <button
              @click="showDetailDividendModal = false"
              class="text-white/70 hover:text-white transition cursor-pointer"
            >
              <X class="w-5 h-5" />
            </button>
          </div>

          <div id="printable-dividend-voucher" class="p-6 space-y-4 overflow-y-auto flex-1 text-xs">
            <div class="grid grid-cols-2 gap-3 p-3.5 bg-slate-50 border border-slate-200 rounded-xl">
              <div>
                <span class="text-[10px] text-slate-400 uppercase font-bold">Tanggal Distribusi</span>
                <p class="font-bold text-slate-800">{{ formatDate(selectedDividendDetail.distribution_date) }}</p>
              </div>
              <div>
                <span class="text-[10px] text-slate-400 uppercase font-bold">Periode Tutup Buku</span>
                <p class="font-bold text-sky-800">{{ selectedDividendDetail.period_name || 'Periode Terpilih' }}</p>
              </div>
              <div>
                <span class="text-[10px] text-slate-400 uppercase font-bold">Rekening Pencairan</span>
                <p class="font-bold text-slate-800">{{ selectedDividendDetail.payment_account_code }} - {{ selectedDividendDetail.payment_account_name }}</p>
              </div>
              <div>
                <span class="text-[10px] text-slate-400 uppercase font-bold">Total Nilai Dividen</span>
                <p class="font-extrabold text-emerald-700 text-sm">{{ formatCurrency(selectedDividendDetail.total_amount) }}</p>
              </div>
            </div>

            <div v-if="selectedDividendDetail.journal_number" class="p-3 bg-sky-50 border border-sky-200 rounded-xl flex items-center justify-between">
              <div>
                <span class="text-[10px] text-sky-700 font-bold uppercase">No. Jurnal Umum Terkait</span>
                <p class="font-mono font-extrabold text-sky-900">{{ selectedDividendDetail.journal_number }}</p>
              </div>
              <button
                @click="viewDividendJournal(selectedDividendDetail.journal_entry_id)"
                class="px-2.5 py-1 rounded-lg bg-sky-600 hover:bg-sky-700 text-white text-[11px] font-bold transition cursor-pointer flex items-center gap-1"
              >
                <BookOpen class="w-3 h-3" />
                <span>Buka Jurnal</span>
              </button>
            </div>

            <!-- Rincian Penerima -->
            <div class="space-y-1.5">
              <h4 class="font-bold text-slate-800 flex items-center gap-1">
                <Users class="w-4 h-4 text-emerald-700" />
                <span>Rincian Pembagian Pemegang Saham</span>
              </h4>
              <div class="border border-slate-200 rounded-xl overflow-hidden">
                <table class="w-full text-left text-xs border-collapse">
                  <thead class="bg-slate-100 font-mono text-[10px] text-slate-700 uppercase">
                    <tr>
                      <th class="py-2 px-3">Nama Penerima</th>
                      <th class="py-2 px-3 text-center">Porsi (%)</th>
                      <th class="py-2 px-3 text-right">Nominal</th>
                      <th class="py-2 px-3">Keterangan</th>
                    </tr>
                  </thead>
                  <tbody class="divide-y divide-slate-200 font-mono">
                    <tr v-for="(r, idx) in (selectedDividendDetail.recipients_data || [])" :key="idx">
                      <td class="py-2 px-3 font-sans font-bold text-slate-900">{{ r.name }}</td>
                      <td class="py-2 px-3 text-center">{{ r.share_percentage }}%</td>
                      <td class="py-2 px-3 text-right font-bold text-emerald-700">{{ formatCurrency(r.amount) }}</td>
                      <td class="py-2 px-3 font-sans text-slate-500 text-[11px]">{{ r.notes || '-' }}</td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>

            <div v-if="selectedDividendDetail.notes" class="p-3 bg-slate-50 border border-slate-200 rounded-xl">
              <span class="text-[10px] text-slate-400 uppercase font-bold block mb-1">Catatan / Keterangan</span>
              <p class="text-slate-700 italic font-sans">{{ selectedDividendDetail.notes }}</p>
            </div>
          </div>

          <div class="px-6 py-3 bg-slate-50 border-t border-slate-200 flex items-center justify-between shrink-0">
            <button
              @click="handlePrintVoucher"
              class="px-3 py-1.5 rounded-xl bg-white border border-slate-300 text-slate-700 hover:bg-slate-100 text-xs font-bold transition flex items-center gap-1.5 cursor-pointer"
            >
              <Printer class="w-3.5 h-3.5" />
              <span>Cetak Bukti</span>
            </button>
            <button
              @click="showDetailDividendModal = false"
              class="px-4 py-1.5 rounded-xl bg-slate-800 hover:bg-slate-900 text-white text-xs font-bold transition cursor-pointer"
            >
              Tutup
            </button>
          </div>
        </div>
      </div>
      </Teleport>

      <!-- ===================================================================== -->
      <!-- MODAL: KONFIRMASI BATALKAN DIVIDEN -->
      <!-- ===================================================================== -->
      <Teleport to="body">
      <div
        v-if="showCancelDividendModal && dividendToCancel"
        class="fixed inset-0 z-[60] flex items-center justify-center p-4 bg-slate-950/70 backdrop-blur-sm"
        @click.self="showCancelDividendModal = false"
      >
        <div class="bg-white rounded-2xl border border-slate-200 shadow-2xl w-full max-w-md overflow-hidden">
          <div class="p-6 text-center space-y-4">
            <div class="w-12 h-12 rounded-full bg-rose-100 text-rose-700 flex items-center justify-center mx-auto">
              <AlertTriangle class="w-6 h-6" />
            </div>
            <div>
              <h3 class="font-extrabold text-base text-slate-900">Batalkan Pembagian Dividen?</h3>
              <p class="text-xs text-slate-500 mt-1">
                Apakah Anda yakin ingin membatalkan transaksi dividen <strong>{{ dividendToCancel.distribution_number }}</strong> sejumlah <strong>{{ formatCurrency(dividendToCancel.total_amount) }}</strong>?
              </p>
            </div>
            <div class="p-3 bg-rose-50 border border-rose-200 rounded-xl text-left text-xs text-rose-900 space-y-1">
              <p class="font-bold">Konsekuensi Pembatalan:</p>
              <ul class="list-disc list-inside text-[11px] text-rose-800 space-y-0.5">
                <li>Jurnal umum terkait akan otomatis dibalik (reversal)</li>
                <li>Saldo akun Laba Ditahan (3201) akan dikembalikan</li>
                <li>Rekening kas/bank pencairan akan didebet kembali</li>
              </ul>
            </div>
          </div>
          <div class="px-6 py-4 bg-slate-50 border-t border-slate-200 flex items-center justify-end gap-3">
            <button
              @click="showCancelDividendModal = false"
              class="px-4 py-2 text-xs font-bold text-slate-600 bg-white hover:bg-slate-100 border border-slate-300 rounded-xl transition cursor-pointer"
            >
              Batal
            </button>
            <button
              @click="handleConfirmCancelDividend"
              :disabled="isCancellingDividend"
              class="px-4 py-2 text-xs font-bold text-white bg-rose-600 hover:bg-rose-700 rounded-xl transition cursor-pointer disabled:opacity-50"
            >
              {{ isCancellingDividend ? 'Membatalkan...' : 'Ya, Batalkan Dividen' }}
            </button>
          </div>
        </div>
      </div>
      </Teleport>

      <!-- ========================================================================= -->
      <!-- UNIVERSAL PRINT ACCOUNTING REPORT MODAL (ALL MODULES TO PDF) -->
      <!-- ========================================================================= -->
      <Teleport to="body">
      <div
        v-if="showPrintReportModal"
        class="fixed inset-0 z-[9999] flex flex-col bg-slate-950/85 backdrop-blur-md overflow-hidden"
      >
          <!-- Modal Control Header (Hidden during print) -->
          <div class="bg-slate-900 border-b border-slate-800 px-6 py-3.5 flex flex-wrap items-center justify-between gap-4 shrink-0 shadow-lg print:hidden">
            <div class="flex items-center gap-3">
              <div class="w-9 h-9 rounded-xl bg-emerald-600/20 text-emerald-400 border border-emerald-500/30 flex items-center justify-center">
                <Printer class="w-5 h-5" />
              </div>
              <div>
                <h3 class="text-sm font-black text-white flex items-center gap-2">
                  <span>Pratinjau Cetak Laporan Keuangan (PDF)</span>
                  <span class="px-2 py-0.5 rounded-full text-[10px] font-mono bg-emerald-900/60 text-emerald-300 border border-emerald-600/40">
                    Format Dokumen Resmi
                  </span>
                </h3>
                <p class="text-xs text-slate-400">
                  Gunakan menu cetak browser atau simpan langsung ke file PDF
                </p>
              </div>
            </div>

            <!-- Report Switcher Tabs inside Print Preview Modal -->
            <div class="flex flex-wrap items-center gap-1.5 bg-slate-800/80 p-1 rounded-xl border border-slate-700/60 max-w-2xl overflow-x-auto">
              <button
                v-for="tile in accountingMenuTiles"
                :key="tile.id"
                @click="handleSwitchPrintReport(tile.id)"
                class="px-2.5 py-1 rounded-lg text-xs font-bold transition flex items-center gap-1.5 cursor-pointer whitespace-nowrap"
                :class="reportToPrint === tile.id 
                  ? 'bg-emerald-600 text-white shadow-xs' 
                  : 'text-slate-300 hover:text-white hover:bg-slate-700/60'"
              >
                <Icon :icon="tile.iconName" class="w-4 h-4" />
                <span>{{ tile.title }}</span>
              </button>
            </div>

            <!-- Action Buttons -->
            <div class="flex items-center gap-2.5">
              <button
                @click="triggerPrintReport"
                class="inline-flex items-center gap-2 px-4 py-2 rounded-xl bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-black shadow-lg shadow-emerald-900/30 transition active:scale-95 cursor-pointer"
              >
                <Printer class="w-4 h-4" />
                <span>Cetak / Simpan PDF</span>
              </button>
              <button
                @click="showPrintReportModal = false"
                class="inline-flex items-center gap-1.5 px-3 py-2 rounded-xl bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs font-bold transition border border-slate-700 cursor-pointer"
              >
                <X class="w-4 h-4" />
                <span>Tutup</span>
              </button>
            </div>
          </div>

          <!-- Document Preview Area (Scrollable in modal, isolated in print) -->
          <div class="flex-1 overflow-y-auto p-4 md:p-8 bg-slate-900/40 print:p-0 print:overflow-visible print:bg-white">
            <div
              id="printable-accounting-report"
              class="max-w-4xl mx-auto bg-white p-8 md:p-12 shadow-2xl rounded-2xl border border-slate-200 text-slate-900 font-sans print:shadow-none print:border-none print:p-0 print:max-w-none print:rounded-none"
            >
              <!-- KOP RESMI LAPORAN KEUANGAN -->
              <div class="border-b-2 border-slate-900 pb-5 mb-6">
                <div class="flex items-start justify-between gap-4">
                  <div class="flex items-center gap-4">
                    <img
                      v-if="storeBrandingStore.storeLogoUrl"
                      :src="storeBrandingStore.storeLogoUrl"
                      alt="Logo"
                      class="w-16 h-16 object-contain rounded-xl border border-slate-200 p-1"
                    />
                    <div v-else class="w-16 h-16 rounded-xl bg-slate-900 text-white font-black flex items-center justify-center text-xl tracking-wider">
                      POS
                    </div>
                    <div>
                      <h1 class="text-xl md:text-2xl font-black text-slate-900 tracking-tight uppercase">
                        {{ storeBrandingStore.storeName || 'NOTANE RETAIL STORE' }}
                      </h1>
                      <p class="text-xs text-slate-600 font-bold mt-0.5">
                        Sistem Pembukuan Standar Keuangan & Akuntansi Ritel Terintegrasi
                      </p>
                      <p class="text-[11px] text-slate-500 font-mono mt-0.5">
                        Double-Entry Bookkeeping & Real-Time Financial Ledger System
                      </p>
                    </div>
                  </div>

                  <div class="text-right font-mono text-[11px] text-slate-500 space-y-0.5 shrink-0">
                    <div>Entitas: <strong class="text-slate-800">{{ selectedOutletName }}</strong></div>
                    <div>Dicetak: <strong class="text-slate-800">{{ formatDate(todayStr) }}</strong></div>
                    <div>Operator: <strong class="text-slate-800">{{ authenticatedUserSignature }}</strong></div>
                  </div>
                </div>

                <!-- Judul Spesifik Laporan -->
                <div class="mt-6 text-center bg-slate-50 border border-slate-200 rounded-xl py-3 px-4">
                  <h2 class="text-base md:text-lg font-black text-slate-900 tracking-wide uppercase">
                    {{ REPORT_METADATA[reportToPrint]?.title }}
                  </h2>
                  <p class="text-xs text-slate-600 font-semibold mt-0.5 font-mono">
                    {{ REPORT_METADATA[reportToPrint]?.subtitle }}
                  </p>
                  <p class="text-[11px] text-emerald-800 font-bold mt-1">
                    Periode / Posisi:
                    <span v-if="reportToPrint === 'income' || reportToPrint === 'journal' || reportToPrint === 'capital'">
                      {{ formatDate(startDate) }} s/d {{ formatDate(endDate) }}
                    </span>
                    <span v-else-if="reportToPrint === 'balance_sheet'">
                      Per Tanggal {{ formatDate(asOfDate) }}
                    </span>
                    <span v-else>
                      Data Master / Posisi Terkini
                    </span>
                  </p>
                </div>
              </div>

              <!-- Loading Indicator saat menyiapkan dokumen cetak -->
              <div v-if="isPreparingPrintData" class="py-16 text-center text-slate-500 font-mono">
                <RefreshCw class="w-8 h-8 animate-spin mx-auto mb-3 text-emerald-600" />
                <p class="font-bold text-xs text-slate-800">Menyiapkan Laporan Resmi...</p>
                <p class="text-[11px] text-slate-400 mt-0.5">Memuat konsolidasi pembukuan double-entry</p>
              </div>

              <!-- ======================================================= -->
              <!-- 1. KONTEN CETAK: LABA RUGI (INCOME STATEMENT) -->
              <!-- ======================================================= -->
              <div v-else-if="reportToPrint === 'income' && incomeStatement" class="space-y-6">
                <!-- Summary Metrics Bar -->
                <div class="grid grid-cols-3 gap-3 p-3 bg-slate-50 border border-slate-200 rounded-xl text-center font-mono">
                  <div>
                    <span class="text-[10px] text-slate-500 uppercase">Total Pendapatan</span>
                    <p class="font-extrabold text-sm text-slate-900">{{ formatCurrency(incomeStatement.total_revenue) }}</p>
                  </div>
                  <div>
                    <span class="text-[10px] text-slate-500 uppercase">Total HPP & Beban</span>
                    <p class="font-extrabold text-sm text-slate-900">{{ formatCurrency(Number(incomeStatement.total_cogs || 0) + Number(incomeStatement.total_operating_expenses || incomeStatement.total_expenses || 0)) }}</p>
                  </div>
                  <div>
                    <span class="text-[10px] text-slate-500 uppercase">Laba Bersih (Net Profit)</span>
                    <p class="font-black text-sm" :class="Number(incomeStatement.net_profit) >= 0 ? 'text-emerald-700' : 'text-rose-700'">
                      {{ formatCurrency(incomeStatement.net_profit) }}
                    </p>
                  </div>
                </div>

                <!-- I. Pendapatan Operasional -->
                <div class="space-y-2">
                  <div class="flex justify-between items-center bg-slate-100 px-3 py-1.5 rounded-lg font-bold text-xs text-slate-800 font-mono">
                    <span>I. PENDAPATAN OPERASIONAL & PENJUALAN</span>
                    <span>{{ formatCurrency(incomeStatement.total_revenue) }}</span>
                  </div>
                  <table class="w-full text-xs font-mono">
                    <tbody class="divide-y divide-slate-100">
                      <tr v-if="!incomeStatement.revenue_details || incomeStatement.revenue_details.length === 0">
                        <td colspan="3" class="py-2 text-slate-400 italic text-center">Tidak ada transaksi pendapatan</td>
                      </tr>
                      <tr v-for="item in (incomeStatement.revenue_details || [])" :key="item.code" class="py-1.5">
                        <td class="py-1.5 pl-3 w-28 text-slate-500">{{ item.code }}</td>
                        <td class="py-1.5 font-sans">{{ item.name }}</td>
                        <td class="py-1.5 pr-3 text-right font-bold text-slate-800">{{ formatCurrency(item.amount) }}</td>
                      </tr>
                    </tbody>
                  </table>
                </div>

                <!-- II. HPP -->
                <div class="space-y-2">
                  <div class="flex justify-between items-center bg-slate-100 px-3 py-1.5 rounded-lg font-bold text-xs text-slate-800 font-mono">
                    <span>II. BEBAN POKOK PENJUALAN (HPP)</span>
                    <span class="text-rose-800">({{ formatCurrency(incomeStatement.total_cogs) }})</span>
                  </div>
                  <table class="w-full text-xs font-mono">
                    <tbody class="divide-y divide-slate-100">
                      <tr v-if="printCogsDetails.length === 0">
                        <td colspan="3" class="py-2 text-slate-400 italic text-center">Tidak ada transaksi HPP</td>
                      </tr>
                      <tr v-for="item in printCogsDetails" :key="item.code">
                        <td class="py-1.5 pl-3 w-28 text-slate-500">{{ item.code }}</td>
                        <td class="py-1.5 font-sans">{{ item.name }}</td>
                        <td class="py-1.5 pr-3 text-right font-bold text-rose-700">({{ formatCurrency(item.amount) }})</td>
                      </tr>
                    </tbody>
                  </table>
                </div>

                <!-- Laba Kotor Highlight -->
                <div class="flex justify-between items-center bg-emerald-50 border border-emerald-200 px-4 py-2.5 rounded-xl font-bold text-xs text-emerald-950 font-mono">
                  <span>LABA KOTOR (GROSS PROFIT)</span>
                  <span class="text-sm font-black text-emerald-800">{{ formatCurrency(incomeStatement.gross_profit) }}</span>
                </div>

                <!-- III. Beban Operasional -->
                <div class="space-y-2">
                  <div class="flex justify-between items-center bg-slate-100 px-3 py-1.5 rounded-lg font-bold text-xs text-slate-800 font-mono">
                    <span>III. BEBAN OPERASIONAL & UMUM</span>
                    <span class="text-rose-800">({{ formatCurrency(incomeStatement.total_operating_expenses || incomeStatement.total_expenses || 0) }})</span>
                  </div>
                  <table class="w-full text-xs font-mono">
                    <tbody class="divide-y divide-slate-100">
                      <tr v-if="printOperatingExpenseDetails.length === 0">
                        <td colspan="3" class="py-2 text-slate-400 italic text-center">Tidak ada transaksi beban operasional</td>
                      </tr>
                      <tr v-for="item in printOperatingExpenseDetails" :key="item.code">
                        <td class="py-1.5 pl-3 w-28 text-slate-500">{{ item.code }}</td>
                        <td class="py-1.5 font-sans">{{ item.name }}</td>
                        <td class="py-1.5 pr-3 text-right font-bold text-rose-700">({{ formatCurrency(item.amount) }})</td>
                      </tr>
                    </tbody>
                  </table>
                </div>

                <!-- Laba Bersih Final -->
                <div class="flex justify-between items-center bg-slate-900 text-white px-5 py-3 rounded-xl font-black text-sm font-mono shadow-sm">
                  <span>LABA BERSIH TAHUN/PERIODE BERJALAN (NET PROFIT)</span>
                  <span class="text-base text-emerald-300">{{ formatCurrency(incomeStatement.net_profit) }}</span>
                </div>
              </div>

              <!-- ======================================================= -->
              <!-- 2. KONTEN CETAK: POSISI ASET & NERACA (BALANCE SHEET) -->
              <!-- ======================================================= -->
              <div v-else-if="reportToPrint === 'balance_sheet' && balanceSheet" class="space-y-6">
                <div class="grid grid-cols-2 gap-6">
                  <!-- AKTIVA -->
                  <div class="border border-slate-200 rounded-xl p-4 space-y-3">
                    <div class="border-b border-slate-200 pb-2 flex justify-between items-center">
                      <h3 class="font-black text-xs uppercase tracking-wider text-slate-900">AKTIVA (ASSETS)</h3>
                      <span class="font-mono font-bold text-xs text-emerald-800">{{ formatCurrency(balanceSheet.total_assets) }}</span>
                    </div>

                    <div class="space-y-2">
                      <span class="text-[11px] font-bold text-slate-600 block uppercase font-mono">A. ASET LANCAR</span>
                      <table class="w-full text-xs font-mono">
                        <tbody class="divide-y divide-slate-100">
                          <tr v-if="currentAssetDetails.length === 0">
                            <td colspan="3" class="py-1 text-slate-400 italic">Tidak ada saldo aset lancar</td>
                          </tr>
                          <tr v-for="a in currentAssetDetails" :key="a.code">
                            <td class="py-1 text-slate-500 w-20">{{ a.code }}</td>
                            <td class="py-1 font-sans">{{ a.name }}</td>
                            <td class="py-1 text-right font-bold">{{ formatCurrency(a.amount ?? a.balance ?? 0) }}</td>
                          </tr>
                        </tbody>
                      </table>
                    </div>

                    <div v-if="nonCurrentAssetDetails.length > 0" class="space-y-2 pt-2 border-t border-slate-100">
                      <span class="text-[11px] font-bold text-slate-600 block uppercase font-mono">B. ASET TIDAK LANCAR</span>
                      <table class="w-full text-xs font-mono">
                        <tbody class="divide-y divide-slate-100">
                          <tr v-for="a in nonCurrentAssetDetails" :key="a.code">
                            <td class="py-1 text-slate-500 w-20">{{ a.code }}</td>
                            <td class="py-1 font-sans">{{ a.name }}</td>
                            <td class="py-1 text-right font-bold">{{ formatCurrency(a.amount ?? a.balance ?? 0) }}</td>
                          </tr>
                        </tbody>
                      </table>
                    </div>
                  </div>

                  <!-- PASIVA -->
                  <div class="border border-slate-200 rounded-xl p-4 space-y-3">
                    <div class="border-b border-slate-200 pb-2 flex justify-between items-center">
                      <h3 class="font-black text-xs uppercase tracking-wider text-slate-900">PASIVA (KEWAJIBAN & EKUITAS)</h3>
                      <span class="font-mono font-bold text-xs text-sky-800">{{ formatCurrency(balanceSheet.total_liabilities_and_equity) }}</span>
                    </div>

                    <div class="space-y-2">
                      <span class="text-[11px] font-bold text-slate-600 block uppercase font-mono">A. KEWAJIBAN / HUTANG</span>
                      <table class="w-full text-xs font-mono">
                        <tbody class="divide-y divide-slate-100">
                          <tr v-if="!balanceSheet.liability_details || balanceSheet.liability_details.length === 0">
                            <td colspan="3" class="py-1 text-slate-400 italic">Nihil</td>
                          </tr>
                          <tr v-for="l in (balanceSheet.liability_details || [])" :key="l.code">
                            <td class="py-1 text-slate-500 w-20">{{ l.code }}</td>
                            <td class="py-1 font-sans">{{ l.name }}</td>
                            <td class="py-1 text-right font-bold">{{ formatCurrency(l.amount ?? l.balance ?? 0) }}</td>
                          </tr>
                        </tbody>
                      </table>
                    </div>

                    <div class="space-y-2 pt-2 border-t border-slate-100">
                      <span class="text-[11px] font-bold text-slate-600 block uppercase font-mono">B. EKUITAS & MODAL USAHA</span>
                      <table class="w-full text-xs font-mono">
                        <tbody class="divide-y divide-slate-100">
                          <tr v-for="e in (balanceSheet.equity_details || [])" :key="e.code">
                            <td class="py-1 text-slate-500 w-20">{{ e.code }}</td>
                            <td class="py-1 font-sans">{{ e.name }}</td>
                            <td class="py-1 text-right font-bold">{{ formatCurrency(e.amount ?? e.balance ?? 0) }}</td>
                          </tr>
                        </tbody>
                      </table>
                    </div>
                  </div>
                </div>

                <!-- Balance Check Summary Bar -->
                <div
                  class="p-3 rounded-xl border flex items-center justify-between text-xs font-mono"
                  :class="balanceSheet.is_balanced ? 'bg-emerald-50 border-emerald-300 text-emerald-900' : 'bg-rose-50 border-rose-300 text-rose-900'"
                >
                  <span class="font-bold flex items-center gap-1.5">
                    <span>STATUS NERACA:</span>
                    <span>{{ balanceSheet.is_balanced ? '✓ SEIMBANG / BALANCE' : '⚠ SELISIH TIDAK SEIMBANG' }}</span>
                  </span>
                  <span>Aktiva: <strong>{{ formatCurrency(balanceSheet.total_assets) }}</strong> | Pasiva: <strong>{{ formatCurrency(balanceSheet.total_liabilities_and_equity) }}</strong></span>
                </div>
              </div>

              <!-- ======================================================= -->
              <!-- 3. KONTEN CETAK: BUKU JURNAL UMUM (JOURNAL) -->
              <!-- ======================================================= -->
              <div v-else-if="reportToPrint === 'journal'" class="space-y-4">
                <table class="w-full text-xs border border-slate-200 border-collapse">
                  <thead class="bg-slate-100 text-slate-700 font-mono text-[11px] uppercase border-b border-slate-200">
                    <tr>
                      <th class="py-2.5 px-3 text-left w-32">No. Jurnal</th>
                      <th class="py-2.5 px-3 text-left w-24">Tanggal</th>
                      <th class="py-2.5 px-3 text-left">Referensi & Keterangan</th>
                      <th class="py-2.5 px-3 text-right w-28">Debet (Rp)</th>
                      <th class="py-2.5 px-3 text-right w-28">Kredit (Rp)</th>
                    </tr>
                  </thead>
                  <tbody class="divide-y divide-slate-200 font-mono">
                    <tr v-if="journalEntries.length === 0">
                      <td colspan="5" class="py-8 text-center text-slate-400 italic">Tidak ada transaksi jurnal pada filter terpilih</td>
                    </tr>
                    <template v-for="jrn in journalEntries" :key="jrn.entry?.id || jrn.id">
                      <tr class="bg-slate-50/60 font-bold">
                        <td class="py-2 px-3 text-emerald-800">
                          <CopyableText :text="jrn.entry ? jrn.entry.journal_number : jrn.journal_number" label="Nomor Jurnal" />
                        </td>
                        <td class="py-2 px-3">{{ formatDate(jrn.entry ? jrn.entry.journal_date : jrn.journal_date) }}</td>
                        <td class="py-2 px-3 font-sans">
                          <span class="px-1.5 py-0.5 rounded text-[10px] bg-slate-200 text-slate-700 mr-1.5 uppercase font-mono">{{ jrn.entry ? jrn.entry.reference_type : jrn.reference_type }}</span>
                          <span>{{ jrn.entry ? jrn.entry.description : jrn.description }}</span>
                        </td>
                        <td class="py-2 px-3 text-right text-slate-900">{{ formatCurrency(jrn.entry ? jrn.entry.total_debit : jrn.total_debit) }}</td>
                        <td class="py-2 px-3 text-right text-slate-900">{{ formatCurrency(jrn.entry ? jrn.entry.total_credit : jrn.total_credit) }}</td>
                      </tr>
                      <tr v-for="line in (jrn.lines || [])" :key="line.id" class="text-[11px] bg-white text-slate-600">
                        <td class="py-1 px-3 pl-6 text-slate-400">{{ line.account_code }}</td>
                        <td class="py-1 px-3"></td>
                        <td class="py-1 px-3 font-sans" :class="Number(line.credit) > 0 ? 'pl-8' : ''">
                          {{ line.account_name }}
                        </td>
                        <td class="py-1 px-3 text-right font-mono">{{ Number(line.debit) > 0 ? formatCurrency(line.debit) : '-' }}</td>
                        <td class="py-1 px-3 text-right font-mono">{{ Number(line.credit) > 0 ? formatCurrency(line.credit) : '-' }}</td>
                      </tr>
                    </template>
                  </tbody>
                </table>
              </div>

              <!-- ======================================================= -->
              <!-- 4. KONTEN CETAK: TUTUP BUKU PERIODE (CLOSINGS) -->
              <!-- ======================================================= -->
              <div v-else-if="reportToPrint === 'closing'" class="space-y-4">
                <table class="w-full text-xs border border-slate-200 border-collapse">
                  <thead class="bg-slate-100 text-slate-700 font-mono text-[11px] uppercase border-b border-slate-200">
                    <tr>
                      <th class="py-2.5 px-3 text-left">Nama Periode</th>
                      <th class="py-2.5 px-3 text-left w-36">Rentang Tanggal</th>
                      <th class="py-2.5 px-3 text-center w-24">Status</th>
                      <th class="py-2.5 px-3 text-left">Catatan Penutupan</th>
                      <th class="py-2.5 px-3 text-left w-28">Ditutup Pada</th>
                    </tr>
                  </thead>
                  <tbody class="divide-y divide-slate-200 font-mono">
                    <tr v-if="accountingPeriods.length === 0">
                      <td colspan="5" class="py-8 text-center text-slate-400 italic">Belum ada data periode akuntansi</td>
                    </tr>
                    <tr v-for="p in accountingPeriods" :key="p.id">
                      <td class="py-2.5 px-3 font-bold font-sans text-slate-900">{{ p.name }}</td>
                      <td class="py-2.5 px-3 text-slate-700">{{ formatDate(p.start_date) }} - {{ formatDate(p.end_date) }}</td>
                      <td class="py-2.5 px-3 text-center">
                        <span class="px-2 py-0.5 rounded text-[10px] font-bold" :class="p.is_closed ? 'bg-rose-100 text-rose-800' : 'bg-emerald-100 text-emerald-800'">
                          {{ p.is_closed ? 'TUTUP (LOCKED)' : 'TERBUKA' }}
                        </span>
                      </td>
                      <td class="py-2.5 px-3 font-sans text-slate-600">{{ p.notes || '-' }}</td>
                      <td class="py-2.5 px-3 text-slate-500 text-[11px]">{{ p.closed_at ? formatDate(p.closed_at) : '-' }}</td>
                    </tr>
                  </tbody>
                </table>
              </div>

              <!-- ======================================================= -->
              <!-- 5. KONTEN CETAK: PEMBAGIAN DIVIDEN (DIVIDENDS) -->
              <!-- ======================================================= -->
              <div v-else-if="reportToPrint === 'dividend'" class="space-y-4">
                <div class="grid grid-cols-2 gap-4 p-3 bg-slate-50 border border-slate-200 rounded-xl text-xs font-mono mb-2">
                  <div>Total Dividen Dibagikan: <strong class="text-emerald-800">{{ formatCurrency(dividendSummary.total_dividends_paid) }}</strong></div>
                  <div class="text-right">Sisa Saldo Laba Ditahan: <strong class="text-slate-800">{{ formatCurrency(dividendSummary.total_retained_earnings) }}</strong></div>
                </div>

                <table class="w-full text-xs border border-slate-200 border-collapse">
                  <thead class="bg-slate-100 text-slate-700 font-mono text-[11px] uppercase border-b border-slate-200">
                    <tr>
                      <th class="py-2.5 px-3 text-left">No. Distribusi & Tanggal</th>
                      <th class="py-2.5 px-3 text-left">Periode Tutup Buku</th>
                      <th class="py-2.5 px-3 text-left">Rekening Pencairan</th>
                      <th class="py-2.5 px-3 text-right">Nominal (Rp)</th>
                      <th class="py-2.5 px-3 text-center w-24">Status</th>
                    </tr>
                  </thead>
                  <tbody class="divide-y divide-slate-200 font-mono">
                    <tr v-if="dividendList.length === 0">
                      <td colspan="5" class="py-8 text-center text-slate-400 italic">Belum ada riwayat pembagian dividen</td>
                    </tr>
                    <tr v-for="d in dividendList" :key="d.id">
                      <td class="py-2.5 px-3">
                        <div class="font-bold text-slate-900">{{ d.distribution_number }}</div>
                        <div class="text-[11px] text-slate-500">{{ formatDate(d.distribution_date) }}</div>
                      </td>
                      <td class="py-2.5 px-3 font-sans text-sky-800 font-semibold">{{ d.period_name || '-' }}</td>
                      <td class="py-2.5 px-3 font-sans">{{ d.payment_account_name || d.payment_account_code }}</td>
                      <td class="py-2.5 px-3 text-right font-bold text-emerald-700">{{ formatCurrency(d.total_amount) }}</td>
                      <td class="py-2.5 px-3 text-center">
                        <span class="px-2 py-0.5 rounded text-[10px] font-bold bg-emerald-100 text-emerald-800">
                          {{ d.journal_entry_id ? 'TERBUKUKAN' : 'DICAIRKAN' }}
                        </span>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>

              <!-- ======================================================= -->
              <!-- 6. KONTEN CETAK: SETORAN MODAL USAHA (CAPITAL) -->
              <!-- ======================================================= -->
              <div v-else-if="reportToPrint === 'capital'" class="space-y-4">
                <div class="p-3 bg-slate-50 border border-slate-200 rounded-xl text-xs font-mono mb-2 flex justify-between items-center">
                  <span>Total Ekuitas Modal Masuk: <strong class="text-emerald-800">{{ formatCurrency(capitalSummary.total_amount) }}</strong></span>
                  <span>Total Transaksi: <strong class="text-slate-800">{{ capitalTotal }} Setoran</strong></span>
                </div>

                <table class="w-full text-xs border border-slate-200 border-collapse">
                  <thead class="bg-slate-100 text-slate-700 font-mono text-[11px] uppercase border-b border-slate-200">
                    <tr>
                      <th class="py-2.5 px-3 text-left">No. Bukti & Tanggal</th>
                      <th class="py-2.5 px-3 text-left">Penyetor / Pemilik</th>
                      <th class="py-2.5 px-3 text-left">Cabang / Outlet</th>
                      <th class="py-2.5 px-3 text-left">Rekening Penerima</th>
                      <th class="py-2.5 px-3 text-right">Jumlah (Rp)</th>
                      <th class="py-2.5 px-3 text-center w-24">Status</th>
                    </tr>
                  </thead>
                  <tbody class="divide-y divide-slate-200 font-mono">
                    <tr v-if="capitalDeposits.length === 0">
                      <td colspan="6" class="py-8 text-center text-slate-400 italic">Belum ada transaksi setoran modal</td>
                    </tr>
                    <tr v-for="c in capitalDeposits" :key="c.id">
                      <td class="py-2.5 px-3">
                        <div class="font-bold text-slate-900">{{ c.deposit_no }}</div>
                        <div class="text-[11px] text-slate-500">{{ formatDate(c.deposit_date) }}</div>
                      </td>
                      <td class="py-2.5 px-3 font-sans font-bold text-slate-800">{{ c.investor_name }}</td>
                      <td class="py-2.5 px-3 font-sans text-slate-600">{{ c.outlet_name || 'Semua Cabang' }}</td>
                      <td class="py-2.5 px-3 font-sans text-slate-600">{{ c.payment_account_name || c.equity_account_name || '-' }}</td>
                      <td class="py-2.5 px-3 text-right font-bold text-emerald-800">{{ formatCurrency(c.amount) }}</td>
                      <td class="py-2.5 px-3 text-center">
                        <span class="px-2 py-0.5 rounded text-[10px] font-bold bg-emerald-100 text-emerald-800">
                          {{ (c.status || 'SELESAI').toUpperCase() }}
                        </span>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>

              <!-- ======================================================= -->
              <!-- 7. KONTEN CETAK: REKENING BANK & KAS (BANK) -->
              <!-- ======================================================= -->
              <div v-else-if="reportToPrint === 'bank'" class="space-y-4">
                <table class="w-full text-xs border border-slate-200 border-collapse">
                  <thead class="bg-slate-100 text-slate-700 font-mono text-[11px] uppercase border-b border-slate-200">
                    <tr>
                      <th class="py-2.5 px-3 text-left">Nama Bank / Kas</th>
                      <th class="py-2.5 px-3 text-left">Nomor Rekening</th>
                      <th class="py-2.5 px-3 text-left">Atas Nama (Pemilik)</th>
                      <th class="py-2.5 px-3 text-left">Cabang / Outlet</th>
                      <th class="py-2.5 px-3 text-center w-28">Jenis Rekening</th>
                      <th class="py-2.5 px-3 text-center w-20">Status</th>
                    </tr>
                  </thead>
                  <tbody class="divide-y divide-slate-200 font-mono">
                    <tr v-if="bankAccounts.length === 0">
                      <td colspan="6" class="py-8 text-center text-slate-400 italic">Belum ada akun bank terdaftar</td>
                    </tr>
                    <tr v-for="b in bankAccounts" :key="b.id">
                      <td class="py-2.5 px-3 font-bold text-slate-900 font-sans">{{ b.bank_name }}</td>
                      <td class="py-2.5 px-3 font-bold text-sky-800">{{ b.account_number }}</td>
                      <td class="py-2.5 px-3 font-sans">{{ b.account_holder }}</td>
                      <td class="py-2.5 px-3 font-sans text-slate-600">{{ b.outlet_name || 'Semua Cabang' }}</td>
                      <td class="py-2.5 px-3 text-center font-sans text-[11px]">
                        <span v-if="b.is_default" class="px-1.5 py-0.5 rounded bg-emerald-100 text-emerald-800 font-bold">UTAMA</span>
                        <span v-else class="text-slate-500">OPERASIONAL</span>
                      </td>
                      <td class="py-2.5 px-3 text-center">
                        <span class="px-2 py-0.5 rounded text-[10px] font-bold" :class="b.is_active ? 'bg-emerald-100 text-emerald-800' : 'bg-slate-100 text-slate-600'">
                          {{ b.is_active ? 'AKTIF' : 'NONAKTIF' }}
                        </span>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>

              <!-- ======================================================= -->
              <!-- 8. KONTEN CETAK: BAGAN AKUN (COA) -->
              <!-- ======================================================= -->
              <div v-else-if="reportToPrint === 'coa'" class="space-y-4">
                <table class="w-full text-xs border border-slate-200 border-collapse">
                  <thead class="bg-slate-100 text-slate-700 font-mono text-[11px] uppercase border-b border-slate-200">
                    <tr>
                      <th class="py-2.5 px-3 text-left w-28">Kode Akun</th>
                      <th class="py-2.5 px-3 text-left">Nama Akun (Bagan Akun)</th>
                      <th class="py-2.5 px-3 text-left w-36">Klasifikasi Akun</th>
                      <th class="py-2.5 px-3 text-center w-28">Saldo Normal</th>
                      <th class="py-2.5 px-3 text-center w-20">Status</th>
                    </tr>
                  </thead>
                  <tbody class="divide-y divide-slate-200 font-mono">
                    <tr v-if="chartOfAccounts.length === 0">
                      <td colspan="5" class="py-8 text-center text-slate-400 italic">Belum ada data bagan akun (COA)</td>
                    </tr>
                      <tr v-for="c in chartOfAccounts" :key="c.id">
                        <td class="py-2 px-3 font-bold text-slate-900">{{ c.code }}</td>
                        <td class="py-2 px-3 font-sans font-medium text-slate-800">{{ c.name }}</td>
                        <td class="py-2 px-3 font-mono text-[11px]">
                          <span v-if="c.account_category_id === 1" class="text-emerald-700 font-bold">ASSET</span>
                          <span v-else-if="c.account_category_id === 2" class="text-amber-700 font-bold">LIABILITY</span>
                          <span v-else-if="c.account_category_id === 3" class="text-purple-700 font-bold">EQUITY</span>
                          <span v-else-if="c.account_category_id === 4" class="text-sky-700 font-bold">REVENUE</span>
                          <span v-else-if="c.account_category_id === 5" class="text-rose-700 font-bold">EXPENSE</span>
                          <span v-else class="text-slate-600 font-bold">{{ c.category_name || '-' }}</span>
                        </td>
                        <td class="py-2 px-3 text-center">
                          <span
                            class="px-2 py-0.5 rounded text-[10px] font-bold"
                            :class="(c.account_category_id === 1 || c.account_category_id === 5 || c.normal_balance === 'debit') ? 'bg-indigo-100 text-indigo-800' : 'bg-amber-100 text-amber-800'"
                          >
                            {{ (c.account_category_id === 1 || c.account_category_id === 5 || c.normal_balance === 'debit') ? 'DEBIT' : 'KREDIT' }}
                          </span>
                        </td>
                        <td class="py-2 px-3 text-center">
                          <span class="px-2 py-0.5 rounded text-[10px] font-bold" :class="c.is_active ? 'bg-emerald-100 text-emerald-800' : 'bg-slate-100 text-slate-600'">
                            {{ c.is_active ? 'AKTIF' : 'NONAKTIF' }}
                          </span>
                        </td>
                      </tr>
                  </tbody>
                </table>
              </div>

              <!-- LEMBAR PENGESAHAN & TANDA TANGAN FORMAL -->
              <div class="mt-12 pt-8 border-t-2 border-slate-900 text-xs">
                <p class="text-[11px] text-slate-500 italic mb-8">
                  Dokumen laporan keuangan ini digenerate secara otomatis oleh sistem akuntansi Notane Retail POS dan dapat digunakan sebagai laporan resmi pertanggungjawaban operasional dan keuangan.
                </p>
                <div class="grid grid-cols-2 gap-12 text-center">
                  <div>
                    <p class="text-slate-600 font-bold mb-20">Disiapkan Oleh,</p>
                    <div class="border-t border-slate-400 pt-1.5 w-52 mx-auto font-bold font-mono text-slate-900">
                      ( {{ authenticatedUserSignature }} )
                    </div>
                    <p class="text-[10px] text-slate-500 font-mono mt-0.5">{{ authenticatedUserRole }}</p>
                  </div>
                  <div>
                    <p class="text-slate-600 font-bold mb-20">Disetujui Oleh,</p>
                    <div class="border-t border-slate-400 pt-1.5 w-52 mx-auto font-bold font-mono text-slate-800">
                      (&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;)
                    </div>
                    <p class="text-[10px] text-slate-500 font-mono mt-0.5">Owner / Direktur</p>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </Teleport>
  </AdminLayout>
</template>

<style scoped>
@media print {
  /* Sembunyikan elemen non-cetak */
  body * {
    visibility: hidden !important;
  }

  /* Tampilkan hanya area printable yang sedang aktif */
  #printable-accounting-report,
  #printable-accounting-report * {
    visibility: visible !important;
  }

  #printable-capital-voucher,
  #printable-capital-voucher * {
    visibility: visible !important;
  }

  #printable-dividend-voucher,
  #printable-dividend-voucher * {
    visibility: visible !important;
  }

  #printable-accounting-report {
    position: absolute !important;
    left: 0 !important;
    top: 0 !important;
    width: 100% !important;
    margin: 0 !important;
    padding: 10mm !important;
    background: #ffffff !important;
    color: #000000 !important;
    box-shadow: none !important;
    border: none !important;
  }

  #printable-capital-voucher,
  #printable-dividend-voucher {
    position: absolute !important;
    left: 0 !important;
    top: 0 !important;
    width: 100% !important;
    margin: 0 !important;
    padding: 10mm !important;
    background: #ffffff !important;
    color: #000000 !important;
  }

  @page {
    size: A4 portrait;
    margin: 10mm;
  }
}
</style>
