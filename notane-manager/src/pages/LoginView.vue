<script setup lang="ts">
import { ref } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useAuthStore } from '../stores/authStore'
import {
  ShieldCheck,
  Lock,
  User,
  KeyRound,
  Eye,
  EyeOff,
  Radio,
  AlertCircle,
  Sparkles,
} from 'lucide-vue-next'

const router = useRouter()
const route = useRoute()
const authStore = useAuthStore()

const activeTab = ref<'password' | 'pin'>('password')
const username = ref('admin')
const password = ref('')
const pin = ref('')
const showPassword = ref(false)
const isLoading = ref(false)
const errorMessage = ref('')

async function handlePasswordLogin() {
  errorMessage.value = ''
  if (!username.value.trim() || !password.value.trim()) {
    errorMessage.value = 'Silakan masukkan username dan kata sandi'
    return
  }

  isLoading.value = true
  const res = await authStore.login(username.value, password.value)
  isLoading.value = false

  if (res.success) {
    const redirect = (route.query.redirect as string) || '/'
    router.replace(redirect)
  } else {
    errorMessage.value = res.message || 'Username atau password salah'
  }
}

async function handlePinLogin() {
  errorMessage.value = ''
  if (!pin.value.trim() || pin.value.trim().length < 4) {
    errorMessage.value = 'Masukkan PIN keamanan minimal 4 digit'
    return
  }

  isLoading.value = true
  const res = await authStore.loginWithPin(pin.value)
  isLoading.value = false

  if (res.success) {
    const redirect = (route.query.redirect as string) || '/'
    router.replace(redirect)
  } else {
    errorMessage.value = res.message || 'PIN keamanan salah'
  }
}

function handlePinKeypad(num: string) {
  if (pin.value.length < 10) {
    pin.value += num
  }
}

function handlePinBackspace() {
  pin.value = pin.value.slice(0, -1)
}

function handlePinClear() {
  pin.value = ''
}
</script>

