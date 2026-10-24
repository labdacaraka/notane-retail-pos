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

const releaseSchema = z.object({
  version: z.string().trim().min(1, 'Versi wajib diisi (contoh: 1.2.0)'),
  release_name: z.string().trim().min(2, 'Nama rilis wajib diisi'),
  changelog: z.string().trim().min(5, 'Changelog minimal 5 karakter'),
  download_url: z.string().trim().url('Format URL unduhan tidak valid'),
  checksum_sha256: z.string().trim().min(10, 'Checksum SHA-256 wajib diisi'),
  is_mandatory: z.boolean(),
  min_license_tier: z.string(),
  has_db_migration: z.boolean(),
})

const form = ref({
  version: '',
  release_name: '',
  changelog: '',
  download_url: '',
  checksum_sha256: '',
  is_mandatory: false,
  min_license_tier: 'BASIC',
  has_db_migration: true,
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

function openCreateModal() {
  form.value = {
    version: '1.2.1',
    release_name: 'Notane Retail POS Enterprise Stability Update',
    changelog: '1. Optimasi query kasir & transaksi offline\n2. Dukungan printer struk Bluetooth\n3. Perbaikan sinkronisasi katalog produk',
    download_url: 'https://update.notane.id/releases/v1.2.1/notane-update-bundle.zip',
    checksum_sha256: 'a1b2c3d4e5f67890a1b2c3d4e5f67890a1b2c3d4e5f67890a1b2c3d4e5f67890',
    is_mandatory: false,
    min_license_tier: 'BASIC',
    has_db_migration: true,
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

      <button
        @click="openCreateModal"
        :disabled="!hasPermission('release:create')"
        class="px-4 py-2 bg-brand-600 hover:bg-brand-500 disabled:opacity-50 text-white rounded-xl text-xs font-semibold flex items-center gap-1.5 transition-all shadow-md shadow-brand-600/20 cursor-pointer"
      >
        <Plus class="w-4 h-4" />
        <span>Terbitkan Versi Rilis Baru</span>
      </button>
    </div>

    <!-- Empty State -->
    <div
      v-if="!providerStore.isLoading && providerStore.releasesPagination.total_items === 0"
      class="py-16 text-center rounded-2xl glass-panel border border-slate-800"
    >
      <Layers class="w-12 h-12 text-slate-600 mx-auto mb-3" />
      <div class="text-sm font-semibold text-slate-300">Belum Ada Rilis Pembaruan Terdaftar</div>
      <div class="text-xs text-slate-500 mt-1">
        {{ searchQuery ? 'Tidak ada rilis yang cocok dengan pencarian Anda.' : 'Terbitkan paket rilis OTA pertama dengan tanda tangan kriptografi Ed25519.' }}
      </div>
      <button
        v-if="!searchQuery"
        @click="openCreateModal"
        class="mt-4 px-4 py-2 bg-brand-600 hover:bg-brand-500 text-white rounded-xl text-xs font-semibold cursor-pointer"
      >
        Terbitkan Rilis Sekarang
      </button>
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
              <div class="px-3 py-1.5 rounded-xl bg-brand-500/10 border border-brand-500/30 text-brand-400 font-mono text-sm font-bold">
                v{{ rel.version }}
              </div>
              <div>
                <h3 class="font-bold text-sm text-white">{{ rel.release_name }}</h3>
                <div class="flex items-center gap-3 text-[11px] text-slate-400 mt-0.5">
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
                  <span>Ed25519 Signature</span>
                </span>
                <span class="text-brand-400 text-[10px] font-semibold">Tanda Tangan Master</span>
              </div>
              <div class="font-mono text-[11px] text-emerald-300/90 truncate select-all" :title="rel.signature_ed25519">
                {{ rel.signature_ed25519 }}
              </div>
            </div>
          </div>

          <!-- Download URL -->
          <div class="flex items-center justify-between pt-1 text-[11px] text-slate-400">
            <div class="flex items-center gap-1.5 truncate max-w-[80%] font-mono">
              <Download class="w-3.5 h-3.5 text-slate-500 shrink-0" />
              <span class="truncate">{{ rel.download_url }}</span>
            </div>
            <a
              :href="rel.download_url"
              target="_blank"
              class="text-brand-400 hover:text-brand-300 font-semibold underline shrink-0 cursor-pointer"
            >
              Unduh Paket
            </a>
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
          <div class="font-bold text-sm text-white">Terbitkan Versi Rilis OTA Baru</div>
          <button @click="isModalOpen = false" class="text-slate-400 hover:text-white cursor-pointer">
            <X class="w-4 h-4" />
          </button>
        </div>

        <form @submit.prevent="handleSubmit" class="space-y-3.5">
          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="block text-xs font-semibold text-slate-300 mb-1">Versi Semver *</label>
              <input
                v-model="form.version"
                type="text"
                placeholder="1.2.0"
                class="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-xl text-xs text-white focus:outline-none focus:border-brand-500 font-mono"
              />
              <span v-if="formErrors.version" class="text-[11px] text-rose-400 mt-0.5 block">{{ formErrors.version }}</span>
            </div>
            <div>
              <label class="block text-xs font-semibold text-slate-300 mb-1">Minimal License Tier</label>
              <select
                v-model="form.min_license_tier"
                class="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-xl text-xs text-white focus:outline-none focus:border-brand-500"
              >
                <option value="BASIC">BASIC (Semua)</option>
                <option value="PRO">PRO Ke Atas</option>
                <option value="ENTERPRISE">ENTERPRISE Only</option>
              </select>
            </div>
          </div>

          <div>
            <label class="block text-xs font-semibold text-slate-300 mb-1">Nama Judul Rilis *</label>
            <input
              v-model="form.release_name"
              type="text"
              placeholder="Notane Retail POS Q4 Enterprise Update"
              class="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-xl text-xs text-white focus:outline-none focus:border-brand-500"
            />
            <span v-if="formErrors.release_name" class="text-[11px] text-rose-400 mt-0.5 block">{{ formErrors.release_name }}</span>
          </div>

          <div>
            <label class="block text-xs font-semibold text-slate-300 mb-1">Catatan Perubahan (Changelog) *</label>
            <textarea
              v-model="form.changelog"
              rows="4"
              placeholder="1. Fitur baru&#10;2. Perbaikan bug kasir..."
              class="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-xl text-xs text-white focus:outline-none focus:border-brand-500 font-sans"
            ></textarea>
            <span v-if="formErrors.changelog" class="text-[11px] text-rose-400 mt-0.5 block">{{ formErrors.changelog }}</span>
          </div>

          <div>
            <label class="block text-xs font-semibold text-slate-300 mb-1">URL Unduhan Bundle ZIP *</label>
            <input
              v-model="form.download_url"
              type="url"
              placeholder="https://update.notane.id/releases/v1.2.0/bundle.zip"
              class="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-xl text-xs text-white focus:outline-none focus:border-brand-500 font-mono"
            />
            <span v-if="formErrors.download_url" class="text-[11px] text-rose-400 mt-0.5 block">{{ formErrors.download_url }}</span>
          </div>

          <div>
            <label class="block text-xs font-semibold text-slate-300 mb-1">Checksum SHA-256 File Bundle *</label>
            <input
              v-model="form.checksum_sha256"
              type="text"
              placeholder="e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
              class="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-xl text-xs text-white focus:outline-none focus:border-brand-500 font-mono text-[11px]"
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
              {{ isSubmitting ? 'Menandatangani...' : 'Terbitkan & Tanda Tangani Ed25519' }}
            </button>
          </div>
        </form>
      </div>
    </div>
  </div>
</template>
