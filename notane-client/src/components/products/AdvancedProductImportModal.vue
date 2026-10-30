<script setup lang="ts">
import { ref, computed } from 'vue'
import * as XLSX from 'xlsx'
import JSZip from 'jszip'
import {
  Upload,
  FileSpreadsheet,
  Image as ImageIcon,
  CheckCircle2,
  AlertTriangle,
  XCircle,
  Download,
  FileArchive,
  RefreshCw,
  Search,
  Check,
  ChevronRight,
  ArrowRight,
} from 'lucide-vue-next'
import { Icon } from '@iconify/vue'
import { api } from '@/services/api'
import { formatCurrency } from '@/utils/formatters'

const props = defineProps<{
  isOpen: boolean
  categories: any[]
  units: any[]
  outlets: any[]
  customerGroups: any[]
  warehouses: any[]
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'imported'): void
}>()

// Wizard steps: 'UPLOAD' -> 'PREVIEW' -> 'RESULT'
const currentStep = ref<'UPLOAD' | 'PREVIEW' | 'RESULT'>('UPLOAD')

// State File & Foto
const dataFileName = ref('')
const isParsingData = ref(false)
const rawRows = ref<any[]>([])
const parsedProducts = ref<any[]>([])
const uploadedPhotos = ref<Map<string, string>>(new Map()) // filename lowercase -> data URL
const uploadedPhotosCount = ref(0)
const photoArchiveName = ref('')
const isExtractingZip = ref(false)

// Pengaturan Import
const updateExisting = ref(true)
const selectedWarehouseId = ref<string>('')
const isSubmitting = ref(false)
const importProgress = ref(0)

// Hasil Import
const importResult = ref<{
  inserted: number
  updated: number
  skipped: number
  images_attached: number
  outlet_prices_saved: number
  member_prices_saved: number
  errors: string[]
} | null>(null)

// Preview table filter & pagination
const previewSearch = ref('')
const previewPage = ref(1)
const previewPageSize = ref(10)

const filteredPreviewProducts = computed(() => {
  if (!previewSearch.value) return parsedProducts.value
  const q = previewSearch.value.toLowerCase()
  return parsedProducts.value.filter(
    p =>
      p.name?.toLowerCase().includes(q) ||
      p.sku?.toLowerCase().includes(q) ||
      p.barcode?.toLowerCase().includes(q) ||
      p.category_name?.toLowerCase().includes(q)
  )
})

const paginatedPreviewProducts = computed(() => {
  const start = (previewPage.value - 1) * previewPageSize.value
  return filteredPreviewProducts.value.slice(start, start + previewPageSize.value)
})

const totalPhotosMatched = computed(() => {
  return parsedProducts.value.filter(p => !!p.image_url).length
})

// ==========================================
// 1. GENERATE TEMPLATE EXCEL & CSV
// ==========================================
function generateTemplateHeaders() {
  const headers = [
    'Barcode',
    'SKU',
    'Nama_Produk',
    'Kategori',
    'Satuan',
    'HPP_Beli',
    'Harga_Jual_Dasar',
    'Stok_Awal',
    'Min_Stok',
    'Foto_File',
  ]

  // Kolom dinamis untuk setiap outlet aktif
  for (const o of props.outlets || []) {
    headers.push(`Harga_Jual_${o.code}`)
  }

  // Kolom dinamis untuk setiap kelompok member aktif
  for (const cg of props.customerGroups || []) {
    const cleanName = cg.name.replace(/\s+/g, '_')
    headers.push(`Harga_Member_${cleanName}`)
  }

  return headers
}

function generateSampleRow() {
  const row: Record<string, any> = {
    Barcode: '8999908123456',
    SKU: 'BRG-KOP-001',
    Nama_Produk: 'Kopi Susu Gula Aren 250ml',
    Kategori: props.categories[0]?.name || 'Minuman',
    Satuan: props.units[0]?.code || 'BTL',
    HPP_Beli: 8000,
    Harga_Jual_Dasar: 15000,
    Stok_Awal: 50,
    Min_Stok: 10,
    Foto_File: 'kopi_susu.jpg',
  }

  // Sample data outlet
  for (const o of props.outlets || []) {
    row[`Harga_Jual_${o.code}`] = 16000
  }

  // Sample data member
  for (const cg of props.customerGroups || []) {
    const cleanName = cg.name.replace(/\s+/g, '_')
    row[`Harga_Member_${cleanName}`] = 13500
  }

  return row
}

function downloadExcelTemplate() {
  const headers = generateTemplateHeaders()
  const sampleRow = generateSampleRow()
  const data = [sampleRow]

  const ws = XLSX.utils.json_to_sheet(data, { header: headers })
  const wb = XLSX.utils.book_new()
  XLSX.utils.book_append_sheet(wb, ws, 'Template Produk')
  XLSX.writeFile(wb, 'Template_Import_Produk_Notane_POS.xlsx')
}

function downloadCsvTemplate() {
  const headers = generateTemplateHeaders()
  const sampleRow = generateSampleRow()
  const data = [sampleRow]

  const ws = XLSX.utils.json_to_sheet(data, { header: headers })
  const csv = XLSX.utils.sheet_to_csv(ws)
  const blob = new Blob([csv], { type: 'text/csv;charset=utf-8;' })
  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.setAttribute('href', url)
  link.setAttribute('download', 'Template_Import_Produk_Notane_POS.csv')
  document.body.appendChild(link)
  link.click()
  document.body.removeChild(link)
}

