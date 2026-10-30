<script setup lang="ts">
import { ref, computed } from 'vue'
import { Copy, Check } from 'lucide-vue-next'
import { copyToClipboard } from '@/utils/clipboard'
import { useDialog } from '@/composables/useDialog'

const props = withDefaults(
  defineProps<{
    value?: string | number | null
    text?: string | number | null
    label?: string
    displayValue?: string | number
    showIcon?: 'hover' | 'always' | 'never'
    badge?: boolean
    mono?: boolean
    bold?: boolean
    size?: 'xs' | 'sm' | 'base'
    toastFeedback?: boolean
  }>(),
  {
    value: '',
    text: '',
    label: '',
    displayValue: '',
    showIcon: 'hover',
    badge: false,
    mono: true,
    bold: true,
    size: 'xs',
    toastFeedback: true,
  }
)

const dialog = useDialog()
const isCopied = ref<boolean>(false)

const actualValue = computed(() => {
  if (props.value !== undefined && props.value !== null && props.value !== '') return props.value
  return props.text ?? ''
})

async function handleClick(e: MouseEvent) {
  e.stopPropagation()
  const text = String(actualValue.value ?? '').trim()
  if (!text) return

  const success = await copyToClipboard(text)
  if (success) {
    isCopied.value = true
    if (props.toastFeedback) {
      const subject = props.label ? `${props.label} "${text}"` : `"${text}"`
      dialog.toast(`${subject} disalin ke clipboard`, 'success', 2000)
    }
    setTimeout(() => {
      isCopied.value = false
    }, 1500)
  }
}
</script>

<template>
  <span
    v-if="actualValue"
    @click="handleClick"
    :title="`Klik untuk menyalin ${label || actualValue}`"
    class="inline-flex items-center gap-1.5 cursor-pointer select-text transition-all group/copy py-0.5 px-1 -mx-1 rounded hover:bg-slate-200/60 active:scale-95 text-left"
    :class="[
      badge ? 'bg-slate-100 border border-slate-200 px-2 py-0.5 rounded-md hover:bg-slate-200' : '',
    ]"
  >
    <span
      class="select-text"
      :class="[
        mono ? 'font-mono' : '',
        bold ? 'font-bold' : '',
        size === 'xs' ? 'text-xs' : size === 'sm' ? 'text-sm' : 'text-base',
      ]"
    >
      <slot>{{ displayValue || actualValue }}</slot>
    </span>

    <span
      v-if="showIcon !== 'never'"
      class="inline-flex items-center justify-center p-0.5 rounded transition-all text-slate-400 group-hover/copy:text-slate-700 shrink-0"
      :class="[
        isCopied ? 'text-emerald-600' : showIcon === 'always' ? 'opacity-80' : 'opacity-0 group-hover/copy:opacity-100'
      ]"
    >
      <Check v-if="isCopied" class="w-3.5 h-3.5 text-emerald-600 animate-in zoom-in-75 duration-150" />
      <Copy v-else class="w-3 h-3" />
    </span>
  </span>
  <span v-else class="text-slate-400">-</span>
</template>
