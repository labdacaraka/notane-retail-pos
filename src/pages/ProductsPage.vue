<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import AdminLayout from '@/layouts/AdminLayout.vue'
import PaginationBar from '@/components/common/PaginationBar.vue'
import SearchableSelect from '@/components/common/SearchableSelect.vue'
import { Search, Scale, Edit2, Trash2, Users, Tag, Check, Building2, Lock } from 'lucide-vue-next'
import { api } from '@/services/api'
import { useAuthStore } from '@/stores/authStore'

const authStore = useAuthStore()

const products = ref<any[]>([])
const categories = ref<any[]>([])
const units = ref<any[]>([])
const brands = ref<any[]>([])
const isLoading = ref<boolean>(false)
const searchQuery = ref<string>('')

// Pagination state
const currentPage = ref<number>(1)
const pageSize = ref<number>(10)

const paginatedProducts = computed(() => {
  const start = (currentPage.value - 1) * pageSize.value
  return products.value.slice(start, start + pageSize.value)
})

// Create Modal
const showCreateModal = ref<boolean>(false)
const form = ref({
  sku: '',
  barcode: '',
  name: '',
  category_id: '',
  base_unit_id: '',
  brand_id: null as string | null,
  cost_price: 0,
  base_selling_price: 0,
  is_weight_embedded: false,
  costing_method: 'WMA',
  initial_stock: 0,
})

// Edit Modal
const showEditModal = ref<boolean>(false)
const editingId = ref<string | null>(null)
const editForm = ref({
  sku: '',
  barcode: '',
  name: '',
  category_id: '',
  base_unit_id: '',
  brand_id: null as string | null,
  cost_price: 0,
  base_selling_price: 0,
  is_weight_embedded: false,
  min_stock: 0,
  max_stock: 0,
  tax_percent: 11,
})

async function fetchProducts() {
  isLoading.value = true
  try {
    const res = await api.get('/products', {
      params: { search: searchQuery.value || undefined },
    })
    if (res.data.success) {
      products.value = res.data.data
    }
  } catch (err) {
    console.error(err)
  } finally {
    isLoading.value = false
  }
}

async function fetchMasterMeta() {
  try {
    const [catRes, unitRes, brandRes] = await Promise.all([
      api.get('/categories'),
      api.get('/units'),
      api.get('/brands'),
    ])
    if (catRes.data.success) categories.value = catRes.data.data
    if (unitRes.data.success) units.value = unitRes.data.data
    if (brandRes.data.success) brands.value = brandRes.data.data

    if (categories.value.length > 0) form.value.category_id = categories.value[0].id
    if (units.value.length > 0) form.value.base_unit_id = units.value[0].id
  } catch (err) {
    console.error(err)
  }
}

async function handleCreateProduct() {
  if (!authStore.hasPermission('master:write')) {
    alert('Akses ditolak: Anda hanya memiliki hak baca master data (master:read)')
    return
  }
  try {
    await api.post('/products', form.value)
    showCreateModal.value = false
    fetchProducts()
  } catch (err: any) {
    alert(err.response?.data?.message || 'Gagal menyimpan produk')
  }
}

function openEditModal(prod: any) {
  if (!authStore.hasPermission('master:write')) {
    alert('Akses ditolak: Anda hanya memiliki hak baca master data (master:read)')
    return
  }
  editingId.value = prod.id
  editForm.value = {
    sku: prod.sku,
    barcode: prod.barcode,
    name: prod.name,
    category_id: prod.category_id,
    base_unit_id: prod.base_unit_id,
    brand_id: prod.brand_id || null,
    cost_price: Number(prod.cost_price),
    base_selling_price: Number(prod.base_selling_price),
    is_weight_embedded: Boolean(prod.is_weight_embedded),
    min_stock: Number(prod.min_stock || 0),
    max_stock: Number(prod.max_stock || 0),
    tax_percent: Number(prod.tax_percent || 11),
  }
  showEditModal.value = true
}

async function handleUpdateProduct() {
  if (!authStore.hasPermission('master:write')) {
    alert('Akses ditolak: Anda hanya memiliki hak baca master data (master:read)')
    return
  }
  if (!editingId.value) return
  try {
    await api.put(`/products/${editingId.value}`, editForm.value)
    showEditModal.value = false
    fetchProducts()
  } catch (err: any) {
    alert(err.response?.data?.message || 'Gagal memperbarui produk')
  }
}

async function handleDeleteProduct(id: string, name: string) {
  if (!authStore.hasPermission('master:write')) {
    alert('Akses ditolak: Anda hanya memiliki hak baca master data (master:read)')
    return
  }
  if (!confirm(`Nonaktifkan produk "${name}"?`)) return
  try {
    await api.delete(`/products/${id}`)
    fetchProducts()
  } catch (err: any) {
    alert(err.response?.data?.message || 'Gagal menonaktifkan produk')
  }
}

