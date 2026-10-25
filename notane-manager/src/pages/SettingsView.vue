<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useProviderStore } from '../stores/providerStore'
import { useAuthStore } from '../stores/authStore'
import {
  Settings,
  ShieldCheck,
  KeyRound,
  Server,
  Copy,
  Check,
  Save,
  AlertTriangle,
} from 'lucide-vue-next'

const providerStore = useProviderStore()
const authStore = useAuthStore()
const copiedPriv = ref(false)
const copiedPub = ref(false)
const isSaving = ref(false)

const isUpdatingAuth = ref(false)
const authMessage = ref('')
const authSuccess = ref(false)
const securityForm = ref({
  old_password: '',
  new_password: '',
  new_pin: '',
})

const form = ref({
  server_port: 19090,
  auto_start_server: true,
  master_private_key: '',
  master_public_key: '',
})

onMounted(async () => {
  await providerStore.fetchSettings()
  form.value = {
    server_port: providerStore.settings.server_port,
    auto_start_server: providerStore.settings.auto_start_server,
    master_private_key: providerStore.settings.master_private_key,
    master_public_key: providerStore.settings.master_public_key,
  }
})

async function copyPrivateKey() {
  await navigator.clipboard.writeText(form.value.master_private_key)
  copiedPriv.value = true
  setTimeout(() => (copiedPriv.value = false), 2000)
}

async function copyPublicKey() {
  await navigator.clipboard.writeText(form.value.master_public_key)
  copiedPub.value = true
  setTimeout(() => (copiedPub.value = false), 2000)
}

async function handleSave() {
  isSaving.value = true
  await providerStore.saveSettings(form.value)
  isSaving.value = false
}

async function handleSaveCredentials() {
  authMessage.value = ''
  authSuccess.value = false

  if (!securityForm.value.old_password.trim()) {
    authMessage.value = 'Kata sandi saat ini wajib diisi untuk verifikasi keamanan'
    return
  }

  if (!securityForm.value.new_password && !securityForm.value.new_pin) {
    authMessage.value = 'Silakan isi kata sandi baru atau PIN baru yang ingin diperbarui'
    return
  }

  isUpdatingAuth.value = true
  const res = await authStore.changeCredentials({
    old_password: securityForm.value.old_password.trim(),
    new_password: securityForm.value.new_password ? securityForm.value.new_password.trim() : undefined,
    new_pin: securityForm.value.new_pin ? securityForm.value.new_pin.trim() : undefined,
  })
  isUpdatingAuth.value = false

  if (res.success) {
    authSuccess.value = true
    authMessage.value = 'Kredensial administrator berhasil diperbarui!'
    securityForm.value = { old_password: '', new_password: '', new_pin: '' }
  } else {
    authMessage.value = res.message || 'Gagal memperbarui kredensial'
  }
}
</script>

