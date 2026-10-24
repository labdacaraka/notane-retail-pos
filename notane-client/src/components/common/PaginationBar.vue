<script setup lang="ts">
import { computed } from 'vue'
import {
  ChevronLeft,
  ChevronRight,
  ChevronsLeft,
  ChevronsRight,
} from 'lucide-vue-next'

const props = withDefaults(
  defineProps<{
    totalItems: number
    currentPage: number
    pageSize?: number
    pageSizeOptions?: number[]
  }>(),
  {
    pageSize: 10,
    pageSizeOptions: () => [10, 25, 50, 100],
  }
)

const emit = defineEmits<{
  (e: 'update:currentPage', page: number): void
  (e: 'update:pageSize', size: number): void
}>()

const totalPages = computed(() => {
  return Math.max(1, Math.ceil(props.totalItems / props.pageSize))
})

const startItem = computed(() => {
  if (props.totalItems === 0) return 0
  return (props.currentPage - 1) * props.pageSize + 1
})

const endItem = computed(() => {
  return Math.min(props.currentPage * props.pageSize, props.totalItems)
})

const visiblePages = computed(() => {
  const current = props.currentPage
  const total = totalPages.value
  const pages: number[] = []

  let start = Math.max(1, current - 2)
  let end = Math.min(total, current + 2)

  if (current <= 3) {
    end = Math.min(total, 5)
  }
  if (current >= total - 2) {
    start = Math.max(1, total - 4)
  }

  for (let i = start; i <= end; i++) {
    pages.push(i)
  }
  return pages
})

function goToPage(page: number) {
  if (page >= 1 && page <= totalPages.value && page !== props.currentPage) {
    emit('update:currentPage', page)
  }
}

function handlePageSizeChange(event: Event) {
  const target = event.target as HTMLSelectElement
  const newSize = parseInt(target.value, 10)
  emit('update:pageSize', newSize)
  emit('update:currentPage', 1)
}
</script>

<template>
  <div class="flex flex-wrap items-center justify-between gap-4 px-4 py-2.5 bg-slate-50 border-t border-slate-300 text-xs text-slate-600 select-none">
    <!-- Records Counter -->
    <div class="flex items-center gap-3">
      <span>
        Menampilkan
        <strong class="text-slate-900 font-mono">{{ startItem }}</strong> -
        <strong class="text-slate-900 font-mono">{{ endItem }}</strong> dari
        <strong class="text-emerald-700 font-mono">{{ totalItems }}</strong> total data
      </span>

      <!-- Page Size Selector -->
      <div class="flex items-center gap-1.5 ml-4 pl-4 border-l border-slate-300">
        <span class="text-[11px] font-semibold text-slate-700">Baris:</span>
        <select
          :value="pageSize"
          @change="handlePageSizeChange"
          class="bg-white border border-slate-300 text-slate-900 text-xs rounded-md px-2 py-0.5 font-mono shadow-sm focus:outline-none focus:border-emerald-600"
        >
          <option v-for="opt in pageSizeOptions" :key="opt" :value="opt">
            {{ opt }}
          </option>
        </select>
      </div>
    </div>

    <!-- Navigation Buttons -->
    <div class="flex items-center gap-1 font-mono">
      <!-- First Page -->
      <button
        @click="goToPage(1)"
        :disabled="currentPage <= 1"
        class="p-1.5 rounded-md border transition shadow-sm disabled:opacity-40 disabled:cursor-not-allowed"
        :class="currentPage > 1 ? 'bg-white border-slate-300 text-slate-700 hover:text-slate-900 hover:bg-slate-100' : 'border-slate-200 text-slate-400 bg-slate-100'"
        title="Halaman Pertama"
      >
        <ChevronsLeft class="w-3.5 h-3.5" />
      </button>

      <!-- Previous Page -->
      <button
        @click="goToPage(currentPage - 1)"
        :disabled="currentPage <= 1"
        class="p-1.5 rounded-md border transition shadow-sm disabled:opacity-40 disabled:cursor-not-allowed"
        :class="currentPage > 1 ? 'bg-white border-slate-300 text-slate-700 hover:text-slate-900 hover:bg-slate-100' : 'border-slate-200 text-slate-400 bg-slate-100'"
        title="Sebelumnya"
      >
        <ChevronLeft class="w-3.5 h-3.5" />
      </button>

      <!-- Numbered Page Buttons -->
      <button
        v-for="page in visiblePages"
        :key="page"
        @click="goToPage(page)"
        class="min-w-[28px] h-7 px-2 text-xs font-bold rounded-md border transition shadow-sm"
        :class="currentPage === page ? 'bg-emerald-600 border-emerald-700 text-white shadow' : 'bg-white border-slate-300 text-slate-700 hover:text-slate-900 hover:bg-slate-100'"
      >
        {{ page }}
      </button>

      <!-- Next Page -->
      <button
        @click="goToPage(currentPage + 1)"
        :disabled="currentPage >= totalPages"
        class="p-1.5 rounded-md border transition shadow-sm disabled:opacity-40 disabled:cursor-not-allowed"
        :class="currentPage < totalPages ? 'bg-white border-slate-300 text-slate-700 hover:text-slate-900 hover:bg-slate-100' : 'border-slate-200 text-slate-400 bg-slate-100'"
        title="Selanjutnya"
      >
        <ChevronRight class="w-3.5 h-3.5" />
      </button>

      <!-- Last Page -->
      <button
        @click="goToPage(totalPages)"
        :disabled="currentPage >= totalPages"
        class="p-1.5 rounded-md border transition shadow-sm disabled:opacity-40 disabled:cursor-not-allowed"
        :class="currentPage < totalPages ? 'bg-white border-slate-300 text-slate-700 hover:text-slate-900 hover:bg-slate-100' : 'border-slate-200 text-slate-400 bg-slate-100'"
        title="Halaman Terakhir"
      >
        <ChevronsRight class="w-3.5 h-3.5" />
      </button>
    </div>
  </div>
</template>
