<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { WifiOff, RefreshCw, ShieldCheck } from 'lucide-vue-next'
import { useNetworkStore } from '@/stores/networkStore'

const netStore = useNetworkStore()
const isReconnecting = ref(false)
const showSuccessNotice = ref(false)
const countdown = ref(5)

let countdownTimer: any = null

// Status terputus murni reaktif dari WebSocket subscription networkStore (Zero Polling!)
const isDisconnected = computed(() => {
  return netStore.connectionIntent === 'CONNECTED' && netStore.connectionStatus !== 'CONNECTED'
})

watch(isDisconnected, (disconnected, oldVal) => {
  if (disconnected) {
    startCountdown()
  } else if (oldVal && !disconnected) {
    // Pulih dari keadaan terputus
    clearInterval(countdownTimer)
    showSuccessNotice.value = true
    setTimeout(() => {
      showSuccessNotice.value = false
    }, 3000)
  }
})

function startCountdown() {
  countdown.value = 5
  clearInterval(countdownTimer)
  countdownTimer = setInterval(() => {
    if (!isDisconnected.value) {
      clearInterval(countdownTimer)
      return
    }
    if (countdown.value > 1) {
      countdown.value--
    } else {
      reconnect()
    }
  }, 1000)
}

async function reconnect() {
  if (isReconnecting.value) return
  isReconnecting.value = true

  try {
    const ok = await netStore.connect()
    if (ok) {
      clearInterval(countdownTimer)
      showSuccessNotice.value = true
      setTimeout(() => {
        showSuccessNotice.value = false
      }, 3000)
    } else {
      countdown.value = 5
    }
  } catch (err) {
    console.warn('[RecoveryBanner] Gagal menghubungkan kembali ke server:', err)
    countdown.value = 5
  } finally {
    isReconnecting.value = false
  }
}

onMounted(() => {
  if (isDisconnected.value) {
    startCountdown()
  }
})

onUnmounted(() => {
  clearInterval(countdownTimer)
})
</script>

<template>
  <div class="fixed top-0 inset-x-0 z-50 pointer-events-none">
    <!-- 1. BANNER KONEKSI TERPUTUS (AMBER / ROSE) -->
    <transition
      enter-active-class="transform transition ease-out duration-300"
      enter-from-class="-translate-y-full opacity-0"
      enter-to-class="translate-y-0 opacity-100"
      leave-active-class="transform transition ease-in duration-200"
      leave-from-class="translate-y-0 opacity-100"
      leave-to-class="-translate-y-full opacity-0"
    >
      <div
        v-if="isDisconnected"
        class="pointer-events-auto bg-rose-800 text-white px-4 py-2.5 shadow-xl border-b border-rose-950"
      >
        <div class="max-w-6xl mx-auto flex flex-wrap items-center justify-between gap-3">
          <div class="flex items-center gap-3">
            <div class="w-8 h-8 rounded-lg bg-rose-900 flex items-center justify-center shrink-0">
              <WifiOff class="w-4 h-4 text-white" />
            </div>
            <div>
              <div class="flex items-center gap-2">
                <span class="text-xs font-bold tracking-wide uppercase bg-rose-950/80 px-2 py-0.5 rounded text-rose-200 font-mono">
                  Koneksi Terputus
                </span>
                <h4 class="text-xs font-bold text-white">Terputus dari Server Toko</h4>
              </div>
              <p class="text-[11px] text-rose-100/90 mt-0.5">
                Mencoba menyambung ulang otomatis dalam <strong class="font-mono text-amber-200">{{ countdown }} detik</strong>...
              </p>
            </div>
          </div>

          <!-- Action Buttons -->
          <div class="flex items-center gap-2">
            <button
              @click="reconnect"
              :disabled="isReconnecting"
              class="px-3.5 py-1.5 bg-white text-rose-700 hover:bg-rose-50 font-bold text-xs rounded-lg shadow-sm transition flex items-center gap-1.5 active:scale-95 disabled:opacity-60 cursor-pointer"
            >
              <RefreshCw class="w-3.5 h-3.5" :class="{ 'animate-spin': isReconnecting }" />
              <span>{{ isReconnecting ? 'Menghubungkan...' : '⚡ Sambungkan Sekarang' }}</span>
            </button>
          </div>
        </div>
      </div>
    </transition>

    <!-- 2. TOAST BERHASIL TERSAMBUNG KEMBALI (EMERALD) -->
    <transition
      enter-active-class="transform transition ease-out duration-300"
      enter-from-class="-translate-y-full opacity-0"
      enter-to-class="translate-y-0 opacity-100"
      leave-active-class="transform transition ease-in duration-200"
      leave-from-class="translate-y-0 opacity-100"
      leave-to-class="-translate-y-full opacity-0"
    >
      <div
        v-if="showSuccessNotice"
        class="pointer-events-auto bg-emerald-600 text-white px-4 py-2 shadow-lg border-b border-emerald-700 text-center"
      >
        <div class="max-w-4xl mx-auto flex items-center justify-center gap-2 text-xs font-bold">
          <ShieldCheck class="w-4 h-4 text-emerald-200" />
          <span>✅ Pusat Data & Server Kasir Berhasil Terhubung Kembali Secara Normal!</span>
        </div>
      </div>
    </transition>
  </div>
</template>
