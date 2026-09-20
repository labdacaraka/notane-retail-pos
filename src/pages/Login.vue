<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { Lock, User, Terminal, ArrowRight } from 'lucide-vue-next'
import { useAuthStore } from '@/stores/authStore'

const router = useRouter()
const authStore = useAuthStore()

const username = ref<string>('admin')
const password = ref<string>('admin123')
const terminalId = ref<string>(authStore.terminalId || 'KASIR-01')
const isLoggingIn = ref<boolean>(false)
const errorMsg = ref<string>('')

async function handleLogin() {
  isLoggingIn.value = true
  errorMsg.value = ''

  try {
    await authStore.login(username.value, password.value, terminalId.value)
    router.push('/')
  } catch (err: any) {
    errorMsg.value = err.response?.data?.message || err.message || 'Username atau password salah'
  } finally {
    isLoggingIn.value = false
  }
}

function quickFill(user: string, pass: string) {
  username.value = user
  password.value = pass
}
</script>

<template>
  <div class="h-screen w-screen bg-slate-100 flex items-center justify-center p-4 relative overflow-hidden select-none">
    <!-- Subtle ambient desktop gradients -->
    <div class="absolute -top-40 -left-40 w-96 h-96 bg-lime-400/10 rounded-full blur-3xl pointer-events-none"></div>
    <div class="absolute -bottom-40 -right-40 w-96 h-96 bg-teal-400/10 rounded-full blur-3xl pointer-events-none"></div>

    <!-- Login Card -->
    <div class="w-full max-w-md bg-white border border-slate-300 rounded-2xl p-8 shadow-xl space-y-6 relative z-10">
      <!-- Brand & Heading -->
      <div class="text-center space-y-3">
        <div class="relative inline-block">
          <img src="/logo.png" alt="Notane Logo" class="w-20 h-20 rounded-2xl shadow-md mx-auto object-contain border border-slate-200 hover:scale-105 transition" />
        </div>
        <div>
          <h1 class="text-2xl font-black text-slate-900 tracking-tight">
            NOTANE <span class="text-notane-gradient">RETAIL POS</span>
          </h1>
          <p class="text-xs text-slate-600 mt-1">Sistem Point of Sales & ERP Terpadu Skala Enterprise</p>
        </div>
      </div>

      <!-- Form -->
      <form @submit.prevent="handleLogin" class="space-y-4">
        <!-- Terminal ID Selector -->
        <div>
          <label class="block text-xs font-bold text-slate-700 mb-1.5">Pilih Terminal Mesin</label>
          <div class="relative">
            <Terminal class="w-4 h-4 text-slate-400 absolute left-3.5 top-1/2 -translate-y-1/2" />
            <select
              v-model="terminalId"
              class="w-full bg-slate-50 border border-slate-300 rounded-lg pl-10 pr-4 py-2.5 text-xs text-slate-900 font-mono focus:bg-white focus:border-emerald-600 focus:outline-none"
            >
              <option value="KASIR-01">KASIR-01 (Mesin Kasir Utama)</option>
              <option value="KASIR-02">KASIR-02 (Mesin Kasir Sekunder)</option>
              <option value="BACKOFFICE-01">BACKOFFICE-01 (Server Toko)</option>
            </select>
          </div>
        </div>

        <!-- Username -->
        <div>
          <label class="block text-xs font-bold text-slate-700 mb-1.5">Nama Pengguna (Username)</label>
          <div class="relative">
            <User class="w-4 h-4 text-slate-400 absolute left-3.5 top-1/2 -translate-y-1/2" />
            <input
              v-model="username"
              type="text"
              required
              class="w-full bg-slate-50 border border-slate-300 rounded-lg pl-10 pr-4 py-2.5 text-sm text-slate-900 focus:bg-white focus:border-emerald-600 focus:outline-none"
            />
          </div>
        </div>

        <!-- Password -->
        <div>
          <label class="block text-xs font-bold text-slate-700 mb-1.5">Kata Sandi (Password)</label>
          <div class="relative">
            <Lock class="w-4 h-4 text-slate-400 absolute left-3.5 top-1/2 -translate-y-1/2" />
            <input
              v-model="password"
              type="password"
              required
              class="w-full bg-slate-50 border border-slate-300 rounded-lg pl-10 pr-4 py-2.5 text-sm text-slate-900 font-mono focus:bg-white focus:border-emerald-600 focus:outline-none"
            />
          </div>
        </div>

        <!-- Error Message -->
        <div v-if="errorMsg" class="bg-rose-50 border border-rose-300 p-3 rounded-lg text-rose-700 text-xs font-medium">
          ⚠️ {{ errorMsg }}
        </div>

        <!-- Submit Button -->
        <button
          type="submit"
          :disabled="isLoggingIn"
          class="w-full bg-emerald-600 hover:bg-emerald-700 text-white font-bold py-3 rounded-lg text-sm transition active:scale-95 shadow-sm border border-emerald-700 flex items-center justify-center gap-2"
        >
          <span>{{ isLoggingIn ? 'Memverifikasi...' : 'MASUK KE SISTEM' }}</span>
          <ArrowRight class="w-4 h-4" />
        </button>
      </form>

      <!-- Quick Demo Users -->
      <div class="pt-3 border-t border-slate-200">
        <div class="text-[11px] font-bold text-slate-500 text-center mb-2">Akun Uji Coba Cepat:</div>
        <div class="grid grid-cols-3 gap-2">
          <button
            @click="quickFill('admin', 'admin123')"
            class="bg-slate-50 hover:bg-slate-100 border border-slate-300 p-2 rounded-lg text-center transition active:scale-95"
          >
            <div class="text-[11px] font-bold text-slate-900">Admin</div>
            <div class="text-[9px] text-slate-500 font-mono">admin123</div>
          </button>
          <button
            @click="quickFill('spv', 'spv123')"
            class="bg-slate-50 hover:bg-slate-100 border border-slate-300 p-2 rounded-lg text-center transition active:scale-95"
          >
            <div class="text-[11px] font-bold text-slate-900">Supervisor</div>
            <div class="text-[9px] text-slate-500 font-mono">spv123</div>
          </button>
          <button
            @click="quickFill('kasir1', 'kasir123')"
            class="bg-slate-50 hover:bg-slate-100 border border-slate-300 p-2 rounded-lg text-center transition active:scale-95"
          >
            <div class="text-[11px] font-bold text-slate-900">Kasir 1</div>
            <div class="text-[9px] text-slate-500 font-mono">kasir123</div>
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
