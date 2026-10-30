<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue'
import AdminLayout from '@/layouts/AdminLayout.vue'
import PaginationBar from '@/components/common/PaginationBar.vue'
import SearchableSelect from '@/components/common/SearchableSelect.vue'
import { Icon } from '@iconify/vue'
import { api } from '@/services/api'
import type { Promotion, Product } from '@/types'
import {
  Tag,
  Plus,
  Search,
  Trash2,
  Edit2,
  X,
  Calendar,
  Gift,
  Loader2,
  CheckCircle2,
  XCircle,
  LayoutList,
  LayoutGrid,
  Sparkles,
} from 'lucide-vue-next'
import { useAuthStore } from '@/stores/authStore'
import { useDialog } from '@/composables/useDialog'
import { formatCurrency, formatQty, formatPercent, parseDecimal } from '@/utils/formatters'

const authStore = useAuthStore()
const dialog = useDialog()

// Data State
const promotions = ref<Promotion[]>([])
const products = ref<Product[]>([])
const isLoading = ref<boolean>(false)

// Filter & View State
const searchQuery = ref<string>('')
const selectedType = ref<string>('')
const selectedStatus = ref<string>('ALL') // ALL, ACTIVE, INACTIVE
const viewMode = ref<'table' | 'grid'>('table')

// Pagination State
const currentPage = ref<number>(1)
const pageSize = ref<number>(10)

// Modal Create/Edit State
const showModal = ref<boolean>(false)
const isEditing = ref<boolean>(false)
const isSubmitting = ref<boolean>(false)

const form = ref({
  id: '',
  code: '',
  name: '',
  promo_type: 'BUY_X_GET_Y' as const,
  start_date: new Date().toISOString().substring(0, 10),
  end_date: new Date(Date.now() + 30 * 86400000).toISOString().substring(0, 10),
  is_active: true,
  description: '',
  rules: [
    {
      rule_type: 'BUY_X_GET_Y',
      buy_product_id: '',
      buy_product_name: '',
      buy_quantity: 2,
      reward_product_id: '',
      reward_product_name: '',
      reward_quantity: 1,
      reward_discount_percent: 100,
      reward_price: 0,
      min_transaction_amount: 0,
      max_usage_per_transaction: 5,
    },
  ],
})

async function fetchPromotions() {
  isLoading.value = true
  try {
    const res = await api.get('/promotions')
    if (res.data?.success && Array.isArray(res.data.data)) {
      promotions.value = res.data.data
    }
  } catch (err: any) {
    dialog.toast('Gagal memuat daftar promosi: ' + (err.response?.data?.message || err.message), 'danger')
  } finally {
    isLoading.value = false
  }
}

async function fetchProducts() {
  try {
    const res = await api.get('/products', { params: { limit: 500 } })
    if (res.data?.success && Array.isArray(res.data.data)) {
      products.value = res.data.data
    }
  } catch {
    // fallback
  }
}

function openCreateModal() {
  isEditing.value = false
  form.value = {
    id: '',
    code: '',
    name: '',
    promo_type: 'BUY_X_GET_Y',
    start_date: new Date().toISOString().substring(0, 10),
    end_date: new Date(Date.now() + 30 * 86400000).toISOString().substring(0, 10),
    is_active: true,
    description: '',
    rules: [
      {
        rule_type: 'BUY_X_GET_Y',
        buy_product_id: '',
        buy_product_name: '',
        buy_quantity: 2,
        reward_product_id: '',
        reward_product_name: '',
        reward_quantity: 1,
        reward_discount_percent: 100,
        reward_price: 0,
        min_transaction_amount: 0,
        max_usage_per_transaction: 5,
      },
    ],
  }
  showModal.value = true
}

function openEditModal(promo: Promotion) {
  isEditing.value = true
  form.value = {
    id: promo.id,
    code: promo.code || '',
    name: promo.name,
    promo_type: promo.promo_type as any,
    start_date: promo.start_date ? promo.start_date.substring(0, 10) : new Date().toISOString().substring(0, 10),
    end_date: promo.end_date ? promo.end_date.substring(0, 10) : new Date(Date.now() + 30 * 86400000).toISOString().substring(0, 10),
    is_active: promo.is_active,
    description: promo.description || '',
    rules: promo.rules && promo.rules.length > 0
      ? promo.rules.map((r) => ({
          rule_type: r.rule_type || promo.promo_type,
          buy_product_id: r.buy_product_id || '',
          buy_product_name: r.buy_product_name || '',
          buy_quantity: parseDecimal(r.buy_quantity) || 1,
          reward_product_id: r.reward_product_id || '',
          reward_product_name: r.reward_product_name || '',
          reward_quantity: parseDecimal(r.reward_quantity) || 1,
          reward_discount_percent: parseDecimal(r.reward_discount_percent) || 0,
          reward_price: parseDecimal(r.reward_price) || 0,
          min_transaction_amount: parseDecimal(r.min_transaction_amount) || 0,
          max_usage_per_transaction: parseDecimal(r.max_usage_per_transaction) || 1,
        }))
      : [
          {
            rule_type: promo.promo_type,
            buy_product_id: '',
            buy_product_name: '',
            buy_quantity: 1,
            reward_product_id: '',
            reward_product_name: '',
            reward_quantity: 1,
            reward_discount_percent: 0,
            reward_price: 0,
            min_transaction_amount: 0,
            max_usage_per_transaction: 1,
          },
        ],
  }
  showModal.value = true
}

function addRule() {
  form.value.rules.push({
    rule_type: form.value.promo_type,
    buy_product_id: '',
    buy_product_name: '',
    buy_quantity: 1,
    reward_product_id: '',
    reward_product_name: '',
    reward_quantity: 1,
    reward_discount_percent: 0,
    reward_price: 0,
    min_transaction_amount: 0,
    max_usage_per_transaction: 1,
  })
}

