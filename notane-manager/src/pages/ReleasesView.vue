<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { useProviderStore, type AppRelease, type ReleaseAssetDto } from '../stores/providerStore'
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
  Monitor,
  Terminal,
  Apple,
  Cpu,
  Check,
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

export interface ServerPlatformSlot {
  id: string // 'windows-x64' | 'windows-x86' | 'linux-x64' | 'macos-arm64' | 'macos-x64'
  label: string
  badgeLabel: string
  os: 'windows' | 'linux' | 'macos'
  arch: 'x86_64' | 'x86' | 'aarch64'
  osType: 'windows' | 'linux' | 'apple'
  description: string
  file_path?: string
  original_filename?: string
  file_size?: number
  checksum_sha256?: string
  download_url?: string
  isUploading?: boolean
  uploadProgress?: number
  uploadMode?: 'file' | 'url'
}

function getDefaultServerPlatforms(): ServerPlatformSlot[] {
  return [
    {
      id: 'windows-x64',
      label: 'Windows 64-bit (x64)',
      badgeLabel: 'Win x64',
      os: 'windows',
      arch: 'x86_64',
      osType: 'windows',
      description: 'Format .zip, .exe, atau .msi (Target: x86_64-pc-windows-msvc)',
      uploadMode: 'file',
    },
    {
      id: 'windows-x86',
      label: 'Windows 32-bit (x86)',
      badgeLabel: 'Win x86',
      os: 'windows',
      arch: 'x86',
      osType: 'windows',
      description: 'Format .zip, .exe, atau .msi (Target: i686-pc-windows-msvc)',
      uploadMode: 'file',
    },
    {
      id: 'linux-x64',
      label: 'Linux 64-bit (x64)',
      badgeLabel: 'Linux x64',
      os: 'linux',
      arch: 'x86_64',
      osType: 'linux',
      description: 'Format .zip, .deb, atau .AppImage (Target: x86_64-unknown-linux-gnu)',
      uploadMode: 'file',
    },
    {
      id: 'macos-arm64',
      label: 'macOS Apple Silicon (M-Series)',
      badgeLabel: 'macOS ARM',
      os: 'macos',
      arch: 'aarch64',
      osType: 'apple',
      description: 'Format .zip atau .dmg (Target: aarch64-apple-darwin)',
      uploadMode: 'file',
    },
    {
      id: 'macos-x64',
      label: 'macOS Intel (x64)',
      badgeLabel: 'macOS Intel',
      os: 'macos',
      arch: 'x86_64',
      osType: 'apple',
      description: 'Format .zip atau .dmg (Target: x86_64-apple-darwin)',
      uploadMode: 'file',
    },
  ]
}

const serverPlatforms = ref<ServerPlatformSlot[]>(getDefaultServerPlatforms())

// Single client file state
const clientFile = ref<{
  file_path?: string
  original_filename?: string
  file_size?: number
  checksum_sha256?: string
  download_url?: string
  isUploading?: boolean
  uploadProgress?: number
  uploadMode?: 'file' | 'url'
}>({
  uploadMode: 'file',
})

const releaseSchema = z.object({
  version: z.string().trim().min(1, 'Versi wajib diisi (contoh: 1.2.0)'),
  release_name: z.string().trim().min(2, 'Nama rilis wajib diisi'),
  changelog: z.string().trim().min(5, 'Changelog minimal 5 karakter'),
  is_mandatory: z.boolean(),
  min_license_tier: z.string(),
  has_db_migration: z.boolean(),
  component: z.enum(['server', 'client']),
  min_server_version: z.string().optional(),
})

