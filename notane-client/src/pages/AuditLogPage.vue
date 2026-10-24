<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue'
import AdminLayout from '@/layouts/AdminLayout.vue'
import PaginationBar from '@/components/common/PaginationBar.vue'
import {
  Search,
  RefreshCw,
  Settings,
  Trash2,
  Eye,
  Copy,
  Check,
  X,
  Clock,
  User,
  Shield,
} from 'lucide-vue-next'
import { Icon } from '@iconify/vue'
import { api } from '@/services/api'
import { useDialog } from '@/composables/useDialog'

const dialog = useDialog()

// States Data
const logs = ref<any[]>([])
const isLoading = ref<boolean>(false)
const totalItems = ref<number>(0)
const retentionDays = ref<number>(30)

// Filters
const page = ref<number>(1)
const pageSize = ref<number>(20)
const searchQuery = ref<string>('')
const selectedMethod = ref<string>('ALL')
const selectedStatus = ref<string>('all')
const selectedEntity = ref<string>('all')

// Modals
const showDetailModal = ref<boolean>(false)
const selectedLog = ref<any | null>(null)
const copiedPayload = ref<boolean>(false)

const showSettingsModal = ref<boolean>(false)
const isSavingSettings = ref<boolean>(false)
const settingsRetentionDays = ref<number>(30)
const totalLogsCount = ref<number>(0)
const oldestLogDate = ref<string | null>(null)
const isCleaningUp = ref<boolean>(false)

const entityList = [
  { value: 'all', label: 'Semua Modul' },
  { value: 'pos', label: 'Kasir & POS' },
  { value: 'auth', label: 'Otentikasi & Akun' },
  { value: 'products', label: 'Master Produk' },
  { value: 'inventory', label: 'Inventori & Stok' },
  { value: 'vouchers', label: 'Voucher & Kuota' },
  { value: 'promotions', label: 'Promosi Ritel' },
  { value: 'flash_sales', label: 'Flash Sale' },
  { value: 'users', label: 'Pengguna & Hak Akses' },
  { value: 'expenses', label: 'Biaya Operasional' },
  { value: 'capital', label: 'Setoran Modal' },
  { value: 'banks', label: 'Rekening Bank' },
  { value: 'reports', label: 'Laporan Analitik' },
  { value: 'system', label: 'Sistem Toko' },
]

async function fetchAuditLogs() {
  isLoading.value = true
  try {
    const params: Record<string, any> = {
      page: page.value,
      limit: pageSize.value,
    }

    if (searchQuery.value.trim()) {
      params.search = searchQuery.value.trim()
    }
    if (selectedMethod.value !== 'ALL') {
      params.method = selectedMethod.value
    }
    if (selectedStatus.value !== 'all') {
      params.status_type = selectedStatus.value
    }
    if (selectedEntity.value !== 'all') {
      params.entity_name = selectedEntity.value
    }

    const res = await api.get('/auth/audit-logs', { params })
    if (res.data.success) {
      logs.value = res.data.data || []
      totalItems.value = res.data.total ?? logs.value.length
      if (res.data.retention_days !== undefined) {
        retentionDays.value = res.data.retention_days
      }
    }
  } catch (err: any) {
    console.error('Gagal mengambil audit logs:', err)
    dialog.toast('Gagal memuat jejak audit dari server', 'danger')
  } finally {
    isLoading.value = false
  }
}

async function fetchSettings() {
  try {
    const res = await api.get('/auth/audit-logs/settings')
    if (res.data.success) {
      const data = res.data.data
      settingsRetentionDays.value = data.retention_days
      retentionDays.value = data.retention_days
      totalLogsCount.value = data.total_logs
      oldestLogDate.value = data.oldest_log_date
    }
  } catch (err) {
    console.error('Gagal memuat pengaturan retensi:', err)
  }
}

function openSettingsModal() {
  fetchSettings()
  showSettingsModal.value = true
}

async function saveSettings() {
  isSavingSettings.value = true
  try {
    const res = await api.put('/auth/audit-logs/settings', {
      retention_days: settingsRetentionDays.value,
    })
    if (res.data.success) {
      retentionDays.value = settingsRetentionDays.value
      dialog.toast('Pengaturan retensi log berhasil disimpan', 'success')
      showSettingsModal.value = false
    }
  } catch (err: any) {
    dialog.toast(err.response?.data?.message || 'Gagal menyimpan pengaturan retensi', 'danger')
  } finally {
    isSavingSettings.value = false
  }
}