function removeRule(index: number) {
  if (form.value.rules.length > 1) {
    form.value.rules.splice(index, 1)
  }
}

async function handleSavePromo() {
  if (!form.value.name.trim()) {
    dialog.toast('Nama promosi wajib diisi', 'warning')
    return
  }

  isSubmitting.value = true
  try {
    const startDate = form.value.start_date
      ? (form.value.start_date.includes('T') ? form.value.start_date : `${form.value.start_date}T00:00:00Z`)
      : new Date().toISOString()
    const endDate = form.value.end_date
      ? (form.value.end_date.includes('T') ? form.value.end_date : `${form.value.end_date}T23:59:59Z`)
      : new Date(Date.now() + 30 * 24 * 3600 * 1000).toISOString()

    const cleanRules = form.value.rules.map((r) => {
      const rewardPriceNum = r.reward_price !== null && r.reward_price !== undefined && (r.reward_price as any) !== ''
        ? Number(r.reward_price)
        : null
      const minTxNum = r.min_transaction_amount !== null && r.min_transaction_amount !== undefined && (r.min_transaction_amount as any) !== ''
        ? Number(r.min_transaction_amount)
        : null

      return {
        rule_type: form.value.promo_type,
        buy_product_id: r.buy_product_id || null,
        buy_quantity: Number(r.buy_quantity) || 1,
        reward_product_id: r.reward_product_id || null,
        reward_quantity: Number(r.reward_quantity) || 1,
        reward_discount_percent: Number(r.reward_discount_percent) || 0,
        reward_price: rewardPriceNum,
        reward_special_price: rewardPriceNum,
        bundle_discount_amount: rewardPriceNum,
        min_transaction_amount: minTxNum,
      }
    })

    const payload: any = {
      name: form.value.name.trim(),
      promo_type: form.value.promo_type,
      start_date: startDate,
      end_date: endDate,
      is_active: new Date(form.value.end_date) <= new Date() ? false : Boolean(form.value.is_active),
      description: form.value.description?.trim() || null,
      rules: cleanRules,
    }

    if (form.value.code && form.value.code.trim()) {
      payload.code = form.value.code.trim()
    }

    if (isEditing.value && form.value.id) {
      await api.put(`/promotions/${form.value.id}`, payload)
      dialog.toast('Program promosi berhasil diperbarui', 'success')
    } else {
      await api.post('/promotions', payload)
      dialog.toast('Program promosi baru berhasil dibuat', 'success')
    }

    showModal.value = false
    await fetchPromotions()
  } catch (err: any) {
    dialog.toast('Gagal menyimpan promosi: ' + (err.response?.data?.message || err.message), 'danger')
  } finally {
    isSubmitting.value = false
  }
}

async function handleToggleStatus(promo: Promotion) {
  const newStatus = !promo.is_active
  try {
    await api.put(`/promotions/${promo.id}`, {
      ...promo,
      is_active: newStatus,
    })
    promo.is_active = newStatus
    dialog.toast(`Promosi "${promo.name}" kini ${newStatus ? 'diaktifkan' : 'dinonaktifkan'}`, 'info')
  } catch (err: any) {
    dialog.toast('Gagal mengubah status promosi: ' + (err.response?.data?.message || err.message), 'danger')
  }
}

async function handleDelete(promo: Promotion) {
  const confirmed = await dialog.confirm({
    title: 'Hapus Program Promosi',
    message: `Apakah Anda yakin ingin menghapus program promosi "${promo.name}"?`,
    subMessage: 'Program promosi yang dihapus tidak akan berlaku lagi pada transaksi kasir.',
    confirmText: 'Ya, Hapus Promosi',
    cancelText: 'Batal',
    type: 'danger',
  })
  if (!confirmed) return

  try {
    await api.delete(`/promotions/${promo.id}`)
    dialog.toast('Promosi berhasil dihapus', 'info')
    await fetchPromotions()
  } catch (err: any) {
    dialog.toast('Gagal menghapus promosi: ' + (err.response?.data?.message || err.message), 'danger')
  }
}

const formatRupiah = (num: any) => formatCurrency(num, true)

function getPromoTypeLabel(type: string): string {
  switch (type) {
    case 'BUY_X_GET_Y':
      return 'Beli X Gratis Y / Diskon'
    case 'MIN_PURCHASE_DISCOUNT':
      return 'Diskon Min. Belanja'
    case 'BUNDLE_PRICE':
      return 'Paket Bundling'
    case 'PURCHASE_WITH_PURCHASE':
      return 'Tebus Murah (PWP)'
    default:
      return type
  }
}

function getPromoTypeBadgeClass(type: string): string {
  switch (type) {
    case 'BUY_X_GET_Y':
      return 'bg-rose-100 text-rose-800 border-rose-300'
    case 'MIN_PURCHASE_DISCOUNT':
      return 'bg-amber-100 text-amber-800 border-amber-300'
    case 'BUNDLE_PRICE':
      return 'bg-indigo-100 text-indigo-800 border-indigo-300'
    case 'PURCHASE_WITH_PURCHASE':
      return 'bg-emerald-100 text-emerald-800 border-emerald-300'
    default:
      return 'bg-slate-100 text-slate-800 border-slate-300'
  }
}