// Customer Group Prices Modal
const showGroupPriceModal = ref<boolean>(false)
const selectedProductForGroupPrice = ref<any>(null)
const customerGroups = ref<any[]>([])
const productGroupPrices = ref<any[]>([])
const groupPriceInputs = ref<Record<number, number | null>>({})
const isSavingGroupPrices = ref<boolean>(false)
const isLoadingGroupPrices = ref<boolean>(false)

async function openGroupPriceModal(prod: any) {
  selectedProductForGroupPrice.value = prod
  groupPriceInputs.value = {}
  showGroupPriceModal.value = true
  isLoadingGroupPrices.value = true

  try {
    const [groupsRes, pricesRes] = await Promise.all([
      api.get('/customer-groups'),
      api.get(`/products/${prod.id}/customer-group-prices`),
    ])

    if (groupsRes.data.success) {
      customerGroups.value = groupsRes.data.data
    }
    if (pricesRes.data.success) {
      productGroupPrices.value = pricesRes.data.data
      for (const gp of pricesRes.data.data) {
        groupPriceInputs.value[gp.customer_group_id] = Number(gp.special_price)
      }
    }
  } catch (err) {
    console.error('Gagal memuat harga grup pelanggan:', err)
  } finally {
    isLoadingGroupPrices.value = false
  }
}

async function handleSaveGroupPrice(groupId: number) {
  if (!authStore.hasPermission('master:write')) {
    alert('Akses ditolak: Anda hanya memiliki hak baca master data')
    return
  }
  const prodId = selectedProductForGroupPrice.value?.id
  if (!prodId) return

  const inputVal = groupPriceInputs.value[groupId]
  isSavingGroupPrices.value = true

  try {
    if (inputVal === null || inputVal === undefined || inputVal <= 0) {
      // If cleared or zero, delete custom group price (fallback to base price)
      const existing = productGroupPrices.value.find((gp) => gp.customer_group_id === groupId)
      if (existing) {
        await api.delete(`/products/customer-group-prices/${existing.id}`)
      }
      groupPriceInputs.value[groupId] = null
    } else {
      await api.post(`/products/${prodId}/customer-group-prices`, {
        customer_group_id: groupId,
        special_price: Number(inputVal),
      })
    }

    // Refresh list
    const pricesRes = await api.get(`/products/${prodId}/customer-group-prices`)
    if (pricesRes.data.success) {
      productGroupPrices.value = pricesRes.data.data
    }
    alert('Harga kelompok pelanggan berhasil diperbarui!')
  } catch (err: any) {
    alert(err.response?.data?.message || 'Gagal menyimpan harga kelompok pelanggan')
  } finally {
    isSavingGroupPrices.value = false
  }
}

// Outlet-Specific Prices Modal
const showOutletPriceModal = ref<boolean>(false)
const selectedProductForOutletPrice = ref<any>(null)
const outletsList = ref<any[]>([])
const productOutletPrices = ref<any[]>([])
const outletPriceInputs = ref<Record<string, { cost_price: number | null; base_selling_price: number | null }>>({})
const isSavingOutletPrices = ref<boolean>(false)
const isLoadingOutletPrices = ref<boolean>(false)

async function openOutletPriceModal(prod: any) {
  selectedProductForOutletPrice.value = prod
  outletPriceInputs.value = {}
  showOutletPriceModal.value = true
  isLoadingOutletPrices.value = true

  try {
    const [_, outletsRes, pricesRes] = await Promise.all([
      authStore.fetchCurrentUser(),
      api.get('/me/outlets'),
      api.get(`/products/${prod.id}/outlet-prices`),
    ])

    if (outletsRes.data.success) {
      outletsList.value = outletsRes.data.data.filter((o: any) => o.is_active !== false)
    }
    if (pricesRes.data.success) {
      productOutletPrices.value = pricesRes.data.data
      for (const op of pricesRes.data.data) {
        outletPriceInputs.value[op.outlet_id] = {
          cost_price: Number(op.cost_price),
          base_selling_price: Number(op.base_selling_price),
        }
      }
    }
  } catch (err) {
    console.error('Gagal memuat harga outlet:', err)
  } finally {
    isLoadingOutletPrices.value = false
  }
}