// ==========================================
// 2. PARSING FILE DATA (EXCEL / CSV)
// ==========================================
async function handleDataFileUpload(event: Event) {
  const input = event.target as HTMLInputElement
  if (!input.files || input.files.length === 0) return

  const file = input.files[0]
  dataFileName.value = file.name
  isParsingData.value = true

  try {
    const buffer = await file.arrayBuffer()
    const workbook = XLSX.read(buffer, { type: 'array' })
    const firstSheetName = workbook.SheetNames[0]
    const worksheet = workbook.Sheets[firstSheetName]
    const jsonData = XLSX.utils.sheet_to_json(worksheet, { defval: '' }) as any[]

    rawRows.value = jsonData
    matchAndBuildProducts()
  } catch (err: any) {
    alert('Gagal membaca file data Excel/CSV: ' + err.message)
  } finally {
    isParsingData.value = false
  }
}

// ==========================================
// 3. UPLOAD & DETEKSI PAKET FOTO
// ==========================================
// A. Upload Multiple File Gambar Langsung
async function handlePhotosBatchUpload(event: Event) {
  const input = event.target as HTMLInputElement
  if (!input.files || input.files.length === 0) return

  const files = Array.from(input.files)
  for (const f of files) {
    if (!f.type.startsWith('image/')) continue
    const base64 = await readFileAsDataUrl(f)
    const baseName = f.name.toLowerCase()
    uploadedPhotos.value.set(baseName, base64)
    // Juga simpan variasi tanpa ekstensi
    const nameWithoutExt = baseName.substring(0, baseName.lastIndexOf('.'))
    if (nameWithoutExt) {
      uploadedPhotos.value.set(nameWithoutExt, base64)
    }
  }

  uploadedPhotosCount.value = uploadedPhotos.value.size
  photoArchiveName.value = `${files.length} file gambar terpilih`
  matchAndBuildProducts()
}

// B. Upload File ZIP Kumpulan Foto
async function handleZipArchiveUpload(event: Event) {
  const input = event.target as HTMLInputElement
  if (!input.files || input.files.length === 0) return

  const file = input.files[0]
  photoArchiveName.value = file.name
  isExtractingZip.value = true

  try {
    const zip = new JSZip()
    const content = await zip.loadAsync(file)
    let extractedCount = 0

    for (const [filename, rawFileObj] of Object.entries(content.files)) {
      const fileObj = rawFileObj as any
      if (fileObj.dir) continue
      const lower = filename.toLowerCase()
      // Cek apakah ekstensi gambar valid
      if (
        lower.endsWith('.jpg') ||
        lower.endsWith('.jpeg') ||
        lower.endsWith('.png') ||
        lower.endsWith('.webp') ||
        lower.endsWith('.gif')
      ) {
        // Ambil hanya nama filenya saja tanpa path subfolder
        const pureFileName = lower.split('/').pop() || lower
        const blob = await fileObj.async('blob')
        const dataUrl = await readBlobAsDataUrl(blob)

        uploadedPhotos.value.set(pureFileName, dataUrl)
        const nameWithoutExt = pureFileName.substring(0, pureFileName.lastIndexOf('.'))
        if (nameWithoutExt) {
          uploadedPhotos.value.set(nameWithoutExt, dataUrl)
        }
        extractedCount++
      }
    }

    uploadedPhotosCount.value = extractedCount
    matchAndBuildProducts()
  } catch (err: any) {
    alert('Gagal mengekstrak file ZIP foto: ' + err.message)
  } finally {
    isExtractingZip.value = false
  }
}

function readFileAsDataUrl(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => resolve(reader.result as string)
    reader.onerror = reject
    reader.readAsDataURL(file)
  })
}

function readBlobAsDataUrl(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => resolve(reader.result as string)
    reader.onerror = reject
    reader.readAsDataURL(blob)
  })
}

// ==========================================
// 4. SMART MATCHING & STRUCTURING DATA
// ==========================================
function findPhotoMatch(photoKey: string, sku: string, barcode: string, name: string): string | null {
  const map = uploadedPhotos.value

  // 1. Jika ada di kolom Foto_File
  if (photoKey) {
    const clean = photoKey.trim().toLowerCase()
    if (clean.startsWith('http://') || clean.startsWith('https://') || clean.startsWith('data:image/')) {
      return photoKey.trim()
    }
    if (map.has(clean)) return map.get(clean)!
    // Coba tanpa ekstensi
    const withoutExt = clean.substring(0, clean.lastIndexOf('.'))
    if (withoutExt && map.has(withoutExt)) return map.get(withoutExt)!
  }

  // 2. Coba cocokkan dengan SKU
  if (sku) {
    const s = sku.trim().toLowerCase()
    if (map.has(s)) return map.get(s)!
    if (map.has(`${s}.jpg`)) return map.get(`${s}.jpg`)!
    if (map.has(`${s}.png`)) return map.get(`${s}.png`)!
    if (map.has(`${s}.webp`)) return map.get(`${s}.webp`)!
  }

  // 3. Coba cocokkan dengan Barcode
  if (barcode) {
    const b = barcode.trim().toLowerCase()
    if (map.has(b)) return map.get(b)!
    if (map.has(`${b}.jpg`)) return map.get(`${b}.jpg`)!
    if (map.has(`${b}.png`)) return map.get(`${b}.png`)!
    if (map.has(`${b}.webp`)) return map.get(`${b}.webp`)!
  }

  // 4. Coba cocokkan dengan Nama Produk
  if (name) {
    const n = name.trim().toLowerCase()
    if (map.has(n)) return map.get(n)!
  }

  return null
}

