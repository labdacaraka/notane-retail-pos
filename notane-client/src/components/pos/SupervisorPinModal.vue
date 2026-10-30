<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, computed, nextTick } from 'vue'
import { ShieldAlert, X, Check, Fingerprint, Camera, KeyRound } from 'lucide-vue-next'
import { Icon } from '@iconify/vue'
import { api } from '@/services/api'
import { useBiometric } from '@/composables/useBiometric'
import SearchableSelect from '@/components/common/SearchableSelect.vue'
import type { BiometricSettings } from '@/types'

interface SupervisorOption {
  id: string
  username: string
  full_name: string
  role_name: string
  has_biometric?: boolean
  has_face_recognition?: boolean
  face_photo_preview?: string | null
  biometric_credential_id?: string | null
}

const props = defineProps<{
  show: boolean
  title?: string
  description?: string
  actionType?: string
  outletId?: string
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'approved', supervisor: { id: string; name: string }): void
}>()

const {
  verifyFingerprint,
  startCamera,
  stopCamera,
  captureFace,
  fetchBiometricSettings,
} = useBiometric()

const supervisors = ref<SupervisorOption[]>([])
const selectedSupervisorId = ref<string>('')
const pin = ref<string>('')
const isChecking = ref<boolean>(false)
const errorMessage = ref<string>('')
const inputRef = ref<HTMLInputElement | null>(null)

// Biometric settings & active modes
const biometricSettings = ref<BiometricSettings>({
  supervisor_biometric_mode: 'OFF',
  supervisor_biometric_policy: 'OPTIONAL',
  face_match_threshold: 75.0,
})

// Tab mode in modal: 'BIOMETRIC' or 'PIN'
const authMode = ref<'BIOMETRIC' | 'PIN'>('BIOMETRIC')

// Live Camera Face Scanner state
const isScanningFace = ref<boolean>(false)
const videoRef = ref<HTMLVideoElement | null>(null)
const faceMatchScore = ref<number | null>(null)
const scanStatusText = ref<string>('Posisikan wajah di tengah lingkaran kamera...')

const selectedSupervisor = computed(() => {
  return supervisors.value.find((s) => s.id === selectedSupervisorId.value)
})

const isBiometricAvailable = computed(() => {
  return biometricSettings.value.supervisor_biometric_mode !== 'OFF'
})

const canUseFingerprint = computed(() => {
  return (
    ['FINGERPRINT', 'ANY'].includes(biometricSettings.value.supervisor_biometric_mode) &&
    (!selectedSupervisor.value || selectedSupervisor.value.has_biometric !== false)
  )
})

const canUseFace = computed(() => {
  return (
    ['FACE_RECOGNITION', 'ANY'].includes(biometricSettings.value.supervisor_biometric_mode) &&
    (!selectedSupervisor.value || selectedSupervisor.value.has_face_recognition !== false)
  )
})

const isPinDisabled = computed(() => {
  return (
    biometricSettings.value.supervisor_biometric_policy === 'STRICT' &&
    selectedSupervisor.value &&
    (selectedSupervisor.value.has_biometric || selectedSupervisor.value.has_face_recognition)
  )
})

const supervisorExtraParams = computed(() => {
  const params: Record<string, any> = {}
  if (props.outletId) {
    params.outlet_id = props.outletId
  }
  return params
})

async function loadData() {
  try {
    const params: Record<string, any> = {}
    if (props.outletId) {
      params.outlet_id = props.outletId
    }
    const [spvRes, bioSettings] = await Promise.all([
      api.get('/auth/supervisors', { params }),
      fetchBiometricSettings(),
    ])

    if (spvRes.data?.success && Array.isArray(spvRes.data.data)) {
      supervisors.value = spvRes.data.data
      if (supervisors.value.length > 0) {
        if (!selectedSupervisorId.value || !supervisors.value.some((s) => s.id === selectedSupervisorId.value)) {
          selectedSupervisorId.value = supervisors.value[0].id
        }
      } else {
        selectedSupervisorId.value = ''
      }
    }

    biometricSettings.value = bioSettings

    // Tentukan mode awal
    if (biometricSettings.value.supervisor_biometric_mode !== 'OFF') {
      authMode.value = 'BIOMETRIC'
    } else {
      authMode.value = 'PIN'
    }
  } catch (err) {
    console.error('Gagal memuat data supervisor:', err)
  }
}

