<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue'
import { useRouter } from 'vue-router'
import {
  Lock,
  User,
  Terminal,
  ArrowRight,
  WifiOff,
  RefreshCw,
  Server,
  Monitor,
  Copy,
  Check,
  HardDrive,
  Database,
  Radio,
  Minimize2,
  Power,
  Users,
  Info,
} from 'lucide-vue-next'
import { useAuthStore } from '@/stores/authStore'
import { useNetworkStore } from '@/stores/networkStore'
import { offlineStorage } from '@/services/offlineStorage'
import ServerSelector from '@/components/common/ServerSelector.vue'

const router = useRouter()
const authStore = useAuthStore()
const networkStore = useNetworkStore()

// Selected Mode: 'SERVER' (Server Host Murni) vs 'CLIENT' (Kasir & Backoffice)
const currentMode = ref<'SERVER' | 'CLIENT'>(networkStore.nodeRole || 'SERVER')

// Form Login State (Hanya untuk Mode Client)
const username = ref<string>('admin')
const password = ref<string>('admin123')
const terminalId = ref<string>(authStore.terminalId || 'KASIR-01')
const isLoggingIn = ref<boolean>(false)
const errorMsg = ref<string>('')

// Tray / Minimized Mode State for Server Mode
const isTrayOnlyMode = ref<boolean>(false)

// Offline Storage Info
const cachedProductsCount = ref<number>(offlineStorage.getCachedProductsCount())
const pendingQueueCount = ref<number>(offlineStorage.getPendingQueueCount())

// Copy feedback
const copiedPairing = ref<boolean>(false)

// Server Control State
const isCheckingServer = ref<boolean>(false)
const isServerOnline = ref<boolean | null>(null)
const serverLatency = ref<number | null>(null)

const isConnected = computed(() => networkStore.connectionStatus === 'CONNECTED' || isServerOnline.value === true)

watch(currentMode, (newMode) => {
  networkStore.setNodeRole(newMode)
  if (newMode === 'SERVER') {
    checkLocalServer()
  } else {
    checkClientConnection()
  }
})

onMounted(async () => {
  cachedProductsCount.value = offlineStorage.getCachedProductsCount()
  pendingQueueCount.value = offlineStorage.getPendingQueueCount()

  if (currentMode.value === 'SERVER') {
    await checkLocalServer()
  } else {
    await checkClientConnection()
  }
})

async function checkLocalServer() {
  isCheckingServer.value = true
  try {
    await networkStore.fetchHostStatus()
    await networkStore.fetchConnectedDevices()
    const res = await networkStore.pingServer('127.0.0.1', 8080)
    isServerOnline.value = res.success
    serverLatency.value = res.latency
  } catch {
    isServerOnline.value = false
    serverLatency.value = null
  } finally {
    isCheckingServer.value = false
  }
}

async function checkClientConnection() {
  isCheckingServer.value = true
  try {
    const res = await networkStore.pingServer()
    isServerOnline.value = res.success
    serverLatency.value = res.latency
  } catch {
    isServerOnline.value = false
    serverLatency.value = null
  } finally {
    isCheckingServer.value = false
  }
}

function handleServerConnected() {
  checkClientConnection()
  errorMsg.value = ''
}

function copyText(text: string) {
  navigator.clipboard.writeText(text)
  copiedPairing.value = true
  setTimeout(() => { copiedPairing.value = false }, 2000)
}

async function handleStartStopServer() {
  if (networkStore.hostStatus.is_running) {
    if (confirm('Apakah Anda yakin ingin menghentikan Server Host? Semua koneksi kasir client akan terputus.')) {
      await networkStore.stopHostServer()
      await checkLocalServer()
    }
  } else {
    await networkStore.startHostServer()
    await checkLocalServer()
  }
}

async function handleRegeneratePairing() {
  if (confirm('Acak ulang kode pairing? Kasir yang belum memasukkan kode baru harus memperbarui kodenya.')) {
    await networkStore.regeneratePairingCode()
  }
}

async function handleKick(deviceId: string) {
  if (confirm(`Putuskan terminal kasir ${deviceId}?`)) {
    await networkStore.kickDevice(deviceId)
  }
}

