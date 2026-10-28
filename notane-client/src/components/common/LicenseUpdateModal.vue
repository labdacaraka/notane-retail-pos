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
  Wifi,
  WifiOff,
  History,
  Clock,
} from 'lucide-vue-next'
import { useLicenseStore } from '@/stores/licenseStore'

defineProps<{
  isOpen: boolean
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

const licenseStore = useLicenseStore()
const activeTab = ref<'LICENSE' | 'UPDATE'>('LICENSE')
const showSyncLogs = ref(false)

// Form Aktivasi
const tokenInput = ref('')
const signatureInput = ref('')
const copiedHwid = ref(false)

onMounted(async () => {
  await licenseStore.fetchVersionInfo()
  await licenseStore.fetchLicenseInfo()
  await licenseStore.fetchSyncLogs()
})

const isLicenseActive = computed(() => licenseStore.licenseInfo.status === 'ACTIVE')
const isGracePeriod = computed(() => licenseStore.licenseInfo.status === 'GRACE_PERIOD')
const isExpired = computed(() => licenseStore.licenseInfo.status === 'EXPIRED')
const isHwidMismatch = computed(() => licenseStore.licenseInfo.status === 'HARDWARE_MISMATCH')
const isClockTampered = computed(() => licenseStore.licenseInfo.status === 'CLOCK_TAMPERED')

async function handleManualSync() {
  await licenseStore.syncLicenseWithProvider()
}

function copyHwid() {
  navigator.clipboard.writeText(licenseStore.licenseInfo.current_hardware_id)
  copiedHwid.value = true
  setTimeout(() => { copiedHwid.value = false }, 2000)
}

async function handleActivate() {
  if (!tokenInput.value || !signatureInput.value) {
    alert('Mohon isi Token Lisensi (JSON) dan Signature Kriptografi (Hex) dari Provider.')
    return
  }
  const ok = await licenseStore.activateLicense(tokenInput.value, signatureInput.value)
  if (ok) {
    tokenInput.value = ''
    signatureInput.value = ''
  }
}

async function handleCheckUpdate() {
  await licenseStore.checkForUpdates()
}

async function handleApplyUpdate() {
  const ok = await licenseStore.applyOneClickUpdate()
  if (ok) {
    alert('Pembaruan sistem berhasil disiapkan! Aplikasi akan dimuat ulang.')
    window.location.reload()
  }
}
</script>

<template>
  <div v-if="isOpen" class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/60 backdrop-blur-xs">
    <div class="bg-white rounded-2xl shadow-2xl border border-slate-200 w-full max-w-2xl overflow-hidden flex flex-col max-h-[90vh] animate-in fade-in zoom-in-95 duration-150">
      
      <!-- Modal Header -->
      <div class="px-6 py-4 bg-gradient-to-r from-slate-900 via-slate-800 to-indigo-950 text-white flex items-center justify-between">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-xl bg-white/10 flex items-center justify-center backdrop-blur-md border border-white/20">
            <ShieldCheck class="w-5 h-5 text-emerald-400" />
          </div>
          <div>
            <h3 class="font-extrabold text-base tracking-tight flex items-center gap-2">
              <span>Pusat Lisensi & Pembaruan Sistem</span>
              <span class="text-[10px] bg-emerald-500/20 text-emerald-300 border border-emerald-500/30 px-2 py-0.5 rounded-full font-mono">
                v{{ licenseStore.versionInfo.version }}
              </span>
            </h3>
            <p class="text-xs text-slate-300">
              Informasi lisensi toko dan status pembaruan
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
            ? 'border-indigo-600 text-indigo-700 bg-white rounded-t-lg'
            : 'border-transparent text-slate-500 hover:text-slate-700'"
        >
          <Key class="w-3.5 h-3.5" />
          <span>Status & Aktivasi Lisensi</span>
        </button>

        <button
          type="button"
          @click="activeTab = 'UPDATE'"
          class="px-4 py-2.5 text-xs font-bold border-b-2 transition flex items-center gap-2 cursor-pointer"
          :class="activeTab === 'UPDATE'
            ? 'border-indigo-600 text-indigo-700 bg-white rounded-t-lg'
            : 'border-transparent text-slate-500 hover:text-slate-700'"
        >
          <DownloadCloud class="w-3.5 h-3.5" />
          <span>Pembaruan Sistem (One-Click)</span>
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
                <div class="text-[11px] font-bold text-slate-500 uppercase tracking-wider">Hardware ID Mesin (Node-Locked)</div>
                <div class="font-mono font-extrabold text-sm text-slate-900 tracking-wider">
                  {{ licenseStore.licenseInfo.current_hardware_id }}
                </div>
                <div class="text-[10px] text-slate-500">Berikan kode ini ke Provider untuk menerbitkan lisensi mesin ini</div>
              </div>
            </div>

            <button
              type="button"
              @click="copyHwid"
              class="px-3 py-1.5 bg-white hover:bg-slate-100 border border-slate-300 rounded-lg text-xs font-bold text-slate-700 transition flex items-center gap-1.5 cursor-pointer shadow-2xs"
            >
              <Check v-if="copiedHwid" class="w-3.5 h-3.5 text-emerald-600" />
              <Copy v-else class="w-3.5 h-3.5" />
              <span>{{ copiedHwid ? 'Tersalin' : 'Salin HWID' }}</span>
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
                  <span>{{ licenseStore.licenseInfo.status }}</span>
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
                Paket: <span class="font-bold text-indigo-700">{{ licenseStore.licenseInfo.tier || '-' }}</span> | Max: {{ licenseStore.licenseInfo.max_terminals }} Terminal
              </div>
              <div v-if="licenseStore.licenseInfo.expires_at" class="text-[10px] text-slate-500 pt-0.5">
                Kedaluwarsa: {{ new Date(licenseStore.licenseInfo.expires_at).toLocaleDateString('id-ID', { year: 'numeric', month: 'long', day: 'numeric' }) }}
              </div>
            </div>
          </div>

          <!-- Provider Connection & Asynchronous Sync Card -->
          <div class="p-3.5 rounded-xl border border-slate-200 bg-slate-50 space-y-3">
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-3">
                <div
                  class="w-8 h-8 rounded-lg flex items-center justify-center text-xs font-bold shrink-0"
                  :class="licenseStore.licenseInfo.sync_status === 'PROVIDER_UNREACHABLE'
                    ? 'bg-amber-100 text-amber-700'
                    : 'bg-emerald-100 text-emerald-700'"
                >
                  <WifiOff v-if="licenseStore.licenseInfo.sync_status === 'PROVIDER_UNREACHABLE'" class="w-4 h-4" />
                  <Wifi v-else class="w-4 h-4" />
                </div>
                <div>
                  <div class="text-[11px] font-bold text-slate-800 flex items-center gap-2">
                    <span>Sinkronisasi Provider Lisensi</span>
                    <span
                      class="text-[9px] px-2 py-0.5 rounded-full font-bold uppercase tracking-wider"
                      :class="licenseStore.licenseInfo.sync_status === 'PROVIDER_UNREACHABLE'
                        ? 'bg-amber-100 text-amber-800 border border-amber-300'
                        : 'bg-emerald-100 text-emerald-800 border border-emerald-300'"
                    >
                      {{ licenseStore.licenseInfo.sync_status === 'PROVIDER_UNREACHABLE' ? 'Mode Offline Aman' : 'Terverifikasi Online' }}
                    </span>
                  </div>
                  <div class="text-[10px] text-slate-500 font-mono">
                    {{ licenseStore.licenseInfo.provider_server_url }}
                  </div>
                </div>
              </div>

              <div class="flex items-center gap-2">
                <button
                  type="button"
                  @click="showSyncLogs = !showSyncLogs"
                  class="px-2.5 py-1.5 bg-white hover:bg-slate-100 border border-slate-300 rounded-lg text-[11px] font-semibold text-slate-700 transition flex items-center gap-1.5 cursor-pointer shadow-2xs"
                >
                  <History class="w-3.5 h-3.5 text-slate-500" />
                  <span>{{ showSyncLogs ? 'Tutup Riwayat' : 'Riwayat Cek' }}</span>
                </button>

                <button
                  type="button"
                  @click="handleManualSync"
                  :disabled="licenseStore.isSyncing"
                  class="px-3 py-1.5 bg-indigo-600 hover:bg-indigo-700 text-white rounded-lg text-[11px] font-bold transition flex items-center gap-1.5 cursor-pointer shadow-2xs disabled:opacity-50"
                >
                  <RefreshCw class="w-3.5 h-3.5" :class="{ 'animate-spin': licenseStore.isSyncing }" />
                  <span>{{ licenseStore.isSyncing ? 'Menghubungkan...' : 'Sinkronkan Sekarang' }}</span>
                </button>
              </div>
            </div>

            <!-- Sync Info Description -->
            <div class="grid grid-cols-2 gap-2 text-[10px] text-slate-600 bg-white p-2.5 rounded-lg border border-slate-200">
              <div>
                <span class="text-slate-400 block">Pemeriksaan Terakhir:</span>
                <span class="font-medium text-slate-800">
                  {{ licenseStore.licenseInfo.last_sync_at ? new Date(licenseStore.licenseInfo.last_sync_at).toLocaleString('id-ID') : 'Saat Aktivasi Awal' }}
                </span>
              </div>
              <div>
                <span class="text-slate-400 block">Status Toleransi Offline:</span>
                <span
                  class="font-medium"
                  :class="licenseStore.licenseInfo.sync_status === 'PROVIDER_UNREACHABLE' ? 'text-amber-700 font-bold' : 'text-emerald-700'"
                >
                  {{ licenseStore.licenseInfo.offline_grace_days_remaining !== null && licenseStore.licenseInfo.offline_grace_days_remaining !== undefined
                    ? `Maksimal toleransi: ${licenseStore.licenseInfo.offline_grace_days_remaining} hari`
                    : 'Penuh (Offline Resilient)' }}
                </span>
              </div>
            </div>

            <!-- Riwayat Sync Logs Table (Collapsible) -->
            <div v-if="showSyncLogs" class="space-y-2 pt-1">
              <div class="text-[10px] font-bold text-slate-700 uppercase tracking-wider flex items-center gap-1">
                <Clock class="w-3 h-3 text-slate-500" />
                <span>Log Riwayat Pemeriksaan Server Lisensi (Background & Manual)</span>
              </div>

              <div class="max-h-36 overflow-y-auto border border-slate-200 rounded-lg bg-white">
                <table class="w-full text-[10px] text-left">
                  <thead class="bg-slate-50 text-slate-500 border-b border-slate-200 sticky top-0">
                    <tr>
                      <th class="p-1.5 font-bold">Waktu</th>
                      <th class="p-1.5 font-bold">Tipe</th>
                      <th class="p-1.5 font-bold">Status</th>
                      <th class="p-1.5 font-bold">Latensi</th>
                      <th class="p-1.5 font-bold">Pesan / Respons</th>
                    </tr>
                  </thead>
                  <tbody class="divide-y divide-slate-100 font-sans">
                    <tr v-if="licenseStore.syncLogs.length === 0">
                      <td colspan="5" class="p-2 text-center text-slate-400">Belum ada log sinkronisasi tercatat</td>
                    </tr>
                    <tr v-for="log in licenseStore.syncLogs" :key="log.id" class="hover:bg-slate-50/50">
                      <td class="p-1.5 text-slate-600 whitespace-nowrap">{{ new Date(log.created_at).toLocaleTimeString('id-ID') }}</td>
                      <td class="p-1.5 font-mono text-[9px] text-slate-500">{{ log.sync_type }}</td>
                      <td class="p-1.5">
                        <span
                          class="px-1.5 py-0.5 rounded text-[9px] font-bold"
                          :class="{
                            'bg-emerald-100 text-emerald-800': log.status === 'SUCCESS',
                            'bg-amber-100 text-amber-800': log.status === 'UNREACHABLE',
                            'bg-rose-100 text-rose-800': log.status === 'FAILED' || log.status === 'REVOKED'
                          }"
                        >
                          {{ log.status }}
                        </span>
                      </td>
                      <td class="p-1.5 font-mono text-slate-600">{{ log.latency_ms ? `${log.latency_ms}ms` : '-' }}</td>
                      <td class="p-1.5 text-slate-600 truncate max-w-[200px]" :title="log.message || ''">
                        {{ log.message || '-' }}
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>
          </div>

          <!-- Form Aktivasi Baru -->
          <div class="border border-slate-200 rounded-xl p-4 bg-slate-50 space-y-3">
            <div class="flex items-center gap-2 text-xs font-bold text-slate-800">
              <Lock class="w-4 h-4 text-indigo-600" />
              <span>Aktivasi / Perbarui Sertifikat Lisensi</span>
            </div>
            <p class="text-[11px] text-slate-500">
              Tempelkan Payload Token dan Signature kriptografi Ed25519 yang diberikan oleh Provider Notane Retail POS:
            </p>

            <div class="space-y-2">
              <div>
                <label class="block text-[10px] font-bold text-slate-600 mb-1">Payload Token Lisensi (JSON)</label>
                <textarea
                  v-model="tokenInput"
                  rows="2"
                  placeholder='{"license_key":"NOTANE-ENT-...","client_name":"...","hardware_id":"..."}'
                  class="w-full text-xs font-mono p-2 bg-white border border-slate-300 rounded-lg focus:outline-none focus:border-indigo-600"
                />
              </div>

              <div>
                <label class="block text-[10px] font-bold text-slate-600 mb-1">Signature Kriptografi (Ed25519 Hex)</label>
                <input
                  v-model="signatureInput"
                  type="text"
                  placeholder="Contoh: 753f3b3350bca2a8c5ac8cfcb8005210160013fc3a389de3225411..."
                  class="w-full text-xs font-mono p-2 bg-white border border-slate-300 rounded-lg focus:outline-none focus:border-indigo-600"
                />
              </div>

              <div class="pt-1 flex justify-end">
                <button
                  type="button"
                  @click="handleActivate"
                  :disabled="licenseStore.isLoading"
                  class="px-4 py-2 bg-indigo-600 hover:bg-indigo-700 text-white rounded-lg text-xs font-bold transition flex items-center gap-2 cursor-pointer shadow-xs disabled:opacity-50"
                >
                  <RefreshCw v-if="licenseStore.isLoading" class="w-3.5 h-3.5 animate-spin" />
                  <Key v-else class="w-3.5 h-3.5" />
                  <span>{{ licenseStore.isLoading ? 'Memvalidasi...' : 'Verifikasi & Simpan Lisensi' }}</span>
                </button>
              </div>
            </div>
          </div>
        </div>

        <!-- ============================================================= -->
        <!-- TAB 2: PEMBARUAN SISTEM (ONE-CLICK UPDATER) -->
        <!-- ============================================================= -->
        <div v-else-if="activeTab === 'UPDATE'" class="space-y-4">
          
          <!-- Current Version Card -->
          <div class="p-4 rounded-xl border border-slate-200 bg-slate-50 flex items-center justify-between">
            <div class="flex items-center gap-3">
              <div class="w-10 h-10 rounded-xl bg-emerald-100 text-emerald-700 flex items-center justify-center font-bold text-sm">
                POS
              </div>
              <div>
                <h4 class="font-bold text-slate-900 text-sm">Notane Retail POS</h4>
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
                  Pembaruan aplikasi kasir akan diterapkan secara aman dan otomatis.
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
              <h4 class="font-bold text-slate-800 text-sm">Aplikasi Anda Sudah Menggunakan Versi Terbaru</h4>
              <p class="text-xs text-slate-500 max-w-md mx-auto">
                {{ licenseStore.updateInfo.message }}
              </p>
            </div>
          </div>
          
          <div v-else class="p-6 rounded-xl border border-dashed border-slate-300 text-center text-slate-500 text-xs">
            Klik tombol <strong>"Periksa Update"</strong> untuk memeriksa apakah tersedia rilis versi terbaru dari Provider Notane.
          </div>
        </div>

      </div>

      <!-- Modal Footer -->
      <div class="px-6 py-3 bg-slate-100 border-t border-slate-200 flex items-center justify-between text-xs text-slate-500">
        <div class="flex items-center gap-1.5 text-[11px]">
          <Layers class="w-3.5 h-3.5 text-slate-400" />
          <span>Sistem Kasir Notane POS</span>
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
