<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import AdminLayout from '@/layouts/AdminLayout.vue'
import { Icon } from '@iconify/vue'
import {
  Plus,
  Search,
  Building2,
  Edit2,
  Trash2,
  Phone,
  MapPin,
  ShieldCheck,
  UserCheck,
  Warehouse,
  CheckCircle2,
  AlertCircle,
  RefreshCw,
  X,
} from 'lucide-vue-next'
import { api } from '@/services/api'

// State
const outlets = ref<any[]>([])
const storeManagers = ref<any[]>([])
const supervisors = ref<any[]>([])
const warehouses = ref<any[]>([])
const isLoading = ref<boolean>(false)
const searchQuery = ref<string>('')
const statusFilter = ref<string>('ALL') // 'ALL' | 'ACTIVE' | 'INACTIVE'
const typeFilter = ref<string>('ALL') // 'ALL' | 'STORE' | 'WAREHOUSE'

// Toast Notification
const toastMessage = ref<string>('')
const toastType = ref<'success' | 'error'>('success')
function showToast(msg: string, type: 'success' | 'error' = 'success') {
  toastMessage.value = msg
  toastType.value = type
  setTimeout(() => {
    toastMessage.value = ''
  }, 4000)
}

// Modal Form State
const showModal = ref<boolean>(false)
const isEditing = ref<boolean>(false)
const isSubmitting = ref<boolean>(false)
const formError = ref<string>('')

const form = ref({
  id: '',
  code: '',
  name: '',
  address: '',
  phone: '',
  is_active: true,
  is_warehouse: false,
  manager_id: '',
  default_warehouse_id: '',
  supervisor_ids: [] as string[],
})

// Modal Delete State
const showDeleteModal = ref<boolean>(false)
const outletToDelete = ref<any>(null)
const isDeleting = ref<boolean>(false)
const deleteError = ref<string>('')

// Load Data
async function loadData() {
  try {
    isLoading.value = true
    const [outletsRes, managersRes, spvRes, whRes] = await Promise.all([
      api.get('/outlets'),
      api.get('/auth/store-managers'),
      api.get('/auth/supervisors'),
      api.get('/warehouses'),
    ])

    if (outletsRes.data?.success) {
      outlets.value = outletsRes.data.data
    }
    if (managersRes.data?.success) {
      storeManagers.value = managersRes.data.data
    }
    if (spvRes.data?.success) {
      supervisors.value = spvRes.data.data
    }
    if (whRes.data?.success) {
      warehouses.value = whRes.data.data
    }
  } catch (err: any) {
    console.error('Gagal memuat master outlet:', err)
    showToast('Gagal memuat data master outlet', 'error')
  } finally {
    isLoading.value = false
  }
}

onMounted(() => {
  loadData()
})

// Filtered Outlets
const filteredOutlets = computed(() => {
  let list = outlets.value

  if (typeFilter.value === 'STORE') {
    list = list.filter((o) => !o.is_warehouse)
  } else if (typeFilter.value === 'WAREHOUSE') {
    list = list.filter((o) => o.is_warehouse)
  }

  if (statusFilter.value === 'ACTIVE') {
    list = list.filter((o) => o.is_active)
  } else if (statusFilter.value === 'INACTIVE') {
    list = list.filter((o) => !o.is_active)
  }

  if (searchQuery.value.trim()) {
    const q = searchQuery.value.toLowerCase().trim()
    list = list.filter((o) => {
      const code = (o.code || '').toLowerCase()
      const name = (o.name || '').toLowerCase()
      const address = (o.address || '').toLowerCase()
      const phone = (o.phone || '').toLowerCase()
      const manager = (o.manager_name || '').toLowerCase()
      return (
        code.includes(q) ||
        name.includes(q) ||
        address.includes(q) ||
        phone.includes(q) ||
        manager.includes(q)
      )
    })
  }

  return list
})

// Warehouse Options (Hanya outlet yang memiliki flag is_warehouse = true)
const warehouseOptions = computed(() => {
  return outlets.value.filter(
    (o) => o.is_warehouse && o.is_active && (!isEditing.value || o.id !== form.value.id)
  )
})

// Stats Calculations
const totalOutletsCount = computed(() => outlets.value.length)
const totalStoresCount = computed(() => outlets.value.filter((o) => !o.is_warehouse && o.is_active).length)
const totalWarehousesCount = computed(() => outlets.value.filter((o) => o.is_warehouse && o.is_active).length)
const totalSupervisorsAssigned = computed(() => {
  const set = new Set<string>()
  outlets.value.forEach((o) => {
    ;(o.supervisors || []).forEach((s: any) => set.add(s.id))
  })
  return set.size
})

