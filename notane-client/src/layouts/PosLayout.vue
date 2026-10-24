<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { LogOut, Volume2, VolumeX } from 'lucide-vue-next'
import { Icon } from '@iconify/vue'
import { useAuthStore } from '@/stores/authStore'
import { useShiftStore } from '@/stores/shiftStore'
import { useLocationStore } from '@/stores/locationStore'
import { useNetworkStore } from '@/stores/networkStore'
import { usePrinterStore } from '@/stores/printerStore'
import ShiftModal from '@/components/pos/ShiftModal.vue'
import NetworkConnectionModal from '@/components/common/NetworkConnectionModal.vue'
import PrinterSettingsModal from '@/components/pos/PrinterSettingsModal.vue'
import UserProfileSecurityModal from '@/components/common/UserProfileSecurityModal.vue'
import SalesReturnModal from '@/components/pos/SalesReturnModal.vue'
import CashDropModal from '@/components/pos/CashDropModal.vue'
import { openCustomerDisplayWindow } from '@/services/customerDisplayChannel'
import { soundService } from '@/services/soundService'
import { useDialog } from '@/composables/useDialog'

import { useStoreBrandingStore } from '@/stores/storeBrandingStore'

const router = useRouter()
const authStore = useAuthStore()
const shiftStore = useShiftStore()
const networkStore = useNetworkStore()
const printerStore = usePrinterStore()
const locationStore = useLocationStore()
const storeBrandingStore = useStoreBrandingStore()
const dialog = useDialog()

const showNetworkModal = ref<boolean>(false)
const showPrinterModal = ref<boolean>(false)
const showSecurityModal = ref<boolean>(false)
const showSalesReturnModal = ref<boolean>(false)
const showCashDropModal = ref<boolean>(false)
const isSoundOn = ref<boolean>(soundService.isEnabled())
const currentTime = ref<string>('')
let timer: any = null

// Dropdown Popovers State
const showSystemMenu = ref<boolean>(false)
const showUserMenu = ref<boolean>(false)
const showShiftMenu = ref<boolean>(false)
const systemMenuRef = ref<HTMLElement | null>(null)
const userMenuRef = ref<HTMLElement | null>(null)
const shiftMenuRef = ref<HTMLElement | null>(null)

function getUserInitials(name?: string): string {
  if (!name) return 'K'
  const parts = name.trim().split(' ').filter(Boolean)
  if (parts.length >= 2) {
    return (parts[0][0] + parts[1][0]).toUpperCase()
  }
  return name.slice(0, 2).toUpperCase()
}

function formatCurrency(val: number): string {
  return new Intl.NumberFormat('id-ID', {
    style: 'currency',
    currency: 'IDR',
    minimumFractionDigits: 0,
    maximumFractionDigits: 0,
  }).format(val)
}

function formatShiftTime(iso?: string): string {
  if (!iso) return '-'
  try {
    const d = new Date(iso)
    return d.toLocaleTimeString('id-ID', { hour: '2-digit', minute: '2-digit' })
  } catch {
    return '-'
  }
}

function handleGlobalClick(e: MouseEvent) {
  const target = e.target as HTMLElement
  if (systemMenuRef.value && !systemMenuRef.value.contains(target)) {
    showSystemMenu.value = false
  }
  if (userMenuRef.value && !userMenuRef.value.contains(target)) {
    showUserMenu.value = false
  }
  if (shiftMenuRef.value && !shiftMenuRef.value.contains(target)) {
    showShiftMenu.value = false
  }
}

function toggleSound() {
  const next = !isSoundOn.value
  soundService.setEnabled(next)
  isSoundOn.value = next
  if (next) {
    soundService.playScanSuccess()
    dialog.toast('Suara feedback kasir diaktifkan', 'success')
  } else {
    dialog.toast('Suara kasir dibisukan (Mute)', 'info')
  }
}

function updateTime() {
  const now = new Date()
  currentTime.value = now.toLocaleTimeString('id-ID', { hour: '2-digit', minute: '2-digit', second: '2-digit' })
}

function handleOpenShiftModal() {
  shiftStore.openShiftModal('OPEN')
}

