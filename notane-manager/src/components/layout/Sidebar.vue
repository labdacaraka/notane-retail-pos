<script setup lang="ts">
import { computed } from 'vue'
import { useRoute } from 'vue-router'
import { useProviderStore } from '../../stores/providerStore'
import {
  LayoutDashboard,
  Users,
  KeyRound,
  FileSignature,
  Cpu,
  Layers,
  ShieldAlert,
  Settings,
  Radio,
  ExternalLink,
} from 'lucide-vue-next'

const route = useRoute()
const providerStore = useProviderStore()

const navItems = [
  { name: 'Dashboard', path: '/', icon: LayoutDashboard },
  { name: 'Pelanggan Toko', path: '/clients', icon: Users, badge: () => providerStore.status.total_clients },
  { name: 'Daftar Lisensi', path: '/licenses', icon: KeyRound, badge: () => providerStore.status.total_licenses },
  { name: 'Aktivasi Manual', path: '/signer', icon: FileSignature },
  { name: 'Perangkat Kasir', path: '/devices', icon: Cpu, badge: () => providerStore.status.total_devices },
  { name: 'Pembaruan Aplikasi', path: '/releases', icon: Layers },
  { name: 'Catatan Keamanan', path: '/audit', icon: ShieldAlert },
  { name: 'Pengaturan', path: '/settings', icon: Settings },
]

const serverStatusColor = computed(() => {
  return providerStore.status.is_server_running ? 'bg-emerald-500' : 'bg-rose-500'
})

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
</script>

<template>
  <aside class="w-64 bg-slate-900/90 border-r border-slate-800/80 flex flex-col justify-between select-none">
    <div>
      <!-- Brand Logo & Header -->
      <div class="h-16 px-5 border-b border-slate-800/80 flex items-center justify-between">
        <div class="flex items-center space-x-3">
          <div class="w-9 h-9 rounded-xl bg-gradient-to-tr from-brand-600 to-emerald-400 flex items-center justify-center shadow-lg shadow-brand-500/20">
            <Radio class="w-5 h-5 text-white animate-pulse" />
          </div>
          <div>
            <div class="font-bold text-sm text-white tracking-wide flex items-center gap-1.5">
              <span>NOTANE</span>
              <span class="text-[10px] bg-brand-500/20 text-brand-400 px-1.5 py-0.5 rounded font-semibold">MANAGER</span>
            </div>
            <div class="text-[11px] text-slate-400">Pengelolaan Lisensi Toko</div>
          </div>
        </div>
      </div>

      <!-- Navigation Menu -->
      <nav class="p-3 space-y-1">
        <router-link
          v-for="item in navItems"
          :key="item.path"
          :to="item.path"
          class="flex items-center justify-between px-3 py-2.5 rounded-lg text-sm font-medium transition-all duration-150"
          :class="[
            route.path === item.path
              ? 'bg-brand-500/15 text-brand-700 dark:text-brand-400 border border-brand-500/30 shadow-xs font-semibold'
              : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-slate-100 hover:bg-slate-100 dark:hover:bg-slate-800/60'
          ]"
        >
          <div class="flex items-center space-x-3">
            <component :is="item.icon" class="w-4 h-4 shrink-0" />
            <span>{{ item.name }}</span>
          </div>
          <span
            v-if="item.badge && item.badge() !== undefined && item.badge() > 0"
            class="text-[11px] font-mono px-1.5 py-0.5 rounded-full bg-slate-100 dark:bg-slate-800 text-slate-700 dark:text-slate-300 border border-slate-200 dark:border-slate-700/60"
          >
            {{ item.badge() }}
          </span>
        </router-link>
      </nav>
    </div>

    <!-- Server Daemon Status Card at Bottom -->
    <div class="p-3.5 border-t border-slate-200 dark:border-slate-800/80 bg-slate-100/60 dark:bg-slate-950/40">
      <div class="p-3 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 flex flex-col gap-2 shadow-2xs">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-2">
            <span class="relative flex h-2.5 w-2.5">
              <span v-if="providerStore.status.is_server_running" class="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"></span>
              <span class="relative inline-flex rounded-full h-2.5 w-2.5" :class="serverStatusColor"></span>
            </span>
            <span class="text-xs font-semibold text-slate-800 dark:text-slate-300">
              {{ providerStore.status.is_server_running ? 'API Aktif' : 'API Nonaktif' }}
            </span>
          </div>
          <span class="text-[11px] font-mono text-slate-600 dark:text-slate-400 bg-slate-100 dark:bg-slate-800 border border-slate-200 dark:border-transparent px-1.5 py-0.5 rounded">
            Port {{ providerStore.status.port }}
          </span>
        </div>
        <div class="text-[11px] text-slate-500 dark:text-slate-400 truncate font-mono">
          {{ displayExternalUrl }}
        </div>
      </div>
    </div>
  </aside>
</template>
