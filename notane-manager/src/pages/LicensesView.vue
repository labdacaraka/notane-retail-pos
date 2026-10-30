<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useProviderStore, type License } from '../stores/providerStore'
import { usePermission } from '../composables/usePermission'
import PaginationBar from '../components/common/PaginationBar.vue'
import ClientSearchSelect from '../components/common/ClientSearchSelect.vue'
import {
  KeyRound,
  Plus,
  Copy,
  Check,
  FileSignature,
  Building,
  Calendar,
  Layers,
  X,
  Search,
  CheckCircle2,
  AlertTriangle,
  Ban,
  ArrowUpDown,
  ArrowUp,
  ArrowDown,
  LayoutGrid,
  Table as TableIcon,
  RefreshCw,
  Trash2,
  Sparkles,
  Cpu,
  Unlink,
  Shield,
} from 'lucide-vue-next'
import { z } from 'zod'

const router = useRouter()
const route = useRoute()
const providerStore = useProviderStore()
const { hasPermission } = usePermission()

// Server-side filter & sort
const searchQuery = ref('')
const selectedStatus = ref<'ALL' | 'ACTIVE' | 'SUSPENDED' | 'EXPIRED'>('ALL')
const selectedTier = ref<'ALL' | 'BASIC' | 'PRO' | 'ENTERPRISE'>('ALL')
const selectedClaim = ref<'ALL' | 'CLAIMED' | 'UNCLAIMED'>('ALL')
const sortBy = ref<'created_at' | 'license_key' | 'client_name' | 'tier' | 'expires_at' | 'status'>('created_at')
const sortOrder = ref<'asc' | 'desc'>('desc')
const viewMode = ref<'table' | 'grid'>('table')

// Modal state
const isModalOpen = ref(false)
const isSubmitting = ref(false)
const copiedKey = ref<string | null>(null)
const formErrors = ref<Record<string, string>>({})

const licenseSchema = z.object({
  client_id: z.string().min(1, 'Klien harus dipilih'),
  tier: z.enum(['BASIC', 'PRO', 'ENTERPRISE']),
  max_terminals: z.number().int().min(1).max(1000),
  expires_in_days: z.number().int().min(1),
  grace_period_days: z.number().int().min(0),
  notes: z.string().optional(),
  custom_key: z.string().optional(),
})

const availableFeatures = [
  { id: 'membership', name: 'Pelanggan & Membership', desc: 'Loyalty point, kartu member, & diskon tier member' },
  { id: 'accounting', name: 'Akuntansi & Pembukuan', desc: 'Bagan akun (COA), jurnal umum, neraca & laba rugi' },
  { id: 'multi_outlet', name: 'Multi-Outlet Cabang', desc: 'Manajemen banyak cabang dan transfer stok antar-outlet' },
  { id: 'inventory_batch', name: 'Manajemen Batch & FEFO', desc: 'Nomor batch & rotasi produk berdasarkan kadaluarsa' },
  { id: 'promotions', name: 'Promosi Retail, Voucher & Flash Sale', desc: 'Promo diskon otomatis, voucher belanja & flash sale' },
  { id: 'expenses', name: 'Biaya Operasional Toko', desc: 'Pencatatan pengeluaran kas / petty cash operasional' },
  { id: 'audit_log', name: 'Audit Trail & Log Keamanan', desc: 'Rekaman riwayat aktivitas kasir & supervisor' },
  { id: 'label_printing', name: 'Cetak Label Harga & Barcode Rak', desc: 'Desain dan cetak label barcode produk & rak display' },
  { id: 'remove_branding', name: 'White-Label (Remove Branding)', desc: 'Hilangkan logo dan atribusi Notane pada struk & antarmuka' },
]

function parseFeatures(featuresStr: string | string[]): string[] {
  if (Array.isArray(featuresStr)) return featuresStr
  try {
    return JSON.parse(featuresStr || '[]')
  } catch {
    return []
  }
}

const form = ref({
  client_id: '',
  tier: 'ENTERPRISE' as 'BASIC' | 'PRO' | 'ENTERPRISE',
  max_terminals: 5,
  expires_in_days: 365,
  grace_period_days: 7,
  features: availableFeatures.map(f => f.id),
  notes: '',
  custom_key: '',
})