async function handleSaveOutletPrice(outletId: string) {
  if (!authStore.hasPermission('master:write')) {
    alert('Akses ditolak: Anda hanya memiliki hak baca master data')
    return
  }
  if (!authStore.hasOutletAccess(outletId)) {
    alert('Akses ditolak: Anda tidak memiliki wewenang untuk mengatur harga di outlet ini!')
    return
  }
  const prodId = selectedProductForOutletPrice.value?.id
  if (!prodId) return

  const input = outletPriceInputs.value[outletId]
  isSavingOutletPrices.value = true

  try {
    const cost = input?.cost_price
    const price = input?.base_selling_price

    if ((cost === null || cost === undefined || cost < 0) && (price === null || price === undefined || price < 0)) {
      // If both cleared, delete override (fallback to global product prices)
      const existing = productOutletPrices.value.find((op) => op.outlet_id === outletId)
      if (existing) {
        await api.delete(`/products/outlet-prices/${existing.id}`)
      }
      delete outletPriceInputs.value[outletId]
    } else {
      const finalCost = cost !== null && cost !== undefined ? Number(cost) : Number(selectedProductForOutletPrice.value?.cost_price || 0)
      const finalPrice = price !== null && price !== undefined ? Number(price) : Number(selectedProductForOutletPrice.value?.base_selling_price || 0)

      await api.post(`/products/${prodId}/outlet-prices`, {
        outlet_id: outletId,
        cost_price: finalCost,
        base_selling_price: finalPrice,
      })
    }

    // Refresh list
    const pricesRes = await api.get(`/products/${prodId}/outlet-prices`)
    if (pricesRes.data.success) {
      productOutletPrices.value = pricesRes.data.data
      for (const op of pricesRes.data.data) {
        outletPriceInputs.value[op.outlet_id] = {
          cost_price: Number(op.cost_price),
          base_selling_price: Number(op.base_selling_price),
        }
      }
    }
    alert('Harga outlet berhasil diperbarui!')
  } catch (err: any) {
    alert(err.response?.data?.message || 'Gagal menyimpan harga outlet')
  } finally {
    isSavingOutletPrices.value = false
  }
}

onMounted(() => {
  authStore.fetchCurrentUser()
  fetchProducts()
  fetchMasterMeta()
})
</script>

