<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import AdminLayout from '@/layouts/AdminLayout.vue'
import PaginationBar from '@/components/common/PaginationBar.vue'
import {
  Ruler,
  Tags,
  Bookmark,
  Truck,
  Plus,
  Edit2,
  Trash2,
  Phone,
  Clock
} from 'lucide-vue-next'
import { api } from '@/services/api'
import { useAuthStore } from '@/stores/authStore'

const authStore = useAuthStore()
const canWrite = computed(() => authStore.hasPermission('master:write') || authStore.hasPermission('settings:manage'))

type ParamTab = 'units' | 'categories' | 'brands' | 'suppliers'
const activeTab = ref<ParamTab>('units')

// Data states
const units = ref<any[]>([])
const categories = ref<any[]>([])
const brands = ref<any[]>([])
const suppliers = ref<any[]>([])
const isLoading = ref<boolean>(false)

// Pagination states
const unitsPage = ref<number>(1)
const unitsPageSize = ref<number>(10)
const paginatedUnits = computed(() => {
  const start = (unitsPage.value - 1) * unitsPageSize.value
  return units.value.slice(start, start + unitsPageSize.value)
})

const categoriesPage = ref<number>(1)
const categoriesPageSize = ref<number>(10)
const paginatedCategories = computed(() => {
  const start = (categoriesPage.value - 1) * categoriesPageSize.value
  return categories.value.slice(start, start + categoriesPageSize.value)
})

const brandsPage = ref<number>(1)
const brandsPageSize = ref<number>(10)
const paginatedBrands = computed(() => {
  const start = (brandsPage.value - 1) * brandsPageSize.value
  return brands.value.slice(start, start + brandsPageSize.value)
})

const suppliersPage = ref<number>(1)
const suppliersPageSize = ref<number>(10)
const paginatedSuppliers = computed(() => {
  const start = (suppliersPage.value - 1) * suppliersPageSize.value
  return suppliers.value.slice(start, start + suppliersPageSize.value)
})

// Modals
const showModal = ref<boolean>(false)
const isEditing = ref<boolean>(false)
const currentEditId = ref<string | null>(null)

// Forms
const unitForm = ref({ code: '', name: '', is_fractional: false })
const categoryForm = ref({ code: '', name: '', parent_id: null as string | null })
const brandForm = ref({ code: '', name: '', description: '' })
const supplierForm = ref({
  code: '',
  name: '',
  contact_person: '',
  phone: '',
  address: '',
  term_of_payment_days: 0,
})

async function fetchAllParameters() {
  isLoading.value = true
  try {
    const [uRes, cRes, bRes, sRes] = await Promise.all([
      api.get('/units'),
      api.get('/categories'),
      api.get('/brands'),
      api.get('/suppliers'),
    ])
    if (uRes.data.success) units.value = uRes.data.data
    if (cRes.data.success) categories.value = cRes.data.data
    if (bRes.data.success) brands.value = bRes.data.data
    if (sRes.data.success) suppliers.value = sRes.data.data
  } catch (err) {
    console.error('Error fetching parameters:', err)
  } finally {
    isLoading.value = false
  }
}

function openCreateModal() {
  isEditing.value = false
  currentEditId.value = null

  if (activeTab.value === 'units') {
    unitForm.value = { code: '', name: '', is_fractional: false }
  } else if (activeTab.value === 'categories') {
    categoryForm.value = { code: '', name: '', parent_id: null }
  } else if (activeTab.value === 'brands') {
    brandForm.value = { code: '', name: '', description: '' }
  } else if (activeTab.value === 'suppliers') {
    supplierForm.value = {
      code: `SUP-${Date.now().toString().slice(-4)}`,
      name: '',
      contact_person: '',
      phone: '',
      address: '',
      term_of_payment_days: 30,
    }
  }
  showModal.value = true
}

