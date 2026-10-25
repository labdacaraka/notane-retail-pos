<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, computed } from 'vue'
import AdminLayout from '@/layouts/AdminLayout.vue'
import PaginationBar from '@/components/common/PaginationBar.vue'
import SearchableSelect from '@/components/common/SearchableSelect.vue'
import {
  Search,
  RefreshCw,
  Plus,
  Eye,
  CheckCircle2,
  AlertCircle,
  X,
  Trash2,
  Edit2,
  Truck,
  Building2,
  FileText,
  Clock,
  Check,
  ArrowUpDown,
  ArrowUp,
  ArrowDown,
  Package,
  RotateCcw,
  FileSpreadsheet,
} from 'lucide-vue-next'
import { Icon } from '@iconify/vue'
import { api } from '@/services/api'
import { useDialog } from '@/composables/useDialog'
import { useLocationStore } from '@/stores/locationStore'
import { useAuthStore } from '@/stores/authStore'
import { useExportStore } from '@/stores/exportStore'
import { formatCurrency, formatQty, formatDate } from '@/utils/formatters'

const locationStore = useLocationStore()
const authStore = useAuthStore()
const exportStore = useExportStore()

const dialog = useDialog()

// Active Tab: Mengikuti alur pengadaan standar: PO -> Penerimaan Barang (GRN) -> Faktur Pembelian -> Retur Pembelian -> Pemasok
const activeTab = ref<'orders' | 'receipts' | 'invoices' | 'returns' | 'suppliers'>('orders')

// State for Invoices (Faktur Pembelian)
const invoices = ref<any[]>([])
const totalInvoices = ref<number>(0)
const suppliers = ref<any[]>([])
const allSuppliers = ref<any[]>([])
const totalSuppliers = ref<number>(0)
const isLoadingSuppliers = ref<boolean>(false)
const warehouses = ref<any[]>([])
const products = ref<any[]>([])
const purchaseOrders = ref<any[]>([])
const totalPurchaseOrders = ref<number>(0)
const isLoadingOrders = ref<boolean>(false)
const isLoading = ref<boolean>(false)
const invoiceSearchQuery = ref<string>('')
const invoiceStatusFilter = ref<string>('')
const supplierSearchQuery = ref<string>('')

// State for Purchase Orders (PO)
const poSearchQuery = ref<string>('')
const poStatusFilter = ref<string>('ALL')
const poPage = ref<number>(1)
const poPageSize = ref<number>(10)
const poSortBy = ref<string>('created_at')
const poSortDir = ref<'asc' | 'desc'>('desc')

function togglePoSort(col: string) {
  if (poSortBy.value === col) {
    poSortDir.value = poSortDir.value === 'asc' ? 'desc' : 'asc'
  } else {
    poSortBy.value = col
    poSortDir.value = 'asc'
  }
  poPage.value = 1
  loadPurchaseOrders()
}

// State for Goods Receipts (Penerimaan Barang Fisik dari PO)
const goodsReceipts = ref<any[]>([])
const totalGoodsReceipts = ref<number>(0)
const isLoadingReceipts = ref<boolean>(false)
const receiptSearchQuery = ref<string>('')
const receiptStatusFilter = ref<string>('ALL')
const grnPage = ref<number>(1)
const grnPageSize = ref<number>(10)
const grnSortBy = ref<string>('created_at')
const grnSortDir = ref<'asc' | 'desc'>('desc')
const showGrnDetailModal = ref<boolean>(false)
const selectedGrnDetail = ref<any>(null)

function toggleGrnSort(col: string) {
  if (grnSortBy.value === col) {
    grnSortDir.value = grnSortDir.value === 'asc' ? 'desc' : 'asc'
  } else {
    grnSortBy.value = col
    grnSortDir.value = 'asc'
  }
  grnPage.value = 1
  loadGoodsReceipts()
}

// Modal Buat Faktur Pembelian atas Penerimaan Barang (GRN)
const showInvoiceModal = ref<boolean>(false)
const isSavingInvoice = ref<boolean>(false)
const selectedGrnForInvoice = ref<any>(null)
const invoiceForm = ref({
  goods_receipt_id: '',
  grn_number: '',
  delivery_order_number: '',
  po_number: '',
  supplier_name: '',
  outlet_name: '',
  supplier_invoice_number: '',
  invoice_date: new Date().toISOString().split('T')[0],
  due_date: new Date(Date.now() + 14 * 24 * 60 * 60 * 1000).toISOString().split('T')[0],
  subtotal: 0,
  discount_amount: 0,
  tax_amount: 0,
  payment_status: 'UNPAID', // 'UNPAID' (Kredit/Tempo) atau 'PAID' (Lunas)
  notes: '',
  items: [] as any[],
})

// State for Purchase Returns (Retur Pembelian ke Supplier)
const returns = ref<any[]>([])
const totalReturns = ref<number>(0)
const isLoadingReturns = ref<boolean>(false)
const returnSearchQuery = ref<string>('')
const returnStatusFilter = ref<string>('ALL')
const retPage = ref<number>(1)
const retPageSize = ref<number>(10)
const retSortBy = ref<string>('created_at')
const retSortDir = ref<'asc' | 'desc'>('desc')
const showReturnModal = ref<boolean>(false)
const showReturnDetailModal = ref<boolean>(false)
const selectedReturnDetail = ref<any>(null)
const isLoadingReturnDetail = ref<boolean>(false)
const isSavingReturn = ref<boolean>(false)

function toggleRetSort(col: string) {
  if (retSortBy.value === col) {
    retSortDir.value = retSortDir.value === 'asc' ? 'desc' : 'asc'
  } else {
    retSortBy.value = col
    retSortDir.value = 'asc'
  }
  retPage.value = 1
  loadPurchaseReturns()
}

const returnForm = ref({
  supplier_id: '',
  outlet_id: '',
  invoice_id: '',
  goods_receipt_id: '',
  return_date: new Date().toISOString().split('T')[0],
  refund_method: 'DEBT_REDUCTION', // 'DEBT_REDUCTION' | 'CASH_REFUND' | 'REPLACEMENT'
  reason: '',
  notes: '',
  items: [] as Array<{
    product_id: string
    product_name?: string
    product_sku?: string
    unit_code?: string
    returned_qty: number
    unit_price: number
    subtotal: number
    notes: string
  }>,
})

const returnSubtotal = computed(() => {
  return returnForm.value.items.reduce((s, it) => s + (Number(it.returned_qty) || 0) * (Number(it.unit_price) || 0), 0)
})

// Server-side Pagination & Sorting for Invoices
const invPage = ref<number>(1)
const invPageSize = ref<number>(10)
const sortBy = ref<string>('created_at')
const sortDir = ref<'asc' | 'desc'>('desc')

function toggleSort(col: string) {
  if (sortBy.value === col) {
    sortDir.value = sortDir.value === 'asc' ? 'desc' : 'asc'
  } else {
    sortBy.value = col
    sortDir.value = 'asc'
  }
  invPage.value = 1
  loadInvoices()
}

// Pagination & Sorting for Suppliers
const suppPage = ref<number>(1)
const suppPageSize = ref<number>(10)
const suppSortBy = ref<string>('name')
const suppSortDir = ref<'asc' | 'desc'>('asc')

function toggleSuppSort(col: string) {
  if (suppSortBy.value === col) {
    suppSortDir.value = suppSortDir.value === 'asc' ? 'desc' : 'asc'
  } else {
    suppSortBy.value = col
    suppSortDir.value = 'asc'
  }
  suppPage.value = 1
  loadSuppliers()
}

// Modal Detail Invoice
const showDetailModal = ref<boolean>(false)
const selectedInvoice = ref<any>(null)
const isLoadingDetail = ref<boolean>(false)

// Modal New GRN (Penerimaan Fisik dari PO)
const showGrnModal = ref<boolean>(false)
const isSavingGrn = ref<boolean>(false)
const outlets = ref<any[]>([])
const grnForm = ref({
  po_id: '',
  po_number: '',
  outlet_id: '',
  outlet_name: '',
  supplier_id: '',
  supplier_name: '',
  delivery_order_number: '',
  receive_date: new Date().toISOString().split('T')[0],
  notes: '',
  items: [] as Array<{
    po_item_id?: string
    product_id: string
    product_name?: string
    product_sku?: string
    unit_code?: string
    ordered_qty: number
    previously_received_qty: number
    remaining_qty: number
    received_qty: number
    unit_price: number
    new_barcode?: string
    new_barcode_desc?: string
    expiry_date?: string
    batch_number?: string
  }>,
})

// State Purchase Order (PO)
const showPoModal = ref<boolean>(false)
const isSavingPo = ref<boolean>(false)
const showPoDetailModal = ref<boolean>(false)
const isLoadingPoDetail = ref<boolean>(false)
const selectedPo = ref<any>(null)
const poForm = ref({
  outlet_id: '',
  supplier_id: '',
  order_date: new Date().toISOString().split('T')[0],
  expected_delivery: new Date(Date.now() + 7 * 24 * 60 * 60 * 1000).toISOString().split('T')[0],
  notes: '',
  items: [] as Array<{
    product_id: string
    product_name?: string
    product_sku?: string
    unit_code?: string
    ordered_qty: number
    unit_price: number
    subtotal?: number
  }>,
})

// Modal Supplier CRUD
const showSupplierModal = ref<boolean>(false)
const isEditingSupplier = ref<boolean>(false)
const editingSupplierId = ref<string | null>(null)
const isSavingSupplier = ref<boolean>(false)
const supplierForm = ref({
  code: '',
  name: '',
  contact_person: '',
  phone: '',
  address: '',
  term_of_payment_days: 14,
})

// Toast notification
const toastMessage = ref<string>('')
const toastType = ref<'success' | 'error' | 'info'>('success')

function showToast(msg: string, type: 'success' | 'error' | 'info' = 'success') {
  toastMessage.value = msg
  toastType.value = type
  setTimeout(() => {
    toastMessage.value = ''
  }, 4000)
}

// Helpers
const formatRupiah = (val: any) => formatCurrency(val)

let searchTimer: ReturnType<typeof setTimeout> | null = null
function handleInvoiceSearchInput() {
  if (searchTimer) clearTimeout(searchTimer)
  searchTimer = setTimeout(() => {
    invPage.value = 1
    loadInvoices()
  }, 350)
}

function handleStatusFilterChange() {
  invPage.value = 1
  loadInvoices()
}

let poSearchTimer: ReturnType<typeof setTimeout> | null = null
function handlePoSearchInput() {
  if (poSearchTimer) clearTimeout(poSearchTimer)
  poSearchTimer = setTimeout(() => {
    poPage.value = 1
    loadPurchaseOrders()
  }, 350)
}

function handlePoStatusFilterChange() {
  poPage.value = 1
  loadPurchaseOrders()
}

let receiptSearchTimer: ReturnType<typeof setTimeout> | null = null
function handleReceiptSearchInput() {
  if (receiptSearchTimer) clearTimeout(receiptSearchTimer)
  receiptSearchTimer = setTimeout(() => {
    grnPage.value = 1
    loadGoodsReceipts()
  }, 350)
}

function handleReceiptStatusFilterChange() {
  grnPage.value = 1
  loadGoodsReceipts()
}

let returnSearchTimer: ReturnType<typeof setTimeout> | null = null
function handleReturnSearchInput() {
  if (returnSearchTimer) clearTimeout(returnSearchTimer)
  returnSearchTimer = setTimeout(() => {
    retPage.value = 1
    loadPurchaseReturns()
  }, 350)
}

function handleReturnStatusFilterChange() {
  retPage.value = 1
  loadPurchaseReturns()
}

let suppSearchTimer: ReturnType<typeof setTimeout> | null = null
function handleSupplierSearchInput() {
  if (suppSearchTimer) clearTimeout(suppSearchTimer)
  suppSearchTimer = setTimeout(() => {
    suppPage.value = 1
    loadSuppliers()
  }, 350)
}

const selectedOutletId = ref<string>(locationStore.currentOutletId || '')

function handleLocationChanged() {
  if (locationStore.currentOutletId) {
    selectedOutletId.value = locationStore.currentOutletId
    if (activeTab.value === 'orders') {
      poPage.value = 1
      loadPurchaseOrders()
    } else if (activeTab.value === 'receipts') {
      grnPage.value = 1
      loadGoodsReceipts()
    } else if (activeTab.value === 'invoices') {
      invPage.value = 1
      loadInvoices()
    } else if (activeTab.value === 'returns') {
      retPage.value = 1
      loadPurchaseReturns()
    }
  }
}

watch(
  () => locationStore.currentOutletId,
  (newId) => {
    if (newId && selectedOutletId.value !== newId) {
      selectedOutletId.value = newId
      if (activeTab.value === 'orders') {
        poPage.value = 1
        loadPurchaseOrders()
      } else if (activeTab.value === 'receipts') {
        grnPage.value = 1
        loadGoodsReceipts()
      } else if (activeTab.value === 'invoices') {
        invPage.value = 1
        loadInvoices()
      } else if (activeTab.value === 'returns') {
        retPage.value = 1
        loadPurchaseReturns()
      }
    }
  }
)

async function loadPurchaseOrders() {
  try {
    isLoadingOrders.value = true
    const params: Record<string, any> = {
      page: poPage.value,
      page_size: poPageSize.value,
      sort_by: poSortBy.value,
      sort_dir: poSortDir.value,
    }
    if (poSearchQuery.value.trim()) {
      params.search = poSearchQuery.value.trim()
    }
    if (poStatusFilter.value && poStatusFilter.value !== 'ALL') {
      params.status = poStatusFilter.value
    }
    if (selectedOutletId.value) {
      params.outlet_id = selectedOutletId.value
    }

    const res = await api.get('/inventory/purchase-orders', { params })
    if (res.data?.success) {
      purchaseOrders.value = res.data.data || []
      totalPurchaseOrders.value = res.data.total_items ?? purchaseOrders.value.length
    }
  } catch (err: any) {
    console.error('Gagal memuat data Purchase Orders:', err)
  } finally {
    isLoadingOrders.value = false
  }
}

async function loadInvoices() {
  try {
    isLoading.value = true
    const res = await api.get('/inventory/purchases', {
      params: {
        page: invPage.value,
        page_size: invPageSize.value,
        search: invoiceSearchQuery.value || undefined,
        status: invoiceStatusFilter.value || undefined,
        outlet_id: selectedOutletId.value || undefined,
        sort_by: sortBy.value,
        sort_dir: sortDir.value,
      },
    })
    if (res.data?.success) {
      invoices.value = res.data.data || []
      totalInvoices.value = res.data.total_items ?? (res.data.data?.length || 0)
    }
  } catch (err: any) {
    console.error('Gagal memuat data faktur pengadaan:', err)
  } finally {
    isLoading.value = false
  }
}

async function loadSuppliers() {
  try {
    isLoadingSuppliers.value = true
    const params: Record<string, any> = {
      page: suppPage.value,
      page_size: suppPageSize.value,
      sort_by: suppSortBy.value,
      sort_dir: suppSortDir.value,
    }
    if (supplierSearchQuery.value.trim()) {
      params.search = supplierSearchQuery.value.trim()
    }

    const res = await api.get('/suppliers', { params })
    if (res.data?.success) {
      suppliers.value = res.data.data || []
      totalSuppliers.value = res.data.total_items ?? suppliers.value.length
    }
  } catch (err: any) {
    console.error('Gagal memuat data supplier:', err)
  } finally {
    isLoadingSuppliers.value = false
  }
}

async function loadLookups() {
  try {
    const [supRes, whRes, prodRes, outRes] = await Promise.all([
      api.get('/suppliers', { params: { limit: 1000 } }),
      api.get('/warehouses'),
      api.get('/products'),
      api.get('/outlets'),
    ])

    if (supRes.data?.success) {
      allSuppliers.value = supRes.data.data || []
    }
    if (whRes.data?.success) warehouses.value = whRes.data.data || []
    if (prodRes.data?.success) products.value = prodRes.data.data || []
    if (outRes.data?.success) outlets.value = outRes.data.data || []
  } catch (err: any) {
    console.error('Gagal memuat master data pembantu:', err)
  }
}

async function loadData() {
  await Promise.all([
    loadPurchaseOrders(),
    loadGoodsReceipts(),
    loadInvoices(),
    loadPurchaseReturns(),
    loadSuppliers(),
    loadLookups(),
  ])
}

watch(activeTab, (tab) => {
  if (tab === 'orders') {
    loadPurchaseOrders()
  } else if (tab === 'receipts') {
    loadGoodsReceipts()
  } else if (tab === 'invoices') {
    loadInvoices()
  } else if (tab === 'returns') {
    loadPurchaseReturns()
  } else if (tab === 'suppliers') {
    loadSuppliers()
  }
})

onMounted(() => {
  authStore.fetchCurrentUser()
  if (locationStore.currentOutletId && !selectedOutletId.value) {
    selectedOutletId.value = locationStore.currentOutletId
  }
  loadData()
  window.addEventListener('notane-location-changed', handleLocationChanged)
})

onUnmounted(() => {
  window.removeEventListener('notane-location-changed', handleLocationChanged)
})

async function loadGoodsReceipts() {
  try {
    isLoadingReceipts.value = true
    const params: Record<string, any> = {
      page: grnPage.value,
      page_size: grnPageSize.value,
      sort_by: grnSortBy.value,
      sort_dir: grnSortDir.value,
    }
    if (receiptSearchQuery.value.trim()) {
      params.search = receiptSearchQuery.value.trim()
    }
    if (receiptStatusFilter.value && receiptStatusFilter.value !== 'ALL') {
      params.status = receiptStatusFilter.value
    }
    if (selectedOutletId.value) {
      params.outlet_id = selectedOutletId.value
    }

    const res = await api.get('/inventory/goods-receipts', { params })
    if (res.data?.success) {
      goodsReceipts.value = res.data.data || []
      totalGoodsReceipts.value = res.data.total_items ?? goodsReceipts.value.length
    }
  } catch (err: any) {
    console.error('Gagal memuat data penerimaan barang:', err)
  } finally {
    isLoadingReceipts.value = false
  }
}

function openCreateGrnModal(poData?: any) {
  if (poData) {
    openGrnFromPo(poData)
    return
  }
  // Ambil PO pertama yang berstatus PENDING atau PARTIAL
  const availablePo = purchaseOrders.value.find((p) => p.status === 'PENDING' || p.status === 'PARTIAL')
  if (!availablePo) {
    showToast('Belum ada Purchase Order (PO) yang aktif untuk diproses penerimaannya. Harap buat PO terlebih dahulu.', 'info')
    activeTab.value = 'orders'
    return
  }
  openGrnFromPo(availablePo)
}

// Handler saat PO dipilih di dalam modal GRN
async function handlePoSelectInGrn(selectedPoId: string) {
  if (!selectedPoId) {
    grnForm.value.po_id = ''
    grnForm.value.items = []
    return
  }
  try {
    const res = await api.get(`/inventory/purchase-orders/${selectedPoId}`)
    if (res.data?.success) {
      const po = res.data.data
      grnForm.value.po_id = po.id
      grnForm.value.po_number = po.po_number
      grnForm.value.supplier_id = po.supplier_id
      grnForm.value.supplier_name = po.supplier_name
      grnForm.value.outlet_id = po.outlet_id
      grnForm.value.outlet_name = po.outlet_name
      const safePoNumber = (po.po_number || '').trim()
      const cleanPoNumber = safePoNumber.replace(/^PO-?/i, '') || Date.now().toString().slice(-4)
      grnForm.value.delivery_order_number = `SJ-${cleanPoNumber}-${Date.now().toString().slice(-4)}`
      grnForm.value.notes = `Penerimaan fisik barang atas PO ${safePoNumber}`
      
      const unfulfilledItems = (po.items || []).filter((it: any) => {
        const ordered = Number(it.ordered_qty) || 0
        const alreadyReceived = Number(it.received_qty) || 0
        return ordered - alreadyReceived > 0
      })

      if (unfulfilledItems.length === 0) {
        showToast('Seluruh item pada PO ini sudah diterima lengkap.', 'info')
      }

      const poItems = unfulfilledItems.map((it: any) => {
        const ordered = Number(it.ordered_qty) || 0
        const alreadyReceived = Number(it.received_qty) || 0
        const remaining = Math.max(0, ordered - alreadyReceived)
        return {
          po_item_id: it.id,
          product_id: it.product_id,
          product_name: it.product_name,
          product_sku: it.product_sku,
          unit_code: it.unit_code,
          ordered_qty: ordered,
          previously_received_qty: alreadyReceived,
          remaining_qty: remaining,
          received_qty: remaining,
          unit_price: Number(it.unit_price) || 0,
          new_barcode: '',
          new_barcode_desc: '',
        }
      })
      grnForm.value.items = poItems
    }
  } catch (err: any) {
    showToast('Gagal mengambil rincian PO rujukan', 'error')
  }
}

// Buka modal GRN langsung dari baris PO
async function openGrnFromPo(poInput: any) {
  let po = poInput
  try {
    const res = await api.get(`/inventory/purchase-orders/${poInput.id}`)
    if (res.data?.success) {
      po = res.data.data
    }
  } catch (e) {
    console.error(e)
  }

  const unfulfilledItems = (po.items || []).filter((it: any) => {
    const ordered = Number(it.ordered_qty) || 0
    const alreadyReceived = Number(it.received_qty) || 0
    return ordered - alreadyReceived > 0
  })

  if (unfulfilledItems.length === 0) {
    showToast('Seluruh item pada PO ini sudah diterima lengkap (100%).', 'info')
    return
  }

  const itemsToReceive = unfulfilledItems.map((it: any) => {
    const ordered = Number(it.ordered_qty) || 0
    const alreadyReceived = Number(it.received_qty) || 0
    const remaining = Math.max(0, ordered - alreadyReceived)
    return {
      po_item_id: it.id,
      product_id: it.product_id,
      product_name: it.product_name,
      product_sku: it.product_sku,
      unit_code: it.unit_code,
      ordered_qty: ordered,
      previously_received_qty: alreadyReceived,
      remaining_qty: remaining,
      received_qty: remaining,
      unit_price: Number(it.unit_price) || 0,
      new_barcode: '',
      new_barcode_desc: '',
    }
  })

  const safePoNumber = (po.po_number || '').trim()
  const cleanPoNumber = safePoNumber.replace(/^PO-?/i, '') || Date.now().toString().slice(-4)

  grnForm.value = {
    po_id: po.id,
    po_number: safePoNumber,
    outlet_id: po.outlet_id,
    outlet_name: po.outlet_name,
    supplier_id: po.supplier_id,
    supplier_name: po.supplier_name,
    delivery_order_number: `SJ-${cleanPoNumber}-${Date.now().toString().slice(-4)}`,
    receive_date: new Date().toISOString().split('T')[0],
    notes: `Penerimaan fisik barang atas PO ${safePoNumber}`,
    items: itemsToReceive,
  }

  if (showPoDetailModal.value) {
    showPoDetailModal.value = false
  }
  showGrnModal.value = true
}

const totalGrnQtyReceived = computed(() => {
  return (grnForm.value.items || []).reduce((sum: number, it: any) => sum + (Number(it.received_qty) || 0), 0)
})

const totalGrnValue = computed(() => {
  return (grnForm.value.items || []).reduce((sum: number, it: any) => sum + ((Number(it.received_qty) || 0) * (Number(it.unit_price) || 0)), 0)
})

async function handleSaveGrn() {
  if (!authStore.hasPermission(['purchasing:create', 'purchasing:update', 'purchasing:write'])) {
    showToast('Akses ditolak: Anda tidak memiliki wewenang memproses penerimaan barang (purchasing:create / purchasing:update)', 'error')
    return
  }
  if (!grnForm.value.po_id) {
    showToast('Pilih dokumen Purchase Order (PO) rujukan terlebih dahulu', 'error')
    return
  }
  if (!grnForm.value.delivery_order_number.trim()) {
    showToast('Nomor Surat Jalan / Pengiriman Pemasok wajib diisi', 'error')
    return
  }

  // Filter hanya item yang memiliki received_qty > 0
  const validItems = grnForm.value.items.filter((it: any) => Number(it.received_qty) > 0)

  if (validItems.length === 0) {
    showToast('Harap masukkan jumlah fisik barang yang diterima (Qty Masuk > 0)', 'error')
    return
  }

  for (const it of validItems) {
    if (Number(it.received_qty) > Number(it.remaining_qty)) {
      showToast(`Qty diterima untuk ${it.product_name} (${it.received_qty}) melebihi sisa pesanan (${it.remaining_qty})`, 'error')
      return
    }
  }

  try {
    isSavingGrn.value = true
    const payload = {
      po_id: grnForm.value.po_id,
      delivery_order_number: grnForm.value.delivery_order_number.trim(),
      receive_date: grnForm.value.receive_date,
      notes: grnForm.value.notes || undefined,
      items: validItems.map((it) => ({
        po_item_id: it.po_item_id,
        product_id: it.product_id,
        received_qty: it.received_qty.toString(),
        expiry_date: it.expiry_date || undefined,
        batch_number: it.batch_number ? it.batch_number.trim() : undefined,
        new_barcode: it.new_barcode ? it.new_barcode.trim() : undefined,
        new_barcode_desc: it.new_barcode_desc ? it.new_barcode_desc.trim() : undefined,
      })),
    }

    const res = await api.post('/inventory/goods-receipts', payload)
    if (res.data?.success) {
      showToast(res.data.message || 'Penerimaan barang (GRN) berhasil dicatat dan stok telah bertambah!', 'success')
      showGrnModal.value = false
      await loadData()
      activeTab.value = 'receipts'
    } else {
      showToast(res.data?.message || 'Gagal menyimpan penerimaan barang', 'error')
    }
  } catch (err: any) {
    showToast(err.response?.data?.message || err.message || 'Gagal menyimpan penerimaan barang', 'error')
  } finally {
    isSavingGrn.value = false
  }
}

