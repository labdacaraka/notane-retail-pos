<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRouter, useRoute } from 'vue-router'
import { useAuthStore } from '../stores/authStore'
import {
  ShieldCheck,
  Lock,
  User,
  Eye,
  EyeOff,
  Radio,
  AlertCircle,
  RefreshCw,
  ShieldAlert,
} from 'lucide-vue-next'

const router = useRouter()
const route = useRoute()
const authStore = useAuthStore()

const username = ref('')
const password = ref('')
const showPassword = ref(false)
const isLoading = ref(false)
const errorMessage = ref('')

// Captcha state
const captchaInput = ref('')
const captchaCode = ref('')
const captchaChars = ref<{ char: string; x: number; y: number; rot: number; color: string }[]>([])
const captchaLines = ref<{ x1: number; y1: number; x2: number; y2: number; color: string }[]>([])
const failedAttempts = ref(0)
const lockoutSeconds = ref(0)
let lockoutTimer: any = null

const CAPTCHA_CHARS = '23456789ABCDEFGHJKLMNPQRSTUVWXYZ'
const COLORS = ['#2563eb', '#4f46e5', '#059669', '#d97706', '#7c3aed', '#dc2626']

function generateCaptcha() {
  captchaInput.value = ''
  let code = ''
  const chars: { char: string; x: number; y: number; rot: number; color: string }[] = []

  for (let i = 0; i < 5; i++) {
    const c = CAPTCHA_CHARS[Math.floor(Math.random() * CAPTCHA_CHARS.length)]
    code += c
    chars.push({
      char: c,
      x: 22 + i * 26 + (Math.random() * 6 - 3),
      y: 30 + (Math.random() * 6 - 3),
      rot: Math.random() * 30 - 15,
      color: COLORS[Math.floor(Math.random() * COLORS.length)],
    })
  }

  const lines: { x1: number; y1: number; x2: number; y2: number; color: string }[] = []
  for (let i = 0; i < 4; i++) {
    lines.push({
      x1: Math.random() * 20,
      y1: Math.random() * 45,
      x2: 130 + Math.random() * 30,
      y2: Math.random() * 45,
      color: COLORS[Math.floor(Math.random() * COLORS.length)],
    })
  }

  captchaCode.value = code
  captchaChars.value = chars
  captchaLines.value = lines
}

onMounted(() => {
  generateCaptcha()
})

function startLockout(seconds: number) {
  lockoutSeconds.value = seconds
  clearInterval(lockoutTimer)
  lockoutTimer = setInterval(() => {
    lockoutSeconds.value--
    if (lockoutSeconds.value <= 0) {
      clearInterval(lockoutTimer)
      failedAttempts.value = 0
    }
  }, 1000)
}

async function handleLogin() {
  errorMessage.value = ''

  if (lockoutSeconds.value > 0) {
    errorMessage.value = `Terlalu banyak percobaan gagal. Silakan tunggu ${lockoutSeconds.value} detik.`
    return
  }

  if (!username.value.trim() || !password.value.trim()) {
    errorMessage.value = 'Silakan masukkan username dan kata sandi'
    return
  }

  if (!captchaInput.value.trim()) {
    errorMessage.value = 'Silakan masukkan kode Captcha verifikasi'
    return
  }

  if (captchaInput.value.trim().toUpperCase() !== captchaCode.value.toUpperCase()) {
    failedAttempts.value++
    errorMessage.value = 'Kode Captcha salah. Silakan coba lagi.'
    generateCaptcha()
    if (failedAttempts.value >= 5) {
      startLockout(30)
      errorMessage.value = 'Terlalu banyak percobaan gagal. Akun dikunci sementara selama 30 detik.'
    }
    return
  }

  isLoading.value = true
  try {
    const res = await authStore.login(username.value.trim(), password.value)
    if (res.success) {
      failedAttempts.value = 0
      const redirect = (route.query.redirect as string) || '/'
      router.replace(redirect)
    } else {
      failedAttempts.value++
      errorMessage.value = res.message || 'Username atau password tidak sesuai'
      generateCaptcha()
      if (failedAttempts.value >= 5) {
        startLockout(30)
        errorMessage.value = 'Terlalu banyak percobaan gagal. Akun dikunci sementara selama 30 detik.'
      }
    }
  } catch (err: any) {
    errorMessage.value = err?.message || 'Gagal menghubungi server'
    generateCaptcha()
  } finally {
    isLoading.value = false
  }
}
</script>

