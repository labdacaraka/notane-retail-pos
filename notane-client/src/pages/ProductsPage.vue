<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, computed } from 'vue'
import { useRouter } from 'vue-router'
import AdminLayout from '@/layouts/AdminLayout.vue'
import PaginationBar from '@/components/common/PaginationBar.vue'
import SearchableSelect from '@/components/common/SearchableSelect.vue'
import AdvancedProductImportModal from '@/components/products/AdvancedProductImportModal.vue'
import ProductBarcodesModal from '@/components/products/ProductBarcodesModal.vue'
import {
  Search,
  Scale,
  Edit2,
  Trash2,
  Users,
  Tag,
  Check,
  Building2,
  Lock,
  Upload,
  Eye,
  Barcode,
  ArrowUpDown,
  ArrowUp,
  ArrowDown,
  Filter,
  Layers,
  Calculator,
  Globe,
  Receipt,
  FileSpreadsheet,
} from 'lucide-vue-next'
import { Icon } from '@iconify/vue'
import { api, getApiBaseUrl } from '@/services/api'
import type { InventoryCostingSettings } from '@/types'
import { useAuthStore } from '@/stores/authStore'
import { useLocationStore } from '@/stores/locationStore'
import { useExportStore } from '@/stores/exportStore'
import { useDialog } from '@/composables/useDialog'
import { formatCurrency, formatQty } from '@/utils/formatters'

const authStore = useAuthStore()
const locationStore = useLocationStore()
const exportStore = useExportStore()
const dialog = useDialog()
const router = useRouter()

const products = ref<any[]>([])
const categories = ref<any[]>([])
const units = ref<any[]>([])
const brands = ref<any[]>([])
const outlets = ref<any[]>([])
const customerGroups = ref<any[]>([])
const warehouses = ref<any[]>([])
const isLoading = ref<boolean>(false)
const searchQuery = ref<string>('')
const selectedCategory = ref<string>('')
const selectedStockStatus = ref<string>('ALL')

// Server-side sorting & pagination
const sortBy = ref<string>('name')
const sortDir = ref<'asc' | 'desc'>('asc')
const currentPage = ref<number>(1)
const pageSize = ref<number>(10)
const totalItems = ref<number>(0)

let searchDebounceTimer: any = null
function handleSearchInput() {
  if (searchDebounceTimer) clearTimeout(searchDebounceTimer)
  searchDebounceTimer = setTimeout(() => {
    currentPage.value = 1
    fetchProducts()
  }, 400)
}

function handleSort(column: string) {
  if (sortBy.value === column) {
    sortDir.value = sortDir.value === 'asc' ? 'desc' : 'asc'
  } else {
    sortBy.value = column
    sortDir.value = 'asc'
  }
  currentPage.value = 1
  fetchProducts()
}

function onPageChange(page: number) {
  currentPage.value = page
  fetchProducts()
}

function onPageSizeChange(size: number) {
  pageSize.value = size
  currentPage.value = 1
  fetchProducts()
}

// Modals
const showImportModal = ref<boolean>(false)
const previewImageModalUrl = ref<string | null>(null)
const showBarcodesModal = ref<boolean>(false)
const selectedProductForBarcodes = ref<any>(null)

const showBatchesModal = ref<boolean>(false)
const selectedProductForBatches = ref<any>(null)
const productBatches = ref<any[]>([])
const isLoadingBatches = ref<boolean>(false)
const filterBatchAllOutlets = ref<boolean>(false)

async function fetchProductBatches() {
  if (!selectedProductForBatches.value) return
  isLoadingBatches.value = true
  try {
    const res = await api.get(`/products/${selectedProductForBatches.value.id}/batches`, {
      params: {
        outlet_id: filterBatchAllOutlets.value ? undefined : (locationStore.currentOutletId || undefined),
      },
    })
    if (res.data.success) {
      productBatches.value = res.data.data
    }
  } catch (err) {
    console.error('Gagal memuat batch produk:', err)
  } finally {
    isLoadingBatches.value = false
  }
}

async function openBatchesModal(prod: any) {
  selectedProductForBatches.value = prod
  filterBatchAllOutlets.value = false
  showBatchesModal.value = true
  await fetchProductBatches()
}

// Modal Riwayat Penjualan Batch (FEFO/FIFO Traceability)
const showBatchSalesModal = ref<boolean>(false)
const selectedBatchForSales = ref<any>(null)
const batchSalesUsage = ref<any>(null)
const isLoadingBatchSales = ref<boolean>(false)

async function openBatchSalesModal(batch: any) {
  selectedBatchForSales.value = batch
  showBatchSalesModal.value = true
  isLoadingBatchSales.value = true
  batchSalesUsage.value = null
  try {
    const res = await api.get(`/batches/${batch.id}/sales`)
    if (res.data.success) {
      batchSalesUsage.value = res.data.data
    }
  } catch (err) {
    console.error('Gagal memuat riwayat transaksi penjualan batch:', err)
  } finally {
    isLoadingBatchSales.value = false
  }
}

function openBarcodesModal(prod: any) {
  selectedProductForBarcodes.value = prod
  showBarcodesModal.value = true
}

// Helper: ubah path relatif /uploads/... menjadi URL absolut backend
function resolveImageUrl(url?: string | null): string | null {
  if (!url) return null
  if (url.startsWith('http://') || url.startsWith('https://') || url.startsWith('data:')) return url
  const base = getApiBaseUrl().replace('/api/v1', '')
  return `${base}${url}`
}

function openImagePreview(url?: string | null) {
  const resolved = resolveImageUrl(url)
  if (resolved) previewImageModalUrl.value = resolved
}

// Global Costing Settings
const globalCostingSettings = ref<InventoryCostingSettings | null>(null)

async function fetchGlobalCostingSettings() {
  try {
    const res = await api.get('/settings/inventory-costing')
    if (res.data.success && res.data.data) {
      globalCostingSettings.value = res.data.data
    }
  } catch (err) {
    console.error('Failed to fetch global costing settings:', err)
  }
}

const globalCostingLabel = computed(() => {
  const m = globalCostingSettings.value?.default_costing_method || 'WMA'
  if (m === 'FIFO' || m === 'FEFO') return 'FIFO & FEFO'
  if (m === 'REPLACEMENT_COST') return 'Harga Ganti'
  return 'Rata-rata (WMA)'
})

function getProductCostingDisplay(method?: string) {
  const m = (method || 'GLOBAL').toUpperCase()
  if (m === 'GLOBAL') {
    const defaultM = globalCostingSettings.value?.default_costing_method || 'WMA'
    const name = (defaultM === 'FIFO' || defaultM === 'FEFO') ? 'FIFO & FEFO' : defaultM === 'REPLACEMENT_COST' ? 'Harga Ganti' : 'Rata-rata'
    return {
      isGlobal: true,
      badgeText: `Global (${name})`,
      effectiveMethod: defaultM,
      tooltip: `Mengikuti pengaturan global toko (${name}). Perubahan di master parameter akan otomatis berlaku untuk produk ini.`
    }
  }
  if (m === 'FIFO' || m === 'FEFO') {
    return {
      isGlobal: false,
      badgeText: 'FIFO & FEFO',
      effectiveMethod: 'FIFO',
      tooltip: 'Dikonfigurasi khusus: HPP dan pengeluaran fisik memprioritaskan batch kedaluwarsa terdekat (FEFO) atau batch tertua (FIFO).'
    }
  }
  if (m === 'REPLACEMENT_COST') {
    return {
      isGlobal: false,
      badgeText: 'Harga Ganti',
      effectiveMethod: 'REPLACEMENT_COST',
      tooltip: 'Dikonfigurasi khusus: HPP selalu langsung disamakan dengan harga beli faktur kulakan terakhir.'
    }
  }
  return {
    isGlobal: false,
    badgeText: 'Rata-rata',
    effectiveMethod: 'WMA',
    tooltip: 'Dikonfigurasi khusus: HPP proporsional antara sisa stok lama dan barang masuk baru.'
  }
}

// Create Modal
const showCreateModal = ref<boolean>(false)
const form = ref({
  sku: '',
  barcode: '',
  name: '',
  category_id: '',
  base_unit_id: '',
  brand_id: null as string | null,
  cost_price: 0,
  base_selling_price: 0,
  is_weight_embedded: false,
  costing_method: 'GLOBAL',
  auto_adjust_selling_price: false,
  markup_percent: null as number | null,
  initial_stock: 0,
  image_url: '',
})

// Edit Modal
const showEditModal = ref<boolean>(false)
const editingId = ref<string | null>(null)
const editForm = ref({
  sku: '',
  barcode: '',
  name: '',
  category_id: '',
  base_unit_id: '',
  brand_id: null as string | null,
  cost_price: 0,
  base_selling_price: 0,
  is_weight_embedded: false,
  costing_method: 'GLOBAL',
  auto_adjust_selling_price: false,
  markup_percent: null as number | null,
  min_stock: 0,
  max_stock: 0,
  tax_percent: 11,
  image_url: '',
})

async function fetchProducts() {
  isLoading.value = true
  try {
    const res = await api.get('/products', {
      params: {
        search: searchQuery.value?.trim() || undefined,
        category_id: selectedCategory.value || undefined,
        stock_status: selectedStockStatus.value !== 'ALL' ? selectedStockStatus.value : undefined,
        sort_by: sortBy.value,
        sort_dir: sortDir.value,
        page: currentPage.value,
        page_size: pageSize.value,
      },
    })
    if (res.data.success) {
      products.value = res.data.data
      totalItems.value = res.data.total_items ?? res.data.data.length
    }
  } catch (err) {
    console.error(err)
  } finally {
    isLoading.value = false
  }
}

async function fetchMasterMeta() {
  try {
    const [catRes, unitRes, brandRes, outletRes, groupRes, whRes] = await Promise.all([
      api.get('/categories'),
      api.get('/units'),
      api.get('/brands'),
      api.get('/outlets'),
      api.get('/customer-groups'),
      api.get('/warehouses'),
    ])
    if (catRes.data.success) categories.value = catRes.data.data
    if (unitRes.data.success) units.value = unitRes.data.data
    if (brandRes.data.success) brands.value = brandRes.data.data
    if (outletRes.data.success) outlets.value = outletRes.data.data
    if (groupRes.data.success) customerGroups.value = groupRes.data.data
    if (whRes.data.success) warehouses.value = whRes.data.data

    if (categories.value.length > 0) form.value.category_id = categories.value[0].id
    if (units.value.length > 0) form.value.base_unit_id = units.value[0].id
  } catch (err) {
    console.error(err)
  }
}

function exportProductsToExcel() {
  const currentParams: Record<string, any> = {
    search: searchQuery.value?.trim() || undefined,
    category_id: selectedCategory.value || undefined,
    stock_status: selectedStockStatus.value !== 'ALL' ? selectedStockStatus.value : undefined,
    sort_by: sortBy.value,
    sort_dir: sortDir.value,
  }

  exportStore.queueServerExport({
    title: 'Master Produk (Format Template Import)',
    module: 'products',
    filterParams: currentParams,
  })
}

// Ref untuk loading state upload foto
const isUploadingPhoto = ref<boolean>(false)

async function handlePhotoFileSelected(event: Event, mode: 'CREATE' | 'EDIT') {
  const input = event.target as HTMLInputElement
  if (!input.files || input.files.length === 0) return
  const file = input.files[0]

  if (!file.type.startsWith('image/')) {
    dialog.alert('File yang dipilih harus berupa gambar (JPG, PNG, WEBP)', 'warning')
    return
  }

  // Tampilkan preview lokal dulu (base64) sambil upload berjalan
  const reader = new FileReader()
  reader.onload = () => {
    const previewUrl = reader.result as string
    if (mode === 'CREATE') form.value.image_url = previewUrl
    else editForm.value.image_url = previewUrl
  }
  reader.readAsDataURL(file)

  // Upload ke server dan simpan URL path (bukan base64)
  isUploadingPhoto.value = true
  try {
    const reader2 = new FileReader()
    const base64Promise = new Promise<string>((resolve) => {
      reader2.onload = () => resolve(reader2.result as string)
      reader2.readAsDataURL(file)
    })
    const dataUrl = await base64Promise

    const res = await api.post('/products/upload-photo', {
      filename: file.name,
      data_base64: dataUrl,
    })

    if (res.data.success) {
      // Ganti preview base64 dengan URL server yang sebenarnya (pendek, hanya path)
      const serverUrl = res.data.image_url as string
      if (mode === 'CREATE') form.value.image_url = serverUrl
      else editForm.value.image_url = serverUrl
    }
  } catch (err: any) {
    console.error('Upload foto gagal, foto disimpan sebagai URL eksternal:', err)
    // Fallback: biarkan base64 preview yang sudah di-set
    // (tidak ideal tapi lebih baik daripada gagal total)
    dialog.alert('Gagal mengupload foto ke server. Pastikan server aktif. Error: ' + (err.response?.data?.message || err.message), 'danger')
    if (mode === 'CREATE') form.value.image_url = ''
    else editForm.value.image_url = ''
  } finally {
    isUploadingPhoto.value = false
    // Reset input agar bisa pilih file yang sama lagi
    input.value = ''
  }
}

const isSavingProduct = ref<boolean>(false)

