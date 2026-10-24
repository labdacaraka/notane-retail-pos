<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { api } from '@/services/api'
import { Search, ChevronDown, X, Check, Loader2 } from 'lucide-vue-next'

const props = withDefaults(
  defineProps<{
    modelValue: string | number | null | undefined
    apiEndpoint?: string
    extraParams?: Record<string, any>
    options?: any[]
    valueKey?: string
    labelKey?: string
    secondaryKey?: string
    placeholder?: string
    searchPlaceholder?: string
    disabled?: boolean
    allowClear?: boolean
    required?: boolean
    initialLabel?: string
  }>(),
  {
    modelValue: null,
    apiEndpoint: '',
    extraParams: () => ({}),
    options: () => [],
    valueKey: 'id',
    labelKey: 'name',
    secondaryKey: '',
    placeholder: '-- Pilih opsi --',
    searchPlaceholder: 'Ketik untuk mencari...',
    disabled: false,
    allowClear: true,
    required: false,
    initialLabel: '',
  }
)

const emit = defineEmits<{
  (e: 'update:modelValue', val: any): void
  (e: 'change', val: any, item: any): void
  (e: 'loaded', items: any[]): void
}>()

const isOpen = ref(false)
const searchQuery = ref('')
const internalOptions = ref<any[]>([])
const isLoading = ref(false)
const highlightedIndex = ref(-1)
const selectedItemCache = ref<any>(null)
let debounceTimer: ReturnType<typeof setTimeout> | null = null

const containerRef = ref<HTMLElement | null>(null)
const searchInputRef = ref<HTMLInputElement | null>(null)

// Menggabungkan options dari props dan dari API
const allOptions = computed(() => {
  if (props.options && props.options.length > 0) {
    return props.options
  }
  return internalOptions.value
})

// Filter pencarian
const filteredOptions = computed(() => {
  const q = searchQuery.value.trim().toLowerCase()
  if (!q) return allOptions.value

  return allOptions.value.filter((item) => {
    const label = String(item[props.labelKey] || '').toLowerCase()
    const sec = props.secondaryKey ? String(item[props.secondaryKey] || '').toLowerCase() : ''
    const val = String(item[props.valueKey] || '').toLowerCase()
    return label.includes(q) || sec.includes(q) || val.includes(q)
  })
})

// Opsi yang sedang terpilih
const selectedItem = computed(() => {
  if (props.modelValue === null || props.modelValue === undefined || props.modelValue === '') {
    return null
  }
  const found = allOptions.value.find((opt) => String(opt[props.valueKey]) === String(props.modelValue))
  if (found) {
    selectedItemCache.value = found
    return found
  }
  if (selectedItemCache.value && String(selectedItemCache.value[props.valueKey]) === String(props.modelValue)) {
    return selectedItemCache.value
  }
  return null
})

// Fetch dari API jika apiEndpoint disediakan (mendukung live searchable query)
async function fetchApiData(queryText?: string) {
  if (!props.apiEndpoint) return
  isLoading.value = true
  try {
    const params: Record<string, any> = {
      limit: 50,
      ...(props.extraParams || {}),
    }
    const q = queryText !== undefined ? queryText.trim() : searchQuery.value.trim()
    if (q) {
      params.search = q
    }

    const res = await api.get(props.apiEndpoint, { params })
    let dataList: any[] = []
    if (Array.isArray(res.data)) {
      dataList = res.data
    } else if (res.data?.data && Array.isArray(res.data.data)) {
      dataList = res.data.data
    } else {
      dataList = []
    }
    internalOptions.value = dataList
    emit('loaded', dataList)

    // Cache the selected item if found
    if (props.modelValue) {
      const match = dataList.find((opt) => String(opt[props.valueKey]) === String(props.modelValue))
      if (match) {
        selectedItemCache.value = match
      }
    }
  } catch (err) {
    console.error(`Gagal memuat opsi dari ${props.apiEndpoint}:`, err)
  } finally {
    isLoading.value = false
  }
}

