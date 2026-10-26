<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useNetworkStore } from '@/stores/networkStore'

defineProps<{
  modelValue: boolean
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', val: boolean): void
}>()

const networkStore = useNetworkStore()

import { discoveryService } from '@/services/discoveryService'

const hostInput = ref(networkStore.targetHost)
const portInput = ref<number | string | undefined>(networkStore.targetPort || '')
const deviceIdInput = ref(networkStore.deviceId)
const deviceNameInput = ref(networkStore.deviceName)
const deviceTypeInput = ref(networkStore.deviceType)
const pairingCodeInput = ref(networkStore.pairingCode)
const autoReconnectInput = ref(networkStore.autoReconnect)

const isTestingPing = ref(false)
const isScanningLan = ref(false)
const pingResult = ref<{ success: boolean; latency: number; error?: string } | null>(null)
const isConnecting = ref(false)
const connectionError = ref<string>('')

const isConnected = computed(() => networkStore.connectionStatus === 'CONNECTED')

onMounted(() => {
  hostInput.value = networkStore.targetHost
  portInput.value = networkStore.targetPort || ''
  deviceIdInput.value = networkStore.deviceId
  deviceNameInput.value = networkStore.deviceName
  deviceTypeInput.value = networkStore.deviceType
  pairingCodeInput.value = networkStore.pairingCode
  autoReconnectInput.value = networkStore.autoReconnect
})

const closeModal = () => {
  emit('update:modelValue', false)
}

const handleScanLan = async () => {
  isScanningLan.value = true
  connectionError.value = ''
  try {
    const found = await discoveryService.scanLanServers()
    if (found.length > 0) {
      hostInput.value = found[0].host
      portInput.value = found[0].port
      pingResult.value = { success: true, latency: found[0].latency }
    } else {
      connectionError.value = 'Tidak ada server Notane POS terdeteksi di subnet lokal'
    }
  } catch (err: any) {
    connectionError.value = err.message || 'Gagal memindai jaringan'
  } finally {
    isScanningLan.value = false
  }
}

const handleTestPing = async () => {
  isTestingPing.value = true
  pingResult.value = null
  connectionError.value = ''
  try {
    const port = portInput.value !== undefined && portInput.value !== '' && !isNaN(Number(portInput.value)) ? Number(portInput.value) : undefined
    const res = await networkStore.pingServer(hostInput.value, port)
    pingResult.value = res
  } finally {
    isTestingPing.value = false
  }
}

const handleConnect = async () => {
  isConnecting.value = true
  connectionError.value = ''
  try {
    const port = portInput.value !== undefined && portInput.value !== '' && !isNaN(Number(portInput.value)) ? Number(portInput.value) : undefined
    networkStore.saveConnectionSettings(
      hostInput.value,
      port,
      deviceIdInput.value,
      deviceNameInput.value,
      deviceTypeInput.value,
      pairingCodeInput.value
    )
    networkStore.autoReconnect = autoReconnectInput.value
    localStorage.setItem('notane_auto_reconnect', autoReconnectInput.value.toString())

    const ok = await networkStore.connect()
    if (ok) {
      setTimeout(() => {
        closeModal()
      }, 500)
    } else {
      connectionError.value = networkStore.disconnectReason || 'Gagal menyambungkan ke host server'
    }
  } finally {
    isConnecting.value = false
  }
}

const handleDisconnect = () => {
  networkStore.disconnect('Diputuskan secara manual oleh pengguna')
  connectionError.value = ''
}
</script>