async function loadData(page?: number, pageSize?: number) {
  await providerStore.fetchLicenses({
    page: page ?? providerStore.licensesPagination.page,
    page_size: pageSize ?? providerStore.licensesPagination.page_size,
    search: searchQuery.value.trim() || undefined,
    status: selectedStatus.value === 'ALL' ? undefined : selectedStatus.value,
    tier: selectedTier.value === 'ALL' ? undefined : selectedTier.value,
    is_claimed: selectedClaim.value === 'ALL' ? undefined : (selectedClaim.value === 'CLAIMED' ? 1 : 0),
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

watch([selectedStatus, selectedTier, selectedClaim], () => {
  loadData(1)
})

onMounted(async () => {
  await providerStore.fetchClients({ page: 1, page_size: 100 })
  await loadData(1)

  if (route.query.action === 'new') {
    openCreateModal()
    if (route.query.client_id) {
      form.value.client_id = String(route.query.client_id)
    }
  }
})

function handleSort(column: 'created_at' | 'license_key' | 'client_name' | 'tier' | 'expires_at' | 'status') {
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
    client_id: route.query.client_id ? String(route.query.client_id) : '',
    tier: 'ENTERPRISE',
    max_terminals: 5,
    expires_in_days: 365,
    grace_period_days: 7,
    features: availableFeatures.map(f => f.id),
    notes: '',
    custom_key: '',
  }
  formErrors.value = {}
  isModalOpen.value = true
}

function toggleFeature(id: string) {
  const idx = form.value.features.indexOf(id)
  if (idx > -1) {
    form.value.features.splice(idx, 1)
  } else {
    form.value.features.push(id)
  }
}

async function handleSubmit() {
  formErrors.value = {}
  const res = licenseSchema.safeParse(form.value)
  if (!res.success) {
    res.error.errors.forEach((err: any) => {
      formErrors.value[err.path[0] as string] = err.message
    })
    return
  }

  isSubmitting.value = true
  const ok = await providerStore.generateLicense({
    ...res.data,
    features: form.value.features,
    custom_key: form.value.custom_key.trim() || undefined,
  })
  isSubmitting.value = false
  if (ok) {
    isModalOpen.value = false
    loadData()
  }
}

async function copyToClipboard(key: string) {
  await navigator.clipboard.writeText(key)
  copiedKey.value = key
  setTimeout(() => (copiedKey.value = null), 2000)
}

function goToOfflineSigner(licenseKey: string) {
  router.push({ path: '/signer', query: { key: licenseKey } })
}

async function toggleStatus(lic: License) {
  const newStatus = lic.status === 'ACTIVE' ? 'SUSPENDED' : 'ACTIVE'
  await providerStore.updateLicenseStatus(lic.id, newStatus)
}

async function confirmUnbindClaim(lic: License) {
  const machineName = lic.claimed_device_name || 'Komputer Sebelumnya'
  const hwid = lic.claimed_hardware_id ? ` (HWID: ${lic.claimed_hardware_id})` : ''
  if (
    confirm(
      `Lepaskan ikatan klaim mesin untuk lisensi '${lic.license_key}'?\n\nMesin '${machineName}'${hwid} akan dilepas dari lisensi ini. Pelanggan selanjutnya dapat mengaktivasi lisensi ini pada komputer baru yang diganti.`
    )
  ) {
    await providerStore.unbindLicenseClaim(lic.id)
  }
}

async function confirmDelete(lic: License) {
  if (confirm(`Yakin ingin menghapus lisensi '${lic.license_key}'? Mesin kasir terkait tidak akan dapat bertransaksi.`)) {
    await providerStore.deleteLicense(lic.id)
  }
}
</script>

<template>
  <div class="space-y-5">
    <!-- Top Bar Controls -->
    <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
      <div class="flex flex-wrap items-center gap-2 w-full sm:w-auto">
        <!-- Search Input -->
        <div class="relative w-full sm:w-72">
          <Search class="w-4 h-4 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2" />
          <input
            v-model="searchQuery"
            type="text"
            placeholder="Cari lisensi, nama klien, catatan..."
            class="w-full pl-9 pr-4 py-2 bg-slate-900 border border-slate-800 rounded-xl text-xs text-slate-200 placeholder-slate-500 focus:outline-none focus:border-brand-500 transition shadow-inner"
          />
        </div>

        <!-- Reload -->
        <button
          @click="loadData()"
          :disabled="providerStore.isLoading"
          title="Muat Ulang Data"
          class="p-2 bg-slate-900 border border-slate-800 hover:bg-slate-800 text-slate-300 rounded-xl transition cursor-pointer"
        >
          <RefreshCw class="w-4 h-4" :class="{ 'animate-spin': providerStore.isLoading }" />
        </button>

        <!-- Status Filter Buttons -->
        <div class="flex items-center bg-slate-900 border border-slate-800 rounded-xl p-1 gap-1">
          <button
            v-for="flt in (['ALL', 'ACTIVE', 'SUSPENDED', 'EXPIRED'] as const)"
            :key="flt"
            @click="selectedStatus = flt"
            class="px-2.5 py-1 rounded-lg text-xs font-medium transition-colors cursor-pointer"
            :class="selectedStatus === flt ? 'bg-brand-500 text-white font-semibold' : 'text-slate-400 hover:text-slate-200'"
          >
            {{ flt }}
          </button>
        </div>

        <!-- Tier Filter -->
        <select
          v-model="selectedTier"
          class="bg-slate-900 border border-slate-800 text-slate-300 rounded-xl px-2.5 py-1.5 text-xs focus:outline-none focus:border-brand-500 cursor-pointer"
        >
          <option value="ALL">Semua Tier</option>
          <option value="BASIC">BASIC</option>
          <option value="PRO">PRO</option>
          <option value="ENTERPRISE">ENTERPRISE</option>
        </select>

        <!-- Status Klaim Filter -->
        <select
          v-model="selectedClaim"
          class="bg-slate-900 border border-slate-800 text-slate-300 rounded-xl px-2.5 py-1.5 text-xs focus:outline-none focus:border-brand-500 cursor-pointer"
        >
          <option value="ALL">Semua Klaim</option>
          <option value="CLAIMED">Terklaim Mesin</option>
          <option value="UNCLAIMED">Belum Diklaim</option>
        </select>
      </div>

      <div class="flex items-center gap-2 self-end sm:self-auto">
        <!-- View Mode Switcher -->
        <div class="flex items-center bg-slate-900 border border-slate-800 rounded-xl p-1 text-slate-400">
          <button
            @click="viewMode = 'table'"
            :class="viewMode === 'table' ? 'bg-slate-800 text-white font-bold' : 'hover:text-slate-200'"
            class="px-2.5 py-1 rounded-lg text-xs flex items-center gap-1.5 transition cursor-pointer"
          >
            <TableIcon class="w-3.5 h-3.5" />
            <span>Tabel</span>
          </button>
          <button
            @click="viewMode = 'grid'"
            :class="viewMode === 'grid' ? 'bg-slate-800 text-white font-bold' : 'hover:text-slate-200'"
            class="px-2.5 py-1 rounded-lg text-xs flex items-center gap-1.5 transition cursor-pointer"
          >
            <LayoutGrid class="w-3.5 h-3.5" />
            <span>Kartu</span>
          </button>
        </div>

        <button
          @click="openCreateModal"
          :disabled="!hasPermission('license:create')"
          class="px-4 py-2 bg-brand-600 hover:bg-brand-500 disabled:opacity-50 text-white rounded-xl text-xs font-semibold flex items-center gap-1.5 transition-all shadow-md shadow-brand-600/20 cursor-pointer"
        >
          <Plus class="w-4 h-4" />
          <span>Terbitkan Lisensi Baru</span>
        </button>
      </div>
    </div>

    <!-- Empty State -->
    <div
      v-if="!providerStore.isLoading && providerStore.licensesPagination.total_items === 0"
      class="py-16 text-center rounded-2xl glass-panel border border-slate-800"
    >
      <KeyRound class="w-12 h-12 text-slate-600 mx-auto mb-3" />
      <div class="text-sm font-semibold text-slate-300">Belum Ada Lisensi Sesuai Kriteria</div>
      <div class="text-xs text-slate-500 mt-1">
        {{ searchQuery || selectedStatus !== 'ALL' || selectedTier !== 'ALL' ? 'Coba ubah kata kunci atau filter pencarian Anda.' : 'Buat lisensi baru untuk diberikan kepada outlet klien Anda.' }}
      </div>
      <button
        v-if="!searchQuery && selectedStatus === 'ALL' && selectedTier === 'ALL'"
        @click="openCreateModal"
        class="mt-4 px-4 py-2 bg-brand-600 hover:bg-brand-500 text-white rounded-xl text-xs font-semibold cursor-pointer"
      >
        Terbitkan Lisensi Pertama
      </button>
      <button
        v-else
        @click="searchQuery = ''; selectedStatus = 'ALL'; selectedTier = 'ALL'; loadData(1)"
        class="mt-4 px-4 py-2 bg-slate-800 hover:bg-slate-700 text-white rounded-xl text-xs font-semibold cursor-pointer"
      >
        Reset Filter
      </button>
    </div>

    <!-- TABLE VIEW (SERVER-SIDE SORTING & PAGING) -->
    <div v-else-if="viewMode === 'table'" class="space-y-3">
      <div class="glass-panel border border-slate-800 rounded-2xl overflow-hidden shadow-xl">
        <div class="overflow-x-auto">
          <table class="w-full text-left border-collapse text-xs">
            <thead>
              <tr class="bg-slate-900/90 text-slate-400 border-b border-slate-800 select-none">
                <th
                  @click="handleSort('license_key')"
                  class="py-3 px-4 font-semibold cursor-pointer hover:text-white transition"
                >
                  <div class="flex items-center gap-1.5">
                    <span>Nomor Lisensi</span>
                    <ArrowUp v-if="sortBy === 'license_key' && sortOrder === 'asc'" class="w-3.5 h-3.5 text-brand-400" />
                    <ArrowDown v-else-if="sortBy === 'license_key' && sortOrder === 'desc'" class="w-3.5 h-3.5 text-brand-400" />
                    <ArrowUpDown v-else class="w-3 h-3 text-slate-600" />
                  </div>
                </th>

                <th
                  @click="handleSort('client_name')"
                  class="py-3 px-4 font-semibold cursor-pointer hover:text-white transition"
                >
                  <div class="flex items-center gap-1.5">
                    <span>Pemilik / Klien</span>
                    <ArrowUp v-if="sortBy === 'client_name' && sortOrder === 'asc'" class="w-3.5 h-3.5 text-brand-400" />
                    <ArrowDown v-else-if="sortBy === 'client_name' && sortOrder === 'desc'" class="w-3.5 h-3.5 text-brand-400" />
                    <ArrowUpDown v-else class="w-3 h-3 text-slate-600" />
                  </div>
                </th>

                <th
                  @click="handleSort('tier')"
                  class="py-3 px-4 font-semibold cursor-pointer hover:text-white transition"
                >
                  <div class="flex items-center gap-1.5">
                    <span>Tier</span>
                    <ArrowUp v-if="sortBy === 'tier' && sortOrder === 'asc'" class="w-3.5 h-3.5 text-brand-400" />
                    <ArrowDown v-else-if="sortBy === 'tier' && sortOrder === 'desc'" class="w-3.5 h-3.5 text-brand-400" />
                    <ArrowUpDown v-else class="w-3 h-3 text-slate-600" />
                  </div>
                </th>

                <th class="py-3 px-4 font-semibold text-slate-400">Kapasitas Kasir (Per Server)</th>

                <th class="py-3 px-4 font-semibold text-slate-400">
                  <div class="flex items-center gap-1.5">
                    <Cpu class="w-3.5 h-3.5 text-slate-500" />
                    <span>Klaim Mesin (HWID)</span>
                  </div>
                </th>

                <th
                  @click="handleSort('status')"
                  class="py-3 px-4 font-semibold cursor-pointer hover:text-white transition"
                >
                  <div class="flex items-center gap-1.5">
                    <span>Status</span>
                    <ArrowUp v-if="sortBy === 'status' && sortOrder === 'asc'" class="w-3.5 h-3.5 text-brand-400" />
                    <ArrowDown v-else-if="sortBy === 'status' && sortOrder === 'desc'" class="w-3.5 h-3.5 text-brand-400" />
                    <ArrowUpDown v-else class="w-3 h-3 text-slate-600" />
                  </div>
                </th>

                <th
                  @click="handleSort('expires_at')"
                  class="py-3 px-4 font-semibold cursor-pointer hover:text-white transition"
                >
                  <div class="flex items-center gap-1.5">
                    <span>Kadaluarsa</span>
                    <ArrowUp v-if="sortBy === 'expires_at' && sortOrder === 'asc'" class="w-3.5 h-3.5 text-brand-400" />
                    <ArrowDown v-else-if="sortBy === 'expires_at' && sortOrder === 'desc'" class="w-3.5 h-3.5 text-brand-400" />
                    <ArrowUpDown v-else class="w-3 h-3 text-slate-600" />
                  </div>
                </th>

                <th class="py-3 px-4 font-semibold text-right text-slate-400">Aksi</th>
              </tr>
            </thead>

            <tbody class="divide-y divide-slate-800/60">
              <tr
                v-for="lic in providerStore.licensesPagination.items"
                :key="lic.id"
                class="hover:bg-slate-800/40 transition-colors group"
              >
                <!-- License Key -->
                <td class="py-3.5 px-4 font-medium text-slate-200">
                  <div class="flex items-center gap-2">
                    <span class="font-mono text-xs font-bold text-white bg-slate-900/90 px-2 py-0.5 rounded border border-slate-700/60 select-all">
                      {{ lic.license_key }}
                    </span>
                    <button
                      @click="copyToClipboard(lic.license_key)"
                      class="p-1 rounded bg-slate-800 hover:bg-slate-700 text-slate-300 transition-colors cursor-pointer"
                      title="Salin Nomor Lisensi"
                    >
                      <component :is="copiedKey === lic.license_key ? Check : Copy" class="w-3 h-3 text-brand-400" />
                    </button>
                  </div>
                </td>

                <!-- Client -->
                <td class="py-3.5 px-4 text-slate-300">
                  <div class="flex items-center gap-1.5">
                    <Building class="w-3.5 h-3.5 text-slate-500 shrink-0" />
                    <span class="font-semibold text-white">{{ lic.client_name || 'Klien Umum' }}</span>
                  </div>
                </td>

                <!-- Tier -->
                <td class="py-3.5 px-4">
                  <div class="flex items-center gap-1.5 flex-wrap">
                    <span class="text-[10px] font-bold px-2 py-0.5 rounded bg-brand-500/10 text-brand-400 font-mono border border-brand-500/20">
                      {{ lic.tier }}
                    </span>
                    <span
                      v-if="parseFeatures(lic.features).includes('remove_branding')"
                      class="text-[9px] font-bold px-1.5 py-0.5 rounded bg-purple-500/20 text-purple-300 border border-purple-500/40 font-mono flex items-center gap-1"
                      title="Fitur White-Label Aktif (Tanpa Logo Notane)"
                    >
                      <Sparkles class="w-2.5 h-2.5" />
                      White-Label
                    </span>
                  </div>
                </td>

                <!-- Terminals Capacity -->
                <td class="py-3.5 px-4 font-mono text-slate-300">
                  <span class="font-bold text-white">Maks. {{ lic.max_terminals }}</span>
                  <span class="text-slate-500"> Kasir / Server</span>
                </td>

                <!-- Machine Claim Status -->
                <td class="py-3.5 px-4">
                  <div v-if="lic.is_claimed === 1" class="flex flex-col gap-0.5">
                    <div class="flex items-center gap-1.5">
                      <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-bold bg-amber-500/20 text-amber-300 border border-amber-500/40">
                        <Cpu class="w-3 h-3 text-amber-400" />
                        Terklaim
                      </span>
                    </div>
                    <span v-if="lic.claimed_device_name" class="text-[10px] text-slate-300 font-medium truncate max-w-[130px]" :title="lic.claimed_device_name">
                      {{ lic.claimed_device_name }}
                    </span>
                    <span v-if="lic.claimed_hardware_id" class="font-mono text-[9px] text-slate-500 truncate max-w-[120px]" :title="lic.claimed_hardware_id">
                      {{ lic.claimed_hardware_id }}
                    </span>
                  </div>
                  <div v-else>
                    <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
                      <CheckCircle2 class="w-3 h-3 text-emerald-400" />
                      Belum Diklaim
                    </span>
                  </div>
                </td>

                <!-- Status -->
                <td class="py-3.5 px-4">
                  <span
                    class="px-2.5 py-0.5 rounded-full text-[10px] font-bold uppercase tracking-wider border inline-block"
                    :class="[
                      lic.status === 'ACTIVE' ? 'bg-emerald-500/20 text-emerald-400 border-emerald-500/30' :
                      lic.status === 'SUSPENDED' ? 'bg-amber-500/20 text-amber-400 border-amber-500/30' :
                      'bg-rose-500/20 text-rose-400 border-rose-500/30'
                    ]"
                  >
                    {{ lic.status }}
                  </span>
                </td>

                <!-- Expires At -->
                <td class="py-3.5 px-4 font-mono text-[11px] text-slate-400 whitespace-nowrap">
                  {{ new Date(lic.expires_at).toLocaleDateString('id-ID', { year: 'numeric', month: 'short', day: 'numeric' }) }}
                </td>

                <!-- Actions -->
                <td class="py-3.5 px-4 text-right">
                  <div class="flex items-center justify-end gap-1.5">
                    <button
                      v-if="lic.is_claimed === 1"
                      @click="confirmUnbindClaim(lic)"
                      :disabled="!hasPermission('license:unbind_claim') && !hasPermission('license:update')"
                      title="Lepas Klaim Mesin (Ganti Komputer/Mesin Rusak)"
                      class="px-2 py-1 bg-amber-500/10 hover:bg-amber-500/20 text-amber-400 border border-amber-500/30 rounded-lg text-[11px] font-semibold flex items-center gap-1 transition cursor-pointer"
                    >
                      <Unlink class="w-3 h-3" />
                      <span>Lepas Klaim</span>
                    </button>

                    <button
                      @click="goToOfflineSigner(lic.license_key)"
                      title="Sign Token Kriptografis Offline"
                      class="px-2 py-1 bg-slate-800 hover:bg-slate-700 text-brand-400 rounded-lg text-[11px] font-semibold flex items-center gap-1 transition cursor-pointer"
                    >
                      <FileSignature class="w-3 h-3" />
                      <span>Sign</span>
                    </button>

                    <button
                      @click="toggleStatus(lic)"
                      :disabled="!hasPermission('license:update')"
                      :title="lic.status === 'ACTIVE' ? 'Suspend Lisensi' : 'Aktifkan Kembali'"
                      class="p-1.5 rounded-lg text-xs font-semibold transition border cursor-pointer"
                      :class="[
                        lic.status === 'ACTIVE'
                          ? 'bg-rose-500/10 hover:bg-rose-500/20 text-rose-400 border-rose-500/30'
                          : 'bg-emerald-500/10 hover:bg-emerald-500/20 text-emerald-400 border-emerald-500/30'
                      ]"
                    >
                      <Ban v-if="lic.status === 'ACTIVE'" class="w-3.5 h-3.5" />
                      <CheckCircle2 v-else class="w-3.5 h-3.5" />
                    </button>

                    <button
                      @click="confirmDelete(lic)"
                      :disabled="!hasPermission('license:revoke')"
                      title="Hapus / Cabut Lisensi"
                      class="p-1.5 text-slate-400 hover:text-rose-400 hover:bg-rose-500/10 rounded-lg transition cursor-pointer"
                    >
                      <Trash2 class="w-3.5 h-3.5" />
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <!-- Pagination Component -->
      <PaginationBar
        :page="providerStore.licensesPagination.page"
        :page-size="providerStore.licensesPagination.page_size"
        :total-items="providerStore.licensesPagination.total_items"
        :total-pages="providerStore.licensesPagination.total_pages"
        :is-loading="providerStore.isLoading"
        @update:page="(p) => loadData(p, providerStore.licensesPagination.page_size)"
        @update:page-size="(s) => loadData(1, s)"
      />
    </div>

    <!-- GRID VIEW (CARDS) -->
    <div v-else class="space-y-4">
      <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div
          v-for="lic in providerStore.licensesPagination.items"
          :key="lic.id"
          class="p-5 rounded-2xl glass-panel border border-slate-800 hover:border-slate-700 transition-all flex flex-col justify-between space-y-4"
        >
          <div class="space-y-3">
            <!-- Header & Key -->
            <div class="flex items-start justify-between gap-2">
              <div>
                <div class="flex items-center gap-2">
                  <span class="font-mono text-sm font-bold text-white tracking-wide bg-slate-900/90 px-2.5 py-1 rounded-lg border border-slate-700/60 select-all">
                    {{ lic.license_key }}
                  </span>
                  <button
                    @click="copyToClipboard(lic.license_key)"
                    class="p-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 text-slate-300 transition-colors cursor-pointer"
                    title="Salin Nomor Lisensi"
                  >
                    <component :is="copiedKey === lic.license_key ? Check : Copy" class="w-3.5 h-3.5 text-brand-400" />
                  </button>
                </div>

                <div class="mt-2 flex items-center gap-2 text-xs text-slate-400">
                  <Building class="w-3.5 h-3.5 text-slate-500" />
                  <span class="font-semibold text-slate-200">{{ lic.client_name || 'Klien Umum' }}</span>
                </div>
              </div>

              <div class="flex flex-col items-end gap-1.5">
                <span
                  class="px-2.5 py-0.5 rounded-full text-[10px] font-bold uppercase tracking-wider border"
                  :class="[
                    lic.status === 'ACTIVE' ? 'bg-emerald-500/20 text-emerald-400 border-emerald-500/30' :
                    lic.status === 'SUSPENDED' ? 'bg-amber-500/20 text-amber-400 border-amber-500/30' :
                    'bg-rose-500/20 text-rose-400 border-rose-500/30'
                  ]"
                >
                  {{ lic.status }}
                </span>
                <div class="flex items-center gap-1.5 flex-wrap">
                  <span class="text-[10px] font-bold px-2 py-0.5 rounded bg-brand-500/10 text-brand-400 font-mono">
                    {{ lic.tier }}
                  </span>
                  <span
                    v-if="parseFeatures(lic.features).includes('remove_branding')"
                    class="text-[9px] font-bold px-1.5 py-0.5 rounded bg-purple-500/20 text-purple-300 border border-purple-500/40 font-mono flex items-center gap-1"
                    title="Fitur White-Label Aktif (Tanpa Logo Notane)"
                  >
                    <Sparkles class="w-2.5 h-2.5" />
                    White-Label
                  </span>
                </div>
              </div>
            </div>

            <!-- Metadata Info -->
            <div class="grid grid-cols-2 gap-2 p-3 rounded-xl bg-slate-950/60 text-xs">
              <div>
                <span class="text-slate-500 block text-[10px]">Kapasitas Klien Kasir</span>
                <span class="font-bold text-slate-200 font-mono">Maks. {{ lic.max_terminals }} Kasir / Server</span>
              </div>
              <div>
                <span class="text-slate-500 block text-[10px]">Masa Berlaku s/d</span>
                <span class="font-bold text-slate-200 font-mono">{{ new Date(lic.expires_at).toLocaleDateString() }}</span>
              </div>
            </div>

            <!-- Claim Node Status in Card -->
            <div class="flex items-center justify-between p-2.5 rounded-xl bg-slate-950/40 border border-slate-800/60 text-xs">
              <div class="flex items-center gap-2">
                <Cpu class="w-4 h-4 text-slate-400" />
                <span class="text-slate-400">Status Klaim Mesin:</span>
              </div>
              <div v-if="lic.is_claimed === 1" class="flex items-center gap-1.5">
                <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-bold bg-amber-500/20 text-amber-300 border border-amber-500/40">
                  Terklaim
                </span>
                <span v-if="lic.claimed_device_name" class="text-[11px] font-semibold text-slate-200 truncate max-w-[120px]" :title="lic.claimed_device_name">
                  {{ lic.claimed_device_name }}
                </span>
              </div>
              <div v-else>
                <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
                  <CheckCircle2 class="w-3 h-3 text-emerald-400" />
                  Belum Diklaim Mesin
                </span>
              </div>
            </div>
          </div>

          <!-- Action Buttons -->
          <div class="pt-3 border-t border-slate-800/80 flex items-center justify-between">
            <button
              @click="goToOfflineSigner(lic.license_key)"
              class="px-3 py-1.5 rounded-lg bg-slate-800 hover:bg-slate-700 text-brand-400 text-xs font-semibold flex items-center gap-1.5 transition-colors cursor-pointer"
            >
              <FileSignature class="w-3.5 h-3.5" />
              <span>Sign Token Offline</span>
            </button>

            <div class="flex items-center gap-1.5">
              <button
                v-if="lic.is_claimed === 1"
                @click="confirmUnbindClaim(lic)"
                :disabled="!hasPermission('license:unbind_claim') && !hasPermission('license:update')"
                title="Lepas Klaim Mesin (Ganti Komputer)"
                class="px-2.5 py-1.5 rounded-lg text-xs font-semibold bg-amber-500/10 hover:bg-amber-500/20 text-amber-400 border border-amber-500/30 flex items-center gap-1 transition-colors cursor-pointer"
              >
                <Unlink class="w-3.5 h-3.5" />
                <span>Lepas Klaim</span>
              </button>

              <button
                @click="toggleStatus(lic)"
                :disabled="!hasPermission('license:update')"
                class="px-3 py-1.5 rounded-lg text-xs font-semibold transition-colors border cursor-pointer"
                :class="[
                  lic.status === 'ACTIVE'
                    ? 'bg-rose-500/10 hover:bg-rose-500/20 text-rose-400 border-rose-500/30'
                    : 'bg-emerald-500/10 hover:bg-emerald-500/20 text-emerald-400 border-emerald-500/30'
                ]"
              >
                {{ lic.status === 'ACTIVE' ? 'Suspend Lisensi' : 'Aktifkan Kembali' }}
              </button>

              <button
                @click="confirmDelete(lic)"
                :disabled="!hasPermission('license:revoke')"
                title="Hapus Lisensi"
                class="p-1.5 text-slate-400 hover:text-rose-400 hover:bg-rose-500/10 rounded-lg transition cursor-pointer"
              >
                <Trash2 class="w-3.5 h-3.5" />
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- Pagination Component -->
      <PaginationBar
        :page="providerStore.licensesPagination.page"
        :page-size="providerStore.licensesPagination.page_size"
        :total-items="providerStore.licensesPagination.total_items"
        :total-pages="providerStore.licensesPagination.total_pages"
        :is-loading="providerStore.isLoading"
        @update:page="(p) => loadData(p, providerStore.licensesPagination.page_size)"
        @update:page-size="(s) => loadData(1, s)"
      />
    </div>

    <!-- Modal Buat Lisensi Baru -->
    <div v-if="isModalOpen" class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/80 backdrop-blur-sm animate-in fade-in">
      <div class="w-full max-w-lg rounded-2xl glass-panel border border-slate-800 bg-slate-900 p-6 shadow-2xl space-y-4 max-h-[90vh] overflow-y-auto">
        <div class="flex items-center justify-between pb-3 border-b border-slate-800">
          <div class="font-bold text-sm text-white">Terbitkan Lisensi Baru</div>
          <button @click="isModalOpen = false" class="text-slate-400 hover:text-white cursor-pointer">
            <X class="w-4 h-4" />
          </button>
        </div>

        <form @submit.prevent="handleSubmit" class="space-y-4">
          <!-- Pilih Klien -->
          <div>
            <label class="block text-xs font-semibold text-slate-300 mb-1">Pilih Klien Tenant *</label>
            <ClientSearchSelect
              v-model="form.client_id"
              :error="formErrors.client_id"
              placeholder="-- Cari & Pilih Klien Tenant --"
            />
          </div>

          <!-- Tier Lisensi -->
          <div>
            <label class="block text-xs font-semibold text-slate-300 mb-1">Tier Lisensi *</label>
            <div class="grid grid-cols-3 gap-2">
              <button
                v-for="t in (['BASIC', 'PRO', 'ENTERPRISE'] as const)"
                :key="t"
                type="button"
                @click="form.tier = t"
                class="py-2 rounded-xl text-xs font-semibold border transition-all cursor-pointer"
                :class="form.tier === t ? 'bg-brand-500 text-white border-brand-400 shadow-sm' : 'bg-slate-950 text-slate-400 border-slate-800 hover:border-slate-700'"
              >
                {{ t }}
              </button>
            </div>
          </div>

          <!-- Kuota Terminal & Masa Berlaku -->
          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="block text-xs font-semibold text-slate-300 mb-1">Kapasitas Terminal Kasir Klien (notane-client) *</label>
              <input
                v-model.number="form.max_terminals"
                type="number"
                min="1"
                max="1000"
                class="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-xl text-xs text-white focus:outline-none focus:border-brand-500 font-mono"
              />
              <span class="text-[10px] text-slate-400 mt-1 block">Batas maksimal komputer kasir client (notane-client) yang dapat terhubung bersamaan ke 1 notane-server.</span>
            </div>
            <div>
              <label class="block text-xs font-semibold text-slate-300 mb-1">Masa Berlaku (Hari)</label>
              <input
                v-model.number="form.expires_in_days"
                type="number"
                min="1"
                class="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-xl text-xs text-white focus:outline-none focus:border-brand-500 font-mono"
              />
            </div>
          </div>

          <!-- Grace Period -->
          <div>
            <label class="block text-xs font-semibold text-slate-300 mb-1">Toleransi Grace Period Offline (Hari)</label>
            <input
              v-model.number="form.grace_period_days"
              type="number"
              min="0"
              placeholder="Contoh: 7 hari toleransi offline"
              class="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-xl text-xs text-white focus:outline-none focus:border-brand-500 font-mono"
            />
          </div>

          <!-- Fitur Diaktifkan -->
          <div>
            <div class="flex items-center justify-between mb-1.5">
              <label class="block text-xs font-semibold text-slate-300">Modul Fitur Opsi (Add-ons)</label>
              <span class="text-[10px] text-slate-500 font-mono">{{ form.features.length }} Modul Dipilih</span>
            </div>
            <div class="p-2.5 rounded-xl bg-slate-950/70 border border-slate-800 text-[11px] text-slate-400 mb-2.5 flex items-start gap-2">
              <ShieldCheck class="w-4 h-4 text-emerald-400 shrink-0 mt-0.5" />
              <span>
                <strong class="text-slate-200">Fitur Inti / Core POS:</strong> POS Kasir & Struk, Master Produk, Pembelian Dasar, Shift Kasir, Pengguna & Mode Offline selalu aktif permanen secara default.
              </span>
            </div>
            <div class="grid grid-cols-1 md:grid-cols-2 gap-2">
              <label
                v-for="feat in availableFeatures"
                :key="feat.id"
                class="flex items-start gap-2.5 p-2.5 rounded-xl border text-xs cursor-pointer transition-colors"
                :class="form.features.includes(feat.id)
                  ? feat.id === 'remove_branding'
                    ? 'bg-purple-500/15 border-purple-500/50 text-purple-300 font-semibold'
                    : 'bg-brand-500/10 border-brand-500/40 text-brand-300'
                  : 'bg-slate-950 border-slate-800 text-slate-400'"
              >
                <input
                  type="checkbox"
                  :checked="form.features.includes(feat.id)"
                  @change="toggleFeature(feat.id)"
                  class="rounded border-slate-700 bg-slate-900 text-brand-600 focus:ring-0 mt-0.5"
                />
                <div class="space-y-0.5">
                  <div class="flex items-center gap-1.5 font-medium">
                    <Sparkles v-if="feat.id === 'remove_branding'" class="w-3.5 h-3.5 text-purple-400" />
                    <span>{{ feat.name }}</span>
                  </div>
                  <div class="text-[10px] text-slate-500 leading-tight">{{ feat.desc }}</div>
                </div>
              </label>
            </div>
            <p v-if="form.features.includes('remove_branding')" class="mt-2 text-[11px] text-purple-300/90 bg-purple-950/40 border border-purple-500/30 p-2.5 rounded-xl flex items-start gap-2">
              <Shield class="w-4 h-4 text-purple-400 shrink-0 mt-0.5" />
              <span><strong>White-Label:</strong> Menghilangkan logo dan atribusi Notane dari aplikasi kasir, layar pelanggan, dan back-office.</span>
            </p>
          </div>

          <!-- Custom License Key (Optional) -->
          <div>
            <label class="block text-xs font-semibold text-slate-300 mb-1">Nomor Lisensi Kustom (Opsional, kosongkan jika ingin dibuat otomatis)</label>
            <input
              v-model="form.custom_key"
              type="text"
              placeholder="Contoh: NOTANE-ENT-2026-9999"
              class="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-xl text-xs text-white focus:outline-none focus:border-brand-500 font-mono"
            />
          </div>

          <!-- Catatan Internal -->
          <div>
            <label class="block text-xs font-semibold text-slate-300 mb-1">Catatan Internal</label>
            <textarea
              v-model="form.notes"
              rows="2"
              placeholder="Contoh: Pembayaran lisensi via Transfer BCA Tahap 1"
              class="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-xl text-xs text-white focus:outline-none focus:border-brand-500"
            ></textarea>
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
              {{ isSubmitting ? 'Menerbitkan...' : 'Terbitkan & Tanda Tangani' }}
            </button>
          </div>
        </form>
      </div>
    </div>
  </div>
</template>
