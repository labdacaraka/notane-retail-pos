<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, computed } from 'vue'
import AdminLayout from '@/layouts/AdminLayout.vue'
import PaginationBar from '@/components/common/PaginationBar.vue'
import SearchableSelect from '@/components/common/SearchableSelect.vue'
import {
  ClipboardCheck,
  Plus,
  Search,
  Building2,
  Eye,
  Trash2,
  X,
  ArrowRight,
  FileText,
  CheckCircle2,
  AlertCircle,
  Truck,
  PackageCheck,
  Ban,
  Clock,
  Printer,
  Check,
} from 'lucide-vue-next'
import { Icon } from '@iconify/vue'
import { api } from '@/services/api'
import { useLocationStore } from '@/stores/locationStore'
import { useAuthStore } from '@/stores/authStore'
import { formatCurrency, formatQty, formatDate } from '@/utils/formatters'
import { soundService } from '@/services/soundService'

const locationStore = useLocationStore()
const authStore = useAuthStore()

const activeTab = ref<'opname' | 'transfer' | 'ledger' | 'fefo'>('opname')
const products = ref<any[]>([])
const suppliers = ref<any[]>([])
const warehouses = ref<any[]>([])
const outletsList = ref<any[]>([])
const reasonCodes = ref<any[]>([])

// FEFO Alert State (Prioritas 2)
const fefoAlerts = ref<any[]>([])
const isLoadingFefo = ref<boolean>(false)
const fefoDays = ref<number>(60)
const fefoSearch = ref<string>('')
const fefoCriticalCount = computed(() => {
  return fefoAlerts.value.filter((a) => a.urgency === 'EXPIRED' || a.urgency === 'CRITICAL').length
})
const filteredFefoAlerts = computed(() => {
  if (!fefoSearch.value.trim()) return fefoAlerts.value
  const q = fefoSearch.value.toLowerCase()
  return fefoAlerts.value.filter(
    (a) =>
      a.product_name?.toLowerCase().includes(q) ||
      a.product_sku?.toLowerCase().includes(q) ||
      a.batch_number?.toLowerCase().includes(q)
  )
})

async function fetchFefoAlerts() {
  isLoadingFefo.value = true
  try {
    const res = await api.get('/inventory/fefo-alerts', {
      params: {
        days: fefoDays.value,
        outlet_id: locationStore.currentOutletId || undefined,
      },
    })
    if (res.data?.success && Array.isArray(res.data.data)) {
      fefoAlerts.value = res.data.data
    }
  } catch (err: any) {
    showToast('Gagal memuat alert FEFO: ' + (err.response?.data?.message || err.message), 'error')
  } finally {
    isLoadingFefo.value = false
  }
}

// Fast Scan Mode State (Prioritas 2)
const isFastScanMode = ref<boolean>(false)
const fastScanBarcode = ref<string>('')
const lastScannedItem = ref<any>(null)

function handleFastScan() {
  const code = fastScanBarcode.value.trim()
  if (!code) return
  fastScanBarcode.value = ''

  // 1. Cari di opnameItems
  const existing = opnameItems.value.find((it) => it.barcode === code || it.sku === code)
  if (existing) {
    existing.actual_qty = Number((Number(existing.actual_qty) + 1).toFixed(3))
    lastScannedItem.value = existing
    soundService.playScanSuccess()
    showToast(`[BIP] +1 Kuantitas: ${existing.name} (Total Fisik: ${existing.actual_qty})`, 'success')
    return
  }

  // 2. Cari di master products
  const master = products.value.find((p) => p.barcode === code || p.sku === code)
  if (master) {
    const newItem = {
      product_id: master.id,
      name: master.name,
      sku: master.sku,
      barcode: master.barcode,
      unit_code: master.unit_code,
      system_qty: master.stock_qty || 0,
      actual_qty: 1,
    }
    opnameItems.value.unshift(newItem)
    lastScannedItem.value = newItem
    soundService.playScanSuccess()
    showToast(`[BIP] +1 Kuantitas: ${master.name} (Fisik Baru: 1)`, 'success')
  } else {
    soundService.playOutOfStock()
    showToast(`Produk dengan barcode '${code}' tidak ditemukan`, 'error')
  }
}

// Toast State
const toastMessage = ref<string>('')
const toastType = ref<'success' | 'error'>('success')
function showToast(msg: string, type: 'success' | 'error' = 'success') {
  toastMessage.value = msg
  toastType.value = type
  setTimeout(() => {
    toastMessage.value = ''
  }, 4000)
}

// Opname Form State & Pagination
const opnameWarehouseId = ref<string>('')
const opnameReasonId = ref<string>('f1000000-0000-0000-0000-000000000003')
const opnameItems = ref<any[]>([])
const opnamePage = ref<number>(1)
const opnamePageSize = ref<number>(10)
const paginatedOpnameItems = computed(() => {
  const start = (opnamePage.value - 1) * opnamePageSize.value
  return opnameItems.value.slice(start, start + opnamePageSize.value)
})

// Ledger State & Pagination
const selectedProductForLedger = ref<string>('')
const ledgerMutations = ref<any[]>([])
const ledgerPage = ref<number>(1)
const ledgerPageSize = ref<number>(10)
const paginatedLedgerMutations = computed(() => {
  const start = (ledgerPage.value - 1) * ledgerPageSize.value
  return ledgerMutations.value.slice(start, start + ledgerPageSize.value)
})

// Transfer State
const transfers = ref<any[]>([])
const transferLoading = ref<boolean>(false)
const transferSearch = ref<string>('')
const filterFromOutlet = ref<string>('')
const filterToOutlet = ref<string>('')
const filterTransferStatus = ref<'ALL' | 'IN_TRANSIT' | 'MY_INBOUND' | 'COMPLETED' | 'CANCELLED'>('ALL')
const transferPage = ref<number>(1)
const transferPageSize = ref<number>(10)
const totalTransfers = ref<number>(0)

// Quick counters
const inTransitCount = computed(() => transfers.value.filter((t) => t.status === 'IN_TRANSIT').length)
const myInboundCount = computed(() => {
  const currentId = locationStore.currentOutletId
  if (!currentId) return 0
  return transfers.value.filter((t) => t.status === 'IN_TRANSIT' && t.to_outlet_id === currentId).length
})

// Create Transfer Modal State
const showCreateTransferModal = ref<boolean>(false)
const isSubmittingTransfer = ref<boolean>(false)
const transferForm = ref({
  from_outlet_id: '',
  to_outlet_id: '',
  transfer_date: new Date().toISOString().slice(0, 10),
  is_immediate: false, // false = Alur 2-Tahap (Kirim -> In-Transit -> Terima), true = Instan
  notes: '',
  items: [] as Array<{
    product_id: string
    name: string
    sku: string
    barcode?: string
    unit_code?: string
    available_qty: number
    quantity: number
    cost_price: number
    subtotal: number
  }>,
})

// Receive Transfer Modal State (Penerimaan Barang Transfer)
const showReceiveTransferModal = ref<boolean>(false)
const isSubmittingReceive = ref<boolean>(false)
const receiveTransferData = ref<any>(null)
const receiveForm = ref({
  transfer_id: '',
  transfer_number: '',
  from_outlet_name: '',
  to_outlet_name: '',
  receive_date: new Date().toISOString().slice(0, 10),
  notes: '',
  items: [] as Array<{
    product_id: string
    product_code: string
    product_name: string
    barcode?: string
    unit_code: string
    shipped_qty: number
    received_quantity: number
    cost_price: number
    notes: string
  }>,
})

// Cancel Transfer Modal State (Pembatalan Dokumen Transfer)
const showCancelTransferModal = ref<boolean>(false)
const isSubmittingCancel = ref<boolean>(false)
const selectedTransferToCancel = ref<any>(null)
const cancelReason = ref<string>('')

// Detail Transfer Modal State
const showTransferDetailModal = ref<boolean>(false)
const selectedTransferDetail = ref<any>(null)
const isLoadingDetail = ref<boolean>(false)

function syncWarehouseFromLocation() {
  if (locationStore.currentOutlet) {
    const targetWhId = locationStore.currentOutlet.id
    if (targetWhId && warehouses.value.some((w: any) => w.id === targetWhId)) {
      opnameWarehouseId.value = targetWhId
    } else if (warehouses.value.length > 0 && !opnameWarehouseId.value) {
      opnameWarehouseId.value = warehouses.value[0].id
    }
  } else if (warehouses.value.length > 0 && !opnameWarehouseId.value) {
    opnameWarehouseId.value = warehouses.value[0].id
  }
}

async function loadInitialData() {
  try {
    const [pRes, sRes, wRes, rRes, oRes] = await Promise.all([
      api.get('/products', {
        params: {
          page_size: 100,
        },
      }),
      api.get('/suppliers'),
      api.get('/warehouses'),
      api.get('/reason-codes'),
      api.get('/outlets'),
    ])

    if (oRes.data?.success) {
      outletsList.value = oRes.data.data
    }

    if (wRes.data?.success) {
      warehouses.value = wRes.data.data
      syncWarehouseFromLocation()
    }

    if (pRes.data?.success) {
      products.value = pRes.data.data
      opnameItems.value = pRes.data.data.map((p: any) => ({
        product_id: p.id,
        name: p.name,
        sku: p.sku,
        unit_code: p.unit_code,
        system_qty: p.stock_qty || 0,
        actual_qty: p.stock_qty || 0,
      }))
      if (pRes.data.data.length > 0 && !selectedProductForLedger.value) {
        selectedProductForLedger.value = pRes.data.data[0].id
        loadLedger()
      }
    }
    if (sRes.data?.success) suppliers.value = sRes.data.data
    if (rRes.data?.success) {
      reasonCodes.value = rRes.data.data
      if (rRes.data.data.length > 0 && !opnameReasonId.value) {
        opnameReasonId.value = rRes.data.data[0].id
      }
    }
  } catch (err) {
    console.error(err)
  }
}

async function handleSaveOpname() {
  if (!authStore.hasPermission('inventory:adjust')) {
    showToast('Akses ditolak: Anda tidak memiliki wewenang penyesuaian stok opname (inventory:adjust)', 'error')
    return
  }
  try {
    const payload = {
      // warehouse_id tidak perlu dikirim karena ditentukan aman di backend berdasarkan current_outlet_id pengguna
      reason_id: opnameReasonId.value,
      notes: 'Stock Opname Fisik Bulanan Toko',
      items: opnameItems.value.map((it) => ({
        product_id: it.product_id,
        actual_qty: it.actual_qty,
      })),
    }

    const res = await api.post('/inventory/adjustments', payload)
    showToast(res.data.message || 'Opname berhasil disimpan!')
    loadInitialData()
  } catch (err: any) {
    showToast(err.response?.data?.message || 'Gagal menyimpan opname', 'error')
  }
}

async function loadLedger() {
  if (!selectedProductForLedger.value) return
  try {
    const res = await api.get(`/inventory/stock-ledger/${selectedProductForLedger.value}`)
    if (res.data.success) {
      ledgerMutations.value = res.data.data
    }
  } catch (err) {
    console.error(err)
  }
}

// ==========================================
// TRANSFER STOK METHODS
// ==========================================
async function loadTransfers() {
  try {
    transferLoading.value = true
    const params: any = {
      page: transferPage.value,
      page_size: transferPageSize.value,
    }
    if (transferSearch.value.trim()) params.search = transferSearch.value.trim()
    if (filterFromOutlet.value) params.from_outlet_id = filterFromOutlet.value
    if (filterToOutlet.value) params.to_outlet_id = filterToOutlet.value

    if (filterTransferStatus.value === 'MY_INBOUND') {
      if (locationStore.currentOutletId) {
        params.to_outlet_id = locationStore.currentOutletId
      }
      params.status = 'IN_TRANSIT'
    } else if (filterTransferStatus.value !== 'ALL') {
      params.status = filterTransferStatus.value
    }

    const res = await api.get('/inventory/transfers', { params })
    if (res.data?.success) {
      transfers.value = res.data.data.items || []
      totalTransfers.value = res.data.data.total || 0
    }
  } catch (err: any) {
    console.error('Gagal memuat daftar transfer:', err)
  } finally {
    transferLoading.value = false
  }
}

function setTransferStatusFilter(status: 'ALL' | 'IN_TRANSIT' | 'MY_INBOUND' | 'COMPLETED' | 'CANCELLED') {
  filterTransferStatus.value = status
  transferPage.value = 1
  loadTransfers()
}

function openCreateTransferModal() {
  const currentId = locationStore.currentOutletId || (outletsList.value.length > 0 ? outletsList.value[0].id : '')
  const otherOutlet = outletsList.value.find((o: any) => o.id !== currentId)

  transferForm.value = {
    from_outlet_id: currentId,
    to_outlet_id: otherOutlet ? otherOutlet.id : '',
    transfer_date: new Date().toISOString().slice(0, 10),
    is_immediate: false,
    notes: '',
    items: [
      {
        product_id: '',
        name: '',
        sku: '',
        barcode: '',
        unit_code: 'PCS',
        available_qty: 0,
        quantity: 1,
        cost_price: 0,
        subtotal: 0,
      },
    ],
  }
  showCreateTransferModal.value = true
}

function addTransferItem() {
  transferForm.value.items.push({
    product_id: '',
    name: '',
    sku: '',
    barcode: '',
    unit_code: 'PCS',
    available_qty: 0,
    quantity: 1,
    cost_price: 0,
    subtotal: 0,
  })
}

