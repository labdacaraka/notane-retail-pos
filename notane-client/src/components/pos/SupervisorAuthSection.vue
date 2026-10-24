<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch, computed } from 'vue'
import {
  Fingerprint,
  Camera,
  KeyRound,
  CheckCircle2,
  AlertCircle,
  RefreshCw,
  X,
  Scan,
} from 'lucide-vue-next'
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

const props = withDefaults(
  defineProps<{
    modelValueSupervisorId?: string
    modelValuePin?: string
    modelValueAuthToken?: string
    actionType?: string
    label?: string
    required?: boolean
    compact?: boolean
    outletId?: string
  }>(),
  {
    modelValueSupervisorId: '',
    modelValuePin: '',
    modelValueAuthToken: '',
    actionType: 'SUPERVISOR_OVERRIDE',
    label: 'Otorisasi Supervisor Penanggung Jawab',
    required: true,
    compact: false,
    outletId: '',
  }
)

const emit = defineEmits<{
  (e: 'update:modelValueSupervisorId', val: string): void
  (e: 'update:modelValuePin', val: string): void
  (e: 'update:modelValueAuthToken', val: string): void
  (e: 'verified', supervisor: { id: string; name: string; authToken?: string; type: 'BIOMETRIC' | 'PIN' }): void
}>()

const {
  verifyFingerprint,
  startCamera,
  stopCamera,
  captureFace,
  fetchBiometricSettings,
} = useBiometric()

const supervisors = ref<SupervisorOption[]>([])
const selectedSupervisorId = ref<string>(props.modelValueSupervisorId || '')
const supervisorPin = ref<string>(props.modelValuePin || '')
const authToken = ref<string>(props.modelValueAuthToken || '')

const biometricSettings = ref<BiometricSettings>({
  supervisor_biometric_mode: 'OFF',
  supervisor_biometric_policy: 'OPTIONAL',
  face_match_threshold: 75,
})

const isVerified = ref<boolean>(!!props.modelValueAuthToken)
const verifiedSupervisorName = ref<string>('')
const verifiedMethod = ref<'FINGERPRINT' | 'FACE' | 'PIN' | ''>('')

const isProcessing = ref<boolean>(false)
const errorMessage = ref<string>('')

// Kamera State
const showCameraScanner = ref<boolean>(false)
const videoRef = ref<HTMLVideoElement | null>(null)
const faceMatchScore = ref<number | null>(null)
const scanStatusText = ref<string>('Posisikan wajah supervisor di dalam lingkaran target...')

const activeSupervisor = computed(() => {
  return supervisors.value.find((s) => s.id === selectedSupervisorId.value)
})

const showFingerprintBtn = computed(() => {
  const mode = biometricSettings.value.supervisor_biometric_mode
  return mode === 'FINGERPRINT' || mode === 'ANY'
})

const showFaceBtn = computed(() => {
  const mode = biometricSettings.value.supervisor_biometric_mode
  return mode === 'FACE_RECOGNITION' || mode === 'ANY'
})

const isPinAllowed = computed(() => {
  const mode = biometricSettings.value.supervisor_biometric_mode
  const policy = biometricSettings.value.supervisor_biometric_policy
  if (mode === 'OFF') return true
  return policy !== 'STRICT'
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
    const [supRes, bio] = await Promise.all([
      api.get('/auth/supervisors', { params }),
      fetchBiometricSettings(),
    ])

    if (supRes.data?.success && Array.isArray(supRes.data.data)) {
      supervisors.value = supRes.data.data
      if (supervisors.value.length > 0) {
        if (!selectedSupervisorId.value || !supervisors.value.some((s) => s.id === selectedSupervisorId.value)) {
          selectedSupervisorId.value = supervisors.value[0].id
          emit('update:modelValueSupervisorId', selectedSupervisorId.value)
        }
      } else {
        selectedSupervisorId.value = ''
        emit('update:modelValueSupervisorId', '')
      }
    }

    if (bio) {
      biometricSettings.value = bio
    }
  } catch (err) {
    console.error('Gagal memuat opsi supervisor atau biometrik:', err)
  }
}