<template>
  <AdminLayout>
    <div class="p-6 space-y-4">
      <!-- Page Header -->
      <div class="flex items-center justify-between">
        <div>
          <h2 class="text-xl font-bold text-slate-900 flex items-center gap-2.5">
            <Icon icon="streamline-ultimate-color:warehouse-cart-packages-2" class="w-6 h-6 drop-shadow-xs" />
            <span>Master Produk & Penetapan Harga</span>
          </h2>
          <p class="text-xs text-slate-500">Kelola SKU, multi-barcode, barcode timbangan elektrik, dan metode HPP.</p>
        </div>

        <button
          v-if="authStore.hasPermission('master:write')"
          @click="showCreateModal = true"
          class="bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-xs px-4 py-2 rounded-lg shadow-sm flex items-center gap-2 transition active:scale-95 border border-emerald-700 cursor-pointer"
        >
          <Icon icon="streamline-ultimate-color:add-circle-bold" class="w-4 h-4" />
          <span>Tambah Produk Baru</span>
        </button>
      </div>

      <!-- Filter / Search -->
      <div class="flex items-center gap-3 bg-white border border-slate-300 p-2.5 rounded-xl shadow-sm">
        <div class="relative flex-1">
          <Search class="w-4 h-4 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2" />
          <input
            v-model="searchQuery"
            placeholder="Cari SKU, Barcode, atau Nama Produk..."
            class="w-full bg-slate-50 border border-slate-300 rounded-lg pl-9 pr-4 py-1.5 text-xs text-slate-900 focus:outline-none focus:bg-white focus:border-emerald-600 font-medium"
            @keyup.enter="fetchProducts"
          />
        </div>
        <button
          @click="fetchProducts"
          class="bg-slate-100 hover:bg-slate-200 border border-slate-300 text-slate-700 text-xs px-4 py-1.5 rounded-lg transition font-bold"
        >
          Cari
        </button>
      </div>

      <!-- Products Table -->
      <div class="bg-white border border-slate-300 rounded-xl overflow-hidden shadow-sm">
        <table class="w-full text-left text-xs">
          <thead class="bg-slate-100 border-b border-slate-300 text-slate-700 uppercase font-bold text-[11px]">
            <tr>
              <th class="py-3 px-4">Nama Produk</th>
              <th class="py-3 px-4">Kategori</th>
              <th class="py-3 px-4">SKU / Barcode</th>
              <th class="py-3 px-4">Satuan Dasar</th>
              <th class="py-3 px-4 text-right">HPP (Cost)</th>
              <th class="py-3 px-4 text-right">Harga Jual</th>
              <th class="py-3 px-4 text-center">Stok Gudang</th>
              <th v-if="authStore.hasPermission('master:write')" class="py-3 px-4 text-center">Aksi</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-slate-200 font-mono">
            <tr v-if="isLoading">
              <td colspan="8" class="p-8 text-center text-slate-500 font-sans">Memuat master produk...</td>
            </tr>
            <tr v-else-if="products.length === 0">
              <td colspan="8" class="p-8 text-center text-slate-500 font-sans">Belum ada produk terdaftar</td>
            </tr>
            <tr v-for="prod in paginatedProducts" :key="prod.id" class="hover:bg-emerald-50/40 transition">
              <td class="py-3 px-4 font-sans">
                <div class="font-bold text-slate-900 flex items-center gap-2">
                  <span v-if="prod.is_weight_embedded" class="bg-amber-100 text-amber-800 border border-amber-300 text-[10px] px-1.5 py-0.5 rounded flex items-center gap-1 font-mono font-bold">
                    <Scale class="w-3 h-3" /> Timbang
                  </span>
                  <span>{{ prod.name }}</span>
                </div>
              </td>
              <td class="py-3 px-4 font-sans text-slate-600">{{ prod.category_name }}</td>
              <td class="py-3 px-4 text-slate-700">
                <div class="font-bold">{{ prod.sku }}</div>
                <div class="text-[10px] text-slate-500">{{ prod.barcode }}</div>
              </td>
              <td class="py-3 px-4 text-slate-700">{{ prod.unit_code }}</td>
              <td class="py-3 px-4 text-right text-slate-600">Rp {{ Number(prod.cost_price)?.toLocaleString() }}</td>
              <td class="py-3 px-4 text-right text-emerald-700 font-bold">Rp {{ Number(prod.base_selling_price)?.toLocaleString() }}</td>
              <td class="py-3 px-4 text-center">
                <span class="bg-slate-100 border border-slate-300 px-2.5 py-1 rounded text-slate-800 font-bold">
                  {{ prod.stock_qty }} {{ prod.unit_code }}
                </span>
              </td>
              <td v-if="authStore.hasPermission('master:write')" class="py-3 px-4 text-center font-sans">
                <div class="flex items-center justify-center gap-1.5">
                  <button
                    @click="openOutletPriceModal(prod)"
                    title="Atur Harga & HPP per Outlet / Cabang"
                    class="p-1.5 rounded-md bg-white hover:bg-blue-50 border border-slate-300 hover:border-blue-500 text-blue-600 transition shadow-xs flex items-center gap-1 text-[11px] font-bold"
                  >
                    <Building2 class="w-3.5 h-3.5" />
                    <span>Harga Outlet</span>
                  </button>
                  <button
                    @click="openGroupPriceModal(prod)"
                    title="Atur Harga Kelompok Pelanggan (Grosir / Member)"
                    class="p-1.5 rounded-md bg-white hover:bg-emerald-50 border border-slate-300 hover:border-emerald-500 text-emerald-600 transition shadow-xs flex items-center gap-1 text-[11px] font-bold"
                  >
                    <Users class="w-3.5 h-3.5" />
                    <span>Harga Grup</span>
                  </button>
                  <button
                    @click="openEditModal(prod)"
                    title="Edit Produk"
                    class="p-1.5 rounded-md bg-white hover:bg-slate-100 border border-slate-300 text-blue-600 transition shadow-xs"
                  >
                    <Edit2 class="w-3.5 h-3.5" />
                  </button>
                  <button
                    @click="handleDeleteProduct(prod.id, prod.name)"
                    title="Nonaktifkan Produk"
                    class="p-1.5 rounded-md bg-white hover:bg-slate-100 border border-slate-300 text-rose-600 transition shadow-xs"
                  >
                    <Trash2 class="w-3.5 h-3.5" />
                  </button>
                </div>
              </td>
            </tr>
          </tbody>
        </table>

        <!-- Desktop Pagination Bar -->
        <PaginationBar
          v-model:currentPage="currentPage"
          v-model:pageSize="pageSize"
          :totalItems="products.length"
        />
      </div>
    </div>

    <!-- Create Product Modal -->
    <div
      v-if="showCreateModal"
      class="fixed inset-0 z-50 bg-slate-900/50 backdrop-blur-xs flex items-center justify-center p-4 select-none"
    >
      <div class="bg-white border border-slate-300 w-full max-w-lg rounded-xl p-6 space-y-4 shadow-2xl">
        <h3 class="font-bold text-base text-slate-900">Tambah Master Produk Baru</h3>

        <div class="space-y-3 text-xs">
          <div>
            <label class="block text-slate-700 font-medium mb-1">Nama Produk *</label>
            <input v-model="form.name" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 focus:bg-white focus:border-emerald-600" />
          </div>

          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="block text-slate-700 font-medium mb-1">SKU *</label>
              <input v-model="form.sku" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono focus:bg-white focus:border-emerald-600" />
            </div>
            <div>
              <label class="block text-slate-700 font-medium mb-1">Barcode (EAN-13)</label>
              <input v-model="form.barcode" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono focus:bg-white focus:border-emerald-600" />
            </div>
          </div>

          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="block text-slate-700 font-medium mb-1">Kategori *</label>
              <SearchableSelect
                v-model="form.category_id"
                api-endpoint="/categories"
                placeholder="-- Pilih Kategori --"
                search-placeholder="Cari kategori barang..."
              />
            </div>
            <div>
              <label class="block text-slate-700 font-medium mb-1">Satuan Dasar (Base UoM) *</label>
              <SearchableSelect
                v-model="form.base_unit_id"
                api-endpoint="/units"
                label-key="name"
                secondary-key="code"
                placeholder="-- Pilih Satuan --"
                search-placeholder="Cari satuan (PCS, KG, DUS)..."
              />
            </div>
          </div>

          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="block text-slate-700 font-medium mb-1">Harga Beli Pokok (HPP)</label>
              <input v-model.number="form.cost_price" type="number" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono focus:bg-white focus:border-emerald-600" />
            </div>
            <div>
              <label class="block text-slate-700 font-medium mb-1">Harga Jual Dasar</label>
              <input v-model.number="form.base_selling_price" type="number" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono focus:bg-white focus:border-emerald-600" />
            </div>
          </div>

          <div class="flex items-center gap-2 pt-2">
            <input v-model="form.is_weight_embedded" type="checkbox" id="w_emb" class="w-4 h-4 rounded text-emerald-600 border-slate-300" />
            <label for="w_emb" class="text-slate-800">Produk Barcode Timbangan (Buah / Sayur Elektrik)</label>
          </div>
        </div>

        <div class="flex justify-end gap-2 pt-4 border-t border-slate-200">
          <button @click="showCreateModal = false" class="bg-slate-100 hover:bg-slate-200 border border-slate-300 text-slate-700 px-4 py-2 rounded-lg text-xs font-bold">Batal</button>
          <button @click="handleCreateProduct" class="bg-emerald-600 hover:bg-emerald-700 border border-emerald-700 text-white px-5 py-2 rounded-lg text-xs font-bold shadow-sm">Simpan Produk</button>
        </div>
      </div>
    </div>

    <!-- Edit Product Modal -->
    <div
      v-if="showEditModal"
      class="fixed inset-0 z-50 bg-slate-900/50 backdrop-blur-xs flex items-center justify-center p-4 select-none"
    >
      <div class="bg-white border border-slate-300 w-full max-w-lg rounded-xl p-6 space-y-4 shadow-2xl">
        <h3 class="font-bold text-base text-slate-900">Edit Master Produk</h3>

        <div class="space-y-3 text-xs">
          <div>
            <label class="block text-slate-700 font-medium mb-1">Nama Produk</label>
            <input v-model="editForm.name" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-semibold focus:bg-white focus:border-emerald-600" />
          </div>

          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="block text-slate-700 font-medium mb-1">SKU</label>
              <input v-model="editForm.sku" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono focus:bg-white focus:border-emerald-600" />
            </div>
            <div>
              <label class="block text-slate-700 font-medium mb-1">Barcode (EAN-13)</label>
              <input v-model="editForm.barcode" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono focus:bg-white focus:border-emerald-600" />
            </div>
          </div>

          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="block text-slate-700 font-medium mb-1">Kategori</label>
              <SearchableSelect
                v-model="editForm.category_id"
                api-endpoint="/categories"
                placeholder="-- Pilih Kategori --"
                search-placeholder="Cari kategori..."
              />
            </div>
            <div>
              <label class="block text-slate-700 font-medium mb-1">Satuan Dasar (Base UoM)</label>
              <SearchableSelect
                v-model="editForm.base_unit_id"
                api-endpoint="/units"
                label-key="name"
                secondary-key="code"
                placeholder="-- Pilih Satuan --"
                search-placeholder="Cari satuan..."
              />
            </div>
          </div>

          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="block text-slate-700 font-medium mb-1">Harga Beli Pokok (HPP)</label>
              <input v-model.number="editForm.cost_price" type="number" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono focus:bg-white focus:border-emerald-600" />
            </div>
            <div>
              <label class="block text-slate-700 font-medium mb-1">Harga Jual Dasar</label>
              <input v-model.number="editForm.base_selling_price" type="number" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono focus:bg-white focus:border-emerald-600" />
            </div>
          </div>

          <div class="grid grid-cols-3 gap-3">
            <div>
              <label class="block text-slate-700 font-medium mb-1">Min. Stok</label>
              <input v-model.number="editForm.min_stock" type="number" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono focus:bg-white focus:border-emerald-600" />
            </div>
            <div>
              <label class="block text-slate-700 font-medium mb-1">Maks. Stok</label>
              <input v-model.number="editForm.max_stock" type="number" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono focus:bg-white focus:border-emerald-600" />
            </div>
            <div>
              <label class="block text-slate-700 font-medium mb-1">PPN (%)</label>
              <input v-model.number="editForm.tax_percent" type="number" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono focus:bg-white focus:border-emerald-600" />
            </div>
          </div>

          <div class="flex items-center gap-2 pt-2">
            <input v-model="editForm.is_weight_embedded" type="checkbox" id="edit_w_emb" class="w-4 h-4 rounded text-emerald-600 border-slate-300" />
            <label for="edit_w_emb" class="text-slate-800">Produk Barcode Timbangan (Buah / Sayur Elektrik)</label>
          </div>
        </div>

        <div class="flex justify-end gap-2 pt-4 border-t border-slate-200">
          <button @click="showEditModal = false" class="bg-slate-100 hover:bg-slate-200 border border-slate-300 text-slate-700 px-4 py-2 rounded-lg text-xs font-bold">Batal</button>
          <button @click="handleUpdateProduct" class="bg-emerald-600 hover:bg-emerald-700 border border-emerald-700 text-white px-5 py-2 rounded-lg text-xs font-bold shadow-sm">Perbarui Produk</button>
        </div>
      </div>
    </div>

    <!-- Customer Group Prices Modal -->
    <div
      v-if="showGroupPriceModal"
      class="fixed inset-0 z-50 bg-slate-900/50 backdrop-blur-xs flex items-center justify-center p-4 select-none"
    >
      <div class="bg-white border border-slate-300 w-full max-w-xl rounded-2xl p-6 space-y-4 shadow-2xl text-slate-900">
        <!-- Header -->
        <div class="flex items-center justify-between border-b border-slate-200 pb-3">
          <div>
            <h3 class="font-bold text-base text-slate-900 flex items-center gap-2">
              <Users class="w-5 h-5 text-emerald-600" />
              <span>Harga per Kelompok Pelanggan</span>
            </h3>
            <p class="text-xs text-slate-500 mt-0.5">
              Produk: <strong class="text-slate-900">{{ selectedProductForGroupPrice?.name }}</strong> 
              (Harga Dasar: <span class="font-mono text-emerald-700 font-bold">Rp {{ Number(selectedProductForGroupPrice?.base_selling_price || 0).toLocaleString() }}</span>)
            </p>
          </div>
          <button @click="showGroupPriceModal = false" class="text-slate-400 hover:text-slate-600 font-bold text-lg">✕</button>
        </div>

        <div class="p-3 bg-amber-50 border border-amber-200 rounded-xl text-xs text-amber-900 flex items-start gap-2">
          <Tag class="w-4 h-4 text-amber-600 shrink-0 mt-0.5" />
          <span>
            Aturan: Jika harga khusus pada suatu kelompok diisi, harga tersebut yang otomatis berlaku di kasir saat member kelompok tersebut bertransaksi. Jika dikosongkan (atau diisi 0), otomatis berlaku <strong>Harga Jual Dasar (Rp {{ Number(selectedProductForGroupPrice?.base_selling_price || 0).toLocaleString() }})</strong>.
          </span>
        </div>

        <!-- Groups Table / List -->
        <div class="space-y-3 max-h-80 overflow-y-auto pr-1">
          <div v-if="isLoadingGroupPrices" class="py-8 text-center text-slate-500 text-xs flex items-center justify-center gap-2">
            <span>Memuat kelompok pelanggan...</span>
          </div>

          <div v-else-if="customerGroups.length === 0" class="py-8 text-center text-slate-500 text-xs">
            Belum ada kelompok pelanggan terdaftar di sistem.
          </div>

          <div
            v-else
            v-for="grp in customerGroups"
            :key="grp.id"
            class="flex items-center justify-between p-3 rounded-xl border border-slate-200 bg-slate-50/60 hover:bg-white hover:border-emerald-300 transition"
          >
            <div>
              <div class="font-bold text-xs text-slate-900 flex items-center gap-1.5">
                <span class="px-2 py-0.5 rounded bg-emerald-100 text-emerald-800 text-[10px] font-mono font-bold">{{ grp.name }}</span>
              </div>
              <p class="text-[11px] text-slate-500 mt-0.5">{{ grp.description || 'Tidak ada deskripsi' }}</p>
            </div>

            <div class="flex items-center gap-2">
              <div class="relative w-44">
                <span class="absolute left-2.5 top-1/2 -translate-y-1/2 text-slate-400 text-xs font-mono">Rp</span>
                <input
                  v-model.number="groupPriceInputs[grp.id]"
                  type="number"
                  :placeholder="String(selectedProductForGroupPrice?.base_selling_price || 'Harga dasar')"
                  class="w-full bg-white border border-slate-300 rounded-lg pl-8 pr-2.5 py-1.5 text-xs text-slate-900 font-mono font-bold text-right focus:outline-none focus:border-emerald-600"
                  @keyup.enter="handleSaveGroupPrice(grp.id)"
                />
              </div>

              <button
                @click="handleSaveGroupPrice(grp.id)"
                :disabled="isSavingGroupPrices"
                title="Simpan Harga"
                class="p-2 bg-emerald-600 hover:bg-emerald-700 disabled:opacity-50 text-white rounded-lg transition shadow-xs flex items-center justify-center"
              >
                <Check class="w-3.5 h-3.5" />
              </button>
            </div>
          </div>
        </div>

        <!-- Footer -->
        <div class="flex justify-end gap-2 pt-3 border-t border-slate-200">
          <button
            @click="showGroupPriceModal = false"
            class="bg-slate-800 hover:bg-slate-900 text-white px-5 py-2 rounded-xl text-xs font-bold transition shadow-xs"
          >
            Selesai
          </button>
        </div>
      </div>
    </div>

    <!-- Outlet-Specific Prices Modal -->
    <div
      v-if="showOutletPriceModal"
      class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/60 backdrop-blur-xs"
    >
      <div class="bg-white rounded-2xl shadow-xl w-full max-w-2xl overflow-hidden border border-slate-200 p-6 space-y-4">
        <!-- Header -->
        <div class="flex items-center justify-between border-b border-slate-200 pb-3">
          <div class="flex items-center gap-2.5">
            <div class="p-2 rounded-xl bg-blue-100 text-blue-700">
              <Building2 class="w-5 h-5" />
            </div>
            <div>
              <h3 class="font-bold text-slate-900 text-base">Harga & HPP per Outlet / Cabang</h3>
              <p class="text-xs text-slate-500">
                {{ selectedProductForOutletPrice?.name }} ({{ selectedProductForOutletPrice?.sku }})
              </p>
            </div>
          </div>
          <button
            @click="showOutletPriceModal = false"
            class="text-slate-400 hover:text-slate-600 p-1 rounded-lg hover:bg-slate-100 transition"
          >
            ✕
          </button>
        </div>

        <!-- Global Price Info Box -->
        <div class="p-3 bg-blue-50/70 border border-blue-200 rounded-xl text-xs text-blue-900 flex items-start gap-2">
          <Building2 class="w-4 h-4 text-blue-600 shrink-0 mt-0.5" />
          <div class="space-y-1">
            <p>
              Harga Global: <strong>HPP: Rp {{ Number(selectedProductForOutletPrice?.cost_price || 0).toLocaleString() }}</strong> | 
              <strong>Harga Jual: Rp {{ Number(selectedProductForOutletPrice?.base_selling_price || 0).toLocaleString() }}</strong>.
            </p>
            <p class="text-[11px] text-blue-700">
              Outlet dengan input kosong akan otomatis memakai Harga Global. Isi HPP dan Harga Jual khusus outlet di bawah lalu klik tombol simpan (✓).
            </p>
          </div>
        </div>

        <!-- Outlets Table / List -->
        <div class="space-y-3 max-h-96 overflow-y-auto pr-1">
          <div v-if="isLoadingOutletPrices" class="py-8 text-center text-slate-500 text-xs flex items-center justify-center gap-2">
            <span>Memuat data outlet...</span>
          </div>

          <div v-else-if="outletsList.length === 0" class="py-8 text-center text-slate-500 text-xs">
            Tidak ada lokasi outlet aktif yang dapat Anda kelola (Anda tidak memiliki hak akses outlet atau belum ditugaskan).
          </div>

          <div
            v-else
            v-for="outlet in outletsList"
            :key="outlet.id"
            :class="[
              'p-3 rounded-xl border transition space-y-2',
              authStore.hasOutletAccess(outlet.id)
                ? 'border-slate-200 bg-slate-50/60 hover:bg-white hover:border-blue-300'
                : 'border-slate-200 bg-slate-100/70 opacity-75'
            ]"
          >
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-2">
                <span class="px-2 py-0.5 rounded bg-blue-100 text-blue-800 text-[10px] font-mono font-bold">
                  {{ outlet.code }}
                </span>
                <span class="font-bold text-xs text-slate-900">{{ outlet.name }}</span>
                <span v-if="outlet.is_warehouse" class="text-[9px] bg-amber-100 text-amber-800 px-1.5 py-0.5 rounded font-semibold">
                  Gudang
                </span>
              </div>
              <div class="flex items-center gap-1.5">
                <span v-if="!authStore.hasOutletAccess(outlet.id)" class="text-[10px] text-amber-800 font-bold bg-amber-50 px-2 py-0.5 rounded-full border border-amber-200 flex items-center gap-1">
                  <Lock class="w-3 h-3 text-amber-600" />
                  <span>Tidak Ada Akses</span>
                </span>
                <span v-else-if="outletPriceInputs[outlet.id]?.cost_price !== undefined || outletPriceInputs[outlet.id]?.base_selling_price !== undefined" class="text-[10px] text-emerald-600 font-bold bg-emerald-50 px-2 py-0.5 rounded-full border border-emerald-200">
                  Kustom Terpasang
                </span>
              </div>
            </div>

            <div class="grid grid-cols-1 sm:grid-cols-12 gap-2 items-center">
              <!-- HPP Outlet -->
              <div class="sm:col-span-5">
                <label class="block text-[10px] font-medium text-slate-500 mb-0.5">HPP / Cost Price</label>
                <div class="relative">
                  <span class="absolute left-2 top-1/2 -translate-y-1/2 text-slate-400 text-xs font-mono">Rp</span>
                  <input
                    v-model.number="(outletPriceInputs[outlet.id] = outletPriceInputs[outlet.id] || { cost_price: null, base_selling_price: null }).cost_price"
                    type="number"
                    :disabled="!authStore.hasOutletAccess(outlet.id)"
                    :placeholder="String(selectedProductForOutletPrice?.cost_price || 0)"
                    class="w-full bg-white border border-slate-300 rounded-lg pl-7 pr-2 py-1.5 text-xs text-slate-900 font-mono font-bold text-right focus:outline-none focus:border-blue-600 disabled:bg-slate-100 disabled:text-slate-400 disabled:cursor-not-allowed"
                    @keyup.enter="authStore.hasOutletAccess(outlet.id) && handleSaveOutletPrice(outlet.id)"
                  />
                </div>
              </div>

              <!-- Harga Jual Outlet -->
              <div class="sm:col-span-5">
                <label class="block text-[10px] font-medium text-slate-500 mb-0.5">Harga Jual Dasar</label>
                <div class="relative">
                  <span class="absolute left-2 top-1/2 -translate-y-1/2 text-slate-400 text-xs font-mono">Rp</span>
                  <input
                    v-model.number="(outletPriceInputs[outlet.id] = outletPriceInputs[outlet.id] || { cost_price: null, base_selling_price: null }).base_selling_price"
                    type="number"
                    :disabled="!authStore.hasOutletAccess(outlet.id)"
                    :placeholder="String(selectedProductForOutletPrice?.base_selling_price || 0)"
                    class="w-full bg-white border border-slate-300 rounded-lg pl-7 pr-2 py-1.5 text-xs text-slate-900 font-mono font-bold text-right focus:outline-none focus:border-blue-600 disabled:bg-slate-100 disabled:text-slate-400 disabled:cursor-not-allowed"
                    @keyup.enter="authStore.hasOutletAccess(outlet.id) && handleSaveOutletPrice(outlet.id)"
                  />
                </div>
              </div>

              <!-- Save Action -->
              <div class="sm:col-span-2 flex items-end justify-end pt-3 sm:pt-0">
                <button
                  @click="handleSaveOutletPrice(outlet.id)"
                  :disabled="!authStore.hasOutletAccess(outlet.id) || isSavingOutletPrices"
                  :title="authStore.hasOutletAccess(outlet.id) ? 'Simpan Harga Outlet' : 'Anda tidak memiliki hak akses outlet ini'"
                  class="w-full py-1.5 bg-blue-600 hover:bg-blue-700 disabled:opacity-40 disabled:cursor-not-allowed text-white rounded-lg transition shadow-xs flex items-center justify-center gap-1 text-xs font-bold"
                >
                  <Lock v-if="!authStore.hasOutletAccess(outlet.id)" class="w-3.5 h-3.5" />
                  <Check v-else class="w-3.5 h-3.5" />
                  <span>{{ authStore.hasOutletAccess(outlet.id) ? 'Simpan' : 'Terkunci' }}</span>
                </button>
              </div>
            </div>
          </div>
        </div>

        <!-- Footer -->
        <div class="flex justify-end gap-2 pt-3 border-t border-slate-200">
          <button
            @click="showOutletPriceModal = false"
            class="bg-slate-800 hover:bg-slate-900 text-white px-5 py-2 rounded-xl text-xs font-bold transition shadow-xs"
          >
            Selesai
          </button>
        </div>
      </div>
    </div>
  </AdminLayout>
</template>
