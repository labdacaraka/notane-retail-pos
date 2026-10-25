<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, watch } from 'vue'
import {
  Server,
  Database,
  Wifi,
  Copy,
  Check,
  RefreshCw,
  Play,
  Square,
  ShieldCheck,
  Laptop,
  Users,
  Activity,
  Cpu,
  Key,
  KeyRound,
  Shuffle,
  X,
  CheckCircle2,
  AlertTriangle,
  XCircle,
  Image,
  Upload,
  Trash2,
  Building2,
  Sparkles,
  DownloadCloud,
} from 'lucide-vue-next'
import { useNetworkStore } from '@/stores/networkStore'
import { useLicenseStore } from '@/stores/licenseStore'
import { useStoreBrandingStore } from '@/stores/storeBrandingStore'
import { useInitialSetupStore } from '@/stores/initialSetupStore'
import LicenseUpdateModal from '@/components/common/LicenseUpdateModal.vue'
import InitialSetupWizardModal from '@/components/common/InitialSetupWizardModal.vue'

const networkStore = useNetworkStore()
const licenseStore = useLicenseStore()
const storeBrandingStore = useStoreBrandingStore()
const initialSetupStore = useInitialSetupStore()

const copiedIp = ref(false)
const copiedCode = ref(false)
const copiedHwid = ref(false)
const isActionLoading = ref(false)
const showLicenseModal = ref(false)
const licenseModalTab = ref<'LICENSE' | 'PROVIDER' | 'UPDATE'>('LICENSE')
const showInitialSetupModal = ref(false)

const selectedIp = ref<string>('')

// Store Branding State
const brandingForm = ref({
  storeName: '',
  storeTagline: '',
  storeLogoUrl: null as string | null,
})
const isSavingBranding = ref(false)
const brandingFeedback = ref('')
const fileInputRef = ref<HTMLInputElement | null>(null)

watch(
  () => [storeBrandingStore.storeName, storeBrandingStore.storeTagline, storeBrandingStore.storeLogoUrl],
  () => {
    brandingForm.value.storeName = storeBrandingStore.storeName
    brandingForm.value.storeTagline = storeBrandingStore.storeTagline
    brandingForm.value.storeLogoUrl = storeBrandingStore.storeLogoUrl
  },
  { immediate: true }
)

async function handleLogoUpload(e: Event) {
  const target = e.target as HTMLInputElement
  if (target.files && target.files[0]) {
    const file = target.files[0]
    try {
      const url = await storeBrandingStore.uploadLogoFile(file)
      brandingForm.value.storeLogoUrl = url
    } catch (err: any) {
      alert('Gagal memproses gambar logo: ' + err.message)
    }
  }
}

async function handleSaveBranding() {
  if (!brandingForm.value.storeName.trim()) {
    alert('Nama toko wajib diisi')
    return
  }
  isSavingBranding.value = true
  brandingFeedback.value = ''
  try {
    await storeBrandingStore.updateBranding({
      storeName: brandingForm.value.storeName.trim(),
      storeTagline: brandingForm.value.storeTagline.trim(),
      storeLogoUrl: brandingForm.value.storeLogoUrl,
    })
    brandingFeedback.value = 'Nama dan logo toko berhasil disimpan dan diterapkan ke seluruh kasir!'
    setTimeout(() => {
      brandingFeedback.value = ''
    }, 4000)
  } catch (err: any) {
    alert('Gagal menyimpan branding toko: ' + err.message)
  } finally {
    isSavingBranding.value = false
  }
}

function handleRemoveLogo() {
  brandingForm.value.storeLogoUrl = null
  if (fileInputRef.value) fileInputRef.value.value = ''
}

async function handleWizardCompleted() {
  await storeBrandingStore.fetchBranding()
  await initialSetupStore.fetchSetupStatus()
}

onMounted(async () => {
  networkStore.setNodeRole('SERVER')
  networkStore.startHostPolling()
  await licenseStore.fetchVersionInfo()
  await licenseStore.fetchLicenseInfo()
  await storeBrandingStore.fetchBranding()
  
  // Periksa apakah setup awal sudah selesai
  const isCompleted = await initialSetupStore.fetchSetupStatus()
  if (!isCompleted) {
    showInitialSetupModal.value = true
  }
})

onUnmounted(() => {
  networkStore.stopHostPolling()
})

const isServerRunning = computed(() => networkStore.hostStatus.is_running)

const displayPort = computed(() => {
  if (typeof window !== 'undefined' && window.location?.port) {
    return Number(window.location.port)
  }
  return (networkStore.hostStatus as any).external_port || networkStore.hostStatus.port || 18080
})

const detectedLanIps = computed(() => {
  const ips = [...(networkStore.hostStatus.host_ips || [])]
  if (typeof window !== 'undefined' && window.location?.hostname) {
    const h = window.location.hostname
    if (h && h !== '127.0.0.1' && h !== 'localhost' && !ips.includes(h)) {
      ips.unshift(h)
    }
  }
  const nonContainerIps = ips.filter((ip) => 
    ip !== '127.0.0.1' && 
    !ip.startsWith('169.254.') && 
    !ip.startsWith('10.88.') && 
    !ip.startsWith('10.89.') &&
    !ip.startsWith('172.')
  )
  if (nonContainerIps.length > 0) {
    return nonContainerIps
  }
  return ips.filter((ip) => ip !== '127.0.0.1' && !ip.startsWith('169.254.'))
})

const primaryIp = computed(() => {
  if (selectedIp.value && detectedLanIps.value.includes(selectedIp.value)) {
    return selectedIp.value
  }
  if (typeof window !== 'undefined' && window.location?.hostname) {
    const h = window.location.hostname
    if (h && h !== '127.0.0.1' && h !== 'localhost') {
      return h
    }
  }
  return detectedLanIps.value[0] || '127.0.0.1'
})

const isLicenseActive = computed(() => licenseStore.licenseInfo.status === 'ACTIVE')
const isGracePeriod = computed(() => licenseStore.licenseInfo.status === 'GRACE_PERIOD')
const isLicenseValid = computed(() => isLicenseActive.value || isGracePeriod.value || licenseStore.licenseInfo.is_valid)