function onSupervisorsLoaded(items: SupervisorOption[]) {
  supervisors.value = items
  if (items.length > 0) {
    if (!selectedSupervisorId.value || !items.some((s) => s.id === selectedSupervisorId.value)) {
      selectedSupervisorId.value = items[0].id
    }
  } else {
    selectedSupervisorId.value = ''
  }
}

function onSupervisorChanged(val: string, item: SupervisorOption) {
  selectedSupervisorId.value = val || ''
  if (item && !supervisors.value.some((s) => s.id === item.id)) {
    supervisors.value.push(item)
  }
  clearPin()
}

watch(
  () => props.outletId,
  (newOutlet, oldOutlet) => {
    if (newOutlet !== oldOutlet) {
      loadData()
    }
  }
)

function focusInput() {
  setTimeout(() => inputRef.value?.focus(), 80)
}

function appendPin(digit: string) {
  if (pin.value.length < 6) {
    pin.value += digit
  }
}

function clearPin() {
  pin.value = ''
  errorMessage.value = ''
}

// ----------------------------------------------------------------------------
// VERIFIKASI PIN
// ----------------------------------------------------------------------------
async function verifyPin() {
  if (pin.value.length < 4) {
    errorMessage.value = 'Masukkan minimal 4 digit PIN Supervisor'
    return
  }

  isChecking.value = true
  errorMessage.value = ''

  try {
    const res = await api.post('/auth/supervisor-approve', {
      supervisor_id: selectedSupervisorId.value || undefined,
      pin: pin.value,
      action: props.actionType || 'SUPERVISOR_OVERRIDE',
    })

    if (res.data.success) {
      emit('approved', {
        id: res.data.data.supervisor_id,
        name: res.data.data.supervisor_name,
      })
      handleClose()
    } else {
      errorMessage.value = res.data.message || 'PIN Supervisor salah'
      pin.value = ''
    }
  } catch (err: any) {
    errorMessage.value =
      err.response?.data?.message || 'PIN Supervisor salah atau tidak memiliki wewenang'
    pin.value = ''
  } finally {
    isChecking.value = false
  }
}

// ----------------------------------------------------------------------------
// VERIFIKASI SIDIK JARI (Touch ID / Windows Hello)
// ----------------------------------------------------------------------------
async function triggerFingerprintAuth() {
  errorMessage.value = ''
  isChecking.value = true

  try {
    const expectedCredId = selectedSupervisor.value?.biometric_credential_id || undefined
    const assertion = await verifyFingerprint(expectedCredId)

    const res = await api.post('/auth/supervisor-biometric-approve', {
      supervisor_id: selectedSupervisorId.value || undefined,
      biometric_type: 'FINGERPRINT',
      credential_id: assertion.credential_id,
      action: props.actionType || 'SUPERVISOR_OVERRIDE',
    })

    if (res.data.success) {
      emit('approved', {
        id: res.data.data.supervisor_id,
        name: res.data.data.supervisor_name,
      })
      handleClose()
    } else {
      errorMessage.value = res.data.message || 'Verifikasi sidik jari ditolak server'
    }
  } catch (err: any) {
    errorMessage.value =
      err.message || err.response?.data?.message || 'Pemindaian sidik jari dibatalkan atau gagal'
  } finally {
    isChecking.value = false
  }
}

// ----------------------------------------------------------------------------
// VERIFIKASI WAJAH (Live Camera HUD)
// ----------------------------------------------------------------------------
async function triggerFaceScanner() {
  errorMessage.value = ''
  isScanningFace.value = true
  faceMatchScore.value = null
  scanStatusText.value = 'Menghubungkan ke kamera...'

  await nextTick()
  if (videoRef.value) {
    try {
      await startCamera(videoRef.value)
      scanStatusText.value = 'Mendeteksi wajah... Posisikan wajah di dalam lingkaran.'
    } catch (err: any) {
      isScanningFace.value = false
      errorMessage.value = err.message || 'Gagal menyalakan kamera kasir'
    }
  }
}

