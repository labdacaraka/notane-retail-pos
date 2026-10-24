<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import AdminLayout from '@/layouts/AdminLayout.vue'
import PaginationBar from '@/components/common/PaginationBar.vue'
import {
  Zap,
  Plus,
  Search,
  Calendar,
  Clock,
  Store,
  Tag,
  Trash2,
  Edit2,
  XCircle,
  HelpCircle,
  Layers,
} from 'lucide-vue-next'
import { Icon } from '@iconify/vue'
import { api } from '@/services/api'
import { useAuthStore } from '@/stores/authStore'
import { useDialog } from '@/composables/useDialog'
import { formatCurrency, formatDate } from '@/utils/formatters'
import type { FlashSale, Outlet, Product } from '@/types'

const authStore = useAuthStore()
const dialog = useDialog()

// List state
const flashSales = ref<FlashSale[]>([])
const totalItems = ref<number>(0)
const isLoading = ref<boolean>(false)
const searchQuery = ref<string>('')
const statusFilter = ref<string>('ALL') // ALL, ACTIVE, UPCOMING, EXPIRED, INACTIVE

// Pagination state
const currentPage = ref<number>(1)
const pageSize = ref<number>(10)

// Modal Create/Edit state
const showModal = ref<boolean>(false)
const isEditing = ref<boolean>(false)
const currentId = ref<string | null>(null)
const isSaving = ref<boolean>(false)

// Master data for modal
const outlets = ref<Outlet[]>([])
const isAllOutlets = ref<boolean>(true)
const selectedOutletIds = ref<string[]>([])

// Product Picker state
const showProductPicker = ref<boolean>(false)
const productSearch = ref<string>('')
const productSearchResults = ref<Product[]>([])
const isSearchingProducts = ref<boolean>(false)

interface FormItem {
  id?: string
  product_id: string
  product_name: string
  product_barcode: string
  product_image_url?: string | null
  original_price: number
  discount_type: 'PERCENTAGE' | 'FIXED_PRICE' | 'NOMINAL_CUT'
  discount_value: number
  max_qty_per_transaction?: number | null
  promo_quota?: number | null
  sold_qty: number
}

const form = ref({
  name: '',
  description: '',
  start_date: new Date().toISOString().slice(0, 10),
  end_date: new Date(Date.now() + 7 * 86400000).toISOString().slice(0, 10),
  is_all_day: true,
  start_time: '09:00',
  end_time: '21:00',
  is_active: true,
  is_stackable_with_member: false,
  items: [] as FormItem[],
})

// Fetch Flash Sales List via API on-demand
async function fetchFlashSales() {
  isLoading.value = true
  try {
    const params: Record<string, any> = {
      page: currentPage.value,
      page_size: pageSize.value,
    }
    if (statusFilter.value && statusFilter.value !== 'ALL') {
      params.status = statusFilter.value
    }
    if (searchQuery.value.trim()) {
      params.search = searchQuery.value.trim()
    }

    const res = await api.get('/flash-sales', { params })
    if (res.data?.success) {
      flashSales.value = res.data.data || []
      if (typeof res.data.total_items === 'number') {
        totalItems.value = res.data.total_items
      } else if (res.data.meta?.total_items !== undefined) {
        totalItems.value = res.data.meta.total_items
      } else {
        totalItems.value = (res.data.data || []).length
      }
    }
  } catch (err: any) {
    console.error('Error fetching flash sales:', err)
  } finally {
    isLoading.value = false
  }
}

function handleTabChange(tabId: string) {
  statusFilter.value = tabId
  currentPage.value = 1
  fetchFlashSales()
}

function onPageChange(page: number) {
  currentPage.value = page
  fetchFlashSales()
}

function onPageSizeChange(size: number) {
  pageSize.value = size
  currentPage.value = 1
  fetchFlashSales()
}

let searchTimer: any = null
function onSearchInput() {
  clearTimeout(searchTimer)
  searchTimer = setTimeout(() => {
    currentPage.value = 1
    fetchFlashSales()
  }, 350)
}

// Fetch Outlets
async function fetchOutlets() {
  try {
    const res = await api.get('/outlets')
    if (res.data.success) {
      outlets.value = res.data.data
    }
  } catch (err) {
    console.error('Error fetching outlets:', err)
  }
}

// Modal Openers
function openCreateModal() {
  isEditing.value = false
  currentId.value = null
  isAllOutlets.value = true
  selectedOutletIds.value = []
  form.value = {
    name: '',
    description: '',
    start_date: new Date().toISOString().slice(0, 10),
    end_date: new Date(Date.now() + 7 * 86400000).toISOString().slice(0, 10),
    is_all_day: true,
    start_time: '09:00',
    end_time: '21:00',
    is_active: true,
    is_stackable_with_member: false,
    items: [],
  }
  showModal.value = true
}