<template>
  <div v-if="modelValue" class="fixed inset-0 z-50 flex items-center justify-center bg-slate-900/50 backdrop-blur-xs p-4">
    <div class="bg-white border-2 border-slate-400 shadow-2xl rounded-sm w-full max-w-lg overflow-hidden flex flex-col font-sans">
      <!-- Title Bar Retro -->
      <div class="bg-slate-800 text-white px-4 py-2.5 flex items-center justify-between border-b border-slate-700 select-none">
        <div class="flex items-center gap-2">
          <div class="w-3 h-3 rounded-full" :class="isConnected ? 'bg-emerald-400 animate-pulse' : 'bg-rose-500'"></div>
          <span class="font-bold text-xs tracking-wider uppercase">Konfigurasi Jaringan & Server Host</span>
        </div>
        <button
          type="button"
          @click="closeModal"
          class="text-slate-300 hover:text-white hover:bg-slate-700 px-2 py-0.5 rounded text-sm font-bold cursor-pointer"
        >
          ✕
        </button>
      </div>

      <!-- Body Content -->
      <div class="p-4 space-y-4 text-xs text-slate-700 bg-slate-50 overflow-y-auto max-h-[80vh]">
        <!-- Status Banner -->
        <div
          class="p-3 border rounded-sm flex items-center justify-between"
          :class="isConnected ? 'bg-emerald-50 border-emerald-300 text-emerald-900' : 'bg-rose-50 border-rose-300 text-rose-900'"
        >
          <div class="flex items-center gap-3">
            <div
              class="w-8 h-8 rounded-sm flex items-center justify-center font-bold text-sm shadow-xs"
              :class="isConnected ? 'bg-emerald-500 text-white' : 'bg-rose-500 text-white'"
            >
              {{ isConnected ? 'LAN' : 'OFF' }}
            </div>
            <div>
              <div class="font-bold text-xs uppercase tracking-wide">
                {{ isConnected ? 'Terhubung ke Host Server' : 'Terputus dari Host Server' }}
              </div>
              <div class="text-[11px] opacity-80 font-mono">
                {{ isConnected ? `Target: ${networkStore.serverUrl}` : (networkStore.disconnectReason || 'Belum ada koneksi aktif') }}
              </div>
            </div>
          </div>

          <div class="text-right">
            <div v-if="isConnected && networkStore.latencyMs !== null" class="font-mono font-bold text-xs text-emerald-700">
              ⚡ {{ networkStore.latencyMs }} ms
            </div>
            <div v-if="networkStore.lastPingTime" class="text-[10px] text-slate-500">
              Uji: {{ networkStore.lastPingTime }}
            </div>
          </div>
        </div>

        <!-- Connection Error Alert -->
        <div v-if="connectionError" class="p-3 rounded-sm border bg-rose-100 border-rose-300 text-rose-900 flex items-start justify-between gap-2">
          <div class="flex items-center gap-2">
            <span class="text-base">⛔</span>
            <div>
              <div class="font-bold text-[11px] uppercase">Gagal Menyambungkan:</div>
              <div class="text-xs font-semibold">{{ connectionError }}</div>
            </div>
          </div>
          <button type="button" @click="connectionError = ''" class="text-xs font-bold hover:underline">✕</button>
        </div>

        <!-- Ping Result Notice -->
        <div v-if="pingResult" class="p-2.5 rounded-sm border text-[11px] flex items-center justify-between" :class="pingResult.success ? 'bg-emerald-100/70 border-emerald-300 text-emerald-800' : 'bg-amber-100/70 border-amber-300 text-amber-900'">
          <span>{{ pingResult.success ? `✓ Host merespons normal (Latensi: ${pingResult.latency} ms)` : `✗ Gagal: ${pingResult.error}` }}</span>
          <button type="button" @click="pingResult = null" class="text-xs font-bold hover:underline">Tutup</button>
        </div>

        <!-- Form Parameter -->
        <div class="bg-white p-3 border border-slate-300 rounded-sm shadow-xs space-y-3">
          <div class="font-bold text-slate-800 border-b border-slate-200 pb-1.5 uppercase text-[11px] flex items-center justify-between">
            <span>Parameter Target Server (LAN / VPS)</span>
            <span class="text-slate-500 font-normal">Mendukung IP Lokal atau Domain</span>
          </div>

          <div class="grid grid-cols-3 gap-2">
            <div class="col-span-2">
              <label class="block font-semibold mb-1 text-slate-600">Alamat Host / Domain VPS</label>
              <input
                v-model="hostInput"
                type="text"
                placeholder="192.168.1.100 atau pos.tokoku.com"
                class="w-full px-2.5 py-1.5 border border-slate-300 rounded-sm font-mono text-xs focus:ring-1 focus:ring-emerald-500 focus:border-emerald-500 bg-white"
              />
            </div>
            <div>
              <label class="block font-semibold mb-1 text-slate-600">
                Port Server <span class="font-normal text-slate-400">(Opsional)</span>
              </label>
              <input
                v-model="portInput"
                type="number"
                placeholder="Opsional (18080)"
                class="w-full px-2.5 py-1.5 border border-slate-300 rounded-sm font-mono text-xs focus:ring-1 focus:ring-emerald-500 focus:border-emerald-500 bg-white"
              />
            </div>
          </div>
          <p class="text-[10px] text-slate-500">
            *Jika server berada di Cloud / VPS menggunakan domain, kolom <strong>Port</strong> dapat dikosongkan.
          </p>

          <div class="flex items-center gap-2 pt-1">
            <button
              type="button"
              @click="handleScanLan"
              :disabled="isScanningLan"
              class="px-3 py-1.5 bg-blue-50 hover:bg-blue-100 text-blue-800 font-bold rounded-sm border border-blue-300 transition-colors flex items-center gap-1.5 cursor-pointer disabled:opacity-50"
            >
              <span v-if="isScanningLan" class="animate-spin">⌛</span>
              <span v-else>🔍</span>
              <span>{{ isScanningLan ? 'Memindai...' : 'Pindai LAN (mDNS)' }}</span>
            </button>

            <button
              type="button"
              @click="handleTestPing"
              :disabled="isTestingPing"
              class="px-3 py-1.5 bg-slate-100 hover:bg-slate-200 text-slate-700 font-semibold rounded-sm border border-slate-300 transition-colors flex items-center gap-1.5 cursor-pointer disabled:opacity-50"
            >
              <span v-if="isTestingPing" class="animate-spin">⌛</span>
              <span v-else>📡</span>
              <span>Uji Latensi (Ping Test)</span>
            </button>
            <span class="text-[10px] text-slate-500">Otomatis deteksi IP server di LAN</span>
          </div>
        </div>

        <!-- Kode Pencocokan (Pairing Code) Security -->
        <div class="bg-amber-50/70 p-3 border border-amber-300 rounded-sm shadow-xs space-y-2">
          <div class="font-bold text-amber-900 border-b border-amber-200 pb-1 uppercase text-[11px] flex items-center justify-between">
            <span class="flex items-center gap-1.5">
              <span>🔐</span>
              <span>Kode Pencocokan Perangkat (Pairing Code)</span>
            </span>
            <span class="text-[10px] text-amber-700 font-normal">Wajib Diisi</span>
          </div>
          <p class="text-[10px] text-amber-800">
            Hanya terminal yang memasukkan Kode Pencocokan yang valid dari Host Server yang diizinkan terhubung. Hal ini melindungi server (termasuk VPS) dari akses perangkat asing.
          </p>
          <div>
            <input
              v-model="pairingCodeInput"
              type="text"
              placeholder="Contoh: NOTANE-8899"
              class="w-full px-3 py-1.5 border border-amber-400 rounded-sm font-mono font-bold text-xs tracking-wider uppercase focus:ring-1 focus:ring-amber-500 focus:border-amber-500 bg-white"
            />
          </div>
        </div>

        <!-- Identitas Terminal Kasir -->
        <div class="bg-white p-3 border border-slate-300 rounded-sm shadow-xs space-y-3">
          <div class="font-bold text-slate-800 border-b border-slate-200 pb-1.5 uppercase text-[11px]">
            Identitas Perangkat Ini
          </div>

          <div class="grid grid-cols-2 gap-2">
            <div>
              <label class="block font-semibold mb-1 text-slate-600">Terminal ID</label>
              <input
                v-model="deviceIdInput"
                type="text"
                placeholder="KASIR-01"
                class="w-full px-2.5 py-1.5 border border-slate-300 rounded-sm font-mono text-xs focus:ring-1 focus:ring-emerald-500 focus:border-emerald-500 uppercase bg-white"
              />
            </div>
            <div>
              <label class="block font-semibold mb-1 text-slate-600">Tipe Perangkat</label>
              <select
                v-model="deviceTypeInput"
                class="w-full px-2 py-1.5 border border-slate-300 rounded-sm text-xs focus:ring-1 focus:ring-emerald-500 focus:border-emerald-500 bg-white"
              >
                <option value="CASHIER">Kasir POS (Register)</option>
                <option value="CUSTOMER_DISPLAY">Layar Pelanggan (Display)</option>
                <option value="TABLET">Tablet Kasir / Waiter</option>
                <option value="BACKOFFICE">Back-Office / Admin</option>
              </select>
            </div>
          </div>

          <div>
            <label class="block font-semibold mb-1 text-slate-600">Nama Perangkat</label>
            <input
              v-model="deviceNameInput"
              type="text"
              placeholder="Kasir Depan Meja 1"
              class="w-full px-2.5 py-1.5 border border-slate-300 rounded-sm text-xs focus:ring-1 focus:ring-emerald-500 focus:border-emerald-500 bg-white"
            />
          </div>

          <div class="flex items-center gap-2 pt-1">
            <input
              id="auto-reconnect-toggle"
              v-model="autoReconnectInput"
              type="checkbox"
              class="w-4 h-4 text-emerald-600 rounded-xs border-slate-300 focus:ring-emerald-500 cursor-pointer"
            />
            <label for="auto-reconnect-toggle" class="cursor-pointer text-slate-700 font-medium">
              Otomatis sambung kembali jika koneksi terputus tidak sengaja (Auto-Reconnect)
            </label>
          </div>
        </div>
      </div>

      <!-- Action Footer -->
      <div class="bg-slate-100 px-4 py-3 border-t border-slate-300 flex items-center justify-between">
        <button
          type="button"
          @click="closeModal"
          class="px-4 py-1.5 bg-white hover:bg-slate-200 text-slate-700 font-semibold border border-slate-300 rounded-sm transition-colors cursor-pointer text-xs"
        >
          Tutup
        </button>

        <div class="flex items-center gap-2">
          <button
            v-if="isConnected"
            type="button"
            @click="handleDisconnect"
            class="px-4 py-1.5 bg-rose-600 hover:bg-rose-700 text-white font-bold border border-rose-700 rounded-sm shadow-xs transition-colors cursor-pointer text-xs flex items-center gap-1.5"
          >
            <span>🔌</span>
            <span>Putuskan Koneksi (Disconnect)</span>
          </button>

          <button
            type="button"
            @click="handleConnect"
            :disabled="isConnecting"
            class="px-5 py-1.5 bg-emerald-600 hover:bg-emerald-700 text-white font-bold border border-emerald-700 rounded-sm shadow-xs transition-colors cursor-pointer text-xs flex items-center gap-1.5 disabled:opacity-50"
          >
            <span v-if="isConnecting" class="animate-spin">⌛</span>
            <span v-else>🔗</span>
            <span>{{ isConnected ? 'Simpan & Reconnect' : 'Sambungkan ke Server' }}</span>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
