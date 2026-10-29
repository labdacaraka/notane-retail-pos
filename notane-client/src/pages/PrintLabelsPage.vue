<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, watch } from 'vue'
import AdminLayout from '@/layouts/AdminLayout.vue'
import {
  Printer,
  Search,
  Building2,
  CheckSquare,
  Square,
  Minus,
  Plus,
  RefreshCw,
  Eye,
  Sliders,
  Layers,
  ArrowUp,
  ArrowDown,
  ArrowUpDown,
} from 'lucide-vue-next'
import { Icon } from '@iconify/vue'
import { api, getApiBaseUrl } from '@/services/api'
import { generateCode128Svg } from '@/utils/barcodeSvg'
import { useDialog } from '@/composables/useDialog'
import { useLabelPrinterStore } from '@/stores/labelPrinterStore'
import LabelPrinterSettingsModal from '@/components/common/LabelPrinterSettingsModal.vue'
import PaginationBar from '@/components/common/PaginationBar.vue'
import { formatCurrency, formatQty } from '@/utils/formatters'

const dialog = useDialog()
const labelPrinterStore = useLabelPrinterStore()
const showLabelPrinterModal = ref<boolean>(false)
const isPrinting = ref<boolean>(false)

// State Modal Pratinjau Cetak Label
const showPreviewModal = ref<boolean>(false)
const previewHtmlContent = ref<string>('')
const previewIframeRef = ref<HTMLIFrameElement | null>(null)

interface ProductItem {
  id: string
  name: string
  sku: string
  barcode: string
  category_id?: string
  category_name?: string
  unit_code?: string
  cost_price: string | number
  base_selling_price: string | number
  image_url?: string | null
  stock_qty?: string | number
  barcode_count?: number
}

interface SelectedLabelItem {
  product: ProductItem
  print_qty: number
}

const outlets = ref<any[]>([])
const selectedOutletId = ref<string>('')
const products = ref<ProductItem[]>([])
const categories = ref<any[]>([])
const isLoading = ref<boolean>(false)
const searchQuery = ref<string>('')
const selectedCategoryId = ref<string>('')

// Server-Side Pagination & Sorting State
const currentPage = ref<number>(1)
const pageSize = ref<number>(25)
const totalProducts = ref<number>(0)
const totalPages = ref<number>(1)
const sortBy = ref<string>('name')
const sortDir = ref<'asc' | 'desc'>('asc')

// Antrean label yang dipilih untuk dicetak (key: product.id)
const selectedItemsMap = ref<Map<string, SelectedLabelItem>>(new Map())

// Konfigurasi Desain & Layout Label
const labelTemplate = ref<'shelf_standard' | 'product_sticker' | 'compact_jewelry'>('shelf_standard')
const paperFormat = ref<'sheet_a4' | 'continuous_roll'>('sheet_a4')
const a4GridColumns = ref<number>(2) // Default 2 kolom sesuai preview layar
const previewMode = ref<'single' | 'all'>('all')

// Toggle Tampilan Elemen Label
const showOutletName = ref<boolean>(labelPrinterStore.showStoreTitle)
const showBarcodeLines = ref<boolean>(labelPrinterStore.showBarcode)
const showUnitName = ref<boolean>(true)
const showPrintDate = ref<boolean>(labelPrinterStore.showDate)
const showSku = ref<boolean>(labelPrinterStore.showSku)
const customStoreTitle = ref<string>(labelPrinterStore.customStoreName || '')

// Sinkronkan perubahan dari store ke state lokal
watch(() => labelPrinterStore.showStoreTitle, (val) => { showOutletName.value = val })
watch(() => labelPrinterStore.showBarcode, (val) => { showBarcodeLines.value = val })
watch(() => labelPrinterStore.showSku, (val) => { showSku.value = val })
watch(() => labelPrinterStore.showDate, (val) => { showPrintDate.value = val })
watch(() => labelPrinterStore.customStoreName, (val) => { if (val) customStoreTitle.value = val })

// Sinkronkan preset ukuran label dengan template
watch(
  () => labelPrinterStore.sizePreset,
  (newPreset) => {
    if (newPreset === '40x25') {
      labelTemplate.value = 'product_sticker'
    } else if (newPreset === '30x20') {
      labelTemplate.value = 'compact_jewelry'
    } else if (newPreset === '65x35') {
      labelTemplate.value = 'shelf_standard'
    }
  },
  { immediate: true }
)

watch(
  () => labelPrinterStore.a4Columns,
  (cols) => {
    if (cols) {
      a4GridColumns.value = cols
    }
  },
  { immediate: true }
)

watch(labelTemplate, (tpl) => {
  if (tpl === 'product_sticker' && labelPrinterStore.sizePreset !== '40x25') {
    labelPrinterStore.sizePreset = '40x25'
    labelPrinterStore.saveConfig()
  } else if (tpl === 'compact_jewelry' && labelPrinterStore.sizePreset !== '30x20') {
    labelPrinterStore.sizePreset = '30x20'
    labelPrinterStore.saveConfig()
  } else if (tpl === 'shelf_standard' && labelPrinterStore.sizePreset !== '65x35') {
    labelPrinterStore.sizePreset = '65x35'
    labelPrinterStore.saveConfig()
  }
})

watch(a4GridColumns, (cols) => {
  if (labelPrinterStore.a4Columns !== cols) {
    labelPrinterStore.a4Columns = cols
    labelPrinterStore.saveConfig()
  }
})

const printDateStr = computed(() => {
  const d = new Date()
  return d.toLocaleDateString('id-ID', {
    day: '2-digit',
    month: '2-digit',
    year: 'numeric',
  })
})

function resolveImageUrl(url?: string | null): string | null {
  if (!url) return null
  if (url.startsWith('http://') || url.startsWith('https://') || url.startsWith('data:')) return url
  const base = getApiBaseUrl().replace('/api/v1', '')
  return `${base}${url}`
}

const activeOutlet = computed(() => {
  return outlets.value.find((o) => o.id === selectedOutletId.value)
})

const activeStoreDisplayTitle = computed(() => {
  if (customStoreTitle.value.trim()) return customStoreTitle.value.trim()
  if (activeOutlet.value?.name) return activeOutlet.value.name.toUpperCase()
  return 'NOTANE RETAIL'
})

// Produk Terfilter di Halaman Aktif (berasal dari Server-Side)
const filteredProducts = computed(() => products.value)

// Total Label Akan Dicetak
const totalSelectedProductsCount = computed(() => selectedItemsMap.value.size)
const totalLabelsToPrint = computed(() => {
  let total = 0
  for (const it of selectedItemsMap.value.values()) {
    total += it.print_qty
  }
  return total
})

// Daftar datar label yang akan dicetak (mengulang sesuai print_qty)
const flatLabelsList = computed(() => {
  const arr: ProductItem[] = []
  for (const item of selectedItemsMap.value.values()) {
    for (let i = 0; i < item.print_qty; i++) {
      arr.push(item.product)
    }
  }
  return arr
})

async function fetchOutletsAndCategories() {
  try {
    const [outRes, catRes] = await Promise.all([
      api.get('/outlets'),
      api.get('/categories'),
    ])
    if (outRes.data.success) {
      outlets.value = outRes.data.data
      const savedOid =
        localStorage.getItem('notane_active_outlet_id') ||
        localStorage.getItem('notane_pos_outlet_id')
      if (savedOid && outlets.value.some((o) => o.id === savedOid)) {
        selectedOutletId.value = savedOid
      } else if (outlets.value.length > 0) {
        selectedOutletId.value = outlets.value[0].id
      }
    }
    if (catRes.data.success) {
      categories.value = catRes.data.data
    }
  } catch (err) {
    console.error('Gagal memuat outlet/kategori:', err)
  }
}

async function fetchProducts() {
  isLoading.value = true
  try {
    const params: any = {
      page: currentPage.value,
      page_size: pageSize.value,
      sort_by: sortBy.value,
      sort_dir: sortDir.value,
    }
    if (selectedOutletId.value) {
      params.outlet_id = selectedOutletId.value
    }
    if (selectedCategoryId.value) {
      params.category_id = selectedCategoryId.value
    }
    if (searchQuery.value.trim()) {
      params.search = searchQuery.value.trim()
    }

    // Mengambil produk dengan harga spesifik outlet, sorting, & paging secara server-side
    const res = await api.get('/products', { params })
    if (res.data.success) {
      products.value = res.data.data
      totalProducts.value = res.data.total_items ?? res.data.data.length
      currentPage.value = res.data.page ?? currentPage.value
      pageSize.value = res.data.page_size ?? pageSize.value
      totalPages.value = res.data.total_pages ?? 1

      // Perbarui harga produk pada antrean terpilih jika outlet berganti / data reload
      for (const [pid, item] of selectedItemsMap.value.entries()) {
        const fresh = products.value.find((p) => p.id === pid)
        if (fresh) {
          selectedItemsMap.value.set(pid, {
            product: fresh,
            print_qty: item.print_qty,
          })
        }
      }
    }
  } catch (err) {
    console.error('Gagal memuat produk:', err)
  } finally {
    isLoading.value = false
  }
}

// Server-Side Sorting Action
function handleSort(col: string) {
  if (sortBy.value === col) {
    sortDir.value = sortDir.value === 'asc' ? 'desc' : 'asc'
  } else {
    sortBy.value = col
    sortDir.value = 'asc'
  }
  currentPage.value = 1
  fetchProducts()
}