const copyHostIp = () => {
  navigator.clipboard.writeText(`http://${primaryIp.value}:${displayPort.value}`)
  copiedIp.value = true
  setTimeout(() => (copiedIp.value = false), 2000)
}

const copyPairingCode = () => {
  const code = networkStore.hostStatus.pairing_code || 'NOTANE-8899'
  navigator.clipboard.writeText(code)
  copiedCode.value = true
  setTimeout(() => (copiedCode.value = false), 2000)
}

// Pairing Code Modal State & Actions
const showPairingModal = ref(false)
const newPairingInput = ref('')
const pairingError = ref('')
const pairingSuccess = ref('')
const isPairingSaving = ref(false)

const openPairingModal = () => {
  newPairingInput.value = networkStore.hostStatus.pairing_code || 'NOTANE-8899'
  pairingError.value = ''
  pairingSuccess.value = ''
  showPairingModal.value = true
}

const handleRegeneratePairing = async () => {
  isPairingSaving.value = true
  pairingError.value = ''
  pairingSuccess.value = ''
  try {
    const code = await networkStore.regeneratePairingCode()
    if (code) {
      newPairingInput.value = code
      pairingSuccess.value = `Kode acak baru dibuat: ${code}`
      setTimeout(() => (pairingSuccess.value = ''), 3000)
    } else {
      pairingError.value = 'Gagal mengacak ulang kode pairing dari server.'
    }
  } catch (err: any) {
    pairingError.value = err.message || 'Terjadi kesalahan saat mengacak kode.'
  } finally {
    isPairingSaving.value = false
  }
}

const handleSaveCustomPairing = async () => {
  const code = newPairingInput.value.trim().toUpperCase()
  if (code.length < 4 || code.length > 32) {
    pairingError.value = 'Kode pairing minimal 4 karakter dan maksimal 32 karakter.'
    return
  }
  isPairingSaving.value = true
  pairingError.value = ''
  pairingSuccess.value = ''
  try {
    const res = await networkStore.updatePairingCode(code)
    if (res.success) {
      pairingSuccess.value = res.message || 'Kode pairing toko berhasil diperbarui!'
      setTimeout(() => {
        showPairingModal.value = false
        pairingSuccess.value = ''
      }, 1500)
    } else {
      pairingError.value = res.message || 'Gagal menyimpan kode pairing.'
    }
  } catch (err: any) {
    pairingError.value = err.message || 'Terjadi kesalahan.'
  } finally {
    isPairingSaving.value = false
  }
}

const copyHwid = () => {
  navigator.clipboard.writeText(licenseStore.licenseInfo.current_hardware_id)
  copiedHwid.value = true
  setTimeout(() => (copiedHwid.value = false), 2000)
}

const toggleServer = async () => {
  if (isServerRunning.value) {
    const ok = window.confirm('Peringatan: Menghentikan server akan memutus koneksi seluruh komputer kasir yang sedang aktif. Lanjutkan menghentikan server?')
    if (!ok) return
  } else if (!isLicenseValid.value) {
    alert('Server Terkunci: Aplikasi belum teraktivasi lisensi resmi. Silakan lakukan aktivasi lisensi terlebih dahulu.')
    showLicenseModal.value = true
    return
  }
  isActionLoading.value = true
  try {
    if (isServerRunning.value) {
      await networkStore.stopHostServer()
    } else {
      await networkStore.startHostServer()
    }
  } finally {
    isActionLoading.value = false
  }
}

const restartServer = async () => {
  if (!isLicenseValid.value) {
    alert('Server Terkunci: Aplikasi belum teraktivasi lisensi resmi. Silakan lakukan aktivasi lisensi terlebih dahulu.')
    showLicenseModal.value = true
    return
  }
  isActionLoading.value = true
  try {
    await networkStore.restartHostServer()
  } finally {
    isActionLoading.value = false
  }
}
</script>