// Modal Detail GRN
async function openGrnDetailModal(grn: any) {
  try {
    const res = await api.get(`/inventory/goods-receipts/${grn.id}`)
    if (res.data?.success) {
      selectedGrnDetail.value = res.data.data
      showGrnDetailModal.value = true
    }
  } catch (e: any) {
    showToast('Gagal memuat rincian penerimaan barang', 'error')
  }
}

// Pembuatan Faktur Pembelian (Purchase Invoice) dari GRN
const invoiceTotalAmount = computed(() => {
  const sub = Number(invoiceForm.value.subtotal) || 0
  const disc = Number(invoiceForm.value.discount_amount) || 0
  const tax = Number(invoiceForm.value.tax_amount) || 0
  return Math.max(0, sub - disc + tax)
})

function openCreateInvoiceModal(grnData?: any) {
  if (grnData) {
    setupInvoiceFromGrn(grnData)
    return
  }
  // Ambil GRN pertama yang berstatus UNINVOICED
  const availableGrn = goodsReceipts.value.find((g) => g.status === 'UNINVOICED')
  if (!availableGrn) {
    showToast('Belum ada Penerimaan Barang (GRN) yang belum difakturkan. Catat penerimaan barang terlebih dahulu.', 'info')
    activeTab.value = 'receipts'
    return
  }
  setupInvoiceFromGrn(availableGrn)
}

async function setupInvoiceFromGrn(grnInput: any) {
  let grn = grnInput
  try {
    const res = await api.get(`/inventory/goods-receipts/${grnInput.id}`)
    if (res.data?.success) {
      grn = res.data.data
    }
  } catch (e) {
    console.error(e)
  }

  selectedGrnForInvoice.value = grn

  const items = (grn.items || []).map((it: any) => ({
    id: it.id,
    product_id: it.product_id,
    product_name: it.product_name,
    product_sku: it.product_sku,
    unit_code: it.unit_code,
    received_qty: Number(it.received_qty) || 0,
    unit_price: Number(it.unit_price) || 0,
    subtotal: (Number(it.received_qty) || 0) * (Number(it.unit_price) || 0),
  }))

  const rawSubtotal = items.reduce((s: number, i: any) => s + i.subtotal, 0)

  invoiceForm.value = {
    goods_receipt_id: grn.id,
    grn_number: grn.grn_number,
    delivery_order_number: grn.delivery_order_number,
    po_number: grn.po_number,
    supplier_name: grn.supplier_name,
    outlet_name: grn.outlet_name,
    supplier_invoice_number: `INV-${Date.now().toString().slice(-6)}`,
    invoice_date: new Date().toISOString().split('T')[0],
    due_date: new Date(Date.now() + 14 * 24 * 60 * 60 * 1000).toISOString().split('T')[0],
    subtotal: rawSubtotal,
    discount_amount: 0,
    tax_amount: 0,
    payment_status: 'UNPAID',
    notes: `Faktur pembelian atas Surat Jalan ${grn.delivery_order_number} (PO ${grn.po_number})`,
    items: items,
  }

  showInvoiceModal.value = true
}

async function handleSaveInvoice() {
  if (!authStore.hasPermission(['purchasing:create', 'purchasing:update', 'purchasing:write'])) {
    showToast('Akses ditolak: Anda tidak memiliki wewenang menerbitkan faktur pembelian (purchasing:create / purchasing:update)', 'error')
    return
  }
  if (!invoiceForm.value.goods_receipt_id) {
    showToast('Pilih dokumen Penerimaan Barang (GRN) terlebih dahulu', 'error')
    return
  }
  if (!invoiceForm.value.supplier_invoice_number.trim()) {
    showToast('Nomor Faktur / Invoice Pemasok wajib diisi', 'error')
    return
  }

  try {
    isSavingInvoice.value = true
    const payload = {
      goods_receipt_id: invoiceForm.value.goods_receipt_id,
      supplier_invoice_number: invoiceForm.value.supplier_invoice_number.trim(),
      invoice_date: invoiceForm.value.invoice_date,
      due_date: invoiceForm.value.due_date,
      payment_status: invoiceForm.value.payment_status,
      discount_amount: Number(invoiceForm.value.discount_amount) || 0,
      tax_amount: Number(invoiceForm.value.tax_amount) || 0,
      notes: invoiceForm.value.notes || undefined,
    }

    const res = await api.post('/inventory/purchase-invoices', payload)
    if (res.data?.success) {
      showToast(res.data.message || 'Faktur pembelian berhasil dicatat!', 'success')
      showInvoiceModal.value = false
      await loadData()
      activeTab.value = 'invoices'
    } else {
      showToast(res.data?.message || 'Gagal menyimpan faktur pembelian', 'error')
    }
  } catch (err: any) {
    showToast(err.response?.data?.message || err.message || 'Gagal menyimpan faktur pembelian', 'error')
  } finally {
    isSavingInvoice.value = false
  }
}

// Purchase Order (PO) Methods
const poSubtotal = computed(() => {
  return poForm.value.items.reduce((sum, it) => {
    const sub = (Number(it.ordered_qty) || 0) * (Number(it.unit_price) || 0)
    return sum + Math.max(0, sub)
  }, 0)
})

function addPoItem() {
  poForm.value.items.push({
    product_id: '',
    ordered_qty: 1,
    unit_price: 0,
  })
}

function removePoItem(idx: number) {
  poForm.value.items.splice(idx, 1)
}

function onPoProductSelected(idx: number, productId: string, productObj?: any) {
  const itemRow = poForm.value.items[idx]
  if (!itemRow) return
  itemRow.product_id = productId
  const p = productObj || products.value.find((prod) => prod.id === productId)
  if (p) {
    itemRow.unit_price = Number(p.cost_price) || 0
    ;(itemRow as any)._product = p
  }
}

function openCreatePoModal() {
  poForm.value = {
    outlet_id: locationStore.currentOutletId || outlets.value[0]?.id || '',
    supplier_id: suppliers.value[0]?.id || '',
    order_date: new Date().toISOString().split('T')[0],
    expected_delivery: new Date(Date.now() + 7 * 24 * 60 * 60 * 1000).toISOString().split('T')[0],
    notes: 'Pesanan Pembelian Rutin',
    items: [],
  }
  showPoModal.value = true
}

const isLoadingReorder = ref<boolean>(false)

async function pullReorderSuggestions() {
  isLoadingReorder.value = true
  try {
    const res = await api.get('/inventory/reorder-suggestions', {
      params: {
        supplier_id: poForm.value.supplier_id || undefined,
        outlet_id: poForm.value.outlet_id || undefined,
      },
    })
    if (res.data?.success && Array.isArray(res.data.data)) {
      const suggestions = res.data.data
      if (suggestions.length === 0) {
        showToast('Tidak ada produk dengan stok menipis (di bawah batas minimum) untuk supplier ini', 'info')
        return
      }

      let addedCount = 0
      for (const sg of suggestions) {
        const exists = poForm.value.items.some((i: any) => i.product_id === sg.product_id)
        if (!exists) {
          poForm.value.items.push({
            product_id: sg.product_id,
            product_name: sg.product_name,
            product_sku: sg.product_sku,
            unit_code: sg.unit_code || 'PCS',
            ordered_qty: Number(sg.suggested_qty) || 1,
            unit_price: Number(sg.purchase_price) || 0,
            subtotal: (Number(sg.suggested_qty) || 1) * (Number(sg.purchase_price) || 0),
          })
          addedCount++
        }
      }

      showToast(`Berhasil menambahkan ${addedCount} produk stok menipis ke daftar PO!`, 'success')
    }
  } catch (err: any) {
    showToast('Gagal menarik rekomendasi reorder: ' + (err.response?.data?.message || err.message), 'error')
  } finally {
    isLoadingReorder.value = false
  }
}

async function handleSavePo() {
  if (!authStore.hasPermission(['purchasing:create', 'purchasing:write'])) {
    showToast('Akses ditolak: Anda tidak memiliki wewenang membuat Purchase Order (purchasing:create)', 'error')
    return
  }
  if (!poForm.value.outlet_id) {
    showToast('Pilih outlet pemesan terlebih dahulu', 'error')
    return
  }
  if (!poForm.value.supplier_id) {
    showToast('Pilih pemasok / supplier terlebih dahulu', 'error')
    return
  }
  if (poForm.value.items.length === 0) {
    showToast('Tambahkan minimal 1 barang dalam pesanan pembelian (PO)', 'error')
    return
  }
  for (const it of poForm.value.items) {
    if (!it.product_id) {
      showToast('Harap pilih produk untuk seluruh baris barang PO', 'error')
      return
    }
    if (Number(it.ordered_qty) <= 0) {
      showToast('Jumlah qty pesanan PO harus lebih besar dari 0', 'error')
      return
    }
  }

  try {
    isSavingPo.value = true
    const payload = {
      outlet_id: poForm.value.outlet_id,
      supplier_id: poForm.value.supplier_id,
      order_date: poForm.value.order_date,
      expected_delivery: poForm.value.expected_delivery || undefined,
      notes: poForm.value.notes || undefined,
      items: poForm.value.items.map((it) => ({
        product_id: it.product_id,
        ordered_qty: it.ordered_qty.toString(),
        unit_price: it.unit_price.toString(),
      })),
    }

    const res = await api.post('/inventory/purchase-orders', payload)
    if (res.data?.success) {
      showToast(res.data.message || 'Purchase Order (PO) berhasil dibuat!', 'success')
      showPoModal.value = false
      await loadData()
    } else {
      showToast(res.data?.message || 'Gagal membuat Purchase Order', 'error')
    }
  } catch (err: any) {
    showToast(err.response?.data?.message || err.message || 'Gagal menyimpan Purchase Order', 'error')
  } finally {
    isSavingPo.value = false
  }
}

async function viewPoDetail(poId: string) {
  try {
    isLoadingPoDetail.value = true
    showPoDetailModal.value = true
    const res = await api.get(`/inventory/purchase-orders/${poId}`)
    if (res.data?.success) {
      selectedPo.value = res.data.data
    } else {
      showToast(res.data?.message || 'Gagal memuat detail PO', 'error')
      showPoDetailModal.value = false
    }
  } catch (err: any) {
    showToast(err.response?.data?.message || 'Gagal memuat detail PO', 'error')
    showPoDetailModal.value = false
  } finally {
    isLoadingPoDetail.value = false
  }
}

async function handleCancelPo(po: any) {
  if (!authStore.hasPermission(['purchasing:void', 'purchasing:write'])) {
    showToast('Akses ditolak: Anda tidak memiliki wewenang membatalkan Purchase Order (purchasing:void)', 'error')
    return
  }
  const confirmed = await dialog.confirm({
    title: 'Batalkan Purchase Order',
    message: `Batalkan pesanan ${po.po_number}?`,
    subMessage: 'PO yang dibatalkan tidak dapat diproses lagi penerimaan barangnya.',
    confirmText: 'Ya, Batalkan PO',
    cancelText: 'Kembali',
    type: 'danger',
  })
  if (!confirmed) return

  try {
    const res = await api.post(`/inventory/purchase-orders/${po.id}/cancel`)
    if (res.data?.success) {
      showToast(res.data.message || 'Purchase Order berhasil dibatalkan', 'success')
      if (showPoDetailModal.value && selectedPo.value?.id === po.id) {
        selectedPo.value.status = 'CANCELLED'
      }
      await loadData()
    } else {
      showToast(res.data?.message || 'Gagal membatalkan PO', 'error')
    }
  } catch (err: any) {
    showToast(err.response?.data?.message || err.message || 'Gagal membatalkan PO', 'error')
  }
}

async function handleCancelGrn(grn: any) {
  if (!authStore.hasPermission(['purchasing:void', 'purchasing:write'])) {
    showToast('Akses ditolak: Anda tidak memiliki wewenang membatalkan penerimaan barang (purchasing:void)', 'error')
    return
  }
  const confirmed = await dialog.confirm({
    title: 'Batalkan Penerimaan Barang (GRN)',
    message: `Batalkan penerimaan ${grn.grn_number}?`,
    subMessage: 'Stok fisik di gudang akan dikurangi kembali, status PO akan disesuaikan, dan jurnal persediaan belum tertagih akan dibalikkan.',
    confirmText: 'Ya, Batalkan GRN',
    cancelText: 'Kembali',
    type: 'danger',
  })
  if (!confirmed) return

  try {
    const res = await api.post(`/inventory/goods-receipts/${grn.id}/cancel`)
    if (res.data?.success) {
      showToast(res.data.message || 'Penerimaan barang berhasil dibatalkan', 'success')
      if (showGrnDetailModal.value && selectedGrnDetail.value?.id === grn.id) {
        showGrnDetailModal.value = false
      }
      await loadData()
    } else {
      showToast(res.data?.message || 'Gagal membatalkan penerimaan barang', 'error')
    }
  } catch (err: any) {
    showToast(err.response?.data?.message || err.message || 'Gagal membatalkan penerimaan barang', 'error')
  }
}

async function handleCancelInvoice(inv: any) {
  if (!authStore.hasPermission(['purchasing:void', 'purchasing:write'])) {
    showToast('Akses ditolak: Anda tidak memiliki wewenang membatalkan faktur pembelian (purchasing:void)', 'error')
    return
  }
  const confirmed = await dialog.confirm({
    title: 'Batalkan Faktur Pembelian',
    message: `Batalkan faktur ${inv.invoice_number}?`,
    subMessage: 'Faktur yang dibatalkan akan memulihkan status penerimaan barang (GRN) ke Belum Difakturkan dan membalikkan pencatatan hutang usaha.',
    confirmText: 'Ya, Batalkan Faktur',
    cancelText: 'Kembali',
    type: 'danger',
  })
  if (!confirmed) return

  try {
    const res = await api.post(`/inventory/purchase-invoices/${inv.id}/cancel`)
    if (res.data?.success) {
      showToast(res.data.message || 'Faktur pembelian berhasil dibatalkan', 'success')
      if (showDetailModal.value && selectedInvoice.value?.id === inv.id) {
        showDetailModal.value = false
      }
      await loadData()
    } else {
      showToast(res.data?.message || 'Gagal membatalkan faktur', 'error')
    }
  } catch (err: any) {
    showToast(err.response?.data?.message || err.message || 'Gagal membatalkan faktur', 'error')
  }
}

// Purchase Return Methods

async function loadPurchaseReturns() {
  try {
    isLoadingReturns.value = true
    const params: Record<string, any> = {
      page: retPage.value,
      page_size: retPageSize.value,
      sort_by: retSortBy.value,
      sort_dir: retSortDir.value,
    }
    if (returnSearchQuery.value.trim()) {
      params.search = returnSearchQuery.value.trim()
    }
    if (returnStatusFilter.value && returnStatusFilter.value !== 'ALL') {
      params.status = returnStatusFilter.value
    }
    if (selectedOutletId.value) {
      params.outlet_id = selectedOutletId.value
    }

    const res = await api.get('/inventory/purchase-returns', { params })
    if (res.data?.success) {
      returns.value = res.data.data || []
      totalReturns.value = res.data.total_items ?? (res.data.data?.length || 0)
    }
  } catch (err: any) {
    console.error('Gagal memuat data retur pembelian:', err)
  } finally {
    isLoadingReturns.value = false
  }
}

function exportPurchasesToExcel() {
  if (activeTab.value === 'orders') {
    const params: Record<string, any> = {
      sort_by: poSortBy.value,
      sort_dir: poSortDir.value,
    }
    if (poSearchQuery.value.trim()) params.search = poSearchQuery.value.trim()
    if (poStatusFilter.value && poStatusFilter.value !== 'ALL') params.status = poStatusFilter.value
    if (selectedOutletId.value) params.outlet_id = selectedOutletId.value

    exportStore.queueServerExport({
      title: 'Pesanan Pembelian (PO)',
      module: 'purchases_orders',
      filterParams: params,
      outletId: selectedOutletId.value || undefined,
    })
  } else if (activeTab.value === 'receipts') {
    const params: Record<string, any> = {
      sort_by: grnSortBy.value,
      sort_dir: grnSortDir.value,
    }
    if (receiptSearchQuery.value.trim()) params.search = receiptSearchQuery.value.trim()
    if (receiptStatusFilter.value && receiptStatusFilter.value !== 'ALL') params.status = receiptStatusFilter.value
    if (selectedOutletId.value) params.outlet_id = selectedOutletId.value

    exportStore.queueServerExport({
      title: 'Penerimaan Barang (GRN)',
      module: 'purchases_receipts',
      filterParams: params,
      outletId: selectedOutletId.value || undefined,
    })
  } else if (activeTab.value === 'invoices') {
    const params: Record<string, any> = {
      sort_by: sortBy.value,
      sort_dir: sortDir.value,
    }
    if (invoiceSearchQuery.value.trim()) params.search = invoiceSearchQuery.value.trim()
    if (invoiceStatusFilter.value) params.status = invoiceStatusFilter.value
    if (selectedOutletId.value) params.outlet_id = selectedOutletId.value

    exportStore.queueServerExport({
      title: 'Faktur Tagihan Pembelian',
      module: 'purchases_invoices',
      filterParams: params,
      outletId: selectedOutletId.value || undefined,
    })
  } else if (activeTab.value === 'returns') {
    const params: Record<string, any> = {
      sort_by: retSortBy.value,
      sort_dir: retSortDir.value,
    }
    if (returnSearchQuery.value.trim()) params.search = returnSearchQuery.value.trim()
    if (returnStatusFilter.value && returnStatusFilter.value !== 'ALL') params.status = returnStatusFilter.value
    if (selectedOutletId.value) params.outlet_id = selectedOutletId.value

    exportStore.queueServerExport({
      title: 'Retur Pembelian Supplier',
      module: 'purchases_returns',
      filterParams: params,
      outletId: selectedOutletId.value || undefined,
    })
  } else {
    exportStore.queueExport({
      title: 'Daftar Pemasok (Suppliers)',
      module: 'purchases',
      filenamePrefix: 'Daftar_Pemasok_Supplier',
      fetchData: async () => {
        const res = await api.get('/suppliers', { params: { limit: 1000 } })
        return res.data?.data || []
      },
      transformToRows: (data) => {
        const rows = data.map((item, idx) => ({
          No: idx + 1,
          'Kode Supplier': item.code || '-',
          'Nama Supplier': item.name,
          'Kontak Person': item.contact_person || '-',
          'No. Telepon / HP': item.phone || '-',
          Email: item.email || '-',
          Alamat: item.address || '-',
          'Termin Pembayaran (Hari)': Number(item.payment_terms_days || 0),
          Status: item.is_active ? 'AKTIF' : 'NONAKTIF',
        }))
        return [{ sheetName: 'Suppliers', rows }]
      },
    })
  }
}

async function openCreateReturnModal(prefill?: { invoice?: any; grn?: any }) {
  if (prefill?.invoice) {
    const inv = prefill.invoice
    let invDetail = inv
    try {
      const res = await api.get(`/inventory/purchases/${inv.id}`)
      if (res.data?.success) {
        invDetail = res.data.data
      }
    } catch (e) {
      console.error(e)
    }

    returnForm.value = {
      supplier_id: invDetail.supplier_id || '',
      outlet_id: invDetail.outlet_id || selectedOutletId.value || '',
      invoice_id: invDetail.id || '',
      goods_receipt_id: invDetail.goods_receipt_id || '',
      return_date: new Date().toISOString().split('T')[0],
      refund_method: invDetail.payment_status === 'UNPAID' ? 'DEBT_REDUCTION' : 'CASH_REFUND',
      reason: 'Barang Rusak / Cacat',
      notes: `Retur atas Faktur ${invDetail.invoice_number}`,
      items: (invDetail.items || []).map((it: any) => ({
        product_id: it.product_id,
        product_name: it.product_name,
        product_sku: it.product_sku,
        unit_code: it.unit_code,
        returned_qty: 1,
        unit_price: Number(it.unit_price) || 0,
        subtotal: Number(it.unit_price) || 0,
        notes: '',
      })),
    }
  } else {
    returnForm.value = {
      supplier_id: suppliers.value[0]?.id || '',
      outlet_id: selectedOutletId.value || outlets.value[0]?.id || '',
      invoice_id: '',
      goods_receipt_id: '',
      return_date: new Date().toISOString().split('T')[0],
      refund_method: 'DEBT_REDUCTION',
      reason: '',
      notes: '',
      items: [],
    }
    if (products.value.length > 0) {
      const p = products.value[0]
      returnForm.value.items.push({
        product_id: p.id,
        product_name: p.name,
        product_sku: p.sku,
        unit_code: p.unit_code || 'PCS',
        returned_qty: 1,
        unit_price: Number(p.cost_price || p.purchase_price || 0),
        subtotal: Number(p.cost_price || p.purchase_price || 0),
        notes: '',
      })
    }
  }

  showReturnModal.value = true
}

function addReturnItem() {
  const p = products.value[0]
  if (!p) return
  returnForm.value.items.push({
    product_id: p.id,
    product_name: p.name,
    product_sku: p.sku,
    unit_code: p.unit_code || 'PCS',
    returned_qty: 1,
    unit_price: Number(p.cost_price || p.purchase_price || 0),
    subtotal: Number(p.cost_price || p.purchase_price || 0),
    notes: '',
  })
}

function handleReturnProductChange(index: number, productId: string) {
  const p = products.value.find((pr) => pr.id === productId)
  if (p && returnForm.value.items[index]) {
    returnForm.value.items[index].product_id = p.id
    returnForm.value.items[index].product_name = p.name
    returnForm.value.items[index].product_sku = p.sku
    returnForm.value.items[index].unit_code = p.unit_code || 'PCS'
    returnForm.value.items[index].unit_price = Number(p.cost_price || p.purchase_price || 0)
    returnForm.value.items[index].subtotal = (Number(returnForm.value.items[index].returned_qty) || 0) * (Number(p.cost_price || p.purchase_price || 0))
  }
}

function updateReturnItemSubtotal(index: number) {
  const it = returnForm.value.items[index]
  if (it) {
    it.subtotal = (Number(it.returned_qty) || 0) * (Number(it.unit_price) || 0)
  }
}

function removeReturnItem(index: number) {
  returnForm.value.items.splice(index, 1)
}

async function handleSaveReturn() {
  if (!authStore.hasPermission(['purchasing:void', 'purchasing:write'])) {
    showToast('Akses ditolak: Anda tidak memiliki wewenang memproses retur pembelian (purchasing:void)', 'error')
    return
  }
  if (!returnForm.value.supplier_id) {
    showToast('Pilih pemasok / supplier terlebih dahulu', 'error')
    return
  }
  if (!returnForm.value.reason || !returnForm.value.reason.trim()) {
    showToast('Alasan retur pembelian wajib diisi', 'error')
    return
  }
  if (returnForm.value.items.length === 0) {
    showToast('Tambahkan minimal 1 item barang yang diretur', 'error')
    return
  }
  for (const it of returnForm.value.items) {
    if (!it.product_id) {
      showToast('Pilih produk yang diretur secara valid', 'error')
      return
    }
    if ((Number(it.returned_qty) || 0) <= 0) {
      showToast('Jumlah retur barang harus lebih besar dari 0', 'error')
      return
    }
  }

  try {
    isSavingReturn.value = true
    const payload = {
      supplier_id: returnForm.value.supplier_id,
      outlet_id: returnForm.value.outlet_id || null,
      invoice_id: returnForm.value.invoice_id || null,
      goods_receipt_id: returnForm.value.goods_receipt_id || null,
      return_date: returnForm.value.return_date,
      refund_method: returnForm.value.refund_method,
      reason: returnForm.value.reason,
      notes: returnForm.value.notes || null,
      items: returnForm.value.items.map((it) => ({
        product_id: it.product_id,
        returned_qty: Number(it.returned_qty),
        unit_price: Number(it.unit_price),
        subtotal: Number(it.returned_qty) * Number(it.unit_price),
        notes: it.notes || null,
      })),
    }

    const res = await api.post('/inventory/purchase-returns', payload)
    if (res.data?.success) {
      showToast(res.data.message || 'Retur pembelian berhasil dicatat dan dijurnal', 'success')
      showReturnModal.value = false
      activeTab.value = 'returns'
      await Promise.all([loadPurchaseReturns(), loadInvoices()])
    } else {
      showToast(res.data?.message || 'Gagal menyimpan retur pembelian', 'error')
    }
  } catch (err: any) {
    showToast(err.response?.data?.message || err.message || 'Gagal menyimpan retur pembelian', 'error')
  } finally {
    isSavingReturn.value = false
  }
}

