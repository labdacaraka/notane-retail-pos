<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import { LogOut } from 'lucide-vue-next'
import { useAuthStore } from '@/stores/authStore'
import { useShiftStore } from '@/stores/shiftStore'
import { useNetworkStore } from '@/stores/networkStore'
import { usePrinterStore } from '@/stores/printerStore'
import ShiftModal from '@/components/pos/ShiftModal.vue'
import NetworkConnectionModal from '@/components/common/NetworkConnectionModal.vue'
import PrinterSettingsModal from '@/components/pos/PrinterSettingsModal.vue'

const router = useRouter()
const authStore = useAuthStore()
const shiftStore = useShiftStore()
const networkStore = useNetworkStore()
const printerStore = usePrinterStore()

const showShiftModal = ref<boolean>(false)
const showNetworkModal = ref<boolean>(false)
const showPrinterModal = ref<boolean>(false)
const shiftModalMode = ref<'OPEN' | 'CLOSE'>('OPEN')
const currentTime = ref<string>('')
let timer: any = null

function updateTime() {
  const now = new Date()
  currentTime.value = now.toLocaleTimeString('id-ID', { hour: '2-digit', minute: '2-digit', second: '2-digit' })
}

function handleOpenShiftModal() {
  shiftModalMode.value = 'OPEN'
  showShiftModal.value = true
}

function handleCloseShiftModal() {
  shiftModalMode.value = 'CLOSE'
  showShiftModal.value = true
}

function openCustomerDisplay() {
  window.open('/customer-display', '_blank', 'width=1024,height=600')
}

function handleLogout() {
  authStore.logout()
  router.push({ name: 'login' })
}

onMounted(() => {
  updateTime()
  timer = setInterval(updateTime, 1000)
  shiftStore.checkCurrentShift()
  // Hanya sambung otomatis jika intent sebelumnya adalah CONNECTED dan autoReconnect aktif
  if (networkStore.connectionIntent === 'CONNECTED' && networkStore.autoReconnect && networkStore.connectionStatus !== 'CONNECTED') {
    networkStore.connect()
  }
})

onUnmounted(() => {
  if (timer) clearInterval(timer)
})
</script>