function handleSearchInput() {
  if (props.apiEndpoint) {
    if (debounceTimer) clearTimeout(debounceTimer)
    debounceTimer = setTimeout(() => {
      fetchApiData(searchQuery.value)
    }, 250)
  }
}

// Buka/Tutup dropdown
function toggleDropdown() {
  if (props.disabled) return
  if (isOpen.value) {
    closeDropdown()
  } else {
    openDropdown()
  }
}

function openDropdown() {
  if (props.disabled) return
  isOpen.value = true
  searchQuery.value = ''
  highlightedIndex.value = -1

  // Refresh data dari API dengan endpoint & extraParams
  if (props.apiEndpoint) {
    fetchApiData('')
  }

  nextTick(() => {
    searchInputRef.value?.focus()
  })
}

function closeDropdown() {
  isOpen.value = false
  searchQuery.value = ''
  highlightedIndex.value = -1
}

function selectOption(item: any) {
  selectedItemCache.value = item
  const val = item[props.valueKey]
  emit('update:modelValue', val)
  emit('change', val, item)
  closeDropdown()
}

function clearSelection(e: MouseEvent) {
  e.stopPropagation()
  if (props.disabled) return
  selectedItemCache.value = null
  emit('update:modelValue', null)
  emit('change', null, null)
}

// Keyboard Navigation
function handleKeyDown(e: KeyboardEvent) {
  if (!isOpen.value) {
    if (e.key === 'Enter' || e.key === 'ArrowDown' || e.key === ' ') {
      e.preventDefault()
      openDropdown()
    }
    return
  }

  if (e.key === 'Escape') {
    e.preventDefault()
    closeDropdown()
  } else if (e.key === 'ArrowDown') {
    e.preventDefault()
    if (filteredOptions.value.length > 0) {
      highlightedIndex.value = (highlightedIndex.value + 1) % filteredOptions.value.length
    }
  } else if (e.key === 'ArrowUp') {
    e.preventDefault()
    if (filteredOptions.value.length > 0) {
      highlightedIndex.value = (highlightedIndex.value - 1 + filteredOptions.value.length) % filteredOptions.value.length
    }
  } else if (e.key === 'Enter') {
    e.preventDefault()
    if (highlightedIndex.value >= 0 && highlightedIndex.value < filteredOptions.value.length) {
      selectOption(filteredOptions.value[highlightedIndex.value])
    }
  }
}

// Click Outside Handler
function handleClickOutside(e: MouseEvent) {
  if (containerRef.value && !containerRef.value.contains(e.target as Node)) {
    closeDropdown()
  }
}

watch(
  () => props.apiEndpoint,
  (newEndpoint) => {
    if (newEndpoint) {
      fetchApiData()
    }
  }
)

watch(
  () => props.extraParams,
  () => {
    if (props.apiEndpoint) {
      fetchApiData()
    }
  },
  { deep: true }
)

watch(
  () => props.modelValue,
  (newVal) => {
    if (newVal && props.apiEndpoint) {
      const match = allOptions.value.find((opt) => String(opt[props.valueKey]) === String(newVal))
      if (match) {
        selectedItemCache.value = match
      }
    }
  }
)

onMounted(() => {
  if (props.apiEndpoint) {
    fetchApiData()
  }
  document.addEventListener('click', handleClickOutside)
})

onUnmounted(() => {
  if (debounceTimer) clearTimeout(debounceTimer)
  document.removeEventListener('click', handleClickOutside)
})

defineExpose({
  reload: fetchApiData,
})
</script>