// Server-Side Filter Actions
let searchDebounceTimer: any = null
function handleSearchInput() {
  clearTimeout(searchDebounceTimer)
  searchDebounceTimer = setTimeout(() => {
    currentPage.value = 1
    fetchProducts()
  }, 300)
}

function clearSearch() {
  searchQuery.value = ''
  currentPage.value = 1
  fetchProducts()
}

function handleCategoryChange() {
  currentPage.value = 1
  fetchProducts()
}

function handlePageSizeChange() {
  currentPage.value = 1
  fetchProducts()
}

function handleLocationChanged(e: any) {
  if (e?.detail?.id && selectedOutletId.value !== e.detail.id) {
    selectedOutletId.value = e.detail.id
  }
}

watch(selectedOutletId, () => {
  currentPage.value = 1
  fetchProducts()
})

onMounted(async () => {
  await fetchOutletsAndCategories()
  await fetchProducts()
  window.addEventListener('notane-location-changed', handleLocationChanged)
})

onUnmounted(() => {
  window.removeEventListener('notane-location-changed', handleLocationChanged)
})

// Checkbox Selection
function isProductSelected(prodId: string): boolean {
  return selectedItemsMap.value.has(prodId)
}

function toggleProductSelection(prod: ProductItem) {
  if (selectedItemsMap.value.has(prod.id)) {
    selectedItemsMap.value.delete(prod.id)
  } else {
    const initialQty =
      labelPrinterStore.defaultQtyMode === 'STOCK'
        ? Math.max(1, Math.round(Number(prod.stock_qty || 1)))
        : 1
    selectedItemsMap.value.set(prod.id, {
      product: prod,
      print_qty: initialQty,
    })
  }
}

const isAllFilteredSelected = computed(() => {
  if (products.value.length === 0) return false
  return products.value.every((p) => selectedItemsMap.value.has(p.id))
})

function toggleSelectAllFiltered() {
  if (isAllFilteredSelected.value) {
    // Unselect all in current page
    products.value.forEach((p) => selectedItemsMap.value.delete(p.id))
  } else {
    // Select all in current page
    products.value.forEach((p) => {
      if (!selectedItemsMap.value.has(p.id)) {
        const initialQty =
          labelPrinterStore.defaultQtyMode === 'STOCK'
            ? Math.max(1, Math.round(Number(p.stock_qty || 1)))
            : 1
        selectedItemsMap.value.set(p.id, {
          product: p,
          print_qty: initialQty,
        })
      }
    })
  }
}

function setQty(prodId: string, qty: number) {
  const item = selectedItemsMap.value.get(prodId)
  if (item) {
    item.print_qty = Math.max(1, qty)
  }
}

function adjustQty(prodId: string, delta: number) {
  const item = selectedItemsMap.value.get(prodId)
  if (item) {
    item.print_qty = Math.max(1, item.print_qty + delta)
  }
}

function clearAllSelections() {
  selectedItemsMap.value.clear()
}

function setAllQtyToOne() {
  for (const item of selectedItemsMap.value.values()) {
    item.print_qty = 1
  }
}

function setAllQtyToTwo() {
  for (const item of selectedItemsMap.value.values()) {
    item.print_qty = 2
  }
}

function setAllQtyByStock() {
  for (const item of selectedItemsMap.value.values()) {
    const s = Math.floor(Number(item.product.stock_qty) || 0)
    item.print_qty = s > 0 ? s : 1
  }
}

// Helper pembuatan dokumen HTML cetak yang kompatibel dengan semua printer (A4 Sheet, Roll, & PDF)
function buildPrintDocumentHtml(): string {
  const printableGrid = document.getElementById('notane-printable-grid')
  let contentHtml = ''

  if (printableGrid && printableGrid.innerHTML.trim().length > 0) {
    contentHtml = printableGrid.innerHTML
  } else {
    // Fallback darurat jika DOM printableGrid belum terisi
    contentHtml = flatLabelsList.value
      .map((prod) => {
        const storeHtml = showOutletName.value
          ? `<div class="tag-store-header">${activeStoreDisplayTitle.value}</div>`
          : ''
        const skuHtml = showSku.value
          ? `<div class="tag-meta-info"><span>${prod.sku || ''}</span>${prod.category_name ? `<span>${prod.category_name}</span>` : ''}</div>`
          : ''
        const barcodeHtml =
          showBarcodeLines.value && prod.barcode
            ? `<div class="tag-barcode-container">
                <div class="tag-barcode-svg">${generateCode128Svg(prod.barcode, 30)}</div>
                <div class="tag-barcode-text">${prod.barcode}</div>
               </div>`
            : ''
        const dateHtml = showPrintDate.value
          ? `<div class="tag-footer-date">${printDateStr.value}</div>`
          : ''

        return `
          <div class="price-tag-card">
            ${storeHtml}
            <div class="tag-product-name">${prod.name}</div>
            ${skuHtml}
            ${barcodeHtml}
            <div class="tag-price-row">
              <div class="tag-price-amount">
                <span class="currency-symbol">Rp</span>
                <span class="price-val">${formatCurrency(prod.base_selling_price)}</span>
              </div>
              ${showUnitName.value ? `<span class="unit-badge">/${prod.unit_code || 'PCS'}</span>` : ''}
            </div>
            ${dateHtml}
          </div>
        `
      })
      .join('\n')
  }

  const gridClass = [
    paperFormat.value === 'continuous_roll' ? 'mode-roll' : 'mode-a4',
    labelTemplate.value === 'shelf_standard'
      ? 'template-shelf'
      : labelTemplate.value === 'product_sticker'
      ? 'template-sticker'
      : 'template-compact',
  ].join(' ')

  return `<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8" />
  <title>Cetak Label Harga - ${activeStoreDisplayTitle.value}</title>
  <style>
    * {
      box-sizing: border-box;
      margin: 0;
      padding: 0;
    }
    @page {
      margin: 3mm;
      size: auto;
    }
    body {
      font-family: system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
      background: #ffffff;
      color: #000000;
      padding: 2mm;
      margin: 0;
      -webkit-print-color-adjust: exact;
      print-color-adjust: exact;
    }

    /* Grid Layout untuk Kertas A4 (Disesuaikan dengan pilihan kolom) */
    .print-grid.mode-a4 {
      display: grid !important;
      grid-template-columns: repeat(${paperFormat.value === 'sheet_a4' ? a4GridColumns.value : 1}, 1fr) !important;
      gap: 3mm !important;
      width: 100% !important;
    }

    /* Layout untuk Printer Thermal Roll Continuous */
    .print-grid.mode-roll {
      display: flex !important;
      flex-direction: column !important;
      align-items: center !important;
      gap: 3mm !important;
      width: 100% !important;
    }

    /* Card Dasar Label Harga */
    .price-tag-card {
      page-break-inside: avoid !important;
      break-inside: avoid !important;
      box-sizing: border-box !important;
      border: 1px solid #000000 !important;
      background: #ffffff !important;
      color: #000000 !important;
      display: flex !important;
      flex-direction: column !important;
      justify-content: space-between !important;
    }

    .mode-a4 .price-tag-card {
      width: 100% !important;
      max-width: 100% !important;
    }
    .mode-a4.template-shelf .price-tag-card {
      min-height: 35mm !important;
      height: 35mm !important;
      padding: 2mm !important;
    }
    .mode-a4.template-sticker .price-tag-card {
      min-height: 25mm !important;
      height: 25mm !important;
      padding: 1.5mm !important;
    }
    .mode-a4.template-compact .price-tag-card {
      min-height: 20mm !important;
      height: 20mm !important;
      padding: 1mm !important;
    }

    .mode-roll.template-shelf .price-tag-card {
      width: 65mm !important;
      height: 35mm !important;
      padding: 2mm !important;
    }
    .mode-roll.template-sticker .price-tag-card {
      width: 40mm !important;
      height: 25mm !important;
      padding: 1.5mm !important;
    }
    .mode-roll.template-compact .price-tag-card {
      width: 30mm !important;
      height: 20mm !important;
      padding: 1mm !important;
    }

    .tag-store-header {
      font-size: 7pt;
      font-weight: 900;
      text-transform: uppercase;
      text-align: center;
      border-bottom: 1px solid #000000;
      padding-bottom: 0.5mm;
      line-height: 1;
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }

    .tag-product-name {
      font-size: 8.5pt;
      font-weight: 900;
      line-height: 1.1;
      margin-top: 1mm;
      display: -webkit-box;
      -webkit-line-clamp: 2;
      -webkit-box-orient: vertical;
      overflow: hidden;
      text-overflow: ellipsis;
    }

    .template-sticker .tag-product-name {
      font-size: 7pt;
    }

    .template-compact .tag-product-name {
      font-size: 6pt;
    }

    .tag-meta-info {
      font-size: 6pt;
      font-family: monospace;
      color: #333333;
      display: flex;
      justify-content: space-between;
      line-height: 1;
      margin-top: 0.5mm;
    }

    .tag-barcode-container {
      margin: 1mm 0;
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
    }

    .tag-barcode-svg {
      width: 90%;
      height: 7mm;
      display: flex;
      align-items: center;
      justify-content: center;
    }

    .template-sticker .tag-barcode-svg {
      height: 5mm;
    }

    .template-compact .tag-barcode-svg {
      height: 4mm;
    }

    .tag-barcode-text {
      font-family: monospace;
      font-size: 6.5pt;
      font-weight: bold;
      letter-spacing: 1px;
      line-height: 1;
      margin-top: 0.3mm;
    }

    .tag-price-row {
      display: flex;
      align-items: baseline;
      justify-content: space-between;
      border-top: 1px dashed #000000;
      padding-top: 1mm;
      margin-top: auto;
    }

    .tag-price-amount {
      display: flex;
      align-items: baseline;
    }

    .currency-symbol {
      font-size: 7pt;
      font-weight: bold;
      margin-right: 0.5mm;
    }

    .price-val {
      font-family: monospace;
      font-size: 13pt;
      font-weight: 900;
      line-height: 1;
    }

    .template-sticker .price-val {
      font-size: 9.5pt;
    }

    .template-compact .price-val {
      font-size: 8pt;
    }

    .unit-badge {
      font-size: 6.5pt;
      font-weight: bold;
      text-transform: uppercase;
    }

    .tag-footer-date {
      font-size: 5.5pt;
      color: #444444;
      text-align: right;
      line-height: 1;
      margin-top: 0.5mm;
    }
  </style>
</head>
<body>
  <div class="print-grid ${gridClass}">
    ${contentHtml}
  </div>
</body>
</html>`
}