async function openEditModal(fs: FlashSale) {
  isEditing.value = true
  currentId.value = fs.id
  isLoading.value = true
  try {
    const res = await api.get(`/flash-sales/${fs.id}`)
    if (res.data.success) {
      const detail = res.data.data
      const fs_data = detail.flash_sale || detail  // fallback if API shape changes
      form.value = {
        name: fs_data.name,
        description: fs_data.description || '',
        start_date: fs_data.start_date,
        end_date: fs_data.end_date,
        is_all_day: fs_data.is_all_day,
        start_time: fs_data.start_time?.slice(0, 5) || '09:00',
        end_time: fs_data.end_time?.slice(0, 5) || '21:00',
        is_active: fs_data.is_active,
        is_stackable_with_member: fs_data.is_stackable_with_member,
        items: (detail.items || []).map((it: any) => ({
          id: it.id,
          product_id: it.product_id,
          product_name: it.product_name,
          product_barcode: it.product_barcode,
          product_image_url: it.product_image_url,
          original_price: Number(it.normal_price || it.original_price || 0),
          discount_type: it.discount_type,
          discount_value: Number(it.discount_value || 0),
          max_qty_per_transaction: it.max_qty_per_transaction ? Number(it.max_qty_per_transaction) : null,
          promo_quota: it.promo_quota ? Number(it.promo_quota) : null,
          sold_qty: Number(it.sold_qty) || 0,
        })),
      }

      if (detail.outlets && detail.outlets.length > 0) {
        isAllOutlets.value = false
        selectedOutletIds.value = detail.outlets.map((o: any) => o.outlet_id)
      } else {
        isAllOutlets.value = true
        selectedOutletIds.value = []
      }
      showModal.value = true
    }
  } catch (err: any) {
    dialog.alert(err.response?.data?.message || 'Gagal memuat detail program flash sale', 'danger')
  } finally {
    isLoading.value = false
  }
}

// Product search in picker
async function searchProducts() {
  isSearchingProducts.value = true
  try {
    const params: any = { limit: 20 }
    if (productSearch.value.trim()) {
      params.search = productSearch.value.trim()
    }
    const res = await api.get('/products', { params })
    if (res.data.success) {
      productSearchResults.value = res.data.data.products || res.data.data || []
    }
  } catch (err) {
    console.error('Error searching products:', err)
  } finally {
    isSearchingProducts.value = false
  }
}

function openProductPicker() {
  productSearch.value = ''
  showProductPicker.value = true
  searchProducts()
}

function selectProductForPromo(prod: Product) {
  // Check if already added
  const exists = form.value.items.some((it) => it.product_id === prod.id)
  if (exists) {
    dialog.toast(`Produk "${prod.name}" sudah ada dalam daftar promo`, 'info')
    return
  }

  const basePrice = Number(prod.base_selling_price || 0)
  form.value.items.push({
    product_id: prod.id,
    product_name: prod.name,
    product_barcode: prod.barcode,
    original_price: basePrice,
    discount_type: 'PERCENTAGE',
    discount_value: 15, // Default 15% diskon
    max_qty_per_transaction: 2, // Default max 2 pcs per nota
    promo_quota: 50, // Default kuota 50 pcs
    sold_qty: 0,
  })

  dialog.toast(`"${prod.name}" ditambahkan ke program flash sale`, 'success')
}

function removeProductFromPromo(index: number) {
  form.value.items.splice(index, 1)
}

// Calculate promo price per item
function calculatePromoPrice(item: FormItem): number {
  const orig = item.original_price || 0
  const val = Number(item.discount_value) || 0
  if (item.discount_type === 'FIXED_PRICE') {
    return Math.max(0, val)
  } else if (item.discount_type === 'PERCENTAGE') {
    const disc = (orig * val) / 100
    return Math.max(0, orig - disc)
  } else if (item.discount_type === 'NOMINAL_CUT') {
    return Math.max(0, orig - val)
  }
  return orig
}

// Save Flash Sale
async function handleSave() {
  if (!form.value.name.trim()) {
    dialog.alert('Nama Program Flash Sale wajib diisi!', 'warning')
    return
  }
  if (!form.value.start_date || !form.value.end_date) {
    dialog.alert('Periode tanggal mulai dan selesai wajib diisi!', 'warning')
    return
  }
  if (form.value.items.length === 0) {
    dialog.alert('Pilih minimal 1 barang untuk diikutsertakan dalam Flash Sale!', 'warning')
    return
  }
  if (!isAllOutlets.value && selectedOutletIds.value.length === 0) {
    dialog.alert('Pilih minimal satu outlet target atau centang "Berlaku di Semua Cabang"!', 'warning')
    return
  }

  isSaving.value = true
  try {
    const payload = {
      name: form.value.name.trim(),
      description: form.value.description.trim() || null,
      start_date: form.value.start_date,
      end_date: form.value.end_date,
      is_all_day: form.value.is_all_day,
      start_time: form.value.is_all_day ? null : form.value.start_time,
      is_active: (() => {
        const today = new Date().toISOString().slice(0, 10)
        const nowTime = new Date().toTimeString().slice(0, 8)
        if (form.value.end_date < today) return false
        if (form.value.end_date === today && !form.value.is_all_day && form.value.end_time && form.value.end_time < nowTime) return false
        return Boolean(form.value.is_active)
      })(),
      is_stackable_with_member: form.value.is_stackable_with_member,
      outlet_ids: isAllOutlets.value ? [] : selectedOutletIds.value,
      items: form.value.items.map((it) => ({
        product_id: it.product_id,
        discount_type: it.discount_type,
        discount_value: Number(it.discount_value),
        max_qty_per_transaction: it.max_qty_per_transaction ? Number(it.max_qty_per_transaction) : null,
        promo_quota: it.promo_quota ? Number(it.promo_quota) : null,
      })),
    }

    if (isEditing.value && currentId.value) {
      await api.put(`/flash-sales/${currentId.value}`, payload)
      dialog.toast('Program Flash Sale berhasil diperbarui!', 'success')
    } else {
      await api.post('/flash-sales', payload)
      dialog.toast('Program Flash Sale baru berhasil dibuat!', 'success')
    }

    showModal.value = false
    fetchFlashSales()
  } catch (err: any) {
    dialog.alert(err.response?.data?.message || 'Gagal menyimpan program Flash Sale', 'danger')
  } finally {
    isSaving.value = false
  }
}

