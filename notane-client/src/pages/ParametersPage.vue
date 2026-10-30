<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import { useRoute } from 'vue-router'
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
  Clock,
  Calculator,
  Save,
  CheckCircle2,
  Layers,
  Sparkles,
  TrendingUp,
  Percent,
  Info,
  Fingerprint,
  Camera,
  ShieldCheck,
  Folder,
  FolderOpen,
  FolderArchive,
  RotateCcw,
  HardDrive,
} from 'lucide-vue-next'
import { Icon } from '@iconify/vue'
import type { BiometricSettings } from '@/types'
import { api } from '@/services/api'
import { useAuthStore } from '@/stores/authStore'
import { useDialog } from '@/composables/useDialog'
import { useDirectorySettingsStore } from '@/stores/directorySettingsStore'

const route = useRoute()
const authStore = useAuthStore()
const dialog = useDialog()
const dirStore = useDirectorySettingsStore()
const canWrite = computed(() => authStore.hasPermission('master:write') || authStore.hasPermission('settings:manage'))

type ParamTab = 'units' | 'categories' | 'brands' | 'suppliers' | 'costing' | 'biometric' | 'storage'
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
    dialog.alert('Akses ditolak: Anda hanya memiliki hak baca master data (master:read)', 'warning')
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

    dialog.toast('Data berhasil disimpan', 'success')
    showModal.value = false
    fetchAllParameters()
  } catch (err: any) {
    dialog.alert(err.response?.data?.message || 'Gagal menyimpan data', 'danger')
  }
}