// Cetak dokumen HTML visual via iframe tersembunyi (Membuka dialog cetak OS asli / ekspor Microsoft Print to PDF)
function printHtmlViaIframe(htmlContent: string) {
  let iframe = document.getElementById('notane-silent-print-iframe') as HTMLIFrameElement
  if (!iframe) {
    iframe = document.createElement('iframe')
    iframe.id = 'notane-silent-print-iframe'
    iframe.style.position = 'fixed'
    iframe.style.right = '0'
    iframe.style.bottom = '0'
    iframe.style.width = '0'
    iframe.style.height = '0'
    iframe.style.border = '0'
    iframe.style.opacity = '0'
    iframe.style.pointerEvents = 'none'
    document.body.appendChild(iframe)
  }

  const doc = iframe.contentWindow?.document
  if (doc) {
    doc.open()
    doc.write(htmlContent)
    doc.close()
    setTimeout(() => {
      try {
        iframe.contentWindow?.focus()
        iframe.contentWindow?.print()
      } catch (e) {
        console.warn('Iframe print failed, falling back to window.print():', e)
        window.print()
      }
    }, 250)
  }
}

// Buka Modal Pratinjau Dokumen Label
function openPrintPreview() {
  if (flatLabelsList.value.length === 0) {
    dialog.alert('Silakan pilih/centang minimal 1 produk dari daftar tabel di sebelah kiri sebelum membuka pratinjau.', 'warning')
    return
  }
  previewHtmlContent.value = buildPrintDocumentHtml()
  showPreviewModal.value = true
}

// Cetak langsung dari dalam Modal Pratinjau
function triggerPrintFromPreview() {
  if (!previewHtmlContent.value) {
    previewHtmlContent.value = buildPrintDocumentHtml()
  }
  if (previewIframeRef.value?.contentWindow) {
    try {
      previewIframeRef.value.contentWindow.focus()
      previewIframeRef.value.contentWindow.print()
      return
    } catch (e) {
      console.warn('previewIframeRef.print failed, using iframe fallback:', e)
    }
  }
  printHtmlViaIframe(previewHtmlContent.value)
}

// Buka dokumen print di tab / jendela peramban terpisah (Blob URL)
function openInExternalWindow() {
  const html = buildPrintDocumentHtml()
  const blob = new Blob([html], { type: 'text/html;charset=utf-8' })
  const url = URL.createObjectURL(blob)
  window.open(url, '_blank')
}

// Fungsi Cetak Label Otomatis
async function handlePrintLabels() {
  if (flatLabelsList.value.length === 0) {
    dialog.alert('Silakan pilih/centang minimal 1 produk dari daftar tabel di sebelah kiri sebelum mencetak label.', 'warning')
    return
  }

  // Jika printer hardware SERIAL/BLUETOOTH belum tersambung
  if (!labelPrinterStore.isConnected && labelPrinterStore.connectionType !== 'SYSTEM' && labelPrinterStore.connectionType !== 'VIRTUAL') {
    showLabelPrinterModal.value = true
    dialog.alert(`Printer label (${labelPrinterStore.connectionType}) belum tersambung. Silakan sambungkan printer pada modal pengaturan.`, 'warning')
    return
  }

  // Jika printer adalah Microsoft Print to PDF / Driver Dokumen ATAU format kertas adalah Lembaran A4 Grid:
  // Gunakan render dokumen HTML visual (membuka dialog cetak OS untuk menyimpan PDF atau mencetak lembaran)
  if (
    labelPrinterStore.isPdfOrDocumentPrinter ||
    (labelPrinterStore.sizePreset === 'a4_grid' && paperFormat.value === 'sheet_a4')
  ) {
    const htmlDoc = buildPrintDocumentHtml()
    printHtmlViaIframe(htmlDoc)
    if (labelPrinterStore.autoClearAfterPrint) {
      selectedItemsMap.value.clear()
    }
    return
  }

  // Siapkan daftar item label untuk dicetak langsung via TSPL ke printer thermal
  const itemsToPrint = Array.from(selectedItemsMap.value.values()).map((it) => ({
    name: it.product.name,
    barcode: it.product.barcode || it.product.sku,
    price: Number(it.product.base_selling_price || 0),
    sku: it.product.sku,
    storeTitle: activeStoreDisplayTitle.value,
    qty: it.print_qty,
  }))

  isPrinting.value = true
  try {
    const res = await labelPrinterStore.printTsplLabels(itemsToPrint)
    if (res.success) {
      dialog.alert(res.message || `Berhasil mencetak ${res.count} label secara otomatis!`, 'success')
      if (labelPrinterStore.autoClearAfterPrint) {
        selectedItemsMap.value.clear()
      }
    } else {
      dialog.alert(res.message, 'warning')
    }
  } catch (err: any) {
    console.error('Gagal mencetak label:', err)
    dialog.alert(`Gagal mencetak label: ${err.message || 'Terjadi gangguan komunikasi dengan printer'}`, 'danger')
  } finally {
    isPrinting.value = false
  }
}
</script>