function onSupervisorsLoaded(items: SupervisorOption[]) {
  supervisors.value = items
  if (items.length > 0) {
    if (!selectedSupervisorId.value || !items.some((s) => s.id === selectedSupervisorId.value)) {
      selectedSupervisorId.value = items[0].id
      emit('update:modelValueSupervisorId', items[0].id)
    }
  } else {
    selectedSupervisorId.value = ''
    emit('update:modelValueSupervisorId', '')
  }
}

function onSupervisorChanged(val: string, item: SupervisorOption) {
  selectedSupervisorId.value = val || ''
  emit('update:modelValueSupervisorId', selectedSupervisorId.value)
  if (item && !supervisors.value.some((s) => s.id === item.id)) {
    supervisors.value.push(item)
  }
  if (isVerified.value) {
    resetVerification()
  }
}

watch(
  () => props.outletId,
  (newOutlet, oldOutlet) => {
    if (newOutlet !== oldOutlet) {
      resetVerification()
      loadData()
    }
  }
)

watch(
  () => props.modelValueSupervisorId,
  (val) => {
    if (val && val !== selectedSupervisorId.value) {
      selectedSupervisorId.value = val
    }
  }
)

watch(
  () => selectedSupervisorId.value,
  (val) => {
    emit('update:modelValueSupervisorId', val)
  }
)

watch(
  () => supervisorPin.value,
  (val) => {
    emit('update:modelValuePin', val)
    if (val.trim().length >= 4) {
      // PIN dimasukkan
      if (!isVerified.value) {
        const sup = activeSupervisor.value
        if (sup) {
          verifiedSupervisorName.value = sup.full_name
        }
      }
    }
  }
)

onMounted(() => {
  loadData()
})

onUnmounted(() => {
  stopCamera(videoRef.value || undefined)
})

// 1. Verifikasi Sidik Jari
async function handleFingerprintAuth() {
  isProcessing.value = true
  errorMessage.value = ''

  try {
    const credId = activeSupervisor.value?.biometric_credential_id || undefined
    const authResult = await verifyFingerprint(credId)

    if (authResult.success) {
      const res = await api.post('/auth/supervisor-biometric-approve', {
        supervisor_id: selectedSupervisorId.value || undefined,
        biometric_type: 'FINGERPRINT',
        credential_id: authResult.credential_id,
        action: props.actionType,
      })

      if (res.data?.success) {
        const data = res.data.data
        isVerified.value = true
        verifiedSupervisorName.value = data.supervisor_name
        verifiedMethod.value = 'FINGERPRINT'
        authToken.value = data.auth_token || ''

        if (data.supervisor_id) {
          selectedSupervisorId.value = data.supervisor_id
          emit('update:modelValueSupervisorId', data.supervisor_id)
        }
        emit('update:modelValueAuthToken', authToken.value)
        emit('verified', {
          id: data.supervisor_id,
          name: data.supervisor_name,
          authToken: data.auth_token,
          type: 'BIOMETRIC',
        })
      } else {
        errorMessage.value = res.data?.message || 'Sidik jari tidak cocok dengan supervisor'
      }
    }
  } catch (err: any) {
    errorMessage.value = err.response?.data?.message || err.message || 'Sensor sidik jari gagal memvalidasi'
  } finally {
    isProcessing.value = false
  }
}

// 2. Verifikasi Wajah (Face Recognition)
async function openFaceScanner() {
  errorMessage.value = ''
  faceMatchScore.value = null
  scanStatusText.value = 'Menyiapkan pemindai kamera...'
  showCameraScanner.value = true

  setTimeout(async () => {
    if (videoRef.value) {
      try {
        await startCamera(videoRef.value)
        scanStatusText.value = 'Arahkan wajah supervisor ke kamera...'
      } catch (err: any) {
        errorMessage.value = err.message || 'Gagal mengakses kamera kasir'
        showCameraScanner.value = false
      }
    }
  }, 100)
}

function closeFaceScanner() {
  stopCamera(videoRef.value || undefined)
  showCameraScanner.value = false
}