const cleanupDays = ref<number>(1)

async function handleManualCleanup(customDays?: number) {
  const targetDays = customDays !== undefined ? customDays : cleanupDays.value
  const isResetAll = targetDays === 0

  const confirmed = await dialog.confirm({
    title: isResetAll ? 'Kosongkan Seluruh Riwayat Log' : 'Pembersihan Log Berdasarkan Umur',
    message: isResetAll
      ? 'PERINGATAN: Tindakan ini akan MENGHAPUS SEMUA riwayat jejak audit di basis data. Anda yakin ingin melanjutkan?'
      : `Apakah Anda yakin ingin menghapus seluruh log aktivitas yang berumur lebih dari ${targetDays} hari yang lalu?`,
    confirmText: isResetAll ? 'Ya, Kosongkan Semua' : 'Ya, Bersihkan Sekarang',
    cancelText: 'Batal',
    type: 'danger',
  })

  if (!confirmed) return

  isCleaningUp.value = true
  try {
    const res = await api.post('/auth/audit-logs/cleanup', null, {
      params: { days: targetDays },
    })
    if (res.data.success) {
      if (res.data.deleted_count === 0) {
        dialog.toast(res.data.message || 'Tidak ada data log yang dihapus', 'warning')
      } else {
        dialog.toast(res.data.message || 'Pembersihan log berhasil', 'success')
      }
      await fetchSettings()
      page.value = 1
      await fetchAuditLogs()
    }
  } catch (err: any) {
    dialog.toast(err.response?.data?.message || 'Gagal membersihkan log audit', 'danger')
  } finally {
    isCleaningUp.value = false
  }
}

function viewDetail(item: any) {
  selectedLog.value = item
  copiedPayload.value = false
  showDetailModal.value = true
}

function copyJsonPayload() {
  if (!selectedLog.value?.log?.new_values) return
  const text = JSON.stringify(selectedLog.value.log.new_values, null, 2)
  navigator.clipboard.writeText(text).then(() => {
    copiedPayload.value = true
    dialog.toast('Data JSON berhasil disalin ke clipboard', 'info', 2000)
    setTimeout(() => {
      copiedPayload.value = false
    }, 2500)
  })
}

// Watch pagination & filters
watch([page, pageSize, selectedMethod, selectedStatus, selectedEntity], () => {
  fetchAuditLogs()
})

let searchTimeout: any = null
function onSearchInput() {
  clearTimeout(searchTimeout)
  searchTimeout = setTimeout(() => {
    page.value = 1
    fetchAuditLogs()
  }, 400)
}

function resetFilters() {
  searchQuery.value = ''
  selectedMethod.value = 'ALL'
  selectedStatus.value = 'all'
  selectedEntity.value = 'all'
  page.value = 1
  fetchAuditLogs()
}

// Format Helpers
function formatDate(dateStr: string) {
  if (!dateStr) return '-'
  const d = new Date(dateStr)
  return d.toLocaleString('id-ID', {
    day: '2-digit',
    month: 'short',
    year: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  })
}

function getMethodClass(method: string | null) {
  switch (method?.toUpperCase()) {
    case 'POST':
      return 'bg-emerald-100 text-emerald-800 border-emerald-300 font-bold'
    case 'PUT':
    case 'PATCH':
      return 'bg-amber-100 text-amber-800 border-amber-300 font-bold'
    case 'DELETE':
      return 'bg-rose-100 text-rose-800 border-rose-300 font-bold'
    case 'GET':
      return 'bg-sky-100 text-sky-800 border-sky-300 font-bold'
    default:
      return 'bg-slate-100 text-slate-700 border-slate-300'
  }
}

