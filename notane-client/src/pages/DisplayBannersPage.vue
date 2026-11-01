<script setup lang="ts">
import { ref, onMounted, computed, watch } from 'vue'
import AdminLayout from '@/layouts/AdminLayout.vue'
import PaginationBar from '@/components/common/PaginationBar.vue'
import { api } from '@/services/api'
import type { CustomerDisplayBanner } from '@/types'
import {
  Plus,
  Search,
  Trash2,
  Edit2,
  X,
  Calendar,
  CheckCircle2,
  ExternalLink,
  Image as ImageIcon,
  ArrowUp,
  ArrowDown,
  Building2,
  Layers,
  Upload,
  LayoutGrid,
  LayoutList,
  RefreshCw,
} from 'lucide-vue-next'
import { useDialog } from '@/composables/useDialog'
import { Icon } from '@iconify/vue'

const dialog = useDialog()

// Data State
const banners = ref<CustomerDisplayBanner[]>([])
const isLoading = ref<boolean>(false)
const searchQuery = ref<string>('')
const statusFilter = ref<'ALL' | 'ACTIVE' | 'INACTIVE'>('ALL')
const sortBy = ref<string>('display_order')
const sortDir = ref<'asc' | 'desc'>('asc')
const viewMode = ref<'grid' | 'table'>('grid')

// Server-side Pagination State
const currentPage = ref<number>(1)
const pageSize = ref<number>(12)
const totalItems = ref<number>(0)
const totalPages = ref<number>(1)

// Summary Statistics from Server
const summary = ref({
  total: 0,
  active: 0,
  inactive: 0,
  brands: 0,
})

// Modal Form State
const showModal = ref<boolean>(false)
const isEditing = ref<boolean>(false)
const isSubmitting = ref<boolean>(false)
const currentBannerId = ref<string | null>(null)

const form = ref({
  title: '',
  subtitle: '',
  brand_name: '',
  badge_text: 'PROMO SPESIAL',
  image_url: '',
  is_active: true,
  display_order: 1,
  start_date: '',
  end_date: '',
})

const formErrors = ref<Record<string, string>>({})
const fileInputRef = ref<HTMLInputElement | null>(null)

let searchDebounceTimer: any = null

function onSearchInput() {
  if (searchDebounceTimer) clearTimeout(searchDebounceTimer)
  searchDebounceTimer = setTimeout(() => {
    currentPage.value = 1
    fetchBanners()
  }, 350)
}

function handleStatusFilter(status: 'ALL' | 'ACTIVE' | 'INACTIVE') {
  statusFilter.value = status
  currentPage.value = 1
  fetchBanners()
}

function handleSort(col: string) {
  if (sortBy.value === col) {
    sortDir.value = sortDir.value === 'asc' ? 'desc' : 'asc'
  } else {
    sortBy.value = col
    sortDir.value = col === 'created_at' || col === 'start_date' || col === 'end_date' ? 'desc' : 'asc'
  }
  currentPage.value = 1
  fetchBanners()
}

// Quick sort dropdown helper
const currentSortOption = computed({
  get: () => `${sortBy.value}-${sortDir.value}`,
  set: (val: string) => {
    const parts = val.split('-')
    sortBy.value = parts[0]
    sortDir.value = (parts[1] as 'asc' | 'desc') || 'asc'
    currentPage.value = 1
    fetchBanners()
  },
})

async function fetchBanners() {
  isLoading.value = true
  try {
    const res = await api.get('/display-banners', {
      params: {
        search: searchQuery.value.trim() || undefined,
        status: statusFilter.value,
        page: currentPage.value,
        page_size: pageSize.value,
        sort_by: sortBy.value,
        sort_dir: sortDir.value,
      },
    })
    if (res.data?.success) {
      banners.value = Array.isArray(res.data.data) ? res.data.data : []
      totalItems.value = res.data.total_items ?? res.data.total ?? banners.value.length
      totalPages.value = res.data.total_pages ?? Math.max(1, Math.ceil(totalItems.value / pageSize.value))
      if (res.data.summary) {
        summary.value = res.data.summary
      }
    }
  } catch (err) {
    console.error('Gagal memuat banner customer display:', err)
  } finally {
    isLoading.value = false
  }
}

watch(currentPage, () => {
  fetchBanners()
})

watch(pageSize, () => {
  currentPage.value = 1
  fetchBanners()
})

function openCreateModal() {
  isEditing.value = false
  currentBannerId.value = null
  const nextOrder = banners.value.length > 0
    ? Math.max(...banners.value.map((b) => b.display_order || 0)) + 1
    : 1

  form.value = {
    title: '',
    subtitle: '',
    brand_name: '',
    badge_text: 'PROMO SPESIAL',
    image_url: '',
    is_active: true,
    display_order: nextOrder,
    start_date: '',
    end_date: '',
  }
  formErrors.value = {}
  showModal.value = true
}

function openEditModal(banner: CustomerDisplayBanner) {
  isEditing.value = true
  currentBannerId.value = banner.id
  form.value = {
    title: banner.title,
    subtitle: banner.subtitle || '',
    brand_name: banner.brand_name || '',
    badge_text: banner.badge_text || 'PROMO SPESIAL',
    image_url: banner.image_url,
    is_active: banner.is_active,
    display_order: banner.display_order,
    start_date: banner.start_date || '',
    end_date: banner.end_date || '',
  }
  formErrors.value = {}
  showModal.value = true
}

