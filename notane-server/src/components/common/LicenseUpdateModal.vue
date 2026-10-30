<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import {
  ShieldCheck,
  Key,
  RefreshCw,
  Cpu,
  CheckCircle2,
  AlertTriangle,
  XCircle,
  Copy,
  Check,
  DownloadCloud,
  Sparkles,
  Lock,
  Layers,
  Server,
  Globe,
  Zap,
} from 'lucide-vue-next'
import { useLicenseStore } from '@/stores/licenseStore'
import { useDialog } from '@/composables/useDialog'

const props = withDefaults(
  defineProps<{
    isOpen: boolean
    initialTab?: 'LICENSE' | 'PROVIDER' | 'UPDATE'
  }>(),
  {
    initialTab: 'LICENSE',
  }
)

const emit = defineEmits<{
  (e: 'close'): void
}>()

const licenseStore = useLicenseStore()
const dialog = useDialog()
const activeTab = ref<'LICENSE' | 'PROVIDER' | 'UPDATE'>(props.initialTab || 'LICENSE')
const activationMode = ref<'ONLINE' | 'OFFLINE'>('ONLINE')

// Form Aktivasi
const onlineLicenseKey = ref('')
const tokenInput = ref('')
const signatureInput = ref('')
const copiedHwid = ref(false)

import { watch } from 'vue'

watch(
  () => props.isOpen,
  (open) => {
    if (open) {
      if (props.initialTab) {
        activeTab.value = props.initialTab
      }
      if (activeTab.value === 'UPDATE') {
        handleCheckUpdate()
      }
    }
  }
)

onMounted(async () => {
  await licenseStore.fetchVersionInfo()
  await licenseStore.fetchLicenseInfo()
  await licenseStore.fetchProviderConfig()
})

const isLicenseActive = computed(() => licenseStore.licenseInfo.status === 'ACTIVE')
const isGracePeriod = computed(() => licenseStore.licenseInfo.status === 'GRACE_PERIOD')
const isExpired = computed(() => licenseStore.licenseInfo.status === 'EXPIRED')
const isHwidMismatch = computed(() => licenseStore.licenseInfo.status === 'HARDWARE_MISMATCH')
const isClockTampered = computed(() => licenseStore.licenseInfo.status === 'CLOCK_TAMPERED')

function copyHwid() {
  navigator.clipboard.writeText(licenseStore.licenseInfo.current_hardware_id)
  copiedHwid.value = true
  setTimeout(() => { copiedHwid.value = false }, 2000)
}

async function handleOnlineActivate() {
  if (!onlineLicenseKey.value.trim()) {
    dialog.alert('Mohon masukkan Nomor Lisensi resmi (contoh: NOTANE-ENT-XXXX-XXXX).', 'warning')
    return
  }
  const ok = await licenseStore.activateLicenseOnline(onlineLicenseKey.value)
  if (ok) {
    onlineLicenseKey.value = ''
  }
}

async function handleActivateOffline() {
  if (!tokenInput.value || !signatureInput.value) {
    dialog.alert('Mohon isi Token Lisensi (JSON) dan Signature Kriptografi (Hex) dari Provider.', 'warning')
    return
  }
  const ok = await licenseStore.activateLicense(tokenInput.value, signatureInput.value)
  if (ok) {
    tokenInput.value = ''
    signatureInput.value = ''
  }
}

async function handleTestConnection() {
  await licenseStore.testProviderConnection()
}

async function handleCheckUpdate() {
  await licenseStore.checkForUpdates()
}

async function handleApplyUpdate() {
  const ok = await licenseStore.applyOneClickUpdate()
  if (ok) {
    await dialog.alert('Pembaruan sistem berhasil disiapkan! Aplikasi akan dimuat ulang.', 'success')
    window.location.reload()
  }
}
</script>