async function handleDelete(id: string, name: string) {
  if (!canWrite.value) {
    dialog.alert('Akses ditolak: Anda hanya memiliki hak baca master data (master:read)', 'warning')
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

  const confirmed = await dialog.confirm({
    title: `Nonaktifkan ${tabName.toUpperCase()}`,
    message: `Hapus atau nonaktifkan ${tabName} "${name}"?`,
    subMessage: `Data ${tabName} ini tidak akan muncul lagi pada pilihan parameter baru.`,
    confirmText: 'Ya, Hapus',
    cancelText: 'Batal',
    type: 'danger',
  })
  if (!confirmed) return

  try {
    const endpoint = `/${activeTab.value}/${id}`
    await api.delete(endpoint)
    dialog.toast(`${tabName} "${name}" berhasil dinonaktifkan`, 'success')
    fetchAllParameters()
  } catch (err: any) {
    dialog.alert(err.response?.data?.message || `Gagal menghapus ${tabName}`, 'danger')
  }
}

// Costing Settings State
const costingSettings = ref<{
  id: string
  default_costing_method: 'WMA' | 'FIFO' | 'FEFO' | 'REPLACEMENT_COST'
  default_auto_adjust_selling_price: boolean
  default_markup_percent: number | null
}>({
  id: '',
  default_costing_method: 'WMA',
  default_auto_adjust_selling_price: false,
  default_markup_percent: null,
})
const isSavingCosting = ref<boolean>(false)
const costingSaveSuccess = ref<boolean>(false)

async function fetchCostingSettings() {
  try {
    const res = await api.get('/settings/inventory-costing')
    if (res.data.success && res.data.data) {
      costingSettings.value = {
        id: res.data.data.id || '',
        default_costing_method: res.data.data.default_costing_method || 'WMA',
        default_auto_adjust_selling_price: res.data.data.default_auto_adjust_selling_price || false,
        default_markup_percent: res.data.data.default_markup_percent !== null ? Number(res.data.data.default_markup_percent) : null,
      }
    }
  } catch (err) {
    console.error('Error fetching costing settings:', err)
  }
}

async function saveCostingSettings() {
  isSavingCosting.value = true
  costingSaveSuccess.value = false
  try {
    const res = await api.post('/settings/inventory-costing', {
      default_costing_method: costingSettings.value.default_costing_method,
      default_auto_adjust_selling_price: costingSettings.value.default_auto_adjust_selling_price,
      default_markup_percent: costingSettings.value.default_markup_percent,
    })
    if (res.data.success) {
      costingSaveSuccess.value = true
      dialog.toast('Pengaturan metode HPP global berhasil disimpan!', 'success')
      setTimeout(() => {
        costingSaveSuccess.value = false
      }, 4000)
    }
  } catch (err: any) {
    console.error('Error saving costing settings:', err)
    dialog.alert(err.response?.data?.message || 'Gagal menyimpan pengaturan metode HPP', 'danger')
  } finally {
    isSavingCosting.value = false
  }
}

// Biometric Settings State
const biometricSettings = ref<BiometricSettings>({
  supervisor_biometric_mode: 'OFF',
  supervisor_biometric_policy: 'OPTIONAL',
  face_match_threshold: 75.0,
})
const isSavingBiometric = ref<boolean>(false)
const biometricSaveSuccess = ref<boolean>(false)

async function fetchBiometricData() {
  try {
    const res = await api.get('/settings/biometric')
    if (res.data?.success && res.data.data) {
      biometricSettings.value = {
        supervisor_biometric_mode: res.data.data.supervisor_biometric_mode || 'OFF',
        supervisor_biometric_policy: res.data.data.supervisor_biometric_policy || 'OPTIONAL',
        face_match_threshold: Number(res.data.data.face_match_threshold) || 75.0,
      }
    }
  } catch (err) {
    console.error('Error fetching biometric settings:', err)
  }
}

async function saveBiometricData() {
  if (!canWrite.value) {
    dialog.alert('Akses ditolak: Anda tidak memiliki hak akses mengubah konfigurasi sistem', 'warning')
    return
  }
  isSavingBiometric.value = true
  biometricSaveSuccess.value = false
  try {
    const res = await api.put('/settings/biometric', biometricSettings.value)
    if (res.data?.success) {
      biometricSaveSuccess.value = true
      dialog.toast('Pengaturan sistem biometrik supervisor berhasil diperbarui!', 'success')
      setTimeout(() => {
        biometricSaveSuccess.value = false
      }, 4000)
    }
  } catch (err: any) {
    dialog.alert(err.response?.data?.message || 'Gagal menyimpan pengaturan biometrik', 'danger')
  } finally {
    isSavingBiometric.value = false
  }
}

onMounted(async () => {
  await dirStore.init()
  if (route.query.tab === 'storage') {
    activeTab.value = 'storage'
  }
  fetchAllParameters()
  fetchCostingSettings()
  fetchBiometricData()
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
          v-if="canWrite && activeTab !== 'costing' && activeTab !== 'biometric' && activeTab !== 'storage'"
          @click="openCreateModal"
          class="bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-xs px-4 py-2.5 rounded-lg shadow-sm border border-emerald-700 flex items-center gap-2 transition active:scale-95"
        >
          <Plus class="w-4 h-4" />
          <span>Tambah {{ activeTab === 'units' ? 'Satuan' : activeTab === 'categories' ? 'Kategori' : activeTab === 'brands' ? 'Merek' : 'Pemasok' }}</span>
        </button>
      </div>

      <!-- Tab Buttons -->
      <div class="flex border-b border-slate-300 gap-6 text-sm font-semibold overflow-x-auto">
        <button
          @click="activeTab = 'units'"
          class="pb-3 flex items-center gap-2 transition border-b-2 whitespace-nowrap cursor-pointer"
          :class="activeTab === 'units' ? 'border-emerald-600 text-emerald-800 font-bold' : 'border-transparent text-slate-600 hover:text-slate-900 font-medium'"
        >
          <Ruler class="w-4 h-4" />
          <span>Satuan UoM ({{ units.length }})</span>
        </button>

        <button
          @click="activeTab = 'categories'"
          class="pb-3 flex items-center gap-2 transition border-b-2 whitespace-nowrap cursor-pointer"
          :class="activeTab === 'categories' ? 'border-emerald-600 text-emerald-800 font-bold' : 'border-transparent text-slate-600 hover:text-slate-900 font-medium'"
        >
          <Tags class="w-4 h-4" />
          <span>Kategori ({{ categories.length }})</span>
        </button>

        <button
          @click="activeTab = 'brands'"
          class="pb-3 flex items-center gap-2 transition border-b-2 whitespace-nowrap cursor-pointer"
          :class="activeTab === 'brands' ? 'border-emerald-600 text-emerald-800 font-bold' : 'border-transparent text-slate-600 hover:text-slate-900 font-medium'"
        >
          <Bookmark class="w-4 h-4" />
          <span>Merek / Prinsipal ({{ brands.length }})</span>
        </button>

        <button
          @click="activeTab = 'suppliers'"
          class="pb-3 flex items-center gap-2 transition border-b-2 whitespace-nowrap cursor-pointer"
          :class="activeTab === 'suppliers' ? 'border-emerald-600 text-emerald-800 font-bold' : 'border-transparent text-slate-600 hover:text-slate-900 font-medium'"
        >
          <Truck class="w-4 h-4" />
          <span>Pemasok / Distributor ({{ suppliers.length }})</span>
        </button>

        <button
          @click="activeTab = 'costing'"
          class="pb-3 flex items-center gap-2 transition border-b-2 whitespace-nowrap cursor-pointer"
          :class="activeTab === 'costing' ? 'border-emerald-600 text-emerald-800 font-bold' : 'border-transparent text-slate-600 hover:text-slate-900 font-medium'"
        >
          <Calculator class="w-4 h-4" />
          <span>Metode HPP & Valuasi Stok</span>
        </button>

        <button
          @click="activeTab = 'biometric'"
          class="pb-3 flex items-center gap-2 transition border-b-2 whitespace-nowrap cursor-pointer"
          :class="activeTab === 'biometric' ? 'border-amber-600 text-amber-900 font-bold' : 'border-transparent text-slate-600 hover:text-slate-900 font-medium'"
        >
          <Fingerprint class="w-4 h-4" />
          <span>Sistem Biometrik Supervisor</span>
        </button>

        <button
          @click="activeTab = 'storage'"
          class="pb-3 flex items-center gap-2 transition border-b-2 whitespace-nowrap cursor-pointer"
          :class="activeTab === 'storage' ? 'border-indigo-600 text-indigo-900 font-bold' : 'border-transparent text-slate-600 hover:text-slate-900 font-medium'"
        >
          <FolderArchive class="w-4 h-4 text-indigo-600" />
          <span>Penyimpanan & Berkas</span>
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

      <!-- TAB 5: GLOBAL INVENTORY COSTING SETTINGS -->
      <div v-if="activeTab === 'costing'" class="space-y-6">
        <!-- Info Banner -->
        <div class="bg-gradient-to-r from-blue-50 to-indigo-50 border border-blue-200 rounded-xl p-5 flex items-start gap-4">
          <div class="p-2.5 bg-blue-100 text-blue-700 rounded-lg shrink-0 mt-0.5">
            <Info class="w-5 h-5" />
          </div>
          <div class="space-y-1">
            <h4 class="text-sm font-bold text-blue-900">Pengaturan Global Penilaian HPP & Valuasi Persediaan</h4>
            <p class="text-xs text-blue-800 leading-relaxed">
              Metode yang dipilih di sini berfungsi sebagai <strong>acuan terpusat (default)</strong> untuk semua produk retail.
              Produk dengan pengaturan <em>"Ikuti Pengaturan Global"</em> akan otomatis menggunakan metode ini sehingga Anda tidak perlu repot mengubah ratusan produk satu per satu.
            </p>
          </div>
        </div>

        <div class="bg-white border border-slate-300 rounded-xl p-6 shadow-sm space-y-6">
          <div>
            <label class="block text-sm font-bold text-slate-900 mb-1">Pilih Metode HPP Standar (Global) *</label>
            <p class="text-xs text-slate-500 mb-4">Pilih bagaimana sistem menghitung Beban Pokok Penjualan (HPP) setiap kali terjadi penerimaan kulakan dan penjualan di kasir.</p>

            <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
              <!-- WMA Card -->
              <div
                @click="canWrite && (costingSettings.default_costing_method = 'WMA')"
                class="p-4 rounded-xl border-2 cursor-pointer transition relative flex flex-col justify-between"
                :class="costingSettings.default_costing_method === 'WMA' ? 'border-emerald-600 bg-emerald-50/50 ring-2 ring-emerald-500/20 shadow-sm' : 'border-slate-200 hover:border-slate-300 bg-white'"
              >
                <div>
                  <div class="flex items-center justify-between mb-2">
                    <span class="px-2 py-0.5 rounded text-[10px] font-bold uppercase tracking-wider bg-emerald-100 text-emerald-800 border border-emerald-300">Rekomendasi</span>
                    <div class="w-4 h-4 rounded-full border flex items-center justify-center" :class="costingSettings.default_costing_method === 'WMA' ? 'border-emerald-600 bg-emerald-600 text-white' : 'border-slate-300'">
                      <div v-if="costingSettings.default_costing_method === 'WMA'" class="w-1.5 h-1.5 bg-white rounded-full"></div>
                    </div>
                  </div>
                  <h5 class="font-bold text-slate-900 text-sm mb-1 flex items-center gap-1.5">
                    <TrendingUp class="w-4 h-4 text-emerald-600" />
                    <span>Rata-rata Tertimbang (WMA)</span>
                  </h5>
                  <p class="text-xs text-slate-600 leading-relaxed">
                    HPP dihitung secara proporsional dari gabungan sisa stok gudang dan barang masuk baru. Meredam lonjakan harga yang fluktuatif.
                  </p>
                </div>
                <div class="mt-4 pt-3 border-t border-slate-200/60 text-[11px] text-slate-500 font-mono">
                  Rumus: (StokLama × HPPLama + QtyBaru × HargaBaru) / TotalStok
                </div>
              </div>

              <!-- FIFO & FEFO Card -->
              <div
                @click="canWrite && (costingSettings.default_costing_method = 'FIFO')"
                class="p-4 rounded-xl border-2 cursor-pointer transition relative flex flex-col justify-between"
                :class="(costingSettings.default_costing_method === 'FIFO' || costingSettings.default_costing_method === 'FEFO') ? 'border-blue-600 bg-blue-50/50 ring-2 ring-blue-500/20 shadow-sm' : 'border-slate-200 hover:border-slate-300 bg-white'"
              >
                <div>
                  <div class="flex items-center justify-between mb-2">
                    <span class="px-2 py-0.5 rounded text-[10px] font-bold uppercase tracking-wider bg-blue-100 text-blue-800 border border-blue-300">Batch & Expiry</span>
                    <div class="w-4 h-4 rounded-full border flex items-center justify-center" :class="(costingSettings.default_costing_method === 'FIFO' || costingSettings.default_costing_method === 'FEFO') ? 'border-blue-600 bg-blue-600 text-white' : 'border-slate-300'">
                      <div v-if="costingSettings.default_costing_method === 'FIFO' || costingSettings.default_costing_method === 'FEFO'" class="w-1.5 h-1.5 bg-white rounded-full"></div>
                    </div>
                  </div>
                  <h5 class="font-bold text-slate-900 text-sm mb-1 flex items-center gap-1.5">
                    <Layers class="w-4 h-4 text-blue-600" />
                    <span>FIFO & FEFO (First In / Expired)</span>
                  </h5>
                  <p class="text-xs text-slate-600 leading-relaxed">
                    Barang dengan tanggal kedaluwarsa terdekat (FEFO) otomatis dipotong lebih dahulu dengan HPP batch tersebut. Jika tanpa tanggal kedaluwarsa, otomatis mengikuti urutan tanggal masuk barang (FIFO).
                  </p>
                </div>
                <div class="mt-4 pt-3 border-t border-slate-200/60 text-[11px] text-slate-500 font-mono">
                  Prioritas: 1. FEFO (Kedaluwarsa) → 2. FIFO (Masuk)
                </div>
              </div>

              <!-- REPLACEMENT COST Card -->
              <div
                @click="canWrite && (costingSettings.default_costing_method = 'REPLACEMENT_COST')"
                class="p-4 rounded-xl border-2 cursor-pointer transition relative flex flex-col justify-between"
                :class="costingSettings.default_costing_method === 'REPLACEMENT_COST' ? 'border-purple-600 bg-purple-50/50 ring-2 ring-purple-500/20 shadow-sm' : 'border-slate-200 hover:border-slate-300 bg-white'"
              >
                <div>
                  <div class="flex items-center justify-between mb-2">
                    <span class="px-2 py-0.5 rounded text-[10px] font-bold uppercase tracking-wider bg-purple-100 text-purple-800 border border-purple-300">Harga Terkini</span>
                    <div class="w-4 h-4 rounded-full border flex items-center justify-center" :class="costingSettings.default_costing_method === 'REPLACEMENT_COST' ? 'border-purple-600 bg-purple-600 text-white' : 'border-slate-300'">
                      <div v-if="costingSettings.default_costing_method === 'REPLACEMENT_COST'" class="w-1.5 h-1.5 bg-white rounded-full"></div>
                    </div>
                  </div>
                  <h5 class="font-bold text-slate-900 text-sm mb-1 flex items-center gap-1.5">
                    <Sparkles class="w-4 h-4 text-purple-600" />
                    <span>Harga Ganti (Replacement Cost)</span>
                  </h5>
                  <p class="text-xs text-slate-600 leading-relaxed">
                    HPP aktif cabang selalu langsung diperbarui mengikuti harga kulakan terakhir. Cocok untuk mengantisipasi kenaikan harga pasar.
                  </p>
                </div>
                <div class="mt-4 pt-3 border-t border-slate-200/60 text-[11px] text-slate-500 font-mono">
                  HPP Aktif = Harga Kulakan Terakhir
                </div>
              </div>
            </div>

            <!-- Banner Penjelasan FEFO & Traceability -->
            <div class="mt-4 p-4 rounded-xl bg-blue-50/70 border border-blue-200 flex items-start gap-3">
              <div class="p-2 bg-blue-100 text-blue-700 rounded-lg shrink-0 mt-0.5">
                <Layers class="w-5 h-5" />
              </div>
              <div class="space-y-1.5 text-xs text-slate-700">
                <h5 class="font-bold text-slate-900 text-sm flex items-center gap-2">
                  <span>Bagaimana FEFO Terhubung dengan HPP & Pengeluaran Stok?</span>
                  <span class="px-2 py-0.5 rounded text-[10px] font-bold bg-blue-100 text-blue-800 border border-blue-300">Terintegrasi Otomatis</span>
                </h5>
                <p class="leading-relaxed text-slate-600">
                  <strong>FEFO (First Expired, First Out)</strong> adalah strategi pengeluaran fisik stok yang memprioritaskan barang dengan tanggal kedaluwarsa terdekat agar meminimalkan risiko retur/basi di rak toko. Sistem secara otomatis mencatat referensi nomor faktur pembelian/transfer dan batch kedaluwarsa ke tabel audit pelacakan setiap kali transaksi kasir terjadi.
                </p>
                <div class="pt-2 grid grid-cols-1 sm:grid-cols-2 gap-2 text-[11px] text-slate-600">
                  <div class="bg-white p-2.5 rounded-lg border border-blue-100 shadow-2xs">
                    <span class="font-semibold text-blue-900 block mb-0.5">📦 Jika Menggunakan FIFO & FEFO:</span>
                    HPP penjualan diambil langsung dari harga beli batch kedaluwarsa terdekat yang dikeluarkan saat checkout kasir.
                  </div>
                  <div class="bg-white p-2.5 rounded-lg border border-blue-100 shadow-2xs">
                    <span class="font-semibold text-emerald-900 block mb-0.5">📊 Jika Menggunakan Rata-rata (WMA):</span>
                    Pengeluaran fisik tetap mengikuti FEFO agar barang tidak kedaluwarsa, sementara nilai buku HPP dihitung rata-rata bergerak.
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- Replacement Cost Options (Conditional) -->
          <div
            v-if="costingSettings.default_costing_method === 'REPLACEMENT_COST'"
            class="p-4 rounded-xl bg-purple-50/70 border border-purple-200 space-y-4 text-xs"
          >
            <div class="flex items-center justify-between">
              <div>
                <h5 class="font-bold text-purple-950 text-xs">Penyesuaian Harga Jual Dasar Otomatis</h5>
                <p class="text-[11px] text-purple-800">
                  Otomatis sesuaikan harga jual dasar kasir ketika kulakan baru memiliki harga beli yang berbeda.
                </p>
              </div>
              <label class="relative inline-flex items-center cursor-pointer">
                <input
                  v-model="costingSettings.default_auto_adjust_selling_price"
                  type="checkbox"
                  class="sr-only peer"
                  :disabled="!canWrite"
                />
                <div class="w-9 h-5 bg-slate-300 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-4 after:w-4 after:transition-all peer-checked:bg-purple-600"></div>
              </label>
            </div>

            <div v-if="costingSettings.default_auto_adjust_selling_price" class="pt-3 border-t border-purple-200 grid grid-cols-1 md:grid-cols-2 gap-4 items-center">
              <div>
                <label class="block font-bold text-purple-950 mb-1">Target Persentase Markup Margin (%)</label>
                <div class="relative">
                  <input
                    v-model.number="costingSettings.default_markup_percent"
                    type="number"
                    step="0.01"
                    min="0"
                    placeholder="Kosongkan jika ingin pertahankan rasio profit lama"
                    class="w-full bg-white border border-purple-300 rounded-lg px-3 py-2 pr-8 text-slate-900 focus:outline-none focus:border-purple-600"
                    :disabled="!canWrite"
                  />
                  <Percent class="w-4 h-4 text-purple-600 absolute right-2.5 top-2.5 pointer-events-none" />
                </div>
                <p class="text-[10px] text-purple-700 mt-1">
                  Jika diisi misal 25%, maka: Harga Jual = HPP Baru × (1 + 25%).
                </p>
              </div>
              <div class="p-3 bg-white/80 rounded-lg border border-purple-200 text-purple-900 text-[11px] leading-relaxed">
                <span class="font-bold">Tips:</span> Jika dikosongkan, sistem akan mempertahankan persentase keuntungan awal produk: <code>(HargaJualLama - HPPLama) / HPPLama</code>.
              </div>
            </div>
          </div>

          <!-- Bottom Action -->
          <div v-if="canWrite" class="pt-4 border-t border-slate-200 flex items-center justify-between">
            <div class="flex items-center gap-2">
              <span v-if="costingSaveSuccess" class="flex items-center gap-1.5 text-xs font-bold text-emerald-700 bg-emerald-50 px-3 py-1.5 rounded-lg border border-emerald-200">
                <CheckCircle2 class="w-4 h-4 text-emerald-600" />
                Pengaturan berhasil disimpan dan langsung aktif!
              </span>
            </div>

            <button
              @click="saveCostingSettings"
              :disabled="isSavingCosting"
              class="px-5 py-2.5 bg-emerald-600 hover:bg-emerald-700 text-white rounded-lg text-xs font-bold shadow-sm flex items-center gap-2 transition active:scale-95 disabled:opacity-50"
            >
              <Save class="w-4 h-4" />
              <span>{{ isSavingCosting ? 'Menyimpan...' : 'Simpan Pengaturan Global' }}</span>
            </button>
          </div>
        </div>
      </div>

      <!-- TAB 6: BIOMETRIC SUPERVISOR SETTINGS -->
      <div v-if="activeTab === 'biometric'" class="space-y-6">
        <div class="bg-white border border-slate-300 rounded-2xl p-6 shadow-sm space-y-6">
          <div class="flex flex-col md:flex-row md:items-center justify-between gap-4 border-b border-slate-200 pb-5">
            <div>
              <div class="flex items-center gap-2.5">
                <div class="p-2 bg-amber-100 border border-amber-300 rounded-xl text-amber-800">
                  <Fingerprint class="w-6 h-6" />
                </div>
                <div>
                  <h3 class="text-base font-extrabold text-slate-900">Konfigurasi Sistem Biometrik Supervisor</h3>
                  <p class="text-xs text-slate-500 mt-0.5">
                    Tentukan metode otorisasi supervisor pada transaksi POS (Void, Diskon, Retur, Cash Drop, Buka/Tutup Kasir).
                  </p>
                </div>
              </div>
            </div>

            <span class="inline-flex items-center gap-1.5 text-xs font-bold px-3 py-1.5 rounded-full border"
              :class="biometricSettings.supervisor_biometric_mode !== 'OFF' ? 'bg-emerald-50 text-emerald-800 border-emerald-300' : 'bg-slate-100 text-slate-700 border-slate-300'">
              <span class="w-2 h-2 rounded-full" :class="biometricSettings.supervisor_biometric_mode !== 'OFF' ? 'bg-emerald-500' : 'bg-slate-400'"></span>
              <span>Status: {{ biometricSettings.supervisor_biometric_mode !== 'OFF' ? 'Biometrik Aktif' : 'Hanya PIN Angka' }}</span>
            </span>
          </div>

          <!-- 1. Pilihan Mode Biometrik Utama -->
          <div class="space-y-3">
            <label class="block text-xs font-bold text-slate-800 uppercase tracking-wider">
              1. Pilih Sistem Biometrik yang Digunakan
            </label>
            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-3.5">
              <!-- Mode: OFF -->
              <div
                @click="canWrite && (biometricSettings.supervisor_biometric_mode = 'OFF')"
                class="p-4 rounded-xl border-2 transition-all cursor-pointer relative"
                :class="biometricSettings.supervisor_biometric_mode === 'OFF'
                  ? 'border-indigo-600 bg-indigo-50/70 shadow-sm ring-2 ring-indigo-400/20'
                  : 'border-slate-200 hover:border-slate-300 bg-slate-50/50 hover:bg-white'"
              >
                <div class="flex items-center justify-between mb-2">
                  <div class="w-9 h-9 rounded-lg bg-indigo-100 text-indigo-700 flex items-center justify-center">
                    <Icon icon="streamline-ultimate-color:dial-pad" class="w-5 h-5" />
                  </div>
                  <span v-if="biometricSettings.supervisor_biometric_mode === 'OFF'" class="w-2.5 h-2.5 rounded-full bg-indigo-600"></span>
                </div>
                <h4 class="font-bold text-xs text-slate-900">Nonaktif (Hanya PIN)</h4>
                <p class="text-[11px] text-slate-500 mt-1 leading-relaxed">
                  Supervisor wajib memasukkan 4-6 digit angka PIN menggunakan papan tombol virtual kasir.
                </p>
              </div>

              <!-- Mode: FINGERPRINT -->
              <div
                @click="canWrite && (biometricSettings.supervisor_biometric_mode = 'FINGERPRINT')"
                class="p-4 rounded-xl border-2 transition-all cursor-pointer relative"
                :class="biometricSettings.supervisor_biometric_mode === 'FINGERPRINT'
                  ? 'border-emerald-600 bg-emerald-50/70 shadow-sm ring-2 ring-emerald-400/20'
                  : 'border-slate-200 hover:border-slate-300 bg-slate-50/50 hover:bg-white'"
              >
                <div class="flex items-center justify-between mb-2">
                  <div class="w-9 h-9 rounded-lg bg-emerald-100 text-emerald-700 flex items-center justify-center">
                    <Fingerprint class="w-5 h-5" />
                  </div>
                  <span v-if="biometricSettings.supervisor_biometric_mode === 'FINGERPRINT'" class="w-2.5 h-2.5 rounded-full bg-emerald-600"></span>
                </div>
                <h4 class="font-bold text-xs text-slate-900">Sensor Sidik Jari</h4>
                <p class="text-[11px] text-slate-500 mt-1 leading-relaxed">
                  Menggunakan sensor Touch ID (Mac), Windows Hello, atau sensor USB sidik jari perangkat kasir.
                </p>
              </div>

              <!-- Mode: FACE_RECOGNITION -->
              <div
                @click="canWrite && (biometricSettings.supervisor_biometric_mode = 'FACE_RECOGNITION')"
                class="p-4 rounded-xl border-2 transition-all cursor-pointer relative"
                :class="biometricSettings.supervisor_biometric_mode === 'FACE_RECOGNITION'
                  ? 'border-blue-600 bg-blue-50/70 shadow-sm ring-2 ring-blue-400/20'
                  : 'border-slate-200 hover:border-slate-300 bg-slate-50/50 hover:bg-white'"
              >
                <div class="flex items-center justify-between mb-2">
                  <div class="w-9 h-9 rounded-lg bg-blue-100 text-blue-700 flex items-center justify-center">
                    <Camera class="w-5 h-5" />
                  </div>
                  <span v-if="biometricSettings.supervisor_biometric_mode === 'FACE_RECOGNITION'" class="w-2.5 h-2.5 rounded-full bg-blue-600"></span>
                </div>
                <h4 class="font-bold text-xs text-slate-900">Pemindai Wajah Kamera</h4>
                <p class="text-[11px] text-slate-500 mt-1 leading-relaxed">
                  Mendeteksi dan mengenali wajah supervisor secara otomatis melalui kamera webcam kasir.
                </p>
              </div>

              <!-- Mode: ANY -->
              <div
                @click="canWrite && (biometricSettings.supervisor_biometric_mode = 'ANY')"
                class="p-4 rounded-xl border-2 transition-all cursor-pointer relative"
                :class="biometricSettings.supervisor_biometric_mode === 'ANY'
                  ? 'border-amber-600 bg-amber-50/70 shadow-sm ring-2 ring-amber-400/20'
                  : 'border-slate-200 hover:border-slate-300 bg-slate-50/50 hover:bg-white'"
              >
                <div class="flex items-center justify-between mb-2">
                  <div class="w-9 h-9 rounded-lg bg-amber-100 text-amber-700 flex items-center justify-center">
                    <ShieldCheck class="w-5 h-5" />
                  </div>
                  <span v-if="biometricSettings.supervisor_biometric_mode === 'ANY'" class="w-2.5 h-2.5 rounded-full bg-amber-600"></span>
                </div>
                <h4 class="font-bold text-xs text-slate-900">Fleksibel (Semua Opsi)</h4>
                <p class="text-[11px] text-slate-500 mt-1 leading-relaxed">
                  Supervisor bebas memilih otorisasi: Sentuh Sidik Jari, Pindai Wajah, ataupun Masukkan PIN.
                </p>
              </div>
            </div>
          </div>

          <!-- 2. Kebijakan Otorisasi & Fallback PIN -->
          <div v-if="biometricSettings.supervisor_biometric_mode !== 'OFF'" class="p-4 bg-slate-50 rounded-xl border border-slate-200 space-y-4">
            <h4 class="text-xs font-bold text-slate-800 uppercase tracking-wider">
              2. Kebijakan Penggunaan Biometrik Toko
            </h4>

            <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
              <label
                class="flex items-start gap-3 p-3.5 rounded-lg border cursor-pointer transition"
                :class="biometricSettings.supervisor_biometric_policy === 'OPTIONAL' ? 'bg-white border-amber-500 shadow-2xs' : 'bg-slate-100/60 border-slate-200'"
              >
                <input
                  type="radio"
                  v-model="biometricSettings.supervisor_biometric_policy"
                  value="OPTIONAL"
                  class="mt-0.5 text-amber-600 focus:ring-amber-500"
                  :disabled="!canWrite"
                />
                <div>
                  <div class="font-bold text-xs text-slate-900">Fleksibel / Opsional (Rekomendasi)</div>
                  <div class="text-[11px] text-slate-500 mt-0.5">
                    Menyediakan tombol Sidik Jari & Wajah, namun kasir tetap dapat memasukkan PIN angka sebagai alternatif jika sensor sedang terkendala.
                  </div>
                </div>
              </label>

              <label
                class="flex items-start gap-3 p-3.5 rounded-lg border cursor-pointer transition"
                :class="biometricSettings.supervisor_biometric_policy === 'STRICT' ? 'bg-white border-rose-500 shadow-2xs' : 'bg-slate-100/60 border-slate-200'"
              >
                <input
                  type="radio"
                  v-model="biometricSettings.supervisor_biometric_policy"
                  value="STRICT"
                  class="mt-0.5 text-rose-600 focus:ring-rose-500"
                  :disabled="!canWrite"
                />
                <div>
                  <div class="font-bold text-xs text-slate-900">Ketat / Wajib Biometrik</div>
                  <div class="text-[11px] text-slate-500 mt-0.5">
                    Supervisor yang memiliki data biometrik terdaftar wajib menggunakan sidik jari atau wajah; PIN angka tidak dapat digunakan.
                  </div>
                </div>
              </label>
            </div>
          </div>

          <!-- 3. Parameter Akurasi Wajah -->
          <div v-if="['FACE_RECOGNITION', 'ANY'].includes(biometricSettings.supervisor_biometric_mode)" class="p-4 bg-blue-50/60 rounded-xl border border-blue-200 space-y-3">
            <div class="flex items-center justify-between">
              <div>
                <h4 class="text-xs font-bold text-blue-950 uppercase tracking-wider">
                  3. Ambang Batas Akurasi Pengenalan Wajah
                </h4>
                <p class="text-[11px] text-blue-800 mt-0.5">
                  Tingkat persentase kecocokan minimum antara pemindaian langsung di kamera kasir dengan foto referensi supervisor.
                </p>
              </div>
              <div class="px-3 py-1 bg-white border border-blue-300 rounded-lg font-mono font-black text-blue-900 text-sm">
                {{ biometricSettings.face_match_threshold }}%
              </div>
            </div>

            <div class="flex items-center gap-4 pt-1">
              <span class="text-[11px] font-bold text-slate-500 font-mono">60%</span>
              <input
                type="range"
                v-model.number="biometricSettings.face_match_threshold"
                min="60"
                max="95"
                step="1"
                class="w-full h-2 bg-blue-200 rounded-lg appearance-none cursor-pointer accent-blue-600"
                :disabled="!canWrite"
              />
              <span class="text-[11px] font-bold text-slate-500 font-mono">95%</span>
            </div>

            <div class="text-[11px] text-blue-700 bg-white/80 p-2.5 rounded-lg border border-blue-100 flex items-center gap-2">
              <Info class="w-4 h-4 shrink-0 text-blue-600" />
              <span>Standar rekomendasi adalah <strong>75%</strong> untuk keseimbangan optimal antara kecepatan deteksi di kasir dan akurasi keamanan.</span>
            </div>
          </div>

          <!-- Informasi Pendaftaran Mandiri -->
          <div class="p-4 bg-amber-50 rounded-xl border border-amber-200 text-amber-900 text-xs flex items-start gap-3">
            <Icon icon="fluent-color:info-24" class="w-5 h-5 shrink-0 mt-0.5" />
            <div>
              <h5 class="font-bold text-amber-950">Cara Mendaftarkan Sidik Jari & Wajah Supervisor:</h5>
              <p class="text-[11px] text-amber-800 mt-0.5 leading-relaxed">
                Supervisor dapat mendaftarkan sidik jari atau memindai wajah mereka sendiri kapan saja dengan mengklik nama profil akun di bilah navigasi atas lalu memilih tab <strong>"Biometrik & Wajah"</strong>.
              </p>
            </div>
          </div>

          <!-- Bottom Action Buttons -->
          <div v-if="canWrite" class="pt-4 border-t border-slate-200 flex items-center justify-between">
            <div class="flex items-center gap-2">
              <span v-if="biometricSaveSuccess" class="flex items-center gap-1.5 text-xs font-bold text-emerald-700 bg-emerald-50 px-3 py-1.5 rounded-lg border border-emerald-200">
                <CheckCircle2 class="w-4 h-4 text-emerald-600" />
                Pengaturan sistem biometrik berhasil disimpan!
              </span>
            </div>

            <button
              @click="saveBiometricData"
              :disabled="isSavingBiometric"
              class="px-5 py-2.5 bg-emerald-600 hover:bg-emerald-700 text-white rounded-lg text-xs font-bold shadow-sm flex items-center gap-2 transition active:scale-95 disabled:opacity-50 cursor-pointer"
            >
              <Save class="w-4 h-4" />
              <span>{{ isSavingBiometric ? 'Menyimpan...' : 'Simpan Pengaturan Biometrik' }}</span>
            </button>
          </div>
        </div>
      </div>

      <!-- TAB 7: STORAGE & DIRECTORY SETTINGS -->
      <div v-if="activeTab === 'storage'" class="space-y-5">
        <!-- Banner Header Info -->
        <div class="bg-white border border-slate-300 rounded-xl p-5 shadow-xs flex flex-col md:flex-row items-start md:items-center justify-between gap-4">
          <div class="flex items-start gap-3.5">
            <div class="p-2.5 rounded-lg bg-slate-100 text-slate-700 border border-slate-200 shrink-0">
              <FolderArchive class="w-6 h-6" />
            </div>
            <div>
              <h2 class="text-base font-bold text-slate-900 tracking-tight">Folder Penyimpanan Berkas & Cadangan</h2>
              <p class="text-xs text-slate-500 mt-0.5 max-w-xl leading-relaxed">
                Tentukan folder lokal untuk menyimpan file hasil ekspor laporan Excel dan file cadangan database.
              </p>
            </div>
          </div>
          <div class="flex items-center gap-2 bg-slate-50 px-3 py-1.5 rounded-lg border border-slate-200 text-xs text-slate-700 shrink-0">
            <span class="w-2 h-2 rounded-full" :class="dirStore.isTauri ? 'bg-emerald-500' : 'bg-slate-400'"></span>
            <span class="font-medium">
              {{ dirStore.isTauri ? 'Desktop Native' : 'Browser Web' }}
            </span>
          </div>
        </div>

        <!-- Dua Card Utama: Ekspor dan Cadangan -->
        <div class="grid grid-cols-1 md:grid-cols-2 gap-5">
          <!-- Card 1: Direktori Ekspor Laporan Excel -->
          <div class="bg-white border border-slate-300 rounded-xl p-5 shadow-xs flex flex-col justify-between space-y-4">
            <div class="space-y-3">
              <div class="flex items-center justify-between">
                <div class="flex items-center gap-2 text-slate-900 font-bold text-sm">
                  <Folder class="w-4 h-4 text-emerald-600" />
                  <span>Folder Ekspor Laporan</span>
                </div>
                <span class="text-[10px] font-mono px-2 py-0.5 rounded bg-slate-100 text-slate-600 border border-slate-200">
                  .xlsx / .csv
                </span>
              </div>
              <p class="text-xs text-slate-500 leading-relaxed">
                File ekspor laporan penjualan, pembelian, stok, dan produk akan disimpan ke folder ini.
              </p>

              <div class="space-y-2 pt-1">
                <label class="block text-xs font-semibold text-slate-700">Lokasi Folder Saat Ini:</label>
                <div class="flex items-center gap-2">
                  <input
                    type="text"
                    :value="dirStore.exportDirectory"
                    @input="(e: any) => dirStore.setExportDirectory(e.target.value)"
                    placeholder="Contoh: /Users/.../Documents/Notane/Exports"
                    class="flex-1 bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-xs font-mono text-slate-800 focus:bg-white focus:border-emerald-600 focus:outline-none"
                  />
                  <button
                    v-if="dirStore.isTauri"
                    type="button"
                    @click="dirStore.chooseExportDirectory()"
                    class="px-3 py-2 rounded-lg bg-slate-800 hover:bg-slate-900 text-white text-xs font-bold transition flex items-center gap-1.5 shadow-xs cursor-pointer shrink-0"
                    title="Pilih folder dari dialog sistem"
                  >
                    <FolderOpen class="w-3.5 h-3.5" />
                    <span>Pilih Folder</span>
                  </button>
                </div>
                <div class="text-[11px] text-slate-400">
                  Default: <code class="text-slate-600 font-mono">{{ dirStore.systemPaths?.default_export_dir || 'Documents/Notane/Exports' }}</code>
                </div>
              </div>
            </div>

            <div class="pt-3 border-t border-slate-100 flex items-center justify-between gap-2">
              <button
                type="button"
                @click="dirStore.resetExportDirectory()"
                class="text-xs text-slate-500 hover:text-slate-800 font-medium flex items-center gap-1 cursor-pointer transition"
              >
                <RotateCcw class="w-3.5 h-3.5" />
                <span>Reset ke Default</span>
              </button>

              <button
                v-if="dirStore.isTauri"
                type="button"
                @click="dirStore.openExportFolder()"
                class="px-3 py-1.5 rounded-lg border border-slate-300 bg-white hover:bg-slate-50 text-slate-700 text-xs font-semibold transition flex items-center gap-1.5 shadow-2xs cursor-pointer"
              >
                <FolderOpen class="w-3.5 h-3.5 text-slate-600" />
                <span>Buka di Folder</span>
              </button>
            </div>
          </div>

          <!-- Card 2: Direktori Cadangan Database -->
          <div class="bg-white border border-slate-300 rounded-xl p-5 shadow-xs flex flex-col justify-between space-y-4">
            <div class="space-y-3">
              <div class="flex items-center justify-between">
                <div class="flex items-center gap-2 text-slate-900 font-bold text-sm">
                  <HardDrive class="w-4 h-4 text-indigo-600" />
                  <span>Folder Cadangan Database</span>
                </div>
                <span class="text-[10px] font-mono px-2 py-0.5 rounded bg-slate-100 text-slate-600 border border-slate-200">
                  .zip
                </span>
              </div>
              <p class="text-xs text-slate-500 leading-relaxed">
                File arsip cadangan database (.zip) yang diekspor dari server toko akan disimpan ke folder ini.
              </p>

              <div class="space-y-2 pt-1">
                <label class="block text-xs font-semibold text-slate-700">Lokasi Folder Saat Ini:</label>
                <div class="flex items-center gap-2">
                  <input
                    type="text"
                    :value="dirStore.backupDirectory"
                    @input="(e: any) => dirStore.setBackupDirectory(e.target.value)"
                    placeholder="Contoh: /Users/.../Documents/Notane/Backups"
                    class="flex-1 bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-xs font-mono text-slate-800 focus:bg-white focus:border-indigo-600 focus:outline-none"
                  />
                  <button
                    v-if="dirStore.isTauri"
                    type="button"
                    @click="dirStore.chooseBackupDirectory()"
                    class="px-3 py-2 rounded-lg bg-slate-800 hover:bg-slate-900 text-white text-xs font-bold transition flex items-center gap-1.5 shadow-xs cursor-pointer shrink-0"
                    title="Pilih folder dari dialog sistem"
                  >
                    <FolderOpen class="w-3.5 h-3.5" />
                    <span>Pilih Folder</span>
                  </button>
                </div>
                <div class="text-[11px] text-slate-400">
                  Default: <code class="text-slate-600 font-mono">{{ dirStore.systemPaths?.default_backup_dir || 'Documents/Notane/Backups' }}</code>
                </div>
              </div>
            </div>

            <div class="pt-3 border-t border-slate-100 flex items-center justify-between gap-2">
              <button
                type="button"
                @click="dirStore.resetBackupDirectory()"
                class="text-xs text-slate-500 hover:text-slate-800 font-medium flex items-center gap-1 cursor-pointer transition"
              >
                <RotateCcw class="w-3.5 h-3.5" />
                <span>Reset ke Default</span>
              </button>

              <button
                v-if="dirStore.isTauri"
                type="button"
                @click="dirStore.openBackupFolder()"
                class="px-3 py-1.5 rounded-lg border border-slate-300 bg-white hover:bg-slate-50 text-slate-700 text-xs font-semibold transition flex items-center gap-1.5 shadow-2xs cursor-pointer"
              >
                <FolderOpen class="w-3.5 h-3.5 text-slate-600" />
                <span>Buka di Folder</span>
              </button>
            </div>
          </div>
        </div>

        <!-- Card 3: Petunjuk Penggunaan -->
        <div class="bg-slate-50 border border-slate-200 rounded-xl p-4 text-xs text-slate-600 flex items-start gap-3">
          <Info class="w-4 h-4 text-slate-500 shrink-0 mt-0.5" />
          <div class="space-y-1 leading-relaxed">
            <p>
              Pada aplikasi desktop, file yang baru selesai diekspor dapat langsung dibuka melalui tombol <strong>"Buka Folder"</strong> di widget ekspor.
            </p>
            <p>
              Jika aplikasi dijalankan melalui peramban web (browser), file akan terunduh otomatis ke folder <strong>Downloads</strong> standar peramban Anda.
            </p>
          </div>
        </div>
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
