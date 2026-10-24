<script setup lang="ts">
import { ref, onUnmounted, nextTick } from 'vue'
import { Icon } from '@iconify/vue'
import { api } from '@/services/api'
import { useAuthStore } from '@/stores/authStore'
import { useBiometric } from '@/composables/useBiometric'

defineProps<{
  show: boolean
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

const authStore = useAuthStore()
const {
  registerFingerprint,
  startCamera,
  stopCamera,
  captureFace,
} = useBiometric()

type ActiveTab = 'password' | 'pin' | 'biometric'
const activeTab = ref<ActiveTab>('password')

// Form Ubah Password Sendiri
const pwdForm = ref({
  current_password: '',
  new_password: '',
  confirm_password: '',
})
const pwdSaving = ref(false)
const pwdError = ref('')
const pwdSuccess = ref('')

// Form Ubah PIN Sendiri
const pinForm = ref({
  current_credential: '', // bisa pin lama atau password
  new_pin: '',
  confirm_pin: '',
})
const pinSaving = ref(false)
const pinError = ref('')
const pinSuccess = ref('')

// Biometric States
const bioSaving = ref(false)
const bioError = ref('')
const bioSuccess = ref('')
const videoRef = ref<HTMLVideoElement | null>(null)
const cameraOpen = ref(false)
const faceCapturedPreview = ref<string | null>(null)
const tempFaceDescriptor = ref<string | null>(null)

async function handleChangePassword() {
  pwdError.value = ''
  pwdSuccess.value = ''

  if (!pwdForm.value.current_password) {
    pwdError.value = 'Password saat ini wajib diisi'
    return
  }
  if (pwdForm.value.new_password.length < 6) {
    pwdError.value = 'Password baru minimal 6 karakter'
    return
  }
  if (pwdForm.value.new_password !== pwdForm.value.confirm_password) {
    pwdError.value = 'Konfirmasi password baru tidak cocok'
    return
  }

  pwdSaving.value = true
  try {
    const res = await api.post('/me/change-password', {
      current_password: pwdForm.value.current_password,
      new_password: pwdForm.value.new_password,
    })
    pwdSuccess.value = res.data?.message || 'Password Anda berhasil diperbarui!'
    pwdForm.value = { current_password: '', new_password: '', confirm_password: '' }
    setTimeout(() => {
      pwdSuccess.value = ''
    }, 3500)
  } catch (err: any) {
    pwdError.value = err.response?.data?.message || err.message || 'Gagal mengubah password'
  } finally {
    pwdSaving.value = false
  }
}

async function handleChangePin() {
  pinError.value = ''
  pinSuccess.value = ''

  const newPin = pinForm.value.new_pin.trim()
  const confirmPin = pinForm.value.confirm_pin.trim()

  if (!pinForm.value.current_credential) {
    pinError.value = 'PIN lama atau password akun saat ini wajib diisi'
    return
  }
  if (newPin.length < 4 || newPin.length > 6 || !/^\d+$/.test(newPin)) {
    pinError.value = 'PIN baru harus berupa 4-6 digit angka'
    return
  }
  if (newPin !== confirmPin) {
    pinError.value = 'Konfirmasi PIN baru tidak cocok'
    return
  }

  pinSaving.value = true
  try {
    const res = await api.post('/me/change-pin', {
      current_pin: pinForm.value.current_credential,
      current_password: pinForm.value.current_credential,
      new_pin: newPin,
    })
    pinSuccess.value = res.data?.message || 'PIN otorisasi Anda berhasil diperbarui!'
    pinForm.value = { current_credential: '', new_pin: '', confirm_pin: '' }
    setTimeout(() => {
      pinSuccess.value = ''
    }, 3500)
  } catch (err: any) {
    pinError.value = err.response?.data?.message || err.message || 'Gagal mengubah PIN'
  } finally {
    pinSaving.value = false
  }
}

// ----------------------------------------------------------------------------
// BIOMETRIC ACTIONS
// ----------------------------------------------------------------------------

async function handleRegisterFingerprint() {
  bioError.value = ''
  bioSuccess.value = ''
  bioSaving.value = true

  try {
    const user = authStore.user
    if (!user) throw new Error('Pengguna belum login')

    const cred = await registerFingerprint(user.id, user.username, user.full_name)

    const res = await api.post('/me/biometric/enroll', {
      biometric_type: 'FINGERPRINT',
      credential_id: cred.credential_id,
      public_key: cred.public_key,
    })

    if (res.data?.success) {
      bioSuccess.value = res.data.message || 'Sensor sidik jari / Touch ID berhasil didaftarkan!'
      if (authStore.user) {
        authStore.user.has_biometric = true
        authStore.user.biometric_credential_id = cred.credential_id
        localStorage.setItem('notane_user', JSON.stringify(authStore.user))
      }
    }
  } catch (err: any) {
    bioError.value = err.response?.data?.message || err.message || 'Gagal mendaftarkan sidik jari'
  } finally {
    bioSaving.value = false
  }
}

async function handleResetFingerprint() {
  bioError.value = ''
  bioSuccess.value = ''
  bioSaving.value = true

  try {
    const res = await api.post('/me/biometric/reset', {
      biometric_type: 'FINGERPRINT',
    })
    if (res.data?.success) {
      bioSuccess.value = 'Data sensor sidik jari berhasil dihapus'
      if (authStore.user) {
        authStore.user.has_biometric = false
        authStore.user.biometric_credential_id = null
        localStorage.setItem('notane_user', JSON.stringify(authStore.user))
      }
    }
  } catch (err: any) {
    bioError.value = err.response?.data?.message || err.message || 'Gagal mereset sidik jari'
  } finally {
    bioSaving.value = false
  }
}

async function handleStartFaceCamera() {
  bioError.value = ''
  cameraOpen.value = true
  faceCapturedPreview.value = null
  tempFaceDescriptor.value = null

  await nextTick()
  if (videoRef.value) {
    try {
      await startCamera(videoRef.value)
    } catch (err: any) {
      cameraOpen.value = false
      bioError.value = err.message || 'Gagal mengakses kamera'
    }
  }
}

function handleCaptureFaceShot() {
  if (!videoRef.value) return
  try {
    const result = captureFace(videoRef.value)
    faceCapturedPreview.value = result.preview
    tempFaceDescriptor.value = result.descriptor
    stopCamera(videoRef.value)
    cameraOpen.value = false
  } catch (err: any) {
    bioError.value = err.message || 'Gagal memproses gambar wajah'
  }
}

async function handleSaveEnrolledFace() {
  if (!tempFaceDescriptor.value || !faceCapturedPreview.value) {
    bioError.value = 'Silakan ambil foto wajah terlebih dahulu'
    return
  }

  bioSaving.value = true
  bioError.value = ''
  try {
    const res = await api.post('/me/biometric/enroll', {
      biometric_type: 'FACE',
      face_descriptor: tempFaceDescriptor.value,
      face_photo_preview: faceCapturedPreview.value,
    })

    if (res.data?.success) {
      bioSuccess.value = 'Data pengenalan wajah berhasil didaftarkan dan diaktifkan!'
      if (authStore.user) {
        authStore.user.has_face_recognition = true
        authStore.user.face_photo_preview = faceCapturedPreview.value
        localStorage.setItem('notane_user', JSON.stringify(authStore.user))
      }
      faceCapturedPreview.value = null
      tempFaceDescriptor.value = null
    }
  } catch (err: any) {
    bioError.value = err.response?.data?.message || err.message || 'Gagal menyimpan data wajah'
  } finally {
    bioSaving.value = false
  }
}

function handleCancelFaceCamera() {
  stopCamera(videoRef.value || undefined)
  cameraOpen.value = false
  faceCapturedPreview.value = null
  tempFaceDescriptor.value = null
}

async function handleResetFace() {
  bioError.value = ''
  bioSuccess.value = ''
  bioSaving.value = true

  try {
    const res = await api.post('/me/biometric/reset', {
      biometric_type: 'FACE',
    })
    if (res.data?.success) {
      bioSuccess.value = 'Data pengenalan wajah berhasil dihapus'
      if (authStore.user) {
        authStore.user.has_face_recognition = false
        authStore.user.face_photo_preview = null
        localStorage.setItem('notane_user', JSON.stringify(authStore.user))
      }
    }
  } catch (err: any) {
    bioError.value = err.response?.data?.message || err.message || 'Gagal mereset data wajah'
  } finally {
    bioSaving.value = false
  }
}

function handleClose() {
  stopCamera(videoRef.value || undefined)
  cameraOpen.value = false
  pwdError.value = ''
  pwdSuccess.value = ''
  pinError.value = ''
  pinSuccess.value = ''
  bioError.value = ''
  bioSuccess.value = ''
  emit('close')
}

onUnmounted(() => {
  stopCamera(videoRef.value || undefined)
})
</script>

<template>
  <Transition name="modal">
    <div
      v-if="show"
      class="fixed inset-0 bg-slate-900/60 backdrop-blur-xs z-50 flex items-center justify-center p-4 animate-fadeIn"
    >
      <div class="bg-white rounded-2xl shadow-2xl w-full max-w-lg border border-slate-200 overflow-hidden flex flex-col max-h-[90vh]">
        <!-- Header Profil -->
        <div class="p-5 border-b border-slate-100 flex items-center justify-between bg-slate-50/80 shrink-0">
          <div class="flex items-center gap-3">
            <div class="w-11 h-11 rounded-xl bg-gradient-to-br from-indigo-500 to-purple-600 flex items-center justify-center text-white font-extrabold text-base shadow-sm">
              {{ authStore.user?.full_name?.charAt(0) || 'U' }}
            </div>
            <div>
              <h3 class="font-extrabold text-sm text-slate-900 leading-tight">Pengaturan Keamanan Akun</h3>
              <div class="flex items-center gap-1.5 mt-0.5">
                <span class="text-xs text-slate-700 font-semibold">{{ authStore.user?.full_name }}</span>
                <span class="text-[10px] bg-indigo-100 text-indigo-800 font-mono font-bold px-1.5 py-0.2 rounded">
                  {{ authStore.user?.role_name }}
                </span>
              </div>
            </div>
          </div>
          <button
            @click="handleClose"
            class="p-1.5 rounded-lg text-slate-400 hover:text-slate-600 hover:bg-slate-100 transition cursor-pointer"
          >
            <Icon icon="fluent-color:dismiss-circle-24" class="w-5 h-5" />
          </button>
        </div>

        <!-- Tab Selector: Password vs PIN vs Biometrik -->
        <div class="flex border-b border-slate-200 bg-white px-5 pt-3 gap-2 shrink-0">
          <button
            type="button"
            @click="activeTab = 'password'"
            class="flex items-center gap-2 pb-2.5 px-3 text-xs font-bold border-b-2 transition cursor-pointer"
            :class="activeTab === 'password'
              ? 'border-indigo-600 text-indigo-700'
              : 'border-transparent text-slate-500 hover:text-slate-800'"
          >
            <Icon icon="fluent-color:person-key-24" class="w-4 h-4" />
            <span>Password</span>
          </button>

          <button
            type="button"
            @click="activeTab = 'pin'"
            class="flex items-center gap-2 pb-2.5 px-3 text-xs font-bold border-b-2 transition cursor-pointer"
            :class="activeTab === 'pin'
              ? 'border-purple-600 text-purple-700'
              : 'border-transparent text-slate-500 hover:text-slate-800'"
          >
            <Icon icon="streamline-ultimate-color:dial-pad" class="w-4 h-4" />
            <span>PIN Otorisasi</span>
          </button>

          <button
            type="button"
            @click="activeTab = 'biometric'"
            class="flex items-center gap-2 pb-2.5 px-3 text-xs font-bold border-b-2 transition cursor-pointer"
            :class="activeTab === 'biometric'
              ? 'border-amber-600 text-amber-800'
              : 'border-transparent text-slate-500 hover:text-slate-800'"
          >
            <Icon icon="streamline-ultimate-color:fingerprint-id" class="w-4 h-4" />
            <span>Biometrik & Wajah</span>
          </button>
        </div>

        <!-- Content Area -->
        <div class="p-5 flex-1 overflow-y-auto">
          <!-- TAB 1: GANTI PASSWORD -->
          <form v-if="activeTab === 'password'" @submit.prevent="handleChangePassword" class="space-y-4">
            <div v-if="pwdError" class="flex items-center gap-2 bg-rose-50 border border-rose-200 text-rose-700 text-xs p-3 rounded-xl">
              <Icon icon="fluent-color:error-circle-24" class="w-4 h-4 shrink-0" />
              <span>{{ pwdError }}</span>
            </div>

            <div v-if="pwdSuccess" class="flex items-center gap-2 bg-emerald-50 border border-emerald-200 text-emerald-700 text-xs p-3 rounded-xl">
              <Icon icon="fluent-color:checkmark-circle-24" class="w-4 h-4 shrink-0" />
              <span>{{ pwdSuccess }}</span>
            </div>

            <div>
              <label class="block text-xs font-bold text-slate-700 mb-1">Password Saat Ini <span class="text-rose-500">*</span></label>
              <input
                v-model="pwdForm.current_password"
                type="password"
                placeholder="Masukkan password lama"
                class="w-full px-3 py-2 text-xs bg-slate-50 border border-slate-300 rounded-lg text-slate-800 focus:bg-white focus:outline-none focus:ring-2 focus:ring-indigo-500"
              />
            </div>

            <div>
              <label class="block text-xs font-bold text-slate-700 mb-1">Password Baru <span class="text-rose-500">*</span></label>
              <input
                v-model="pwdForm.new_password"
                type="password"
                placeholder="Minimal 6 karakter"
                class="w-full px-3 py-2 text-xs bg-slate-50 border border-slate-300 rounded-lg text-slate-800 focus:bg-white focus:outline-none focus:ring-2 focus:ring-indigo-500"
              />
            </div>

            <div>
              <label class="block text-xs font-bold text-slate-700 mb-1">Konfirmasi Password Baru <span class="text-rose-500">*</span></label>
              <input
                v-model="pwdForm.confirm_password"
                type="password"
                placeholder="Ulangi password baru"
                class="w-full px-3 py-2 text-xs bg-slate-50 border border-slate-300 rounded-lg text-slate-800 focus:bg-white focus:outline-none focus:ring-2 focus:ring-indigo-500"
              />
            </div>

            <div class="pt-2 flex justify-end gap-2">
              <button
                type="button"
                @click="handleClose"
                class="px-4 py-2 text-xs font-bold text-slate-600 hover:bg-slate-100 rounded-lg border border-slate-200 cursor-pointer"
              >
                Batal
              </button>
              <button
                type="submit"
                :disabled="pwdSaving"
                class="px-4 py-2 text-xs font-bold text-white bg-indigo-600 hover:bg-indigo-700 rounded-lg shadow-xs flex items-center gap-1.5 cursor-pointer disabled:opacity-50"
              >
                <Icon v-if="pwdSaving" icon="fluent-color:spinner-ios-20" class="w-4 h-4 animate-spin" />
                <span>{{ pwdSaving ? 'Menyimpan...' : 'Perbarui Password' }}</span>
              </button>
            </div>
          </form>

          <!-- TAB 2: GANTI PIN OTORISASI -->
          <form v-if="activeTab === 'pin'" @submit.prevent="handleChangePin" class="space-y-4">
            <div v-if="pinError" class="flex items-center gap-2 bg-rose-50 border border-rose-200 text-rose-700 text-xs p-3 rounded-xl">
              <Icon icon="fluent-color:error-circle-24" class="w-4 h-4 shrink-0" />
              <span>{{ pinError }}</span>
            </div>

            <div v-if="pinSuccess" class="flex items-center gap-2 bg-emerald-50 border border-emerald-200 text-emerald-700 text-xs p-3 rounded-xl">
              <Icon icon="fluent-color:checkmark-circle-24" class="w-4 h-4 shrink-0" />
              <span>{{ pinSuccess }}</span>
            </div>

            <div>
              <label class="block text-xs font-bold text-slate-700 mb-1">PIN Lama atau Password Akun <span class="text-rose-500">*</span></label>
              <input
                v-model="pinForm.current_credential"
                type="password"
                placeholder="Masukkan PIN lama atau Password login"
                class="w-full px-3 py-2 text-xs bg-slate-50 border border-slate-300 rounded-lg text-slate-800 focus:bg-white focus:outline-none focus:ring-2 focus:ring-purple-500"
              />
            </div>

            <div>
              <label class="block text-xs font-bold text-slate-700 mb-1">PIN Baru (4-6 Digit Angka) <span class="text-rose-500">*</span></label>
              <input
                v-model="pinForm.new_pin"
                type="password"
                maxlength="6"
                placeholder="Contoh: 123456"
                class="w-full px-3 py-2 text-center text-lg tracking-widest font-mono bg-slate-50 border border-slate-300 rounded-lg text-slate-800 focus:bg-white focus:outline-none focus:ring-2 focus:ring-purple-500"
              />
            </div>

            <div>
              <label class="block text-xs font-bold text-slate-700 mb-1">Konfirmasi PIN Baru <span class="text-rose-500">*</span></label>
              <input
                v-model="pinForm.confirm_pin"
                type="password"
                maxlength="6"
                placeholder="Ulangi 4-6 digit PIN"
                class="w-full px-3 py-2 text-center text-lg tracking-widest font-mono bg-slate-50 border border-slate-300 rounded-lg text-slate-800 focus:bg-white focus:outline-none focus:ring-2 focus:ring-purple-500"
              />
            </div>

            <div class="pt-2 flex justify-end gap-2">
              <button
                type="button"
                @click="handleClose"
                class="px-4 py-2 text-xs font-bold text-slate-600 hover:bg-slate-100 rounded-lg border border-slate-200 cursor-pointer"
              >
                Batal
              </button>
              <button
                type="submit"
                :disabled="pinSaving"
                class="px-4 py-2 text-xs font-bold text-white bg-purple-600 hover:bg-purple-700 rounded-lg shadow-xs flex items-center gap-1.5 cursor-pointer disabled:opacity-50"
              >
                <Icon v-if="pinSaving" icon="fluent-color:spinner-ios-20" class="w-4 h-4 animate-spin" />
                <span>{{ pinSaving ? 'Menyimpan...' : 'Perbarui PIN' }}</span>
              </button>
            </div>
          </form>

          <!-- TAB 3: BIOMETRIK & WAJAH SUPERVISOR -->
          <div v-if="activeTab === 'biometric'" class="space-y-4">
            <div v-if="bioError" class="flex items-center gap-2 bg-rose-50 border border-rose-200 text-rose-700 text-xs p-3 rounded-xl">
              <Icon icon="fluent-color:error-circle-24" class="w-4 h-4 shrink-0" />
              <span>{{ bioError }}</span>
            </div>

            <div v-if="bioSuccess" class="flex items-center gap-2 bg-emerald-50 border border-emerald-200 text-emerald-700 text-xs p-3 rounded-xl">
              <Icon icon="fluent-color:checkmark-circle-24" class="w-4 h-4 shrink-0" />
              <span>{{ bioSuccess }}</span>
            </div>

            <p class="text-[11px] text-slate-600 leading-relaxed">
              Daftarkan sidik jari atau pemindai wajah Anda untuk memproses persetujuan otorisasi supervisor di kasir tanpa perlu mengetik PIN manual.
            </p>

            <!-- CARD 1: SENSOR SIDIK JARI -->
            <div class="p-4 rounded-xl border border-slate-200 bg-slate-50/70 space-y-3">
              <div class="flex items-center justify-between">
                <div class="flex items-center gap-2.5">
                  <div class="w-8 h-8 rounded-lg bg-emerald-100 text-emerald-700 flex items-center justify-center">
                    <Icon icon="streamline-ultimate-color:fingerprint-id" class="w-4 h-4" />
                  </div>
                  <div>
                    <h4 class="font-bold text-xs text-slate-900">Sensor Sidik Jari (Touch ID / Windows Hello)</h4>
                    <p class="text-[10px] text-slate-500">Mendukung platform authenticator lokal OS</p>
                  </div>
                </div>

                <span
                  class="text-[10px] font-bold px-2 py-0.5 rounded-full border flex items-center gap-1"
                  :class="authStore.user?.has_biometric ? 'bg-emerald-100 text-emerald-800 border-emerald-300' : 'bg-slate-200 text-slate-600 border-slate-300'"
                >
                  <span class="w-1.5 h-1.5 rounded-full" :class="authStore.user?.has_biometric ? 'bg-emerald-600' : 'bg-slate-400'"></span>
                  <span>{{ authStore.user?.has_biometric ? 'Aktif' : 'Belum Terdaftar' }}</span>
                </span>
              </div>

              <div class="flex items-center gap-2 pt-1">
                <button
                  type="button"
                  @click="handleRegisterFingerprint"
                  :disabled="bioSaving"
                  class="px-3 py-1.5 bg-emerald-600 hover:bg-emerald-700 text-white rounded-lg text-xs font-bold shadow-2xs flex items-center gap-1.5 cursor-pointer disabled:opacity-50 transition"
                >
                  <Icon icon="flat-color-icons:ok" class="w-3.5 h-3.5" />
                  <span>{{ authStore.user?.has_biometric ? 'Daftarkan Ulang Sidik Jari' : 'Daftarkan Sidik Jari Sekarang' }}</span>
                </button>

                <button
                  v-if="authStore.user?.has_biometric"
                  type="button"
                  @click="handleResetFingerprint"
                  :disabled="bioSaving"
                  class="px-3 py-1.5 bg-rose-50 hover:bg-rose-100 text-rose-700 border border-rose-200 rounded-lg text-xs font-bold cursor-pointer disabled:opacity-50 transition"
                >
                  Hapus
                </button>
              </div>
            </div>

            <!-- CARD 2: PENGENALAN WAJAH (KAMERA AI) -->
            <div class="p-4 rounded-xl border border-slate-200 bg-slate-50/70 space-y-3">
              <div class="flex items-center justify-between">
                <div class="flex items-center gap-2.5">
                  <div class="w-8 h-8 rounded-lg bg-blue-100 text-blue-700 flex items-center justify-center">
                    <Icon icon="fluent-color:camera-24" class="w-4 h-4" />
                  </div>
                  <div>
                    <h4 class="font-bold text-xs text-slate-900">Pemindai Wajah (Face Recognition)</h4>
                    <p class="text-[10px] text-slate-500">Mendeteksi profil wajah via webcam perangkat</p>
                  </div>
                </div>

                <div class="flex items-center gap-2">
                  <img
                    v-if="authStore.user?.face_photo_preview"
                    :src="authStore.user.face_photo_preview"
                    alt="Face Avatar"
                    class="w-7 h-7 rounded-full object-cover border border-blue-400 shadow-2xs"
                  />
                  <span
                    class="text-[10px] font-bold px-2 py-0.5 rounded-full border flex items-center gap-1"
                    :class="authStore.user?.has_face_recognition ? 'bg-blue-100 text-blue-800 border-blue-300' : 'bg-slate-200 text-slate-600 border-slate-300'"
                  >
                    <span class="w-1.5 h-1.5 rounded-full" :class="authStore.user?.has_face_recognition ? 'bg-blue-600' : 'bg-slate-400'"></span>
                    <span>{{ authStore.user?.has_face_recognition ? 'Aktif' : 'Belum Terdaftar' }}</span>
                  </span>
                </div>
              </div>

              <!-- Live Camera Viewfinder Modal/Box -->
              <div v-if="cameraOpen" class="space-y-3 pt-2">
                <div class="relative w-full aspect-4/3 bg-black rounded-xl overflow-hidden shadow-inner flex items-center justify-center border-2 border-blue-500">
                  <video ref="videoRef" class="w-full h-full object-cover transform scale-x-[-1]"></video>
                  <!-- Circular Face Guide Reticle -->
                  <div class="absolute inset-0 pointer-events-none flex items-center justify-center">
                    <div class="w-44 h-44 rounded-full border-2 border-dashed border-blue-400/80 animate-pulse flex items-center justify-center">
                      <div class="w-40 h-40 rounded-full border border-blue-300/40"></div>
                    </div>
                  </div>
                  <div class="absolute bottom-2 inset-x-0 text-center pointer-events-none">
                    <span class="bg-black/60 text-white text-[10px] px-2.5 py-1 rounded-full font-medium">
                      Posisikan wajah di dalam lingkaran lalu tekan tombol Ambil Foto
                    </span>
                  </div>
                </div>

                <div class="flex items-center justify-center gap-2">
                  <button
                    type="button"
                    @click="handleCaptureFaceShot"
                    class="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg text-xs font-bold shadow-xs flex items-center gap-1.5 cursor-pointer active:scale-95"
                  >
                    <Icon icon="fluent-color:camera-24" class="w-4 h-4" />
                    <span>Ambil Foto Wajah</span>
                  </button>
                  <button
                    type="button"
                    @click="handleCancelFaceCamera"
                    class="px-3 py-2 bg-slate-200 hover:bg-slate-300 text-slate-800 rounded-lg text-xs font-bold cursor-pointer"
                  >
                    Batal
                  </button>
                </div>
              </div>

              <!-- Preview Hasil Foto yang Baru Diambil -->
              <div v-else-if="faceCapturedPreview" class="flex items-center gap-4 p-3 bg-blue-50/70 border border-blue-200 rounded-xl">
                <img :src="faceCapturedPreview" alt="Captured Face" class="w-16 h-16 rounded-xl object-cover border-2 border-blue-500 shadow-xs" />
                <div class="flex-1 space-y-1">
                  <div class="font-bold text-xs text-blue-950">Foto Wajah Berhasil Diambil</div>
                  <div class="text-[10px] text-blue-700 leading-tight">Pastikan foto jelas dan menghadap lurus ke depan.</div>
                  <div class="flex items-center gap-2 pt-1">
                    <button
                      type="button"
                      @click="handleSaveEnrolledFace"
                      :disabled="bioSaving"
                      class="px-3 py-1.5 bg-blue-600 hover:bg-blue-700 text-white rounded-lg text-xs font-bold shadow-2xs flex items-center gap-1 cursor-pointer disabled:opacity-50"
                    >
                      <Icon v-if="bioSaving" icon="fluent-color:spinner-ios-20" class="w-3.5 h-3.5 animate-spin" />
                      <span>{{ bioSaving ? 'Menyimpan...' : 'Simpan & Aktifkan Wajah' }}</span>
                    </button>
                    <button
                      type="button"
                      @click="handleStartFaceCamera"
                      class="px-2.5 py-1.5 bg-slate-200 hover:bg-slate-300 text-slate-800 rounded-lg text-xs font-medium cursor-pointer"
                    >
                      Ulangi Foto
                    </button>
                  </div>
                </div>
              </div>

              <!-- Tombol Standar Mulai Kamera -->
              <div v-else class="flex items-center gap-2 pt-1">
                <button
                  type="button"
                  @click="handleStartFaceCamera"
                  class="px-3 py-1.5 bg-blue-600 hover:bg-blue-700 text-white rounded-lg text-xs font-bold shadow-2xs flex items-center gap-1.5 cursor-pointer transition active:scale-95"
                >
                  <Icon icon="fluent-color:camera-24" class="w-3.5 h-3.5" />
                  <span>{{ authStore.user?.has_face_recognition ? 'Pindai Ulang Wajah' : 'Buka Kamera & Daftarkan Wajah' }}</span>
                </button>

                <button
                  v-if="authStore.user?.has_face_recognition"
                  type="button"
                  @click="handleResetFace"
                  :disabled="bioSaving"
                  class="px-3 py-1.5 bg-rose-50 hover:bg-rose-100 text-rose-700 border border-rose-200 rounded-lg text-xs font-bold cursor-pointer disabled:opacity-50 transition"
                >
                  Hapus
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </Transition>
</template>
