<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import AdminLayout from '@/layouts/AdminLayout.vue'
import { useNetworkStore } from '@/stores/networkStore'
import { Icon } from '@iconify/vue'
import {
  Wifi,
  WifiOff,
  Activity,
  Server,
  Monitor,
  CheckCircle2,
  AlertCircle,
  RefreshCw,
  Zap,
} from 'lucide-vue-next'

const networkStore = useNetworkStore()

const hostInput = ref(networkStore.targetHost)
const portInput = ref(networkStore.targetPort)
const deviceIdInput = ref(networkStore.deviceId)
const deviceNameInput = ref(networkStore.deviceName)
const deviceTypeInput = ref(networkStore.deviceType)
const pairingCodeInput = ref(networkStore.pairingCode)
const autoReconnectInput = ref(networkStore.autoReconnect)

const isTestingPing = ref(false)
const pingResult = ref<{ success: boolean; latency: number; error?: string } | null>(null)
const isConnecting = ref(false)
const notification = ref<{ type: 'success' | 'error'; message: string } | null>(null)

const isConnected = computed(() => networkStore.connectionStatus === 'CONNECTED')

onMounted(() => {
  hostInput.value = networkStore.targetHost
  portInput.value = networkStore.targetPort
  deviceIdInput.value = networkStore.deviceId
  deviceNameInput.value = networkStore.deviceName
  deviceTypeInput.value = networkStore.deviceType
  pairingCodeInput.value = networkStore.pairingCode
  autoReconnectInput.value = networkStore.autoReconnect
})

const handleTestPing = async () => {
  isTestingPing.value = true
  pingResult.value = null
  try {
    const res = await networkStore.pingServer(hostInput.value, portInput.value)
    pingResult.value = res
    if (res.success) {
      showNotice('success', `Server merespons dalam waktu ${res.latency} ms`)
    } else {
      showNotice('error', res.error || 'Server tidak merespons ping')
    }
  } finally {
    isTestingPing.value = false
  }
}

const handleConnect = async () => {
  isConnecting.value = true
  try {
    networkStore.saveConnectionSettings(
      hostInput.value,
      portInput.value,
      deviceIdInput.value,
      deviceNameInput.value,
      deviceTypeInput.value,
      pairingCodeInput.value
    )
    networkStore.autoReconnect = autoReconnectInput.value
    localStorage.setItem('notane_auto_reconnect', autoReconnectInput.value.toString())

    const ok = await networkStore.connect()
    if (ok) {
      showNotice('success', `Berhasil terhubung ke Server Host (${networkStore.serverUrl})!`)
    } else {
      showNotice('error', networkStore.disconnectReason || 'Gagal menyambungkan ke host server')
    }
  } finally {
    isConnecting.value = false
  }
}

const handleDisconnect = () => {
  networkStore.disconnect('Diputuskan secara manual oleh pengguna')
  showNotice('error', 'Koneksi ke Server Host diputuskan.')
}

function showNotice(type: 'success' | 'error', message: string) {
  notification.value = { type, message }
  setTimeout(() => {
    if (notification.value?.message === message) {
      notification.value = null
    }
  }, 4500)
}
</script>