<template>
  <div class="min-h-screen bg-slate-100 text-slate-800 font-sans flex flex-col select-none">
    <!-- Top Header (Clean Light Desktop Bar) -->
    <header class="bg-white border-b border-slate-200 px-6 py-3.5 flex items-center justify-between shadow-xs sticky top-0 z-30">
      <div class="flex items-center gap-3">
        <div class="w-10 h-10 rounded-xl bg-indigo-600 flex items-center justify-center text-white shadow-xs">
          <Server class="w-5 h-5" />
        </div>
        <div>
          <div class="flex items-center gap-2">
            <h1 class="text-base font-extrabold text-slate-900 tracking-tight">Notane Retail Server</h1>
            <span class="px-2 py-0.5 rounded-full text-[10px] font-black uppercase bg-indigo-50 text-indigo-700 border border-indigo-200">
              Komputer Utama Toko
            </span>
          </div>
          <p class="text-xs text-slate-500">Pusat data dan layanan kasir toko</p>
        </div>
      </div>

      <!-- Header Right: Quick Status & License Button -->
      <div class="flex items-center gap-3">
        <button
          type="button"
          @click="showInitialSetupModal = true"
          class="px-3 py-1.5 rounded-xl border border-indigo-200 bg-indigo-50 hover:bg-indigo-100 text-indigo-700 text-xs font-bold transition flex items-center gap-1.5 cursor-pointer shadow-2xs hover:opacity-90 active:scale-95"
          title="Buka panduan pengaturan awal toko dan data"
        >
          <Sparkles class="w-3.5 h-3.5 text-indigo-600" />
          <span>Panduan Awal Toko</span>
        </button>

        <!-- Tombol Cek & Pembaruan Server -->
        <button
          type="button"
          @click="licenseModalTab = 'UPDATE'; showLicenseModal = true"
          class="px-3 py-1.5 rounded-xl border border-emerald-200 bg-emerald-50 hover:bg-emerald-100 text-emerald-800 text-xs font-bold transition flex items-center gap-1.5 cursor-pointer shadow-2xs hover:opacity-90 active:scale-95"
          title="Periksa dan terapkan pembaruan server toko"
        >
          <DownloadCloud class="w-3.5 h-3.5 text-emerald-600" />
          <span>Pembaruan Server</span>
          <span
            v-if="licenseStore.updateInfo?.has_update"
            class="px-1.5 py-0.2 rounded-full text-[9px] bg-emerald-600 text-white font-extrabold animate-pulse"
          >
            Ada Versi Baru
          </span>
        </button>

        <button
          type="button"
          @click="licenseModalTab = 'LICENSE'; showLicenseModal = true"
          class="px-3 py-1.5 rounded-xl border text-xs font-bold transition flex items-center gap-2 cursor-pointer shadow-2xs hover:opacity-90 active:scale-95"
          :class="isLicenseActive
            ? 'bg-purple-50 text-purple-900 border-purple-200 hover:bg-purple-100'
            : isGracePeriod
            ? 'bg-amber-50 text-amber-900 border-amber-200 hover:bg-amber-100'
            : 'bg-rose-50 text-rose-900 border-rose-200 hover:bg-rose-100'"
        >
          <Key class="w-3.5 h-3.5" :class="isLicenseActive ? 'text-purple-600' : 'text-rose-600'" />
          <span>Lisensi: {{ isLicenseActive ? (licenseStore.licenseInfo.tier || 'Aktif') : isGracePeriod ? 'Masa Tenggang' : 'Belum Diaktivasi' }}</span>
        </button>

        <div
          class="px-3 py-1.5 rounded-xl border text-xs font-extrabold flex items-center gap-2"
          :class="isServerRunning
            ? 'bg-emerald-50 text-emerald-800 border-emerald-200'
            : !isLicenseValid
            ? 'bg-rose-50 text-rose-800 border-rose-200'
            : 'bg-amber-50 text-amber-800 border-amber-200'"
        >
          <span class="w-2.5 h-2.5 rounded-full" :class="isServerRunning ? 'bg-emerald-500 animate-pulse' : !isLicenseValid ? 'bg-rose-500' : 'bg-amber-500'"></span>
          <span>{{ isServerRunning ? 'SERVER AKTIF' : !isLicenseValid ? 'BELUM DIAKTIVASI' : 'SERVER BERHENTI' }}</span>
        </div>
      </div>
    </header>

    <!-- Main Content Area -->
    <main class="flex-1 p-6 max-w-6xl w-full mx-auto space-y-6 overflow-y-auto">
      <!-- 1. STATUS SERVER CARD BANNER -->
      <div
        class="bg-white rounded-2xl p-6 border shadow-xs transition-all duration-300 relative overflow-hidden"
        :class="isServerRunning
          ? 'border-emerald-300 bg-gradient-to-r from-emerald-50/40 via-white to-white'
          : !isLicenseValid
          ? 'border-rose-300 bg-gradient-to-r from-rose-50/40 via-white to-white'
          : 'border-amber-300 bg-gradient-to-r from-amber-50/40 via-white to-white'"
      >
        <div class="flex flex-col md:flex-row items-start md:items-center justify-between gap-6">
          <div class="flex items-start gap-4">
            <div
              class="w-14 h-14 rounded-2xl flex items-center justify-center shrink-0 border shadow-xs"
              :class="isServerRunning
                ? 'bg-emerald-100 border-emerald-200 text-emerald-700'
                : !isLicenseValid
                ? 'bg-rose-100 border-rose-200 text-rose-700'
                : 'bg-amber-100 border-amber-200 text-amber-700'"
            >
              <Key v-if="!isLicenseValid && !isServerRunning" class="w-7 h-7" />
              <Activity v-else class="w-7 h-7" :class="{ 'animate-pulse': isServerRunning }" />
            </div>
            <div>
              <div class="flex items-center gap-2">
                <span class="text-xs font-bold text-slate-500 uppercase tracking-wider">Status Server</span>
                <span
                  class="px-2.5 py-0.5 rounded-full text-[11px] font-extrabold uppercase border"
                  :class="isServerRunning
                    ? 'bg-emerald-100 text-emerald-800 border-emerald-300'
                    : !isLicenseValid
                    ? 'bg-rose-100 text-rose-800 border-rose-300'
                    : networkStore.isDaemonOnline
                    ? 'bg-amber-100 text-amber-800 border-amber-300'
                    : 'bg-slate-100 text-slate-800 border-slate-300'"
                >
                  {{ isServerRunning
                    ? '● AKTIF'
                    : !isLicenseValid
                    ? '✕ BELUM DIAKTIVASI'
                    : networkStore.isDaemonOnline
                    ? '○ SIAP'
                    : '✕ BELUM AKTIF' }}
                </span>
              </div>
              <h2 class="text-xl font-black text-slate-900 mt-1">
                {{ isServerRunning
                  ? 'Server Siap Melayani Transaksi Kasir'
                  : !isLicenseValid
                  ? 'Server Terkunci - Lisensi Belum Diaktivasi'
                  : networkStore.isDaemonOnline
                  ? 'Server Siap Dijalankan'
                  : 'Layanan Server Belum Aktif' }}
              </h2>
              <p class="text-xs text-slate-600 mt-1 max-w-xl">
                {{ isServerRunning
                  ? 'Komputer kasir di toko dapat terhubung dan mencatat transaksi ke komputer ini.'
                  : !isLicenseValid
                  ? 'Layanan kasir belum dapat digunakan sebelum lisensi toko diaktivasi. Klik tombol Aktivasi Lisensi untuk memasukkan nomor lisensi toko Anda.'
                  : networkStore.isDaemonOnline
                  ? 'Klik tombol Nyalakan Server agar komputer kasir dapat login dan mencatat penjualan.'
                  : 'Layanan server belum berjalan di komputer ini. Silakan mulai layanan server terlebih dahulu.' }}
              </p>
            </div>
          </div>

          <!-- Controls Button -->
          <div class="flex items-center gap-2.5 shrink-0">
            <!-- Jika Server Berjalan: Tombol Utama adalah Mulai Ulang (Restart) -->
            <button
              v-if="isServerRunning"
              @click="restartServer"
              :disabled="isActionLoading"
              class="px-4 py-2.5 bg-indigo-600 hover:bg-indigo-700 text-white font-bold text-xs rounded-xl flex items-center gap-1.5 transition active:scale-95 disabled:opacity-50 cursor-pointer shadow-xs"
              title="Muat ulang socket dan koneksi server POS tanpa mematikan komputer"
            >
              <RefreshCw class="w-3.5 h-3.5" :class="{ 'animate-spin': isActionLoading }" />
              <span>Mulai Ulang Server</span>
            </button>

            <!-- Tombol Hentikan Sementara (Maintenance) jika sedang berjalan -->
            <button
              v-if="isServerRunning"
              @click="toggleServer"
              :disabled="isActionLoading"
              class="px-3.5 py-2.5 bg-rose-50 hover:bg-rose-100 text-rose-700 border border-rose-200 font-bold text-xs rounded-xl flex items-center gap-1.5 transition active:scale-95 disabled:opacity-50 cursor-pointer shadow-2xs"
              title="Hentikan server untuk keperluan pemeliharaan/maintenance database"
            >
              <Square class="w-3.5 h-3.5 text-rose-600" />
              <span>Hentikan Sementara</span>
            </button>

            <!-- Jika Belum Teraktivasi Lisensi: Tombol Prioritas adalah Aktivasi Lisensi -->
            <button
              v-else-if="!isLicenseValid"
              @click="showLicenseModal = true"
              class="px-5 py-2.5 bg-purple-600 hover:bg-purple-700 text-white font-bold text-xs rounded-xl flex items-center gap-2 transition active:scale-95 cursor-pointer shadow-xs"
            >
              <Key class="w-4 h-4" />
              <span>Aktivasi Lisensi Sekarang</span>
            </button>

            <!-- Jika Server Berhenti / Standby dan Lisensi Valid: Tombol Nyalakan Server -->
            <button
              v-else
              @click="toggleServer"
              :disabled="isActionLoading"
              class="px-5 py-2.5 bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-xs rounded-xl flex items-center gap-2 transition active:scale-95 disabled:opacity-50 cursor-pointer shadow-xs"
            >
              <Play class="w-4 h-4" />
              <span>Nyalakan Server</span>
            </button>
          </div>
        </div>
      </div>

      <!-- 1B. STORE BRANDING & CUSTOM LOGO SETTINGS -->
      <div class="bg-white rounded-2xl p-6 border border-slate-200 shadow-xs space-y-5">
        <div class="flex flex-col sm:flex-row items-start sm:items-center justify-between pb-4 border-b border-slate-100 gap-3">
          <div class="flex items-center gap-3">
            <div class="w-10 h-10 rounded-xl bg-indigo-50 border border-indigo-200 text-indigo-700 flex items-center justify-center shrink-0">
              <Building2 class="w-5 h-5" />
            </div>
            <div>
              <h3 class="text-sm font-black text-slate-900 tracking-tight flex items-center gap-2">
                <span>Branding & Logo Toko</span>
                <span class="text-[10px] font-mono px-2 py-0.5 rounded-full bg-indigo-50 text-indigo-700 border border-indigo-200 font-bold uppercase">Store Identity</span>
              </h3>
              <p class="text-xs text-slate-500 mt-0.5">
                Kustomisasi nama dan logo toko Anda untuk ditampilkan di layar register kasir POS, Layar Pelanggan (Customer Display), dan Back-Office.
              </p>
            </div>
          </div>

          <div v-if="brandingFeedback" class="bg-emerald-50 text-emerald-800 border border-emerald-200 text-xs px-3 py-1.5 rounded-xl font-bold flex items-center gap-1.5 animate-fadeIn">
            <CheckCircle2 class="w-4 h-4 text-emerald-600" />
            <span>{{ brandingFeedback }}</span>
          </div>
        </div>

        <div class="grid grid-cols-1 lg:grid-cols-12 gap-6 items-start">
          <!-- Logo Upload & Live Preview (Cols 4) -->
          <div class="lg:col-span-4 bg-slate-50/80 rounded-2xl p-4 border border-slate-200/80 flex flex-col items-center text-center space-y-3">
            <span class="text-xs font-bold text-slate-700">Preview Logo Toko</span>
            
            <!-- Logo Preview Frame -->
            <div class="w-28 h-28 rounded-2xl bg-white border-2 border-slate-200 shadow-xs p-2 flex items-center justify-center overflow-hidden relative group">
              <img
                v-if="brandingForm.storeLogoUrl"
                :src="brandingForm.storeLogoUrl"
                alt="Logo Toko"
                class="w-full h-full object-contain"
              />
              <div v-else class="flex flex-col items-center justify-center text-slate-300 space-y-1">
                <Image class="w-8 h-8" />
                <span class="text-[10px] font-medium text-slate-400">Belum Ada Logo</span>
              </div>

              <!-- Overlay tombol hapus jika ada logo -->
              <button
                v-if="brandingForm.storeLogoUrl"
                type="button"
                @click="handleRemoveLogo"
                class="absolute inset-0 bg-slate-900/60 text-white opacity-0 group-hover:opacity-100 transition flex flex-col items-center justify-center gap-1 cursor-pointer"
                title="Hapus Logo"
              >
                <Trash2 class="w-5 h-5 text-rose-300" />
                <span class="text-[10px] font-bold text-rose-200">Hapus Logo</span>
              </button>
            </div>

            <div class="w-full space-y-2">
              <input
                ref="fileInputRef"
                type="file"
                accept="image/png, image/jpeg, image/webp, image/svg+xml"
                class="hidden"
                @change="handleLogoUpload"
              />
              <button
                type="button"
                @click="fileInputRef?.click()"
                class="w-full bg-white hover:bg-indigo-50 hover:text-indigo-700 hover:border-indigo-300 border border-slate-300 text-slate-700 text-xs font-bold py-2 px-3 rounded-xl transition flex items-center justify-center gap-1.5 cursor-pointer shadow-2xs active:scale-95"
              >
                <Upload class="w-3.5 h-3.5" />
                <span>{{ brandingForm.storeLogoUrl ? 'Ganti Logo Toko' : 'Pilih File Logo (PNG/JPG/SVG)' }}</span>
              </button>

              <p class="text-[10px] text-slate-400">
                Format transparan (PNG/SVG) disarankan untuk hasil terbaik.
              </p>
            </div>
          </div>

          <!-- Store Name & Tagline Inputs (Cols 8) -->
          <div class="lg:col-span-8 space-y-4">
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <div>
                <label class="block text-xs font-bold text-slate-700 mb-1.5">Nama Toko / Bisnis Retail</label>
                <input
                  v-model="brandingForm.storeName"
                  type="text"
                  placeholder="Contoh: SUPERMARKET MAKMUR JAYA"
                  class="w-full bg-slate-50 border border-slate-300 rounded-xl px-3.5 py-2 text-xs font-bold text-slate-800 focus:bg-white focus:outline-none focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-600 transition"
                />
              </div>

              <div>
                <label class="block text-xs font-bold text-slate-700 mb-1.5">Slogan / Tagline Toko</label>
                <input
                  v-model="brandingForm.storeTagline"
                  type="text"
                  placeholder="Contoh: Belanja Hemat, Lengkap & Terpercaya"
                  class="w-full bg-slate-50 border border-slate-300 rounded-xl px-3.5 py-2 text-xs text-slate-800 focus:bg-white focus:outline-none focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-600 transition"
                />
              </div>
            </div>

            <!-- Co-Branding or White-Label Info Notice -->
            <div v-if="licenseStore.licenseInfo.features?.includes('remove_branding') || licenseStore.licenseInfo.features?.includes('white_label')" class="bg-purple-50/70 border border-purple-200 rounded-xl p-3 flex items-start gap-3">
              <div class="p-1 rounded bg-purple-100 text-purple-700 mt-0.5 shrink-0">
                <Sparkles class="w-4 h-4" />
              </div>
              <div class="text-xs text-purple-950">
                <span class="font-bold flex items-center gap-1.5">
                  <span>Mode Lisensi White-Label Aktif</span>
                  <span class="bg-purple-200 text-purple-800 text-[10px] font-mono font-bold px-1.5 py-0.5 rounded">100% Bebas Branding</span>
                </span>
                <p class="text-purple-900/80 mt-0.5 text-[11px] leading-relaxed">
                  Fitur <strong>White-Label</strong> diaktifkan pada lisensi ini. Seluruh logo, identitas visual, dan badge "Powered by Notane POS" disembunyikan sepenuhnya dari terminal kasir client, customer display, dan dashboard back-office.
                </p>
              </div>
            </div>
            <div v-else class="bg-indigo-50/60 border border-indigo-200/80 rounded-xl p-3 flex items-start gap-3">
              <div class="p-1 rounded bg-indigo-100 text-indigo-700 mt-0.5 shrink-0">
                <img src="/logo.png" alt="Notane" class="w-4 h-4 object-contain rounded" />
              </div>
              <div class="text-xs text-indigo-950">
                <span class="font-bold">Ketentuan Co-Branding:</span>
                <p class="text-indigo-900/80 mt-0.5 text-[11px] leading-relaxed">
                  Logo dan nama toko Anda akan menjadi sorotan utama di seluruh register kasir, customer display, dan back-office. Badge subtil <strong>"Powered by Notane POS"</strong> tetap disematkan secara proporsional sebagai jaminan mutu dan lisensi sistem.
                </p>
              </div>
            </div>

            <div class="flex items-center justify-end gap-3 pt-2">
              <button
                type="button"
                @click="showInitialSetupModal = true"
                class="bg-indigo-50 hover:bg-indigo-100 text-indigo-700 font-bold text-xs px-4 py-2.5 rounded-xl border border-indigo-200 transition active:scale-95 flex items-center gap-1.5 cursor-pointer shadow-xs"
              >
                <Sparkles class="w-4 h-4 text-indigo-600" />
                <span>Buka Wizard Setup Lengkap</span>
              </button>

              <button
                type="button"
                @click="handleSaveBranding"
                :disabled="isSavingBranding || !brandingForm.storeName"
                class="bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 text-white font-bold text-xs px-5 py-2.5 rounded-xl transition active:scale-95 flex items-center gap-2 cursor-pointer shadow-xs"
              >
                <Check v-if="!isSavingBranding" class="w-4 h-4" />
                <RefreshCw v-else class="w-4 h-4 animate-spin" />
                <span>{{ isSavingBranding ? 'Menyimpan...' : 'Simpan & Terapkan Branding Toko' }}</span>
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- 2. NETWORK & CONNECTION INFO GRID (3 Cards) -->
      <div class="grid grid-cols-1 md:grid-cols-3 gap-5">
        <!-- Card 1: IP Jaringan LAN -->
        <div class="bg-white p-5 rounded-2xl border border-slate-200 space-y-3 shadow-xs">
          <div class="flex items-center justify-between text-slate-500 text-xs">
            <span class="font-bold flex items-center gap-1.5 uppercase tracking-wider text-slate-700">
              <Wifi class="w-4 h-4 text-sky-600" />
              Alamat IP Host LAN
            </span>
            <span class="text-[11px] font-mono text-slate-400">Port {{ displayPort }}</span>
          </div>

          <div class="bg-slate-50 border border-slate-200 rounded-xl p-3 flex items-center justify-between">
            <span class="font-mono font-extrabold text-sm text-sky-700 truncate">
              http://{{ primaryIp }}:{{ displayPort }}
            </span>
            <button
              @click="copyHostIp"
              class="p-1.5 hover:bg-slate-200 text-slate-500 hover:text-slate-800 rounded-lg transition cursor-pointer shrink-0 ml-2"
              title="Salin Alamat IP"
            >
              <Check v-if="copiedIp" class="w-4 h-4 text-emerald-600" />
              <Copy v-else class="w-4 h-4" />
            </button>
          </div>

          <!-- Multiple Adapters Selector jika ada lebih dari 1 IP -->
          <div v-if="detectedLanIps.length > 1" class="pt-1">
            <label class="block text-[10px] font-bold text-slate-500 mb-1">Pilih Adapter Jaringan:</label>
            <select
              v-model="selectedIp"
              class="w-full text-xs font-mono bg-slate-50 border border-slate-200 rounded-lg p-1.5 text-slate-800 focus:outline-none focus:border-indigo-500"
            >
              <option v-for="ip in detectedLanIps" :key="ip" :value="ip">
                {{ ip }} (LAN Adapter)
              </option>
            </select>
          </div>

          <p class="text-[11px] text-slate-500 leading-relaxed">
            <template v-if="detectedLanIps.length > 0">
              Ketikkan alamat ini pada aplikasi <strong>Notane POS Client</strong> di komputer meja kasir.
            </template>
            <template v-else>
              ⚠️ Komputer belum terhubung ke LAN/Wi-Fi. Sambungkan Wi-Fi atau kabel LAN router agar komputer kasir lain dapat terhubung.
            </template>
          </p>
        </div>

        <!-- Card 2: Pairing Code -->
        <div class="bg-white p-5 rounded-2xl border border-slate-200 space-y-3 shadow-xs">
          <div class="flex items-center justify-between text-slate-500 text-xs">
            <span class="font-bold flex items-center gap-1.5 uppercase tracking-wider text-slate-700">
              <ShieldCheck class="w-4 h-4 text-amber-600" />
              Kode Pairing Toko
            </span>
            <span class="text-[11px] font-mono text-slate-400">Keamanan LAN</span>
          </div>

          <div class="bg-slate-50 border border-slate-200 rounded-xl p-3 flex items-center justify-between">
            <span class="font-mono font-black text-base text-amber-700 tracking-wider">
              {{ networkStore.hostStatus.pairing_code || 'NOTANE-8899' }}
            </span>
            <div class="flex items-center gap-1.5">
              <button
                @click="openPairingModal"
                class="px-2.5 py-1 bg-amber-50 hover:bg-amber-100 text-amber-800 border border-amber-300 rounded-lg text-xs font-bold transition flex items-center gap-1 cursor-pointer active:scale-95 shadow-2xs"
                title="Ganti Kode Pairing Toko"
              >
                <KeyRound class="w-3.5 h-3.5 text-amber-700" />
                <span>Ganti</span>
              </button>
              <button
                @click="copyPairingCode"
                class="p-1.5 hover:bg-slate-200 text-slate-500 hover:text-slate-800 rounded-lg transition cursor-pointer"
                title="Salin Kode Pairing"
              >
                <Check v-if="copiedCode" class="w-4 h-4 text-emerald-600" />
                <Copy v-else class="w-4 h-4" />
              </button>
            </div>
          </div>
          <p class="text-[11px] text-slate-500 leading-relaxed">
            Kode otentikasi wajib saat komputer kasir baru pertama kali menghubungkan diri ke server.
          </p>
        </div>

        <!-- Card 3: Database PostgreSQL -->
        <div class="bg-white p-5 rounded-2xl border border-slate-200 space-y-3 shadow-xs">
          <div class="flex items-center justify-between text-slate-500 text-xs">
            <span class="font-bold flex items-center gap-1.5 uppercase tracking-wider text-slate-700">
              <Database class="w-4 h-4 text-indigo-600" />
              Mesin Database
            </span>
            <span class="text-[11px] font-mono text-emerald-700 font-bold">PORT 15432</span>
          </div>

          <div class="bg-slate-50 border border-slate-200 rounded-xl p-3 flex items-center justify-between">
            <span class="font-mono text-xs font-bold text-slate-800">
              PostgreSQL Terisolasi
            </span>
            <span class="w-2.5 h-2.5 rounded-full bg-emerald-500 animate-pulse"></span>
          </div>
          <p class="text-[11px] text-slate-500 leading-relaxed">
            Database terisolasi berjalan mandiri tanpa mengganggu software atau aplikasi lain di komputer ini.
          </p>
        </div>
      </div>

      <!-- 3. SISTEM LISENSI & HARDWARE ID HOST (Khusus Server Host) -->
      <div class="bg-white rounded-2xl border border-slate-200 p-6 shadow-xs space-y-5">
        <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 border-b border-slate-200 pb-4">
          <div class="flex items-center gap-3">
            <div class="w-10 h-10 rounded-xl bg-purple-100 text-purple-700 flex items-center justify-center font-bold">
              <Key class="w-5 h-5" />
            </div>
            <div>
              <div class="flex items-center gap-2">
                <h3 class="text-sm font-black text-slate-900 uppercase tracking-tight">
                  Lisensi Server & Hardware ID Mesin
                </h3>
                <span
                  class="font-black px-2 py-0.5 rounded-full text-[10px] border flex items-center gap-1 inline-flex"
                  :class="{
                    'bg-emerald-50 text-emerald-800 border-emerald-300': isLicenseActive,
                    'bg-amber-50 text-amber-800 border-amber-300': isGracePeriod,
                    'bg-rose-50 text-rose-800 border-rose-300': !isLicenseActive && !isGracePeriod,
                  }"
                >
                  <CheckCircle2 v-if="isLicenseActive" class="w-3 h-3 text-emerald-600" />
                  <AlertTriangle v-else-if="isGracePeriod" class="w-3 h-3 text-amber-600" />
                  <XCircle v-else class="w-3 h-3 text-rose-600" />
                  <span>{{ licenseStore.licenseInfo.status }}</span>
                </span>
              </div>
              <p class="text-xs text-slate-500">
                Proteksi Asimetris Kriptografi Ed25519 & Node-Locking Server Notane Retail
              </p>
            </div>
          </div>

          <div class="flex items-center gap-2">
            <button
              type="button"
              @click="showLicenseModal = true"
              class="px-4 py-2 bg-indigo-600 hover:bg-indigo-700 text-white font-bold text-xs rounded-xl flex items-center gap-1.5 transition active:scale-95 shadow-xs cursor-pointer"
            >
              <Key class="w-3.5 h-3.5" />
              <span>Aktivasi / Perbarui Lisensi</span>
            </button>
          </div>
        </div>

        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
          <!-- Hardware Fingerprint Box -->
          <div class="p-4 rounded-xl border border-slate-200 bg-slate-50/80 space-y-2">
            <div class="flex items-center justify-between text-xs">
              <span class="font-bold text-slate-700 flex items-center gap-1.5 uppercase tracking-wider text-[11px]">
                <Cpu class="w-4 h-4 text-indigo-600" />
                Hardware ID (HWID Mesin Server)
              </span>
              <button
                type="button"
                @click="copyHwid"
                class="px-2.5 py-1 bg-white hover:bg-slate-100 border border-slate-200 rounded-lg text-[11px] font-bold text-slate-700 transition flex items-center gap-1 cursor-pointer shadow-2xs"
              >
                <Check v-if="copiedHwid" class="w-3 h-3 text-emerald-600" />
                <Copy v-else class="w-3 h-3" />
                <span>{{ copiedHwid ? 'Tersalin' : 'Salin HWID' }}</span>
              </button>
            </div>
            <div class="font-mono font-bold text-xs text-slate-900 bg-white p-2.5 rounded-lg border border-slate-200 break-all select-all">
              {{ licenseStore.licenseInfo.current_hardware_id || 'Memuat HWID...' }}
            </div>
            <p class="text-[11px] text-slate-500">
              Salin kode HWID ini dan serahkan kepada Provider Notane untuk penerbitan token lisensi resmi.
            </p>
          </div>

          <!-- Rincian Paket Lisensi -->
          <div class="p-4 rounded-xl border border-slate-200 bg-slate-50/80 space-y-2 text-xs">
            <div class="font-bold text-slate-700 uppercase tracking-wider text-[11px] flex items-center gap-1.5">
              <ShieldCheck class="w-4 h-4 text-purple-600" />
              Detail Lisensi Server
            </div>
            <div class="bg-white p-2.5 rounded-lg border border-slate-200 space-y-1">
              <div class="flex justify-between">
                <span class="text-slate-500">Nama Pelanggan / Toko:</span>
                <span class="font-bold text-slate-900">{{ licenseStore.licenseInfo.client_name || 'Belum Terdaftar' }}</span>
              </div>
              <div class="flex justify-between">
                <span class="text-slate-500">Paket Lisensi:</span>
                <span class="font-bold text-indigo-700 font-mono">{{ licenseStore.licenseInfo.tier || 'Standard' }}</span>
              </div>
              <div class="flex justify-between">
                <span class="text-slate-500">Batas Terminal Kasir:</span>
                <span class="font-bold text-slate-900 font-mono">{{ licenseStore.licenseInfo.max_terminals }} Kasir</span>
              </div>
              <div class="flex justify-between" v-if="licenseStore.licenseInfo.expires_at">
                <span class="text-slate-500">Masa Berlaku:</span>
                <span class="font-bold text-slate-900">
                  {{ new Date(licenseStore.licenseInfo.expires_at).toLocaleDateString('id-ID', { year: 'numeric', month: 'long', day: 'numeric' }) }}
                </span>
              </div>
            </div>
            <p class="text-[11px] text-slate-500">
              {{ licenseStore.licenseInfo.message }}
            </p>
          </div>
        </div>
      </div>

      <!-- 4. CONNECTED CLIENTS TERMINAL LIST -->
      <div class="bg-white rounded-2xl border border-slate-200 overflow-hidden shadow-xs">
        <div class="px-6 py-4 border-b border-slate-200 flex items-center justify-between">
          <div class="flex items-center gap-2">
            <Users class="w-4 h-4 text-indigo-600" />
            <h3 class="text-xs font-bold text-slate-900 uppercase tracking-wider">
              Terminal Kasir yang Terhubung (Online Clients)
            </h3>
          </div>
          <span class="text-xs font-bold font-mono px-2.5 py-0.5 rounded-full bg-slate-100 text-slate-700 border border-slate-200">
            {{ networkStore.connectedDevices.length }} Perangkat Aktif
          </span>
        </div>

        <!-- Empty State -->
        <div v-if="networkStore.connectedDevices.length === 0" class="p-12 text-center text-slate-400">
          <Laptop class="w-12 h-12 mx-auto mb-3 text-slate-300" />
          <h4 class="text-sm font-bold text-slate-700">Belum Ada Terminal Kasir yang Terhubung</h4>
          <p class="text-xs text-slate-500 mt-1 max-w-md mx-auto">
            Buka aplikasi Notane POS Client di komputer meja kasir, lalu sambungkan ke IP <strong>{{ primaryIp }}</strong> (Port <strong>{{ displayPort }}</strong>).
          </p>
        </div>

        <!-- Table Connected Clients -->
        <div v-else class="overflow-x-auto">
          <table class="w-full text-left text-xs border-collapse">
            <thead class="bg-slate-50 text-slate-600 font-mono uppercase text-[11px] border-b border-slate-200">
              <tr>
                <th class="py-3 px-4">Nama Terminal</th>
                <th class="py-3 px-4">Tipe Perangkat</th>
                <th class="py-3 px-4">Alamat IP Client</th>
                <th class="py-3 px-4">Waktu Terhubung</th>
                <th class="py-3 px-4 text-center">Status</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-100 font-medium">
              <tr v-for="cl in networkStore.connectedDevices" :key="cl.device_id" class="hover:bg-slate-50/70">
                <td class="py-3 px-4 font-bold text-slate-900 flex items-center gap-2">
                  <Laptop class="w-3.5 h-3.5 text-indigo-600" />
                  <span>{{ cl.device_name }}</span>
                </td>
                <td class="py-3 px-4 text-slate-600 font-mono">{{ cl.device_type }}</td>
                <td class="py-3 px-4 text-sky-700 font-mono font-bold">{{ cl.client_ip }}</td>
                <td class="py-3 px-4 text-slate-500">{{ new Date(cl.connected_at).toLocaleTimeString('id-ID') }}</td>
                <td class="py-3 px-4 text-center">
                  <span class="px-2 py-0.5 rounded text-[10px] font-bold bg-emerald-50 text-emerald-700 border border-emerald-200 font-mono">
                    {{ cl.status || 'ONLINE' }}
                  </span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </main>

    <!-- Footer Status Bar -->
    <footer class="bg-white border-t border-slate-200 px-6 py-2.5 text-[11px] text-slate-500 flex items-center justify-between">
      <div class="flex items-center gap-4">
        <span>Versi Host: <strong class="text-slate-700">v{{ licenseStore.versionInfo.version }} Enterprise</strong></span>
        <span>Uptime: <strong class="font-mono text-slate-700">{{ networkStore.hostStatus.uptime_seconds || 0 }} detik</strong></span>
      </div>
      <div>
        <span>Notane Retail &copy; 2026. Mode Host Server Aktif.</span>
      </div>
    </footer>

    <!-- MODAL GANTI KODE PAIRING TOKO -->
    <div
      v-if="showPairingModal"
      class="fixed inset-0 z-50 bg-slate-900/60 backdrop-blur-xs flex items-center justify-center p-4 animate-fadeIn"
      @click.self="showPairingModal = false"
    >
      <div class="bg-white rounded-3xl max-w-md w-full shadow-2xl border border-slate-200 overflow-hidden transform transition-all">
        <!-- Header Modal -->
        <div class="px-6 py-4.5 bg-gradient-to-r from-amber-50 to-orange-50/60 border-b border-amber-200 flex items-center justify-between">
          <div class="flex items-center gap-3">
            <div class="w-10 h-10 rounded-2xl bg-amber-100 border border-amber-300 text-amber-800 flex items-center justify-center shadow-xs">
              <KeyRound class="w-5 h-5" />
            </div>
            <div>
              <h3 class="text-sm font-black text-slate-900 tracking-tight">Ganti Kode Pairing Toko</h3>
              <p class="text-xs text-slate-500">Otentikasi Keamanan Terminal Kasir LAN</p>
            </div>
          </div>
          <button
            @click="showPairingModal = false"
            class="p-1.5 rounded-xl hover:bg-white/80 text-slate-400 hover:text-slate-700 transition cursor-pointer"
          >
            <X class="w-5 h-5" />
          </button>
        </div>

        <!-- Body Modal -->
        <div class="p-6 space-y-4">
          <div>
            <label class="block text-xs font-bold text-slate-700 mb-1.5">
              Kode Pairing Baru
            </label>
            <div class="flex items-center gap-2">
              <div class="relative flex-1">
                <input
                  v-model="newPairingInput"
                  type="text"
                  placeholder="Contoh: NOTANE-8899 atau TOKO-BARU"
                  class="w-full bg-slate-50 border border-slate-300 rounded-xl px-3.5 py-2.5 text-sm font-mono font-black text-slate-900 uppercase tracking-wider focus:bg-white focus:outline-none focus:ring-2 focus:ring-amber-500/20 focus:border-amber-600 transition"
                  maxlength="32"
                  @input="newPairingInput = newPairingInput.toUpperCase()"
                />
              </div>
              <button
                type="button"
                @click="handleRegeneratePairing"
                :disabled="isPairingSaving"
                class="px-3 py-2.5 bg-slate-100 hover:bg-amber-100 text-slate-700 hover:text-amber-900 border border-slate-300 hover:border-amber-300 rounded-xl text-xs font-bold transition flex items-center gap-1.5 cursor-pointer shadow-2xs active:scale-95 disabled:opacity-50"
                title="Acak kode secara otomatis (NOTANE-XXXX)"
              >
                <Shuffle class="w-3.5 h-3.5" :class="{ 'animate-spin': isPairingSaving }" />
                <span>Acak</span>
              </button>
            </div>
            <p class="text-[11px] text-slate-500 mt-1.5">
              Gunakan huruf besar, angka, atau tanda minus (-). Minimal 4 dan maksimal 32 karakter.
            </p>
          </div>

          <!-- Alert / Status -->
          <div v-if="pairingError" class="p-3 bg-rose-50 border border-rose-200 text-rose-800 rounded-xl text-xs flex items-center gap-2 animate-shake">
            <AlertTriangle class="w-4 h-4 shrink-0 text-rose-600" />
            <span>{{ pairingError }}</span>
          </div>

          <div v-if="pairingSuccess" class="p-3 bg-emerald-50 border border-emerald-200 text-emerald-800 rounded-xl text-xs flex items-center gap-2">
            <CheckCircle2 class="w-4 h-4 shrink-0 text-emerald-600" />
            <span>{{ pairingSuccess }}</span>
          </div>

          <div class="bg-amber-50/70 border border-amber-200/80 rounded-2xl p-3.5 text-xs text-amber-900 leading-relaxed">
            <span class="font-bold flex items-center gap-1 mb-1">
              <ShieldCheck class="w-4 h-4 text-amber-700" />
              Pemberitahuan Keamanan:
            </span>
            Setelah kode pairing diganti, komputer kasir yang baru akan memerlukan kode baru ini untuk pertama kali terhubung ke host server.
          </div>
        </div>

        <!-- Footer Modal -->
        <div class="px-6 py-4 bg-slate-50 border-t border-slate-200 flex items-center justify-end gap-3">
          <button
            type="button"
            @click="showPairingModal = false"
            class="px-4 py-2 rounded-xl text-xs font-bold text-slate-600 hover:text-slate-900 hover:bg-slate-200 transition cursor-pointer"
          >
            Batal
          </button>
          <button
            type="button"
            @click="handleSaveCustomPairing"
            :disabled="isPairingSaving || !newPairingInput"
            class="px-5 py-2 rounded-xl text-xs font-bold bg-amber-600 hover:bg-amber-700 text-white shadow-xs transition active:scale-95 disabled:opacity-50 cursor-pointer flex items-center gap-1.5"
          >
            <Check v-if="!isPairingSaving" class="w-4 h-4" />
            <RefreshCw v-else class="w-4 h-4 animate-spin" />
            <span>{{ isPairingSaving ? 'Menyimpan...' : 'Simpan Kode Baru' }}</span>
          </button>
        </div>
      </div>
    </div>

    <!-- License & Update Modal (Dedicated in Host App) -->
    <LicenseUpdateModal
      :is-open="showLicenseModal"
      :initial-tab="licenseModalTab"
      @close="showLicenseModal = false"
    />

    <!-- Initial Setup Wizard Modal (First-Run & Store Setup) -->
    <InitialSetupWizardModal
      v-model="showInitialSetupModal"
      @completed="handleWizardCompleted"
    />
  </div>
</template>
