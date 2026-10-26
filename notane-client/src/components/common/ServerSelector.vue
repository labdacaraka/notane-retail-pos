<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useNetworkStore } from '@/stores/networkStore'
import { discoveryService, type DiscoveredServer } from '@/services/discoveryService'
import {
  Server,
  Radio,
  Wifi,
  WifiOff,
  RefreshCw,
  CheckCircle2,
  AlertCircle,
  Zap,
  Lock,
  ArrowRight,
} from 'lucide-vue-next'

const emit = defineEmits<{
  (e: 'connected'): void
  (e: 'close'): void
}>()

const networkStore = useNetworkStore()

const activeTab = ref<'SCAN' | 'MANUAL'>('SCAN')
const isScanning = ref<boolean>(false)
const scanProgress = ref<{ current: number; total: number }>({ current: 0, total: 0 })
const discoveredServers = ref<DiscoveredServer[]>([])

const manualHost = ref<string>(networkStore.targetHost || '127.0.0.1')
const manualPort = ref<number | string | undefined>(networkStore.targetPort || '')
const pairingCodeInput = ref<string>(networkStore.pairingCode || 'NOTANE-8899')

const isConnecting = ref<boolean>(false)
const connectionError = ref<string>('')
const connectionSuccess = ref<string>('')

const isConnected = computed(() => networkStore.connectionStatus === 'CONNECTED')

onMounted(async () => {
  manualHost.value = networkStore.targetHost
  manualPort.value = networkStore.targetPort || ''
  pairingCodeInput.value = networkStore.pairingCode

  // Load cached discovered servers if available
  discoveredServers.value = discoveryService.getCachedDiscoveredServers()

  // Run initial scan
  await runScan()
})

async function runScan() {
  isScanning.value = true
  connectionError.value = ''
  scanProgress.value = { current: 0, total: 0 }
  try {
    const results = await discoveryService.scanLanServers((scanned, total) => {
      scanProgress.value = { current: scanned, total }
    })
    discoveredServers.value = results
    if (results.length === 0) {
      activeTab.value = 'MANUAL'
    }
  } finally {
    isScanning.value = false
  }
}

function selectDiscoveredServer(server: DiscoveredServer) {
  manualHost.value = server.host
  manualPort.value = server.port
  handleConnect()
}

async function handleConnect() {
  isConnecting.value = true
  connectionError.value = ''
  connectionSuccess.value = ''

  try {
    let host = (manualHost.value || '').trim()
    let port: number | undefined = undefined

    if (manualPort.value !== undefined && manualPort.value !== '' && !isNaN(Number(manualPort.value))) {
      const p = Number(manualPort.value)
      if (p > 0) port = p
    }

    // Jika user memasukkan port di dalam input host (misal: domain.com:8080 atau http://domain.com:18080)
    if (host.includes('://')) {
      try {
        const parsed = new URL(host)
        if (parsed.port && !port) {
          port = Number(parsed.port)
        }
      } catch (_) {}
    } else if (host.includes(':')) {
      const parts = host.split(':')
      if (parts.length === 2 && !isNaN(Number(parts[1]))) {
        host = parts[0]
        if (!port) {
          port = Number(parts[1])
        }
      }
    }

    // Jika berupa IP lokal numerik dan port tidak diisi, gunakan port default 18080
    const isIpv4 = /^(\d{1,3}\.){3}\d{1,3}$/.test(host)
    if (isIpv4 && !port) {
      port = 18080
    }

    const ok = await networkStore.connect({
      host,
      port,
      pairingCode: pairingCodeInput.value,
      deviceId: networkStore.deviceId,
      deviceName: networkStore.deviceName,
      deviceType: networkStore.deviceType,
    })

    if (ok) {
      const displayTarget = port ? `${host}:${port}` : host
      connectionSuccess.value = `Berhasil terhubung ke server ${displayTarget}`
      emit('connected')
    } else {
      connectionError.value = networkStore.disconnectReason || 'Gagal terhubung ke host server'
    }
  } catch (err: any) {
    connectionError.value = err.message || 'Koneksi gagal'
  } finally {
    isConnecting.value = false
  }
}
</script>