function matchAndBuildProducts() {
  if (rawRows.value.length === 0) return

  const result: any[] = []

  for (let idx = 0; idx < rawRows.value.length; idx++) {
    const row = rawRows.value[idx]

    // Normalisasi nama kolom yang fleksibel
    const barcode = String(
      row.Barcode || row.barcode || row['Bar Code'] || row['Kode Barcode'] || ''
    ).trim()
    const sku = String(row.SKU || row.sku || row['Kode Produk'] || row['Kode Barang'] || barcode).trim()
    const name = String(row.Nama_Produk || row.name || row['Nama Produk'] || row['Nama Barang'] || '').trim()

    if (!barcode && !sku && !name) continue // Abaikan baris kosong

    const category_name = String(row.Kategori || row.category || row['Kategori Produk'] || '').trim()
    const unit_code = String(row.Satuan || row.unit || row['Satuan Dasar'] || 'PCS').trim()
    const cost_price = Number(row.HPP_Beli || row.cost_price || row['Harga Beli'] || 0)
    const base_selling_price = Number(
      row.Harga_Jual_Dasar || row.base_selling_price || row['Harga Jual'] || row.Harga || 0
    )
    const initial_stock = Number(row.Stok_Awal || row.initial_stock || row['Stok Awal'] || 0)
    const min_stock = Number(row.Min_Stok || row.min_stock || 0)
    const photoFileKey = String(row.Foto_File || row.Foto || row.Image || row.Gambar || '').trim()

    // Cari foto yang cocok
    const matchedPhoto = findPhotoMatch(photoFileKey, sku, barcode, name)

    // Deteksi Harga per Outlet
    const outlet_prices: any[] = []
    for (const o of props.outlets || []) {
      const val =
        row[`Harga_Jual_${o.code}`] ||
        row[`Harga_${o.code}`] ||
        row[`Harga Jual ${o.code}`] ||
        row[`Harga ${o.name}`]
      if (val !== undefined && val !== '' && !isNaN(Number(val))) {
        outlet_prices.push({
          outlet_id: o.id,
          outlet_code: o.code,
          cost_price: cost_price,
          selling_price: Number(val),
        })
      }
    }

    // Deteksi Harga per Member (Customer Group)
    const member_prices: any[] = []
    for (const cg of props.customerGroups || []) {
      const cleanName = cg.name.replace(/\s+/g, '_')
      const val =
        row[`Harga_Member_${cleanName}`] ||
        row[`Harga_Member_${cg.name}`] ||
        row[`Harga Member ${cg.name}`] ||
        row[`Harga_${cleanName}`]
      if (val !== undefined && val !== '' && !isNaN(Number(val))) {
        member_prices.push({
          customer_group_id: cg.id,
          customer_group_name: cg.name,
          special_price: Number(val),
        })
      }
    }

    result.push({
      _id: idx,
      sku: sku || barcode,
      barcode: barcode || sku,
      name: name || 'Produk Tanpa Nama',
      category_name: category_name || 'Umum',
      unit_code: unit_code || 'PCS',
      cost_price: cost_price,
      base_selling_price: base_selling_price,
      initial_stock: initial_stock,
      min_stock: min_stock,
      photoFileKey: photoFileKey,
      image_url: matchedPhoto,
      outlet_prices: outlet_prices,
      member_prices: member_prices,
      isValid: !!(sku || barcode) && !!name && base_selling_price > 0,
    })
  }

  parsedProducts.value = result
}

// ==========================================
// 5. EKSEKUSI BATCH IMPORT KE SERVER
// ==========================================
async function executeImport() {
  if (parsedProducts.value.length === 0) {
    alert('Tidak ada data produk yang siap di-import.')
    return
  }

  isSubmitting.value = true
  importProgress.value = 20

  try {
    const payload = {
      update_existing: updateExisting.value,
      default_warehouse_id: selectedWarehouseId.value || undefined,
      products: parsedProducts.value.map(p => ({
        sku: p.sku,
        barcode: p.barcode,
        name: p.name,
        category_name: p.category_name,
        unit_code: p.unit_code,
        cost_price: p.cost_price,
        base_selling_price: p.base_selling_price,
        initial_stock: p.initial_stock > 0 ? p.initial_stock : undefined,
        min_stock: p.min_stock,
        image_url: p.image_url || undefined,
        outlet_prices: p.outlet_prices.length > 0 ? p.outlet_prices : undefined,
        member_prices: p.member_prices.length > 0 ? p.member_prices : undefined,
      })),
    }

    importProgress.value = 60
    const res = await api.post('/products/import', payload)
    importProgress.value = 100

    if (res.data.success) {
      importResult.value = res.data.data
      currentStep.value = 'RESULT'
      emit('imported')
    } else {
      alert('Gagal melakukan import: ' + (res.data.message || 'Terjadi kesalahan sistem'))
    }
  } catch (err: any) {
    alert('Terjadi kesalahan saat mengirim data import: ' + (err.response?.data?.message || err.message))
  } finally {
    isSubmitting.value = false
  }
}

function resetWizard() {
  currentStep.value = 'UPLOAD'
  dataFileName.value = ''
  rawRows.value = []
  parsedProducts.value = []
  uploadedPhotos.value.clear()
  uploadedPhotosCount.value = 0
  photoArchiveName.value = ''
  importResult.value = null
  previewPage.value = 1
}