<template>
  <div ref="containerRef" class="relative w-full text-left select-none" @keydown="handleKeyDown">
    <!-- Trigger Input Button (Tampilan Input Select2) -->
    <div
      role="combobox"
      tabindex="0"
      :aria-expanded="isOpen"
      @click="toggleDropdown"
      class="w-full flex items-center justify-between px-3 py-2 text-xs rounded-lg border transition duration-150 cursor-pointer shadow-xs"
      :class="[
        disabled
          ? 'bg-slate-100 border-slate-200 text-slate-400 cursor-not-allowed'
          : isOpen
          ? 'bg-white border-emerald-600 ring-2 ring-emerald-500/20 text-slate-900'
          : 'bg-slate-50 hover:bg-white border-slate-300 text-slate-900',
      ]"
    >
      <!-- Display Value / Placeholder -->
      <div class="flex items-center gap-2 overflow-hidden flex-1 mr-2">
        <template v-if="selectedItem">
          <span class="font-semibold text-slate-900 truncate">
            {{ selectedItem[labelKey] }}
          </span>
          <span
            v-if="secondaryKey && selectedItem[secondaryKey]"
            class="text-[10px] bg-slate-200 text-slate-700 font-mono px-1.5 py-0.5 rounded border border-slate-300 shrink-0"
          >
            {{ selectedItem[secondaryKey] }}
          </span>
          <span
            v-if="selectedItem.is_warehouse !== undefined"
            class="text-[10px] px-1.5 py-0.5 rounded font-mono font-bold shrink-0"
            :class="selectedItem.is_warehouse ? 'bg-amber-100 text-amber-800 border border-amber-200' : 'bg-blue-100 text-blue-800 border border-blue-200'"
          >
            {{ selectedItem.is_warehouse ? 'Gudang' : 'Toko' }}
          </span>
          <span
            v-if="selectedItem.has_biometric || selectedItem.has_face_recognition"
            class="text-[10px] bg-emerald-50 text-emerald-700 border border-emerald-200 px-1 py-0.5 rounded font-mono font-bold shrink-0 flex items-center gap-0.5"
          >
            <span v-if="selectedItem.has_biometric">👆</span>
            <span v-if="selectedItem.has_face_recognition">📷</span>
          </span>
          <span
            v-if="selectedItem.stock_qty !== undefined"
            class="text-[10px] font-mono px-1.5 py-0.5 rounded shrink-0 font-bold"
            :class="Number(selectedItem.stock_qty) <= 0 ? 'bg-rose-100 text-rose-700 border border-rose-200' : 'bg-emerald-100 text-emerald-800 border border-emerald-200'"
          >
            Stok: {{ selectedItem.stock_qty }}
          </span>
        </template>
        <template v-else-if="modelValue && initialLabel">
          <span class="font-semibold text-slate-900 truncate">
            {{ initialLabel }}
          </span>
        </template>
        <span v-else class="text-slate-400 truncate">
          {{ placeholder }}
        </span>
      </div>

      <!-- Action Icons (Clear, Loading, Caret) -->
      <div class="flex items-center gap-1 shrink-0 text-slate-400">
        <span
          v-if="allowClear && selectedItem && !disabled"
          @click="clearSelection"
          class="hover:text-rose-600 p-0.5 rounded transition cursor-pointer"
          title="Hapus Pilihan"
        >
          <X class="w-3.5 h-3.5" />
        </span>
        <Loader2 v-if="isLoading" class="w-3.5 h-3.5 animate-spin text-emerald-600" />
        <ChevronDown
          class="w-3.5 h-3.5 transition-transform duration-200"
          :class="isOpen ? 'rotate-180 text-emerald-600' : 'text-slate-400'"
        />
      </div>
    </div>

    <!-- Dropdown Popover Menu -->
    <div
      v-if="isOpen"
      class="absolute left-0 right-0 top-full mt-1 bg-white border border-slate-300 rounded-lg shadow-2xl z-[200] overflow-hidden text-slate-900 animate-in fade-in zoom-in-95 duration-100 min-w-[280px]"
    >
      <!-- Search Box Input -->
      <div class="p-2 border-b border-slate-200 bg-slate-50 flex items-center gap-2">
        <Search class="w-3.5 h-3.5 text-slate-400 shrink-0 ml-1" />
        <input
          ref="searchInputRef"
          v-model="searchQuery"
          @input="handleSearchInput"
          type="text"
          :placeholder="searchPlaceholder"
          class="w-full bg-white border border-slate-300 rounded px-2.5 py-1 text-xs text-slate-900 focus:outline-none focus:border-emerald-600 focus:ring-1 focus:ring-emerald-600"
          @click.stop
        />
        <span
          v-if="searchQuery"
          @click="searchQuery = ''; handleSearchInput()"
          class="text-slate-400 hover:text-slate-600 cursor-pointer p-0.5"
        >
          <X class="w-3.5 h-3.5" />
        </span>
      </div>

      <!-- Options List -->
      <div
        class="max-h-64 overflow-y-auto divide-y divide-slate-100 py-1"
        style="overscroll-behavior: contain;"
        @wheel.stop
      >
        <div
          v-if="isLoading"
          class="p-4 text-center text-xs text-slate-500 flex items-center justify-center gap-2"
        >
          <Loader2 class="w-4 h-4 animate-spin text-emerald-600" />
          <span>Mencari data dari server...</span>
        </div>

        <div
          v-else-if="filteredOptions.length === 0"
          class="p-4 text-center text-xs text-slate-400 font-mono"
        >
          Tidak ada data yang cocok dengan "{{ searchQuery }}"
        </div>

        <div
          v-for="(item, index) in filteredOptions"
          :key="item[valueKey] ?? index"
          @click="selectOption(item)"
          @mouseenter="highlightedIndex = index"
          class="px-3 py-2 text-xs flex items-center justify-between cursor-pointer transition"
          :class="[
            String(item[valueKey]) === String(modelValue)
              ? 'bg-emerald-50 text-emerald-900 font-bold'
              : highlightedIndex === index
              ? 'bg-slate-100 text-slate-900'
              : 'text-slate-700 hover:bg-slate-50',
          ]"
        >
          <div class="flex items-center gap-2 truncate flex-1 min-w-0 pr-2">
            <span class="truncate font-medium">{{ item[labelKey] }}</span>
            <span
              v-if="secondaryKey && item[secondaryKey]"
              class="text-[10px] bg-slate-100 text-slate-600 font-mono px-1.5 py-0.5 rounded border border-slate-200 shrink-0"
            >
              {{ item[secondaryKey] }}
            </span>
            <span
              v-if="item.is_warehouse !== undefined"
              class="text-[10px] px-1.5 py-0.5 rounded font-mono font-bold shrink-0"
              :class="item.is_warehouse ? 'bg-amber-100 text-amber-800 border border-amber-200' : 'bg-blue-100 text-blue-800 border border-blue-200'"
            >
              {{ item.is_warehouse ? 'Gudang' : 'Toko' }}
            </span>
            <span
              v-if="item.has_biometric || item.has_face_recognition"
              class="text-[10px] bg-emerald-50 text-emerald-700 border border-emerald-200 px-1 py-0.5 rounded font-mono font-bold shrink-0 flex items-center gap-0.5"
            >
              <span v-if="item.has_biometric">👆</span>
              <span v-if="item.has_face_recognition">📷</span>
            </span>
            <span
              v-if="item.stock_qty !== undefined"
              class="text-[10px] font-mono px-1.5 py-0.5 rounded shrink-0 font-bold ml-auto"
              :class="Number(item.stock_qty) <= 0 ? 'bg-rose-50 text-rose-600 border border-rose-200' : 'bg-emerald-50 text-emerald-700 border border-emerald-200'"
            >
              Stok: {{ item.stock_qty }}
            </span>
          </div>

          <!-- Checkmark Icon jika terpilih -->
          <Check
            v-if="String(item[valueKey]) === String(modelValue)"
            class="w-4 h-4 text-emerald-600 shrink-0 ml-2"
          />
        </div>
      </div>

      <!-- Footer Info -->
      <div
        v-if="filteredOptions.length > 0"
        class="px-3 py-1.5 bg-slate-50 border-t border-slate-200 text-[10px] font-mono text-slate-500 flex items-center justify-between"
      >
        <span>Total: {{ filteredOptions.length }} item</span>
        <span class="text-slate-400">Pilih atau tekan Enter</span>
      </div>
    </div>
  </div>
</template>