async function captureAndVerifyFace() {
  if (!videoRef.value) return
  isProcessing.value = true
  errorMessage.value = ''

  try {
    const captured = captureFace(videoRef.value)
    scanStatusText.value = 'Menganalisis tensor biometrik wajah di server...'

    const res = await api.post('/auth/supervisor-biometric-approve', {
      supervisor_id: selectedSupervisorId.value || undefined,
      biometric_type: 'FACE',
      live_face_descriptor: captured.descriptor,
      action: props.actionType,
    })

    if (res.data?.success) {
      const data = res.data.data
      isVerified.value = true
      verifiedSupervisorName.value = data.supervisor_name
      verifiedMethod.value = 'FACE'
      authToken.value = data.auth_token || ''

      if (data.supervisor_id) {
        selectedSupervisorId.value = data.supervisor_id
        emit('update:modelValueSupervisorId', data.supervisor_id)
      }
      emit('update:modelValueAuthToken', authToken.value)
      emit('verified', {
        id: data.supervisor_id,
        name: data.supervisor_name,
        authToken: data.auth_token,
        type: 'BIOMETRIC',
      })

      closeFaceScanner()
    } else {
      errorMessage.value = res.data?.message || 'Wajah tidak cocok dengan template supervisor'
    }
  } catch (err: any) {
    errorMessage.value = err.response?.data?.message || err.message || 'Gagal memvalidasi wajah supervisor'
  } finally {
    isProcessing.value = false
  }
}

function resetVerification() {
  isVerified.value = false
  verifiedSupervisorName.value = ''
  verifiedMethod.value = ''
  authToken.value = ''
  supervisorPin.value = ''
  errorMessage.value = ''
  emit('update:modelValueAuthToken', '')
  emit('update:modelValuePin', '')
}
</script>

