<script setup lang="ts">
import { onMounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useProviderStore } from '../stores/providerStore'
import {
  KeyRound,
  Users,
  Cpu,
  Layers,
  Radio,
  ArrowRight,
  ShieldCheck,
  CheckCircle2,
  Clock,
  Sparkles,
  FileSignature,
} from 'lucide-vue-next'

const router = useRouter()
const providerStore = useProviderStore()

const displayExternalUrl = computed(() => {
  const backendUrl = providerStore.status.external_url
  if (backendUrl && !backendUrl.includes('127.0.0.1') && !backendUrl.includes('localhost')) {
    return backendUrl
  }
  if (typeof window !== 'undefined' && window.location?.origin) {
    const origin = window.location.origin
    if (!origin.startsWith('tauri://') && !origin.includes('tauri.localhost') && !origin.includes(':517')) {
      return origin
    }
  }
  return backendUrl || `http://127.0.0.1:${providerStore.status.port || 19090}`
})

onMounted(async () => {
  await providerStore.fetchStatus()
  await providerStore.fetchLicenses()
  await providerStore.fetchAuditLogs()
})
</script>

<template>
  <div class="space-y-6">
    <!-- Server Status Banner -->
    <div class="relative overflow-hidden rounded-2xl glass-panel p-6 border border-brand-500/20 bg-gradient-to-r from-slate-900 via-slate-900/90 to-brand-950/30">
      <div class="absolute -right-8 -top-8 w-48 h-48 bg-brand-500/10 rounded-full blur-3xl pointer-events-none"></div>

      <div class="flex flex-col md:flex-row md:items-center justify-between gap-4 relative z-10">
        <div class="space-y-2">
          <div class="flex items-center gap-2.5">
            <span class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-semibold"
                  :class="providerStore.status.is_server_running ? 'bg-emerald-500/20 text-emerald-400 border border-emerald-500/30' : 'bg-rose-500/20 text-rose-400 border border-rose-500/30'">
              <span class="w-2 h-2 rounded-full" :class="providerStore.status.is_server_running ? 'bg-emerald-400 animate-pulse' : 'bg-rose-400'"></span>
              {{ providerStore.status.is_server_running ? 'LAYANAN AKTIF' : 'LAYANAN BERHENTI' }}
            </span>
            <span class="text-xs text-slate-400">Sistem Lisensi Toko</span>
          </div>

          <h2 class="text-xl font-bold text-white tracking-tight flex items-center gap-2">
            Notane Retail Manager
            <Sparkles class="w-4 h-4 text-brand-400" />
          </h2>
          <p class="text-xs text-slate-300 max-w-xl leading-relaxed">
            Pusat pengelolaan lisensi resmi toko dan penerbitan pembaruan aplikasi Notane POS.
          </p>
        </div>

        <div class="flex flex-wrap items-center gap-2.5">
          <button
            @click="router.push('/signer')"
            class="px-4 py-2 rounded-xl bg-brand-600 hover:bg-brand-500 text-white font-semibold text-xs transition-all shadow-lg shadow-brand-600/30 flex items-center gap-2"
          >
            <FileSignature class="w-4 h-4" />
            <span>Generate Token Offline</span>
          </button>
          <button
            @click="router.push('/licenses')"
            class="px-4 py-2 rounded-xl bg-slate-800 hover:bg-slate-700 text-slate-200 font-semibold text-xs transition-all border border-slate-700 flex items-center gap-1.5"
          >
            <span>Terbitkan Lisensi</span>
            <ArrowRight class="w-3.5 h-3.5" />
          </button>
        </div>
      </div>

      <!-- Quick Endpoint Badges -->
      <div class="mt-5 pt-4 border-t border-slate-800/80 flex flex-wrap gap-4 text-xs font-mono text-slate-400">
        <div class="flex items-center gap-1.5">
          <span class="text-brand-400 font-bold">API HOST:</span>
          <span class="text-slate-200 bg-slate-800/80 px-2 py-0.5 rounded">{{ displayExternalUrl }}</span>
        </div>
        <div class="flex items-center gap-1.5">
          <span class="text-slate-500">Aktivasi:</span>
          <span>POST /api/v1/license/activate</span>
        </div>
        <div class="flex items-center gap-1.5">
          <span class="text-slate-500">Verifikasi:</span>
          <span>POST /api/v1/license/verify</span>
        </div>
        <div class="flex items-center gap-1.5">
          <span class="text-slate-500">Update OTA:</span>
          <span>POST /api/v1/updates/check</span>
        </div>
      </div>
    </div>

    <!-- Metric KPI Cards -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
      <!-- Card 1: Lisensi Aktif -->
      <div class="p-5 rounded-2xl glass-panel border border-slate-800/80 flex flex-col justify-between">
        <div class="flex items-center justify-between">
          <span class="text-xs font-semibold text-slate-400 uppercase tracking-wider">Lisensi Terbit</span>
          <div class="p-2.5 rounded-xl bg-brand-500/10 text-brand-400 border border-brand-500/20">
            <KeyRound class="w-5 h-5" />
          </div>
        </div>
        <div class="mt-4">
          <div class="text-3xl font-extrabold text-white font-mono">{{ providerStore.status.total_licenses }}</div>
          <div class="mt-1 flex items-center gap-1.5 text-xs text-emerald-400">
            <CheckCircle2 class="w-3.5 h-3.5" />
            <span>{{ providerStore.status.active_licenses }} Berstatus Aktif</span>
          </div>
        </div>
      </div>

      <!-- Card 2: Terminal HWID Terkunci -->
      <div class="p-5 rounded-2xl glass-panel border border-slate-800/80 flex flex-col justify-between">
        <div class="flex items-center justify-between">
          <span class="text-xs font-semibold text-slate-400 uppercase tracking-wider">Mesin Kasir (HWID)</span>
          <div class="p-2.5 rounded-xl bg-blue-500/10 text-blue-400 border border-blue-500/20">
            <Cpu class="w-5 h-5" />
          </div>
        </div>
        <div class="mt-4">
          <div class="text-3xl font-extrabold text-white font-mono">{{ providerStore.status.total_devices }}</div>
          <div class="mt-1 text-xs text-slate-400">
            Node-Locking Hardware Terikat
          </div>
        </div>
      </div>

      <!-- Card 3: Klien Tenant -->
      <div class="p-5 rounded-2xl glass-panel border border-slate-800/80 flex flex-col justify-between">
        <div class="flex items-center justify-between">
          <span class="text-xs font-semibold text-slate-400 uppercase tracking-wider">Klien Tenant</span>
          <div class="p-2.5 rounded-xl bg-purple-500/10 text-purple-400 border border-purple-500/20">
            <Users class="w-5 h-5" />
          </div>
        </div>
        <div class="mt-4">
          <div class="text-3xl font-extrabold text-white font-mono">{{ providerStore.status.total_clients }}</div>
          <div class="mt-1 text-xs text-slate-400">
            Entitas Perusahaan Penyewa
          </div>
        </div>
      </div>

      <!-- Card 4: Rilis Update OTA -->
      <div class="p-5 rounded-2xl glass-panel border border-slate-800/80 flex flex-col justify-between">
        <div class="flex items-center justify-between">
          <span class="text-xs font-semibold text-slate-400 uppercase tracking-wider">Rilis Versi OTA</span>
          <div class="p-2.5 rounded-xl bg-amber-500/10 text-amber-400 border border-amber-500/20">
            <Layers class="w-5 h-5" />
          </div>
        </div>
        <div class="mt-4">
          <div class="text-3xl font-extrabold text-white font-mono">{{ providerStore.status.total_releases }}</div>
          <div class="mt-1 text-xs text-slate-400">
            Paket Update Tertanda Tangan
          </div>
        </div>
      </div>
    </div>

    <!-- Two Columns: Lisensi Terbaru & Aktivitas Audit -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <!-- Lisensi Aktif Terkini -->
      <div class="p-5 rounded-2xl glass-panel border border-slate-800/80 flex flex-col">
        <div class="flex items-center justify-between mb-4">
          <h3 class="font-bold text-sm text-white flex items-center gap-2">
            <KeyRound class="w-4 h-4 text-brand-400" />
            <span>Lisensi Terdaftar Terbaru</span>
          </h3>
          <router-link to="/licenses" class="text-xs text-brand-400 hover:text-brand-300 font-semibold flex items-center gap-1">
            Lihat Semua <ArrowRight class="w-3 h-3" />
          </router-link>
        </div>

        <div v-if="providerStore.licenses.length === 0" class="py-8 text-center text-xs text-slate-500">
          Belum ada lisensi terbit. Klik 'Terbitkan Lisensi' untuk membuat baru.
        </div>

        <div v-else class="space-y-2.5 overflow-y-auto max-h-[300px]">
          <div
            v-for="lic in providerStore.licenses.slice(0, 5)"
            :key="lic.id"
            class="p-3 rounded-xl bg-slate-900/70 border border-slate-800 flex items-center justify-between hover:border-slate-700 transition-colors"
          >
            <div>
              <div class="font-mono text-xs font-bold text-slate-200">{{ lic.license_key }}</div>
              <div class="text-[11px] text-slate-400 mt-0.5">
                {{ lic.client_name || 'Klien Umum' }} • <span class="text-brand-400 font-medium">{{ lic.tier }}</span>
              </div>
            </div>
            <div class="text-right">
              <span class="inline-block px-2 py-0.5 rounded text-[10px] font-semibold uppercase tracking-wider"
                    :class="lic.status === 'ACTIVE' ? 'bg-emerald-500/20 text-emerald-400 border border-emerald-500/30' : 'bg-slate-800 text-slate-400'">
                {{ lic.status }}
              </span>
              <div class="text-[10px] text-slate-500 mt-1 font-mono">
                Maks: {{ lic.max_terminals }} Kasir
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Log Aktivitas & Keamanan Terkini -->
      <div class="p-5 rounded-2xl glass-panel border border-slate-800/80 flex flex-col">
        <div class="flex items-center justify-between mb-4">
          <h3 class="font-bold text-sm text-white flex items-center gap-2">
            <Clock class="w-4 h-4 text-blue-400" />
            <span>Audit Trail & Tamper Alert Terkini</span>
          </h3>
          <router-link to="/audit" class="text-xs text-blue-400 hover:text-blue-300 font-semibold flex items-center gap-1">
            Lihat Semua <ArrowRight class="w-3 h-3" />
          </router-link>
        </div>

        <div v-if="providerStore.auditLogs.length === 0" class="py-8 text-center text-xs text-slate-500">
          Belum ada log aktivitas masuk.
        </div>

        <div v-else class="space-y-2 overflow-y-auto max-h-[300px]">
          <div
            v-for="log in providerStore.auditLogs.slice(0, 6)"
            :key="log.id"
            class="p-2.5 rounded-lg bg-slate-900/60 border border-slate-800 text-xs flex items-center justify-between"
          >
            <div class="flex items-center gap-2">
              <span class="px-1.5 py-0.5 rounded text-[10px] font-mono font-bold"
                    :class="log.status === 'SUCCESS' ? 'bg-emerald-500/20 text-emerald-400' : 'bg-rose-500/20 text-rose-400'">
                {{ log.action }}
              </span>
              <span class="text-slate-300 truncate max-w-[180px]">{{ log.details || '-' }}</span>
            </div>
            <div class="text-[10px] text-slate-500 font-mono">
              {{ new Date(log.created_at).toLocaleTimeString() }}
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