<template>
  <div class="min-h-screen w-screen flex items-center justify-center p-4 bg-slate-950 relative overflow-hidden select-none">
    <!-- Ambient glowing backgrounds -->
    <div class="absolute -top-40 -left-40 w-96 h-96 bg-brand-500/10 rounded-full blur-3xl pointer-events-none"></div>
    <div class="absolute -bottom-40 -right-40 w-96 h-96 bg-indigo-500/10 rounded-full blur-3xl pointer-events-none"></div>
    <div class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[600px] h-[600px] bg-slate-900/40 rounded-full blur-3xl pointer-events-none"></div>

    <div class="w-full max-w-md relative z-10 space-y-6">
      <!-- App Brand Header -->
      <div class="text-center space-y-2">
        <div class="inline-flex items-center justify-center w-14 h-14 rounded-2xl bg-gradient-to-tr from-brand-600 to-indigo-500 shadow-xl shadow-brand-500/25 mb-1 border border-brand-400/40">
          <Radio class="w-7 h-7 text-white" />
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

      <!-- Main Login Card -->
      <div class="p-7 rounded-3xl border border-slate-800 bg-slate-900/90 backdrop-blur-xl shadow-2xl space-y-5">
        <div class="pb-2 border-b border-slate-800/80">
          <h2 class="text-sm font-bold text-white flex items-center gap-2">
            <Lock class="w-4 h-4 text-brand-400" />
            <span>Autentikasi Administrator</span>
          </h2>
          <p class="text-xs text-slate-400 mt-0.5">
            Masuk dengan kredensial akun dan verifikasi keamanan captcha.
          </p>
        </div>

        <!-- Lockout Warning -->
        <div v-if="lockoutSeconds > 0" class="p-3 rounded-xl bg-amber-500/15 border border-amber-500/30 text-amber-300 text-xs flex items-center gap-2">
          <ShieldAlert class="w-4 h-4 shrink-0 text-amber-400" />
          <span>Akses dikunci sementara. Mohon tunggu <strong>{{ lockoutSeconds }} detik</strong> sebelum mencoba lagi.</span>
        </div>

        <!-- Error Alert -->
        <div v-else-if="errorMessage" class="p-3 rounded-xl bg-rose-500/15 border border-rose-500/30 text-rose-300 text-xs flex items-center gap-2 animate-in fade-in duration-150">
          <AlertCircle class="w-4 h-4 shrink-0 text-rose-400" />
          <span>{{ errorMessage }}</span>
        </div>

        <!-- Login Form -->
        <form @submit.prevent="handleLogin" class="space-y-4">
          <!-- Username -->
          <div>
            <label class="block text-xs font-semibold text-slate-300 mb-1.5">Username</label>
            <div class="relative">
              <User class="w-4 h-4 text-slate-500 absolute left-3.5 top-1/2 -translate-y-1/2" />
              <input
                v-model="username"
                type="text"
                autocomplete="username"
                placeholder="Masukkan username admin..."
                :disabled="lockoutSeconds > 0"
                class="w-full pl-10 pr-4 py-2.5 bg-slate-950/80 border border-slate-800 rounded-xl text-xs text-white placeholder-slate-500 focus:outline-none focus:border-brand-500 focus:ring-1 focus:ring-brand-500 font-mono transition"
              />
            </div>
          </div>

          <!-- Password -->
          <div>
            <label class="block text-xs font-semibold text-slate-300 mb-1.5">Kata Sandi</label>
            <div class="relative">
              <Lock class="w-4 h-4 text-slate-500 absolute left-3.5 top-1/2 -translate-y-1/2" />
              <input
                v-model="password"
                :type="showPassword ? 'text' : 'password'"
                autocomplete="current-password"
                placeholder="Masukkan kata sandi..."
                :disabled="lockoutSeconds > 0"
                class="w-full pl-10 pr-10 py-2.5 bg-slate-950/80 border border-slate-800 rounded-xl text-xs text-white placeholder-slate-500 focus:outline-none focus:border-brand-500 focus:ring-1 focus:ring-brand-500 font-mono transition"
              />
              <button
                type="button"
                @click="showPassword = !showPassword"
                class="absolute right-3 top-1/2 -translate-y-1/2 text-slate-500 hover:text-slate-300 p-1 cursor-pointer"
              >
                <component :is="showPassword ? EyeOff : Eye" class="w-4 h-4" />
              </button>
            </div>
          </div>

          <!-- Captcha Challenge -->
          <div>
            <div class="flex items-center justify-between mb-1.5">
              <label class="block text-xs font-semibold text-slate-300">
                Verifikasi Keamanan Captcha <span class="text-rose-500">*</span>
              </label>
              <button
                type="button"
                @click="generateCaptcha"
                class="text-xs text-brand-400 hover:text-brand-300 flex items-center gap-1 cursor-pointer"
                title="Ganti Kode Captcha"
              >
                <RefreshCw class="w-3 h-3" />
                <span>Muat Ulang</span>
              </button>
            </div>

            <div class="flex items-center gap-3">
              <!-- Captcha SVG Canvas -->
              <div class="relative w-40 h-11 shrink-0 rounded-xl overflow-hidden border border-slate-700/80 bg-slate-950 shadow-inner flex items-center justify-center select-none">
                <svg viewBox="0 0 160 45" class="w-full h-full">
                  <!-- Background grid pattern -->
                  <defs>
                    <pattern id="captcha-grid" width="10" height="10" patternUnits="userSpaceOnUse">
                      <path d="M 10 0 L 0 0 0 10" fill="none" stroke="rgba(148, 163, 184, 0.08)" stroke-width="0.8" />
                    </pattern>
                  </defs>
                  <rect width="100%" height="100%" fill="url(#captcha-grid)" />

                  <!-- Noise lines -->
                  <line
                    v-for="(line, idx) in captchaLines"
                    :key="`line-${idx}`"
                    :x1="line.x1"
                    :y1="line.y1"
                    :x2="line.x2"
                    :y2="line.y2"
                    :stroke="line.color"
                    stroke-width="1.2"
                    opacity="0.35"
                  />

                  <!-- Characters -->
                  <text
                    v-for="(item, idx) in captchaChars"
                    :key="`char-${idx}`"
                    :x="item.x"
                    :y="item.y"
                    :fill="item.color"
                    :transform="`rotate(${item.rot}, ${item.x}, ${item.y})`"
                    font-size="22"
                    font-family="monospace"
                    font-weight="bold"
                    letter-spacing="2"
                  >
                    {{ item.char }}
                  </text>
                </svg>
              </div>

              <!-- Captcha Input -->
              <input
                v-model="captchaInput"
                type="text"
                maxlength="5"
                placeholder="Ketik 5 kode..."
                :disabled="lockoutSeconds > 0"
                class="w-full px-3.5 py-2.5 bg-slate-950/80 border border-slate-800 rounded-xl text-xs text-white placeholder-slate-500 uppercase tracking-widest font-mono text-center focus:outline-none focus:border-brand-500 focus:ring-1 focus:ring-brand-500 transition"
              />
            </div>
            <p class="text-[10px] text-slate-500 mt-1">
              Ketik 5 karakter di atas untuk memverifikasi bahwa Anda bukan robot.
            </p>
          </div>

          <!-- Submit Button -->
          <button
            type="submit"
            :disabled="isLoading || lockoutSeconds > 0"
            class="w-full py-2.5 bg-gradient-to-r from-brand-600 to-indigo-600 hover:from-brand-500 hover:to-indigo-500 disabled:opacity-50 text-white rounded-xl text-xs font-bold transition-all shadow-lg shadow-brand-600/30 flex items-center justify-center gap-2 cursor-pointer active:scale-[0.99]"
          >
            <ShieldCheck class="w-4 h-4" />
            <span>{{ isLoading ? 'Memverifikasi Akses...' : 'Masuk ke Studio Provider' }}</span>
          </button>
        </form>
      </div>

      <div class="text-center text-[11px] text-slate-500 font-sans">
        Notane Provider &bull; Manajemen Lisensi & Pembaruan Sistem
      </div>
    </div>
  </div>
</template>