// Delete Flash Sale
async function handleDelete(fs: FlashSale) {
  const confirmed = await dialog.confirm({
    title: 'Hapus Program Flash Sale',
    message: `Apakah Anda yakin ingin menghapus promo "${fs.name}"?`,
    subMessage: 'Promo ini akan dihapus secara permanen beserta kuota dan riwayat alokasinya.',
    confirmText: 'Ya, Hapus',
    cancelText: 'Batal',
    type: 'danger',
  })
  if (!confirmed) return

  try {
    await api.delete(`/flash-sales/${fs.id}`)
    dialog.toast(`Flash sale "${fs.name}" berhasil dihapus`, 'success')
    fetchFlashSales()
  } catch (err: any) {
    dialog.alert(err.response?.data?.message || 'Gagal menghapus flash sale', 'danger')
  }
}

function onWsMessage(e: any) {
  if (e.detail?.type === 'FLASH_SALES_EXPIRED') {
    fetchFlashSales()
  }
}

onMounted(() => {
  fetchFlashSales()
  fetchOutlets()
  window.addEventListener('notane-ws-message', onWsMessage)
})

onUnmounted(() => {
  window.removeEventListener('notane-ws-message', onWsMessage)
})
</script>

<template>
  <AdminLayout>
    <div class="p-6 space-y-5 max-w-7xl mx-auto select-none">
      <!-- 1. Header Banner -->
      <div class="bg-gradient-to-r from-amber-600 via-orange-600 to-amber-700 rounded-2xl p-6 text-white shadow-lg relative overflow-hidden flex flex-wrap items-center justify-between gap-4">
        <div class="absolute -right-10 -bottom-10 opacity-15 pointer-events-none">
          <Zap class="w-64 h-64 text-white" />
        </div>

        <div class="space-y-1.5 z-10">
          <div class="inline-flex items-center gap-2 px-3 py-1 bg-white/20 backdrop-blur-md rounded-full text-xs font-black uppercase tracking-wider text-amber-100 border border-white/20 shadow-xs">
            <Zap class="w-3.5 h-3.5 text-yellow-300 animate-bounce" />
            <span>Retail Promotional Engine</span>
          </div>
          <h1 class="text-2xl font-black tracking-tight flex items-center gap-2.5">
            <Icon icon="flat-color-icons:flash-on" class="w-7 h-7 drop-shadow-sm inline-block" />
            <span>Program Flash Sale & Diskon Khusus</span>
          </h1>
          <p class="text-xs text-amber-100 max-w-2xl leading-relaxed">
            Buat program diskon kilat berbasis periode dan jam tertentu, atur kuota barang & batasan per transaksi, serta tentukan cabang target dengan hierarki diskon standar retail modern.
          </p>
        </div>

        <div class="z-10 flex items-center gap-3">
          <button
            v-if="authStore.hasPermission(['promotions:write', 'master:write'])"
            @click="openCreateModal"
            class="bg-white hover:bg-amber-50 text-amber-900 font-extrabold text-xs px-5 py-2.5 rounded-xl shadow-md flex items-center gap-2 transition active:scale-95 cursor-pointer border border-white/60"
          >
            <Plus class="w-4 h-4 text-amber-600 font-black" />
            <span>Buat Flash Sale Baru</span>
          </button>
        </div>
      </div>

      <!-- 2. Filter & Search Controls -->
      <div class="bg-white p-4 rounded-xl border border-slate-200 shadow-xs flex flex-wrap items-center justify-between gap-4">
        <!-- Status Tabs -->
        <!-- Status Tabs with colorful icons -->
        <div class="flex items-center gap-1.5 bg-slate-100 p-1 rounded-lg border border-slate-200 text-xs font-bold text-slate-600 overflow-x-auto">
          <button
            v-for="tab in [
              { id: 'ALL', label: 'Semua Program', icon: 'fluent:apps-list-detail-24-regular' },
              { id: 'ACTIVE', label: 'Sedang Aktif', icon: 'fluent:flash-24-filled', iconClass: 'text-amber-500' },
              { id: 'UPCOMING', label: 'Akan Datang', icon: 'fluent:clock-24-regular', iconClass: 'text-blue-500' },
              { id: 'EXPIRED', label: 'Selesai', icon: 'fluent:checkmark-circle-24-regular', iconClass: 'text-slate-500' },
              { id: 'INACTIVE', label: 'Nonaktif', icon: 'fluent:dismiss-circle-24-regular', iconClass: 'text-rose-500' },
            ]"
            :key="tab.id"
            @click="handleTabChange(tab.id)"
            class="px-3 py-1.5 rounded-md transition cursor-pointer flex items-center gap-1.5 whitespace-nowrap"
            :class="statusFilter === tab.id ? 'bg-white text-amber-900 font-black shadow-xs' : 'hover:text-slate-900 text-slate-600'"
          >
            <Icon :icon="tab.icon" class="w-3.5 h-3.5" :class="tab.iconClass" />
            <span>{{ tab.label }}</span>
          </button>
        </div>

        <!-- Search Bar -->
        <div class="relative w-full sm:w-72">
          <Search class="w-4 h-4 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2" />
          <input
            v-model="searchQuery"
            @input="onSearchInput"
            type="text"
            placeholder="Cari nama program promo..."
            class="w-full pl-9 pr-4 py-2 text-xs rounded-lg border border-slate-300 focus:outline-none focus:ring-2 focus:ring-amber-500/20 focus:border-amber-500 bg-slate-50 transition"
          />
        </div>
      </div>

      <!-- 3. List of Flash Sale Cards / Table -->
      <div v-if="isLoading" class="p-12 text-center text-slate-400 flex flex-col items-center gap-3 bg-white rounded-xl border border-slate-200">
        <div class="w-8 h-8 border-3 border-amber-500 border-t-transparent rounded-full animate-spin"></div>
        <span class="text-xs font-medium">Memuat data program flash sale...</span>
      </div>

      <div v-else-if="flashSales.length === 0" class="p-12 text-center bg-white rounded-xl border border-slate-200 space-y-3">
        <div class="w-12 h-12 bg-amber-50 text-amber-500 rounded-full flex items-center justify-center mx-auto border border-amber-200">
          <Icon icon="flat-color-icons:flash-on" class="w-7 h-7" />
        </div>
        <h3 class="text-sm font-bold text-slate-700">Belum Ada Program Flash Sale</h3>
        <p class="text-xs text-slate-400 max-w-md mx-auto">
          Tidak ada program promosi flash sale yang sesuai dengan kriteria filter Anda. Klik tombol "Buat Flash Sale Baru" untuk memulai promosi kilat.
        </p>
      </div>

      <div v-else class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
        <div
          v-for="fs in flashSales"
          :key="fs.id"
          class="bg-white rounded-2xl border border-slate-200/90 shadow-xs hover:shadow-md transition flex flex-col overflow-hidden relative group"
        >
          <!-- Top Accent Status Bar -->
          <div
            class="h-1.5 w-full"
            :class="{
              'bg-emerald-500': fs.is_active && (fs.is_running || fs.status_label === 'BERJALAN' || fs.status_label === 'MENUNGGU_JAM_AKTIF' || fs.status === 'ACTIVE'),
              'bg-blue-500': fs.is_active && (fs.status_label === 'AKAN_DATANG' || fs.status === 'UPCOMING'),
              'bg-slate-400': fs.status_label === 'BERAKHIR' || fs.status === 'EXPIRED',
              'bg-rose-400': !fs.is_active,
            }"
          ></div>

          <div class="p-5 flex-1 flex flex-col justify-between space-y-4">
            <!-- Title & Status Badges -->
            <div>
              <div class="flex items-start justify-between gap-2 mb-2">
                <span
                  class="text-[10px] font-black px-2.5 py-1 rounded-full uppercase tracking-wider flex items-center gap-1 border"
                  :class="{
                    'bg-emerald-50 text-emerald-700 border-emerald-200': fs.is_active && (fs.is_running || fs.status_label === 'BERJALAN' || fs.status_label === 'MENUNGGU_JAM_AKTIF' || fs.status === 'ACTIVE'),
                    'bg-blue-50 text-blue-700 border-blue-200': fs.is_active && (fs.status_label === 'AKAN_DATANG' || fs.status === 'UPCOMING'),
                    'bg-slate-100 text-slate-600 border-slate-200': fs.status_label === 'BERAKHIR' || fs.status === 'EXPIRED',
                    'bg-rose-50 text-rose-700 border-rose-200': !fs.is_active,
                  }"
                >
                  <span class="w-1.5 h-1.5 rounded-full" :class="{
                    'bg-emerald-500 animate-pulse': fs.is_active && (fs.is_running || fs.status_label === 'BERJALAN' || fs.status_label === 'MENUNGGU_JAM_AKTIF' || fs.status === 'ACTIVE'),
                    'bg-blue-500': fs.is_active && (fs.status_label === 'AKAN_DATANG' || fs.status === 'UPCOMING'),
                    'bg-slate-400': fs.status_label === 'BERAKHIR' || fs.status === 'EXPIRED',
                    'bg-rose-500': !fs.is_active,
                  }"></span>
                  <span v-if="!fs.is_active">Nonaktif</span>
                  <span v-else-if="fs.is_running || fs.status_label === 'BERJALAN'">Sedang Aktif</span>
                  <span v-else-if="fs.status_label === 'MENUNGGU_JAM_AKTIF'">Menunggu Jam Promo</span>
                  <span v-else-if="fs.status_label === 'AKAN_DATANG' || fs.status === 'UPCOMING'">Akan Datang</span>
                  <span v-else>Telah Berakhir</span>
                </span>

                <!-- Stacking Badge -->
                <span
                  v-if="fs.is_stackable_with_member"
                  class="text-[10px] font-bold bg-purple-50 text-purple-700 border border-purple-200 px-2 py-0.5 rounded-md flex items-center gap-1"
                  title="Dapat diakumulasi dengan diskon member"
                >
                  <Layers class="w-3 h-3" />
                  <span>+ Member</span>
                </span>
                <span
                  v-else
                  class="text-[10px] font-bold bg-amber-50 text-amber-800 border border-amber-200 px-2 py-0.5 rounded-md flex items-center gap-1"
                  title="Best Value: Tidak menumpuk di atas member untuk menjaga margin HPP"
                >
                  <span>Best Deal</span>
                </span>
              </div>

              <h3 class="text-base font-extrabold text-slate-900 group-hover:text-amber-600 transition leading-snug">
                {{ fs.name }}
              </h3>
              <p v-if="fs.description" class="text-xs text-slate-500 line-clamp-2 mt-1">
                {{ fs.description }}
              </p>
            </div>

            <!-- Details: Date, Time, Outlets, Items -->
            <div class="space-y-2 bg-slate-50 p-3 rounded-xl border border-slate-100 text-xs text-slate-600">
              <div class="flex items-center gap-2">
                <Calendar class="w-3.5 h-3.5 text-slate-400 shrink-0" />
                <span class="font-medium font-mono text-[11px]">
                  {{ formatDate(fs.start_date) }} - {{ formatDate(fs.end_date) }}
                </span>
              </div>

              <div class="flex items-center gap-2">
                <Clock class="w-3.5 h-3.5 text-slate-400 shrink-0" />
                <span v-if="fs.is_all_day" class="text-[11px] text-emerald-700 font-bold">
                  Sepanjang Hari (24 Jam)
                </span>
                <span v-else class="text-[11px] font-mono font-bold text-amber-800">
                  {{ fs.start_time?.slice(0, 5) }} - {{ fs.end_time?.slice(0, 5) }} WIB
                </span>
              </div>

              <div class="flex items-center justify-between pt-1 border-t border-slate-200/60 text-[11px]">
                <div class="flex items-center gap-1.5">
                  <Store class="w-3.5 h-3.5 text-slate-400" />
                  <span v-if="!fs.total_outlets || fs.total_outlets === 0" class="text-emerald-700 font-bold">
                    Semua Cabang
                  </span>
                  <span v-else class="font-bold text-slate-700">
                    {{ fs.total_outlets }} Cabang Terpilih
                  </span>
                </div>

                <div class="flex items-center gap-1.5 font-bold text-amber-900 bg-amber-100/70 px-2 py-0.5 rounded-md">
                  <Tag class="w-3 h-3 text-amber-600" />
                  <span>{{ fs.total_items || 0 }} Barang Promo</span>
                </div>
              </div>
            </div>

            <!-- Actions Footer -->
            <div class="flex items-center justify-end gap-2 pt-2 border-t border-slate-100">
              <button
                @click="openEditModal(fs)"
                class="px-3 py-1.5 rounded-lg border border-slate-200 hover:border-amber-400 bg-white hover:bg-amber-50 text-slate-700 hover:text-amber-900 text-xs font-bold transition flex items-center gap-1.5 cursor-pointer shadow-2xs"
              >
                <Edit2 class="w-3.5 h-3.5 text-amber-600" />
                <span>Ubah Detail</span>
              </button>

              <button
                @click="handleDelete(fs)"
                class="px-2.5 py-1.5 rounded-lg border border-slate-200 hover:border-rose-400 bg-white hover:bg-rose-50 text-slate-400 hover:text-rose-600 text-xs transition cursor-pointer shadow-2xs"
                title="Hapus Program Flash Sale"
              >
                <Trash2 class="w-3.5 h-3.5" />
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- Pagination -->
      <PaginationBar
        v-if="totalItems > 0"
        :current-page="currentPage"
        :page-size="pageSize"
        :total-items="totalItems"
        @update:current-page="onPageChange"
        @update:page-size="onPageSizeChange"
      />
    </div>

    <!-- 4. MODAL TAMBAH / UBAH FLASH SALE -->
    <div
      v-if="showModal"
      class="fixed inset-0 z-50 bg-slate-900/60 backdrop-blur-xs flex items-center justify-center p-4 overflow-y-auto"
    >
      <div class="bg-white rounded-2xl shadow-2xl border border-slate-200 w-full max-w-4xl max-h-[92vh] flex flex-col overflow-hidden animate-scaleIn">
        <!-- Modal Header -->
        <div class="bg-gradient-to-r from-amber-600 to-orange-600 text-white px-6 py-4 flex items-center justify-between shrink-0">
          <div class="flex items-center gap-2.5">
            <div class="w-9 h-9 bg-white/20 rounded-xl flex items-center justify-center border border-white/20">
              <Icon icon="flat-color-icons:flash-on" class="w-6 h-6" />
            </div>
            <div>
              <h2 class="text-base font-extrabold leading-none">
                {{ isEditing ? 'Ubah Konfigurasi Flash Sale' : 'Buat Program Flash Sale Baru' }}
              </h2>
              <p class="text-xs text-amber-100 mt-1">
                Atur jadwal promo, outlet target, kuota stok diskon, dan akumulasi member.
              </p>
            </div>
          </div>

          <button
            @click="showModal = false"
            class="text-white/80 hover:text-white p-1.5 rounded-lg hover:bg-white/10 transition cursor-pointer"
          >
            <XCircle class="w-6 h-6" />
          </button>
        </div>

        <!-- Modal Body (Scrollable) -->
        <div class="p-6 overflow-y-auto space-y-6 flex-1 text-xs">
          <!-- Step 1: Info Dasar Program -->
          <div class="bg-slate-50 p-4 rounded-xl border border-slate-200 space-y-4">
            <h4 class="font-extrabold text-slate-800 text-sm flex items-center gap-2">
              <span class="w-5 h-5 bg-amber-500 text-white rounded-full flex items-center justify-center text-[10px]">1</span>
              <span>Informasi Utama Flash Sale</span>
            </h4>

            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div class="space-y-1">
                <label class="font-bold text-slate-700">Nama Flash Sale <span class="text-rose-500">*</span></label>
                <input
                  v-model="form.name"
                  type="text"
                  placeholder="Contoh: FLASH SALE PAYDAY 9.9"
                  class="w-full px-3 py-2 bg-white rounded-lg border border-slate-300 focus:ring-2 focus:ring-amber-500/20 focus:border-amber-500 font-bold"
                />
              </div>

              <div class="space-y-1">
                <label class="font-bold text-slate-700">Deskripsi / Catatan Promo</label>
                <input
                  v-model="form.description"
                  type="text"
                  placeholder="Catatan internal atau keterangan promo..."
                  class="w-full px-3 py-2 bg-white rounded-lg border border-slate-300 focus:ring-2 focus:ring-amber-500/20 focus:border-amber-500"
                />
              </div>
            </div>

            <!-- Periode & Jam -->
            <div class="grid grid-cols-1 md:grid-cols-3 gap-4 pt-2 border-t border-slate-200/70">
              <div class="space-y-1">
                <label class="font-bold text-slate-700">Tanggal Mulai <span class="text-rose-500">*</span></label>
                <input
                  v-model="form.start_date"
                  type="date"
                  class="w-full px-3 py-2 bg-white rounded-lg border border-slate-300 focus:ring-2 focus:ring-amber-500/20 focus:border-amber-500 font-mono"
                />
              </div>

              <div class="space-y-1">
                <label class="font-bold text-slate-700">Tanggal Selesai <span class="text-rose-500">*</span></label>
                <input
                  v-model="form.end_date"
                  type="date"
                  class="w-full px-3 py-2 bg-white rounded-lg border border-slate-300 focus:ring-2 focus:ring-amber-500/20 focus:border-amber-500 font-mono"
                />
              </div>

              <div class="space-y-1">
                <label class="font-bold text-slate-700">Waktu Berlaku</label>
                <div class="flex items-center gap-3 pt-2">
                  <label class="flex items-center gap-2 cursor-pointer font-bold text-slate-700">
                    <input
                      v-model="form.is_all_day"
                      type="checkbox"
                      class="rounded text-amber-600 focus:ring-amber-500"
                    />
                    <span>24 Jam Sepanjang Hari</span>
                  </label>
                </div>
              </div>
            </div>

            <!-- Time Window jika bukan 24 jam -->
            <div v-if="!form.is_all_day" class="grid grid-cols-2 gap-4 bg-amber-50/60 p-3 rounded-lg border border-amber-200/70">
              <div class="space-y-1">
                <label class="font-bold text-amber-900">Jam Mulai (WIB)</label>
                <input
                  v-model="form.start_time"
                  type="time"
                  class="w-full px-3 py-1.5 bg-white rounded-lg border border-amber-300 font-mono font-bold"
                />
              </div>
              <div class="space-y-1">
                <label class="font-bold text-amber-900">Jam Selesai (WIB)</label>
                <input
                  v-model="form.end_time"
                  type="time"
                  class="w-full px-3 py-1.5 bg-white rounded-lg border border-amber-300 font-mono font-bold"
                />
              </div>
            </div>

            <!-- Akumulasi Diskon Member Policy (Standar Retail Modern) -->
            <div class="bg-indigo-50/70 p-3.5 rounded-xl border border-indigo-200/80 space-y-2">
              <div class="flex items-center justify-between">
                <div class="flex items-center gap-2">
                  <Icon icon="fluent-color:lock-shield-24" class="w-5 h-5" />
                  <span class="font-extrabold text-indigo-950 text-xs">Aturan Akumulasi Diskon Member (Standard Retail Policy)</span>
                </div>
                <label class="relative inline-flex items-center cursor-pointer">
                  <input
                    v-model="form.is_stackable_with_member"
                    type="checkbox"
                    class="sr-only peer"
                  />
                  <div class="w-9 h-5 bg-slate-300 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-indigo-600"></div>
                </label>
              </div>

              <p v-if="!form.is_stackable_with_member" class="text-[11px] text-indigo-900 leading-relaxed">
                <strong>Mode Best Value (Direkomendasikan Standar Industri):</strong> Diskon Flash Sale <em>tidak dapat ditumpuk</em> dengan diskon member biasa. Sistem secara otomatis menerapkan harga terbaik yang paling menguntungkan pelanggan untuk melindungi margin HPP toko.
              </p>
              <p v-else class="text-[11px] text-purple-900 leading-relaxed font-bold">
                <strong>Mode Akumulasi Bertingkat Aktif:</strong> Pelanggan member akan menerima potongan diskon member tambahan di atas harga promo Flash Sale.
              </p>
            </div>
          </div>

          <!-- Step 2: Outlet Target -->
          <div class="bg-slate-50 p-4 rounded-xl border border-slate-200 space-y-3">
            <div class="flex items-center justify-between">
              <h4 class="font-extrabold text-slate-800 text-sm flex items-center gap-2">
                <span class="w-5 h-5 bg-amber-500 text-white rounded-full flex items-center justify-center text-[10px]">2</span>
                <span>Target Gerai / Cabang Outlet</span>
              </h4>

              <label class="flex items-center gap-2 cursor-pointer font-extrabold text-amber-900 bg-amber-100/80 px-3 py-1 rounded-lg border border-amber-300">
                <input
                  v-model="isAllOutlets"
                  type="checkbox"
                  class="rounded text-amber-600 focus:ring-amber-500"
                />
                <span>Berlaku di Semua Cabang</span>
              </label>
            </div>

            <div v-if="!isAllOutlets" class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 gap-2 pt-2">
              <label
                v-for="out in outlets"
                :key="out.id"
                class="flex items-center gap-2 p-2.5 rounded-lg border cursor-pointer transition"
                :class="selectedOutletIds.includes(out.id) ? 'bg-amber-50 border-amber-400 text-amber-950 font-bold' : 'bg-white border-slate-200 text-slate-700'"
              >
                <input
                  type="checkbox"
                  :value="out.id"
                  v-model="selectedOutletIds"
                  class="rounded text-amber-600 focus:ring-amber-500"
                />
                <span class="truncate">{{ out.name }}</span>
              </label>
            </div>
          </div>

          <!-- Step 3: Daftar Produk Promo Flash Sale -->
          <div class="bg-slate-50 p-4 rounded-xl border border-slate-200 space-y-4">
            <div class="flex items-center justify-between">
              <div>
                <h4 class="font-extrabold text-slate-800 text-sm flex items-center gap-2">
                  <span class="w-5 h-5 bg-amber-500 text-white rounded-full flex items-center justify-center text-[10px]">3</span>
                  <span>Daftar Barang Promo Flash Sale ({{ form.items.length }})</span>
                </h4>
                <p class="text-[11px] text-slate-500 mt-0.5">
                  Tentukan tipe potongan (Harga Pas, Diskon %, atau Potongan Rp), batas per transaksi, dan kuota.
                </p>
              </div>

              <button
                type="button"
                @click="openProductPicker()"
                class="bg-amber-600 hover:bg-amber-700 text-white font-bold text-xs px-3.5 py-1.5 rounded-lg shadow-sm flex items-center gap-1.5 transition active:scale-95 cursor-pointer"
              >
                <Plus class="w-4 h-4 font-black" />
                <span>Pilih Barang Dari Master</span>
              </button>
            </div>

            <!-- Items Table -->
            <div v-if="form.items.length === 0" class="p-8 text-center bg-white rounded-xl border border-dashed border-slate-300 space-y-2">
              <Tag class="w-8 h-8 text-slate-300 mx-auto" />
              <div class="font-bold text-slate-600">Belum Ada Barang yang Dipilih</div>
              <p class="text-[11px] text-slate-400">
                Klik tombol "Pilih Barang Dari Master" di atas untuk menambahkan produk ke program flash sale ini.
              </p>
            </div>

            <div v-else class="space-y-3">
              <div
                v-for="(item, idx) in form.items"
                :key="item.product_id"
                class="bg-white p-3.5 rounded-xl border border-slate-200 shadow-2xs space-y-3"
              >
                <div class="flex flex-wrap items-center justify-between gap-2 border-b border-slate-100 pb-2">
                  <div class="flex items-center gap-2.5">
                    <span class="w-5 h-5 bg-slate-200 text-slate-700 font-bold rounded-md flex items-center justify-center text-[10px]">
                      {{ idx + 1 }}
                    </span>
                    <div>
                      <div class="font-extrabold text-slate-900 text-xs flex items-center gap-2">
                        <span>{{ item.product_name }}</span>
                        <span class="font-mono text-[10px] text-slate-400 bg-slate-100 px-1.5 py-0.5 rounded">
                          {{ item.product_barcode }}
                        </span>
                      </div>
                      <div class="text-[11px] text-slate-500">
                        Harga Normal: <strong class="text-slate-700">{{ formatCurrency(item.original_price) }}</strong>
                      </div>
                    </div>
                  </div>

                  <div class="flex items-center gap-3">
                    <!-- Real-time Resulting Promo Price -->
                    <div class="text-right">
                      <div class="text-[10px] text-slate-400 uppercase font-bold">Harga Flash Sale</div>
                      <div class="font-black text-amber-700 text-sm font-mono">
                        {{ formatCurrency(calculatePromoPrice(item)) }}
                      </div>
                    </div>

                    <button
                      type="button"
                      @click="removeProductFromPromo(idx)"
                      class="text-slate-400 hover:text-rose-600 p-1.5 rounded-lg hover:bg-rose-50 transition cursor-pointer"
                      title="Hapus barang dari promo"
                    >
                      <Trash2 class="w-4 h-4" />
                    </button>
                  </div>
                </div>

                <!-- Input Controls Per Item -->
                <div class="grid grid-cols-1 sm:grid-cols-4 gap-3 text-xs">
                  <!-- Discount Type -->
                  <div class="space-y-1">
                    <label class="font-bold text-slate-600">Tipe Potongan</label>
                    <select
                      v-model="item.discount_type"
                      class="w-full px-2.5 py-1.5 bg-slate-50 rounded-lg border border-slate-300 font-bold"
                    >
                      <option value="PERCENTAGE">Diskon Persen (%)</option>
                      <option value="FIXED_PRICE">Harga Pas (Special Price)</option>
                      <option value="NOMINAL_CUT">Potongan Nominal (Rp)</option>
                    </select>
                  </div>

                  <!-- Discount Value -->
                  <div class="space-y-1">
                    <label class="font-bold text-slate-600">
                      {{ item.discount_type === 'PERCENTAGE' ? 'Nilai Diskon (%)' : item.discount_type === 'FIXED_PRICE' ? 'Harga Pas (Rp)' : 'Potongan (Rp)' }}
                    </label>
                    <input
                      v-model.number="item.discount_value"
                      type="number"
                      min="0"
                      class="w-full px-2.5 py-1.5 bg-slate-50 rounded-lg border border-slate-300 font-mono font-bold"
                    />
                  </div>

                  <!-- Max Qty Per Transaction -->
                  <div class="space-y-1">
                    <label class="font-bold text-slate-600" title="Batas maksimal barang yang boleh dibeli per 1 nota transaksi kasir">
                      Maks / Nota (Pcs)
                    </label>
                    <input
                      v-model.number="item.max_qty_per_transaction"
                      type="number"
                      min="1"
                      placeholder="Tanpa Batas"
                      class="w-full px-2.5 py-1.5 bg-slate-50 rounded-lg border border-slate-300 font-mono"
                    />
                  </div>

                  <!-- Promo Quota -->
                  <div class="space-y-1">
                    <label class="font-bold text-slate-600" title="Alokasi kuota total barang promo flash sale">
                      Total Kuota (Pcs)
                    </label>
                    <div class="flex items-center gap-1.5">
                      <input
                        v-model.number="item.promo_quota"
                        type="number"
                        min="1"
                        placeholder="Unlimited"
                        class="w-full px-2.5 py-1.5 bg-slate-50 rounded-lg border border-slate-300 font-mono"
                      />
                      <span v-if="isEditing && item.sold_qty > 0" class="text-[10px] text-slate-500 whitespace-nowrap">
                        Terjual: <strong>{{ item.sold_qty }}</strong>
                      </span>
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- Modal Footer -->
        <div class="bg-slate-100 px-6 py-3.5 border-t border-slate-200 flex items-center justify-between shrink-0">
          <div class="text-[11px] text-slate-500 flex items-center gap-1.5">
            <HelpCircle class="w-4 h-4 text-slate-400" />
            <span>Perubahan program Flash Sale langsung aktif pada kasir POS secara real-time.</span>
          </div>

          <div class="flex items-center gap-3">
            <button
              type="button"
              @click="showModal = false"
              class="px-4 py-2 rounded-lg border border-slate-300 hover:bg-slate-200 text-slate-700 font-bold transition cursor-pointer"
            >
              Batal
            </button>
            <button
              type="button"
              @click="handleSave"
              :disabled="isSaving"
              class="bg-amber-600 hover:bg-amber-700 disabled:opacity-50 text-white font-extrabold px-6 py-2 rounded-lg shadow-sm flex items-center gap-2 transition active:scale-95 cursor-pointer"
            >
              <span v-if="isSaving">Menyimpan...</span>
              <span v-else>{{ isEditing ? 'Simpan Perubahan' : 'Terbitkan Flash Sale' }}</span>
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- 5. PRODUCT PICKER DIALOG -->
    <div
      v-if="showProductPicker"
      class="fixed inset-0 z-[80] bg-slate-900/60 backdrop-blur-xs flex items-center justify-center p-4"
    >
      <div class="bg-white rounded-2xl shadow-2xl border border-slate-200 w-full max-w-2xl max-h-[80vh] flex flex-col overflow-hidden animate-scaleIn">
        <div class="bg-slate-900 text-white px-5 py-3.5 flex items-center justify-between shrink-0">
          <div class="flex items-center gap-2">
            <Icon icon="flat-color-icons:package" class="w-5 h-5" />
            <span class="font-bold text-sm">Pilih Barang dari Master Produk</span>
          </div>
          <button @click="showProductPicker = false" class="text-slate-400 hover:text-white cursor-pointer">
            <XCircle class="w-5 h-5" />
          </button>
        </div>

        <div class="p-4 border-b border-slate-200 bg-slate-50 shrink-0">
          <div class="relative">
            <Search class="w-4 h-4 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2" />
            <input
              v-model="productSearch"
              @input="searchProducts"
              type="text"
              placeholder="Ketik nama produk atau scan barcode..."
              class="w-full pl-9 pr-4 py-2 text-xs rounded-lg border border-slate-300 focus:outline-none focus:ring-2 focus:ring-amber-500/20 focus:border-amber-500 bg-white"
              autofocus
            />
          </div>
        </div>

        <div class="p-4 overflow-y-auto flex-1 space-y-2 text-xs">
          <div v-if="isSearchingProducts" class="py-8 text-center text-slate-400">
            Mencari produk...
          </div>
          <div v-else-if="productSearchResults.length === 0" class="py-8 text-center text-slate-400">
            Ketik kata kunci untuk mencari produk master.
          </div>
          <div
            v-else
            v-for="prod in productSearchResults"
            :key="prod.id"
            class="flex items-center justify-between p-3 rounded-xl border border-slate-200 hover:border-amber-400 hover:bg-amber-50/50 transition cursor-pointer"
            @click="selectProductForPromo(prod)"
          >
            <div>
              <div class="font-extrabold text-slate-900 text-xs">{{ prod.name }}</div>
              <div class="text-[11px] text-slate-500 font-mono flex items-center gap-2 mt-0.5">
                <span>SKU: {{ prod.sku || '-' }}</span>
                <span>•</span>
                <span>Barcode: {{ prod.barcode }}</span>
              </div>
            </div>

            <div class="flex items-center gap-3">
              <div class="font-bold text-slate-800 font-mono">
                {{ formatCurrency(prod.base_selling_price) }}
              </div>
              <button
                type="button"
                class="bg-amber-600 hover:bg-amber-700 text-white font-bold px-3 py-1 rounded-md text-[11px] shadow-2xs"
              >
                Pilih
              </button>
            </div>
          </div>
        </div>

        <div class="bg-slate-100 px-5 py-3 border-t border-slate-200 flex justify-end shrink-0">
          <button
            type="button"
            @click="showProductPicker = false"
            class="bg-slate-800 hover:bg-slate-900 text-white font-bold text-xs px-4 py-1.5 rounded-lg cursor-pointer"
          >
            Selesai Memilih
          </button>
        </div>
      </div>
    </div>
  </AdminLayout>
</template>
