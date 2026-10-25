<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRouter } from 'vue-router'
import {
  Lock,
  User,
  Terminal,
  ArrowRight,
  WifiOff,
  HardDrive,
  Database,
  Store,
} from 'lucide-vue-next'
import { useAuthStore } from '@/stores/authStore'
import { useNetworkStore } from '@/stores/networkStore'
import { useStoreBrandingStore } from '@/stores/storeBrandingStore'
import { offlineStorage } from '@/services/offlineStorage'
import ServerSelector from '@/components/common/ServerSelector.vue'

const router = useRouter()
const authStore = useAuthStore()
const networkStore = useNetworkStore()
const storeBrandingStore = useStoreBrandingStore()

// Form Login State (Mode Client Kasir & Back-Office)
const username = ref<string>('admin')
const password = ref<string>('admin123')
const terminalId = ref<string>(authStore.terminalId || 'KASIR-01')
const isLoggingIn = ref<boolean>(false)
const errorMsg = ref<string>('')

// Offline Storage Info
const cachedProductsCount = ref<number>(offlineStorage.getCachedProductsCount())
const pendingQueueCount = ref<number>(offlineStorage.getPendingQueueCount())

// Server Connection State
const isCheckingServer = ref<boolean>(false)

const isConnected = computed(() => networkStore.connectionStatus === 'CONNECTED')

onMounted(async () => {
  // Pastikan node role adalah client
  networkStore.setNodeRole('CLIENT')
  cachedProductsCount.value = offlineStorage.getCachedProductsCount()
  pendingQueueCount.value = offlineStorage.getPendingQueueCount()
  storeBrandingStore.fetchBranding().catch(() => {})
  await checkClientConnection()
})

async function checkClientConnection() {
  if (networkStore.connectionStatus === 'CONNECTED') {
    storeBrandingStore.fetchBranding().catch(() => {})
    return
  }
  isCheckingServer.value = true
  try {
    await networkStore.connect()
    storeBrandingStore.fetchBranding().catch(() => {})
  } catch {
    // Handled by store
  } finally {
    isCheckingServer.value = false
  }
}

function handleServerConnected() {
  errorMsg.value = ''
  storeBrandingStore.fetchBranding().catch(() => {})
}

async function submitLogin(targetRoute: string = '/') {
  if (!isConnected.value) {
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
      networkStore.connectionStatus = 'DISCONNECTED'
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
            <template v-if="storeBrandingStore.removeBranding">
              <img
                v-if="storeBrandingStore.storeLogoUrl"
                :src="storeBrandingStore.storeLogoUrl"
                :alt="storeBrandingStore.storeName"
                class="w-12 h-12 object-contain rounded-xl"
              />
              <div v-else class="w-12 h-12 rounded-xl bg-slate-100 text-slate-700 flex items-center justify-center font-bold">
                <Store class="w-6 h-6 text-slate-700" />
              </div>
            </template>
            <template v-else>
              <img src="/logo.png" alt="Notane Logo" class="w-12 h-12 object-contain" />
            </template>
          </div>

          <h1 class="text-2xl font-black text-slate-900 tracking-tight">
            <template v-if="storeBrandingStore.removeBranding">
              {{ storeBrandingStore.storeName || 'RETAIL POS' }}
            </template>
            <template v-else>
              NOTANE <span class="text-notane-gradient">RETAIL POS</span>
            </template>
          </h1>
          <p class="text-xs text-slate-600">
            {{ storeBrandingStore.removeBranding ? (storeBrandingStore.storeTagline || 'Terminal Kasir POS & Back-Office Toko') : 'Terminal Kasir POS & Back-Office Toko' }}
          </p>
        </div>

        <!-- CLIENT TERMINAL VIEW -->
        <div class="space-y-4">
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
                    placeholder="Masukkan nama pengguna"
                    class="w-full bg-slate-50 border border-slate-300 rounded-lg pl-10 pr-4 py-2 text-xs text-slate-900 focus:bg-white focus:border-blue-600 focus:outline-none"
                  />
                </div>
              </div>

              <!-- Password -->
              <div>
                <label class="block text-xs font-bold text-slate-700 mb-1">Kata Sandi</label>
                <div class="relative">
                  <Lock class="w-4 h-4 text-slate-400 absolute left-3.5 top-1/2 -translate-y-1/2" />
                  <input
                    v-model="password"
                    type="password"
                    required
                    placeholder="Masukkan kata sandi"
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
                  <span v-else-if="!isConnected">Hubungkan Server Dahulu</span>
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
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