async function captureAndVerifyFace() {
  if (!videoRef.value) return
  isChecking.value = true
  errorMessage.value = ''

  try {
    const captured = captureFace(videoRef.value)
    scanStatusText.value = 'Menganalisis tensor biometrik wajah di server...'

    const res = await api.post('/auth/supervisor-biometric-approve', {
      supervisor_id: selectedSupervisorId.value || undefined,
      biometric_type: 'FACE',
      live_face_descriptor: captured.descriptor,
      action: props.actionType || 'SUPERVISOR_OVERRIDE',
    })

    if (res.data.success) {
      scanStatusText.value = '✅ Wajah Terverifikasi! Otorisasi disetujui.'
      setTimeout(() => {
        emit('approved', {
          id: res.data.data.supervisor_id,
          name: res.data.data.supervisor_name,
        })
        handleClose()
      }, 700)
    } else {
      errorMessage.value = res.data.message || 'Pengenalan wajah tidak cocok dengan supervisor'
    }
  } catch (err: any) {
    errorMessage.value =
      err.response?.data?.message || err.message || 'Gagal memverifikasi pemindai wajah'
  } finally {
    isChecking.value = false
  }
}

function stopFaceScanner() {
  stopCamera(videoRef.value || undefined)
  isScanningFace.value = false
  faceMatchScore.value = null
}

function handleClose() {
  stopFaceScanner()
  clearPin()
  errorMessage.value = ''
  emit('close')
}

function handleKeyDown(e: KeyboardEvent) {
  if (e.key === 'Escape' && props.show) {
    handleClose()
  } else if (e.key === 'Enter' && props.show && authMode.value === 'PIN') {
    verifyPin()
  }
}

onMounted(() => {
  window.addEventListener('keydown', handleKeyDown)
  loadData()
  focusInput()
})

watch(
  () => props.show,
  (val) => {
    if (val) {
      clearPin()
      loadData()
      focusInput()
    } else {
      stopFaceScanner()
    }
  }
)

onUnmounted(() => {
  stopFaceScanner()
  window.removeEventListener('keydown', handleKeyDown)
})
</script>