function getRuleSummaryText(promo: Promotion): string {
  if (!promo.rules || promo.rules.length === 0) return 'Tidak ada aturan terkonfigurasi'
  const first = promo.rules[0]
  if (promo.promo_type === 'BUY_X_GET_Y') {
    const buyProd = first.buy_product_name || (first.buy_product_id ? 'Item Terpilih' : 'Item Bebas')
    const disc = Number(first.reward_discount_percent) >= 100 ? 'Gratis' : `Diskon ${formatPercent(first.reward_discount_percent)}`
    return `Beli ${formatQty(first.buy_quantity)}x [${buyProd}], dapat ${formatQty(first.reward_quantity)}x (${disc})`
  }
  if (promo.promo_type === 'MIN_PURCHASE_DISCOUNT') {
    const minSpend = formatRupiah(first.min_transaction_amount || 0)
    const reward = Number(first.reward_discount_percent) > 0 ? formatPercent(first.reward_discount_percent) : formatRupiah(first.reward_price || 0)
    return `Min. belanja ${minSpend} dapat diskon ${reward}`
  }
  if (promo.promo_type === 'PURCHASE_WITH_PURCHASE') {
    const minSpend = formatRupiah(first.min_transaction_amount || 0)
    const tebusPrice = formatRupiah(first.reward_price || 0)
    return `Min. belanja ${minSpend}, tebus murah seharga ${tebusPrice}`
  }
  if (promo.promo_type === 'BUNDLE_PRICE') {
    return `Paket bundling ${formatQty(first.buy_quantity)} item dengan diskon ${formatPercent(first.reward_discount_percent)}`
  }
  return 'Aturan kustom'
}

const filteredPromotions = computed(() => {
  return promotions.value.filter((p) => {
    const matchQuery =
      !searchQuery.value ||
      p.name.toLowerCase().includes(searchQuery.value.toLowerCase()) ||
      (p.description && p.description.toLowerCase().includes(searchQuery.value.toLowerCase()))

    const matchType = !selectedType.value || p.promo_type === selectedType.value

    let matchStatus = true
    if (selectedStatus.value === 'ACTIVE') matchStatus = p.is_active === true
    else if (selectedStatus.value === 'INACTIVE') matchStatus = p.is_active === false

    return matchQuery && matchType && matchStatus
  })
})

const paginatedPromotions = computed(() => {
  const start = (currentPage.value - 1) * pageSize.value
  return filteredPromotions.value.slice(start, start + pageSize.value)
})

const activePromosCount = computed(() => promotions.value.filter((p) => p.is_active).length)
const inactivePromosCount = computed(() => promotions.value.filter((p) => !p.is_active).length)

function onWsMessage(e: any) {
  if (e.detail?.type === 'PROMOTIONS_EXPIRED') {
    fetchPromotions()
  }
}

onMounted(() => {
  fetchPromotions()
  fetchProducts()
  window.addEventListener('notane-ws-message', onWsMessage)
})

onUnmounted(() => {
  window.removeEventListener('notane-ws-message', onWsMessage)
})
</script>