<template>
  <div class="h-screen flex flex-col bg-slate-100 text-slate-900 overflow-hidden select-none">
    <!-- Top POS Header Bar -->
    <header class="bg-white border-b border-slate-300 px-4 py-2.5 flex items-center justify-between select-none shadow-xs">
      <!-- Left: Logo & Station Status -->
      <div class="flex items-center gap-3">
        <div class="flex items-center gap-2.5">
          <img src="/logo.png" alt="Notane Logo" class="w-8 h-8 rounded-lg shadow-sm border border-slate-200 object-contain hover:scale-105 transition" />
          <div>
            <h1 class="font-bold text-sm text-slate-900 tracking-wide leading-none">NOTANE RETAIL</h1>
            <span class="text-[10px] text-emerald-700 font-mono font-bold">POS ENTERPRISE</span>
          </div>
        </div>

        <div class="hidden sm:flex items-center gap-2 pl-3 border-l border-slate-300 text-xs">
          <span class="bg-slate-100 border border-slate-300 px-2 py-0.5 rounded text-slate-700 font-mono font-bold">
            {{ authStore.terminalId }}
          </span>

          <!-- Interactive Network Status Badge -->
          <button
            type="button"
            @click="showNetworkModal = true"
            class="flex items-center gap-1.5 text-[11px] font-bold px-2 py-0.5 rounded-sm border cursor-pointer transition-all hover:opacity-90 active:scale-95"
            :class="{
              'bg-emerald-50 text-emerald-800 border-emerald-300': networkStore.connectionStatus === 'CONNECTED',
              'bg-rose-50 text-rose-800 border-rose-300': networkStore.connectionStatus === 'DISCONNECTED' || networkStore.connectionStatus === 'ERROR',
              'bg-amber-50 text-amber-800 border-amber-300': networkStore.connectionStatus === 'CONNECTING'
            }"
            title="Klik untuk konfigurasi jaringan & server host"
          >
            <span
              class="w-2 h-2 rounded-full"
              :class="{
                'bg-emerald-500 animate-ping': networkStore.connectionStatus === 'CONNECTED',
                'bg-rose-500': networkStore.connectionStatus === 'DISCONNECTED' || networkStore.connectionStatus === 'ERROR',
                'bg-amber-500 animate-pulse': networkStore.connectionStatus === 'CONNECTING'
              }"
            ></span>
            <span v-if="networkStore.connectionStatus === 'CONNECTED'">
              LAN Host ({{ networkStore.latencyMs !== null ? `${networkStore.latencyMs}ms` : 'Aktif' }})
            </span>
            <span v-else-if="networkStore.connectionStatus === 'CONNECTING'">Menghubungkan...</span>
            <span v-else>Terputus dari Server</span>
          </button>

          <!-- Interactive Thermal Printer Badge -->
          <button
            type="button"
            @click="showPrinterModal = true"
            class="flex items-center gap-1.5 text-[11px] font-bold px-2 py-0.5 rounded-sm border cursor-pointer transition-all hover:opacity-90 active:scale-95"
            :class="{
              'bg-emerald-50 text-emerald-800 border-emerald-300': printerStore.isConnected,
              'bg-slate-100 text-slate-700 border-slate-300': !printerStore.isConnected
            }"
            title="Klik untuk konfigurasi printer thermal (Bluetooth / Serial)"
          >
            <Icon icon="streamline-ultimate-color:print-text" class="w-3.5 h-3.5" />
            <span v-if="printerStore.isConnected">
              {{ printerStore.connectionType === 'VIRTUAL' ? 'Sim: ' : '' }}{{ printerStore.printerName.length > 18 ? printerStore.printerName.substring(0, 18) + '...' : printerStore.printerName }} ({{ printerStore.paperWidth }})
            </span>
            <span v-else>Printer Belum Diatur</span>
          </button>
        </div>
      </div>

      <!-- Center: Shift Status & Real-Time Clock -->
      <div class="flex items-center gap-3">
        <div v-if="shiftStore.isShiftOpen" class="flex items-center gap-2">
          <span class="bg-emerald-50 text-emerald-800 border border-emerald-300 text-xs px-2.5 py-1 rounded-full font-bold flex items-center gap-1.5">
            <span class="w-2 h-2 rounded-full bg-emerald-500"></span>
            Shift Aktif
          </span>
          <button
            @click="handleCloseShiftModal"
            class="text-xs bg-rose-50 hover:bg-rose-100 text-rose-700 border border-rose-300 px-2.5 py-1 rounded-lg transition active:scale-95 font-bold shadow-xs"
          >
            Tutup Shift (Settlement)
          </button>
        </div>
        <div v-else>
          <button
            @click="handleOpenShiftModal"
            class="text-xs bg-amber-100 hover:bg-amber-200 text-amber-900 border border-amber-300 font-bold px-3 py-1 rounded-lg transition active:scale-95 shadow-xs flex items-center gap-1.5 cursor-pointer"
          >
            <Icon icon="streamline-ultimate-color:time-clock-circle" class="w-4 h-4" />
            <span>Buka Shift Kasir</span>
          </button>
        </div>

        <div class="hidden md:block font-mono text-sm font-bold text-slate-700 px-2 border-l border-slate-300">
          {{ currentTime }}
        </div>
      </div>

      <!-- Right: User Actions -->
      <div class="flex items-center gap-2">
        <button
          @click="openCustomerDisplay"
          class="flex items-center gap-1.5 text-xs bg-slate-100 hover:bg-slate-200 text-slate-700 px-2.5 py-1.5 rounded-lg border border-slate-300 transition font-medium cursor-pointer"
          title="Buka Layar Hadap Pelanggan (Customer Display)"
        >
          <Icon icon="streamline-ultimate-color:wifi-laptop" class="w-4 h-4" />
          <span class="hidden lg:inline">Customer Display</span>
        </button>

        <button
          @click="router.push('/admin/products')"
          class="flex items-center gap-1.5 text-xs bg-slate-100 hover:bg-slate-200 text-slate-700 px-2.5 py-1.5 rounded-lg border border-slate-300 transition font-medium cursor-pointer"
          title="Akses Back-Office Dashboard"
        >
          <Icon icon="streamline-ultimate-color:layout-dashboard" class="w-4 h-4" />
          <span class="hidden lg:inline">Back-Office</span>
        </button>

        <div class="flex items-center gap-2 pl-2 border-l border-slate-300">
          <div class="text-right hidden sm:block">
            <div class="text-xs font-bold text-slate-900">{{ authStore.user?.full_name }}</div>
            <div class="text-[10px] text-slate-500 uppercase font-mono font-medium">{{ authStore.user?.role_name }}</div>
          </div>
          <button
            @click="handleLogout"
            class="text-slate-500 hover:text-rose-700 p-1.5 rounded-lg hover:bg-slate-100 transition"
            title="Keluar / Logout"
          >
            <LogOut class="w-4 h-4" />
          </button>
        </div>
      </div>
    </header>

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
      :show="showShiftModal"
      :mode="shiftModalMode"
      @close="showShiftModal = false"
      @success="shiftStore.checkCurrentShift()"
    />

    <!-- Network Connection Modal -->
    <NetworkConnectionModal v-model="showNetworkModal" />

    <!-- Thermal Printer Settings Modal -->
    <PrinterSettingsModal
      :show="showPrinterModal"
      @close="showPrinterModal = false"
    />
  </div>
</template>