function handleClose() {
  resetWizard()
  emit('close')
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="isOpen"
      class="fixed inset-0 z-50 bg-slate-900/60 backdrop-blur-xs flex items-center justify-center p-4 select-none font-sans"
    >
    <div
      class="bg-white border border-slate-300 w-full max-w-5xl rounded-2xl shadow-2xl flex flex-col max-h-[92vh] overflow-hidden animate-fadeIn"
    >
      <!-- 1. HEADER MODAL -->
      <div class="px-6 py-4 border-b border-slate-200 flex items-center justify-between bg-slate-50">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-xl bg-emerald-600 text-white flex items-center justify-center shadow-xs">
            <FileSpreadsheet class="w-5 h-5" />
          </div>
          <div>
            <h3 class="font-bold text-base text-slate-900 flex items-center gap-2">
              <span>Impor Data Produk dari Excel</span>
              <span class="text-[10px] bg-slate-100 text-slate-700 font-mono font-bold px-2 py-0.5 rounded border border-slate-200">
                Multi-Outlet & Multi-Harga
              </span>
            </h3>
            <p class="text-xs text-slate-500">
              Impor massal master produk, penetapan barcode, stok awal, dan daftar harga outlet.
            </p>
          </div>
        </div>

        <button
          type="button"
          @click="handleClose"
          class="w-8 h-8 rounded-lg hover:bg-slate-200/80 text-slate-400 hover:text-slate-700 flex items-center justify-center transition cursor-pointer text-sm font-bold"
        >
          ✕
        </button>
      </div>

      <!-- 2. STEP INDICATOR -->
      <div class="px-6 py-2.5 bg-slate-100/70 border-b border-slate-200 flex items-center justify-between text-xs">
        <div class="flex items-center gap-6">
          <div
            class="flex items-center gap-2 font-bold cursor-pointer"
            :class="currentStep === 'UPLOAD' ? 'text-emerald-700' : 'text-slate-500'"
            @click="currentStep === 'PREVIEW' ? (currentStep = 'UPLOAD') : null"
          >
            <span
              class="w-5 h-5 rounded-full flex items-center justify-center text-[10px] font-mono"
              :class="currentStep === 'UPLOAD' ? 'bg-emerald-600 text-white font-bold' : 'bg-slate-200 text-slate-700'"
            >
              1
            </span>
            <span>Upload File & Foto</span>
          </div>

          <ChevronRight class="w-3.5 h-3.5 text-slate-400" />

          <div
            class="flex items-center gap-2 font-bold"
            :class="currentStep === 'PREVIEW' ? 'text-emerald-700' : 'text-slate-500'"
          >
            <span
              class="w-5 h-5 rounded-full flex items-center justify-center text-[10px] font-mono"
              :class="currentStep === 'PREVIEW' ? 'bg-emerald-600 text-white font-bold' : 'bg-slate-200 text-slate-700'"
            >
              2
            </span>
            <span>Pratinjau & Deteksi Foto ({{ parsedProducts.length }})</span>
          </div>

          <ChevronRight class="w-3.5 h-3.5 text-slate-400" />

          <div
            class="flex items-center gap-2 font-bold"
            :class="currentStep === 'RESULT' ? 'text-emerald-700' : 'text-slate-500'"
          >
            <span
              class="w-5 h-5 rounded-full flex items-center justify-center text-[10px] font-mono"
              :class="currentStep === 'RESULT' ? 'bg-emerald-600 text-white font-bold' : 'bg-slate-200 text-slate-700'"
            >
              3
            </span>
            <span>Selesai & Laporan</span>
          </div>
        </div>

        <!-- Download Template Shortcuts -->
        <div class="flex items-center gap-2">
          <button
            type="button"
            @click="downloadExcelTemplate"
            class="px-2.5 py-1 bg-white hover:bg-emerald-50 text-emerald-800 border border-emerald-300 rounded-md text-[11px] font-bold flex items-center gap-1.5 shadow-2xs transition cursor-pointer"
            title="Download Template Format Excel (.xlsx) dengan kolom outlet dan member aktif"
          >
            <Download class="w-3.5 h-3.5 text-emerald-600" />
            <span>Template Excel (.xlsx)</span>
          </button>
          <button
            type="button"
            @click="downloadCsvTemplate"
            class="px-2.5 py-1 bg-white hover:bg-slate-50 text-slate-700 border border-slate-300 rounded-md text-[11px] font-bold flex items-center gap-1.5 shadow-2xs transition cursor-pointer"
          >
            <Download class="w-3.5 h-3.5 text-slate-500" />
            <span>Template CSV</span>
          </button>
        </div>
      </div>

      <!-- 3. BODY MODAL -->
      <div class="flex-1 overflow-y-auto p-6 space-y-6">
        <!-- ============================================== -->
        <!-- STEP 1: UPLOAD FILE DATA & FOTO                -->
        <!-- ============================================== -->
        <div v-if="currentStep === 'UPLOAD'" class="space-y-6">
          <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
            <!-- Box 1: File Data Produk (Excel / CSV) -->
            <div class="border-2 border-dashed border-emerald-300 hover:border-emerald-500 bg-emerald-50/20 hover:bg-emerald-50/40 rounded-2xl p-6 transition flex flex-col items-center justify-center text-center space-y-3 relative group">
              <input
                type="file"
                accept=".xlsx, .xls, .csv"
                class="absolute inset-0 opacity-0 cursor-pointer w-full h-full z-10"
                @change="handleDataFileUpload"
              />
              <div class="w-14 h-14 rounded-2xl bg-emerald-100 text-emerald-700 flex items-center justify-center shadow-inner group-hover:scale-105 transition">
                <FileSpreadsheet class="w-7 h-7" />
              </div>
              <div>
                <h4 class="font-extrabold text-sm text-slate-900">
                  {{ dataFileName || 'Pilih File Data Excel / CSV' }}
                </h4>
                <p class="text-xs text-slate-500 mt-1 max-w-xs">
                  Seret atau klik untuk memilih file tabel data produk (.xlsx, .xls, .csv).
                </p>
              </div>
              <div v-if="rawRows.length > 0" class="text-xs font-mono font-bold text-emerald-700 bg-emerald-100 px-3 py-1 rounded-full border border-emerald-300 flex items-center gap-1.5">
                <CheckCircle2 class="w-4 h-4 text-emerald-600" />
                <span>{{ rawRows.length }} Baris Produk Terbaca</span>
              </div>
            </div>

            <!-- Box 2: Paket Foto Produk (ZIP atau Multiple Files) -->
            <div class="border-2 border-dashed border-purple-300 hover:border-purple-500 bg-purple-50/20 hover:bg-purple-50/40 rounded-2xl p-6 transition flex flex-col items-center justify-center text-center space-y-3 relative group">
              <div class="w-14 h-14 rounded-2xl bg-purple-100 text-purple-700 flex items-center justify-center shadow-inner group-hover:scale-105 transition">
                <ImageIcon class="w-7 h-7" />
              </div>
              <div>
                <h4 class="font-extrabold text-sm text-slate-900">
                  {{ photoArchiveName || 'Upload Paket Foto Produk' }}
                </h4>
                <p class="text-xs text-slate-500 mt-1 max-w-xs">
                  Upload file <strong>.ZIP</strong> kumpulan foto ATAU pilih <strong>banyak file foto</strong> langsung.
                </p>
              </div>

              <!-- Dua Pilihan Upload Foto -->
              <div class="flex items-center gap-3 pt-1">
                <label class="px-3 py-1.5 bg-purple-600 hover:bg-purple-700 text-white rounded-lg text-xs font-bold transition flex items-center gap-1.5 cursor-pointer shadow-xs active:scale-95">
                  <FileArchive class="w-4 h-4" />
                  <span>Upload .ZIP Foto</span>
                  <input
                    type="file"
                    accept=".zip"
                    class="hidden"
                    @change="handleZipArchiveUpload"
                  />
                </label>

                <label class="px-3 py-1.5 bg-white hover:bg-slate-50 text-slate-700 border border-slate-300 rounded-lg text-xs font-bold transition flex items-center gap-1.5 cursor-pointer shadow-xs active:scale-95">
                  <Upload class="w-4 h-4 text-slate-500" />
                  <span>Pilih File Foto</span>
                  <input
                    type="file"
                    accept="image/*"
                    multiple
                    class="hidden"
                    @change="handlePhotosBatchUpload"
                  />
                </label>
              </div>

              <div v-if="uploadedPhotosCount > 0" class="text-xs font-mono font-bold text-purple-800 bg-purple-100 px-3 py-1 rounded-full border border-purple-300 flex items-center gap-1.5">
                <CheckCircle2 class="w-4 h-4 text-purple-600" />
                <span>{{ uploadedPhotosCount }} Foto Siap Dicocokkan</span>
              </div>
            </div>
          </div>

          <!-- Petunjuk Format Data & Foto Produk -->
          <div class="bg-slate-50 border border-slate-200 rounded-2xl p-5 space-y-3">
            <h4 class="font-extrabold text-xs text-slate-800 uppercase tracking-wider flex items-center gap-2">
              <Icon icon="fluent-color:lightbulb-24" class="w-4 h-4" />
              <span>Panduan Pengisian Data & Foto:</span>
            </h4>
            <div class="grid grid-cols-1 md:grid-cols-3 gap-4 text-xs text-slate-600">
              <div class="p-3 bg-white rounded-xl border border-slate-200 space-y-1">
                <div class="font-bold text-slate-900 flex items-center gap-1.5">
                  <span class="w-5 h-5 rounded-md bg-blue-100 text-blue-800 flex items-center justify-center font-mono text-[11px] font-black">1</span>
                  <span>Deteksi Foto Otomatis</span>
                </div>
                <p class="text-[11px] leading-relaxed text-slate-500">
                  Nama file di kolom <code>Foto_File</code> (misal: <code>aqua.jpg</code>) atau nama foto yang sesuai dengan <strong>SKU</strong> / <strong>Barcode</strong> otomatis terhubung ke produk.
                </p>
              </div>

              <div class="p-3 bg-white rounded-xl border border-slate-200 space-y-1">
                <div class="font-bold text-slate-900 flex items-center gap-1.5">
                  <span class="w-5 h-5 rounded-md bg-emerald-100 text-emerald-800 flex items-center justify-center font-mono text-[11px] font-black">2</span>
                  <span>Harga Khusus per Outlet</span>
                </div>
                <p class="text-[11px] leading-relaxed text-slate-500">
                  Sertakan kolom <code>Harga_Jual_[KODE_OUTLET]</code> di template. Sistem akan otomatis memasukkan harga khusus cabang yang bersangkutan.
                </p>
              </div>

              <div class="p-3 bg-white rounded-xl border border-slate-200 space-y-1">
                <div class="font-bold text-slate-900 flex items-center gap-1.5">
                  <span class="w-5 h-5 rounded-md bg-purple-100 text-purple-800 flex items-center justify-center font-mono text-[11px] font-black">3</span>
                  <span>Harga Member & Grosir</span>
                </div>
                <p class="text-[11px] leading-relaxed text-slate-500">
                  Sertakan kolom <code>Harga_Member_[NAMA_GRUP]</code> di template. Kasir otomatis mengenakan harga khusus saat member bertransaksi.
                </p>
              </div>
            </div>
          </div>
        </div>

        <!-- ============================================== -->
        <!-- STEP 2: PRATINJAU & DETEKSI KECOCOKAN FOTO     -->
        <!-- ============================================== -->
        <div v-else-if="currentStep === 'PREVIEW'" class="space-y-4">
          <!-- Summary Cards -->
          <div class="grid grid-cols-2 md:grid-cols-4 gap-3">
            <div class="bg-slate-50 border border-slate-200 p-3 rounded-xl flex items-center gap-3">
              <div class="w-9 h-9 rounded-lg bg-blue-100 text-blue-700 flex items-center justify-center font-black">
                {{ parsedProducts.length }}
              </div>
              <div class="text-xs">
                <div class="font-bold text-slate-900">Total Baris</div>
                <div class="text-[10px] text-slate-500">Produk terbaca</div>
              </div>
            </div>

            <div class="bg-emerald-50/50 border border-emerald-200 p-3 rounded-xl flex items-center gap-3">
              <div class="w-9 h-9 rounded-lg bg-emerald-100 text-emerald-800 flex items-center justify-center font-black">
                {{ totalPhotosMatched }}
              </div>
              <div class="text-xs">
                <div class="font-bold text-emerald-900">Foto Cocok</div>
                <div class="text-[10px] text-emerald-600">Berhasil terhubung</div>
              </div>
            </div>

            <div class="bg-purple-50/50 border border-purple-200 p-3 rounded-xl flex items-center gap-3">
              <div class="w-9 h-9 rounded-lg bg-purple-100 text-purple-800 flex items-center justify-center font-black">
                {{ props.outlets?.length || 0 }}
              </div>
              <div class="text-xs">
                <div class="font-bold text-purple-900">Outlet Aktif</div>
                <div class="text-[10px] text-purple-600">Multi-cabang</div>
              </div>
            </div>

            <div class="bg-amber-50/50 border border-amber-200 p-3 rounded-xl flex items-center gap-3">
              <div class="w-9 h-9 rounded-lg bg-amber-100 text-amber-800 flex items-center justify-center font-black">
                {{ props.customerGroups?.length || 0 }}
              </div>
              <div class="text-xs">
                <div class="font-bold text-amber-900">Grup Member</div>
                <div class="text-[10px] text-amber-600">Penetapan harga</div>
              </div>
            </div>
          </div>

          <!-- Filter & Options Bar -->
          <div class="flex flex-wrap items-center justify-between gap-3 bg-slate-50 border border-slate-200 p-3 rounded-xl">
            <div class="relative flex-1 min-w-[200px] max-w-sm">
              <Search class="w-4 h-4 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2" />
              <input
                v-model="previewSearch"
                placeholder="Cari dalam pratinjau produk..."
                class="w-full bg-white border border-slate-300 rounded-lg pl-9 pr-3 py-1.5 text-xs text-slate-900 focus:outline-none focus:border-emerald-600"
              />
            </div>

            <div class="flex items-center gap-4 text-xs">
              <label class="flex items-center gap-2 cursor-pointer font-bold text-slate-700">
                <input
                  type="checkbox"
                  v-model="updateExisting"
                  class="rounded text-emerald-600 focus:ring-emerald-500 w-4 h-4"
                />
                <span>Perbarui jika Barcode/SKU sudah ada</span>
              </label>

              <div v-if="props.warehouses && props.warehouses.length > 0" class="flex items-center gap-1.5">
                <span class="text-slate-500 font-medium">Gudang Stok Awal:</span>
                <select
                  v-model="selectedWarehouseId"
                  class="bg-white border border-slate-300 rounded-md px-2 py-1 text-xs text-slate-800 focus:outline-none"
                >
                  <option value="">Pilih Lokasi Gudang (Opsional)</option>
                  <option v-for="w in props.warehouses" :key="w.id" :value="w.id">
                    {{ w.name }} ({{ w.code }})
                  </option>
                </select>
              </div>
            </div>
          </div>

          <!-- Preview Table -->
          <div class="border border-slate-200 rounded-xl overflow-hidden shadow-xs bg-white">
            <table class="w-full text-left text-xs">
              <thead class="bg-slate-100 border-b border-slate-200 text-slate-700 uppercase font-bold text-[10px]">
                <tr>
                  <th class="py-2.5 px-3 text-center">Foto</th>
                  <th class="py-2.5 px-3">SKU / Barcode</th>
                  <th class="py-2.5 px-3">Nama Produk</th>
                  <th class="py-2.5 px-3">Kategori</th>
                  <th class="py-2.5 px-3">Satuan</th>
                  <th class="py-2.5 px-3 text-right">Harga Jual</th>
                  <th class="py-2.5 px-3 text-center">Harga Outlet</th>
                  <th class="py-2.5 px-3 text-center">Harga Member</th>
                  <th class="py-2.5 px-3 text-center">Status</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-slate-100 font-mono text-xs">
                <tr
                  v-for="item in paginatedPreviewProducts"
                  :key="item._id"
                  class="hover:bg-slate-50 transition"
                >
                  <!-- Thumbnail Foto -->
                  <td class="py-2 px-3 text-center">
                    <div class="w-10 h-10 rounded-lg border border-slate-200 bg-slate-100 flex items-center justify-center mx-auto overflow-hidden shadow-2xs">
                      <img
                        v-if="item.image_url"
                        :src="item.image_url"
                        alt=""
                        class="w-full h-full object-cover"
                      />
                      <ImageIcon v-else class="w-4 h-4 text-slate-400" />
                    </div>
                  </td>

                  <!-- SKU / Barcode -->
                  <td class="py-2 px-3">
                    <div class="font-bold text-slate-900">{{ item.sku }}</div>
                    <div class="text-[10px] text-slate-500 font-normal">{{ item.barcode }}</div>
                  </td>

                  <!-- Nama Produk -->
                  <td class="py-2 px-3 font-sans">
                    <div class="font-bold text-slate-800 leading-snug">{{ item.name }}</div>
                    <div v-if="item.photoFileKey" class="text-[10px] text-slate-400 mt-0.5 flex items-center gap-1 font-mono">
                      <span>Ref: {{ item.photoFileKey }}</span>
                      <span v-if="item.image_url" class="text-emerald-600 font-bold">✓ Cocok</span>
                      <span v-else class="text-amber-600">✗ Tidak Ditemukan</span>
                    </div>
                  </td>

                  <!-- Kategori -->
                  <td class="py-2 px-3 font-sans text-slate-600">{{ item.category_name }}</td>

                  <!-- Satuan -->
                  <td class="py-2 px-3 text-slate-700 font-bold">{{ item.unit_code }}</td>

                  <!-- Harga Jual -->
                  <td class="py-2 px-3 text-right font-bold text-emerald-700">
                    {{ formatCurrency(item.base_selling_price, true) }}
                  </td>

                  <!-- Info Harga Outlet -->
                  <td class="py-2 px-3 text-center font-sans">
                    <span
                      v-if="item.outlet_prices.length > 0"
                      class="px-2 py-0.5 rounded-full bg-blue-100 text-blue-800 text-[10px] font-bold border border-blue-200"
                    >
                      {{ item.outlet_prices.length }} Outlet
                    </span>
                    <span v-else class="text-slate-400 text-[11px]">-</span>
                  </td>

                  <!-- Info Harga Member -->
                  <td class="py-2 px-3 text-center font-sans">
                    <span
                      v-if="item.member_prices.length > 0"
                      class="px-2 py-0.5 rounded-full bg-purple-100 text-purple-800 text-[10px] font-bold border border-purple-200"
                    >
                      {{ item.member_prices.length }} Grup
                    </span>
                    <span v-else class="text-slate-400 text-[11px]">-</span>
                  </td>

                  <!-- Validasi -->
                  <td class="py-2 px-3 text-center font-sans">
                    <span
                      v-if="item.isValid"
                      class="px-2 py-0.5 rounded-full bg-emerald-100 text-emerald-800 text-[10px] font-bold border border-emerald-300 flex items-center justify-center gap-1 mx-auto w-max"
                    >
                      <Check class="w-3 h-3" /> Siap
                    </span>
                    <span
                      v-else
                      class="px-2 py-0.5 rounded-full bg-rose-100 text-rose-800 text-[10px] font-bold border border-rose-300 flex items-center justify-center gap-1 mx-auto w-max"
                    >
                      <XCircle class="w-3 h-3" /> Kurang Lengkap
                    </span>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <!-- Pagination Preview -->
          <div class="flex items-center justify-between text-xs text-slate-500 px-1 font-mono">
            <div>
              Menampilkan {{ paginatedPreviewProducts.length }} dari {{ filteredPreviewProducts.length }} produk
            </div>
            <div class="flex items-center gap-1">
              <button
                type="button"
                @click="previewPage = Math.max(1, previewPage - 1)"
                :disabled="previewPage <= 1"
                class="px-2.5 py-1 rounded bg-slate-100 hover:bg-slate-200 disabled:opacity-40 border border-slate-300 font-bold"
              >
                Prev
              </button>
              <span class="px-2 font-bold">{{ previewPage }}</span>
              <button
                type="button"
                @click="previewPage++"
                :disabled="previewPage * previewPageSize >= filteredPreviewProducts.length"
                class="px-2.5 py-1 rounded bg-slate-100 hover:bg-slate-200 disabled:opacity-40 border border-slate-300 font-bold"
              >
                Next
              </button>
            </div>
          </div>
        </div>

        <!-- ============================================== -->
        <!-- STEP 3: HASIL IMPORT & LAPORAN                 -->
        <!-- ============================================== -->
        <div v-else-if="currentStep === 'RESULT' && importResult" class="space-y-6 text-center py-4">
          <div class="w-16 h-16 rounded-full bg-emerald-100 text-emerald-600 border border-emerald-300 flex items-center justify-center mx-auto shadow-lg shadow-emerald-600/10 animate-bounce">
            <CheckCircle2 class="w-9 h-9" />
          </div>

          <div>
            <h4 class="text-lg font-black text-slate-900">Proses Import Produk Berhasil Selesai!</h4>
            <p class="text-xs text-slate-500 mt-1 max-w-md mx-auto">
              Seluruh master produk, foto yang cocok, harga khusus outlet, dan harga member telah tersimpan ke dalam database.
            </p>
          </div>

          <!-- Statistik Rincian -->
          <div class="grid grid-cols-2 md:grid-cols-4 gap-4 max-w-2xl mx-auto text-left">
            <div class="bg-emerald-50 border border-emerald-200 p-4 rounded-xl space-y-1">
              <div class="text-[11px] font-bold text-emerald-800 uppercase">Produk Baru</div>
              <div class="text-2xl font-black text-emerald-900 font-mono">{{ importResult.inserted }}</div>
              <div class="text-[10px] text-emerald-600">Berhasil ditambahkan</div>
            </div>

            <div class="bg-blue-50 border border-blue-200 p-4 rounded-xl space-y-1">
              <div class="text-[11px] font-bold text-blue-800 uppercase">Produk Diperbarui</div>
              <div class="text-2xl font-black text-blue-900 font-mono">{{ importResult.updated }}</div>
              <div class="text-[10px] text-blue-600">Harga/stok/foto terupdate</div>
            </div>

            <div class="bg-purple-50 border border-purple-200 p-4 rounded-xl space-y-1">
              <div class="text-[11px] font-bold text-purple-800 uppercase">Foto Dihubungkan</div>
              <div class="text-2xl font-black text-purple-900 font-mono">{{ importResult.images_attached }}</div>
              <div class="text-[10px] text-purple-600">Terkoneksi ke produk</div>
            </div>

            <div class="bg-amber-50 border border-amber-200 p-4 rounded-xl space-y-1">
              <div class="text-[11px] font-bold text-amber-800 uppercase">Harga Khusus</div>
              <div class="text-2xl font-black text-amber-900 font-mono">
                {{ importResult.outlet_prices_saved + importResult.member_prices_saved }}
              </div>
              <div class="text-[10px] text-amber-600">Outlet & Member tersimpan</div>
            </div>
          </div>

          <!-- Catatan / Error Log jika ada -->
          <div v-if="importResult.errors && importResult.errors.length > 0" class="max-w-2xl mx-auto text-left bg-rose-50 border border-rose-200 rounded-xl p-4 text-xs space-y-2">
            <div class="font-bold text-rose-900 flex items-center gap-1.5">
              <AlertTriangle class="w-4 h-4 text-rose-600" />
              <span>Catatan / Baris yang Dilewati ({{ importResult.errors.length }}):</span>
            </div>
            <ul class="list-disc list-inside space-y-1 text-[11px] text-rose-700 max-h-32 overflow-y-auto font-mono">
              <li v-for="(err, i) in importResult.errors" :key="i">{{ err }}</li>
            </ul>
          </div>
        </div>
      </div>

      <!-- 4. FOOTER CONTROLS -->
      <div class="px-6 py-4 border-t border-slate-200 bg-slate-50 flex items-center justify-between">
        <button
          type="button"
          @click="handleClose"
          class="px-4 py-2 rounded-lg border border-slate-300 bg-white hover:bg-slate-100 text-slate-700 text-xs font-bold transition cursor-pointer shadow-2xs"
        >
          {{ currentStep === 'RESULT' ? 'Tutup' : 'Batal' }}
        </button>

        <div class="flex items-center gap-3">
          <!-- Step 1 Next Button -->
          <button
            v-if="currentStep === 'UPLOAD'"
            type="button"
            :disabled="parsedProducts.length === 0"
            @click="currentStep = 'PREVIEW'"
            class="px-5 py-2 bg-emerald-600 hover:bg-emerald-700 disabled:opacity-50 text-white rounded-lg text-xs font-bold transition flex items-center gap-2 shadow-sm border border-emerald-700 cursor-pointer active:scale-95"
          >
            <span>Lanjut ke Pratinjau ({{ parsedProducts.length }} Produk)</span>
            <ArrowRight class="w-4 h-4" />
          </button>

          <!-- Step 2 Controls -->
          <template v-else-if="currentStep === 'PREVIEW'">
            <button
              type="button"
              @click="currentStep = 'UPLOAD'"
              class="px-4 py-2 border border-slate-300 rounded-lg text-xs font-bold text-slate-700 hover:bg-slate-100 transition cursor-pointer"
            >
              Kembali ke Upload
            </button>

            <button
              type="button"
              :disabled="isSubmitting || parsedProducts.length === 0"
              @click="executeImport"
              class="px-6 py-2 bg-emerald-600 hover:bg-emerald-700 disabled:opacity-50 text-white rounded-lg text-xs font-bold transition flex items-center gap-2 shadow-sm border border-emerald-700 cursor-pointer active:scale-95"
            >
              <RefreshCw v-if="isSubmitting" class="w-4 h-4 animate-spin" />
              <CheckCircle2 v-else class="w-4 h-4" />
              <span>{{ isSubmitting ? 'Memproses Import...' : `Eksekusi Import (${parsedProducts.length} Produk)` }}</span>
            </button>
          </template>

          <!-- Step 3 Finish Button -->
          <button
            v-else-if="currentStep === 'RESULT'"
            type="button"
            @click="handleClose"
            class="px-6 py-2 bg-emerald-600 hover:bg-emerald-700 text-white rounded-lg text-xs font-bold transition flex items-center gap-2 shadow-sm border border-emerald-700 cursor-pointer"
          >
            <Check class="w-4 h-4" />
            <span>Selesai</span>
          </button>
        </div>
      </div>
    </div>
  </div>
  </Teleport>
</template>