function getStatusBadge(code: number | null) {
  if (!code) return { text: '200 OK', class: 'bg-emerald-50 text-emerald-700 border-emerald-200' }
  if (code >= 200 && code < 300) {
    return { text: `${code} OK`, class: 'bg-emerald-50 text-emerald-700 border-emerald-200' }
  } else if (code >= 400 && code < 500) {
    return { text: `${code} Client Error`, class: 'bg-amber-50 text-amber-800 border-amber-200' }
  } else if (code >= 500) {
    return { text: `${code} Server Error`, class: 'bg-rose-50 text-rose-800 border-rose-200' }
  }
  return { text: `${code}`, class: 'bg-slate-50 text-slate-700 border-slate-200' }
}

// WebSocket Event Listener
function handleWsMessage(event: CustomEvent) {
  if (event.detail?.type === 'AUDIT_LOGS_PURGED') {
    fetchAuditLogs()
  }
}

onMounted(() => {
  fetchAuditLogs()
  window.addEventListener('notane-ws-message', handleWsMessage as EventListener)
})

onUnmounted(() => {
  window.removeEventListener('notane-ws-message', handleWsMessage as EventListener)
})
</script>

<template>
  <AdminLayout>
    <div class="p-6 md:p-8 space-y-6">
      <!-- Header -->
      <div class="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h2 class="text-xl md:text-2xl font-bold text-slate-900 flex items-center gap-2.5">
            <Icon icon="streamline-ultimate-color:shield-check-1" class="w-7 h-7 drop-shadow-xs" />
            <span>Jejak Audit Aktivitas & Keamanan (Audit Trail 360°)</span>
          </h2>
          <p class="text-xs text-slate-600 mt-1">
            Intersepsi otomatis setiap request pengguna, pelacakan mutasi data, histori siapa & kapan akses fitur, serta retensi log otomatis.
          </p>
        </div>

        <div class="flex items-center gap-2.5">
          <!-- Tombol Pengaturan Retensi -->
          <button
            @click="openSettingsModal"
            class="flex items-center gap-1.5 bg-white hover:bg-slate-50 border border-slate-300 text-slate-700 text-xs font-semibold px-3.5 py-2 rounded-lg shadow-2xs transition active:scale-95"
            title="Konfigurasi Masa Retensi Log"
          >
            <Settings class="w-4 h-4 text-slate-500" />
            <span>Pengaturan Retensi</span>
            <span class="ml-1 px-1.5 py-0.2 bg-emerald-50 text-emerald-700 border border-emerald-200 rounded text-[10px] font-bold">
              {{ retentionDays > 0 ? `${retentionDays} Hari` : 'Permanen' }}
            </span>
          </button>

          <!-- Tombol Refresh -->
          <button
            @click="fetchAuditLogs"
            :disabled="isLoading"
            class="flex items-center gap-1.5 bg-slate-900 hover:bg-slate-800 text-white text-xs font-bold px-4 py-2 rounded-lg shadow-sm transition active:scale-95 disabled:opacity-50"
          >
            <RefreshCw class="w-3.5 h-3.5" :class="{ 'animate-spin': isLoading }" />
            <span>Muat Ulang</span>
          </button>
        </div>
      </div>

      <!-- Filter Controls Bar -->
      <div class="bg-white border border-slate-300 rounded-xl p-4 shadow-2xs space-y-3">
        <div class="grid grid-cols-1 md:grid-cols-12 gap-3 items-center">
          <!-- Search Box -->
          <div class="md:col-span-5 relative">
            <Search class="w-4 h-4 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2" />
            <input
              v-model="searchQuery"
              @input="onSearchInput"
              type="text"
              placeholder="Cari aksi, pengguna, endpoint URL, atau terminal..."
              class="w-full pl-9 pr-3.5 py-2 text-xs border border-slate-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-emerald-500/20 focus:border-emerald-600 font-sans"
            />
          </div>

          <!-- Method Filter -->
          <div class="md:col-span-2">
            <select
              v-model="selectedMethod"
              class="w-full py-2 px-3 text-xs border border-slate-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-emerald-500/20 focus:border-emerald-600 bg-white"
            >
              <option value="ALL">Semua Metode HTTP</option>
              <option value="GET">GET (Akses / Baca)</option>
              <option value="POST">POST (Buat / Transaksi)</option>
              <option value="PUT">PUT (Ubah / Update)</option>
              <option value="DELETE">DELETE (Hapus)</option>
            </select>
          </div>

          <!-- Entity Module Filter -->
          <div class="md:col-span-3">
            <select
              v-model="selectedEntity"
              class="w-full py-2 px-3 text-xs border border-slate-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-emerald-500/20 focus:border-emerald-600 bg-white"
            >
              <option v-for="ent in entityList" :key="ent.value" :value="ent.value">
                {{ ent.label }}
              </option>
            </select>
          </div>

          <!-- Status Outcome Filter -->
          <div class="md:col-span-2 flex items-center gap-2">
            <select
              v-model="selectedStatus"
              class="w-full py-2 px-3 text-xs border border-slate-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-emerald-500/20 focus:border-emerald-600 bg-white"
            >
              <option value="all">Semua Status</option>
              <option value="success">Berhasil (2xx)</option>
              <option value="error">Error (4xx / 5xx)</option>
            </select>

            <button
              v-if="searchQuery || selectedMethod !== 'ALL' || selectedEntity !== 'all' || selectedStatus !== 'all'"
              @click="resetFilters"
              class="text-xs text-slate-500 hover:text-slate-800 p-2 rounded hover:bg-slate-100"
              title="Reset Filter"
            >
              <X class="w-4 h-4" />
            </button>
          </div>
        </div>

        <!-- Retention Info Notice -->
        <div class="flex flex-wrap items-center justify-between text-[11px] text-slate-500 border-t border-slate-100 pt-2.5">
          <div class="flex items-center gap-2">
            <Clock class="w-3.5 h-3.5 text-emerald-600" />
            <span>
              Background scheduler membersihkan log kadaluwarsa otomatis setiap <strong>5 menit</strong> berdasarkan retensi
              <strong>{{ retentionDays > 0 ? `${retentionDays} hari` : 'permanen' }}</strong>.
            </span>
          </div>
          <span class="font-medium text-slate-600">Total Tercatat: {{ totalItems }} baris</span>
        </div>
      </div>

      <!-- Logs Table -->
      <div class="bg-white border border-slate-300 rounded-xl overflow-hidden shadow-2xs">
        <div class="overflow-x-auto">
          <table class="w-full text-left text-xs border-collapse min-w-[950px]">
            <thead class="bg-slate-100/80 border-b border-slate-300 text-slate-700 font-mono text-[11px] uppercase tracking-wider">
              <tr>
                <th class="py-3 px-4">Waktu & Durasi</th>
                <th class="py-3 px-3">Metode & Status</th>
                <th class="py-3 px-4">Aksi / Endpoint API</th>
                <th class="py-3 px-3">Modul</th>
                <th class="py-3 px-4">Pelaksana (Aktor)</th>
                <th class="py-3 px-3">Terminal & IP</th>
                <th class="py-3 px-3 text-center">Payload</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-200">
              <tr
                v-for="item in logs"
                :key="item.log.id"
                class="hover:bg-slate-50/80 transition cursor-pointer"
                @click="viewDetail(item)"
              >
                <!-- Waktu & Latensi -->
                <td class="py-3 px-4 whitespace-nowrap">
                  <div class="font-semibold text-slate-800 text-[11px]">
                    {{ formatDate(item.log.created_at) }}
                  </div>
                  <div class="text-[10px] text-slate-500 flex items-center gap-1 mt-0.5">
                    <Clock class="w-3 h-3 text-slate-400" />
                    <span>{{ item.log.execution_time_ms ?? 0 }} ms</span>
                  </div>
                </td>

                <!-- Metode & Status HTTP -->
                <td class="py-3 px-3 whitespace-nowrap">
                  <div class="flex items-center gap-1.5">
                    <span
                      class="px-2 py-0.5 rounded text-[10px] font-mono border"
                      :class="getMethodClass(item.log.http_method || 'POST')"
                    >
                      {{ item.log.http_method || 'ACTION' }}
                    </span>
                    <span
                      class="px-1.5 py-0.5 rounded text-[10px] font-mono border font-semibold"
                      :class="getStatusBadge(item.log.status_code).class"
                    >
                      {{ item.log.status_code ?? 200 }}
                    </span>
                  </div>
                </td>

                <!-- Aksi / Endpoint -->
                <td class="py-3 px-4 max-w-xs">
                  <div class="font-semibold text-slate-900 truncate" :title="item.log.action">
                    {{ item.log.action }}
                  </div>
                  <div class="text-[10px] text-slate-500 font-mono truncate" :title="item.log.endpoint || '-'">
                    {{ item.log.endpoint || '-' }}
                  </div>
                </td>

                <!-- Modul -->
                <td class="py-3 px-3 whitespace-nowrap">
                  <span class="px-2 py-0.5 rounded-full text-[10px] font-medium bg-slate-100 text-slate-700 border border-slate-300 capitalize">
                    {{ item.log.entity_name }}
                  </span>
                </td>

                <!-- Pelaksana / Aktor -->
                <td class="py-3 px-4 whitespace-nowrap">
                  <div class="font-semibold text-slate-900 flex items-center gap-1.5">
                    <User class="w-3.5 h-3.5 text-slate-500" />
                    <span>{{ item.user_full_name || item.log.user_name || 'System / Guest' }}</span>
                  </div>
                  <div v-if="item.log.role_name" class="text-[10px] text-emerald-700 font-mono font-semibold ml-5">
                    {{ item.log.role_name }}
                  </div>
                  <div v-if="item.supervisor_full_name" class="text-[10px] text-amber-700 font-medium ml-5 flex items-center gap-1">
                    <Shield class="w-2.5 h-2.5" /> Spv: {{ item.supervisor_full_name }}
                  </div>
                </td>

                <!-- Terminal & IP -->
                <td class="py-3 px-3 whitespace-nowrap">
                  <div class="text-[11px] font-mono text-slate-700">
                    {{ item.log.terminal_id || 'CLIENT' }}
                  </div>
                  <div class="text-[10px] text-slate-400 font-mono">
                    {{ item.log.ip_address || '127.0.0.1' }}
                  </div>
                </td>

                <!-- Action Button -->
                <td class="py-3 px-3 text-center whitespace-nowrap" @click.stop>
                  <button
                    @click="viewDetail(item)"
                    class="p-1.5 bg-slate-100 hover:bg-slate-200 border border-slate-300 text-slate-700 rounded-md transition shadow-2xs active:scale-95 text-[11px] inline-flex items-center gap-1"
                    title="Lihat Detail Payload & Parameter"
                  >
                    <Eye class="w-3.5 h-3.5" />
                    <span>Detail</span>
                  </button>
                </td>
              </tr>

              <tr v-if="logs.length === 0 && !isLoading">
                <td colspan="7" class="py-16 text-center text-slate-400">
                  <Icon icon="streamline-ultimate-color:bin-1" class="w-10 h-10 mx-auto mb-2 opacity-50" />
                  <p class="font-medium">Tidak ada rekaman log audit yang cocok dengan filter pencarian.</p>
                </td>
              </tr>

              <tr v-if="isLoading">
                <td colspan="7" class="py-16 text-center text-slate-500">
                  <RefreshCw class="w-6 h-6 animate-spin mx-auto mb-2 text-emerald-600" />
                  <p class="text-xs">Memuat data log aktivitas dari server...</p>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <!-- Pagination Bar -->
        <PaginationBar
          v-model:currentPage="page"
          v-model:pageSize="pageSize"
          :totalItems="totalItems"
        />
      </div>
    </div>

    <!-- MODAL DETAIL AUDIT LOG -->
    <div
      v-if="showDetailModal && selectedLog"
      class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/50 backdrop-blur-xs"
    >
      <div class="bg-white rounded-2xl max-w-2xl w-full border border-slate-300 shadow-2xl overflow-hidden flex flex-col max-h-[90vh]">
        <!-- Modal Header -->
        <div class="px-6 py-4 bg-slate-50 border-b border-slate-200 flex items-center justify-between">
          <div class="flex items-center gap-2.5">
            <div class="p-2 bg-emerald-100 rounded-lg text-emerald-800">
              <Icon icon="streamline-ultimate-color:shield-check-1" class="w-5 h-5" />
            </div>
            <div>
              <h3 class="text-sm font-bold text-slate-900">Rincian Lengkap Jejak Aktivitas #{{ selectedLog.log.id }}</h3>
              <p class="text-[11px] text-slate-500">{{ formatDate(selectedLog.log.created_at) }}</p>
            </div>
          </div>

          <button
            @click="showDetailModal = false"
            class="text-slate-400 hover:text-slate-700 p-1.5 rounded-lg hover:bg-slate-200/60 transition"
          >
            <X class="w-4 h-4" />
          </button>
        </div>

        <!-- Modal Body -->
        <div class="p-6 overflow-y-auto space-y-5 text-xs">
          <!-- Summary Cards -->
          <div class="grid grid-cols-2 md:grid-cols-4 gap-3 bg-slate-50 p-3.5 rounded-xl border border-slate-200 font-mono">
            <div>
              <div class="text-[10px] text-slate-500 uppercase">Metode HTTP</div>
              <div class="font-bold text-slate-800 mt-0.5">
                <span class="px-2 py-0.5 rounded text-[10px] border" :class="getMethodClass(selectedLog.log.http_method)">
                  {{ selectedLog.log.http_method || 'ACTION' }}
                </span>
              </div>
            </div>

            <div>
              <div class="text-[10px] text-slate-500 uppercase">Status Respon</div>
              <div class="font-bold mt-0.5">
                <span class="px-2 py-0.5 rounded text-[10px] border font-semibold" :class="getStatusBadge(selectedLog.log.status_code).class">
                  {{ selectedLog.log.status_code ?? 200 }}
                </span>
              </div>
            </div>

            <div>
              <div class="text-[10px] text-slate-500 uppercase">Durasi Eksekusi</div>
              <div class="font-bold text-slate-800 mt-0.5 flex items-center gap-1">
                <Clock class="w-3 h-3 text-slate-400" />
                <span>{{ selectedLog.log.execution_time_ms ?? 0 }} ms</span>
              </div>
            </div>

            <div>
              <div class="text-[10px] text-slate-500 uppercase">Modul Entitas</div>
              <div class="font-bold text-slate-800 mt-0.5 capitalize">
                {{ selectedLog.log.entity_name }}
              </div>
            </div>
          </div>

          <!-- Section: Siapa & Dimana -->
          <div class="space-y-2">
            <h4 class="font-bold text-slate-800 text-[11px] uppercase tracking-wider text-slate-500">
              Identitas Pengguna & Perangkat
            </h4>
            <div class="grid grid-cols-2 gap-2.5 bg-white border border-slate-200 rounded-xl p-3">
              <div>
                <span class="text-slate-500 text-[10px]">Nama Pengguna:</span>
                <p class="font-bold text-slate-900">{{ selectedLog.user_full_name || selectedLog.log.user_name || 'System / Guest' }}</p>
              </div>
              <div>
                <span class="text-slate-500 text-[10px]">Peran (Role):</span>
                <p class="font-bold text-emerald-800 font-mono">{{ selectedLog.log.role_name || '-' }}</p>
              </div>
              <div>
                <span class="text-slate-500 text-[10px]">Terminal / Device:</span>
                <p class="font-mono text-slate-700">{{ selectedLog.log.terminal_id || 'CLIENT' }}</p>
              </div>
              <div>
                <span class="text-slate-500 text-[10px]">Alamat IP:</span>
                <p class="font-mono text-slate-700">{{ selectedLog.log.ip_address || '127.0.0.1' }}</p>
              </div>
              <div v-if="selectedLog.log.user_agent" class="col-span-2">
                <span class="text-slate-500 text-[10px]">User-Agent:</span>
                <p class="font-mono text-[10px] text-slate-600 truncate" :title="selectedLog.log.user_agent">
                  {{ selectedLog.log.user_agent }}
                </p>
              </div>
            </div>
          </div>

          <!-- Section: Endpoint URL -->
          <div class="space-y-1.5">
            <h4 class="font-bold text-slate-800 text-[11px] uppercase tracking-wider text-slate-500">
              URL Endpoint & Aksi
            </h4>
            <div class="bg-slate-900 text-emerald-400 p-2.5 rounded-lg font-mono text-[11px] break-all select-all">
              {{ selectedLog.log.action }}
              <span v-if="selectedLog.log.endpoint && selectedLog.log.endpoint !== selectedLog.log.action" class="block text-slate-400 text-[10px] mt-1">
                URI: {{ selectedLog.log.endpoint }}
              </span>
            </div>
          </div>

          <!-- Section: Payload Data JSON -->
          <div class="space-y-1.5">
            <div class="flex items-center justify-between">
              <h4 class="font-bold text-slate-800 text-[11px] uppercase tracking-wider text-slate-500">
                Data yang Dikirim (Request Payload / Query)
              </h4>
              <button
                v-if="selectedLog.log.new_values"
                @click="copyJsonPayload"
                class="flex items-center gap-1 text-[11px] text-emerald-700 hover:text-emerald-800 font-medium py-0.5 px-2 rounded hover:bg-emerald-50 transition"
              >
                <Check v-if="copiedPayload" class="w-3 h-3 text-emerald-600" />
                <Copy v-else class="w-3 h-3" />
                <span>{{ copiedPayload ? 'Tersalin' : 'Salin JSON' }}</span>
              </button>
            </div>

            <div class="bg-slate-950 text-slate-200 rounded-xl p-3 font-mono text-[11px] overflow-x-auto max-h-56 border border-slate-800">
              <pre v-if="selectedLog.log.new_values">{{ JSON.stringify(selectedLog.log.new_values, null, 2) }}</pre>
              <div v-else class="text-slate-500 italic">Tidak ada payload body pada request ini (GET atau empty body).</div>
            </div>
          </div>
        </div>

        <!-- Modal Footer -->
        <div class="px-6 py-3.5 bg-slate-50 border-t border-slate-200 flex justify-end">
          <button
            @click="showDetailModal = false"
            class="px-4 py-2 bg-slate-200 hover:bg-slate-300 text-slate-700 font-bold rounded-lg text-xs transition"
          >
            Tutup
          </button>
        </div>
      </div>
    </div>

    <!-- MODAL PENGATURAN RETENSI AUDIT LOG -->
    <div
      v-if="showSettingsModal"
      class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/50 backdrop-blur-xs"
    >
      <div class="bg-white rounded-2xl max-w-md w-full border border-slate-300 shadow-2xl overflow-hidden flex flex-col">
        <!-- Modal Header -->
        <div class="px-6 py-4 bg-slate-50 border-b border-slate-200 flex items-center justify-between">
          <div class="flex items-center gap-2.5">
            <div class="p-2 bg-slate-200 text-slate-700 rounded-lg">
              <Settings class="w-5 h-5" />
            </div>
            <div>
              <h3 class="text-sm font-bold text-slate-900">Pengaturan Retensi Log Otomatis</h3>
              <p class="text-[11px] text-slate-500">Kelola pembersihan otomatis log aktivitas</p>
            </div>
          </div>

          <button
            @click="showSettingsModal = false"
            class="text-slate-400 hover:text-slate-700 p-1.5 rounded-lg hover:bg-slate-200/60 transition"
          >
            <X class="w-4 h-4" />
          </button>
        </div>

        <!-- Modal Body -->
        <div class="p-6 space-y-4 text-xs">
          <!-- Info Box -->
          <div class="bg-emerald-50 border border-emerald-200 rounded-xl p-3.5 text-emerald-900 space-y-1">
            <div class="font-bold flex items-center gap-1.5 text-emerald-800">
              <Clock class="w-4 h-4 text-emerald-600" />
              <span>Background Scheduler Aktif (5 Menit)</span>
            </div>
            <p class="text-[11px] text-emerald-700 leading-relaxed">
              Server memeriksa dan menghapus log aktivitas yang telah melampaui batas hari retensi secara otomatis setiap 5 menit di latar belakang.
            </p>
          </div>

          <!-- Statistik Log Saat Ini -->
          <div class="grid grid-cols-2 gap-3 bg-slate-50 p-3 rounded-xl border border-slate-200 font-mono">
            <div>
              <div class="text-[10px] text-slate-500">Total Baris Log:</div>
              <div class="font-bold text-slate-800 text-sm mt-0.5">{{ totalLogsCount }} baris</div>
            </div>
            <div>
              <div class="text-[10px] text-slate-500">Log Tertua:</div>
              <div class="font-bold text-slate-800 text-[11px] mt-0.5">
                {{ oldestLogDate ? formatDate(oldestLogDate) : '-' }}
              </div>
            </div>
          </div>

          <!-- Dropdown Pilihan Hari -->
          <div class="space-y-1.5">
            <label class="block font-bold text-slate-700">Masa Retensi Penyimpanan Log:</label>
            <select
              v-model="settingsRetentionDays"
              class="w-full py-2.5 px-3 text-xs border border-slate-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-emerald-500/20 focus:border-emerald-600 font-sans bg-white"
            >
              <option :value="7">7 Hari (1 Minggu)</option>
              <option :value="14">14 Hari (2 Minggu)</option>
              <option :value="30">30 Hari (1 Bulan - Direkomendasikan)</option>
              <option :value="60">60 Hari (2 Bulan)</option>
              <option :value="90">90 Hari (3 Bulan)</option>
              <option :value="180">180 Hari (6 Bulan)</option>
              <option :value="365">365 Hari (1 Tahun)</option>
              <option :value="0">0 (Simpan Selamanya / Jangan Dihapus Otomatis)</option>
            </select>
            <p class="text-[10px] text-slate-500">
              Log yang berumur lebih dari batas hari di atas akan otomatis terhapus dari database host.
            </p>
          </div>

          <!-- Manual Cleanup Section -->
          <div class="pt-3 border-t border-slate-200 space-y-3">
            <div class="font-bold text-slate-800 text-xs">Pembersihan Manual:</div>

            <!-- Pilihan Umur Log untuk Dibersihkan -->
            <div class="space-y-1.5">
              <label class="block text-[11px] text-slate-600">Hapus Log yang Lebih Tua dari:</label>
              <div class="flex items-center gap-2">
                <select
                  v-model="cleanupDays"
                  class="flex-1 py-1.5 px-2.5 text-xs border border-slate-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-emerald-500/20 bg-white"
                >
                  <option :value="1">Lebih dari 1 Hari Lalu</option>
                  <option :value="7">Lebih dari 7 Hari Lalu</option>
                  <option :value="14">Lebih dari 14 Hari Lalu</option>
                  <option :value="30">Lebih dari 30 Hari Lalu</option>
                  <option :value="60">Lebih dari 60 Hari Lalu</option>
                </select>
                <button
                  @click="handleManualCleanup(cleanupDays)"
                  :disabled="isCleaningUp || totalLogsCount === 0"
                  class="px-3 py-1.5 bg-slate-100 hover:bg-slate-200 border border-slate-300 text-slate-700 font-bold rounded-lg text-xs transition active:scale-95 disabled:opacity-50 whitespace-nowrap"
                >
                  Bersihkan
                </button>
              </div>
            </div>

            <!-- Tombol Kosongkan Total -->
            <div class="p-2.5 bg-rose-50 border border-rose-200 rounded-xl flex items-center justify-between">
              <div>
                <div class="font-bold text-rose-900 text-[11px]">Kosongkan Seluruh Riwayat:</div>
                <div class="text-[10px] text-rose-700">Hapus semua data log saat ini (Reset 0)</div>
              </div>
              <button
                @click="handleManualCleanup(0)"
                :disabled="isCleaningUp || totalLogsCount === 0"
                class="px-3 py-1.5 bg-rose-600 hover:bg-rose-700 text-white font-bold rounded-lg text-xs transition active:scale-95 disabled:opacity-50 flex items-center gap-1 shadow-2xs"
              >
                <Trash2 class="w-3.5 h-3.5" />
                <span>Kosongkan Semua</span>
              </button>
            </div>
          </div>
        </div>

        <!-- Modal Footer -->
        <div class="px-6 py-3.5 bg-slate-50 border-t border-slate-200 flex items-center justify-end gap-2.5">
          <button
            @click="showSettingsModal = false"
            class="px-4 py-2 bg-slate-200 hover:bg-slate-300 text-slate-700 font-bold rounded-lg text-xs transition"
          >
            Batal
          </button>
          <button
            @click="saveSettings"
            :disabled="isSavingSettings"
            class="px-4 py-2 bg-emerald-600 hover:bg-emerald-700 text-white font-bold rounded-lg text-xs transition shadow-sm active:scale-95 disabled:opacity-50"
          >
            {{ isSavingSettings ? 'Menyimpan...' : 'Simpan Pengaturan' }}
          </button>
        </div>
      </div>
    </div>
  </AdminLayout>
</template>