async function handleCreateProduct() {
  if (!authStore.hasPermission(['products:create', 'master:write'])) {
    dialog.alert('Akses ditolak: Anda tidak memiliki hak menambah produk baru (products:create)', 'warning')
    return
  }
  if (!form.value.name.trim()) {
    dialog.alert('Nama produk wajib diisi!', 'warning')
    return
  }
  if (!form.value.sku.trim()) {
    dialog.alert('SKU produk wajib diisi!', 'warning')
    return
  }
  if (!form.value.category_id) {
    dialog.alert('Kategori produk wajib dipilih!', 'warning')
    return
  }
  if (!form.value.base_unit_id) {
    dialog.alert('Satuan dasar produk wajib dipilih!', 'warning')
    return
  }

  isSavingProduct.value = true
  try {
    const payload = {
      ...form.value,
      brand_id: form.value.brand_id || null,
      barcode: form.value.barcode?.trim() || form.value.sku.trim(),
      costing_method: form.value.costing_method,
      auto_adjust_selling_price: Boolean(form.value.auto_adjust_selling_price),
      markup_percent: form.value.markup_percent !== null && form.value.markup_percent !== undefined && String(form.value.markup_percent) !== '' ? Number(form.value.markup_percent) : null,
      cost_price: Number(form.value.cost_price || 0),
      base_selling_price: Number(form.value.base_selling_price || 0),
      image_url: form.value.image_url || undefined,
    }
    await api.post('/products', payload)
    showCreateModal.value = false
    dialog.toast('Produk baru berhasil disimpan!', 'success')
    fetchProducts()
  } catch (err: any) {
    dialog.alert(err.response?.data?.message || err.message || 'Gagal menyimpan produk', 'danger')
  } finally {
    isSavingProduct.value = false
  }
}

function openEditModal(prod: any) {
  if (!authStore.hasPermission(['products:update', 'master:write'])) {
    dialog.alert('Akses ditolak: Anda tidak memiliki hak mengubah data produk (products:update)', 'warning')
    return
  }
  editingId.value = prod.id
  editForm.value = {
    sku: prod.sku,
    barcode: prod.barcode,
    name: prod.name,
    category_id: prod.category_id,
    base_unit_id: prod.base_unit_id,
    brand_id: prod.brand_id || null,
    costing_method: prod.costing_method || 'GLOBAL',
    auto_adjust_selling_price: Boolean(prod.auto_adjust_selling_price),
    markup_percent: prod.markup_percent !== undefined && prod.markup_percent !== null ? Number(prod.markup_percent) : null,
    cost_price: Number(prod.cost_price),
    base_selling_price: Number(prod.base_selling_price),
    is_weight_embedded: Boolean(prod.is_weight_embedded),
    min_stock: Number(prod.min_stock || 0),
    max_stock: Number(prod.max_stock || 0),
    tax_percent: Number(prod.tax_percent || 11),
    image_url: prod.image_url || '',
  }
  showEditModal.value = true
}

async function handleUpdateProduct() {
  if (!authStore.hasPermission(['products:update', 'master:write'])) {
    dialog.alert('Akses ditolak: Anda tidak memiliki hak mengubah data produk (products:update)', 'warning')
    return
  }
  if (!editingId.value) return
  if (!editForm.value.name.trim()) {
    dialog.alert('Nama produk wajib diisi!', 'warning')
    return
  }

  isSavingProduct.value = true
  try {
    const payload = {
      ...editForm.value,
      brand_id: editForm.value.brand_id || null,
      costing_method: editForm.value.costing_method,
      auto_adjust_selling_price: Boolean(editForm.value.auto_adjust_selling_price),
      markup_percent: editForm.value.markup_percent !== null && editForm.value.markup_percent !== undefined && String(editForm.value.markup_percent) !== '' ? Number(editForm.value.markup_percent) : null,
      cost_price: Number(editForm.value.cost_price || 0),
      base_selling_price: Number(editForm.value.base_selling_price || 0),
      image_url: editForm.value.image_url || undefined,
    }
    await api.put(`/products/${editingId.value}`, payload)
    showEditModal.value = false
    dialog.toast('Perubahan produk berhasil disimpan!', 'success')
    fetchProducts()
  } catch (err: any) {
    dialog.alert(err.response?.data?.message || err.message || 'Gagal memperbarui produk', 'danger')
  } finally {
    isSavingProduct.value = false
  }
}

async function handleDeleteProduct(id: string, name: string) {
  if (!authStore.hasPermission(['products:delete', 'master:write'])) {
    dialog.alert('Akses ditolak: Anda tidak memiliki hak menghapus produk (products:delete)', 'warning')
    return
  }
  const confirmed = await dialog.confirm({
    title: 'Nonaktifkan Produk',
    message: `Apakah Anda yakin ingin menonaktifkan produk "${name}"?`,
    subMessage: 'Produk yang dinonaktifkan tidak akan muncul lagi pada katalog kasir POS.',
    confirmText: 'Ya, Nonaktifkan',
    cancelText: 'Batal',
    type: 'danger',
  })
  if (!confirmed) return
  try {
    await api.delete(`/products/${id}`)
    dialog.toast(`Produk "${name}" berhasil dinonaktifkan`, 'success')
    fetchProducts()
  } catch (err: any) {
    dialog.alert(err.response?.data?.message || 'Gagal menonaktifkan produk', 'danger')
  }
}

// Customer Group Prices Modal
const showGroupPriceModal = ref<boolean>(false)
const selectedProductForGroupPrice = ref<any>(null)
const productGroupPrices = ref<any[]>([])
const groupPriceInputs = ref<Record<number, number | null>>({})
const isSavingGroupPrices = ref<boolean>(false)
const isLoadingGroupPrices = ref<boolean>(false)

// Outlet Customer Group Prices Modal (Harga Member per Outlet)
const showOutletCgpModal = ref<boolean>(false)
const selectedProductForOutletCgp = ref<any>(null)
const outletCgpOutlets = ref<any[]>([])
const outletCgpGroups = ref<any[]>([])
const outletCgpPrices = ref<any[]>([]) // existing prices from DB
const outletCgpInputs = ref<Record<string, Record<number, number | null>>>({}) // [outlet_id][group_id] = price
const isLoadingOutletCgp = ref<boolean>(false)
const isSavingOutletCgp = ref<boolean>(false)
const outletCgpSelectedOutlet = ref<string | null>(null)

async function openGroupPriceModal(prod: any) {
  selectedProductForGroupPrice.value = prod
  groupPriceInputs.value = {}
  showGroupPriceModal.value = true
  isLoadingGroupPrices.value = true

  try {
    const [groupsRes, pricesRes] = await Promise.all([
      api.get('/customer-groups'),
      api.get(`/products/${prod.id}/customer-group-prices`),
    ])

    if (groupsRes.data.success) {
      customerGroups.value = groupsRes.data.data
    }
    if (pricesRes.data.success) {
      productGroupPrices.value = pricesRes.data.data
      for (const gp of pricesRes.data.data) {
        groupPriceInputs.value[gp.customer_group_id] = Number(gp.special_price)
      }
    }
  } catch (err) {
    console.error('Gagal memuat harga grup pelanggan:', err)
  } finally {
    isLoadingGroupPrices.value = false
  }
}

async function handleSaveGroupPrice(groupId: number) {
  if (!authStore.hasPermission(['products:customer_group_prices', 'master:write'])) {
    dialog.alert('Akses ditolak: Anda tidak memiliki wewenang mengatur harga grup pelanggan (products:customer_group_prices)', 'warning')
    return
  }
  const prodId = selectedProductForGroupPrice.value?.id
  if (!prodId) return

  const inputVal = groupPriceInputs.value[groupId]
  isSavingGroupPrices.value = true

  try {
    if (inputVal === null || inputVal === undefined || inputVal <= 0) {
      // If cleared or zero, delete custom group price (fallback to base price)
      const existing = productGroupPrices.value.find((gp) => gp.customer_group_id === groupId)
      if (existing) {
        await api.delete(`/products/customer-group-prices/${existing.id}`)
      }
      groupPriceInputs.value[groupId] = null
    } else {
      await api.post(`/products/${prodId}/customer-group-prices`, {
        customer_group_id: groupId,
        special_price: Number(inputVal),
      })
    }

    // Refresh list
    const pricesRes = await api.get(`/products/${prodId}/customer-group-prices`)
    if (pricesRes.data.success) {
      productGroupPrices.value = pricesRes.data.data
    }
    dialog.toast('Harga kelompok pelanggan berhasil diperbarui!', 'success')
  } catch (err: any) {
    dialog.alert(err.response?.data?.message || 'Gagal menyimpan harga kelompok pelanggan', 'danger')
  } finally {
    isSavingGroupPrices.value = false
  }
}

// Outlet-Specific Prices Modal
const showOutletPriceModal = ref<boolean>(false)
const selectedProductForOutletPrice = ref<any>(null)
const outletsList = ref<any[]>([])
const productOutletPrices = ref<any[]>([])
const outletPriceInputs = ref<Record<string, { cost_price: number | null; base_selling_price: number | null }>>({})
const isSavingOutletPrices = ref<boolean>(false)
const isLoadingOutletPrices = ref<boolean>(false)

async function openOutletPriceModal(prod: any) {
  selectedProductForOutletPrice.value = prod
  outletPriceInputs.value = {}
  showOutletPriceModal.value = true
  isLoadingOutletPrices.value = true

  try {
    const [_, outletsRes, pricesRes] = await Promise.all([
      authStore.fetchCurrentUser(),
      api.get('/me/outlets'),
      api.get(`/products/${prod.id}/outlet-prices`),
    ])

    if (outletsRes.data.success) {
      outletsList.value = outletsRes.data.data.filter((o: any) => o.is_active !== false)
    }
    if (pricesRes.data.success) {
      productOutletPrices.value = pricesRes.data.data
      for (const op of pricesRes.data.data) {
        outletPriceInputs.value[op.outlet_id] = {
          cost_price: Number(op.cost_price),
          base_selling_price: Number(op.base_selling_price),
        }
      }
    }
  } catch (err) {
    console.error('Gagal memuat harga outlet:', err)
  } finally {
    isLoadingOutletPrices.value = false
  }
}

function getOutletPriceDetail(outletId: string) {
  return productOutletPrices.value.find((p) => p.outlet_id === outletId)
}

async function handleSaveOutletPrice(outletId: string) {
  if (!authStore.hasPermission(['products:outlet_prices', 'master:write'])) {
    dialog.alert('Akses ditolak: Anda tidak memiliki wewenang mengatur harga khusus outlet (products:outlet_prices)', 'warning')
    return
  }
  if (!authStore.hasOutletAccess(outletId)) {
    dialog.alert('Akses ditolak: Anda tidak memiliki wewenang untuk mengatur harga di outlet ini!', 'warning')
    return
  }
  const prodId = selectedProductForOutletPrice.value?.id
  if (!prodId) return

  const input = outletPriceInputs.value[outletId]
  isSavingOutletPrices.value = true

  try {
    const cost = input?.cost_price
    const price = input?.base_selling_price

    if ((cost === null || cost === undefined || cost < 0) && (price === null || price === undefined || price < 0)) {
      // If both cleared, delete override (fallback to global product prices)
      const existing = productOutletPrices.value.find((op) => op.outlet_id === outletId)
      if (existing) {
        await api.delete(`/products/outlet-prices/${existing.id}`)
      }
      delete outletPriceInputs.value[outletId]
    } else {
      const finalCost = cost !== null && cost !== undefined ? Number(cost) : Number(selectedProductForOutletPrice.value?.cost_price || 0)
      const finalPrice = price !== null && price !== undefined ? Number(price) : Number(selectedProductForOutletPrice.value?.base_selling_price || 0)

      await api.post(`/products/${prodId}/outlet-prices`, {
        outlet_id: outletId,
        cost_price: finalCost,
        base_selling_price: finalPrice,
      })
    }

    // Refresh list
    const pricesRes = await api.get(`/products/${prodId}/outlet-prices`)
    if (pricesRes.data.success) {
      productOutletPrices.value = pricesRes.data.data
      for (const op of pricesRes.data.data) {
        outletPriceInputs.value[op.outlet_id] = {
          cost_price: Number(op.cost_price),
          base_selling_price: Number(op.base_selling_price),
        }
      }
    }
    dialog.toast('Harga outlet berhasil diperbarui!', 'success')
  } catch (err: any) {
    dialog.alert(err.response?.data?.message || 'Gagal menyimpan harga outlet', 'danger')
  } finally {
    isSavingOutletPrices.value = false
  }
}

async function openOutletCgpModal(prod: any) {
  selectedProductForOutletCgp.value = prod
  outletCgpInputs.value = {}
  showOutletCgpModal.value = true
  isLoadingOutletCgp.value = true
  outletCgpSelectedOutlet.value = null

  try {
    const [outletsRes, groupsRes, pricesRes] = await Promise.all([
      api.get('/me/outlets'),
      api.get('/customer-groups'),
      api.get(`/products/${prod.id}/outlet-customer-group-prices`),
    ])
    if (outletsRes.data.success) {
      outletCgpOutlets.value = outletsRes.data.data.filter((o: any) => o.is_active !== false)
      if (outletCgpOutlets.value.length > 0) {
        outletCgpSelectedOutlet.value = outletCgpOutlets.value[0].id
      }
    }
    if (groupsRes.data.success) {
      outletCgpGroups.value = groupsRes.data.data.filter((g: any) => g.is_active !== false)
    }
    if (pricesRes.data.success) {
      outletCgpPrices.value = pricesRes.data.data
      // Populate inputs
      for (const p of pricesRes.data.data) {
        if (!outletCgpInputs.value[p.outlet_id]) outletCgpInputs.value[p.outlet_id] = {}
        outletCgpInputs.value[p.outlet_id][p.customer_group_id] = Number(p.special_price)
      }
    }
  } catch (err) {
    console.error('Gagal memuat harga member outlet:', err)
  } finally {
    isLoadingOutletCgp.value = false
  }
}

const savedSuccessCgpKey = ref<string | null>(null)

function getOutletCgpVal(outletId: string, groupId: number): number | '' {
  const val = outletCgpInputs.value[outletId]?.[groupId]
  return val !== undefined && val !== null ? val : ''
}

function updateOutletCgpVal(outletId: string, groupId: number, event: Event) {
  if (!outletCgpInputs.value[outletId]) {
    outletCgpInputs.value[outletId] = {}
  }
  const raw = (event.target as HTMLInputElement).value
  const num = Number(raw)
  outletCgpInputs.value[outletId][groupId] = raw === '' || isNaN(num) || num <= 0 ? null : num
}