function handleFileUpload(e: Event) {
  const target = e.target as HTMLInputElement
  if (!target.files || target.files.length === 0) return

  const file = target.files[0]
  if (file.size > 5 * 1024 * 1024) {
    dialog.alert('Ukuran file maksimal adalah 5MB.')
    return
  }

  const reader = new FileReader()
  reader.onload = (event) => {
    if (event.target?.result) {
      form.value.image_url = event.target.result as string
    }
  }
  reader.readAsDataURL(file)
}

function validateForm(): boolean {
  formErrors.value = {}
  if (!form.value.title.trim()) {
    formErrors.value.title = 'Judul promosi wajib diisi'
  }
  if (!form.value.image_url.trim()) {
    formErrors.value.image_url = 'Gambar banner promosi wajib diunggah atau diisi tautannya'
  }
  if (form.value.start_date && form.value.end_date && form.value.start_date > form.value.end_date) {
    formErrors.value.end_date = 'Tanggal selesai tidak boleh lebih awal dari tanggal mulai'
  }
  return Object.keys(formErrors.value).length === 0
}

async function handleSaveBanner() {
  if (!validateForm()) return

  isSubmitting.value = true
  try {
    const payload = {
      title: form.value.title.trim(),
      subtitle: form.value.subtitle.trim() || undefined,
      brand_name: form.value.brand_name.trim() || undefined,
      badge_text: form.value.badge_text.trim() || undefined,
      image_url: form.value.image_url.trim(),
      is_active: form.value.is_active,
      display_order: Number(form.value.display_order) || 0,
      start_date: form.value.start_date || undefined,
      end_date: form.value.end_date || undefined,
    }

    if (isEditing.value && currentBannerId.value) {
      await api.put(`/display-banners/${currentBannerId.value}`, payload)
      dialog.alert('Banner promosi berhasil diperbarui!')
    } else {
      await api.post('/display-banners', payload)
      dialog.alert('Banner promosi baru berhasil ditambahkan!')
    }

    showModal.value = false
    await fetchBanners()
  } catch (err: any) {
    console.error('Error saving banner:', err)
    dialog.alert(err.response?.data?.message || 'Gagal menyimpan banner promosi')
  } finally {
    isSubmitting.value = false
  }
}

async function toggleActive(banner: CustomerDisplayBanner) {
  try {
    const updatedStatus = !banner.is_active
    await api.put(`/display-banners/${banner.id}`, {
      is_active: updatedStatus,
    })
    banner.is_active = updatedStatus
    if (statusFilter.value !== 'ALL') {
      await fetchBanners()
    } else {
      summary.value.active += updatedStatus ? 1 : -1
      summary.value.inactive += updatedStatus ? -1 : 1
    }
  } catch (err: any) {
    console.error('Gagal mengubah status banner:', err)
    dialog.alert(err.response?.data?.message || 'Gagal mengubah status banner')
  }
}

async function moveOrder(banner: CustomerDisplayBanner, direction: 'UP' | 'DOWN') {
  const currentIdx = banners.value.findIndex((b) => b.id === banner.id)
  if (currentIdx < 0) return

  const targetIdx = direction === 'UP' ? currentIdx - 1 : currentIdx + 1
  if (targetIdx < 0 || targetIdx >= banners.value.length) return

  const targetBanner = banners.value[targetIdx]
  const tempOrder = banner.display_order
  banner.display_order = targetBanner.display_order
  targetBanner.display_order = tempOrder

  try {
    await api.post('/display-banners/reorder', {
      items: [
        { id: banner.id, display_order: banner.display_order },
        { id: targetBanner.id, display_order: targetBanner.display_order },
      ],
    })
    await fetchBanners()
  } catch (err) {
    console.error('Gagal mengubah urutan banner:', err)
    await fetchBanners()
  }
}

async function handleDelete(banner: CustomerDisplayBanner) {
  const confirmed = await dialog.confirm(
    `Apakah Anda yakin ingin menghapus banner promosi "${banner.title}"?`
  )
  if (!confirmed) return

  try {
    await api.delete(`/display-banners/${banner.id}`)
    dialog.alert('Banner promosi berhasil dihapus')
    await fetchBanners()
  } catch (err: any) {
    console.error('Error deleting banner:', err)
    dialog.alert(err.response?.data?.message || 'Gagal menghapus banner promosi')
  }
}

function openCustomerDisplayPreview() {
  const width = 1024
  const height = 768
  const left = window.screen.width - width
  const top = 0
  window.open(
    '/customer-display',
    'CustomerDisplayWindow',
    `width=${width},height=${height},left=${left},top=${top},menubar=no,toolbar=no,location=no,status=no`
  )
}

function formatDateDisplay(d?: string | null): string {
  if (!d) return '-'
  return d
}

onMounted(() => {
  fetchBanners()
})
</script>