<template>
  <AdminLayout>
    <div class="p-6 space-y-4">
      <!-- 1. Header Page Standar Back-Office -->
      <div class="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h2 class="text-xl font-bold text-slate-900 flex items-center gap-2.5">
            <Icon icon="streamline-ultimate-color:discount" class="w-6 h-6 drop-shadow-xs" />
            <span>Program Promosi Toko (Beli X Gratis Y, Bundling & Tebus Murah)</span>
          </h2>
          <p class="text-xs text-slate-500">
            Kelola promo Beli X Gratis Y, diskon minimum belanja, paket bundling hemat, dan tebus murah di kasir.
          </p>
        </div>

        <button
          v-if="authStore.hasPermission(['promotions:write', 'master:write'])"
          @click="openCreateModal"
          class="bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-xs px-4 py-2 rounded-lg shadow-sm flex items-center gap-2 transition active:scale-95 border border-emerald-700 cursor-pointer"
        >
          <Plus class="w-4 h-4" />
          <span>Tambah Promosi Baru</span>
        </button>
      </div>

      <!-- 2. KPI Summary Cards Standar Desktop -->
      <div class="grid grid-cols-1 sm:grid-cols-3 gap-3">
        <div class="bg-white border border-slate-300 p-3.5 rounded-xl flex items-center gap-3 shadow-xs">
          <div class="w-10 h-10 rounded-lg bg-amber-100 text-amber-800 flex items-center justify-center font-bold">
            <Tag class="w-5 h-5" />
          </div>
          <div>
            <div class="text-[11px] text-slate-500 uppercase tracking-wider font-bold">Total Program Promo</div>
            <div class="text-lg font-black text-slate-900">{{ promotions.length }} Program</div>
          </div>
        </div>

        <div class="bg-white border border-slate-300 p-3.5 rounded-xl flex items-center gap-3 shadow-xs">
          <div class="w-10 h-10 rounded-lg bg-emerald-100 text-emerald-800 flex items-center justify-center font-bold">
            <CheckCircle2 class="w-5 h-5" />
          </div>
          <div>
            <div class="text-[11px] text-slate-500 uppercase tracking-wider font-bold">Promosi Berjalan (Aktif)</div>
            <div class="text-lg font-black text-emerald-700">{{ activePromosCount }} Program Aktif</div>
          </div>
        </div>

        <div class="bg-white border border-slate-300 p-3.5 rounded-xl flex items-center gap-3 shadow-xs">
          <div class="w-10 h-10 rounded-lg bg-slate-100 text-slate-600 flex items-center justify-center font-bold">
            <XCircle class="w-5 h-5" />
          </div>
          <div>
            <div class="text-[11px] text-slate-500 uppercase tracking-wider font-bold">Non-Aktif / Draft</div>
            <div class="text-lg font-black text-slate-600">{{ inactivePromosCount }} Program</div>
          </div>
        </div>
      </div>

      <!-- 3. Filter & Search Toolbar Bar -->
      <div class="flex flex-wrap items-center gap-3 bg-white border border-slate-300 p-2.5 rounded-xl shadow-xs">
        <div class="relative flex-1 min-w-[240px]">
          <Search class="w-4 h-4 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2" />
          <input
            v-model="searchQuery"
            type="text"
            placeholder="Cari promo berdasarkan nama atau keterangan..."
            class="w-full bg-slate-50 border border-slate-300 rounded-lg pl-9 pr-4 py-1.5 text-xs text-slate-900 placeholder-slate-400 focus:outline-none focus:bg-white focus:border-emerald-600 font-medium"
          />
        </div>

        <div class="flex items-center gap-2">
          <span class="text-xs text-slate-600 font-semibold">Jenis:</span>
          <select
            v-model="selectedType"
            class="bg-slate-50 border border-slate-300 rounded-lg px-3 py-1.5 text-xs text-slate-900 focus:outline-none focus:bg-white focus:border-emerald-600 font-medium"
          >
            <option value="">Semua Jenis Promo</option>
            <option value="BUY_X_GET_Y">Beli X Gratis Y</option>
            <option value="MIN_PURCHASE_DISCOUNT">Diskon Min. Belanja</option>
            <option value="BUNDLE_PRICE">Paket Bundling</option>
            <option value="PURCHASE_WITH_PURCHASE">Tebus Murah (PWP)</option>
          </select>
        </div>

        <div class="flex items-center gap-2">
          <span class="text-xs text-slate-600 font-semibold">Status:</span>
          <select
            v-model="selectedStatus"
            class="bg-slate-50 border border-slate-300 rounded-lg px-3 py-1.5 text-xs text-slate-900 focus:outline-none focus:bg-white focus:border-emerald-600 font-medium"
          >
            <option value="ALL">Semua Status</option>
            <option value="ACTIVE">Aktif Saja</option>
            <option value="INACTIVE">Non-Aktif Saja</option>
          </select>
        </div>

        <!-- Mode Switcher: Tabel vs Grid -->
        <div class="flex items-center border border-slate-300 rounded-lg overflow-hidden bg-slate-100 p-0.5">
          <button
            type="button"
            @click="viewMode = 'table'"
            class="p-1.5 rounded-md transition cursor-pointer"
            :class="viewMode === 'table' ? 'bg-white shadow-xs text-emerald-800' : 'text-slate-500 hover:text-slate-900'"
            title="Tampilan Tabel Standar Back-Office"
          >
            <LayoutList class="w-4 h-4" />
          </button>
          <button
            type="button"
            @click="viewMode = 'grid'"
            class="p-1.5 rounded-md transition cursor-pointer"
            :class="viewMode === 'grid' ? 'bg-white shadow-xs text-emerald-800' : 'text-slate-500 hover:text-slate-900'"
            title="Tampilan Kartu Visual"
          >
            <LayoutGrid class="w-4 h-4" />
          </button>
        </div>
      </div>

      <!-- 4. Loading State -->
      <div v-if="isLoading" class="p-12 text-center text-slate-500 bg-white border border-slate-300 rounded-xl shadow-xs">
        <Loader2 class="animate-spin w-8 h-8 mx-auto mb-2 text-emerald-600" />
        <span class="text-xs font-medium">Memuat data promosi retail...</span>
      </div>

      <!-- 5. Empty State -->
      <div
        v-else-if="filteredPromotions.length === 0"
        class="bg-white rounded-xl border border-slate-300 p-12 text-center text-slate-500 shadow-xs space-y-3"
      >
        <div class="w-14 h-14 bg-amber-50 text-amber-600 rounded-2xl flex items-center justify-center mx-auto border border-amber-200">
          <Icon icon="streamline-ultimate-color:discount" class="w-8 h-8" />
        </div>
        <h3 class="text-base font-bold text-slate-800">Belum Ada Program Promosi</h3>
        <p class="text-xs text-slate-400 max-w-md mx-auto">
          Buat promo Beli X Gratis Y, diskon belanja grosir, atau promo tebus murah untuk menarik transaksi di kasir.
        </p>
        <div v-if="authStore.hasPermission(['promotions:write', 'master:write'])" class="pt-2">
          <button
            @click="openCreateModal"
            class="px-4 py-2 bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-xs rounded-lg inline-flex items-center gap-1.5 transition cursor-pointer shadow-sm"
          >
            <Plus class="w-4 h-4" />
            <span>Tambah Promosi Sekarang</span>
          </button>
        </div>
      </div>

      <!-- 6A. TAMPILAN TABEL STANDAR DESKTOP (Default) -->
      <div v-else-if="viewMode === 'table'" class="bg-white border border-slate-300 rounded-xl overflow-hidden shadow-xs">
        <div class="overflow-x-auto">
          <table class="w-full text-left text-xs border-collapse min-w-[950px]">
            <thead class="bg-slate-100 border-b border-slate-300 text-slate-700 font-mono uppercase tracking-wider font-bold text-[11px]">
              <tr>
                <th class="p-3.5">Nama & Deskripsi Promo</th>
                <th class="p-3.5">Mekanisme Promo</th>
                <th class="p-3.5">Aturan & Syarat Reward</th>
                <th class="p-3.5">Periode Berlaku</th>
                <th class="p-3.5 text-center">Status</th>
                <th v-if="authStore.hasPermission(['promotions:write', 'master:write'])" class="p-3.5 text-center">Aksi</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-200 font-sans">
              <tr
                v-for="promo in paginatedPromotions"
                :key="promo.id"
                class="hover:bg-emerald-50/30 transition group"
              >
                <!-- Nama Promo -->
                <td class="p-3.5">
                  <div class="font-bold text-slate-900 flex items-center gap-2">
                    <Sparkles class="w-4 h-4 text-amber-500 shrink-0" />
                    <span>{{ promo.name }}</span>
                  </div>
                  <div v-if="promo.description" class="text-[11px] text-slate-500 mt-0.5 line-clamp-1">
                    {{ promo.description }}
                  </div>
                  <div class="text-[10px] text-slate-400 font-mono mt-0.5">
                    ID: {{ promo.id.substring(0, 8) }}
                  </div>
                </td>

                <!-- Tipe Promo -->
                <td class="p-3.5">
                  <span
                    class="px-2.5 py-1 rounded-full text-[10px] font-bold uppercase border tracking-wider"
                    :class="getPromoTypeBadgeClass(promo.promo_type)"
                  >
                    {{ getPromoTypeLabel(promo.promo_type) }}
                  </span>
                </td>

                <!-- Aturan Promo -->
                <td class="p-3.5 text-slate-700 font-medium">
                  <div class="flex items-center gap-1.5">
                    <Gift class="w-3.5 h-3.5 text-rose-500 shrink-0" />
                    <span>{{ getRuleSummaryText(promo) }}</span>
                  </div>
                  <div v-if="promo.rules && promo.rules.length > 1" class="text-[10px] text-slate-400 mt-0.5 font-mono">
                    +{{ promo.rules.length - 1 }} variasi aturan lainnya
                  </div>
                </td>

                <!-- Periode Masa Berlaku -->
                <td class="p-3.5">
                  <div class="flex items-center gap-1.5 text-slate-700 font-medium text-[11px]">
                    <Calendar class="w-3.5 h-3.5 text-slate-400 shrink-0" />
                    <span>
                      {{ new Date(promo.start_date).toLocaleDateString('id-ID') }} - {{ new Date(promo.end_date).toLocaleDateString('id-ID') }}
                    </span>
                  </div>
                </td>

                <!-- Status Aktif / Toggle Button -->
                <td class="p-3.5 text-center">
                  <button
                    v-if="authStore.hasPermission(['promotions:write', 'master:write'])"
                    type="button"
                    @click="handleToggleStatus(promo)"
                    class="px-2.5 py-1 rounded-full text-[10px] font-black uppercase transition cursor-pointer border inline-flex items-center gap-1 active:scale-95"
                    :class="promo.is_active
                      ? 'bg-emerald-50 text-emerald-800 border-emerald-300 hover:bg-emerald-100'
                      : 'bg-slate-100 text-slate-600 border-slate-300 hover:bg-slate-200'"
                    :title="promo.is_active ? 'Klik untuk nonaktifkan promo' : 'Klik untuk mengaktifkan promo'"
                  >
                    <span class="w-1.5 h-1.5 rounded-full" :class="promo.is_active ? 'bg-emerald-600' : 'bg-slate-400'"></span>
                    <span>{{ promo.is_active ? 'AKTIF' : 'NON-AKTIF' }}</span>
                  </button>
                  <span
                    v-else
                    class="px-2 py-0.5 rounded-full text-[10px] font-bold uppercase border"
                    :class="promo.is_active ? 'bg-emerald-50 text-emerald-800 border-emerald-300' : 'bg-slate-100 text-slate-600 border-slate-300'"
                  >
                    {{ promo.is_active ? 'AKTIF' : 'NON-AKTIF' }}
                  </span>
                </td>

                <!-- Tombol Aksi -->
                <td v-if="authStore.hasPermission(['promotions:write', 'master:write'])" class="p-3.5 text-center">
                  <div class="flex items-center justify-center gap-1.5">
                    <button
                      @click="openEditModal(promo)"
                      class="p-1.5 text-slate-600 hover:text-emerald-700 hover:bg-emerald-50 rounded-lg transition border border-transparent hover:border-emerald-300 cursor-pointer"
                      title="Edit Program Promo"
                    >
                      <Edit2 class="w-3.5 h-3.5" />
                    </button>
                    <button
                      @click="handleDelete(promo)"
                      class="p-1.5 text-slate-600 hover:text-rose-700 hover:bg-rose-50 rounded-lg transition border border-transparent hover:border-rose-300 cursor-pointer"
                      title="Hapus Program Promo"
                    >
                      <Trash2 class="w-3.5 h-3.5" />
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <!-- Pagination Bar -->
        <PaginationBar
          v-model:currentPage="currentPage"
          v-model:pageSize="pageSize"
          :totalItems="filteredPromotions.length"
        />
      </div>

      <!-- 6B. TAMPILAN GRID KARTU (Jika Dipilih User) -->
      <div v-else class="space-y-4">
        <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
          <div
            v-for="promo in paginatedPromotions"
            :key="promo.id"
            class="bg-white rounded-xl border border-slate-300 p-4 shadow-xs hover:shadow-md transition flex flex-col justify-between"
          >
            <div class="space-y-3">
              <div class="flex items-center justify-between">
                <span
                  class="px-2.5 py-1 text-[10px] font-bold rounded-lg uppercase tracking-wider border"
                  :class="getPromoTypeBadgeClass(promo.promo_type)"
                >
                  {{ getPromoTypeLabel(promo.promo_type) }}
                </span>

                <span
                  class="px-2 py-0.5 text-[10px] font-black rounded-full uppercase border"
                  :class="promo.is_active ? 'bg-emerald-50 text-emerald-700 border-emerald-300' : 'bg-slate-100 text-slate-500 border-slate-300'"
                >
                  {{ promo.is_active ? '● AKTIF' : 'NON-AKTIF' }}
                </span>
              </div>

              <h3 class="text-sm font-bold text-slate-900 leading-snug">{{ promo.name }}</h3>
              <p v-if="promo.description" class="text-xs text-slate-500 line-clamp-2">{{ promo.description }}</p>

              <!-- Rules Summary -->
              <div class="p-2.5 bg-slate-50 rounded-lg border border-slate-200 text-xs space-y-1.5 font-medium text-slate-700">
                <div v-for="(rule, idx) in promo.rules" :key="idx" class="flex items-start gap-1.5">
                  <Gift class="w-4 h-4 text-rose-500 text-sm mt-0.5 shrink-0" />
                  <span class="text-[11px]">
                    <template v-if="promo.promo_type === 'BUY_X_GET_Y'">
                      Beli <b>{{ formatQty(rule.buy_quantity) }}</b> {{ rule.buy_product_name || 'Item Tertentu' }}, Gratis/Diskon <b>{{ formatQty(rule.reward_quantity) }}</b> (Diskon {{ formatPercent(rule.reward_discount_percent) }})
                    </template>
                    <template v-else-if="promo.promo_type === 'MIN_PURCHASE_DISCOUNT'">
                      Min. Belanja <b>{{ formatRupiah(rule.min_transaction_amount || 0) }}</b> &bull; Diskon {{ Number(rule.reward_discount_percent) > 0 ? formatPercent(rule.reward_discount_percent) : formatRupiah(rule.reward_price || 0) }}
                    </template>
                    <template v-else>
                      Aturan: {{ rule.rule_type }}
                    </template>
                  </span>
                </div>
              </div>

              <div class="text-[11px] text-slate-500 flex items-center gap-1.5">
                <Calendar class="w-3.5 h-3.5 text-slate-400" />
                <span>Periode: {{ new Date(promo.start_date).toLocaleDateString('id-ID') }} - {{ new Date(promo.end_date).toLocaleDateString('id-ID') }}</span>
              </div>
            </div>

            <!-- Footer Card Actions -->
            <div class="pt-3 mt-3 border-t border-slate-100 flex items-center justify-between">
              <span class="text-[10px] text-slate-400 font-mono">ID: {{ promo.id.substring(0, 8) }}</span>
              <div v-if="authStore.hasPermission(['promotions:write', 'master:write'])" class="flex items-center gap-2">
                <button
                  @click="openEditModal(promo)"
                  class="p-1.5 text-slate-600 hover:text-emerald-700 hover:bg-emerald-50 rounded-lg transition text-xs font-bold flex items-center gap-1 cursor-pointer"
                  title="Edit Promosi"
                >
                  <Edit2 class="w-3.5 h-3.5" />
                  <span>Edit</span>
                </button>
                <button
                  @click="handleDelete(promo)"
                  class="p-1.5 text-rose-600 hover:bg-rose-50 rounded-lg transition text-xs font-bold flex items-center gap-1 cursor-pointer"
                  title="Hapus Promosi"
                >
                  <Trash2 class="w-3.5 h-3.5" />
                  <span>Hapus</span>
                </button>
              </div>
            </div>
          </div>
        </div>

        <!-- Pagination Bar untuk Mode Grid -->
        <div class="bg-white border border-slate-300 rounded-xl overflow-hidden shadow-xs">
          <PaginationBar
            v-model:currentPage="currentPage"
            v-model:pageSize="pageSize"
            :totalItems="filteredPromotions.length"
          />
        </div>
      </div>

      <!-- 7. Modal Form: Tambah / Edit Promosi Standar Back-Office -->
      <div
        v-if="showModal"
        class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/50 backdrop-blur-xs select-none"
      >
        <div class="bg-white rounded-xl shadow-2xl border border-slate-300 w-full max-w-2xl max-h-[92vh] flex flex-col overflow-hidden">
          <!-- Header Modal Standar -->
          <div class="px-6 py-4 bg-slate-50 border-b border-slate-200 flex items-center justify-between">
            <div class="flex items-center gap-2.5">
              <Icon icon="streamline-ultimate-color:discount" class="w-5 h-5" />
              <h3 class="font-bold text-sm text-slate-900">
                {{ isEditing ? 'Edit Program Promosi Retail' : 'Buat Program Promosi Retail Baru' }}
              </h3>
            </div>
            <button
              @click="showModal = false"
              class="text-slate-400 hover:text-slate-700 p-1 rounded-lg cursor-pointer transition"
            >
              <X class="w-5 h-5" />
            </button>
          </div>

          <!-- Content Modal Form -->
          <div class="p-6 overflow-y-auto flex-1 space-y-4 text-xs">
            <div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
              <div class="sm:col-span-2">
                <label class="block text-slate-700 font-bold uppercase mb-1">Nama Program Promosi *</label>
                <input
                  v-model="form.name"
                  type="text"
                  placeholder="Contoh: Beli 2 Susu UHT Gratis 1 Roti Tawar"
                  class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-medium focus:bg-white focus:border-emerald-600 focus:outline-none"
                />
              </div>
              <div>
                <label class="block text-slate-700 font-bold uppercase mb-1">Kode Promo (Opsional)</label>
                <input
                  v-model="form.code"
                  type="text"
                  placeholder="Otomatis jika kosong"
                  class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-medium focus:bg-white focus:border-emerald-600 focus:outline-none uppercase"
                />
              </div>
            </div>

            <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <div>
                <label class="block text-slate-700 font-bold uppercase mb-1">Mekanisme Promosi</label>
                <select
                  v-model="form.promo_type"
                  class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-medium focus:bg-white focus:border-emerald-600 focus:outline-none"
                >
                  <option value="BUY_X_GET_Y">Beli X Gratis Y / Diskon Item</option>
                  <option value="MIN_PURCHASE_DISCOUNT">Diskon Belanja (Min. Belanja)</option>
                  <option value="BUNDLE_PRICE">Paket Bundling Spesial</option>
                  <option value="PURCHASE_WITH_PURCHASE">Tebus Murah (PWP)</option>
                </select>
              </div>

              <div>
                <label class="block text-slate-700 font-bold uppercase mb-1">Status Keaktifan</label>
                <select
                  v-model="form.is_active"
                  class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-medium focus:bg-white focus:border-emerald-600 focus:outline-none"
                >
                  <option :value="true">Aktif (Berlaku di Kasir)</option>
                  <option :value="false">Non-Aktif (Draft / Berhenti)</option>
                </select>
              </div>
            </div>

            <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <div>
                <label class="block text-slate-700 font-bold uppercase mb-1">Tanggal Mulai</label>
                <input
                  v-model="form.start_date"
                  type="date"
                  class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono focus:bg-white focus:border-emerald-600 focus:outline-none"
                />
              </div>
              <div>
                <label class="block text-slate-700 font-bold uppercase mb-1">Tanggal Berakhir</label>
                <input
                  v-model="form.end_date"
                  type="date"
                  class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono focus:bg-white focus:border-emerald-600 focus:outline-none"
                />
              </div>
            </div>

            <!-- Rules Configuration Box -->
            <div class="border border-slate-300 rounded-xl p-4 bg-slate-50 space-y-3">
              <div class="flex items-center justify-between">
                <span class="text-xs font-black text-slate-800 uppercase flex items-center gap-1.5">
                  <Gift class="w-3.5 h-3.5 text-rose-600" />
                  <span>Aturan & Syarat Item Promo</span>
                </span>
                <button
                  type="button"
                  @click="addRule"
                  class="text-[11px] font-bold text-emerald-700 hover:text-emerald-800 flex items-center gap-1 cursor-pointer bg-emerald-50 px-2 py-0.5 rounded border border-emerald-300"
                >
                  <Plus class="w-3.5 h-3.5" />
                  <span>Tambah Baris Aturan</span>
                </button>
              </div>

              <div
                v-for="(rule, idx) in form.rules"
                :key="idx"
                class="bg-white p-3.5 rounded-xl border border-slate-300 space-y-3 relative shadow-2xs"
              >
                <button
                  v-if="form.rules.length > 1"
                  type="button"
                  @click="removeRule(idx)"
                  class="absolute top-2 right-2 text-slate-400 hover:text-rose-600 p-1 cursor-pointer transition"
                  title="Hapus baris aturan"
                >
                  <Trash2 class="w-3.5 h-3.5" />
                </button>

                <!-- BUY_X_GET_Y -->
                <template v-if="form.promo_type === 'BUY_X_GET_Y'">
                  <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
                    <div>
                      <label class="block text-[11px] font-bold text-slate-700 mb-1">Produk Syarat (Beli):</label>
                      <SearchableSelect
                        :model-value="rule.buy_product_id"
                        api-endpoint="/products"
                        placeholder="-- Semua Produk / Bebas --"
                        search-placeholder="Ketik nama produk, SKU, barcode..."
                        secondary-key="barcode"
                        :initial-label="rule.buy_product_name"
                        :allow-clear="true"
                        @change="(val: any, prod: any) => {
                          rule.buy_product_id = val || ''
                          rule.buy_product_name = prod?.name || ''
                        }"
                      />
                    </div>
                    <div>
                      <label class="block text-[11px] font-bold text-slate-700 mb-1">Jumlah Beli (X):</label>
                      <input v-model.number="rule.buy_quantity" type="number" min="1" class="w-full px-2.5 py-2 text-xs bg-slate-50 border border-slate-300 rounded-lg focus:bg-white focus:border-emerald-600 font-mono" />
                    </div>
                  </div>

                  <div class="grid grid-cols-1 sm:grid-cols-3 gap-3 pt-2 border-t border-slate-100">
                    <div>
                      <label class="block text-[11px] font-bold text-slate-700 mb-1">Produk Hadiah (Gratis/Diskon):</label>
                      <SearchableSelect
                        :model-value="rule.reward_product_id"
                        api-endpoint="/products"
                        placeholder="-- Sama Dengan Produk Beli --"
                        search-placeholder="Ketik nama produk hadiah, SKU, barcode..."
                        secondary-key="barcode"
                        :initial-label="rule.reward_product_name"
                        :allow-clear="true"
                        @change="(val: any, prod: any) => {
                          rule.reward_product_id = val || ''
                          rule.reward_product_name = prod?.name || ''
                        }"
                      />
                    </div>
                    <div>
                      <label class="block text-[11px] font-bold text-slate-700 mb-1">Jumlah Hadiah (Y):</label>
                      <input v-model.number="rule.reward_quantity" type="number" min="1" class="w-full px-2.5 py-2 text-xs bg-slate-50 border border-slate-300 rounded-lg focus:bg-white focus:border-emerald-600 font-mono" />
                    </div>
                    <div>
                      <label class="block text-[11px] font-bold text-slate-700 mb-1">Diskon Hadiah (%):</label>
                      <input v-model.number="rule.reward_discount_percent" type="number" min="0" max="100" class="w-full px-2.5 py-2 text-xs bg-slate-50 border border-slate-300 rounded-lg focus:bg-white focus:border-emerald-600 font-mono" />
                      <span class="text-[10px] text-slate-400">100% = Gratis Murni</span>
                    </div>
                  </div>
                </template>

                <!-- MIN_PURCHASE_DISCOUNT -->
                <template v-else-if="form.promo_type === 'MIN_PURCHASE_DISCOUNT'">
                  <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
                    <div>
                      <label class="block text-[11px] font-bold text-slate-700 mb-1">Min. Total Belanja (Rp):</label>
                      <input v-model.number="rule.min_transaction_amount" type="number" step="1000" min="0" class="w-full px-2.5 py-2 text-xs bg-slate-50 border border-slate-300 rounded-lg focus:bg-white focus:border-emerald-600 font-mono" />
                    </div>
                    <div>
                      <label class="block text-[11px] font-bold text-slate-700 mb-1">Persen Potongan (%):</label>
                      <input v-model.number="rule.reward_discount_percent" type="number" min="0" max="100" class="w-full px-2.5 py-2 text-xs bg-slate-50 border border-slate-300 rounded-lg focus:bg-white focus:border-emerald-600 font-mono" />
                    </div>
                  </div>
                  <div>
                    <label class="block text-[11px] font-bold text-slate-700 mb-1">Atau Potongan Nominal Tetap (Rp):</label>
                    <input v-model.number="rule.reward_price" type="number" step="1000" min="0" class="w-full px-2.5 py-2 text-xs bg-slate-50 border border-slate-300 rounded-lg focus:bg-white focus:border-emerald-600 font-mono" />
                  </div>
                </template>

                <!-- PURCHASE_WITH_PURCHASE -->
                <template v-else-if="form.promo_type === 'PURCHASE_WITH_PURCHASE'">
                  <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
                    <div>
                      <label class="block text-[11px] font-bold text-slate-700 mb-1">Syarat Min. Belanja (Rp):</label>
                      <input v-model.number="rule.min_transaction_amount" type="number" step="1000" min="0" class="w-full px-2.5 py-2 text-xs bg-slate-50 border border-slate-300 rounded-lg focus:bg-white focus:border-emerald-600 font-mono" />
                    </div>
                    <div>
                      <label class="block text-[11px] font-bold text-slate-700 mb-1">Produk Tebus Murah:</label>
                      <SearchableSelect
                        :model-value="rule.reward_product_id"
                        api-endpoint="/products"
                        placeholder="-- Pilih Produk Tebusan --"
                        search-placeholder="Ketik nama produk, SKU, barcode..."
                        secondary-key="barcode"
                        :initial-label="rule.reward_product_name"
                        :allow-clear="true"
                        @change="(val: any, prod: any) => {
                          rule.reward_product_id = val || ''
                          rule.reward_product_name = prod?.name || ''
                        }"
                      />
                    </div>
                  </div>
                  <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
                    <div>
                      <label class="block text-[11px] font-bold text-slate-700 mb-1">Harga Tebus Spesial (Rp):</label>
                      <input v-model.number="rule.reward_price" type="number" step="500" min="0" class="w-full px-2.5 py-2 text-xs bg-slate-50 border border-slate-300 rounded-lg focus:bg-white focus:border-emerald-600 font-mono" />
                    </div>
                    <div>
                      <label class="block text-[11px] font-bold text-slate-700 mb-1">Maks. Tebus per Transaksi:</label>
                      <input v-model.number="rule.max_usage_per_transaction" type="number" min="1" class="w-full px-2.5 py-2 text-xs bg-slate-50 border border-slate-300 rounded-lg focus:bg-white focus:border-emerald-600 font-mono" />
                    </div>
                  </div>
                </template>

                <!-- BUNDLE_PRICE -->
                <template v-else>
                  <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
                    <div>
                      <label class="block text-[11px] font-bold text-slate-700 mb-1">Produk Dalam Paket:</label>
                      <SearchableSelect
                        :model-value="rule.buy_product_id"
                        api-endpoint="/products"
                        placeholder="-- Pilih Produk Paket --"
                        search-placeholder="Ketik nama produk, SKU, barcode..."
                        secondary-key="barcode"
                        :initial-label="rule.buy_product_name"
                        :allow-clear="false"
                        @change="(val: any, prod: any) => {
                          rule.buy_product_id = val || ''
                          rule.buy_product_name = prod?.name || ''
                        }"
                      />
                    </div>
                    <div>
                      <label class="block text-[11px] font-bold text-slate-700 mb-1">Qty Dalam Paket:</label>
                      <input v-model.number="rule.buy_quantity" type="number" min="1" class="w-full px-2.5 py-2 text-xs bg-slate-50 border border-slate-300 rounded-lg focus:bg-white focus:border-emerald-600 font-mono" />
                    </div>
                  </div>
                  <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
                    <div>
                      <label class="block text-[11px] font-bold text-slate-700 mb-1">Diskon Bundle (%):</label>
                      <input v-model.number="rule.reward_discount_percent" type="number" min="0" max="100" class="w-full px-2.5 py-2 text-xs bg-slate-50 border border-slate-300 rounded-lg focus:bg-white focus:border-emerald-600 font-mono" />
                    </div>
                    <div>
                      <label class="block text-[11px] font-bold text-slate-700 mb-1">Atau Potongan Nominal (Rp):</label>
                      <input v-model.number="rule.reward_price" type="number" step="1000" min="0" class="w-full px-2.5 py-2 text-xs bg-slate-50 border border-slate-300 rounded-lg focus:bg-white focus:border-emerald-600 font-mono" />
                    </div>
                  </div>
                </template>
              </div>
            </div>

            <div>
              <label class="block text-slate-700 font-bold uppercase mb-1">Deskripsi Tambahan</label>
              <textarea
                v-model="form.description"
                rows="2"
                placeholder="Keterangan syarat dan ketentuan promosi..."
                class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 focus:bg-white focus:border-emerald-600 focus:outline-none"
              ></textarea>
            </div>
          </div>

          <!-- Footer Modal Standar -->
          <div class="px-6 py-4 bg-slate-50 border-t border-slate-200 flex items-center justify-end gap-2">
            <button
              type="button"
              @click="showModal = false"
              class="px-4 py-2 border border-slate-300 bg-slate-100 hover:bg-slate-200 text-slate-700 font-bold text-xs rounded-lg transition cursor-pointer"
            >
              Batal
            </button>
            <button
              type="button"
              @click="handleSavePromo"
              :disabled="isSubmitting || !form.name.trim()"
              class="px-5 py-2 bg-emerald-600 hover:bg-emerald-700 disabled:opacity-50 text-white font-bold text-xs rounded-lg shadow-sm border border-emerald-700 transition cursor-pointer"
            >
              <span v-if="isSubmitting">Menyimpan...</span>
              <span v-else>{{ isEditing ? 'Simpan Perubahan' : 'Buat Promosi' }}</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  </AdminLayout>
</template>
