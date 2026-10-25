<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { useProviderStore, type DeviceActivation } from '../stores/providerStore'
import { usePermission } from '../composables/usePermission'
import PaginationBar from '../components/common/PaginationBar.vue'
import {
  Cpu,
  Unlink,
  Search,
  KeyRound,
  Building,
  Monitor,
  Wifi,
  Clock,
  CheckCircle2,
  XCircle,
  ArrowUpDown,
  ArrowUp,
  ArrowDown,
  RefreshCw,
} from 'lucide-vue-next'

const providerStore = useProviderStore()
const { hasPermission } = usePermission()

// Server-side filter & sort
const searchQuery = ref('')
const selectedStatus = ref<'ALL' | 'ACTIVE' | 'INACTIVE'>('ALL')
const sortBy = ref<'last_ping_at' | 'hardware_id' | 'device_name' | 'license_key' | 'client_name' | 'is_active'>('last_ping_at')
const sortOrder = ref<'asc' | 'desc'>('desc')

async function loadData(page?: number, pageSize?: number) {
  let statusParam: string | undefined = undefined
  if (selectedStatus.value === 'ACTIVE') statusParam = 'active'
  else if (selectedStatus.value === 'INACTIVE') statusParam = 'inactive'

  await providerStore.fetchDevices({
    page: page ?? providerStore.devicesPagination.page,
    page_size: pageSize ?? providerStore.devicesPagination.page_size,
    search: searchQuery.value.trim() || undefined,
    status: statusParam,
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

function handleSort(column: 'last_ping_at' | 'hardware_id' | 'device_name' | 'license_key' | 'client_name' | 'is_active') {
  if (sortBy.value === column) {
    sortOrder.value = sortOrder.value === 'asc' ? 'desc' : 'asc'
  } else {
    sortBy.value = column
    sortOrder.value = 'asc'
  }
  loadData(1)
}

async function confirmUnbind(dev: DeviceActivation) {
  if (
    confirm(
      `Yakin ingin melepaskan perangkat '${dev.device_name || dev.hardware_id}'? Kuota kasir untuk lisensi ini akan bertambah 1 slot.`
    )
  ) {
    await providerStore.unbindDevice(dev.id)
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
            placeholder="Cari nomor lisensi, perangkat, atau nama kasir..."
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

        <!-- Status Filter -->
        <div class="flex items-center bg-slate-900 border border-slate-800 rounded-xl p-1 gap-1">
          <button
            v-for="flt in (['ALL', 'ACTIVE', 'INACTIVE'] as const)"
            :key="flt"
            @click="selectedStatus = flt"
            class="px-2.5 py-1 rounded-lg text-xs font-medium transition-colors cursor-pointer"
            :class="selectedStatus === flt ? 'bg-brand-500 text-white font-semibold' : 'text-slate-400 hover:text-slate-200'"
          >
            {{ flt }}
          </button>
        </div>
      </div>

      <div class="text-xs text-slate-400 font-mono">
        Total Perangkat: <span class="text-brand-400 font-bold">{{ providerStore.devicesPagination.total_items }}</span>
      </div>
    </div>

    <!-- Empty State -->
    <div
      v-if="!providerStore.isLoading && providerStore.devicesPagination.total_items === 0"
      class="py-16 text-center rounded-2xl glass-panel border border-slate-800"
    >
      <Cpu class="w-12 h-12 text-slate-600 mx-auto mb-3" />
      <div class="text-sm font-semibold text-slate-300">Belum Ada Terminal Kasir Terikat</div>
      <div class="text-xs text-slate-500 mt-1">
        {{ searchQuery || selectedStatus !== 'ALL' ? 'Tidak ada data perangkat yang cocok dengan pencarian.' : 'Saat mesin kasir pertama kali mengaktivasi lisensi, HWID akan otomatis terdaftar di sini.' }}
      </div>
      <button
        v-if="searchQuery || selectedStatus !== 'ALL'"
        @click="searchQuery = ''; selectedStatus = 'ALL'; loadData(1)"
        class="mt-4 px-4 py-2 bg-slate-800 hover:bg-slate-700 text-white rounded-xl text-xs font-semibold cursor-pointer"
      >
        Reset Filter
      </button>
    </div>

    <!-- Table of Bound Devices -->
    <div v-else class="space-y-3">
      <div class="glass-panel border border-slate-800 rounded-2xl overflow-hidden shadow-xl">
        <div class="overflow-x-auto">
          <table class="w-full text-left border-collapse text-xs">
            <thead>
              <tr class="bg-slate-900/90 text-slate-400 border-b border-slate-800 select-none text-[11px]">
                <th
                  @click="handleSort('device_name')"
                  class="py-3 px-4 font-semibold cursor-pointer hover:text-white transition"
                >
                  <div class="flex items-center gap-1.5">
                    <span>Perangkat & Pelanggan</span>
                    <ArrowUp v-if="sortBy === 'device_name' && sortOrder === 'asc'" class="w-3.5 h-3.5 text-brand-400" />
                    <ArrowDown v-else-if="sortBy === 'device_name' && sortOrder === 'desc'" class="w-3.5 h-3.5 text-brand-400" />
                    <ArrowUpDown v-else class="w-3 h-3 text-slate-600" />
                  </div>
                </th>

                <th
                  @click="handleSort('hardware_id')"
                  class="py-3 px-4 font-semibold cursor-pointer hover:text-white transition"
                >
                  <div class="flex items-center gap-1.5">
                    <span>Kode Perangkat (ID Mesin)</span>
                    <ArrowUp v-if="sortBy === 'hardware_id' && sortOrder === 'asc'" class="w-3.5 h-3.5 text-brand-400" />
                    <ArrowDown v-else-if="sortBy === 'hardware_id' && sortOrder === 'desc'" class="w-3.5 h-3.5 text-brand-400" />
                    <ArrowUpDown v-else class="w-3 h-3 text-slate-600" />
                  </div>
                </th>

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

                <th class="py-3 px-4 font-semibold text-slate-400">Alamat IP & Sistem</th>

                <th
                  @click="handleSort('last_ping_at')"
                  class="py-3 px-4 font-semibold cursor-pointer hover:text-white transition"
                >
                  <div class="flex items-center gap-1.5">
                    <span>Aktivitas Terakhir</span>
                    <ArrowUp v-if="sortBy === 'last_ping_at' && sortOrder === 'asc'" class="w-3.5 h-3.5 text-brand-400" />
                    <ArrowDown v-else-if="sortBy === 'last_ping_at' && sortOrder === 'desc'" class="w-3.5 h-3.5 text-brand-400" />
                    <ArrowUpDown v-else class="w-3 h-3 text-slate-600" />
                  </div>
                </th>

                <th
                  @click="handleSort('is_active')"
                  class="py-3 px-4 font-semibold cursor-pointer hover:text-white transition text-center"
                >
                  <div class="flex items-center justify-center gap-1.5">
                    <span>Status</span>
                    <ArrowUp v-if="sortBy === 'is_active' && sortOrder === 'asc'" class="w-3.5 h-3.5 text-brand-400" />
                    <ArrowDown v-else-if="sortBy === 'is_active' && sortOrder === 'desc'" class="w-3.5 h-3.5 text-brand-400" />
                    <ArrowUpDown v-else class="w-3 h-3 text-slate-600" />
                  </div>
                </th>

                <th class="py-3 px-4 text-right font-semibold text-slate-400">Aksi</th>
              </tr>
            </thead>

            <tbody class="divide-y divide-slate-800/60">
              <tr
                v-for="d in providerStore.devicesPagination.items"
                :key="d.id"
                class="hover:bg-slate-800/40 transition-colors"
              >
                <!-- Terminal & Client -->
                <td class="py-3.5 px-4 font-medium text-slate-200">
                  <div class="flex items-center gap-2.5">
                    <div class="p-2 rounded-lg bg-blue-500/10 text-blue-400 shrink-0">
                      <Monitor class="w-4 h-4" />
                    </div>
                    <div>
                      <div class="font-bold text-white">{{ d.device_name || 'Terminal Kasir' }}</div>
                      <div class="text-[11px] text-slate-400 flex items-center gap-1 mt-0.5">
                        <Building class="w-3 h-3 text-slate-500" />
                        <span>{{ d.client_name || 'Klien Umum' }}</span>
                      </div>
                    </div>
                  </div>
                </td>

                <!-- HWID -->
                <td class="py-3.5 px-4 font-mono text-xs">
                  <span class="font-semibold text-slate-200 bg-slate-950 px-2 py-1 rounded border border-slate-800 select-all block truncate max-w-[200px]" :title="d.hardware_id">
                    {{ d.hardware_id }}
                  </span>
                </td>

                <!-- License Key -->
                <td class="py-3.5 px-4 font-mono text-xs">
                  <span class="text-brand-400 font-bold flex items-center gap-1.5">
                    <KeyRound class="w-3.5 h-3.5 text-brand-500" />
                    <span>{{ d.license_key || '-' }}</span>
                  </span>
                </td>

                <!-- IP & OS -->
                <td class="py-3.5 px-4 text-[11px] text-slate-400 space-y-0.5">
                  <div class="flex items-center gap-1 text-slate-300 font-mono">
                    <Wifi class="w-3 h-3 text-slate-500" />
                    <span>{{ d.ip_address || '127.0.0.1' }}</span>
                  </div>
                  <div class="truncate max-w-[160px] text-slate-500" :title="d.os_info || 'Unknown OS'">
                    {{ d.os_info || 'Desktop POS' }} • v{{ d.app_version || '1.0.0' }}
                  </div>
                </td>

                <!-- Last Ping -->
                <td class="py-3.5 px-4 text-[11px] font-mono text-slate-400 whitespace-nowrap">
                  <div class="flex items-center gap-1.5 text-slate-300">
                    <Clock class="w-3 h-3 text-slate-500" />
                    <span>{{ new Date(d.last_ping_at).toLocaleTimeString() }}</span>
                  </div>
                  <div class="text-[10px] text-slate-500">
                    {{ new Date(d.last_ping_at).toLocaleDateString() }}
                  </div>
                </td>

                <!-- Status -->
                <td class="py-3.5 px-4 text-center">
                  <span
                    class="px-2 py-0.5 rounded-full text-[10px] font-bold border inline-flex items-center gap-1"
                    :class="d.is_active === 1
                      ? 'bg-emerald-500/10 text-emerald-400 border-emerald-500/30'
                      : 'bg-slate-800 text-slate-400 border-slate-700'"
                  >
                    <CheckCircle2 v-if="d.is_active === 1" class="w-3 h-3" />
                    <XCircle v-else class="w-3 h-3" />
                    <span>{{ d.is_active === 1 ? 'Aktif' : 'Nonaktif' }}</span>
                  </span>
                </td>

                <!-- Action (Unbind) -->
                <td class="py-3.5 px-4 text-right">
                  <button
                    @click="confirmUnbind(d)"
                    :disabled="!hasPermission('device:unbind')"
                    class="px-2.5 py-1 bg-rose-500/10 hover:bg-rose-500/20 text-rose-400 border border-rose-500/30 rounded-lg text-xs font-semibold flex items-center gap-1 transition-colors ml-auto cursor-pointer"
                    title="Reset HWID / Lepas Tautan"
                  >
                    <Unlink class="w-3 h-3" />
                    <span>Reset Slot</span>
                  </button>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <!-- Pagination Component -->
      <PaginationBar
        :page="providerStore.devicesPagination.page"
        :page-size="providerStore.devicesPagination.page_size"
        :total-items="providerStore.devicesPagination.total_items"
        :total-pages="providerStore.devicesPagination.total_pages"
        :is-loading="providerStore.isLoading"
        @update:page="(p) => loadData(p, providerStore.devicesPagination.page_size)"
        @update:page-size="(s) => loadData(1, s)"
      />
    </div>
  </div>
</template>