<template>
  <div class="space-y-3 font-sans">
    <!-- Header / Label -->
    <div class="flex items-center justify-between">
      <label class="block text-xs font-bold text-slate-800">
        {{ label }} <span v-if="required" class="text-rose-500">*</span>
      </label>
      <div v-if="biometricSettings.supervisor_biometric_mode !== 'OFF'" class="flex items-center gap-1.5 text-[10px] text-emerald-700 bg-emerald-50 border border-emerald-200 px-2 py-0.5 rounded-full font-semibold">
        <span class="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse"></span>
        Biometrik Aktif
      </div>
    </div>

    <!-- Error Alert -->
    <div
      v-if="errorMessage"
      class="bg-rose-50 border border-rose-200 text-rose-700 rounded-xl p-2.5 text-xs flex items-center justify-between gap-2"
    >
      <div class="flex items-center gap-2">
        <AlertCircle class="w-4 h-4 shrink-0 text-rose-500" />
        <span>{{ errorMessage }}</span>
      </div>
      <button @click="errorMessage = ''" class="text-rose-400 hover:text-rose-700">
        <X class="w-3.5 h-3.5" />
      </button>
    </div>

    <!-- STATE 1: SUDAH TEROTORISASI BIOMETRIK / TOKEN -->
    <div
      v-if="isVerified"
      class="bg-emerald-50/90 border-2 border-emerald-500/80 rounded-2xl p-3.5 flex items-center justify-between shadow-xs transition-all duration-200 animate-in fade-in"
    >
      <div class="flex items-center gap-3">
        <div class="w-10 h-10 rounded-xl bg-emerald-500 text-white flex items-center justify-center shrink-0 shadow-xs">
          <CheckCircle2 class="w-6 h-6" />
        </div>
        <div>
          <div class="flex items-center gap-2">
            <span class="text-xs font-black text-emerald-950 uppercase tracking-wider">Otorisasi Terverifikasi</span>
            <span
              v-if="verifiedMethod === 'FINGERPRINT'"
              class="text-[10px] font-bold bg-emerald-200/80 text-emerald-900 px-1.5 py-0.5 rounded flex items-center gap-1"
            >
              <Fingerprint class="w-3 h-3" /> Sidik Jari
            </span>
            <span
              v-else-if="verifiedMethod === 'FACE'"
              class="text-[10px] font-bold bg-cyan-200/80 text-cyan-900 px-1.5 py-0.5 rounded flex items-center gap-1"
            >
              <Camera class="w-3 h-3" /> Wajah
            </span>
            <span
              v-else
              class="text-[10px] font-bold bg-slate-200 text-slate-800 px-1.5 py-0.5 rounded flex items-center gap-1"
            >
              <KeyRound class="w-3 h-3" /> PIN
            </span>
          </div>
          <p class="text-sm font-bold text-slate-900 mt-0.5">
            {{ verifiedSupervisorName || activeSupervisor?.full_name || 'Supervisor Toko' }}
          </p>
        </div>
      </div>

      <button
        type="button"
        @click="resetVerification"
        class="text-xs text-emerald-800 hover:text-rose-700 font-bold flex items-center gap-1 px-2.5 py-1.5 rounded-lg hover:bg-white/80 transition cursor-pointer"
        title="Ulangi otorisasi"
      >
        <RefreshCw class="w-3.5 h-3.5" />
        <span>Ganti</span>
      </button>
    </div>

    <!-- STATE 2: FORM PILIHAN BIOMETRIK & PIN -->
    <div v-else class="space-y-2.5">
      <!-- Selector Supervisor (Searchable Select2) -->
      <div>
        <SearchableSelect
          v-model="selectedSupervisorId"
          api-endpoint="/auth/supervisors"
          :extra-params="supervisorExtraParams"
          value-key="id"
          label-key="full_name"
          secondary-key="role_name"
          :placeholder="!outletId ? '-- Pilih Cabang / Lokasi Terlebih Dahulu --' : '-- Pilih Supervisor Penanggung Jawab --'"
          search-placeholder="Cari nama atau role supervisor..."
          :allow-clear="false"
          :disabled="!outletId || isProcessing"
          @loaded="onSupervisorsLoaded"
          @change="onSupervisorChanged"
        />
      </div>

      <!-- Peringatan jika cabang belum dipilih -->
      <div
        v-if="!outletId"
        class="text-center p-2.5 bg-slate-50 border border-dashed border-slate-300 rounded-xl text-slate-500 text-[11px]"
      >
        Pilih cabang terlebih dahulu untuk menampilkan daftar supervisor yang berwenang.
      </div>

      <!-- Peringatan jika belum ada supervisor yang ditugaskan di cabang ini -->
      <div
        v-else-if="supervisors.length === 0 && !isProcessing"
        class="bg-amber-50 border border-amber-300 rounded-xl p-3 text-xs text-amber-900 flex items-start gap-2.5 animate-fadeIn"
      >
        <AlertCircle class="w-4 h-4 text-amber-600 shrink-0 mt-0.5" />
        <div>
          <p class="font-bold">Belum ada supervisor yang ditugaskan di cabang ini</p>
          <p class="text-[11px] text-amber-800 mt-0.5 leading-relaxed">
            Harap menghubungi admin untuk penugasan supervisor di cabang tersebut.
          </p>
        </div>
      </div>

      <template v-else-if="outletId && supervisors.length > 0">
        <!-- Action Buttons Biometrik (Sidik Jari & Kamera) -->
        <div v-if="showFingerprintBtn || showFaceBtn" class="grid grid-cols-2 gap-2">
          <!-- Tombol Sidik Jari -->
          <button
            v-if="showFingerprintBtn"
            type="button"
            :disabled="isProcessing"
            @click="handleFingerprintAuth"
            class="flex items-center justify-center gap-2 px-3 py-2.5 rounded-xl border-2 border-emerald-600/40 bg-emerald-50 hover:bg-emerald-100 text-emerald-900 font-bold text-xs transition active:scale-95 cursor-pointer disabled:opacity-50"
          >
            <Fingerprint class="w-4 h-4 text-emerald-600 animate-pulse" />
            <span>{{ isProcessing ? 'Memindai...' : 'Sentuh Sidik Jari' }}</span>
          </button>

          <!-- Tombol Kamera Wajah -->
          <button
            v-if="showFaceBtn"
            type="button"
            :disabled="isProcessing"
            @click="openFaceScanner"
            class="flex items-center justify-center gap-2 px-3 py-2.5 rounded-xl border-2 border-cyan-600/40 bg-cyan-50 hover:bg-cyan-100 text-cyan-900 font-bold text-xs transition active:scale-95 cursor-pointer disabled:opacity-50"
          >
            <Camera class="w-4 h-4 text-cyan-600" />
            <span>Pindai Wajah</span>
          </button>
        </div>

        <!-- Field Input PIN (Jika diizinkan) -->
        <div v-if="isPinAllowed">
          <div class="relative">
            <input
              v-model="supervisorPin"
              type="password"
              maxlength="10"
              placeholder="Atau masukkan 6-digit PIN Supervisor"
              class="w-full bg-slate-50 border border-slate-300 rounded-xl pl-9 pr-3 py-2 text-xs text-slate-900 font-mono tracking-widest focus:bg-white focus:border-emerald-600 focus:outline-none"
            />
            <KeyRound class="w-4 h-4 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none" />
          </div>
        </div>

        <!-- Notifikasi jika mode STRICT -->
        <div
          v-else
          class="bg-amber-50 border border-amber-200 rounded-xl p-2.5 text-[11px] text-amber-900 flex items-center gap-2"
        >
          <AlertCircle class="w-4 h-4 text-amber-600 shrink-0" />
          <span>Kebijakan toko mewajibkan otorisasi biometrik supervisor. Input PIN keypad dinonaktifkan.</span>
        </div>
      </template>

      <!-- Notifikasi jika mode STRICT -->
      <div
        v-else
        class="bg-amber-50 border border-amber-200 rounded-xl p-2.5 text-[11px] text-amber-900 flex items-center gap-2"
      >
        <AlertCircle class="w-4 h-4 text-amber-600 shrink-0" />
        <span>Kebijakan toko mewajibkan otorisasi biometrik supervisor. Input PIN keypad dinonaktifkan.</span>
      </div>
    </div>

    <!-- MODAL POPUP KAMERA PEMINDAI WAJAH INLINE -->
    <div
      v-if="showCameraScanner"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/75 backdrop-blur-xs p-4 select-none font-sans"
    >
      <div class="bg-slate-900 border border-cyan-500/50 rounded-2xl max-w-sm w-full overflow-hidden shadow-2xl text-white">
        <!-- Header -->
        <div class="p-3.5 bg-slate-950/80 border-b border-slate-800 flex items-center justify-between">
          <div class="flex items-center gap-2 text-cyan-400">
            <Camera class="w-4 h-4" />
            <span class="text-xs font-black uppercase tracking-wider">Pemindai Wajah Supervisor</span>
          </div>
          <button @click="closeFaceScanner" class="text-slate-400 hover:text-white">
            <X class="w-4 h-4" />
          </button>
        </div>

        <!-- Camera Video Feed & HUD -->
        <div class="relative w-full aspect-4/3 bg-black flex items-center justify-center overflow-hidden">
          <video
            ref="videoRef"
            autoplay
            playsinline
            muted
            class="w-full h-full object-cover scale-x-[-1]"
          ></video>

          <!-- Circular Face Target Frame -->
          <div class="absolute inset-0 flex items-center justify-center pointer-events-none">
            <div class="w-44 h-56 rounded-full border-2 border-dashed border-cyan-400/80 shadow-[0_0_20px_rgba(6,182,212,0.4)] flex items-center justify-center">
              <Scan class="w-8 h-8 text-cyan-400 animate-pulse opacity-70" />
            </div>
          </div>

          <!-- Match Score Overlay -->
          <div
            v-if="faceMatchScore !== null"
            class="absolute top-2 right-2 bg-slate-900/90 border border-cyan-400 px-2 py-0.5 rounded-full text-[10px] font-mono text-cyan-300 font-bold"
          >
            Akurasi: {{ faceMatchScore }}%
          </div>
        </div>

        <!-- Camera Status & Button -->
        <div class="p-3.5 bg-slate-950/90 text-center space-y-3">
          <p class="text-xs text-slate-300">{{ scanStatusText }}</p>

          <div class="flex items-center gap-2">
            <button
              type="button"
              @click="closeFaceScanner"
              class="flex-1 py-2 rounded-xl text-xs font-bold bg-slate-800 hover:bg-slate-700 text-slate-300 cursor-pointer"
            >
              Batal
            </button>
            <button
              type="button"
              :disabled="isProcessing"
              @click="captureAndVerifyFace"
              class="flex-2 py-2 rounded-xl text-xs font-black bg-cyan-600 hover:bg-cyan-500 text-white shadow-lg shadow-cyan-600/30 transition active:scale-95 cursor-pointer disabled:opacity-50"
            >
              {{ isProcessing ? 'Memverifikasi...' : 'Pindai & Cocokkan' }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
