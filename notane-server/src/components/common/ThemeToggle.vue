<script setup lang="ts">
import { useTheme } from '@/composables/useTheme'
import { Sun, Moon } from 'lucide-vue-next'

const props = withDefaults(
  defineProps<{
    showLabel?: boolean
  }>(),
  {
    showLabel: false,
  }
)

const { isDark, toggleTheme } = useTheme()
</script>

<template>
  <button
    type="button"
    @click="toggleTheme"
    :title="isDark ? 'Beralih ke Mode Terang (Light Mode)' : 'Beralih ke Mode Gelap (Dark Mode)'"
    class="flex items-center gap-1.5 px-2.5 py-1.5 rounded-xl border text-xs font-semibold transition-all duration-200 cursor-pointer select-none active:scale-95 shadow-2xs"
    :class="
      isDark
        ? 'bg-slate-800 hover:bg-slate-700 text-amber-300 border-slate-700 hover:border-amber-400/50'
        : 'bg-white hover:bg-slate-50 text-slate-700 border-slate-300 hover:border-indigo-300'
    "
  >
    <Sun v-if="isDark" class="w-3.5 h-3.5 text-amber-400 transition-transform duration-300 rotate-0" />
    <Moon v-else class="w-3.5 h-3.5 text-indigo-600 transition-transform duration-300 -rotate-12" />
    <span v-if="props.showLabel" class="text-xs">
      {{ isDark ? 'Mode Gelap' : 'Mode Terang' }}
    </span>
  </button>
</template>