// Form Actions
function openCreateModal(asWarehouse = false) {
  isEditing.value = false
  formError.value = ''
  form.value = {
    id: '',
    code: asWarehouse ? `WH-00${totalWarehousesCount.value + 1}` : `OUT-00${totalStoresCount.value + 1}`,
    name: '',
    address: '',
    phone: '',
    is_active: true,
    is_warehouse: asWarehouse,
    manager_id: storeManagers.value[0]?.id || '',
    default_warehouse_id: warehouseOptions.value[0]?.id || '',
    supervisor_ids: [],
  }
  showModal.value = true
}

function openEditModal(item: any) {
  isEditing.value = true
  formError.value = ''
  form.value = {
    id: item.id,
    code: item.code,
    name: item.name,
    address: item.address || '',
    phone: item.phone || '',
    is_active: item.is_active,
    is_warehouse: item.is_warehouse || false,
    manager_id: item.manager_id || '',
    default_warehouse_id: item.default_warehouse_id || '',
    supervisor_ids: (item.supervisors || []).map((s: any) => s.id),
  }
  showModal.value = true
}

function toggleSupervisorSelection(spvId: string) {
  const idx = form.value.supervisor_ids.indexOf(spvId)
  if (idx > -1) {
    form.value.supervisor_ids.splice(idx, 1)
  } else {
    form.value.supervisor_ids.push(spvId)
  }
}

async function handleSave() {
  if (!form.value.code.trim()) {
    formError.value = 'Kode lokasi wajib diisi'
    return
  }
  if (!form.value.name.trim()) {
    formError.value = 'Nama lokasi gerai / gudang wajib diisi'
    return
  }

  isSubmitting.value = true
  formError.value = ''

  const payload = {
    code: form.value.code.trim(),
    name: form.value.name.trim(),
    address: form.value.address.trim() || undefined,
    phone: form.value.phone.trim() || undefined,
    is_active: form.value.is_active,
    is_warehouse: form.value.is_warehouse,
    manager_id: form.value.manager_id || undefined,
    default_warehouse_id: !form.value.is_warehouse ? (form.value.default_warehouse_id || undefined) : undefined,
    supervisor_ids: form.value.supervisor_ids,
  }

  try {
    if (isEditing.value) {
      const res = await api.put(`/outlets/${form.value.id}`, payload)
      if (res.data?.success) {
        showToast('Data berhasil diperbarui', 'success')
        showModal.value = false
        loadData()
      } else {
        formError.value = res.data?.message || 'Gagal memperbarui data'
      }
    } else {
      const res = await api.post('/outlets', payload)
      if (res.data?.success) {
        showToast(
          form.value.is_warehouse
            ? 'Gudang baru berhasil ditambahkan'
            : 'Outlet gerai baru berhasil ditambahkan',
          'success'
        )
        showModal.value = false
        loadData()
      } else {
        formError.value = res.data?.message || 'Gagal menambahkan data'
      }
    }
  } catch (err: any) {
    formError.value = err.response?.data?.message || err.message || 'Gagal menyimpan data'
  } finally {
    isSubmitting.value = false
  }
}

function openDeleteModal(item: any) {
  outletToDelete.value = item
  deleteError.value = ''
  showDeleteModal.value = true
}

async function confirmDelete() {
  if (!outletToDelete.value) return
  isDeleting.value = true
  deleteError.value = ''

  try {
    const res = await api.delete(`/outlets/${outletToDelete.value.id}`)
    if (res.data?.success) {
      showToast('Data berhasil dihapus', 'success')
      showDeleteModal.value = false
      loadData()
    } else {
      deleteError.value = res.data?.message || 'Gagal menghapus data'
    }
  } catch (err: any) {
    deleteError.value = err.response?.data?.message || err.message || 'Gagal menghapus data'
  } finally {
    isDeleting.value = false
  }
}
</script>