function openEditModal(item: any) {
  isEditing.value = true
  currentEditId.value = item.id

  if (activeTab.value === 'units') {
    unitForm.value = { code: item.code, name: item.name, is_fractional: item.is_fractional }
  } else if (activeTab.value === 'categories') {
    categoryForm.value = { code: item.code, name: item.name, parent_id: item.parent_id }
  } else if (activeTab.value === 'brands') {
    brandForm.value = { code: item.code, name: item.name, description: item.description || '' }
  } else if (activeTab.value === 'suppliers') {
    supplierForm.value = {
      code: item.code,
      name: item.name,
      contact_person: item.contact_person || '',
      phone: item.phone || '',
      address: item.address || '',
      term_of_payment_days: Number(item.term_of_payment_days || 0),
    }
  }
  showModal.value = true
}

async function handleSave() {
  if (!canWrite.value) {
    alert('Akses ditolak: Anda hanya memiliki hak baca master data (master:read)')
    return
  }
  try {
    if (activeTab.value === 'units') {
      if (isEditing.value && currentEditId.value) {
        await api.put(`/units/${currentEditId.value}`, unitForm.value)
      } else {
        await api.post('/units', unitForm.value)
      }
    } else if (activeTab.value === 'categories') {
      if (isEditing.value && currentEditId.value) {
        await api.put(`/categories/${currentEditId.value}`, categoryForm.value)
      } else {
        await api.post('/categories', categoryForm.value)
      }
    } else if (activeTab.value === 'brands') {
      if (isEditing.value && currentEditId.value) {
        await api.put(`/brands/${currentEditId.value}`, brandForm.value)
      } else {
        await api.post('/brands', brandForm.value)
      }
    } else if (activeTab.value === 'suppliers') {
      if (isEditing.value && currentEditId.value) {
        await api.put(`/suppliers/${currentEditId.value}`, supplierForm.value)
      } else {
        await api.post('/suppliers', supplierForm.value)
      }
    }

    showModal.value = false
    fetchAllParameters()
  } catch (err: any) {
    alert(err.response?.data?.message || 'Gagal menyimpan data')
  }
}

async function handleDelete(id: string, name: string) {
  if (!canWrite.value) {
    alert('Akses ditolak: Anda hanya memiliki hak baca master data (master:read)')
    return
  }
  const tabName =
    activeTab.value === 'units'
      ? 'satuan'
      : activeTab.value === 'categories'
      ? 'kategori'
      : activeTab.value === 'brands'
      ? 'merek'
      : 'pemasok'

  if (!confirm(`Hapus / nonaktifkan ${tabName} "${name}"?`)) return

  try {
    const endpoint = `/${activeTab.value}/${id}`
    await api.delete(endpoint)
    fetchAllParameters()
  } catch (err: any) {
    alert(err.response?.data?.message || `Gagal menghapus ${tabName}`)
  }
}

onMounted(() => {
  fetchAllParameters()
})
</script>

