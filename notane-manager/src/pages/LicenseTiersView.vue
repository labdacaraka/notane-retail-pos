<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { useRouter } from 'vue-router'
import { useProviderStore, type LicenseTier } from '../stores/providerStore'
import { usePermission } from '../composables/usePermission'
import PaginationBar from '../components/common/PaginationBar.vue'
import {
  Award,
  Plus,
  Trash2,
  Edit2,
  X,
  Search,
  ArrowUpDown,
  ArrowUp,
  ArrowDown,
  LayoutGrid,
  Table as TableIcon,
  RefreshCw,
  CheckCircle2,
  XCircle,
  Layers,
  Monitor,
  Calendar,
  Clock,
  CheckSquare,
  Square,
} from 'lucide-vue-next'
import { z } from 'zod'

const router = useRouter()
const providerStore = useProviderStore()
const { hasPermission } = usePermission()

// Server-side filter & sort
const searchQuery = ref('')
const sortBy = ref<'created_at' | 'name' | 'code' | 'default_max_terminals' | 'is_active'>('created_at')
const sortOrder = ref<'asc' | 'desc'>('desc')
const viewMode = ref<'table' | 'grid'>('table')

// Modal state
const isModalOpen = ref(false)
const isEditing = ref(false)
const editingTierId = ref<string | null>(null)
const isSubmitting = ref(false)
const formErrors = ref<Record<string, string>>({})

const availableModules = [
  { id: 'membership', name: 'Pelanggan & Membership', desc: 'Loyalty point, kartu member, & diskon tier member' },
  { id: 'accounting', name: 'Akuntansi & Pembukuan', desc: 'Bagan akun (COA), jurnal umum, neraca & laba rugi' },
  { id: 'analytics', name: 'Statistik & Analisis Bisnis', desc: 'Dasbor statistik eksekutif, grafik tren, analisis performa penjualan' },
  { id: 'multi_outlet', name: 'Multi-Outlet Cabang', desc: 'Manajemen banyak cabang dan transfer stok antar-outlet' },
  { id: 'inventory_batch', name: 'Manajemen Batch & FEFO', desc: 'Nomor batch & rotasi produk berdasarkan kadaluarsa' },
  { id: 'promotions', name: 'Promosi Retail & Flash Sale', desc: 'Promo diskon otomatis, voucher belanja & flash sale' },
  { id: 'expenses', name: 'Biaya Operasional Toko', desc: 'Pencatatan pengeluaran operasional & kas kecil' },
  { id: 'other_income', name: 'Pendapatan Lain-lain', desc: 'Pencatatan pendapatan non-penjualan & kas masuk lain' },
  { id: 'customer_display', name: 'Customer Display & Banner', desc: 'Layar kedua pelanggan dengan banner promosi produk' },
  { id: 'audit_log', name: 'Audit Trail & Keamanan', desc: 'Rekaman riwayat aktivitas kasir & supervisor' },
  { id: 'label_printing', name: 'Cetak Label Harga & Barcode', desc: 'Desain dan cetak label barcode produk & rak display' },
  { id: 'remove_branding', name: 'White-Label (Remove Branding)', desc: 'Hilangkan logo dan atribusi Notane pada struk & UI' },
]

const tierSchema = z.object({
  name: z.string().trim().min(2, 'Nama paket tier minimal 2 karakter'),
  code: z.string().trim().min(2, 'Kode tier minimal 2 karakter').regex(/^[A-Za-z0-9_-]+$/, 'Kode hanya boleh alfanumerik, dash atau underscore'),
  description: z.string().optional(),
  default_max_terminals: z.number().int().min(1, 'Minimal 1 terminal').max(1000, 'Maksimal 1000 terminal'),
  default_validity_days: z.number().int().min(1, 'Minimal 1 hari').max(3650, 'Maksimal 3650 hari (10 tahun)'),
  default_grace_period_days: z.number().int().min(0, 'Minimal 0 hari').max(180, 'Maksimal 180 hari'),
  features: z.array(z.string()).min(0),
  is_active: z.boolean(),
})

const form = ref({
  name: '',
  code: '',
  description: '',
  default_max_terminals: 3,
  default_validity_days: 365,
  default_grace_period_days: 7,
  features: [] as string[],
  is_active: true,
})

function parseFeatures(featuresStr: string | string[]): string[] {
  if (Array.isArray(featuresStr)) return featuresStr
  try {
    return JSON.parse(featuresStr || '[]')
  } catch {
    return []
  }
}

async function loadData(page?: number, pageSize?: number) {
  await providerStore.fetchTiers({
    page: page ?? providerStore.tiersPagination.page,
    page_size: pageSize ?? providerStore.tiersPagination.page_size,
    search: searchQuery.value.trim() || undefined,
    sort_by: sortBy.value,
    sort_order: sortOrder.value,
  })
}

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