async function viewReturnDetail(returnId: string) {
  try {
    isLoadingReturnDetail.value = true
    showReturnDetailModal.value = true
    const res = await api.get(`/inventory/purchase-returns/${returnId}`)
    if (res.data?.success) {
      selectedReturnDetail.value = res.data.data
    } else {
      showToast(res.data?.message || 'Gagal memuat detail retur', 'error')
      showReturnDetailModal.value = false
    }
  } catch (err: any) {
    showToast(err.response?.data?.message || 'Gagal memuat detail retur', 'error')
    showReturnDetailModal.value = false
  } finally {
    isLoadingReturnDetail.value = false
  }
}

async function viewInvoiceDetail(invoiceId: string) {
  try {
    isLoadingDetail.value = true
    showDetailModal.value = true
    const res = await api.get(`/inventory/purchases/${invoiceId}`)
    if (res.data?.success) {
      selectedInvoice.value = res.data.data
    } else {
      showToast(res.data?.message || 'Gagal memuat detail faktur', 'error')
      showDetailModal.value = false
    }
  } catch (err: any) {
    showToast(err.response?.data?.message || 'Gagal memuat detail', 'error')
    showDetailModal.value = false
  } finally {
    isLoadingDetail.value = false
  }
}

async function handlePayInvoice(invoice: any) {
  if (!authStore.hasPermission(['purchasing:pay', 'purchasing:write'])) {
    showToast('Akses ditolak: Anda tidak memiliki wewenang melakukan pelunasan tagihan (purchasing:pay)', 'error')
    return
  }
  const confirmed = await dialog.confirm({
    title: 'Pelunasan Faktur Pembelian',
    message: `Konfirmasi pembayaran lunas untuk faktur ${invoice.invoice_number}?`,
    subMessage: `Total pembayaran senilai ${formatRupiah(invoice.total_amount)} akan dicatat ke jurnal pengeluaran kas.`,
    confirmText: 'Ya, Bayar Lunas',
    cancelText: 'Batal',
    type: 'success',
  })
  if (!confirmed) return

  try {
    const res = await api.post(`/inventory/purchases/${invoice.id}/pay`)
    if (res.data?.success) {
      showToast(res.data.message || 'Faktur pembelian berhasil dilunasi!', 'success')
      if (showDetailModal.value && selectedInvoice.value?.id === invoice.id) {
        selectedInvoice.value.payment_status = 'PAID'
        selectedInvoice.value.paid_amount = selectedInvoice.value.total_amount
      }
      loadData()
    } else {
      showToast(res.data?.message || 'Gagal melunasi tagihan', 'error')
    }
  } catch (err: any) {
    showToast(err.response?.data?.message || err.message || 'Gagal memperbarui status bayar', 'error')
  }
}

// Supplier CRUD Actions
function openAddSupplierModal() {
  isEditingSupplier.value = false
  editingSupplierId.value = null
  supplierForm.value = {
    code: `SUP-${(suppliers.value.length + 1).toString().padStart(3, '0')}`,
    name: '',
    contact_person: '',
    phone: '',
    address: '',
    term_of_payment_days: 14,
  }
  showSupplierModal.value = true
}

function openEditSupplierModal(supp: any) {
  isEditingSupplier.value = true
  editingSupplierId.value = supp.id
  supplierForm.value = {
    code: supp.code || '',
    name: supp.name || '',
    contact_person: supp.contact_person || '',
    phone: supp.phone || '',
    address: supp.address || '',
    term_of_payment_days: supp.term_of_payment_days || 14,
  }
  showSupplierModal.value = true
}

async function handleSaveSupplier() {
  const reqPerm = isEditingSupplier.value ? ['suppliers:update', 'purchasing:update', 'purchasing:write', 'master:write'] : ['suppliers:create', 'purchasing:create', 'purchasing:write', 'master:write']
  if (!authStore.hasPermission(reqPerm)) {
    showToast('Akses ditolak: Anda tidak memiliki wewenang mengelola pemasok / supplier', 'error')
    return
  }
  if (!supplierForm.value.name) {
    showToast('Nama supplier wajib diisi', 'error')
    return
  }

  try {
    isSavingSupplier.value = true
    if (isEditingSupplier.value && editingSupplierId.value) {
      const res = await api.put(`/suppliers/${editingSupplierId.value}`, supplierForm.value)
      if (res.data?.success) {
        showToast('Data supplier berhasil diperbarui', 'success')
        showSupplierModal.value = false
        loadData()
      } else {
        showToast(res.data?.message || 'Gagal memperbarui supplier', 'error')
      }
    } else {
      const res = await api.post('/suppliers', supplierForm.value)
      if (res.data?.success) {
        showToast('Supplier baru berhasil ditambahkan', 'success')
        showSupplierModal.value = false
        loadData()
      } else {
        showToast(res.data?.message || 'Gagal menambahkan supplier', 'error')
      }
    }
  } catch (err: any) {
    showToast(err.response?.data?.message || err.message || 'Gagal menyimpan data supplier', 'error')
  } finally {
    isSavingSupplier.value = false
  }
}