function removeTransferItem(index: number) {
  if (transferForm.value.items.length > 1) {
    transferForm.value.items.splice(index, 1)
  }
}

// Catalog Picker for Stock Transfer
const showTransferProductPicker = ref(false)
const transferProductSearch = ref('')
const transferProductResults = ref<any[]>([])
const isSearchingTransferProducts = ref(false)
let transferSearchDebounce: ReturnType<typeof setTimeout> | null = null

async function searchTransferProducts() {
  if (transferSearchDebounce) clearTimeout(transferSearchDebounce)
  transferSearchDebounce = setTimeout(async () => {
    isSearchingTransferProducts.value = true
    try {
      const res = await api.get('/products', {
        params: {
          outlet_id: transferForm.value.from_outlet_id,
          search: transferProductSearch.value.trim(),
          limit: 30,
        },
      })
      const list = res.data?.data || res.data || []
      transferProductResults.value = list
    } catch (err) {
      console.error('Error searching products for transfer:', err)
      transferProductResults.value = []
    } finally {
      isSearchingTransferProducts.value = false
    }
  }, 250)
}

function openTransferProductPicker() {
  if (!transferForm.value.from_outlet_id) {
    showToast('Pilih outlet asal pengiriman terlebih dahulu', 'error')
    return
  }
  showTransferProductPicker.value = true
  transferProductSearch.value = ''
  searchTransferProducts()
}

function addProductToTransfer(product: any) {
  const firstRow = transferForm.value.items[0]
  if (transferForm.value.items.length === 1 && !firstRow.product_id) {
    onProductSelectedInTransfer(firstRow, product)
  } else {
    const existing = transferForm.value.items.find((it) => it.product_id === product.id)
    if (existing) {
      existing.quantity += 1
      onQtyChanged(existing)
      showToast(`Jumlah ${product.name} ditambah (+1)`, 'success')
      return
    }
    const newRow = {
      product_id: product.id,
      name: product.name,
      sku: product.sku || '',
      barcode: product.barcode || '',
      unit_code: product.unit_code || 'PCS',
      available_qty: Number(product.stock_qty || 0),
      quantity: 1,
      cost_price: Number(product.cost_price || 0),
      subtotal: Number(product.cost_price || 0),
    }
    transferForm.value.items.push(newRow)
  }
  showToast(`${product.name} ditambahkan`, 'success')
}

// Watcher untuk sinkronisasi stok item saat outlet asal diganti
watch(
  () => transferForm.value.from_outlet_id,
  async (newOutletId) => {
    if (!newOutletId) return
    for (const item of transferForm.value.items) {
      if (item.product_id) {
        try {
          const res = await api.get('/products', {
            params: {
              outlet_id: newOutletId,
              search: item.sku || item.name || '',
            },
          })
          const list = res.data?.data || res.data || []
          const match = list.find((p: any) => p.id === item.product_id)
          if (match) {
            item.available_qty = Number(match.stock_qty || 0)
            item.cost_price = Number(match.cost_price || item.cost_price)
            item.subtotal = item.quantity * item.cost_price
          }
        } catch (e) {
          console.error('Failed refreshing stock for item', e)
        }
      }
    }
    if (showTransferProductPicker.value) {
      searchTransferProducts()
    }
  }
)

async function onProductSelectedInTransfer(itemRow: any, productObj: any) {
  if (!productObj) {
    itemRow.product_id = ''
    itemRow.name = ''
    itemRow.sku = ''
    itemRow.barcode = ''
    itemRow.unit_code = 'PCS'
    itemRow.cost_price = 0
    itemRow.available_qty = 0
    itemRow.subtotal = 0
    return
  }

  itemRow.product_id = productObj.id
  itemRow.name = productObj.name
  itemRow.sku = productObj.sku
  itemRow.barcode = productObj.barcode || ''
  itemRow.unit_code = productObj.unit_code || 'PCS'
  itemRow.cost_price = Number(productObj.cost_price) || 0
  itemRow.available_qty = productObj.stock_qty !== undefined ? Number(productObj.stock_qty) : 0
  itemRow.subtotal = itemRow.quantity * itemRow.cost_price

  // Jika stock_qty belum disertakan dalam objek produk, ambil dari backend
  if (productObj.stock_qty === undefined) {
    try {
      const res = await api.get('/products', {
        params: {
          outlet_id: transferForm.value.from_outlet_id,
          search: productObj.code || productObj.sku || productObj.name,
        },
      })
      const list = res.data?.data || res.data || []
      const match = list.find((p: any) => p.id === productObj.id)
      if (match) {
        itemRow.available_qty = Number(match.stock_qty || 0)
        itemRow.cost_price = Number(match.cost_price || itemRow.cost_price)
        itemRow.subtotal = itemRow.quantity * itemRow.cost_price
      }
    } catch (e) {
      console.error('Failed fetching stock:', e)
    }
  }
}

function onQtyChanged(itemRow: any) {
  itemRow.subtotal = Number(itemRow.quantity || 0) * Number(itemRow.cost_price || 0)
}

function calculateTransferTotal() {
  return transferForm.value.items.reduce(
    (sum, it) => sum + Number(it.quantity || 0) * Number(it.cost_price || 0),
    0
  )
}

async function handleSaveTransfer() {
  if (!transferForm.value.from_outlet_id) {
    showToast('Pilih outlet asal pengiriman', 'error')
    return
  }
  if (!transferForm.value.to_outlet_id) {
    showToast('Pilih outlet tujuan penerimaan', 'error')
    return
  }
  if (transferForm.value.from_outlet_id === transferForm.value.to_outlet_id) {
    showToast('Outlet asal dan outlet tujuan tidak boleh sama', 'error')
    return
  }
  const validItems = transferForm.value.items.filter((it) => it.product_id && Number(it.quantity) > 0)
  if (validItems.length === 0) {
    showToast('Tambahkan minimal 1 barang dengan kuantitas lebih dari 0', 'error')
    return
  }

  // Validasi ketersediaan stok
  for (const it of validItems) {
    if (Number(it.quantity) > Number(it.available_qty)) {
      showToast(
        `Stok '${it.name}' di outlet asal tidak mencukupi (Tersedia: ${it.available_qty}, Diminta: ${it.quantity})`,
        'error'
      )
      return
    }
  }

  if (!authStore.hasPermission(['inventory:transfer', 'inventory:write'])) {
    showToast('Akses ditolak: Anda tidak memiliki wewenang membuat transfer stok (inventory:transfer)', 'error')
    return
  }

  try {
    isSubmittingTransfer.value = true
    const payload = {
      from_outlet_id: transferForm.value.from_outlet_id,
      to_outlet_id: transferForm.value.to_outlet_id,
      transfer_date: transferForm.value.transfer_date,
      is_immediate: transferForm.value.is_immediate,
      notes: transferForm.value.notes.trim() || undefined,
      items: validItems.map((it) => ({
        product_id: it.product_id,
        quantity: Number(it.quantity),
      })),
    }

    const res = await api.post('/inventory/transfers', payload)
    if (res.data?.success) {
      showToast(res.data.message || 'Transfer stok berhasil diproses!')
      showCreateTransferModal.value = false
      loadTransfers()
      loadInitialData()
    } else {
      showToast(res.data?.message || 'Gagal memproses transfer stok', 'error')
    }
  } catch (err: any) {
    showToast(err.response?.data?.message || 'Terjadi kesalahan sistem', 'error')
  } finally {
    isSubmittingTransfer.value = false
  }
}

async function viewTransferDetail(id: string) {
  try {
    isLoadingDetail.value = true
    showTransferDetailModal.value = true
    const res = await api.get(`/inventory/transfers/${id}`)
    if (res.data?.success) {
      selectedTransferDetail.value = res.data.data
    }
  } catch (err: any) {
    showToast('Gagal memuat rincian transfer', 'error')
    showTransferDetailModal.value = false
  } finally {
    isLoadingDetail.value = false
  }
}

async function openReceiveTransferModal(transfer: any) {
  try {
    isLoadingDetail.value = true
    const res = await api.get(`/inventory/transfers/${transfer.id}`)
    if (res.data?.success) {
      const data = res.data.data
      receiveTransferData.value = data
      receiveForm.value = {
        transfer_id: data.id,
        transfer_number: data.transfer_number,
        from_outlet_name: data.from_outlet_name,
        to_outlet_name: data.to_outlet_name,
        receive_date: new Date().toISOString().slice(0, 10),
        notes: '',
        items: (data.items || []).map((it: any) => ({
          product_id: it.product_id,
          product_code: it.product_code,
          product_name: it.product_name,
          barcode: it.barcode,
          unit_code: it.unit_code || 'PCS',
          shipped_qty: Number(it.quantity || 0),
          received_quantity: Number(it.quantity || 0), // default terima penuh
          cost_price: Number(it.cost_price || 0),
          notes: '',
        })),
      }
      showReceiveTransferModal.value = true
    } else {
      showToast(res.data?.message || 'Gagal memuat rincian transfer', 'error')
    }
  } catch (err: any) {
    showToast(err.response?.data?.message || 'Gagal memuat rincian transfer', 'error')
  } finally {
    isLoadingDetail.value = false
  }
}

function setAllItemsReceivedFull() {
  receiveForm.value.items.forEach((it) => {
    it.received_quantity = it.shipped_qty
  })
  showToast('Semua item diatur sesuai kuantitas dikirim')
}

async function handleConfirmReceiveTransfer() {
  if (!receiveForm.value.receive_date) {
    showToast('Tentukan tanggal penerimaan barang', 'error')
    return
  }
  for (const it of receiveForm.value.items) {
    if (Number(it.received_quantity) < 0) {
      showToast(`Kuantitas barang '${it.product_name}' tidak boleh bernilai negatif`, 'error')
      return
    }
  }

  if (!authStore.hasPermission(['inventory:transfer', 'inventory:write'])) {
    showToast('Akses ditolak: Anda tidak memiliki wewenang menerima transfer stok (inventory:transfer)', 'error')
    return
  }

  try {
    isSubmittingReceive.value = true
    const payload = {
      receive_date: receiveForm.value.receive_date,
      notes: receiveForm.value.notes.trim() || undefined,
      items: receiveForm.value.items.map((it) => ({
        product_id: it.product_id,
        received_quantity: Number(it.received_quantity),
        notes: it.notes.trim() || undefined,
      })),
    }

    const res = await api.post(`/inventory/transfers/${receiveForm.value.transfer_id}/receive`, payload)
    if (res.data?.success) {
      showToast(res.data.message || 'Penerimaan barang transfer berhasil diproses!')
      showReceiveTransferModal.value = false
      loadTransfers()
      loadInitialData()
    } else {
      showToast(res.data?.message || 'Gagal memproses penerimaan transfer', 'error')
    }
  } catch (err: any) {
    showToast(err.response?.data?.message || 'Terjadi kesalahan saat menerima transfer', 'error')
  } finally {
    isSubmittingReceive.value = false
  }
}

function openCancelTransferModal(transfer: any) {
  if (!authStore.hasPermission(['inventory:transfer_cancel', 'inventory:write'])) {
    showToast('Akses ditolak: Anda tidak memiliki wewenang membatalkan transfer stok (inventory:transfer_cancel)', 'error')
    return
  }
  selectedTransferToCancel.value = transfer
  cancelReason.value = ''
  showCancelTransferModal.value = true
}

async function handleConfirmCancelTransfer() {
  if (!cancelReason.value.trim() || cancelReason.value.trim().length < 3) {
    showToast('Harap masukkan alasan pembatalan transfer (minimal 3 karakter)', 'error')
    return
  }

  try {
    isSubmittingCancel.value = true
    const res = await api.post(`/inventory/transfers/${selectedTransferToCancel.value.id}/cancel`, {
      reason: cancelReason.value.trim(),
    })
    if (res.data?.success) {
      showToast(res.data.message || 'Transfer berhasil dibatalkan dan barang telah dikembalikan!')
      showCancelTransferModal.value = false
      loadTransfers()
      loadInitialData()
    } else {
      showToast(res.data?.message || 'Gagal membatalkan transfer', 'error')
    }
  } catch (err: any) {
    showToast(err.response?.data?.message || 'Terjadi kesalahan sistem saat membatalkan transfer', 'error')
  } finally {
    isSubmittingCancel.value = false
  }
}

function printTransferSlip() {
  window.print()
}

function handleLocationChanged() {
  syncWarehouseFromLocation()
  loadInitialData()
}

watch(
  () => locationStore.currentOutletId,
  () => {
    syncWarehouseFromLocation()
    loadInitialData()
  }
)

watch(
  () => activeTab.value,
  (newTab) => {
    if (newTab === 'transfer') {
      loadTransfers()
    }
  }
)

onMounted(() => {
  authStore.fetchCurrentUser()
  loadInitialData()
  window.addEventListener('notane-location-changed', handleLocationChanged)
})

onUnmounted(() => {
  window.removeEventListener('notane-location-changed', handleLocationChanged)
})
</script>

