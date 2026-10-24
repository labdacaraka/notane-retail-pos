<script setup lang="ts">
import { watch, onMounted, onUnmounted } from 'vue'
import { useDialogStore } from '@/stores/dialogStore'
import { X, AlertTriangle, AlertCircle, Info, CheckCircle2 } from 'lucide-vue-next'

const dialog = useDialogStore()
let alertOpenedTime = 0
let confirmOpenedTime = 0

watch(() => dialog.isAlertOpen, (val) => {
  if (val) alertOpenedTime = Date.now()
})

watch(() => dialog.isConfirmOpen, (val) => {
  if (val) confirmOpenedTime = Date.now()
})

function handleBackdropConfirm() {
  if (Date.now() - confirmOpenedTime < 300) return
  dialog.handleConfirmAction(false)
}

function handleBackdropAlert() {
  if (Date.now() - alertOpenedTime < 300) return
  dialog.handleAlertClose()
}

function handleKeyDown(e: KeyboardEvent) {
  if (dialog.isConfirmOpen) {
    if (e.key === 'Escape') {
      e.preventDefault()
      dialog.handleConfirmAction(false)
    } else if (e.key === 'Enter') {
      if (Date.now() - confirmOpenedTime < 250) return
      e.preventDefault()
      dialog.handleConfirmAction(true)
    }
  } else if (dialog.isAlertOpen) {
    if (e.key === 'Escape') {
      e.preventDefault()
      dialog.handleAlertClose()
    } else if (e.key === 'Enter') {
      if (Date.now() - alertOpenedTime < 250) return
      e.preventDefault()
      dialog.handleAlertClose()
    }
  }
}

onMounted(() => {
  window.addEventListener('keydown', handleKeyDown)
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeyDown)
})
</script>