<template>
  <AdminLayout>
    <div class="h-full flex flex-col bg-slate-100 overflow-hidden font-sans text-xs">
      <!-- 1. TOP HEADER -->
      <div class="bg-white border-b border-slate-300 px-6 py-3.5 flex items-center justify-between shadow-xs shrink-0">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-xl bg-blue-100 border border-blue-200 flex items-center justify-center text-blue-700 shadow-xs">
            <Icon icon="fluent-color:wifi-24" class="w-6 h-6" />
          </div>
          <div>
            <div class="flex items-center gap-2">
              <h1 class="text-base font-extrabold text-slate-900 tracking-tight">Koneksi Terminal Client ke Server Host LAN</h1>
              <span class="text-[10px] bg-blue-50 text-blue-700 border border-blue-200 font-bold px-2 py-0.5 rounded-full flex items-center gap-1 font-mono">
                <Monitor class="w-3 h-3" />
                Mode Client Station
              </span>
            </div>
            <p class="text-xs text-slate-500 mt-0.5">
              Terminal kasir ini mengambil data katalog, pelanggan, dan transaksi secara real-time dari Server Host melalui jaringan lokal (LAN/Wi-Fi).
            </p>
          </div>
        </div>

        <!-- Quick Status Pill -->
        <div class="flex items-center gap-2">
          <div
            class="flex items-center gap-2 px-3 py-1.5 rounded-lg border font-bold text-xs"
            :class="isConnected
              ? 'bg-emerald-50 text-emerald-800 border-emerald-300'
              : 'bg-rose-50 text-rose-800 border-rose-300'"
          >
            <span
              class="w-2.5 h-2.5 rounded-full"
              :class="isConnected ? 'bg-emerald-500 animate-ping' : 'bg-rose-500'"
            ></span>
            <span>{{ isConnected ? `Terhubung ke ${networkStore.serverUrl}` : 'Terputus dari Server' }}</span>
          </div>
        </div>
      </div>

      <!-- Toast Notification -->
      <div
        v-if="notification"
        class="mx-6 mt-3 px-4 py-2.5 rounded-lg border text-xs font-medium flex items-center justify-between shadow-sm z-20"
        :class="notification.type === 'success' ? 'bg-emerald-50 text-emerald-800 border-emerald-200' : 'bg-rose-50 text-rose-800 border-rose-200'"
      >
        <div class="flex items-center gap-2">
          <CheckCircle2 v-if="notification.type === 'success'" class="w-4 h-4 text-emerald-600" />
          <AlertCircle v-else class="w-4 h-4 text-rose-600" />
          <span>{{ notification.message }}</span>
        </div>
        <button @click="notification = null" class="text-slate-400 hover:text-slate-600 text-sm font-bold ml-4">✕</button>
      </div>

      <!-- 2. MAIN WORKSPACE -->
      <div class="flex-1 overflow-y-auto p-6 max-w-5xl mx-auto w-full space-y-6">
        <!-- Status Banner Card -->
        <div class="bg-white rounded-xl border border-slate-300 p-5 shadow-xs grid grid-cols-1 md:grid-cols-3 gap-4">
          <div class="flex items-center gap-3 p-3 bg-slate-50 rounded-lg border border-slate-200">
            <div
              class="w-10 h-10 rounded-lg flex items-center justify-center font-bold"
              :class="isConnected ? 'bg-emerald-100 text-emerald-700' : 'bg-rose-100 text-rose-700'"
            >
              <Wifi v-if="isConnected" class="w-5 h-5" />
              <WifiOff v-else class="w-5 h-5" />
            </div>
            <div>
              <div class="text-[11px] text-slate-500 font-semibold uppercase">Status Sambungan</div>
              <div class="text-sm font-bold" :class="isConnected ? 'text-emerald-700' : 'text-rose-700'">
                {{ isConnected ? 'TERHUBUNG (ONLINE)' : 'TERPUTUS (OFFLINE)' }}
              </div>
            </div>
          </div>

          <div class="flex items-center gap-3 p-3 bg-slate-50 rounded-lg border border-slate-200">
            <div class="w-10 h-10 rounded-lg bg-indigo-100 text-indigo-700 flex items-center justify-center font-bold">
              <Activity class="w-5 h-5" />
            </div>
            <div>
              <div class="text-[11px] text-slate-500 font-semibold uppercase">Latensi Jaringan (Ping)</div>
              <div class="text-sm font-bold text-slate-800 font-mono">
                {{ isConnected && networkStore.latencyMs !== null ? `${networkStore.latencyMs} ms` : '-' }}
              </div>
            </div>
          </div>

          <div class="flex items-center gap-3 p-3 bg-slate-50 rounded-lg border border-slate-200">
            <div class="w-10 h-10 rounded-lg bg-purple-100 text-purple-700 flex items-center justify-center font-bold">
              <Server class="w-5 h-5" />
            </div>
            <div>
              <div class="text-[11px] text-slate-500 font-semibold uppercase">Alamat Server Host</div>
              <div class="text-sm font-bold text-slate-800 font-mono truncate">
                {{ networkStore.serverUrl }}
              </div>
            </div>
          </div>
        </div>

        <!-- Connection Configuration Form -->
        <div class="bg-white rounded-xl border border-slate-300 shadow-xs overflow-hidden">
          <div class="bg-slate-50 border-b border-slate-200 px-5 py-3 flex items-center justify-between">
            <div class="font-bold text-slate-800 flex items-center gap-2">
              <Icon icon="streamline-ultimate-color:database-connect" class="w-4 h-4" />
              <span>Pengaturan Koneksi LAN ke Server Host</span>
            </div>
            <span class="text-[11px] text-slate-500 font-mono">Terminal: {{ networkStore.deviceId }}</span>
          </div>

          <div class="p-6 space-y-5">
            <!-- Server Address -->
            <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
              <div class="md:col-span-2">
                <label class="block text-xs font-bold text-slate-700 mb-1">
                  Alamat IP / Host Server Utama <span class="text-rose-500">*</span>
                </label>
                <input
                  v-model="hostInput"
                  type="text"
                  placeholder="Contoh: 192.168.1.100 atau 127.0.0.1"
                  class="w-full px-3.5 py-2 text-xs bg-slate-50 border border-slate-300 rounded-lg text-slate-800 font-mono focus:bg-white focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
                <p class="text-[11px] text-slate-400 mt-1">Masukkan IP komputer Server Host yang ada di jaringan toko yang sama.</p>
              </div>

              <div>
                <label class="block text-xs font-bold text-slate-700 mb-1">
                  Port Server <span class="text-rose-500">*</span>
                </label>
                <input
                  v-model="portInput"
                  type="number"
                  placeholder="8080"
                  class="w-full px-3.5 py-2 text-xs bg-slate-50 border border-slate-300 rounded-lg text-slate-800 font-mono focus:bg-white focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
              </div>
            </div>

            <!-- Terminal Identity -->
            <div class="grid grid-cols-1 md:grid-cols-3 gap-4 border-t border-slate-200 pt-4">
              <div>
                <label class="block text-xs font-bold text-slate-700 mb-1">
                  ID Terminal Ini <span class="text-rose-500">*</span>
                </label>
                <input
                  v-model="deviceIdInput"
                  type="text"
                  placeholder="KASIR-01"
                  class="w-full px-3.5 py-2 text-xs bg-slate-50 border border-slate-300 rounded-lg text-slate-800 font-mono uppercase focus:bg-white focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
              </div>

              <div>
                <label class="block text-xs font-bold text-slate-700 mb-1">
                  Nama Perangkat
                </label>
                <input
                  v-model="deviceNameInput"
                  type="text"
                  placeholder="Terminal Kasir Meja 1"
                  class="w-full px-3.5 py-2 text-xs bg-slate-50 border border-slate-300 rounded-lg text-slate-800 focus:bg-white focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
              </div>

              <div>
                <label class="block text-xs font-bold text-slate-700 mb-1">
                  Kode Pencocokan (Pairing Code) <span class="text-rose-500">*</span>
                </label>
                <input
                  v-model="pairingCodeInput"
                  type="text"
                  placeholder="NOTANE-8899"
                  class="w-full px-3.5 py-2 text-xs bg-slate-50 border border-slate-300 rounded-lg text-slate-800 font-mono uppercase focus:bg-white focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
                <p class="text-[11px] text-slate-400 mt-1">Dapatkan kode ini dari layar Server Host.</p>
              </div>
            </div>

            <!-- Auto-Reconnect Switch -->
            <div class="border-t border-slate-200 pt-4 flex items-center justify-between">
              <div>
                <div class="text-xs font-bold text-slate-800">Sambung Otomatis (Auto-Reconnect)</div>
                <div class="text-[11px] text-slate-400">Secara otomatis menyambung kembali jika jaringan Wi-Fi/LAN terputus.</div>
              </div>
              <input
                type="checkbox"
                v-model="autoReconnectInput"
                class="w-4 h-4 text-blue-600 rounded focus:ring-blue-500 cursor-pointer"
              />
            </div>
          </div>

          <!-- Action Footer -->
          <div class="bg-slate-50 border-t border-slate-200 px-6 py-3.5 flex items-center justify-between">
            <button
              type="button"
              @click="handleTestPing"
              :disabled="isTestingPing"
              class="px-4 py-2 bg-slate-100 hover:bg-slate-200 text-slate-700 font-bold rounded-lg border border-slate-300 transition flex items-center gap-1.5 cursor-pointer active:scale-95"
            >
              <RefreshCw class="w-3.5 h-3.5" :class="{ 'animate-spin': isTestingPing }" />
              <span>{{ isTestingPing ? 'Menguji...' : 'Uji Sambungan (Ping)' }}</span>
            </button>

            <div class="flex items-center gap-2">
              <button
                v-if="isConnected"
                type="button"
                @click="handleDisconnect"
                class="px-4 py-2 bg-rose-50 hover:bg-rose-100 text-rose-700 font-bold rounded-lg border border-rose-300 transition cursor-pointer active:scale-95"
              >
                Putuskan Sambungan
              </button>

              <button
                type="button"
                @click="handleConnect"
                :disabled="isConnecting"
                class="px-5 py-2 bg-blue-600 hover:bg-blue-700 text-white font-bold rounded-lg border border-blue-700 transition flex items-center gap-2 cursor-pointer shadow-sm active:scale-95"
              >
                <Zap class="w-4 h-4" />
                <span>{{ isConnecting ? 'Menyambungkan...' : (isConnected ? 'Simpan & Sambung Ulang' : 'Sambungkan ke Server') }}</span>
              </button>
            </div>
          </div>
        </div>

        <!-- Role Switcher Guidance Note -->
        <div class="bg-amber-50 border border-amber-200 rounded-xl p-4 flex items-start gap-3">
          <div class="p-2 bg-amber-100 rounded-lg text-amber-800 shrink-0">
            <Server class="w-5 h-5" />
          </div>
          <div class="flex-1">
            <div class="font-bold text-amber-900 text-xs">Perangkat Ini Dikonfigurasi Sebagai: CLIENT STATION (TERMINAL KASIR)</div>
            <p class="text-[11px] text-amber-700 mt-0.5 leading-relaxed">
              Komputer ini hanya bertindak sebagai terminal kasir dan tidak menjalankan server database lokal.
              Jika komputer ini sebenarnya adalah komputer server toko yang menyimpan database utama, Anda dapat beralih ke <strong>Mode Server Host</strong> melalui tombol ganti peran di bilah atas aplikasi.
            </p>
            <div class="mt-2">
              <button
                type="button"
                @click="networkStore.setNodeRole('SERVER')"
                class="px-3 py-1 bg-amber-700 hover:bg-amber-800 text-white text-[11px] font-bold rounded-md transition cursor-pointer"
              >
                Beralih ke Mode Server Host
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </AdminLayout>
</template>