<template>
  <AdminLayout>
    <div class="p-8 space-y-6">
      <!-- Toast Notification -->
      <transition
        enter-active-class="transform ease-out duration-300 transition"
        enter-from-class="translate-y-2 opacity-0 sm:translate-y-0 sm:translate-x-2"
        enter-to-class="translate-y-0 opacity-100 sm:translate-x-0"
        leave-active-class="transition ease-in duration-100"
        leave-from-class="opacity-100"
        leave-to-class="opacity-0"
      >
        <div
          v-if="toastMessage"
          class="fixed bottom-6 right-6 z-50 flex items-center gap-3 px-4 py-3 rounded-xl shadow-xl text-xs font-bold text-white transition-all"
          :class="toastType === 'success' ? 'bg-emerald-600 border border-emerald-500' : 'bg-rose-600 border border-rose-500'"
        >
          <CheckCircle2 v-if="toastType === 'success'" class="w-4 h-4 shrink-0" />
          <AlertCircle v-else class="w-4 h-4 shrink-0" />
          <span>{{ toastMessage }}</span>
        </div>
      </transition>

      <!-- Header -->
      <div class="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h2 class="text-xl font-bold text-slate-900 flex items-center gap-2.5">
            <Icon icon="streamline-ultimate-color:warehouse-storage-2" class="w-6 h-6 drop-shadow-xs" />
            <span>Inventori, Transfer Stok & Mutasi</span>
          </h2>
          <p class="text-xs text-slate-600">Pengendalian stok fisik, transfer antar outlet dengan penjurnalan otomatis, dan jejak kartu stok.</p>
        </div>

        <!-- Navigation Tabs -->
        <div class="flex items-center gap-1.5 bg-slate-200/80 border border-slate-300 p-1 rounded-lg">
          <button
            @click="activeTab = 'opname'"
            class="px-3.5 py-1.5 rounded-md text-xs font-bold transition flex items-center gap-1.5"
            :class="activeTab === 'opname' ? 'bg-white text-emerald-800 shadow-xs border border-slate-300/80' : 'text-slate-600 hover:text-slate-900'"
          >
            <ClipboardCheck class="w-4 h-4 text-emerald-600" />
            <span>Stock Opname Fisik</span>
          </button>
          <button
            @click="activeTab = 'transfer'"
            class="px-3.5 py-1.5 rounded-md text-xs font-bold transition flex items-center gap-1.5"
            :class="activeTab === 'transfer' ? 'bg-white text-emerald-800 shadow-xs border border-slate-300/80' : 'text-slate-600 hover:text-slate-900'"
          >
            <Icon icon="fluent-color:arrow-sync-24" class="w-4 h-4" />
            <span>Transfer Stok Antar Outlet</span>
          </button>
          <button
            @click="activeTab = 'ledger'"
            class="px-3.5 py-1.5 rounded-md text-xs font-bold transition flex items-center gap-1.5"
            :class="activeTab === 'ledger' ? 'bg-white text-emerald-800 shadow-xs border border-slate-300/80' : 'text-slate-600 hover:text-slate-900'"
          >
            <Icon icon="streamline-ultimate-color:task-list-approve" class="w-4 h-4" />
            <span>Kartu Stok (Ledger)</span>
          </button>
          <button
            @click="activeTab = 'fefo'; fetchFefoAlerts()"
            class="px-3.5 py-1.5 rounded-md text-xs font-bold transition flex items-center gap-1.5"
            :class="activeTab === 'fefo' ? 'bg-white text-rose-800 shadow-xs border border-slate-300/80' : 'text-slate-600 hover:text-slate-900'"
          >
            <Icon icon="fluent:calendar-clock-24-filled" class="w-4 h-4 text-rose-600" />
            <span>Alert FEFO (Expired)</span>
            <span v-if="fefoCriticalCount > 0" class="px-1.5 py-0.2 rounded-full bg-rose-500 text-white text-[10px] font-black animate-pulse">
              {{ fefoCriticalCount }}
            </span>
          </button>
        </div>
      </div>

      <!-- Tab 1: Stock Opname -->
      <div v-if="activeTab === 'opname'" class="space-y-4">
        <!-- Header & Action Opname -->
        <div class="bg-white border border-slate-300 rounded-xl p-5 flex flex-col sm:flex-row sm:items-center justify-between gap-4 shadow-sm">
          <div>
            <h3 class="font-bold text-sm text-slate-900 flex items-center gap-2">
              <ClipboardCheck class="w-4 h-4 text-emerald-600" />
              <span>Penyesuaian Fisik Toko (Stock Opname)</span>
            </h3>
            <p class="text-xs text-slate-500 mt-0.5">Input jumlah fisik aktual yang dihitung di rak toko atau gunakan scanner barcode.</p>
          </div>

          <div class="flex items-center gap-2.5">
            <!-- Mode Scan Cepat Toggle -->
            <button
              type="button"
              @click="isFastScanMode = !isFastScanMode"
              class="px-3.5 py-2 rounded-lg text-xs font-bold border transition flex items-center gap-1.5 cursor-pointer shadow-xs"
              :class="isFastScanMode ? 'bg-amber-500 text-white border-amber-600 ring-2 ring-amber-300' : 'bg-slate-50 hover:bg-slate-100 text-slate-700 border-slate-300'"
            >
              <Icon icon="fluent:barcode-scanner-24-filled" class="text-base" />
              <span>{{ isFastScanMode ? '⚡ Mode Scan Cepat: AKTIF' : 'Mode Scan Barcode (+1)' }}</span>
            </button>

            <button
              v-if="authStore.hasPermission('inventory:adjust')"
              @click="handleSaveOpname"
              class="bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-xs px-5 py-2 rounded-lg shadow-sm border border-emerald-700 transition active:scale-95 cursor-pointer"
            >
              Simpan & Bukukan Jurnal
            </button>
          </div>
        </div>

        <!-- Fast Scan Input Banner (Jika Aktif) -->
        <div v-if="isFastScanMode" class="bg-gradient-to-r from-amber-500 to-amber-600 text-white p-4 rounded-xl shadow-md space-y-2 animate-fade-in">
          <div class="flex items-center justify-between text-xs font-bold">
            <span class="flex items-center gap-1.5">
              <Icon icon="fluent:flash-24-filled" class="text-lg" />
              <span>MODE SCAN CEPAT (+1 PER SCAN) AKTIF</span>
            </span>
            <span class="text-amber-100 text-[11px]">Tembakkan scanner barcode ke kemasan produk fisik secara bertubi-tubi</span>
          </div>

          <div class="flex gap-2">
            <div class="relative flex-1">
              <input
                v-model="fastScanBarcode"
                @keyup.enter="handleFastScan"
                ref="fastScanInputRef"
                type="text"
                autofocus
                placeholder="Arahkan barcode scanner kemari (Scan & Enter otomatis menambah +1)..."
                class="w-full px-4 py-2.5 rounded-lg bg-white text-slate-900 font-mono text-sm font-bold border-2 border-white shadow-inner focus:outline-none focus:ring-2 focus:ring-amber-200"
              />
            </div>
            <button
              @click="handleFastScan"
              type="button"
              class="px-5 py-2.5 bg-amber-900 hover:bg-amber-950 text-white font-black text-xs rounded-lg transition"
            >
              +1 Fisik
            </button>
          </div>

          <div v-if="lastScannedItem" class="text-[11px] bg-white/20 p-2 rounded-lg flex items-center justify-between font-medium">
            <span>Terakhir Dihitung: <b>{{ lastScannedItem.name }}</b></span>
            <span>Total Fisik Saat Ini: <b class="text-white text-xs font-mono font-black">{{ formatQty(lastScannedItem.actual_qty) }} {{ lastScannedItem.unit_code }}</b></span>
          </div>
        </div>

        <div class="bg-white border border-slate-300 rounded-xl overflow-hidden shadow-sm">
          <div class="overflow-x-auto">
            <table class="w-full text-left text-xs border-collapse min-w-[750px]">
              <thead class="bg-slate-100 border-b border-slate-300 text-slate-700 font-mono text-[11px] uppercase tracking-wider">
                <tr>
                  <th class="py-3 px-4">SKU</th>
                  <th class="py-3 px-4">Nama Produk</th>
                  <th class="py-3 px-4 text-right">Stok Sistem</th>
                  <th class="py-3 px-4 text-right w-36">Fisik Aktual</th>
                  <th class="py-3 px-4 text-right">Selisih</th>
                  <th class="py-3 px-4 text-center">Satuan</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-slate-200">
                <tr v-for="it in paginatedOpnameItems" :key="it.product_id" class="hover:bg-slate-50 transition">
                  <td class="py-3 px-4 font-mono font-bold text-slate-700">{{ it.sku }}</td>
                  <td class="py-3 px-4 font-bold text-slate-900">{{ it.name }}</td>
                  <td class="py-3 px-4 text-right font-mono text-slate-600">{{ formatQty(it.system_qty) }}</td>
                  <td class="py-2 px-4 text-right">
                    <input
                      type="number"
                      v-model.number="it.actual_qty"
                      class="w-full text-right bg-white border border-slate-300 focus:border-emerald-600 rounded-lg px-2.5 py-1.5 font-mono text-xs font-bold text-slate-900 focus:outline-none"
                    />
                  </td>
                  <td
                    class="py-3 px-4 text-right font-mono font-bold"
                    :class="it.actual_qty - it.system_qty === 0 ? 'text-slate-400' : it.actual_qty - it.system_qty > 0 ? 'text-emerald-700' : 'text-rose-700'"
                  >
                    {{ it.actual_qty - it.system_qty > 0 ? '+' : '' }}{{ formatQty(it.actual_qty - it.system_qty) }}
                  </td>
                  <td class="py-3 px-4 text-center font-mono text-slate-500">{{ it.unit_code }}</td>
                </tr>
              </tbody>
            </table>
          </div>
          <PaginationBar
            v-model:currentPage="opnamePage"
            v-model:pageSize="opnamePageSize"
            :totalItems="opnameItems.length"
          />
        </div>
      </div>

      <!-- Tab 2: Transfer Stok Antar Outlet -->
      <div v-else-if="activeTab === 'transfer'" class="space-y-4">
        <!-- Toolbar & Filter -->
        <div class="bg-white border border-slate-300 rounded-xl p-5 shadow-sm space-y-4">
          <div class="flex flex-wrap items-center justify-between gap-4">
            <div>
              <h3 class="font-bold text-sm text-slate-900 flex items-center gap-2">
                <Icon icon="fluent-color:arrow-sync-24" class="w-5 h-5" />
                <span>Mutasi Transfer Stok Antar Outlet & Gudang</span>
              </h3>
              <p class="text-xs text-slate-500 mt-0.5">
                Alur perpindahan barang mirip pembelian: Pengiriman &rarr; Dalam Perjalanan (In-Transit) &rarr; Verifikasi Penerimaan Fisik di Tujuan.
              </p>
            </div>
            <button
              v-if="authStore.hasPermission(['inventory:transfer', 'inventory:write'])"
              @click="openCreateTransferModal"
              class="inline-flex items-center gap-2 bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-xs px-4 py-2.5 rounded-lg shadow-sm border border-emerald-700 transition active:scale-95"
            >
              <Plus class="w-4 h-4" />
              <span>+ Buat Transfer Stok Baru</span>
            </button>
          </div>

          <!-- Status Filter Pills (Sub-tab Alur Transfer) -->
          <div class="flex flex-wrap items-center gap-2 pt-2 border-t border-slate-100">
            <button
              @click="setTransferStatusFilter('ALL')"
              class="px-3 py-1.5 rounded-lg text-xs font-bold transition flex items-center gap-1.5"
              :class="filterTransferStatus === 'ALL' ? 'bg-slate-900 text-white shadow-xs' : 'bg-slate-100 text-slate-600 hover:bg-slate-200'"
            >
              <span>Semua Dokumen</span>
            </button>
            <button
              @click="setTransferStatusFilter('IN_TRANSIT')"
              class="px-3 py-1.5 rounded-lg text-xs font-bold transition flex items-center gap-1.5"
              :class="filterTransferStatus === 'IN_TRANSIT' ? 'bg-amber-500 text-white shadow-xs' : 'bg-amber-50 text-amber-800 border border-amber-200 hover:bg-amber-100'"
            >
              <Truck class="w-3.5 h-3.5" />
              <span>Dalam Perjalanan (In-Transit)</span>
              <span v-if="inTransitCount > 0" class="px-1.5 py-0.2 rounded-full text-[10px] bg-amber-200 text-amber-900 font-mono font-bold">
                {{ inTransitCount }}
              </span>
            </button>
            <button
              @click="setTransferStatusFilter('MY_INBOUND')"
              class="px-3 py-1.5 rounded-lg text-xs font-bold transition flex items-center gap-1.5"
              :class="filterTransferStatus === 'MY_INBOUND' ? 'bg-emerald-600 text-white shadow-xs' : 'bg-emerald-50 text-emerald-800 border border-emerald-300 hover:bg-emerald-100'"
            >
              <PackageCheck class="w-3.5 h-3.5" />
              <span>Menuju Outlet Ini (Perlu Diterima)</span>
              <span v-if="myInboundCount > 0" class="px-1.5 py-0.2 rounded-full text-[10px] bg-emerald-200 text-emerald-950 font-mono font-bold animate-pulse">
                {{ myInboundCount }}
              </span>
            </button>
            <button
              @click="setTransferStatusFilter('COMPLETED')"
              class="px-3 py-1.5 rounded-lg text-xs font-bold transition flex items-center gap-1.5"
              :class="filterTransferStatus === 'COMPLETED' ? 'bg-emerald-700 text-white shadow-xs' : 'bg-slate-100 text-slate-600 hover:bg-slate-200'"
            >
              <CheckCircle2 class="w-3.5 h-3.5" />
              <span>Selesai Diterima</span>
            </button>
            <button
              @click="setTransferStatusFilter('CANCELLED')"
              class="px-3 py-1.5 rounded-lg text-xs font-bold transition flex items-center gap-1.5"
              :class="filterTransferStatus === 'CANCELLED' ? 'bg-rose-600 text-white shadow-xs' : 'bg-slate-100 text-slate-600 hover:bg-slate-200'"
            >
              <Ban class="w-3.5 h-3.5" />
              <span>Dibatalkan</span>
            </button>
          </div>

          <!-- Secondary Filter Rows: Pencarian, Outlet Asal, Outlet Tujuan -->
          <div class="grid grid-cols-1 md:grid-cols-4 gap-3 pt-2 border-t border-slate-100">
            <!-- Search -->
            <div class="relative">
              <Search class="w-3.5 h-3.5 text-slate-400 absolute left-3 top-2.5" />
              <input
                v-model="transferSearch"
                @input="loadTransfers"
                placeholder="Cari No Transfer / Catatan..."
                class="w-full pl-9 pr-3 py-1.5 bg-slate-50 border border-slate-300 rounded-lg text-xs font-mono focus:bg-white focus:border-emerald-600 focus:outline-none"
              />
            </div>

            <!-- Dari Outlet -->
            <div>
              <select
                v-model="filterFromOutlet"
                @change="loadTransfers"
                class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-1.5 text-xs text-slate-700 focus:bg-white focus:border-emerald-600 focus:outline-none"
              >
                <option value="">Semua Outlet Asal</option>
                <option v-for="o in outletsList" :key="o.id" :value="o.id">
                  Dari: {{ o.name }} ({{ o.code }})
                </option>
              </select>
            </div>

            <!-- Ke Outlet -->
            <div>
              <select
                v-model="filterToOutlet"
                @change="loadTransfers"
                class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-1.5 text-xs text-slate-700 focus:bg-white focus:border-emerald-600 focus:outline-none"
              >
                <option value="">Semua Outlet Tujuan</option>
                <option v-for="o in outletsList" :key="o.id" :value="o.id">
                  Ke: {{ o.name }} ({{ o.code }})
                </option>
              </select>
            </div>

            <!-- Refresh Button -->
            <div class="flex justify-end">
              <button
                @click="loadTransfers"
                class="px-3 py-1.5 text-xs font-bold text-slate-600 hover:text-slate-900 bg-slate-100 border border-slate-300 rounded-lg hover:bg-slate-200 transition"
              >
                Muat Ulang
              </button>
            </div>
          </div>
        </div>

        <!-- Transfer List Table -->
        <div class="bg-white border border-slate-300 rounded-xl overflow-hidden shadow-sm">
          <div class="overflow-x-auto">
            <table class="w-full text-left text-xs border-collapse min-w-[950px]">
              <thead class="bg-slate-100 border-b border-slate-300 text-slate-700 font-mono text-[11px] uppercase tracking-wider">
                <tr>
                  <th class="py-3 px-4">No. Dokumen & Waktu</th>
                  <th class="py-3 px-4">Alur Perpindahan</th>
                  <th class="py-3 px-4 text-center">Kuantitas Fisik</th>
                  <th class="py-3 px-4 text-right">Nilai Mutasi (HPP)</th>
                  <th class="py-3 px-4 text-center">Status Alur</th>
                  <th class="py-3 px-4">Petugas Terkait</th>
                  <th class="py-3 px-4 text-center w-52">Aksi</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-slate-200">
                <tr v-if="transferLoading">
                  <td colspan="7" class="py-12 text-center text-slate-400">
                    <Icon icon="fluent-color:arrow-clockwise-dashes-24" class="w-6 h-6 animate-spin mx-auto mb-2" />
                    <span>Memuat riwayat transfer stok...</span>
                  </td>
                </tr>
                <tr v-else-if="transfers.length === 0">
                  <td colspan="7" class="py-12 text-center text-slate-400 font-sans">
                    <Icon icon="flat-color-icons:package" class="w-10 h-10 mx-auto mb-2 opacity-80" />
                    <span>Tidak ada dokumen transfer stok sesuai kriteria filter.</span>
                  </td>
                </tr>
                <tr v-for="trf in transfers" :key="trf.id" class="hover:bg-slate-50 transition">
                  <td class="py-3 px-4">
                    <div class="font-mono font-bold text-sky-800 text-xs">{{ trf.transfer_number }}</div>
                    <div class="text-[11px] text-slate-500 font-mono mt-0.5">
                      {{ formatDate(trf.transfer_date) }}
                    </div>
                    <div v-if="trf.notes" class="text-[10.5px] text-slate-500 italic mt-0.5 truncate max-w-xs">
                      "{{ trf.notes }}"
                    </div>
                  </td>
                  <td class="py-3 px-4">
                    <div class="flex items-center gap-1.5 flex-wrap">
                      <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded-md bg-rose-50 border border-rose-200 text-rose-800 font-bold text-[11px]">
                        <Building2 class="w-3 h-3 text-rose-600" />
                        {{ trf.from_outlet_name }}
                      </span>
                      <ArrowRight class="w-3.5 h-3.5 text-slate-400 shrink-0" />
                      <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded-md bg-emerald-50 border border-emerald-200 text-emerald-800 font-bold text-[11px]">
                        <Building2 class="w-3 h-3 text-emerald-600" />
                        {{ trf.to_outlet_name }}
                      </span>
                    </div>
                  </td>
                  <td class="py-3 px-4 text-center font-mono">
                    <div class="font-bold text-slate-800">{{ trf.items_count }} SKU</div>
                    <div class="text-[11px] text-slate-500">
                      Kirim: <span class="font-bold text-slate-700">{{ formatQty(trf.total_shipped_qty || 0) }}</span>
                      <span v-if="trf.status === 'COMPLETED'"> &bull; Diterima: <span class="font-bold text-emerald-700">{{ formatQty(trf.total_received_qty || 0) }}</span></span>
                    </div>
                  </td>
                  <td class="py-3 px-4 text-right font-mono font-bold text-emerald-700 text-xs">
                    {{ formatCurrency(trf.total_amount) }}
                  </td>
                  <td class="py-3 px-4 text-center">
                    <span
                      v-if="trf.status === 'IN_TRANSIT'"
                      class="inline-flex items-center gap-1 px-2.5 py-1 rounded-full text-[10.5px] font-bold bg-amber-50 text-amber-800 border border-amber-300 animate-pulse"
                    >
                      <Truck class="w-3.5 h-3.5 text-amber-600" />
                      <span>Dalam Perjalanan</span>
                    </span>
                    <span
                      v-else-if="trf.status === 'COMPLETED'"
                      class="inline-flex items-center gap-1 px-2.5 py-1 rounded-full text-[10.5px] font-bold bg-emerald-50 text-emerald-800 border border-emerald-300"
                    >
                      <CheckCircle2 class="w-3.5 h-3.5 text-emerald-600" />
                      <span>Selesai Diterima</span>
                    </span>
                    <span
                      v-else-if="trf.status === 'CANCELLED'"
                      class="inline-flex items-center gap-1 px-2.5 py-1 rounded-full text-[10.5px] font-bold bg-rose-50 text-rose-800 border border-rose-300"
                    >
                      <Ban class="w-3.5 h-3.5 text-rose-600" />
                      <span>Dibatalkan</span>
                    </span>
                    <span
                      v-else
                      class="inline-flex items-center px-2 py-0.5 rounded-full text-[10.5px] font-bold bg-slate-100 text-slate-700 border border-slate-300"
                    >
                      {{ trf.status }}
                    </span>
                  </td>
                  <td class="py-3 px-4 text-xs font-sans text-slate-600">
                    <div><span class="text-slate-400">Pengirim:</span> {{ trf.creator_name || '-' }}</div>
                    <div v-if="trf.receiver_name"><span class="text-slate-400">Penerima:</span> <span class="font-bold text-emerald-800">{{ trf.receiver_name }}</span></div>
                    <div v-if="trf.canceller_name"><span class="text-slate-400">Dibatalkan:</span> <span class="font-bold text-rose-700">{{ trf.canceller_name }}</span></div>
                  </td>
                  <td class="py-3 px-4 text-center">
                    <div class="flex items-center justify-center gap-1.5 flex-wrap">
                      <!-- Tombol Terima Barang (Jika In-Transit) -->
                      <button
                        v-if="trf.status === 'IN_TRANSIT' && authStore.hasPermission(['inventory:transfer', 'inventory:write'])"
                        @click="openReceiveTransferModal(trf)"
                        title="Verifikasi dan konfirmasi penerimaan fisik barang di outlet tujuan"
                        class="inline-flex items-center gap-1 px-2.5 py-1 rounded-lg text-xs font-bold text-white bg-emerald-600 hover:bg-emerald-700 shadow-xs border border-emerald-700 transition active:scale-95 cursor-pointer"
                      >
                        <PackageCheck class="w-3.5 h-3.5" />
                        <span>Terima</span>
                      </button>

                      <!-- Tombol Batalkan (Jika In-Transit) -->
                      <button
                        v-if="trf.status === 'IN_TRANSIT' && authStore.hasPermission(['inventory:transfer_cancel', 'inventory:write'])"
                        @click="openCancelTransferModal(trf)"
                        title="Batalkan transfer dan kembalikan stok fisik ke outlet asal"
                        class="inline-flex items-center gap-1 px-2 py-1 rounded-lg text-xs font-bold text-rose-700 bg-rose-50 hover:bg-rose-100 border border-rose-200 transition cursor-pointer"
                      >
                        <Ban class="w-3 h-3 text-rose-600" />
                        <span>Batal</span>
                      </button>

                      <!-- Tombol Rincian -->
                      <button
                        @click="viewTransferDetail(trf.id)"
                        class="inline-flex items-center gap-1 px-2 py-1 rounded-lg text-xs font-bold text-slate-700 bg-slate-100 hover:bg-slate-200 border border-slate-300 transition cursor-pointer"
                      >
                        <Eye class="w-3.5 h-3.5 text-slate-600" />
                        <span>Rincian</span>
                      </button>
                    </div>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
          <PaginationBar
            v-model:currentPage="transferPage"
            v-model:pageSize="transferPageSize"
            :totalItems="totalTransfers"
            @update:currentPage="loadTransfers"
          />
        </div>
      </div>

      <!-- Tab 3: Kartu Stok (Ledger) -->
      <div v-else-if="activeTab === 'ledger'" class="space-y-4">
        <div class="bg-white border border-slate-300 rounded-xl p-5 flex flex-wrap items-center gap-4 shadow-sm">
          <label class="text-xs font-bold text-slate-700">Pilih Produk:</label>
          <div class="w-80">
            <SearchableSelect
              v-model="selectedProductForLedger"
              api-endpoint="/products"
              secondary-key="sku"
              placeholder="-- Pilih Produk --"
              search-placeholder="Cari SKU / nama produk..."
              @change="loadLedger"
            />
          </div>
          <button
            @click="loadLedger"
            class="bg-slate-100 border border-slate-300 hover:bg-slate-200 text-slate-700 font-bold text-xs px-3.5 py-2 rounded-lg transition"
          >
            Tampilkan Kartu Stok
          </button>
        </div>

        <div class="bg-white border border-slate-300 rounded-xl overflow-hidden shadow-sm">
          <div class="overflow-x-auto">
            <table class="w-full text-left text-xs border-collapse min-w-[750px]">
              <thead class="bg-slate-100 border-b border-slate-300 text-slate-700 font-mono text-[11px] uppercase tracking-wider">
                <tr>
                  <th class="py-3 px-4">Waktu</th>
                  <th class="py-3 px-4">Tipe Referensi</th>
                  <th class="py-3 px-4">Catatan / Keterangan</th>
                  <th class="py-3 px-4 text-right">Masuk</th>
                  <th class="py-3 px-4 text-right">Keluar</th>
                  <th class="py-3 px-4 text-right">Saldo Akhir</th>
                  <th class="py-3 px-4 text-right">HPP Mutasi</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-slate-200">
                <tr v-if="paginatedLedgerMutations.length === 0">
                  <td colspan="7" class="py-12 text-center text-slate-400 font-sans">Belum ada riwayat mutasi kartu stok untuk produk ini.</td>
                </tr>
                <tr v-for="m in paginatedLedgerMutations" :key="m.id" class="hover:bg-slate-50 transition">
                  <td class="py-3 px-4 font-mono text-slate-600">{{ formatDate(m.created_at) }}</td>
                  <td class="py-3 px-4">
                    <span class="bg-slate-100 border border-slate-300 text-slate-700 text-[10px] px-2 py-0.5 rounded font-mono font-bold">
                      {{ m.reference_type }}
                    </span>
                  </td>
                  <td class="py-3 px-4 text-slate-700 font-sans">{{ m.notes || '-' }}</td>
                  <td class="py-3 px-4 text-right font-mono font-bold text-emerald-700">
                    {{ m.qty_in > 0 ? '+' + formatQty(m.qty_in) : '-' }}
                  </td>
                  <td class="py-3 px-4 text-right font-mono font-bold text-rose-700">
                    {{ m.qty_out > 0 ? '-' + formatQty(m.qty_out) : '-' }}
                  </td>
                  <td class="py-3 px-4 text-right font-mono font-bold text-slate-900 bg-slate-50">
                    {{ formatQty(m.balance) }}
                  </td>
                  <td class="py-3 px-4 text-right font-mono text-slate-700">{{ formatCurrency(m.cost_price) }}</td>
                </tr>
              </tbody>
            </table>
          </div>
          <PaginationBar
            v-model:currentPage="ledgerPage"
            v-model:pageSize="ledgerPageSize"
            :totalItems="ledgerMutations.length"
          />
        </div>
      </div>

      <!-- Tab 4: Peringatan FEFO & Kadaluwarsa Produk (Prioritas 2) -->
      <div v-else-if="activeTab === 'fefo'" class="space-y-4">
        <div class="bg-white border border-slate-300 rounded-xl p-5 flex flex-col sm:flex-row sm:items-center justify-between gap-4 shadow-sm">
          <div>
            <h3 class="font-bold text-sm text-slate-900 flex items-center gap-2">
              <Icon icon="fluent:calendar-clock-24-filled" class="text-rose-600 text-lg" />
              <span>Peringatan Kadaluwarsa & Sistem FEFO (First-Expired, First-Out)</span>
            </h3>
            <p class="text-xs text-slate-500 mt-0.5">
              Pantau batch produk yang mendekati tanggal expired untuk segera dipajang di rak depan dengan label promo atau diretur ke distributor.
            </p>
          </div>

          <!-- Filter Days, Search & Refresh -->
          <div class="flex items-center gap-2">
            <div class="relative">
              <input
                v-model="fefoSearch"
                placeholder="Cari nama produk / SKU / batch..."
                class="w-56 px-3 py-2 text-xs bg-slate-50 border border-slate-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-rose-500 font-medium"
              />
            </div>

            <select
              v-model.number="fefoDays"
              @change="fetchFefoAlerts"
              class="px-3 py-2 text-xs font-bold bg-slate-50 border border-slate-300 rounded-lg focus:ring-2 focus:ring-rose-500"
            >
              <option :value="14">Expired &lt; 14 Hari (Kritis)</option>
              <option :value="30">Expired &lt; 30 Hari (Bulan Ini)</option>
              <option :value="60">Expired &lt; 60 Hari (2 Bulan)</option>
              <option :value="90">Expired &lt; 90 Hari (3 Bulan)</option>
            </select>

            <button
              @click="fetchFefoAlerts"
              :disabled="isLoadingFefo"
              class="px-3.5 py-2 bg-slate-100 hover:bg-slate-200 text-slate-700 font-bold text-xs rounded-lg border border-slate-300 flex items-center gap-1.5 transition cursor-pointer"
            >
              <Icon v-if="isLoadingFefo" icon="fluent:spinner-ios-20-regular" class="animate-spin text-sm" />
              <Icon v-else icon="fluent:arrow-clockwise-20-filled" class="text-sm" />
              <span>Segarkan</span>
            </button>
          </div>
        </div>

        <!-- Table FEFO -->
        <div class="bg-white border border-slate-300 rounded-xl overflow-hidden shadow-sm">
          <div v-if="isLoadingFefo" class="p-12 text-center text-slate-400">
            <Icon icon="fluent:spinner-ios-20-regular" class="animate-spin text-3xl mx-auto mb-2 text-rose-600" />
            <span>Memeriksa tanggal kadaluwarsa batch produk...</span>
          </div>

          <div v-else-if="filteredFefoAlerts.length === 0" class="p-12 text-center text-slate-500">
            <Icon icon="fluent:checkmark-circle-24-filled" class="text-4xl text-emerald-500 mx-auto mb-2" />
            <h4 class="font-bold text-sm text-slate-800">Semua Stok Aman!</h4>
            <p class="text-xs text-slate-400 mt-1">Tidak ada produk yang mendekati tanggal kadaluwarsa dalam rentang {{ fefoDays }} hari ke depan.</p>
          </div>

          <div v-else class="overflow-x-auto">
            <table class="w-full text-left text-xs border-collapse min-w-[850px]">
              <thead class="bg-slate-100 border-b border-slate-300 text-slate-700 font-mono text-[11px] uppercase tracking-wider">
                <tr>
                  <th class="py-3 px-4">Produk & SKU</th>
                  <th class="py-3 px-4">No. Batch / Lot</th>
                  <th class="py-3 px-4">Tgl Expired</th>
                  <th class="py-3 px-4 text-center">Sisa Waktu</th>
                  <th class="py-3 px-4 text-right">Qty Masuk</th>
                  <th class="py-3 px-4">Dokumen GRN & Pemasok</th>
                  <th class="py-3 px-4 text-center">Rekomendasi Tindakan</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-slate-200">
                <tr
                  v-for="item in filteredFefoAlerts"
                  :key="item.grn_item_id"
                  class="hover:bg-slate-50 transition-colors"
                  :class="{
                    'bg-rose-50/50': item.urgency === 'EXPIRED',
                    'bg-orange-50/40': item.urgency === 'CRITICAL',
                    'bg-amber-50/30': item.urgency === 'WARNING',
                  }"
                >
                  <td class="py-3 px-4">
                    <div class="font-bold text-slate-900">{{ item.product_name }}</div>
                    <div class="text-[11px] text-slate-500 font-mono">{{ item.product_sku }} <span v-if="item.product_barcode">&bull; {{ item.product_barcode }}</span></div>
                  </td>

                  <td class="py-3 px-4 font-mono font-bold text-slate-700">
                    {{ item.batch_number || '-' }}
                  </td>

                  <td class="py-3 px-4 font-sans font-bold text-slate-800">
                    {{ new Date(item.expiry_date).toLocaleDateString('id-ID', { year: 'numeric', month: 'long', day: 'numeric' }) }}
                  </td>

                  <td class="py-3 px-4 text-center">
                    <span
                      class="px-2.5 py-1 rounded-full text-[10px] font-black uppercase tracking-wider inline-flex items-center gap-1"
                      :class="{
                        'bg-rose-600 text-white': item.urgency === 'EXPIRED',
                        'bg-orange-500 text-white': item.urgency === 'CRITICAL',
                        'bg-amber-400 text-amber-950': item.urgency === 'WARNING',
                        'bg-sky-100 text-sky-800 border border-sky-300': item.urgency === 'NOTICE',
                      }"
                    >
                      <template v-if="item.days_remaining < 0">
                        LEWAT {{ Math.abs(item.days_remaining) }} HARI (EXPIRED)
                      </template>
                      <template v-else-if="item.days_remaining === 0">
                        HARI INI EXPIRED
                      </template>
                      <template v-else>
                        {{ item.days_remaining }} HARI LAGI
                      </template>
                    </span>
                  </td>

                  <td class="py-3 px-4 text-right font-mono font-bold text-slate-800">
                    {{ formatQty(item.received_qty) }} {{ item.unit_code || 'PCS' }}
                  </td>

                  <td class="py-3 px-4">
                    <div class="font-bold font-mono text-slate-700 text-[11px]">{{ item.grn_number }}</div>
                    <div class="text-[11px] text-slate-500">{{ item.supplier_name || 'Pemasok Terdaftar' }}</div>
                  </td>

                  <td class="py-3 px-4 text-center">
                    <span
                      v-if="item.urgency === 'EXPIRED'"
                      class="text-[11px] font-bold text-rose-700 bg-rose-100 px-2 py-0.5 rounded border border-rose-300"
                    >
                      Tarik Dari Rak / Buang (Waste)
                    </span>
                    <span
                      v-else-if="item.urgency === 'CRITICAL'"
                      class="text-[11px] font-bold text-orange-800 bg-orange-100 px-2 py-0.5 rounded border border-orange-300"
                    >
                      Display Depan / Promo Diskon Segera
                    </span>
                    <span
                      v-else
                      class="text-[11px] font-bold text-slate-600 bg-slate-100 px-2 py-0.5 rounded border border-slate-200"
                    >
                      Prioritas Jual FEFO
                    </span>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>

      <!-- MODAL: BUAT TRANSFER STOK BARU -->
      <div
        v-if="showCreateTransferModal"
        class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/60 backdrop-blur-xs overflow-y-auto"
      >
        <div class="bg-white rounded-2xl border border-slate-200 shadow-2xl w-full max-w-4xl overflow-hidden flex flex-col max-h-[92vh]">
          <!-- Modal Header -->
          <div class="px-6 py-4 bg-gradient-to-r from-emerald-800 to-teal-800 text-white flex items-center justify-between">
            <div class="flex items-center gap-3">
              <div class="p-2 rounded-xl bg-white/10 border border-white/20">
                <Icon icon="fluent-color:arrow-sync-24" class="w-6 h-6" />
              </div>
              <div>
                <h3 class="font-extrabold text-base">Buat Formulir Transfer Stok Antar Outlet</h3>
                <p class="text-emerald-100 text-xs mt-0.5">Memindahkan persediaan fisik antar cabang/gudang dan otomatis menjurnal akun CoA masing-masing.</p>
              </div>
            </div>
            <button
              @click="showCreateTransferModal = false"
              class="p-1 rounded-lg text-white/80 hover:text-white hover:bg-white/10 transition"
            >
              <X class="w-5 h-5" />
            </button>
          </div>

          <!-- Modal Body Form -->
          <div class="p-6 overflow-y-auto space-y-5 flex-1">
            <!-- Pilihan Alur Kerja Transfer: 2-Tahap vs Instan -->
            <div class="grid grid-cols-1 md:grid-cols-2 gap-3 p-3 bg-slate-100/80 rounded-xl border border-slate-200">
              <label
                class="flex items-start gap-3 p-3 rounded-lg border cursor-pointer transition"
                :class="!transferForm.is_immediate ? 'bg-white border-emerald-500 shadow-xs ring-1 ring-emerald-500' : 'bg-white/60 border-slate-200 hover:bg-white'"
              >
                <input
                  type="radio"
                  :value="false"
                  v-model="transferForm.is_immediate"
                  class="mt-1 text-emerald-600 focus:ring-emerald-500"
                />
                <div>
                  <div class="font-bold text-xs text-slate-900 flex items-center gap-1.5 flex-wrap">
                    <Truck class="w-3.5 h-3.5 text-amber-600" />
                    <span>Alur 2-Tahap (Kirim &rarr; In-Transit &rarr; Terima)</span>
                    <span class="px-1.5 py-0.5 rounded text-[10px] bg-emerald-100 text-emerald-800 font-bold">Rekomendasi</span>
                  </div>
                  <p class="text-[11px] text-slate-500 mt-1 leading-relaxed">
                    Stok langsung keluar dari outlet asal dan masuk status <strong class="text-amber-800">Dalam Perjalanan</strong> (In-Transit). Outlet penerima akan memeriksa fisik barang yang tiba sebelum memasukkan ke stok.
                  </p>
                </div>
              </label>

              <label
                class="flex items-start gap-3 p-3 rounded-lg border cursor-pointer transition"
                :class="transferForm.is_immediate ? 'bg-white border-emerald-500 shadow-xs ring-1 ring-emerald-500' : 'bg-white/60 border-slate-200 hover:bg-white'"
              >
                <input
                  type="radio"
                  :value="true"
                  v-model="transferForm.is_immediate"
                  class="mt-1 text-emerald-600 focus:ring-emerald-500"
                />
                <div>
                  <div class="font-bold text-xs text-slate-900 flex items-center gap-1.5">
                    <CheckCircle2 class="w-3.5 h-3.5 text-emerald-600" />
                    <span>Transfer Instan (Langsung Diterima)</span>
                  </div>
                  <p class="text-[11px] text-slate-500 mt-1 leading-relaxed">
                    Stok seketika berpindah langsung dari outlet asal ke outlet tujuan tanpa menunggu konfirmasi penerimaan (cocok untuk gudang yang bersebelahan/satu lokasi).
                  </p>
                </div>
              </label>
            </div>

            <!-- Header Inputs: Asal, Tujuan, Tanggal, Catatan -->
            <div class="grid grid-cols-1 md:grid-cols-3 gap-4 bg-slate-50 p-4 rounded-xl border border-slate-200">
              <!-- Outlet Asal -->
              <div>
                <label class="block text-xs font-bold text-slate-700 mb-1.5 flex items-center gap-1.5">
                  <Building2 class="w-3.5 h-3.5 text-rose-600" />
                  <span>Outlet / Gudang Asal (Pengirim) *</span>
                </label>
                <select
                  v-model="transferForm.from_outlet_id"
                  class="w-full bg-white border border-slate-300 rounded-lg px-3 py-2 text-xs font-bold text-slate-800 focus:border-emerald-600 focus:outline-none"
                >
                  <option v-for="o in outletsList" :key="o.id" :value="o.id">
                    {{ o.is_warehouse ? '🏢 [Gudang]' : '🏪 [Outlet]' }} {{ o.name }} ({{ o.code }})
                  </option>
                </select>
                <p class="text-[10.5px] text-slate-500 mt-1">Stok & akun persediaan lokasi ini akan dikurangi (Kredit).</p>
              </div>

              <!-- Outlet Tujuan -->
              <div>
                <label class="block text-xs font-bold text-slate-700 mb-1.5 flex items-center gap-1.5">
                  <Building2 class="w-3.5 h-3.5 text-emerald-600" />
                  <span>Outlet / Gudang Tujuan (Penerima) *</span>
                </label>
                <select
                  v-model="transferForm.to_outlet_id"
                  class="w-full bg-white border border-slate-300 rounded-lg px-3 py-2 text-xs font-bold text-slate-800 focus:border-emerald-600 focus:outline-none"
                >
                  <option v-for="o in outletsList" :key="o.id" :value="o.id" :disabled="o.id === transferForm.from_outlet_id">
                    {{ o.is_warehouse ? '🏢 [Gudang]' : '🏪 [Outlet]' }} {{ o.name }} ({{ o.code }}) {{ o.id === transferForm.from_outlet_id ? '(Asal)' : '' }}
                  </option>
                </select>
                <p class="text-[10.5px] text-slate-500 mt-1">Stok & akun persediaan outlet ini akan bertambah (Debit).</p>
              </div>

              <!-- Tanggal Transfer -->
              <div>
                <label class="block text-xs font-bold text-slate-700 mb-1.5">Tanggal Mutasi Transfer *</label>
                <input
                  type="date"
                  v-model="transferForm.transfer_date"
                  class="w-full bg-white border border-slate-300 rounded-lg px-3 py-2 text-xs font-mono text-slate-800 focus:border-emerald-600 focus:outline-none"
                />
              </div>

              <!-- Catatan -->
              <div class="md:col-span-3">
                <label class="block text-xs font-bold text-slate-700 mb-1">Keterangan / Alasan Transfer (Opsional)</label>
                <input
                  v-model="transferForm.notes"
                  placeholder="Misal: Penyeimbangan stok akhir pekan dari gudang pusat ke cabang..."
                  class="w-full bg-white border border-slate-300 rounded-lg px-3 py-2 text-xs text-slate-800 focus:border-emerald-600 focus:outline-none"
                />
              </div>
            </div>

            <!-- Items Table Section -->
            <div class="space-y-3">
              <div class="flex flex-wrap items-center justify-between gap-2">
                <h4 class="text-xs font-bold uppercase tracking-wider text-slate-700 font-mono flex items-center gap-2">
                  <Icon icon="flat-color-icons:package" class="w-4 h-4" />
                  <span>Daftar Barang yang Ditransfer</span>
                </h4>
                <div class="flex items-center gap-2">
                  <button
                    type="button"
                    @click="openTransferProductPicker"
                    class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-teal-50 hover:bg-teal-100 text-teal-800 border border-teal-200 text-xs font-bold transition shadow-xs cursor-pointer"
                  >
                    <Search class="w-3.5 h-3.5 text-teal-700" />
                    <span>Pilih dari Katalog Produk</span>
                  </button>
                  <button
                    type="button"
                    @click="addTransferItem"
                    class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-emerald-50 hover:bg-emerald-100 text-emerald-800 border border-emerald-200 text-xs font-bold transition shadow-xs cursor-pointer"
                  >
                    <Plus class="w-3.5 h-3.5 text-emerald-700" />
                    <span>Tambah Baris Produk</span>
                  </button>
                </div>
              </div>

              <div class="border border-slate-300 rounded-xl bg-white shadow-xs relative">
                <table class="w-full text-left text-xs border-collapse">
                  <thead class="bg-slate-100 border-b border-slate-300 text-slate-700 font-mono text-[11px] uppercase">
                    <tr>
                      <th class="py-2.5 px-3 min-w-[300px]">Pilih Produk</th>
                      <th class="py-2.5 px-3 text-right w-24">Stok Asal</th>
                      <th class="py-2.5 px-3 text-right w-28">Qty Transfer *</th>
                      <th class="py-2.5 px-3 text-right w-32">HPP Satuan</th>
                      <th class="py-2.5 px-3 text-right w-36">Subtotal Nilai</th>
                      <th class="py-2.5 px-3 text-center w-12">Hapus</th>
                    </tr>
                  </thead>
                  <tbody class="divide-y divide-slate-200">
                    <tr v-for="(row, idx) in transferForm.items" :key="idx" class="hover:bg-slate-50/60 transition">
                      <!-- Product Picker -->
                      <td class="py-2 px-3 min-w-[300px] relative align-top">
                        <SearchableSelect
                          :model-value="row.product_id"
                          api-endpoint="/products"
                          :extra-params="{ outlet_id: transferForm.from_outlet_id }"
                          secondary-key="sku"
                          placeholder="-- Pilih Produk --"
                          search-placeholder="Cari nama, SKU, atau barcode..."
                          @change="(_: any, prod: any) => onProductSelectedInTransfer(row, prod)"
                        />
                      </td>

                      <!-- Stok Asal -->
                      <td class="py-2 px-3 text-right font-mono font-bold align-middle" :class="row.available_qty <= 0 ? 'text-rose-600' : 'text-slate-700'">
                        {{ formatQty(row.available_qty) }}
                      </td>

                      <!-- Qty Transfer -->
                      <td class="py-2 px-3 text-right align-middle">
                        <input
                          type="number"
                          min="1"
                          :max="row.available_qty"
                          v-model.number="row.quantity"
                          @input="onQtyChanged(row)"
                          class="w-full text-right bg-white border border-slate-300 rounded-lg px-2.5 py-1.5 font-mono text-xs font-bold text-slate-900 focus:border-emerald-600 focus:outline-none"
                          :class="(row.quantity > row.available_qty || row.available_qty <= 0) ? 'border-rose-500 bg-rose-50 text-rose-800' : ''"
                        />
                        <div v-if="row.available_qty <= 0" class="text-[10px] text-rose-600 font-bold mt-0.5">
                          Stok kosong (0)
                        </div>
                        <div v-else-if="row.quantity > row.available_qty" class="text-[10px] text-rose-600 font-bold mt-0.5">
                          Melebihi stok (Maks: {{ row.available_qty }})
                        </div>
                      </td>

                      <!-- HPP -->
                      <td class="py-2 px-3 text-right font-mono text-slate-600 align-middle">
                        {{ formatCurrency(row.cost_price) }}
                      </td>

                      <!-- Subtotal -->
                      <td class="py-2 px-3 text-right font-mono font-bold text-emerald-800 align-middle">
                        {{ formatCurrency(row.quantity * row.cost_price) }}
                      </td>

                      <!-- Delete -->
                      <td class="py-2 px-3 text-center align-middle">
                        <button
                          type="button"
                          @click="removeTransferItem(idx)"
                          class="p-1 rounded-md text-slate-400 hover:text-rose-600 hover:bg-rose-50 transition"
                          :disabled="transferForm.items.length <= 1"
                        >
                          <Trash2 class="w-4 h-4" />
                        </button>
                      </td>
                    </tr>
                  </tbody>
                  <tfoot class="bg-slate-50 border-t border-slate-300 font-bold">
                    <tr>
                      <td colspan="4" class="py-3 px-4 text-right text-slate-700 font-mono text-xs uppercase">
                        Total Nilai Mutasi Persediaan:
                      </td>
                      <td class="py-3 px-3 text-right font-mono text-sm text-emerald-800 font-extrabold">
                        {{ formatCurrency(calculateTransferTotal()) }}
                      </td>
                      <td></td>
                    </tr>
                  </tfoot>
                </table>
              </div>
            </div>
          </div>

          <!-- Modal Footer Actions -->
          <div class="px-6 py-4 bg-slate-100 border-t border-slate-200 flex items-center justify-between">
            <span class="text-xs text-slate-500 font-mono">
              * Jurnal double-entry akan otomatis terbentuk saat transfer disimpan.
            </span>
            <div class="flex items-center gap-3">
              <button
                type="button"
                @click="showCreateTransferModal = false"
                class="px-4 py-2 rounded-lg text-xs font-bold text-slate-700 hover:bg-slate-200 transition"
              >
                Batal
              </button>
              <button
                type="button"
                @click="handleSaveTransfer"
                :disabled="isSubmittingTransfer"
                class="px-5 py-2.5 rounded-lg text-xs font-bold text-white bg-emerald-600 hover:bg-emerald-700 shadow-md border border-emerald-700 transition active:scale-95 disabled:opacity-50"
              >
                {{ isSubmittingTransfer ? 'Memproses Transfer...' : 'Simpan & Bukukan Transfer Stok' }}
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- MODAL: PILIH BARANG DARI KATALOG PRODUK UNTUK TRANSFER -->
      <div
        v-if="showTransferProductPicker"
        class="fixed inset-0 z-[70] flex items-center justify-center p-4 bg-slate-950/70 backdrop-blur-xs overflow-y-auto"
      >
        <div class="bg-white rounded-2xl border border-slate-200 shadow-2xl w-full max-w-2xl max-h-[85vh] flex flex-col overflow-hidden animate-scaleIn">
          <!-- Header -->
          <div class="px-5 py-3.5 bg-slate-900 text-white flex items-center justify-between shrink-0">
            <div class="flex items-center gap-2">
              <Icon icon="flat-color-icons:package" class="w-5 h-5" />
              <span class="font-bold text-sm">Pilih Barang dari Master Produk</span>
            </div>
            <button @click="showTransferProductPicker = false" class="text-slate-400 hover:text-white transition cursor-pointer">
              <X class="w-5 h-5" />
            </button>
          </div>

          <!-- Search Input Bar -->
          <div class="p-4 border-b border-slate-200 bg-slate-50 shrink-0">
            <div class="relative">
              <Search class="w-4 h-4 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2" />
              <input
                v-model="transferProductSearch"
                @input="searchTransferProducts"
                type="text"
                placeholder="Ketik nama produk, SKU, atau scan barcode..."
                class="w-full pl-9 pr-4 py-2 text-xs rounded-lg border border-slate-300 focus:outline-none focus:ring-2 focus:ring-emerald-500/20 focus:border-emerald-600 bg-white"
                autofocus
              />
            </div>
            <div class="flex items-center justify-between mt-2 text-[11px] text-slate-500">
              <span>Menampilkan stok di outlet asal: <strong>{{ outletsList.find(o => o.id === transferForm.from_outlet_id)?.name || 'Pilih Outlet' }}</strong></span>
              <span v-if="transferProductResults.length > 0">{{ transferProductResults.length }} produk ditemukan</span>
            </div>
          </div>

          <!-- Results List -->
          <div class="p-4 overflow-y-auto flex-1 space-y-2 text-xs">
            <div v-if="isSearchingTransferProducts" class="py-12 text-center text-slate-400 flex flex-col items-center justify-center gap-2">
              <Icon icon="fluent-color:arrow-clockwise-dashes-24" class="w-6 h-6 animate-spin" />
              <span>Mencari produk...</span>
            </div>
            <div v-else-if="transferProductResults.length === 0" class="py-12 text-center text-slate-400">
              <Icon icon="flat-color-icons:package" class="w-10 h-10 mx-auto mb-2 opacity-50" />
              <span>Tidak ada produk yang cocok dengan "{{ transferProductSearch }}"</span>
            </div>
            <div
              v-else
              v-for="prod in transferProductResults"
              :key="prod.id"
              class="flex items-center justify-between p-3 rounded-xl border border-slate-200 hover:border-emerald-500 hover:bg-emerald-50/40 transition group"
            >
              <div class="flex-1 min-w-0 pr-3">
                <div class="font-extrabold text-slate-900 text-xs truncate">{{ prod.name }}</div>
                <div class="text-[11px] text-slate-500 font-mono flex items-center gap-3 mt-0.5">
                  <span>SKU: {{ prod.sku || '-' }}</span>
                  <span v-if="prod.barcode">Barcode: {{ prod.barcode }}</span>
                  <span>HPP: {{ formatCurrency(prod.cost_price) }}</span>
                </div>
              </div>

              <div class="flex items-center gap-3 shrink-0">
                <div class="text-right">
                  <div
                    class="font-mono font-bold text-xs px-2 py-0.5 rounded border"
                    :class="Number(prod.stock_qty || 0) <= 0 ? 'bg-rose-50 border-rose-200 text-rose-700' : 'bg-emerald-50 border-emerald-200 text-emerald-800'"
                  >
                    Stok: {{ formatQty(prod.stock_qty || 0) }} {{ prod.unit_code || 'PCS' }}
                  </div>
                </div>
                <button
                  type="button"
                  @click="addProductToTransfer(prod)"
                  class="px-3 py-1.5 rounded-lg bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-xs flex items-center gap-1 transition shadow-xs active:scale-95 cursor-pointer"
                >
                  <Plus class="w-3.5 h-3.5" />
                  <span>Pilih</span>
                </button>
              </div>
            </div>
          </div>

          <!-- Footer -->
          <div class="bg-slate-100 px-5 py-2.5 border-t border-slate-200 flex items-center justify-between text-xs text-slate-500 shrink-0">
            <span>Klik "Pilih" untuk menambahkan produk ke daftar transfer stok.</span>
            <button
              type="button"
              @click="showTransferProductPicker = false"
              class="px-3 py-1.5 rounded-lg bg-white border border-slate-300 hover:bg-slate-50 text-slate-700 font-bold transition cursor-pointer"
            >
              Selesai
            </button>
          </div>
        </div>
      </div>

      <!-- MODAL: PENERIMAAN BARANG TRANSFER (GOODS RECEIPT / INBOUND VERIFICATION) -->
      <div
        v-if="showReceiveTransferModal"
        class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/60 backdrop-blur-xs overflow-y-auto"
      >
        <div class="bg-white rounded-2xl border border-slate-200 shadow-2xl w-full max-w-3xl overflow-hidden flex flex-col max-h-[90vh]">
          <!-- Modal Header -->
          <div class="px-6 py-4 bg-gradient-to-r from-emerald-800 to-teal-800 text-white flex items-center justify-between">
            <div class="flex items-center gap-3">
              <div class="p-2 rounded-xl bg-white/10 border border-white/20">
                <PackageCheck class="w-6 h-6 text-emerald-300" />
              </div>
              <div>
                <h3 class="font-extrabold text-base">Penerimaan & Verifikasi Fisik Barang Transfer</h3>
                <p class="text-emerald-100 text-xs mt-0.5">
                  Verifikasi kuantitas fisik barang yang tiba di outlet tujuan sebelum dimasukkan ke stok.
                </p>
              </div>
            </div>
            <button
              @click="showReceiveTransferModal = false"
              class="p-1 rounded-lg text-white/80 hover:text-white hover:bg-white/10 transition cursor-pointer"
            >
              <X class="w-5 h-5" />
            </button>
          </div>

          <!-- Modal Body Form -->
          <div class="p-6 overflow-y-auto space-y-5 flex-1">
            <!-- Header Info Transfer -->
            <div class="grid grid-cols-1 md:grid-cols-3 gap-4 bg-slate-50 p-4 rounded-xl border border-slate-200 text-xs">
              <div>
                <span class="text-slate-400 font-mono uppercase text-[10px]">No. Dokumen Transfer</span>
                <div class="font-mono font-bold text-sky-800 text-sm mt-0.5">{{ receiveForm.transfer_number }}</div>
              </div>
              <div>
                <span class="text-slate-400 font-mono uppercase text-[10px]">Dari Outlet Asal</span>
                <div class="font-bold text-rose-800 text-sm mt-0.5">{{ receiveForm.from_outlet_name }}</div>
              </div>
              <div>
                <span class="text-slate-400 font-mono uppercase text-[10px]">Outlet Penerima (Tujuan)</span>
                <div class="font-bold text-emerald-800 text-sm mt-0.5">{{ receiveForm.to_outlet_name }}</div>
              </div>
            </div>

            <!-- Tanggal Penerimaan & Catatan -->
            <div class="grid grid-cols-1 md:grid-cols-2 gap-4 bg-emerald-50/40 p-4 rounded-xl border border-emerald-200">
              <div>
                <label class="block text-xs font-bold text-slate-700 mb-1.5 flex items-center gap-1.5">
                  <Clock class="w-3.5 h-3.5 text-emerald-700" />
                  <span>Tanggal Fisik Diterima *</span>
                </label>
                <input
                  type="date"
                  v-model="receiveForm.receive_date"
                  class="w-full bg-white border border-slate-300 rounded-lg px-3 py-2 text-xs font-mono text-slate-800 focus:border-emerald-600 focus:outline-none"
                />
              </div>

              <div>
                <label class="block text-xs font-bold text-slate-700 mb-1.5">Catatan Kondisi / Ekspedisi</label>
                <input
                  v-model="receiveForm.notes"
                  placeholder="Misal: Diterima dalam kondisi baik dan tersegel rapi..."
                  class="w-full bg-white border border-slate-300 rounded-lg px-3 py-2 text-xs text-slate-800 focus:border-emerald-600 focus:outline-none"
                />
              </div>
            </div>

            <!-- Items Table Section -->
            <div class="space-y-2">
              <div class="flex items-center justify-between">
                <h4 class="text-xs font-bold font-mono uppercase text-slate-700 tracking-wider">
                  Verifikasi Kuantitas Barang yang Tiba
                </h4>
                <button
                  type="button"
                  @click="setAllItemsReceivedFull"
                  class="inline-flex items-center gap-1.5 px-3 py-1 text-xs font-bold text-emerald-700 bg-emerald-50 hover:bg-emerald-100 border border-emerald-300 rounded-lg transition cursor-pointer shadow-2xs"
                >
                  <Check class="w-3.5 h-3.5" />
                  <span>Terima Semua Sesuai Surat Jalan</span>
                </button>
              </div>

              <div class="border border-slate-300 rounded-xl overflow-hidden shadow-xs">
                <table class="w-full text-left text-xs border-collapse">
                  <thead class="bg-slate-100 border-b border-slate-300 text-slate-700 font-mono text-[11px] uppercase">
                    <tr>
                      <th class="py-2.5 px-3">SKU & Nama Barang</th>
                      <th class="py-2.5 px-3 text-center">Dikirim</th>
                      <th class="py-2.5 px-3 text-center w-36">Diterima Fisik</th>
                      <th class="py-2.5 px-3 text-center">Selisih</th>
                      <th class="py-2.5 px-3">Catatan Item</th>
                    </tr>
                  </thead>
                  <tbody class="divide-y divide-slate-200">
                    <tr v-for="it in receiveForm.items" :key="it.product_id" class="hover:bg-slate-50 transition">
                      <td class="py-2.5 px-3">
                        <div class="font-bold text-slate-900">{{ it.product_name }}</div>
                        <div class="text-[11px] font-mono text-slate-500">{{ it.product_code }}</div>
                      </td>
                      <td class="py-2.5 px-3 text-center font-mono font-bold text-slate-700">
                        {{ formatQty(it.shipped_qty) }} {{ it.unit_code }}
                      </td>
                      <td class="py-2 px-3 text-center">
                        <input
                          type="number"
                          v-model.number="it.received_quantity"
                          min="0"
                          :max="it.shipped_qty"
                          class="w-full text-center bg-white border border-slate-300 focus:border-emerald-600 rounded-lg px-2 py-1.5 font-mono text-xs font-bold text-slate-900 focus:outline-none"
                        />
                      </td>
                      <td class="py-2.5 px-3 text-center font-mono font-bold">
                        <span
                          v-if="it.shipped_qty - it.received_quantity === 0"
                          class="text-emerald-700 text-[11px]"
                        >
                          Lengkap (0)
                        </span>
                        <span
                          v-else-if="it.shipped_qty - it.received_quantity > 0"
                          class="text-rose-600 text-[11px] bg-rose-50 px-2 py-0.5 rounded border border-rose-200"
                        >
                          Kurang -{{ formatQty(it.shipped_qty - it.received_quantity) }}
                        </span>
                        <span v-else class="text-amber-700 text-[11px]">
                          +{{ formatQty(it.received_quantity - it.shipped_qty) }}
                        </span>
                      </td>
                      <td class="py-2 px-3">
                        <input
                          v-model="it.notes"
                          placeholder="Catatan kondisi/kerusakan..."
                          class="w-full bg-white border border-slate-200 focus:border-emerald-600 rounded-lg px-2.5 py-1 text-xs text-slate-700 focus:outline-none"
                        />
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>

              <!-- Info Box Akuntansi -->
              <div class="p-3 bg-indigo-50/60 rounded-xl border border-indigo-200 text-xs text-indigo-900 flex items-start gap-2.5">
                <Icon icon="fluent-color:book-coins-24" class="w-5 h-5 shrink-0 mt-0.5" />
                <div>
                  <span class="font-bold">Otomasi Akuntansi & Persediaan:</span>
                  <p class="text-[11px] text-indigo-700 mt-0.5 leading-relaxed">
                    Setelah dikonfirmasi, stok fisik di <strong>{{ receiveForm.to_outlet_name }}</strong> akan bertambah sesuai kuantitas yang diterima. Sistem otomatis membukukan Debit Persediaan Outlet Tujuan vs Kredit Persediaan Dalam Perjalanan (1202).
                  </p>
                </div>
              </div>
            </div>
          </div>

          <!-- Modal Footer -->
          <div class="px-6 py-4 bg-slate-100 border-t border-slate-200 flex items-center justify-between">
            <span class="text-xs text-slate-500 font-mono">
              Pastikan fisik barang telah dihitung dengan benar.
            </span>
            <div class="flex items-center gap-3">
              <button
                type="button"
                @click="showReceiveTransferModal = false"
                class="px-4 py-2 rounded-lg text-xs font-bold text-slate-700 hover:bg-slate-200 transition cursor-pointer"
              >
                Batal
              </button>
              <button
                type="button"
                @click="handleConfirmReceiveTransfer"
                :disabled="isSubmittingReceive"
                class="px-5 py-2.5 rounded-lg text-xs font-bold text-white bg-emerald-600 hover:bg-emerald-700 shadow-md border border-emerald-700 transition active:scale-95 disabled:opacity-50 cursor-pointer"
              >
                {{ isSubmittingReceive ? 'Menyimpan Penerimaan...' : 'Konfirmasi Penerimaan Barang' }}
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- MODAL: PEMBATALAN TRANSFER STOK -->
      <div
        v-if="showCancelTransferModal"
        class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/60 backdrop-blur-xs overflow-y-auto"
      >
        <div class="bg-white rounded-2xl border border-slate-200 shadow-2xl w-full max-w-md overflow-hidden flex flex-col">
          <div class="p-6 text-center space-y-4">
            <div class="w-12 h-12 rounded-full bg-rose-100 text-rose-600 flex items-center justify-center mx-auto">
              <Ban class="w-6 h-6" />
            </div>

            <div>
              <h3 class="font-extrabold text-base text-slate-900">Batalkan Transfer Stok?</h3>
              <p class="text-xs text-slate-500 mt-1">
                Dokumen: <strong class="font-mono text-slate-800">{{ selectedTransferToCancel?.transfer_number }}</strong>
              </p>
            </div>

            <div class="p-3 bg-amber-50 rounded-xl border border-amber-200 text-left text-xs text-amber-900 leading-relaxed">
              <span class="font-bold">Perhatian:</span> Kuantitas stok yang sempat dipotong di outlet asal akan otomatis <strong>dikembalikan utuh</strong> dan jurnal in-transit (1202) akan dibalikkan.
            </div>

            <div class="text-left space-y-1.5">
              <label class="block text-xs font-bold text-slate-700">Alasan Pembatalan *</label>
              <textarea
                v-model="cancelReason"
                rows="3"
                placeholder="Misal: Salah pilih outlet tujuan, atau pengiriman dibatalkan oleh pihak pengirim..."
                class="w-full bg-slate-50 border border-slate-300 rounded-lg p-2.5 text-xs text-slate-800 focus:bg-white focus:border-rose-600 focus:outline-none resize-none"
              ></textarea>
            </div>

            <div class="flex items-center gap-2 pt-2">
              <button
                type="button"
                @click="showCancelTransferModal = false"
                class="w-1/2 py-2.5 rounded-lg text-xs font-bold text-slate-700 bg-slate-100 hover:bg-slate-200 transition cursor-pointer"
              >
                Kembali
              </button>
              <button
                type="button"
                @click="handleConfirmCancelTransfer"
                :disabled="isSubmittingCancel"
                class="w-1/2 py-2.5 rounded-lg text-xs font-bold text-white bg-rose-600 hover:bg-rose-700 shadow-md border border-rose-700 transition active:scale-95 disabled:opacity-50 cursor-pointer"
              >
                {{ isSubmittingCancel ? 'Membatalkan...' : 'Ya, Batalkan' }}
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- MODAL: RINCIAN DETAIL TRANSFER STOK & SURAT JALAN -->
      <div
        v-if="showTransferDetailModal"
        class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/60 backdrop-blur-xs overflow-y-auto"
      >
        <div class="bg-white rounded-2xl border border-slate-200 shadow-2xl w-full max-w-3xl overflow-hidden flex flex-col max-h-[90vh]">
          <!-- Modal Header -->
          <div class="px-6 py-4 bg-slate-900 text-white flex items-center justify-between">
            <div class="flex items-center gap-3">
              <div class="p-2 rounded-xl bg-white/10">
                <FileText class="w-5 h-5 text-emerald-400" />
              </div>
              <div>
                <h3 class="font-extrabold text-base">Surat Jalan & Rincian Dokumen Mutasi Transfer</h3>
                <p class="text-slate-400 text-xs mt-0.5">Detail pelacakan perpindahan fisik barang, petugas, dan jurnal akuntansi terkait.</p>
              </div>
            </div>
            <button
              @click="showTransferDetailModal = false"
              class="p-1 rounded-lg text-white/80 hover:text-white hover:bg-white/10 transition cursor-pointer"
            >
              <X class="w-5 h-5" />
            </button>
          </div>

          <!-- Modal Body Detail -->
          <div class="p-6 overflow-y-auto space-y-6 flex-1">
            <div v-if="isLoadingDetail" class="py-12 text-center text-slate-400">
              <Icon icon="fluent-color:arrow-clockwise-dashes-24" class="w-6 h-6 animate-spin mx-auto mb-2" />
              <span>Memuat rincian dokumen transfer...</span>
            </div>
            <template v-else-if="selectedTransferDetail">
              <!-- Status & Timeline Stepper -->
              <div class="p-4 bg-slate-50 rounded-xl border border-slate-200 space-y-3">
                <div class="flex items-center justify-between">
                  <div class="text-xs font-bold font-mono uppercase text-slate-500">Status Alur Transfer</div>
                  <div>
                    <span
                      v-if="selectedTransferDetail.status === 'IN_TRANSIT'"
                      class="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-bold bg-amber-100 text-amber-900 border border-amber-300"
                    >
                      <Truck class="w-4 h-4 text-amber-700 animate-bounce" />
                      <span>Sedang Dalam Perjalanan (In-Transit)</span>
                    </span>
                    <span
                      v-else-if="selectedTransferDetail.status === 'COMPLETED'"
                      class="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-bold bg-emerald-100 text-emerald-900 border border-emerald-300"
                    >
                      <CheckCircle2 class="w-4 h-4 text-emerald-700" />
                      <span>Selesai Diterima di Outlet Tujuan</span>
                    </span>
                    <span
                      v-else-if="selectedTransferDetail.status === 'CANCELLED'"
                      class="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-bold bg-rose-100 text-rose-900 border border-rose-300"
                    >
                      <Ban class="w-4 h-4 text-rose-700" />
                      <span>Dokumen Transfer Dibatalkan</span>
                    </span>
                  </div>
                </div>

                <!-- Stepper Progress Cards -->
                <div class="grid grid-cols-1 md:grid-cols-3 gap-3 text-xs pt-1">
                  <!-- Tahap 1: Pengiriman -->
                  <div class="p-3 bg-white rounded-lg border border-slate-200">
                    <div class="font-bold text-slate-700 flex items-center gap-1.5 text-[11px] uppercase font-mono">
                      <Truck class="w-3.5 h-3.5 text-rose-600" />
                      <span>1. Dikirim Dari Asal</span>
                    </div>
                    <div class="font-bold text-slate-900 mt-1">{{ selectedTransferDetail.from_outlet_name }}</div>
                    <div class="text-[11px] text-slate-500 mt-0.5">Petugas: {{ selectedTransferDetail.creator_name || '-' }}</div>
                    <div class="text-[10.5px] font-mono text-slate-400 mt-0.5">{{ formatDate(selectedTransferDetail.transfer_date) }}</div>
                  </div>

                  <!-- Tahap 2: In-Transit -->
                  <div
                    class="p-3 rounded-lg border"
                    :class="selectedTransferDetail.status === 'CANCELLED' ? 'bg-rose-50/50 border-rose-200' : 'bg-white border-slate-200'"
                  >
                    <div class="font-bold text-slate-700 flex items-center gap-1.5 text-[11px] uppercase font-mono">
                      <Clock class="w-3.5 h-3.5 text-amber-600" />
                      <span>2. Ekspedisi / Perjalanan</span>
                    </div>
                    <div class="font-bold text-slate-900 mt-1">
                      {{ selectedTransferDetail.status === 'IN_TRANSIT' ? 'Menuju Tujuan' : selectedTransferDetail.status === 'CANCELLED' ? 'Batal Dikirim' : 'Tiba di Lokasi' }}
                    </div>
                    <div class="text-[11px] text-slate-500 mt-0.5">
                      {{ selectedTransferDetail.status === 'CANCELLED' ? 'Barang dikembalikan ke asal' : 'Proses perpindahan fisik' }}
                    </div>
                  </div>

                  <!-- Tahap 3: Penerimaan / Pembatalan -->
                  <div
                    class="p-3 rounded-lg border"
                    :class="selectedTransferDetail.status === 'COMPLETED' ? 'bg-emerald-50 border-emerald-300' : selectedTransferDetail.status === 'CANCELLED' ? 'bg-rose-50 border-rose-300' : 'bg-slate-100 border-slate-200 opacity-60'"
                  >
                    <div class="font-bold text-slate-700 flex items-center gap-1.5 text-[11px] uppercase font-mono">
                      <PackageCheck v-if="selectedTransferDetail.status !== 'CANCELLED'" class="w-3.5 h-3.5 text-emerald-600" />
                      <Ban v-else class="w-3.5 h-3.5 text-rose-600" />
                      <span>3. {{ selectedTransferDetail.status === 'CANCELLED' ? 'Pembatalan' : 'Diterima di Tujuan' }}</span>
                    </div>
                    <template v-if="selectedTransferDetail.status === 'COMPLETED'">
                      <div class="font-bold text-emerald-950 mt-1">{{ selectedTransferDetail.to_outlet_name }}</div>
                      <div class="text-[11px] text-emerald-800 mt-0.5">Penerima: {{ selectedTransferDetail.receiver_name || '-' }}</div>
                      <div v-if="selectedTransferDetail.received_date" class="text-[10.5px] font-mono text-emerald-700 mt-0.5">
                        {{ formatDate(selectedTransferDetail.received_date) }}
                      </div>
                    </template>
                    <template v-else-if="selectedTransferDetail.status === 'CANCELLED'">
                      <div class="font-bold text-rose-900 mt-1">Dibatalkan</div>
                      <div class="text-[11px] text-rose-700 mt-0.5">Oleh: {{ selectedTransferDetail.canceller_name || '-' }}</div>
                      <div v-if="selectedTransferDetail.cancellation_reason" class="text-[10.5px] text-rose-600 italic mt-0.5">
                        "{{ selectedTransferDetail.cancellation_reason }}"
                      </div>
                    </template>
                    <template v-else>
                      <div class="font-bold text-slate-600 mt-1">Menunggu Penerimaan</div>
                      <div class="text-[11px] text-slate-500 mt-0.5">Di {{ selectedTransferDetail.to_outlet_name }}</div>
                    </template>
                  </div>
                </div>
              </div>

              <!-- Header Details Card -->
              <div class="grid grid-cols-2 md:grid-cols-4 gap-4 bg-slate-50 p-4 rounded-xl border border-slate-200 text-xs">
                <div>
                  <span class="text-slate-400 font-mono uppercase text-[10px]">No. Dokumen</span>
                  <div class="font-mono font-bold text-sky-800 text-sm mt-0.5">{{ selectedTransferDetail.transfer_number }}</div>
                </div>
                <div>
                  <span class="text-slate-400 font-mono uppercase text-[10px]">Tanggal Kirim</span>
                  <div class="font-mono font-bold text-slate-800 text-sm mt-0.5">{{ formatDate(selectedTransferDetail.transfer_date) }}</div>
                </div>
                <div>
                  <span class="text-slate-400 font-mono uppercase text-[10px]">Total Nilai Mutasi HPP</span>
                  <div class="font-mono font-bold text-emerald-700 text-sm mt-0.5">{{ formatCurrency(selectedTransferDetail.total_amount) }}</div>
                </div>
                <div>
                  <span class="text-slate-400 font-mono uppercase text-[10px]">Outlet Pengirim</span>
                  <div class="font-sans font-bold text-slate-800 text-sm mt-0.5">{{ selectedTransferDetail.from_outlet_name }}</div>
                </div>
              </div>

              <!-- Notes if any -->
              <div v-if="selectedTransferDetail.notes" class="text-xs text-slate-600 bg-slate-50 p-3 rounded-lg border border-slate-200">
                <span class="font-bold text-slate-800">Catatan Pengiriman:</span> {{ selectedTransferDetail.notes }}
              </div>

              <!-- Item List -->
              <div class="space-y-2">
                <h4 class="text-xs font-bold font-mono uppercase text-slate-700 tracking-wider">Item Barang yang Ditransfer</h4>
                <div class="border border-slate-300 rounded-xl overflow-hidden shadow-xs">
                  <table class="w-full text-left text-xs border-collapse">
                    <thead class="bg-slate-100 border-b border-slate-300 text-slate-700 font-mono text-[11px] uppercase">
                      <tr>
                        <th class="py-2.5 px-3">SKU & Nama Produk</th>
                        <th class="py-2.5 px-3 text-right">Qty Dikirim</th>
                        <th class="py-2.5 px-3 text-right">Qty Diterima</th>
                        <th class="py-2.5 px-3 text-right">HPP Satuan</th>
                        <th class="py-2.5 px-3 text-right">Subtotal</th>
                      </tr>
                    </thead>
                    <tbody class="divide-y divide-slate-200 font-mono">
                      <tr v-for="it in selectedTransferDetail.items" :key="it.id" class="hover:bg-slate-50 transition">
                        <td class="py-2.5 px-3 text-slate-600">
                          <div class="font-bold text-slate-900 font-sans">{{ it.product_name }}</div>
                          <div class="text-[10px] text-slate-500">{{ it.product_code }} <span v-if="it.barcode">&bull; {{ it.barcode }}</span></div>
                          <div v-if="it.notes" class="text-[10.5px] italic text-amber-700 mt-0.5">Catatan: {{ it.notes }}</div>
                        </td>
                        <td class="py-2.5 px-3 text-right font-bold text-slate-800">
                          {{ formatQty(it.quantity) }} {{ it.unit_code }}
                        </td>
                        <td class="py-2.5 px-3 text-right font-bold">
                          <span v-if="selectedTransferDetail.status === 'COMPLETED'" class="text-emerald-700">
                            {{ formatQty(it.received_quantity || it.quantity) }} {{ it.unit_code }}
                          </span>
                          <span v-else-if="selectedTransferDetail.status === 'IN_TRANSIT'" class="text-amber-700 italic text-[11px]">
                            Menunggu Tiba
                          </span>
                          <span v-else class="text-slate-400">-</span>
                        </td>
                        <td class="py-2.5 px-3 text-right text-slate-600">{{ formatCurrency(it.cost_price) }}</td>
                        <td class="py-2.5 px-3 text-right font-bold text-emerald-700">{{ formatCurrency(it.subtotal) }}</td>
                      </tr>
                    </tbody>
                  </table>
                </div>
              </div>

              <!-- Jurnal Akuntansi Double-Entry (Mendukung Multi-Jurnal: Kirim, Terima, Batal) -->
              <div v-if="selectedTransferDetail.journals && selectedTransferDetail.journals.length > 0" class="space-y-4 pt-2">
                <div class="flex items-center justify-between">
                  <h4 class="text-xs font-bold font-mono uppercase text-slate-700 tracking-wider flex items-center gap-2">
                    <Icon icon="streamline-ultimate-color:accounting-calculator-1" class="w-4 h-4" />
                    <span>Jurnal Akuntansi Pembukuan Otomatis ({{ selectedTransferDetail.journals.length }} Transaksi)</span>
                  </h4>
                </div>

                <div v-for="jrn in selectedTransferDetail.journals" :key="jrn.id" class="border border-indigo-200 bg-indigo-50/20 rounded-xl overflow-hidden shadow-xs space-y-1">
                  <div class="bg-indigo-100/70 px-4 py-2 flex items-center justify-between text-xs text-indigo-950 font-bold border-b border-indigo-200">
                    <span class="font-mono">{{ jrn.journal_number }} &bull; {{ formatDate(jrn.journal_date) }}</span>
                    <span class="font-sans text-[11px] font-normal text-indigo-800">{{ jrn.description }}</span>
                  </div>
                  <table class="w-full text-left text-xs border-collapse">
                    <thead class="bg-indigo-50/60 border-b border-indigo-200 text-indigo-900 font-mono text-[10.5px] uppercase">
                      <tr>
                        <th class="py-2 px-3">Kode & Nama Akun</th>
                        <th class="py-2 px-3">Memo / Keterangan</th>
                        <th class="py-2 px-3 text-right">Debit</th>
                        <th class="py-2 px-3 text-right">Kredit</th>
                      </tr>
                    </thead>
                    <tbody class="divide-y divide-indigo-100 font-mono">
                      <tr v-for="line in jrn.lines" :key="line.id" class="bg-white/80">
                        <td class="py-2 px-3">
                          <span class="font-bold text-sky-800">{{ line.account_code }}</span>
                          <span class="text-slate-800 font-sans ml-1.5 font-semibold">{{ line.account_name }}</span>
                        </td>
                        <td class="py-2 px-3 font-sans italic text-slate-500 text-[11px]">{{ line.memo }}</td>
                        <td class="py-2 px-3 text-right font-bold text-slate-800">
                          {{ line.debit > 0 ? formatCurrency(line.debit) : '-' }}
                        </td>
                        <td class="py-2 px-3 text-right font-bold text-slate-800">
                          {{ line.credit > 0 ? formatCurrency(line.credit) : '-' }}
                        </td>
                      </tr>
                    </tbody>
                    <tfoot class="bg-indigo-50/80 border-t border-indigo-200 font-mono font-bold text-xs text-indigo-950">
                      <tr>
                        <td colspan="2" class="py-2 px-3 text-right">Keseimbangan:</td>
                        <td class="py-2 px-3 text-right text-emerald-800 font-extrabold">{{ formatCurrency(jrn.total_debit) }}</td>
                        <td class="py-2 px-3 text-right text-emerald-800 font-extrabold">{{ formatCurrency(jrn.total_credit) }}</td>
                      </tr>
                    </tfoot>
                  </table>
                </div>
              </div>
            </template>
          </div>

          <!-- Modal Footer -->
          <div class="px-6 py-4 bg-slate-100 border-t border-slate-200 flex items-center justify-between">
            <button
              type="button"
              @click="printTransferSlip"
              class="inline-flex items-center gap-1.5 px-4 py-2 rounded-lg text-xs font-bold text-slate-700 bg-white border border-slate-300 hover:bg-slate-50 shadow-xs transition cursor-pointer"
            >
              <Printer class="w-4 h-4 text-slate-600" />
              <span>Cetak Surat Jalan / Bukti Transfer</span>
            </button>

            <button
              @click="showTransferDetailModal = false"
              class="px-5 py-2 rounded-lg text-xs font-bold text-slate-700 bg-white border border-slate-300 hover:bg-slate-50 shadow-xs transition cursor-pointer"
            >
              Tutup
            </button>
          </div>
        </div>
      </div>
    </div>
  </AdminLayout>
</template>
