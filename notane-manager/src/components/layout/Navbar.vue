<script setup lang="ts">
import { ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useProviderStore } from '../../stores/providerStore'
import { useAuthStore } from '../../stores/authStore'
import {
  Power,
  RefreshCw,
  Copy,
  Check,
  ShieldCheck,
  LogOut,
  User,
  Lock,
} from 'lucide-vue-next'
import ThemeToggle from '../common/ThemeToggle.vue'

const route = useRoute()
const router = useRouter()
const providerStore = useProviderStore()
const authStore = useAuthStore()
const copiedKey = ref(false)

const routeTitles: Record<string, { title: string; subtitle: string }> = {
  dashboard: { title: 'Pusat Kontrol Toko', subtitle: 'Ringkasan lisensi, kasir aktif, dan status layanan' },
  clients: { title: 'Daftar Pelanggan / Toko', subtitle: 'Daftar toko dan outlet ritel pengguna aplikasi' },
  tiers: { title: 'Tier Lisensi POS', subtitle: 'Pengaturan paket langganan dan bundling modul fitur retail' },
  licenses: { title: 'Pengelolaan Lisensi', subtitle: 'Penerbitan nomor lisensi, batas kasir, dan masa berlaku' },
  signer: { title: 'Aktivasi Lisensi Manual', subtitle: 'Buat kode aktivasi lisensi untuk komputer toko tanpa internet' },
  devices: { title: 'Perangkat Terdaftar', subtitle: 'Daftar komputer kasir dan server toko yang terhubung' },
  releases: { title: 'Pembaruan Aplikasi', subtitle: 'Kelola dan terbitkan pembaruan untuk server dan kasir' },
  audit: { title: 'Catatan Keamanan', subtitle: 'Riwayat pendaftaran lisensi dan pemeriksaan keamanan' },
  settings: { title: 'Pengaturan Sistem', subtitle: 'Port layanan, kunci pengaman, dan konfigurasi sistem' },
}

async function copyPublicKey() {
  await navigator.clipboard.writeText(providerStore.status.master_public_key)
  copiedKey.value = true
  setTimeout(() => (copiedKey.value = false), 2000)
}

async function refreshAll() {
  await providerStore.fetchStatus()
  await providerStore.fetchClients()
  await providerStore.fetchLicenses()
  await providerStore.fetchDevices()
  await providerStore.fetchReleases()
  await providerStore.fetchAuditLogs()
}

async function handleLogout() {
  await authStore.logout()
  router.push('/login')
}
</script>

<template>
  <header class="h-16 px-6 bg-white/90 dark:bg-slate-900/80 border-b border-slate-200 dark:border-slate-800/80 backdrop-blur-md flex items-center justify-between transition-colors duration-200">
    <div>
      <h1 class="text-base font-bold text-slate-900 dark:text-white">
        {{ routeTitles[route.name as string]?.title || 'Notane Manager' }}
      </h1>
      <p class="text-xs text-slate-500 dark:text-slate-400">
        {{ routeTitles[route.name as string]?.subtitle || 'Sistem Pengelolaan Lisensi' }}
      </p>
    </div>

    <div class="flex items-center space-x-3">
      <!-- Master Public Key Info -->
      <button
        @click="copyPublicKey"
        title="Salin Kunci Pengaman Publik"
        class="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg bg-slate-100 hover:bg-slate-200 dark:bg-slate-800/80 dark:hover:bg-slate-800 border border-slate-200 dark:border-slate-700/60 text-xs font-mono text-slate-700 dark:text-slate-300 transition-colors cursor-pointer"
      >
        <ShieldCheck class="w-3.5 h-3.5 text-emerald-600 dark:text-emerald-400" />
        <span class="truncate max-w-[120px]">{{ providerStore.status.master_public_key.slice(0, 12) }}...</span>
        <component :is="copiedKey ? Check : Copy" class="w-3 h-3 text-slate-400" />
      </button>

      <!-- Refresh Button -->
      <button
        @click="refreshAll"
        title="Muat Ulang Data"
        class="p-2 rounded-lg bg-slate-100 hover:bg-slate-200 dark:bg-slate-800/80 dark:hover:bg-slate-800 border border-slate-200 dark:border-slate-700/60 text-slate-700 dark:text-slate-300 hover:text-slate-900 dark:hover:text-white transition-colors cursor-pointer"
      >
        <RefreshCw class="w-4 h-4" :class="{ 'animate-spin': providerStore.isLoading }" />
      </button>

      <!-- Server Toggle Button -->
      <button
        @click="providerStore.toggleServer(!providerStore.status.is_server_running)"
        class="flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-semibold transition-all shadow-sm cursor-pointer"
        :class="[
          providerStore.status.is_server_running
            ? 'bg-rose-50 text-rose-600 dark:bg-rose-500/15 dark:text-rose-300 hover:bg-rose-100 dark:hover:bg-rose-500/25 border border-rose-200 dark:border-rose-500/30'
            : 'bg-emerald-50 text-emerald-600 dark:bg-emerald-500/15 dark:text-emerald-300 hover:bg-emerald-100 dark:hover:bg-emerald-500/25 border border-emerald-200 dark:border-emerald-500/30'
        ]"
      >
        <Power class="w-3.5 h-3.5" />
        <span>{{ providerStore.status.is_server_running ? 'Hentikan API' : 'Nyalakan API' }}</span>
      </button>

      <!-- Switch Dark / Light Mode -->
      <ThemeToggle />

      <!-- User Chip & Lock/Logout -->
      <div class="h-6 w-px bg-slate-200 dark:bg-slate-800 mx-1"></div>

      <div class="flex items-center gap-2">
        <div class="flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-slate-100 dark:bg-slate-800/80 border border-slate-200 dark:border-slate-700/60 text-xs text-slate-700 dark:text-slate-200 font-medium">
          <User class="w-3.5 h-3.5 text-brand-600 dark:text-brand-400" />
          <span class="truncate max-w-[100px]">{{ authStore.currentUser?.username || 'admin' }}</span>
        </div>

        <button
          @click="handleLogout"
          title="Kunci Layar & Keluar"
          class="p-2 rounded-lg bg-slate-100 hover:bg-rose-50 hover:text-rose-600 dark:bg-slate-800/80 dark:hover:bg-rose-500/20 dark:hover:text-rose-400 text-slate-500 dark:text-slate-400 border border-slate-200 dark:border-slate-700/60 transition-colors cursor-pointer"
        >
          <Lock class="w-3.5 h-3.5" />
        </button>
      </div>
    </div>
  </header>
</template>