<template>
  <AdminLayout title="Cetak Label Harga Masal">
    <!-- Header Page Toolbar (Non-Printable) -->
    <div class="no-print space-y-4 mb-5">
      <!-- Title & Main Control Bar -->
      <div class="bg-white border border-slate-300 rounded-2xl p-5 shadow-xs flex flex-col md:flex-row md:items-center justify-between gap-4">
        <div class="flex items-center gap-3.5">
          <div class="w-12 h-12 rounded-xl bg-gradient-to-br from-emerald-500 to-teal-700 text-white flex items-center justify-center shadow-sm shrink-0">
            <Icon icon="streamline-ultimate-color:tags-1" class="w-7 h-7" />
          </div>
          <div>
            <div class="flex items-center gap-2">
              <h2 class="text-lg font-black text-slate-900 tracking-tight">Cetak Label Harga Produk</h2>
              <span class="px-2.5 py-0.5 rounded-full text-[10px] font-black uppercase tracking-wider bg-emerald-100 text-emerald-800 border border-emerald-300">
                Multi-Outlet Price Sync
              </span>
            </div>
            <p class="text-xs text-slate-500 mt-0.5">
              Cetak label harga rak display dan sticker barcode kemasan secara masal dengan sinkronisasi harga terkini outlet.
            </p>
          </div>
        </div>

        <!-- Tombol Aksi Print & Pengaturan Printer -->
        <div class="flex flex-wrap items-center gap-2.5 self-end md:self-auto">
          <!-- Button Pengaturan Printer Label -->
          <button
            type="button"
            @click="showLabelPrinterModal = true"
            class="px-3.5 py-2.5 rounded-xl border flex items-center gap-2 font-bold text-xs shadow-xs transition active:scale-95 cursor-pointer"
            :class="{
              'bg-emerald-50 text-emerald-800 border-emerald-300 hover:bg-emerald-100': labelPrinterStore.isConnected,
              'bg-amber-50 text-amber-800 border-amber-300 hover:bg-amber-100': !labelPrinterStore.isConnected
            }"
            title="Klik untuk memilih printer dan konfigurasi hardware (Driver Sistem / USB Serial / Bluetooth)"
          >
            <Icon icon="streamline-ultimate-color:print-text" class="w-4 h-4 shrink-0" />
            <div class="text-left leading-tight hidden sm:block">
              <div class="text-[9px] text-slate-500 font-normal uppercase tracking-wider">Printer Label:</div>
              <div class="truncate max-w-[140px] text-slate-900 font-extrabold">
                {{ labelPrinterStore.connectionType === 'VIRTUAL' ? 'Sim: ' : '' }}{{ labelPrinterStore.printerName }}
              </div>
            </div>
            <span class="text-[10px] px-1.5 py-0.5 rounded font-mono font-bold bg-white/90 border border-slate-200">
              {{ labelPrinterStore.effectiveWidthMm }}×{{ labelPrinterStore.effectiveHeightMm }}mm
            </span>
            <Sliders class="w-3.5 h-3.5 text-slate-500 shrink-0 ml-0.5" />
          </button>

          <!-- Tombol Cetak Otomatis (Silent Print Langsung ke Printer Terpilih) -->
          <button
            type="button"
            @click="handlePrintLabels"
            :disabled="isPrinting || flatLabelsList.length === 0"
            class="px-5 py-2.5 rounded-xl bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-xs flex items-center gap-2 shadow-md transition active:scale-95 cursor-pointer disabled:opacity-50"
            title="Cetak otomatis langsung ke printer yang telah dipilih pada pengaturan tanpa dialog"
          >
            <Printer class="w-4 h-4" :class="{ 'animate-pulse': isPrinting }" />
            <span>{{ isPrinting ? 'Mencetak Label...' : `Cetak ${totalLabelsToPrint} Label (Otomatis)` }}</span>
          </button>

          <!-- Tombol Pratinjau Dokumen Cetak Label -->
          <button
            type="button"
            @click="openPrintPreview"
            class="px-3.5 py-2.5 rounded-xl bg-white hover:bg-slate-50 border border-slate-300 text-slate-700 hover:text-slate-900 font-bold text-xs flex items-center gap-1.5 shadow-xs transition active:scale-95 cursor-pointer"
            :class="{ 'opacity-70 hover:opacity-100': flatLabelsList.length === 0 }"
            title="Buka pratinjau lembaran dokumen label sebelum dicetak"
          >
            <Eye class="w-4 h-4 text-emerald-600" />
            <span class="hidden sm:inline">Pratinjau</span>
          </button>
        </div>
      </div>

      <!-- Pemilihan Outlet & Info Sinkronisasi Harga -->
      <div class="bg-gradient-to-r from-blue-50 to-indigo-50 border border-blue-200 rounded-2xl p-4 shadow-xs flex flex-col sm:flex-row sm:items-center justify-between gap-3 text-xs">
        <div class="flex items-center gap-3">
          <div class="w-9 h-9 rounded-xl bg-blue-600 text-white flex items-center justify-center shrink-0 shadow-2xs">
            <Building2 class="w-5 h-5" />
          </div>
          <div>
            <div class="font-bold text-blue-950 flex items-center gap-1.5">
              <span>Pilih Target Outlet / Cabang:</span>
            </div>
            <p class="text-[11px] text-blue-700 mt-0.5">
              Harga jual produk yang tercetak pada label otomatis mencocokkan harga aktif pada outlet ini.
            </p>
          </div>
        </div>

        <div class="flex items-center gap-2 shrink-0">
          <select
            v-model="selectedOutletId"
            class="px-3 py-2 rounded-xl border border-blue-300 bg-white font-bold text-xs text-slate-800 focus:outline-none focus:ring-2 focus:ring-blue-500 shadow-2xs"
          >
            <option v-for="out in outlets" :key="out.id" :value="out.id">
              {{ out.name }} ({{ out.code }})
            </option>
          </select>
          <button
            type="button"
            @click="fetchProducts"
            :disabled="isLoading"
            title="Muat Ulang Harga Terkini"
            class="p-2 rounded-xl bg-white border border-blue-300 hover:bg-blue-100 text-blue-700 transition cursor-pointer"
          >
            <RefreshCw class="w-4 h-4" :class="{ 'animate-spin': isLoading }" />
          </button>
        </div>
      </div>
    </div>

    <!-- Main Workspace (Grid 2 Kolom) (Non-Printable) -->
    <div class="no-print grid grid-cols-1 lg:grid-cols-12 gap-5 items-start">
      <!-- KOLOM KIRI: Katalog Produk & Pemilihan Masal (7 Kolom) -->
      <div class="lg:col-span-7 bg-white border border-slate-300 rounded-2xl shadow-xs overflow-hidden flex flex-col">
        <!-- Toolbar Filter Produk -->
        <div class="p-4 border-b border-slate-200 space-y-3 bg-slate-50/60">
          <div class="flex flex-col sm:flex-row items-center justify-between gap-2.5">
            <div class="font-bold text-xs text-slate-800 uppercase tracking-wide flex items-center gap-1.5">
              <Layers class="w-4 h-4 text-emerald-600" />
              <span>Daftar Produk Toko ({{ totalProducts }})</span>
            </div>

            <!-- Quick Bulk Actions -->
            <div class="flex items-center gap-1.5 self-end sm:self-auto text-[11px]">
              <button
                type="button"
                @click="setAllQtyToOne"
                :disabled="selectedItemsMap.size === 0"
                class="px-2 py-1 rounded bg-white hover:bg-slate-100 border border-slate-300 text-slate-700 font-bold transition disabled:opacity-40 cursor-pointer shadow-2xs"
              >
                Semua Qty=1
              </button>
              <button
                type="button"
                @click="setAllQtyToTwo"
                :disabled="selectedItemsMap.size === 0"
                class="px-2 py-1 rounded bg-white hover:bg-slate-100 border border-slate-300 text-slate-700 font-bold transition disabled:opacity-40 cursor-pointer shadow-2xs"
              >
                Semua Qty=2
              </button>
              <button
                type="button"
                @click="setAllQtyByStock"
                :disabled="selectedItemsMap.size === 0"
                class="px-2 py-1 rounded bg-white hover:bg-slate-100 border border-slate-300 text-slate-700 font-bold transition disabled:opacity-40 cursor-pointer shadow-2xs"
                title="Atur jumlah cetak sesuai stok fisik produk saat ini"
              >
                Sesuai Stok
              </button>
              <button
                type="button"
                @click="clearAllSelections"
                :disabled="selectedItemsMap.size === 0"
                class="px-2 py-1 rounded bg-rose-50 hover:bg-rose-100 border border-rose-200 text-rose-700 font-bold transition disabled:opacity-40 cursor-pointer shadow-2xs"
              >
                Reset
              </button>
            </div>
          </div>

          <!-- Input Search & Filter Kategori -->
          <div class="grid grid-cols-1 sm:grid-cols-12 gap-2 text-xs">
            <div class="sm:col-span-7 relative">
              <input
                v-model="searchQuery"
                @input="handleSearchInput"
                placeholder="Cari nama produk, SKU, barcode..."
                class="w-full pl-8 pr-8 py-1.5 rounded-xl border border-slate-300 bg-white font-sans text-xs focus:outline-none focus:ring-2 focus:ring-emerald-500 shadow-2xs"
              />
              <Search class="w-4 h-4 absolute left-2.5 top-2 text-slate-400" />
              <button
                v-if="searchQuery"
                @click="clearSearch"
                type="button"
                class="absolute right-2.5 top-2 text-slate-400 hover:text-slate-600 text-xs cursor-pointer"
              >
                ✕
              </button>
            </div>

            <div class="sm:col-span-4">
              <select
                v-model="selectedCategoryId"
                @change="handleCategoryChange"
                class="w-full px-2.5 py-1.5 rounded-xl border border-slate-300 bg-white font-sans text-xs focus:outline-none focus:ring-2 focus:ring-emerald-500 shadow-2xs"
              >
                <option value="">Semua Kategori</option>
                <option v-for="cat in categories" :key="cat.id" :value="cat.id">
                  {{ cat.name }}
                </option>
              </select>
            </div>

            <div class="sm:col-span-1 flex items-center justify-end">
              <button
                type="button"
                @click="fetchProducts"
                :disabled="isLoading"
                class="w-full h-full py-1.5 px-2 bg-slate-100 hover:bg-slate-200 border border-slate-300 text-slate-700 rounded-xl flex items-center justify-center transition cursor-pointer disabled:opacity-50"
                title="Segarkan Data Produk"
              >
                <RefreshCw class="w-3.5 h-3.5" :class="{ 'animate-spin': isLoading }" />
              </button>
            </div>
          </div>
        </div>

        <!-- Tabel Produk -->
        <div class="overflow-x-auto max-h-[580px] overflow-y-auto">
          <table class="w-full text-left text-xs">
            <thead class="bg-slate-100 text-[10px] font-bold text-slate-500 uppercase border-b border-slate-200 sticky top-0 z-10 select-none">
              <tr>
                <th class="px-3 py-2.5 w-10 text-center">
                  <button
                    type="button"
                    @click="toggleSelectAllFiltered"
                    class="p-0.5 rounded text-emerald-600 hover:bg-emerald-50 transition cursor-pointer"
                    title="Pilih / Batalkan Semua Produk di Halaman Ini"
                  >
                    <CheckSquare v-if="isAllFilteredSelected" class="w-4 h-4" />
                    <Square v-else class="w-4 h-4 text-slate-400" />
                  </button>
                </th>
                <th
                  class="px-3 py-2.5 cursor-pointer hover:bg-slate-200/70 transition group"
                  @click="handleSort('name')"
                  title="Klik untuk mengurutkan berdasarkan Nama Produk"
                >
                  <div class="flex items-center gap-1.5">
                    <span>Produk</span>
                    <ArrowUp v-if="sortBy === 'name' && sortDir === 'asc'" class="w-3 h-3 text-emerald-600" />
                    <ArrowDown v-else-if="sortBy === 'name' && sortDir === 'desc'" class="w-3 h-3 text-emerald-600" />
                    <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-40 group-hover:opacity-100" />
                  </div>
                </th>
                <th
                  class="px-3 py-2.5 text-right w-28 cursor-pointer hover:bg-slate-200/70 transition group"
                  @click="handleSort('base_selling_price')"
                  title="Klik untuk mengurutkan berdasarkan Harga"
                >
                  <div class="flex items-center justify-end gap-1.5">
                    <span>Harga Terkini</span>
                    <ArrowUp v-if="sortBy === 'base_selling_price' && sortDir === 'asc'" class="w-3 h-3 text-emerald-600" />
                    <ArrowDown v-else-if="sortBy === 'base_selling_price' && sortDir === 'desc'" class="w-3 h-3 text-emerald-600" />
                    <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-40 group-hover:opacity-100" />
                  </div>
                </th>
                <th
                  class="px-3 py-2.5 text-center w-20 cursor-pointer hover:bg-slate-200/70 transition group"
                  @click="handleSort('stock_qty')"
                  title="Klik untuk mengurutkan berdasarkan Stok"
                >
                  <div class="flex items-center justify-center gap-1.5">
                    <span>Stok</span>
                    <ArrowUp v-if="sortBy === 'stock_qty' && sortDir === 'asc'" class="w-3 h-3 text-emerald-600" />
                    <ArrowDown v-else-if="sortBy === 'stock_qty' && sortDir === 'desc'" class="w-3 h-3 text-emerald-600" />
                    <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-40 group-hover:opacity-100" />
                  </div>
                </th>
                <th class="px-3 py-2.5 text-center w-28">Jml Cetak</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-100">
              <tr
                v-for="prod in filteredProducts"
                :key="prod.id"
                class="hover:bg-slate-50/80 transition cursor-pointer"
                :class="{ 'bg-emerald-50/40': isProductSelected(prod.id) }"
                @click="toggleProductSelection(prod)"
              >
                <!-- Checkbox -->
                <td class="px-3 py-2.5 text-center" @click.stop="toggleProductSelection(prod)">
                  <button type="button" class="text-emerald-600 cursor-pointer">
                    <CheckSquare v-if="isProductSelected(prod.id)" class="w-4 h-4 fill-emerald-100" />
                    <Square v-else class="w-4 h-4 text-slate-300" />
                  </button>
                </td>

                <!-- Produk Info -->
                <td class="px-3 py-2.5">
                  <div class="flex items-center gap-2.5">
                    <div class="w-8 h-8 rounded-lg border border-slate-200 bg-slate-50 flex items-center justify-center shrink-0 overflow-hidden shadow-2xs">
                      <img
                        v-if="prod.image_url"
                        :src="resolveImageUrl(prod.image_url)!"
                        :alt="prod.name"
                        class="w-full h-full object-cover"
                      />
                      <Icon v-else icon="fluent-color:image-24" class="w-4 h-4 text-slate-300" />
                    </div>
                    <div class="min-w-0">
                      <div class="font-bold text-slate-900 truncate max-w-[200px]" :title="prod.name">
                        {{ prod.name }}
                      </div>
                      <div class="flex items-center gap-1.5 text-[10px] text-slate-500 font-mono">
                        <span>{{ prod.sku }}</span>
                        <span>•</span>
                        <span>{{ prod.barcode || '-' }}</span>
                      </div>
                    </div>
                  </div>
                </td>

                <!-- Harga Terkini di Outlet Terpilih -->
                <td class="px-3 py-2.5 text-right font-mono whitespace-nowrap">
                  <div class="font-black text-emerald-700 text-xs">
                    Rp {{ formatCurrency(prod.base_selling_price) }}
                  </div>
                  <div class="text-[9px] text-slate-400 font-sans">
                    / {{ prod.unit_code || 'PCS' }}
                  </div>
                </td>

                <!-- Stok -->
                <td class="px-3 py-2.5 text-center font-mono text-[11px] text-slate-600">
                  {{ formatQty(prod.stock_qty) }}
                </td>

                <!-- Qty Cetak Stepper -->
                <td class="px-3 py-2.5 text-center" @click.stop>
                  <div v-if="isProductSelected(prod.id)" class="flex items-center justify-center gap-1">
                    <button
                      type="button"
                      @click="adjustQty(prod.id, -1)"
                      class="w-6 h-6 rounded bg-slate-100 hover:bg-slate-200 text-slate-700 flex items-center justify-center font-bold cursor-pointer transition active:scale-95"
                    >
                      <Minus class="w-3 h-3" />
                    </button>
                    <input
                      :value="selectedItemsMap.get(prod.id)?.print_qty || 1"
                      @change="setQty(prod.id, Number(($event.target as HTMLInputElement).value))"
                      type="number"
                      min="1"
                      class="w-11 text-center font-mono font-bold text-xs py-0.5 rounded border border-slate-300 bg-white"
                    />
                    <button
                      type="button"
                      @click="adjustQty(prod.id, 1)"
                      class="w-6 h-6 rounded bg-slate-100 hover:bg-slate-200 text-slate-700 flex items-center justify-center font-bold cursor-pointer transition active:scale-95"
                    >
                      <Plus class="w-3 h-3" />
                    </button>
                  </div>
                  <span v-else class="text-[10px] text-slate-400 italic">Belum dipilih</span>
                </td>
              </tr>

              <tr v-if="filteredProducts.length === 0">
                <td colspan="5" class="py-12 text-center text-slate-400 font-medium">
                  Tidak ada produk yang cocok dengan pencarian.
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <!-- Footer Total Produk & Pagination Bar -->
        <div class="p-3 bg-slate-50 border-t border-slate-200 flex flex-col sm:flex-row items-center justify-between gap-2.5 text-xs text-slate-600 shrink-0">
          <div class="flex items-center gap-2 flex-wrap">
            <span>
              Total <strong class="text-slate-800">{{ totalProducts }}</strong> produk
            </span>
            <span
              v-if="totalSelectedProductsCount > 0"
              class="font-bold text-emerald-700 bg-emerald-50 border border-emerald-200 px-2 py-0.5 rounded-full text-[11px]"
            >
              {{ totalSelectedProductsCount }} produk dipilih ({{ totalLabelsToPrint }} label)
            </span>
          </div>

          <PaginationBar
            v-model:currentPage="currentPage"
            v-model:pageSize="pageSize"
            :totalItems="totalProducts"
            @update:currentPage="fetchProducts"
            @update:pageSize="handlePageSizeChange"
          />
        </div>
      </div>

      <!-- KOLOM KANAN: Pengaturan Format Label & Live Pratinjau (5 Kolom) -->
      <div class="lg:col-span-5 space-y-4">
        <!-- Panel Konfigurasi Template & Opsi Cetak -->
        <div class="bg-white border border-slate-300 rounded-2xl p-4 shadow-xs space-y-3.5 text-xs">
          <div class="flex items-center justify-between border-b border-slate-200 pb-2.5">
            <div class="font-bold text-slate-800 flex items-center gap-1.5 uppercase tracking-wide">
              <Sliders class="w-4 h-4 text-emerald-600" />
              <span>Pengaturan Desain Label</span>
            </div>
            <span class="text-[10px] text-slate-400 font-sans">Sesuaikan dengan kertas Anda</span>
          </div>

          <!-- Format Template Label -->
          <div class="space-y-1.5">
            <label class="block font-bold text-slate-700">Tipe Label:</label>
            <div class="grid grid-cols-3 gap-2">
              <button
                type="button"
                @click="labelTemplate = 'shelf_standard'"
                class="px-2 py-1.5 rounded-xl border text-[11px] font-bold text-center transition cursor-pointer"
                :class="labelTemplate === 'shelf_standard' ? 'bg-emerald-50 border-emerald-500 text-emerald-800 shadow-2xs' : 'bg-slate-50 border-slate-200 text-slate-600 hover:bg-slate-100'"
              >
                🏷️ Label Rak (65×35mm)
              </button>
              <button
                type="button"
                @click="labelTemplate = 'product_sticker'"
                class="px-2 py-1.5 rounded-xl border text-[11px] font-bold text-center transition cursor-pointer"
                :class="labelTemplate === 'product_sticker' ? 'bg-emerald-50 border-emerald-500 text-emerald-800 shadow-2xs' : 'bg-slate-50 border-slate-200 text-slate-600 hover:bg-slate-100'"
              >
                📦 Sticker Produk (40×25mm)
              </button>
              <button
                type="button"
                @click="labelTemplate = 'compact_jewelry'"
                class="px-2 py-1.5 rounded-xl border text-[11px] font-bold text-center transition cursor-pointer"
                :class="labelTemplate === 'compact_jewelry' ? 'bg-emerald-50 border-emerald-500 text-emerald-800 shadow-2xs' : 'bg-slate-50 border-slate-200 text-slate-600 hover:bg-slate-100'"
              >
                🔖 Mini / Tag (30×20mm)
              </button>
            </div>
          </div>

          <!-- Format Kertas Printer -->
          <div class="space-y-1.5">
            <label class="block font-bold text-slate-700">Format Kertas Printer:</label>
            <div class="grid grid-cols-2 gap-2">
              <button
                type="button"
                @click="paperFormat = 'sheet_a4'"
                class="px-3 py-1.5 rounded-xl border text-[11px] font-bold text-center transition cursor-pointer"
                :class="paperFormat === 'sheet_a4' ? 'bg-blue-50 border-blue-500 text-blue-800 shadow-2xs ring-1 ring-blue-400' : 'bg-slate-50 border-slate-200 text-slate-600 hover:bg-slate-100'"
              >
                📄 Kertas Lembaran A4 (Grid)
              </button>
              <button
                type="button"
                @click="paperFormat = 'continuous_roll'"
                class="px-3 py-1.5 rounded-xl border text-[11px] font-bold text-center transition cursor-pointer"
                :class="paperFormat === 'continuous_roll' ? 'bg-blue-50 border-blue-500 text-blue-800 shadow-2xs ring-1 ring-blue-400' : 'bg-slate-50 border-slate-200 text-slate-600 hover:bg-slate-100'"
              >
                📜 Thermal Roll (1 Kolom)
              </button>
            </div>
          </div>

          <!-- Pengaturan Jumlah Kolom A4 (Tampil saat mode A4) -->
          <div v-if="paperFormat === 'sheet_a4'" class="space-y-1.5 p-2.5 rounded-xl bg-blue-50/70 border border-blue-200">
            <div class="flex items-center justify-between">
              <label class="block font-bold text-blue-950 text-xs">Jumlah Kolom Stiker A4:</label>
              <span class="text-[10px] text-blue-700 font-bold bg-white px-2 py-0.5 rounded-full border border-blue-200 shadow-2xs">
                {{ a4GridColumns }} Kolom Sejajar
              </span>
            </div>
            <div class="grid grid-cols-3 gap-1.5">
              <button
                type="button"
                @click="a4GridColumns = 2"
                class="px-2 py-1.5 rounded-lg border text-[11px] font-bold text-center transition cursor-pointer"
                :class="a4GridColumns === 2 ? 'bg-blue-600 text-white border-blue-700 shadow-2xs' : 'bg-white border-blue-200 text-blue-900 hover:bg-blue-100'"
              >
                2 Kolom
              </button>
              <button
                type="button"
                @click="a4GridColumns = 3"
                class="px-2 py-1.5 rounded-lg border text-[11px] font-bold text-center transition cursor-pointer"
                :class="a4GridColumns === 3 ? 'bg-blue-600 text-white border-blue-700 shadow-2xs' : 'bg-white border-blue-200 text-blue-900 hover:bg-blue-100'"
              >
                3 Kolom
              </button>
              <button
                type="button"
                @click="a4GridColumns = 4"
                class="px-2 py-1.5 rounded-lg border text-[11px] font-bold text-center transition cursor-pointer"
                :class="a4GridColumns === 4 ? 'bg-blue-600 text-white border-blue-700 shadow-2xs' : 'bg-white border-blue-200 text-blue-900 hover:bg-blue-100'"
              >
                4 Kolom
              </button>
            </div>
            <p class="text-[10px] text-blue-600 italic">
              *Tampilan di pratinjau dan kertas cetak akan otomatis mengikuti {{ a4GridColumns }} kolom.
            </p>
          </div>

          <!-- Kustomisasi Judul Toko -->
          <div>
            <label class="block font-bold text-slate-700 mb-1">Nama Toko di Atas Label:</label>
            <input
              v-model="customStoreTitle"
              :placeholder="`Default: ${activeStoreDisplayTitle}`"
              class="w-full px-2.5 py-1.5 rounded-xl border border-slate-300 text-xs font-semibold focus:outline-none focus:ring-2 focus:ring-emerald-500"
            />
          </div>

          <!-- Toggle Elemen Tambahan -->
          <div class="grid grid-cols-2 gap-2 pt-1 border-t border-slate-200 text-[11px]">
            <label class="flex items-center gap-2 cursor-pointer select-none text-slate-700">
              <input v-model="showOutletName" type="checkbox" class="rounded text-emerald-600 focus:ring-emerald-500 cursor-pointer" />
              <span>Nama Toko / Brand</span>
            </label>
            <label class="flex items-center gap-2 cursor-pointer select-none text-slate-700">
              <input v-model="showBarcodeLines" type="checkbox" class="rounded text-emerald-600 focus:ring-emerald-500 cursor-pointer" />
              <span>Garis Barcode (Scannable)</span>
            </label>
            <label class="flex items-center gap-2 cursor-pointer select-none text-slate-700">
              <input v-model="showUnitName" type="checkbox" class="rounded text-emerald-600 focus:ring-emerald-500 cursor-pointer" />
              <span>Satuan (PCS/PACK/BTL)</span>
            </label>
            <label class="flex items-center gap-2 cursor-pointer select-none text-slate-700">
              <input v-model="showPrintDate" type="checkbox" class="rounded text-emerald-600 focus:ring-emerald-500 cursor-pointer" />
              <span>Tanggal Cetak</span>
            </label>
            <label class="flex items-center gap-2 cursor-pointer select-none text-slate-700 col-span-2">
              <input v-model="showSku" type="checkbox" class="rounded text-emerald-600 focus:ring-emerald-500 cursor-pointer" />
              <span>Kode SKU Produk</span>
            </label>
          </div>
        </div>

        <!-- Live Pratinjau Desain Label (Preview Visual) -->
        <div class="bg-white border border-slate-300 rounded-2xl p-4 shadow-xs space-y-2.5">
          <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-2">
            <div class="font-bold text-xs text-slate-800 uppercase tracking-wide flex items-center gap-1.5">
              <Eye class="w-4 h-4 text-emerald-600" />
              <span>Pratinjau Hasil Cetak</span>
            </div>

            <!-- Tab Switcher: Semua Label vs Sampel 1 -->
            <div class="flex items-center gap-1 bg-slate-100 p-0.5 rounded-lg border border-slate-200">
              <button
                type="button"
                @click="previewMode = 'all'"
                :class="previewMode === 'all' ? 'bg-white text-emerald-700 font-bold shadow-xs' : 'text-slate-600 hover:text-slate-900 font-medium'"
                class="px-2 py-1 text-[11px] rounded-md transition cursor-pointer flex items-center gap-1"
              >
                <Layers class="w-3.5 h-3.5" />
                <span>Daftar Label ({{ flatLabelsList.length }})</span>
              </button>
              <button
                type="button"
                @click="previewMode = 'single'"
                :class="previewMode === 'single' ? 'bg-white text-emerald-700 font-bold shadow-xs' : 'text-slate-600 hover:text-slate-900 font-medium'"
                class="px-2 py-1 text-[11px] rounded-md transition cursor-pointer flex items-center gap-1"
              >
                <Eye class="w-3.5 h-3.5" />
                <span>Sampel 1</span>
              </button>
            </div>
          </div>

          <!-- Empty Preview State -->
          <div
            v-if="flatLabelsList.length === 0"
            class="p-8 text-center bg-slate-50 border border-dashed border-slate-300 rounded-xl space-y-1.5 text-slate-400"
          >
            <Icon icon="streamline-ultimate-color:tags-1" class="w-10 h-10 mx-auto opacity-40 mb-1" />
            <div class="font-bold text-slate-700 text-xs">Belum Ada Label yang Dipilih</div>
            <div class="text-[11px] max-w-xs mx-auto">
              Centang checkbox produk di sebelah kiri atau klik tombol "Semua Qty=1" untuk memunculkan daftar label.
            </div>
          </div>

          <!-- Live List of All Selected Labels -->
          <div
            v-else-if="previewMode === 'all'"
            class="bg-slate-200/80 p-3 rounded-xl max-h-[460px] overflow-y-auto space-y-2.5"
          >
            <div class="text-[11px] text-slate-600 font-bold flex items-center justify-between pb-1 border-b border-slate-300">
              <span>Daftar {{ flatLabelsList.length }} Label Siap Dicetak (Outlet: {{ activeOutlet?.name }})</span>
              <span class="text-[10px] text-slate-500 font-normal">Sesuai urutan cetak</span>
            </div>

            <div
              class="grid gap-2.5 justify-items-center"
              :class="{
                'grid-cols-1 max-w-[260px] mx-auto': paperFormat === 'continuous_roll',
                'grid-cols-1 sm:grid-cols-2': paperFormat === 'sheet_a4' && a4GridColumns === 2,
                'grid-cols-1 sm:grid-cols-3': paperFormat === 'sheet_a4' && a4GridColumns === 3,
                'grid-cols-2 sm:grid-cols-4': paperFormat === 'sheet_a4' && a4GridColumns === 4,
              }"
            >
              <div
                v-for="(prod, idx) in flatLabelsList"
                :key="`${prod.id}-${idx}`"
                class="bg-white border border-slate-400 rounded shadow-sm overflow-hidden text-slate-900 transition-all select-none relative w-full"
                :class="{
                  'max-w-[240px] p-2.5': labelTemplate === 'shelf_standard' && a4GridColumns <= 2,
                  'max-w-[180px] p-2': labelTemplate === 'product_sticker' || a4GridColumns === 3,
                  'max-w-[140px] p-1.5': labelTemplate === 'compact_jewelry' || a4GridColumns >= 4,
                }"
              >
                <div class="absolute top-1 right-1 px-1 py-0.2 rounded bg-slate-100 text-[8px] font-mono text-slate-500 font-bold border border-slate-200">
                  #{{ idx + 1 }}
                </div>

                <!-- Top Store Name -->
                <div
                  v-if="showOutletName"
                  class="text-[9px] font-black uppercase text-center border-b border-slate-800 pb-0.5 tracking-wider truncate pr-6"
                >
                  {{ activeStoreDisplayTitle }}
                </div>

                <!-- Product Name -->
                <div
                  class="font-black leading-tight text-slate-950 mt-1 line-clamp-2"
                  :class="labelTemplate === 'shelf_standard' ? 'text-xs' : 'text-[11px]'"
                >
                  {{ prod.name }}
                </div>

                <!-- SKU & Category -->
                <div
                  v-if="showSku"
                  class="text-[8px] text-slate-600 font-mono mt-0.5 flex justify-between"
                >
                  <span>{{ prod.sku }}</span>
                  <span>{{ prod.category_name }}</span>
                </div>

                <!-- Barcode SVG Render -->
                <div
                  v-if="showBarcodeLines && prod.barcode"
                  class="my-1 flex flex-col items-center justify-center"
                >
                  <div
                    class="w-full flex items-center justify-center overflow-hidden"
                    :style="{ height: labelTemplate === 'shelf_standard' ? '28px' : '20px' }"
                    v-html="generateCode128Svg(prod.barcode, 30)"
                  ></div>
                  <div class="font-mono text-[9px] font-bold tracking-widest text-slate-900 leading-none mt-0.5">
                    {{ prod.barcode }}
                  </div>
                </div>

                <!-- Giant Price Display -->
                <div class="mt-1 pt-1 border-t border-dashed border-slate-300 flex items-baseline justify-between">
                  <div>
                    <span class="text-[9px] font-bold text-slate-600 mr-0.5">Rp</span>
                    <span
                      class="font-mono font-black text-slate-950 tracking-tight"
                      :class="labelTemplate === 'shelf_standard' ? 'text-base' : 'text-xs'"
                    >
                      {{ formatCurrency(prod.base_selling_price) }}
                    </span>
                  </div>
                  <span v-if="showUnitName" class="text-[8px] font-bold text-slate-600 uppercase">
                    /{{ prod.unit_code || 'PCS' }}
                  </span>
                </div>

                <!-- Bottom Footer Date -->
                <div
                  v-if="showPrintDate"
                  class="text-[7px] text-slate-400 text-right mt-1 font-mono leading-none"
                >
                  Dicetak: {{ printDateStr }}
                </div>
              </div>
            </div>
          </div>

          <!-- Live Sample Card Preview (Single) -->
          <div
            v-else
            class="bg-slate-200/80 p-4 rounded-xl max-h-[460px] overflow-y-auto space-y-3 flex flex-col items-center"
          >
            <div class="text-[10px] text-slate-500 font-semibold mb-1 w-full flex justify-between">
              <span>Sampel Label (Sesuai Outlet {{ activeOutlet?.name }}):</span>
              <span class="font-mono text-emerald-700 font-bold">Rp {{ formatCurrency(flatLabelsList[0]?.base_selling_price) }}</span>
            </div>

            <!-- Single Tag Card Showcase -->
            <div
              class="bg-white border border-slate-400 rounded shadow-md overflow-hidden text-slate-900 transition-all select-none"
              :class="{
                'w-[240px] p-2.5': labelTemplate === 'shelf_standard',
                'w-[180px] p-2': labelTemplate === 'product_sticker',
                'w-[150px] p-1.5': labelTemplate === 'compact_jewelry',
              }"
            >
              <!-- Top Store Name -->
              <div
                v-if="showOutletName"
                class="text-[9px] font-black uppercase text-center border-b border-slate-800 pb-0.5 tracking-wider truncate"
              >
                {{ activeStoreDisplayTitle }}
              </div>

              <!-- Product Name -->
              <div
                class="font-black leading-tight text-slate-950 mt-1 line-clamp-2"
                :class="labelTemplate === 'shelf_standard' ? 'text-xs' : 'text-[11px]'"
              >
                {{ flatLabelsList[0]?.name }}
              </div>

              <!-- SKU & Category -->
              <div
                v-if="showSku"
                class="text-[8px] text-slate-600 font-mono mt-0.5 flex justify-between"
              >
                <span>{{ flatLabelsList[0]?.sku }}</span>
                <span>{{ flatLabelsList[0]?.category_name }}</span>
              </div>

              <!-- Barcode SVG Render -->
              <div
                v-if="showBarcodeLines && flatLabelsList[0]?.barcode"
                class="my-1.5 flex flex-col items-center justify-center"
              >
                <div
                  class="w-full flex items-center justify-center overflow-hidden"
                  :style="{ height: labelTemplate === 'shelf_standard' ? '30px' : '22px' }"
                  v-html="generateCode128Svg(flatLabelsList[0]?.barcode, 32)"
                ></div>
                <div class="font-mono text-[9px] font-bold tracking-widest text-slate-900 leading-none mt-0.5">
                  {{ flatLabelsList[0]?.barcode }}
                </div>
              </div>

              <!-- Giant Price Display -->
              <div class="mt-1 pt-1 border-t border-dashed border-slate-300 flex items-baseline justify-between">
                <div>
                  <span class="text-[9px] font-bold text-slate-600 mr-0.5">Rp</span>
                  <span
                    class="font-mono font-black text-slate-950 tracking-tight"
                    :class="labelTemplate === 'shelf_standard' ? 'text-lg' : 'text-sm'"
                  >
                    {{ formatCurrency(flatLabelsList[0]?.base_selling_price) }}
                  </span>
                </div>
                <span v-if="showUnitName" class="text-[8px] font-bold text-slate-600 uppercase">
                  /{{ flatLabelsList[0]?.unit_code || 'PCS' }}
                </span>
              </div>

              <!-- Bottom Footer Date -->
              <div
                v-if="showPrintDate"
                class="text-[7px] text-slate-400 text-right mt-1 font-mono leading-none"
              >
                Dicetak: {{ printDateStr }}
              </div>
            </div>

            <div class="text-[11px] text-slate-500 italic text-center">
              + {{ flatLabelsList.length - 1 }} label lainnya dalam antrean cetak
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- ===================================================================== -->
    <!-- PRINTABLE AREA (Hanya tampil saat print via @media print) -->
    <!-- ===================================================================== -->
    <div class="print-only print-sheet-container">
      <div
        id="notane-printable-grid"
        class="print-grid"
        :class="{
          'mode-roll': paperFormat === 'continuous_roll',
          'mode-a4': paperFormat === 'sheet_a4',
          'template-shelf': labelTemplate === 'shelf_standard',
          'template-sticker': labelTemplate === 'product_sticker',
          'template-compact': labelTemplate === 'compact_jewelry',
        }"
      >
        <div
          v-for="(prod, idx) in flatLabelsList"
          :key="`${prod.id}-${idx}`"
          class="price-tag-card"
        >
          <!-- Store Header -->
          <div v-if="showOutletName" class="tag-store-header">
            {{ activeStoreDisplayTitle }}
          </div>

          <!-- Product Name -->
          <div class="tag-product-name">
            {{ prod.name }}
          </div>

          <!-- SKU & Category -->
          <div v-if="showSku" class="tag-meta-info">
            <span>{{ prod.sku }}</span>
            <span v-if="prod.category_name">{{ prod.category_name }}</span>
          </div>

          <!-- Barcode Vector SVG -->
          <div v-if="showBarcodeLines && prod.barcode" class="tag-barcode-container">
            <div
              class="tag-barcode-svg"
              v-html="generateCode128Svg(prod.barcode, 30)"
            ></div>
            <div class="tag-barcode-text">
              {{ prod.barcode }}
            </div>
          </div>

          <!-- Price Display (Huge & Bold) -->
          <div class="tag-price-row">
            <div class="tag-price-amount">
              <span class="currency-symbol">Rp</span>
              <span class="price-val">{{ formatCurrency(prod.base_selling_price) }}</span>
            </div>
            <span v-if="showUnitName" class="unit-badge">
              /{{ prod.unit_code || 'PCS' }}
            </span>
          </div>

          <!-- Date Footer -->
          <div v-if="showPrintDate" class="tag-footer-date">
            {{ printDateStr }}
          </div>
        </div>
      </div>
    </div>

    <!-- Modal Pengaturan Printer Label -->
    <LabelPrinterSettingsModal
      :show="showLabelPrinterModal"
      @close="showLabelPrinterModal = false"
    />

    <!-- ===================================================================== -->
    <!-- MODAL PRATINJAU DOKUMEN CETAK LABEL (Interactive Print Preview)      -->
    <!-- ===================================================================== -->
    <div
      v-if="showPreviewModal"
      class="fixed inset-0 z-50 bg-slate-900/60 backdrop-blur-xs flex items-center justify-center p-3 sm:p-5"
      @click.self="showPreviewModal = false"
    >
      <div
        class="bg-white rounded-2xl shadow-2xl border border-slate-300 w-full max-w-5xl h-[90vh] flex flex-col overflow-hidden animate-in fade-in zoom-in-95 duration-150"
      >
        <!-- Preview Modal Header -->
        <div class="px-5 py-3.5 bg-slate-50 border-b border-slate-200 flex flex-wrap items-center justify-between gap-3 shrink-0">
          <div class="flex items-center gap-3">
            <div class="w-10 h-10 rounded-xl bg-emerald-100 border border-emerald-300 text-emerald-800 flex items-center justify-center shadow-2xs shrink-0">
              <Eye class="w-5 h-5" />
            </div>
            <div>
              <div class="flex items-center gap-2">
                <h3 class="text-sm font-bold text-slate-900">Pratinjau Dokumen Cetak Label</h3>
                <span class="px-2 py-0.5 rounded-full text-[10px] font-bold bg-emerald-100 text-emerald-800 border border-emerald-300">
                  {{ flatLabelsList.length }} Label
                </span>
                <span class="px-2 py-0.5 rounded-full text-[10px] font-bold bg-blue-100 text-blue-800 border border-blue-300">
                  {{ paperFormat === 'sheet_a4' ? `Lembaran A4 (${a4GridColumns} Kolom)` : 'Continuous Roll' }}
                </span>
              </div>
              <p class="text-xs text-slate-500 mt-0.5">
                Target: <strong class="text-slate-700">{{ activeStoreDisplayTitle }}</strong> &bull; Printer: <strong class="text-slate-800 font-mono">{{ labelPrinterStore.printerName }}</strong>
              </p>
            </div>
          </div>

          <div class="flex items-center gap-2">
            <!-- Tombol Cetak Dokumen / Ekspor PDF -->
            <button
              type="button"
              @click="triggerPrintFromPreview"
              class="px-4 py-2 rounded-xl bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-xs flex items-center gap-2 shadow-xs transition active:scale-95 cursor-pointer"
              title="Kirim ke printer atau simpan ke PDF via dialog sistem operasi"
            >
              <Printer class="w-4 h-4" />
              <span>Cetak / Ekspor PDF</span>
            </button>

            <!-- Tombol Buka Tab Terpisah -->
            <button
              type="button"
              @click="openInExternalWindow"
              class="p-2 rounded-xl bg-white hover:bg-slate-100 border border-slate-300 text-slate-700 hover:text-slate-900 transition cursor-pointer"
              title="Buka dokumen dalam jendela / tab browser baru"
            >
              <Icon icon="fluent:open-24-filled" class="w-4 h-4" />
            </button>

            <!-- Tombol Tutup -->
            <button
              type="button"
              @click="showPreviewModal = false"
              class="w-8 h-8 rounded-xl hover:bg-slate-200 text-slate-500 hover:text-slate-800 flex items-center justify-center transition cursor-pointer text-lg font-bold"
            >
              ✕
            </button>
          </div>
        </div>

        <!-- Preview Modal Body (Embedded Iframe) -->
        <div class="flex-1 bg-slate-200/90 p-3 sm:p-5 overflow-hidden flex flex-col items-center justify-center">
          <div class="w-full h-full bg-white rounded-xl shadow-lg border border-slate-400 overflow-hidden relative">
            <iframe
              ref="previewIframeRef"
              :srcdoc="previewHtmlContent"
              class="w-full h-full border-0"
              title="Pratinjau Cetak Lembaran Label"
            ></iframe>
          </div>
        </div>

        <!-- Preview Modal Footer -->
        <div class="px-5 py-2.5 bg-slate-50 border-t border-slate-200 flex flex-wrap items-center justify-between text-xs text-slate-500 shrink-0 gap-2">
          <div class="flex items-center gap-2">
            <span>Ukuran Label: <strong class="text-slate-800 font-mono">{{ labelPrinterStore.effectiveWidthMm }}×{{ labelPrinterStore.effectiveHeightMm }}mm</strong></span>
            <span>&bull;</span>
            <span>Format: <strong class="text-slate-800">{{ paperFormat === 'sheet_a4' ? 'Lembaran Stiker A4' : 'Roll Thermal' }}</strong></span>
            <span>&bull;</span>
            <span v-if="labelPrinterStore.isPdfOrDocumentPrinter" class="text-blue-700 font-bold flex items-center gap-1">
              <Icon icon="fluent:document-pdf-24-filled" class="w-3.5 h-3.5 text-rose-600" />
              Mode PDF Virtual (Simpan sebagai file PDF)
            </span>
            <span v-else class="text-slate-600">
              Mode Cetak Printer Standar
            </span>
          </div>

          <div class="flex items-center gap-2">
            <button
              type="button"
              @click="showPreviewModal = false"
              class="px-4 py-1.5 rounded-lg border border-slate-300 bg-white hover:bg-slate-100 text-slate-700 font-bold transition cursor-pointer"
            >
              Tutup Pratinjau
            </button>
          </div>
        </div>
      </div>
    </div>
  </AdminLayout>