async function submitLogin(targetRoute: string = '/') {
  if (currentMode.value === 'CLIENT' && !isConnected.value) {
    errorMsg.value = 'Terminal kasir belum terhubung ke Server Host. Silakan pilih dan sambungkan server terlebih dahulu.'
    return
  }

  isLoggingIn.value = true
  errorMsg.value = ''

  try {
    await authStore.login(username.value, password.value, terminalId.value)
    router.push(targetRoute)
  } catch (err: any) {
    if (err.message === 'Network Error' || !err.response) {
      isServerOnline.value = false
      errorMsg.value = 'Tidak dapat menjangkau server host. Pastikan IP server dan koneksi Wi-Fi/LAN terhubung.'
    } else {
      errorMsg.value = err.response?.data?.message || err.message || 'Username atau password salah'
    }
  } finally {
    isLoggingIn.value = false
  }
}

function handleLogin() {
  submitLogin('/')
}

function quickFill(user: string, pass: string) {
  username.value = user
  password.value = pass
}
</script>

<template>
  <div class="h-screen w-full bg-slate-100 overflow-y-auto font-sans relative select-none">
    <!-- Ambient desktop gradients -->
    <div class="fixed -top-40 -left-40 w-96 h-96 bg-lime-400/10 rounded-full blur-3xl pointer-events-none"></div>
    <div class="fixed -bottom-40 -right-40 w-96 h-96 bg-blue-400/10 rounded-full blur-3xl pointer-events-none"></div>

    <div class="min-h-full w-full flex flex-col items-center justify-center p-4 py-8">
      <div class="w-full max-w-2xl space-y-4 relative z-10">
        <!-- App Brand Header -->
        <div class="text-center space-y-1">
          <div class="inline-flex items-center justify-center p-2 bg-white border border-slate-200 rounded-2xl shadow-sm mb-1">
            <img src="/logo.png" alt="Notane Logo" class="w-12 h-12 object-contain" />
          </div>
          <h1 class="text-2xl font-black text-slate-900 tracking-tight">
            NOTANE <span class="text-notane-gradient">RETAIL POS</span>
          </h1>
          <p class="text-xs text-slate-600">
            {{ currentMode === 'SERVER' ? 'Panel Kontrol Server Host & Database Toko' : 'Terminal Kasir POS & Back-Office Toko' }}
          </p>
        </div>

        <!-- SERVER MODE BANNER (HANYA MUNCUL DI MODE SERVER) -->
        <div v-if="currentMode === 'SERVER'" class="bg-white border border-slate-300 rounded-2xl p-3.5 shadow-sm flex items-center justify-between">
          <div class="flex items-center gap-3">
            <div class="w-10 h-10 rounded-xl bg-emerald-600 text-white flex items-center justify-center font-bold shadow-xs">
              <Server class="w-5 h-5" />
            </div>
            <div>
              <div class="flex items-center gap-2">
                <span class="font-black text-xs tracking-wide uppercase text-emerald-950">
                  MODE SERVER (HOST ENGINE)
                </span>
                <span class="text-[10px] bg-emerald-200/70 text-emerald-800 px-1.5 py-0.2 rounded-full font-bold">
                  Aktif
                </span>
              </div>
              <div class="text-[11px] text-slate-500 font-medium">Khusus Host Engine, Database PostgreSQL & System Tray</div>
            </div>
          </div>

          <button
            type="button"
            @click="currentMode = 'CLIENT'"
            class="px-3.5 py-2 bg-blue-50 hover:bg-blue-100 text-blue-700 border border-blue-300 rounded-xl text-xs font-bold transition flex items-center gap-1.5 cursor-pointer shadow-2xs"
          >
            <Monitor class="w-3.5 h-3.5" />
            <span>Beralih ke Mode Client</span>
          </button>
        </div>

        <!-- ============================================================== -->
        <!-- VIEW A: MODE SERVER (KHUSUS ENGINE SERVER & SYSTEM TRAY) -->
        <!-- ============================================================== -->
        <div v-if="currentMode === 'SERVER'" class="space-y-4">
          <!-- Tray Only Minimized Mode Screen -->
          <div v-if="isTrayOnlyMode" class="bg-white border-2 border-emerald-500 rounded-2xl p-6 shadow-xl space-y-4 text-center">
            <div class="w-14 h-14 bg-emerald-100 rounded-2xl flex items-center justify-center text-emerald-600 mx-auto shadow-inner">
              <Minimize2 class="w-7 h-7" />
            </div>
            <div>
              <h2 class="text-base font-black text-slate-900">Host Server Berjalan di Latar Belakang (System Tray)</h2>
              <p class="text-xs text-slate-600 mt-1 max-w-md mx-auto">
                Mesin ini bertindak murni sebagai Host Server & Database toko. Layanan Axum REST API, WebSocket, dan mDNS tetap aktif melayani seluruh kasir client di jaringan.
              </p>
            </div>

            <div class="p-3 bg-slate-50 rounded-xl border border-slate-200 font-mono text-xs text-slate-700 max-w-sm mx-auto space-y-1 text-left">
              <div class="flex justify-between">
                <span>Axum Port:</span>
                <span class="font-bold text-emerald-700">0.0.0.0:8080 (Running)</span>
              </div>
              <div class="flex justify-between">
                <span>Database:</span>
                <span class="font-bold text-blue-700">PostgreSQL (Port 5432)</span>
              </div>
              <div class="flex justify-between">
                <span>Broadcaster:</span>
                <span class="font-bold text-purple-700">mDNS LAN Active</span>
              </div>
              <div class="flex justify-between">
                <span>Kode Pairing:</span>
                <span class="font-bold text-amber-700">{{ networkStore.hostStatus.pairing_code || 'NOTANE-8899' }}</span>
              </div>
            </div>

            <button
              type="button"
              @click="isTrayOnlyMode = false"
              class="px-5 py-2.5 bg-slate-800 hover:bg-slate-900 text-white text-xs font-bold rounded-xl transition cursor-pointer"
            >
              Buka Kembali Panel Kontrol Server
            </button>
          </div>

          <!-- Normal Server Control Dashboard -->
          <div v-else class="space-y-4">
            <!-- Server Status & Control Bar -->
            <div class="bg-white border border-slate-300 rounded-2xl p-5 shadow-sm space-y-4">
              <div class="flex items-center justify-between border-b border-slate-200 pb-3">
                <div class="flex items-center gap-2.5">
                  <span class="relative flex h-3 w-3">
                    <span
                      v-if="networkStore.hostStatus.is_running"
                      class="animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75"
                    ></span>
                    <span
                      class="relative inline-flex rounded-full h-3 w-3"
                      :class="networkStore.hostStatus.is_running ? 'bg-emerald-500' : 'bg-rose-500'"
                    ></span>
                  </span>
                  <div>
                    <h2 class="text-sm font-black text-slate-900 tracking-tight flex items-center gap-2">
                      <span>PANEL KONTROL SERVER HOST</span>
                      <span
                        class="text-[10px] font-bold px-2 py-0.2 rounded-full border"
                        :class="networkStore.hostStatus.is_running
                          ? 'bg-emerald-50 text-emerald-800 border-emerald-300'
                          : 'bg-rose-50 text-rose-800 border-rose-300'"
                      >
                        {{ networkStore.hostStatus.is_running ? 'STATUS: RUNNING' : 'STATUS: STOPPED' }}
                      </span>
                    </h2>
                    <p class="text-[11px] text-slate-500">
                      Dual Port Engine: Port Utama 8080 (REST/WS) & Port Kontrol 8081 | Uptime: {{ networkStore.formattedUptime }}
                    </p>
                  </div>
                </div>

                <!-- Server Action Buttons -->
                <div class="flex items-center gap-2">
                  <button
                    type="button"
                    @click="handleStartStopServer"
                    class="px-3 py-1.5 rounded-lg text-xs font-bold transition flex items-center gap-1.5 cursor-pointer shadow-2xs"
                    :class="networkStore.hostStatus.is_running
                      ? 'bg-rose-50 hover:bg-rose-100 text-rose-700 border border-rose-300'
                      : 'bg-emerald-600 hover:bg-emerald-700 text-white'"
                  >
                    <Power class="w-3.5 h-3.5" />
                    <span>{{ networkStore.hostStatus.is_running ? 'Hentikan Server' : 'Nyalakan Server' }}</span>
                  </button>

                  <button
                    type="button"
                    @click="isTrayOnlyMode = true"
                    class="px-3 py-1.5 bg-slate-100 hover:bg-slate-200 text-slate-700 border border-slate-300 rounded-lg text-xs font-bold transition flex items-center gap-1.5 cursor-pointer"
                    title="Minimalkan ke System Tray"
                  >
                    <Minimize2 class="w-3.5 h-3.5" />
                    <span>Tray Only</span>
                  </button>
                </div>
              </div>

              <!-- 4 Pilar Arsitektur Server (Diagram) -->
              <div class="grid grid-cols-2 gap-3 text-xs">
                <!-- 1. Axum Web Server -->
                <div class="p-3.5 rounded-xl border border-slate-200 bg-slate-50 flex items-start gap-3">
                  <div class="w-8 h-8 rounded-lg bg-emerald-100 text-emerald-700 flex items-center justify-center shrink-0">
                    <Server class="w-4 h-4" />
                  </div>
                  <div class="flex-1 min-w-0">
                    <div class="font-extrabold text-slate-800 text-xs">Axum Web Server</div>
                    <div class="text-[11px] font-mono text-slate-500">Port: 8080 (REST & WS)</div>
                    <span class="text-[9px] font-bold text-emerald-700 bg-emerald-100 px-1.5 py-0.2 rounded mt-1 inline-block">
                      {{ networkStore.hostStatus.is_running ? 'Aktif Melayani Kasir' : 'Dihentikan' }}
                    </span>
                  </div>
                </div>

                <!-- 2. PostgreSQL Database -->
                <div class="p-3.5 rounded-xl border border-slate-200 bg-slate-50 flex items-start gap-3">
                  <div class="w-8 h-8 rounded-lg bg-blue-100 text-blue-700 flex items-center justify-center shrink-0">
                    <Database class="w-4 h-4" />
                  </div>
                  <div class="flex-1 min-w-0">
                    <div class="font-extrabold text-slate-800 text-xs">PostgreSQL Database</div>
                    <div class="text-[11px] font-mono text-slate-500">Port: 5432 (notane_pos)</div>
                    <span class="text-[9px] font-bold text-blue-700 bg-blue-100 px-1.5 py-0.2 rounded mt-1 inline-block">
                      Database Terhubung
                    </span>
                  </div>
                </div>

                <!-- 3. mDNS Broadcaster -->
                <div class="p-3.5 rounded-xl border border-slate-200 bg-slate-50 flex items-start gap-3">
                  <div class="w-8 h-8 rounded-lg bg-purple-100 text-purple-700 flex items-center justify-center shrink-0">
                    <Radio class="w-4 h-4" />
                  </div>
                  <div class="flex-1 min-w-0">
                    <div class="font-extrabold text-slate-800 text-xs">mDNS Broadcaster</div>
                    <div class="text-[11px] font-mono text-slate-500">
                      IP: {{ networkStore.hostStatus.host_ips?.[0] || '127.0.0.1' }}
                    </div>
                    <span class="text-[9px] font-bold text-purple-700 bg-purple-100 px-1.5 py-0.2 rounded mt-1 inline-block">
                      Broadcasting ke LAN Toko
                    </span>
                  </div>
                </div>

                <!-- 4. Kode Pairing Otorisasi -->
                <div class="p-3.5 rounded-xl border border-amber-300 bg-amber-50/70 flex items-start gap-3">
                  <div class="w-8 h-8 rounded-lg bg-amber-200 text-amber-800 flex items-center justify-center shrink-0">
                    <Lock class="w-4 h-4" />
                  </div>
                  <div class="flex-1 min-w-0">
                    <div class="font-extrabold text-amber-900 text-xs flex items-center justify-between">
                      <span>Kode Pairing Terminal</span>
                      <div class="flex items-center gap-2">
                        <button
                          type="button"
                          @click="handleRegeneratePairing"
                          class="text-[10px] text-amber-700 hover:text-amber-900 font-mono font-bold hover:underline cursor-pointer"
                        >
                          Acak Ulang
                        </button>
                        <button
                          type="button"
                          @click="copyText(networkStore.hostStatus.pairing_code || 'NOTANE-8899')"
                          class="text-[10px] text-amber-700 hover:text-amber-900 font-mono font-bold flex items-center gap-0.5 cursor-pointer"
                        >
                          <Check v-if="copiedPairing" class="w-3 h-3 text-emerald-600" />
                          <Copy v-else class="w-3 h-3" />
                          <span>{{ copiedPairing ? 'Tersalin' : 'Salin' }}</span>
                        </button>
                      </div>
                    </div>
                    <div class="text-sm font-bold font-mono text-amber-900 tracking-wider mt-0.5">
                      {{ networkStore.hostStatus.pairing_code || 'NOTANE-8899' }}
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <!-- Live Connected Client Terminals Table -->
            <div class="bg-white border border-slate-300 rounded-2xl p-5 shadow-sm space-y-3">
              <div class="flex items-center justify-between border-b border-slate-200 pb-2.5">
                <div class="flex items-center gap-2">
                  <Users class="w-4 h-4 text-slate-600" />
                  <span class="font-extrabold text-xs text-slate-800 uppercase tracking-wide">
                    Terminal Kasir Terhubung ({{ networkStore.connectedDevices.length }})
                  </span>
                </div>
                <button
                  type="button"
                  @click="checkLocalServer"
                  :disabled="isCheckingServer"
                  class="text-blue-600 hover:text-blue-800 text-[11px] font-bold flex items-center gap-1 cursor-pointer"
                >
                  <RefreshCw class="w-3 h-3" :class="{ 'animate-spin': isCheckingServer }" />
                  <span>Refresh Daftar</span>
                </button>
              </div>

              <div v-if="networkStore.connectedDevices.length > 0" class="overflow-x-auto">
                <table class="w-full text-left text-xs">
                  <thead class="bg-slate-50 text-slate-500 font-semibold border-b border-slate-200">
                    <tr>
                      <th class="p-2.5">Device ID</th>
                      <th class="p-2.5">Nama Perangkat</th>
                      <th class="p-2.5">IP Client</th>
                      <th class="p-2.5">Tipe</th>
                      <th class="p-2.5 text-right">Aksi</th>
                    </tr>
                  </thead>
                  <tbody class="divide-y divide-slate-100">
                    <tr v-for="dev in networkStore.connectedDevices" :key="dev.device_id" class="hover:bg-slate-50/50">
                      <td class="p-2.5 font-bold font-mono text-slate-800">{{ dev.device_id }}</td>
                      <td class="p-2.5 text-slate-700">{{ dev.device_name }}</td>
                      <td class="p-2.5 font-mono text-slate-500">{{ dev.client_ip }}</td>
                      <td class="p-2.5">
                        <span class="px-2 py-0.5 rounded text-[10px] font-bold bg-blue-100 text-blue-800">
                          {{ dev.device_type }}
                        </span>
                      </td>
                      <td class="p-2.5 text-right">
                        <button
                          type="button"
                          @click="handleKick(dev.device_id)"
                          class="px-2 py-1 text-[10px] font-bold text-rose-700 hover:bg-rose-50 border border-rose-300 rounded cursor-pointer"
                        >
                          Putuskan
                        </button>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>

              <div v-else class="p-4 bg-slate-50 rounded-xl text-center text-xs text-slate-500">
                Belum ada kasir client yang terhubung. Terminal kasir cabang dapat menghubungkan diri menggunakan IP LAN di atas.
              </div>
            </div>

            <!-- IMPORTANT ARCHITECTURE NOTICE (PC UTAMA TIDAK BISA KASIR / BACKOFFICE) -->
            <div class="bg-slate-800 text-slate-100 rounded-2xl p-5 shadow-sm space-y-3">
              <div class="flex items-start gap-3">
                <div class="p-2 bg-slate-700 rounded-xl text-amber-400 shrink-0 mt-0.5">
                  <Info class="w-5 h-5" />
                </div>
                <div class="space-y-1">
                  <h3 class="text-xs font-bold uppercase tracking-wider text-white">
                    Peraturan Arsitektur Mode Server
                  </h3>
                  <p class="text-xs text-slate-300 leading-relaxed">
                    Komputer ini beroperasi sebagai <strong>Server Host murni</strong> (Axum REST API, WebSocket, dan PostgreSQL). Mode Server tidak menyediakan antarmuka kasir maupun akses data back-office.
                  </p>
                  <p class="text-[11px] text-slate-400">
                    Untuk melakukan transaksi kasir atau melihat laporan keuangan, stok, dan master data, jalankan aplikasi melalui <strong>Mode Client</strong> (di komputer ini via IP 127.0.0.1 atau komputer kasir lain di LAN).
                  </p>
                </div>
              </div>

              <div class="pt-2 border-t border-slate-700 flex justify-end">
                <button
                  type="button"
                  @click="currentMode = 'CLIENT'"
                  class="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white font-bold rounded-xl text-xs transition flex items-center gap-1.5 cursor-pointer shadow-xs"
                >
                  <Monitor class="w-3.5 h-3.5" />
                  <span>Buka Mode Client (Kasir & Laporan Data)</span>
                  <ArrowRight class="w-3.5 h-3.5" />
                </button>
              </div>
            </div>
          </div>
        </div>

        <!-- ============================================================== -->
        <!-- VIEW B: MODE CLIENT (KASIR POS & BACK-OFFICE LAPORAN) -->
        <!-- ============================================================== -->
        <div v-else-if="currentMode === 'CLIENT'" class="space-y-4">
          <!-- 1. Offline Storage Status Banner -->
          <div class="bg-white border border-slate-300 rounded-2xl p-4 shadow-sm flex items-center justify-between">
            <div class="flex items-center gap-3">
              <div class="w-9 h-9 rounded-xl bg-amber-100 text-amber-700 flex items-center justify-center font-bold">
                <HardDrive class="w-5 h-5" />
              </div>
              <div>
                <div class="font-extrabold text-xs text-slate-900 flex items-center gap-1.5">
                  <span>Offline Storage Ready</span>
                  <span class="text-[9px] bg-amber-100 text-amber-800 px-1.5 py-0.2 rounded font-mono font-bold">
                    Client Cache
                  </span>
                </div>
                <div class="text-[11px] text-slate-500">
                  {{ cachedProductsCount }} Produk di-cache | {{ pendingQueueCount }} Antrean Transaksi Tertunda
                </div>
              </div>
            </div>

            <div class="text-right">
              <span class="text-[10px] text-slate-400 block">Kesiapan Offline</span>
              <span class="text-xs font-bold text-emerald-600">Siap Operasional</span>
            </div>
          </div>

          <!-- 2. SERVER SELECTOR (mDNS Discovery + Manual Host Selector) -->
          <ServerSelector @connected="handleServerConnected" />

          <!-- 3. Form Login Kasir & Back-Office (Hanya aktif jika terhubung ke server) -->
          <div class="bg-white border border-slate-300 rounded-2xl p-6 shadow-sm space-y-4">
            <div class="border-b border-slate-200 pb-2 flex items-center justify-between">
              <div>
                <h2 class="text-sm font-black text-slate-900">Masuk Terminal Kasir / Back-Office</h2>
                <p class="text-[11px] text-slate-500">Data operasional akan disinkronkan secara real-time ke Server Host</p>
              </div>
              <div class="text-right">
                <span
                  class="text-[10px] font-bold px-2 py-0.5 rounded-full border"
                  :class="isConnected ? 'bg-emerald-50 text-emerald-700 border-emerald-300' : 'bg-rose-50 text-rose-700 border-rose-300'"
                >
                  {{ isConnected ? 'Terhubung ke Host' : 'Belum Terhubung' }}
                </span>
              </div>
            </div>

            <!-- Alert jika belum connect -->
            <div v-if="!isConnected" class="p-3 bg-amber-50 border border-amber-300 rounded-xl text-xs text-amber-800 flex items-start gap-2">
              <WifiOff class="w-4 h-4 text-amber-600 shrink-0 mt-0.5" />
              <div>
                <span class="font-bold">Sambungkan ke Server Host:</span>
                Gunakan panel pemindai <strong>Server Selector</strong> di atas untuk menyambungkan terminal ini ke PC Server Utama sebelum login.
              </div>
            </div>

            <form @submit.prevent="handleLogin" class="space-y-3">
              <!-- Terminal Selection -->
              <div>
                <label class="block text-xs font-bold text-slate-700 mb-1">Pilih Nomor Terminal</label>
                <div class="relative">
                  <Terminal class="w-4 h-4 text-slate-400 absolute left-3.5 top-1/2 -translate-y-1/2" />
                  <select
                    v-model="terminalId"
                    class="w-full bg-slate-50 border border-slate-300 rounded-lg pl-10 pr-4 py-2 text-xs font-mono text-slate-900 focus:bg-white focus:border-blue-600 focus:outline-none"
                  >
                    <option value="KASIR-01">KASIR-01 (Terminal Kasir 1)</option>
                    <option value="KASIR-02">KASIR-02 (Terminal Kasir 2)</option>
                    <option value="KASIR-03">KASIR-03 (Terminal Kasir 3)</option>
                    <option value="BACKOFFICE-01">BACKOFFICE-01 (Admin & Laporan)</option>
                    <option value="TABLET-01">TABLET-01 (Tablet Pramusaji)</option>
                  </select>
                </div>
              </div>

              <!-- Username -->
              <div>
                <label class="block text-xs font-bold text-slate-700 mb-1">Username Staf</label>
                <div class="relative">
                  <User class="w-4 h-4 text-slate-400 absolute left-3.5 top-1/2 -translate-y-1/2" />
                  <input
                    v-model="username"
                    type="text"
                    required
                    placeholder="kasir1 atau admin"
                    class="w-full bg-slate-50 border border-slate-300 rounded-lg pl-10 pr-4 py-2 text-xs text-slate-900 focus:bg-white focus:border-blue-600 focus:outline-none"
                  />
                </div>
              </div>

              <!-- Password -->
              <div>
                <label class="block text-xs font-bold text-slate-700 mb-1">Password</label>
                <div class="relative">
                  <Lock class="w-4 h-4 text-slate-400 absolute left-3.5 top-1/2 -translate-y-1/2" />
                  <input
                    v-model="password"
                    type="password"
                    required
                    placeholder="kasir123 atau admin123"
                    class="w-full bg-slate-50 border border-slate-300 rounded-lg pl-10 pr-4 py-2 text-xs font-mono text-slate-900 focus:bg-white focus:border-blue-600 focus:outline-none"
                  />
                </div>
              </div>

              <!-- Error Message -->
              <div v-if="errorMsg" class="bg-rose-50 border border-rose-300 p-2.5 rounded-lg text-rose-700 text-xs font-medium">
                ⚠️ {{ errorMsg }}
              </div>

              <!-- Dual Submit Buttons: Kasir POS vs Back-Office Admin -->
              <div class="grid grid-cols-2 gap-2 pt-1">
                <button
                  type="submit"
                  :disabled="isLoggingIn || !isConnected"
                  class="font-bold py-2.5 rounded-lg text-xs transition active:scale-95 shadow-xs border flex items-center justify-center gap-1.5 cursor-pointer disabled:cursor-not-allowed"
                  :class="!isConnected ? 'bg-slate-200 text-slate-400 border-slate-300' : 'bg-emerald-600 hover:bg-emerald-700 text-white border-emerald-700'"
                >
                  <span v-if="isLoggingIn">Memverifikasi...</span>
                  <span v-else-if="!isConnected">HUBUNGKAN SERVER DAHULU</span>
                  <span v-else>Masuk Kasir POS</span>
                  <ArrowRight class="w-3.5 h-3.5" />
                </button>

                <button
                  type="button"
                  @click="submitLogin('/admin')"
                  :disabled="isLoggingIn || !isConnected"
                  class="font-bold py-2.5 rounded-lg text-xs transition active:scale-95 shadow-xs border flex items-center justify-center gap-1.5 cursor-pointer disabled:cursor-not-allowed"
                  :class="!isConnected ? 'bg-slate-200 text-slate-400 border-slate-300' : 'bg-slate-800 hover:bg-slate-900 text-white border-slate-900'"
                >
                  <span>Buka Data & Laporan</span>
                  <Database class="w-3.5 h-3.5" />
                </button>
              </div>
            </form>

            <!-- Quick login kasir -->
            <div class="pt-2 border-t border-slate-200">
              <div class="text-[11px] font-bold text-slate-500 mb-1">Akun Cepat:</div>
              <div class="grid grid-cols-3 gap-2">
                <button
                  type="button"
                  @click="quickFill('kasir1', 'kasir123')"
                  class="p-1.5 bg-slate-50 hover:bg-slate-100 border border-slate-200 rounded-lg text-center cursor-pointer text-xs font-bold text-slate-800"
                >
                  Kasir 1
                </button>
                <button
                  type="button"
                  @click="quickFill('spv', 'spv123')"
                  class="p-1.5 bg-slate-50 hover:bg-slate-100 border border-slate-200 rounded-lg text-center cursor-pointer text-xs font-bold text-slate-800"
                >
                  Supervisor
                </button>
                <button
                  type="button"
                  @click="quickFill('admin', 'admin123')"
                  class="p-1.5 bg-slate-50 hover:bg-slate-100 border border-slate-200 rounded-lg text-center cursor-pointer text-xs font-bold text-slate-800"
                >
                  Administrator
                </button>
              </div>
            </div>

            <!-- Subtle Switch Link at bottom -->
            <div class="pt-3 border-t border-slate-100 text-center">
              <button
                type="button"
                @click="currentMode = 'SERVER'"
                class="text-[11px] text-slate-400 hover:text-slate-600 transition underline cursor-pointer"
              >
                ⚙️ Alihkan perangkat ini ke Mode Server Host
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