<template>
  <AdminLayout>
    <div class="p-8 space-y-6">
      <!-- Page Header -->
      <div class="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h2 class="text-xl font-bold text-slate-900 flex items-center gap-2.5">
            <Icon icon="streamline-ultimate-color:tags-settings" class="w-6 h-6 drop-shadow-xs" />
            <span>Master Parameter Retail</span>
          </h2>
          <p class="text-xs text-slate-600">
            Kelola master satuan (UoM), kategori barang, merek / prinsipal, dan pemasok (distributor).
          </p>
        </div>

        <button
          v-if="canWrite"
          @click="openCreateModal"
          class="bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-xs px-4 py-2.5 rounded-lg shadow-sm border border-emerald-700 flex items-center gap-2 transition active:scale-95"
        >
          <Plus class="w-4 h-4" />
          <span>Tambah {{ activeTab === 'units' ? 'Satuan' : activeTab === 'categories' ? 'Kategori' : activeTab === 'brands' ? 'Merek' : 'Pemasok' }}</span>
        </button>
      </div>

      <!-- Tab Buttons -->
      <div class="flex border-b border-slate-300 gap-6 text-sm font-semibold">
        <button
          @click="activeTab = 'units'"
          class="pb-3 flex items-center gap-2 transition border-b-2"
          :class="activeTab === 'units' ? 'border-emerald-600 text-emerald-800 font-bold' : 'border-transparent text-slate-600 hover:text-slate-900 font-medium'"
        >
          <Ruler class="w-4 h-4" />
          <span>Satuan UoM ({{ units.length }})</span>
        </button>

        <button
          @click="activeTab = 'categories'"
          class="pb-3 flex items-center gap-2 transition border-b-2"
          :class="activeTab === 'categories' ? 'border-emerald-600 text-emerald-800 font-bold' : 'border-transparent text-slate-600 hover:text-slate-900 font-medium'"
        >
          <Tags class="w-4 h-4" />
          <span>Kategori ({{ categories.length }})</span>
        </button>

        <button
          @click="activeTab = 'brands'"
          class="pb-3 flex items-center gap-2 transition border-b-2"
          :class="activeTab === 'brands' ? 'border-emerald-600 text-emerald-800 font-bold' : 'border-transparent text-slate-600 hover:text-slate-900 font-medium'"
        >
          <Bookmark class="w-4 h-4" />
          <span>Merek / Prinsipal ({{ brands.length }})</span>
        </button>

        <button
          @click="activeTab = 'suppliers'"
          class="pb-3 flex items-center gap-2 transition border-b-2"
          :class="activeTab === 'suppliers' ? 'border-emerald-600 text-emerald-800 font-bold' : 'border-transparent text-slate-600 hover:text-slate-900 font-medium'"
        >
          <Truck class="w-4 h-4" />
          <span>Pemasok / Distributor ({{ suppliers.length }})</span>
        </button>
      </div>

      <!-- TAB 1: UNITS -->
      <div v-if="activeTab === 'units'" class="bg-white border border-slate-300 rounded-xl overflow-hidden shadow-sm">
        <table class="w-full text-left text-xs border-collapse">
          <thead class="bg-slate-100 border-b border-slate-300 text-slate-700 font-mono uppercase tracking-wider text-[11px]">
            <tr>
              <th class="p-4">Kode Satuan</th>
              <th class="p-4">Nama Satuan</th>
              <th class="p-4">Tipe Kuantitas</th>
              <th v-if="canWrite" class="p-4 text-center">Aksi</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-slate-200 font-sans">
            <tr v-for="u in paginatedUnits" :key="u.id" class="hover:bg-emerald-50/40 transition">
              <td class="p-4 font-mono font-bold text-emerald-700">{{ u.code }}</td>
              <td class="p-4 font-semibold text-slate-900">{{ u.name }}</td>
              <td class="p-4">
                <span
                  class="px-2.5 py-1 rounded-full text-[10px] font-bold border"
                  :class="u.is_fractional ? 'bg-amber-50 text-amber-800 border-amber-300' : 'bg-slate-100 text-slate-700 border-slate-300'"
                >
                  {{ u.is_fractional ? 'Pecahan / Desimal (Timbangan)' : 'Bilangan Bulat (Pieces)' }}
                </span>
              </td>
              <td v-if="canWrite" class="p-4 text-center">
                <div class="flex items-center justify-center gap-2">
                  <button @click="openEditModal(u)" class="p-1.5 rounded-lg bg-slate-100 hover:bg-slate-200 border border-slate-300 text-blue-700 transition" title="Edit">
                    <Edit2 class="w-3.5 h-3.5" />
                  </button>
                  <button @click="handleDelete(u.id, u.name)" class="p-1.5 rounded-lg bg-slate-100 hover:bg-slate-200 border border-slate-300 text-rose-700 transition" title="Hapus">
                    <Trash2 class="w-3.5 h-3.5" />
                  </button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
        <PaginationBar
          v-model:currentPage="unitsPage"
          v-model:pageSize="unitsPageSize"
          :totalItems="units.length"
        />
      </div>

      <!-- TAB 2: CATEGORIES -->
      <div v-if="activeTab === 'categories'" class="bg-white border border-slate-300 rounded-xl overflow-hidden shadow-sm">
        <table class="w-full text-left text-xs border-collapse">
          <thead class="bg-slate-100 border-b border-slate-300 text-slate-700 font-mono uppercase tracking-wider text-[11px]">
            <tr>
              <th class="p-4">Kode Kategori</th>
              <th class="p-4">Nama Kategori</th>
              <th v-if="canWrite" class="p-4 text-center">Aksi</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-slate-200 font-sans">
            <tr v-for="c in paginatedCategories" :key="c.id" class="hover:bg-emerald-50/40 transition">
              <td class="p-4 font-mono font-bold text-emerald-700">{{ c.code }}</td>
              <td class="p-4 font-semibold text-slate-900">{{ c.name }}</td>
              <td v-if="canWrite" class="p-4 text-center">
                <div class="flex items-center justify-center gap-2">
                  <button @click="openEditModal(c)" class="p-1.5 rounded-lg bg-slate-100 hover:bg-slate-200 border border-slate-300 text-blue-700 transition" title="Edit">
                    <Edit2 class="w-3.5 h-3.5" />
                  </button>
                  <button @click="handleDelete(c.id, c.name)" class="p-1.5 rounded-lg bg-slate-100 hover:bg-slate-200 border border-slate-300 text-rose-700 transition" title="Hapus">
                    <Trash2 class="w-3.5 h-3.5" />
                  </button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
        <PaginationBar
          v-model:currentPage="categoriesPage"
          v-model:pageSize="categoriesPageSize"
          :totalItems="categories.length"
        />
      </div>

      <!-- TAB 3: BRANDS -->
      <div v-if="activeTab === 'brands'" class="bg-white border border-slate-300 rounded-xl overflow-hidden shadow-sm">
        <table class="w-full text-left text-xs border-collapse">
          <thead class="bg-slate-100 border-b border-slate-300 text-slate-700 font-mono uppercase tracking-wider text-[11px]">
            <tr>
              <th class="p-4">Kode Merek</th>
              <th class="p-4">Nama Brand / Prinsipal</th>
              <th class="p-4">Deskripsi</th>
              <th v-if="canWrite" class="p-4 text-center">Aksi</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-slate-200 font-sans">
            <tr v-for="b in paginatedBrands" :key="b.id" class="hover:bg-emerald-50/40 transition">
              <td class="p-4 font-mono font-bold text-emerald-700">{{ b.code }}</td>
              <td class="p-4 font-semibold text-slate-900">{{ b.name }}</td>
              <td class="p-4 text-slate-600">{{ b.description || '-' }}</td>
              <td v-if="canWrite" class="p-4 text-center">
                <div class="flex items-center justify-center gap-2">
                  <button @click="openEditModal(b)" class="p-1.5 rounded-lg bg-slate-100 hover:bg-slate-200 border border-slate-300 text-blue-700 transition" title="Edit">
                    <Edit2 class="w-3.5 h-3.5" />
                  </button>
                  <button @click="handleDelete(b.id, b.name)" class="p-1.5 rounded-lg bg-slate-100 hover:bg-slate-200 border border-slate-300 text-rose-700 transition" title="Hapus">
                    <Trash2 class="w-3.5 h-3.5" />
                  </button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
        <PaginationBar
          v-model:currentPage="brandsPage"
          v-model:pageSize="brandsPageSize"
          :totalItems="brands.length"
        />
      </div>

      <!-- TAB 4: SUPPLIERS -->
      <div v-if="activeTab === 'suppliers'" class="bg-white border border-slate-300 rounded-xl overflow-hidden shadow-sm">
        <table class="w-full text-left text-xs border-collapse">
          <thead class="bg-slate-100 border-b border-slate-300 text-slate-700 font-mono uppercase tracking-wider text-[11px]">
            <tr>
              <th class="p-4">Kode</th>
              <th class="p-4">Nama Distributor / PT</th>
              <th class="p-4">Kontak Person & Telepon</th>
              <th class="p-4">Alamat Gudang</th>
              <th class="p-4 text-center">Tempo Bayar</th>
              <th v-if="canWrite" class="p-4 text-center">Aksi</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-slate-200 font-sans">
            <tr v-for="s in paginatedSuppliers" :key="s.id" class="hover:bg-emerald-50/40 transition">
              <td class="p-4 font-mono font-bold text-emerald-700">{{ s.code }}</td>
              <td class="p-4 font-semibold text-slate-900">{{ s.name }}</td>
              <td class="p-4 space-y-0.5">
                <div class="text-slate-800 font-medium">{{ s.contact_person || '-' }}</div>
                <div class="flex items-center gap-1.5 text-slate-600 text-[11px]">
                  <Phone class="w-3 h-3" />
                  <span>{{ s.phone || '-' }}</span>
                </div>
              </td>
              <td class="p-4 text-slate-600 max-w-xs truncate">{{ s.address || '-' }}</td>
              <td class="p-4 text-center">
                <span class="inline-flex items-center gap-1 font-mono text-[11px] font-bold text-slate-700 bg-slate-100 border border-slate-300 px-2 py-0.5 rounded-md">
                  <Clock class="w-3 h-3 text-slate-500" />
                  <span>{{ s.term_of_payment_days }} Hari</span>
                </span>
              </td>
              <td v-if="canWrite" class="p-4 text-center">
                <div class="flex items-center justify-center gap-2">
                  <button @click="openEditModal(s)" class="p-1.5 rounded-lg bg-slate-100 hover:bg-slate-200 border border-slate-300 text-blue-700 transition" title="Edit">
                    <Edit2 class="w-3.5 h-3.5" />
                  </button>
                  <button @click="handleDelete(s.id, s.name)" class="p-1.5 rounded-lg bg-slate-100 hover:bg-slate-200 border border-slate-300 text-rose-700 transition" title="Hapus">
                    <Trash2 class="w-3.5 h-3.5" />
                  </button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>
        <PaginationBar
          v-model:currentPage="suppliersPage"
          v-model:pageSize="suppliersPageSize"
          :totalItems="suppliers.length"
        />
      </div>
    </div>

    <!-- Modal Form (Dynamic based on active tab) -->
    <div
      v-if="showModal"
      class="fixed inset-0 z-50 bg-slate-900/50 backdrop-blur-xs flex items-center justify-center p-4 select-none"
    >
      <div class="bg-white border border-slate-300 w-full max-w-md rounded-xl p-6 space-y-4 shadow-2xl text-slate-900">
        <h3 class="font-bold text-base text-slate-900 border-b border-slate-200 pb-3">
          {{ isEditing ? 'Edit' : 'Tambah' }}
          {{ activeTab === 'units' ? 'Satuan Barang' : activeTab === 'categories' ? 'Kategori Produk' : activeTab === 'brands' ? 'Merek / Prinsipal' : 'Pemasok / Distributor' }}
        </h3>

        <!-- Form Unit -->
        <div v-if="activeTab === 'units'" class="space-y-3 text-xs">
          <div>
            <label class="block text-slate-700 font-bold mb-1">Kode Satuan *</label>
            <input v-model="unitForm.code" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono uppercase focus:bg-white focus:border-emerald-600 focus:outline-none" placeholder="e.g. PCS / DUS / KG" />
          </div>
          <div>
            <label class="block text-slate-700 font-bold mb-1">Nama Satuan *</label>
            <input v-model="unitForm.name" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 focus:bg-white focus:border-emerald-600 focus:outline-none" placeholder="e.g. Pieces / Karton / Kilogram" />
          </div>
          <div class="flex items-center gap-2 pt-2">
            <input v-model="unitForm.is_fractional" type="checkbox" id="frac_check" class="w-4 h-4 rounded text-emerald-600 bg-white border-slate-300 focus:ring-emerald-500" />
            <label for="frac_check" class="text-slate-800 font-medium">Satuan Mendukung Angka Desimal (Timbangan)</label>
          </div>
        </div>

        <!-- Form Category -->
        <div v-if="activeTab === 'categories'" class="space-y-3 text-xs">
          <div>
            <label class="block text-slate-700 font-bold mb-1">Kode Kategori *</label>
            <input v-model="categoryForm.code" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono uppercase focus:bg-white focus:border-emerald-600 focus:outline-none" placeholder="e.g. CAT-SNACK" />
          </div>
          <div>
            <label class="block text-slate-700 font-bold mb-1">Nama Kategori *</label>
            <input v-model="categoryForm.name" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 focus:bg-white focus:border-emerald-600 focus:outline-none" placeholder="e.g. Makanan Ringan & Biskuit" />
          </div>
        </div>

        <!-- Form Brand -->
        <div v-if="activeTab === 'brands'" class="space-y-3 text-xs">
          <div>
            <label class="block text-slate-700 font-bold mb-1">Kode Merek *</label>
            <input v-model="brandForm.code" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono uppercase focus:bg-white focus:border-emerald-600 focus:outline-none" placeholder="e.g. BRD-UNILEVER" />
          </div>
          <div>
            <label class="block text-slate-700 font-bold mb-1">Nama Merek *</label>
            <input v-model="brandForm.name" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 focus:bg-white focus:border-emerald-600 focus:outline-none" placeholder="e.g. Unilever Indonesia" />
          </div>
          <div>
            <label class="block text-slate-700 font-bold mb-1">Deskripsi</label>
            <input v-model="brandForm.description" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 focus:bg-white focus:border-emerald-600 focus:outline-none" placeholder="e.g. Produsen FMCG & Kebersihan" />
          </div>
        </div>

        <!-- Form Supplier -->
        <div v-if="activeTab === 'suppliers'" class="space-y-3 text-xs">
          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="block text-slate-700 font-bold mb-1">Kode Pemasok *</label>
              <input v-model="supplierForm.code" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono focus:bg-white focus:border-emerald-600 focus:outline-none" placeholder="SUP-004" />
            </div>
            <div>
              <label class="block text-slate-700 font-bold mb-1">Tempo Bayar (Hari)</label>
              <input v-model.number="supplierForm.term_of_payment_days" type="number" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono focus:bg-white focus:border-emerald-600 focus:outline-none" placeholder="30" />
            </div>
          </div>
          <div>
            <label class="block text-slate-700 font-bold mb-1">Nama Perusahaan / Distributor *</label>
            <input v-model="supplierForm.name" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 focus:bg-white focus:border-emerald-600 focus:outline-none" placeholder="PT Indomarco Distribusi" />
          </div>
          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="block text-slate-700 font-bold mb-1">Kontak Person</label>
              <input v-model="supplierForm.contact_person" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 focus:bg-white focus:border-emerald-600 focus:outline-none" placeholder="Budi Santoso" />
            </div>
            <div>
              <label class="block text-slate-700 font-bold mb-1">No Telepon</label>
              <input v-model="supplierForm.phone" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono focus:bg-white focus:border-emerald-600 focus:outline-none" placeholder="021-555666" />
            </div>
          </div>
          <div>
            <label class="block text-slate-700 font-bold mb-1">Alamat Kantor / Gudang</label>
            <textarea v-model="supplierForm.address" rows="2" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 focus:bg-white focus:border-emerald-600 focus:outline-none" placeholder="Jl. Industri Raya No. 88"></textarea>
          </div>
        </div>

        <div class="flex justify-end gap-2 pt-4 border-t border-slate-200">
          <button @click="showModal = false" class="bg-slate-100 hover:bg-slate-200 border border-slate-300 text-slate-700 font-bold px-4 py-2 rounded-lg text-xs transition">
            Batal
          </button>
          <button @click="handleSave" class="bg-emerald-600 hover:bg-emerald-700 text-white font-bold px-5 py-2 rounded-lg text-xs shadow-sm border border-emerald-700 transition active:scale-95">
            Simpan
          </button>
        </div>
      </div>
    </div>
  </AdminLayout>
</template>