<template>
  <!-- ========================================================================= -->
  <!-- 1. MODAL KONFIRMASI IN-APP (PENGGANTI WINDOW.CONFIRM)                     -->
  <!-- ========================================================================= -->
  <Teleport to="body">
    <div
      v-if="dialog.isConfirmOpen"
      class="fixed inset-0 z-[9999] flex items-center justify-center p-4 bg-slate-900/60 backdrop-blur-xs select-none animate-in fade-in duration-150"
      @click.self="handleBackdropConfirm"
    >
      <div
        class="bg-white border-2 border-slate-400 shadow-2xl rounded-2xl w-full max-w-md overflow-hidden flex flex-col font-sans transition-all scale-100"
        role="dialog"
        aria-modal="true"
      >
        <!-- Title Bar -->
        <div
          class="px-5 py-3.5 flex items-center justify-between text-white border-b select-none shadow-xs"
          :class="{
            'bg-rose-700 border-rose-800': dialog.confirmOptions.type === 'danger',
            'bg-amber-600 border-amber-700': dialog.confirmOptions.type === 'warning',
            'bg-blue-700 border-blue-800': dialog.confirmOptions.type === 'info',
            'bg-emerald-700 border-emerald-800': dialog.confirmOptions.type === 'success',
          }"
        >
          <div class="flex items-center gap-2.5">
            <AlertTriangle v-if="dialog.confirmOptions.type === 'danger' || dialog.confirmOptions.type === 'warning'" class="w-5 h-5 text-white animate-bounce" />
            <Info v-else-if="dialog.confirmOptions.type === 'info'" class="w-5 h-5 text-white" />
            <CheckCircle2 v-else class="w-5 h-5 text-white" />
            <span class="font-black text-xs uppercase tracking-wider">{{ dialog.confirmOptions.title }}</span>
          </div>
          <button
            type="button"
            @click="dialog.handleConfirmAction(false)"
            class="text-white/80 hover:text-white hover:bg-white/20 p-1 rounded-lg transition cursor-pointer"
            title="Tutup (Esc)"
          >
            <X class="w-4 h-4" />
          </button>
        </div>

        <!-- Body Content -->
        <div class="p-6 bg-slate-50/80 space-y-4">
          <div class="flex items-start gap-4">
            <div
              class="w-12 h-12 rounded-xl flex items-center justify-center shrink-0 shadow-xs border"
              :class="{
                'bg-rose-100 text-rose-700 border-rose-200': dialog.confirmOptions.type === 'danger',
                'bg-amber-100 text-amber-700 border-amber-200': dialog.confirmOptions.type === 'warning',
                'bg-blue-100 text-blue-700 border-blue-200': dialog.confirmOptions.type === 'info',
                'bg-emerald-100 text-emerald-700 border-emerald-200': dialog.confirmOptions.type === 'success',
              }"
            >
              <AlertCircle v-if="dialog.confirmOptions.type === 'danger'" class="w-7 h-7" />
              <AlertTriangle v-else-if="dialog.confirmOptions.type === 'warning'" class="w-7 h-7" />
              <Info v-else-if="dialog.confirmOptions.type === 'info'" class="w-7 h-7" />
              <CheckCircle2 v-else class="w-7 h-7" />
            </div>

            <div class="space-y-1.5 flex-1 min-w-0">
              <div class="text-sm font-bold text-slate-900 leading-snug break-words">
                {{ dialog.confirmOptions.message }}
              </div>
              <div v-if="dialog.confirmOptions.subMessage" class="text-xs text-slate-500 leading-relaxed break-words">
                {{ dialog.confirmOptions.subMessage }}
              </div>
            </div>
          </div>
        </div>

        <!-- Action Footer -->
        <div class="bg-white px-5 py-3.5 border-t border-slate-200 flex items-center justify-end gap-2.5">
          <button
            type="button"
            @click="dialog.handleConfirmAction(false)"
            class="px-4 py-2 bg-slate-100 hover:bg-slate-200 text-slate-700 font-bold border border-slate-300 rounded-xl transition cursor-pointer text-xs active:scale-95 shadow-2xs"
          >
            {{ dialog.confirmOptions.cancelText || 'Batal' }}
          </button>

          <button
            type="button"
            @click="dialog.handleConfirmAction(true)"
            class="px-5 py-2 text-white font-bold rounded-xl shadow-sm transition cursor-pointer text-xs flex items-center gap-1.5 active:scale-95"
            :class="{
              'bg-rose-600 hover:bg-rose-700 border border-rose-700': dialog.confirmOptions.type === 'danger',
              'bg-amber-600 hover:bg-amber-700 border border-amber-700': dialog.confirmOptions.type === 'warning',
              'bg-blue-600 hover:bg-blue-700 border border-blue-700': dialog.confirmOptions.type === 'info',
              'bg-emerald-600 hover:bg-emerald-700 border border-emerald-700': dialog.confirmOptions.type === 'success',
            }"
          >
            <span>{{ dialog.confirmOptions.confirmText || 'Ya, Lanjutkan' }}</span>
          </button>
        </div>
      </div>
    </div>
  </Teleport>

  <!-- ========================================================================= -->
  <!-- 2. MODAL ALERT IN-APP (PENGGANTI WINDOW.ALERT)                            -->
  <!-- ========================================================================= -->
  <Teleport to="body">
    <div
      v-if="dialog.isAlertOpen"
      class="fixed inset-0 z-[9999] flex items-center justify-center p-4 bg-slate-900/60 backdrop-blur-xs select-none animate-in fade-in duration-150"
      @click.self="handleBackdropAlert"
    >
      <div
        class="bg-white border-2 border-slate-300 shadow-2xl rounded-2xl w-full max-w-md overflow-hidden flex flex-col font-sans transition-all scale-100"
        role="dialog"
        aria-modal="true"
      >
        <!-- Header -->
        <div
          class="px-5 py-3.5 flex items-center justify-between text-white border-b select-none shadow-xs"
          :class="{
            'bg-rose-700 border-rose-800': dialog.alertOptions.type === 'danger',
            'bg-amber-600 border-amber-700': dialog.alertOptions.type === 'warning',
            'bg-blue-700 border-blue-800': dialog.alertOptions.type === 'info',
            'bg-emerald-700 border-emerald-800': dialog.alertOptions.type === 'success',
          }"
        >
          <div class="flex items-center gap-2">
            <span class="font-black text-xs uppercase tracking-wider">{{ dialog.alertOptions.title }}</span>
          </div>
          <button
            type="button"
            @click="dialog.handleAlertClose()"
            class="text-white/80 hover:text-white hover:bg-white/20 p-1 rounded-lg transition cursor-pointer"
          >
            <X class="w-4 h-4" />
          </button>
        </div>

        <!-- Body -->
        <div class="p-6 bg-slate-50 space-y-3">
          <div class="flex items-start gap-3.5">
            <div
              class="w-11 h-11 rounded-xl flex items-center justify-center shrink-0 shadow-xs border"
              :class="{
                'bg-rose-100 text-rose-700 border-rose-200': dialog.alertOptions.type === 'danger',
                'bg-amber-100 text-amber-700 border-amber-200': dialog.alertOptions.type === 'warning',
                'bg-blue-100 text-blue-700 border-blue-200': dialog.alertOptions.type === 'info',
                'bg-emerald-100 text-emerald-700 border-emerald-200': dialog.alertOptions.type === 'success',
              }"
            >
              <AlertCircle v-if="dialog.alertOptions.type === 'danger'" class="w-6 h-6" />
              <AlertTriangle v-else-if="dialog.alertOptions.type === 'warning'" class="w-6 h-6" />
              <Info v-else-if="dialog.alertOptions.type === 'info'" class="w-6 h-6" />
              <CheckCircle2 v-else class="w-6 h-6" />
            </div>

            <div class="text-xs font-semibold text-slate-800 leading-relaxed break-words flex-1 min-w-0 pt-1">
              {{ dialog.alertOptions.message }}
            </div>
          </div>
        </div>

        <!-- Footer -->
        <div class="bg-white px-5 py-3 border-t border-slate-200 flex items-center justify-end">
          <button
            type="button"
            @click="dialog.handleAlertClose()"
            class="px-5 py-2 text-white font-bold rounded-xl shadow-xs transition cursor-pointer text-xs active:scale-95"
            :class="{
              'bg-rose-600 hover:bg-rose-700 border border-rose-700': dialog.alertOptions.type === 'danger',
              'bg-amber-600 hover:bg-amber-700 border border-amber-700': dialog.alertOptions.type === 'warning',
              'bg-blue-600 hover:bg-blue-700 border border-blue-700': dialog.alertOptions.type === 'info',
              'bg-emerald-600 hover:bg-emerald-700 border border-emerald-700': dialog.alertOptions.type === 'success',
            }"
          >
            {{ dialog.alertOptions.buttonText || 'Mengerti' }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>

  <!-- ========================================================================= -->
  <!-- 3. TOAST NOTIFICATIONS (POPUP FLOATING DI ATAS KANAN)                    -->
  <!-- ========================================================================= -->
  <Teleport to="body">
    <div class="fixed top-5 right-5 z-[99999] flex flex-col gap-2.5 max-w-sm w-full pointer-events-none">
      <TransitionGroup
        enter-active-class="transform ease-out duration-200 transition"
        enter-from-class="translate-y-2 opacity-0 sm:translate-y-0 sm:translate-x-4"
        enter-to-class="translate-y-0 opacity-100 sm:translate-x-0"
        leave-active-class="transition ease-in duration-150"
        leave-from-class="opacity-100"
        leave-to-class="opacity-0"
      >
        <div
          v-for="toast in dialog.toasts"
          :key="toast.id"
          class="pointer-events-auto w-full bg-white border rounded-xl shadow-lg p-3.5 flex items-center justify-between gap-3 text-xs select-none"
          :class="{
            'border-emerald-300 text-emerald-950 bg-emerald-50/90': toast.type === 'success',
            'border-rose-300 text-rose-950 bg-rose-50/90': toast.type === 'danger',
            'border-amber-300 text-amber-950 bg-amber-50/90': toast.type === 'warning',
            'border-blue-300 text-blue-950 bg-blue-50/90': toast.type === 'info',
          }"
        >
          <div class="flex items-center gap-2.5 min-w-0">
            <CheckCircle2 v-if="toast.type === 'success'" class="w-4 h-4 text-emerald-600 shrink-0" />
            <AlertCircle v-else-if="toast.type === 'danger'" class="w-4 h-4 text-rose-600 shrink-0" />
            <AlertTriangle v-else-if="toast.type === 'warning'" class="w-4 h-4 text-amber-600 shrink-0" />
            <Info v-else class="w-4 h-4 text-blue-600 shrink-0" />
            <span class="font-bold truncate">{{ toast.message }}</span>
          </div>

          <button
            type="button"
            @click="dialog.removeToast(toast.id)"
            class="text-slate-400 hover:text-slate-700 p-0.5 rounded transition cursor-pointer shrink-0"
          >
            <X class="w-3.5 h-3.5" />
          </button>
        </div>
      </TransitionGroup>
    </div>
  </Teleport>
</template>
