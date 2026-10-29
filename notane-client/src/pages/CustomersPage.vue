<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import AdminLayout from '@/layouts/AdminLayout.vue'
import PaginationBar from '@/components/common/PaginationBar.vue'
import SearchableSelect from '@/components/common/SearchableSelect.vue'
import {
  Users,
  UserPlus,
  Search,
  Award,
  Edit2,
  Trash2,
  Phone,
  Mail,
  FolderTree,
  ArrowUpDown,
  ArrowUp,
  ArrowDown,
  ArrowUpRight,
  ArrowDownLeft,
  RefreshCw,
  Sliders,
  Sparkles,
  Percent,
  Coins,
  HelpCircle,
  Check,
  Info
} from 'lucide-vue-next'
import { api } from '@/services/api'
import { useAuthStore } from '@/stores/authStore'
import { useDialog } from '@/composables/useDialog'
import { formatCurrency, formatQty } from '@/utils/formatters'
import CustomerPointHistoryModal from '@/components/customers/CustomerPointHistoryModal.vue'
import type { LoyaltySettings } from '@/types'

const authStore = useAuthStore()
const dialog = useDialog()

// Point History Modal
const showPointHistoryModal = ref<boolean>(false)
const selectedCustomerForPoints = ref<any | null>(null)

function openPointHistoryModal(c: any) {
  selectedCustomerForPoints.value = c
  showPointHistoryModal.value = true
}

// Active Tab: 'customers' | 'point_history' | 'groups' | 'loyalty'
const activeTab = ref<'customers' | 'point_history' | 'groups' | 'loyalty'>('customers')

// Riwayat Mutasi Poin (Tab Riwayat Poin)
const pointLedgers = ref<any[]>([])
const isLoadingPoints = ref<boolean>(false)
const pointPage = ref<number>(1)
const pointPageSize = ref<number>(10)
const totalPointRecords = ref<number>(0)
const pointSearchQuery = ref<string>('')
const selectedPointTypeFilter = ref<string>('ALL')
const pointStartDate = ref<string>('')
const pointEndDate = ref<string>('')
const pointSummary = ref<{
  total_earned: number
  total_redeemed: number
  total_active_points: number
}>({
  total_earned: 0,
  total_redeemed: 0,
  total_active_points: 0,
})

async function fetchPointLedgers() {
  isLoadingPoints.value = true
  try {
    const res = await api.get('/customers/point-ledgers', {
      params: {
        page: pointPage.value,
        page_size: pointPageSize.value,
        search: pointSearchQuery.value || undefined,
        transaction_type: selectedPointTypeFilter.value === 'ALL' ? undefined : selectedPointTypeFilter.value,
        start_date: pointStartDate.value || undefined,
        end_date: pointEndDate.value || undefined,
      },
    })
    if (res.data.success) {
      pointLedgers.value = res.data.data || []
      totalPointRecords.value = res.data.pagination?.total_items ?? 0
      if (res.data.summary) {
        pointSummary.value = res.data.summary
      }
    }
  } catch (err) {
    console.error('Error fetching point ledgers:', err)
  } finally {
    isLoadingPoints.value = false
  }
}

let pointSearchTimer: ReturnType<typeof setTimeout> | null = null
function handlePointSearchInput() {
  if (pointSearchTimer) clearTimeout(pointSearchTimer)
  pointSearchTimer = setTimeout(() => {
    pointPage.value = 1
    fetchPointLedgers()
  }, 300)
}

function handlePointFilterChange() {
  pointPage.value = 1
  fetchPointLedgers()
}

function switchToPointHistory(customerNameOrCode?: string) {
  if (customerNameOrCode) {
    pointSearchQuery.value = customerNameOrCode
  }
  activeTab.value = 'point_history'
  pointPage.value = 1
  fetchPointLedgers()
}

function formatDate(isoStr: string) {
  if (!isoStr) return '-'
  const d = new Date(isoStr)
  return d.toLocaleString('id-ID', {
    day: '2-digit',
    month: 'short',
    year: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  })
}

// Customers Data
const customers = ref<any[]>([])
const groups = ref<any[]>([])
const isLoading = ref<boolean>(false)
const searchQuery = ref<string>('')
const selectedGroupFilter = ref<number | ''>('')

// Server-side Pagination & Sorting for Customers
const custPage = ref<number>(1)
const custPageSize = ref<number>(10)
const totalCustomers = ref<number>(0)
const sortBy = ref<string>('name')
const sortDir = ref<'asc' | 'desc'>('asc')

function toggleSort(col: string) {
  if (sortBy.value === col) {
    sortDir.value = sortDir.value === 'asc' ? 'desc' : 'asc'
  } else {
    sortBy.value = col
    sortDir.value = 'asc'
  }
  custPage.value = 1
  fetchCustomers()
}

// Pagination for Groups
const groupPage = ref<number>(1)
const groupPageSize = ref<number>(10)
const paginatedGroups = computed(() => {
  const start = (groupPage.value - 1) * groupPageSize.value
  return groups.value.slice(start, start + groupPageSize.value)
})

// Modal state for Customer
const showCustomerModal = ref<boolean>(false)
const isEditingCustomer = ref<boolean>(false)
const currentCustomerId = ref<string | null>(null)
const customerForm = ref({
  customer_group_id: 1,
  code: '',
  name: '',
  phone: '',
  email: '',
  credit_limit: 0,
})

// Modal state for Customer Group
const showGroupModal = ref<boolean>(false)
const isEditingGroup = ref<boolean>(false)
const currentGroupId = ref<number | null>(null)
const groupForm = ref({
  name: '',
  description: '',
})

async function fetchGroups() {
  try {
    const res = await api.get('/customer-groups')
    if (res.data.success) {
      groups.value = res.data.data
    }
  } catch (err) {
    console.error('Error fetching groups:', err)
  }
}

let searchTimer: ReturnType<typeof setTimeout> | null = null
function handleSearchInput() {
  if (searchTimer) clearTimeout(searchTimer)
  searchTimer = setTimeout(() => {
    custPage.value = 1
    fetchCustomers()
  }, 300)
}

function handleFilterChange() {
  custPage.value = 1
  fetchCustomers()
}

async function fetchCustomers() {
  isLoading.value = true
  try {
    const res = await api.get('/customers', {
      params: {
        page: custPage.value,
        page_size: custPageSize.value,
        search: searchQuery.value || undefined,
        group_id: selectedGroupFilter.value || undefined,
        sort_by: sortBy.value,
        sort_dir: sortDir.value,
      },
    })
    if (res.data.success) {
      customers.value = res.data.data || []
      totalCustomers.value = res.data.total_items ?? (res.data.data?.length || 0)
    }
  } catch (err) {
    console.error('Error fetching customers:', err)
  } finally {
    isLoading.value = false
  }
}

function openCreateCustomerModal() {
  isEditingCustomer.value = false
  currentCustomerId.value = null
  customerForm.value = {
    customer_group_id: groups.value[0]?.id || 1,
    code: `CUST-${Date.now().toString().slice(-6)}`,
    name: '',
    phone: '',
    email: '',
    credit_limit: 0,
  }
  showCustomerModal.value = true
}

function openEditCustomerModal(c: any) {
  isEditingCustomer.value = true
  currentCustomerId.value = c.id
  customerForm.value = {
    customer_group_id: c.customer_group_id || 1,
    code: c.code,
    name: c.name,
    phone: c.phone || '',
    email: c.email || '',
    credit_limit: Number(c.credit_limit || 0),
  }
  showCustomerModal.value = true
}

