<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { useProviderStore, type AppRelease } from '../stores/providerStore'
import { usePermission } from '../composables/usePermission'
import PaginationBar from '../components/common/PaginationBar.vue'
import {
  Layers,
  Plus,
  Trash2,
  Download,
  ShieldCheck,
  Calendar,
  CheckCircle2,
  X,
  FileCode,
  Tag,
  Search,
  RefreshCw,
  ArrowUpDown,
  ArrowUp,
  ArrowDown,
  Database,
  AlertCircle,
  Upload,
  Globe,
  FileArchive,
} from 'lucide-vue-next'
import { z } from 'zod'

const providerStore = useProviderStore()
const { hasPermission } = usePermission()

// Server-side filter & sort
const searchQuery = ref('')
const sortBy = ref<'created_at' | 'version' | 'release_name' | 'release_date'>('created_at')
const sortOrder = ref<'asc' | 'desc'>('desc')

// Modal state
const isModalOpen = ref(false)
const isSubmitting = ref(false)
const formErrors = ref<Record<string, string>>({})

// Upload state
const uploadMode = ref<'file' | 'url'>('file')
const isUploading = ref(false)
const uploadProgress = ref(0)
const uploadedFileName = ref('')
const fileInput = ref<HTMLInputElement | null>(null)

const releaseSchema = z.object({
  version: z.string().trim().min(1, 'Versi wajib diisi (contoh: 1.2.0)'),
  release_name: z.string().trim().min(2, 'Nama rilis wajib diisi'),
  changelog: z.string().trim().min(5, 'Changelog minimal 5 karakter'),
  download_url: z.string().trim().min(1, 'URL atau berkas unduhan wajib disediakan'),
  checksum_sha256: z.string().trim().min(10, 'Checksum SHA-256 wajib diisi'),
  is_mandatory: z.boolean(),
  min_license_tier: z.string(),
  has_db_migration: z.boolean(),
  component: z.enum(['server', 'client']),
  min_server_version: z.string().optional(),
  file_path: z.string().optional(),
  file_size: z.number().optional(),
  original_filename: z.string().optional(),
})

const form = ref<{
  version: string
  release_name: string
  changelog: string
  download_url: string
  checksum_sha256: string
  is_mandatory: boolean
  min_license_tier: string
  has_db_migration: boolean
  component: 'server' | 'client'
  min_server_version: string
  file_path?: string
  file_size?: number
  original_filename?: string
}>({
  version: '',
  release_name: '',
  changelog: '',
  download_url: '',
  checksum_sha256: '',
  is_mandatory: false,
  min_license_tier: 'BASIC',
  has_db_migration: true,
  component: 'server',
  min_server_version: '1.1.0',
  file_path: undefined,
  file_size: undefined,
  original_filename: undefined,
})

async function loadData(page?: number, pageSize?: number) {
  await providerStore.fetchReleases({
    page: page ?? providerStore.releasesPagination.page,
    page_size: pageSize ?? providerStore.releasesPagination.page_size,
    search: searchQuery.value.trim() || undefined,
    sort_by: sortBy.value,
    sort_order: sortOrder.value,
  })
}

// Debounce search
let searchTimeout: any = null
watch(searchQuery, () => {
  clearTimeout(searchTimeout)
  searchTimeout = setTimeout(() => {
    loadData(1)
  }, 350)
})

onMounted(async () => {
  await loadData(1)
})

function handleSort(column: 'created_at' | 'version' | 'release_name' | 'release_date') {
  if (sortBy.value === column) {
    sortOrder.value = sortOrder.value === 'asc' ? 'desc' : 'asc'
  } else {
    sortBy.value = column
    sortOrder.value = 'asc'
  }
  loadData(1)
}

async function handleFileUpload(e: Event) {
  const target = e.target as HTMLInputElement
  if (!target.files || target.files.length === 0) return
  const file = target.files[0]
  isUploading.value = true
  uploadProgress.value = 0
  uploadedFileName.value = file.name

  const res = await providerStore.uploadReleaseFile(file, (pct) => {
    uploadProgress.value = pct
  })
  isUploading.value = false

  if (res) {
    form.value.file_path = res.file_path
    form.value.original_filename = res.original_filename
    form.value.file_size = res.file_size
    form.value.checksum_sha256 = res.checksum_sha256
    form.value.download_url = res.download_url
  }
}