<template>
  <div
    v-if="show"
    class="fixed inset-0 z-50 bg-slate-900/60 backdrop-blur-xs flex items-center justify-center p-4 select-none animate-fadeIn"
  >
    <div
      class="bg-white border border-slate-300 w-full max-w-sm rounded-2xl shadow-2xl overflow-hidden text-slate-900 flex flex-col"
    >
      <!-- Header -->
      <div class="bg-slate-100 p-4 border-b border-slate-300 flex items-center justify-between">
        <div class="flex items-center gap-2 text-amber-700">
          <ShieldAlert class="w-5 h-5 shrink-0" />
          <h3 class="font-bold text-sm text-slate-900 leading-tight">
            {{ title || 'Otorisasi Supervisor' }}
          </h3>
        </div>
        <button
          @click="handleClose"
          class="text-slate-400 hover:text-slate-700 p-1 rounded-lg hover:bg-slate-200 transition cursor-pointer"
        >
          <X class="w-4 h-4" />
        </button>
      </div>

      <!-- Body -->
      <div class="p-5 space-y-4">
        <p class="text-xs text-slate-600 text-center leading-relaxed">
          {{
            description ||
            'Aksi ini memerlukan persetujuan Supervisor Toko. Silakan pilih supervisor dan lakukan verifikasi:'
          }}
        </p>

        <!-- Supervisor Selector -->
        <div>
          <label class="block text-xs font-bold text-slate-700 mb-1">
            Supervisor Penanggung Jawab:
          </label>
          <div class="relative">
            <SearchableSelect
              v-model="selectedSupervisorId"
              api-endpoint="/auth/supervisors"
              :extra-params="supervisorExtraParams"
              value-key="id"
              label-key="full_name"
              secondary-key="role_name"
              placeholder="-- Pilih Supervisor Penanggung Jawab --"
              search-placeholder="Cari nama atau role supervisor..."
              :allow-clear="false"
              :disabled="isChecking"
              @loaded="onSupervisorsLoaded"
              @change="onSupervisorChanged"
            />
          </div>

          <!-- Badges Kemampuan Biometrik Supervisor -->
          <div v-if="selectedSupervisor" class="flex items-center gap-2 mt-1.5 px-1">
            <span
              v-if="selectedSupervisor.has_biometric"
              class="inline-flex items-center gap-1 text-[10px] font-bold text-emerald-700 bg-emerald-50 px-2 py-0.5 rounded-md border border-emerald-200"
            >
              <Fingerprint class="w-3 h-3 text-emerald-600" />
              <span>Sidik Jari Aktif</span>
            </span>
            <span
              v-if="selectedSupervisor.has_face_recognition"
              class="inline-flex items-center gap-1 text-[10px] font-bold text-blue-700 bg-blue-50 px-2 py-0.5 rounded-md border border-blue-200"
            >
              <Camera class="w-3 h-3 text-blue-600" />
              <span>Wajah Aktif</span>
            </span>
          </div>
        </div>

        <!-- Peringatan jika belum ada supervisor yang ditugaskan di cabang ini -->
        <div
          v-if="outletId && supervisors.length === 0 && !isChecking"
          class="bg-amber-50 border border-amber-300 rounded-xl p-3 text-xs text-amber-900 flex items-start gap-2.5 animate-fadeIn"
        >
          <ShieldAlert class="w-4 h-4 text-amber-600 shrink-0 mt-0.5" />
          <div>
            <p class="font-bold">Belum ada supervisor yang ditugaskan di cabang ini</p>
            <p class="text-[11px] text-amber-800 mt-0.5 leading-relaxed">
              Harap menghubungi admin untuk penugasan supervisor di cabang tersebut.
            </p>
          </div>
        </div>

        <template v-if="!outletId || supervisors.length > 0">
          <!-- Mode Switcher (Jika Biometrik Aktif & PIN diizinkan) -->
          <div
            v-if="isBiometricAvailable && !isPinDisabled"
            class="flex rounded-xl bg-slate-100 p-1 border border-slate-200 text-xs font-bold"
          >
          <button
            type="button"
            @click="authMode = 'BIOMETRIC'"
            class="flex-1 py-1.5 rounded-lg flex items-center justify-center gap-1.5 transition cursor-pointer"
            :class="
              authMode === 'BIOMETRIC'
                ? 'bg-white text-amber-800 shadow-2xs font-extrabold'
                : 'text-slate-600 hover:text-slate-900'
            "
          >
            <Fingerprint class="w-3.5 h-3.5" />
            <span>Biometrik</span>
          </button>
          <button
            type="button"
            @click="
              authMode = 'PIN';
              stopFaceScanner()
            "
            class="flex-1 py-1.5 rounded-lg flex items-center justify-center gap-1.5 transition cursor-pointer"
            :class="
              authMode === 'PIN'
                ? 'bg-white text-indigo-800 shadow-2xs font-extrabold'
                : 'text-slate-600 hover:text-slate-900'
            "
          >
            <KeyRound class="w-3.5 h-3.5" />
            <span>PIN Angka</span>
          </button>
        </div>

        <!-- 1. TAMPILAN MODE BIOMETRIK -->
        <div v-if="authMode === 'BIOMETRIC' && isBiometricAvailable" class="space-y-3">
          <!-- LIVE FACE SCANNER VIEW (Saat Kamera Terbuka) -->
          <div v-if="isScanningFace" class="space-y-3">
            <div
              class="relative w-full aspect-4/3 bg-black rounded-xl overflow-hidden shadow-inner flex items-center justify-center border-2 border-blue-500"
            >
              <video
                ref="videoRef"
                class="w-full h-full object-cover transform scale-x-[-1]"
              ></video>
              <!-- HUD Circular Scanner Reticle -->
              <div class="absolute inset-0 pointer-events-none flex items-center justify-center">
                <div
                  class="w-36 h-36 rounded-full border-2 border-dashed border-blue-400 animate-pulse flex items-center justify-center"
                >
                  <div class="w-32 h-32 rounded-full border border-blue-300/40"></div>
                </div>
              </div>
              <div class="absolute bottom-2 inset-x-0 text-center pointer-events-none px-2">
                <span
                  class="bg-black/70 text-white text-[10px] px-2.5 py-1 rounded-full font-medium"
                >
                  {{ scanStatusText }}
                </span>
              </div>
            </div>

            <div class="flex items-center gap-2">
              <button
                type="button"
                @click="captureAndVerifyFace"
                :disabled="isChecking"
                class="flex-1 py-2.5 bg-blue-600 hover:bg-blue-700 text-white rounded-xl text-xs font-bold shadow-xs flex items-center justify-center gap-1.5 cursor-pointer active:scale-95 disabled:opacity-50"
              >
                <Icon
                  v-if="isChecking"
                  icon="fluent-color:spinner-ios-20"
                  class="w-4 h-4 animate-spin"
                />
                <Camera v-else class="w-4 h-4" />
                <span>{{ isChecking ? 'Menganalisis...' : 'Verifikasi Wajah' }}</span>
              </button>
              <button
                type="button"
                @click="stopFaceScanner"
                class="px-3 py-2.5 bg-slate-200 hover:bg-slate-300 text-slate-800 rounded-xl text-xs font-bold cursor-pointer"
              >
                Tutup
              </button>
            </div>
          </div>

          <!-- PILIHAN TOMBOL BIOMETRIK (SIDIK JARI / WAJAH) -->
          <div v-else class="space-y-2.5">
            <!-- Tombol Sidik Jari / Touch ID -->
            <button
              v-if="canUseFingerprint"
              type="button"
              @click="triggerFingerprintAuth"
              :disabled="isChecking"
              class="w-full py-3 px-4 bg-emerald-600 hover:bg-emerald-700 text-white rounded-xl font-bold text-xs shadow-sm flex items-center justify-center gap-2.5 transition active:scale-95 cursor-pointer disabled:opacity-50"
            >
              <Fingerprint class="w-5 h-5 text-emerald-100" />
              <span>Sentuh Sensor Sidik Jari / Touch ID</span>
            </button>

            <!-- Tombol Pemindai Wajah -->
            <button
              v-if="canUseFace"
              type="button"
              @click="triggerFaceScanner"
              :disabled="isChecking"
              class="w-full py-3 px-4 bg-indigo-600 hover:bg-indigo-700 text-white rounded-xl font-bold text-xs shadow-sm flex items-center justify-center gap-2.5 transition active:scale-95 cursor-pointer disabled:opacity-50"
            >
              <Camera class="w-5 h-5 text-indigo-100" />
              <span>Pindai Wajah Supervisor (Kamera)</span>
            </button>

            <!-- Peringatan jika belum enroll -->
            <div
              v-if="
                selectedSupervisor &&
                !selectedSupervisor.has_biometric &&
                !selectedSupervisor.has_face_recognition
              "
              class="p-2.5 bg-amber-50 border border-amber-200 rounded-xl text-amber-800 text-[11px] text-center"
            >
              Supervisor ini belum mendaftarkan biometrik.
              <button
                type="button"
                @click="authMode = 'PIN'"
                class="underline font-bold text-amber-950 ml-1 cursor-pointer"
              >
                Gunakan PIN Angka
              </button>
            </div>
          </div>
        </div>

        <!-- 2. TAMPILAN MODE PIN ANGKA -->
        <div v-else class="space-y-3.5">
          <!-- PIN Input Field -->
          <div class="relative">
            <input
              ref="inputRef"
              v-model="pin"
              type="password"
              maxlength="6"
              placeholder="••••••"
              class="w-full bg-slate-50 border-2 border-amber-500/70 rounded-xl py-3 text-center text-3xl tracking-widest font-mono text-slate-900 focus:bg-white focus:outline-none focus:border-amber-600 focus:ring-4 focus:ring-amber-500/20"
              @keyup.enter="verifyPin"
            />
          </div>

          <!-- Numeric Keypad -->
          <div class="grid grid-cols-3 gap-2 pt-1">
            <button
              v-for="n in 9"
              :key="n"
              @click="appendPin(n.toString())"
              class="bg-slate-100 hover:bg-slate-200 text-slate-900 font-mono text-lg font-bold py-3 rounded-xl border border-slate-300 transition active:scale-95 shadow-xs cursor-pointer"
            >
              {{ n }}
            </button>
            <button
              @click="clearPin"
              class="bg-rose-50 hover:bg-rose-100 text-rose-700 font-bold py-3 rounded-xl border border-rose-300 transition active:scale-95 text-xs shadow-xs cursor-pointer"
            >
              HAPUS
            </button>
            <button
              @click="appendPin('0')"
              class="bg-slate-100 hover:bg-slate-200 text-slate-900 font-mono text-lg font-bold py-3 rounded-xl border border-slate-300 transition active:scale-95 shadow-xs cursor-pointer"
            >
              0
            </button>
            <button
              @click="verifyPin"
              :disabled="isChecking"
              class="bg-emerald-600 hover:bg-emerald-700 text-white font-bold py-3 rounded-xl border border-emerald-700 transition active:scale-95 flex items-center justify-center gap-1 text-xs shadow-xs cursor-pointer disabled:opacity-50"
            >
              <Check class="w-4 h-4" />
              <span>OK</span>
            </button>
          </div>
        </div>
        </template>

        <!-- Error Msg -->
        <div
          v-if="errorMessage"
          class="bg-rose-50 border border-rose-300 p-2.5 rounded-xl text-rose-700 text-xs text-center font-medium animate-shake"
        >
          ⚠️ {{ errorMessage }}
        </div>
      </div>
    </div>
  </div>
</template>
