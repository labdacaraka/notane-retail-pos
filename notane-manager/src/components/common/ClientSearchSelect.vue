<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { apiClient } from '../../services/api'
import type { Client, PaginatedResponse } from '../../stores/providerStore'
import { Search, Building, ChevronDown, X, Check, Loader2, User, Phone, Mail } from 'lucide-vue-next'

const props = withDefaults(
  defineProps<{
    modelValue: string
    placeholder?: string
    searchPlaceholder?: string
    disabled?: boolean
    error?: string
  }>(),
  {
    placeholder: '-- Cari & Pilih Klien Tenant --',
    searchPlaceholder: 'Ketik nama klien, perusahaan, email, atau telepon...',
    disabled: false,
    error: '',
  }
)

const emit = defineEmits<{
  (e: 'update:modelValue', val: string): void
  (e: 'change', client: Client | null): void
}>()

const isOpen = ref(false)
const searchQuery = ref('')
const clients = ref<Client[]>([])
const isLoading = ref(false)
const selectedClient = ref<Client | null>(null)
const containerRef = ref<HTMLElement | null>(null)
const searchInputRef = ref<HTMLInputElement | null>(null)
let debounceTimer: ReturnType<typeof setTimeout> | null = null

// Server-side search clients
async function fetchClients(queryText: string = '') {
  isLoading.value = true
  try {
    const params: Record<string, any> = {
      page: 1,
      page_size: 30,
      sort_by: 'client_name',
      sort_order: 'asc',
    }
    if (queryText.trim()) {
      params.search = queryText.trim()
    }

    const { data } = await apiClient.get<PaginatedResponse<Client>>('/api/v1/admin/clients', {
      params,
    })

    if (data && Array.isArray(data.items)) {
      clients.value = data.items

      // Jika ada modelValue dan belum ada selectedClient, cocokkan
      if (props.modelValue && (!selectedClient.value || selectedClient.value.id !== props.modelValue)) {
        const found = clients.value.find((c) => c.id === props.modelValue)
        if (found) {
          selectedClient.value = found
        }
      }
    }
  } catch (err) {
    console.error('Gagal mencari klien:', err)
  } finally {
    isLoading.value = false
  }
}

// Cari klien spesifik jika modelValue sudah ada di awal tetapi tidak ada dalam list pertama
async function fetchClientById(clientId: string) {
  if (!clientId) {
    selectedClient.value = null
    return
  }
  try {
    // Coba temukan di state clients terlebih dahulu
    const existing = clients.value.find((c) => c.id === clientId)
    if (existing) {
      selectedClient.value = existing
      return
    }

    // Jika tidak ada, fetch server-side dengan search
    const { data } = await apiClient.get<PaginatedResponse<Client>>('/api/v1/admin/clients', {
      params: { search: clientId, page: 1, page_size: 10 },
    })
    const found = data?.items?.find((c) => c.id === clientId) || data?.items?.[0]
    if (found) {
      selectedClient.value = found
      if (!clients.value.some((c) => c.id === found.id)) {
        clients.value.unshift(found)
      }
    }
  } catch (e) {
    console.warn('Gagal memuat detail klien terpilih:', e)
  }
}

function handleSearchInput() {
  if (debounceTimer) clearTimeout(debounceTimer)
  debounceTimer = setTimeout(() => {
    fetchClients(searchQuery.value)
  }, 250)
}

function toggleDropdown() {
  if (props.disabled) return
  isOpen.value = !isOpen.value
  if (isOpen.value) {
    searchQuery.value = ''
    fetchClients('')
    nextTick(() => {
      searchInputRef.value?.focus()
    })
  }
}

function selectClient(client: Client) {
  selectedClient.value = client
  emit('update:modelValue', client.id)
  emit('change', client)
  isOpen.value = false
}

function clearSelection(event?: MouseEvent) {
  if (event) {
    event.stopPropagation()
  }
  selectedClient.value = null
  emit('update:modelValue', '')
  emit('change', null)
}