function handleSort(column: 'created_at' | 'name' | 'code' | 'default_max_terminals' | 'is_active') {
  if (sortBy.value === column) {
    sortOrder.value = sortOrder.value === 'asc' ? 'desc' : 'asc'
  } else {
    sortBy.value = column
    sortOrder.value = 'asc'
  }
  loadData(1)
}

function openCreateModal() {
  isEditing.value = false
  editingTierId.value = null
  form.value = {
    name: '',
    code: '',
    description: '',
    default_max_terminals: 3,
    default_validity_days: 365,
    default_grace_period_days: 7,
    features: ['membership', 'expenses', 'other_income'],
    is_active: true,
  }
  formErrors.value = {}
  isModalOpen.value = true
}

function openEditModal(tier: LicenseTier) {
  isEditing.value = true
  editingTierId.value = tier.id
  form.value = {
    name: tier.name,
    code: tier.code,
    description: tier.description || '',
    default_max_terminals: Number(tier.default_max_terminals) || 1,
    default_validity_days: Number(tier.default_validity_days) || 365,
    default_grace_period_days: Number(tier.default_grace_period_days) ?? 7,
    features: parseFeatures(tier.features),
    is_active: tier.is_active === 1,
  }
  formErrors.value = {}
  isModalOpen.value = true
}

function toggleFeature(id: string) {
  const idx = form.value.features.indexOf(id)
  if (idx >= 0) {
    form.value.features.splice(idx, 1)
  } else {
    form.value.features.push(id)
  }
}

function selectAllFeatures() {
  form.value.features = availableModules.map((m) => m.id)
}

function clearAllFeatures() {
  form.value.features = []
}

async function handleSubmit() {
  formErrors.value = {}
  const result = tierSchema.safeParse(form.value)
  if (!result.success) {
    result.error.errors.forEach((err) => {
      const field = err.path[0] as string
      formErrors.value[field] = err.message
    })
    return
  }

  isSubmitting.value = true
  try {
    let success = false
    if (isEditing.value && editingTierId.value) {
      success = await providerStore.updateTier(editingTierId.value, {
        name: form.value.name.trim(),
        code: form.value.code.trim().toUpperCase(),
        description: form.value.description.trim() || undefined,
        default_max_terminals: form.value.default_max_terminals,
        default_validity_days: form.value.default_validity_days,
        default_grace_period_days: form.value.default_grace_period_days,
        features: form.value.features,
        is_active: form.value.is_active,
      })
    } else {
      success = await providerStore.saveTier({
        name: form.value.name.trim(),
        code: form.value.code.trim().toUpperCase(),
        description: form.value.description.trim() || undefined,
        default_max_terminals: form.value.default_max_terminals,
        default_validity_days: form.value.default_validity_days,
        default_grace_period_days: form.value.default_grace_period_days,
        features: form.value.features,
        is_active: form.value.is_active,
      })
    }

    if (success) {
      isModalOpen.value = false
    }
  } finally {
    isSubmitting.value = false
  }
}

async function confirmDelete(tier: LicenseTier) {
  if (confirm(`Apakah Anda yakin ingin menghapus tier lisensi "${tier.name}" (${tier.code})?`)) {
    await providerStore.deleteTier(tier.id)
  }
}
</script>

