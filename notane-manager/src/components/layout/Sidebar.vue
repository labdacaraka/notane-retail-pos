<script setup lang="ts">
import { ref, computed } from 'vue'
import { useRoute } from 'vue-router'
import { useProviderStore } from '../../stores/providerStore'
import {
  LayoutDashboard,
  Users,
  Award,
  KeyRound,
  FileSignature,
  Cpu,
  Layers,
  ShieldAlert,
  Settings,
  Radio,
  PanelLeftClose,
  PanelLeft,
} from 'lucide-vue-next'

const route = useRoute()
const providerStore = useProviderStore()

const isCollapsed = ref(localStorage.getItem('notane_manager_sidebar_collapsed') === 'true')

function toggleSidebar() {
  isCollapsed.value = !isCollapsed.value
  localStorage.setItem('notane_manager_sidebar_collapsed', String(isCollapsed.value))
}

const navItems = [
  { name: 'Dashboard', path: '/', icon: LayoutDashboard },
  { name: 'Pelanggan Toko', path: '/clients', icon: Users, badge: () => providerStore.status.total_clients },
  { name: 'Tier Lisensi', path: '/tiers', icon: Award },
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
  <aside
    :class="isCollapsed ? 'w-20' : 'w-64'"
    class="bg-white dark:bg-slate-900 border-r border-slate-200 dark:border-slate-800 flex flex-col justify-between select-none transition-all duration-200 shrink-0"
  >
    <div>
      <!-- Brand Logo & Header with Toggle Button -->
      <div
        class="h-16 px-4 border-b border-slate-200 dark:border-slate-800 flex items-center justify-between"
        :class="{ 'justify-center': isCollapsed }"
      >
        <!-- Expanded Brand Info -->
        <div v-if="!isCollapsed" class="flex items-center space-x-3 overflow-hidden">
          <div class="w-9 h-9 shrink-0 rounded-xl bg-gradient-to-tr from-brand-600 to-indigo-600 flex items-center justify-center shadow-md shadow-brand-500/20">
            <Radio class="w-5 h-5 text-white" />
          </div>
          <div class="truncate">
            <div class="font-bold text-sm text-slate-900 dark:text-white tracking-wide flex items-center gap-1.5">
              <span>NOTANE</span>
              <span class="text-[10px] bg-brand-500/15 text-brand-600 dark:text-brand-400 px-1.5 py-0.5 rounded font-semibold">MANAGER</span>
            </div>
            <div class="text-[11px] text-slate-500 dark:text-slate-400 truncate">Studio Lisensi Retail</div>
          </div>
        </div>

        <!-- Collapsed Icon Only -->
        <div v-else class="flex flex-col items-center">
          <div class="w-9 h-9 rounded-xl bg-gradient-to-tr from-brand-600 to-indigo-600 flex items-center justify-center shadow-md shadow-brand-500/20 mb-1">
            <Radio class="w-5 h-5 text-white" />
          </div>
        </div>

        <!-- Toggle Button -->
        <button
          @click="toggleSidebar"
          :title="isCollapsed ? 'Perluas Menu Sidebar' : 'Sembunyikan / Perkecil Sidebar'"
          class="p-1.5 rounded-lg text-slate-400 hover:text-slate-700 dark:hover:text-slate-200 hover:bg-slate-100 dark:hover:bg-slate-800 transition cursor-pointer"
          :class="{ 'mt-1': isCollapsed }"
        >
          <PanelLeftClose v-if="!isCollapsed" class="w-4 h-4" />
          <PanelLeft v-else class="w-4 h-4" />
        </button>
      </div>

      <!-- Navigation Menu -->
      <nav class="p-2 space-y-1">
        <router-link
          v-for="item in navItems"
          :key="item.path"
          :to="item.path"
          class="group relative flex items-center rounded-xl text-sm font-medium transition-all duration-150 cursor-pointer"
          :class="[
            isCollapsed ? 'justify-center py-2.5 px-0' : 'justify-between px-3 py-2.5',
            route.path === item.path
              ? 'bg-brand-50 dark:bg-brand-500/15 text-brand-600 dark:text-brand-400 border border-brand-200 dark:border-brand-500/30 font-semibold shadow-xs'
              : 'text-slate-600 dark:text-slate-400 hover:text-slate-900 dark:hover:text-white hover:bg-slate-100 dark:hover:bg-slate-800/60'
          ]"
        >
          <!-- Left side icon and name -->
          <div class="flex items-center space-x-3" :class="{ 'justify-center': isCollapsed }">
            <component
              :is="item.icon"
              class="w-4 h-4 shrink-0 transition-colors"
              :class="route.path === item.path ? 'text-brand-600 dark:text-brand-400' : 'text-slate-400 group-hover:text-slate-700 dark:group-hover:text-slate-200'"
            />
            <span v-if="!isCollapsed" class="truncate">{{ item.name }}</span>
          </div>

          <!-- Badge when expanded -->
          <span
            v-if="!isCollapsed && item.badge && item.badge() !== undefined && item.badge() > 0"
            class="text-[11px] font-mono px-1.5 py-0.5 rounded-full bg-slate-100 dark:bg-slate-800 text-slate-700 dark:text-slate-300 border border-slate-200 dark:border-slate-700/60"
          >
            {{ item.badge() }}
          </span>

          <!-- Floating Tooltip on Hover when Collapsed -->
          <div
            v-if="isCollapsed"
            class="absolute left-full ml-3 px-3 py-1.5 rounded-xl bg-slate-900 dark:bg-slate-800 text-white text-xs font-semibold shadow-xl opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none whitespace-nowrap z-50 border border-slate-700/60 flex items-center gap-2"
          >
            <span>{{ item.name }}</span>
            <span
              v-if="item.badge && item.badge() !== undefined && item.badge() > 0"
              class="text-[10px] font-mono px-1.5 py-0.2 rounded-full bg-brand-500/30 text-brand-300 font-bold"
            >
              {{ item.badge() }}
            </span>
          </div>
        </router-link>
      </nav>
    </div>

    <!-- Server Daemon Status Card at Bottom -->
    <div class="p-3 border-t border-slate-200 dark:border-slate-800 bg-slate-50/70 dark:bg-slate-950/40">
      <!-- Expanded status card -->
      <div
        v-if="!isCollapsed"
        class="p-3 rounded-xl bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 flex flex-col gap-1.5 shadow-xs"
      >
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-2">
            <span class="relative flex h-2.5 w-2.5">
              <span
                v-if="providerStore.status.is_server_running"
                class="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"
              ></span>
              <span class="relative inline-flex rounded-full h-2.5 w-2.5" :class="serverStatusColor"></span>
            </span>
            <span class="text-xs font-semibold text-slate-800 dark:text-slate-200">
              {{ providerStore.status.is_server_running ? 'API Aktif' : 'API Nonaktif' }}
            </span>
          </div>
          <span class="text-[11px] font-mono text-slate-600 dark:text-slate-400 bg-slate-100 dark:bg-slate-800 px-1.5 py-0.5 rounded border border-slate-200 dark:border-slate-700">
            Port {{ providerStore.status.port }}
          </span>
        </div>
        <div class="text-[11px] text-slate-500 dark:text-slate-400 truncate font-mono">
          {{ displayExternalUrl }}
        </div>
      </div>

      <!-- Collapsed status icon with tooltip -->
      <div v-else class="relative group flex justify-center py-1">
        <span class="relative flex h-3.5 w-3.5 cursor-pointer">
          <span
            v-if="providerStore.status.is_server_running"
            class="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"
          ></span>
          <span class="relative inline-flex rounded-full h-3.5 w-3.5" :class="serverStatusColor"></span>
        </span>
        <!-- Tooltip -->
        <div
          class="absolute left-full ml-3 px-3 py-1.5 rounded-xl bg-slate-900 dark:bg-slate-800 text-white text-xs font-semibold shadow-xl opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none whitespace-nowrap z-50 border border-slate-700/60"
        >
          {{ providerStore.status.is_server_running ? `API Aktif (Port ${providerStore.status.port})` : 'API Nonaktif' }}
        </div>
      </div>
    </div>
  </aside>
</template>