async function handleSaveOutletCgpPrice(outletId: string, groupId: number) {
  if (!authStore.hasPermission(['products:customer_group_prices', 'master:write'])) {
    dialog.alert('Akses ditolak: Anda tidak memiliki wewenang mengatur harga member outlet (products:customer_group_prices)', 'warning')
    return
  }
  const prodId = selectedProductForOutletCgp.value?.id
  if (!prodId || !outletId) return

  const inputVal = outletCgpInputs.value[outletId]?.[groupId]
  isSavingOutletCgp.value = true

  try {
    if (inputVal === null || inputVal === undefined || inputVal <= 0) {
      // Hapus override → fallback ke harga member global
      const existing = outletCgpPrices.value.find(
        (p) => p.outlet_id === outletId && p.customer_group_id === groupId
      )
      if (existing) {
        await api.delete(`/products/outlet-customer-group-prices/${existing.id}`)
      }
      if (outletCgpInputs.value[outletId]) {
        outletCgpInputs.value[outletId][groupId] = null
      }
    } else {
      await api.post(`/products/${prodId}/outlet-customer-group-prices`, {
        outlet_id: outletId,
        customer_group_id: groupId,
        special_price: Number(inputVal),
      })
    }
    // Refresh
    const pricesRes = await api.get(`/products/${prodId}/outlet-customer-group-prices`)
    if (pricesRes.data.success) {
      outletCgpPrices.value = pricesRes.data.data
      outletCgpInputs.value = {}
      for (const p of pricesRes.data.data) {
        if (!outletCgpInputs.value[p.outlet_id]) outletCgpInputs.value[p.outlet_id] = {}
        outletCgpInputs.value[p.outlet_id][p.customer_group_id] = Number(p.special_price)
      }
    }

    // Success indicator
    savedSuccessCgpKey.value = `${outletId}_${groupId}`
    setTimeout(() => {
      if (savedSuccessCgpKey.value === `${outletId}_${groupId}`) {
        savedSuccessCgpKey.value = null
      }
    }, 2500)
    dialog.toast('Harga member khusus outlet berhasil disimpan!', 'success')
  } catch (err: any) {
    dialog.alert(err.response?.data?.message || 'Gagal menyimpan harga member outlet', 'danger')
  } finally {
    isSavingOutletCgp.value = false
  }
}

async function handleSaveAllOutletCgp(outletId: string) {
  if (!authStore.hasPermission(['products:customer_group_prices', 'master:write'])) {
    dialog.alert('Akses ditolak: Anda tidak memiliki wewenang mengatur harga member outlet (products:customer_group_prices)', 'warning')
    return
  }
  const prodId = selectedProductForOutletCgp.value?.id
  if (!prodId || !outletId) return

  isSavingOutletCgp.value = true
  try {
    for (const group of outletCgpGroups.value) {
      const inputVal = outletCgpInputs.value[outletId]?.[group.id]
      if (inputVal === null || inputVal === undefined || inputVal <= 0) {
        const existing = outletCgpPrices.value.find(
          (p) => p.outlet_id === outletId && p.customer_group_id === group.id
        )
        if (existing) {
          await api.delete(`/products/outlet-customer-group-prices/${existing.id}`)
        }
      } else {
        await api.post(`/products/${prodId}/outlet-customer-group-prices`, {
          outlet_id: outletId,
          customer_group_id: group.id,
          special_price: Number(inputVal),
        })
      }
    }

    const pricesRes = await api.get(`/products/${prodId}/outlet-customer-group-prices`)
    if (pricesRes.data.success) {
      outletCgpPrices.value = pricesRes.data.data
      outletCgpInputs.value = {}
      for (const p of pricesRes.data.data) {
        if (!outletCgpInputs.value[p.outlet_id]) outletCgpInputs.value[p.outlet_id] = {}
        outletCgpInputs.value[p.outlet_id][p.customer_group_id] = Number(p.special_price)
      }
    }
    dialog.toast('Semua harga member untuk outlet ini berhasil disimpan!', 'success')
  } catch (err: any) {
    dialog.alert(err.response?.data?.message || 'Gagal menyimpan harga member outlet', 'danger')
  } finally {
    isSavingOutletCgp.value = false
  }
}

// Active Price Action Menu Dropdown
const activePriceMenuId = ref<string | null>(null)
function togglePriceMenu(prodId: string) {
  activePriceMenuId.value = activePriceMenuId.value === prodId ? null : prodId
}
function closePriceMenu() {
  activePriceMenuId.value = null
}

function handleLocationChanged() {
  currentPage.value = 1
  fetchProducts()
}

watch(
  () => locationStore.currentOutletId,
  (newId, oldId) => {
    if (newId !== oldId) {
      currentPage.value = 1
      fetchProducts()
    }
  }
)

onMounted(async () => {
  authStore.fetchCurrentUser()
  if (!locationStore.currentOutletId && locationStore.accessibleOutlets.length === 0) {
    await locationStore.fetchAccessibleOutlets()
  }
  fetchProducts()
  fetchMasterMeta()
  fetchGlobalCostingSettings()
  window.addEventListener('notane-location-changed', handleLocationChanged)
  window.addEventListener('click', closePriceMenu)
})

onUnmounted(() => {
  window.removeEventListener('notane-location-changed', handleLocationChanged)
  window.removeEventListener('click', closePriceMenu)
})
</script>