<template>
  <div class="space-y-6">
    <!-- Top Bar Header -->
    <div class="flex flex-col md:flex-row md:items-center justify-between gap-4">
      <div>
        <h1 class="text-2xl font-bold tracking-tight text-slate-900 dark:text-white flex items-center gap-2">
          <Award class="w-7 h-7 text-brand-600 dark:text-brand-400" />
          <span>Tier Lisensi POS</span>
        </h1>
        <p class="text-sm text-slate-500 dark:text-slate-400 mt-1">
          Kelola paket langganan lisensi retail dan tentukan kuota terminal, masa berlaku, toleransi offline, serta bundling fitur opsi.
        </p>
      </div>

      <div class="flex items-center gap-2">
        <button
          @click="loadData()"
          class="p-2.5 rounded-xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900/60 text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white hover:border-slate-300 dark:hover:border-slate-700 transition shadow-sm cursor-pointer"
          title="Segarkan Data"
        >
          <RefreshCw class="w-4 h-4" :class="{ 'animate-spin': providerStore.isLoading }" />
        </button>

        <button
          @click="openCreateModal"
          :disabled="!hasPermission('tier:create')"
          class="flex items-center gap-2 px-4 py-2.5 rounded-xl font-medium text-sm text-white bg-gradient-to-r from-brand-600 to-indigo-600 hover:from-brand-500 hover:to-indigo-500 transition shadow-lg shadow-brand-500/25 active:scale-[0.98] disabled:opacity-50 cursor-pointer"
        >
          <Plus class="w-4 h-4" />
          <span>Tambah Tier Baru</span>
        </button>
      </div>
    </div>

    <!-- Filters & Search Toolbar -->
    <div class="p-4 rounded-2xl bg-white dark:bg-slate-900/60 border border-slate-200 dark:border-slate-800 shadow-sm flex flex-col md:flex-row items-center justify-between gap-4">
      <!-- Search Input -->
      <div class="relative w-full md:w-96">
        <Search class="w-4 h-4 text-slate-400 absolute left-3.5 top-1/2 -translate-y-1/2" />
        <input
          v-model="searchQuery"
          type="text"
          placeholder="Cari nama, kode, atau deskripsi tier..."
          class="w-full pl-10 pr-4 py-2 text-sm rounded-xl border border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-950/60 text-slate-900 dark:text-white placeholder-slate-400 focus:outline-none focus:border-brand-500 focus:ring-1 focus:ring-brand-500 transition"
        />
        <button
          v-if="searchQuery"
          @click="searchQuery = ''"
          class="absolute right-3 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-600 dark:hover:text-white cursor-pointer"
        >
          <X class="w-4 h-4" />
        </button>
      </div>

      <!-- View Mode Toggle -->
      <div class="flex items-center gap-1 p-1 rounded-xl border border-slate-200 dark:border-slate-800 bg-slate-100 dark:bg-slate-950/60">
        <button
          @click="viewMode = 'table'"
          :class="[
            'p-1.5 rounded-lg text-xs font-medium flex items-center gap-1.5 transition cursor-pointer',
            viewMode === 'table'
              ? 'bg-white dark:bg-slate-800 text-slate-900 dark:text-white shadow-sm'
              : 'text-slate-500 hover:text-slate-900 dark:hover:text-white'
          ]"
          title="Tampilan Tabel"
        >
          <TableIcon class="w-4 h-4" />
          <span class="hidden sm:inline">Tabel</span>
        </button>
        <button
          @click="viewMode = 'grid'"
          :class="[
            'p-1.5 rounded-lg text-xs font-medium flex items-center gap-1.5 transition cursor-pointer',
            viewMode === 'grid'
              ? 'bg-white dark:bg-slate-800 text-slate-900 dark:text-white shadow-sm'
              : 'text-slate-500 hover:text-slate-900 dark:hover:text-white'
          ]"
          title="Tampilan Grid Card"
        >
          <LayoutGrid class="w-4 h-4" />
          <span class="hidden sm:inline">Grid</span>
        </button>
      </div>
    </div>

    <!-- Empty State -->
    <div
      v-if="providerStore.tiersPagination.items.length === 0 && !providerStore.isLoading"
      class="p-12 text-center rounded-2xl bg-white dark:bg-slate-900/40 border border-slate-200 dark:border-slate-800"
    >
      <div class="w-14 h-14 rounded-2xl bg-brand-500/10 text-brand-600 dark:text-brand-400 flex items-center justify-center mx-auto mb-4">
        <Award class="w-7 h-7" />
      </div>
      <h3 class="text-base font-semibold text-slate-900 dark:text-white">Belum Ada Tier Lisensi</h3>
      <p class="text-sm text-slate-500 dark:text-slate-400 max-w-sm mx-auto mt-1 mb-5">
        Buat paket tier lisensi pertama Anda untuk menentukan bundling fitur, kuota terminal, dan masa berlaku lisensi bagi klien.
      </p>
      <button
        @click="openCreateModal"
        :disabled="!hasPermission('tier:create')"
        class="inline-flex items-center gap-2 px-4 py-2 rounded-xl text-sm font-medium text-white bg-brand-600 hover:bg-brand-500 transition shadow-sm cursor-pointer"
      >
        <Plus class="w-4 h-4" />
        <span>Tambah Tier Sekarang</span>
      </button>
    </div>

    <!-- TABLE VIEW -->
    <div v-else-if="viewMode === 'table'" class="space-y-4">
      <div class="rounded-2xl border border-slate-200 dark:border-slate-800 bg-white dark:bg-slate-900/60 overflow-hidden shadow-sm">
        <div class="overflow-x-auto">
          <table class="w-full text-left text-sm text-slate-600 dark:text-slate-300">
            <thead class="bg-slate-50 dark:bg-slate-950/60 text-xs uppercase font-semibold text-slate-500 dark:text-slate-400 border-b border-slate-200 dark:border-slate-800">
              <tr>
                <th @click="handleSort('name')" class="py-3.5 px-4 cursor-pointer hover:text-slate-900 dark:hover:text-white transition">
                  <div class="flex items-center gap-1.5">
                    <span>Nama Paket</span>
                    <ArrowUp v-if="sortBy === 'name' && sortOrder === 'asc'" class="w-3.5 h-3.5 text-brand-500" />
                    <ArrowDown v-else-if="sortBy === 'name' && sortOrder === 'desc'" class="w-3.5 h-3.5 text-brand-500" />
                    <ArrowUpDown v-else class="w-3.5 h-3.5 opacity-40" />
                  </div>
                </th>
                <th @click="handleSort('code')" class="py-3.5 px-4 cursor-pointer hover:text-slate-900 dark:hover:text-white transition">
                  <div class="flex items-center gap-1.5">
                    <span>Kode</span>
                    <ArrowUp v-if="sortBy === 'code' && sortOrder === 'asc'" class="w-3.5 h-3.5 text-brand-500" />
                    <ArrowDown v-else-if="sortBy === 'code' && sortOrder === 'desc'" class="w-3.5 h-3.5 text-brand-500" />
                    <ArrowUpDown v-else class="w-3.5 h-3.5 opacity-40" />
                  </div>
                </th>
                <th class="py-3.5 px-4">
                  <span>Parameter Lisensi</span>
                </th>
                <th class="py-3.5 px-4">Fitur Modul</th>
                <th @click="handleSort('is_active')" class="py-3.5 px-4 cursor-pointer hover:text-slate-900 dark:hover:text-white transition text-center">
                  <div class="flex items-center justify-center gap-1.5">
                    <span>Status</span>
                    <ArrowUp v-if="sortBy === 'is_active' && sortOrder === 'asc'" class="w-3.5 h-3.5 text-brand-500" />
                    <ArrowDown v-else-if="sortBy === 'is_active' && sortOrder === 'desc'" class="w-3.5 h-3.5 text-brand-500" />
                    <ArrowUpDown v-else class="w-3.5 h-3.5 opacity-40" />
                  </div>
                </th>
                <th @click="handleSort('created_at')" class="py-3.5 px-4 cursor-pointer hover:text-slate-900 dark:hover:text-white transition">
                  <div class="flex items-center gap-1.5">
                    <span>Dibuat</span>
                    <ArrowUp v-if="sortBy === 'created_at' && sortOrder === 'asc'" class="w-3.5 h-3.5 text-brand-500" />
                    <ArrowDown v-else-if="sortBy === 'created_at' && sortOrder === 'desc'" class="w-3.5 h-3.5 text-brand-500" />
                    <ArrowUpDown v-else class="w-3.5 h-3.5 opacity-40" />
                  </div>
                </th>
                <th class="py-3.5 px-4 text-right">Aksi</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-200 dark:divide-slate-800/60">
              <tr
                v-for="t in providerStore.tiersPagination.items"
                :key="t.id"
                class="hover:bg-slate-50 dark:hover:bg-slate-800/40 transition"
              >
                <!-- Name -->
                <td class="py-3.5 px-4 font-medium text-slate-900 dark:text-white">
                  <div class="flex items-center gap-2">
                    <span class="w-2.5 h-2.5 rounded-full" :class="t.is_active === 1 ? 'bg-emerald-500' : 'bg-slate-400'"></span>
                    <div>
                      <div class="font-semibold">{{ t.name }}</div>
                      <div v-if="t.description" class="text-xs text-slate-400 truncate max-w-xs">{{ t.description }}</div>
                    </div>
                  </div>
                </td>

                <!-- Code -->
                <td class="py-3.5 px-4">
                  <span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-mono font-bold bg-indigo-50 dark:bg-indigo-950/60 text-indigo-600 dark:text-indigo-400 border border-indigo-200 dark:border-indigo-800">
                    {{ t.code }}
                  </span>
                </td>

                <!-- Parameters (Terminals, Validity, Grace) -->
                <td class="py-3.5 px-4 text-xs font-mono">
                  <div class="flex flex-col gap-1">
                    <span class="inline-flex items-center gap-1 text-slate-700 dark:text-slate-300">
                      <Monitor class="w-3 h-3 text-brand-500 shrink-0" />
                      <strong>{{ t.default_max_terminals }}</strong> Terminal Kasir
                    </span>
                    <span class="inline-flex items-center gap-1 text-slate-600 dark:text-slate-400">
                      <Calendar class="w-3 h-3 text-indigo-500 shrink-0" />
                      <span>{{ t.default_validity_days }} Hari Masa Berlaku</span>
                    </span>
                    <span class="inline-flex items-center gap-1 text-slate-500 text-[11px]">
                      <Clock class="w-3 h-3 text-amber-500 shrink-0" />
                      <span>Grace Offline: {{ t.default_grace_period_days }} Hari</span>
                    </span>
                  </div>
                </td>

                <!-- Features Badges -->
                <td class="py-3.5 px-4 max-w-xs">
                  <div class="flex flex-wrap gap-1 items-center">
                    <span
                      v-for="f in parseFeatures(t.features).slice(0, 3)"
                      :key="f"
                      class="px-2 py-0.5 rounded text-[11px] bg-slate-100 dark:bg-slate-800 text-slate-700 dark:text-slate-300 font-medium"
                    >
                      {{ availableModules.find(m => m.id === f)?.name.split(' ')[0] || f }}
                    </span>
                    <span
                      v-if="parseFeatures(t.features).length > 3"
                      class="px-1.5 py-0.5 rounded text-[10px] bg-brand-50 dark:bg-brand-950/60 text-brand-600 dark:text-brand-400 font-semibold"
                    >
                      +{{ parseFeatures(t.features).length - 3 }} lainnya
                    </span>
                    <span
                      v-if="parseFeatures(t.features).length === 0"
                      class="text-xs text-slate-400 italic"
                    >
                      Tidak ada fitur aktif
                    </span>
                  </div>
                </td>

                <!-- Status -->
                <td class="py-3.5 px-4 text-center">
                  <span
                    v-if="t.is_active === 1"
                    class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium bg-emerald-50 dark:bg-emerald-950/50 text-emerald-600 dark:text-emerald-400 border border-emerald-200 dark:border-emerald-800"
                  >
                    <CheckCircle2 class="w-3 h-3" />
                    <span>Aktif</span>
                  </span>
                  <span
                    v-else
                    class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium bg-slate-100 dark:bg-slate-800 text-slate-500 border border-slate-300 dark:border-slate-700"
                  >
                    <XCircle class="w-3 h-3" />
                    <span>Nonaktif</span>
                  </span>
                </td>

                <!-- Created Date -->
                <td class="py-3.5 px-4 font-mono text-xs text-slate-400 whitespace-nowrap">
                  {{ new Date(t.created_at).toLocaleDateString('id-ID', { year: 'numeric', month: 'short', day: 'numeric' }) }}
                </td>

                <!-- Actions -->
                <td class="py-3.5 px-4 text-right">
                  <div class="flex items-center justify-end gap-1">
                    <button
                      @click="openEditModal(t)"
                      :disabled="!hasPermission('tier:update')"
                      title="Edit Tier"
                      class="p-1.5 text-slate-400 hover:text-amber-500 hover:bg-amber-500/10 rounded-lg transition cursor-pointer"
                    >
                      <Edit2 class="w-4 h-4" />
                    </button>
                    <button
                      @click="confirmDelete(t)"
                      :disabled="!hasPermission('tier:delete')"
                      title="Hapus Tier"
                      class="p-1.5 text-slate-400 hover:text-rose-500 hover:bg-rose-500/10 rounded-lg transition cursor-pointer"
                    >
                      <Trash2 class="w-4 h-4" />
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <!-- Pagination -->
      <PaginationBar
        :page="providerStore.tiersPagination.page"
        :page-size="providerStore.tiersPagination.page_size"
        :total-items="providerStore.tiersPagination.total_items"
        :total-pages="providerStore.tiersPagination.total_pages"
        :is-loading="providerStore.isLoading"
        @update:page="(p) => loadData(p, providerStore.tiersPagination.page_size)"
        @update:page-size="(s) => loadData(1, s)"
      />
    </div>

    <!-- GRID VIEW -->
    <div v-else class="space-y-4">
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        <div
          v-for="t in providerStore.tiersPagination.items"
          :key="t.id"
          class="p-5 rounded-2xl bg-white dark:bg-slate-900/60 border border-slate-200 dark:border-slate-800 hover:border-slate-300 dark:hover:border-slate-700 transition flex flex-col justify-between shadow-sm"
        >
          <div class="space-y-3">
            <div class="flex items-start justify-between">
              <div>
                <span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-mono font-bold bg-indigo-50 dark:bg-indigo-950/60 text-indigo-600 dark:text-indigo-400 border border-indigo-200 dark:border-indigo-800">
                  {{ t.code }}
                </span>
                <h3 class="font-bold text-base text-slate-900 dark:text-white mt-1">{{ t.name }}</h3>
                <p v-if="t.description" class="text-xs text-slate-500 dark:text-slate-400 mt-0.5 line-clamp-2">
                  {{ t.description }}
                </p>
              </div>
              <div class="flex items-center gap-1">
                <button
                  @click="openEditModal(t)"
                  :disabled="!hasPermission('tier:update')"
                  class="p-1.5 text-slate-400 hover:text-amber-500 hover:bg-amber-500/10 rounded-lg transition cursor-pointer"
                  title="Edit Tier"
                >
                  <Edit2 class="w-4 h-4" />
                </button>
                <button
                  @click="confirmDelete(t)"
                  :disabled="!hasPermission('tier:delete')"
                  class="p-1.5 text-slate-400 hover:text-rose-500 hover:bg-rose-500/10 rounded-lg transition cursor-pointer"
                  title="Hapus Tier"
                >
                  <Trash2 class="w-4 h-4" />
                </button>
              </div>
            </div>

            <div class="p-3 rounded-xl bg-slate-50 dark:bg-slate-950/50 border border-slate-200/60 dark:border-slate-800/60 space-y-1.5 text-xs">
              <div class="flex items-center justify-between">
                <span class="text-slate-500 dark:text-slate-400 flex items-center gap-1">
                  <Monitor class="w-3.5 h-3.5 text-brand-500" />
                  <span>Kapasitas Terminal:</span>
                </span>
                <span class="font-mono font-bold text-slate-900 dark:text-white">{{ t.default_max_terminals }} Unit</span>
              </div>
              <div class="flex items-center justify-between">
                <span class="text-slate-500 dark:text-slate-400 flex items-center gap-1">
                  <Calendar class="w-3.5 h-3.5 text-indigo-500" />
                  <span>Masa Berlaku:</span>
                </span>
                <span class="font-mono font-bold text-slate-900 dark:text-white">{{ t.default_validity_days }} Hari</span>
              </div>
              <div class="flex items-center justify-between">
                <span class="text-slate-500 dark:text-slate-400 flex items-center gap-1">
                  <Clock class="w-3.5 h-3.5 text-amber-500" />
                  <span>Toleransi Grace Offline:</span>
                </span>
                <span class="font-mono font-bold text-slate-900 dark:text-white">{{ t.default_grace_period_days }} Hari</span>
              </div>
            </div>

            <div class="space-y-1.5">
              <div class="text-xs font-semibold text-slate-500 dark:text-slate-400 flex items-center justify-between">
                <span>Modul Fitur</span>
                <span>{{ parseFeatures(t.features).length }} Aktif</span>
              </div>
              <div class="flex flex-wrap gap-1">
                <span
                  v-for="f in parseFeatures(t.features).slice(0, 4)"
                  :key="f"
                  class="px-2 py-0.5 rounded text-[11px] bg-slate-100 dark:bg-slate-800 text-slate-700 dark:text-slate-300"
                >
                  {{ availableModules.find(m => m.id === f)?.name.split(' ')[0] || f }}
                </span>
                <span
                  v-if="parseFeatures(t.features).length > 4"
                  class="px-1.5 py-0.5 rounded text-[10px] bg-brand-50 dark:bg-brand-950/60 text-brand-600 dark:text-brand-400 font-semibold"
                >
                  +{{ parseFeatures(t.features).length - 4 }}
                </span>
              </div>
            </div>
          </div>

          <div class="pt-4 mt-4 border-t border-slate-100 dark:border-slate-800/80 flex items-center justify-between text-xs">
            <span
              v-if="t.is_active === 1"
              class="inline-flex items-center gap-1 text-emerald-600 dark:text-emerald-400 font-medium"
            >
              <CheckCircle2 class="w-3.5 h-3.5" />
              <span>Status Aktif</span>
            </span>
            <span
              v-else
              class="inline-flex items-center gap-1 text-slate-400 font-medium"
            >
              <XCircle class="w-3.5 h-3.5" />
              <span>Nonaktif</span>
            </span>
            <span class="text-slate-400 font-mono text-[11px]">
              {{ new Date(t.created_at).toLocaleDateString('id-ID') }}
            </span>
          </div>
        </div>
      </div>

      <!-- Pagination -->
      <PaginationBar
        :page="providerStore.tiersPagination.page"
        :page-size="providerStore.tiersPagination.page_size"
        :total-items="providerStore.tiersPagination.total_items"
        :total-pages="providerStore.tiersPagination.total_pages"
        :is-loading="providerStore.isLoading"
        @update:page="(p) => loadData(p, providerStore.tiersPagination.page_size)"
        @update:page-size="(s) => loadData(1, s)"
      />
    </div>

    <!-- MODAL CREATE / EDIT TIER -->
    <Teleport to="body">
      <div
        v-if="isModalOpen"
        class="fixed inset-0 z-50 flex items-center justify-center p-4 overflow-y-auto bg-slate-900/60 backdrop-blur-sm"
      >
        <div
          class="relative w-full max-w-2xl max-h-[90vh] flex flex-col rounded-2xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 shadow-2xl overflow-hidden"
        >
          <!-- Header -->
          <div class="px-6 py-4 border-b border-slate-200 dark:border-slate-800 flex items-center justify-between">
            <div class="flex items-center gap-2">
              <Award class="w-5 h-5 text-brand-600 dark:text-brand-400" />
              <h2 class="text-lg font-bold text-slate-900 dark:text-white">
                {{ isEditing ? 'Edit Tier Lisensi' : 'Tambah Paket Tier Baru' }}
              </h2>
            </div>
            <button
              @click="isModalOpen = false"
              class="p-1 rounded-lg text-slate-400 hover:text-slate-600 dark:hover:text-white transition cursor-pointer"
            >
              <X class="w-5 h-5" />
            </button>
          </div>

          <!-- Body -->
          <form @submit.prevent="handleSubmit" class="p-6 overflow-y-auto space-y-4">
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <!-- Name -->
              <div>
                <label class="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1.5">
                  Nama Paket Tier <span class="text-rose-500">*</span>
                </label>
                <input
                  v-model="form.name"
                  type="text"
                  placeholder="Contoh: Retail Starter, Waralaba Pro"
                  class="w-full px-3.5 py-2 text-sm rounded-xl border border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-950/60 text-slate-900 dark:text-white placeholder-slate-400 focus:outline-none focus:border-brand-500 focus:ring-1 focus:ring-brand-500 transition"
                />
                <span v-if="formErrors.name" class="text-xs text-rose-500 mt-1 block">{{ formErrors.name }}</span>
              </div>

              <!-- Code -->
              <div>
                <label class="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1.5">
                  Kode / Slug Tier <span class="text-rose-500">*</span>
                </label>
                <input
                  v-model="form.code"
                  type="text"
                  placeholder="Contoh: STARTER, PRO, ENTERPRISE"
                  class="w-full px-3.5 py-2 text-sm uppercase font-mono rounded-xl border border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-950/60 text-slate-900 dark:text-white placeholder-slate-400 focus:outline-none focus:border-brand-500 focus:ring-1 focus:ring-brand-500 transition"
                />
                <span v-if="formErrors.code" class="text-xs text-rose-500 mt-1 block">{{ formErrors.code }}</span>
              </div>
            </div>

            <!-- Parameters Grid (Terminals, Validity, Grace Period) -->
            <div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
              <!-- Default Max Terminals -->
              <div>
                <label class="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1.5">
                  Kapasitas Terminal POS <span class="text-rose-500">*</span>
                </label>
                <input
                  v-model.number="form.default_max_terminals"
                  type="number"
                  min="1"
                  max="1000"
                  class="w-full px-3.5 py-2 text-sm font-mono rounded-xl border border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-950/60 text-slate-900 dark:text-white focus:outline-none focus:border-brand-500 focus:ring-1 focus:ring-brand-500 transition"
                />
                <span v-if="formErrors.default_max_terminals" class="text-xs text-rose-500 mt-1 block">{{ formErrors.default_max_terminals }}</span>
                <span class="text-[10px] text-slate-400 mt-0.5 block">Maks unit notane-client</span>
              </div>

              <!-- Default Validity Days -->
              <div>
                <label class="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1.5">
                  Masa Berlaku (Hari) <span class="text-rose-500">*</span>
                </label>
                <input
                  v-model.number="form.default_validity_days"
                  type="number"
                  min="1"
                  max="3650"
                  class="w-full px-3.5 py-2 text-sm font-mono rounded-xl border border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-950/60 text-slate-900 dark:text-white focus:outline-none focus:border-brand-500 focus:ring-1 focus:ring-brand-500 transition"
                />
                <span v-if="formErrors.default_validity_days" class="text-xs text-rose-500 mt-1 block">{{ formErrors.default_validity_days }}</span>
                <span class="text-[10px] text-slate-400 mt-0.5 block">Contoh: 30, 365, 730 hari</span>
              </div>

              <!-- Default Grace Period Days -->
              <div>
                <label class="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1.5">
                  Grace Period Offline <span class="text-rose-500">*</span>
                </label>
                <input
                  v-model.number="form.default_grace_period_days"
                  type="number"
                  min="0"
                  max="180"
                  class="w-full px-3.5 py-2 text-sm font-mono rounded-xl border border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-950/60 text-slate-900 dark:text-white focus:outline-none focus:border-brand-500 focus:ring-1 focus:ring-brand-500 transition"
                />
                <span v-if="formErrors.default_grace_period_days" class="text-xs text-rose-500 mt-1 block">{{ formErrors.default_grace_period_days }}</span>
                <span class="text-[10px] text-slate-400 mt-0.5 block">Toleransi tanpa internet</span>
              </div>
            </div>

            <!-- Active Status Toggle -->
            <div>
              <label class="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1.5">
                Status Paket
              </label>
              <div class="flex items-center h-[38px]">
                <label class="relative inline-flex items-center cursor-pointer">
                  <input type="checkbox" v-model="form.is_active" class="sr-only peer" />
                  <div class="w-11 h-6 bg-slate-200 dark:bg-slate-800 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-brand-600"></div>
                  <span class="ml-3 text-xs font-medium text-slate-700 dark:text-slate-300">
                    {{ form.is_active ? 'Aktif (Dapat dipilih)' : 'Nonaktif (Diarsipkan)' }}
                  </span>
                </label>
              </div>
            </div>

            <!-- Description -->
            <div>
              <label class="block text-xs font-semibold text-slate-700 dark:text-slate-300 mb-1.5">
                Deskripsi Paket (Opsional)
              </label>
              <textarea
                v-model="form.description"
                rows="2"
                placeholder="Penjelasan sasaran bisnis atau peruntukan tier lisensi ini..."
                class="w-full px-3.5 py-2 text-sm rounded-xl border border-slate-200 dark:border-slate-800 bg-slate-50 dark:bg-slate-950/60 text-slate-900 dark:text-white placeholder-slate-400 focus:outline-none focus:border-brand-500 focus:ring-1 focus:ring-brand-500 transition resize-none"
              ></textarea>
            </div>

            <!-- Features Selection -->
            <div class="pt-2">
              <div class="flex items-center justify-between mb-2">
                <div>
                  <h4 class="text-xs font-bold uppercase tracking-wider text-slate-700 dark:text-slate-300 flex items-center gap-1.5">
                    <Layers class="w-4 h-4 text-brand-500" />
                    <span>Modul Fitur Opsi (Add-ons)</span>
                  </h4>
                  <p class="text-xs text-slate-400">Pilih modul yang secara otomatis disertakan pada tier ini</p>
                </div>
                <div class="flex items-center gap-2">
                  <button
                    type="button"
                    @click="selectAllFeatures"
                    class="text-xs text-brand-600 dark:text-brand-400 hover:underline cursor-pointer flex items-center gap-1"
                  >
                    <CheckSquare class="w-3.5 h-3.5" />
                    <span>Pilih Semua</span>
                  </button>
                  <span class="text-slate-300 dark:text-slate-700">•</span>
                  <button
                    type="button"
                    @click="clearAllFeatures"
                    class="text-xs text-slate-500 hover:underline cursor-pointer flex items-center gap-1"
                  >
                    <Square class="w-3.5 h-3.5" />
                    <span>Hapus Semua</span>
                  </button>
                </div>
              </div>

              <div class="grid grid-cols-1 sm:grid-cols-2 gap-2.5 max-h-60 overflow-y-auto p-1">
                <div
                  v-for="mod in availableModules"
                  :key="mod.id"
                  @click="toggleFeature(mod.id)"
                  :class="[
                    'p-3 rounded-xl border transition cursor-pointer flex items-start gap-3 select-none',
                    form.features.includes(mod.id)
                      ? 'border-brand-500/50 bg-brand-50/50 dark:bg-brand-950/20 text-slate-900 dark:text-white'
                      : 'border-slate-200 dark:border-slate-800 bg-slate-50/60 dark:bg-slate-950/40 text-slate-500 dark:text-slate-400 opacity-70 hover:opacity-100'
                  ]"
                >
                  <input
                    type="checkbox"
                    :checked="form.features.includes(mod.id)"
                    class="mt-0.5 rounded text-brand-600 focus:ring-brand-500 pointer-events-none"
                  />
                  <div>
                    <div class="text-xs font-semibold leading-tight text-slate-900 dark:text-white">
                      {{ mod.name }}
                    </div>
                    <div class="text-[11px] text-slate-400 mt-0.5 leading-snug">
                      {{ mod.desc }}
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <!-- Footer Buttons -->
            <div class="pt-4 border-t border-slate-200 dark:border-slate-800 flex items-center justify-end gap-3">
              <button
                type="button"
                @click="isModalOpen = false"
                class="px-4 py-2 rounded-xl text-sm font-medium text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white transition cursor-pointer"
              >
                Batal
              </button>
              <button
                type="submit"
                :disabled="isSubmitting"
                class="px-5 py-2 rounded-xl text-sm font-medium text-white bg-gradient-to-r from-brand-600 to-indigo-600 hover:from-brand-500 hover:to-indigo-500 transition shadow-md disabled:opacity-50 cursor-pointer"
              >
                {{ isSubmitting ? 'Menyimpan...' : (isEditing ? 'Simpan Perubahan' : 'Buat Tier Lisensi') }}
              </button>
            </div>
          </form>
        </div>
      </div>
    </Teleport>
  </div>
</template>