<template>
  <AdminLayout>
    <div class="p-6 space-y-6">
      <!-- 1. Header Section -->
      <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4 bg-white p-6 rounded-2xl border border-slate-200/80 shadow-xs">
        <div>
          <div class="flex items-center gap-2.5">
            <div class="w-10 h-10 rounded-xl bg-blue-50 border border-blue-200 flex items-center justify-center text-blue-600 shadow-2xs">
              <Icon icon="fluent-color:image-24" class="w-6 h-6" />
            </div>
            <div>
              <h1 class="text-xl font-black text-slate-900 tracking-tight">Banner Promosi Layar Pelanggan</h1>
              <p class="text-xs text-slate-500 mt-0.5">
                Kelola materi promosi toko dan brand mitra yang ditayangkan pada Layar Kedua (Customer Display)
              </p>
            </div>
          </div>
        </div>

        <div class="flex items-center gap-2.5">
          <button
            type="button"
            @click="fetchBanners"
            class="p-2 rounded-xl border border-slate-300 bg-white hover:bg-slate-50 text-slate-700 transition cursor-pointer shadow-2xs"
            title="Muat ulang data banner"
          >
            <RefreshCw class="w-4 h-4 text-slate-500" :class="{ 'animate-spin': isLoading }" />
          </button>

          <button
            type="button"
            @click="openCustomerDisplayPreview"
            class="px-3.5 py-2 rounded-xl border border-slate-300 bg-white hover:bg-slate-50 text-slate-700 font-bold text-xs flex items-center gap-1.5 transition active:scale-95 cursor-pointer shadow-2xs"
            title="Buka layar customer display untuk melihat tayangan langsung"
          >
            <ExternalLink class="w-4 h-4 text-slate-500" />
            <span>Pratinjau Layar Pelanggan</span>
          </button>

          <button
            type="button"
            @click="openCreateModal"
            class="px-4 py-2 rounded-xl bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-xs flex items-center gap-1.5 shadow-sm transition active:scale-95 cursor-pointer"
          >
            <Plus class="w-4 h-4" />
            <span>Tambah Banner Baru</span>
          </button>
        </div>
      </div>

      <!-- 2. Stat Cards (Server-side Aggregation) -->
      <div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
        <div class="bg-white p-5 rounded-2xl border border-slate-200/80 shadow-xs flex items-center justify-between">
          <div>
            <span class="text-xs font-bold text-slate-500 uppercase tracking-wider">Total Banner</span>
            <div class="text-2xl font-black text-slate-900 mt-1 font-mono">{{ summary.total }}</div>
          </div>
          <div class="w-11 h-11 rounded-xl bg-slate-100 flex items-center justify-center text-slate-600">
            <Layers class="w-5 h-5" />
          </div>
        </div>

        <div class="bg-white p-5 rounded-2xl border border-slate-200/80 shadow-xs flex items-center justify-between">
          <div>
            <span class="text-xs font-bold text-emerald-600 uppercase tracking-wider">Sedang Aktif Ditayangkan</span>
            <div class="text-2xl font-black text-emerald-700 mt-1 font-mono">{{ summary.active }}</div>
          </div>
          <div class="w-11 h-11 rounded-xl bg-emerald-50 flex items-center justify-center text-emerald-600">
            <CheckCircle2 class="w-5 h-5" />
          </div>
        </div>

        <div class="bg-white p-5 rounded-2xl border border-slate-200/80 shadow-xs flex items-center justify-between">
          <div>
            <span class="text-xs font-bold text-indigo-600 uppercase tracking-wider">Mitra Brand & Sponsor</span>
            <div class="text-2xl font-black text-indigo-700 mt-1 font-mono">{{ summary.brands }}</div>
          </div>
          <div class="w-11 h-11 rounded-xl bg-indigo-50 flex items-center justify-center text-indigo-600">
            <Building2 class="w-5 h-5" />
          </div>
        </div>
      </div>

      <!-- 3. Filter & Controls Bar -->
      <div class="flex flex-col lg:flex-row lg:items-center justify-between gap-3 bg-white p-4 rounded-2xl border border-slate-200/80 shadow-xs">
        <!-- Status Tabs -->
        <div class="flex items-center gap-1 bg-slate-100 p-1 rounded-xl text-xs font-bold shrink-0">
          <button
            type="button"
            @click="handleStatusFilter('ALL')"
            class="px-3.5 py-1.5 rounded-lg transition cursor-pointer"
            :class="statusFilter === 'ALL' ? 'bg-white text-slate-900 shadow-2xs font-extrabold' : 'text-slate-600 hover:text-slate-900'"
          >
            Semua ({{ summary.total }})
          </button>
          <button
            type="button"
            @click="handleStatusFilter('ACTIVE')"
            class="px-3.5 py-1.5 rounded-lg transition cursor-pointer"
            :class="statusFilter === 'ACTIVE' ? 'bg-white text-emerald-700 shadow-2xs font-extrabold' : 'text-slate-600 hover:text-slate-900'"
          >
            Aktif ({{ summary.active }})
          </button>
          <button
            type="button"
            @click="handleStatusFilter('INACTIVE')"
            class="px-3.5 py-1.5 rounded-lg transition cursor-pointer"
            :class="statusFilter === 'INACTIVE' ? 'bg-white text-rose-700 shadow-2xs font-extrabold' : 'text-slate-600 hover:text-slate-900'"
          >
            Nonaktif ({{ summary.inactive }})
          </button>
        </div>

        <!-- Right Controls: Search, Sort Dropdown, and View Mode Switcher -->
        <div class="flex flex-wrap items-center gap-2.5">
          <!-- Search Input (Server-side) -->
          <div class="relative w-full sm:w-64">
            <Search class="w-4 h-4 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2" />
            <input
              v-model="searchQuery"
              @input="onSearchInput"
              type="text"
              placeholder="Cari judul atau brand..."
              class="w-full pl-9 pr-3.5 py-2 text-xs bg-slate-50 border border-slate-300 rounded-xl focus:bg-white focus:outline-none focus:border-emerald-600 transition"
            />
          </div>

          <!-- Quick Sort Dropdown (Server-side) -->
          <div class="flex items-center gap-1.5 bg-slate-50 border border-slate-300 rounded-xl px-2.5 py-1.5 text-xs">
            <ArrowUpDown class="w-3.5 h-3.5 text-slate-500 shrink-0" />
            <select
              v-model="currentSortOption"
              class="bg-transparent border-none text-xs font-semibold text-slate-700 focus:outline-none cursor-pointer pr-1"
            >
              <option value="display_order-asc">Urutan (1 &rarr; 9)</option>
              <option value="display_order-desc">Urutan (9 &rarr; 1)</option>
              <option value="title-asc">Judul Promosi (A-Z)</option>
              <option value="title-desc">Judul Promosi (Z-A)</option>
              <option value="brand_name-asc">Brand Mitra (A-Z)</option>
              <option value="created_at-desc">Dibuat (Terbaru)</option>
              <option value="start_date-desc">Masa Berlaku (Mulai)</option>
            </select>
          </div>

          <!-- View Mode Toggle: Grid vs Table -->
          <div class="flex items-center bg-slate-100 p-1 rounded-xl border border-slate-200">
            <button
              type="button"
              @click="viewMode = 'grid'"
              class="p-1.5 rounded-lg transition cursor-pointer"
              :class="viewMode === 'grid' ? 'bg-white text-emerald-700 shadow-2xs' : 'text-slate-500 hover:text-slate-800'"
              title="Tampilan Grid Card"
            >
              <LayoutGrid class="w-4 h-4" />
            </button>
            <button
              type="button"
              @click="viewMode = 'table'"
              class="p-1.5 rounded-lg transition cursor-pointer"
              :class="viewMode === 'table' ? 'bg-white text-emerald-700 shadow-2xs' : 'text-slate-500 hover:text-slate-800'"
              title="Tampilan Tabel Data"
            >
              <LayoutList class="w-4 h-4" />
            </button>
          </div>
        </div>
      </div>

      <!-- 4. Content Area: Grid View / Table View / Loading / Empty -->
      <div v-if="isLoading && banners.length === 0" class="p-16 text-center text-slate-400 text-xs font-bold bg-white rounded-2xl border border-slate-200">
        <RefreshCw class="w-6 h-6 text-slate-400 mx-auto mb-2 animate-spin" />
        Memuat data banner promosi...
      </div>

      <div
        v-else-if="banners.length === 0"
        class="bg-white rounded-2xl border border-dashed border-slate-300 p-12 text-center"
      >
        <ImageIcon class="w-12 h-12 text-slate-300 mx-auto mb-3" />
        <h3 class="text-sm font-bold text-slate-700">Belum Ada Banner Promosi</h3>
        <p class="text-xs text-slate-500 mt-1 max-w-sm mx-auto">
          Mulai tambahkan banner promosi toko atau materi kerja sama dengan brand mitra untuk ditayangkan di layar pelanggan.
        </p>
        <button
          type="button"
          @click="openCreateModal"
          class="mt-4 px-4 py-2 bg-emerald-600 hover:bg-emerald-700 text-white rounded-xl font-bold text-xs inline-flex items-center gap-1.5 transition active:scale-95 cursor-pointer"
        >
          <Plus class="w-4 h-4" />
          <span>Tambah Banner Pertama</span>
        </button>
      </div>

      <!-- VIEW A: GRID CARD VIEW -->
      <div v-else-if="viewMode === 'grid'" class="space-y-4">
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
          <div
            v-for="(banner, idx) in banners"
            :key="banner.id"
            class="bg-white rounded-2xl border border-slate-200/80 shadow-xs overflow-hidden flex flex-col justify-between transition-all hover:shadow-md hover:border-slate-300"
          >
            <div>
              <!-- Banner Preview Container -->
              <div class="relative aspect-16/9 bg-slate-900 overflow-hidden group">
                <img
                  :src="banner.image_url"
                  :alt="banner.title"
                  class="w-full h-full object-cover transition duration-300 group-hover:scale-105"
                  loading="lazy"
                />

                <!-- Overlay -->
                <div class="absolute inset-0 bg-gradient-to-t from-black/80 via-black/20 to-transparent"></div>

                <!-- Badges -->
                <div class="absolute top-2.5 left-2.5 flex flex-wrap gap-1.5">
                  <span class="px-2 py-0.5 rounded-md text-[10px] font-black uppercase tracking-wider bg-emerald-500 text-white shadow-xs">
                    {{ banner.badge_text || 'PROMO' }}
                  </span>
                  <span
                    v-if="banner.brand_name"
                    class="px-2 py-0.5 rounded-md text-[10px] font-bold text-indigo-900 bg-indigo-100 border border-indigo-300/80 shadow-xs flex items-center gap-1 truncate max-w-[140px]"
                  >
                    <Building2 class="w-3 h-3 text-indigo-600 shrink-0" />
                    <span class="truncate">{{ banner.brand_name }}</span>
                  </span>
                </div>

                <!-- Display Order Badge -->
                <div class="absolute top-2.5 right-2.5 bg-slate-900/85 border border-white/20 text-white text-[10px] font-mono font-bold px-2 py-0.5 rounded-md shadow-xs">
                  #{{ banner.display_order }}
                </div>

                <!-- Bottom Content Overlay -->
                <div class="absolute bottom-2.5 inset-x-2.5 text-white">
                  <h4 class="font-black text-xs leading-snug line-clamp-1 drop-shadow-xs">
                    {{ banner.title }}
                  </h4>
                  <p v-if="banner.subtitle" class="text-[10px] text-slate-200 line-clamp-1 mt-0.5 opacity-90 drop-shadow-xs">
                    {{ banner.subtitle }}
                  </p>
                </div>
              </div>

              <!-- Detail Info -->
              <div class="p-3.5 space-y-2">
                <div class="flex items-center justify-between text-[11px] text-slate-500">
                  <span class="flex items-center gap-1 font-medium truncate">
                    <Calendar class="w-3.5 h-3.5 text-slate-400 shrink-0" />
                    <span v-if="banner.start_date || banner.end_date" class="truncate">
                      {{ banner.start_date || 'Awal' }} s/d {{ banner.end_date || 'Seterusnya' }}
                    </span>
                    <span v-else class="text-slate-400 italic">Berlaku seterusnya</span>
                  </span>

                  <div class="flex items-center gap-1.5 shrink-0 ml-2">
                    <span
                      class="w-2 h-2 rounded-full"
                      :class="banner.is_active ? 'bg-emerald-500' : 'bg-slate-300'"
                    ></span>
                    <span class="font-bold text-[10px]" :class="banner.is_active ? 'text-emerald-700' : 'text-slate-400'">
                      {{ banner.is_active ? 'Aktif' : 'Nonaktif' }}
                    </span>
                  </div>
                </div>
              </div>
            </div>

            <!-- Card Actions Footer -->
            <div class="px-3.5 py-2.5 bg-slate-50 border-t border-slate-100 flex items-center justify-between">
              <!-- Reorder Buttons -->
              <div class="flex items-center gap-1">
                <button
                  type="button"
                  @click="moveOrder(banner, 'UP')"
                  :disabled="idx === 0 && currentPage === 1"
                  class="p-1 rounded-lg border border-slate-200 text-slate-500 hover:bg-white hover:text-slate-800 disabled:opacity-30 cursor-pointer"
                  title="Pindah urutan ke atas"
                >
                  <ArrowUp class="w-3.5 h-3.5" />
                </button>
                <button
                  type="button"
                  @click="moveOrder(banner, 'DOWN')"
                  :disabled="idx === banners.length - 1 && currentPage === totalPages"
                  class="p-1 rounded-lg border border-slate-200 text-slate-500 hover:bg-white hover:text-slate-800 disabled:opacity-30 cursor-pointer"
                  title="Pindah urutan ke bawah"
                >
                  <ArrowDown class="w-3.5 h-3.5" />
                </button>
              </div>

              <!-- Action Buttons -->
              <div class="flex items-center gap-1.5">
                <button
                  type="button"
                  @click="toggleActive(banner)"
                  class="text-[11px] px-2 py-0.5 rounded-lg font-bold border transition cursor-pointer"
                  :class="banner.is_active
                    ? 'bg-emerald-50 text-emerald-800 border-emerald-300 hover:bg-emerald-100'
                    : 'bg-slate-200 text-slate-700 border-slate-300 hover:bg-slate-300'"
                >
                  {{ banner.is_active ? 'Aktif' : 'Nyalakan' }}
                </button>

                <button
                  type="button"
                  @click="openEditModal(banner)"
                  class="p-1 rounded-lg border border-slate-200 bg-white hover:bg-slate-100 text-slate-600 transition cursor-pointer"
                  title="Ubah Banner"
                >
                  <Edit2 class="w-3.5 h-3.5" />
                </button>

                <button
                  type="button"
                  @click="handleDelete(banner)"
                  class="p-1 rounded-lg border border-rose-200 bg-rose-50 hover:bg-rose-100 text-rose-600 transition cursor-pointer"
                  title="Hapus Banner"
                >
                  <Trash2 class="w-3.5 h-3.5" />
                </button>
              </div>
            </div>
          </div>
        </div>

        <!-- Server-side Pagination Bar for Grid View -->
        <PaginationBar
          v-model:currentPage="currentPage"
          v-model:pageSize="pageSize"
          :totalItems="totalItems"
          :pageSizeOptions="[8, 12, 24, 48]"
        />
      </div>

      <!-- VIEW B: TABLE DATA VIEW WITH SORTING HEADERS -->
      <div v-else class="bg-white rounded-2xl border border-slate-200 shadow-xs overflow-hidden">
        <div class="overflow-x-auto">
          <table class="w-full text-left border-collapse text-xs">
            <thead>
              <tr class="bg-slate-100/90 text-slate-700 font-bold border-b border-slate-200 select-none">
                <!-- Col: Urutan -->
                <th
                  class="py-3 px-3.5 cursor-pointer hover:bg-slate-200/70 transition w-24"
                  @click="handleSort('display_order')"
                >
                  <div class="flex items-center gap-1.5">
                    <span># Urutan</span>
                    <ArrowUp v-if="sortBy === 'display_order' && sortDir === 'asc'" class="w-3.5 h-3.5 text-emerald-600" />
                    <ArrowDown v-else-if="sortBy === 'display_order' && sortDir === 'desc'" class="w-3.5 h-3.5 text-emerald-600" />
                    <ArrowUpDown v-else class="w-3 h-3 text-slate-400" />
                  </div>
                </th>

                <!-- Col: Banner & Judul -->
                <th
                  class="py-3 px-3.5 cursor-pointer hover:bg-slate-200/70 transition"
                  @click="handleSort('title')"
                >
                  <div class="flex items-center gap-1.5">
                    <span>Banner & Judul Promosi</span>
                    <ArrowUp v-if="sortBy === 'title' && sortDir === 'asc'" class="w-3.5 h-3.5 text-emerald-600" />
                    <ArrowDown v-else-if="sortBy === 'title' && sortDir === 'desc'" class="w-3.5 h-3.5 text-emerald-600" />
                    <ArrowUpDown v-else class="w-3 h-3 text-slate-400" />
                  </div>
                </th>

                <!-- Col: Brand Mitra -->
                <th
                  class="py-3 px-3.5 cursor-pointer hover:bg-slate-200/70 transition"
                  @click="handleSort('brand_name')"
                >
                  <div class="flex items-center gap-1.5">
                    <span>Brand Mitra</span>
                    <ArrowUp v-if="sortBy === 'brand_name' && sortDir === 'asc'" class="w-3.5 h-3.5 text-emerald-600" />
                    <ArrowDown v-else-if="sortBy === 'brand_name' && sortDir === 'desc'" class="w-3.5 h-3.5 text-emerald-600" />
                    <ArrowUpDown v-else class="w-3 h-3 text-slate-400" />
                  </div>
                </th>

                <!-- Col: Masa Berlaku -->
                <th
                  class="py-3 px-3.5 cursor-pointer hover:bg-slate-200/70 transition"
                  @click="handleSort('start_date')"
                >
                  <div class="flex items-center gap-1.5">
                    <span>Masa Berlaku</span>
                    <ArrowUp v-if="sortBy === 'start_date' && sortDir === 'asc'" class="w-3.5 h-3.5 text-emerald-600" />
                    <ArrowDown v-else-if="sortBy === 'start_date' && sortDir === 'desc'" class="w-3.5 h-3.5 text-emerald-600" />
                    <ArrowUpDown v-else class="w-3 h-3 text-slate-400" />
                  </div>
                </th>

                <!-- Col: Status -->
                <th
                  class="py-3 px-3.5 cursor-pointer hover:bg-slate-200/70 transition w-28"
                  @click="handleSort('is_active')"
                >
                  <div class="flex items-center gap-1.5">
                    <span>Status</span>
                    <ArrowUp v-if="sortBy === 'is_active' && sortDir === 'asc'" class="w-3.5 h-3.5 text-emerald-600" />
                    <ArrowDown v-else-if="sortBy === 'is_active' && sortDir === 'desc'" class="w-3.5 h-3.5 text-emerald-600" />
                    <ArrowUpDown v-else class="w-3 h-3 text-slate-400" />
                  </div>
                </th>

                <!-- Col: Dibuat -->
                <th
                  class="py-3 px-3.5 cursor-pointer hover:bg-slate-200/70 transition w-32"
                  @click="handleSort('created_at')"
                >
                  <div class="flex items-center gap-1.5">
                    <span>Dibuat</span>
                    <ArrowUp v-if="sortBy === 'created_at' && sortDir === 'asc'" class="w-3.5 h-3.5 text-emerald-600" />
                    <ArrowDown v-else-if="sortBy === 'created_at' && sortDir === 'desc'" class="w-3.5 h-3.5 text-emerald-600" />
                    <ArrowUpDown v-else class="w-3 h-3 text-slate-400" />
                  </div>
                </th>

                <!-- Col: Aksi -->
                <th class="py-3 px-3.5 text-right w-44">
                  <span>Aksi</span>
                </th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-100">
              <tr
                v-for="(banner, idx) in banners"
                :key="banner.id"
                class="hover:bg-slate-50/80 transition"
              >
                <!-- Urutan & Move buttons -->
                <td class="py-3 px-3.5 font-mono font-bold text-slate-800">
                  <div class="flex items-center gap-1.5">
                    <span class="w-7 text-center">#{{ banner.display_order }}</span>
                    <div class="flex flex-col gap-0.5">
                      <button
                        type="button"
                        @click="moveOrder(banner, 'UP')"
                        :disabled="idx === 0 && currentPage === 1"
                        class="p-0.5 rounded text-slate-400 hover:text-slate-800 disabled:opacity-20 cursor-pointer"
                        title="Geser ke atas"
                      >
                        <ArrowUp class="w-3 h-3" />
                      </button>
                      <button
                        type="button"
                        @click="moveOrder(banner, 'DOWN')"
                        :disabled="idx === banners.length - 1 && currentPage === totalPages"
                        class="p-0.5 rounded text-slate-400 hover:text-slate-800 disabled:opacity-20 cursor-pointer"
                        title="Geser ke bawah"
                      >
                        <ArrowDown class="w-3 h-3" />
                      </button>
                    </div>
                  </div>
                </td>

                <!-- Banner Image & Judul -->
                <td class="py-3 px-3.5">
                  <div class="flex items-center gap-3">
                    <div class="w-16 h-10 rounded-lg overflow-hidden border border-slate-200 bg-slate-900 shrink-0 shadow-2xs">
                      <img :src="banner.image_url" :alt="banner.title" class="w-full h-full object-cover" loading="lazy" />
                    </div>
                    <div>
                      <div class="flex items-center gap-1.5">
                        <span class="font-bold text-slate-900 leading-tight">{{ banner.title }}</span>
                        <span class="px-1.5 py-0.5 rounded text-[9px] font-black uppercase tracking-wider bg-emerald-100 text-emerald-800 border border-emerald-200">
                          {{ banner.badge_text || 'PROMO' }}
                        </span>
                      </div>
                      <p v-if="banner.subtitle" class="text-[11px] text-slate-500 line-clamp-1 mt-0.5">
                        {{ banner.subtitle }}
                      </p>
                    </div>
                  </div>
                </td>

                <!-- Brand Mitra -->
                <td class="py-3 px-3.5">
                  <span
                    v-if="banner.brand_name"
                    class="inline-flex items-center gap-1 px-2.5 py-1 rounded-md text-xs font-bold text-indigo-900 bg-indigo-50 border border-indigo-200"
                  >
                    <Building2 class="w-3 h-3 text-indigo-600" />
                    {{ banner.brand_name }}
                  </span>
                  <span v-else class="text-slate-400 italic text-[11px]">- (Toko Sendiri)</span>
                </td>

                <!-- Masa Berlaku -->
                <td class="py-3 px-3.5 text-slate-600 text-[11px]">
                  <div v-if="banner.start_date || banner.end_date" class="font-mono">
                    <div>{{ formatDateDisplay(banner.start_date) }}</div>
                    <div class="text-slate-400 text-[10px]">s/d {{ formatDateDisplay(banner.end_date) }}</div>
                  </div>
                  <span v-else class="text-slate-400 italic">Seterusnya</span>
                </td>

                <!-- Status Aktif -->
                <td class="py-3 px-3.5">
                  <button
                    type="button"
                    @click="toggleActive(banner)"
                    class="px-2.5 py-1 rounded-full text-[10px] font-extrabold uppercase tracking-wider border transition cursor-pointer"
                    :class="banner.is_active
                      ? 'bg-emerald-50 text-emerald-700 border-emerald-300 hover:bg-emerald-100'
                      : 'bg-slate-100 text-slate-600 border-slate-300 hover:bg-slate-200'"
                  >
                    {{ banner.is_active ? 'Aktif' : 'Nonaktif' }}
                  </button>
                </td>

                <!-- Tanggal Dibuat -->
                <td class="py-3 px-3.5 text-slate-500 font-mono text-[11px]">
                  {{ banner.created_at ? banner.created_at.substring(0, 10) : '-' }}
                </td>

                <!-- Aksi -->
                <td class="py-3 px-3.5 text-right">
                  <div class="flex items-center justify-end gap-1.5">
                    <button
                      type="button"
                      @click="openEditModal(banner)"
                      class="px-2 py-1 rounded-lg border border-slate-200 bg-white hover:bg-slate-100 text-slate-700 font-bold text-xs flex items-center gap-1 transition cursor-pointer shadow-2xs"
                      title="Ubah banner"
                    >
                      <Edit2 class="w-3 h-3 text-slate-500" />
                      <span>Edit</span>
                    </button>
                    <button
                      type="button"
                      @click="handleDelete(banner)"
                      class="p-1 rounded-lg border border-rose-200 bg-rose-50 hover:bg-rose-100 text-rose-600 transition cursor-pointer"
                      title="Hapus banner"
                    >
                      <Trash2 class="w-3.5 h-3.5" />
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <!-- Server-side Pagination Bar for Table View -->
        <PaginationBar
          v-model:currentPage="currentPage"
          v-model:pageSize="pageSize"
          :totalItems="totalItems"
          :pageSizeOptions="[10, 20, 50, 100]"
        />
      </div>

      <!-- 5. Form Modal Create/Edit (Teleport to body with dedicated fixed backdrop) -->
      <Teleport to="body">
        <div
          v-if="showModal"
          class="fixed inset-0 z-50 overflow-y-auto"
        >
          <!-- Dedicated Fixed Full-Screen Backdrop (Always covers 100vw x 100vh) -->
          <div
            class="fixed inset-0 bg-slate-900/60 backdrop-blur-xs transition-opacity"
            @click="showModal = false"
          ></div>

          <!-- Centered Scrollable Dialog Container -->
          <div class="flex min-h-full items-center justify-center p-4 text-center pointer-events-none">
            <div
              class="relative bg-white rounded-2xl max-w-xl w-full border border-slate-300 shadow-2xl overflow-hidden animate-in fade-in zoom-in-95 my-8 text-left pointer-events-auto"
            >
              <!-- Modal Header -->
              <div class="px-6 py-4 bg-slate-50 border-b border-slate-200 flex items-center justify-between">
                <div class="flex items-center gap-2.5">
                  <div class="w-9 h-9 rounded-xl bg-emerald-50 border border-emerald-200 flex items-center justify-center text-emerald-700">
                    <ImageIcon class="w-4 h-4" />
                  </div>
                  <h3 class="font-black text-sm text-slate-900">
                    {{ isEditing ? 'Edit Banner Promosi' : 'Tambah Banner Promosi Baru' }}
                  </h3>
                </div>
                <button @click="showModal = false" class="text-slate-400 hover:text-slate-700 cursor-pointer">
                  <X class="w-4 h-4" />
                </button>
              </div>

              <!-- Modal Body Form -->
              <form @submit.prevent="handleSaveBanner" class="p-6 space-y-4">
                <!-- 1. Judul Banner -->
                <div>
                  <label class="block text-xs font-bold text-slate-700 mb-1">
                    Judul Promosi / Penawaran <span class="text-rose-500">*</span>
                  </label>
                  <input
                    v-model="form.title"
                    type="text"
                    placeholder="Contoh: Promo Akhir Pekan Hemat 30% / Beli 2 Gratis 1"
                    class="w-full px-3.5 py-2 text-xs border border-slate-300 rounded-xl focus:outline-none focus:border-emerald-600"
                  />
                  <span v-if="formErrors.title" class="text-[11px] text-rose-500 font-semibold block mt-1">
                    {{ formErrors.title }}
                  </span>
                </div>

                <!-- 2. Brand Partner & Badge Text (2 Kolom) -->
                <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
                  <div>
                    <label class="block text-xs font-bold text-slate-700 mb-1">
                      Nama Brand Mitra / Sponsor (Opsional)
                    </label>
                    <input
                      v-model="form.brand_name"
                      type="text"
                      placeholder="Contoh: Unilever / Indofood / Nestle"
                      class="w-full px-3.5 py-2 text-xs border border-slate-300 rounded-xl focus:outline-none focus:border-emerald-600"
                    />
                    <span class="text-[10px] text-slate-400 block mt-0.5">
                      Isi jika banner adalah kerja sama promosi dengan brand pihak ketiga.
                    </span>
                  </div>

                  <div>
                    <label class="block text-xs font-bold text-slate-700 mb-1">
                      Label Badge Promosi
                    </label>
                    <input
                      v-model="form.badge_text"
                      type="text"
                      placeholder="Contoh: PROMO SPESIAL / SPONSOR RESMI"
                      class="w-full px-3.5 py-2 text-xs border border-slate-300 rounded-xl focus:outline-none focus:border-emerald-600"
                    />
                  </div>
                </div>

                <!-- 3. Subtitle / Deskripsi Singkat -->
                <div>
                  <label class="block text-xs font-bold text-slate-700 mb-1">
                    Keterangan Singkat / Tagline
                  </label>
                  <textarea
                    v-model="form.subtitle"
                    rows="2"
                    placeholder="Deskripsi singkat promo, ketentuan, atau pesan menarik untuk pelanggan..."
                    class="w-full px-3.5 py-2 text-xs border border-slate-300 rounded-xl focus:outline-none focus:border-emerald-600 resize-none"
                  ></textarea>
                </div>

                <!-- 4. Upload Gambar Banner & URL -->
                <div>
                  <label class="block text-xs font-bold text-slate-700 mb-1">
                    Gambar Banner Promosi <span class="text-rose-500">*</span>
                  </label>

                  <div class="flex items-center gap-2 mb-2">
                    <input
                      ref="fileInputRef"
                      type="file"
                      accept="image/png,image/jpeg,image/webp,image/svg+xml"
                      class="hidden"
                      @change="handleFileUpload"
                    />
                    <button
                      type="button"
                      @click="fileInputRef?.click()"
                      class="px-3 py-1.5 bg-slate-100 hover:bg-slate-200 text-slate-700 rounded-xl font-bold text-xs border border-slate-300 flex items-center gap-1.5 transition cursor-pointer"
                    >
                      <Upload class="w-3.5 h-3.5 text-slate-500" />
                      <span>Pilih File Gambar dari Komputer</span>
                    </button>
                    <span class="text-[10px] text-slate-400">(Maksimal 5MB, format 16:9 disarankan)</span>
                  </div>

                  <!-- Input Alternatif: URL Gambar -->
                  <input
                    v-model="form.image_url"
                    type="text"
                    placeholder="Atau tempelkan tautan/URL gambar di sini..."
                    class="w-full px-3.5 py-2 text-xs border border-slate-300 rounded-xl focus:outline-none focus:border-emerald-600 font-mono text-[11px]"
                  />
                  <span v-if="formErrors.image_url" class="text-[11px] text-rose-500 font-semibold block mt-1">
                    {{ formErrors.image_url }}
                  </span>

                  <!-- Live Preview -->
                  <div v-if="form.image_url" class="mt-2 relative aspect-16/9 rounded-xl overflow-hidden border border-slate-300 bg-slate-900 shadow-inner">
                    <img :src="form.image_url" alt="Pratinjau Banner" class="w-full h-full object-cover" />
                    <div class="absolute top-2 left-2 px-2 py-0.5 rounded text-[10px] font-bold bg-black/70 text-white">
                      Pratinjau Rasio 16:9
                    </div>
                  </div>
                </div>

                <!-- 5. Urutan & Jadwal Berlaku -->
                <div class="grid grid-cols-1 sm:grid-cols-3 gap-3">
                  <div>
                    <label class="block text-xs font-bold text-slate-700 mb-1">Urutan Tayang</label>
                    <input
                      v-model.number="form.display_order"
                      type="number"
                      min="0"
                      class="w-full px-3 py-2 text-xs border border-slate-300 rounded-xl font-mono focus:outline-none focus:border-emerald-600"
                    />
                  </div>

                  <div>
                    <label class="block text-xs font-bold text-slate-700 mb-1">Tanggal Mulai</label>
                    <input
                      v-model="form.start_date"
                      type="date"
                      class="w-full px-3 py-2 text-xs border border-slate-300 rounded-xl focus:outline-none focus:border-emerald-600"
                    />
                  </div>

                  <div>
                    <label class="block text-xs font-bold text-slate-700 mb-1">Tanggal Selesai</label>
                    <input
                      v-model="form.end_date"
                      type="date"
                      class="w-full px-3 py-2 text-xs border border-slate-300 rounded-xl focus:outline-none focus:border-emerald-600"
                    />
                    <span v-if="formErrors.end_date" class="text-[10px] text-rose-500 font-semibold block mt-0.5">
                      {{ formErrors.end_date }}
                    </span>
                  </div>
                </div>

                <!-- 6. Status Aktif Toggle -->
                <div class="flex items-center gap-2 pt-2">
                  <input
                    id="is_active"
                    v-model="form.is_active"
                    type="checkbox"
                    class="w-4 h-4 rounded text-emerald-600 focus:ring-emerald-500 border-slate-300 cursor-pointer"
                  />
                  <label for="is_active" class="text-xs font-bold text-slate-700 cursor-pointer">
                    Aktifkan penayangan banner ini di Layar Pelanggan
                  </label>
                </div>

                <!-- Modal Footer -->
                <div class="pt-4 border-t border-slate-200 flex items-center justify-end gap-2.5">
                  <button
                    type="button"
                    @click="showModal = false"
                    class="px-4 py-2 border border-slate-300 rounded-xl text-slate-700 font-bold text-xs hover:bg-slate-100 transition cursor-pointer"
                  >
                    Batal
                  </button>
                  <button
                    type="submit"
                    :disabled="isSubmitting"
                    class="px-5 py-2 bg-emerald-600 hover:bg-emerald-700 text-white rounded-xl font-bold text-xs shadow-sm transition active:scale-95 cursor-pointer disabled:opacity-50"
                  >
                    {{ isSubmitting ? 'Menyimpan...' : isEditing ? 'Simpan Perubahan' : 'Terbitkan Banner' }}
                  </button>
                </div>
              </form>
            </div>
          </div>
        </div>
      </Teleport>
    </div>
  </AdminLayout>
</template>