<template>
  <div class="min-h-screen w-screen flex items-center justify-center p-4 bg-slate-950 relative overflow-hidden select-none">
    <!-- Ambient glowing backgrounds -->
    <div class="absolute -top-40 -left-40 w-96 h-96 bg-brand-500/10 rounded-full blur-3xl pointer-events-none"></div>
    <div class="absolute -bottom-40 -right-40 w-96 h-96 bg-emerald-500/10 rounded-full blur-3xl pointer-events-none"></div>
    <div class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[600px] h-[600px] bg-slate-900/40 rounded-full blur-3xl pointer-events-none"></div>

    <div class="w-full max-w-md relative z-10 space-y-6">
      <!-- App Brand Header -->
      <div class="text-center space-y-2">
        <div class="inline-flex items-center justify-center w-14 h-14 rounded-2xl bg-gradient-to-tr from-brand-600 to-emerald-400 shadow-xl shadow-brand-500/25 mb-1 border border-brand-400/40">
          <Radio class="w-7 h-7 text-white animate-pulse" />
        </div>
        <h1 class="text-xl font-extrabold text-white tracking-tight flex items-center justify-center gap-2">
          <span>NOTANE</span>
          <span class="text-xs bg-brand-500/20 text-brand-400 border border-brand-500/30 px-2 py-0.5 rounded font-mono font-semibold">
            PROVIDER
          </span>
        </h1>
        <p class="text-xs text-slate-400 max-w-xs mx-auto">
          Notane License & Server Management Studio
        </p>
      </div>

      <!-- Main Login Glassmorphic Card -->
      <div class="p-7 rounded-3xl glass-panel border border-slate-800 bg-slate-900/80 shadow-2xl space-y-5">
        <!-- Auth Method Tabs -->
        <div class="grid grid-cols-2 p-1 rounded-xl bg-slate-950 border border-slate-800 text-xs font-semibold">
          <button
            type="button"
            @click="activeTab = 'password'"
            class="py-2 rounded-lg transition-all flex items-center justify-center gap-1.5"
            :class="activeTab === 'password' ? 'bg-brand-600 text-white shadow-sm' : 'text-slate-400 hover:text-slate-200'"
          >
            <Lock class="w-3.5 h-3.5" />
            <span>Kata Sandi</span>
          </button>
          <button
            type="button"
            @click="activeTab = 'pin'"
            class="py-2 rounded-lg transition-all flex items-center justify-center gap-1.5"
            :class="activeTab === 'pin' ? 'bg-brand-600 text-white shadow-sm' : 'text-slate-400 hover:text-slate-200'"
          >
            <KeyRound class="w-3.5 h-3.5" />
            <span>Quick PIN</span>
          </button>
        </div>

        <!-- Error Alert -->
        <div v-if="errorMessage" class="p-3 rounded-xl bg-rose-500/15 border border-rose-500/30 text-rose-300 text-xs flex items-center gap-2 animate-in fade-in duration-150">
          <AlertCircle class="w-4 h-4 shrink-0 text-rose-400" />
          <span>{{ errorMessage }}</span>
        </div>

        <!-- Tab 1: Username & Password -->
        <form v-if="activeTab === 'password'" @submit.prevent="handlePasswordLogin" class="space-y-4">
          <div>
            <label class="block text-xs font-semibold text-slate-300 mb-1">Username Administrator</label>
            <div class="relative">
              <User class="w-4 h-4 text-slate-500 absolute left-3 top-1/2 -translate-y-1/2" />
              <input
                v-model="username"
                type="text"
                autocomplete="username"
                placeholder="admin"
                class="w-full pl-9 pr-4 py-2.5 bg-slate-950 border border-slate-800 rounded-xl text-xs text-white placeholder-slate-600 focus:outline-none focus:border-brand-500 font-mono"
              />
            </div>
          </div>

          <div>
            <label class="block text-xs font-semibold text-slate-300 mb-1">Kata Sandi</label>
            <div class="relative">
              <Lock class="w-4 h-4 text-slate-500 absolute left-3 top-1/2 -translate-y-1/2" />
              <input
                v-model="password"
                :type="showPassword ? 'text' : 'password'"
                autocomplete="current-password"
                placeholder="Masukkan kata sandi..."
                class="w-full pl-9 pr-10 py-2.5 bg-slate-950 border border-slate-800 rounded-xl text-xs text-white placeholder-slate-600 focus:outline-none focus:border-brand-500 font-mono"
              />
              <button
                type="button"
                @click="showPassword = !showPassword"
                class="absolute right-3 top-1/2 -translate-y-1/2 text-slate-500 hover:text-slate-300 p-1"
              >
                <component :is="showPassword ? EyeOff : Eye" class="w-3.5 h-3.5" />
              </button>
            </div>
          </div>

          <button
            type="submit"
            :disabled="isLoading"
            class="w-full py-2.5 bg-brand-600 hover:bg-brand-500 disabled:opacity-50 text-white rounded-xl text-xs font-bold transition-all shadow-lg shadow-brand-600/30 flex items-center justify-center gap-2"
          >
            <ShieldCheck class="w-4 h-4" />
            <span>{{ isLoading ? 'Memverifikasi Akses...' : 'Masuk ke Studio Provider' }}</span>
          </button>
        </form>

        <!-- Tab 2: Quick PIN Unlock -->
        <form v-else @submit.prevent="handlePinLogin" class="space-y-4">
          <div>
            <label class="block text-xs font-semibold text-slate-300 mb-1 text-center">Masukkan 6-Digit PIN Keamanan</label>
            <div class="relative max-w-[220px] mx-auto">
              <input
                v-model="pin"
                type="password"
                maxlength="10"
                placeholder="••••••"
                class="w-full text-center py-2.5 bg-slate-950 border border-slate-800 rounded-xl text-base tracking-widest text-brand-400 font-mono focus:outline-none focus:border-brand-500"
              />
            </div>
          </div>

          <!-- Virtual Keypad -->
          <div class="grid grid-cols-3 gap-2 max-w-[220px] mx-auto">
            <button
              v-for="n in ['1','2','3','4','5','6','7','8','9']"
              :key="n"
              type="button"
              @click="handlePinKeypad(n)"
              class="py-2 rounded-xl bg-slate-950 hover:bg-slate-800 border border-slate-800/80 text-sm font-bold text-white transition-colors"
            >
              {{ n }}
            </button>
            <button
              type="button"
              @click="handlePinClear"
              class="py-2 rounded-xl bg-slate-950 hover:bg-rose-500/20 text-rose-400 border border-slate-800/80 text-xs font-semibold"
            >
              C
            </button>
            <button
              type="button"
              @click="handlePinKeypad('0')"
              class="py-2 rounded-xl bg-slate-950 hover:bg-slate-800 border border-slate-800/80 text-sm font-bold text-white"
            >
              0
            </button>
            <button
              type="button"
              @click="handlePinBackspace"
              class="py-2 rounded-xl bg-slate-950 hover:bg-slate-800 border border-slate-800/80 text-xs font-semibold text-slate-400"
            >
              ⌫
            </button>
          </div>

          <button
            type="submit"
            :disabled="isLoading || pin.length < 4"
            class="w-full py-2.5 bg-brand-600 hover:bg-brand-500 disabled:opacity-50 text-white rounded-xl text-xs font-bold transition-all shadow-lg shadow-brand-600/30 flex items-center justify-center gap-2"
          >
            <ShieldCheck class="w-4 h-4" />
            <span>{{ isLoading ? 'Membuka Akses...' : 'Buka Kunci PIN' }}</span>
          </button>
        </form>

        <!-- Default Credentials Hint -->
        <div class="p-3 rounded-xl bg-slate-950/70 border border-slate-800 text-[11px] text-slate-400 space-y-1">
          <div class="flex items-center gap-1.5 text-slate-300 font-semibold">
            <Sparkles class="w-3.5 h-3.5 text-brand-400" />
            <span>Kredensial Bawaan Pertama Kali:</span>
          </div>
          <div class="font-mono text-slate-400">
            Username: <span class="text-brand-300 font-bold">admin</span> &bull; Password: <span class="text-brand-300 font-bold">admin</span>
          </div>
          <div class="font-mono text-slate-400">
            Quick PIN: <span class="text-brand-300 font-bold">123456</span>
          </div>
        </div>
      </div>

      <div class="text-center text-[11px] text-slate-600 font-mono">
        Notane Retail Enterprise Cryptographic Core &bull; Zero Trust
      </div>
    </div>
  </div>
</template>