<template>
  <div class="max-w-3xl space-y-6">
    <div class="p-6 rounded-2xl glass-panel border border-slate-800 space-y-6">
      <div class="flex items-center gap-3 pb-4 border-b border-slate-800">
        <div class="p-2.5 rounded-xl bg-brand-500/10 text-brand-400">
          <Settings class="w-5 h-5" />
        </div>
        <div>
          <h2 class="text-sm font-bold text-white">Konfigurasi Server Layanan</h2>
          <p class="text-xs text-slate-400">Pengaturan port layanan dan kunci keamanan lisensi</p>
        </div>
      </div>

      <div class="space-y-4">
        <!-- Port Server -->
        <div>
          <label class="block text-xs font-semibold text-slate-300 mb-1 flex items-center gap-1.5">
            <Server class="w-3.5 h-3.5 text-blue-400" />
            <span>Port Layanan Server</span>
          </label>
          <input
            v-model.number="form.server_port"
            type="number"
            class="w-full sm:w-48 px-3 py-2 bg-slate-950 border border-slate-800 rounded-xl text-xs font-mono text-white focus:outline-none focus:border-brand-500"
          />
          <span class="text-[11px] text-slate-500 mt-1 block">Default: 19090 (atau sesuaikan dengan konfigurasi VPS).</span>
        </div>

        <!-- Master Private Key (Warning!) -->
        <div class="pt-4 border-t border-slate-800/80 space-y-2">
          <div class="flex items-center justify-between">
            <label class="block text-xs font-semibold text-rose-400 flex items-center gap-1.5">
              <KeyRound class="w-3.5 h-3.5 text-rose-400" />
              <span>Kunci Rahasia Utama (Private Key)</span>
            </label>
            <button
              @click="copyPrivateKey"
              type="button"
              class="px-2 py-1 rounded bg-slate-800 hover:bg-slate-700 text-xs font-mono text-slate-300 flex items-center gap-1"
            >
              <component :is="copiedPriv ? Check : Copy" class="w-3 h-3 text-brand-400" />
              <span>{{ copiedPriv ? 'Tersalin' : 'Salin Kunci' }}</span>
            </button>
          </div>

          <input
            v-model="form.master_private_key"
            type="text"
            class="w-full px-3 py-2 bg-slate-950 border border-rose-500/30 rounded-xl text-xs font-mono text-rose-300 focus:outline-none focus:border-rose-400 select-all"
          />

          <div class="p-3 rounded-xl bg-rose-500/10 border border-rose-500/20 text-xs text-rose-300 flex items-start gap-2">
            <AlertTriangle class="w-4 h-4 shrink-0 mt-0.5" />
            <span>
              <strong>PENTING:</strong> Jaga kerahasiaan kunci ini. Kunci ini digunakan untuk menandatangani seluruh lisensi toko.
            </span>
          </div>
        </div>

        <!-- Master Public Key -->
        <div class="pt-4 border-t border-slate-800/80 space-y-2">
          <div class="flex items-center justify-between">
            <label class="block text-xs font-semibold text-slate-300 flex items-center gap-1.5">
              <ShieldCheck class="w-3.5 h-3.5 text-emerald-400" />
              <span>Kunci Publik Pasangan (Tersimpan di Aplikasi Toko)</span>
            </label>
            <button
              @click="copyPublicKey"
              type="button"
              class="px-2 py-1 rounded bg-slate-800 hover:bg-slate-700 text-xs font-mono text-slate-300 flex items-center gap-1"
            >
              <component :is="copiedPub ? Check : Copy" class="w-3 h-3 text-brand-400" />
              <span>{{ copiedPub ? 'Tersalin' : 'Salin Kunci' }}</span>
            </button>
          </div>

          <input
            readonly
            :value="form.master_public_key"
            type="text"
            class="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-xl text-xs font-mono text-emerald-400 focus:outline-none select-all"
          />
          <span class="text-[11px] text-slate-500 block">
            Public key ini adalah verifier yang tersemat di binary Rust klien Notane POS (<code class="text-slate-400">NOTANE_MASTER_PUBLIC_KEY_HEX</code>).
          </span>
        </div>
      </div>

      <div class="pt-4 border-t border-slate-800 flex justify-end">
        <button
          @click="handleSave"
          :disabled="isSaving"
          class="px-5 py-2.5 bg-brand-600 hover:bg-brand-500 disabled:opacity-50 text-white rounded-xl text-xs font-bold transition-all shadow-md shadow-brand-600/30 flex items-center gap-2"
        >
          <Save class="w-4 h-4" />
          <span>{{ isSaving ? 'Menyimpan...' : 'Simpan Perubahan' }}</span>
        </button>
      </div>
    </div>

    <!-- Security & Administrator Credentials Card -->
    <div class="p-6 rounded-2xl glass-panel border border-slate-800 space-y-5">
      <div class="flex items-center gap-3 pb-4 border-b border-slate-800">
        <div class="p-2.5 rounded-xl bg-purple-500/10 text-purple-400">
          <KeyRound class="w-5 h-5" />
        </div>
        <div>
          <h2 class="text-sm font-bold text-white">Keamanan & Kredensial Administrator</h2>
          <p class="text-xs text-slate-400">Ubah kata sandi akun master dan PIN keamanan quick unlock</p>
        </div>
      </div>

      <div v-if="authMessage" class="p-3 rounded-xl text-xs flex items-center gap-2"
           :class="authSuccess ? 'bg-emerald-500/15 border border-emerald-500/30 text-emerald-300' : 'bg-rose-500/15 border border-rose-500/30 text-rose-300'">
        <component :is="authSuccess ? Check : AlertTriangle" class="w-4 h-4 shrink-0" />
        <span>{{ authMessage }}</span>
      </div>

      <div class="space-y-4">
        <div>
          <label class="block text-xs font-semibold text-slate-300 mb-1">Kata Sandi Saat Ini *</label>
          <input
            v-model="securityForm.old_password"
            type="password"
            placeholder="Masukkan kata sandi lama..."
            class="w-full sm:w-72 px-3 py-2 bg-slate-950 border border-slate-800 rounded-xl text-xs font-mono text-white focus:outline-none focus:border-brand-500"
          />
        </div>

        <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
          <div>
            <label class="block text-xs font-semibold text-slate-300 mb-1">Kata Sandi Baru (Opsional)</label>
            <input
              v-model="securityForm.new_password"
              type="password"
              placeholder="Minimal 4 karakter..."
              class="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-xl text-xs font-mono text-white focus:outline-none focus:border-brand-500"
            />
          </div>

          <div>
            <label class="block text-xs font-semibold text-slate-300 mb-1">PIN Keamanan Baru (Opsional)</label>
            <input
              v-model="securityForm.new_pin"
              type="password"
              maxlength="10"
              placeholder="4-10 digit PIN..."
              class="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-xl text-xs font-mono text-white focus:outline-none focus:border-brand-500"
            />
          </div>
        </div>
      </div>

      <div class="pt-4 border-t border-slate-800 flex justify-end">
        <button
          @click="handleSaveCredentials"
          :disabled="isUpdatingAuth"
          class="px-5 py-2.5 bg-purple-600 hover:bg-purple-500 disabled:opacity-50 text-white rounded-xl text-xs font-bold transition-all shadow-md shadow-purple-600/30 flex items-center gap-2"
        >
          <Save class="w-4 h-4" />
          <span>{{ isUpdatingAuth ? 'Memperbarui...' : 'Perbarui Kredensial' }}</span>
        </button>
      </div>
    </div>
  </div>
</template>