<template>
  <AdminLayout>
    <div class="p-6 space-y-4">
      <!-- Page Header -->
      <div class="flex items-center justify-between">
        <div>
          <h2 class="text-xl font-bold text-slate-900 flex items-center gap-2.5">
            <Icon icon="streamline-ultimate-color:warehouse-cart-packages-2" class="w-6 h-6 drop-shadow-xs" />
            <span>Master Produk & Penetapan Harga</span>
          </h2>
          <div class="flex items-center flex-wrap gap-2.5 mt-1">
            <p class="text-xs text-slate-500">Kelola SKU, multi-barcode, barcode timbangan elektrik, dan metode HPP.</p>
            <div
              v-if="locationStore.currentOutlet"
              class="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-md text-[11px] font-semibold bg-emerald-50 text-emerald-800 border border-emerald-300 shadow-2xs"
              title="Data harga jual dan stok fisik tabel di bawah otomatis sinkron dengan outlet aktif ini"
            >
              <Icon icon="fluent-color:building-store-24" class="w-3.5 h-3.5 text-emerald-700" />
              <span>Lokasi Terpilih: <strong>[{{ locationStore.currentOutlet.code }}] {{ locationStore.currentOutlet.name }}</strong></span>
            </div>
          </div>
        </div>

        <div class="flex items-center gap-2">
          <!-- Tombol Ekspor Excel Format Template Import -->
          <button
            v-if="authStore.hasPermission(['products:export', 'products:read', 'master:read'])"
            @click="exportProductsToExcel"
            class="bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-xs px-3.5 py-2 rounded-lg shadow-sm flex items-center gap-2 transition active:scale-95 border border-emerald-700 cursor-pointer"
            title="Ekspor seluruh produk terfilter ke format Excel template import (bisa langsung di-import di server lain)"
          >
            <FileSpreadsheet class="w-4 h-4" />
            <span>Ekspor Excel (Format Import)</span>
          </button>

          <button
            v-if="authStore.hasPermission(['products:import', 'master:write'])"
            @click="showImportModal = true"
            class="bg-purple-600 hover:bg-purple-700 text-white font-bold text-xs px-3.5 py-2 rounded-lg shadow-sm flex items-center gap-2 transition active:scale-95 border border-purple-700 cursor-pointer"
            title="Import data produk massal dari Excel/CSV, deteksi foto otomatis, harga outlet & member"
          >
            <Icon icon="streamline-ultimate-color:common-file-upload" class="w-4 h-4" />
            <span>Import Produk Canggih</span>
          </button>

          <button
            type="button"
            @click="router.push('/admin/parameters')"
            class="bg-white hover:bg-slate-50 text-slate-700 hover:text-slate-900 border border-slate-300 font-bold text-xs px-3.5 py-2 rounded-lg shadow-xs flex items-center gap-1.5 transition active:scale-95 cursor-pointer"
            title="Buka Master Parameter untuk mengatur metode perhitungan HPP global"
          >
            <Calculator class="w-4 h-4 text-emerald-600" />
            <span>HPP Global: <strong class="text-emerald-700 underline decoration-emerald-400">{{ globalCostingLabel }}</strong></span>
          </button>

          <button
            type="button"
            @click="router.push('/admin/price-labels')"
            class="bg-white hover:bg-slate-50 text-slate-700 hover:text-slate-900 border border-slate-300 font-bold text-xs px-3.5 py-2 rounded-lg shadow-xs flex items-center gap-2 transition active:scale-95 cursor-pointer"
            title="Cetak label harga rak display dan barcode produk secara massal"
          >
            <Icon icon="streamline-ultimate-color:tags-1" class="w-4 h-4" />
            <span>Cetak Label Harga</span>
          </button>

          <button
            v-if="authStore.hasPermission(['products:create', 'master:write'])"
            @click="showCreateModal = true"
            class="bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-xs px-4 py-2 rounded-lg shadow-sm flex items-center gap-2 transition active:scale-95 border border-emerald-700 cursor-pointer"
          >
            <Icon icon="streamline-ultimate-color:add-circle-bold" class="w-4 h-4" />
            <span>Tambah Produk Baru</span>
          </button>
        </div>
      </div>

      <!-- Filter / Search Bar (Responsive) -->
      <div class="flex flex-wrap items-center gap-3 bg-white border border-slate-300 p-3 rounded-xl shadow-sm">
        <div class="relative flex-1 min-w-[220px]">
          <Search class="w-4 h-4 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2" />
          <input
            v-model="searchQuery"
            @input="handleSearchInput"
            @keyup.enter="fetchProducts"
            placeholder="Cari SKU, Barcode, atau Nama Produk..."
            class="w-full bg-slate-50 border border-slate-300 rounded-lg pl-9 pr-4 py-1.5 text-xs text-slate-900 focus:outline-none focus:bg-white focus:border-emerald-600 font-medium"
          />
        </div>

        <!-- Filter Kategori (Searchable dari API) -->
        <div class="flex items-center gap-1.5 min-w-[220px]">
          <span class="text-slate-500 text-[11px] font-semibold flex items-center gap-1 shrink-0">
            <Filter class="w-3.5 h-3.5" /> Kategori:
          </span>
          <div class="w-48">
            <SearchableSelect
              v-model="selectedCategory"
              api-endpoint="/categories"
              :options="categories"
              value-key="id"
              label-key="name"
              secondary-key="code"
              placeholder="Semua Kategori"
              search-placeholder="Cari kategori..."
              :allow-clear="true"
              @change="currentPage = 1; fetchProducts()"
            />
          </div>
        </div>

        <!-- Filter Status Stok -->
        <div class="flex items-center gap-1.5 min-w-[140px]">
          <span class="text-slate-500 text-[11px] font-semibold">Stok:</span>
          <select
            v-model="selectedStockStatus"
            @change="currentPage = 1; fetchProducts()"
            class="bg-slate-50 border border-slate-300 text-slate-900 text-xs rounded-lg px-2.5 py-1.5 focus:outline-none focus:border-emerald-600 focus:bg-white"
          >
            <option value="ALL">Semua Stok</option>
            <option value="AVAILABLE">Tersedia (> 0)</option>
            <option value="LOW">Menipis (≤ Min)</option>
            <option value="EMPTY">Habis (0)</option>
          </select>
        </div>

        <button
          @click="currentPage = 1; fetchProducts()"
          class="bg-slate-100 hover:bg-slate-200 border border-slate-300 text-slate-700 text-xs px-4 py-1.5 rounded-lg transition font-bold cursor-pointer"
        >
          Terapkan
        </button>
      </div>

      <!-- Products Table with Responsive Overflow -->
      <div class="bg-white border border-slate-300 rounded-xl overflow-hidden shadow-sm">
        <div class="overflow-x-auto">
          <table class="w-full text-left text-xs min-w-[950px]">
            <thead class="bg-slate-100 border-b border-slate-300 text-slate-700 uppercase font-bold text-[11px] select-none">
              <tr>
                <th class="py-3 px-3 text-center w-12">Foto</th>
                <th
                  @click="handleSort('name')"
                  class="py-3 px-4 cursor-pointer hover:bg-slate-200/70 transition"
                  title="Klik untuk mengurutkan berdasarkan nama produk"
                >
                  <div class="flex items-center gap-1.5">
                    <span>Nama Produk</span>
                    <ArrowUp v-if="sortBy === 'name' && sortDir === 'asc'" class="w-3.5 h-3.5 text-emerald-700" />
                    <ArrowDown v-else-if="sortBy === 'name' && sortDir === 'desc'" class="w-3.5 h-3.5 text-emerald-700" />
                    <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                  </div>
                </th>
                <th
                  @click="handleSort('category_name')"
                  class="py-3 px-4 cursor-pointer hover:bg-slate-200/70 transition"
                >
                  <div class="flex items-center gap-1.5">
                    <span>Kategori</span>
                    <ArrowUp v-if="sortBy === 'category_name' && sortDir === 'asc'" class="w-3.5 h-3.5 text-emerald-700" />
                    <ArrowDown v-else-if="sortBy === 'category_name' && sortDir === 'desc'" class="w-3.5 h-3.5 text-emerald-700" />
                    <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                  </div>
                </th>
                <th
                  @click="handleSort('sku')"
                  class="py-3 px-4 cursor-pointer hover:bg-slate-200/70 transition"
                >
                  <div class="flex items-center gap-1.5">
                    <span>SKU / Barcode</span>
                    <ArrowUp v-if="sortBy === 'sku' && sortDir === 'asc'" class="w-3.5 h-3.5 text-emerald-700" />
                    <ArrowDown v-else-if="sortBy === 'sku' && sortDir === 'desc'" class="w-3.5 h-3.5 text-emerald-700" />
                    <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                  </div>
                </th>
                <th class="py-3 px-4">Satuan Dasar</th>
                <th
                  @click="handleSort('cost_price')"
                  class="py-3 px-4 text-right cursor-pointer hover:bg-slate-200/70 transition"
                >
                  <div class="flex items-center justify-end gap-1.5">
                    <span>HPP (Cost)</span>
                    <ArrowUp v-if="sortBy === 'cost_price' && sortDir === 'asc'" class="w-3.5 h-3.5 text-emerald-700" />
                    <ArrowDown v-else-if="sortBy === 'cost_price' && sortDir === 'desc'" class="w-3.5 h-3.5 text-emerald-700" />
                    <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                  </div>
                </th>
                <th
                  @click="handleSort('base_selling_price')"
                  class="py-3 px-4 text-right cursor-pointer hover:bg-slate-200/70 transition"
                  :title="`Harga jual untuk ${locationStore.currentOutlet?.name || 'Pusat'}`"
                >
                  <div class="flex items-center justify-end gap-1.5">
                    <div class="flex items-center gap-1">
                      <span>Harga Jual</span>
                      <span v-if="locationStore.currentOutlet" class="text-[9px] px-1 py-0.5 rounded bg-emerald-100 text-emerald-800 font-mono font-bold">{{ locationStore.currentOutlet.code }}</span>
                    </div>
                    <ArrowUp v-if="sortBy === 'base_selling_price' && sortDir === 'asc'" class="w-3.5 h-3.5 text-emerald-700" />
                    <ArrowDown v-else-if="sortBy === 'base_selling_price' && sortDir === 'desc'" class="w-3.5 h-3.5 text-emerald-700" />
                    <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                  </div>
                </th>
                <th
                  @click="handleSort('stock_qty')"
                  class="py-3 px-4 text-center cursor-pointer hover:bg-slate-200/70 transition"
                  :title="`Stok fisik untuk ${locationStore.currentOutlet?.name || 'Semua Gudang'}`"
                >
                  <div class="flex items-center justify-center gap-1.5">
                    <div class="flex items-center gap-1">
                      <span>Stok Fisik</span>
                      <span v-if="locationStore.currentOutlet" class="text-[9px] px-1 py-0.5 rounded bg-slate-200 text-slate-700 font-mono font-bold">{{ locationStore.currentOutlet.code }}</span>
                    </div>
                    <ArrowUp v-if="sortBy === 'stock_qty' && sortDir === 'asc'" class="w-3.5 h-3.5 text-emerald-700" />
                    <ArrowDown v-else-if="sortBy === 'stock_qty' && sortDir === 'desc'" class="w-3.5 h-3.5 text-emerald-700" />
                    <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                  </div>
                </th>
                <th v-if="authStore.hasPermission(['master:write', 'products:barcodes', 'products:outlet_prices', 'products:customer_group_prices'])" class="py-3 px-4 text-center">Aksi</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-200 font-mono">
              <tr v-if="isLoading">
                <td colspan="9" class="p-8 text-center text-slate-500 font-sans">
                  <div class="flex items-center justify-center gap-2">
                    <Icon icon="fluent:spinner-ios-20-filled" class="w-4 h-4 animate-spin text-emerald-600" />
                    <span>Memuat master produk dari server...</span>
                  </div>
                </td>
              </tr>
              <tr v-else-if="products.length === 0">
                <td colspan="9" class="p-8 text-center text-slate-500 font-sans">
                  Tidak ada produk yang sesuai dengan kriteria pencarian
                </td>
              </tr>
              <tr v-for="prod in products" :key="prod.id" class="hover:bg-emerald-50/40 transition">
                <!-- Foto Thumbnail -->
                <td class="py-2.5 px-3 text-center">
                  <div
                    @click="openImagePreview(prod.image_url)"
                    class="w-10 h-10 rounded-lg border border-slate-200 bg-slate-100 flex items-center justify-center mx-auto overflow-hidden shadow-2xs group relative"
                    :class="{ 'cursor-pointer hover:border-emerald-500': !!prod.image_url }"
                    title="Klik untuk pratinjau foto besar"
                  >
                    <img
                      v-if="prod.image_url"
                      :src="resolveImageUrl(prod.image_url)!"
                      :alt="prod.name"
                      class="w-full h-full object-cover transition-transform group-hover:scale-110"
                    />
                    <div v-else class="flex flex-col items-center justify-center gap-0.5">
                      <Icon icon="fluent-color:image-24" class="w-4 h-4 text-slate-300" />
                      <span class="text-[8px] text-slate-400 font-semibold leading-none">No Image</span>
                    </div>
                    <div
                      v-if="prod.image_url"
                      class="absolute inset-0 bg-black/30 opacity-0 group-hover:opacity-100 transition flex items-center justify-center text-white"
                    >
                      <Eye class="w-3.5 h-3.5" />
                    </div>
                  </div>
                </td>
                <td class="py-3 px-4 font-sans">
                  <div class="font-bold text-slate-900 flex items-center gap-2">
                    <span v-if="prod.is_weight_embedded" class="bg-amber-100 text-amber-800 border border-amber-300 text-[10px] px-1.5 py-0.5 rounded flex items-center gap-1 font-mono font-bold">
                      <Scale class="w-3 h-3" /> Timbang
                    </span>
                    <span>{{ prod.name }}</span>
                  </div>
                </td>
                <td class="py-3 px-4 font-sans text-slate-600">{{ prod.category_name }}</td>
                <td class="py-3 px-4 text-slate-700">
                  <div class="font-bold flex items-center gap-1.5">
                    <span>{{ prod.sku }}</span>
                  </div>
                  <div class="flex items-center gap-1.5 mt-0.5">
                    <span class="font-mono text-[11px] text-slate-700 bg-slate-100 px-1.5 py-0.5 rounded border border-slate-200">
                      {{ prod.barcode || '-' }}
                    </span>
                    <!-- Badge Multi-Barcode -->
                    <button
                      type="button"
                      @click="openBarcodesModal(prod)"
                      class="px-1.5 py-0.5 rounded text-[10px] font-bold border transition cursor-pointer flex items-center gap-0.5"
                      :class="prod.barcode_count > 0 ? 'bg-indigo-50 border-indigo-300 text-indigo-700 hover:bg-indigo-100' : 'bg-slate-50 border-slate-200 text-slate-500 hover:bg-slate-100'"
                      :title="prod.barcode_count > 0 ? `${prod.barcode_count} barcode alias aktif. Klik untuk kelola.` : 'Klik untuk tambah barcode alternatif / kemasan baru'"
                    >
                      <Barcode class="w-3 h-3" />
                      <span>{{ prod.barcode_count > 0 ? `+${prod.barcode_count}` : '+ Barcode' }}</span>
                    </button>
                  </div>
                </td>
                <!-- Satuan Dasar (Sesuai Header Kolom) -->
                <td class="py-3 px-4 text-center font-sans">
                  <span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-semibold bg-slate-100 border border-slate-200 text-slate-700 font-mono">
                    {{ prod.unit_code }}
                  </span>
                </td>
                <td class="py-3 px-4 text-right">
                  <div class="text-slate-700 font-semibold">{{ formatCurrency(prod.cost_price, true) }}</div>
                  <div class="flex items-center justify-end gap-1 mt-0.5">
                    <span 
                      class="text-[9px] px-1.5 py-0.5 rounded font-bold border inline-flex items-center gap-1"
                      :class="getProductCostingDisplay(prod.costing_method).isGlobal 
                        ? 'bg-slate-100 text-slate-700 border-slate-300' 
                        : getProductCostingDisplay(prod.costing_method).effectiveMethod === 'FIFO' 
                          ? 'bg-emerald-50 text-emerald-700 border-emerald-200' 
                          : getProductCostingDisplay(prod.costing_method).effectiveMethod === 'REPLACEMENT_COST'
                            ? 'bg-purple-50 text-purple-700 border-purple-200'
                            : 'bg-blue-50 text-blue-700 border-blue-200'"
                      :title="getProductCostingDisplay(prod.costing_method).tooltip"
                    >
                      <Globe v-if="getProductCostingDisplay(prod.costing_method).isGlobal" class="w-2.5 h-2.5 text-slate-400" />
                      <span>{{ getProductCostingDisplay(prod.costing_method).badgeText }}</span>
                    </span>
                  </div>
                </td>
                <td class="py-3 px-4 text-right text-emerald-700 font-bold">{{ formatCurrency(prod.base_selling_price, true) }}</td>
                <td class="py-3 px-4 text-center">
                  <span
                    class="px-2.5 py-1 rounded font-bold border"
                    :class="Number(prod.stock_qty) <= 0 ? 'bg-rose-50 border-rose-200 text-rose-700' : Number(prod.stock_qty) <= Number(prod.min_stock) ? 'bg-amber-50 border-amber-200 text-amber-800' : 'bg-slate-100 border-slate-300 text-slate-800'"
                  >
                    {{ formatQty(prod.stock_qty) }} {{ prod.unit_code }}
                  </span>
                </td>
                <td v-if="authStore.hasPermission(['master:write', 'products:barcodes', 'products:outlet_prices', 'products:customer_group_prices'])" class="py-3 px-4 text-center font-sans">
                  <div class="flex items-center justify-center gap-1.5">
                    <button
                      @click="openBatchesModal(prod)"
                      title="Lihat Batch Stok & Riwayat HPP Kulakan (FIFO)"
                      class="px-2 py-1 rounded-md bg-white hover:bg-emerald-50 border border-slate-300 hover:border-emerald-500 text-emerald-700 transition shadow-2xs flex items-center gap-1 text-[11px] font-bold cursor-pointer"
                    >
                      <Layers class="w-3.5 h-3.5" />
                      <span>Batch</span>
                    </button>
                    <button
                      v-if="authStore.hasPermission(['products:barcodes', 'master:write'])"
                      @click="openBarcodesModal(prod)"
                      title="Kelola Multi-Barcode (Barcode Utama & Alias Kemasan)"
                      class="px-2 py-1 rounded-md bg-white hover:bg-indigo-50 border border-slate-300 hover:border-indigo-500 text-indigo-600 transition shadow-2xs flex items-center gap-1 text-[11px] font-bold cursor-pointer"
                    >
                      <Barcode class="w-3.5 h-3.5" />
                      <span>Barcode</span>
                    </button>

                    <!-- Dropdown Menu Atur Harga -->
                    <div
                      v-if="authStore.hasPermission(['products:outlet_prices', 'products:customer_group_prices', 'master:write'])"
                      class="relative inline-block text-left"
                      @click.stop
                    >
                      <button
                        type="button"
                        @click="togglePriceMenu(prod.id)"
                        class="px-2 py-1 rounded-md bg-white hover:bg-amber-50 border border-slate-300 hover:border-amber-400 text-amber-800 transition shadow-2xs flex items-center gap-1 text-[11px] font-bold cursor-pointer"
                        title="Atur Harga Khusus (Outlet, Kelompok Pelanggan, Member per Outlet)"
                      >
                        <Tag class="w-3.5 h-3.5 text-amber-600" />
                        <span>Harga</span>
                        <Icon icon="fluent:chevron-down-12-filled" class="w-2.5 h-2.5 text-slate-400 ml-0.5" />
                      </button>

                      <!-- Dropdown Popup -->
                      <div
                        v-if="activePriceMenuId === prod.id"
                        class="absolute right-0 mt-1 w-52 bg-white rounded-lg shadow-xl border border-slate-200 py-1 z-30 text-xs animate-fadeIn text-left font-sans"
                      >
                        <div class="px-3 py-1 text-[10px] font-bold text-slate-400 uppercase tracking-wider border-b border-slate-100">
                          Penetapan Harga Khusus
                        </div>
                        <button
                          v-if="authStore.hasPermission(['products:outlet_prices', 'master:write'])"
                          type="button"
                          @click="openOutletPriceModal(prod); closePriceMenu()"
                          class="w-full px-3 py-1.5 text-left flex items-center gap-2 hover:bg-blue-50 text-slate-700 hover:text-blue-700 transition cursor-pointer"
                        >
                          <Building2 class="w-3.5 h-3.5 text-blue-600" />
                          <span>Harga Outlet / Cabang</span>
                        </button>
                        <button
                          v-if="authStore.hasPermission(['products:customer_group_prices', 'master:write'])"
                          type="button"
                          @click="openGroupPriceModal(prod); closePriceMenu()"
                          class="w-full px-3 py-1.5 text-left flex items-center gap-2 hover:bg-emerald-50 text-slate-700 hover:text-emerald-700 transition cursor-pointer"
                        >
                          <Users class="w-3.5 h-3.5 text-emerald-600" />
                          <span>Harga Kelompok Pelanggan</span>
                        </button>
                        <button
                          v-if="authStore.hasPermission(['products:customer_group_prices', 'master:write'])"
                          type="button"
                          @click="openOutletCgpModal(prod); closePriceMenu()"
                          class="w-full px-3 py-1.5 text-left flex items-center gap-2 hover:bg-purple-50 text-slate-700 hover:text-purple-700 transition cursor-pointer"
                        >
                          <Icon icon="fluent-color:building-people-24" class="w-3.5 h-3.5" />
                          <span>Harga Member per Outlet</span>
                        </button>
                      </div>
                    </div>

                    <!-- Action Icon Buttons -->
                    <button
                      v-if="authStore.hasPermission(['products:update', 'master:write'])"
                      @click="openEditModal(prod)"
                      title="Edit Data Produk"
                      class="p-1.5 rounded-md bg-white hover:bg-blue-50 border border-slate-300 hover:border-blue-500 text-blue-600 transition shadow-2xs cursor-pointer"
                    >
                      <Edit2 class="w-3.5 h-3.5" />
                    </button>
                    <button
                      v-if="authStore.hasPermission(['products:delete', 'master:write'])"
                      @click="handleDeleteProduct(prod.id, prod.name)"
                      title="Nonaktifkan Produk"
                      class="p-1.5 rounded-md bg-white hover:bg-rose-50 border border-slate-300 hover:border-rose-500 text-rose-600 transition shadow-2xs cursor-pointer"
                    >
                      <Trash2 class="w-3.5 h-3.5" />
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <!-- Server-side Pagination Bar -->
        <PaginationBar
          v-model:currentPage="currentPage"
          v-model:pageSize="pageSize"
          :totalItems="totalItems"
          @update:currentPage="onPageChange"
          @update:pageSize="onPageSizeChange"
        />
      </div>
    </div>

    <!-- Create Product Modal -->
    <div
      v-if="showCreateModal"
      class="fixed inset-0 z-50 bg-slate-900/50 backdrop-blur-xs flex items-center justify-center p-4 select-none"
    >
      <div class="bg-white border border-slate-300 w-full max-w-lg rounded-xl p-6 space-y-4 shadow-2xl">
        <h3 class="font-bold text-base text-slate-900">Tambah Master Produk Baru</h3>

        <div class="space-y-3 text-xs">
          <!-- Foto Produk Upload & Preview -->
          <div>
            <label class="block text-slate-700 font-bold mb-1">Foto Produk</label>
            <div class="flex items-center gap-3 bg-slate-50 border border-slate-200 p-2.5 rounded-xl">
              <div class="w-14 h-14 rounded-lg border border-slate-300 bg-white flex items-center justify-center overflow-hidden shrink-0 shadow-2xs relative group">
                <img
                  v-if="form.image_url"
                  :src="resolveImageUrl(form.image_url)!"
                  alt="Preview"
                  class="w-full h-full object-cover"
                />
                <Icon v-else icon="fluent-color:image-24" class="w-6 h-6 text-slate-400" />
              </div>

              <div class="flex-1 space-y-1.5 min-w-0">
                <div class="flex items-center gap-2">
                  <label
                    class="px-2.5 py-1 bg-white hover:bg-slate-100 border border-slate-300 rounded-md text-[11px] font-bold text-slate-700 cursor-pointer shadow-2xs transition active:scale-95 flex items-center gap-1.5"
                    :class="{ 'opacity-50 pointer-events-none': isUploadingPhoto }"
                  >
                    <Icon v-if="isUploadingPhoto" icon="line-md:loading-loop" class="w-3.5 h-3.5 text-emerald-600 animate-spin" />
                    <Upload v-else class="w-3.5 h-3.5 text-slate-500" />
                    <span>{{ isUploadingPhoto ? 'Mengupload...' : 'Upload Foto' }}</span>
                    <input
                      type="file"
                      accept="image/*"
                      class="hidden"
                      :disabled="isUploadingPhoto"
                      @change="handlePhotoFileSelected($event, 'CREATE')"
                    />
                  </label>
                  <button
                    v-if="form.image_url"
                    type="button"
                    :disabled="isUploadingPhoto"
                    @click="form.image_url = ''"
                    class="text-rose-600 hover:text-rose-800 text-[11px] font-bold px-1.5 py-0.5 disabled:opacity-50"
                  >
                    Hapus
                  </button>
                </div>
                <input
                  v-model="form.image_url"
                  placeholder="Atau tempel URL gambar (https://...)"
                  class="w-full bg-white border border-slate-300 rounded-md px-2 py-0.5 text-[11px] text-slate-800 focus:outline-none focus:border-emerald-600 truncate"
                />
              </div>
            </div>
          </div>

          <div>
            <label class="block text-slate-700 font-medium mb-1">Nama Produk *</label>
            <input v-model="form.name" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 focus:bg-white focus:border-emerald-600" />
          </div>

          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="block text-slate-700 font-medium mb-1">SKU *</label>
              <input v-model="form.sku" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono focus:bg-white focus:border-emerald-600" />
            </div>
            <div>
              <label class="block text-slate-700 font-medium mb-1">Barcode (EAN-13)</label>
              <input v-model="form.barcode" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono focus:bg-white focus:border-emerald-600" />
            </div>
          </div>

          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="block text-slate-700 font-medium mb-1">Kategori *</label>
              <SearchableSelect
                v-model="form.category_id"
                api-endpoint="/categories"
                placeholder="-- Pilih Kategori --"
                search-placeholder="Cari kategori barang..."
              />
            </div>
            <div>
              <label class="block text-slate-700 font-medium mb-1">Satuan Dasar (Base UoM) *</label>
              <SearchableSelect
                v-model="form.base_unit_id"
                api-endpoint="/units"
                label-key="name"
                secondary-key="code"
                placeholder="-- Pilih Satuan --"
                search-placeholder="Cari satuan (PCS, KG, DUS)..."
              />
            </div>
          </div>

          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="block text-slate-700 font-medium mb-1">Harga Beli Pokok (HPP Default)</label>
              <input v-model.number="form.cost_price" type="number" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono focus:bg-white focus:border-emerald-600" />
            </div>
            <div>
              <label class="block text-slate-700 font-medium mb-1">Harga Jual Dasar Default</label>
              <input v-model.number="form.base_selling_price" type="number" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono focus:bg-white focus:border-emerald-600" />
            </div>
          </div>

          <!-- Metode Perhitungan HPP & Penyesuaian Harga Kulakan -->
          <div class="p-3 bg-slate-50 border border-slate-200 rounded-xl space-y-2">
            <div class="flex items-center justify-between">
              <label class="block text-slate-800 font-bold">Metode Perhitungan HPP (Costing Method)</label>
              <span class="text-[10px] text-emerald-700 font-semibold bg-emerald-50 px-2 py-0.5 rounded-full border border-emerald-200">Dinamis per Outlet</span>
            </div>
            <select 
              v-model="form.costing_method" 
              class="w-full bg-white border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-medium focus:outline-none focus:border-emerald-600 cursor-pointer text-xs"
            >
              <option value="GLOBAL">Ikuti Pengaturan Global (Saat ini: {{ globalCostingLabel }}) — Standar Toko</option>
              <option value="WMA">Khusus Produk Ini: Rata-rata Tertimbang (Weighted Moving Average)</option>
              <option value="FIFO">Khusus Produk Ini: FIFO & FEFO (Prioritas Kedaluwarsa & Batch Lot)</option>
              <option value="REPLACEMENT_COST">Khusus Produk Ini: Harga Ganti (Replacement Cost)</option>
            </select>
            <p class="text-[11px] text-slate-500 leading-relaxed">
              <span v-if="form.costing_method === 'GLOBAL'">
                🌐 <strong>Pengaturan Global:</strong> Otomatis mengikuti metode HPP toko (<strong>{{ globalCostingLabel }}</strong>). Jika master parameter diubah, produk ini akan otomatis ikut diperbarui.
              </span>
              <span v-else-if="form.costing_method === 'WMA'">
                💡 <strong>Rata-rata Tertimbang:</strong> Nilai HPP outlet dihitung proporsional dari gabungan stok lama dan kulakan baru.
              </span>
              <span v-else-if="form.costing_method === 'FIFO' || form.costing_method === 'FEFO'">
                📦 <strong>FIFO & FEFO:</strong> Setiap kulakan dicatat per batch lot. Batch dengan tanggal kedaluwarsa terdekat (FEFO) atau masuk tertua (FIFO) dipotong terlebih dahulu saat kasir menjual barang.
              </span>
              <span v-else-if="form.costing_method === 'REPLACEMENT_COST'">
                ⚡ <strong>Harga Ganti:</strong> Nilai HPP outlet langsung disamakan dengan harga beli faktur pembelian terbaru.
              </span>
            </p>

            <!-- Opsi Tambahan jika Replacement Cost -->
            <div v-if="getProductCostingDisplay(form.costing_method).effectiveMethod === 'REPLACEMENT_COST'" class="pt-2 border-t border-slate-200 space-y-2">
              <div class="flex items-center gap-2">
                <input 
                  v-model="form.auto_adjust_selling_price" 
                  type="checkbox" 
                  id="create_auto_adj_selling" 
                  class="w-4 h-4 rounded text-emerald-600 border-slate-300 cursor-pointer"
                />
                <label for="create_auto_adj_selling" class="text-slate-800 font-medium cursor-pointer">
                  Otomatis sesuaikan harga jual dasar outlet saat kulakan baru
                </label>
              </div>
              <div v-if="form.auto_adjust_selling_price" class="flex items-center gap-2 pl-6">
                <label class="text-[11px] text-slate-600">Target Markup Keuntungan:</label>
                <div class="relative w-28">
                  <input 
                    v-model.number="form.markup_percent" 
                    type="number" 
                    placeholder="Otomatis" 
                    class="w-full bg-white border border-slate-300 rounded-lg px-2 py-1 text-right font-mono text-xs pr-6"
                  />
                  <span class="absolute right-2 top-1/2 -translate-y-1/2 text-slate-400 font-bold text-xs">%</span>
                </div>
                <span class="text-[10px] text-slate-400">(Kosongkan untuk mempertahankan rasio laba lama)</span>
              </div>
            </div>
          </div>

          <div class="flex items-center gap-2 pt-2">
            <input v-model="form.is_weight_embedded" type="checkbox" id="w_emb" class="w-4 h-4 rounded text-emerald-600 border-slate-300" />
            <label for="w_emb" class="text-slate-800">Produk Barcode Timbangan (Buah / Sayur Elektrik)</label>
          </div>
        </div>

        <div class="flex justify-end gap-2 pt-4 border-t border-slate-200">
          <button @click="showCreateModal = false" :disabled="isSavingProduct" class="bg-slate-100 hover:bg-slate-200 border border-slate-300 text-slate-700 px-4 py-2 rounded-lg text-xs font-bold cursor-pointer disabled:opacity-50">Batal</button>
          <button @click="handleCreateProduct" :disabled="isSavingProduct" class="bg-emerald-600 hover:bg-emerald-700 disabled:opacity-50 border border-emerald-700 text-white px-5 py-2 rounded-lg text-xs font-bold shadow-sm cursor-pointer flex items-center gap-1.5">
            <span v-if="isSavingProduct">Menyimpan...</span>
            <span v-else>Simpan Produk</span>
          </button>
        </div>
      </div>
    </div>

    <!-- Edit Product Modal -->
    <div
      v-if="showEditModal"
      class="fixed inset-0 z-50 bg-slate-900/50 backdrop-blur-xs flex items-center justify-center p-4 select-none"
    >
      <div class="bg-white border border-slate-300 w-full max-w-lg rounded-xl p-6 space-y-4 shadow-2xl">
        <h3 class="font-bold text-base text-slate-900">Edit Master Produk</h3>

        <div class="space-y-3 text-xs">
          <!-- Foto Produk Upload & Preview -->
          <div>
            <label class="block text-slate-700 font-bold mb-1">Foto Produk</label>
            <div class="flex items-center gap-3 bg-slate-50 border border-slate-200 p-2.5 rounded-xl">
              <div class="w-14 h-14 rounded-lg border border-slate-300 bg-white flex items-center justify-center overflow-hidden shrink-0 shadow-2xs relative group">
                <img
                  v-if="editForm.image_url"
                  :src="resolveImageUrl(editForm.image_url)!"
                  alt="Preview"
                  class="w-full h-full object-cover"
                />
                <Icon v-else icon="fluent-color:image-24" class="w-6 h-6 text-slate-400" />
              </div>

              <div class="flex-1 space-y-1.5 min-w-0">
                <div class="flex items-center gap-2">
                  <label
                    class="px-2.5 py-1 bg-white hover:bg-slate-100 border border-slate-300 rounded-md text-[11px] font-bold text-slate-700 cursor-pointer shadow-2xs transition active:scale-95 flex items-center gap-1.5"
                    :class="{ 'opacity-50 pointer-events-none': isUploadingPhoto }"
                  >
                    <Icon v-if="isUploadingPhoto" icon="line-md:loading-loop" class="w-3.5 h-3.5 text-emerald-600 animate-spin" />
                    <Upload v-else class="w-3.5 h-3.5 text-slate-500" />
                    <span>{{ isUploadingPhoto ? 'Mengupload...' : 'Ganti Foto' }}</span>
                    <input
                      type="file"
                      accept="image/*"
                      class="hidden"
                      :disabled="isUploadingPhoto"
                      @change="handlePhotoFileSelected($event, 'EDIT')"
                    />
                  </label>
                  <button
                    v-if="editForm.image_url"
                    type="button"
                    :disabled="isUploadingPhoto"
                    @click="editForm.image_url = ''"
                    class="text-rose-600 hover:text-rose-800 text-[11px] font-bold px-1.5 py-0.5 disabled:opacity-50"
                  >
                    Hapus
                  </button>
                </div>
                <input
                  v-model="editForm.image_url"
                  placeholder="Atau tempel URL gambar (https://...)"
                  class="w-full bg-white border border-slate-300 rounded-md px-2 py-0.5 text-[11px] text-slate-800 focus:outline-none focus:border-emerald-600 truncate"
                />
              </div>
            </div>
          </div>

          <div>
            <label class="block text-slate-700 font-medium mb-1">Nama Produk</label>
            <input v-model="editForm.name" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-semibold focus:bg-white focus:border-emerald-600" />
          </div>

          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="block text-slate-700 font-medium mb-1">SKU</label>
              <input v-model="editForm.sku" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono focus:bg-white focus:border-emerald-600" />
            </div>
            <div>
              <label class="block text-slate-700 font-medium mb-1">Barcode (EAN-13)</label>
              <input v-model="editForm.barcode" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono focus:bg-white focus:border-emerald-600" />
            </div>
          </div>

          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="block text-slate-700 font-medium mb-1">Kategori</label>
              <SearchableSelect
                v-model="editForm.category_id"
                api-endpoint="/categories"
                placeholder="-- Pilih Kategori --"
                search-placeholder="Cari kategori..."
              />
            </div>
            <div>
              <label class="block text-slate-700 font-medium mb-1">Satuan Dasar (Base UoM)</label>
              <SearchableSelect
                v-model="editForm.base_unit_id"
                api-endpoint="/units"
                label-key="name"
                secondary-key="code"
                placeholder="-- Pilih Satuan --"
                search-placeholder="Cari satuan..."
              />
            </div>
          </div>

          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="block text-slate-700 font-medium mb-1">Harga Beli Pokok (HPP Global)</label>
              <input v-model.number="editForm.cost_price" type="number" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono focus:bg-white focus:border-emerald-600" />
            </div>
            <div>
              <label class="block text-slate-700 font-medium mb-1">Harga Jual Dasar Global</label>
              <input v-model.number="editForm.base_selling_price" type="number" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono focus:bg-white focus:border-emerald-600" />
            </div>
          </div>

          <!-- Metode Perhitungan HPP & Penyesuaian Harga Kulakan -->
          <div class="p-3 bg-slate-50 border border-slate-200 rounded-xl space-y-2">
            <div class="flex items-center justify-between">
              <label class="block text-slate-800 font-bold">Metode Perhitungan HPP (Costing Method)</label>
              <span class="text-[10px] text-emerald-700 font-semibold bg-emerald-50 px-2 py-0.5 rounded-full border border-emerald-200">Dinamis per Outlet</span>
            </div>
            <select 
              v-model="editForm.costing_method" 
              class="w-full bg-white border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-medium focus:outline-none focus:border-emerald-600 cursor-pointer text-xs"
            >
              <option value="GLOBAL">Ikuti Pengaturan Global (Saat ini: {{ globalCostingLabel }}) — Standar Toko</option>
              <option value="WMA">Khusus Produk Ini: Rata-rata Tertimbang (Weighted Moving Average)</option>
              <option value="FIFO">Khusus Produk Ini: FIFO & FEFO (Prioritas Kedaluwarsa & Batch Lot)</option>
              <option value="REPLACEMENT_COST">Khusus Produk Ini: Harga Ganti (Replacement Cost)</option>
            </select>
            <p class="text-[11px] text-slate-500 leading-relaxed">
              <span v-if="editForm.costing_method === 'GLOBAL'">
                🌐 <strong>Pengaturan Global:</strong> Otomatis mengikuti metode HPP toko (<strong>{{ globalCostingLabel }}</strong>). Jika master parameter diubah, produk ini akan otomatis ikut diperbarui.
              </span>
              <span v-else-if="editForm.costing_method === 'WMA'">
                💡 <strong>Rata-rata Tertimbang:</strong> Nilai HPP outlet dihitung proporsional dari gabungan stok lama dan kulakan baru.
              </span>
              <span v-else-if="editForm.costing_method === 'FIFO' || editForm.costing_method === 'FEFO'">
                📦 <strong>FIFO & FEFO:</strong> Setiap kulakan dicatat per batch lot. Batch dengan tanggal kedaluwarsa terdekat (FEFO) atau masuk tertua (FIFO) dipotong terlebih dahulu saat kasir menjual barang.
              </span>
              <span v-else-if="editForm.costing_method === 'REPLACEMENT_COST'">
                ⚡ <strong>Harga Ganti:</strong> Nilai HPP outlet langsung disamakan dengan harga beli faktur pembelian terbaru.
              </span>
            </p>

            <!-- Opsi Tambahan jika Replacement Cost -->
            <div v-if="getProductCostingDisplay(editForm.costing_method).effectiveMethod === 'REPLACEMENT_COST'" class="pt-2 border-t border-slate-200 space-y-2">
              <div class="flex items-center gap-2">
                <input 
                  v-model="editForm.auto_adjust_selling_price" 
                  type="checkbox" 
                  id="edit_auto_adj_selling" 
                  class="w-4 h-4 rounded text-emerald-600 border-slate-300 cursor-pointer"
                />
                <label for="edit_auto_adj_selling" class="text-slate-800 font-medium cursor-pointer">
                  Otomatis sesuaikan harga jual dasar outlet saat kulakan baru
                </label>
              </div>
              <div v-if="editForm.auto_adjust_selling_price" class="flex items-center gap-2 pl-6">
                <label class="text-[11px] text-slate-600">Target Markup Keuntungan:</label>
                <div class="relative w-28">
                  <input 
                    v-model.number="editForm.markup_percent" 
                    type="number" 
                    placeholder="Otomatis" 
                    class="w-full bg-white border border-slate-300 rounded-lg px-2 py-1 text-right font-mono text-xs pr-6"
                  />
                  <span class="absolute right-2 top-1/2 -translate-y-1/2 text-slate-400 font-bold text-xs">%</span>
                </div>
                <span class="text-[10px] text-slate-400">(Kosongkan untuk mempertahankan rasio laba lama)</span>
              </div>
            </div>
          </div>

          <div class="grid grid-cols-3 gap-3">
            <div>
              <label class="block text-slate-700 font-medium mb-1">Min. Stok</label>
              <input v-model.number="editForm.min_stock" type="number" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono focus:bg-white focus:border-emerald-600" />
            </div>
            <div>
              <label class="block text-slate-700 font-medium mb-1">Maks. Stok</label>
              <input v-model.number="editForm.max_stock" type="number" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono focus:bg-white focus:border-emerald-600" />
            </div>
            <div>
              <label class="block text-slate-700 font-medium mb-1">PPN (%)</label>
              <input v-model.number="editForm.tax_percent" type="number" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono focus:bg-white focus:border-emerald-600" />
            </div>
          </div>

          <div class="flex items-center gap-2 pt-2">
            <input v-model="editForm.is_weight_embedded" type="checkbox" id="edit_w_emb" class="w-4 h-4 rounded text-emerald-600 border-slate-300" />
            <label for="edit_w_emb" class="text-slate-800">Produk Barcode Timbangan (Buah / Sayur Elektrik)</label>
          </div>
        </div>

        <div class="flex justify-end gap-2 pt-4 border-t border-slate-200">
          <button @click="showEditModal = false" :disabled="isSavingProduct" class="bg-slate-100 hover:bg-slate-200 border border-slate-300 text-slate-700 px-4 py-2 rounded-lg text-xs font-bold cursor-pointer disabled:opacity-50">Batal</button>
          <button @click="handleUpdateProduct" :disabled="isSavingProduct" class="bg-emerald-600 hover:bg-emerald-700 disabled:opacity-50 border border-emerald-700 text-white px-5 py-2 rounded-lg text-xs font-bold shadow-sm cursor-pointer flex items-center gap-1.5">
            <span v-if="isSavingProduct">Menyimpan...</span>
            <span v-else>Perbarui Produk</span>
          </button>
        </div>
      </div>
    </div>

    <!-- Customer Group Prices Modal -->
    <div
      v-if="showGroupPriceModal"
      class="fixed inset-0 z-50 bg-slate-900/50 backdrop-blur-xs flex items-center justify-center p-4 select-none"
    >
      <div class="bg-white border border-slate-300 w-full max-w-xl rounded-2xl p-6 space-y-4 shadow-2xl text-slate-900">
        <!-- Header -->
        <div class="flex items-center justify-between border-b border-slate-200 pb-3">
          <div>
            <h3 class="font-bold text-base text-slate-900 flex items-center gap-2">
              <Users class="w-5 h-5 text-emerald-600" />
              <span>Harga per Kelompok Pelanggan</span>
            </h3>
            <p class="text-xs text-slate-500 mt-0.5">
              Produk: <strong class="text-slate-900">{{ selectedProductForGroupPrice?.name }}</strong> 
              (Harga Dasar: <span class="font-mono text-emerald-700 font-bold">Rp {{ Number(selectedProductForGroupPrice?.base_selling_price || 0).toLocaleString() }}</span>)
            </p>
          </div>
          <button @click="showGroupPriceModal = false" class="text-slate-400 hover:text-slate-600 font-bold text-lg">✕</button>
        </div>

        <div class="p-3 bg-amber-50 border border-amber-200 rounded-xl text-xs text-amber-900 flex items-start gap-2">
          <Tag class="w-4 h-4 text-amber-600 shrink-0 mt-0.5" />
          <span>
            Aturan: Jika harga khusus pada suatu kelompok diisi, harga tersebut yang otomatis berlaku di kasir saat member kelompok tersebut bertransaksi. Jika dikosongkan (atau diisi 0), otomatis berlaku <strong>Harga Jual Dasar (Rp {{ Number(selectedProductForGroupPrice?.base_selling_price || 0).toLocaleString() }})</strong>.
          </span>
        </div>

        <!-- Groups Table / List -->
        <div class="space-y-3 max-h-80 overflow-y-auto pr-1">
          <div v-if="isLoadingGroupPrices" class="py-8 text-center text-slate-500 text-xs flex items-center justify-center gap-2">
            <span>Memuat kelompok pelanggan...</span>
          </div>

          <div v-else-if="customerGroups.length === 0" class="py-8 text-center text-slate-500 text-xs">
            Belum ada kelompok pelanggan terdaftar di sistem.
          </div>

          <div
            v-else
            v-for="grp in customerGroups"
            :key="grp.id"
            class="flex items-center justify-between p-3 rounded-xl border border-slate-200 bg-slate-50/60 hover:bg-white hover:border-emerald-300 transition"
          >
            <div>
              <div class="font-bold text-xs text-slate-900 flex items-center gap-1.5">
                <span class="px-2 py-0.5 rounded bg-emerald-100 text-emerald-800 text-[10px] font-mono font-bold">{{ grp.name }}</span>
              </div>
              <p class="text-[11px] text-slate-500 mt-0.5">{{ grp.description || 'Tidak ada deskripsi' }}</p>
            </div>

            <div class="flex items-center gap-2">
              <div class="relative w-44">
                <span class="absolute left-2.5 top-1/2 -translate-y-1/2 text-slate-400 text-xs font-mono">Rp</span>
                <input
                  v-model.number="groupPriceInputs[grp.id]"
                  type="number"
                  :placeholder="String(selectedProductForGroupPrice?.base_selling_price || 'Harga dasar')"
                  class="w-full bg-white border border-slate-300 rounded-lg pl-8 pr-2.5 py-1.5 text-xs text-slate-900 font-mono font-bold text-right focus:outline-none focus:border-emerald-600"
                  @keyup.enter="handleSaveGroupPrice(grp.id)"
                />
              </div>

              <button
                @click="handleSaveGroupPrice(grp.id)"
                :disabled="isSavingGroupPrices"
                title="Simpan Harga"
                class="p-2 bg-emerald-600 hover:bg-emerald-700 disabled:opacity-50 text-white rounded-lg transition shadow-xs flex items-center justify-center"
              >
                <Check class="w-3.5 h-3.5" />
              </button>
            </div>
          </div>
        </div>

        <!-- Footer -->
        <div class="flex justify-end gap-2 pt-3 border-t border-slate-200">
          <button
            @click="showGroupPriceModal = false"
            class="bg-slate-800 hover:bg-slate-900 text-white px-5 py-2 rounded-xl text-xs font-bold transition shadow-xs"
          >
            Selesai
          </button>
        </div>
      </div>
    </div>

    <!-- Outlet-Specific Prices Modal -->
    <div
      v-if="showOutletPriceModal"
      class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/60 backdrop-blur-xs"
    >
      <div class="bg-white rounded-2xl shadow-xl w-full max-w-2xl overflow-hidden border border-slate-200 p-6 space-y-4">
        <!-- Header -->
        <div class="flex items-center justify-between border-b border-slate-200 pb-3">
          <div class="flex items-center gap-2.5">
            <div class="p-2 rounded-xl bg-blue-100 text-blue-700">
              <Building2 class="w-5 h-5" />
            </div>
            <div>
              <h3 class="font-bold text-slate-900 text-base">Harga & HPP per Outlet / Cabang</h3>
              <p class="text-xs text-slate-500">
                {{ selectedProductForOutletPrice?.name }} ({{ selectedProductForOutletPrice?.sku }})
              </p>
            </div>
          </div>
          <button
            @click="showOutletPriceModal = false"
            class="text-slate-400 hover:text-slate-600 p-1 rounded-lg hover:bg-slate-100 transition"
          >
            ✕
          </button>
        </div>

        <!-- Global Price Info Box -->
        <div class="p-3 bg-blue-50/70 border border-blue-200 rounded-xl text-xs text-blue-900 flex items-start gap-2">
          <Building2 class="w-4 h-4 text-blue-600 shrink-0 mt-0.5" />
          <div class="space-y-1">
            <p>
              Harga Global: <strong>HPP: Rp {{ Number(selectedProductForOutletPrice?.cost_price || 0).toLocaleString() }}</strong> | 
              <strong>Harga Jual: Rp {{ Number(selectedProductForOutletPrice?.base_selling_price || 0).toLocaleString() }}</strong>.
            </p>
            <p class="text-[11px] text-blue-700">
              Outlet dengan input kosong akan otomatis memakai Harga Global. Isi HPP dan Harga Jual khusus outlet di bawah lalu klik tombol simpan (✓).
            </p>
          </div>
        </div>

        <!-- Outlets Table / List -->
        <div class="space-y-3 max-h-96 overflow-y-auto pr-1">
          <div v-if="isLoadingOutletPrices" class="py-8 text-center text-slate-500 text-xs flex items-center justify-center gap-2">
            <span>Memuat data outlet...</span>
          </div>

          <div v-else-if="outletsList.length === 0" class="py-8 text-center text-slate-500 text-xs">
            Tidak ada lokasi outlet aktif yang dapat Anda kelola (Anda tidak memiliki hak akses outlet atau belum ditugaskan).
          </div>

          <div
            v-else
            v-for="outlet in outletsList"
            :key="outlet.id"
            :class="[
              'p-3 rounded-xl border transition space-y-2',
              authStore.hasOutletAccess(outlet.id)
                ? 'border-slate-200 bg-slate-50/60 hover:bg-white hover:border-blue-300'
                : 'border-slate-200 bg-slate-100/70 opacity-75'
            ]"
          >
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-2">
                <span class="px-2 py-0.5 rounded bg-blue-100 text-blue-800 text-[10px] font-mono font-bold">
                  {{ outlet.code }}
                </span>
                <span class="font-bold text-xs text-slate-900">{{ outlet.name }}</span>
                <span v-if="outlet.is_warehouse" class="text-[9px] bg-amber-100 text-amber-800 px-1.5 py-0.5 rounded font-semibold">
                  Gudang
                </span>
              </div>
              <div class="flex items-center gap-1.5">
                <span v-if="!authStore.hasOutletAccess(outlet.id)" class="text-[10px] text-amber-800 font-bold bg-amber-50 px-2 py-0.5 rounded-full border border-amber-200 flex items-center gap-1">
                  <Lock class="w-3 h-3 text-amber-600" />
                  <span>Tidak Ada Akses</span>
                </span>
                <span v-else-if="outletPriceInputs[outlet.id]?.cost_price !== undefined || outletPriceInputs[outlet.id]?.base_selling_price !== undefined" class="text-[10px] text-emerald-600 font-bold bg-emerald-50 px-2 py-0.5 rounded-full border border-emerald-200">
                  Kustom Terpasang
                </span>
              </div>
            </div>

            <div class="grid grid-cols-1 sm:grid-cols-12 gap-2 items-center">
              <!-- HPP Outlet -->
              <div class="sm:col-span-5">
                <label class="block text-[10px] font-medium text-slate-500 mb-0.5 flex items-center justify-between">
                  <span>HPP / Cost Price</span>
                  <span v-if="getOutletPriceDetail(outlet.id)?.last_purchase_price" class="text-blue-600 font-mono text-[9px] font-semibold" :title="'Harga kulakan terakhir di outlet/gudang ini'">
                    Kulakan: {{ formatCurrency(getOutletPriceDetail(outlet.id)?.last_purchase_price) }}
                  </span>
                </label>
                <div class="relative">
                  <span class="absolute left-2 top-1/2 -translate-y-1/2 text-slate-400 text-xs font-mono">Rp</span>
                  <input
                    v-model.number="(outletPriceInputs[outlet.id] = outletPriceInputs[outlet.id] || { cost_price: null, base_selling_price: null }).cost_price"
                    type="number"
                    :disabled="!authStore.hasOutletAccess(outlet.id)"
                    :placeholder="String(selectedProductForOutletPrice?.cost_price || 0)"
                    class="w-full bg-white border border-slate-300 rounded-lg pl-7 pr-2 py-1.5 text-xs text-slate-900 font-mono font-bold text-right focus:outline-none focus:border-blue-600 disabled:bg-slate-100 disabled:text-slate-400 disabled:cursor-not-allowed"
                    @keyup.enter="authStore.hasOutletAccess(outlet.id) && handleSaveOutletPrice(outlet.id)"
                  />
                </div>
              </div>

              <!-- Harga Jual Outlet -->
              <div class="sm:col-span-5">
                <label class="block text-[10px] font-medium text-slate-500 mb-0.5">Harga Jual Dasar</label>
                <div class="relative">
                  <span class="absolute left-2 top-1/2 -translate-y-1/2 text-slate-400 text-xs font-mono">Rp</span>
                  <input
                    v-model.number="(outletPriceInputs[outlet.id] = outletPriceInputs[outlet.id] || { cost_price: null, base_selling_price: null }).base_selling_price"
                    type="number"
                    :disabled="!authStore.hasOutletAccess(outlet.id)"
                    :placeholder="String(selectedProductForOutletPrice?.base_selling_price || 0)"
                    class="w-full bg-white border border-slate-300 rounded-lg pl-7 pr-2 py-1.5 text-xs text-slate-900 font-mono font-bold text-right focus:outline-none focus:border-blue-600 disabled:bg-slate-100 disabled:text-slate-400 disabled:cursor-not-allowed"
                    @keyup.enter="authStore.hasOutletAccess(outlet.id) && handleSaveOutletPrice(outlet.id)"
                  />
                </div>
              </div>

              <!-- Save Action -->
              <div class="sm:col-span-2 flex items-end justify-end pt-3 sm:pt-0">
                <button
                  @click="handleSaveOutletPrice(outlet.id)"
                  :disabled="!authStore.hasOutletAccess(outlet.id) || isSavingOutletPrices"
                  :title="authStore.hasOutletAccess(outlet.id) ? 'Simpan Harga Outlet' : 'Anda tidak memiliki hak akses outlet ini'"
                  class="w-full py-1.5 bg-blue-600 hover:bg-blue-700 disabled:opacity-40 disabled:cursor-not-allowed text-white rounded-lg transition shadow-xs flex items-center justify-center gap-1 text-xs font-bold"
                >
                  <Lock v-if="!authStore.hasOutletAccess(outlet.id)" class="w-3.5 h-3.5" />
                  <Check v-else class="w-3.5 h-3.5" />
                  <span>{{ authStore.hasOutletAccess(outlet.id) ? 'Simpan' : 'Terkunci' }}</span>
                </button>
              </div>
            </div>
          </div>
        </div>

        <!-- Footer -->
        <div class="flex justify-end gap-2 pt-3 border-t border-slate-200">
          <button
            @click="showOutletPriceModal = false"
            class="bg-slate-800 hover:bg-slate-900 text-white px-5 py-2 rounded-xl text-xs font-bold transition shadow-xs"
          >
            Selesai
          </button>
        </div>
      </div>
    </div>

  <!-- ===== MODAL: HARGA MEMBER PER OUTLET ===== -->
  <div
    v-if="showOutletCgpModal"
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm"
  >
    <div class="bg-white rounded-2xl shadow-2xl w-full max-w-2xl mx-4 flex flex-col max-h-[90vh]">
      <!-- Header -->
      <div class="flex items-center justify-between px-6 py-4 border-b border-slate-200">
        <div class="flex items-center gap-3">
          <div class="w-9 h-9 rounded-xl bg-gradient-to-br from-purple-500 to-indigo-600 flex items-center justify-center">
            <Icon icon="fluent-color:building-people-24" class="w-5 h-5" />
          </div>
          <div>
            <h2 class="font-extrabold text-slate-900 text-base">Harga Member per Outlet</h2>
            <p class="text-xs text-slate-500 mt-0.5">{{ selectedProductForOutletCgp?.name }}</p>
          </div>
        </div>
        <button @click="showOutletCgpModal = false" class="text-slate-400 hover:text-slate-600 font-bold text-lg">✕</button>
      </div>

      <!-- Info banner -->
      <div class="mx-6 mt-4 bg-purple-50 border border-purple-200 rounded-xl px-4 py-3 text-xs text-purple-800">
        <strong>Prioritas harga di kasir:</strong>
        <span class="ml-1">Harga Member+Outlet (ini)</span>
        <span class="text-purple-400 mx-1">›</span>
        <span>Harga Member Global</span>
        <span class="text-purple-400 mx-1">›</span>
        <span>Harga Outlet</span>
        <span class="text-purple-400 mx-1">›</span>
        <span>Harga Dasar Produk</span>
      </div>

      <!-- Outlet Tabs -->
      <div class="px-6 pt-4">
        <div class="flex gap-1 border-b border-slate-200">
          <button
            v-for="outlet in outletCgpOutlets"
            :key="outlet.id"
            @click="outletCgpSelectedOutlet = outlet.id"
            class="px-3 py-2 text-xs font-semibold border-b-2 transition-colors"
            :class="outletCgpSelectedOutlet === outlet.id
              ? 'border-purple-500 text-purple-700 bg-purple-50/50'
              : 'border-transparent text-slate-500 hover:text-slate-700'"
          >
            {{ outlet.name }}
          </button>
        </div>
      </div>

      <!-- Price Matrix -->
      <div class="flex-1 overflow-y-auto px-6 py-4">
        <div v-if="isLoadingOutletCgp" class="text-center py-8 text-slate-400 text-sm">Memuat data...</div>
        <div v-else-if="!outletCgpSelectedOutlet" class="text-center py-8 text-slate-400 text-sm">Pilih outlet di atas</div>
        <div v-else>
          <p class="text-xs text-slate-500 mb-3">
            Isi harga khusus per kelompok member untuk outlet ini. Kosongkan / isi 0 untuk kembali ke harga member global atau harga dasar outlet.
          </p>
          <div class="space-y-2">
            <div
              v-for="group in outletCgpGroups"
              :key="group.id"
              class="flex items-center gap-3 bg-slate-50 border border-slate-200 rounded-xl px-4 py-3"
            >
              <div class="flex-1">
                <div class="font-semibold text-slate-800 text-sm">{{ group.name }}</div>
                <div class="text-xs text-slate-400 mt-0.5">
                  Harga member global:
                  <span class="font-mono font-bold text-slate-600">
                    Rp {{ Number(outletCgpGroups.find((g: any) => g.id === group.id)?.base_price || 0).toLocaleString('id-ID') }}
                  </span>
                </div>
              </div>
              <div class="flex items-center gap-2">
                <span class="text-xs text-slate-500">Rp</span>
                <input
                  type="number"
                  min="0"
                  step="100"
                  :placeholder="'Harga khusus ' + group.name"
                  :value="outletCgpSelectedOutlet ? getOutletCgpVal(outletCgpSelectedOutlet, group.id) : ''"
                  @input="outletCgpSelectedOutlet && updateOutletCgpVal(outletCgpSelectedOutlet, group.id, $event)"
                  @keyup.enter="outletCgpSelectedOutlet && handleSaveOutletCgpPrice(outletCgpSelectedOutlet, group.id)"
                  class="w-32 px-3 py-1.5 border border-slate-300 rounded-lg text-sm text-right focus:outline-none focus:border-purple-400 focus:ring-2 focus:ring-purple-100 font-mono"
                  :class="(Number(outletCgpSelectedOutlet && outletCgpInputs[outletCgpSelectedOutlet]?.[group.id]) || 0) > 0 ? 'bg-purple-50 border-purple-400 text-purple-900 font-bold' : ''"
                />
                <button
                  type="button"
                  @click="outletCgpSelectedOutlet && handleSaveOutletCgpPrice(outletCgpSelectedOutlet, group.id)"
                  :disabled="isSavingOutletCgp"
                  :title="'Simpan harga ' + group.name + ' untuk outlet ini'"
                  class="px-3 py-1.5 text-xs font-bold rounded-lg transition cursor-pointer flex items-center justify-center min-w-[76px] shadow-xs"
                  :class="savedSuccessCgpKey === `${outletCgpSelectedOutlet}_${group.id}`
                    ? 'bg-emerald-600 text-white'
                    : 'bg-purple-600 hover:bg-purple-700 disabled:opacity-50 text-white'"
                >
                  <span v-if="savedSuccessCgpKey === `${outletCgpSelectedOutlet}_${group.id}`">✓ Tersimpan</span>
                  <span v-else>Simpan</span>
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Footer -->
      <div class="flex items-center justify-between px-6 py-4 border-t border-slate-200 bg-slate-50 rounded-b-2xl">
        <span class="text-xs text-slate-500">
          * Tekan Enter atau klik Simpan untuk tiap baris, atau Simpan Semua untuk seluruh kelompok.
        </span>
        <div class="flex items-center gap-2">
          <button
            type="button"
            v-if="outletCgpSelectedOutlet"
            @click="handleSaveAllOutletCgp(outletCgpSelectedOutlet)"
            :disabled="isSavingOutletCgp"
            class="px-4 py-2 bg-purple-600 hover:bg-purple-700 disabled:opacity-50 text-white rounded-xl text-xs font-bold transition cursor-pointer shadow-xs flex items-center gap-1.5"
          >
            <span>Simpan Semua di Outlet Ini</span>
          </button>
          <button
            type="button"
            @click="showOutletCgpModal = false"
            class="bg-slate-800 hover:bg-slate-900 text-white px-5 py-2 rounded-xl text-xs font-bold transition cursor-pointer"
          >
            Selesai
          </button>
        </div>
      </div>
    </div>
  </div>

  <!-- Modal Batch Stok Produk (FIFO & Pelacakan Kulakan) -->
  <div
    v-if="showBatchesModal"
    class="fixed inset-0 z-50 bg-slate-900/50 backdrop-blur-xs flex items-center justify-center p-4 select-none"
  >
    <div class="bg-white border border-slate-300 w-full max-w-4xl rounded-2xl p-6 space-y-4 shadow-2xl text-slate-900">
      <div class="flex items-center justify-between border-b border-slate-200 pb-3">
        <div>
          <h3 class="font-bold text-base text-slate-900 flex items-center gap-2">
            <Layers class="w-5 h-5 text-emerald-600" />
            <span>Batch Stok & Pelacakan HPP (FEFO & FIFO)</span>
          </h3>
          <p class="text-xs text-slate-500 mt-0.5">
            Produk: <strong class="text-slate-900">{{ selectedProductForBatches?.name }}</strong> (SKU: {{ selectedProductForBatches?.sku }})
          </p>
        </div>
        <button @click="showBatchesModal = false" class="text-slate-400 hover:text-slate-600 font-bold text-lg cursor-pointer">✕</button>
      </div>

      <!-- Active Cost & Filter Banner -->
      <div class="p-3 bg-emerald-50 border border-emerald-200 rounded-xl text-xs text-emerald-900 flex flex-wrap items-center justify-between gap-3">
        <div>
          <div class="font-bold text-emerald-800">
            Metode Biaya: 
            <span class="underline font-mono">
              {{ getProductCostingDisplay(selectedProductForBatches?.costing_method).badgeText }}
            </span>
          </div>
          <div class="text-[11px] text-emerald-700 mt-0.5">
            HPP Aktif Outlet: <strong>Rp {{ Number(selectedProductForBatches?.cost_price || 0).toLocaleString() }}</strong>
          </div>
        </div>
        <div class="flex items-center gap-2">
          <label class="flex items-center gap-1.5 cursor-pointer text-emerald-950 font-semibold bg-white/80 px-2.5 py-1 rounded-lg border border-emerald-300 text-[11px]">
            <input 
              type="checkbox" 
              v-model="filterBatchAllOutlets" 
              @change="fetchProductBatches" 
              class="rounded text-emerald-600 focus:ring-emerald-500 cursor-pointer"
            />
            <span>Tampilkan Semua Outlet</span>
          </label>
          <span class="text-[11px] bg-white px-2.5 py-1 rounded-lg border border-emerald-300 font-mono font-bold text-emerald-800 shadow-2xs">
            {{ filterBatchAllOutlets ? 'Semua Outlet' : (locationStore.currentOutlet?.name || 'Outlet Aktif') }}
          </span>
        </div>
      </div>

      <!-- Batches Table -->
      <div class="max-h-96 overflow-y-auto border border-slate-200 rounded-xl">
        <table class="w-full text-left text-xs border-collapse">
          <thead class="bg-slate-50 text-slate-700 font-bold border-b border-slate-200 sticky top-0">
            <tr>
              <th class="py-2.5 px-3">No. Batch</th>
              <th class="py-2.5 px-3">Dokumen Asal</th>
              <th class="py-2.5 px-3">Kedaluwarsa (FEFO)</th>
              <th class="py-2.5 px-3">Tgl Masuk</th>
              <th class="py-2.5 px-3">Outlet</th>
              <th class="py-2.5 px-3 text-right">Sisa Stok</th>
              <th class="py-2.5 px-3 text-right">HPP Kulakan</th>
              <th class="py-2.5 px-3 text-center">Status</th>
              <th class="py-2.5 px-3 text-center">Aksi / Penjualan</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-slate-100">
            <tr v-if="isLoadingBatches">
              <td colspan="9" class="py-8 text-center text-slate-500">Memuat rincian batch...</td>
            </tr>
            <tr v-else-if="productBatches.length === 0">
              <td colspan="9" class="py-8 text-center text-slate-500">
                Belum ada batch stok tercatat untuk produk ini pada outlet yang dipilih.
              </td>
            </tr>
            <tr 
              v-else 
              v-for="b in productBatches" 
              :key="b.id"
              :class="Number(b.remaining_qty) > 0 ? 'bg-white hover:bg-slate-50' : 'bg-slate-50/70 text-slate-400 opacity-60'"
            >
              <td class="py-2.5 px-3 font-mono font-semibold text-slate-900">{{ b.batch_number }}</td>
              <td class="py-2.5 px-3 text-slate-600 font-mono text-[11px]">
                {{ b.source_document_number || b.reference_type || '-' }}
              </td>
              <td class="py-2.5 px-3">
                <span 
                  v-if="b.expiry_date" 
                  class="px-2 py-0.5 rounded text-[10px] font-mono font-bold bg-amber-50 text-amber-800 border border-amber-200"
                >
                  {{ b.expiry_date }}
                </span>
                <span v-else class="text-slate-400 text-[11px]">-</span>
              </td>
              <td class="py-2.5 px-3 text-slate-600 text-[11px]">
                {{ new Date(b.received_at).toLocaleDateString('id-ID', { day: '2-digit', month: 'short', year: 'numeric', hour: '2-digit', minute: '2-digit' }) }}
              </td>
              <td class="py-2.5 px-3 text-slate-700">{{ b.outlet_name || '-' }}</td>
              <td class="py-2.5 px-3 text-right font-mono font-bold" :class="Number(b.remaining_qty) > 0 ? 'text-slate-900' : 'text-slate-400'">
                {{ Number(b.remaining_qty).toLocaleString() }} / {{ Number(b.initial_qty).toLocaleString() }}
              </td>
              <td class="py-2.5 px-3 text-right font-mono font-bold text-emerald-700">
                Rp {{ Number(b.cost_price).toLocaleString() }}
              </td>
              <td class="py-2.5 px-3 text-center">
                <span 
                  v-if="Number(b.remaining_qty) > 0"
                  class="px-2 py-0.5 rounded-full text-[10px] font-bold bg-emerald-100 text-emerald-800"
                >
                  Tersedia
                </span>
                <span 
                  v-else 
                  class="px-2 py-0.5 rounded-full text-[10px] font-bold bg-slate-200 text-slate-600"
                >
                  Habis
                </span>
              </td>
              <td class="py-2.5 px-3 text-center">
                <button 
                  type="button"
                  @click="openBatchSalesModal(b)"
                  class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg text-[11px] font-bold bg-indigo-50 hover:bg-indigo-100 text-indigo-700 border border-indigo-200 transition-all cursor-pointer shadow-2xs hover:shadow-xs active:scale-95"
                  title="Lihat transaksi penjualan yang menggunakan stok batch ini"
                >
                  <Receipt class="w-3.5 h-3.5 text-indigo-600" />
                  <span>Riwayat POS</span>
                </button>
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <div class="flex justify-end pt-2 border-t border-slate-200">
        <button 
          @click="showBatchesModal = false"
          class="bg-slate-100 hover:bg-slate-200 border border-slate-300 text-slate-700 px-4 py-2 rounded-lg text-xs font-bold cursor-pointer"
        >
          Tutup
        </button>
      </div>
    </div>
  </div>

  <!-- Modal Riwayat Transaksi Penjualan Batch (Traceability) -->
  <Teleport to="body">
    <div
      v-if="showBatchSalesModal"
      class="fixed inset-0 z-[80] bg-slate-900/60 backdrop-blur-xs flex items-center justify-center p-4 select-none"
    >
      <div class="bg-white border border-slate-300 w-full max-w-4xl rounded-2xl p-6 space-y-4 shadow-2xl text-slate-900">
        <div class="flex items-center justify-between border-b border-slate-200 pb-3">
          <div>
            <h3 class="font-bold text-base text-slate-900 flex items-center gap-2">
              <Receipt class="w-5 h-5 text-indigo-600" />
              <span>Riwayat Penggunaan Batch pada Transaksi Penjualan</span>
            </h3>
            <p class="text-xs text-slate-500 mt-0.5">
              Produk: <strong class="text-slate-900">{{ selectedProductForBatches?.name }}</strong> 
              &bull; No. Batch: <span class="font-mono bg-indigo-50 text-indigo-800 px-1.5 py-0.5 rounded font-bold border border-indigo-200">{{ selectedBatchForSales?.batch_number }}</span>
            </p>
          </div>
          <button @click="showBatchSalesModal = false" class="text-slate-400 hover:text-slate-600 font-bold text-lg cursor-pointer">✕</button>
        </div>

        <!-- Summary KPI Batch -->
        <div class="grid grid-cols-2 md:grid-cols-4 gap-3">
          <div class="p-3 bg-slate-50 border border-slate-200 rounded-xl">
            <div class="text-[11px] text-slate-500 font-medium">Stok Awal Batch</div>
            <div class="text-sm font-bold font-mono text-slate-900 mt-0.5">
              {{ formatQty(batchSalesUsage?.initial_qty || selectedBatchForSales?.initial_qty || 0) }} unit
            </div>
          </div>
          <div class="p-3 bg-emerald-50 border border-emerald-200 rounded-xl">
            <div class="text-[11px] text-emerald-700 font-medium">Sisa Stok Tersedia</div>
            <div class="text-sm font-bold font-mono text-emerald-800 mt-0.5">
              {{ formatQty(batchSalesUsage?.remaining_qty || selectedBatchForSales?.remaining_qty || 0) }} unit
            </div>
          </div>
          <div class="p-3 bg-indigo-50 border border-indigo-200 rounded-xl">
            <div class="text-[11px] text-indigo-700 font-medium">Terpotong di Penjualan</div>
            <div class="text-sm font-bold font-mono text-indigo-800 mt-0.5">
              {{ formatQty(batchSalesUsage?.total_consumed_qty || 0) }} unit
            </div>
          </div>
          <div class="p-3 bg-purple-50 border border-purple-200 rounded-xl">
            <div class="text-[11px] text-purple-700 font-medium">Total Beban Modal (HPP)</div>
            <div class="text-sm font-bold font-mono text-purple-800 mt-0.5">
              Rp {{ Number(batchSalesUsage?.total_consumed_cost || 0).toLocaleString() }}
            </div>
          </div>
        </div>

        <!-- Sales Table -->
        <div class="max-h-96 overflow-y-auto border border-slate-200 rounded-xl">
          <table class="w-full text-left text-xs border-collapse">
            <thead class="bg-slate-50 text-slate-700 font-bold border-b border-slate-200 sticky top-0">
              <tr>
                <th class="py-2.5 px-3">No. Faktur</th>
                <th class="py-2.5 px-3">Waktu Transaksi</th>
                <th class="py-2.5 px-3">Outlet</th>
                <th class="py-2.5 px-3">Pelanggan</th>
                <th class="py-2.5 px-3">Kasir</th>
                <th class="py-2.5 px-3 text-right">Qty Terpakai</th>
                <th class="py-2.5 px-3 text-right">HPP Satuan</th>
                <th class="py-2.5 px-3 text-right">Subtotal Modal</th>
                <th class="py-2.5 px-3 text-center">Metode</th>
                <th class="py-2.5 px-3 text-center">Status</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-100">
              <tr v-if="isLoadingBatchSales">
                <td colspan="10" class="py-8 text-center text-slate-500">Memuat riwayat transaksi penjualan...</td>
              </tr>
              <tr v-else-if="!batchSalesUsage?.sales || batchSalesUsage.sales.length === 0">
                <td colspan="10" class="py-10 text-center">
                  <div class="flex flex-col items-center justify-center space-y-2 text-slate-400">
                    <Receipt class="w-8 h-8 text-slate-300 stroke-[1.5]" />
                    <p class="text-xs font-semibold text-slate-600">Belum Ada Transaksi Penjualan</p>
                    <p class="text-[11px] text-slate-400 max-w-sm">
                      Stok pada lot batch ini belum pernah terpotong pada transaksi penjualan POS kasir.
                    </p>
                  </div>
                </td>
              </tr>
              <tr 
                v-else 
                v-for="s in batchSalesUsage.sales" 
                :key="s.id"
                class="hover:bg-slate-50/80 transition-colors"
              >
                <td class="py-2.5 px-3 font-mono font-bold text-indigo-700">
                  {{ s.invoice_number }}
                </td>
                <td class="py-2.5 px-3 text-slate-600 text-[11px]">
                  {{ new Date(s.sale_created_at || s.created_at).toLocaleDateString('id-ID', { day: '2-digit', month: 'short', year: 'numeric', hour: '2-digit', minute: '2-digit' }) }}
                </td>
                <td class="py-2.5 px-3 text-slate-700">{{ s.outlet_name || '-' }}</td>
                <td class="py-2.5 px-3 text-slate-600">{{ s.customer_name || 'Umum / Non-Member' }}</td>
                <td class="py-2.5 px-3 text-slate-600">{{ s.cashier_name || 'Kasir' }}</td>
                <td class="py-2.5 px-3 text-right font-mono font-bold text-slate-900">
                  {{ Number(s.quantity).toLocaleString() }}
                </td>
                <td class="py-2.5 px-3 text-right font-mono text-slate-600">
                  Rp {{ Number(s.cost_price).toLocaleString() }}
                </td>
                <td class="py-2.5 px-3 text-right font-mono font-bold text-emerald-700">
                  Rp {{ Number(s.subtotal_cost).toLocaleString() }}
                </td>
                <td class="py-2.5 px-3 text-center">
                  <span class="px-1.5 py-0.5 rounded text-[10px] font-mono font-semibold bg-slate-100 text-slate-700 border border-slate-200">
                    {{ s.allocation_method || 'FIFO' }}
                  </span>
                </td>
                <td class="py-2.5 px-3 text-center">
                  <span 
                    v-if="s.sale_status === 'COMPLETED'"
                    class="px-2 py-0.5 rounded-full text-[10px] font-bold bg-emerald-100 text-emerald-800"
                  >
                    Selesai
                  </span>
                  <span 
                    v-else-if="s.sale_status === 'VOID'"
                    class="px-2 py-0.5 rounded-full text-[10px] font-bold bg-rose-100 text-rose-800"
                  >
                    Void (Batal)
                  </span>
                  <span 
                    v-else 
                    class="px-2 py-0.5 rounded-full text-[10px] font-bold bg-slate-100 text-slate-600"
                  >
                    {{ s.sale_status }}
                  </span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <div class="flex justify-end pt-2 border-t border-slate-200">
          <button 
            @click="showBatchSalesModal = false"
            class="bg-slate-100 hover:bg-slate-200 border border-slate-300 text-slate-700 px-4 py-2 rounded-lg text-xs font-bold cursor-pointer"
          >
            Kembali ke Rincian Batch
          </button>
        </div>
      </div>
    </div>
  </Teleport>

  </AdminLayout>

  <!-- Modal Preview Foto Ukuran Penuh (Teleport agar tidak terganggu nesting) -->
  <Teleport to="body">
    <div
      v-if="previewImageModalUrl"
      class="fixed inset-0 z-[100] bg-slate-900/80 backdrop-blur-xs flex items-center justify-center p-4 cursor-pointer"
      @click="previewImageModalUrl = null"
    >
      <div class="relative max-w-lg max-h-[85vh] bg-white p-2 rounded-2xl shadow-2xl overflow-hidden" @click.stop>
        <button
          type="button"
          @click="previewImageModalUrl = null"
          class="absolute top-4 right-4 bg-slate-900/70 hover:bg-slate-900 text-white w-8 h-8 rounded-full flex items-center justify-center font-bold text-sm shadow-md transition cursor-pointer z-10"
        >
          ✕
        </button>
        <img :src="previewImageModalUrl" alt="Foto Produk" class="max-w-full max-h-[80vh] object-contain rounded-xl" />
      </div>
    </div>
  </Teleport>

  <!-- Modal Sistem Import Produk Canggih (sudah pakai Teleport di dalam komponen) -->
  <AdvancedProductImportModal
    :is-open="showImportModal"
    :categories="categories"
    :units="units"
    :outlets="outlets"
    :customer-groups="customerGroups"
    :warehouses="warehouses"
    @close="showImportModal = false"
    @imported="fetchProducts"
  />

  <!-- Modal Kelola Multi-Barcode Produk (Teleport to body) -->
  <Teleport to="body">
    <ProductBarcodesModal
      :show="showBarcodesModal"
      :product="selectedProductForBarcodes"
      @close="showBarcodesModal = false"
      @updated="fetchProducts"
    />
  </Teleport>
</template>
