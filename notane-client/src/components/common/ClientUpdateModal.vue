<script setup lang="ts">
import { ref, watch, onMounted, computed } from 'vue'
import {
  DownloadCloud,
  RefreshCw,
  Sparkles,
  CheckCircle2,
  AlertTriangle,
  X,
  Server,
  Monitor,
  Calendar,
  Layers,
} from 'lucide-vue-next'
import { useLicenseStore } from '@/stores/licenseStore'
import { useNetworkStore } from '@/stores/networkStore'
import { useDialog } from '@/composables/useDialog'

const props = defineProps<{
  isOpen: boolean
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

const licenseStore = useLicenseStore()
const networkStore = useNetworkStore()
const dialog = useDialog()

const isChecking = ref(false)
const isUpdating = ref(false)
const updateSuccess = ref(false)

onMounted(async () => {
  if (networkStore.connectionStatus === 'CONNECTED') {
    await licenseStore.fetchVersionInfo()
  }
})

watch(
  () => props.isOpen,
  async (open) => {
    if (open) {
      updateSuccess.value = false
      if (networkStore.connectionStatus === 'CONNECTED') {
        await licenseStore.fetchVersionInfo()
        await handleCheckUpdate()
      }
    }
  }
)

async function handleCheckUpdate() {
  isChecking.value = true
  try {
    await licenseStore.checkForUpdates('client')
  } finally {
    isChecking.value = false
  }
}

async function handleApplyUpdate() {
  if (!licenseStore.updateInfo?.can_update) return
  isUpdating.value = true
  try {
    const res = await licenseStore.applyClientUiUpdate()
    if (res.success) {
      updateSuccess.value = true
      dialog.toast(res.message, 'success')
      setTimeout(() => {
        window.location.reload()
      }, 1500)
    } else {
      dialog.alert(res.message, 'danger')
    }
  } finally {
    isUpdating.value = false
  }
}

const isServerCompatible = computed(() => {
  if (!licenseStore.updateInfo) return true
  return licenseStore.updateInfo.server_compatible !== false
})
</script>

<template>
  <div
    v-if="isOpen"
    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/60 backdrop-blur-xs animate-in fade-in"
  >
    <div
      class="bg-white rounded-2xl shadow-2xl border border-slate-200 w-full max-w-xl overflow-hidden flex flex-col max-h-[90vh]"
    >
      <!-- Modal Header -->
      <div class="px-6 py-4 border-b border-slate-100 flex items-center justify-between bg-gradient-to-r from-cyan-50/50 via-white to-slate-50">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-xl bg-cyan-600 text-white flex items-center justify-center shadow-xs">
            <DownloadCloud class="w-5 h-5" />
          </div>
          <div>
            <h3 class="font-extrabold text-slate-900 text-sm">Pembaruan Notane Client</h3>
            <p class="text-xs text-slate-500">Pembaruan Antarmuka Kasir & Kompatibilitas Versi Server</p>
          </div>
        </div>

        <button
          type="button"
          @click="emit('close')"
          class="p-2 text-slate-400 hover:text-slate-600 hover:bg-slate-100 rounded-xl transition cursor-pointer"
        >
          <X class="w-4 h-4" />
        </button>
      </div>

      <!-- Modal Body -->
      <div class="p-6 space-y-4 overflow-y-auto">
        <!-- Dual Version Cards: Client vs Server -->
        <div class="grid grid-cols-2 gap-3">
          <!-- Client Card -->
          <div class="p-3.5 rounded-xl border border-cyan-200 bg-cyan-50/50 space-y-1">
            <div class="flex items-center justify-between text-cyan-800 text-xs font-bold">
              <span class="flex items-center gap-1.5">
                <Monitor class="w-3.5 h-3.5 text-cyan-600" />
                <span>Client UI Saat Ini</span>
              </span>
              <span class="px-1.5 py-0.2 rounded text-[10px] font-mono bg-cyan-100 border border-cyan-300">
                UI Terminal
              </span>
            </div>
            <div class="font-mono text-base font-extrabold text-cyan-950">
              v{{ licenseStore.clientVersionInfo.version }}
            </div>
            <div class="text-[10px] text-cyan-700">Antarmuka Desktop Kasir & Back-Office</div>
          </div>

          <!-- Server Card -->
          <div class="p-3.5 rounded-xl border border-indigo-200 bg-indigo-50/50 space-y-1">
            <div class="flex items-center justify-between text-indigo-800 text-xs font-bold">
              <span class="flex items-center gap-1.5">
                <Server class="w-3.5 h-3.5 text-indigo-600" />
                <span>Notane Server Terhubung</span>
              </span>
              <span class="px-1.5 py-0.2 rounded text-[10px] font-mono bg-indigo-100 border border-indigo-300">
                Engine & DB
              </span>
            </div>
            <div class="font-mono text-base font-extrabold text-indigo-950">
              v{{ licenseStore.versionInfo.version }}
            </div>
            <div class="text-[10px] text-indigo-700">
              Host: {{ networkStore.targetHost }}:{{ networkStore.targetPort }}
            </div>
          </div>
        </div>

        <!-- Action Button: Periksa Pembaruan -->
        <div class="flex items-center justify-between pt-1">
          <div class="text-xs text-slate-500">
            Sumber Update: <span class="font-mono font-semibold text-slate-700">Notane Provider Management</span>
          </div>
          <button
            type="button"
            @click="handleCheckUpdate"
            :disabled="isChecking || isUpdating"
            class="px-3.5 py-1.5 bg-white hover:bg-slate-50 border border-slate-300 text-slate-700 rounded-xl text-xs font-bold transition flex items-center gap-1.5 cursor-pointer shadow-2xs hover:border-slate-400 disabled:opacity-50"
          >
            <RefreshCw class="w-3.5 h-3.5" :class="{ 'animate-spin': isChecking }" />
            <span>{{ isChecking ? 'Memeriksa...' : 'Cek Pembaruan Client' }}</span>
          </button>
        </div>

        <!-- Update Results Section -->
        <div v-if="licenseStore.updateInfo" class="space-y-3">
          <!-- HAS UPDATE -->
          <div v-if="licenseStore.updateInfo.has_update" class="space-y-3">
            <!-- Server Incompatible Alert Warning (BLOCKING) -->
            <div
              v-if="!isServerCompatible"
              class="p-4 rounded-xl border border-rose-300 bg-rose-50 text-rose-900 space-y-2 animate-in fade-in"
            >
              <div class="flex items-start gap-2.5">
                <AlertTriangle class="w-5 h-5 text-rose-600 shrink-0 mt-0.5" />
                <div>
                  <h4 class="font-extrabold text-xs">Pembaruan Server Diperlukan Terlebih Dahulu</h4>
                  <p class="text-[11px] text-rose-800 leading-relaxed mt-1">
                    {{ licenseStore.updateInfo.message }}
                  </p>
                  <div class="mt-2 text-[10px] font-mono bg-white/80 p-2 rounded border border-rose-200">
                    Syarat Minimum Server: <strong class="text-rose-900">v{{ licenseStore.updateInfo.min_server_version }}</strong> | Versi Server Saat Ini: <strong class="text-rose-900">v{{ licenseStore.versionInfo.version }}</strong>
                  </div>
                </div>
              </div>
            </div>

            <!-- Server Compatible Alert (READY) -->
            <div
              v-else
              class="p-4 rounded-xl border border-emerald-300 bg-emerald-50 text-emerald-900 space-y-2 animate-in fade-in"
            >
              <div class="flex items-center justify-between">
                <div class="flex items-center gap-2">
                  <CheckCircle2 class="w-4 h-4 text-emerald-600 shrink-0" />
                  <span class="font-extrabold text-xs">Server Host Kompatibel</span>
                </div>
                <span class="text-[10px] font-mono font-bold px-2 py-0.5 rounded-full bg-emerald-100 text-emerald-800 border border-emerald-300">
                  Siap Diperbarui
                </span>
              </div>
              <p class="text-[11px] text-emerald-800">
                Notane Server v{{ licenseStore.versionInfo.version }} memenuhi syarat minimum rilis ini. Pembaruan hanya akan memutakhirkan tampilan UI kasir tanpa mengganggu database.
              </p>
            </div>

            <!-- Release Details Card -->
            <div class="p-4 rounded-xl border border-slate-200 bg-slate-50/70 space-y-2.5">
              <div class="flex items-center justify-between">
                <div>
                  <span class="text-xs font-extrabold text-slate-900">
                    {{ licenseStore.updateInfo.release_name || `Rilis Client v${licenseStore.updateInfo.latest_version}` }}
                  </span>
                  <div class="text-[10px] text-slate-500 font-mono mt-0.5 flex items-center gap-2">
                    <span class="flex items-center gap-1">
                      <Calendar class="w-3 h-3 text-slate-400" />
                      {{ licenseStore.updateInfo.release_date || 'Terbaru' }}
                    </span>
                    <span>•</span>
                    <span class="text-cyan-700 font-bold">Target: Client UI</span>
                  </div>
                </div>

                <div class="px-2.5 py-1 rounded-lg bg-cyan-600 text-white font-mono font-extrabold text-xs shadow-2xs">
                  v{{ licenseStore.updateInfo.latest_version }}
                </div>
              </div>

              <!-- Changelog -->
              <div class="space-y-1">
                <div class="text-[10px] font-bold text-slate-500 uppercase tracking-wider">Catatan Perubahan UI:</div>
                <div class="p-3 rounded-lg bg-white border border-slate-200 text-xs text-slate-700 whitespace-pre-line leading-relaxed font-sans max-h-36 overflow-y-auto">
                  {{ licenseStore.updateInfo.changelog || 'Pembaruan antarmuka kasir dan stabilitas performa.' }}
                </div>
              </div>

              <!-- Action Update Button -->
              <div class="pt-2 flex items-center justify-between">
                <div class="text-[11px] text-slate-500">
                  <span v-if="!isServerCompatible" class="text-rose-600 font-semibold">
                    Tombol dinonaktifkan hingga Notane Server diperbarui.
                  </span>
                  <span v-else>
                    Klik untuk memuat ulang antarmuka ke versi terbaru.
                  </span>
                </div>

                <button
                  type="button"
                  @click="handleApplyUpdate"
                  :disabled="!isServerCompatible || isUpdating"
                  class="px-4 py-2 bg-cyan-600 hover:bg-cyan-700 disabled:opacity-40 disabled:cursor-not-allowed text-white rounded-xl text-xs font-bold transition flex items-center gap-2 cursor-pointer shadow-md shadow-cyan-600/20 active:scale-95"
                >
                  <RefreshCw v-if="isUpdating" class="w-3.5 h-3.5 animate-spin" />
                  <Sparkles v-else class="w-3.5 h-3.5" />
                  <span>{{ isUpdating ? 'Menerapkan UI Baru...' : 'Perbarui UI Klien Sekarang' }}</span>
                </button>
              </div>
            </div>
          </div>

          <!-- UP TO DATE -->
          <div v-else class="p-6 rounded-xl border border-slate-200 bg-slate-50/50 text-center space-y-2">
            <CheckCircle2 class="w-8 h-8 text-emerald-500 mx-auto" />
            <h4 class="font-extrabold text-slate-900 text-sm">Antarmuka Client Sudah Mutakhir</h4>
            <p class="text-xs text-slate-500 max-w-sm mx-auto">
              {{ licenseStore.updateInfo.message || 'Anda telah menggunakan versi antarmuka Notane Client terbaru.' }}
            </p>
          </div>
        </div>

        <div v-else class="p-8 rounded-xl border border-dashed border-slate-200 text-center text-xs text-slate-400">
          Klik tombol <strong>"Cek Pembaruan Client"</strong> untuk mencari rilis antarmuka terbaru dari Notane Provider.
        </div>
      </div>

      <!-- Modal Footer -->
      <div class="px-6 py-3 bg-slate-50 border-t border-slate-100 flex items-center justify-between text-xs text-slate-500">
        <div class="flex items-center gap-1.5 font-mono text-[11px]">
          <Layers class="w-3.5 h-3.5 text-slate-400" />
          <span>Notane Architecture Dual-Bundle (Client UI Isolated)</span>
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