async function handleDeleteSupplier(supp: any) {
  if (!authStore.hasPermission(['suppliers:delete', 'purchasing:write', 'master:write'])) {
    showToast('Akses ditolak: Anda tidak memiliki wewenang menghapus pemasok / supplier', 'error')
    return
  }
  const confirmed = await dialog.confirm({
    title: 'Hapus Pemasok',
    message: `Hapus pemasok "${supp.name}" (${supp.code})?`,
    subMessage: 'Pemasok yang dihapus tidak akan dapat dipilih lagi pada faktur pembelian baru.',
    confirmText: 'Ya, Hapus',
    cancelText: 'Batal',
    type: 'danger',
  })
  if (!confirmed) return

  try {
    const res = await api.delete(`/suppliers/${supp.id}`)
    if (res.data?.success) {
      showToast('Pemasok berhasil dihapus', 'success')
      loadData()
    } else {
      showToast(res.data?.message || 'Gagal menghapus pemasok', 'error')
    }
  } catch (err: any) {
    showToast(err.response?.data?.message || err.message || 'Gagal menghapus pemasok', 'error')
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
          <div class="w-11 h-11 rounded-xl bg-blue-50 border border-blue-300 flex items-center justify-center shadow-xs">
            <Icon icon="streamline-ultimate-color:delivery-truck-cargo" class="w-7 h-7" />
          </div>
          <div>
            <h1 class="text-lg font-black text-slate-900 tracking-tight flex items-center gap-2">
              <span>Pengadaan & Pembelian (Procurement)</span>
              <span class="text-[10px] bg-blue-100 text-blue-900 font-mono px-2 py-0.5 rounded-full border border-blue-300 font-bold">
                SUPPLY CHAIN
              </span>
            </h1>
            <p class="text-xs text-slate-500 mt-0.5">
              Penerimaan faktur masuk (GRN), pencatatan tagihan hutang supplier, pesanan PO & direktori pemasok
            </p>
          </div>
        </div>

        <div class="flex items-center gap-2.5">
          <button
            type="button"
            @click="loadData"
            :disabled="isLoading"
            class="px-3.5 py-2 rounded-lg border border-slate-300 bg-white hover:bg-slate-50 text-slate-700 font-bold text-xs flex items-center gap-2 transition active:scale-95 cursor-pointer shadow-xs"
            title="Muat Ulang Data"
          >
            <RefreshCw class="w-3.5 h-3.5" :class="{ 'animate-spin': isLoading }" />
            <span>Segarkan</span>
          </button>

          <!-- Tombol Ekspor Excel Sesuai Sub-Tab Aktif -->
          <button
            v-if="authStore.hasPermission(['purchasing:export', 'purchasing:read', 'purchasing:view'])"
            type="button"
            @click="exportPurchasesToExcel"
            class="px-3.5 py-2 rounded-lg bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-xs flex items-center gap-1.5 transition active:scale-95 cursor-pointer shadow-xs border border-emerald-700"
            title="Ekspor tabel aktif sesuai filter dan urutan ke Excel di latar belakang"
          >
            <FileSpreadsheet class="w-4 h-4" />
            <span>Ekspor Excel</span>
          </button>

          <button
            v-if="activeTab === 'orders'"
            type="button"
            @click="openCreatePoModal"
            class="px-4 py-2 rounded-lg bg-indigo-600 hover:bg-indigo-700 text-white font-bold text-xs flex items-center gap-2 transition active:scale-95 cursor-pointer shadow-xs border border-indigo-700"
          >
            <Plus class="w-4 h-4" />
            <span>+ Buat PO Baru</span>
          </button>

          <button
            v-else-if="activeTab === 'receipts'"
            type="button"
            @click="openCreateGrnModal()"
            class="px-4 py-2 rounded-lg bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-xs flex items-center gap-2 transition active:scale-95 cursor-pointer shadow-xs border border-emerald-700"
          >
            <Plus class="w-4 h-4" />
            <span>+ Terima Barang (GRN)</span>
          </button>

          <button
            v-else-if="activeTab === 'invoices'"
            type="button"
            @click="openCreateInvoiceModal()"
            class="px-4 py-2 rounded-lg bg-blue-600 hover:bg-blue-700 text-white font-bold text-xs flex items-center gap-2 transition active:scale-95 cursor-pointer shadow-xs border border-blue-700"
          >
            <Plus class="w-4 h-4" />
            <span>+ Buat Faktur Pembelian</span>
          </button>

          <button
            v-else-if="activeTab === 'returns'"
            type="button"
            @click="openCreateReturnModal()"
            class="px-4 py-2 rounded-lg bg-rose-600 hover:bg-rose-700 text-white font-bold text-xs flex items-center gap-2 transition active:scale-95 cursor-pointer shadow-xs border border-rose-700"
          >
            <RotateCcw class="w-4 h-4" />
            <span>+ Buat Retur Pembelian</span>
          </button>

          <button
            v-else-if="activeTab === 'suppliers'"
            type="button"
            @click="openAddSupplierModal"
            class="px-4 py-2 rounded-lg bg-slate-800 hover:bg-slate-900 text-white font-bold text-xs flex items-center gap-2 transition active:scale-95 cursor-pointer shadow-xs border border-slate-900"
          >
            <Plus class="w-4 h-4" />
            <span>+ Tambah Supplier Baru</span>
          </button>
        </div>
      </div>

      <!-- 2. Navigation Tabs Bar -->
      <div class="px-6 pt-3 shrink-0 flex items-center gap-2 border-b border-slate-200 bg-white">
        <button
          type="button"
          @click="activeTab = 'orders'"
          class="flex items-center gap-2 px-4 py-2.5 font-bold text-xs border-b-2 transition cursor-pointer"
          :class="activeTab === 'orders'
            ? 'border-indigo-600 text-indigo-800'
            : 'border-transparent text-slate-500 hover:text-slate-900'"
        >
          <Clock class="w-4 h-4" />
          <span>1. Pesanan Pembelian (PO)</span>
          <span class="ml-1 text-[10px] px-2 py-0.5 rounded-full bg-slate-100 text-slate-700 font-mono font-bold">
            {{ totalPurchaseOrders }}
          </span>
        </button>

        <button
          type="button"
          @click="activeTab = 'receipts'"
          class="flex items-center gap-2 px-4 py-2.5 font-bold text-xs border-b-2 transition cursor-pointer"
          :class="activeTab === 'receipts'
            ? 'border-emerald-600 text-emerald-800'
            : 'border-transparent text-slate-500 hover:text-slate-900'"
        >
          <Truck class="w-4 h-4" />
          <span>2. Penerimaan Barang (GRN)</span>
          <span class="ml-1 text-[10px] px-2 py-0.5 rounded-full bg-slate-100 text-slate-700 font-mono font-bold">
            {{ totalGoodsReceipts }}
          </span>
        </button>

        <button
          type="button"
          @click="activeTab = 'invoices'"
          class="flex items-center gap-2 px-4 py-2.5 font-bold text-xs border-b-2 transition cursor-pointer"
          :class="activeTab === 'invoices'
            ? 'border-blue-600 text-blue-800'
            : 'border-transparent text-slate-500 hover:text-slate-900'"
        >
          <FileText class="w-4 h-4" />
          <span>3. Faktur Pembelian & Hutang</span>
          <span class="ml-1 text-[10px] px-2 py-0.5 rounded-full bg-slate-100 text-slate-700 font-mono font-bold">
            {{ totalInvoices }}
          </span>
        </button>

        <button
          type="button"
          @click="activeTab = 'returns'"
          class="flex items-center gap-2 px-4 py-2.5 font-bold text-xs border-b-2 transition cursor-pointer"
          :class="activeTab === 'returns'
            ? 'border-rose-600 text-rose-800'
            : 'border-transparent text-slate-500 hover:text-slate-900'"
        >
          <RotateCcw class="w-4 h-4" />
          <span>4. Retur Pembelian</span>
          <span class="ml-1 text-[10px] px-2 py-0.5 rounded-full bg-slate-100 text-slate-700 font-mono font-bold">
            {{ totalReturns }}
          </span>
        </button>

        <button
          type="button"
          @click="activeTab = 'suppliers'"
          class="flex items-center gap-2 px-4 py-2.5 font-bold text-xs border-b-2 transition cursor-pointer"
          :class="activeTab === 'suppliers'
            ? 'border-slate-800 text-slate-900'
            : 'border-transparent text-slate-500 hover:text-slate-900'"
        >
          <Building2 class="w-4 h-4" />
          <span>Master Pemasok (Supplier)</span>
          <span class="ml-1 text-[10px] px-2 py-0.5 rounded-full bg-slate-100 text-slate-700 font-mono font-bold">
            {{ totalSuppliers }}
          </span>
        </button>
      </div>

      <!-- 3. Tab Content -->
      <div class="flex-1 px-6 py-3 min-h-0 flex flex-col">
        <!-- TAB 1: PESANAN PEMBELIAN (PO) -->
        <div v-if="activeTab === 'orders'" class="bg-white border border-slate-300 rounded-xl flex flex-col h-full shadow-xs overflow-hidden">
          <!-- Filter Controls PO -->
          <div class="px-4 py-3 border-b border-slate-200 flex flex-wrap items-center justify-between gap-3 bg-slate-50/70 shrink-0">
            <div class="flex items-center gap-3 flex-1 min-w-[280px] max-w-lg">
              <div class="relative flex-1">
                <Search class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-slate-400" />
                <input
                  v-model="poSearchQuery"
                  @input="handlePoSearchInput"
                  type="text"
                  placeholder="Cari no. PO, nama pemasok, catatan..."
                  class="w-full pl-9 pr-3.5 py-1.5 rounded-lg border border-slate-300 bg-white text-xs text-slate-800 placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-indigo-500 transition"
                />
              </div>

              <select
                v-model="poStatusFilter"
                @change="handlePoStatusFilterChange"
                class="bg-white border border-slate-300 rounded-lg px-3 py-1.5 text-xs text-slate-800 font-semibold focus:outline-none focus:ring-2 focus:ring-indigo-500"
              >
                <option value="ALL">Semua Status PO</option>
                <option value="PENDING">Menunggu Pengiriman (PENDING)</option>
                <option value="PARTIAL">Diterima Sebagian (PARTIAL)</option>
                <option value="COMPLETED">Selesai Lengkap (COMPLETED)</option>
                <option value="CANCELLED">Dibatalkan (CANCELLED)</option>
              </select>

              <select
                v-model="selectedOutletId"
                @change="poPage = 1; loadPurchaseOrders()"
                class="bg-white border border-slate-300 rounded-lg px-3 py-1.5 text-xs text-slate-800 font-semibold focus:outline-none focus:ring-2 focus:ring-indigo-500"
              >
                <option value="">Semua Outlet</option>
                <option v-for="out in outlets" :key="out.id" :value="out.id">
                  {{ out.name }}
                </option>
              </select>
            </div>

            <div class="flex items-center gap-2">
              <div class="text-xs text-slate-500 font-bold">
                Total PO: <span class="font-mono text-slate-900">{{ totalPurchaseOrders }} Dokumen</span>
              </div>
              <button
                type="button"
                @click="loadPurchaseOrders"
                class="p-1.5 rounded-lg border border-slate-300 bg-white hover:bg-slate-100 text-slate-600 transition cursor-pointer"
                title="Refresh Daftar PO"
              >
                <RefreshCw class="w-3.5 h-3.5" :class="{ 'animate-spin': isLoadingOrders }" />
              </button>
            </div>
          </div>

          <div class="flex-1 overflow-auto min-h-0">
            <table class="w-full text-left text-xs text-slate-700 min-w-[880px]">
              <thead class="bg-slate-50 border-b border-slate-200 text-[11px] font-bold text-slate-500 uppercase tracking-wider sticky top-0 z-10 select-none">
                <tr>
                  <th @click="togglePoSort('po_number')" class="px-4 py-2.5 cursor-pointer hover:bg-slate-100 transition">
                    <div class="flex items-center gap-1">
                      <span>No. PO</span>
                      <ArrowUp v-if="poSortBy === 'po_number' && poSortDir === 'asc'" class="w-3.5 h-3.5 text-indigo-600" />
                      <ArrowDown v-else-if="poSortBy === 'po_number' && poSortDir === 'desc'" class="w-3.5 h-3.5 text-indigo-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th @click="togglePoSort('outlet_name')" class="px-4 py-2.5 cursor-pointer hover:bg-slate-100 transition">
                    <div class="flex items-center gap-1">
                      <span>Outlet Pemesan</span>
                      <ArrowUp v-if="poSortBy === 'outlet_name' && poSortDir === 'asc'" class="w-3.5 h-3.5 text-indigo-600" />
                      <ArrowDown v-else-if="poSortBy === 'outlet_name' && poSortDir === 'desc'" class="w-3.5 h-3.5 text-indigo-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th @click="togglePoSort('supplier_name')" class="px-4 py-2.5 cursor-pointer hover:bg-slate-100 transition">
                    <div class="flex items-center gap-1">
                      <span>Pemasok (Supplier)</span>
                      <ArrowUp v-if="poSortBy === 'supplier_name' && poSortDir === 'asc'" class="w-3.5 h-3.5 text-indigo-600" />
                      <ArrowDown v-else-if="poSortBy === 'supplier_name' && poSortDir === 'desc'" class="w-3.5 h-3.5 text-indigo-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th @click="togglePoSort('order_date')" class="px-4 py-2.5 cursor-pointer hover:bg-slate-100 transition">
                    <div class="flex items-center gap-1">
                      <span>Tanggal Order</span>
                      <ArrowUp v-if="poSortBy === 'order_date' && poSortDir === 'asc'" class="w-3.5 h-3.5 text-indigo-600" />
                      <ArrowDown v-else-if="poSortBy === 'order_date' && poSortDir === 'desc'" class="w-3.5 h-3.5 text-indigo-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th @click="togglePoSort('expected_delivery')" class="px-4 py-2.5 cursor-pointer hover:bg-slate-100 transition">
                    <div class="flex items-center gap-1">
                      <span>Estimasi Tiba</span>
                      <ArrowUp v-if="poSortBy === 'expected_delivery' && poSortDir === 'asc'" class="w-3.5 h-3.5 text-indigo-600" />
                      <ArrowDown v-else-if="poSortBy === 'expected_delivery' && poSortDir === 'desc'" class="w-3.5 h-3.5 text-indigo-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th @click="togglePoSort('total_amount')" class="px-4 py-2.5 text-right cursor-pointer hover:bg-slate-100 transition">
                    <div class="flex items-center justify-end gap-1">
                      <span>Nilai Total PO</span>
                      <ArrowUp v-if="poSortBy === 'total_amount' && poSortDir === 'asc'" class="w-3.5 h-3.5 text-indigo-600" />
                      <ArrowDown v-else-if="poSortBy === 'total_amount' && poSortDir === 'desc'" class="w-3.5 h-3.5 text-indigo-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th @click="togglePoSort('status')" class="px-4 py-2.5 text-center cursor-pointer hover:bg-slate-100 transition">
                    <div class="flex items-center justify-center gap-1">
                      <span>Status PO</span>
                      <ArrowUp v-if="poSortBy === 'status' && poSortDir === 'asc'" class="w-3.5 h-3.5 text-indigo-600" />
                      <ArrowDown v-else-if="poSortBy === 'status' && poSortDir === 'desc'" class="w-3.5 h-3.5 text-indigo-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th class="px-4 py-2.5 text-center">Aksi</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-slate-200">
                <tr v-for="po in purchaseOrders" :key="po.id" class="hover:bg-indigo-50/30 transition">
                  <td class="px-4 py-3 font-mono font-bold text-slate-900 whitespace-nowrap">
                    {{ po.po_number }}
                  </td>
                  <td class="px-4 py-3 whitespace-nowrap">
                    <div class="font-bold text-slate-800 flex items-center gap-1.5">
                      <Building2 class="w-3.5 h-3.5 text-indigo-600 shrink-0" />
                      <span>{{ po.outlet_name || 'Outlet Pusat' }}</span>
                    </div>
                  </td>
                  <td class="px-4 py-3 font-bold text-slate-900">
                    {{ po.supplier_name }}
                  </td>
                  <td class="px-4 py-3 text-slate-600 whitespace-nowrap font-mono">
                    {{ formatDate(po.order_date) }}
                  </td>
                  <td class="px-4 py-3 text-slate-600 whitespace-nowrap font-mono">
                    {{ po.expected_delivery ? formatDate(po.expected_delivery) : '-' }}
                  </td>
                  <td class="px-4 py-3 text-right font-mono font-black text-slate-900 whitespace-nowrap">
                    {{ formatRupiah(po.total_amount) }}
                  </td>
                  <td class="px-4 py-3 text-center whitespace-nowrap">
                    <span
                      v-if="po.status === 'COMPLETED'"
                      class="px-2.5 py-0.5 rounded-full text-[10px] font-extrabold bg-emerald-100 text-emerald-800 border border-emerald-300"
                    >
                      SELESAI (LENGKAP)
                    </span>
                    <span
                      v-else-if="po.status === 'PARTIAL'"
                      class="px-2.5 py-0.5 rounded-full text-[10px] font-extrabold bg-blue-100 text-blue-800 border border-blue-300"
                    >
                      DITERIMA SEBAGIAN
                    </span>
                    <span
                      v-else-if="po.status === 'PENDING' || po.status === 'ORDERED'"
                      class="px-2.5 py-0.5 rounded-full text-[10px] font-extrabold bg-amber-100 text-amber-900 border border-amber-300"
                    >
                      MENUNGGU PENGIRIMAN
                    </span>
                    <span
                      v-else-if="po.status === 'CANCELLED'"
                      class="px-2.5 py-0.5 rounded-full text-[10px] font-extrabold bg-rose-100 text-rose-800 border border-rose-300"
                    >
                      DIBATALKAN
                    </span>
                    <span
                      v-else
                      class="px-2 py-0.5 rounded text-[10px] font-bold bg-slate-100 text-slate-800 border border-slate-300"
                    >
                      {{ po.status }}
                    </span>
                  </td>
                  <td class="px-4 py-3 text-center whitespace-nowrap">
                    <div class="flex items-center justify-center gap-1.5">
                      <!-- Tombol Terima Barang (GRN) -->
                      <button
                        v-if="po.status !== 'COMPLETED' && po.status !== 'CANCELLED'"
                        type="button"
                        @click="openGrnFromPo(po)"
                        class="px-2.5 py-1 rounded-lg bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-[11px] flex items-center gap-1 transition cursor-pointer shadow-2xs"
                        title="Terima Fisik Barang (Bisa Bertahap/Parsial)"
                      >
                        <Truck class="w-3.5 h-3.5" />
                        <span>Terima Barang</span>
                      </button>

                      <!-- Tombol Detail PO -->
                      <button
                        type="button"
                        @click="viewPoDetail(po.id)"
                        class="p-1.5 rounded-lg border border-slate-300 bg-white hover:bg-slate-100 text-indigo-600 transition cursor-pointer shadow-2xs"
                        title="Lihat Detail Pesanan PO"
                      >
                        <Eye class="w-3.5 h-3.5" />
                      </button>

                      <!-- Tombol Batalkan PO -->
                      <button
                        v-if="(po.status === 'PENDING' || po.status === 'ORDERED') && authStore.hasPermission(['purchasing:void', 'purchasing:write'])"
                        type="button"
                        @click="handleCancelPo(po)"
                        class="p-1.5 rounded-lg border border-slate-300 bg-white hover:bg-rose-50 text-rose-600 transition cursor-pointer shadow-2xs hover:border-rose-300"
                        title="Batalkan Dokumen PO"
                      >
                        <X class="w-3.5 h-3.5" />
                      </button>
                    </div>
                  </td>
                </tr>

                <tr v-if="purchaseOrders.length === 0 && !isLoadingOrders">
                  <td colspan="8" class="px-6 py-12 text-center text-slate-400">
                    <Clock class="w-12 h-12 mx-auto mb-2 opacity-30 text-slate-400" />
                    <p class="font-bold text-sm text-slate-600">Belum ada dokumen Purchase Order (PO)</p>
                    <p class="text-xs text-slate-400 mt-0.5">Klik "+ Buat PO Baru" di atas untuk membuat pesanan pembelian ke pemasok</p>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <!-- Pagination Bar PO -->
          <div class="px-4 py-2.5 border-t border-slate-200 bg-slate-50 shrink-0">
            <PaginationBar
              v-model:currentPage="poPage"
              v-model:pageSize="poPageSize"
              :totalItems="totalPurchaseOrders"
              @update:currentPage="loadPurchaseOrders"
              @update:pageSize="() => { poPage = 1; loadPurchaseOrders() }"
            />
          </div>
        </div>

        <!-- TAB 2: PENERIMAAN BARANG (GOODS RECEIPT / GRN) -->
        <div v-else-if="activeTab === 'receipts'" class="bg-white border border-slate-300 rounded-xl flex flex-col h-full shadow-xs overflow-hidden">
          <!-- Filter Controls GRN -->
          <div class="px-4 py-3 border-b border-slate-200 flex flex-wrap items-center justify-between gap-3 bg-slate-50/70 shrink-0">
            <div class="flex items-center gap-3 flex-1 min-w-[280px] max-w-lg">
              <div class="relative flex-1">
                <Search class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-slate-400" />
                <input
                  v-model="receiptSearchQuery"
                  @input="handleReceiptSearchInput"
                  type="text"
                  placeholder="Cari no. GRN, surat jalan supplier, PO rujukan, supplier..."
                  class="w-full pl-9 pr-3.5 py-1.5 rounded-lg border border-slate-300 bg-white text-xs text-slate-800 placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-emerald-500 transition"
                />
              </div>

              <select
                v-model="receiptStatusFilter"
                @change="handleReceiptStatusFilterChange"
                class="bg-white border border-slate-300 rounded-lg px-3 py-1.5 text-xs text-slate-800 font-semibold focus:outline-none focus:ring-2 focus:ring-emerald-500"
              >
                <option value="ALL">Semua Status Faktur</option>
                <option value="UNINVOICED">Belum Difakturkan (UNINVOICED)</option>
                <option value="INVOICED">Sudah Difakturkan (INVOICED)</option>
              </select>

              <select
                v-model="selectedOutletId"
                @change="grnPage = 1; loadGoodsReceipts()"
                class="bg-white border border-slate-300 rounded-lg px-3 py-1.5 text-xs text-slate-800 font-semibold focus:outline-none focus:ring-2 focus:ring-emerald-500"
              >
                <option value="">Semua Outlet</option>
                <option v-for="out in outlets" :key="out.id" :value="out.id">
                  {{ out.name }}
                </option>
              </select>
            </div>

            <div class="flex items-center gap-2">
              <div class="text-xs text-slate-500 font-bold">
                Total Penerimaan: <span class="font-mono text-slate-900">{{ totalGoodsReceipts }} GRN</span>
              </div>
              <button
                type="button"
                @click="loadGoodsReceipts"
                class="p-1.5 rounded-lg border border-slate-300 bg-white hover:bg-slate-100 text-slate-600 transition cursor-pointer"
                title="Refresh Daftar GRN"
              >
                <RefreshCw class="w-3.5 h-3.5" :class="{ 'animate-spin': isLoadingReceipts }" />
              </button>
            </div>
          </div>

          <!-- Goods Receipts Table -->
          <div class="flex-1 overflow-auto min-h-0">
            <table class="w-full text-left text-xs text-slate-700 min-w-[950px]">
              <thead class="bg-slate-50 border-b border-slate-200 text-[11px] font-bold text-slate-500 uppercase tracking-wider sticky top-0 z-10 select-none">
                <tr>
                  <th @click="toggleGrnSort('grn_number')" class="px-4 py-2.5 cursor-pointer hover:bg-slate-100 transition">
                    <div class="flex items-center gap-1">
                      <span>No. GRN Fisik</span>
                      <ArrowUp v-if="grnSortBy === 'grn_number' && grnSortDir === 'asc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowDown v-else-if="grnSortBy === 'grn_number' && grnSortDir === 'desc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th @click="toggleGrnSort('delivery_order_number')" class="px-4 py-2.5 cursor-pointer hover:bg-slate-100 transition">
                    <div class="flex items-center gap-1">
                      <span>Surat Jalan Supplier</span>
                      <ArrowUp v-if="grnSortBy === 'delivery_order_number' && grnSortDir === 'asc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowDown v-else-if="grnSortBy === 'delivery_order_number' && grnSortDir === 'desc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th @click="toggleGrnSort('po_number')" class="px-4 py-2.5 cursor-pointer hover:bg-slate-100 transition">
                    <div class="flex items-center gap-1">
                      <span>Dokumen PO</span>
                      <ArrowUp v-if="grnSortBy === 'po_number' && grnSortDir === 'asc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowDown v-else-if="grnSortBy === 'po_number' && grnSortDir === 'desc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th @click="toggleGrnSort('supplier_name')" class="px-4 py-2.5 cursor-pointer hover:bg-slate-100 transition">
                    <div class="flex items-center gap-1">
                      <span>Pemasok (Supplier)</span>
                      <ArrowUp v-if="grnSortBy === 'supplier_name' && grnSortDir === 'asc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowDown v-else-if="grnSortBy === 'supplier_name' && grnSortDir === 'desc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th @click="toggleGrnSort('outlet_name')" class="px-4 py-2.5 cursor-pointer hover:bg-slate-100 transition">
                    <div class="flex items-center gap-1">
                      <span>Outlet Penerima</span>
                      <ArrowUp v-if="grnSortBy === 'outlet_name' && grnSortDir === 'asc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowDown v-else-if="grnSortBy === 'outlet_name' && grnSortDir === 'desc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th @click="toggleGrnSort('receive_date')" class="px-4 py-2.5 cursor-pointer hover:bg-slate-100 transition">
                    <div class="flex items-center gap-1">
                      <span>Tanggal Diterima</span>
                      <ArrowUp v-if="grnSortBy === 'receive_date' && grnSortDir === 'asc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowDown v-else-if="grnSortBy === 'receive_date' && grnSortDir === 'desc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th @click="toggleGrnSort('total_received_qty')" class="px-4 py-2.5 text-right cursor-pointer hover:bg-slate-100 transition">
                    <div class="flex items-center justify-end gap-1">
                      <span>Qty Masuk</span>
                      <ArrowUp v-if="grnSortBy === 'total_received_qty' && grnSortDir === 'asc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowDown v-else-if="grnSortBy === 'total_received_qty' && grnSortDir === 'desc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th @click="toggleGrnSort('total_amount')" class="px-4 py-2.5 text-right cursor-pointer hover:bg-slate-100 transition">
                    <div class="flex items-center justify-end gap-1">
                      <span>Nilai Barang</span>
                      <ArrowUp v-if="grnSortBy === 'total_amount' && grnSortDir === 'asc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowDown v-else-if="grnSortBy === 'total_amount' && grnSortDir === 'desc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th @click="toggleGrnSort('status')" class="px-4 py-2.5 text-center cursor-pointer hover:bg-slate-100 transition">
                    <div class="flex items-center justify-center gap-1">
                      <span>Status Faktur</span>
                      <ArrowUp v-if="grnSortBy === 'status' && grnSortDir === 'asc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowDown v-else-if="grnSortBy === 'status' && grnSortDir === 'desc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th class="px-4 py-2.5 text-center">Aksi</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-slate-200">
                <tr
                  v-for="grn in goodsReceipts"
                  :key="grn.id"
                  class="hover:bg-emerald-50/30 transition group"
                >
                  <td class="px-4 py-3 font-mono font-bold text-slate-900 whitespace-nowrap">
                    <span class="text-emerald-700">{{ grn.grn_number }}</span>
                  </td>
                  <td class="px-4 py-3 font-mono text-slate-700 whitespace-nowrap font-bold">
                    {{ grn.delivery_order_number }}
                  </td>
                  <td class="px-4 py-3 font-mono text-indigo-700 whitespace-nowrap font-semibold">
                    {{ grn.po_number || '-' }}
                  </td>
                  <td class="px-4 py-3 font-bold text-slate-800">
                    {{ grn.supplier_name }}
                  </td>
                  <td class="px-4 py-3 whitespace-nowrap">
                    <div class="font-bold text-slate-800 flex items-center gap-1.5">
                      <Building2 class="w-3.5 h-3.5 text-slate-500 shrink-0" />
                      <span>{{ grn.outlet_name || 'Outlet Pusat' }}</span>
                    </div>
                  </td>
                  <td class="px-4 py-3 text-slate-600 whitespace-nowrap font-mono">
                    {{ formatDate(grn.receive_date) }}
                  </td>
                  <td class="px-4 py-3 text-right font-mono font-bold text-slate-800 whitespace-nowrap">
                    {{ formatQty(grn.total_received_qty) }}
                  </td>
                  <td class="px-4 py-3 text-right font-mono font-black text-slate-900 whitespace-nowrap">
                    {{ formatRupiah(grn.total_amount) }}
                  </td>
                  <td class="px-4 py-3 text-center whitespace-nowrap">
                    <span
                      v-if="grn.status === 'CANCELLED'"
                      class="px-2.5 py-0.5 rounded-full text-[10px] font-extrabold bg-rose-100 text-rose-800 border border-rose-300"
                    >
                      DIBATALKAN
                    </span>
                    <span
                      v-else-if="grn.status === 'INVOICED'"
                      class="px-2.5 py-0.5 rounded-full text-[10px] font-extrabold bg-emerald-100 text-emerald-800 border border-emerald-300"
                    >
                      SUDAH DIFAKTURKAN
                    </span>
                    <span
                      v-else-if="grn.status === 'PARTIALLY_INVOICED'"
                      class="px-2.5 py-0.5 rounded-full text-[10px] font-extrabold bg-blue-100 text-blue-800 border border-blue-300"
                    >
                      SEBAGIAN DIFAKTURKAN
                    </span>
                    <span
                      v-else
                      class="px-2.5 py-0.5 rounded-full text-[10px] font-extrabold bg-amber-100 text-amber-900 border border-amber-300"
                    >
                      BELUM DIFAKTURKAN
                    </span>
                  </td>
                  <td class="px-4 py-3 text-center whitespace-nowrap">
                    <div class="flex items-center justify-center gap-1.5">
                      <!-- Tombol Terbitkan Faktur Pembelian -->
                      <button
                        v-if="grn.status === 'UNINVOICED' || grn.status === 'PARTIALLY_INVOICED'"
                        type="button"
                        @click="openCreateInvoiceModal(grn)"
                        class="px-2.5 py-1 rounded-lg bg-blue-600 hover:bg-blue-700 text-white font-bold text-[11px] flex items-center gap-1 transition cursor-pointer shadow-2xs"
                        title="Terbitkan Faktur Pembelian untuk Penerimaan Barang Ini"
                      >
                        <FileText class="w-3.5 h-3.5" />
                        <span>Buat Faktur</span>
                      </button>

                      <!-- Tombol Batalkan GRN -->
                      <button
                        v-if="grn.status === 'UNINVOICED' && authStore.hasPermission(['purchasing:void', 'purchasing:write'])"
                        type="button"
                        @click="handleCancelGrn(grn)"
                        class="p-1.5 rounded-lg border border-rose-300 bg-white hover:bg-rose-50 text-rose-600 transition cursor-pointer shadow-2xs"
                        title="Batalkan Penerimaan Barang Ini (Kembalikan Stok)"
                      >
                        <Trash2 class="w-3.5 h-3.5" />
                      </button>

                      <!-- Tombol Detail GRN -->
                      <button
                        type="button"
                        @click="openGrnDetailModal(grn)"
                        class="p-1.5 rounded-lg border border-slate-300 bg-white hover:bg-slate-100 text-emerald-600 transition cursor-pointer shadow-2xs"
                        title="Lihat Detail Penerimaan Fisik (GRN)"
                      >
                        <Eye class="w-3.5 h-3.5" />
                      </button>
                    </div>
                  </td>
                </tr>

                <tr v-if="goodsReceipts.length === 0 && !isLoadingReceipts">
                  <td colspan="10" class="px-6 py-12 text-center text-slate-400">
                    <Truck class="w-12 h-12 mx-auto mb-2 opacity-30 text-slate-400" />
                    <p class="font-bold text-sm text-slate-600">Belum ada dokumen Penerimaan Barang (GRN)</p>
                    <p class="text-xs text-slate-400 mt-0.5">Buka tab "Pesanan Pembelian (PO)" lalu klik "Terima Barang" saat barang fisik tiba di gudang</p>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <!-- Pagination Bar GRN -->
          <div class="px-4 py-2.5 border-t border-slate-200 bg-slate-50 shrink-0">
            <PaginationBar
              v-model:currentPage="grnPage"
              v-model:pageSize="grnPageSize"
              :totalItems="totalGoodsReceipts"
              @update:currentPage="loadGoodsReceipts"
              @update:pageSize="() => { grnPage = 1; loadGoodsReceipts() }"
            />
          </div>
        </div>

        <!-- TAB 3: FAKTUR PEMBELIAN & HUTANG (PURCHASE INVOICES) -->
        <div v-else-if="activeTab === 'invoices'" class="bg-white border border-slate-300 rounded-xl flex flex-col h-full shadow-xs overflow-hidden">
          <!-- Filter Controls -->
          <div class="px-4 py-3 border-b border-slate-200 flex flex-wrap items-center justify-between gap-3 bg-slate-50/70 shrink-0">
            <div class="flex items-center gap-3 flex-1 min-w-[280px] max-w-lg">
              <div class="relative flex-1">
                <Search class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-slate-400" />
                <input
                  v-model="invoiceSearchQuery"
                  @input="handleInvoiceSearchInput"
                  type="text"
                  placeholder="Cari no. faktur internal, surat jalan supplier, nama pemasok..."
                  class="w-full pl-9 pr-3.5 py-1.5 rounded-lg border border-slate-300 bg-white text-xs text-slate-800 placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-blue-500 transition"
                />
              </div>

              <select
                v-model="invoiceStatusFilter"
                @change="handleStatusFilterChange"
                class="bg-white border border-slate-300 rounded-lg px-3 py-1.5 text-xs text-slate-800 font-semibold focus:outline-none focus:ring-2 focus:ring-blue-500"
              >
                <option value="">Semua Status</option>
                <option value="PAID">Lunas (PAID)</option>
                <option value="UNPAID">Belum Lunas (UNPAID)</option>
              </select>

              <select
                v-model="selectedOutletId"
                @change="invPage = 1; loadInvoices()"
                class="bg-white border border-slate-300 rounded-lg px-3 py-1.5 text-xs text-slate-800 font-semibold focus:outline-none focus:ring-2 focus:ring-blue-500"
              >
                <option value="">Semua Outlet</option>
                <option v-for="out in outlets" :key="out.id" :value="out.id">
                  {{ out.name }}
                </option>
              </select>
            </div>

            <div class="text-xs text-slate-500 font-bold">
              Total Tagihan: <span class="font-mono text-slate-900">{{ totalInvoices }} Faktur</span>
            </div>
          </div>

          <!-- Invoices Table -->
          <div class="flex-1 overflow-auto min-h-0">
            <table class="w-full text-left text-xs text-slate-700 min-w-[950px]">
              <thead class="bg-slate-50 border-b border-slate-200 text-[11px] font-bold text-slate-500 uppercase tracking-wider sticky top-0 z-10 select-none">
                <tr>
                  <th @click="toggleSort('invoice_number')" class="px-4 py-2.5 cursor-pointer hover:bg-slate-100 transition">
                    <div class="flex items-center gap-1">
                      <span>No. Faktur Internal</span>
                      <ArrowUp v-if="sortBy === 'invoice_number' && sortDir === 'asc'" class="w-3.5 h-3.5 text-blue-600" />
                      <ArrowDown v-else-if="sortBy === 'invoice_number' && sortDir === 'desc'" class="w-3.5 h-3.5 text-blue-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th class="px-4 py-2.5">Faktur / SJ Supplier</th>
                  <th class="px-4 py-2.5">Rujukan GRN / PO</th>
                  <th class="px-4 py-2.5">Outlet & Gudang</th>
                  <th @click="toggleSort('supplier_name')" class="px-4 py-2.5 cursor-pointer hover:bg-slate-100 transition">
                    <div class="flex items-center gap-1">
                      <span>Pemasok (Supplier)</span>
                      <ArrowUp v-if="sortBy === 'supplier_name' && sortDir === 'asc'" class="w-3.5 h-3.5 text-blue-600" />
                      <ArrowDown v-else-if="sortBy === 'supplier_name' && sortDir === 'desc'" class="w-3.5 h-3.5 text-blue-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th @click="toggleSort('invoice_date')" class="px-4 py-2.5 cursor-pointer hover:bg-slate-100 transition">
                    <div class="flex items-center gap-1">
                      <span>Tgl Faktur</span>
                      <ArrowUp v-if="sortBy === 'invoice_date' && sortDir === 'asc'" class="w-3.5 h-3.5 text-blue-600" />
                      <ArrowDown v-else-if="sortBy === 'invoice_date' && sortDir === 'desc'" class="w-3.5 h-3.5 text-blue-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th @click="toggleSort('due_date')" class="px-4 py-2.5 cursor-pointer hover:bg-slate-100 transition">
                    <div class="flex items-center gap-1">
                      <span>Jatuh Tempo</span>
                      <ArrowUp v-if="sortBy === 'due_date' && sortDir === 'asc'" class="w-3.5 h-3.5 text-blue-600" />
                      <ArrowDown v-else-if="sortBy === 'due_date' && sortDir === 'desc'" class="w-3.5 h-3.5 text-blue-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th @click="toggleSort('total_amount')" class="px-4 py-2.5 text-right cursor-pointer hover:bg-slate-100 transition">
                    <div class="flex items-center justify-end gap-1">
                      <span>Total Tagihan</span>
                      <ArrowUp v-if="sortBy === 'total_amount' && sortDir === 'asc'" class="w-3.5 h-3.5 text-blue-600" />
                      <ArrowDown v-else-if="sortBy === 'total_amount' && sortDir === 'desc'" class="w-3.5 h-3.5 text-blue-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th @click="toggleSort('payment_status')" class="px-4 py-2.5 text-center cursor-pointer hover:bg-slate-100 transition">
                    <div class="flex items-center justify-center gap-1">
                      <span>Status Bayar</span>
                      <ArrowUp v-if="sortBy === 'payment_status' && sortDir === 'asc'" class="w-3.5 h-3.5 text-blue-600" />
                      <ArrowDown v-else-if="sortBy === 'payment_status' && sortDir === 'desc'" class="w-3.5 h-3.5 text-blue-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th class="px-4 py-2.5 text-center">Aksi</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-slate-200">
                <tr
                  v-for="inv in invoices"
                  :key="inv.id"
                  class="hover:bg-blue-50/40 transition group"
                >
                  <td class="px-4 py-3 font-mono font-bold text-slate-900 whitespace-nowrap">
                    {{ inv.invoice_number }}
                  </td>
                  <td class="px-4 py-3 font-mono text-slate-600 font-bold">
                    {{ inv.supplier_invoice_number }}
                  </td>
                  <td class="px-4 py-3 font-mono text-slate-600 whitespace-nowrap">
                    <div v-if="inv.grn_number" class="text-emerald-700 font-bold text-[11px]">
                      {{ inv.grn_number }}
                    </div>
                    <div v-if="inv.po_number" class="text-indigo-600 text-[10px]">
                      PO: {{ inv.po_number }}
                    </div>
                    <span v-if="!inv.grn_number && !inv.po_number" class="text-slate-400">-</span>
                  </td>
                  <td class="px-4 py-3 whitespace-nowrap">
                    <div class="font-bold text-slate-800 flex items-center gap-1.5">
                      <Building2 class="w-3.5 h-3.5 text-blue-600 shrink-0" />
                      <span>{{ inv.outlet_name || 'Semua / Pusat' }}</span>
                    </div>
                  </td>
                  <td class="px-4 py-3 font-bold text-slate-800">
                    {{ inv.supplier_name }}
                  </td>
                  <td class="px-4 py-3 text-slate-600 whitespace-nowrap">
                    {{ formatDate(inv.invoice_date) }}
                  </td>
                  <td class="px-4 py-3 text-slate-600 whitespace-nowrap">
                    {{ formatDate(inv.due_date) }}
                  </td>
                  <td class="px-4 py-3 text-right font-mono font-black text-slate-900 whitespace-nowrap">
                    {{ formatCurrency(inv.total_amount) }}
                  </td>
                  <td class="px-4 py-3 text-center whitespace-nowrap">
                    <span
                      v-if="inv.payment_status === 'CANCELLED'"
                      class="px-2.5 py-0.5 rounded-full text-[10px] font-extrabold bg-rose-100 text-rose-800 border border-rose-300"
                    >
                      DIBATALKAN
                    </span>
                    <span
                      v-else-if="inv.payment_status === 'PAID'"
                      class="px-2.5 py-0.5 rounded-full text-[10px] font-extrabold bg-emerald-100 text-emerald-800 border border-emerald-300"
                    >
                      LUNAS
                    </span>
                    <span
                      v-else
                      class="px-2.5 py-0.5 rounded-full text-[10px] font-extrabold bg-amber-100 text-amber-900 border border-amber-300"
                    >
                      BELUM LUNAS
                    </span>
                  </td>
                  <td class="px-4 py-3 text-center whitespace-nowrap">
                    <div class="flex items-center justify-center gap-1.5">
                      <button
                        type="button"
                        @click="viewInvoiceDetail(inv.id)"
                        class="p-1.5 rounded-lg border border-slate-300 bg-white hover:bg-slate-100 text-blue-600 transition cursor-pointer shadow-2xs"
                        title="Lihat Detail Faktur"
                      >
                        <Eye class="w-3.5 h-3.5" />
                      </button>

                      <button
                        v-if="inv.payment_status === 'UNPAID' && authStore.hasPermission(['purchasing:pay', 'purchasing:write'])"
                        type="button"
                        @click="handlePayInvoice(inv)"
                        class="px-2.5 py-1 rounded-lg border border-emerald-300 bg-white hover:bg-emerald-50 text-emerald-700 font-bold text-[11px] flex items-center gap-1 transition cursor-pointer shadow-2xs"
                        title="Tandai Sudah Dibayar"
                      >
                        <Check class="w-3.5 h-3.5" />
                        <span>Bayar</span>
                      </button>

                      <!-- Tombol Retur Barang dari Faktur -->
                      <button
                        v-if="inv.payment_status !== 'CANCELLED' && authStore.hasPermission(['purchasing:void', 'purchasing:write'])"
                        type="button"
                        @click="openCreateReturnModal({ invoice: inv })"
                        class="p-1.5 rounded-lg border border-rose-300 bg-white hover:bg-rose-50 text-rose-600 transition cursor-pointer shadow-2xs"
                        title="Retur Barang dari Faktur Ini"
                      >
                        <RotateCcw class="w-3.5 h-3.5" />
                      </button>

                      <!-- Tombol Batalkan Faktur -->
                      <button
                        v-if="inv.payment_status === 'UNPAID' && authStore.hasPermission(['purchasing:void', 'purchasing:write'])"
                        type="button"
                        @click="handleCancelInvoice(inv)"
                        class="p-1.5 rounded-lg border border-rose-300 bg-white hover:bg-rose-50 text-rose-700 transition cursor-pointer shadow-2xs"
                        title="Batalkan Faktur Pembelian Ini"
                      >
                        <Trash2 class="w-3.5 h-3.5" />
                      </button>
                    </div>
                  </td>
                </tr>

                <tr v-if="invoices.length === 0 && !isLoading">
                  <td colspan="10" class="px-6 py-12 text-center text-slate-400">
                    <FileText class="w-12 h-12 mx-auto mb-2 opacity-30 text-slate-400" />
                    <p class="font-bold text-sm text-slate-600">Belum ada faktur pembelian terbit</p>
                    <p class="text-xs text-slate-400 mt-0.5">Buka tab "Penerimaan Barang (GRN)" lalu klik "Buat Faktur" untuk menerbitkan tagihan</p>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <!-- Pagination Bar -->
          <div class="px-4 py-2.5 border-t border-slate-200 bg-slate-50 shrink-0">
            <PaginationBar
              v-model:currentPage="invPage"
              v-model:pageSize="invPageSize"
              :totalItems="totalInvoices"
              @update:currentPage="loadInvoices"
              @update:pageSize="() => { invPage = 1; loadInvoices() }"
            />
          </div>
        </div>

        <!-- TAB 4: RETUR PEMBELIAN (PURCHASE RETURNS) -->
        <div v-else-if="activeTab === 'returns'" class="bg-white border border-slate-300 rounded-xl flex flex-col h-full shadow-xs overflow-hidden">
          <!-- Filter Controls -->
          <div class="px-4 py-3 border-b border-slate-200 flex flex-wrap items-center justify-between gap-3 bg-slate-50/70 shrink-0">
            <div class="flex items-center gap-3 flex-1 min-w-[280px] max-w-lg">
              <div class="relative flex-1">
                <Search class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-slate-400" />
                <input
                  v-model="returnSearchQuery"
                  @input="handleReturnSearchInput"
                  type="text"
                  placeholder="Cari no. retur, nama pemasok, alasan retur..."
                  class="w-full pl-9 pr-3.5 py-1.5 rounded-lg border border-slate-300 bg-white text-xs text-slate-800 placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-rose-500 transition"
                />
              </div>

              <select
                v-model="returnStatusFilter"
                @change="handleReturnStatusFilterChange"
                class="px-3 py-1.5 rounded-lg border border-slate-300 bg-white text-xs text-slate-700 font-bold focus:outline-none focus:ring-2 focus:ring-rose-500 transition cursor-pointer"
              >
                <option value="ALL">Semua Status</option>
                <option value="COMPLETED">Selesai / Completed</option>
                <option value="CANCELLED">Dibatalkan</option>
              </select>
            </div>

            <div class="flex items-center gap-2">
              <button
                type="button"
                @click="loadPurchaseReturns"
                class="p-1.5 rounded-lg border border-slate-300 bg-white hover:bg-slate-100 text-slate-600 transition cursor-pointer"
                title="Refresh Daftar Retur"
              >
                <RefreshCw class="w-3.5 h-3.5" :class="{ 'animate-spin': isLoadingReturns }" />
              </button>
            </div>
          </div>

          <!-- Table -->
          <div class="flex-1 overflow-auto min-h-0">
            <table class="w-full text-left text-xs text-slate-700 min-w-[1000px]">
              <thead class="bg-slate-50 border-b border-slate-200 text-[11px] font-bold text-slate-500 uppercase tracking-wider sticky top-0 z-10 select-none">
                <tr>
                  <th @click="toggleRetSort('return_number')" class="px-4 py-2.5 cursor-pointer hover:bg-slate-100 transition">
                    <div class="flex items-center gap-1">
                      <span>No. Retur</span>
                      <ArrowUp v-if="retSortBy === 'return_number' && retSortDir === 'asc'" class="w-3.5 h-3.5 text-rose-600" />
                      <ArrowDown v-else-if="retSortBy === 'return_number' && retSortDir === 'desc'" class="w-3.5 h-3.5 text-rose-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th @click="toggleRetSort('return_date')" class="px-4 py-2.5 cursor-pointer hover:bg-slate-100 transition">
                    <div class="flex items-center gap-1">
                      <span>Tanggal</span>
                      <ArrowUp v-if="retSortBy === 'return_date' && retSortDir === 'asc'" class="w-3.5 h-3.5 text-rose-600" />
                      <ArrowDown v-else-if="retSortBy === 'return_date' && retSortDir === 'desc'" class="w-3.5 h-3.5 text-rose-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th @click="toggleRetSort('supplier_name')" class="px-4 py-2.5 cursor-pointer hover:bg-slate-100 transition">
                    <div class="flex items-center gap-1">
                      <span>Pemasok</span>
                      <ArrowUp v-if="retSortBy === 'supplier_name' && retSortDir === 'asc'" class="w-3.5 h-3.5 text-rose-600" />
                      <ArrowDown v-else-if="retSortBy === 'supplier_name' && retSortDir === 'desc'" class="w-3.5 h-3.5 text-rose-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th class="px-4 py-2.5">Rujukan</th>
                  <th @click="toggleRetSort('outlet_name')" class="px-4 py-2.5 cursor-pointer hover:bg-slate-100 transition">
                    <div class="flex items-center gap-1">
                      <span>Outlet / Gudang</span>
                      <ArrowUp v-if="retSortBy === 'outlet_name' && retSortDir === 'asc'" class="w-3.5 h-3.5 text-rose-600" />
                      <ArrowDown v-else-if="retSortBy === 'outlet_name' && retSortDir === 'desc'" class="w-3.5 h-3.5 text-rose-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th class="px-4 py-2.5">Alasan Retur</th>
                  <th class="px-4 py-2.5 text-center">Metode Kompensasi</th>
                  <th @click="toggleRetSort('total_amount')" class="px-4 py-2.5 text-right cursor-pointer hover:bg-slate-100 transition">
                    <div class="flex items-center justify-end gap-1">
                      <span>Total Nilai Retur</span>
                      <ArrowUp v-if="retSortBy === 'total_amount' && retSortDir === 'asc'" class="w-3.5 h-3.5 text-rose-600" />
                      <ArrowDown v-else-if="retSortBy === 'total_amount' && retSortDir === 'desc'" class="w-3.5 h-3.5 text-rose-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th @click="toggleRetSort('status')" class="px-4 py-2.5 text-center cursor-pointer hover:bg-slate-100 transition">
                    <div class="flex items-center justify-center gap-1">
                      <span>Status</span>
                      <ArrowUp v-if="retSortBy === 'status' && retSortDir === 'asc'" class="w-3.5 h-3.5 text-rose-600" />
                      <ArrowDown v-else-if="retSortBy === 'status' && retSortDir === 'desc'" class="w-3.5 h-3.5 text-rose-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th class="px-4 py-2.5 text-center">Aksi</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-slate-200">
                <tr v-for="ret in returns" :key="ret.id" class="hover:bg-rose-50/20 transition">
                  <td class="px-4 py-3 font-mono font-bold text-rose-900 whitespace-nowrap">
                    {{ ret.return_number }}
                  </td>
                  <td class="px-4 py-3 text-slate-600 whitespace-nowrap font-mono">
                    {{ formatDate(ret.return_date) }}
                  </td>
                  <td class="px-4 py-3 font-bold text-slate-900">
                    {{ ret.supplier_name }}
                  </td>
                  <td class="px-4 py-3 text-slate-600 whitespace-nowrap font-mono text-[11px]">
                    <div v-if="ret.invoice_number" class="text-blue-700 font-bold">
                      Faktur: {{ ret.invoice_number }}
                    </div>
                    <div v-if="ret.grn_number" class="text-emerald-700">
                      GRN: {{ ret.grn_number }}
                    </div>
                    <span v-if="!ret.invoice_number && !ret.grn_number" class="text-slate-400">-</span>
                  </td>
                  <td class="px-4 py-3 whitespace-nowrap">
                    <div class="font-bold text-slate-800 flex items-center gap-1.5">
                      <Building2 class="w-3.5 h-3.5 text-rose-600 shrink-0" />
                      <span>{{ ret.outlet_name || 'Outlet Pusat' }}</span>
                    </div>
                  </td>
                  <td class="px-4 py-3 text-slate-700 max-w-[200px] truncate" :title="ret.reason">
                    {{ ret.reason }}
                  </td>
                  <td class="px-4 py-3 text-center whitespace-nowrap">
                    <span
                      v-if="ret.refund_method === 'DEBT_REDUCTION'"
                      class="px-2.5 py-0.5 rounded-full text-[10px] font-extrabold bg-blue-100 text-blue-800 border border-blue-300"
                    >
                      POTONG HUTANG
                    </span>
                    <span
                      v-else-if="ret.refund_method === 'CASH_REFUND'"
                      class="px-2.5 py-0.5 rounded-full text-[10px] font-extrabold bg-emerald-100 text-emerald-800 border border-emerald-300"
                    >
                      KAS KEMBALI
                    </span>
                    <span
                      v-else
                      class="px-2.5 py-0.5 rounded-full text-[10px] font-extrabold bg-purple-100 text-purple-800 border border-purple-300"
                    >
                      GANTI BARANG
                    </span>
                  </td>
                  <td class="px-4 py-3 text-right font-mono font-black text-rose-950 whitespace-nowrap">
                    {{ formatRupiah(ret.total_amount) }}
                  </td>
                  <td class="px-4 py-3 text-center whitespace-nowrap">
                    <span
                      v-if="ret.status === 'COMPLETED'"
                      class="px-2.5 py-0.5 rounded-full text-[10px] font-extrabold bg-emerald-100 text-emerald-800 border border-emerald-300"
                    >
                      SELESAI
                    </span>
                    <span
                      v-else-if="ret.status === 'CANCELLED'"
                      class="px-2.5 py-0.5 rounded-full text-[10px] font-extrabold bg-rose-100 text-rose-800 border border-rose-300"
                    >
                      BATAL
                    </span>
                    <span
                      v-else
                      class="px-2.5 py-0.5 rounded-full text-[10px] font-extrabold bg-slate-100 text-slate-800 border border-slate-300"
                    >
                      {{ ret.status }}
                    </span>
                  </td>
                  <td class="px-4 py-3 text-center whitespace-nowrap">
                    <button
                      type="button"
                      @click="viewReturnDetail(ret.id)"
                      class="p-1.5 rounded-lg border border-slate-300 bg-white hover:bg-slate-100 text-rose-600 transition cursor-pointer shadow-2xs"
                      title="Lihat Detail Retur"
                    >
                      <Eye class="w-3.5 h-3.5" />
                    </button>
                  </td>
                </tr>

                <tr v-if="returns.length === 0 && !isLoadingReturns">
                  <td colspan="10" class="px-6 py-12 text-center text-slate-400">
                    <RotateCcw class="w-12 h-12 mx-auto mb-2 opacity-30 text-rose-400" />
                    <p class="font-bold text-sm text-slate-600">Belum ada dokumen Retur Pembelian</p>
                    <p class="text-xs text-slate-400 mt-0.5">Klik "+ Buat Retur Pembelian" atau gunakan tombol "Retur" pada Faktur Pembelian untuk mencatat pengembalian barang ke supplier</p>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <!-- Pagination Bar -->
          <div class="px-4 py-2.5 border-t border-slate-200 bg-slate-50 shrink-0">
            <PaginationBar
              v-model:currentPage="retPage"
              v-model:pageSize="retPageSize"
              :totalItems="totalReturns"
              @update:currentPage="loadPurchaseReturns"
              @update:pageSize="() => { retPage = 1; loadPurchaseReturns() }"
            />
          </div>
        </div>

        <!-- TAB 5: MASTER PEMASOK / SUPPLIER -->
        <div v-else-if="activeTab === 'suppliers'" class="bg-white border border-slate-300 rounded-xl flex flex-col h-full shadow-xs overflow-hidden">
          <!-- Filter Controls -->
          <div class="px-4 py-3 border-b border-slate-200 flex items-center justify-between gap-3 bg-slate-50/70 shrink-0">
            <div class="relative flex-1 max-w-md">
              <Search class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-slate-400" />
              <input
                v-model="supplierSearchQuery"
                @input="handleSupplierSearchInput"
                type="text"
                placeholder="Cari kode, nama pemasok, kontak person, telepon..."
                class="w-full pl-9 pr-3.5 py-1.5 rounded-lg border border-slate-300 bg-white text-xs text-slate-800 placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-emerald-500 transition"
              />
            </div>
            <div class="flex items-center gap-2">
              <div class="text-xs text-slate-500 font-bold">
                Total Pemasok: <span class="font-mono text-slate-900">{{ totalSuppliers }} Vendor</span>
              </div>
              <button
                type="button"
                @click="loadSuppliers"
                class="p-1.5 rounded-lg border border-slate-300 bg-white hover:bg-slate-100 text-slate-600 transition cursor-pointer"
                title="Refresh Daftar Pemasok"
              >
                <RefreshCw class="w-3.5 h-3.5" :class="{ 'animate-spin': isLoadingSuppliers }" />
              </button>
            </div>
          </div>

          <!-- Suppliers Table -->
          <div class="flex-1 overflow-auto min-h-0">
            <table class="w-full text-left text-xs text-slate-700 min-w-[850px]">
              <thead class="bg-slate-50 border-b border-slate-200 text-[11px] font-bold text-slate-500 uppercase tracking-wider sticky top-0 z-10 select-none">
                <tr>
                  <th @click="toggleSuppSort('code')" class="px-4 py-2.5 cursor-pointer hover:bg-slate-100 transition">
                    <div class="flex items-center gap-1">
                      <span>Kode</span>
                      <ArrowUp v-if="suppSortBy === 'code' && suppSortDir === 'asc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowDown v-else-if="suppSortBy === 'code' && suppSortDir === 'desc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th @click="toggleSuppSort('name')" class="px-4 py-2.5 cursor-pointer hover:bg-slate-100 transition">
                    <div class="flex items-center gap-1">
                      <span>Nama Pemasok / PT</span>
                      <ArrowUp v-if="suppSortBy === 'name' && suppSortDir === 'asc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowDown v-else-if="suppSortBy === 'name' && suppSortDir === 'desc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th @click="toggleSuppSort('contact_person')" class="px-4 py-2.5 cursor-pointer hover:bg-slate-100 transition">
                    <div class="flex items-center gap-1">
                      <span>Kontak Person</span>
                      <ArrowUp v-if="suppSortBy === 'contact_person' && suppSortDir === 'asc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowDown v-else-if="suppSortBy === 'contact_person' && suppSortDir === 'desc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th class="px-4 py-2.5">No. Telepon</th>
                  <th class="px-4 py-2.5 text-center">Term Pembayaran</th>
                  <th class="px-4 py-2.5">Alamat Kantor</th>
                  <th class="px-4 py-2.5 text-center">Aksi</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-slate-200">
                <tr
                  v-for="supp in suppliers"
                  :key="supp.id"
                  class="hover:bg-emerald-50/30 transition group"
                >
                  <td class="px-4 py-3 font-mono font-bold text-slate-900 whitespace-nowrap">
                    {{ supp.code }}
                  </td>
                  <td class="px-4 py-3 font-bold text-slate-900">
                    {{ supp.name }}
                  </td>
                  <td class="px-4 py-3 text-slate-700">
                    {{ supp.contact_person || '-' }}
                  </td>
                  <td class="px-4 py-3 font-mono text-slate-600">
                    {{ supp.phone || '-' }}
                  </td>
                  <td class="px-4 py-3 text-center font-bold text-slate-800">
                    <span class="px-2 py-0.5 rounded bg-slate-100 border border-slate-300 font-mono">
                      {{ supp.term_of_payment_days }} Hari
                    </span>
                  </td>
                  <td class="px-4 py-3 text-slate-600 max-w-xs truncate">
                    {{ supp.address || '-' }}
                  </td>
                  <td class="px-4 py-3 text-center whitespace-nowrap">
                    <div class="flex items-center justify-center gap-1.5">
                      <button
                        type="button"
                        @click="openEditSupplierModal(supp)"
                        class="p-1.5 rounded-lg border border-slate-300 bg-white hover:bg-slate-100 text-amber-700 transition cursor-pointer shadow-2xs"
                        title="Edit Data Pemasok"
                      >
                        <Edit2 class="w-3.5 h-3.5" />
                      </button>
                      <button
                        type="button"
                        @click="handleDeleteSupplier(supp)"
                        class="p-1.5 rounded-lg border border-slate-300 bg-white hover:bg-rose-50 text-rose-600 transition cursor-pointer shadow-2xs hover:border-rose-300"
                        title="Hapus Pemasok"
                      >
                        <Trash2 class="w-3.5 h-3.5" />
                      </button>
                    </div>
                  </td>
                </tr>

                <tr v-if="suppliers.length === 0 && !isLoadingSuppliers">
                  <td colspan="7" class="px-6 py-12 text-center text-slate-400">
                    <Building2 class="w-12 h-12 mx-auto mb-2 opacity-30 text-slate-400" />
                    <p class="font-bold text-sm text-slate-600">Tidak ada pemasok ditemukan</p>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <!-- Pagination Bar -->
          <div class="px-4 py-2.5 border-t border-slate-200 bg-slate-50 shrink-0">
            <PaginationBar
              v-model:currentPage="suppPage"
              v-model:pageSize="suppPageSize"
              :totalItems="totalSuppliers"
              @update:currentPage="loadSuppliers"
              @update:pageSize="() => { suppPage = 1; loadSuppliers() }"
            />
          </div>
        </div>
      </div>

      <!-- 4. Modal Buat Penerimaan Barang (GRN) Baru Berdasarkan PO -->
      <div
        v-if="showGrnModal"
        class="fixed inset-0 z-50 bg-slate-900/60 backdrop-blur-xs flex items-center justify-center p-4"
        @click.self="showGrnModal = false"
      >
        <div class="bg-white rounded-2xl shadow-2xl border border-slate-300 w-full max-w-4xl overflow-hidden flex flex-col max-h-[92vh] animate-in fade-in zoom-in-95 duration-150">
          <!-- Header -->
          <div class="px-6 py-4 bg-emerald-950 text-white flex items-center justify-between shrink-0">
            <div class="flex items-center gap-3">
              <div class="w-9 h-9 rounded-xl bg-emerald-500/20 border border-emerald-400/40 flex items-center justify-center">
                <Truck class="w-5 h-5 text-emerald-400" />
              </div>
              <div>
                <div class="text-xs font-bold text-emerald-300 uppercase tracking-wider">Penerimaan Barang Fisik (Goods Receipt Note)</div>
                <div class="text-base font-black tracking-tight">Catat Penerimaan Barang Masuk dari PO</div>
              </div>
            </div>
            <button
              type="button"
              @click="showGrnModal = false"
              class="text-emerald-300 hover:text-white p-1 rounded-lg hover:bg-emerald-900 transition cursor-pointer"
            >
              <X class="w-5 h-5" />
            </button>
          </div>

          <!-- Body -->
          <div class="p-6 overflow-y-auto space-y-5 flex-1 min-h-0 text-xs">
            <!-- Header Inputs -->
            <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3.5 bg-emerald-50/50 p-4 rounded-xl border border-emerald-200">
              <div class="sm:col-span-2 lg:col-span-2">
                <label class="block font-bold text-slate-700 mb-1">
                  Rujukan Dokumen Purchase Order (PO) *
                </label>
                <select
                  v-model="grnForm.po_id"
                  @change="handlePoSelectInGrn(grnForm.po_id)"
                  class="w-full px-3 py-2 rounded-lg border border-slate-300 bg-white font-sans text-xs focus:ring-2 focus:ring-emerald-500 focus:outline-none font-bold text-slate-800"
                >
                  <option value="" disabled>-- Pilih Dokumen Purchase Order (PO) --</option>
                  <option
                    v-if="grnForm.po_id && !purchaseOrders.filter((p) => p.status === 'PENDING' || p.status === 'PARTIAL' || p.status === 'ORDERED').some((p) => p.id === grnForm.po_id)"
                    :value="grnForm.po_id"
                  >
                    {{ grnForm.po_number }} - {{ grnForm.supplier_name }}
                  </option>
                  <option
                    v-for="po in purchaseOrders.filter((p) => p.status === 'PENDING' || p.status === 'PARTIAL' || p.status === 'ORDERED')"
                    :key="po.id"
                    :value="po.id"
                  >
                    {{ po.po_number }} - {{ po.supplier_name }} ({{ po.status }})
                  </option>
                </select>
                <p class="text-[10px] text-slate-500 mt-1">
                  Penerimaan fisik barang harus mengacu pada Purchase Order (PO) yang telah dibuat sebelumnya.
                </p>
              </div>

              <div>
                <label class="block font-bold text-slate-700 mb-1">Pemasok (Supplier)</label>
                <input
                  :value="grnForm.supplier_name || 'Otomatis dari PO'"
                  disabled
                  class="w-full px-3 py-2 rounded-lg border border-slate-300 bg-slate-100 font-bold text-slate-800 text-xs"
                />
              </div>

              <div>
                <label class="block font-bold text-slate-700 mb-1">Outlet Penerima</label>
                <input
                  :value="grnForm.outlet_name || 'Otomatis dari PO'"
                  disabled
                  class="w-full px-3 py-2 rounded-lg border border-slate-300 bg-slate-100 font-bold text-slate-800 text-xs"
                />
              </div>

              <div>
                <label class="block font-bold text-slate-700 mb-1">No. Surat Jalan / Pengiriman Supplier *</label>
                <input
                  v-model="grnForm.delivery_order_number"
                  type="text"
                  placeholder="Contoh: SJ-2026/09/8812"
                  class="w-full px-3 py-2 rounded-lg border border-slate-300 bg-white font-mono text-xs font-bold focus:ring-2 focus:ring-emerald-500 focus:outline-none"
                />
              </div>

              <div>
                <label class="block font-bold text-slate-700 mb-1">Tanggal Terima Fisik *</label>
                <input
                  v-model="grnForm.receive_date"
                  type="date"
                  class="w-full px-3 py-2 rounded-lg border border-slate-300 bg-white font-sans text-xs focus:ring-2 focus:ring-emerald-500 focus:outline-none font-bold"
                />
              </div>

              <div class="sm:col-span-2">
                <label class="block font-bold text-slate-700 mb-1">Catatan Pengiriman / Pemeriksaan Fisik</label>
                <input
                  v-model="grnForm.notes"
                  type="text"
                  placeholder="Catatan kondisi fisik barang atau surat jalan..."
                  class="w-full px-3 py-2 rounded-lg border border-slate-300 bg-white font-sans text-xs focus:ring-2 focus:ring-emerald-500 focus:outline-none"
                />
              </div>
            </div>

            <!-- Items Table Penerimaan Fisik -->
            <div>
              <div class="flex items-center justify-between mb-2">
                <div class="font-bold text-slate-800 uppercase tracking-wide flex items-center gap-2">
                  <span>Rincian Barang yang Diterima (Penerimaan Parsial Didukung)</span>
                  <span class="text-[10px] text-emerald-700 font-normal font-sans">
                    (Kuantitas fisik yang datang dapat disesuaikan per baris, misal pesan 100 datang 20 dulu)
                  </span>
                </div>
              </div>

              <div class="border border-slate-200 rounded-xl overflow-hidden shadow-2xs">
                <table class="w-full text-left text-xs text-slate-700">
                  <thead class="bg-slate-100 text-[10px] font-bold text-slate-500 uppercase border-b border-slate-200">
                    <tr>
                      <th class="px-3 py-2.5">Produk & SKU</th>
                      <th class="px-3 py-2.5 w-24 text-right">Qty PO</th>
                      <th class="px-3 py-2.5 w-24 text-right">Diterima Lalu</th>
                      <th class="px-3 py-2.5 w-24 text-right">Sisa Belum Tiba</th>
                      <th class="px-3 py-2.5 w-32 text-right text-emerald-800 font-extrabold">Qty Masuk Kini *</th>
                      <th class="px-3 py-2.5 w-32">Tgl Expired</th>
                      <th class="px-3 py-2.5 w-28">No. Batch</th>
                      <th class="px-3 py-2.5 w-28 text-center">Status Pemenuhan</th>
                      <th class="px-3 py-2.5 w-36">Barcode Tambahan</th>
                    </tr>
                  </thead>
                  <tbody class="divide-y divide-slate-100">
                    <tr v-for="(it, idx) in grnForm.items" :key="idx" class="hover:bg-emerald-50/20">
                      <!-- Product Info -->
                      <td class="px-3 py-2.5">
                        <div class="font-bold text-slate-900">{{ it.product_name }}</div>
                        <div class="text-[10px] text-slate-500 font-mono flex items-center gap-1.5 mt-0.5">
                          <span>SKU: {{ it.product_sku }}</span>
                          <span v-if="it.unit_code" class="px-1 py-0.2 rounded bg-slate-100 border border-slate-200 text-slate-600">
                            {{ it.unit_code }}
                          </span>
                        </div>
                      </td>

                      <!-- Qty PO -->
                      <td class="px-3 py-2.5 text-right font-mono text-slate-600 font-medium">
                        {{ formatQty(it.ordered_qty) }}
                      </td>

                      <!-- Diterima Lalu -->
                      <td class="px-3 py-2.5 text-right font-mono text-slate-500">
                        {{ formatQty(it.previously_received_qty || 0) }}
                      </td>

                      <!-- Sisa Belum Datang -->
                      <td class="px-3 py-2.5 text-right font-mono font-bold text-amber-700">
                        {{ formatQty(it.remaining_qty) }}
                      </td>

                      <!-- Qty Masuk Kini (Input) -->
                      <td class="px-3 py-2.5">
                        <input
                          v-model.number="it.received_qty"
                          type="number"
                          step="any"
                          min="0"
                          :max="it.remaining_qty"
                          class="w-full px-2.5 py-1.5 rounded-lg border border-emerald-400 bg-emerald-50/60 text-right font-mono text-xs focus:ring-2 focus:ring-emerald-500 focus:outline-none font-black text-emerald-950"
                          title="Jumlah kuantitas fisik barang yang diterima saat ini"
                        />
                      </td>

                      <!-- Tgl Expired -->
                      <td class="px-3 py-2.5">
                        <input
                          v-model="it.expiry_date"
                          type="date"
                          class="w-full px-2 py-1 rounded border border-slate-300 text-[11px] font-sans focus:ring-1 focus:ring-emerald-500"
                          title="Tanggal kadaluwarsa produk untuk FEFO"
                        />
                      </td>

                      <!-- No. Batch -->
                      <td class="px-3 py-2.5">
                        <input
                          v-model="it.batch_number"
                          type="text"
                          placeholder="No. Lot / Batch"
                          class="w-full px-2 py-1 rounded border border-slate-300 text-[11px] font-mono focus:ring-1 focus:ring-emerald-500"
                        />
                      </td>

                      <!-- Status Pemenuhan -->
                      <td class="px-3 py-2.5 text-center whitespace-nowrap">
                        <span
                          v-if="Number(it.received_qty) >= Number(it.remaining_qty) && Number(it.remaining_qty) > 0"
                          class="px-2 py-0.5 rounded text-[10px] font-bold bg-emerald-100 text-emerald-800 border border-emerald-300"
                        >
                          Lengkap (100%)
                        </span>
                        <span
                          v-else-if="Number(it.received_qty) > 0 && Number(it.received_qty) < Number(it.remaining_qty)"
                          class="px-2 py-0.5 rounded text-[10px] font-bold bg-blue-100 text-blue-800 border border-blue-300"
                        >
                          Parsial ({{ it.received_qty }}/{{ it.remaining_qty }})
                        </span>
                        <span
                          v-else
                          class="px-2 py-0.5 rounded text-[10px] font-bold bg-slate-100 text-slate-600 border border-slate-200"
                        >
                          Tidak Datang (0)
                        </span>
                      </td>

                      <!-- Barcode Baru -->
                      <td class="px-3 py-2.5">
                        <input
                          v-model="it.new_barcode"
                          type="text"
                          placeholder="Scan / ketik barcode..."
                          class="w-full px-2 py-1 rounded border border-slate-300 text-[11px] font-mono focus:ring-1 focus:ring-emerald-500"
                        />
                      </td>
                    </tr>

                    <tr v-if="grnForm.items.length === 0">
                      <td colspan="7" class="py-8 text-center text-slate-400">
                        <Package class="w-8 h-8 mx-auto mb-1 text-slate-300" />
                        <p class="font-bold text-xs">Pilih Purchase Order (PO) rujukan di atas</p>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>

            <!-- Info Catatan Otomatis -->
            <div class="p-3 bg-emerald-50 border border-emerald-200 rounded-xl text-emerald-900 text-[11px] flex items-start gap-2.5">
              <CheckCircle2 class="w-4 h-4 text-emerald-600 shrink-0 mt-0.5" />
              <div class="leading-relaxed">
                <strong>Otomatisasi Sistem:</strong> Menyimpan penerimaan barang ini akan langsung menambah stok fisik gudang/outlet terkait, mencatat mutasi stok GRN, menghitung Moving Average Cost produk, dan membukukan jurnal Persediaan vs Hutang Belum Ditagih (Unbilled GRN). Faktur pembelian dapat diterbitkan kemudian saat tagihan dari pemasok tiba.
              </div>
            </div>
          </div>

          <!-- Footer -->
          <div class="px-6 py-3.5 bg-slate-50 border-t border-slate-200 flex items-center justify-between gap-2.5 shrink-0">
            <div class="text-xs text-slate-600 flex items-center gap-4">
              <div>Total Masuk: <span class="font-black text-emerald-700 font-mono">{{ formatQty(totalGrnQtyReceived) }}</span> item</div>
              <div>Estimasi Nilai: <span class="font-black text-slate-900 font-mono">{{ formatCurrency(totalGrnValue) }}</span></div>
            </div>
            <div class="flex items-center gap-2.5">
              <button
                type="button"
                @click="showGrnModal = false"
                :disabled="isSavingGrn"
                class="px-4 py-2 rounded-lg border border-slate-300 bg-white hover:bg-slate-100 text-slate-700 font-bold text-xs transition cursor-pointer"
              >
                Batal
              </button>
              <button
                type="button"
                @click="handleSaveGrn"
                :disabled="isSavingGrn || grnForm.items.length === 0"
                class="px-5 py-2 rounded-lg bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-xs flex items-center gap-2 shadow-xs transition active:scale-95 cursor-pointer disabled:opacity-50"
              >
                <RefreshCw v-if="isSavingGrn" class="w-4 h-4 animate-spin" />
                <Check v-else class="w-4 h-4" />
                <span>Simpan Penerimaan Barang (GRN)</span>
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- MODAL: TERBITKAN FAKTUR PEMBELIAN DARI GRN (showInvoiceModal) -->
      <div
        v-if="showInvoiceModal"
        class="fixed inset-0 z-50 bg-slate-900/60 backdrop-blur-xs flex items-center justify-center p-4"
        @click.self="showInvoiceModal = false"
      >
        <div class="bg-white rounded-2xl shadow-2xl border border-slate-300 w-full max-w-4xl overflow-hidden flex flex-col max-h-[92vh] animate-in fade-in zoom-in-95 duration-150">
          <div class="px-6 py-4 bg-blue-950 text-white flex items-center justify-between shrink-0">
            <div class="flex items-center gap-3">
              <div class="w-9 h-9 rounded-xl bg-blue-500/20 border border-blue-400/40 flex items-center justify-center">
                <FileText class="w-5 h-5 text-blue-400" />
              </div>
              <div>
                <div class="text-xs font-bold text-blue-300 uppercase tracking-wider">Faktur Pembelian & Pengakuan Hutang</div>
                <div class="text-base font-black tracking-tight">Terbitkan Faktur Pembelian dari Penerimaan Barang (GRN)</div>
              </div>
            </div>
            <button
              type="button"
              @click="showInvoiceModal = false"
              class="text-blue-300 hover:text-white p-1 rounded-lg hover:bg-blue-900 transition cursor-pointer"
            >
              <X class="w-5 h-5" />
            </button>
          </div>

          <div class="p-6 overflow-y-auto space-y-5 flex-1 min-h-0 text-xs">
            <!-- Informasi GRN Rujukan -->
            <div class="p-3.5 bg-blue-50/80 border border-blue-200 rounded-xl grid grid-cols-2 sm:grid-cols-4 gap-3">
              <div>
                <div class="text-[10px] font-bold text-slate-500 uppercase">Dokumen GRN</div>
                <div class="font-mono font-bold text-blue-900 text-sm">{{ invoiceForm.grn_number }}</div>
              </div>
              <div>
                <div class="text-[10px] font-bold text-slate-500 uppercase">Surat Jalan Supplier</div>
                <div class="font-mono font-bold text-slate-800">{{ invoiceForm.delivery_order_number }}</div>
              </div>
              <div>
                <div class="text-[10px] font-bold text-slate-500 uppercase">Dokumen PO</div>
                <div class="font-mono font-bold text-indigo-700">{{ invoiceForm.po_number || '-' }}</div>
              </div>
              <div>
                <div class="text-[10px] font-bold text-slate-500 uppercase">Pemasok & Outlet</div>
                <div class="font-bold text-slate-800">{{ invoiceForm.supplier_name }} ({{ invoiceForm.outlet_name }})</div>
              </div>
            </div>

            <!-- Form Tagihan Faktur -->
            <div class="grid grid-cols-1 sm:grid-cols-3 gap-3.5 bg-slate-50 p-4 rounded-xl border border-slate-200">
              <div>
                <label class="block font-bold text-slate-700 mb-1">Nomor Faktur / Invoice dari Pemasok *</label>
                <input
                  v-model="invoiceForm.supplier_invoice_number"
                  type="text"
                  placeholder="Contoh: INV-2026/09/9921"
                  class="w-full px-3 py-2 rounded-lg border border-slate-300 bg-white font-mono text-xs font-bold focus:ring-2 focus:ring-blue-500 focus:outline-none"
                />
              </div>

              <div>
                <label class="block font-bold text-slate-700 mb-1">Tanggal Faktur Tagihan *</label>
                <input
                  v-model="invoiceForm.invoice_date"
                  type="date"
                  class="w-full px-3 py-2 rounded-lg border border-slate-300 bg-white font-sans text-xs focus:ring-2 focus:ring-blue-500 focus:outline-none font-bold"
                />
              </div>

              <div>
                <label class="block font-bold text-slate-700 mb-1">Tanggal Jatuh Tempo Pembayaran *</label>
                <input
                  v-model="invoiceForm.due_date"
                  type="date"
                  class="w-full px-3 py-2 rounded-lg border border-slate-300 bg-white font-sans text-xs focus:ring-2 focus:ring-blue-500 focus:outline-none font-bold text-rose-700"
                />
              </div>
            </div>

            <!-- Opsi Status Pembayaran Tagihan -->
            <div class="p-3.5 bg-slate-50 border border-slate-200 rounded-xl space-y-2">
              <div class="font-bold text-slate-800 text-xs">
                Ketentuan Pembayaran Faktur:
              </div>
              <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
                <label
                  class="flex items-start gap-3 p-3 rounded-xl border cursor-pointer transition select-none"
                  :class="invoiceForm.payment_status === 'UNPAID'
                    ? 'border-amber-400 bg-amber-50/70 text-amber-950 shadow-xs'
                    : 'border-slate-200 bg-white hover:bg-slate-50 text-slate-700'"
                >
                  <input
                    type="radio"
                    name="inv_pay_status"
                    value="UNPAID"
                    v-model="invoiceForm.payment_status"
                    class="mt-0.5 text-amber-600 focus:ring-amber-500"
                  />
                  <div>
                    <div class="font-bold text-xs flex items-center gap-1.5">
                      <span>Tempo / Hutang Usaha (AP)</span>
                      <span class="text-[10px] px-1.5 py-0.2 rounded bg-amber-200/80 text-amber-900 font-extrabold">BELUM LUNAS</span>
                    </div>
                    <p class="text-[11px] text-slate-500 mt-0.5 leading-snug">
                      Faktur dicatat sebagai <strong>Hutang Usaha</strong> ke pemasok dan dapat dilunasi saat jatuh tempo.
                    </p>
                  </div>
                </label>

                <label
                  class="flex items-start gap-3 p-3 rounded-xl border cursor-pointer transition select-none"
                  :class="invoiceForm.payment_status === 'PAID'
                    ? 'border-emerald-400 bg-emerald-50/70 text-emerald-950 shadow-xs'
                    : 'border-slate-200 bg-white hover:bg-slate-50 text-slate-700'"
                >
                  <input
                    type="radio"
                    name="inv_pay_status"
                    value="PAID"
                    v-model="invoiceForm.payment_status"
                    class="mt-0.5 text-emerald-600 focus:ring-emerald-500"
                  />
                  <div>
                    <div class="font-bold text-xs flex items-center gap-1.5">
                      <span>Lunas Langsung (Tunai / Kas Toko)</span>
                      <span class="text-[10px] px-1.5 py-0.2 rounded bg-emerald-200/80 text-emerald-900 font-extrabold">LUNAS</span>
                    </div>
                    <p class="text-[11px] text-slate-500 mt-0.5 leading-snug">
                      Tagihan langsung dibayar lunas seketika dari kas toko saat faktur diterbitkan.
                    </p>
                  </div>
                </label>
              </div>
            </div>

            <!-- Tabel Barang yang Ditagihkan -->
            <div>
              <div class="font-bold text-slate-800 uppercase tracking-wide mb-2">
                Barang yang Ditagihkan Berdasarkan GRN Ini:
              </div>
              <div class="border border-slate-200 rounded-xl overflow-hidden shadow-2xs">
                <table class="w-full text-left text-xs text-slate-700">
                  <thead class="bg-slate-100 text-[10px] font-bold text-slate-500 uppercase border-b border-slate-200">
                    <tr>
                      <th class="px-3 py-2">Produk</th>
                      <th class="px-3 py-2 text-right">Qty Diterima</th>
                      <th class="px-3 py-2 text-right">Harga Satuan Beli (Rp)</th>
                      <th class="px-3 py-2 text-right">Subtotal (Rp)</th>
                    </tr>
                  </thead>
                  <tbody class="divide-y divide-slate-100 font-mono">
                    <tr v-for="it in (selectedGrnForInvoice?.items || [])" :key="it.id">
                      <td class="px-3 py-2 font-sans font-bold text-slate-900">
                        {{ it.product_name }}
                        <span class="text-[10px] text-slate-400 font-mono ml-1">({{ it.product_sku }})</span>
                      </td>
                      <td class="px-3 py-2 text-right font-bold text-blue-900">
                        {{ formatQty(it.received_qty) }} {{ it.unit_code }}
                      </td>
                      <td class="px-3 py-2 text-right text-slate-600">
                        {{ formatCurrency(it.unit_price) }}
                      </td>
                      <td class="px-3 py-2 text-right font-black text-slate-900">
                        {{ formatCurrency(Number(it.received_qty) * Number(it.unit_price)) }}
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>

            <!-- Ringkasan Perhitungan Subtotal, Diskon, Pajak, Total -->
            <div class="flex justify-end">
              <div class="w-80 bg-blue-50/70 p-4 rounded-xl border border-blue-200 font-mono space-y-2 text-xs">
                <div class="flex justify-between text-slate-600">
                  <span>Subtotal Nilai Barang:</span>
                  <span class="font-bold text-slate-900">{{ formatRupiah(invoiceForm.subtotal) }}</span>
                </div>
                <div class="flex items-center justify-between gap-2 text-slate-600">
                  <span class="shrink-0">Potongan Diskon (Rp):</span>
                  <input
                    v-model.number="invoiceForm.discount_amount"
                    type="number"
                    min="0"
                    step="1000"
                    class="w-32 px-2 py-1 rounded border border-slate-300 bg-white text-right font-mono text-xs focus:ring-1 focus:ring-blue-500 font-bold"
                  />
                </div>
                <div class="flex items-center justify-between gap-2 text-slate-600">
                  <span class="shrink-0">PPN / Pajak Masukan (Rp):</span>
                  <input
                    v-model.number="invoiceForm.tax_amount"
                    type="number"
                    min="0"
                    step="1000"
                    class="w-32 px-2 py-1 rounded border border-slate-300 bg-white text-right font-mono text-xs focus:ring-1 focus:ring-blue-500 font-bold"
                  />
                </div>
                <div class="border-t border-blue-200 pt-2 flex justify-between text-slate-900 font-black text-sm">
                  <span class="font-sans">Total Tagihan Faktur:</span>
                  <span class="text-blue-900">{{ formatRupiah(invoiceTotalAmount) }}</span>
                </div>
              </div>
            </div>
          </div>

          <div class="px-6 py-3.5 bg-slate-50 border-t border-slate-200 flex items-center justify-end gap-2.5 shrink-0">
            <button
              type="button"
              @click="showInvoiceModal = false"
              :disabled="isSavingInvoice"
              class="px-4 py-2 rounded-lg border border-slate-300 bg-white hover:bg-slate-100 text-slate-700 font-bold text-xs transition cursor-pointer"
            >
              Batal
            </button>
            <button
              type="button"
              @click="handleSaveInvoice"
              :disabled="isSavingInvoice"
              class="px-5 py-2 rounded-lg bg-blue-600 hover:bg-blue-700 text-white font-bold text-xs flex items-center gap-2 shadow-xs transition active:scale-95 cursor-pointer disabled:opacity-50"
            >
              <RefreshCw v-if="isSavingInvoice" class="w-4 h-4 animate-spin" />
              <Check v-else class="w-4 h-4" />
              <span>Terbitkan Faktur Pembelian</span>
            </button>
          </div>
        </div>
      </div>

      <!-- MODAL: DETAIL PENERIMAAN BARANG GRN (showGrnDetailModal) -->
      <div
        v-if="showGrnDetailModal"
        class="fixed inset-0 z-50 bg-slate-900/60 backdrop-blur-xs flex items-center justify-center p-4"
        @click.self="showGrnDetailModal = false"
      >
        <div class="bg-white rounded-2xl shadow-2xl border border-slate-300 w-full max-w-3xl overflow-hidden flex flex-col max-h-[90vh] animate-in fade-in zoom-in-95 duration-150">
          <div class="px-6 py-4 bg-emerald-950 text-white flex items-center justify-between shrink-0">
            <div class="flex items-center gap-3">
              <div class="w-9 h-9 rounded-xl bg-emerald-500/20 border border-emerald-400/40 flex items-center justify-center">
                <Truck class="w-5 h-5 text-emerald-400" />
              </div>
              <div>
                <div class="text-xs font-bold text-emerald-300 uppercase tracking-wider">Dokumen Penerimaan Barang (GRN)</div>
                <div class="text-base font-black tracking-tight flex items-center gap-2">
                  <span>{{ selectedGrnDetail?.grn_number }}</span>
                  <span
                    v-if="selectedGrnDetail?.status === 'CANCELLED'"
                    class="text-[10px] bg-rose-500/20 text-rose-300 px-2 py-0.5 rounded border border-rose-400/30 font-bold"
                  >
                    DIBATALKAN
                  </span>
                  <span
                    v-else-if="selectedGrnDetail?.status === 'INVOICED'"
                    class="text-[10px] bg-emerald-500/20 text-emerald-300 px-2 py-0.5 rounded border border-emerald-400/30 font-bold"
                  >
                    SUDAH DIFAKTURKAN
                  </span>
                  <span
                    v-else
                    class="text-[10px] bg-amber-500/20 text-amber-300 px-2 py-0.5 rounded border border-amber-400/30 font-bold"
                  >
                    BELUM DIFAKTURKAN
                  </span>
                </div>
              </div>
            </div>
            <button
              type="button"
              @click="showGrnDetailModal = false"
              class="text-emerald-300 hover:text-white p-1 rounded-lg hover:bg-emerald-900 transition cursor-pointer"
            >
              <X class="w-5 h-5" />
            </button>
          </div>

          <div class="p-6 overflow-y-auto space-y-5 flex-1 min-h-0 text-xs" v-if="selectedGrnDetail">
            <div class="grid grid-cols-2 sm:grid-cols-4 gap-3 bg-slate-50 p-3.5 rounded-xl border border-slate-200">
              <div>
                <div class="text-[10px] font-bold text-slate-400 uppercase">No. Surat Jalan</div>
                <div class="font-mono font-bold text-slate-800 mt-0.5">{{ selectedGrnDetail.delivery_order_number }}</div>
              </div>
              <div>
                <div class="text-[10px] font-bold text-slate-400 uppercase">Dokumen PO</div>
                <div class="font-mono font-bold text-indigo-700 mt-0.5">{{ selectedGrnDetail.po_number || '-' }}</div>
              </div>
              <div>
                <div class="text-[10px] font-bold text-slate-400 uppercase">Pemasok</div>
                <div class="font-bold text-slate-800 mt-0.5">{{ selectedGrnDetail.supplier_name }}</div>
              </div>
              <div>
                <div class="text-[10px] font-bold text-slate-400 uppercase">Tanggal Terima</div>
                <div class="font-bold text-slate-800 mt-0.5">{{ formatDate(selectedGrnDetail.receive_date) }}</div>
              </div>
            </div>

            <div>
              <div class="text-xs font-bold text-slate-800 mb-2 uppercase tracking-wide">
                Daftar Fisik Barang yang Diterima:
              </div>
              <div class="border border-slate-200 rounded-xl overflow-hidden">
                <table class="w-full text-left text-xs text-slate-700">
                  <thead class="bg-slate-100 text-[10px] font-bold text-slate-500 uppercase border-b border-slate-200">
                    <tr>
                      <th class="px-3 py-2">Produk</th>
                      <th class="px-3 py-2 text-right">Qty Diterima</th>
                      <th class="px-3 py-2 text-right">Harga Satuan Beli</th>
                      <th class="px-3 py-2 text-right">Subtotal</th>
                    </tr>
                  </thead>
                  <tbody class="divide-y divide-slate-100 font-mono">
                    <tr v-for="it in (selectedGrnDetail.items || [])" :key="it.id">
                      <td class="px-3 py-2.5 font-sans font-bold text-slate-900">
                        {{ it.product_name }}
                        <div class="text-[10px] text-slate-400 font-mono">{{ it.product_sku }}</div>
                      </td>
                      <td class="px-3 py-2.5 text-right font-bold text-emerald-800">
                        {{ formatQty(it.received_qty) }} {{ it.unit_code }}
                      </td>
                      <td class="px-3 py-2.5 text-right text-slate-600">
                        {{ formatCurrency(it.unit_price) }}
                      </td>
                      <td class="px-3 py-2.5 text-right font-black text-slate-900">
                        {{ formatCurrency(Number(it.received_qty) * Number(it.unit_price)) }}
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>

            <div class="flex justify-end">
              <div class="w-72 bg-emerald-50/60 p-3.5 rounded-xl border border-emerald-200 font-mono space-y-1">
                <div class="flex justify-between text-slate-700 text-xs">
                  <span>Total Nilai Barang:</span>
                  <span class="font-black text-emerald-950 text-sm">{{ formatRupiah(selectedGrnDetail.total_amount) }}</span>
                </div>
              </div>
            </div>
          </div>

          <div class="px-6 py-3.5 bg-slate-50 border-t border-slate-200 flex items-center justify-between shrink-0">
            <div class="flex items-center gap-2">
              <button
                v-if="selectedGrnDetail?.status === 'UNINVOICED'"
                type="button"
                @click="showGrnDetailModal = false; openCreateInvoiceModal(selectedGrnDetail)"
                class="px-4 py-2 rounded-lg bg-blue-600 hover:bg-blue-700 text-white font-bold text-xs flex items-center gap-2 shadow-xs transition active:scale-95 cursor-pointer"
              >
                <FileText class="w-4 h-4" />
                <span>Terbitkan Faktur Pembelian</span>
              </button>

              <button
                v-if="selectedGrnDetail?.status === 'UNINVOICED' && authStore.hasPermission(['purchasing:void', 'purchasing:write'])"
                type="button"
                @click="handleCancelGrn(selectedGrnDetail)"
                class="px-3 py-2 rounded-lg border border-rose-300 bg-white hover:bg-rose-50 text-rose-700 font-bold text-xs flex items-center gap-1.5 transition active:scale-95 cursor-pointer"
              >
                <Trash2 class="w-3.5 h-3.5" />
                <span>Batalkan Penerimaan Ini</span>
              </button>
            </div>

            <button
              type="button"
              @click="showGrnDetailModal = false"
              class="px-4 py-2 rounded-lg border border-slate-300 bg-white hover:bg-slate-100 text-slate-700 font-bold text-xs transition cursor-pointer"
            >
              Tutup
            </button>
          </div>
        </div>
      </div>

      <!-- 5. Modal Detail Faktur GRN -->
      <div
        v-if="showDetailModal"
        class="fixed inset-0 z-50 bg-slate-900/60 backdrop-blur-xs flex items-center justify-center p-4"
        @click.self="showDetailModal = false"
      >
        <div class="bg-white rounded-2xl shadow-2xl border border-slate-300 w-full max-w-3xl overflow-hidden flex flex-col max-h-[90vh] animate-in fade-in zoom-in-95 duration-150">
          <div class="px-6 py-4 bg-slate-900 text-white flex items-center justify-between shrink-0">
            <div class="flex items-center gap-3">
              <div class="w-9 h-9 rounded-xl bg-blue-500/20 border border-blue-400/40 flex items-center justify-center">
                <FileText class="w-5 h-5 text-blue-400" />
              </div>
              <div>
                <div class="text-xs font-bold text-slate-400 uppercase tracking-wider">Detail Faktur Penerimaan (GRN)</div>
                <div class="text-base font-black tracking-tight flex items-center gap-2">
                  <span>{{ selectedInvoice?.invoice_number }}</span>
                  <span
                    v-if="selectedInvoice?.payment_status === 'CANCELLED'"
                    class="text-[10px] bg-rose-500/20 text-rose-300 px-2 py-0.5 rounded border border-rose-400/30 font-bold"
                  >
                    DIBATALKAN
                  </span>
                  <span
                    v-else-if="selectedInvoice?.payment_status === 'PAID'"
                    class="text-[10px] bg-emerald-500/20 text-emerald-300 px-2 py-0.5 rounded border border-emerald-400/30 font-bold"
                  >
                    LUNAS
                  </span>
                  <span
                    v-else
                    class="text-[10px] bg-amber-500/20 text-amber-300 px-2 py-0.5 rounded border border-amber-400/30 font-bold"
                  >
                    BELUM LUNAS
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

          <div class="p-6 overflow-y-auto space-y-5 flex-1 min-h-0 text-xs">
            <div v-if="isLoadingDetail" class="py-12 text-center text-slate-400">
              <RefreshCw class="w-8 h-8 mx-auto mb-2 text-blue-500 animate-spin" />
              <p class="font-bold">Memuat rincian faktur...</p>
            </div>

            <div v-else-if="selectedInvoice" class="space-y-5">
              <div class="grid grid-cols-2 sm:grid-cols-6 gap-3 bg-slate-50 p-3.5 rounded-xl border border-slate-200">
                <div>
                  <div class="text-[10px] font-bold text-slate-400 uppercase">Outlet Pemesan</div>
                  <div class="font-bold text-blue-800 mt-0.5">{{ selectedInvoice.outlet_name || '-' }}</div>
                </div>
                <div>
                  <div class="text-[10px] font-bold text-slate-400 uppercase">Pemasok (Supplier)</div>
                  <div class="font-bold text-slate-800 mt-0.5">{{ selectedInvoice.supplier_name }}</div>
                </div>
                <div>
                  <div class="text-[10px] font-bold text-slate-400 uppercase">Faktur / SJ Supplier</div>
                  <div class="font-mono font-bold text-slate-800 mt-0.5">{{ selectedInvoice.supplier_invoice_number }}</div>
                </div>
                <div>
                  <div class="text-[10px] font-bold text-slate-400 uppercase">Dokumen PO</div>
                  <div class="font-mono font-bold text-indigo-700 mt-0.5">
                    {{ selectedInvoice.po_number || '-' }}
                  </div>
                </div>
                <div>
                  <div class="text-[10px] font-bold text-slate-400 uppercase">Tanggal Faktur</div>
                  <div class="font-bold text-slate-800 mt-0.5">{{ formatDate(selectedInvoice.invoice_date) }}</div>
                </div>
                <div>
                  <div class="text-[10px] font-bold text-slate-400 uppercase">Jatuh Tempo</div>
                  <div class="font-bold text-rose-700 mt-0.5">{{ formatDate(selectedInvoice.due_date) }}</div>
                </div>
              </div>

              <div>
                <div class="text-xs font-bold text-slate-800 mb-2 uppercase tracking-wide">
                  Item Barang Faktur Pembelian (Perbandingan Pesanan vs Realisasi Diterima)
                </div>
                <div class="border border-slate-200 rounded-xl overflow-hidden">
                  <table class="w-full text-left text-xs text-slate-700">
                    <thead class="bg-slate-100 text-[10px] font-bold text-slate-500 uppercase border-b border-slate-200">
                      <tr>
                        <th class="px-3 py-2">Produk</th>
                        <th class="px-3 py-2 text-right">Qty Dipesan</th>
                        <th class="px-3 py-2 text-right text-blue-800">Qty Diterima</th>
                        <th class="px-3 py-2 text-center">Selisih Realisasi</th>
                        <th class="px-3 py-2 text-right">Harga Satuan Beli</th>
                        <th class="px-3 py-2 text-right">Diskon</th>
                        <th class="px-3 py-2 text-right">Subtotal</th>
                      </tr>
                    </thead>
                    <tbody class="divide-y divide-slate-100 font-mono">
                      <tr v-for="(it, idx) in selectedInvoice.items" :key="idx" class="hover:bg-slate-50/80">
                        <td class="px-3 py-2.5 font-sans font-bold text-slate-900">
                          {{ it.product_name }}
                          <div class="text-[10px] font-normal text-slate-400 font-mono">{{ it.product_sku }}</div>
                        </td>
                        <td class="px-3 py-2.5 text-right font-medium text-slate-600">
                          {{ formatQty(it.ordered_qty || it.received_qty) }} {{ it.unit_code }}
                        </td>
                        <td class="px-3 py-2.5 text-right font-bold text-blue-900">
                          {{ formatQty(it.received_qty) }} {{ it.unit_code }}
                        </td>
                        <td class="px-3 py-2.5 text-center font-sans">
                          <span
                            v-if="Number(it.received_qty) === Number(it.ordered_qty || it.received_qty)"
                            class="px-2 py-0.5 rounded text-[10px] font-bold bg-emerald-100 text-emerald-800 border border-emerald-300"
                          >
                            Sesuai (100%)
                          </span>
                          <span
                            v-else-if="Number(it.received_qty) < Number(it.ordered_qty)"
                            class="px-2 py-0.5 rounded text-[10px] font-bold bg-amber-100 text-amber-900 border border-amber-300"
                          >
                            Kurang {{ formatQty(Number(it.ordered_qty) - Number(it.received_qty)) }}
                          </span>
                          <span
                            v-else
                            class="px-2 py-0.5 rounded text-[10px] font-bold bg-blue-100 text-blue-900 border border-blue-300"
                          >
                            Lebih +{{ formatQty(Number(it.received_qty) - Number(it.ordered_qty)) }}
                          </span>
                        </td>
                        <td class="px-3 py-2.5 text-right text-slate-600">
                          {{ formatCurrency(it.unit_price) }}
                        </td>
                        <td class="px-3 py-2.5 text-right text-rose-600">
                          {{ Number(it.discount_amount) > 0 ? `-${formatCurrency(it.discount_amount)}` : '-' }}
                        </td>
                        <td class="px-3 py-2.5 text-right font-black text-slate-900">
                          {{ formatCurrency(it.subtotal) }}
                        </td>
                      </tr>
                    </tbody>
                  </table>
                </div>
              </div>

              <div class="flex justify-end">
                <div class="w-80 bg-slate-50 p-4 rounded-xl border border-slate-200 space-y-2 font-mono">
                  <div class="flex justify-between text-slate-600">
                    <span>Subtotal:</span>
                    <span>{{ formatRupiah(selectedInvoice.subtotal) }}</span>
                  </div>
                  <div class="flex justify-between text-slate-600" v-if="Number(selectedInvoice.discount_amount) > 0">
                    <span>Diskon:</span>
                    <span>-{{ formatRupiah(selectedInvoice.discount_amount) }}</span>
                  </div>
                  <div class="flex justify-between text-slate-600" v-if="Number(selectedInvoice.tax_amount) > 0">
                    <span>PPN:</span>
                    <span>{{ formatRupiah(selectedInvoice.tax_amount) }}</span>
                  </div>
                  <div class="h-px bg-slate-300 my-1"></div>
                  <div class="flex justify-between text-slate-900 font-extrabold text-sm">
                    <span class="font-sans">Total Tagihan:</span>
                    <span>{{ formatRupiah(selectedInvoice.total_amount) }}</span>
                  </div>
                  <div class="flex justify-between text-emerald-700 font-bold">
                    <span>Sudah Dibayar:</span>
                    <span>{{ formatRupiah(selectedInvoice.paid_amount) }}</span>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <div class="px-6 py-3.5 bg-slate-50 border-t border-slate-200 flex items-center justify-between shrink-0">
            <div class="flex items-center gap-2">
              <button
                v-if="selectedInvoice && selectedInvoice.payment_status === 'UNPAID' && authStore.hasPermission(['purchasing:pay', 'purchasing:write'])"
                type="button"
                @click="handlePayInvoice(selectedInvoice)"
                class="px-4 py-2 rounded-lg bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-xs flex items-center gap-2 shadow-xs transition active:scale-95 cursor-pointer"
              >
                <Check class="w-4 h-4" />
                <span>Tandai Lunas</span>
              </button>

              <button
                v-if="selectedInvoice && selectedInvoice.payment_status !== 'CANCELLED' && authStore.hasPermission(['purchasing:void', 'purchasing:write'])"
                type="button"
                @click="showDetailModal = false; openCreateReturnModal({ invoice: selectedInvoice })"
                class="px-3 py-2 rounded-lg border border-rose-300 bg-white hover:bg-rose-50 text-rose-700 font-bold text-xs flex items-center gap-1.5 transition active:scale-95 cursor-pointer"
              >
                <RotateCcw class="w-3.5 h-3.5" />
                <span>Retur Barang</span>
              </button>

              <button
                v-if="selectedInvoice && selectedInvoice.payment_status === 'UNPAID' && authStore.hasPermission(['purchasing:void', 'purchasing:write'])"
                type="button"
                @click="handleCancelInvoice(selectedInvoice)"
                class="px-3 py-2 rounded-lg border border-rose-300 bg-white hover:bg-rose-50 text-rose-700 font-bold text-xs flex items-center gap-1.5 transition active:scale-95 cursor-pointer"
              >
                <Trash2 class="w-3.5 h-3.5" />
                <span>Batalkan Faktur Ini</span>
              </button>
            </div>

            <button
              type="button"
              @click="showDetailModal = false"
              class="px-4 py-2 rounded-lg border border-slate-300 bg-white hover:bg-slate-100 text-slate-700 font-bold text-xs transition cursor-pointer"
            >
              Tutup
            </button>
          </div>
        </div>
      </div>

      <!-- 6. Modal Buat Dokumen Purchase Order (PO) Baru -->
      <div
        v-if="showPoModal"
        class="fixed inset-0 z-50 bg-slate-900/60 backdrop-blur-xs flex items-center justify-center p-4"
        @click.self="showPoModal = false"
      >
        <div class="bg-white rounded-2xl shadow-2xl border border-slate-300 w-full max-w-4xl overflow-hidden flex flex-col max-h-[92vh] animate-in fade-in zoom-in-95 duration-150">
          <!-- Header -->
          <div class="px-6 py-4 bg-indigo-950 text-white flex items-center justify-between shrink-0">
            <div class="flex items-center gap-3">
              <div class="w-9 h-9 rounded-xl bg-indigo-500/20 border border-indigo-400/40 flex items-center justify-center">
                <Clock class="w-5 h-5 text-indigo-400" />
              </div>
              <div>
                <div class="text-xs font-bold text-indigo-300 uppercase tracking-wider">Formulir Pemesanan Barang (PO)</div>
                <div class="text-base font-black tracking-tight">Buat Dokumen Purchase Order Baru</div>
              </div>
            </div>
            <button
              type="button"
              @click="showPoModal = false"
              class="text-indigo-300 hover:text-white p-1 rounded-lg hover:bg-indigo-900 transition cursor-pointer"
            >
              <X class="w-5 h-5" />
            </button>
          </div>

          <!-- Body -->
          <div class="p-6 overflow-y-auto space-y-5 flex-1 min-h-0 text-xs">
            <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-3.5 bg-indigo-50/50 p-4 rounded-xl border border-indigo-200">
              <div>
                <label class="block font-bold text-slate-700 mb-1">Outlet Pemesan *</label>
                <SearchableSelect
                  v-model="poForm.outlet_id"
                  api-endpoint="/outlets"
                  placeholder="-- Pilih Outlet Pemesan --"
                  search-placeholder="Cari outlet..."
                  :allow-clear="false"
                />
              </div>

              <div>
                <label class="block font-bold text-slate-700 mb-1">Pemasok / Supplier *</label>
                <SearchableSelect
                  v-model="poForm.supplier_id"
                  api-endpoint="/suppliers"
                  placeholder="-- Pilih Supplier --"
                  search-placeholder="Cari pemasok..."
                />
              </div>

              <div>
                <label class="block font-bold text-slate-700 mb-1">Tanggal Dokumen PO *</label>
                <input
                  v-model="poForm.order_date"
                  type="date"
                  class="w-full px-3 py-2 rounded-lg border border-slate-300 bg-white font-sans text-xs focus:ring-2 focus:ring-indigo-500 focus:outline-none"
                />
              </div>

              <div>
                <label class="block font-bold text-slate-700 mb-1">Estimasi Barang Tiba</label>
                <input
                  v-model="poForm.expected_delivery"
                  type="date"
                  class="w-full px-3 py-2 rounded-lg border border-slate-300 bg-white font-sans text-xs focus:ring-2 focus:ring-indigo-500 focus:outline-none"
                />
              </div>

              <div class="sm:col-span-2 lg:col-span-4">
                <label class="block font-bold text-slate-700 mb-1">Catatan / Instruksi Pesanan ke Vendor</label>
                <input
                  v-model="poForm.notes"
                  type="text"
                  placeholder="Catatan jadwal pengiriman, ekspedisi atau spesifikasi produk..."
                  class="w-full px-3 py-2 rounded-lg border border-slate-300 bg-white font-sans text-xs focus:ring-2 focus:ring-indigo-500 focus:outline-none"
                />
              </div>
            </div>

            <!-- Items Table -->
            <div>
              <div class="flex items-center justify-between mb-2">
                <div class="font-bold text-slate-800 uppercase tracking-wide">
                  Daftar Barang yang Dipesan
                </div>
                <div class="flex items-center gap-2">
                  <button
                    type="button"
                    @click="pullReorderSuggestions"
                    :disabled="isLoadingReorder"
                    class="px-2.5 py-1 rounded-lg bg-amber-50 hover:bg-amber-100 text-amber-800 border border-amber-300 font-bold text-xs flex items-center gap-1.5 transition cursor-pointer"
                    title="Tarik otomatis produk yang stoknya saat ini di bawah batas minimum (Auto-Reorder)"
                  >
                    <Icon v-if="isLoadingReorder" icon="fluent:spinner-ios-20-regular" class="animate-spin text-sm" />
                    <Icon v-else icon="fluent:flash-20-filled" class="text-amber-600 text-sm" />
                    <span>Tarik Stok Menipis (Auto Reorder)</span>
                  </button>

                  <button
                    type="button"
                    @click="addPoItem"
                    class="px-2.5 py-1 rounded-lg bg-indigo-50 hover:bg-indigo-100 text-indigo-700 border border-indigo-200 font-bold text-xs flex items-center gap-1 transition cursor-pointer"
                  >
                    <Plus class="w-3.5 h-3.5" />
                    <span>Tambah Baris Barang</span>
                  </button>
                </div>
              </div>

              <div class="border border-slate-200 rounded-xl overflow-visible">
                <table class="w-full text-left text-xs text-slate-700">
                  <thead class="bg-slate-100 text-[10px] font-bold text-slate-500 uppercase border-b border-slate-200 rounded-t-xl">
                    <tr>
                      <th class="px-3 py-2">Produk</th>
                      <th class="px-3 py-2 w-36 text-right">Qty Pesanan</th>
                      <th class="px-3 py-2 w-44 text-right">Estimasi Harga Satuan (Rp)</th>
                      <th class="px-3 py-2 text-right w-40">Subtotal</th>
                      <th class="px-3 py-2 text-center w-12">Hapus</th>
                    </tr>
                  </thead>
                  <tbody class="divide-y divide-slate-100">
                    <tr v-if="poForm.items.length === 0">
                      <td colspan="5" class="py-10 text-center text-slate-400 bg-slate-50/50">
                        <div class="flex flex-col items-center justify-center gap-2">
                          <div class="w-10 h-10 rounded-full bg-slate-100 flex items-center justify-center text-slate-400">
                            <Package class="w-5 h-5" />
                          </div>
                          <span class="font-bold text-slate-600 text-xs">Belum ada barang dalam pesanan pembelian (PO)</span>
                          <span class="text-[11px] text-slate-400">Klik tombol <strong>"Tambah Baris Barang"</strong> di atas untuk menambahkan produk.</span>
                        </div>
                      </td>
                    </tr>
                    <tr v-for="(it, idx) in poForm.items" :key="idx" class="hover:bg-slate-50/80">
                      <!-- Product Selector -->
                      <td class="px-3 py-2 min-w-[280px]">
                        <SearchableSelect
                          :model-value="it.product_id"
                          api-endpoint="/products"
                          :extra-params="poForm.outlet_id ? { outlet_id: poForm.outlet_id } : {}"
                          secondary-key="sku"
                          placeholder="-- Pilih Produk (Cari Nama/SKU/Barcode) --"
                          search-placeholder="Ketik nama produk, SKU, barcode..."
                          :allow-clear="false"
                          @change="(val: any, prod: any) => onPoProductSelected(idx, val, prod)"
                        />
                        <!-- Info Satuan & Barcode -->
                        <div v-if="it.product_id" class="flex items-center gap-2 mt-1">
                          <span class="font-mono text-[10px] text-slate-500 bg-slate-100 px-1.5 py-0.5 rounded border border-slate-200">
                            Barcode: {{ (it as any)._product?.barcode || products.find((p) => p.id === it.product_id)?.barcode || '-' }}
                          </span>
                          <span v-if="(it as any)._product?.unit_code || products.find((p) => p.id === it.product_id)?.unit_code" class="text-[10px] text-indigo-700 bg-indigo-50 px-1.5 py-0.5 rounded border border-indigo-200 font-bold">
                            Satuan: {{ (it as any)._product?.unit_code || products.find((p) => p.id === it.product_id)?.unit_code }}
                          </span>
                        </div>
                      </td>

                      <!-- Qty -->
                      <td class="px-3 py-2">
                        <div class="flex items-center gap-1.5">
                          <input
                            v-model.number="it.ordered_qty"
                            type="number"
                            step="any"
                            min="0.001"
                            class="w-full px-2 py-1.5 rounded-lg border border-slate-300 text-right font-mono text-xs focus:ring-2 focus:ring-indigo-500 focus:outline-none font-bold text-slate-900"
                          />
                          <span v-if="(it as any)._product?.unit_code || products.find((p) => p.id === it.product_id)?.unit_code" class="text-[11px] text-slate-500 font-mono font-bold shrink-0">
                            {{ (it as any)._product?.unit_code || products.find((p) => p.id === it.product_id)?.unit_code }}
                          </span>
                        </div>
                      </td>

                      <!-- Unit Price -->
                      <td class="px-3 py-2">
                        <input
                          v-model.number="it.unit_price"
                          type="number"
                          step="100"
                          min="0"
                          class="w-full px-2 py-1.5 rounded-lg border border-slate-300 text-right font-mono text-xs focus:ring-2 focus:ring-indigo-500 focus:outline-none"
                        />
                      </td>

                      <!-- Subtotal -->
                      <td class="px-3 py-2 text-right font-mono font-black text-slate-900 whitespace-nowrap">
                        {{ formatRupiah((Number(it.ordered_qty) || 0) * (Number(it.unit_price) || 0)) }}
                      </td>

                      <!-- Remove -->
                      <td class="px-3 py-2 text-center">
                        <button
                          type="button"
                          @click="removePoItem(idx)"
                          class="p-1 rounded text-rose-500 hover:bg-rose-50 cursor-pointer transition"
                          title="Hapus baris barang ini"
                        >
                          <Trash2 class="w-3.5 h-3.5" />
                        </button>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>

            <!-- Total Ringkasan PO -->
            <div class="flex justify-end">
              <div class="w-72 bg-indigo-50/60 p-4 rounded-xl border border-indigo-200 font-mono space-y-1.5">
                <div class="flex justify-between text-slate-600 text-xs">
                  <span>Total Anggaran PO:</span>
                  <span class="font-black text-indigo-950 text-sm">{{ formatRupiah(poSubtotal) }}</span>
                </div>
                <div class="text-[10px] text-indigo-700 font-sans italic">
                  * PO ini dapat dirujuk saat penerimaan fisik faktur pembelian (GRN)
                </div>
              </div>
            </div>
          </div>

          <!-- Footer -->
          <div class="px-6 py-3.5 bg-slate-50 border-t border-slate-200 flex items-center justify-end gap-2.5 shrink-0">
            <button
              type="button"
              @click="showPoModal = false"
              :disabled="isSavingPo"
              class="px-4 py-2 rounded-lg border border-slate-300 bg-white hover:bg-slate-100 text-slate-700 font-bold text-xs transition cursor-pointer"
            >
              Batal
            </button>
            <button
              type="button"
              @click="handleSavePo"
              :disabled="isSavingPo"
              class="px-5 py-2 rounded-lg bg-indigo-600 hover:bg-indigo-700 text-white font-bold text-xs flex items-center gap-2 shadow-xs transition active:scale-95 cursor-pointer disabled:opacity-50"
            >
              <RefreshCw v-if="isSavingPo" class="w-4 h-4 animate-spin" />
              <Check v-else class="w-4 h-4" />
              <span>Simpan Purchase Order (PO)</span>
            </button>
          </div>
        </div>
      </div>

      <!-- 7. Modal Detail Purchase Order (PO) -->
      <div
        v-if="showPoDetailModal"
        class="fixed inset-0 z-50 bg-slate-900/60 backdrop-blur-xs flex items-center justify-center p-4"
        @click.self="showPoDetailModal = false"
      >
        <div class="bg-white rounded-2xl shadow-2xl border border-slate-300 w-full max-w-4xl overflow-hidden flex flex-col max-h-[90vh] animate-in fade-in zoom-in-95 duration-150">
          <div class="px-6 py-4 bg-indigo-950 text-white flex items-center justify-between shrink-0">
            <div class="flex items-center gap-3">
              <div class="w-9 h-9 rounded-xl bg-indigo-500/20 border border-indigo-400/40 flex items-center justify-center">
                <Clock class="w-5 h-5 text-indigo-400" />
              </div>
              <div>
                <div class="text-xs font-bold text-indigo-300 uppercase tracking-wider">Rincian Purchase Order (PO)</div>
                <div class="text-base font-black tracking-tight flex items-center gap-2">
                  <span>{{ selectedPo?.po_number }}</span>
                  <span
                    v-if="selectedPo?.status === 'COMPLETED'"
                    class="text-[10px] bg-emerald-500/20 text-emerald-300 px-2 py-0.5 rounded border border-emerald-400/30 font-bold"
                  >
                    SELESAI (LENGKAP)
                  </span>
                  <span
                    v-else-if="selectedPo?.status === 'PARTIAL'"
                    class="text-[10px] bg-blue-500/20 text-blue-300 px-2 py-0.5 rounded border border-blue-400/30 font-bold"
                  >
                    DITERIMA SEBAGIAN
                  </span>
                  <span
                    v-else-if="selectedPo?.status === 'CANCELLED'"
                    class="text-[10px] bg-rose-500/20 text-rose-300 px-2 py-0.5 rounded border border-rose-400/30 font-bold"
                  >
                    DIBATALKAN
                  </span>
                  <span
                    v-else
                    class="text-[10px] bg-amber-500/20 text-amber-300 px-2 py-0.5 rounded border border-amber-400/30 font-bold"
                  >
                    MENUNGGU PENGIRIMAN
                  </span>
                </div>
              </div>
            </div>
            <button
              type="button"
              @click="showPoDetailModal = false"
              class="text-indigo-300 hover:text-white p-1 rounded-lg hover:bg-indigo-900 transition cursor-pointer"
            >
              <X class="w-5 h-5" />
            </button>
          </div>

          <div class="p-6 overflow-y-auto space-y-5 flex-1 min-h-0 text-xs">
            <div v-if="isLoadingPoDetail" class="py-12 text-center text-slate-400">
              <RefreshCw class="w-8 h-8 mx-auto mb-2 text-indigo-500 animate-spin" />
              <p class="font-bold">Memuat rincian pesanan PO...</p>
            </div>

            <div v-else-if="selectedPo" class="space-y-5">
              <div class="grid grid-cols-2 sm:grid-cols-5 gap-3 bg-slate-50 p-3.5 rounded-xl border border-slate-200">
                <div>
                  <div class="text-[10px] font-bold text-slate-400 uppercase">Outlet Pemesan</div>
                  <div class="font-bold text-indigo-800 mt-0.5">{{ selectedPo.outlet_name || '-' }}</div>
                </div>
                <div>
                  <div class="text-[10px] font-bold text-slate-400 uppercase">Pemasok (Supplier)</div>
                  <div class="font-bold text-slate-800 mt-0.5">{{ selectedPo.supplier_name }}</div>
                </div>
                <div>
                  <div class="text-[10px] font-bold text-slate-400 uppercase">Tanggal Order</div>
                  <div class="font-bold text-slate-800 mt-0.5">{{ formatDate(selectedPo.order_date) }}</div>
                </div>
                <div>
                  <div class="text-[10px] font-bold text-slate-400 uppercase">Estimasi Tiba</div>
                  <div class="font-bold text-slate-800 mt-0.5">
                    {{ selectedPo.expected_delivery ? formatDate(selectedPo.expected_delivery) : '-' }}
                  </div>
                </div>
                <div>
                  <div class="text-[10px] font-bold text-slate-400 uppercase">Status Pemenuhan</div>
                  <div class="font-bold text-slate-900 mt-0.5">{{ selectedPo.status }}</div>
                </div>
              </div>

              <div>
                <div class="text-xs font-bold text-slate-800 mb-2 uppercase tracking-wide">
                  Item Barang Dipesan & Realisasi Penerimaan
                </div>
                <div class="border border-slate-200 rounded-xl overflow-hidden">
                  <table class="w-full text-left text-xs text-slate-700">
                    <thead class="bg-slate-100 text-[10px] font-bold text-slate-500 uppercase border-b border-slate-200">
                      <tr>
                        <th class="px-3 py-2">Produk</th>
                        <th class="px-3 py-2 text-right">Qty Dipesan</th>
                        <th class="px-3 py-2 text-right text-emerald-700">Telah Diterima</th>
                        <th class="px-3 py-2 text-right text-amber-700">Sisa Belum Datang</th>
                        <th class="px-3 py-2 text-right">Harga Beli Satuan</th>
                        <th class="px-3 py-2 text-right">Total Anggaran</th>
                      </tr>
                    </thead>
                    <tbody class="divide-y divide-slate-100 font-mono">
                      <tr v-for="(it, idx) in selectedPo.items" :key="idx" class="hover:bg-slate-50/80">
                        <td class="px-3 py-2.5 font-sans font-bold text-slate-900">
                          {{ it.product_name }}
                          <div class="text-[10px] font-normal text-slate-400 font-mono">{{ it.product_sku }}</div>
                        </td>
                        <td class="px-3 py-2.5 text-right font-bold text-slate-800">
                          {{ formatQty(it.ordered_qty) }} {{ it.unit_code }}
                        </td>
                        <td class="px-3 py-2.5 text-right font-black text-emerald-700">
                          {{ formatQty(it.received_qty) }} {{ it.unit_code }}
                        </td>
                        <td class="px-3 py-2.5 text-right font-black text-amber-700">
                          {{ formatQty(it.remaining_qty) }} {{ it.unit_code }}
                        </td>
                        <td class="px-3 py-2.5 text-right text-slate-600">
                          {{ formatCurrency(it.unit_price) }}
                        </td>
                        <td class="px-3 py-2.5 text-right font-black text-slate-900">
                          {{ formatCurrency(Number(it.ordered_qty) * Number(it.unit_price)) }}
                        </td>
                      </tr>
                    </tbody>
                  </table>
                </div>
              </div>

              <div class="flex justify-end">
                <div class="w-80 bg-indigo-50/60 p-4 rounded-xl border border-indigo-200 space-y-2 font-mono">
                  <div class="flex justify-between text-slate-900 font-extrabold text-sm">
                    <span class="font-sans">Total Anggaran PO:</span>
                    <span>{{ formatRupiah(selectedPo.total_amount) }}</span>
                  </div>
                  <div class="text-[11px] text-slate-500 font-sans" v-if="selectedPo.notes">
                    <strong>Catatan:</strong> {{ selectedPo.notes }}
                  </div>
                </div>
              </div>
            </div>
          </div>

          <div class="px-6 py-3.5 bg-slate-50 border-t border-slate-200 flex items-center justify-between shrink-0">
            <div class="flex items-center gap-2">
              <button
                v-if="selectedPo && selectedPo.status !== 'COMPLETED' && selectedPo.status !== 'CANCELLED'"
                type="button"
                @click="openGrnFromPo(selectedPo)"
                class="px-4 py-2 rounded-lg bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-xs flex items-center gap-2 shadow-xs transition active:scale-95 cursor-pointer"
              >
                <Truck class="w-4 h-4" />
                <span>Terima Barang Ini (Buat Faktur GRN)</span>
              </button>

              <button
                v-if="selectedPo && (selectedPo.status === 'PENDING' || selectedPo.status === 'ORDERED') && authStore.hasPermission(['purchasing:void', 'purchasing:write'])"
                type="button"
                @click="handleCancelPo(selectedPo)"
                class="px-3 py-2 rounded-lg border border-rose-300 bg-white hover:bg-rose-50 text-rose-600 font-bold text-xs flex items-center gap-1.5 transition cursor-pointer"
              >
                <X class="w-4 h-4" />
                <span>Batalkan Dokumen PO</span>
              </button>
            </div>

            <button
              type="button"
              @click="showPoDetailModal = false"
              class="px-4 py-2 rounded-lg border border-slate-300 bg-white hover:bg-slate-100 text-slate-700 font-bold text-xs transition cursor-pointer"
            >
              Tutup
            </button>
          </div>
        </div>
      </div>

      <!-- 8. Modal Form Supplier (Tambah / Edit) -->
      <div
        v-if="showSupplierModal"
        class="fixed inset-0 z-50 bg-slate-900/60 backdrop-blur-xs flex items-center justify-center p-4"
        @click.self="showSupplierModal = false"
      >
        <div class="bg-white rounded-2xl shadow-2xl border border-slate-300 w-full max-w-md overflow-hidden animate-in fade-in zoom-in-95 duration-150">
          <div class="p-6">
            <div class="flex items-center gap-3.5 mb-4">
              <div class="w-11 h-11 rounded-xl bg-emerald-50 border border-emerald-300 flex items-center justify-center shrink-0">
                <Building2 class="w-6 h-6 text-emerald-600" />
              </div>
              <div>
                <h3 class="text-base font-black text-slate-900">
                  {{ isEditingSupplier ? 'Edit Data Pemasok' : 'Tambah Pemasok Baru' }}
                </h3>
                <p class="text-xs text-slate-500">Katalog vendor distributor ritel</p>
              </div>
            </div>

            <div class="space-y-3 text-xs">
              <div>
                <label class="block font-bold text-slate-700 mb-1">Kode Pemasok *</label>
                <input
                  v-model="supplierForm.code"
                  type="text"
                  class="w-full px-3 py-2 rounded-lg border border-slate-300 font-mono text-xs focus:ring-2 focus:ring-emerald-500 focus:outline-none"
                  placeholder="SUP-001"
                />
              </div>

              <div>
                <label class="block font-bold text-slate-700 mb-1">Nama Pemasok / Perusahaan *</label>
                <input
                  v-model="supplierForm.name"
                  type="text"
                  class="w-full px-3 py-2 rounded-lg border border-slate-300 text-xs focus:ring-2 focus:ring-emerald-500 focus:outline-none"
                  placeholder="PT Indomarco / Distributor..."
                />
              </div>

              <div class="grid grid-cols-2 gap-2.5">
                <div>
                  <label class="block font-bold text-slate-700 mb-1">Kontak Person</label>
                  <input
                    v-model="supplierForm.contact_person"
                    type="text"
                    class="w-full px-3 py-2 rounded-lg border border-slate-300 text-xs focus:ring-2 focus:ring-emerald-500 focus:outline-none"
                    placeholder="Nama PIC / Sales"
                  />
                </div>
                <div>
                  <label class="block font-bold text-slate-700 mb-1">No. Telepon / WA</label>
                  <input
                    v-model="supplierForm.phone"
                    type="text"
                    class="w-full px-3 py-2 rounded-lg border border-slate-300 font-mono text-xs focus:ring-2 focus:ring-emerald-500 focus:outline-none"
                    placeholder="0812-..."
                  />
                </div>
              </div>

              <div>
                <label class="block font-bold text-slate-700 mb-1">Term of Payment (Hari)</label>
                <input
                  v-model.number="supplierForm.term_of_payment_days"
                  type="number"
                  min="0"
                  class="w-full px-3 py-2 rounded-lg border border-slate-300 font-mono text-xs focus:ring-2 focus:ring-emerald-500 focus:outline-none"
                  placeholder="14"
                />
              </div>

              <div>
                <label class="block font-bold text-slate-700 mb-1">Alamat Kantor / Gudang</label>
                <textarea
                  v-model="supplierForm.address"
                  rows="2"
                  class="w-full px-3 py-2 rounded-lg border border-slate-300 text-xs focus:ring-2 focus:ring-emerald-500 focus:outline-none"
                  placeholder="Alamat lengkap supplier..."
                ></textarea>
              </div>
            </div>
          </div>

          <div class="px-6 py-3.5 bg-slate-50 border-t border-slate-200 flex items-center justify-end gap-2.5">
            <button
              type="button"
              @click="showSupplierModal = false"
              :disabled="isSavingSupplier"
              class="px-4 py-2 rounded-lg border border-slate-300 bg-white hover:bg-slate-100 text-slate-700 font-bold text-xs transition cursor-pointer"
            >
              Batal
            </button>
            <button
              type="button"
              @click="handleSaveSupplier"
              :disabled="isSavingSupplier"
              class="px-4 py-2 rounded-lg bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-xs flex items-center gap-2 shadow-xs transition active:scale-95 cursor-pointer disabled:opacity-50"
            >
              <RefreshCw v-if="isSavingSupplier" class="w-4 h-4 animate-spin" />
              <Check v-else class="w-4 h-4" />
              <span>Simpan Pemasok</span>
            </button>
          </div>
        </div>
      </div>

      <!-- 8. Modal Buat Dokumen Retur Pembelian (Purchase Return) -->
      <div
        v-if="showReturnModal"
        class="fixed inset-0 z-50 bg-slate-900/60 backdrop-blur-xs flex items-center justify-center p-4"
        @click.self="showReturnModal = false"
      >
        <div class="bg-white rounded-2xl shadow-2xl border border-slate-300 w-full max-w-4xl overflow-hidden flex flex-col max-h-[92vh] animate-in fade-in zoom-in-95 duration-150">
          <!-- Header -->
          <div class="px-6 py-4 bg-rose-950 text-white flex items-center justify-between shrink-0">
            <div class="flex items-center gap-3">
              <div class="w-9 h-9 rounded-xl bg-rose-500/20 border border-rose-400/40 flex items-center justify-center">
                <RotateCcw class="w-5 h-5 text-rose-400" />
              </div>
              <div>
                <div class="text-xs font-bold text-rose-300 uppercase tracking-wider">Pengembalian Barang ke Supplier</div>
                <div class="text-base font-black tracking-tight">Formulir Retur Pembelian Baru</div>
              </div>
            </div>
            <button
              type="button"
              @click="showReturnModal = false"
              class="text-rose-300 hover:text-white p-1 rounded-lg hover:bg-rose-900 transition cursor-pointer"
            >
              <X class="w-5 h-5" />
            </button>
          </div>

          <!-- Body -->
          <div class="p-6 overflow-y-auto space-y-5 flex-1 min-h-0 text-xs">
            <!-- Parameter Header Form -->
            <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3.5 bg-rose-50/40 p-4 rounded-xl border border-rose-200">
              <div>
                <label class="block font-bold text-slate-700 mb-1">Pemasok (Supplier) <span class="text-rose-600">*</span></label>
                <select
                  v-model="returnForm.supplier_id"
                  class="w-full px-3 py-2 rounded-lg border border-slate-300 bg-white text-xs font-bold text-slate-800 focus:ring-2 focus:ring-rose-500 focus:outline-none"
                >
                  <option value="" disabled>-- Pilih Pemasok --</option>
                  <option v-for="s in suppliers" :key="s.id" :value="s.id">
                    {{ s.name }} ({{ s.code }})
                  </option>
                </select>
              </div>

              <div>
                <label class="block font-bold text-slate-700 mb-1">Outlet / Gudang Pengirim</label>
                <select
                  v-model="returnForm.outlet_id"
                  class="w-full px-3 py-2 rounded-lg border border-slate-300 bg-white text-xs font-bold text-slate-800 focus:ring-2 focus:ring-rose-500 focus:outline-none"
                >
                  <option value="">Semua / Outlet Utama</option>
                  <option v-for="o in outlets" :key="o.id" :value="o.id">
                    {{ o.name }}
                  </option>
                </select>
              </div>

              <div>
                <label class="block font-bold text-slate-700 mb-1">Tanggal Retur</label>
                <input
                  v-model="returnForm.return_date"
                  type="date"
                  class="w-full px-3 py-2 rounded-lg border border-slate-300 bg-white font-mono text-xs focus:ring-2 focus:ring-rose-500 focus:outline-none"
                />
              </div>

              <div>
                <label class="block font-bold text-slate-700 mb-1">Metode Kompensasi Retur</label>
                <select
                  v-model="returnForm.refund_method"
                  class="w-full px-3 py-2 rounded-lg border border-slate-300 bg-white text-xs font-bold text-slate-800 focus:ring-2 focus:ring-rose-500 focus:outline-none"
                >
                  <option value="DEBT_REDUCTION">Potong Hutang Usaha (Invoice)</option>
                  <option value="CASH_REFUND">Pengembalian Kas / Bank</option>
                  <option value="REPLACEMENT">Penggantian Barang / Kredit Toko</option>
                </select>
              </div>

              <div>
                <label class="block font-bold text-slate-700 mb-1">Alasan Retur <span class="text-rose-600">*</span></label>
                <input
                  v-model="returnForm.reason"
                  type="text"
                  placeholder="Misal: Barang Rusak, Cacat Pabrik, Kadaluarsa"
                  class="w-full px-3 py-2 rounded-lg border border-slate-300 bg-white text-xs focus:ring-2 focus:ring-rose-500 focus:outline-none"
                />
              </div>

              <div>
                <label class="block font-bold text-slate-700 mb-1">Catatan Tambahan</label>
                <input
                  v-model="returnForm.notes"
                  type="text"
                  placeholder="Keterangan pendukung..."
                  class="w-full px-3 py-2 rounded-lg border border-slate-300 bg-white text-xs focus:ring-2 focus:ring-rose-500 focus:outline-none"
                />
              </div>
            </div>

            <!-- Tabel Daftar Produk yang Diretur -->
            <div>
              <div class="flex items-center justify-between mb-2">
                <div class="font-bold text-slate-800 uppercase tracking-wide text-xs">
                  Daftar Barang yang Dikembalikan:
                </div>
                <button
                  type="button"
                  @click="addReturnItem"
                  class="px-3 py-1.5 rounded-lg bg-rose-600 hover:bg-rose-700 text-white font-bold text-xs flex items-center gap-1.5 transition active:scale-95 cursor-pointer shadow-2xs"
                >
                  <Plus class="w-3.5 h-3.5" />
                  <span>Tambah Baris Produk</span>
                </button>
              </div>

              <div class="border border-slate-300 rounded-xl overflow-hidden shadow-2xs bg-white">
                <table class="w-full text-left text-xs text-slate-700">
                  <thead class="bg-slate-100 text-[11px] font-bold text-slate-600 uppercase border-b border-slate-200">
                    <tr>
                      <th class="px-3 py-2.5 w-12 text-center">#</th>
                      <th class="px-3 py-2.5 min-w-[220px]">Pilih Produk</th>
                      <th class="px-3 py-2.5 w-28 text-right">Qty Diretur</th>
                      <th class="px-3 py-2.5 w-36 text-right">Harga Satuan Beli</th>
                      <th class="px-3 py-2.5 w-36 text-right">Subtotal</th>
                      <th class="px-3 py-2.5 min-w-[150px]">Keterangan Kerusakan</th>
                      <th class="px-3 py-2.5 w-12 text-center">Aksi</th>
                    </tr>
                  </thead>
                  <tbody class="divide-y divide-slate-100">
                    <tr v-for="(it, idx) in returnForm.items" :key="idx" class="hover:bg-rose-50/20">
                      <td class="px-3 py-2 text-center text-slate-400 font-mono">
                        {{ idx + 1 }}
                      </td>
                      <td class="px-3 py-2">
                        <select
                          :value="it.product_id"
                          @change="(e: any) => handleReturnProductChange(idx, e.target.value)"
                          class="w-full px-2.5 py-1.5 rounded border border-slate-300 text-xs font-bold text-slate-800 focus:ring-1 focus:ring-rose-500"
                        >
                          <option value="" disabled>-- Pilih Produk --</option>
                          <option v-for="p in products" :key="p.id" :value="p.id">
                            {{ p.name }} ({{ p.sku || p.barcode }})
                          </option>
                        </select>
                      </td>
                      <td class="px-3 py-2">
                        <input
                          v-model.number="it.returned_qty"
                          @input="updateReturnItemSubtotal(idx)"
                          type="number"
                          step="any"
                          min="0.0001"
                          class="w-full px-2 py-1.5 rounded border border-slate-300 font-mono text-right text-xs font-bold focus:ring-1 focus:ring-rose-500"
                        />
                      </td>
                      <td class="px-3 py-2">
                        <input
                          v-model.number="it.unit_price"
                          @input="updateReturnItemSubtotal(idx)"
                          type="number"
                          step="any"
                          min="0"
                          class="w-full px-2 py-1.5 rounded border border-slate-300 font-mono text-right text-xs focus:ring-1 focus:ring-rose-500"
                        />
                      </td>
                      <td class="px-3 py-2 text-right font-mono font-black text-rose-950">
                        {{ formatCurrency(it.subtotal) }}
                      </td>
                      <td class="px-3 py-2">
                        <input
                          v-model="it.notes"
                          type="text"
                          placeholder="Kondisi barang..."
                          class="w-full px-2 py-1.5 rounded border border-slate-300 text-xs focus:ring-1 focus:ring-rose-500"
                        />
                      </td>
                      <td class="px-3 py-2 text-center">
                        <button
                          type="button"
                          @click="removeReturnItem(idx)"
                          class="p-1 rounded text-slate-400 hover:text-rose-600 hover:bg-rose-50 transition cursor-pointer"
                          title="Hapus Baris"
                        >
                          <Trash2 class="w-4 h-4" />
                        </button>
                      </td>
                    </tr>

                    <tr v-if="returnForm.items.length === 0">
                      <td colspan="7" class="py-8 text-center text-slate-400">
                        <RotateCcw class="w-8 h-8 mx-auto mb-1 text-slate-300" />
                        <p class="font-bold text-xs">Belum ada barang yang ditambahkan untuk diretur</p>
                        <button
                          type="button"
                          @click="addReturnItem"
                          class="mt-2 text-rose-600 font-bold hover:underline"
                        >
                          + Tambah Baris Produk Sekarang
                        </button>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>

            <!-- Otomatisasi & Ringkasan -->
            <div class="grid grid-cols-1 md:grid-cols-2 gap-4 items-center">
              <div class="p-3.5 bg-rose-50 border border-rose-200 rounded-xl text-rose-950 text-[11px] flex items-start gap-2.5">
                <CheckCircle2 class="w-4 h-4 text-rose-600 shrink-0 mt-0.5" />
                <div class="leading-relaxed">
                  <strong>Otomatisasi Sistem Akuntansi & Stok:</strong><br />
                  1. Stok fisik produk akan otomatis dikurangi dari outlet/gudang terkait.<br />
                  2. Tercatat mutasi stok keluar (<em>PURCHASE_RETURN</em>).<br />
                  3. Jurnal otomatis: <strong>Debet Hutang Usaha / Kas</strong> vs <strong>Kredit Persediaan Barang Dagang</strong>.
                </div>
              </div>

              <div class="flex justify-end">
                <div class="w-full max-w-sm bg-slate-50 p-4 rounded-xl border border-slate-200 font-mono space-y-1.5">
                  <div class="flex justify-between text-slate-600 text-xs">
                    <span>Jumlah Item Barang:</span>
                    <span class="font-bold text-slate-800">{{ returnForm.items.length }} produk</span>
                  </div>
                  <div class="h-px bg-slate-300 my-1"></div>
                  <div class="flex justify-between text-slate-900 font-black text-sm">
                    <span class="font-sans">Total Nilai Retur:</span>
                    <span class="text-rose-900 text-base">{{ formatCurrency(returnSubtotal) }}</span>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- Footer -->
          <div class="px-6 py-3.5 bg-slate-50 border-t border-slate-200 flex items-center justify-between shrink-0">
            <button
              type="button"
              @click="showReturnModal = false"
              :disabled="isSavingReturn"
              class="px-4 py-2 rounded-lg border border-slate-300 bg-white hover:bg-slate-100 text-slate-700 font-bold text-xs transition cursor-pointer"
            >
              Batal
            </button>

            <button
              type="button"
              @click="handleSaveReturn"
              :disabled="isSavingReturn"
              class="px-5 py-2 rounded-lg bg-rose-600 hover:bg-rose-700 text-white font-bold text-xs flex items-center gap-2 shadow-xs transition active:scale-95 cursor-pointer disabled:opacity-50"
            >
              <RefreshCw v-if="isSavingReturn" class="w-4 h-4 animate-spin" />
              <Check v-else class="w-4 h-4" />
              <span>Simpan & Proses Retur</span>
            </button>
          </div>
        </div>
      </div>

      <!-- 9. Modal Detail Retur Pembelian -->
      <div
        v-if="showReturnDetailModal"
        class="fixed inset-0 z-50 bg-slate-900/60 backdrop-blur-xs flex items-center justify-center p-4"
        @click.self="showReturnDetailModal = false"
      >
        <div class="bg-white rounded-2xl shadow-2xl border border-slate-300 w-full max-w-3xl overflow-hidden flex flex-col max-h-[90vh] animate-in fade-in zoom-in-95 duration-150">
          <div class="px-6 py-4 bg-rose-950 text-white flex items-center justify-between shrink-0">
            <div class="flex items-center gap-3">
              <div class="w-9 h-9 rounded-xl bg-rose-500/20 border border-rose-400/40 flex items-center justify-center">
                <RotateCcw class="w-5 h-5 text-rose-400" />
              </div>
              <div>
                <div class="text-xs font-bold text-rose-300 uppercase tracking-wider">Detail Nota Retur Pembelian</div>
                <div class="text-base font-black tracking-tight flex items-center gap-2">
                  <span>{{ selectedReturnDetail?.return_number }}</span>
                  <span
                    v-if="selectedReturnDetail?.status === 'COMPLETED'"
                    class="text-[10px] bg-emerald-500/20 text-emerald-300 px-2 py-0.5 rounded border border-emerald-400/30 font-bold"
                  >
                    SELESAI
                  </span>
                  <span
                    v-else
                    class="text-[10px] bg-rose-500/20 text-rose-300 px-2 py-0.5 rounded border border-rose-400/30 font-bold"
                  >
                    {{ selectedReturnDetail?.status }}
                  </span>
                </div>
              </div>
            </div>
            <button
              type="button"
              @click="showReturnDetailModal = false"
              class="text-rose-300 hover:text-white p-1 rounded-lg hover:bg-rose-900 transition cursor-pointer"
            >
              <X class="w-5 h-5" />
            </button>
          </div>

          <div class="p-6 overflow-y-auto space-y-5 flex-1 min-h-0 text-xs">
            <div v-if="isLoadingReturnDetail" class="py-12 text-center text-slate-400">
              <RefreshCw class="w-8 h-8 mx-auto mb-2 text-rose-500 animate-spin" />
              <p class="font-bold">Memuat rincian retur pembelian...</p>
            </div>

            <div v-else-if="selectedReturnDetail" class="space-y-5">
              <div class="grid grid-cols-2 sm:grid-cols-4 gap-3 bg-slate-50 p-3.5 rounded-xl border border-slate-200">
                <div>
                  <div class="text-[10px] font-bold text-slate-400 uppercase">Pemasok</div>
                  <div class="font-bold text-slate-800 mt-0.5">{{ selectedReturnDetail.supplier_name }}</div>
                </div>
                <div>
                  <div class="text-[10px] font-bold text-slate-400 uppercase">Tanggal Retur</div>
                  <div class="font-bold text-slate-800 mt-0.5">{{ formatDate(selectedReturnDetail.return_date) }}</div>
                </div>
                <div>
                  <div class="text-[10px] font-bold text-slate-400 uppercase">Outlet / Gudang</div>
                  <div class="font-bold text-rose-800 mt-0.5">{{ selectedReturnDetail.outlet_name || 'Outlet Pusat' }}</div>
                </div>
                <div>
                  <div class="text-[10px] font-bold text-slate-400 uppercase">Metode Kompensasi</div>
                  <div class="font-bold text-slate-800 mt-0.5">
                    <span v-if="selectedReturnDetail.refund_method === 'DEBT_REDUCTION'" class="text-blue-700">Potong Hutang Usaha</span>
                    <span v-else-if="selectedReturnDetail.refund_method === 'CASH_REFUND'" class="text-emerald-700">Pengembalian Kas/Bank</span>
                    <span v-else class="text-purple-700">Ganti Barang</span>
                  </div>
                </div>
                <div class="col-span-2">
                  <div class="text-[10px] font-bold text-slate-400 uppercase">Alasan Retur</div>
                  <div class="font-medium text-slate-800 mt-0.5">{{ selectedReturnDetail.reason }}</div>
                </div>
                <div class="col-span-2">
                  <div class="text-[10px] font-bold text-slate-400 uppercase">Rujukan Dokumen</div>
                  <div class="font-mono text-slate-800 mt-0.5">
                    {{ selectedReturnDetail.invoice_number ? `Faktur: ${selectedReturnDetail.invoice_number}` : '' }}
                    {{ selectedReturnDetail.grn_number ? ` (GRN: ${selectedReturnDetail.grn_number})` : '' }}
                    {{ !selectedReturnDetail.invoice_number && !selectedReturnDetail.grn_number ? '-' : '' }}
                  </div>
                </div>
              </div>

              <!-- Daftar Barang -->
              <div>
                <div class="text-xs font-bold text-slate-800 mb-2 uppercase tracking-wide">
                  Rincian Barang yang Diretur:
                </div>
                <div class="border border-slate-200 rounded-xl overflow-hidden">
                  <table class="w-full text-left text-xs text-slate-700">
                    <thead class="bg-slate-100 text-[10px] font-bold text-slate-500 uppercase border-b border-slate-200">
                      <tr>
                        <th class="px-3 py-2">Produk</th>
                        <th class="px-3 py-2 text-right">Qty Diretur</th>
                        <th class="px-3 py-2 text-right">Harga Beli Satuan</th>
                        <th class="px-3 py-2 text-right">Subtotal</th>
                        <th class="px-3 py-2">Keterangan</th>
                      </tr>
                    </thead>
                    <tbody class="divide-y divide-slate-100 font-mono">
                      <tr v-for="it in (selectedReturnDetail.items || [])" :key="it.id">
                        <td class="px-3 py-2.5 font-sans font-bold text-slate-900">
                          {{ it.product_name }}
                          <div class="text-[10px] text-slate-400 font-mono">{{ it.product_sku }}</div>
                        </td>
                        <td class="px-3 py-2.5 text-right font-bold text-rose-800">
                          {{ formatQty(it.returned_qty) }} {{ it.unit_code }}
                        </td>
                        <td class="px-3 py-2.5 text-right text-slate-600">
                          {{ formatCurrency(it.unit_price) }}
                        </td>
                        <td class="px-3 py-2.5 text-right font-black text-rose-950">
                          {{ formatCurrency(it.subtotal) }}
                        </td>
                        <td class="px-3 py-2.5 font-sans text-slate-600 text-[11px]">
                          {{ it.notes || '-' }}
                        </td>
                      </tr>
                    </tbody>
                  </table>
                </div>
              </div>

              <div class="flex justify-end">
                <div class="w-80 bg-rose-50/60 p-4 rounded-xl border border-rose-200 space-y-2 font-mono">
                  <div class="flex justify-between text-slate-600 text-xs">
                    <span>Subtotal Barang:</span>
                    <span>{{ formatCurrency(selectedReturnDetail.subtotal) }}</span>
                  </div>
                  <div class="h-px bg-rose-300 my-1"></div>
                  <div class="flex justify-between text-rose-950 font-black text-sm">
                    <span class="font-sans">Total Nilai Retur:</span>
                    <span>{{ formatCurrency(selectedReturnDetail.total_amount) }}</span>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <div class="px-6 py-3.5 bg-slate-50 border-t border-slate-200 flex items-center justify-end shrink-0">
            <button
              type="button"
              @click="showReturnDetailModal = false"
              class="px-4 py-2 rounded-lg border border-slate-300 bg-white hover:bg-slate-100 text-slate-700 font-bold text-xs transition cursor-pointer"
            >
              Tutup
            </button>
          </div>
        </div>
      </div>

    </div>
  </AdminLayout>
</template>
