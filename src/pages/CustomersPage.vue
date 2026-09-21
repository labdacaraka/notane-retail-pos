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
  Plus
} from 'lucide-vue-next'
import { api } from '@/services/api'
import { useAuthStore } from '@/stores/authStore'

const authStore = useAuthStore()

// Active Tab: 'customers' | 'groups'
const activeTab = ref<'customers' | 'groups'>('customers')

// Customers Data
const customers = ref<any[]>([])
const groups = ref<any[]>([])
const isLoading = ref<boolean>(false)
const searchQuery = ref<string>('')
const selectedGroupFilter = ref<number | ''>('')

// Pagination for Customers
const custPage = ref<number>(1)
const custPageSize = ref<number>(10)
const paginatedCustomers = computed(() => {
  const start = (custPage.value - 1) * custPageSize.value
  return customers.value.slice(start, start + custPageSize.value)
})

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

async function fetchCustomers() {
  isLoading.value = true
  try {
    const res = await api.get('/customers', {
      params: {
        search: searchQuery.value || undefined,
        group_id: selectedGroupFilter.value || undefined,
      },
    })
    if (res.data.success) {
      customers.value = res.data.data
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
  if (!authStore.hasPermission('master:write')) {
    alert('Akses ditolak: Anda hanya memiliki hak baca master data (master:read)')
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
  if (!authStore.hasPermission('master:write')) {
    alert('Akses ditolak: Anda hanya memiliki hak baca master data (master:read)')
    return
  }
  if (!confirm(`Nonaktifkan pelanggan "${name}"?`)) return
  try {
    await api.delete(`/customers/${id}`)
    fetchCustomers()
  } catch (err: any) {
    alert(err.response?.data?.message || 'Gagal menonaktifkan pelanggan')
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
  if (!groupForm.value.name) {
    alert('Nama grup pelanggan wajib diisi')
    return
  }

  try {
    if (isEditingGroup.value && currentGroupId.value) {
      await api.put(`/customer-groups/${currentGroupId.value}`, groupForm.value)
    } else {
      await api.post('/customer-groups', groupForm.value)
    }
    showGroupModal.value = false
    fetchGroups()
    fetchCustomers()
  } catch (err: any) {
    alert(err.response?.data?.message || 'Gagal menyimpan grup pelanggan')
  }
}

async function handleDeleteGroup(id: number, name: string) {
  if (!confirm(`Nonaktifkan grup "${name}"?`)) return
  try {
    await api.delete(`/customer-groups/${id}`)
    fetchGroups()
  } catch (err: any) {
    alert(err.response?.data?.message || 'Gagal menonaktifkan grup')
  }
}

function formatCurrency(val: number | string) {
  return new Intl.NumberFormat('id-ID', {
    style: 'currency',
    currency: 'IDR',
    maximumFractionDigits: 0,
  }).format(Number(val) || 0)
}

onMounted(() => {
  fetchGroups()
  fetchCustomers()
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

        <div v-if="authStore.hasPermission('master:write')" class="flex items-center gap-2">
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
          <span>Daftar Pelanggan ({{ customers.length }})</span>
        </button>

        <button
          @click="activeTab = 'groups'"
          class="pb-2.5 flex items-center gap-2 transition border-b-2"
          :class="activeTab === 'groups' ? 'border-emerald-600 text-emerald-800 font-bold' : 'border-transparent text-slate-500 hover:text-slate-900'"
        >
          <FolderTree class="w-4 h-4" />
          <span>Grup & Tier Pelanggan ({{ groups.length }})</span>
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
              @input="fetchCustomers"
              type="text"
              placeholder="Cari berdasarkan nama, kode pelanggan, atau nomor HP..."
              class="w-full bg-slate-50 border border-slate-300 rounded-lg pl-9 pr-4 py-1.5 text-xs text-slate-900 placeholder-slate-400 focus:outline-none focus:bg-white focus:border-emerald-600 font-medium"
            />
          </div>

          <div class="flex items-center gap-2">
            <span class="text-xs text-slate-600 font-semibold">Grup:</span>
            <select
              v-model="selectedGroupFilter"
              @change="fetchCustomers"
              class="bg-slate-50 border border-slate-300 rounded-lg px-3 py-1.5 text-xs text-slate-900 focus:outline-none focus:bg-white focus:border-emerald-600 font-medium"
            >
              <option value="">Semua Grup</option>
              <option v-for="g in groups" :key="g.id" :value="g.id">{{ g.name }}</option>
            </select>
          </div>
        </div>

        <!-- Table -->
        <div class="bg-white border border-slate-300 rounded-xl overflow-hidden shadow-sm">
          <table class="w-full text-left text-xs border-collapse">
            <thead class="bg-slate-100 border-b border-slate-300 text-slate-700 font-mono uppercase tracking-wider font-bold text-[11px]">
              <tr>
                <th class="p-3.5">Kode</th>
                <th class="p-3.5">Nama Pelanggan</th>
                <th class="p-3.5">Grup / Tier</th>
                <th class="p-3.5">Kontak (HP / Email)</th>
                <th class="p-3.5 text-right">Poin Loyalti</th>
                <th class="p-3.5 text-right font-medium">Limit Piutang</th>
                <th v-if="authStore.hasPermission('master:write')" class="p-3.5 text-center">Aksi</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-200 font-sans">
              <tr v-if="isLoading">
                <td colspan="7" class="p-8 text-center text-slate-500">Memuat data pelanggan...</td>
              </tr>
              <tr v-else-if="customers.length === 0">
                <td colspan="7" class="p-8 text-center text-slate-500">Tidak ada pelanggan ditemukan</td>
              </tr>
              <tr v-for="c in paginatedCustomers" :key="c.id" class="hover:bg-emerald-50/40 transition">
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
                  <span class="inline-flex items-center gap-1 font-mono font-bold text-amber-800 bg-amber-100 border border-amber-300 px-2 py-0.5 rounded-md text-[11px]">
                    <Award class="w-3.5 h-3.5 text-amber-700" />
                    <span>{{ c.reward_points }} Poin</span>
                  </span>
                </td>
                <td class="p-3.5 text-right font-mono text-slate-800 font-medium">
                  {{ formatCurrency(c.credit_limit) }}
                </td>
                <td v-if="authStore.hasPermission('master:write')" class="p-3.5 text-center">
                  <div class="flex items-center justify-center gap-1.5">
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

          <PaginationBar
            v-model:currentPage="custPage"
            v-model:pageSize="custPageSize"
            :totalItems="customers.length"
          />
        </div>
      </div>

      <!-- TAB 2: CUSTOMER GROUPS -->
      <div v-if="activeTab === 'groups'" class="space-y-4">
        <div class="bg-white border border-slate-300 rounded-xl overflow-hidden shadow-sm">
          <table class="w-full text-left text-xs border-collapse">
            <thead class="bg-slate-100 border-b border-slate-300 text-slate-700 font-mono uppercase tracking-wider font-bold text-[11px]">
              <tr>
                <th class="p-3.5">ID</th>
                <th class="p-3.5">Nama Grup Pelanggan</th>
                <th class="p-3.5">Deskripsi / Benefit Grup</th>
                <th v-if="authStore.hasPermission('master:write')" class="p-3.5 text-center">Aksi</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-200 font-sans">
              <tr v-for="g in paginatedGroups" :key="g.id" class="hover:bg-emerald-50/40 transition">
                <td class="p-3.5 font-mono text-slate-500 font-bold">#{{ g.id }}</td>
                <td class="p-3.5 font-bold text-slate-900 tracking-wide">{{ g.name }}</td>
                <td class="p-3.5 text-slate-600">{{ g.description || '-' }}</td>
                <td v-if="authStore.hasPermission('master:write')" class="p-3.5 text-center">
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

          <PaginationBar
            v-model:currentPage="groupPage"
            v-model:pageSize="groupPageSize"
            :totalItems="groups.length"
          />
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
  </AdminLayout>
</template>
