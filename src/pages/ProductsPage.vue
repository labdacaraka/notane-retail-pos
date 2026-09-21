<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import AdminLayout from '@/layouts/AdminLayout.vue'
import PaginationBar from '@/components/common/PaginationBar.vue'
import SearchableSelect from '@/components/common/SearchableSelect.vue'
import { Search, Scale, Edit2, Trash2 } from 'lucide-vue-next'
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

onMounted(() => {
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
  </AdminLayout>
</template>