<template>
  <AdminLayout>
    <div class="h-full flex flex-col bg-slate-100 overflow-hidden font-sans select-none">
      <!-- TOAST NOTIFICATION -->
      <transition
        enter-active-class="transform ease-out duration-300 transition"
        enter-from-class="translate-y-2 opacity-0 sm:translate-y-0 sm:translate-x-2"
        enter-to-class="translate-y-0 opacity-100 sm:translate-x-0"
        leave-active-class="transition ease-in duration-100"
        leave-from-class="opacity-100"
        leave-to-class="opacity-0"
      >
        <div
          v-if="toastMessage"
          class="fixed bottom-5 right-5 z-50 flex items-center gap-3 px-4 py-3 rounded-xl shadow-lg border text-xs font-bold"
          :class="
            toastType === 'success'
              ? 'bg-emerald-600 text-white border-emerald-500'
              : 'bg-rose-600 text-white border-rose-500'
          "
        >
          <CheckCircle2 v-if="toastType === 'success'" class="w-4 h-4 shrink-0" />
          <AlertCircle v-else class="w-4 h-4 shrink-0" />
          <span>{{ toastMessage }}</span>
        </div>
      </transition>

      <!-- 1. HEADER & TOP ACTIONS BAR -->
      <div class="bg-white border-b border-slate-300 px-6 py-4 shrink-0 shadow-2xs space-y-4">
        <!-- Row 1: Judul Halaman & Tombol Aksi Utama di Pojok Kanan -->
        <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
          <div class="flex items-center gap-3">
            <div
              class="w-11 h-11 rounded-2xl bg-amber-50 border border-amber-300 flex items-center justify-center shadow-xs text-amber-700 shrink-0"
            >
              <Icon icon="fluent-color:building-store-24" class="w-7 h-7" />
            </div>
            <div>
              <div class="flex items-center gap-2">
                <h1 class="text-lg font-black text-slate-900 tracking-tight leading-none">
                  Master Outlet & Gudang (Lokasi)
                </h1>
                <span
                  class="bg-amber-100 text-amber-800 text-[10px] font-mono font-bold px-2 py-0.5 rounded border border-amber-300"
                >
                  {{ totalOutletsCount }} Lokasi Terdaftar
                </span>
              </div>
              <p class="text-xs text-slate-500 mt-1">
                Satu master tabel untuk mengelola Cabang Toko Kasir dan Gudang Logistik Penyimpanan Stok
              </p>
            </div>
          </div>

          <!-- Tombol Tambah Lokasi di Pojok Kanan Atas -->
          <div class="flex items-center gap-2 shrink-0">
            <button
              @click="openCreateModal(false)"
              class="bg-amber-600 hover:bg-amber-700 text-white px-4 py-2 rounded-lg text-xs font-bold flex items-center gap-1.5 shadow-sm border border-amber-700 transition active:scale-95 cursor-pointer"
            >
              <Plus class="w-4 h-4" />
              <span>Tambah Lokasi</span>
            </button>
          </div>
        </div>

        <!-- Row 2: Search & Filter Toolbar -->
        <div class="flex items-center justify-between gap-2.5 flex-wrap pt-1 border-t border-slate-100">
          <div class="flex items-center gap-2.5 flex-wrap flex-1">
            <div class="relative w-64">
              <Search class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-slate-400" />
              <input
                v-model="searchQuery"
                type="text"
                placeholder="Cari kode, nama, manajer..."
                class="w-full pl-9 pr-3.5 py-1.5 rounded-lg border border-slate-300 bg-white text-xs text-slate-900 focus:outline-none focus:ring-2 focus:ring-amber-500/20 focus:border-amber-600 shadow-2xs font-sans"
              />
            </div>

            <!-- Type Filter -->
            <select
              v-model="typeFilter"
              class="px-3 py-1.5 rounded-lg border border-slate-300 bg-white text-xs font-bold text-slate-700 focus:outline-none focus:border-amber-600 shadow-2xs cursor-pointer"
            >
              <option value="ALL">Semua Tipe</option>
              <option value="STORE">🏬 Toko Saja</option>
              <option value="WAREHOUSE">🏭 Gudang Saja</option>
            </select>

            <!-- Status Filter -->
            <select
              v-model="statusFilter"
              class="px-3 py-1.5 rounded-lg border border-slate-300 bg-white text-xs font-bold text-slate-700 focus:outline-none focus:border-amber-600 shadow-2xs cursor-pointer"
            >
              <option value="ALL">Semua Status</option>
              <option value="ACTIVE">Aktif Saja</option>
              <option value="INACTIVE">Nonaktif</option>
            </select>
          </div>

          <button
            @click="loadData"
            class="p-2 rounded-lg border border-slate-300 bg-white hover:bg-slate-50 text-slate-600 transition shadow-2xs cursor-pointer active:scale-95"
            title="Segarkan Data"
          >
            <RefreshCw class="w-4 h-4" :class="{ 'animate-spin': isLoading }" />
          </button>
        </div>
      </div>

      <!-- 2. STATS TILES -->
      <div class="px-6 py-3.5 grid grid-cols-2 sm:grid-cols-4 gap-3 shrink-0">
        <div class="bg-white p-3.5 rounded-xl border border-slate-300 shadow-2xs flex items-center gap-3">
          <div class="w-10 h-10 rounded-lg bg-amber-50 border border-amber-200 flex items-center justify-center shrink-0">
            <Building2 class="w-5 h-5 text-amber-600" />
          </div>
          <div>
            <div class="text-[11px] font-bold text-slate-500 uppercase tracking-wider">Total Lokasi</div>
            <div class="text-base font-black text-slate-900 font-mono">{{ totalOutletsCount }} Unit</div>
          </div>
        </div>

        <div class="bg-white p-3.5 rounded-xl border border-slate-300 shadow-2xs flex items-center gap-3">
          <div class="w-10 h-10 rounded-lg bg-emerald-50 border border-emerald-200 flex items-center justify-center shrink-0">
            <Icon icon="fluent-color:building-store-24" class="w-6 h-6" />
          </div>
          <div>
            <div class="text-[11px] font-bold text-slate-500 uppercase tracking-wider">Cabang Toko Ritel</div>
            <div class="text-base font-black text-emerald-700 font-mono">{{ totalStoresCount }} Toko</div>
          </div>
        </div>

        <div class="bg-white p-3.5 rounded-xl border border-slate-300 shadow-2xs flex items-center gap-3">
          <div class="w-10 h-10 rounded-lg bg-blue-50 border border-blue-200 flex items-center justify-center shrink-0">
            <Warehouse class="w-5 h-5 text-blue-600" />
          </div>
          <div>
            <div class="text-[11px] font-bold text-slate-500 uppercase tracking-wider">Gudang Penyimpanan</div>
            <div class="text-base font-black text-blue-700 font-mono">{{ totalWarehousesCount }} Gudang</div>
          </div>
        </div>

        <div class="bg-white p-3.5 rounded-xl border border-slate-300 shadow-2xs flex items-center gap-3">
          <div class="w-10 h-10 rounded-lg bg-indigo-50 border border-indigo-200 flex items-center justify-center shrink-0">
            <ShieldCheck class="w-5 h-5 text-indigo-600" />
          </div>
          <div>
            <div class="text-[11px] font-bold text-slate-500 uppercase tracking-wider">Supervisor Terpetakan</div>
            <div class="text-base font-black text-indigo-700 font-mono">{{ totalSupervisorsAssigned }} Akun</div>
          </div>
        </div>
      </div>

      <!-- 3. TABLE OUTLETS -->
      <div class="flex-1 px-6 pb-6 overflow-hidden flex flex-col min-h-0">
        <div class="bg-white rounded-xl border border-slate-300 shadow-2xs flex-1 flex flex-col overflow-hidden">
          <div class="flex-1 overflow-y-auto">
            <table class="w-full text-left border-collapse">
              <thead class="sticky top-0 bg-slate-100 border-b border-slate-300 text-[11px] font-black text-slate-700 uppercase tracking-wider z-10">
                <tr>
                  <th class="py-3 px-4">Kode & Nama Lokasi</th>
                  <th class="py-3 px-4">Tipe</th>
                  <th class="py-3 px-4">Alamat & Telepon</th>
                  <th class="py-3 px-4">Penanggung Jawab</th>
                  <th class="py-3 px-4">Gudang Suplai</th>
                  <th class="py-3 px-4">Supervisor Otorisasi</th>
                  <th class="py-3 px-4 text-center">Status</th>
                  <th class="py-3 px-4 text-center">Aksi</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-slate-200 text-xs text-slate-800">
                <tr
                  v-for="item in filteredOutlets"
                  :key="item.id"
                  class="hover:bg-amber-50/40 transition group"
                >
                  <!-- Kode & Nama Outlet -->
                  <td class="py-3.5 px-4">
                    <div class="font-mono font-bold text-amber-800 text-[11px]">{{ item.code }}</div>
                    <div class="font-extrabold text-slate-900 text-sm mt-0.5">{{ item.name }}</div>
                  </td>

                  <!-- Tipe Lokasi (Badge Toko vs Gudang) -->
                  <td class="py-3.5 px-4">
                    <span
                      v-if="item.is_warehouse"
                      class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-blue-50 border border-blue-200 text-blue-900 font-bold text-xs"
                    >
                      <Warehouse class="w-3.5 h-3.5 text-blue-600 shrink-0" />
                      <span>Gudang (Warehouse)</span>
                    </span>
                    <span
                      v-else
                      class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-amber-50 border border-amber-200 text-amber-900 font-bold text-xs"
                    >
                      <Building2 class="w-3.5 h-3.5 text-amber-600 shrink-0" />
                      <span>Toko / Gerai Ritel</span>
                    </span>
                  </td>

                  <!-- Alamat & Telepon -->
                  <td class="py-3.5 px-4 max-w-xs">
                    <div class="flex items-start gap-1 text-slate-600">
                      <MapPin class="w-3.5 h-3.5 text-slate-400 shrink-0 mt-0.5" />
                      <span class="line-clamp-2">{{ item.address || 'Alamat belum diatur' }}</span>
                    </div>
                    <div v-if="item.phone" class="flex items-center gap-1 text-slate-500 font-mono text-[11px] mt-1">
                      <Phone class="w-3 h-3 text-slate-400 shrink-0" />
                      <span>{{ item.phone }}</span>
                    </div>
                  </td>

                  <!-- Store Manager / Penanggung Jawab -->
                  <td class="py-3.5 px-4">
                    <div v-if="item.manager_name" class="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-indigo-50 border border-indigo-200 text-indigo-900 font-bold text-xs">
                      <UserCheck class="w-3.5 h-3.5 text-indigo-600" />
                      <span>{{ item.manager_name }}</span>
                    </div>
                    <span v-else class="text-slate-400 italic text-[11px]">Belum ditugaskan</span>
                  </td>

                  <!-- Gudang Suplai -->
                  <td class="py-3.5 px-4">
                    <div v-if="item.is_warehouse" class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-md bg-slate-100 border border-slate-300 text-slate-600 text-[11px] font-mono font-bold">
                      <span>Pusat Inventori Fisik</span>
                    </div>
                    <div v-else-if="item.default_warehouse_name" class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-md bg-emerald-50 border border-emerald-200 text-emerald-800 text-[11px] font-mono font-bold">
                      <Warehouse class="w-3.5 h-3.5 text-emerald-600" />
                      <span>{{ item.default_warehouse_name }}</span>
                    </div>
                    <span v-else class="text-slate-400 italic text-[11px]">Belum disetting</span>
                  </td>

                  <!-- Supervisor Penanggung Jawab -->
                  <td class="py-3.5 px-4 max-w-xs">
                    <div v-if="item.supervisors && item.supervisors.length > 0" class="flex flex-wrap gap-1">
                      <span
                        v-for="spv in item.supervisors"
                        :key="spv.id"
                        class="inline-flex items-center gap-1 px-2 py-0.5 rounded-md bg-amber-50 border border-amber-200 text-amber-900 text-[11px] font-bold"
                        :title="`Role: ${spv.role_name}`"
                      >
                        <ShieldCheck class="w-3 h-3 text-amber-600 shrink-0" />
                        <span>{{ spv.full_name }}</span>
                      </span>
                    </div>
                    <span v-else class="text-slate-400 italic text-[11px]">Belum ada supervisor</span>
                  </td>

                  <!-- Status -->
                  <td class="py-3.5 px-4 text-center">
                    <span
                      v-if="item.is_active"
                      class="inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-[11px] font-bold bg-emerald-100 text-emerald-800 border border-emerald-300"
                    >
                      <span class="w-1.5 h-1.5 rounded-full bg-emerald-600"></span>
                      <span>Aktif</span>
                    </span>
                    <span
                      v-else
                      class="inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-[11px] font-bold bg-slate-100 text-slate-600 border border-slate-300"
                    >
                      <span class="w-1.5 h-1.5 rounded-full bg-slate-400"></span>
                      <span>Nonaktif</span>
                    </span>
                  </td>

                  <!-- Aksi -->
                  <td class="py-3.5 px-4 text-center">
                    <div class="flex items-center justify-center gap-1.5">
                      <button
                        @click="openEditModal(item)"
                        class="p-1.5 rounded-lg border border-slate-300 bg-white hover:bg-amber-50 text-slate-700 hover:text-amber-800 transition cursor-pointer"
                        title="Edit Data Lokasi"
                      >
                        <Edit2 class="w-3.5 h-3.5" />
                      </button>
                      <button
                        @click="openDeleteModal(item)"
                        class="p-1.5 rounded-lg border border-slate-300 bg-white hover:bg-rose-50 text-slate-700 hover:text-rose-700 transition cursor-pointer"
                        title="Hapus Lokasi"
                      >
                        <Trash2 class="w-3.5 h-3.5" />
                      </button>
                    </div>
                  </td>
                </tr>

                <tr v-if="filteredOutlets.length === 0">
                  <td colspan="8" class="py-12 text-center text-slate-400 text-xs">
                    <div class="w-12 h-12 mx-auto mb-2 text-slate-300">
                      <Building2 class="w-full h-full" />
                    </div>
                    <span>Tidak ada data lokasi yang ditemukan.</span>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>

      <!-- 4. MODAL TAMBAH / EDIT OUTLET & GUDANG -->
      <div
        v-if="showModal"
        class="fixed inset-0 z-50 bg-slate-900/50 backdrop-blur-xs flex items-center justify-center p-4 animate-fadeIn"
      >
        <div class="bg-white rounded-2xl shadow-2xl border border-slate-300 w-full max-w-xl overflow-hidden flex flex-col text-slate-900 max-h-[90vh]">
          <!-- Modal Header -->
          <div class="bg-slate-100 px-6 py-4 border-b border-slate-300 flex items-center justify-between shrink-0">
            <div class="flex items-center gap-2.5">
              <div class="w-8 h-8 rounded-lg bg-amber-100 border border-amber-300 flex items-center justify-center text-amber-800 font-bold">
                <Building2 v-if="!form.is_warehouse" class="w-4 h-4" />
                <Warehouse v-else class="w-4 h-4 text-blue-700" />
              </div>
              <div>
                <h3 class="font-black text-sm text-slate-900 leading-none">
                  {{ isEditing ? 'Edit Data Lokasi Master' : (form.is_warehouse ? 'Tambah Gudang (Warehouse) Baru' : 'Tambah Cabang Outlet Baru') }}
                </h3>
                <p class="text-[11px] text-slate-500 mt-0.5">
                  Konfigurasi identitas gerai / gudang, penugasan Manager, dan alokasi suplai stok
                </p>
              </div>
            </div>
            <button
              @click="showModal = false"
              class="text-slate-400 hover:text-slate-700 p-1.5 rounded-lg hover:bg-slate-200 transition cursor-pointer"
            >
              <X class="w-4 h-4" />
            </button>
          </div>

          <!-- Modal Body -->
          <div class="p-6 overflow-y-auto space-y-4 text-xs">
            <div v-if="formError" class="p-3 bg-rose-50 border border-rose-200 rounded-xl text-rose-700 flex items-center gap-2">
              <AlertCircle class="w-4 h-4 shrink-0" />
              <span>{{ formError }}</span>
            </div>

            <!-- Tipe Lokasi: Toko atau Gudang -->
            <div class="bg-slate-50 p-3 rounded-xl border border-slate-200">
              <label class="block font-bold text-slate-800 mb-2">Tipe Lokasi *</label>
              <div class="grid grid-cols-2 gap-2.5">
                <button
                  type="button"
                  @click="form.is_warehouse = false"
                  :class="!form.is_warehouse ? 'bg-amber-600 text-white border-amber-700 shadow-xs' : 'bg-white text-slate-700 border-slate-300 hover:bg-slate-100'"
                  class="flex items-center gap-2.5 p-2.5 rounded-lg border text-left transition cursor-pointer"
                >
                  <Building2 class="w-5 h-5 shrink-0" :class="!form.is_warehouse ? 'text-white' : 'text-amber-600'" />
                  <div>
                    <div class="font-black text-xs leading-none">Cabang / Toko Ritel</div>
                    <div class="text-[10px] opacity-80 mt-0.5">Gerai kasir & transaksi penjualan</div>
                  </div>
                </button>

                <button
                  type="button"
                  @click="form.is_warehouse = true"
                  :class="form.is_warehouse ? 'bg-blue-600 text-white border-blue-700 shadow-xs' : 'bg-white text-slate-700 border-slate-300 hover:bg-slate-100'"
                  class="flex items-center gap-2.5 p-2.5 rounded-lg border text-left transition cursor-pointer"
                >
                  <Warehouse class="w-5 h-5 shrink-0" :class="form.is_warehouse ? 'text-white' : 'text-blue-600'" />
                  <div>
                    <div class="font-black text-xs leading-none">Gudang (Warehouse)</div>
                    <div class="text-[10px] opacity-80 mt-0.5">Pusat logistik & penyedia stok</div>
                  </div>
                </button>
              </div>
            </div>

            <!-- Kode & Nama Lokasi -->
            <div class="grid grid-cols-3 gap-3">
              <div>
                <label class="block font-bold text-slate-700 mb-1">Kode Lokasi *</label>
                <input
                  v-model="form.code"
                  type="text"
                  :placeholder="form.is_warehouse ? 'WH-001' : 'OUT-001'"
                  class="w-full px-3 py-2 rounded-lg border border-slate-300 font-mono text-xs uppercase focus:ring-2 focus:ring-amber-500 focus:outline-none"
                />
              </div>
              <div class="col-span-2">
                <label class="block font-bold text-slate-700 mb-1">Nama {{ form.is_warehouse ? 'Gudang' : 'Toko / Gerai' }} *</label>
                <input
                  v-model="form.name"
                  type="text"
                  :placeholder="form.is_warehouse ? 'Contoh: Gudang Distribusi Cikarang' : 'Contoh: Notane Retail Flagship Tebet'"
                  class="w-full px-3 py-2 rounded-lg border border-slate-300 text-xs focus:ring-2 focus:ring-amber-500 focus:outline-none"
                />
              </div>
            </div>

            <!-- Alamat & Telepon -->
            <div>
              <label class="block font-bold text-slate-700 mb-1">Alamat Lengkap</label>
              <textarea
                v-model="form.address"
                rows="2"
                placeholder="Jl. Raya Utama No. 123, Jakarta Selatan..."
                class="w-full px-3 py-2 rounded-lg border border-slate-300 text-xs focus:ring-2 focus:ring-amber-500 focus:outline-none"
              ></textarea>
            </div>

            <div>
              <label class="block font-bold text-slate-700 mb-1">Nomor Telepon</label>
              <input
                v-model="form.phone"
                type="text"
                placeholder="021-5551234 / 0812..."
                class="w-full px-3 py-2 rounded-lg border border-slate-300 font-mono text-xs focus:ring-2 focus:ring-amber-500 focus:outline-none"
              />
            </div>

            <!-- Gudang Penyuplai Stok (Hanya untuk Toko Ritel) -->
            <div v-if="!form.is_warehouse" class="pt-2 border-t border-slate-200">
              <label class="block font-bold text-slate-800 mb-1">
                Gudang Penyuplai Stok (Warehouse)
                <span class="text-[10px] text-slate-500 font-normal ml-1">
                  (Pilih gudang yang menyuplai stok toko ini)
                </span>
              </label>
              <select
                v-model="form.default_warehouse_id"
                class="w-full px-3 py-2 rounded-lg border border-slate-300 bg-white text-xs focus:ring-2 focus:ring-amber-500 focus:outline-none"
              >
                <option value="">-- Pilih Gudang Penyuplai --</option>
                <option v-for="w in warehouseOptions" :key="w.id" :value="w.id">
                  {{ w.code }} - {{ w.name }}
                </option>
              </select>
            </div>

            <div v-else class="p-3 bg-blue-50 border border-blue-200 rounded-xl text-blue-900 text-xs flex items-center gap-2">
              <Warehouse class="w-4 h-4 text-blue-600 shrink-0" />
              <span>Lokasi ini berstatus sebagai <strong>Gudang Fisik</strong> dan dapat dipilih sebagai penyuplai stok untuk berbagai toko ritel lainnya.</span>
            </div>

            <!-- Store Manager / Warehouse Manager -->
            <div class="pt-2 border-t border-slate-200">
              <label class="block font-bold text-slate-800 mb-1">
                Penanggung Jawab (Store Manager / Warehouse Head)
                <span class="text-[10px] text-slate-500 font-normal ml-1">
                  (Akun berizin <code class="font-mono bg-slate-100 px-1 py-0.5 rounded text-indigo-700">auth:store_manager</code>)
                </span>
              </label>
              <select
                v-model="form.manager_id"
                class="w-full px-3 py-2 rounded-lg border border-slate-300 bg-white text-xs focus:ring-2 focus:ring-indigo-500 focus:outline-none"
              >
                <option value="">-- Belum Ditugaskan --</option>
                <option v-for="m in storeManagers" :key="m.id" :value="m.id">
                  {{ m.full_name }} ({{ m.role_name }}) - @{{ m.username }}
                </option>
              </select>
            </div>

            <!-- Supervisor Otorisasi (Berbasis Permission auth:supervisor) -->
            <div class="pt-2 border-t border-slate-200">
              <div class="flex items-center justify-between mb-1.5">
                <label class="block font-bold text-slate-800">
                  Supervisor Penanggung Jawab
                  <span class="text-[10px] text-slate-500 font-normal ml-1">
                    (Akun berizin <code class="font-mono bg-slate-100 px-1 py-0.5 rounded text-amber-700">auth:supervisor</code>)
                  </span>
                </label>
                <span class="text-[10px] font-mono text-amber-700 font-bold">
                  {{ form.supervisor_ids.length }} Terpilih
                </span>
              </div>

              <div class="bg-slate-50 border border-slate-300 rounded-xl p-3 max-h-40 overflow-y-auto space-y-1.5">
                <div
                  v-for="spv in supervisors"
                  :key="spv.id"
                  @click="toggleSupervisorSelection(spv.id)"
                  class="flex items-center justify-between p-2 rounded-lg border text-xs cursor-pointer transition select-none"
                  :class="
                    form.supervisor_ids.includes(spv.id)
                      ? 'bg-amber-100/70 border-amber-400 text-amber-950 font-bold'
                      : 'bg-white border-slate-200 text-slate-700 hover:bg-slate-100'
                  "
                >
                  <div class="flex items-center gap-2">
                    <input
                      type="checkbox"
                      :checked="form.supervisor_ids.includes(spv.id)"
                      class="rounded text-amber-600 pointer-events-none"
                    />
                    <div>
                      <div class="leading-none">{{ spv.full_name }}</div>
                      <div class="text-[10px] text-slate-500 font-mono mt-0.5">@{{ spv.username }} • Role: {{ spv.role_name }}</div>
                    </div>
                  </div>
                  <ShieldCheck class="w-4 h-4 text-amber-600" :class="{ 'opacity-0': !form.supervisor_ids.includes(spv.id) }" />
                </div>
                <div v-if="supervisors.length === 0" class="text-center py-4 text-slate-400 text-xs italic">
                  Belum ada user yang memiliki izin supervisor.
                </div>
              </div>
            </div>

            <!-- Status Aktif Checkbox -->
            <div class="pt-2 border-t border-slate-200 flex items-center gap-2">
              <input
                id="activeSwitch"
                v-model="form.is_active"
                type="checkbox"
                class="w-4 h-4 rounded text-amber-600 focus:ring-amber-500"
              />
              <label for="activeSwitch" class="font-bold text-slate-800 cursor-pointer">
                Lokasi Berstatus Aktif & Beroperasi
              </label>
            </div>
          </div>

          <!-- Modal Footer -->
          <div class="bg-slate-100 px-6 py-3.5 border-t border-slate-300 flex items-center justify-end gap-2 shrink-0">
            <button
              type="button"
              @click="showModal = false"
              :disabled="isSubmitting"
              class="px-4 py-2 rounded-lg border border-slate-300 bg-white hover:bg-slate-50 text-slate-700 font-bold text-xs transition cursor-pointer"
            >
              Batal
            </button>
            <button
              type="button"
              @click="handleSave"
              :disabled="isSubmitting"
              class="px-4 py-2 rounded-lg bg-amber-600 hover:bg-amber-700 text-white font-bold text-xs flex items-center gap-2 shadow-xs transition active:scale-95 cursor-pointer disabled:opacity-50"
            >
              <RefreshCw v-if="isSubmitting" class="w-4 h-4 animate-spin" />
              <span>{{ isEditing ? 'Perbarui Data' : 'Simpan Lokasi' }}</span>
            </button>
          </div>
        </div>
      </div>

      <!-- 5. MODAL HAPUS OUTLET / GUDANG -->
      <div
        v-if="showDeleteModal"
        class="fixed inset-0 z-50 bg-slate-900/50 backdrop-blur-xs flex items-center justify-center p-4 animate-fadeIn"
      >
        <div class="bg-white rounded-2xl shadow-2xl border border-slate-300 w-full max-w-sm overflow-hidden p-6 text-slate-900">
          <div class="w-12 h-12 rounded-2xl bg-rose-50 border border-rose-200 flex items-center justify-center text-rose-600 mx-auto mb-3">
            <Trash2 class="w-6 h-6" />
          </div>
          <h3 class="text-base font-black text-center text-slate-900">Hapus Lokasi?</h3>
          <p class="text-xs text-slate-500 text-center mt-1">
            Apakah Anda yakin ingin menghapus <strong class="text-slate-900">{{ outletToDelete?.name }}</strong> ({{ outletToDelete?.code }})?
          </p>

          <div v-if="deleteError" class="mt-3 p-3 bg-rose-50 border border-rose-200 rounded-xl text-xs text-rose-700">
            {{ deleteError }}
          </div>

          <div class="flex items-center gap-2 mt-5">
            <button
              @click="showDeleteModal = false"
              :disabled="isDeleting"
              class="flex-1 py-2 rounded-xl border border-slate-300 bg-white hover:bg-slate-50 text-slate-700 font-bold text-xs transition cursor-pointer"
            >
              Batal
            </button>
            <button
              @click="confirmDelete"
              :disabled="isDeleting"
              class="flex-1 py-2 rounded-xl bg-rose-600 hover:bg-rose-700 text-white font-bold text-xs flex items-center justify-center gap-2 shadow-xs transition active:scale-95 cursor-pointer"
            >
              <RefreshCw v-if="isDeleting" class="w-4 h-4 animate-spin" />
              <span>Ya, Hapus</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  </AdminLayout>
</template>