function handleCloseShiftModal() {
  shiftStore.openShiftModal('CLOSE')
}

function openCustomerDisplay() {
  openCustomerDisplayWindow()
}

function handleLogout() {
  showUserMenu.value = false
  authStore.logout()
  router.push({ name: 'login' })
}

async function handleExitImpersonation() {
  await authStore.stopImpersonating()
  dialog.toast('Anda telah kembali ke akun asli Anda.', 'info')
}

onMounted(() => {
  updateTime()
  timer = setInterval(updateTime, 1000)
  shiftStore.checkCurrentShift()
  locationStore.fetchAccessibleOutlets()
  storeBrandingStore.fetchBranding()
  document.addEventListener('mousedown', handleGlobalClick)
  // Hanya sambung otomatis jika intent sebelumnya adalah CONNECTED dan autoReconnect aktif
  if (networkStore.connectionIntent === 'CONNECTED' && networkStore.autoReconnect && networkStore.connectionStatus !== 'CONNECTED') {
    networkStore.connect()
  }
})

onUnmounted(() => {
  if (timer) clearInterval(timer)
  document.removeEventListener('mousedown', handleGlobalClick)
})
</script>

<template>
  <div class="h-screen flex flex-col bg-slate-100 text-slate-900 overflow-hidden select-none">
    <!-- Impersonation Active Banner -->
    <div
      v-if="authStore.isImpersonating"
      class="bg-gradient-to-r from-purple-800 via-indigo-800 to-purple-900 text-white px-4 py-2 flex items-center justify-between shadow-md z-40 text-xs shrink-0 border-b border-purple-950"
    >
      <div class="flex items-center gap-2.5 font-medium">
        <span class="p-1 rounded-md bg-purple-600/60 border border-purple-400/40 text-white animate-pulse">
          <Icon icon="fluent-color:people-sync-24" class="w-4 h-4" />
        </span>
        <span>
          Mode Impersonate: Anda sedang login sebagai <strong>{{ authStore.user?.full_name }}</strong> (<span class="font-mono text-purple-200 font-bold">{{ authStore.user?.role_name }}</span>).
        </span>
      </div>

      <button
        @click="handleExitImpersonation"
        class="bg-white/15 hover:bg-white text-white hover:text-purple-950 border border-white/30 px-3 py-1 rounded-lg font-bold text-xs transition flex items-center gap-1.5 cursor-pointer shadow-xs active:scale-95"
      >
        <Icon icon="flat-color-icons:leave" class="w-4 h-4" />
        <span>Kembali ke Akun Asli ({{ authStore.impersonator?.full_name || authStore.impersonator?.username || 'Admin' }})</span>
      </button>
    </div>

    <!-- Top POS Header Bar (Symmetrical & Segmented Grouping) -->
    <header class="bg-white border-b border-slate-200 px-4 h-14 flex items-center justify-between select-none shadow-xs z-30 shrink-0">
      <!-- Left Zone: Identitas Brand & Hardware Hub (Group 1 & 2) -->
      <div class="flex-1 flex items-center justify-start gap-2 min-w-0">
        <!-- Group 1: Brand Toko & Co-branding Notane POS -->
        <div class="inline-flex items-center gap-2.5 bg-slate-100/80 border border-slate-200/80 rounded-xl p-1 pr-3 shadow-2xs shrink-0">
          <!-- Logo Toko Utama (Eyecatching) -->
          <div class="relative shrink-0">
            <img
              v-if="storeBrandingStore.storeLogoUrl"
              :src="storeBrandingStore.storeLogoUrl"
              alt="Logo Toko"
              class="w-7 h-7 rounded-lg shadow-xs border border-slate-200/90 bg-white object-contain hover:scale-105 transition p-0.5"
            />
            <div
              v-else
              class="w-7 h-7 rounded-lg shadow-xs border border-indigo-200 bg-gradient-to-tr from-indigo-600 to-indigo-500 text-white font-black text-xs flex items-center justify-center hover:scale-105 transition"
            >
              {{ (storeBrandingStore.storeName || 'T').charAt(0).toUpperCase() }}
            </div>
          </div>

          <!-- Nama Toko & Powered by Notane POS -->
          <div class="leading-none text-left">
            <div class="flex items-center gap-1.5">
              <span
                class="font-black text-xs text-slate-900 tracking-tight max-w-[130px] lg:max-w-[160px] truncate block"
                :title="storeBrandingStore.storeName"
              >
                {{ storeBrandingStore.storeName }}
              </span>
            </div>
            <!-- Subtitle Co-branding & Cabang -->
            <div class="flex items-center gap-1.5 mt-0.5">
              <span class="text-[9px] text-slate-500 font-medium max-w-[85px] truncate">
                {{ shiftStore.currentShift?.outlet_name || locationStore.currentOutlet?.name || 'Cabang Aktif' }}
              </span>
              <template v-if="!storeBrandingStore.removeBranding">
                <span class="text-slate-300 text-[9px]">•</span>
                <div class="flex items-center gap-1" title="Sistem Resmi Notane Retail POS">
                  <img src="/logo.png" alt="Notane" class="w-2.5 h-2.5 object-contain rounded opacity-80" />
                  <span class="text-[8px] font-mono font-bold text-slate-400 uppercase tracking-tight">Notane POS</span>
                </div>
              </template>
            </div>
          </div>
        </div>

        <!-- Group 2: Unified Hardware & Station Status Hub Capsule -->
        <div ref="systemMenuRef" class="relative shrink-0">
          <button
            type="button"
            @click="showSystemMenu = !showSystemMenu"
            class="h-9 inline-flex items-center gap-2 px-2.5 rounded-xl border transition-all cursor-pointer shadow-2xs active:scale-95"
            :class="networkStore.connectionStatus === 'CONNECTED'
              ? 'bg-slate-100/80 hover:bg-slate-200/80 border-slate-200/80 text-slate-700'
              : 'bg-rose-50 hover:bg-rose-100 border-rose-200 text-rose-800'"
            title="Klik untuk melihat detail koneksi server host, printer thermal, dan terminal"
          >
            <!-- Dot Network Status -->
            <span class="flex items-center gap-1.5">
              <span
                class="w-2 h-2 rounded-full"
                :class="{
                  'bg-emerald-500 animate-pulse': networkStore.connectionStatus === 'CONNECTED',
                  'bg-rose-500': networkStore.connectionStatus === 'DISCONNECTED' || networkStore.connectionStatus === 'ERROR',
                  'bg-amber-500 animate-ping': networkStore.connectionStatus === 'CONNECTING'
                }"
              ></span>
              <span class="text-[11px] font-bold hidden lg:inline">
                {{ networkStore.connectionStatus === 'CONNECTED' ? 'Online' : 'Offline' }}
              </span>
            </span>

            <span class="h-3.5 w-px bg-slate-300"></span>

            <!-- Printer Status Icon -->
            <Icon
              icon="streamline-ultimate-color:print-text"
              class="w-3.5 h-3.5"
              :class="printerStore.isConnected ? 'opacity-100' : 'opacity-40 grayscale'"
            />

            <span class="h-3.5 w-px bg-slate-300"></span>

            <!-- Terminal ID -->
            <span class="text-[10px] font-mono font-bold text-slate-600 bg-white/70 px-1 py-0.5 rounded border border-slate-200/60">
              {{ authStore.terminalId }}
            </span>

            <Icon icon="fluent:chevron-down-12-regular" class="w-3 h-3 text-slate-400" />
          </button>

          <!-- System Status Popover Dropdown Card -->
          <div
            v-if="showSystemMenu"
            class="absolute left-0 top-full mt-2 w-80 bg-white rounded-xl shadow-2xl border border-slate-200 z-50 p-3.5 space-y-3 animate-in fade-in zoom-in-95 duration-100 text-left"
          >
            <div class="flex items-center justify-between pb-2 border-b border-slate-100">
              <div class="font-bold text-xs text-slate-900 flex items-center gap-1.5">
                <Icon icon="fluent-color:server-multiple-20" class="w-4 h-4" />
                <span>Status Perangkat & Jaringan</span>
              </div>
              <span class="text-[10px] font-mono font-bold px-2 py-0.5 rounded bg-blue-50 text-blue-700 border border-blue-200">
                Term: {{ authStore.terminalId }}
              </span>
            </div>

            <!-- Section 1: Server LAN Host -->
            <div class="bg-slate-50 rounded-lg p-2.5 border border-slate-200/80">
              <div class="flex items-center justify-between mb-1">
                <span class="text-[11px] font-bold text-slate-700 flex items-center gap-1.5">
                  <span
                    class="w-2 h-2 rounded-full"
                    :class="{
                      'bg-emerald-500': networkStore.connectionStatus === 'CONNECTED',
                      'bg-rose-500': networkStore.connectionStatus === 'DISCONNECTED' || networkStore.connectionStatus === 'ERROR',
                      'bg-amber-500': networkStore.connectionStatus === 'CONNECTING'
                    }"
                  ></span>
                  Server LAN Host
                </span>
                <span
                  class="text-[10px] font-mono font-bold px-1.5 py-0.5 rounded"
                  :class="networkStore.connectionStatus === 'CONNECTED' ? 'bg-emerald-100 text-emerald-800' : 'bg-rose-100 text-rose-800'"
                >
                  {{ networkStore.connectionStatus === 'CONNECTED' ? (networkStore.latencyMs !== null ? `${networkStore.latencyMs}ms` : 'Aktif') : 'Terputus' }}
                </span>
              </div>
              <div class="text-[10px] text-slate-500 font-mono mb-2">
                Target: {{ networkStore.targetHost }}:{{ networkStore.targetPort }}
              </div>
              <button
                type="button"
                @click="showSystemMenu = false; showNetworkModal = true"
                class="w-full bg-white hover:bg-slate-100 text-slate-700 border border-slate-300 text-xs font-bold py-1 px-2 rounded-md transition shadow-2xs flex items-center justify-center gap-1 cursor-pointer"
              >
                <Icon icon="fluent:settings-16-regular" class="w-3.5 h-3.5" />
                <span>Konfigurasi Jaringan & Host</span>
              </button>
            </div>

            <!-- Section 2: Thermal Printer -->
            <div class="bg-slate-50 rounded-lg p-2.5 border border-slate-200/80">
              <div class="flex items-center justify-between mb-1">
                <span class="text-[11px] font-bold text-slate-700 flex items-center gap-1.5">
                  <Icon icon="streamline-ultimate-color:print-text" class="w-3.5 h-3.5" />
                  Thermal Printer
                </span>
                <span
                  class="text-[10px] font-mono font-bold px-1.5 py-0.5 rounded"
                  :class="printerStore.isConnected ? 'bg-emerald-100 text-emerald-800' : 'bg-slate-200 text-slate-700'"
                >
                  {{ printerStore.isConnected ? printerStore.paperWidth : 'Belum Diatur' }}
                </span>
              </div>
              <div class="text-[10px] text-slate-600 truncate mb-2">
                {{ printerStore.printerName }}
                <span class="text-slate-400">({{ printerStore.connectionType }})</span>
              </div>
              <button
                type="button"
                @click="showSystemMenu = false; showPrinterModal = true"
                class="w-full bg-white hover:bg-slate-100 text-slate-700 border border-slate-300 text-xs font-bold py-1 px-2 rounded-md transition shadow-2xs flex items-center justify-center gap-1 cursor-pointer"
              >
                <Icon icon="fluent:print-16-regular" class="w-3.5 h-3.5" />
                <span>Pengaturan Printer Struk</span>
              </button>
            </div>

            <!-- Section 3: Offline Storage Readiness -->
            <div class="flex items-center justify-between text-[11px] text-slate-600 px-1 pt-1">
              <span class="flex items-center gap-1 text-slate-500 font-medium">
                <Icon icon="fluent-color:database-plug-connected-20" class="w-3.5 h-3.5" />
                Penyimpanan Lokal:
              </span>
              <span class="font-bold text-emerald-700 bg-emerald-50 px-2 py-0.5 rounded border border-emerald-200 text-[10px]">
                SQLite Buffer Siap
              </span>
            </div>
          </div>
        </div>
      </div>

      <!-- Center Zone: Operasional Kasir & Jam (Simetris Persis di Sumbu Tengah) -->
      <div class="flex-none flex items-center justify-center px-2">
        <!-- Segmented Capsule Operasional Kasir (Shift Aktif) -->
        <div v-if="shiftStore.isShiftOpen" class="inline-flex items-center p-1 bg-slate-100/90 border border-slate-200/90 rounded-xl shadow-2xs gap-1">
          <!-- Dropdown Tombol Shift Aktif ▾ -->
          <div ref="shiftMenuRef" class="relative">
            <button
              type="button"
              @click="showShiftMenu = !showShiftMenu"
              class="h-[30px] inline-flex items-center gap-1.5 bg-emerald-600 hover:bg-emerald-700 text-white px-2.5 rounded-lg text-xs font-bold transition-all shadow-xs active:scale-95 cursor-pointer"
              title="Klik untuk menu operasional shift (Setor Kas, Tutup Shift)"
            >
              <span class="w-1.5 h-1.5 rounded-full bg-white animate-ping"></span>
              <span>Shift Aktif</span>
              <Icon icon="fluent:chevron-down-12-regular" class="w-3 h-3 text-emerald-100 ml-0.5" />
            </button>

            <!-- Shift Management Popover Card -->
            <div
              v-if="showShiftMenu"
              class="absolute left-1/2 -translate-x-1/2 top-full mt-2 w-72 bg-white rounded-xl shadow-2xl border border-slate-200 z-50 p-3 space-y-2.5 animate-in fade-in zoom-in-95 duration-100 text-left"
            >
              <!-- Shift Overview Header -->
              <div class="p-2.5 bg-emerald-50/70 rounded-lg border border-emerald-100 text-left">
                <div class="flex items-center justify-between mb-1">
                  <span class="text-xs font-bold text-emerald-950 flex items-center gap-1.5">
                    <Icon icon="streamline-ultimate-color:time-clock-circle" class="w-4 h-4" />
                    Shift Sedang Berjalan
                  </span>
                  <span class="text-[10px] font-mono bg-emerald-200/80 text-emerald-900 px-1.5 py-0.5 rounded font-bold">
                    Term: {{ authStore.terminalId }}
                  </span>
                </div>
                <div class="text-[11px] text-slate-600 space-y-1 font-medium mt-1.5 pt-1.5 border-t border-emerald-200/60">
                  <div class="flex justify-between items-center">
                    <span class="text-slate-500">Dibuka Pukul:</span>
                    <strong class="text-slate-800 font-mono">{{ formatShiftTime(shiftStore.currentShift?.opened_at) }}</strong>
                  </div>
                  <div class="flex justify-between items-center">
                    <span class="text-slate-500">Modal Kas Awal:</span>
                    <strong class="text-emerald-700 font-mono">{{ formatCurrency(shiftStore.currentShift?.opening_cash || 0) }}</strong>
                  </div>
                </div>
              </div>

              <!-- Action: Setor Kas (Cash Drop) -->
              <button
                type="button"
                @click="showShiftMenu = false; showCashDropModal = true"
                class="w-full flex items-center gap-2.5 p-2 rounded-lg text-xs font-medium text-amber-900 bg-amber-50/80 hover:bg-amber-100 border border-amber-200/90 transition cursor-pointer text-left group shadow-2xs"
              >
                <div class="w-7 h-7 rounded-md bg-amber-200/80 text-amber-800 flex items-center justify-center shrink-0 group-hover:scale-105 transition">
                  <Icon icon="fluent:vault-20-filled" class="w-4 h-4" />
                </div>
                <div>
                  <div class="font-bold text-amber-950 leading-tight">Setor Kas (Cash Drop)</div>
                  <div class="text-[10px] text-amber-700/80 mt-0.5">Pindahkan kas berlebih laci ke brankas</div>
                </div>
              </button>

              <!-- Action: Tutup Shift (Settlement) -->
              <button
                type="button"
                @click="showShiftMenu = false; handleCloseShiftModal()"
                class="w-full flex items-center gap-2.5 p-2 rounded-lg text-xs font-medium text-slate-700 bg-slate-50 hover:bg-slate-100 border border-slate-200 transition cursor-pointer text-left group shadow-2xs"
              >
                <div class="w-7 h-7 rounded-md bg-slate-200 text-slate-700 flex items-center justify-center shrink-0 group-hover:scale-105 transition">
                  <Icon icon="streamline-ultimate-color:time-clock-circle" class="w-4 h-4" />
                </div>
                <div>
                  <div class="font-bold text-slate-900 leading-tight">Tutup Shift (Settlement)</div>
                  <div class="text-[10px] text-slate-500 mt-0.5">Rekap fisik kas & akhiri sesi kasir</div>
                </div>
              </button>
            </div>
          </div>

          <!-- Digital Clock Monospace -->
          <div class="h-[30px] inline-flex items-center gap-1.5 px-2.5 bg-white rounded-lg border border-slate-200/70 shadow-2xs font-mono text-xs font-bold text-slate-700">
            <Icon icon="fluent:clock-12-regular" class="w-3.5 h-3.5 text-slate-400" />
            <span>{{ currentTime }}</span>
          </div>

          <!-- Quick Action: Retur (F7) -->
          <button
            type="button"
            @click="showSalesReturnModal = true"
            class="h-[30px] inline-flex items-center gap-1.5 bg-white hover:bg-rose-50 text-rose-700 hover:text-rose-800 border border-slate-200/70 hover:border-rose-200 px-2.5 rounded-lg text-xs font-bold transition-all shadow-2xs active:scale-95 cursor-pointer"
            title="Klaim retur faktur belanja konsumen (Shortcut: F7)"
          >
            <Icon icon="fluent:arrow-undo-20-filled" class="w-3.5 h-3.5 text-rose-600" />
            <span class="hidden md:inline">Retur</span>
            <span class="text-[10px] font-mono text-rose-500 font-bold">(F7)</span>
          </button>
        </div>

        <!-- Segmented Capsule (Shift Belum Dibuka) -->
        <div v-else class="inline-flex items-center p-1 bg-slate-100/90 border border-slate-200/90 rounded-xl shadow-2xs gap-1.5">
          <button
            type="button"
            @click="handleOpenShiftModal"
            class="h-[30px] inline-flex items-center gap-1.5 bg-emerald-600 hover:bg-emerald-700 text-white font-bold px-3 rounded-lg text-xs transition-all active:scale-95 shadow-xs cursor-pointer"
          >
            <Icon icon="streamline-ultimate-color:time-clock-circle" class="w-3.5 h-3.5" />
            <span>Buka Shift Kasir</span>
          </button>
          <div class="h-[30px] inline-flex items-center gap-1.5 px-2.5 bg-white rounded-lg border border-slate-200/70 shadow-2xs font-mono text-xs font-bold text-slate-600">
            <Icon icon="fluent:clock-12-regular" class="w-3.5 h-3.5 text-slate-400" />
            <span>{{ currentTime }}</span>
          </div>
        </div>
      </div>

      <!-- Right Zone: Utilities & User Security Menu (Group 3 & 4) -->
      <div class="flex-1 flex items-center justify-end gap-2 min-w-0">
        <!-- Group 3: Quick Utilities Segmented Toolbar -->
        <div class="inline-flex items-center p-1 bg-slate-100/80 border border-slate-200/80 rounded-xl shadow-2xs gap-1 shrink-0">
          <!-- Sound Toggle -->
          <button
            type="button"
            @click="toggleSound"
            class="w-7 h-7 rounded-lg flex items-center justify-center transition cursor-pointer"
            :class="isSoundOn ? 'bg-white text-emerald-700 shadow-2xs border border-slate-200/70' : 'text-slate-400 hover:bg-slate-200/60'"
            :title="isSoundOn ? 'Suara Kasir: Aktif (Klik untuk Mute)' : 'Suara Kasir: Bisu/Mute (Klik untuk Aktifkan)'"
          >
            <Volume2 v-if="isSoundOn" class="w-3.5 h-3.5 text-emerald-600 drop-shadow-2xs" />
            <VolumeX v-else class="w-3.5 h-3.5 text-slate-400" />
          </button>

          <!-- Customer Display -->
          <button
            type="button"
            @click="openCustomerDisplay"
            class="w-7 h-7 rounded-lg flex items-center justify-center text-slate-600 hover:bg-white hover:shadow-2xs transition cursor-pointer border border-transparent hover:border-slate-200/70"
            title="Buka Layar Hadap Pelanggan (Customer Display)"
          >
            <Icon icon="streamline-ultimate-color:wifi-laptop" class="w-3.5 h-3.5" />
          </button>

          <!-- Back-Office Navigation -->
          <button
            type="button"
            @click="router.push('/admin/products')"
            class="h-7 px-2 rounded-lg flex items-center gap-1 text-slate-700 hover:bg-white hover:shadow-2xs transition cursor-pointer text-xs font-bold border border-transparent hover:border-slate-200/70"
            title="Akses Dashboard Back-Office Administrator"
          >
            <Icon icon="streamline-ultimate-color:layout-dashboard" class="w-3.5 h-3.5" />
            <span class="hidden xl:inline text-[11px]">Back-Office</span>
          </button>
        </div>

        <!-- Group 4: Kasir Profil & Logout Segmented Capsule -->
        <div class="inline-flex items-center p-1 bg-slate-100/80 border border-slate-200/80 rounded-xl shadow-2xs gap-1 shrink-0">
          <!-- User Profile Dropdown Pill -->
          <div ref="userMenuRef" class="relative">
            <button
              type="button"
              @click="showUserMenu = !showUserMenu"
              class="h-7 inline-flex items-center gap-1.5 px-1.5 rounded-lg hover:bg-white hover:shadow-2xs transition cursor-pointer border border-transparent hover:border-slate-200/70 group"
              title="Menu Pengguna & Keamanan Akun"
            >
              <!-- Avatar Circle -->
              <div class="w-5 h-5 rounded-full bg-gradient-to-tr from-indigo-600 to-indigo-500 text-white font-black text-[10px] flex items-center justify-center shadow-xs">
                {{ getUserInitials(authStore.user?.full_name) }}
              </div>

              <!-- User Name -->
              <span class="text-xs font-bold text-slate-800 truncate max-w-[85px] lg:max-w-[110px] hidden sm:inline">
                {{ authStore.user?.full_name?.split(' ')[0] || authStore.user?.username }}
              </span>

              <Icon icon="fluent:chevron-down-12-regular" class="w-3 h-3 text-slate-400 group-hover:text-slate-600" />
            </button>

            <!-- User Menu Popover Card -->
            <div
              v-if="showUserMenu"
              class="absolute right-0 top-full mt-2 w-64 bg-white rounded-xl shadow-2xl border border-slate-200 z-50 p-2.5 space-y-2 animate-in fade-in zoom-in-95 duration-100 text-left"
            >
              <!-- User Information Header -->
              <div class="p-2 bg-slate-50 rounded-lg border border-slate-200/80">
                <div class="flex items-center gap-2.5 mb-1.5">
                  <div class="w-9 h-9 rounded-full bg-indigo-600 text-white font-black text-sm flex items-center justify-center shadow-xs">
                    {{ getUserInitials(authStore.user?.full_name) }}
                  </div>
                  <div class="overflow-hidden">
                    <div class="text-xs font-bold text-slate-900 truncate">
                      {{ authStore.user?.full_name }}
                    </div>
                    <div class="text-[10px] font-mono text-slate-500 truncate">
                      @{{ authStore.user?.username }}
                    </div>
                  </div>
                </div>
                <div class="flex items-center justify-between text-[10px] font-mono pt-1 border-t border-slate-200 text-slate-600">
                  <span>Peran: <strong class="text-indigo-700 font-bold">{{ authStore.user?.role_name }}</strong></span>
                  <span>Term: <strong class="text-slate-800">{{ authStore.terminalId }}</strong></span>
                </div>
              </div>

              <!-- Action: Ubah Password & PIN Kasir -->
              <button
                type="button"
                @click="showUserMenu = false; showSecurityModal = true"
                class="w-full flex items-center gap-2 px-2.5 py-2 rounded-lg text-xs font-medium text-slate-700 hover:bg-slate-100 hover:text-indigo-700 transition cursor-pointer text-left"
              >
                <Icon icon="flat-color-icons:key" class="w-4 h-4" />
                <span>Ganti PIN & Password</span>
              </button>

              <!-- Action: Logout -->
              <button
                type="button"
                @click="handleLogout"
                class="w-full flex items-center gap-2 px-2.5 py-2 rounded-lg text-xs font-bold text-rose-700 hover:bg-rose-50 transition cursor-pointer text-left border-t border-slate-100"
              >
                <LogOut class="w-4 h-4 text-rose-600" />
                <span>Keluar / Logout</span>
              </button>
            </div>
          </div>

          <!-- Quick Logout Icon Button -->
          <button
            type="button"
            @click="handleLogout"
            class="w-7 h-7 rounded-lg flex items-center justify-center text-slate-400 hover:text-rose-700 hover:bg-rose-50 transition active:scale-95 cursor-pointer"
            title="Keluar / Logout Kasir"
          >
            <LogOut class="w-3.5 h-3.5" />
          </button>
        </div>
      </div>
    </header>

    <!-- Modal Mandiri Ganti Password & PIN Kasir -->
    <UserProfileSecurityModal
      :show="showSecurityModal"
      @close="showSecurityModal = false"
    />

    <!-- Slot Main Register Screen -->
    <main class="flex-1 flex flex-col overflow-hidden">
      <slot />
    </main>

    <!-- Bottom Status Bar / Shortcut Guide -->
    <footer class="bg-slate-200/90 border-t border-slate-300 px-4 py-1.5 flex items-center justify-between text-[11px] text-slate-700 font-medium select-none">
      <div class="flex items-center gap-3 overflow-x-auto">
        <span class="flex items-center gap-1 font-mono"><kbd class="bg-white border border-slate-300 text-slate-800 shadow-xs px-1.5 py-0.5 rounded text-[10px]">F1</kbd> Cari Barang</span>
        <span class="flex items-center gap-1 font-mono"><kbd class="bg-white border border-slate-300 text-slate-800 shadow-xs px-1.5 py-0.5 rounded text-[10px]">F4</kbd> Buka Laci</span>
        <span class="flex items-center gap-1 font-mono"><kbd class="bg-white border border-slate-300 text-slate-800 shadow-xs px-1.5 py-0.5 rounded text-[10px]">F6</kbd> Tahan Nota</span>
        <span class="flex items-center gap-1 font-mono"><kbd class="bg-white border border-slate-300 text-slate-800 shadow-xs px-1.5 py-0.5 rounded text-[10px]">F7</kbd> Panggil Nota</span>
        <span class="flex items-center gap-1 font-mono"><kbd class="bg-white border border-slate-300 text-slate-800 shadow-xs px-1.5 py-0.5 rounded text-[10px]">F12</kbd> Bayar / Selesai</span>
        <span class="flex items-center gap-1 font-mono"><kbd class="bg-white border border-slate-300 text-slate-800 shadow-xs px-1.5 py-0.5 rounded text-[10px]">Esc</kbd> Batal / Tutup</span>
      </div>
      <div class="font-mono text-[10px] text-slate-500 font-semibold hidden sm:block">
        Notane Retail POS v1.0.0
      </div>
    </footer>

    <!-- Shift Modal -->
    <ShiftModal
      :show="shiftStore.showShiftModal"
      :mode="shiftStore.shiftModalMode"
      @close="shiftStore.closeShiftModal()"
      @success="shiftStore.checkCurrentShift()"
    />

    <!-- Network Connection Modal -->
    <NetworkConnectionModal v-model="showNetworkModal" />

    <!-- Thermal Printer Settings Modal -->
    <PrinterSettingsModal
      :show="showPrinterModal"
      @close="showPrinterModal = false"
    />

    <!-- Modal Retur Penjualan Konsumen (P1) -->
    <SalesReturnModal
      v-model="showSalesReturnModal"
      @return-completed="dialog.toast('Retur Penjualan berhasil disetujui & dicatat!', 'success')"
    />

    <!-- Modal Setor Kas Laci Meja Kasir (P1) -->
    <CashDropModal
      v-if="shiftStore.currentShift"
      v-model="showCashDropModal"
      :shift-id="shiftStore.currentShift.id"
      @drop-completed="dialog.toast('Setor kas laci (cash drop) berhasil disimpan!', 'success')"
    />
  </div>
</template>
