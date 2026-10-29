<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { useProviderStore } from '../stores/providerStore'
import PaginationBar from '../components/common/PaginationBar.vue'
import { formatNumber } from '@/utils/formatters'
import {
  ShieldAlert,
  Search,
  CheckCircle2,
  XCircle,
  AlertTriangle,
  Clock,
  KeyRound,
  Cpu,
  RefreshCw,
  ArrowUpDown,
  ArrowUp,
  ArrowDown,
} from 'lucide-vue-next'

const providerStore = useProviderStore()

// Server-side state
const searchQuery = ref('')
const selectedStatus = ref<'ALL' | 'SUCCESS' | 'REJECTED' | 'BLOCKED'>('ALL')
const sortBy = ref<'created_at' | 'action' | 'status'>('created_at')
const sortOrder = ref<'asc' | 'desc'>('desc')

async function loadData(page?: number, pageSize?: number) {
  await providerStore.fetchAuditLogs({
    page: page ?? providerStore.auditLogsPagination.page,
    page_size: pageSize ?? providerStore.auditLogsPagination.page_size,
    search: searchQuery.value.trim() || undefined,
    status: selectedStatus.value === 'ALL' ? undefined : selectedStatus.value,
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

watch(selectedStatus, () => {
  loadData(1)
})

onMounted(async () => {
  await loadData(1)
})

function handleSort(column: 'created_at' | 'action' | 'status') {
  if (sortBy.value === column) {
    sortOrder.value = sortOrder.value === 'asc' ? 'desc' : 'asc'
  } else {
    sortBy.value = column
    sortOrder.value = 'asc'
  }
  loadData(1)
}
</script>

<template>
  <div class="space-y-5">
    <!-- Top Controls -->
    <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
      <div class="flex flex-wrap items-center gap-2 w-full sm:w-auto">
        <div class="relative w-full sm:w-72">
          <Search class="w-4 h-4 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2" />
          <input
            v-model="searchQuery"
            type="text"
            placeholder="Cari aksi, lisensi, HWID, atau IP..."
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

        <!-- Status Filter -->
        <div class="flex items-center bg-slate-900 border border-slate-800 rounded-xl p-1 gap-1">
          <button
            v-for="st in (['ALL', 'SUCCESS', 'REJECTED', 'BLOCKED'] as const)"
            :key="st"
            @click="selectedStatus = st"
            class="px-2.5 py-1 rounded-lg text-xs font-medium transition-colors cursor-pointer"
            :class="selectedStatus === st ? 'bg-brand-500 text-white font-semibold' : 'text-slate-400 hover:text-slate-200'"
          >
            {{ st }}
          </button>
        </div>
      </div>

      <div class="text-xs text-slate-400 font-mono">
        Total Riwayat: <span class="text-brand-400 font-bold">{{ formatNumber(providerStore.auditLogsPagination.total_items) }}</span>
      </div>
    </div>

    <!-- Empty State -->
    <div
      v-if="!providerStore.isLoading && providerStore.auditLogsPagination.total_items === 0"
      class="py-16 text-center rounded-2xl glass-panel border border-slate-800"
    >
      <ShieldAlert class="w-12 h-12 text-slate-600 mx-auto mb-3" />
      <div class="text-sm font-semibold text-slate-300">Belum Ada Catatan Audit</div>
      <div class="text-xs text-slate-500 mt-1">
        {{ searchQuery || selectedStatus !== 'ALL' ? 'Tidak ada data audit yang cocok dengan filter pencarian.' : 'Seluruh percobaan aktivasi, verifikasi, atau tampering akan dicatat di sini secara otomatis.' }}
      </div>
      <button
        v-if="searchQuery || selectedStatus !== 'ALL'"
        @click="searchQuery = ''; selectedStatus = 'ALL'; loadData(1)"
        class="mt-4 px-4 py-2 bg-slate-800 hover:bg-slate-700 text-white rounded-xl text-xs font-semibold cursor-pointer"
      >
        Reset Filter
      </button>
    </div>

    <!-- Table Logs -->
    <div v-else class="space-y-3">
      <div class="glass-panel border border-slate-800 rounded-2xl overflow-hidden shadow-xl">
        <div class="overflow-x-auto">
          <table class="w-full text-left border-collapse text-xs">
            <thead>
              <tr class="bg-slate-900/90 text-slate-400 border-b border-slate-800 select-none text-[11px]">
                <th
                  @click="handleSort('created_at')"
                  class="py-3 px-4 font-semibold cursor-pointer hover:text-white transition"
                >
                  <div class="flex items-center gap-1.5">
                    <span>Waktu</span>
                    <ArrowUp v-if="sortBy === 'created_at' && sortOrder === 'asc'" class="w-3.5 h-3.5 text-brand-400" />
                    <ArrowDown v-else-if="sortBy === 'created_at' && sortOrder === 'desc'" class="w-3.5 h-3.5 text-brand-400" />
                    <ArrowUpDown v-else class="w-3 h-3 text-slate-600" />
                  </div>
                </th>

                <th
                  @click="handleSort('action')"
                  class="py-3 px-4 font-semibold cursor-pointer hover:text-white transition"
                >
                  <div class="flex items-center gap-1.5">
                    <span>Aksi</span>
                    <ArrowUp v-if="sortBy === 'action' && sortOrder === 'asc'" class="w-3.5 h-3.5 text-brand-400" />
                    <ArrowDown v-else-if="sortBy === 'action' && sortOrder === 'desc'" class="w-3.5 h-3.5 text-brand-400" />
                    <ArrowUpDown v-else class="w-3 h-3 text-slate-600" />
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

                <th class="py-3 px-4 font-semibold text-slate-400">Lisensi & HWID</th>
                <th class="py-3 px-4 font-semibold text-slate-400">IP Pengirim</th>
                <th class="py-3 px-4 font-semibold text-slate-400">Keterangan</th>
              </tr>
            </thead>

            <tbody class="divide-y divide-slate-800/60 font-mono">
              <tr
                v-for="log in providerStore.auditLogsPagination.items"
                :key="log.id"
                class="hover:bg-slate-800/40 transition-colors"
              >
                <!-- Timestamp -->
                <td class="py-3 px-4 text-slate-400 text-[11px] whitespace-nowrap">
                  {{ new Date(log.created_at).toLocaleString('id-ID') }}
                </td>

                <!-- Action -->
                <td class="py-3 px-4">
                  <span
                    class="px-2 py-0.5 rounded text-[10px] font-bold tracking-wider inline-block"
                    :class="[
                      log.action === 'ACTIVATE' ? 'bg-blue-500/20 text-blue-400 border border-blue-500/30' :
                      log.action === 'VERIFY' ? 'bg-emerald-500/20 text-emerald-400 border border-emerald-500/30' :
                      'bg-purple-500/20 text-purple-400 border border-purple-500/30'
                    ]"
                  >
                    {{ log.action }}
                  </span>
                </td>

                <!-- Status -->
                <td class="py-3 px-4">
                  <span
                    class="inline-flex items-center gap-1 px-2 py-0.5 rounded text-[10px] font-bold"
                    :class="[
                      log.status === 'SUCCESS' ? 'text-emerald-400 bg-emerald-500/10' :
                      log.status === 'REJECTED' ? 'text-rose-400 bg-rose-500/10' :
                      'text-amber-400 bg-amber-500/10'
                    ]"
                  >
                    <component :is="log.status === 'SUCCESS' ? CheckCircle2 : XCircle" class="w-3 h-3" />
                    <span>{{ log.status }}</span>
                  </span>
                </td>

                <!-- License & HWID -->
                <td class="py-3 px-4 text-[11px]">
                  <div class="space-y-0.5">
                    <div v-if="log.license_key" class="text-white font-bold flex items-center gap-1">
                      <KeyRound class="w-3 h-3 text-brand-400 shrink-0" />
                      <span>{{ log.license_key }}</span>
                    </div>
                    <div v-if="log.hardware_id" class="text-slate-400 text-[10px] truncate max-w-[180px]" :title="log.hardware_id">
                      HWID: {{ log.hardware_id }}
                    </div>
                  </div>
                </td>

                <!-- IP Address -->
                <td class="py-3 px-4 text-slate-400 text-[11px] whitespace-nowrap">
                  {{ log.ip_address || '127.0.0.1' }}
                </td>

                <!-- Details -->
                <td class="py-3 px-4 font-sans text-xs text-slate-300 max-w-[220px]">
                  <span class="truncate block" :title="log.details || '-'">{{ log.details || '-' }}</span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <!-- Pagination Component -->
      <PaginationBar
        :page="providerStore.auditLogsPagination.page"
        :page-size="providerStore.auditLogsPagination.page_size"
        :total-items="providerStore.auditLogsPagination.total_items"
        :total-pages="providerStore.auditLogsPagination.total_pages"
        :is-loading="providerStore.isLoading"
        @update:page="(p) => loadData(p, providerStore.auditLogsPagination.page_size)"
        @update:page-size="(s) => loadData(1, s)"
      />
    </div>
  </div>
</template>