<template>
  <div class="bg-white border border-slate-300 rounded-2xl p-5 shadow-sm space-y-4">
    <!-- Header -->
    <div class="flex items-center justify-between border-b border-slate-200 pb-3">
      <div class="flex items-center gap-2.5">
        <div class="w-8 h-8 rounded-xl bg-blue-100 border border-blue-200 flex items-center justify-center text-blue-700 shadow-2xs">
          <Radio class="w-4 h-4" />
        </div>
        <div>
          <h3 class="text-sm font-extrabold text-slate-900 tracking-tight flex items-center gap-1.5">
            <span>Server Selector & LAN Discovery</span>
            <span class="text-[10px] bg-blue-50 text-blue-700 font-bold px-1.5 py-0.2 rounded border border-blue-200">
              mDNS / LAN
            </span>
          </h3>
          <p class="text-[11px] text-slate-500">Pilih PC Server Utama yang ingin dihubungi oleh kasir ini</p>
        </div>
      </div>

      <!-- Current Status Pill -->
      <div
        class="text-[11px] font-bold px-2.5 py-1 rounded-full border flex items-center gap-1.5"
        :class="isConnected ? 'bg-emerald-50 text-emerald-800 border-emerald-300' : 'bg-rose-50 text-rose-800 border-rose-300'"
      >
        <span class="w-2 h-2 rounded-full" :class="isConnected ? 'bg-emerald-500 animate-pulse' : 'bg-rose-500'"></span>
        <span>{{ isConnected ? 'Terhubung' : 'Terputus' }}</span>
      </div>
    </div>

    <!-- Mode Tab: Scan Otomatis vs Input Manual -->
    <div class="grid grid-cols-2 gap-2 bg-slate-100 p-1 rounded-xl text-xs font-bold select-none">
      <button
        type="button"
        @click="activeTab = 'SCAN'"
        class="py-1.5 rounded-lg transition flex items-center justify-center gap-1.5 cursor-pointer"
        :class="activeTab === 'SCAN' ? 'bg-white text-slate-900 shadow-2xs' : 'text-slate-600 hover:text-slate-900'"
      >
        <Radio class="w-3.5 h-3.5 text-blue-600" />
        <span>Pindai Jaringan (Auto Scan)</span>
      </button>

      <button
        type="button"
        @click="activeTab = 'MANUAL'"
        class="py-1.5 rounded-lg transition flex items-center justify-center gap-1.5 cursor-pointer"
        :class="activeTab === 'MANUAL' ? 'bg-white text-slate-900 shadow-2xs' : 'text-slate-600 hover:text-slate-900'"
      >
        <Server class="w-3.5 h-3.5 text-slate-600" />
        <span>Input Alamat Manual</span>
      </button>
    </div>

    <!-- Alert Messages -->
    <div v-if="connectionError" class="p-2.5 bg-rose-50 border border-rose-300 rounded-xl text-xs text-rose-800 flex items-start gap-2">
      <AlertCircle class="w-4 h-4 text-rose-600 shrink-0 mt-0.5" />
      <div class="flex-1">
        <span class="font-bold">Gagal Menyambungkan:</span> {{ connectionError }}
      </div>
    </div>

    <div v-if="connectionSuccess" class="p-2.5 bg-emerald-50 border border-emerald-300 rounded-xl text-xs text-emerald-800 flex items-center gap-2">
      <CheckCircle2 class="w-4 h-4 text-emerald-600 shrink-0" />
      <span>{{ connectionSuccess }}</span>
    </div>

    <!-- Tab Content: Auto Scan -->
    <div v-if="activeTab === 'SCAN'" class="space-y-3">
      <div class="flex items-center justify-between text-xs">
        <span class="font-bold text-slate-700">Daftar Server Terdeteksi di LAN:</span>
        <button
          type="button"
          @click="runScan"
          :disabled="isScanning"
          class="text-blue-600 hover:text-blue-800 font-bold flex items-center gap-1 cursor-pointer disabled:opacity-50"
        >
          <RefreshCw class="w-3 h-3" :class="{ 'animate-spin': isScanning }" />
          <span>{{ isScanning ? 'Memindai LAN...' : 'Pindai Ulang' }}</span>
        </button>
      </div>

      <!-- Loading scan bar -->
      <div v-if="isScanning" class="p-3 bg-blue-50/80 border border-blue-200 rounded-xl text-xs text-blue-900 space-y-1.5 animate-pulse">
        <div class="flex items-center justify-between">
          <span class="font-semibold">Mencari Server Notane POS di subnet lokal...</span>
          <span class="font-mono text-[10px]">{{ scanProgress.current }} / {{ scanProgress.total }}</span>
        </div>
        <div class="w-full bg-blue-200 h-1.5 rounded-full overflow-hidden">
          <div
            class="bg-blue-600 h-full transition-all duration-200"
            :style="{ width: scanProgress.total ? `${(scanProgress.current / scanProgress.total) * 100}%` : '40%' }"
          ></div>
        </div>
      </div>

      <!-- Discovered List -->
      <div v-if="discoveredServers.length > 0" class="space-y-2">
        <div
          v-for="server in discoveredServers"
          :key="server.id"
          class="p-3 rounded-xl border transition flex items-center justify-between"
          :class="server.host === networkStore.targetHost && isConnected
            ? 'bg-emerald-50/80 border-emerald-300 ring-1 ring-emerald-400'
            : 'bg-slate-50 border-slate-200 hover:border-blue-400 hover:bg-blue-50/30'"
        >
          <div class="flex items-center gap-3">
            <div
              class="w-8 h-8 rounded-lg flex items-center justify-center font-bold text-xs"
              :class="server.isLocal ? 'bg-slate-200 text-slate-800' : 'bg-blue-100 text-blue-800'"
            >
              <Server class="w-4 h-4" />
            </div>
            <div>
              <div class="font-extrabold text-slate-900 text-xs flex items-center gap-1.5">
                <span>{{ server.name }}</span>
                <span v-if="server.isLocal" class="text-[9px] bg-slate-200 text-slate-700 px-1.5 rounded">PC Ini</span>
                <span class="text-[9px] bg-emerald-100 text-emerald-800 font-mono px-1 rounded flex items-center gap-0.5">
                  <Zap class="w-2.5 h-2.5" />
                  {{ server.latency }}ms
                </span>
              </div>
              <div class="text-[11px] text-slate-500 font-mono">http://{{ server.host }}:{{ server.port }}</div>
            </div>
          </div>

          <div>
            <button
              type="button"
              @click="selectDiscoveredServer(server)"
              :disabled="isConnecting"
              class="px-3 py-1.5 rounded-lg text-xs font-bold transition flex items-center gap-1 cursor-pointer disabled:opacity-50"
              :class="server.host === networkStore.targetHost && isConnected
                ? 'bg-emerald-600 text-white'
                : 'bg-blue-600 hover:bg-blue-700 text-white'"
            >
              <span>{{ server.host === networkStore.targetHost && isConnected ? 'Terhubung' : 'Pilih & Sambung' }}</span>
              <ArrowRight class="w-3 h-3" />
            </button>
          </div>
        </div>
      </div>

      <!-- Empty state for Scan -->
      <div v-else-if="!isScanning" class="p-4 bg-slate-50 border border-slate-200 rounded-xl text-center space-y-2">
        <WifiOff class="w-8 h-8 text-slate-400 mx-auto" />
        <div class="text-xs font-bold text-slate-700">Tidak ada server Notane POS terdeteksi otomatis</div>
        <p class="text-[11px] text-slate-500 max-w-sm mx-auto">
          Pastikan PC Server Utama sudah menyala dan berada pada jaringan Wi-Fi/LAN yang sama. Atau masukkan alamat IP manual.
        </p>
        <button
          type="button"
          @click="activeTab = 'MANUAL'"
          class="inline-flex items-center gap-1 px-3 py-1.5 bg-slate-200 hover:bg-slate-300 text-slate-800 text-xs font-bold rounded-lg transition cursor-pointer"
        >
          <span>Beralih ke Input Manual</span>
          <ArrowRight class="w-3 h-3" />
        </button>
      </div>
    </div>

    <!-- Tab Content: Manual Input -->
    <div v-if="activeTab === 'MANUAL'" class="space-y-3">
      <div class="grid grid-cols-3 gap-2">
        <div class="col-span-2">
          <label class="block text-[11px] font-bold text-slate-700 mb-1">Alamat IP / Domain Server Host</label>
          <input
            v-model="manualHost"
            type="text"
            placeholder="192.168.1.15 atau pos.domain.com"
            class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-xs font-mono text-slate-900 focus:bg-white focus:border-blue-600 focus:outline-none"
          />
        </div>
        <div>
          <label class="block text-[11px] font-bold text-slate-700 mb-1">
            Port <span class="font-normal text-slate-400">(Opsional)</span>
          </label>
          <input
            v-model="manualPort"
            type="number"
            placeholder="Opsional (18080)"
            class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-xs font-mono text-slate-900 focus:bg-white focus:border-blue-600 focus:outline-none"
          />
        </div>
      </div>
      <p class="text-[10px] text-slate-500 leading-tight">
        *Jika server berada di Cloud / VPS menggunakan domain (misal: <code>pos.domain.com</code>), kolom <strong>Port</strong> tidak perlu diisi.
      </p>

      <!-- Pairing Code -->
      <div>
        <label class="block text-[11px] font-bold text-slate-700 mb-1 flex items-center justify-between">
          <span class="flex items-center gap-1">
            <Lock class="w-3 h-3 text-amber-600" />
            <span>Kode Pairing (Lihat di PC Server Utama)</span>
          </span>
          <span class="text-[10px] text-slate-500 font-normal">Default: NOTANE-8899</span>
        </label>
        <input
          v-model="pairingCodeInput"
          type="text"
          placeholder="NOTANE-8899"
          class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-xs font-mono font-bold tracking-wider uppercase text-slate-900 focus:bg-white focus:border-blue-600 focus:outline-none"
        />
      </div>

      <button
        type="button"
        @click="handleConnect"
        :disabled="isConnecting"
        class="w-full bg-blue-600 hover:bg-blue-700 text-white font-bold py-2.5 rounded-lg text-xs transition flex items-center justify-center gap-1.5 shadow-2xs cursor-pointer disabled:opacity-50"
      >
        <RefreshCw v-if="isConnecting" class="w-3.5 h-3.5 animate-spin" />
        <Wifi v-else class="w-3.5 h-3.5" />
        <span>{{ isConnecting ? 'Menyambungkan ke Server...' : 'Sambungkan ke Server Ini' }}</span>
      </button>
    </div>
  </div>
</template>