const form = ref<{
  version: string
  release_name: string
  changelog: string
  is_mandatory: boolean
  min_license_tier: string
  has_db_migration: boolean
  component: 'server' | 'client'
  min_server_version: string
}>({
  version: '',
  release_name: '',
  changelog: '',
  is_mandatory: false,
  min_license_tier: 'BASIC',
  has_db_migration: true,
  component: 'server',
  min_server_version: '1.1.0',
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

function detectPlatformForFile(filename: string): string | null {
  const f = filename.toLowerCase()
  if (f.includes('windows-x86') || (f.includes('windows') && (f.includes('x86') || f.includes('i686') || f.includes('32')))) {
    return 'windows-x86'
  }
  if (f.includes('windows-x64') || (f.includes('windows') && (f.includes('x64') || f.includes('amd64'))) || f.endsWith('.msi') || f.endsWith('.exe')) {
    return 'windows-x64'
  }
  if (f.includes('macos-arm64') || (f.includes('macos') && (f.includes('arm64') || f.includes('aarch64')))) {
    return 'macos-arm64'
  }
  if (f.includes('macos-x64') || (f.includes('macos') && f.includes('x64')) || (f.includes('macos') && f.endsWith('.dmg'))) {
    return 'macos-x64'
  }
  if (f.includes('linux') || f.endsWith('.deb') || f.endsWith('.appimage')) {
    return 'linux-x64'
  }
  return null
}

async function handleMultiServerUpload(e: Event) {
  const target = e.target as HTMLInputElement
  if (!target.files || target.files.length === 0) return
  const files = Array.from(target.files)

  for (const file of files) {
    const matchedSlotId = detectPlatformForFile(file.name)
    const slot = serverPlatforms.value.find((p) => p.id === matchedSlotId) || serverPlatforms.value[0]
    if (slot) {
      slot.isUploading = true
      slot.uploadProgress = 0
      const res = await providerStore.uploadReleaseFile(file, (pct) => {
        slot.uploadProgress = pct
      })
      slot.isUploading = false
      if (res) {
        slot.file_path = res.file_path
        slot.original_filename = res.original_filename
        slot.file_size = res.file_size
        slot.checksum_sha256 = res.checksum_sha256
        slot.download_url = res.download_url
      }
    }
  }
  target.value = ''
}

async function handleSlotUpload(slot: ServerPlatformSlot, e: Event) {
  const target = e.target as HTMLInputElement
  if (!target.files || target.files.length === 0) return
  const file = target.files[0]

  slot.isUploading = true
  slot.uploadProgress = 0
  const res = await providerStore.uploadReleaseFile(file, (pct) => {
    slot.uploadProgress = pct
  })
  slot.isUploading = false
  if (res) {
    slot.file_path = res.file_path
    slot.original_filename = res.original_filename
    slot.file_size = res.file_size
    slot.checksum_sha256 = res.checksum_sha256
    slot.download_url = res.download_url
  }
  target.value = ''
}

async function handleClientUpload(e: Event) {
  const target = e.target as HTMLInputElement
  if (!target.files || target.files.length === 0) return
  const file = target.files[0]

  clientFile.value.isUploading = true
  clientFile.value.uploadProgress = 0
  const res = await providerStore.uploadReleaseFile(file, (pct) => {
    clientFile.value.uploadProgress = pct
  })
  clientFile.value.isUploading = false
  if (res) {
    clientFile.value.file_path = res.file_path
    clientFile.value.original_filename = res.original_filename
    clientFile.value.file_size = res.file_size
    clientFile.value.checksum_sha256 = res.checksum_sha256
    clientFile.value.download_url = res.download_url
  }
  target.value = ''
}

function parseVersion(v: string): [number, number, number] {
  const clean = (v || '').trim().replace(/^v/i, '')
  const parts = clean.split('.').map((p) => parseInt(p.split('-')[0], 10) || 0)
  return [parts[0] || 0, parts[1] || 0, parts[2] || 0]
}

function openCreateModal(comp: 'server' | 'client' = 'server') {
  serverPlatforms.value = getDefaultServerPlatforms()
  clientFile.value = { uploadMode: 'file' }

  const compReleases = (providerStore.releases || []).filter((r: any) => (r.component || 'server') === comp)
  let nextVer = '1.5.17'
  if (compReleases.length > 0) {
    const sorted = [...compReleases].sort((a: any, b: any) => {
      const ta = parseVersion(a.version)
      const tb = parseVersion(b.version)
      return (tb[0] - ta[0]) || (tb[1] - ta[1]) || (tb[2] - ta[2])
    })
    const [maj, min, pat] = parseVersion(sorted[0].version)
    nextVer = `${maj}.${min}.${pat + 1}`
  }

  if (comp === 'client') {
    form.value = {
      version: nextVer,
      release_name: `Notane Retail POS - Client Terminal UI Update v${nextVer}`,
      changelog: '1. Desain antarmuka baru kasir & POS responsif\n2. Indikator status real-time printer & barcode scanner\n3. Pembaruan performa dan stabilitas klien kasir',
      is_mandatory: false,
      min_license_tier: 'BASIC',
      has_db_migration: false,
      component: 'client',
      min_server_version: '',
    }
  } else {
    form.value = {
      version: nextVer,
      release_name: `Pembaruan Notane Server v${nextVer}`,
      changelog: '1. Peningkatan kecepatan simpan transaksi kasir\n2. Pembaruan struktur data laporan penjualan\n3. Peningkatan kestabilan sambungan jaringan toko',
      is_mandatory: false,
      min_license_tier: 'BASIC',
      has_db_migration: false,
      component: 'server',
      min_server_version: '',
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

  let assets: ReleaseAssetDto[] = []

  if (form.value.component === 'server') {
    const filled = serverPlatforms.value.filter((p) => !!p.checksum_sha256 && !!(p.file_path || p.download_url))
    if (filled.length === 0) {
      formErrors.value.platforms = 'Minimal satu berkas OS server (misal Windows, Linux, atau macOS) wajib diunggah atau diisi URL-nya!'
      return
    }
    assets = filled.map((p) => ({
      platform: p.id,
      os: p.os,
      arch: p.arch,
      download_url: p.download_url || (p.file_path ? `/api/v1/updates/download/${p.id}` : ''),
      checksum_sha256: p.checksum_sha256 || '',
      file_path: p.file_path,
      file_size: p.file_size,
      original_filename: p.original_filename,
    }))
  } else {
    if (!clientFile.value.checksum_sha256 || (!clientFile.value.file_path && !clientFile.value.download_url)) {
      formErrors.value.client_file = 'Berkas UI Klien (.zip) wajib diunggah terlebih dahulu!'
      return
    }
    assets = [
      {
        platform: 'universal',
        os: 'all',
        arch: 'all',
        download_url: clientFile.value.download_url || '',
        checksum_sha256: clientFile.value.checksum_sha256 || '',
        file_path: clientFile.value.file_path,
        file_size: clientFile.value.file_size,
        original_filename: clientFile.value.original_filename,
      },
    ]
  }

  isSubmitting.value = true
  const ok = await providerStore.createRelease({
    ...res.data,
    assets,
    download_url: assets[0]?.download_url,
    checksum_sha256: assets[0]?.checksum_sha256,
    file_path: assets[0]?.file_path,
    file_size: assets[0]?.file_size,
    original_filename: assets[0]?.original_filename,
  })
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

function formatPlatformLabel(platform: string): string {
  switch (platform) {
    case 'windows-x64':
      return 'Windows 64-bit (x64)'
    case 'windows-x86':
      return 'Windows 32-bit (x86)'
    case 'linux-x64':
      return 'Linux 64-bit (x64)'
    case 'macos-arm64':
      return 'macOS Apple Silicon'
    case 'macos-x64':
      return 'macOS Intel (x64)'
    case 'universal':
      return 'Universal (Semua OS)'
    default:
      return platform
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
            class="w-full pl-9 pr-4 py-2 bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-xl text-xs text-slate-800 dark:text-slate-200 placeholder-slate-400 dark:placeholder-slate-500 focus:outline-none focus:border-brand-500 transition shadow-xs"
          />
        </div>

        <button
          @click="loadData()"
          :disabled="providerStore.isLoading"
          title="Muat Ulang Data"
          class="p-2 bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 hover:bg-slate-100 dark:hover:bg-slate-800 text-slate-600 dark:text-slate-300 rounded-xl transition cursor-pointer"
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
          <span>Rilis Server (Multi-OS)</span>
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

    <!-- Releases List -->
    <div class="space-y-4">
      <div v-if="providerStore.isLoading && providerStore.releasesPagination.items.length === 0" class="p-12 text-center text-slate-500 text-sm">
        <RefreshCw class="w-6 h-6 animate-spin mx-auto mb-2 text-brand-500" />
        Memuat daftar rilis...
      </div>

      <div v-else-if="providerStore.releasesPagination.items.length === 0" class="p-12 text-center text-slate-500 text-sm glass-panel border border-slate-800 rounded-2xl">
        <Layers class="w-8 h-8 text-slate-600 mx-auto mb-2" />
        Belum ada paket rilis pembaruan yang diterbitkan.
      </div>

      <div v-else class="space-y-4">
        <!-- Sort Ribbon -->
        <div class="flex items-center justify-between text-xs text-slate-400 px-2">
          <span>Total: {{ providerStore.releasesPagination.total_items }} versi rilis</span>
          <div class="flex items-center gap-3">
            <span>Urutkan:</span>
            <button
              @click="handleSort('created_at')"
              class="flex items-center gap-1 hover:text-white transition"
              :class="{ 'text-brand-400 font-bold': sortBy === 'created_at' }"
            >
              <span>Waktu Dibuat</span>
              <ArrowUp v-if="sortBy === 'created_at' && sortOrder === 'asc'" class="w-3 h-3" />
              <ArrowDown v-else-if="sortBy === 'created_at' && sortOrder === 'desc'" class="w-3 h-3" />
              <ArrowUpDown v-else class="w-3 h-3 text-slate-600" />
            </button>
            <button
              @click="handleSort('version')"
              class="flex items-center gap-1 hover:text-white transition"
              :class="{ 'text-brand-400 font-bold': sortBy === 'version' }"
            >
              <span>Versi</span>
              <ArrowUp v-if="sortBy === 'version' && sortOrder === 'asc'" class="w-3 h-3" />
              <ArrowDown v-else-if="sortBy === 'version' && sortOrder === 'desc'" class="w-3 h-3" />
              <ArrowUpDown v-else class="w-3 h-3 text-slate-600" />
            </button>
          </div>
        </div>

        <div
          v-for="rel in providerStore.releasesPagination.items"
          :key="rel.id"
          class="p-5 rounded-2xl glass-panel border border-slate-200 dark:border-slate-800 hover:border-slate-300 dark:hover:border-slate-700 transition-all space-y-4"
        >
          <!-- Release Header -->
          <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-3 pb-3 border-b border-slate-200 dark:border-slate-800/80">
            <div class="flex items-center gap-3">
              <div
                class="px-3 py-1.5 rounded-xl font-mono text-sm font-bold border"
                :class="rel.component === 'client' ? 'bg-cyan-500/10 border-cyan-500/30 text-cyan-500 dark:text-cyan-400' : 'bg-indigo-500/10 border-indigo-500/30 text-indigo-600 dark:text-indigo-400'"
              >
                v{{ rel.version }}
              </div>
              <div>
                <div class="flex items-center gap-2">
                  <h3 class="font-bold text-sm text-slate-900 dark:text-white">{{ rel.release_name }}</h3>
                  <!-- Component Badge -->
                  <span
                    class="px-2 py-0.5 rounded-full text-[10px] font-semibold border font-mono"
                    :class="rel.component === 'client' ? 'bg-cyan-50 dark:bg-cyan-950/80 text-cyan-700 dark:text-cyan-300 border-cyan-200 dark:border-cyan-700/50' : 'bg-indigo-50 dark:bg-indigo-950/80 text-indigo-700 dark:text-indigo-300 border-indigo-200 dark:border-indigo-700/50'"
                  >
                    {{ rel.component === 'client' ? 'Client UI' : 'Server Core & UI' }}
                  </span>
                  <!-- Min Server Version (for client releases) -->
                  <span
                    v-if="rel.component === 'client' && rel.min_server_version"
                    class="px-2 py-0.5 rounded-full text-[10px] font-semibold bg-amber-50 dark:bg-amber-950/60 text-amber-700 dark:text-amber-300 border border-amber-200 dark:border-amber-700/40 font-mono"
                  >
                    Min Server: v{{ rel.min_server_version }}
                  </span>
                </div>
                <div class="flex items-center gap-3 text-[11px] text-slate-500 dark:text-slate-400 mt-1">
                  <span class="flex items-center gap-1 font-mono">
                    <Calendar class="w-3 h-3 text-slate-400 dark:text-slate-500" />
                    {{ rel.release_date }}
                  </span>
                  <span>•</span>
                  <span class="text-brand-600 dark:text-brand-400 font-mono text-[10px]">Tier Min: {{ rel.min_license_tier }}</span>
                </div>
              </div>
            </div>

            <div class="flex items-center gap-2">
              <span
                v-if="rel.is_mandatory === 1"
                class="px-2.5 py-0.5 rounded-full text-[10px] font-bold bg-amber-500/20 text-amber-500 dark:text-amber-400 border border-amber-500/30 flex items-center gap-1"
              >
                <AlertCircle class="w-3 h-3" />
                <span>Wajib Update</span>
              </span>

              <span
                v-if="rel.has_db_migration === 1"
                class="px-2.5 py-0.5 rounded-full text-[10px] font-bold bg-blue-500/20 text-blue-500 dark:text-blue-400 border border-blue-500/30 flex items-center gap-1"
              >
                <Database class="w-3 h-3" />
                <span>Ada Migrasi DB</span>
              </span>

              <button
                @click="confirmDelete(rel)"
                :disabled="!hasPermission('release:delete')"
                class="p-1.5 text-slate-400 hover:text-rose-600 dark:text-slate-500 dark:hover:text-rose-400 hover:bg-rose-50 dark:hover:bg-rose-500/10 rounded-lg transition-colors ml-2 cursor-pointer"
                title="Hapus Rilis"
              >
                <Trash2 class="w-4 h-4" />
              </button>
            </div>
          </div>

          <!-- Changelog -->
          <div class="space-y-1.5">
            <div class="text-[11px] font-semibold text-slate-500 dark:text-slate-400 uppercase tracking-wider">Catatan Perubahan (Changelog):</div>
            <pre class="p-3 rounded-xl bg-slate-50 dark:bg-slate-950/80 border border-slate-200 dark:border-slate-900 text-xs text-slate-800 dark:text-slate-300 font-sans whitespace-pre-wrap leading-relaxed shadow-xs">{{ rel.changelog }}</pre>
          </div>

          <!-- Multi-Platform Binaries Display (for Server releases) -->
          <div v-if="rel.component === 'server' && rel.assets && rel.assets.length > 0" class="space-y-2 pt-1">
            <div class="text-[11px] font-semibold text-slate-500 dark:text-slate-400 uppercase tracking-wider flex items-center justify-between">
              <span class="flex items-center gap-1.5">
                <Cpu class="w-3.5 h-3.5 text-indigo-500 dark:text-indigo-400" />
                <span>Paket Platform OS Tersedia ({{ rel.assets.length }} Binary):</span>
              </span>
              <span class="text-[10px] text-emerald-600 dark:text-emerald-400 font-mono">Dideteksi Otomatis Sesuai OS Klien</span>
            </div>
            <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-2">
              <div
                v-for="asset in rel.assets"
                :key="asset.id"
                class="p-2.5 rounded-xl bg-slate-50 dark:bg-slate-950/80 border border-slate-200 dark:border-slate-800/90 hover:border-slate-300 dark:hover:border-slate-700 flex items-start justify-between gap-2 text-xs transition shadow-xs"
              >
                <div class="flex items-start gap-2 min-w-0">
                  <Monitor v-if="asset.os === 'windows'" class="w-4 h-4 text-blue-500 dark:text-blue-400 shrink-0 mt-0.5" />
                  <Terminal v-else-if="asset.os === 'linux'" class="w-4 h-4 text-amber-500 dark:text-amber-400 shrink-0 mt-0.5" />
                  <Apple v-else-if="asset.os === 'macos'" class="w-4 h-4 text-slate-700 dark:text-slate-200 shrink-0 mt-0.5" />
                  <Cpu v-else class="w-4 h-4 text-brand-600 dark:text-brand-400 shrink-0 mt-0.5" />
                  <div class="min-w-0">
                    <div class="font-bold text-slate-800 dark:text-slate-200 flex items-center gap-1.5">
                      <span>{{ formatPlatformLabel(asset.platform) }}</span>
                      <span v-if="asset.file_size" class="text-[10px] text-slate-500 dark:text-slate-400 font-mono font-normal">({{ ((asset.file_size || 0) / (1024 * 1024)).toFixed(1) }} MB)</span>
                    </div>
                    <div class="text-[10px] text-slate-500 dark:text-slate-400 font-mono truncate" :title="asset.original_filename || asset.download_url">
                      {{ asset.original_filename || asset.download_url }}
                    </div>
                    <div class="text-[9px] text-slate-400 dark:text-slate-500 font-mono truncate select-all" :title="asset.checksum_sha256">
                      SHA: {{ asset.checksum_sha256.slice(0, 16) }}...
                    </div>
                  </div>
                </div>
                <a
                  v-if="asset.download_url.startsWith('http://') || asset.download_url.startsWith('https://')"
                  :href="asset.download_url"
                  target="_blank"
                  class="p-1 rounded bg-slate-100 hover:bg-slate-200 text-slate-700 dark:bg-slate-800 dark:text-slate-300 dark:hover:text-white dark:hover:bg-slate-700 shrink-0 cursor-pointer"
                  title="Unduh Berkas"
                >
                  <Download class="w-3.5 h-3.5" />
                </a>
                <span v-else class="text-[9px] font-semibold text-emerald-600 dark:text-emerald-400 bg-emerald-50 dark:bg-emerald-500/10 px-1.5 py-0.5 rounded border border-emerald-200 dark:border-emerald-500/20 shrink-0">
                  Lokal
                </span>
              </div>
            </div>
          </div>

          <!-- Cryptographic Metadata -->
          <div class="grid grid-cols-1 md:grid-cols-2 gap-3 pt-2 text-xs">
            <!-- SHA-256 Checksum -->
            <div class="p-3 rounded-xl bg-slate-50 dark:bg-slate-950/60 border border-slate-200 dark:border-slate-900 space-y-1 shadow-xs">
              <div class="flex items-center justify-between text-[11px] text-slate-500 dark:text-slate-400">
                <span class="flex items-center gap-1">
                  <FileCode class="w-3.5 h-3.5 text-slate-400 dark:text-slate-500" />
                  <span>SHA-256 Checksum</span>
                </span>
                <span class="text-emerald-600 dark:text-emerald-400 text-[10px] font-semibold">Integritas File</span>
              </div>
              <div class="font-mono text-[11px] text-slate-700 dark:text-slate-300 truncate select-all" :title="rel.checksum_sha256">
                {{ rel.checksum_sha256 }}
              </div>
            </div>

            <!-- Ed25519 Signature -->
            <div class="p-3 rounded-xl bg-slate-50 dark:bg-slate-950/60 border border-slate-200 dark:border-slate-900 space-y-1 shadow-xs">
              <div class="flex items-center justify-between text-[11px] text-slate-500 dark:text-slate-400">
                <span class="flex items-center gap-1">
                  <ShieldCheck class="w-3.5 h-3.5 text-emerald-500 dark:text-emerald-400" />
                  <span>Tanda Tangan Keamanan</span>
                </span>
                <span class="text-brand-600 dark:text-brand-400 text-[10px] font-semibold">Terverifikasi</span>
              </div>
              <div class="font-mono text-[11px] text-emerald-700 dark:text-emerald-300/90 truncate select-all" :title="rel.signature_ed25519">
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
                <span>Tervalidasi Enkripsi Mesin, Lisensi, & Arsitektur OS</span>
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
    <div v-if="isModalOpen" class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/60 backdrop-blur-sm animate-in fade-in">
      <div class="w-full max-w-3xl rounded-2xl glass-panel border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900 p-6 shadow-2xl space-y-4 max-h-[92vh] overflow-y-auto">
        <div class="flex items-center justify-between pb-3 border-b border-slate-200 dark:border-slate-800">
          <div class="font-bold text-sm text-slate-900 dark:text-white flex items-center gap-2">
            <span>Terbitkan Rilis OTA Baru:</span>
            <span class="px-2.5 py-0.5 rounded-full text-xs font-mono font-bold" :class="form.component === 'client' ? 'bg-cyan-50 text-cyan-700 border border-cyan-200 dark:bg-cyan-500/20 dark:text-cyan-300 dark:border-cyan-500/30' : 'bg-indigo-50 text-indigo-700 border border-indigo-200 dark:bg-indigo-500/20 dark:text-indigo-300 dark:border-indigo-500/30'">
              {{ form.component === 'client' ? 'Notane Client (Universal UI)' : 'Notane Server (Multi-Platform OS)' }}
            </span>
          </div>
          <button @click="isModalOpen = false" class="text-slate-400 hover:text-slate-700 dark:hover:text-white cursor-pointer">
            <X class="w-5 h-5" />
          </button>
        </div>

        <form @submit.prevent="handleSubmit" class="space-y-4">
          <!-- Component Selector -->
          <div>
            <label class="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1.5">Target Komponen Aplikasi *</label>
            <div class="grid grid-cols-2 gap-2">
              <button
                type="button"
                @click="form.component = 'server'; form.has_db_migration = true"
                class="px-3 py-2 rounded-xl border text-xs font-semibold transition cursor-pointer flex items-center justify-center gap-2"
                :class="form.component === 'server' ? 'bg-indigo-50 border-indigo-500 text-indigo-700 dark:bg-indigo-600/30 dark:border-indigo-500 dark:text-indigo-300 ring-2 ring-indigo-500/20' : 'bg-slate-50 dark:bg-slate-950 border-slate-200 dark:border-slate-800 text-slate-600 dark:text-slate-400 hover:border-slate-300 dark:hover:border-slate-700'"
              >
                <Cpu class="w-4 h-4" />
                <span>Notane Server (Multi-OS Binaries)</span>
              </button>
              <button
                type="button"
                @click="form.component = 'client'; form.has_db_migration = false"
                class="px-3 py-2 rounded-xl border text-xs font-semibold transition cursor-pointer flex items-center justify-center gap-2"
                :class="form.component === 'client' ? 'bg-cyan-50 border-cyan-500 text-cyan-700 dark:bg-cyan-600/30 dark:border-cyan-500 dark:text-cyan-300 ring-2 ring-cyan-500/20' : 'bg-slate-50 dark:bg-slate-950 border-slate-200 dark:border-slate-800 text-slate-600 dark:text-slate-400 hover:border-slate-300 dark:hover:border-slate-700'"
              >
                <Monitor class="w-4 h-4" />
                <span>Notane Client (Universal UI)</span>
              </button>
            </div>
          </div>

          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">Nomor Versi *</label>
              <input
                v-model="form.version"
                type="text"
                placeholder="Contoh: 1.2.0"
                class="w-full px-3 py-2 bg-slate-50 dark:bg-slate-950 border border-slate-200 dark:border-slate-800 rounded-xl text-xs text-slate-900 dark:text-white focus:outline-none focus:border-brand-500 font-mono"
              />
              <span v-if="formErrors.version" class="text-[11px] text-rose-500 mt-0.5 block">{{ formErrors.version }}</span>
            </div>
            <div>
              <label class="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">Paket Lisensi Minimal</label>
              <select
                v-model="form.min_license_tier"
                class="w-full px-3 py-2 bg-slate-50 dark:bg-slate-950 border border-slate-200 dark:border-slate-800 rounded-xl text-xs text-slate-900 dark:text-white focus:outline-none focus:border-brand-500"
              >
                <option value="BASIC">Standar (Semua Toko)</option>
                <option value="PRO">Paket Pro Ke Atas</option>
                <option value="ENTERPRISE">Khusus Paket Lengkap</option>
              </select>
            </div>
          </div>

          <!-- Minimal Server Version Requirement for Client -->
          <div v-if="form.component === 'client'" class="p-3 rounded-xl bg-cyan-50 dark:bg-cyan-950/20 border border-cyan-200 dark:border-cyan-800/40">
            <label class="block text-xs font-semibold text-cyan-800 dark:text-cyan-300 mb-1">Syarat Minimal Versi Server Toko</label>
            <input
              v-model="form.min_server_version"
              type="text"
              placeholder="Contoh: 1.1.0"
              class="w-full px-3 py-2 bg-white dark:bg-slate-950 border border-cyan-300 dark:border-cyan-800/60 rounded-xl text-xs text-slate-900 dark:text-white focus:outline-none focus:border-cyan-500 font-mono"
            />
            <p class="text-[11px] text-slate-500 dark:text-slate-400 mt-1">
              Aplikasi kasir tidak dapat diperbarui sebelum server toko memenuhi versi ini.
            </p>
          </div>

          <div>
            <label class="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">Judul Pembaruan *</label>
            <input
              v-model="form.release_name"
              type="text"
              placeholder="Contoh: Pembaruan Stabilitas Server Toko & Multi-OS"
              class="w-full px-3 py-2 bg-slate-50 dark:bg-slate-950 border border-slate-200 dark:border-slate-800 rounded-xl text-xs text-slate-900 dark:text-white focus:outline-none focus:border-brand-500"
            />
            <span v-if="formErrors.release_name" class="text-[11px] text-rose-500 mt-0.5 block">{{ formErrors.release_name }}</span>
          </div>

          <div>
            <label class="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1">Catatan Pembaruan (Changelog) *</label>
            <textarea
              v-model="form.changelog"
              rows="3"
              placeholder="1. Peningkatan kecepatan simpan transaksi&#10;2. Pembaruan multi-platform..."
              class="w-full px-3 py-2 bg-slate-50 dark:bg-slate-950 border border-slate-200 dark:border-slate-800 rounded-xl text-xs text-slate-900 dark:text-white focus:outline-none focus:border-brand-500 font-sans"
            ></textarea>
            <span v-if="formErrors.changelog" class="text-[11px] text-rose-500 mt-0.5 block">{{ formErrors.changelog }}</span>
          </div>

          <!-- ================================================================= -->
          <!-- SERVER: MULTI-PLATFORM OS FILE UPLOAD CARDS                      -->
          <!-- ================================================================= -->
          <div v-if="form.component === 'server'" class="space-y-3">
            <div class="flex items-center justify-between">
              <div>
                <label class="block text-xs font-bold text-slate-800 dark:text-slate-200">Berkas Rilis Multi-Platform (Sesuai Git Workflow) *</label>
                <p class="text-[11px] text-slate-500 dark:text-slate-400">Unggah berkas binary / bundle installer untuk setiap sistem operasi target.</p>
              </div>
              <label class="px-3 py-1.5 bg-slate-100 hover:bg-slate-200 text-slate-800 border border-slate-300 dark:bg-slate-800 dark:hover:bg-slate-700 dark:text-slate-200 dark:border-slate-700 rounded-xl text-[11px] font-semibold flex items-center gap-1.5 cursor-pointer transition">
                <Upload class="w-3.5 h-3.5 text-brand-600 dark:text-brand-400" />
                <span>Unggah Otomatis Banyak Berkas</span>
                <input
                  type="file"
                  multiple
                  class="hidden"
                  @change="handleMultiServerUpload"
                  accept=".zip,.tar.gz,.exe,.msi,.deb,.appimage,.dmg"
                />
              </label>
            </div>

            <span v-if="formErrors.platforms" class="text-xs text-rose-500 font-semibold block bg-rose-50 dark:bg-rose-500/10 border border-rose-200 dark:border-rose-500/20 p-2 rounded-xl">
              {{ formErrors.platforms }}
            </span>

            <div class="space-y-2.5">
              <div
                v-for="slot in serverPlatforms"
                :key="slot.id"
                class="p-3.5 rounded-xl border transition-all"
                :class="slot.checksum_sha256 ? 'bg-emerald-50 dark:bg-emerald-950/20 border-emerald-300 dark:border-emerald-600/50' : 'bg-slate-50 dark:bg-slate-950/60 border-slate-200 dark:border-slate-800 hover:border-slate-300 dark:hover:border-slate-700'"
              >
                <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-2">
                  <div class="flex items-center gap-2.5">
                    <div class="p-2 rounded-lg bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 shadow-xs">
                      <Monitor v-if="slot.osType === 'windows'" class="w-4 h-4 text-blue-500 dark:text-blue-400" />
                      <Terminal v-else-if="slot.osType === 'linux'" class="w-4 h-4 text-amber-500 dark:text-amber-400" />
                      <Apple v-else class="w-4 h-4 text-slate-700 dark:text-slate-200" />
                    </div>
                    <div>
                      <div class="flex items-center gap-2">
                        <span class="font-bold text-xs text-slate-900 dark:text-white">{{ slot.label }}</span>
                        <span class="px-2 py-0.5 rounded text-[10px] font-mono" :class="slot.checksum_sha256 ? 'bg-emerald-100 text-emerald-800 dark:bg-emerald-500/20 dark:text-emerald-300 font-bold' : 'bg-slate-200 text-slate-600 dark:bg-slate-800 dark:text-slate-400'">
                          {{ slot.checksum_sha256 ? '✓ Siap' : 'Belum Ada' }}
                        </span>
                      </div>
                      <div class="text-[11px] text-slate-500 dark:text-slate-400 font-sans">{{ slot.description }}</div>
                    </div>
                  </div>

                  <!-- Upload or URL Action -->
                  <div class="flex items-center gap-2 self-end sm:self-auto">
                    <label class="px-3 py-1.5 bg-white hover:bg-slate-100 border border-slate-300 hover:border-slate-400 text-slate-700 dark:bg-slate-900 dark:hover:bg-slate-800 dark:border-slate-700 dark:hover:border-slate-600 dark:text-slate-200 rounded-xl text-xs font-semibold flex items-center gap-1.5 cursor-pointer transition shadow-xs">
                      <Upload class="w-3.5 h-3.5 text-brand-600 dark:text-brand-400" />
                      <span>{{ slot.checksum_sha256 ? 'Ganti Berkas' : 'Pilih Berkas' }}</span>
                      <input
                        type="file"
                        class="hidden"
                        @change="(e) => handleSlotUpload(slot, e)"
                        accept=".zip,.tar.gz,.exe,.msi,.deb,.appimage,.dmg"
                      />
                    </label>
                  </div>
                </div>

                <!-- Upload Progress -->
                <div v-if="slot.isUploading" class="mt-2.5 space-y-1">
                  <div class="flex justify-between text-[11px] text-slate-500 dark:text-slate-400">
                    <span>Mengunggah {{ slot.label }}...</span>
                    <span>{{ slot.uploadProgress }}%</span>
                  </div>
                  <div class="w-full bg-slate-200 dark:bg-slate-800 rounded-full h-1.5 overflow-hidden">
                    <div class="bg-brand-500 h-1.5 transition-all duration-200" :style="{ width: `${slot.uploadProgress}%` }"></div>
                  </div>
                </div>

                <!-- Slot File Details -->
                <div v-else-if="slot.checksum_sha256" class="mt-2.5 pt-2 border-t border-slate-200 dark:border-slate-800/80 grid grid-cols-1 sm:grid-cols-2 gap-2 text-[11px]">
                  <div class="text-slate-700 dark:text-slate-300 font-mono truncate" :title="slot.original_filename">
                    📄 {{ slot.original_filename }} ({{ ((slot.file_size || 0) / (1024 * 1024)).toFixed(2) }} MB)
                  </div>
                  <div class="text-emerald-600 dark:text-emerald-400 font-mono truncate select-all" :title="slot.checksum_sha256">
                    SHA: {{ slot.checksum_sha256 }}
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- ================================================================= -->
          <!-- CLIENT: UNIVERSAL UI FILE UPLOAD CARD                            -->
          <!-- ================================================================= -->
          <div v-else class="space-y-2">
            <label class="block text-xs font-bold text-slate-800 dark:text-slate-200">Berkas Universal Client UI (.zip) *</label>
            <p class="text-[11px] text-slate-500 dark:text-slate-400">Paket web antarmuka kasir universal untuk semua sistem operasi (notane-client-ui-v*.zip).</p>

            <span v-if="formErrors.client_file" class="text-xs text-rose-500 font-semibold block bg-rose-50 dark:bg-rose-500/10 border border-rose-200 dark:border-rose-500/20 p-2 rounded-xl">
              {{ formErrors.client_file }}
            </span>

            <div
              class="border-2 border-dashed rounded-xl p-5 text-center cursor-pointer transition"
              :class="clientFile.checksum_sha256 ? 'border-emerald-500 dark:border-emerald-600/60 bg-emerald-50 dark:bg-emerald-950/20' : 'border-slate-300 dark:border-slate-700 hover:border-brand-500 bg-slate-50 dark:bg-slate-950/50'"
            >
              <label class="cursor-pointer block space-y-2">
                <input
                  type="file"
                  class="hidden"
                  @change="handleClientUpload"
                  accept=".zip,.tar.gz"
                />
                <div v-if="!clientFile.checksum_sha256 && !clientFile.isUploading" class="space-y-1">
                  <Upload class="w-8 h-8 text-slate-400 mx-auto" />
                  <div class="text-xs font-semibold text-slate-800 dark:text-slate-200">Klik untuk memilih berkas universal UI kasir (.zip)</div>
                  <div class="text-[11px] text-slate-500">Mendukung berkas zip hasil build git workflow</div>
                </div>
                <div v-else-if="clientFile.isUploading" class="space-y-2 py-2">
                  <RefreshCw class="w-6 h-6 text-brand-500 animate-spin mx-auto" />
                  <div class="text-xs text-slate-700 dark:text-slate-300 font-semibold">Mengunggah paket UI ({{ clientFile.uploadProgress }}%)...</div>
                  <div class="w-full bg-slate-200 dark:bg-slate-800 rounded-full h-1.5 overflow-hidden">
                    <div class="bg-brand-500 h-1.5 transition-all duration-200" :style="{ width: `${clientFile.uploadProgress}%` }"></div>
                  </div>
                </div>
                <div v-else class="space-y-1 py-1">
                  <div class="flex items-center justify-center gap-1.5 text-emerald-600 dark:text-emerald-400 font-bold text-xs">
                    <CheckCircle2 class="w-4 h-4" />
                    <span>Berkas UI Universal Berhasil Terunggah</span>
                  </div>
                  <div class="text-xs text-slate-800 dark:text-slate-300 font-mono">{{ clientFile.original_filename }}</div>
                  <div class="text-[11px] text-slate-500 dark:text-slate-400">
                    Ukuran: {{ ((clientFile.file_size || 0) / (1024 * 1024)).toFixed(2) }} MB | SHA-256 Otomatis Dihitung
                  </div>
                  <div class="pt-1">
                    <span class="text-[11px] text-brand-600 dark:text-brand-400 underline hover:text-brand-500">Klik untuk mengganti berkas</span>
                  </div>
                </div>
              </label>
            </div>
          </div>

          <!-- Checkboxes -->
          <div class="grid grid-cols-2 gap-3 pt-1">
            <label class="flex items-center gap-2 text-xs text-slate-700 dark:text-slate-300 cursor-pointer">
              <input
                v-model="form.is_mandatory"
                type="checkbox"
                class="rounded border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-950 text-brand-600 focus:ring-0"
              />
              <span>Rilis Wajib Diperbarui</span>
            </label>

            <label class="flex items-center gap-2 text-xs text-slate-700 dark:text-slate-300 cursor-pointer">
              <input
                v-model="form.has_db_migration"
                type="checkbox"
                class="rounded border-slate-300 dark:border-slate-700 bg-white dark:bg-slate-950 text-brand-600 focus:ring-0"
              />
              <span>Mengandung Migrasi Database</span>
            </label>
          </div>

          <div class="pt-3 border-t border-slate-200 dark:border-slate-800 flex justify-end gap-2">
            <button
              type="button"
              @click="isModalOpen = false"
              class="px-4 py-2 bg-slate-100 hover:bg-slate-200 text-slate-700 border border-slate-200 dark:bg-slate-800 dark:hover:bg-slate-700 dark:text-slate-300 dark:border-slate-700 rounded-xl text-xs font-semibold cursor-pointer"
            >
              Batal
            </button>
            <button
              type="submit"
              :disabled="isSubmitting"
              class="px-5 py-2 bg-brand-600 hover:bg-brand-500 disabled:opacity-50 text-white rounded-xl text-xs font-semibold cursor-pointer flex items-center gap-1.5"
            >
              <RefreshCw v-if="isSubmitting" class="w-3.5 h-3.5 animate-spin" />
              <span>{{ isSubmitting ? 'Menerbitkan...' : 'Terbitkan Pembaruan' }}</span>
            </button>
          </div>
        </form>
      </div>
    </div>
  </div>
</template>
