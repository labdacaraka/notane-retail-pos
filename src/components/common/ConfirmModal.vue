<script setup lang="ts">
defineProps<{
  show: boolean
  title?: string
  message: string
  subMessage?: string
  confirmText?: string
  cancelText?: string
  type?: 'danger' | 'warning' | 'info'
}>()

const emit = defineEmits<{
  (e: 'confirm'): void
  (e: 'close'): void
}>()
</script>

<template>
  <div
    v-if="show"
    class="fixed inset-0 z-50 flex items-center justify-center bg-slate-900/50 backdrop-blur-xs p-4 select-none animate-in fade-in duration-150"
  >
    <div
      class="bg-white border-2 border-slate-400 shadow-2xl rounded-sm w-full max-w-md overflow-hidden flex flex-col font-sans"
    >
      <!-- Title Bar Retro -->
      <div
        class="px-4 py-2 flex items-center justify-between text-white border-b select-none"
        :class="type === 'danger' ? 'bg-rose-800 border-rose-900' : 'bg-slate-800 border-slate-700'"
      >
        <div class="flex items-center gap-2">
          <span class="text-sm">⚠️</span>
          <span class="font-bold text-xs uppercase tracking-wider">{{ title || 'Konfirmasi Tindakan' }}</span>
        </div>
        <button
          type="button"
          @click="emit('close')"
          class="text-slate-300 hover:text-white px-2 py-0.5 rounded text-sm font-bold cursor-pointer"
        >
          ✕
        </button>
      </div>

      <!-- Body -->
      <div class="p-4 bg-slate-50 space-y-3">
        <div class="flex items-start gap-3">
          <div
            class="w-10 h-10 rounded-sm flex items-center justify-center text-lg font-bold shrink-0 shadow-xs border"
            :class="type === 'danger' ? 'bg-rose-100 text-rose-700 border-rose-300' : 'bg-amber-100 text-amber-800 border-amber-300'"
          >
            {{ type === 'danger' ? '⛔' : '⚠️' }}
          </div>
          <div class="space-y-1">
            <div class="text-xs font-bold text-slate-800 leading-snug">
              {{ message }}
            </div>
            <div v-if="subMessage" class="text-[11px] text-slate-500 leading-relaxed">
              {{ subMessage }}
            </div>
          </div>
        </div>
      </div>

      <!-- Footer Actions -->
      <div class="bg-slate-100 px-4 py-3 border-t border-slate-300 flex items-center justify-end gap-2">
        <button
          type="button"
          @click="emit('close')"
          class="px-4 py-1.5 bg-white hover:bg-slate-200 text-slate-700 font-semibold border border-slate-300 rounded-sm transition-colors cursor-pointer text-xs"
        >
          {{ cancelText || 'Batal' }}
        </button>

        <button
          type="button"
          @click="emit('confirm')"
          class="px-4 py-1.5 text-white font-bold rounded-sm shadow-xs transition-colors cursor-pointer text-xs flex items-center gap-1.5"
          :class="type === 'danger'
            ? 'bg-rose-600 hover:bg-rose-700 border border-rose-700'
            : 'bg-amber-600 hover:bg-amber-700 border border-amber-700'"
        >
          <span>{{ confirmText || 'Ya, Lanjutkan' }}</span>
        </button>
      </div>
    </div>
  </div>
</template>
