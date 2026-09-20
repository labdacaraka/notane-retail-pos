<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import AdminLayout from '@/layouts/AdminLayout.vue'
import PaginationBar from '@/components/common/PaginationBar.vue'
import { UserCheck } from 'lucide-vue-next'
import { api } from '@/services/api'

const logs = ref<any[]>([])
const isLoading = ref<boolean>(false)

// Pagination states
const page = ref<number>(1)
const pageSize = ref<number>(15)
const paginatedLogs = computed(() => {
  const start = (page.value - 1) * pageSize.value
  return logs.value.slice(start, start + pageSize.value)
})

async function fetchAuditLogs() {
  isLoading.value = true
  try {
    const res = await api.get('/auth/audit-logs', { params: { limit: 200 } })
    if (res.data.success) {
      logs.value = res.data.data
    }
  } catch (err) {
    console.error(err)
  } finally {
    isLoading.value = false
  }
}

onMounted(() => {
  fetchAuditLogs()
})
</script>

<template>
  <AdminLayout>
    <div class="p-8 space-y-6">
      <!-- Header -->
      <div class="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h2 class="text-xl font-bold text-slate-900 flex items-center gap-2.5">
            <Icon icon="streamline-ultimate-color:shield-check-1" class="w-6 h-6 drop-shadow-xs" />
            <span>Jejak Audit Keamanan (Audit Trail 360°)</span>
          </h2>
          <p class="text-xs text-slate-600">Rekam jejak setiap aksi sensitif kasir dan otorisasi PIN supervisor (Void, Diskon, Buka Laci).</p>
        </div>

        <button
          @click="fetchAuditLogs"
          class="bg-slate-100 hover:bg-slate-200 border border-slate-300 text-slate-700 text-xs font-bold px-4 py-2 rounded-lg shadow-sm transition active:scale-95"
        >
          Muat Ulang
        </button>
      </div>

      <!-- Logs Table -->
      <div class="bg-white border border-slate-300 rounded-xl overflow-hidden shadow-sm">
        <table class="w-full text-left text-xs border-collapse">
          <thead class="bg-slate-100 border-b border-slate-300 text-slate-700 font-mono text-[11px] uppercase tracking-wider">
            <tr>
              <th class="py-3.5 px-4">Waktu</th>
              <th class="py-3.5 px-4">Aksi Sensitif</th>
              <th class="py-3.5 px-4">Entitas</th>
              <th class="py-3.5 px-4">Terminal</th>
              <th class="py-3.5 px-4">Pengguna Pelaksana</th>
              <th class="py-3.5 px-4">Otorisasi Supervisor</th>
              <th class="py-3.5 px-4">Perubahan Data (Payload)</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-slate-200 font-mono">
            <tr v-for="item in paginatedLogs" :key="item.log.id" class="hover:bg-emerald-50/40 transition">
              <td class="py-3.5 px-4 text-slate-500 font-sans">
                {{ new Date(item.log.created_at).toLocaleString('id-ID') }}
              </td>
              <td class="py-3.5 px-4">
                <span
                  class="px-2 py-0.5 rounded-full text-[10px] font-bold border"
                  :class="item.log.action.includes('VOID') ? 'bg-rose-50 text-rose-700 border-rose-300' : 'bg-emerald-50 text-emerald-800 border border-emerald-300'"
                >
                  {{ item.log.action }}
                </span>
              </td>
              <td class="py-3.5 px-4 text-slate-700 font-sans">{{ item.log.entity_name }}</td>
              <td class="py-3.5 px-4 text-slate-500">{{ item.log.terminal_id }}</td>
              <td class="py-3.5 px-4 font-sans font-semibold text-slate-900">
                {{ item.user_full_name || 'System' }}
              </td>
              <td class="py-3.5 px-4 font-sans">
                <span v-if="item.supervisor_full_name" class="text-amber-700 font-bold flex items-center gap-1">
                  <UserCheck class="w-3.5 h-3.5" /> {{ item.supervisor_full_name }}
                </span>
                <span v-else class="text-slate-400">-</span>
              </td>
              <td class="py-3.5 px-4 text-[11px] text-slate-500 max-w-xs truncate font-mono">
                {{ item.log.new_values ? JSON.stringify(item.log.new_values) : '-' }}
              </td>
            </tr>

            <tr v-if="logs.length === 0">
              <td colspan="7" class="py-16 text-center text-slate-400 font-sans">Belum ada rekaman jejak audit.</td>
            </tr>
          </tbody>
        </table>
        <PaginationBar
          v-model:currentPage="page"
          v-model:pageSize="pageSize"
          :totalItems="logs.length"
        />
      </div>
    </div>
  </AdminLayout>
</template>