function openCreateModal(comp: 'server' | 'client' = 'server') {
  uploadMode.value = 'file'
  uploadedFileName.value = ''
  uploadProgress.value = 0
  if (comp === 'client') {
    form.value = {
      version: '1.2.0',
      release_name: 'Notane Retail POS - Client Terminal UI Update',
      changelog: '1. Desain antarmuka baru kasir & POS responsif\n2. Indikator status real-time printer & barcode scanner\n3. Dukungan modal cek & perbarui antarmuka langsung',
      download_url: '',
      checksum_sha256: '',
      is_mandatory: false,
      min_license_tier: 'BASIC',
      has_db_migration: false,
      component: 'client',
      min_server_version: '1.1.0',
      file_path: undefined,
      file_size: undefined,
      original_filename: undefined,
    }
  } else {
    form.value = {
      version: '1.2.0',
      release_name: 'Pembaruan Notane Server v1.2.0',
      changelog: '1. Peningkatan kecepatan simpan transaksi kasir\n2. Pembaruan struktur data laporan penjualan\n3. Peningkatan kestabilan sambungan jaringan toko',
      download_url: '',
      checksum_sha256: '',
      is_mandatory: false,
      min_license_tier: 'BASIC',
      has_db_migration: true,
      component: 'server',
      min_server_version: '',
      file_path: undefined,
      file_size: undefined,
      original_filename: undefined,
    }
  }
  formErrors.value = {}
  isModalOpen.value = true
}

async function handleSubmit() {
  formErrors.value = {}
  const res = releaseSchema.safeParse(form.value)
  if (!res.success) {
    res.error.errors.forEach((err: any) => {
      formErrors.value[err.path[0] as string] = err.message
    })
    return
  }

  isSubmitting.value = true
  const ok = await providerStore.createRelease(res.data)
  isSubmitting.value = false
  if (ok) {
    isModalOpen.value = false
    loadData()
  }
}

async function confirmDelete(rel: AppRelease) {
  if (confirm(`Hapus versi rilis v${rel.version}? Klien tidak akan lagi menerima notifikasi pembaruan untuk versi ini.`)) {
    await providerStore.deleteRelease(rel.id)
  }
}
</script>