async function handleSaveCustomer() {
  const reqPerm = isEditingCustomer.value ? ['customers:update', 'customers:manage', 'master:write'] : ['customers:create', 'customers:manage', 'master:write']
  if (!authStore.hasPermission(reqPerm)) {
    dialog.alert('Akses ditolak: Anda tidak memiliki wewenang mengelola data pelanggan (customers:create / customers:update)', 'warning')
    return
  }
  if (!customerForm.value.code || !customerForm.value.name) {
    alert('Kode dan nama pelanggan wajib diisi')
    return
  }

  try {
    if (isEditingCustomer.value && currentCustomerId.value) {
      await api.put(`/customers/${currentCustomerId.value}`, customerForm.value)
    } else {
      await api.post('/customers', customerForm.value)
    }
    showCustomerModal.value = false
    fetchCustomers()
  } catch (err: any) {
    alert(err.response?.data?.message || 'Gagal menyimpan data pelanggan')
  }
}

async function handleDeleteCustomer(id: string, name: string) {
  if (!authStore.hasPermission(['customers:delete', 'customers:manage', 'master:write'])) {
    dialog.alert('Akses ditolak: Anda tidak memiliki wewenang menonaktifkan pelanggan (customers:delete)', 'warning')
    return
  }
  const confirmed = await dialog.confirm({
    title: 'Nonaktifkan Pelanggan',
    message: `Apakah Anda yakin ingin menonaktifkan pelanggan "${name}"?`,
    subMessage: 'Pelanggan yang dinonaktifkan tidak akan dapat dipilih pada transaksi kasir POS.',
    confirmText: 'Ya, Nonaktifkan',
    cancelText: 'Batal',
    type: 'danger',
  })
  if (!confirmed) return
  try {
    await api.delete(`/customers/${id}`)
    dialog.toast(`Pelanggan "${name}" dinonaktifkan`, 'success')
    fetchCustomers()
  } catch (err: any) {
    dialog.alert(err.response?.data?.message || 'Gagal menonaktifkan pelanggan', 'danger')
  }
}

// Group Modal Handlers
function openCreateGroupModal() {
  isEditingGroup.value = false
  currentGroupId.value = null
  groupForm.value = { name: '', description: '' }
  showGroupModal.value = true
}

function openEditGroupModal(g: any) {
  isEditingGroup.value = true
  currentGroupId.value = g.id
  groupForm.value = { name: g.name, description: g.description || '' }
  showGroupModal.value = true
}

async function handleSaveGroup() {
  const reqPerm = isEditingGroup.value ? ['customers:update', 'customers:manage', 'master:write'] : ['customers:create', 'customers:manage', 'master:write']
  if (!authStore.hasPermission(reqPerm)) {
    dialog.alert('Akses ditolak: Anda tidak memiliki wewenang mengelola grup pelanggan (customers:manage)', 'warning')
    return
  }
  if (!groupForm.value.name) {
    dialog.alert('Nama grup pelanggan wajib diisi', 'warning')
    return
  }

  try {
    if (isEditingGroup.value && currentGroupId.value) {
      await api.put(`/customer-groups/${currentGroupId.value}`, groupForm.value)
    } else {
      await api.post('/customer-groups', groupForm.value)
    }
    showGroupModal.value = false
    dialog.toast('Grup pelanggan berhasil disimpan', 'success')
    fetchGroups()
    fetchCustomers()
  } catch (err: any) {
    dialog.alert(err.response?.data?.message || 'Gagal menyimpan grup pelanggan', 'danger')
  }
}

async function handleDeleteGroup(id: number, name: string) {
  if (!authStore.hasPermission(['customers:delete', 'customers:manage', 'master:write'])) {
    dialog.alert('Akses ditolak: Anda tidak memiliki wewenang menghapus grup pelanggan (customers:delete)', 'warning')
    return
  }
  const confirmed = await dialog.confirm({
    title: 'Nonaktifkan Grup Pelanggan',
    message: `Apakah Anda yakin ingin menonaktifkan grup "${name}"?`,
    subMessage: 'Harga khusus grup ini tidak akan berlaku lagi untuk pelanggan terkait.',
    confirmText: 'Ya, Nonaktifkan',
    cancelText: 'Batal',
    type: 'danger',
  })
  if (!confirmed) return
  try {
    await api.delete(`/customer-groups/${id}`)
    dialog.toast(`Grup "${name}" dinonaktifkan`, 'success')
    fetchGroups()
  } catch (err: any) {
    dialog.alert(err.response?.data?.message || 'Gagal menonaktifkan grup', 'danger')
  }
}

// Loyalty Settings State
const loyaltyForm = ref<LoyaltySettings>({
  is_enabled: true,
  point_value_rupiah: 100,
  min_spend_per_point: 10000,
  max_point_usage_percent: 50,
  min_redeem_points: 1,
})
const isSavingLoyalty = ref(false)

const canManageLoyalty = computed(() =>
  authStore.hasPermission(['loyalty:manage', 'settings:manage', 'master:write'])
)

async function fetchLoyaltySettings() {
  try {
    const res = await api.get('/settings/loyalty')
    if (res.data?.success && res.data.data) {
      loyaltyForm.value = {
        id: res.data.data.id,
        is_enabled: res.data.data.is_enabled,
        point_value_rupiah: parseFloat(res.data.data.point_value_rupiah) || 100,
        min_spend_per_point: parseFloat(res.data.data.min_spend_per_point) || 10000,
        max_point_usage_percent: parseFloat(res.data.data.max_point_usage_percent) || 50,
        min_redeem_points: res.data.data.min_redeem_points || 1,
      }
    }
  } catch (err) {
    console.error('Gagal mengambil pengaturan loyalty:', err)
  }
}

async function handleSaveLoyalty() {
  // 1. Client-Side Pre-flight Permission Guard
  if (!canManageLoyalty.value) {
    dialog.alert('Akses Ditolak: Anda tidak memiliki hak akses untuk mengubah aturan loyalitas dan poin.', 'danger')
    return
  }

  // 2. Client-Side Form Validation
  if (!loyaltyForm.value.point_value_rupiah || loyaltyForm.value.point_value_rupiah <= 0) {
    dialog.alert('Nilai 1 poin dalam rupiah harus lebih besar dari 0', 'warning')
    return
  }

  if (loyaltyForm.value.max_point_usage_percent < 0 || loyaltyForm.value.max_point_usage_percent > 100) {
    dialog.alert('Batas persentase penggunaan poin harus berada di antara 0% dan 100%', 'warning')
    return
  }

  try {
    isSavingLoyalty.value = true
    const res = await api.put('/settings/loyalty', loyaltyForm.value)
    if (res.data?.success) {
      dialog.toast('Pengaturan konversi dan aturan poin berhasil disimpan', 'success')
    } else {
      dialog.alert(res.data?.message || 'Gagal menyimpan pengaturan poin.', 'danger')
    }
  } catch (err: any) {
    dialog.alert(err.response?.data?.message || err.message, 'danger')
  } finally {
    isSavingLoyalty.value = false
  }
}

onMounted(() => {
  fetchGroups()
  fetchCustomers()
  fetchLoyaltySettings()
  fetchPointLedgers()
})
</script>

