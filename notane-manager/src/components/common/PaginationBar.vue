<script setup lang="ts">
import { computed } from 'vue'
import { ChevronLeft, ChevronRight, ChevronsLeft, ChevronsRight } from 'lucide-vue-next'
import { formatNumber } from '@/utils/formatters'

const props = withDefaults(
  defineProps<{
    page: number
    pageSize: number
    totalItems: number
    totalPages: number
    isLoading?: boolean
    pageSizeOptions?: number[]
  }>(),
  {
    isLoading: false,
    pageSizeOptions: () => [10, 25, 50, 100],
  }
)

const emit = defineEmits<{
  (e: 'update:page', page: number): void
  (e: 'update:pageSize', size: number): void
}>()

const fromItem = computed(() => {
  if (props.totalItems === 0) return 0
  return (props.page - 1) * props.pageSize + 1
})

const toItem = computed(() => {
  return Math.min(props.page * props.pageSize, props.totalItems)
})

function changePage(target: number) {
  if (target < 1 || target > props.totalPages || target === props.page || props.isLoading) return
  emit('update:page', target)
}

function handlePageSizeChange(e: Event) {
  const target = e.target as HTMLSelectElement
  emit('update:pageSize', Number(target.value))
}

const displayedPages = computed(() => {
  const total = props.totalPages
  const current = props.page
  if (total <= 5) {
    return Array.from({ length: total }, (_, i) => i + 1)
  }
  if (current <= 3) {
    return [1, 2, 3, 4, 5]
  }
  if (current >= total - 2) {
    return [total - 4, total - 3, total - 2, total - 1, total]
  }
  return [current - 2, current - 1, current, current + 1, current + 2]
})
</script>

<template>
  <div class="flex flex-col sm:flex-row items-center justify-between gap-3 px-4 py-3 bg-slate-900/80 border border-slate-800/80 rounded-xl text-xs text-slate-400">
    <!-- Info & Page Size -->
    <div class="flex items-center gap-3 w-full sm:w-auto justify-between sm:justify-start">
      <div class="flex items-center gap-1.5">
        <span>Menampilkan</span>
        <span class="font-bold text-slate-200">{{ formatNumber(fromItem) }}</span>
        <span>-</span>
        <span class="font-bold text-slate-200">{{ formatNumber(toItem) }}</span>
        <span>dari</span>
        <span class="font-bold text-slate-200">{{ formatNumber(totalItems) }}</span>
        <span>data</span>
      </div>

      <div class="flex items-center gap-1.5 pl-3 border-l border-slate-800">
        <label class="text-[11px] text-slate-500">Per hal:</label>
        <select
          :value="pageSize"
          @change="handlePageSizeChange"
          :disabled="isLoading"
          class="bg-slate-800 border border-slate-700 text-slate-200 text-xs rounded-lg px-2 py-1 focus:outline-none focus:border-brand-500 cursor-pointer disabled:opacity-50"
        >
          <option v-for="opt in pageSizeOptions" :key="opt" :value="opt">
            {{ opt }}
          </option>
        </select>
      </div>
    </div>

    <!-- Page Navigation Buttons -->
    <div class="flex items-center gap-1 w-full sm:w-auto justify-center sm:justify-end">
      <!-- First -->
      <button
        type="button"
        @click="changePage(1)"
        :disabled="page <= 1 || isLoading"
        title="Halaman Pertama"
        class="p-1.5 rounded-lg border border-slate-800 hover:bg-slate-800 text-slate-300 disabled:opacity-30 disabled:hover:bg-transparent transition cursor-pointer"
      >
        <ChevronsLeft class="w-3.5 h-3.5" />
      </button>

      <!-- Previous -->
      <button
        type="button"
        @click="changePage(page - 1)"
        :disabled="page <= 1 || isLoading"
        title="Sebelumnya"
        class="p-1.5 rounded-lg border border-slate-800 hover:bg-slate-800 text-slate-300 disabled:opacity-30 disabled:hover:bg-transparent transition cursor-pointer"
      >
        <ChevronLeft class="w-3.5 h-3.5" />
      </button>

      <!-- Page Numbers -->
      <div class="flex items-center gap-1 mx-1">
        <button
          v-for="p in displayedPages"
          :key="p"
          type="button"
          @click="changePage(p)"
          :disabled="isLoading"
          class="w-7 h-7 rounded-lg text-xs font-semibold flex items-center justify-center transition cursor-pointer"
          :class="p === page
            ? 'bg-brand-600 text-white font-bold shadow-xs'
            : 'border border-slate-800 hover:bg-slate-800 text-slate-300'"
        >
          {{ p }}
        </button>
      </div>

      <!-- Next -->
      <button
        type="button"
        @click="changePage(page + 1)"
        :disabled="page >= totalPages || isLoading"
        title="Selanjutnya"
        class="p-1.5 rounded-lg border border-slate-800 hover:bg-slate-800 text-slate-300 disabled:opacity-30 disabled:hover:bg-transparent transition cursor-pointer"
      >
        <ChevronRight class="w-3.5 h-3.5" />
      </button>

      <!-- Last -->
      <button
        type="button"
        @click="changePage(totalPages)"
        :disabled="page >= totalPages || isLoading"
        title="Halaman Terakhir"
        class="p-1.5 rounded-lg border border-slate-800 hover:bg-slate-800 text-slate-300 disabled:opacity-30 disabled:hover:bg-transparent transition cursor-pointer"
      >
        <ChevronsRight class="w-3.5 h-3.5" />
      </button>
    </div>
  </div>
</template>