function handleClickOutside(event: MouseEvent) {
  if (containerRef.value && !containerRef.value.contains(event.target as Node)) {
    isOpen.value = false
  }
}

function handleKeydown(event: KeyboardEvent) {
  if (event.key === 'Escape' && isOpen.value) {
    isOpen.value = false
  }
}

watch(
  () => props.modelValue,
  (newVal) => {
    if (newVal) {
      fetchClientById(newVal)
    } else {
      selectedClient.value = null
    }
  },
  { immediate: true }
)

onMounted(() => {
  document.addEventListener('click', handleClickOutside)
  document.addEventListener('keydown', handleKeydown)
  fetchClients('')
})

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside)
  document.removeEventListener('keydown', handleKeydown)
  if (debounceTimer) clearTimeout(debounceTimer)
})
</script>

<template>
  <div ref="containerRef" class="relative w-full">
    <!-- Trigger Button -->
    <div
      @click="toggleDropdown"
      class="w-full px-3 py-2 bg-slate-950 border rounded-xl text-xs text-white flex items-center justify-between cursor-pointer transition-all select-none shadow-xs"
      :class="[
        error ? 'border-rose-500/80 ring-1 ring-rose-500/30' : isOpen ? 'border-brand-500 ring-1 ring-brand-500/30' : 'border-slate-800 hover:border-slate-700',
        disabled ? 'opacity-50 cursor-not-allowed bg-slate-900' : ''
      ]"
    >
      <!-- Left Content -->
      <div class="flex items-center gap-2.5 min-w-0 flex-1">
        <div class="w-6 h-6 rounded-lg bg-brand-500/10 border border-brand-500/20 text-brand-400 flex items-center justify-center shrink-0">
          <Building class="w-3.5 h-3.5" />
        </div>

        <div v-if="selectedClient" class="min-w-0 flex-1 text-left">
          <div class="font-bold text-white truncate flex items-center gap-2">
            <span>{{ selectedClient.client_name }}</span>
            <span class="text-[10px] font-mono px-1.5 py-0.2 rounded bg-slate-800 text-slate-300 font-normal">
              {{ selectedClient.company_name }}
            </span>
          </div>
          <div v-if="selectedClient.email || selectedClient.phone" class="text-[10px] text-slate-400 font-mono truncate">
            {{ selectedClient.email || selectedClient.phone }}
          </div>
        </div>

        <div v-else class="text-slate-500 text-xs truncate">
          {{ placeholder }}
        </div>
      </div>

      <!-- Right Controls -->
      <div class="flex items-center gap-1.5 ml-2 shrink-0">
        <button
          v-if="selectedClient && !disabled"
          type="button"
          @click="clearSelection"
          class="p-1 rounded-md text-slate-400 hover:text-rose-400 hover:bg-slate-800 transition"
          title="Hapus pilihan"
        >
          <X class="w-3.5 h-3.5" />
        </button>

        <ChevronDown
          class="w-4 h-4 text-slate-400 transition-transform duration-200"
          :class="isOpen ? 'rotate-180 text-brand-400' : ''"
        />
      </div>
    </div>

    <!-- Error message under dropdown -->
    <span v-if="error" class="text-[11px] text-rose-400 mt-1 block">
      {{ error }}
    </span>

    <!-- Dropdown Menu Floating Box -->
    <div
      v-if="isOpen"
      class="absolute left-0 right-0 top-full mt-1.5 z-50 bg-slate-900 border border-slate-800 rounded-xl shadow-2xl overflow-hidden backdrop-blur-xl animate-in fade-in zoom-in-95 duration-100"
    >
      <!-- Search Input Header -->
      <div class="p-2 border-b border-slate-800 bg-slate-950/80">
        <div class="relative">
          <Search class="w-3.5 h-3.5 text-slate-400 absolute left-2.5 top-1/2 -translate-y-1/2" />
          <input
            ref="searchInputRef"
            v-model="searchQuery"
            @input="handleSearchInput"
            type="text"
            :placeholder="searchPlaceholder"
            class="w-full pl-8 pr-8 py-1.5 bg-slate-900 border border-slate-700 rounded-lg text-xs text-white placeholder-slate-500 focus:outline-none focus:border-brand-500 transition font-sans"
          />
          <div v-if="isLoading" class="absolute right-2.5 top-1/2 -translate-y-1/2">
            <Loader2 class="w-3.5 h-3.5 text-brand-400 animate-spin" />
          </div>
          <button
            v-else-if="searchQuery"
            type="button"
            @click="searchQuery = ''; fetchClients('')"
            class="absolute right-2.5 top-1/2 -translate-y-1/2 text-slate-400 hover:text-white"
          >
            <X class="w-3 h-3" />
          </button>
        </div>
      </div>

      <!-- Clients List -->
      <div class="max-h-60 overflow-y-auto divide-y divide-slate-800/60 p-1">
        <div v-if="isLoading && clients.length === 0" class="py-6 text-center text-xs text-slate-400 flex items-center justify-center gap-2">
          <Loader2 class="w-4 h-4 animate-spin text-brand-400" />
          <span>Mencari data klien...</span>
        </div>

        <div v-else-if="clients.length === 0" class="py-6 text-center text-xs text-slate-500">
          <p>Tidak ditemukan klien dengan kata kunci "{{ searchQuery }}"</p>
          <p class="text-[10px] text-slate-600 mt-0.5">Pastikan kata kunci sesuai nama, perusahaan, atau email.</p>
        </div>

        <div
          v-else
          v-for="c in clients"
          :key="c.id"
          @click="selectClient(c)"
          class="p-2.5 rounded-lg flex items-center justify-between cursor-pointer transition text-left group"
          :class="modelValue === c.id ? 'bg-brand-500/15 border border-brand-500/30' : 'hover:bg-slate-800/70 border border-transparent'"
        >
          <div class="flex items-center gap-2.5 min-w-0 flex-1">
            <div
              class="w-7 h-7 rounded-lg flex items-center justify-center font-bold text-xs uppercase shrink-0 transition"
              :class="modelValue === c.id ? 'bg-brand-500 text-white shadow-xs' : 'bg-slate-800 text-slate-300 group-hover:bg-slate-700'"
            >
              {{ c.client_name.charAt(0) || 'C' }}
            </div>

            <div class="min-w-0 flex-1">
              <div class="flex items-center gap-2">
                <span class="text-xs font-bold text-white group-hover:text-brand-300 transition truncate">
                  {{ c.client_name }}
                </span>
                <span class="text-[10px] px-1.5 py-0.2 rounded bg-slate-800 text-slate-400 font-mono truncate">
                  {{ c.company_name }}
                </span>
              </div>
              <div class="flex items-center gap-3 text-[10px] text-slate-400 font-mono mt-0.5">
                <span v-if="c.email" class="flex items-center gap-1 truncate">
                  <Mail class="w-2.5 h-2.5 text-slate-500 shrink-0" />
                  {{ c.email }}
                </span>
                <span v-if="c.phone" class="flex items-center gap-1 shrink-0">
                  <Phone class="w-2.5 h-2.5 text-slate-500 shrink-0" />
                  {{ c.phone }}
                </span>
              </div>
            </div>
          </div>

          <div v-if="modelValue === c.id" class="ml-2 text-brand-400 shrink-0">
            <Check class="w-4 h-4" />
          </div>
        </div>
      </div>

      <!-- Footer Info -->
      <div class="p-2 border-t border-slate-800 bg-slate-950/60 text-[10px] text-slate-400 flex items-center justify-between">
        <span>Menampilkan {{ clients.length }} klien</span>
        <span class="font-mono text-slate-500">Pencarian Real-Time Sisi Server</span>
      </div>
    </div>
  </div>
</template>