</template>

<style>
/* ========================================================================== */
/* SCREEN STYLING (Default browser view)                                      */
/* ========================================================================== */
.print-only {
  display: none !important;
}

/* ========================================================================== */
/* PRINT STYLING (@media print)                                               */
/* ========================================================================== */
@media print {
  /* Hilangkan limitasi viewport dan overflow dari AdminLayout */
  html,
  body,
  #app,
  .h-screen,
  main {
    height: auto !important;
    min-height: auto !important;
    max-height: none !important;
    overflow: visible !important;
    position: static !important;
    background: #ffffff !important;
    color: #000000 !important;
    margin: 0 !important;
    padding: 0 !important;
  }

  /* Sembunyikan semua UI admin di AdminLayout dan komponen non-print */
  nav,
  aside,
  header,
  footer,
  button,
  .no-print {
    display: none !important;
  }

  /* Tampilkan hanya wadah cetak stiker */
  .print-only {
    display: block !important;
    visibility: visible !important;
    position: static !important;
    width: 100% !important;
  }

  @page {
    margin: 3mm;
    size: auto;
  }

  .print-sheet-container {
    width: 100%;
    margin: 0;
    padding: 0;
  }

  /* Grid Layout untuk Kertas A4 */
  .print-grid.mode-a4 {
    display: grid !important;
    grid-template-columns: repeat(3, 1fr) !important;
    gap: 3mm !important;
    width: 100% !important;
  }

  .print-grid.mode-a4.template-sticker {
    grid-template-columns: repeat(4, 1fr) !important;
    gap: 2mm !important;
  }

  .print-grid.mode-a4.template-compact {
    grid-template-columns: repeat(5, 1fr) !important;
    gap: 2mm !important;
  }

  /* Layout untuk Printer Thermal Roll Continuous */
  .print-grid.mode-roll {
    display: flex !important;
    flex-direction: column !important;
    align-items: center !important;
    gap: 3mm !important;
    width: 100% !important;
  }

  /* Card Dasar Label Harga */
  .price-tag-card {
    page-break-inside: avoid !important;
    break-inside: avoid !important;
    box-sizing: border-box !important;
    border: 1px solid #000000 !important;
    background: #ffffff !important;
    color: #000000 !important;
    display: flex !important;
    flex-direction: column !important;
    justify-content: space-between !important;
  }

  /* Dimensi Template: Shelf Standard (65x35mm) */
  .template-shelf .price-tag-card {
    width: 65mm !important;
    height: 35mm !important;
    padding: 2mm !important;
  }

  /* Dimensi Template: Product Sticker (40x25mm) */
  .template-sticker .price-tag-card {
    width: 40mm !important;
    height: 25mm !important;
    padding: 1.5mm !important;
  }

  /* Dimensi Template: Compact Jewelry (30x20mm) */
  .template-compact .price-tag-card {
    width: 30mm !important;
    height: 20mm !important;
    padding: 1mm !important;
  }

  /* Elemen-elemen di Dalam Label */
  .tag-store-header {
    font-size: 7pt !important;
    font-weight: 900 !important;
    text-transform: uppercase !important;
    text-align: center !important;
    border-bottom: 1px solid #000000 !important;
    padding-bottom: 0.5mm !important;
    line-height: 1 !important;
    white-space: nowrap !important;
    overflow: hidden !important;
    text-overflow: ellipsis !important;
  }

  .tag-product-name {
    font-size: 8.5pt !important;
    font-weight: 900 !important;
    line-height: 1.1 !important;
    margin-top: 1mm !important;
    display: -webkit-box !important;
    -webkit-line-clamp: 2 !important;
    -webkit-box-orient: vertical !important;
    overflow: hidden !important;
    text-overflow: ellipsis !important;
  }

  .template-sticker .tag-product-name {
    font-size: 7pt !important;
  }

  .template-compact .tag-product-name {
    font-size: 6pt !important;
  }

  .tag-meta-info {
    font-size: 6pt !important;
    font-family: monospace !important;
    color: #333333 !important;
    display: flex !important;
    justify-content: space-between !important;
    line-height: 1 !important;
    margin-top: 0.5mm !important;
  }

  .tag-barcode-container {
    margin: 1mm 0 !important;
    display: flex !important;
    flex-direction: column !important;
    align-items: center !important;
    justify-content: center !important;
  }

  .tag-barcode-svg {
    width: 90% !important;
    height: 7mm !important;
    display: flex !important;
    align-items: center !important;
    justify-content: center !important;
  }

  .template-sticker .tag-barcode-svg {
    height: 5mm !important;
  }

  .template-compact .tag-barcode-svg {
    height: 4mm !important;
  }

  .tag-barcode-text {
    font-family: monospace !important;
    font-size: 6.5pt !important;
    font-weight: bold !important;
    letter-spacing: 1px !important;
    line-height: 1 !important;
    margin-top: 0.3mm !important;
  }

  .tag-price-row {
    display: flex !important;
    align-items: baseline !important;
    justify-content: space-between !important;
    border-top: 1px dashed #000000 !important;
    padding-top: 1mm !important;
    margin-top: auto !important;
  }

  .tag-price-amount {
    display: flex !important;
    align-items: baseline !important;
  }

  .currency-symbol {
    font-size: 7pt !important;
    font-weight: bold !important;
    margin-right: 0.5mm !important;
  }

  .price-val {
    font-family: monospace !important;
    font-size: 13pt !important;
    font-weight: 900 !important;
    line-height: 1 !important;
  }

  .template-sticker .price-val {
    font-size: 9.5pt !important;
  }

  .template-compact .price-val {
    font-size: 8pt !important;
  }

  .unit-badge {
    font-size: 6.5pt !important;
    font-weight: bold !important;
    text-transform: uppercase !important;
  }

  .tag-footer-date {
    font-size: 5.5pt !important;
    color: #444444 !important;
    text-align: right !important;
    line-height: 1 !important;
    margin-top: 0.5mm !important;
  }
}
</style>