<template>
  <div v-if="isOpen" class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/60 backdrop-blur-xs">
    <div class="bg-white rounded-2xl shadow-2xl border border-slate-200 w-full max-w-2xl overflow-hidden flex flex-col max-h-[92vh] animate-in fade-in zoom-in-95 duration-150">
      
      <!-- Modal Header -->
      <div class="px-6 py-4 bg-slate-900 text-white flex items-center justify-between">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-xl bg-white/10 flex items-center justify-center border border-white/20">
            <ShieldCheck class="w-5 h-5 text-emerald-400" />
          </div>
          <div>
            <h3 class="font-bold text-base tracking-tight flex items-center gap-2">
              <span>Lisensi & Pembaruan Sistem</span>
              <span class="text-[10px] bg-emerald-500/20 text-emerald-300 border border-emerald-500/30 px-2 py-0.5 rounded-full font-mono">
                v{{ licenseStore.versionInfo.version }}
              </span>
            </h3>
            <p class="text-xs text-slate-400">
              Pengaturan lisensi toko dan status pembaruan server
            </p>
          </div>
        </div>

        <button
          @click="emit('close')"
          class="text-slate-400 hover:text-white p-1 rounded-lg hover:bg-white/10 transition cursor-pointer"
        >
          ✕
        </button>
      </div>

      <!-- Navigation Tabs -->
      <div class="flex border-b border-slate-200 bg-slate-50 px-6 pt-2">
        <button
          type="button"
          @click="activeTab = 'LICENSE'"
          class="px-4 py-2.5 text-xs font-bold border-b-2 transition flex items-center gap-2 cursor-pointer"
          :class="activeTab === 'LICENSE'
            ? 'border-indigo-600 text-indigo-700 bg-white rounded-t-lg shadow-2xs'
            : 'border-transparent text-slate-500 hover:text-slate-700'"
        >
          <Key class="w-3.5 h-3.5" />
          <span>Status & Aktivasi Lisensi</span>
        </button>

        <button
          type="button"
          @click="activeTab = 'PROVIDER'"
          class="px-4 py-2.5 text-xs font-bold border-b-2 transition flex items-center gap-2 cursor-pointer"
          :class="activeTab === 'PROVIDER'
            ? 'border-indigo-600 text-indigo-700 bg-white rounded-t-lg shadow-2xs'
            : 'border-transparent text-slate-500 hover:text-slate-700'"
        >
          <Server class="w-3.5 h-3.5" />
          <span>Server Provider</span>
          <span
            class="w-2 h-2 rounded-full"
            :class="licenseStore.testResult?.success ? 'bg-emerald-500' : 'bg-slate-300'"
          />
        </button>

        <button
          type="button"
          @click="activeTab = 'UPDATE'"
          class="px-4 py-2.5 text-xs font-bold border-b-2 transition flex items-center gap-2 cursor-pointer"
          :class="activeTab === 'UPDATE'
            ? 'border-indigo-600 text-indigo-700 bg-white rounded-t-lg shadow-2xs'
            : 'border-transparent text-slate-500 hover:text-slate-700'"
        >
          <DownloadCloud class="w-3.5 h-3.5" />
          <span>Pembaruan Sistem (OTA)</span>
          <span
            v-if="licenseStore.updateInfo?.has_update"
            class="w-2 h-2 rounded-full bg-rose-500 animate-ping"
          />
        </button>
      </div>

      <!-- Modal Body -->
      <div class="p-6 overflow-y-auto space-y-5 flex-1">
        
        <!-- Alerts (Error / Success) -->
        <div v-if="licenseStore.errorMessage" class="p-3 bg-rose-50 border border-rose-200 rounded-xl text-rose-800 text-xs flex items-center gap-2">
          <AlertTriangle class="w-4 h-4 shrink-0 text-rose-600" />
          <span>{{ licenseStore.errorMessage }}</span>
        </div>

        <div v-if="licenseStore.successMessage" class="p-3 bg-emerald-50 border border-emerald-200 rounded-xl text-emerald-800 text-xs flex items-center gap-2">
          <CheckCircle2 class="w-4 h-4 shrink-0 text-emerald-600" />
          <span>{{ licenseStore.successMessage }}</span>
        </div>

        <!-- ============================================================= -->
        <!-- TAB 1: STATUS & AKTIVASI LISENSI -->
        <!-- ============================================================= -->
        <div v-if="activeTab === 'LICENSE'" class="space-y-4">
          
          <!-- Hardware Fingerprint Card -->
          <div class="p-4 rounded-xl border border-slate-200 bg-slate-50/80 flex items-center justify-between">
            <div class="flex items-center gap-3">
              <div class="w-9 h-9 rounded-lg bg-indigo-100 text-indigo-700 flex items-center justify-center shrink-0">
                <Cpu class="w-5 h-5" />
              </div>
              <div>
                <div class="text-[11px] font-bold text-slate-500 uppercase tracking-wider">Kode Perangkat Komputer (ID Mesin)</div>
                <div class="font-mono font-extrabold text-sm text-slate-900 tracking-wider">
                  {{ licenseStore.licenseInfo.current_hardware_id }}
                </div>
                <div class="text-[10px] text-slate-500">Lisensi toko terhubung secara aman dengan komputer ini</div>
              </div>
            </div>

            <button
              type="button"
              @click="copyHwid"
              class="px-3 py-1.5 bg-white hover:bg-slate-100 border border-slate-300 rounded-lg text-xs font-bold text-slate-700 transition flex items-center gap-1.5 cursor-pointer shadow-2xs"
            >
              <Check v-if="copiedHwid" class="w-3.5 h-3.5 text-emerald-600" />
              <Copy v-else class="w-3.5 h-3.5" />
              <span>{{ copiedHwid ? 'Tersalin' : 'Salin ID Mesin' }}</span>
            </button>
          </div>

          <!-- License Status Badge & Details -->
          <div class="grid grid-cols-2 gap-3 text-xs">
            <div class="p-3.5 rounded-xl border border-slate-200 bg-white space-y-1">
              <span class="text-slate-500 block text-[11px] font-medium">Status Lisensi:</span>
              <div class="flex items-center gap-2">
                <span
                  class="font-black px-2.5 py-0.5 rounded-full text-[10px] border flex items-center gap-1 inline-flex"
                  :class="{
                    'bg-emerald-50 text-emerald-800 border-emerald-300': isLicenseActive,
                    'bg-amber-50 text-amber-800 border-amber-300': isGracePeriod,
                    'bg-rose-50 text-rose-800 border-rose-300': isExpired || isHwidMismatch || isClockTampered,
                    'bg-slate-100 text-slate-700 border-slate-300': !licenseStore.licenseInfo.is_valid && !isExpired
                  }"
                >
                  <CheckCircle2 v-if="isLicenseActive" class="w-3 h-3 text-emerald-600" />
                  <AlertTriangle v-else-if="isGracePeriod" class="w-3 h-3 text-amber-600" />
                  <XCircle v-else class="w-3 h-3 text-rose-600" />
                  <span>{{ isLicenseActive ? 'AKTIF' : isGracePeriod ? 'MASA TENGGANG' : isExpired ? 'KEDALUWARSA' : 'BELUM DIAKTIVASI' }}</span>
                </span>
              </div>
              <p class="text-[11px] text-slate-600 pt-1 leading-snug">
                {{ licenseStore.licenseInfo.message }}
              </p>
            </div>

            <div class="p-3.5 rounded-xl border border-slate-200 bg-white space-y-1">
              <span class="text-slate-500 block text-[11px] font-medium">Pelanggan & Paket:</span>
              <div class="font-extrabold text-slate-800 text-sm">
                {{ licenseStore.licenseInfo.client_name || 'Belum Terdaftar' }}
              </div>
              <div class="text-[11px] text-slate-600 font-mono">
                Paket: <span class="font-bold text-indigo-700">{{ licenseStore.licenseInfo.tier || '-' }}</span> | Maksimal: {{ licenseStore.licenseInfo.max_terminals }} Kasir
              </div>
              <div v-if="licenseStore.licenseInfo.expires_at" class="text-[10px] text-slate-500 pt-0.5">
                Masa Berlaku: {{ new Date(licenseStore.licenseInfo.expires_at).toLocaleDateString('id-ID', { year: 'numeric', month: 'long', day: 'numeric' }) }}
              </div>
            </div>
          </div>

          <!-- Provider Connection Mini Banner -->
          <div class="p-2.5 bg-indigo-50/70 border border-indigo-100 rounded-xl flex items-center justify-between text-xs">
            <div class="flex items-center gap-2">
              <Server class="w-4 h-4 text-indigo-600 shrink-0" />
              <div class="text-[11px]">
                <span class="text-slate-500">Pusat Layanan Notane: </span>
                <span class="font-mono font-bold text-indigo-900">{{ licenseStore.providerConfig.current_url }}</span>
              </div>
            </div>
            <button
              type="button"
              @click="activeTab = 'PROVIDER'"
              class="text-[11px] text-indigo-600 hover:text-indigo-800 font-bold underline cursor-pointer"
            >
              Info Server
            </button>
          </div>

          <!-- Form Aktivasi -->
          <div class="border border-slate-200 rounded-xl p-4 bg-slate-50 space-y-3">
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-2 text-xs font-bold text-slate-800">
                <Lock class="w-4 h-4 text-indigo-600" />
                <span>Aktivasi Lisensi Resmi</span>
              </div>

              <!-- Switcher Online / Manual -->
              <div class="flex bg-slate-200/80 p-0.5 rounded-lg text-[10px] font-bold">
                <button
                  type="button"
                  @click="activationMode = 'ONLINE'"
                  class="px-2.5 py-1 rounded-md transition cursor-pointer"
                  :class="activationMode === 'ONLINE' ? 'bg-white text-indigo-700 shadow-2xs' : 'text-slate-600'"
                >
                  ⚡ Online (1-Click)
                </button>
                <button
                  type="button"
                  @click="activationMode = 'OFFLINE'"
                  class="px-2.5 py-1 rounded-md transition cursor-pointer"
                  :class="activationMode === 'OFFLINE' ? 'bg-white text-indigo-700 shadow-2xs' : 'text-slate-600'"
                >
                  📄 Manual (Offline)
                </button>
              </div>
            </div>

            <!-- MODE 1: AKTIVASI ONLINE (1-CLICK) -->
            <div v-if="activationMode === 'ONLINE'" class="space-y-3 pt-1">
              <p class="text-[11px] text-slate-500">
                Masukkan Nomor Lisensi dari Provider. Sistem akan menghubungi server provider untuk mendaftarkan Hardware ID dan mengambil sertifikat digital secara otomatis:
              </p>

              <div>
                <label class="block text-[10px] font-bold text-slate-600 mb-1">Nomor Lisensi (License Key)</label>
                <div class="relative">
                  <input
                    v-model="onlineLicenseKey"
                    type="text"
                    placeholder="Contoh: NOTANE-ENT-2026-ABCD-1234"
                    class="w-full text-xs font-mono uppercase p-2.5 pl-9 bg-white border border-slate-300 rounded-lg focus:outline-none focus:border-indigo-600"
                    @keyup.enter="handleOnlineActivate"
                  />
                  <Key class="w-4 h-4 text-slate-400 absolute left-2.5 top-2.5" />
                </div>
              </div>

              <div class="flex justify-end pt-1">
                <button
                  type="button"
                  @click="handleOnlineActivate"
                  :disabled="licenseStore.isActivatingOnline || !onlineLicenseKey.trim()"
                  class="px-4 py-2 bg-indigo-600 hover:bg-indigo-700 text-white rounded-lg text-xs font-bold transition flex items-center gap-2 cursor-pointer shadow-xs disabled:opacity-50"
                >
                  <RefreshCw v-if="licenseStore.isActivatingOnline" class="w-3.5 h-3.5 animate-spin" />
                  <Zap v-else class="w-3.5 h-3.5" />
                  <span>{{ licenseStore.isActivatingOnline ? 'Mengaktivasi ke Provider...' : 'Aktivasi Online Sekarang' }}</span>
                </button>
              </div>
            </div>

            <!-- MODE 2: AKTIVASI MANUAL (OFFLINE) -->
            <div v-else class="space-y-2 pt-1">
              <p class="text-[11px] text-slate-500">
                Gunakan mode ini jika komputer kasir/server terisolasi total dari jaringan internet:
              </p>

              <div>
                <label class="block text-[10px] font-bold text-slate-600 mb-1">Kode Lisensi Toko</label>
                <textarea
                  v-model="tokenInput"
                  rows="2"
                  placeholder='Tempelkan teks lisensi toko Anda di sini...'
                  class="w-full text-xs font-mono p-2 bg-white border border-slate-300 rounded-lg focus:outline-none focus:border-indigo-600"
                />
              </div>

              <div>
                <label class="block text-[10px] font-bold text-slate-600 mb-1">Tanda Tangan Keamanan Lisensi</label>
                <input
                  v-model="signatureInput"
                  type="text"
                  placeholder="Tempelkan kode tanda tangan keamanan di sini..."
                  class="w-full text-xs font-mono p-2 bg-white border border-slate-300 rounded-lg focus:outline-none focus:border-indigo-600"
                />
              </div>

              <div class="pt-1 flex justify-end">
                <button
                  type="button"
                  @click="handleActivateOffline"
                  :disabled="licenseStore.isLoading"
                  class="px-4 py-2 bg-indigo-600 hover:bg-indigo-700 text-white rounded-lg text-xs font-bold transition flex items-center gap-2 cursor-pointer shadow-xs disabled:opacity-50"
                >
                  <RefreshCw v-if="licenseStore.isLoading" class="w-3.5 h-3.5 animate-spin" />
                  <Key v-else class="w-3.5 h-3.5" />
                  <span>{{ licenseStore.isLoading ? 'Memeriksa...' : 'Simpan Lisensi Toko' }}</span>
                </button>
              </div>
            </div>

          </div>
        </div>

        <!-- ============================================================= -->
        <!-- TAB 2: SERVER PROVIDER -->
        <!-- ============================================================= -->
        <div v-else-if="activeTab === 'PROVIDER'" class="space-y-4">
          
          <div class="p-4 rounded-xl border border-slate-200 bg-slate-50 space-y-4">
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-2">
                <div class="w-8 h-8 rounded-lg bg-indigo-100 text-indigo-700 flex items-center justify-center font-bold">
                  <Server class="w-4 h-4" />
                </div>
                <div>
                  <h4 class="font-bold text-slate-900 text-sm">Pusat Layanan Notane</h4>
                  <p class="text-[11px] text-slate-500">
                    Sambungan resmi untuk aktivasi dan pembaruan sistem toko
                  </p>
                </div>
              </div>

              <span
                class="text-[10px] font-bold px-2.5 py-1 rounded-full border"
                :class="licenseStore.providerConfig.is_production
                  ? 'bg-emerald-50 text-emerald-800 border-emerald-300'
                  : 'bg-amber-50 text-amber-800 border-amber-300'"
              >
                {{ licenseStore.providerConfig.is_production ? 'Server Resmi' : 'Server Uji Coba' }}
              </span>
            </div>

            <!-- Active Target URL Display Card -->
            <div class="p-3.5 bg-white border border-slate-200 rounded-xl space-y-2">
              <div class="flex items-center justify-between">
                <label class="text-[11px] font-bold text-slate-700 flex items-center gap-1.5">
                  <Globe class="w-3.5 h-3.5 text-indigo-600" />
                  <span>Alamat Server Layanan</span>
                </label>
              </div>

              <div class="p-2.5 bg-slate-900 text-emerald-400 font-mono text-xs rounded-lg select-all flex items-center justify-between">
                <span class="truncate">{{ licenseStore.providerConfig.current_url || 'Memuat alamat server...' }}</span>
              </div>
            </div>

            <!-- Catatan Keamanan Sambungan (Anti-Cloning Protection) -->
            <div class="p-3 rounded-lg border border-indigo-200 bg-indigo-50/60 text-indigo-950 flex items-start gap-2.5 text-xs">
              <Lock class="w-4 h-4 text-indigo-600 shrink-0 mt-0.5" />
              <div class="space-y-1">
                <div class="font-bold text-indigo-900 text-[11px]">Keamanan Lisensi Terkunci (Anti-Cloning Protection)</div>
                <p class="text-[11px] leading-relaxed text-indigo-800/90">
                  Alamat server lisensi dikunci mati di dalam binary rilis dan tidak dapat diubah di level antarmuka/toko untuk menjamin keaslian lisensi dan mencegah pemalsuan URL server provider.
                </p>
              </div>
            </div>

            <!-- Action Buttons -->
            <div class="flex items-center justify-end pt-2 border-t border-slate-200">
              <button
                type="button"
                @click="handleTestConnection"
                :disabled="licenseStore.isTestingConnection"
                class="px-4 py-2 bg-indigo-600 hover:bg-indigo-700 text-white rounded-lg text-xs font-bold transition flex items-center gap-1.5 cursor-pointer shadow-xs disabled:opacity-50"
              >
                <RefreshCw class="w-3.5 h-3.5" :class="{ 'animate-spin': licenseStore.isTestingConnection }" />
                <span>{{ licenseStore.isTestingConnection ? 'Menguji Sambungan...' : 'Uji Sambungan Server' }}</span>
              </button>
            </div>
          </div>

          <!-- Connection Test Feedback Card -->
          <div v-if="licenseStore.testResult" class="p-4 rounded-xl border text-xs space-y-2 animate-in fade-in"
            :class="licenseStore.testResult.success
              ? 'bg-emerald-50/70 border-emerald-300 text-emerald-900'
              : 'bg-rose-50/70 border-rose-300 text-rose-900'"
          >
            <div class="flex items-center justify-between font-bold">
              <div class="flex items-center gap-2">
                <CheckCircle2 v-if="licenseStore.testResult.success" class="w-4 h-4 text-emerald-600" />
                <AlertTriangle v-else class="w-4 h-4 text-rose-600" />
                <span>{{ licenseStore.testResult.message }}</span>
              </div>
              <span v-if="licenseStore.testResult.latency_ms !== undefined" class="font-mono text-[11px] px-2 py-0.5 rounded-full bg-white border border-emerald-200 text-emerald-700">
                ⚡ {{ licenseStore.testResult.latency_ms }} ms
              </span>
            </div>

            <div class="text-[11px] font-mono text-slate-600 space-y-0.5 pt-1">
              <div>Target: <strong>{{ licenseStore.testResult.target_url }}</strong></div>
              <div v-if="licenseStore.testResult.service">Layanan: <strong>{{ licenseStore.testResult.service }}</strong></div>
            </div>
          </div>

          <div v-else class="p-4 rounded-xl border border-dashed border-slate-200 bg-slate-50/50 text-center text-slate-500 text-xs">
            Klik tombol <strong>"Uji Koneksi Provider"</strong> di atas untuk memverifikasi apakah server Notane Server dapat terhubung ke Provider.
          </div>

        </div>

        <!-- ============================================================= -->
        <!-- TAB 3: PEMBARUAN SISTEM (ONE-CLICK UPDATER) -->
        <!-- ============================================================= -->
        <div v-else-if="activeTab === 'UPDATE'" class="space-y-4">
          
          <!-- Current Version Card -->
          <div class="p-4 rounded-xl border border-slate-200 bg-slate-50 flex items-center justify-between">
            <div class="flex items-center gap-3">
              <div class="w-10 h-10 rounded-xl bg-emerald-100 text-emerald-700 flex items-center justify-center font-bold text-sm">
                POS
              </div>
              <div>
                <h4 class="font-bold text-slate-900 text-sm">Notane Retail Server</h4>
                <div class="text-xs text-slate-500">
                  Versi Terpasang: <span class="font-bold text-emerald-700">v{{ licenseStore.versionInfo.version }}</span>
                  <span v-if="licenseStore.versionInfo.build_date"> | Rilis: {{ licenseStore.versionInfo.build_date }}</span>
                </div>
              </div>
            </div>

            <button
              type="button"
              @click="handleCheckUpdate"
              :disabled="licenseStore.isCheckingUpdate"
              class="px-3.5 py-1.5 bg-white hover:bg-slate-100 border border-slate-300 text-slate-800 rounded-lg text-xs font-bold transition flex items-center gap-1.5 cursor-pointer shadow-2xs disabled:opacity-50"
            >
              <RefreshCw class="w-3.5 h-3.5" :class="{ 'animate-spin': licenseStore.isCheckingUpdate }" />
              <span>{{ licenseStore.isCheckingUpdate ? 'Memeriksa...' : 'Periksa Pembaruan' }}</span>
            </button>
          </div>

          <!-- Update Result -->
          <div v-if="licenseStore.updateInfo" class="space-y-3">
            
            <!-- Update Available -->
            <div v-if="licenseStore.updateInfo.has_update" class="p-4 rounded-xl border border-emerald-300 bg-emerald-50/50 space-y-3">
              <div class="flex items-center justify-between">
                <div class="flex items-center gap-2">
                  <Sparkles class="w-4 h-4 text-emerald-600" />
                  <span class="font-extrabold text-slate-900 text-sm">Pembaruan Tersedia: v{{ licenseStore.updateInfo.latest_version }}</span>
                </div>
                <span class="text-[10px] font-bold px-2 py-0.5 rounded-full bg-emerald-100 text-emerald-800 border border-emerald-300">
                  Versi Baru
                </span>
              </div>

              <!-- Changelog -->
              <div v-if="licenseStore.updateInfo.changelog" class="p-3 bg-white border border-slate-200 rounded-lg text-xs text-slate-700 whitespace-pre-line font-sans">
                {{ licenseStore.updateInfo.changelog }}
              </div>

              <!-- Action Update Button -->
              <div class="flex items-center justify-between pt-1">
                <div class="text-[11px] text-slate-500">
                  Pembaruan sistem dan struktur data akan diterapkan secara aman dan otomatis.
                </div>
                <button
                  type="button"
                  @click="handleApplyUpdate"
                  :disabled="licenseStore.isApplyingUpdate || !licenseStore.updateInfo.can_update"
                  class="px-4 py-2 bg-emerald-600 hover:bg-emerald-700 text-white rounded-lg text-xs font-bold transition flex items-center gap-2 cursor-pointer shadow-xs disabled:opacity-50"
                >
                  <DownloadCloud class="w-4 h-4" :class="{ 'animate-bounce': licenseStore.isApplyingUpdate }" />
                  <span>{{ licenseStore.isApplyingUpdate ? 'Mengunduh & Memperbarui...' : 'Perbarui Sekarang' }}</span>
                </button>
              </div>
            </div>

            <!-- Up-to-date -->
            <div v-else class="p-6 rounded-xl border border-slate-200 bg-white text-center space-y-2">
              <CheckCircle2 class="w-8 h-8 text-emerald-500 mx-auto" />
              <h4 class="font-bold text-slate-800 text-sm">Server Anda Sudah Menggunakan Versi Terbaru</h4>
              <p class="text-xs text-slate-500 max-w-md mx-auto">
                {{ licenseStore.updateInfo.message }}
              </p>
            </div>
          </div>
          
          <div v-else class="p-6 rounded-xl border border-dashed border-slate-300 text-center text-slate-500 text-xs">
            Klik tombol <strong>"Periksa Pembaruan"</strong> untuk memeriksa apakah tersedia versi terbaru untuk server toko Anda.
          </div>
        </div>

      </div>

      <!-- Modal Footer -->
      <div class="px-6 py-3 bg-slate-100 border-t border-slate-200 flex items-center justify-between text-xs text-slate-500">
        <div class="flex items-center gap-1.5 text-[11px]">
          <Layers class="w-3.5 h-3.5 text-slate-400" />
          <span>Sistem Kasir & Server Notane POS</span>
        </div>
        <button
          type="button"
          @click="emit('close')"
          class="px-4 py-1.5 bg-slate-200 hover:bg-slate-300 text-slate-800 rounded-lg font-bold transition cursor-pointer"
        >
          Tutup
        </button>
      </div>

    </div>
  </div>
</template>