<template>
  <div class="space-y-5">
    <!-- Header Controls -->
    <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
      <div class="flex flex-wrap items-center gap-2 w-full sm:w-auto">
        <div class="relative w-full sm:w-72">
          <Search class="w-4 h-4 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2" />
          <input
            v-model="searchQuery"
            type="text"
            placeholder="Cari versi, nama rilis, changelog..."
            class="w-full pl-9 pr-4 py-2 bg-slate-900 border border-slate-800 rounded-xl text-xs text-slate-200 placeholder-slate-500 focus:outline-none focus:border-brand-500 transition shadow-inner"
          />
        </div>

        <button
          @click="loadData()"
          :disabled="providerStore.isLoading"
          title="Muat Ulang Data"
          class="p-2 bg-slate-900 border border-slate-800 hover:bg-slate-800 text-slate-300 rounded-xl transition cursor-pointer"
        >
          <RefreshCw class="w-4 h-4" :class="{ 'animate-spin': providerStore.isLoading }" />
        </button>
      </div>

      <div class="flex items-center gap-2">
        <button
          @click="openCreateModal('server')"
          :disabled="!hasPermission('release:create')"
          class="px-3.5 py-2 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 text-white rounded-xl text-xs font-semibold flex items-center gap-1.5 transition-all shadow-md shadow-indigo-600/20 cursor-pointer"
        >
          <Plus class="w-4 h-4" />
          <span>Rilis Server</span>
        </button>

        <button
          @click="openCreateModal('client')"
          :disabled="!hasPermission('release:create')"
          class="px-3.5 py-2 bg-cyan-600 hover:bg-cyan-500 disabled:opacity-50 text-white rounded-xl text-xs font-semibold flex items-center gap-1.5 transition-all shadow-md shadow-cyan-600/20 cursor-pointer"
        >
          <Plus class="w-4 h-4" />
          <span>Rilis Client (UI)</span>
        </button>
      </div>
    </div>

    <!-- Empty State -->
    <div
      v-if="!providerStore.isLoading && providerStore.releasesPagination.total_items === 0"
      class="py-16 text-center rounded-2xl glass-panel border border-slate-800"
    >
      <Layers class="w-12 h-12 text-slate-600 mx-auto mb-3" />
      <div class="text-sm font-semibold text-slate-300">Belum Ada Rilis Pembaruan Terdaftar</div>
      <div class="text-xs text-slate-500 mt-1">
        {{ searchQuery ? 'Tidak ada rilis yang cocok dengan pencarian Anda.' : 'Belum ada paket pembaruan yang diterbitkan untuk aplikasi toko.' }}
      </div>
      <div v-if="!searchQuery" class="mt-4 flex items-center justify-center gap-2">
        <button
          @click="openCreateModal('server')"
          class="px-4 py-2 bg-indigo-600 hover:bg-indigo-500 text-white rounded-xl text-xs font-semibold cursor-pointer"
        >
          Rilis Notane Server
        </button>
        <button
          @click="openCreateModal('client')"
          class="px-4 py-2 bg-cyan-600 hover:bg-cyan-500 text-white rounded-xl text-xs font-semibold cursor-pointer"
        >
          Rilis Notane Client
        </button>
      </div>
      <button
        v-else
        @click="searchQuery = ''; loadData(1)"
        class="mt-4 px-4 py-2 bg-slate-800 hover:bg-slate-700 text-white rounded-xl text-xs font-semibold cursor-pointer"
      >
        Reset Pencarian
      </button>
    </div>

    <!-- List of Releases -->
    <div v-else class="space-y-4">
      <div class="space-y-4">
        <div
          v-for="rel in providerStore.releasesPagination.items"
          :key="rel.id"
          class="p-5 rounded-2xl glass-panel border border-slate-800 hover:border-slate-700 transition-all space-y-4"
        >
          <!-- Release Header -->
          <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3 pb-3 border-b border-slate-800/80">
            <div class="flex items-center gap-3">
              <div
                class="px-3 py-1.5 rounded-xl font-mono text-sm font-bold border"
                :class="rel.component === 'client' ? 'bg-cyan-500/10 border-cyan-500/30 text-cyan-400' : 'bg-indigo-500/10 border-indigo-500/30 text-indigo-400'"
              >
                v{{ rel.version }}
              </div>
              <div>
                <div class="flex items-center gap-2">
                  <h3 class="font-bold text-sm text-white">{{ rel.release_name }}</h3>
                  <!-- Component Badge -->
                  <span
                    class="px-2 py-0.5 rounded-full text-[10px] font-semibold border font-mono"
                    :class="rel.component === 'client' ? 'bg-cyan-950/80 text-cyan-300 border-cyan-700/50' : 'bg-indigo-950/80 text-indigo-300 border-indigo-700/50'"
                  >
                    {{ rel.component === 'client' ? 'Client UI' : 'Server Core & UI' }}
                  </span>
                  <!-- Min Server Version (for client releases) -->
                  <span
                    v-if="rel.component === 'client' && rel.min_server_version"
                    class="px-2 py-0.5 rounded-full text-[10px] font-semibold bg-amber-950/60 text-amber-300 border border-amber-700/40 font-mono"
                  >
                    Min Server: v{{ rel.min_server_version }}
                  </span>
                </div>
                <div class="flex items-center gap-3 text-[11px] text-slate-400 mt-1">
                  <span class="flex items-center gap-1 font-mono">
                    <Calendar class="w-3 h-3 text-slate-500" />
                    {{ rel.release_date }}
                  </span>
                  <span>•</span>
                  <span class="text-brand-400 font-mono text-[10px]">Tier Min: {{ rel.min_license_tier }}</span>
                </div>
              </div>
            </div>

            <div class="flex items-center gap-2">
              <span
                v-if="rel.is_mandatory === 1"
                class="px-2.5 py-0.5 rounded-full text-[10px] font-bold bg-amber-500/20 text-amber-400 border border-amber-500/30 flex items-center gap-1"
              >
                <AlertCircle class="w-3 h-3" />
                <span>Wajib Update</span>
              </span>

              <span
                v-if="rel.has_db_migration === 1"
                class="px-2.5 py-0.5 rounded-full text-[10px] font-bold bg-blue-500/20 text-blue-400 border border-blue-500/30 flex items-center gap-1"
              >
                <Database class="w-3 h-3" />
                <span>Ada Migrasi DB</span>
              </span>

              <button
                @click="confirmDelete(rel)"
                :disabled="!hasPermission('release:delete')"
                class="p-1.5 text-slate-500 hover:text-rose-400 hover:bg-rose-500/10 rounded-lg transition-colors ml-2 cursor-pointer"
                title="Hapus Rilis"
              >
                <Trash2 class="w-4 h-4" />
              </button>
            </div>
          </div>

          <!-- Changelog -->
          <div class="space-y-1.5">
            <div class="text-[11px] font-semibold text-slate-400 uppercase tracking-wider">Catatan Perubahan (Changelog):</div>
            <pre class="p-3 rounded-xl bg-slate-950/80 border border-slate-900 text-xs text-slate-300 font-sans whitespace-pre-wrap leading-relaxed">{{ rel.changelog }}</pre>
          </div>

          <!-- Cryptographic Metadata -->
          <div class="grid grid-cols-1 md:grid-cols-2 gap-3 pt-2 text-xs">
            <!-- SHA-256 Checksum -->
            <div class="p-3 rounded-xl bg-slate-950/60 border border-slate-900 space-y-1">
              <div class="flex items-center justify-between text-[11px] text-slate-400">
                <span class="flex items-center gap-1">
                  <FileCode class="w-3.5 h-3.5 text-slate-500" />
                  <span>SHA-256 Checksum</span>
                </span>
                <span class="text-emerald-400 text-[10px] font-semibold">Integritas File</span>
              </div>
              <div class="font-mono text-[11px] text-slate-300 truncate select-all" :title="rel.checksum_sha256">
                {{ rel.checksum_sha256 }}
              </div>
            </div>

            <!-- Ed25519 Signature -->
            <div class="p-3 rounded-xl bg-slate-950/60 border border-slate-900 space-y-1">
              <div class="flex items-center justify-between text-[11px] text-slate-400">
                <span class="flex items-center gap-1">
                  <ShieldCheck class="w-3.5 h-3.5 text-emerald-400" />
                  <span>Tanda Tangan Keamanan</span>
                </span>
                <span class="text-brand-400 text-[10px] font-semibold">Terverifikasi</span>
              </div>
              <div class="font-mono text-[11px] text-emerald-300/90 truncate select-all" :title="rel.signature_ed25519">
                {{ rel.signature_ed25519 }}
              </div>
            </div>
          </div>

          <!-- Download URL & File Info -->
          <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between pt-1 gap-2 text-[11px] text-slate-400">
            <div class="flex flex-wrap items-center gap-2 font-mono">
              <span v-if="rel.original_filename" class="px-2 py-0.5 rounded-md bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 text-[10px] font-semibold flex items-center gap-1">
                <FileArchive class="w-3 h-3" />
                <span>{{ rel.original_filename }} ({{ ((rel.file_size || 0) / (1024 * 1024)).toFixed(2) }} MB)</span>
              </span>
              <span class="px-2 py-0.5 rounded-md bg-indigo-500/10 text-indigo-400 border border-indigo-500/20 text-[10px] font-semibold flex items-center gap-1">
                <ShieldCheck class="w-3 h-3" />
                <span>Tervalidasi Enkripsi Mesin & Lisensi</span>
              </span>
              <div class="flex items-center gap-1.5 truncate max-w-xs text-slate-500">
                <Download class="w-3.5 h-3.5 text-slate-500 shrink-0" />
                <span class="truncate">{{ rel.download_url }}</span>
              </div>
            </div>
            <a
              v-if="rel.download_url.startsWith('http://') || rel.download_url.startsWith('https://')"
              :href="rel.download_url"
              target="_blank"
              class="text-brand-400 hover:text-brand-300 font-semibold underline shrink-0 cursor-pointer"
            >
              Unduh Eksternal
            </a>
            <span v-else class="text-[11px] text-emerald-400/90 font-medium shrink-0 flex items-center gap-1">
              <CheckCircle2 class="w-3.5 h-3.5 text-emerald-400" />
              <span>Siap Diunduh Aman via Server POS</span>
            </span>
          </div>
        </div>
      </div>

      <!-- Pagination Component -->
      <PaginationBar
        :page="providerStore.releasesPagination.page"
        :page-size="providerStore.releasesPagination.page_size"
        :total-items="providerStore.releasesPagination.total_items"
        :total-pages="providerStore.releasesPagination.total_pages"
        :is-loading="providerStore.isLoading"
        @update:page="(p) => loadData(p, providerStore.releasesPagination.page_size)"
        @update:page-size="(s) => loadData(1, s)"
      />
    </div>

    <!-- Modal Create Release -->
    <div v-if="isModalOpen" class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/80 backdrop-blur-sm animate-in fade-in">
      <div class="w-full max-w-lg rounded-2xl glass-panel border border-slate-800 bg-slate-900 p-6 shadow-2xl space-y-4 max-h-[90vh] overflow-y-auto">
        <div class="flex items-center justify-between pb-3 border-b border-slate-800">
          <div class="font-bold text-sm text-white">
            Terbitkan Rilis OTA Baru:
            <span :class="form.component === 'client' ? 'text-cyan-400' : 'text-indigo-400'">
              {{ form.component === 'client' ? 'Notane Client (UI Only)' : 'Notane Server (Core & UI)' }}
            </span>
          </div>
          <button @click="isModalOpen = false" class="text-slate-400 hover:text-white cursor-pointer">
            <X class="w-4 h-4" />
          </button>
        </div>

        <form @submit.prevent="handleSubmit" class="space-y-3.5">
          <!-- Component Selector -->
          <div>
            <label class="block text-xs font-semibold text-slate-300 mb-1.5">Target Komponen Aplikasi *</label>
            <div class="grid grid-cols-2 gap-2">
              <button
                type="button"
                @click="form.component = 'server'; form.has_db_migration = true"
                class="px-3 py-2 rounded-xl border text-xs font-semibold transition cursor-pointer flex items-center justify-center gap-1.5"
                :class="form.component === 'server' ? 'bg-indigo-600/30 border-indigo-500 text-indigo-300' : 'bg-slate-950 border-slate-800 text-slate-400 hover:border-slate-700'"
              >
                <span>Notane Server</span>
              </button>
              <button
                type="button"
                @click="form.component = 'client'; form.has_db_migration = false"
                class="px-3 py-2 rounded-xl border text-xs font-semibold transition cursor-pointer flex items-center justify-center gap-1.5"
                :class="form.component === 'client' ? 'bg-cyan-600/30 border-cyan-500 text-cyan-300' : 'bg-slate-950 border-slate-800 text-slate-400 hover:border-slate-700'"
              >
                <span>Notane Client (UI)</span>
              </button>
            </div>
          </div>

          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="block text-xs font-semibold text-slate-300 mb-1">Nomor Versi *</label>
              <input
                v-model="form.version"
                type="text"
                placeholder="Contoh: 1.2.0"
                class="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-xl text-xs text-white focus:outline-none focus:border-brand-500 font-mono"
              />
              <span v-if="formErrors.version" class="text-[11px] text-rose-400 mt-0.5 block">{{ formErrors.version }}</span>
            </div>
            <div>
              <label class="block text-xs font-semibold text-slate-300 mb-1">Paket Lisensi Minimal</label>
              <select
                v-model="form.min_license_tier"
                class="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-xl text-xs text-white focus:outline-none focus:border-brand-500"
              >
                <option value="BASIC">Standar (Semua Toko)</option>
                <option value="PRO">Paket Pro Ke Atas</option>
                <option value="ENTERPRISE">Khusus Paket Lengkap</option>
              </select>
            </div>
          </div>

          <!-- Minimal Server Version Requirement for Client -->
          <div v-if="form.component === 'client'" class="p-3 rounded-xl bg-cyan-950/20 border border-cyan-800/40">
            <label class="block text-xs font-semibold text-cyan-300 mb-1">Syarat Minimal Versi Server Toko</label>
            <input
              v-model="form.min_server_version"
              type="text"
              placeholder="Contoh: 1.1.0"
              class="w-full px-3 py-2 bg-slate-950 border border-cyan-800/60 rounded-xl text-xs text-white focus:outline-none focus:border-cyan-400 font-mono"
            />
            <p class="text-[11px] text-slate-400 mt-1">
              Aplikasi kasir tidak dapat diperbarui sebelum server toko memenuhi versi ini.
            </p>
          </div>

          <div>
            <label class="block text-xs font-semibold text-slate-300 mb-1">Judul Pembaruan *</label>
            <input
              v-model="form.release_name"
              type="text"
              placeholder="Contoh: Pembaruan Tampilan Kasir & Cetak Struk"
              class="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-xl text-xs text-white focus:outline-none focus:border-brand-500"
            />
            <span v-if="formErrors.release_name" class="text-[11px] text-rose-400 mt-0.5 block">{{ formErrors.release_name }}</span>
          </div>

          <div>
            <label class="block text-xs font-semibold text-slate-300 mb-1">Catatan Pembaruan *</label>
            <textarea
              v-model="form.changelog"
              rows="4"
              placeholder="1. Peningkatan kemudahan kasir&#10;2. Perbaikan nota penjualan..."
              class="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-xl text-xs text-white focus:outline-none focus:border-brand-500 font-sans"
            ></textarea>
            <span v-if="formErrors.changelog" class="text-[11px] text-rose-400 mt-0.5 block">{{ formErrors.changelog }}</span>
          </div>

          <!-- Mode Penyediaan Berkas -->
          <div class="space-y-1.5">
            <label class="block text-xs font-semibold text-slate-300">Penyediaan Berkas Pembaruan *</label>
            <div class="grid grid-cols-2 gap-2">
              <button
                type="button"
                @click="uploadMode = 'file'"
                class="px-3 py-2 rounded-xl border text-xs font-semibold transition cursor-pointer flex items-center justify-center gap-1.5"
                :class="uploadMode === 'file' ? 'bg-indigo-600/30 border-indigo-500 text-indigo-300' : 'bg-slate-950 border-slate-800 text-slate-400 hover:border-slate-700'"
              >
                <Upload class="w-3.5 h-3.5" />
                <span>Unggah Berkas (Aman)</span>
              </button>
              <button
                type="button"
                @click="uploadMode = 'url'"
                class="px-3 py-2 rounded-xl border text-xs font-semibold transition cursor-pointer flex items-center justify-center gap-1.5"
                :class="uploadMode === 'url' ? 'bg-indigo-600/30 border-indigo-500 text-indigo-300' : 'bg-slate-950 border-slate-800 text-slate-400 hover:border-slate-700'"
              >
                <Globe class="w-3.5 h-3.5" />
                <span>Tautan URL Eksternal</span>
              </button>
            </div>
          </div>

          <!-- Upload Berkas Dropzone -->
          <div v-if="uploadMode === 'file'" class="space-y-2">
            <div
              @click="fileInput?.click()"
              class="border-2 border-dashed rounded-xl p-4 text-center cursor-pointer transition"
              :class="form.file_path ? 'border-emerald-600/60 bg-emerald-950/20' : 'border-slate-700 hover:border-brand-500 bg-slate-950/50'"
            >
              <input
                ref="fileInput"
                type="file"
                class="hidden"
                @change="handleFileUpload"
                accept=".zip,.tar.gz,.tar,.bin,.exe,.msi,.deb,.appimage,.dmg,.pkg"
              />
              <div v-if="!form.file_path && !isUploading" class="space-y-1">
                <Upload class="w-7 h-7 text-slate-400 mx-auto" />
                <div class="text-xs font-semibold text-slate-200">Klik untuk memilih berkas instalasi / update</div>
                <div class="text-[11px] text-slate-500">Mendukung berkas .zip, .exe, .msi, .deb, .dmg, .appimage, dll.</div>
              </div>
              <div v-else-if="isUploading" class="space-y-2 py-2">
                <RefreshCw class="w-6 h-6 text-brand-400 animate-spin mx-auto" />
                <div class="text-xs text-slate-300 font-semibold">Mengunggah {{ uploadedFileName }} ({{ uploadProgress }}%)...</div>
                <div class="w-full bg-slate-800 rounded-full h-1.5 overflow-hidden">
                  <div class="bg-brand-500 h-1.5 transition-all duration-200" :style="{ width: `${uploadProgress}%` }"></div>
                </div>
              </div>
              <div v-else class="space-y-1 py-1">
                <div class="flex items-center justify-center gap-1.5 text-emerald-400 font-bold text-xs">
                  <CheckCircle2 class="w-4 h-4" />
                  <span>Berkas Berhasil Terunggah</span>
                </div>
                <div class="text-xs text-slate-300 font-mono">{{ form.original_filename }}</div>
                <div class="text-[11px] text-slate-400">
                  Ukuran: {{ ((form.file_size || 0) / (1024 * 1024)).toFixed(2) }} MB | SHA-256 Otomatis Dihitung
                </div>
                <div class="pt-1">
                  <span class="text-[11px] text-brand-400 underline hover:text-brand-300">Klik untuk mengganti berkas</span>
                </div>
              </div>
            </div>
            <span v-if="formErrors.download_url" class="text-[11px] text-rose-400 mt-0.5 block">
              Silakan pilih dan unggah berkas rilis terlebih dahulu.
            </span>
          </div>

          <!-- Tautan Unduhan Manual (URL) -->
          <div v-else>
            <label class="block text-xs font-semibold text-slate-300 mb-1">Alamat / Tautan Unduhan File *</label>
            <input
              v-model="form.download_url"
              type="url"
              placeholder="https://update.notane.id/releases/v1.2.0/bundle.zip"
              class="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-xl text-xs text-white focus:outline-none focus:border-brand-500 font-mono"
            />
            <span v-if="formErrors.download_url" class="text-[11px] text-rose-400 mt-0.5 block">{{ formErrors.download_url }}</span>
          </div>

          <div>
            <div class="flex items-center justify-between mb-1">
              <label class="block text-xs font-semibold text-slate-300">Kode Keamanan File (SHA-256) *</label>
              <span v-if="uploadMode === 'file' && form.checksum_sha256" class="text-[10px] text-emerald-400 font-mono flex items-center gap-1">
                <CheckCircle2 class="w-3 h-3" />
                Dihitung Otomatis
              </span>
            </div>
            <input
              v-model="form.checksum_sha256"
              :readonly="uploadMode === 'file' && !!form.checksum_sha256"
              type="text"
              placeholder="e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
              class="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-xl text-xs text-white focus:outline-none focus:border-brand-500 font-mono text-[11px]"
              :class="{ 'opacity-70 bg-slate-950/80': uploadMode === 'file' && form.checksum_sha256 }"
            />
            <span v-if="formErrors.checksum_sha256" class="text-[11px] text-rose-400 mt-0.5 block">{{ formErrors.checksum_sha256 }}</span>
          </div>

          <!-- Checkboxes -->
          <div class="grid grid-cols-2 gap-3 pt-1">
            <label class="flex items-center gap-2 text-xs text-slate-300 cursor-pointer">
              <input
                v-model="form.is_mandatory"
                type="checkbox"
                class="rounded border-slate-700 bg-slate-950 text-brand-600 focus:ring-0"
              />
              <span>Rilis Wajib Diperbarui</span>
            </label>

            <label class="flex items-center gap-2 text-xs text-slate-300 cursor-pointer">
              <input
                v-model="form.has_db_migration"
                type="checkbox"
                class="rounded border-slate-700 bg-slate-950 text-brand-600 focus:ring-0"
              />
              <span>Mengandung Migrasi Database</span>
            </label>
          </div>

          <div class="pt-3 border-t border-slate-800 flex justify-end gap-2">
            <button
              type="button"
              @click="isModalOpen = false"
              class="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-300 rounded-xl text-xs font-semibold cursor-pointer"
            >
              Batal
            </button>
            <button
              type="submit"
              :disabled="isSubmitting"
              class="px-4 py-2 bg-brand-600 hover:bg-brand-500 disabled:opacity-50 text-white rounded-xl text-xs font-semibold cursor-pointer"
            >
              {{ isSubmitting ? 'Menerbitkan...' : 'Terbitkan Pembaruan' }}
            </button>
          </div>
        </form>
      </div>
    </div>
  </div>
</template>