<template>
  <AdminLayout>
    <div class="p-6 space-y-4">
      <!-- Page Header -->
      <div class="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h2 class="text-xl font-bold text-slate-900 flex items-center gap-2.5">
            <Icon icon="streamline-ultimate-color:multiple-users-1" class="w-6 h-6 drop-shadow-xs" />
            <span>Master Pelanggan & Member Loyalti</span>
          </h2>
          <p class="text-xs text-slate-500">
            Kelola data pelanggan, member VIP, limit piutang, poin reward, dan grup pelanggan.
          </p>
        </div>

        <div v-if="authStore.hasPermission(['customers:manage', 'master:write'])" class="flex items-center gap-2">
          <button
            v-if="activeTab === 'customers'"
            @click="openCreateCustomerModal"
            class="bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-xs px-4 py-2 rounded-lg shadow-sm flex items-center gap-2 transition active:scale-95 border border-emerald-700"
          >
            <UserPlus class="w-4 h-4" />
            <span>Tambah Pelanggan</span>
          </button>

          <button
            v-if="activeTab === 'groups'"
            @click="openCreateGroupModal"
            class="bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-xs px-4 py-2 rounded-lg shadow-sm flex items-center gap-2 transition active:scale-95 border border-emerald-700"
          >
            <Plus class="w-4 h-4" />
            <span>Tambah Grup Member</span>
          </button>
        </div>
      </div>

      <!-- Tab Navigation -->
      <div class="flex border-b border-slate-300 gap-6 text-sm font-semibold">
        <button
          @click="activeTab = 'customers'"
          class="pb-2.5 flex items-center gap-2 transition border-b-2"
          :class="activeTab === 'customers' ? 'border-emerald-600 text-emerald-800 font-bold' : 'border-transparent text-slate-500 hover:text-slate-900'"
        >
          <Users class="w-4 h-4" />
          <span>Daftar Pelanggan ({{ totalCustomers }})</span>
        </button>

        <button
          @click="switchToPointHistory()"
          class="pb-2.5 flex items-center gap-2 transition border-b-2"
          :class="activeTab === 'point_history' ? 'border-amber-600 text-amber-800 font-bold' : 'border-transparent text-slate-500 hover:text-slate-900'"
        >
          <Coins class="w-4 h-4 text-amber-600" />
          <span>Riwayat Mutasi Poin ({{ totalPointRecords }})</span>
        </button>

        <button
          @click="activeTab = 'groups'"
          class="pb-2.5 flex items-center gap-2 transition border-b-2"
          :class="activeTab === 'groups' ? 'border-emerald-600 text-emerald-800 font-bold' : 'border-transparent text-slate-500 hover:text-slate-900'"
        >
          <FolderTree class="w-4 h-4" />
          <span>Grup & Tier Pelanggan ({{ groups.length }})</span>
        </button>

        <button
          @click="activeTab = 'loyalty'"
          class="pb-2.5 flex items-center gap-2 transition border-b-2"
          :class="activeTab === 'loyalty' ? 'border-amber-600 text-amber-800 font-bold' : 'border-transparent text-slate-500 hover:text-slate-900'"
        >
          <Award class="w-4 h-4 text-amber-600" />
          <span>Aturan Poin & Konversi Rupiah</span>
        </button>
      </div>

      <!-- TAB 1: CUSTOMERS -->
      <div v-if="activeTab === 'customers'" class="space-y-4">
        <!-- Search and Filter -->
        <div class="flex flex-wrap items-center gap-3 bg-white border border-slate-300 p-2.5 rounded-xl shadow-sm">
          <div class="relative flex-1 min-w-[240px]">
            <Search class="w-4 h-4 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2" />
            <input
              v-model="searchQuery"
              @input="handleSearchInput"
              type="text"
              placeholder="Cari berdasarkan nama, kode pelanggan, atau nomor HP..."
              class="w-full bg-slate-50 border border-slate-300 rounded-lg pl-9 pr-4 py-1.5 text-xs text-slate-900 placeholder-slate-400 focus:outline-none focus:bg-white focus:border-emerald-600 font-medium"
            />
          </div>

          <div class="flex items-center gap-2">
            <span class="text-xs text-slate-600 font-semibold">Grup:</span>
            <select
              v-model="selectedGroupFilter"
              @change="handleFilterChange"
              class="bg-slate-50 border border-slate-300 rounded-lg px-3 py-1.5 text-xs text-slate-900 focus:outline-none focus:bg-white focus:border-emerald-600 font-medium"
            >
              <option value="">Semua Grup</option>
              <option v-for="g in groups" :key="g.id" :value="g.id">{{ g.name }}</option>
            </select>
          </div>
        </div>

        <!-- Table -->
        <div class="bg-white border border-slate-300 rounded-xl overflow-hidden shadow-sm">
          <div class="overflow-x-auto">
            <table class="w-full text-left text-xs border-collapse min-w-[850px]">
              <thead class="bg-slate-100 border-b border-slate-300 text-slate-700 font-mono uppercase tracking-wider font-bold text-[11px]">
                <tr>
                  <th @click="toggleSort('code')" class="p-3.5 cursor-pointer hover:bg-slate-200/70 select-none">
                    <div class="flex items-center gap-1.5">
                      <span>Kode</span>
                      <ArrowUp v-if="sortBy === 'code' && sortDir === 'asc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowDown v-else-if="sortBy === 'code' && sortDir === 'desc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th @click="toggleSort('name')" class="p-3.5 cursor-pointer hover:bg-slate-200/70 select-none">
                    <div class="flex items-center gap-1.5">
                      <span>Nama Pelanggan</span>
                      <ArrowUp v-if="sortBy === 'name' && sortDir === 'asc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowDown v-else-if="sortBy === 'name' && sortDir === 'desc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th class="p-3.5">Grup / Tier</th>
                  <th class="p-3.5">Kontak (HP / Email)</th>
                  <th @click="toggleSort('reward_points')" class="p-3.5 text-right cursor-pointer hover:bg-slate-200/70 select-none">
                    <div class="flex items-center justify-end gap-1.5">
                      <span>Poin Loyalti</span>
                      <ArrowUp v-if="sortBy === 'reward_points' && sortDir === 'asc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowDown v-else-if="sortBy === 'reward_points' && sortDir === 'desc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th @click="toggleSort('credit_limit')" class="p-3.5 text-right font-medium cursor-pointer hover:bg-slate-200/70 select-none">
                    <div class="flex items-center justify-end gap-1.5">
                      <span>Limit Piutang</span>
                      <ArrowUp v-if="sortBy === 'credit_limit' && sortDir === 'asc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowDown v-else-if="sortBy === 'credit_limit' && sortDir === 'desc'" class="w-3.5 h-3.5 text-emerald-600" />
                      <ArrowUpDown v-else class="w-3 h-3 text-slate-400 opacity-60" />
                    </div>
                  </th>
                  <th v-if="authStore.hasPermission(['customers:manage', 'master:write'])" class="p-3.5 text-center">Aksi</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-slate-200 font-sans">
                <tr v-if="isLoading">
                  <td colspan="7" class="p-8 text-center text-slate-500">Memuat data pelanggan...</td>
                </tr>
                <tr v-else-if="customers.length === 0">
                  <td colspan="7" class="p-8 text-center text-slate-500">Tidak ada pelanggan ditemukan</td>
                </tr>
                <tr v-for="c in customers" :key="c.id" class="hover:bg-emerald-50/40 transition">
                  <td class="p-3.5 font-mono font-bold text-emerald-700">{{ c.code }}</td>
                  <td class="p-3.5 font-bold text-slate-900">{{ c.name }}</td>
                  <td class="p-3.5">
                    <span
                      class="px-2.5 py-1 rounded-full text-[10px] font-extrabold uppercase tracking-wide border"
                      :class="{
                        'bg-emerald-100 text-emerald-800 border-emerald-300': c.group_name === 'MEMBER',
                        'bg-purple-100 text-purple-800 border-purple-300': c.group_name === 'PLATINUM',
                        'bg-blue-100 text-blue-800 border-blue-300': c.group_name === 'WHOLESALE',
                        'bg-slate-100 text-slate-700 border-slate-300': c.group_name === 'REGULAR',
                      }"
                    >
                      {{ c.group_name }}
                    </span>
                  </td>
                  <td class="p-3.5 space-y-0.5">
                    <div class="flex items-center gap-1.5 text-slate-800 font-medium">
                      <Phone class="w-3 h-3 text-slate-500" />
                      <span>{{ c.phone || '-' }}</span>
                    </div>
                    <div v-if="c.email" class="flex items-center gap-1.5 text-slate-500 text-[11px]">
                      <Mail class="w-3 h-3 text-slate-400" />
                      <span>{{ c.email }}</span>
                    </div>
                  </td>
                  <td class="p-3.5 text-right">
                    <button
                      type="button"
                      @click="openPointHistoryModal(c)"
                      title="Klik untuk melihat riwayat mutasi poin"
                      class="inline-flex items-center gap-1 font-mono font-bold text-amber-800 bg-amber-100 hover:bg-amber-200 border border-amber-300 px-2 py-0.5 rounded-md text-[11px] transition shadow-2xs cursor-pointer group"
                    >
                      <Award class="w-3.5 h-3.5 text-amber-700 group-hover:scale-110 transition-transform" />
                      <span>{{ formatQty(c.reward_points) }} Poin</span>
                    </button>
                  </td>
                  <td class="p-3.5 text-right font-mono text-slate-800 font-medium">
                    {{ formatCurrency(c.credit_limit) }}
                  </td>
                  <td v-if="authStore.hasPermission(['customers:manage', 'master:write'])" class="p-3.5 text-center">
                    <div class="flex items-center justify-center gap-1.5">
                      <button
                        @click="openPointHistoryModal(c)"
                        title="Lihat Riwayat Poin"
                        class="p-1.5 rounded-md bg-white hover:bg-amber-50 border border-slate-300 hover:border-amber-400 text-amber-600 transition shadow-xs"
                      >
                        <Coins class="w-3.5 h-3.5" />
                      </button>
                      <button
                        @click="openEditCustomerModal(c)"
                        title="Edit Pelanggan"
                        class="p-1.5 rounded-md bg-white hover:bg-slate-100 border border-slate-300 text-blue-600 transition shadow-xs"
                      >
                        <Edit2 class="w-3.5 h-3.5" />
                      </button>
                      <button
                        @click="handleDeleteCustomer(c.id, c.name)"
                        title="Nonaktifkan Pelanggan"
                        class="p-1.5 rounded-md bg-white hover:bg-slate-100 border border-slate-300 text-rose-600 transition shadow-xs"
                      >
                        <Trash2 class="w-3.5 h-3.5" />
                      </button>
                    </div>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <PaginationBar
            v-model:currentPage="custPage"
            v-model:pageSize="custPageSize"
            :totalItems="totalCustomers"
            @update:currentPage="fetchCustomers"
            @update:pageSize="() => { custPage = 1; fetchCustomers() }"
          />
        </div>
      </div>

      <!-- TAB 2: RIWAYAT MUTASI POIN (TABLE WITH PAGING) -->
      <div v-if="activeTab === 'point_history'" class="space-y-4">
        <!-- 3 Summary Stats Cards -->
        <div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
          <div class="bg-linear-to-br from-amber-50 to-amber-100/50 border border-amber-200/80 rounded-xl p-4 shadow-xs">
            <div class="flex items-center justify-between text-amber-700 text-xs font-semibold">
              <span>Total Saldo Poin Beredar</span>
              <Award class="w-4 h-4 text-amber-600" />
            </div>
            <div class="mt-2 flex items-baseline gap-1.5">
              <span class="text-2xl font-black font-mono text-amber-900">
                {{ formatQty(pointSummary.total_active_points) }}
              </span>
              <span class="text-xs font-bold text-amber-700">Poin Aktif</span>
            </div>
            <p class="text-[11px] text-amber-600/80 mt-1">Total saldo reward seluruh member</p>
          </div>

          <div class="bg-linear-to-br from-emerald-50 to-emerald-100/50 border border-emerald-200/80 rounded-xl p-4 shadow-xs">
            <div class="flex items-center justify-between text-emerald-700 text-xs font-semibold">
              <span>Total Poin Diberikan (Masuk)</span>
              <ArrowDownLeft class="w-4 h-4 text-emerald-600" />
            </div>
            <div class="mt-2 flex items-baseline gap-1.5">
              <span class="text-2xl font-black font-mono text-emerald-800">
                +{{ formatQty(pointSummary.total_earned) }}
              </span>
              <span class="text-xs font-bold text-emerald-600">Poin</span>
            </div>
            <p class="text-[11px] text-emerald-600/80 mt-1">Akumulasi reward belanja pelanggan</p>
          </div>

          <div class="bg-linear-to-br from-rose-50 to-rose-100/50 border border-rose-200/80 rounded-xl p-4 shadow-xs">
            <div class="flex items-center justify-between text-rose-700 text-xs font-semibold">
              <span>Total Poin Ditukarkan (Keluar)</span>
              <ArrowUpRight class="w-4 h-4 text-rose-600" />
            </div>
            <div class="mt-2 flex items-baseline gap-1.5">
              <span class="text-2xl font-black font-mono text-rose-800">
                -{{ formatQty(pointSummary.total_redeemed) }}
              </span>
              <span class="text-xs font-bold text-rose-600">Poin</span>
            </div>
            <p class="text-[11px] text-rose-600/80 mt-1">Dipakai untuk potongan transaksi POS</p>
          </div>
        </div>

        <!-- Filter & Search Toolbar -->
        <div class="flex flex-wrap items-center justify-between gap-3 bg-white border border-slate-300 p-2.5 rounded-xl shadow-xs">
          <!-- Search -->
          <div class="relative flex-1 min-w-[220px]">
            <Search class="w-4 h-4 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2" />
            <input
              v-model="pointSearchQuery"
              @input="handlePointSearchInput"
              type="text"
              placeholder="Cari member, no. nota, atau keterangan..."
              class="w-full bg-slate-50 border border-slate-300 rounded-lg pl-9 pr-4 py-1.5 text-xs text-slate-900 placeholder-slate-400 focus:outline-none focus:bg-white focus:border-amber-500 font-medium"
            />
          </div>

          <!-- Transaction Type Filter -->
          <div class="w-48">
            <select
              v-model="selectedPointTypeFilter"
              @change="handlePointFilterChange"
              class="w-full bg-slate-50 border border-slate-300 rounded-lg px-2.5 py-1.5 text-xs text-slate-800 font-medium focus:outline-none focus:bg-white focus:border-amber-500"
            >
              <option value="ALL">Semua Jenis Mutasi</option>
              <option value="EARNED">Poin Belanja (Masuk)</option>
              <option value="REDEEMED">Penukaran Poin (Keluar)</option>
              <option value="CORRECTION">Koreksi Manual</option>
            </select>
          </div>

          <!-- Date Filters -->
          <div class="flex items-center gap-2 text-xs text-slate-600">
            <div class="flex items-center gap-1.5 bg-slate-50 border border-slate-300 rounded-lg px-2 py-1">
              <span class="text-slate-400 text-[11px]">Dari:</span>
              <input
                v-model="pointStartDate"
                @change="handlePointFilterChange"
                type="date"
                class="bg-transparent text-slate-800 text-xs focus:outline-none"
              />
            </div>
            <div class="flex items-center gap-1.5 bg-slate-50 border border-slate-300 rounded-lg px-2 py-1">
              <span class="text-slate-400 text-[11px]">Sampai:</span>
              <input
                v-model="pointEndDate"
                @change="handlePointFilterChange"
                type="date"
                class="bg-transparent text-slate-800 text-xs focus:outline-none"
              />
            </div>
            <button
              @click="fetchPointLedgers"
              :disabled="isLoadingPoints"
              title="Muat Ulang Data"
              class="p-1.5 rounded-lg border border-slate-300 text-slate-600 hover:text-slate-900 hover:bg-slate-100 transition shadow-2xs"
            >
              <RefreshCw class="w-3.5 h-3.5" :class="{ 'animate-spin': isLoadingPoints }" />
            </button>
          </div>
        </div>

        <!-- Point History Table with Paging -->
        <div class="bg-white border border-slate-300 rounded-xl overflow-hidden shadow-xs">
          <div class="overflow-x-auto">
            <table class="w-full text-left text-xs border-collapse min-w-[850px]">
              <thead class="bg-slate-100 border-b border-slate-300 text-slate-700 font-mono uppercase tracking-wider font-bold text-[11px]">
                <tr>
                  <th class="p-3.5">Waktu</th>
                  <th class="p-3.5">Pelanggan / Member</th>
                  <th class="p-3.5">Tipe Mutasi</th>
                  <th class="p-3.5">No. Nota / Referensi</th>
                  <th class="p-3.5 text-right">Masuk (+)</th>
                  <th class="p-3.5 text-right">Keluar (-)</th>
                  <th class="p-3.5 text-right">Saldo Akhir</th>
                  <th class="p-3.5">Keterangan</th>
                  <th class="p-3.5">Operator</th>
                  <th class="p-3.5 text-center">Aksi</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-slate-200 font-sans">
                <tr v-if="isLoadingPoints">
                  <td colspan="10" class="p-8 text-center text-slate-500">
                    <div class="flex items-center justify-center gap-2">
                      <RefreshCw class="w-4 h-4 animate-spin text-amber-600" />
                      <span>Memuat riwayat transaksi poin...</span>
                    </div>
                  </td>
                </tr>
                <tr v-else-if="pointLedgers.length === 0">
                  <td colspan="10" class="p-10 text-center text-slate-500">
                    <div class="flex flex-col items-center justify-center gap-2 max-w-sm mx-auto">
                      <div class="w-10 h-10 rounded-full bg-slate-100 flex items-center justify-center text-slate-400">
                        <Coins class="w-5 h-5" />
                      </div>
                      <p class="font-bold text-slate-700 text-sm">Tidak Ada Riwayat Poin</p>
                      <p class="text-xs text-slate-400">
                        Tidak ada transaksi mutasi poin yang sesuai dengan kriteria filter saat ini.
                      </p>
                    </div>
                  </td>
                </tr>
                <tr
                  v-for="item in pointLedgers"
                  :key="item.id"
                  class="hover:bg-amber-50/30 transition"
                >
                  <!-- Waktu -->
                  <td class="p-3.5 font-mono text-slate-600 whitespace-nowrap text-[11px]">
                    {{ formatDate(item.created_at) }}
                  </td>

                  <!-- Pelanggan -->
                  <td class="p-3.5">
                    <div class="font-bold text-slate-900">{{ item.customer_name }}</div>
                    <div class="flex items-center gap-1.5 mt-0.5">
                      <span class="font-mono text-[10px] text-emerald-700 font-semibold">{{ item.customer_code }}</span>
                      <span
                        v-if="item.customer_group_name"
                        class="px-1.5 py-0.2 rounded text-[9px] font-extrabold uppercase border bg-amber-50 text-amber-800 border-amber-200"
                      >
                        {{ item.customer_group_name }}
                      </span>
                    </div>
                  </td>

                  <!-- Tipe -->
                  <td class="p-3.5 whitespace-nowrap">
                    <span
                      v-if="item.transaction_type === 'EARNED'"
                      class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-extrabold uppercase bg-emerald-100 text-emerald-800 border border-emerald-300"
                    >
                      <ArrowDownLeft class="w-3 h-3" />
                      Belanja
                    </span>
                    <span
                      v-else-if="item.transaction_type === 'REDEEMED'"
                      class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-extrabold uppercase bg-rose-100 text-rose-800 border border-rose-300"
                    >
                      <ArrowUpRight class="w-3 h-3" />
                      Penukaran
                    </span>
                    <span
                      v-else-if="item.transaction_type === 'CORRECTION'"
                      class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-extrabold uppercase bg-blue-100 text-blue-800 border border-blue-300"
                    >
                      <Sliders class="w-3 h-3" />
                      Koreksi
                    </span>
                    <span
                      v-else
                      class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-extrabold uppercase bg-slate-100 text-slate-800 border border-slate-300"
                    >
                      {{ item.transaction_type }}
                    </span>
                  </td>

                  <!-- No. Nota -->
                  <td class="p-3.5 font-mono font-bold whitespace-nowrap">
                    <span v-if="item.reference_number" class="text-blue-700 bg-blue-50 px-2 py-0.5 rounded border border-blue-200 text-[11px]">
                      {{ item.reference_number }}
                    </span>
                    <span v-else class="text-slate-400 font-normal">-</span>
                  </td>

                  <!-- Masuk -->
                  <td class="p-3.5 text-right font-mono font-bold whitespace-nowrap">
                    <span v-if="item.points_in > 0" class="text-emerald-700">
                      +{{ formatQty(item.points_in) }}
                    </span>
                    <span v-else class="text-slate-300 font-normal">-</span>
                  </td>

                  <!-- Keluar -->
                  <td class="p-3.5 text-right font-mono font-bold whitespace-nowrap">
                    <span v-if="item.points_out > 0" class="text-rose-700">
                      -{{ formatQty(item.points_out) }}
                    </span>
                    <span v-else class="text-slate-300 font-normal">-</span>
                  </td>

                  <!-- Saldo Akhir -->
                  <td class="p-3.5 text-right font-mono font-black text-slate-900 whitespace-nowrap">
                    {{ formatQty(item.balance) }}
                  </td>

                  <!-- Keterangan -->
                  <td class="p-3.5 text-slate-600 max-w-[200px] truncate" :title="item.notes || ''">
                    {{ item.notes || '-' }}
                  </td>

                  <!-- Operator -->
                  <td class="p-3.5 text-slate-500 whitespace-nowrap">
                    <span class="inline-flex items-center gap-1">
                      <Users class="w-3 h-3 text-slate-400" />
                      <span>{{ item.created_by_name || item.created_by_username || 'Sistem' }}</span>
                    </span>
                  </td>

                  <!-- Aksi -->
                  <td class="p-3.5 text-center">
                    <button
                      @click="openPointHistoryModal({ id: item.customer_id, name: item.customer_name, code: item.customer_code, phone: item.customer_phone, group_name: item.customer_group_name, reward_points: item.balance })"
                      title="Lihat Riwayat & Sesuaikan Poin Member Ini"
                      class="p-1.5 rounded-md bg-white hover:bg-amber-50 border border-slate-300 hover:border-amber-400 text-amber-600 transition shadow-2xs"
                    >
                      <Coins class="w-3.5 h-3.5" />
                    </button>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <!-- Pagination Bar for Point History Table -->
          <PaginationBar
            v-model:currentPage="pointPage"
            v-model:pageSize="pointPageSize"
            :totalItems="totalPointRecords"
            :pageSizeOptions="[10, 25, 50, 100]"
            @update:currentPage="fetchPointLedgers"
            @update:pageSize="() => { pointPage = 1; fetchPointLedgers() }"
          />
        </div>
      </div>

      <!-- TAB 3: CUSTOMER GROUPS -->
      <div v-if="activeTab === 'groups'" class="space-y-4">
        <div class="bg-white border border-slate-300 rounded-xl overflow-hidden shadow-sm">
          <div class="overflow-x-auto">
            <table class="w-full text-left text-xs border-collapse min-w-[550px]">
              <thead class="bg-slate-100 border-b border-slate-300 text-slate-700 font-mono uppercase tracking-wider font-bold text-[11px]">
                <tr>
                  <th class="p-3.5">ID</th>
                  <th class="p-3.5">Nama Grup Pelanggan</th>
                  <th class="p-3.5">Deskripsi / Benefit Grup</th>
                  <th v-if="authStore.hasPermission(['customers:manage', 'master:write'])" class="p-3.5 text-center">Aksi</th>
                </tr>
              </thead>
            <tbody class="divide-y divide-slate-200 font-sans">
              <tr v-for="g in paginatedGroups" :key="g.id" class="hover:bg-emerald-50/40 transition">
                <td class="p-3.5 font-mono text-slate-500 font-bold">#{{ g.id }}</td>
                <td class="p-3.5 font-bold text-slate-900 tracking-wide">{{ g.name }}</td>
                <td class="p-3.5 text-slate-600">{{ g.description || '-' }}</td>
                <td v-if="authStore.hasPermission(['customers:manage', 'master:write'])" class="p-3.5 text-center">
                  <div class="flex items-center justify-center gap-1.5">
                    <button
                      @click="openEditGroupModal(g)"
                      class="p-1.5 rounded-md bg-white hover:bg-slate-100 border border-slate-300 text-blue-600 transition shadow-xs"
                    >
                      <Edit2 class="w-3.5 h-3.5" />
                    </button>
                    <button
                      @click="handleDeleteGroup(g.id, g.name)"
                      class="p-1.5 rounded-md bg-white hover:bg-slate-100 border border-slate-300 text-rose-600 transition shadow-xs"
                    >
                      <Trash2 class="w-3.5 h-3.5" />
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
          </div>

          <PaginationBar
            v-model:currentPage="groupPage"
            v-model:pageSize="groupPageSize"
            :totalItems="groups.length"
          />
        </div>
      </div>

      <!-- TAB 3: LOYALTY POINT SETTINGS -->
      <div v-if="activeTab === 'loyalty'" class="space-y-6">
        <!-- Permission Info Banner if Read-Only -->
        <div v-if="!canManageLoyalty" class="bg-amber-50 border border-amber-200 text-amber-900 rounded-2xl p-4 text-xs flex items-center gap-3 font-medium shadow-xs">
          <Info class="w-5 h-5 text-amber-600 shrink-0" />
          <div>
            <p class="font-bold">Mode Lihat Saja (Read-Only)</p>
            <p class="text-amber-700 mt-0.5">Anda hanya memiliki hak akses melihat pengaturan poin loyalitas. Diperlukan hak akses 'loyalty:manage' atau 'settings:manage' untuk mengubah aturan ini.</p>
          </div>
        </div>

        <!-- Status Card -->
        <div class="bg-white border border-slate-300 rounded-2xl p-6 shadow-sm flex flex-wrap items-center justify-between gap-4">
          <div class="flex items-center gap-4">
            <div class="w-12 h-12 rounded-2xl bg-amber-50 border border-amber-200 flex items-center justify-center text-amber-600 shadow-xs">
              <Coins class="w-6 h-6" />
            </div>
            <div>
              <h3 class="font-extrabold text-slate-900 text-base flex items-center gap-2">
                <span>Program Loyalitas & Poin Reward Member</span>
                <span
                  class="px-2.5 py-0.5 rounded-full text-xs font-bold"
                  :class="loyaltyForm.is_enabled ? 'bg-emerald-100 text-emerald-800 border border-emerald-300' : 'bg-slate-100 text-slate-600 border border-slate-300'"
                >
                  {{ loyaltyForm.is_enabled ? 'AKTIF' : 'NONAKTIF' }}
                </span>
              </h3>
              <p class="text-xs text-slate-500 mt-0.5">
                Konfigurasikan nilai tukar poin reward pelanggan, batasan persentase pembayaran di kasir, dan ambang batas perolehan poin.
              </p>
            </div>
          </div>

          <label class="relative inline-flex items-center cursor-pointer" :class="{ 'opacity-60 cursor-not-allowed': !canManageLoyalty }">
            <input
              type="checkbox"
              v-model="loyaltyForm.is_enabled"
              :disabled="!canManageLoyalty"
              class="sr-only peer"
            />
            <div class="w-11 h-6 bg-slate-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-emerald-600"></div>
            <span class="ml-3 text-xs font-bold text-slate-800">
              {{ loyaltyForm.is_enabled ? 'Program Poin Diaktifkan' : 'Program Poin Dinonaktifkan' }}
            </span>
          </label>
        </div>

        <!-- Form Grid -->
        <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
          <!-- Nilai Konversi Poin ke Rupiah -->
          <div class="bg-white border border-slate-300 rounded-2xl p-5 shadow-sm space-y-3">
            <div class="flex items-center gap-2 text-slate-900 font-bold text-sm">
              <Coins class="w-4 h-4 text-amber-600" />
              <span>Nilai Konversi 1 Poin ke Rupiah</span>
            </div>
            <p class="text-xs text-slate-500">
              Berapa nilai Rupiah yang didapatkan pelanggan untuk setiap 1 poin reward saat digunakan untuk berbelanja di kasir.
            </p>
            <div class="relative">
              <span class="absolute left-3.5 top-1/2 -translate-y-1/2 font-bold text-slate-500 text-xs">Rp</span>
              <input
                v-model.number="loyaltyForm.point_value_rupiah"
                type="number"
                min="1"
                :disabled="!canManageLoyalty"
                class="w-full bg-slate-50 border border-slate-300 rounded-xl pl-10 pr-16 py-2.5 text-sm font-mono font-bold text-slate-900 focus:bg-white focus:border-amber-600 focus:outline-none disabled:bg-slate-100 disabled:cursor-not-allowed"
              />
              <span class="absolute right-3.5 top-1/2 -translate-y-1/2 font-bold text-slate-500 text-xs">/ 1 Poin</span>
            </div>
            <div class="p-3 bg-amber-50/70 border border-amber-200 rounded-xl text-[11px] text-amber-900">
              💡 <strong>Contoh:</strong> 1 Poin = {{ formatCurrency(loyaltyForm.point_value_rupiah, true) }}. Jika pelanggan memiliki <strong>100 Poin</strong>, mereka memiliki saldo potongan sebesar <strong>{{ formatCurrency(100 * loyaltyForm.point_value_rupiah, true) }}</strong>.
            </div>
          </div>

          <!-- Minimal Belanja untuk Dapat 1 Poin -->
          <div class="bg-white border border-slate-300 rounded-2xl p-5 shadow-sm space-y-3">
            <div class="flex items-center gap-2 text-slate-900 font-bold text-sm">
              <Sparkles class="w-4 h-4 text-emerald-600" />
              <span>Minimal Belanja per 1 Poin</span>
            </div>
            <p class="text-xs text-slate-500">
              Kelipatan nominal belanja riil yang harus dibayarkan pelanggan agar otomatis mendapatkan 1 poin reward baru.
            </p>
            <div class="relative">
              <span class="absolute left-3.5 top-1/2 -translate-y-1/2 font-bold text-slate-500 text-xs">Rp</span>
              <input
                v-model.number="loyaltyForm.min_spend_per_point"
                type="number"
                min="100"
                :disabled="!canManageLoyalty"
                class="w-full bg-slate-50 border border-slate-300 rounded-xl pl-10 pr-20 py-2.5 text-sm font-mono font-bold text-slate-900 focus:bg-white focus:border-emerald-600 focus:outline-none disabled:bg-slate-100 disabled:cursor-not-allowed"
              />
              <span class="absolute right-3.5 top-1/2 -translate-y-1/2 font-bold text-slate-500 text-xs">= 1 Poin</span>
            </div>
            <div class="p-3 bg-emerald-50/70 border border-emerald-200 rounded-xl text-[11px] text-emerald-900">
              💡 <strong>Contoh:</strong> Belanja Rp 50.000 akan otomatis menghasilkan <strong>{{ Math.floor(50000 / (loyaltyForm.min_spend_per_point || 10000)) }} Poin</strong> reward bagi member.
            </div>
          </div>

          <!-- Batas Maksimal Persentase Penggunaan Poin -->
          <div class="bg-white border border-slate-300 rounded-2xl p-5 shadow-sm space-y-4 md:col-span-2">
            <div class="flex items-center justify-between">
              <div>
                <div class="flex items-center gap-2 text-slate-900 font-bold text-sm">
                  <Percent class="w-4 h-4 text-indigo-600" />
                  <span>Batas Maksimal Penggunaan Poin per Transaksi</span>
                </div>
                <p class="text-xs text-slate-500 mt-0.5">
                  Aturan persentase maksimal tagihan yang boleh dibayarkan menggunakan poin. Sisa tagihan wajib dilunasi dengan metode pembayaran lain (Tunai, QRIS, EDC, dsb).
                </p>
              </div>
              <div class="flex items-center gap-1.5 font-mono font-extrabold text-xl text-indigo-700 bg-indigo-50 border border-indigo-200 px-3 py-1 rounded-xl">
                <span>{{ loyaltyForm.max_point_usage_percent }}</span>
                <span class="text-xs font-normal text-indigo-500">%</span>
              </div>
            </div>

            <!-- Slider + Presets -->
            <div class="space-y-3 pt-2">
              <input
                type="range"
                min="0"
                max="100"
                step="5"
                v-model.number="loyaltyForm.max_point_usage_percent"
                :disabled="!canManageLoyalty"
                class="w-full accent-indigo-600 h-2 bg-slate-200 rounded-lg cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
              />

              <div class="flex items-center gap-2">
                <span class="text-xs text-slate-500 font-semibold mr-1">Preset Cepat:</span>
                <button
                  type="button"
                  @click="loyaltyForm.max_point_usage_percent = 25"
                  :disabled="!canManageLoyalty"
                  class="px-2.5 py-1 text-xs rounded-lg border font-bold transition disabled:opacity-50 disabled:cursor-not-allowed"
                  :class="loyaltyForm.max_point_usage_percent === 25 ? 'bg-indigo-600 text-white border-indigo-700' : 'bg-slate-50 border-slate-300 text-slate-700 hover:bg-slate-100'"
                >
                  25% Tagihan
                </button>
                <button
                  type="button"
                  @click="loyaltyForm.max_point_usage_percent = 50"
                  :disabled="!canManageLoyalty"
                  class="px-2.5 py-1 text-xs rounded-lg border font-bold transition disabled:opacity-50 disabled:cursor-not-allowed"
                  :class="loyaltyForm.max_point_usage_percent === 50 ? 'bg-indigo-600 text-white border-indigo-700' : 'bg-slate-50 border-slate-300 text-slate-700 hover:bg-slate-100'"
                >
                  50% Tagihan (Standar)
                </button>
                <button
                  type="button"
                  @click="loyaltyForm.max_point_usage_percent = 70"
                  :disabled="!canManageLoyalty"
                  class="px-2.5 py-1 text-xs rounded-lg border font-bold transition disabled:opacity-50 disabled:cursor-not-allowed"
                  :class="loyaltyForm.max_point_usage_percent === 70 ? 'bg-indigo-600 text-white border-indigo-700' : 'bg-slate-50 border-slate-300 text-slate-700 hover:bg-slate-100'"
                >
                  70% Tagihan
                </button>
                <button
                  type="button"
                  @click="loyaltyForm.max_point_usage_percent = 100"
                  :disabled="!canManageLoyalty"
                  class="px-2.5 py-1 text-xs rounded-lg border font-bold transition disabled:opacity-50 disabled:cursor-not-allowed"
                  :class="loyaltyForm.max_point_usage_percent === 100 ? 'bg-indigo-600 text-white border-indigo-700' : 'bg-slate-50 border-slate-300 text-slate-700 hover:bg-slate-100'"
                >
                  100% (Bisa Penuh)
                </button>
              </div>
            </div>

            <div class="p-3 bg-slate-50 border border-slate-200 rounded-xl text-xs text-slate-700 space-y-1">
              <div v-if="loyaltyForm.max_point_usage_percent < 100" class="flex items-center gap-1.5 text-amber-800 font-bold">
                <HelpCircle class="w-4 h-4 text-amber-600 shrink-0" />
                <span>Pelanggan tidak dapat menggunakan poin secara penuh (Maksimal {{ loyaltyForm.max_point_usage_percent }}% dari total belanja).</span>
              </div>
              <div v-else class="flex items-center gap-1.5 text-emerald-800 font-bold">
                <Check class="w-4 h-4 text-emerald-600 shrink-0" />
                <span>Pelanggan dapat melunasi belanjaan hingga 100% menggunakan poin jika saldo poin mencukupi.</span>
              </div>
              <p class="text-[11px] text-slate-500">
                Fitur ini memastikan toko tetap menerima pembayaran riil (uang kas/digital) untuk setiap transaksi, menjaga margin arus kas ritel.
              </p>
            </div>
          </div>

          <!-- Minimal Penukaran Poin -->
          <div class="bg-white border border-slate-300 rounded-2xl p-5 shadow-sm space-y-3">
            <div class="flex items-center gap-2 text-slate-900 font-bold text-sm">
              <Sliders class="w-4 h-4 text-blue-600" />
              <span>Minimal Penukaran Poin</span>
            </div>
            <p class="text-xs text-slate-500">
              Jumlah poin minimal yang harus dimiliki dan ditukarkan dalam satu transaksi di kasir.
            </p>
            <div class="relative">
              <input
                v-model.number="loyaltyForm.min_redeem_points"
                type="number"
                min="1"
                :disabled="!canManageLoyalty"
                class="w-full bg-slate-50 border border-slate-300 rounded-xl px-3.5 py-2.5 text-sm font-mono font-bold text-slate-900 focus:bg-white focus:border-blue-600 focus:outline-none disabled:bg-slate-100 disabled:cursor-not-allowed"
              />
              <span class="absolute right-3.5 top-1/2 -translate-y-1/2 font-bold text-slate-500 text-xs">Poin</span>
            </div>
          </div>

          <!-- Interactive Simulation Box -->
          <div class="bg-slate-900 text-white rounded-2xl p-5 shadow-md space-y-3 flex flex-col justify-between">
            <div>
              <div class="flex items-center gap-2 text-amber-400 font-bold text-xs uppercase tracking-wider">
                <Sparkles class="w-4 h-4" />
                <span>Simulasi Realtime di Kasir POS</span>
              </div>
              <p class="text-xs text-slate-300 mt-1">
                Contoh jika pelanggan belanja senilai <strong>Rp 100.000</strong> dan memiliki <strong>1.000 Poin</strong> (Rp 100.000):
              </p>

              <div class="space-y-1.5 mt-3 text-xs bg-slate-800/80 p-3.5 rounded-xl border border-slate-700 font-mono">
                <div class="flex justify-between text-slate-300">
                  <span>Total Tagihan:</span>
                  <span class="font-bold">Rp 100.000</span>
                </div>
                <div class="flex justify-between text-amber-400">
                  <span>Maksimal Poin ({{ loyaltyForm.max_point_usage_percent }}%):</span>
                  <span class="font-bold">-{{ formatCurrency(100000 * loyaltyForm.max_point_usage_percent / 100, true) }}</span>
                </div>
                <div class="flex justify-between text-slate-400 text-[11px]">
                  <span>Poin Dipotong:</span>
                  <span>{{ Math.round((100000 * loyaltyForm.max_point_usage_percent / 100) / (loyaltyForm.point_value_rupiah || 100)) }} Poin</span>
                </div>
                <div class="pt-1.5 border-t border-slate-700 flex justify-between text-emerald-400 font-bold text-sm">
                  <span>Wajib Bayar Lainnya:</span>
                  <span>{{ formatCurrency(100000 - (100000 * loyaltyForm.max_point_usage_percent / 100), true) }}</span>
                </div>
              </div>
            </div>

            <p class="text-[10.5px] text-slate-400 italic">
              * Sisa {{ formatCurrency(100000 - (100000 * loyaltyForm.max_point_usage_percent / 100), true) }} dapat dibayar kasir menggunakan Tunai, QRIS, atau EDC BCA/Mandiri.
            </p>
          </div>
        </div>

        <!-- Action Button -->
        <div class="flex justify-end pt-2">
          <button
            v-if="canManageLoyalty"
            @click="handleSaveLoyalty"
            :disabled="isSavingLoyalty || !canManageLoyalty"
            class="bg-amber-600 hover:bg-amber-700 disabled:opacity-50 text-white font-bold text-xs px-6 py-2.5 rounded-xl shadow-sm flex items-center gap-2 transition active:scale-95 border border-amber-700"
          >
            <Check class="w-4 h-4" />
            <span>{{ isSavingLoyalty ? 'Menyimpan...' : 'Simpan Perubahan Aturan Poin' }}</span>
          </button>
        </div>
      </div>
    </div>

    <!-- Modal Form: Tambah / Edit Pelanggan -->
    <div
      v-if="showCustomerModal"
      class="fixed inset-0 z-50 bg-slate-900/50 backdrop-blur-xs flex items-center justify-center p-4 select-none"
    >
      <div class="bg-white border border-slate-300 w-full max-w-lg rounded-xl p-6 space-y-4 shadow-2xl">
        <h3 class="font-bold text-base text-slate-900">
          {{ isEditingCustomer ? 'Edit Data Pelanggan' : 'Tambah Pelanggan Baru' }}
        </h3>

        <div class="space-y-3 text-xs">
          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="block text-slate-700 font-medium mb-1">Kode Pelanggan / No Member *</label>
              <input
                v-model="customerForm.code"
                class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono focus:bg-white focus:border-emerald-600"
                placeholder="e.g. MBR-002"
              />
            </div>
            <div>
              <label class="block text-slate-700 font-medium mb-1">Grup / Kategori Member</label>
              <SearchableSelect
                v-model="customerForm.customer_group_id"
                api-endpoint="/customer-groups"
                placeholder="-- Pilih Grup Member --"
                search-placeholder="Cari grup member..."
              />
            </div>
          </div>

          <div>
            <label class="block text-slate-700 font-medium mb-1">Nama Lengkap *</label>
            <input
              v-model="customerForm.name"
              class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 focus:bg-white focus:border-emerald-600"
              placeholder="e.g. Budi Gunawan"
            />
          </div>

          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="block text-slate-700 font-medium mb-1">Nomor WhatsApp / HP</label>
              <input
                v-model="customerForm.phone"
                class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono focus:bg-white focus:border-emerald-600"
                placeholder="08123456789"
              />
            </div>
            <div>
              <label class="block text-slate-700 font-medium mb-1">Email</label>
              <input
                v-model="customerForm.email"
                type="email"
                class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 focus:bg-white focus:border-emerald-600"
                placeholder="budi@example.com"
              />
            </div>
          </div>

          <div>
            <label class="block text-slate-700 font-medium mb-1">Limit Kredit Piutang (Rp)</label>
            <input
              v-model.number="customerForm.credit_limit"
              type="number"
              class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono focus:bg-white focus:border-emerald-600"
              placeholder="0"
            />
            <span class="text-[10px] text-slate-500">Maksimum batas piutang bon belanja kasir</span>
          </div>
        </div>

        <div class="flex justify-end gap-2 pt-4 border-t border-slate-200">
          <button @click="showCustomerModal = false" class="bg-slate-100 hover:bg-slate-200 border border-slate-300 text-slate-700 px-4 py-2 rounded-lg text-xs font-bold">
            Batal
          </button>
          <button @click="handleSaveCustomer" class="bg-emerald-600 hover:bg-emerald-700 border border-emerald-700 text-white px-5 py-2 rounded-lg text-xs font-bold shadow-sm">
            Simpan Pelanggan
          </button>
        </div>
      </div>
    </div>

    <!-- Modal Form: Tambah / Edit Customer Group -->
    <div
      v-if="showGroupModal"
      class="fixed inset-0 z-50 bg-slate-900/50 backdrop-blur-xs flex items-center justify-center p-4 select-none"
    >
      <div class="bg-white border border-slate-300 w-full max-w-md rounded-xl p-6 space-y-4 shadow-2xl">
        <h3 class="font-bold text-base text-slate-900">
          {{ isEditingGroup ? 'Edit Grup Pelanggan' : 'Tambah Grup Pelanggan Baru' }}
        </h3>

        <div class="space-y-3 text-xs">
          <div>
            <label class="block text-slate-700 font-medium mb-1">Nama Grup / Kategori *</label>
            <input
              v-model="groupForm.name"
              class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-bold tracking-wide focus:bg-white focus:border-emerald-600"
              placeholder="e.g. VIP / GOLD / MITRA"
            />
          </div>

          <div>
            <label class="block text-slate-700 font-medium mb-1">Deskripsi</label>
            <textarea
              v-model="groupForm.description"
              rows="3"
              class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 focus:bg-white focus:border-emerald-600"
              placeholder="Keterangan hak istimewa atau benefit grup member..."
            ></textarea>
          </div>
        </div>

        <div class="flex justify-end gap-2 pt-4 border-t border-slate-200">
          <button @click="showGroupModal = false" class="bg-slate-100 hover:bg-slate-200 border border-slate-300 text-slate-700 px-4 py-2 rounded-lg text-xs font-bold">
            Batal
          </button>
          <button @click="handleSaveGroup" class="bg-emerald-600 hover:bg-emerald-700 border border-emerald-700 text-white px-5 py-2 rounded-lg text-xs font-bold shadow-sm">
            Simpan Grup
          </button>
        </div>
      </div>
    </div>

    <!-- Modal Riwayat Poin Pelanggan -->
    <CustomerPointHistoryModal
      :show="showPointHistoryModal"
      :customer="selectedCustomerForPoints"
      @close="showPointHistoryModal = false"
      @updated="fetchCustomers"
    />
  </AdminLayout>
</template>
