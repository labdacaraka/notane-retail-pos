<script setup lang="ts">
import { ref } from 'vue'
import { Icon } from '@iconify/vue'
import { api } from '@/services/api'
import { useAuthStore } from '@/stores/authStore'

defineProps<{
  show: boolean
}>()

const emit = defineEmits<{
  (e: 'close'): void
}>()

const authStore = useAuthStore()

type ActiveTab = 'password' | 'pin'
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

function handleClose() {
  pwdError.value = ''
  pwdSuccess.value = ''
  pinError.value = ''
  pinSuccess.value = ''
  emit('close')
}
</script>

<template>
  <Transition name="modal">
    <div
      v-if="show"
      class="fixed inset-0 bg-slate-900/60 backdrop-blur-xs z-50 flex items-center justify-center p-4 animate-fadeIn"
    >
      <div class="bg-white rounded-2xl shadow-2xl w-full max-w-md border border-slate-200 overflow-hidden flex flex-col">
        <!-- Header Profil -->
        <div class="p-5 border-b border-slate-100 flex items-center justify-between bg-slate-50/80">
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

        <!-- Tab Selector: Password vs PIN -->
        <div class="flex border-b border-slate-200 bg-white px-5 pt-3 gap-2">
          <button
            type="button"
            @click="activeTab = 'password'"
            class="flex items-center gap-2 pb-2.5 px-3 text-xs font-bold border-b-2 transition cursor-pointer"
            :class="activeTab === 'password'
              ? 'border-indigo-600 text-indigo-700'
              : 'border-transparent text-slate-500 hover:text-slate-800'"
          >
            <Icon icon="fluent-color:person-key-24" class="w-4 h-4" />
            <span>Ganti Password</span>
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
            <span>Ganti PIN Otorisasi</span>
          </button>
        </div>

        <!-- Content Area -->
        <div class="p-5 flex-1">
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
                class="px-4 py-2 text-xs font-bold text-slate-600 hover:bg-slate-100 rounded-lg transition cursor-pointer"
              >
                Tutup
              </button>
              <button
                type="submit"
                :disabled="pwdSaving"
                class="px-4 py-2 text-xs font-bold bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 text-white rounded-lg shadow-sm border border-indigo-700 transition flex items-center gap-1.5 cursor-pointer"
              >
                <Icon icon="fluent-color:arrow-clockwise-dashes-24" class="w-3.5 h-3.5" />
                <span>{{ pwdSaving ? 'Menyimpan...' : 'Perbarui Password' }}</span>
              </button>
            </div>
          </form>

          <!-- TAB 2: GANTI PIN -->
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
              <label class="block text-xs font-bold text-slate-700 mb-1">
                PIN Lama atau Password Akun <span class="text-rose-500">*</span>
              </label>
              <input
                v-model="pinForm.current_credential"
                type="password"
                placeholder="Masukkan PIN lama atau password akun"
                class="w-full px-3 py-2 text-xs bg-slate-50 border border-slate-300 rounded-lg text-slate-800 focus:bg-white focus:outline-none focus:ring-2 focus:ring-purple-500"
              />
              <p class="text-[11px] text-slate-400 mt-1">Anda dapat memasukkan PIN lama Anda atau password akun untuk verifikasi keamanan.</p>
            </div>

            <div>
              <label class="block text-xs font-bold text-slate-700 mb-1">
                PIN Baru (4-6 Digit Angka) <span class="text-rose-500">*</span>
              </label>
              <input
                v-model="pinForm.new_pin"
                type="password"
                maxlength="6"
                placeholder="Contoh: 123456"
                class="w-full px-3 py-2 text-xs font-mono tracking-widest bg-slate-50 border border-slate-300 rounded-lg text-slate-800 focus:bg-white focus:outline-none focus:ring-2 focus:ring-purple-500"
              />
              <p class="text-[11px] text-slate-400 mt-1">Digunakan untuk otorisasi cepat kasir (Void nota, Diskon manual, Buka Laci kasir).</p>
            </div>

            <div>
              <label class="block text-xs font-bold text-slate-700 mb-1">
                Konfirmasi PIN Baru <span class="text-rose-500">*</span>
              </label>
              <input
                v-model="pinForm.confirm_pin"
                type="password"
                maxlength="6"
                placeholder="Ulangi PIN baru"
                class="w-full px-3 py-2 text-xs font-mono tracking-widest bg-slate-50 border border-slate-300 rounded-lg text-slate-800 focus:bg-white focus:outline-none focus:ring-2 focus:ring-purple-500"
              />
            </div>

            <div class="pt-2 flex justify-end gap-2">
              <button
                type="button"
                @click="handleClose"
                class="px-4 py-2 text-xs font-bold text-slate-600 hover:bg-slate-100 rounded-lg transition cursor-pointer"
              >
                Tutup
              </button>
              <button
                type="submit"
                :disabled="pinSaving"
                class="px-4 py-2 text-xs font-bold bg-purple-600 hover:bg-purple-700 disabled:opacity-50 text-white rounded-lg shadow-sm border border-purple-700 transition flex items-center gap-1.5 cursor-pointer"
              >
                <Icon icon="fluent-color:arrow-clockwise-dashes-24" class="w-3.5 h-3.5" />
                <span>{{ pinSaving ? 'Menyimpan...' : 'Perbarui PIN' }}</span>
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>
  </Transition>
</template>
