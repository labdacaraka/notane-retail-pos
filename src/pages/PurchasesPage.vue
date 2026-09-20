<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import AdminLayout from '@/layouts/AdminLayout.vue'
import PaginationBar from '@/components/common/PaginationBar.vue'
import {
  Search,
  RefreshCw,
  Plus,
  Eye,
  CheckCircle2,
  AlertCircle,
  X,
  Trash2,
  Edit2,
  Truck,
  Building2,
  FileText,
  Clock,
  Check,
} from 'lucide-vue-next'
import { Icon } from '@iconify/vue'
import { api } from '@/services/api'

// Active Tab
const activeTab = ref<'invoices' | 'suppliers' | 'orders'>('invoices')

// State for Invoices (GRN)
const invoices = ref<any[]>([])
const suppliers = ref<any[]>([])
const warehouses = ref<any[]>([])
const products = ref<any[]>([])
const purchaseOrders = ref<any[]>([])
const isLoading = ref<boolean>(false)
const invoiceSearchQuery = ref<string>('')
const supplierSearchQuery = ref<string>('')

// Pagination
const invPage = ref<number>(1)
const invPageSize = ref<number>(10)
const suppPage = ref<number>(1)
const suppPageSize = ref<number>(10)

// Modal Detail Invoice
const showDetailModal = ref<boolean>(false)
const selectedInvoice = ref<any>(null)
const isLoadingDetail = ref<boolean>(false)

// Modal New GRN
const showGrnModal = ref<boolean>(false)
const isSavingGrn = ref<boolean>(false)
const grnForm = ref({
  supplier_id: '',
  supplier_invoice_number: '',
  warehouse_id: 'b0000000-0000-0000-0000-000000000002',
  invoice_date: new Date().toISOString().split('T')[0],
  due_date: new Date(Date.now() + 14 * 24 * 60 * 60 * 1000).toISOString().split('T')[0],
  notes: '',
  items: [
    {
      product_id: '',
      received_qty: 1,
      unit_price: 0,
      discount_amount: 0,
    },
  ],
})

// Modal Supplier CRUD
const showSupplierModal = ref<boolean>(false)
const isEditingSupplier = ref<boolean>(false)
const editingSupplierId = ref<string | null>(null)
const isSavingSupplier = ref<boolean>(false)
const supplierForm = ref({
  code: '',
  name: '',
  contact_person: '',
  phone: '',
  address: '',
  term_of_payment_days: 14,
})

// Toast notification
const toastMessage = ref<string>('')
const toastType = ref<'success' | 'error'>('success')

function showToast(msg: string, type: 'success' | 'error' = 'success') {
  toastMessage.value = msg
  toastType.value = type
  setTimeout(() => {
    toastMessage.value = ''
  }, 4000)
}

// Helpers
function formatRupiah(val: any): string {
  const num = Number(val) || 0
  return new Intl.NumberFormat('id-ID', {
    style: 'currency',
    currency: 'IDR',
    minimumFractionDigits: 0,
    maximumFractionDigits: 0,
  }).format(num)
}

function formatDate(dateStr: string): string {
  if (!dateStr) return '-'
  try {
    const d = new Date(dateStr)
    return new Intl.DateTimeFormat('id-ID', {
      day: '2-digit',
      month: 'short',
      year: 'numeric',
    }).format(d)
  } catch {
    return dateStr
  }
}

// Load Data
async function loadData() {
  try {
    isLoading.value = true
    const [invRes, supRes, whRes, prodRes, poRes] = await Promise.all([
      api.get('/inventory/purchases'),
      api.get('/suppliers'),
      api.get('/warehouses'),
      api.get('/products'),
      api.get('/inventory/purchase-orders'),
    ])

    if (invRes.data?.success) invoices.value = invRes.data.data
    if (supRes.data?.success) suppliers.value = supRes.data.data
    if (whRes.data?.success) warehouses.value = whRes.data.data
    if (prodRes.data?.success) products.value = prodRes.data.data
    if (poRes.data?.success) purchaseOrders.value = poRes.data.data
  } catch (err: any) {
    console.error('Gagal memuat data pengadaan:', err)
    showToast('Gagal memuat data pengadaan', 'error')
  } finally {
    isLoading.value = false
  }
}

onMounted(() => {
  loadData()
})

// Computed Filtered & Paginated
const filteredInvoices = computed(() => {
  let list = invoices.value
  if (invoiceSearchQuery.value.trim()) {
    const q = invoiceSearchQuery.value.toLowerCase().trim()
    list = list.filter(
      (it) =>
        (it.invoice_number || '').toLowerCase().includes(q) ||
        (it.supplier_invoice_number || '').toLowerCase().includes(q) ||
        (it.supplier_name || '').toLowerCase().includes(q)
    )
  }
  return list
})

const paginatedInvoices = computed(() => {
  const start = (invPage.value - 1) * invPageSize.value
  return filteredInvoices.value.slice(start, start + invPageSize.value)
})

const filteredSuppliers = computed(() => {
  let list = suppliers.value
  if (supplierSearchQuery.value.trim()) {
    const q = supplierSearchQuery.value.toLowerCase().trim()
    list = list.filter(
      (s) =>
        (s.code || '').toLowerCase().includes(q) ||
        (s.name || '').toLowerCase().includes(q) ||
        (s.contact_person || '').toLowerCase().includes(q) ||
        (s.phone || '').toLowerCase().includes(q)
    )
  }
  return list
})

const paginatedSuppliers = computed(() => {
  const start = (suppPage.value - 1) * suppPageSize.value
  return filteredSuppliers.value.slice(start, start + suppPageSize.value)
})

// GRN Items Calculation
const grnSubtotal = computed(() => {
  return grnForm.value.items.reduce((sum, it) => {
    const sub = (Number(it.received_qty) || 0) * (Number(it.unit_price) || 0) - (Number(it.discount_amount) || 0)
    return sum + Math.max(0, sub)
  }, 0)
})

function addGrnItem() {
  grnForm.value.items.push({
    product_id: '',
    received_qty: 1,
    unit_price: 0,
    discount_amount: 0,
  })
}

function removeGrnItem(idx: number) {
  if (grnForm.value.items.length > 1) {
    grnForm.value.items.splice(idx, 1)
  }
}

function onProductSelected(idx: number, productId: string) {
  grnForm.value.items[idx].product_id = productId
  const p = products.value.find((prod) => prod.id === productId)
  if (p) {
    grnForm.value.items[idx].unit_price = Number(p.cost_price) || 0
  }
}

function openCreateGrnModal() {
  grnForm.value = {
    supplier_id: suppliers.value[0]?.id || '',
    supplier_invoice_number: `SJ-${Date.now().toString().slice(-6)}`,
    warehouse_id: warehouses.value[0]?.id || 'b0000000-0000-0000-0000-000000000002',
    invoice_date: new Date().toISOString().split('T')[0],
    due_date: new Date(Date.now() + 14 * 24 * 60 * 60 * 1000).toISOString().split('T')[0],
    notes: 'Penerimaan barang masuk dari supplier',
    items: [
      {
        product_id: products.value[0]?.id || '',
        received_qty: 10,
        unit_price: Number(products.value[0]?.cost_price) || 0,
        discount_amount: 0,
      },
    ],
  }
  showGrnModal.value = true
}

async function handleSaveGrn() {
  if (!grnForm.value.supplier_id) {
    showToast('Pilih supplier terlebih dahulu', 'error')
    return
  }
  if (!grnForm.value.supplier_invoice_number) {
    showToast('No. Surat Jalan / Faktur Supplier wajib diisi', 'error')
    return
  }

  // Validate items
  for (const it of grnForm.value.items) {
    if (!it.product_id) {
      showToast('Harap pilih produk untuk setiap baris barang', 'error')
      return
    }
    if (Number(it.received_qty) <= 0) {
      showToast('Jumlah qty barang masuk harus lebih dari 0', 'error')
      return
    }
  }

  try {
    isSavingGrn.value = true
    const payload = {
      po_id: null,
      supplier_id: grnForm.value.supplier_id,
      warehouse_id: grnForm.value.warehouse_id,
      supplier_invoice_number: grnForm.value.supplier_invoice_number,
      invoice_date: grnForm.value.invoice_date,
      due_date: grnForm.value.due_date,
      notes: grnForm.value.notes,
      items: grnForm.value.items.map((it) => ({
        product_id: it.product_id,
        received_qty: it.received_qty.toString(),
        unit_price: it.unit_price.toString(),
        discount_amount: it.discount_amount ? it.discount_amount.toString() : '0',
      })),
    }

    const res = await api.post('/inventory/grn', payload)
    if (res.data?.success) {
      showToast(res.data.message || 'Faktur GRN berhasil disimpan dan stok telah bertambah!', 'success')
      showGrnModal.value = false
      loadData()
    } else {
      showToast(res.data?.message || 'Gagal menyimpan GRN', 'error')
    }
  } catch (err: any) {
    showToast(err.response?.data?.message || err.message || 'Gagal menyimpan faktur GRN', 'error')
  } finally {
    isSavingGrn.value = false
  }
}

async function viewInvoiceDetail(invoiceId: string) {
  try {
    isLoadingDetail.value = true
    showDetailModal.value = true
    const res = await api.get(`/inventory/purchases/${invoiceId}`)
    if (res.data?.success) {
      selectedInvoice.value = res.data.data
    } else {
      showToast(res.data?.message || 'Gagal memuat detail faktur', 'error')
      showDetailModal.value = false
    }
  } catch (err: any) {
    showToast(err.response?.data?.message || 'Gagal memuat detail', 'error')
    showDetailModal.value = false
  } finally {
    isLoadingDetail.value = false
  }
}

async function handlePayInvoice(invoice: any) {
  if (!confirm(`Konfirmasi pembayaran lunas untuk faktur ${invoice.invoice_number} senilai ${formatRupiah(invoice.total_amount)}?`)) {
    return
  }

  try {
    const res = await api.post(`/inventory/purchases/${invoice.id}/pay`)
    if (res.data?.success) {
      showToast(res.data.message || 'Faktur pembelian berhasil dilunasi!', 'success')
      if (showDetailModal.value && selectedInvoice.value?.id === invoice.id) {
        selectedInvoice.value.payment_status = 'PAID'
        selectedInvoice.value.paid_amount = selectedInvoice.value.total_amount
      }
      loadData()
    } else {
      showToast(res.data?.message || 'Gagal melunasi tagihan', 'error')
    }
  } catch (err: any) {
    showToast(err.response?.data?.message || err.message || 'Gagal memperbarui status bayar', 'error')
  }
}

// Supplier CRUD Actions
function openAddSupplierModal() {
  isEditingSupplier.value = false
  editingSupplierId.value = null
  supplierForm.value = {
    code: `SUP-${(suppliers.value.length + 1).toString().padStart(3, '0')}`,
    name: '',
    contact_person: '',
    phone: '',
    address: '',
    term_of_payment_days: 14,
  }
  showSupplierModal.value = true
}

function openEditSupplierModal(supp: any) {
  isEditingSupplier.value = true
  editingSupplierId.value = supp.id
  supplierForm.value = {
    code: supp.code || '',
    name: supp.name || '',
    contact_person: supp.contact_person || '',
    phone: supp.phone || '',
    address: supp.address || '',
    term_of_payment_days: supp.term_of_payment_days || 14,
  }
  showSupplierModal.value = true
}

async function handleSaveSupplier() {
  if (!supplierForm.value.name) {
    showToast('Nama supplier wajib diisi', 'error')
    return
  }

  try {
    isSavingSupplier.value = true
    if (isEditingSupplier.value && editingSupplierId.value) {
      const res = await api.put(`/suppliers/${editingSupplierId.value}`, supplierForm.value)
      if (res.data?.success) {
        showToast('Data supplier berhasil diperbarui', 'success')
        showSupplierModal.value = false
        loadData()
      } else {
        showToast(res.data?.message || 'Gagal memperbarui supplier', 'error')
      }
    } else {
      const res = await api.post('/suppliers', supplierForm.value)
      if (res.data?.success) {
        showToast('Supplier baru berhasil ditambahkan', 'success')
        showSupplierModal.value = false
        loadData()
      } else {
        showToast(res.data?.message || 'Gagal menambahkan supplier', 'error')
      }
    }
  } catch (err: any) {
    showToast(err.response?.data?.message || err.message || 'Gagal menyimpan data supplier', 'error')
  } finally {
    isSavingSupplier.value = false
  }
}

async function handleDeleteSupplier(supp: any) {
  if (!confirm(`Hapus pemasok "${supp.name}" (${supp.code})?`)) return

  try {
    const res = await api.delete(`/suppliers/${supp.id}`)
    if (res.data?.success) {
      showToast('Pemasok berhasil dihapus', 'success')
      loadData()
    } else {
      showToast(res.data?.message || 'Gagal menghapus pemasok', 'error')
    }
  } catch (err: any) {
    showToast(err.response?.data?.message || err.message || 'Gagal menghapus pemasok', 'error')
  }
}
</script>

<template>
  <AdminLayout>
    <div class="h-full flex flex-col bg-slate-100 overflow-hidden">
      <!-- Toast Notification -->
      <transition
        enter-active-class="transition duration-300 ease-out"
        enter-from-class="transform -translate-y-4 opacity-0"
        enter-to-class="transform translate-y-0 opacity-100"
        leave-active-class="transition duration-200 ease-in"
        leave-from-class="transform translate-y-0 opacity-100"
        leave-to-class="transform -translate-y-4 opacity-0"
      >
        <div
          v-if="toastMessage"
          class="fixed top-20 right-6 z-50 flex items-center gap-2.5 px-4 py-3 rounded-xl shadow-xl text-xs font-bold border backdrop-blur-md"
          :class="toastType === 'success'
            ? 'bg-emerald-500/90 text-white border-emerald-400'
            : 'bg-rose-500/90 text-white border-rose-400'"
        >
          <CheckCircle2 v-if="toastType === 'success'" class="w-4 h-4 shrink-0" />
          <AlertCircle v-else class="w-4 h-4 shrink-0" />
          <span>{{ toastMessage }}</span>
        </div>
      </transition>

      <!-- 1. Header Toolbar -->
      <div class="bg-white border-b border-slate-300 px-6 py-4 shrink-0 flex items-center justify-between shadow-xs">
        <div class="flex items-center gap-3.5">
          <div class="w-11 h-11 rounded-xl bg-blue-50 border border-blue-300 flex items-center justify-center shadow-xs">
            <Icon icon="streamline-ultimate-color:delivery-truck-cargo" class="w-7 h-7" />
          </div>
          <div>
            <h1 class="text-lg font-black text-slate-900 tracking-tight flex items-center gap-2">
              <span>Pengadaan & Pembelian (Procurement)</span>
              <span class="text-[10px] bg-blue-100 text-blue-900 font-mono px-2 py-0.5 rounded-full border border-blue-300 font-bold">
                SUPPLY CHAIN
              </span>
            </h1>
            <p class="text-xs text-slate-500 mt-0.5">
              Penerimaan faktur masuk (GRN), pencatatan tagihan hutang supplier, pesanan PO & direktori pemasok
            </p>
          </div>
        </div>

        <div class="flex items-center gap-2.5">
          <button
            type="button"
            @click="loadData"
            :disabled="isLoading"
            class="px-3.5 py-2 rounded-lg border border-slate-300 bg-white hover:bg-slate-50 text-slate-700 font-bold text-xs flex items-center gap-2 transition active:scale-95 cursor-pointer shadow-xs"
            title="Muat Ulang Data"
          >
            <RefreshCw class="w-3.5 h-3.5" :class="{ 'animate-spin': isLoading }" />
            <span>Segarkan</span>
          </button>

          <button
            v-if="activeTab === 'invoices'"
            type="button"
            @click="openCreateGrnModal"
            class="px-4 py-2 rounded-lg bg-blue-600 hover:bg-blue-700 text-white font-bold text-xs flex items-center gap-2 transition active:scale-95 cursor-pointer shadow-xs border border-blue-700"
          >
            <Plus class="w-4 h-4" />
            <span>+ Penerimaan GRN Baru</span>
          </button>

          <button
            v-else-if="activeTab === 'suppliers'"
            type="button"
            @click="openAddSupplierModal"
            class="px-4 py-2 rounded-lg bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-xs flex items-center gap-2 transition active:scale-95 cursor-pointer shadow-xs border border-emerald-700"
          >
            <Plus class="w-4 h-4" />
            <span>+ Tambah Supplier Baru</span>
          </button>
        </div>
      </div>

      <!-- 2. Navigation Tabs Bar -->
      <div class="px-6 pt-3 shrink-0 flex items-center gap-2 border-b border-slate-200 bg-white">
        <button
          type="button"
          @click="activeTab = 'invoices'"
          class="flex items-center gap-2 px-4 py-2.5 font-bold text-xs border-b-2 transition cursor-pointer"
          :class="activeTab === 'invoices'
            ? 'border-blue-600 text-blue-800'
            : 'border-transparent text-slate-500 hover:text-slate-900'"
        >
          <FileText class="w-4 h-4" />
          <span>Faktur Pembelian Masuk (GRN)</span>
          <span class="ml-1 text-[10px] px-2 py-0.5 rounded-full bg-slate-100 text-slate-700 font-mono font-bold">
            {{ invoices.length }}
          </span>
        </button>

        <button
          type="button"
          @click="activeTab = 'suppliers'"
          class="flex items-center gap-2 px-4 py-2.5 font-bold text-xs border-b-2 transition cursor-pointer"
          :class="activeTab === 'suppliers'
            ? 'border-blue-600 text-blue-800'
            : 'border-transparent text-slate-500 hover:text-slate-900'"
        >
          <Building2 class="w-4 h-4" />
          <span>Master Pemasok / Supplier</span>
          <span class="ml-1 text-[10px] px-2 py-0.5 rounded-full bg-slate-100 text-slate-700 font-mono font-bold">
            {{ suppliers.length }}
          </span>
        </button>

        <button
          type="button"
          @click="activeTab = 'orders'"
          class="flex items-center gap-2 px-4 py-2.5 font-bold text-xs border-b-2 transition cursor-pointer"
          :class="activeTab === 'orders'
            ? 'border-blue-600 text-blue-800'
            : 'border-transparent text-slate-500 hover:text-slate-900'"
        >
          <Clock class="w-4 h-4" />
          <span>Pesanan Pembelian (PO)</span>
          <span class="ml-1 text-[10px] px-2 py-0.5 rounded-full bg-slate-100 text-slate-700 font-mono font-bold">
            {{ purchaseOrders.length }}
          </span>
        </button>
      </div>

      <!-- 3. Tab Content -->
      <div class="flex-1 px-6 py-3 min-h-0 flex flex-col">
        <!-- TAB 1: FAKTUR PEMBELIAN MASUK (GRN) -->
        <div v-if="activeTab === 'invoices'" class="bg-white border border-slate-300 rounded-xl flex flex-col h-full shadow-xs overflow-hidden">
          <!-- Filter Controls -->
          <div class="px-4 py-3 border-b border-slate-200 flex items-center justify-between gap-3 bg-slate-50/70 shrink-0">
            <div class="relative flex-1 max-w-md">
              <Search class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-slate-400" />
              <input
                v-model="invoiceSearchQuery"
                type="text"
                placeholder="Cari no. faktur internal, surat jalan supplier, nama pemasok..."
                class="w-full pl-9 pr-3.5 py-1.5 rounded-lg border border-slate-300 bg-white text-xs text-slate-800 placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-blue-500 transition"
              />
            </div>
            <div class="text-xs text-slate-500 font-bold">
              Total Tagihan: <span class="font-mono text-slate-900">{{ invoices.length }} Faktur</span>
            </div>
          </div>

          <!-- Invoices Table -->
          <div class="flex-1 overflow-auto min-h-0">
            <table class="w-full text-left text-xs text-slate-700">
              <thead class="bg-slate-50 border-b border-slate-200 text-[11px] font-bold text-slate-500 uppercase tracking-wider sticky top-0 z-10">
                <tr>
                  <th class="px-4 py-2.5">No. Faktur GRN</th>
                  <th class="px-4 py-2.5">Faktur / SJ Supplier</th>
                  <th class="px-4 py-2.5">Pemasok (Supplier)</th>
                  <th class="px-4 py-2.5">Tgl Faktur</th>
                  <th class="px-4 py-2.5">Jatuh Tempo</th>
                  <th class="px-4 py-2.5 text-right">Total Tagihan</th>
                  <th class="px-4 py-2.5 text-center">Status Bayar</th>
                  <th class="px-4 py-2.5 text-center">Aksi</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-slate-200">
                <tr
                  v-for="inv in paginatedInvoices"
                  :key="inv.id"
                  class="hover:bg-blue-50/40 transition group"
                >
                  <td class="px-4 py-3 font-mono font-bold text-slate-900 whitespace-nowrap">
                    {{ inv.invoice_number }}
                  </td>
                  <td class="px-4 py-3 font-mono text-slate-600">
                    {{ inv.supplier_invoice_number }}
                  </td>
                  <td class="px-4 py-3 font-bold text-slate-800">
                    {{ inv.supplier_name }}
                  </td>
                  <td class="px-4 py-3 text-slate-600 whitespace-nowrap">
                    {{ formatDate(inv.invoice_date) }}
                  </td>
                  <td class="px-4 py-3 text-slate-600 whitespace-nowrap">
                    {{ formatDate(inv.due_date) }}
                  </td>
                  <td class="px-4 py-3 text-right font-mono font-black text-slate-900 whitespace-nowrap">
                    {{ formatRupiah(inv.total_amount) }}
                  </td>
                  <td class="px-4 py-3 text-center whitespace-nowrap">
                    <span
                      v-if="inv.payment_status === 'PAID'"
                      class="px-2.5 py-0.5 rounded-full text-[10px] font-extrabold bg-emerald-100 text-emerald-800 border border-emerald-300"
                    >
                      LUNAS
                    </span>
                    <span
                      v-else
                      class="px-2.5 py-0.5 rounded-full text-[10px] font-extrabold bg-amber-100 text-amber-900 border border-amber-300"
                    >
                      BELUM LUNAS
                    </span>
                  </td>
                  <td class="px-4 py-3 text-center whitespace-nowrap">
                    <div class="flex items-center justify-center gap-1.5">
                      <button
                        type="button"
                        @click="viewInvoiceDetail(inv.id)"
                        class="p-1.5 rounded-lg border border-slate-300 bg-white hover:bg-slate-100 text-blue-600 transition cursor-pointer shadow-2xs"
                        title="Lihat Detail Faktur"
                      >
                        <Eye class="w-3.5 h-3.5" />
                      </button>

                      <button
                        v-if="inv.payment_status !== 'PAID'"
                        type="button"
                        @click="handlePayInvoice(inv)"
                        class="px-2.5 py-1 rounded-lg border border-emerald-300 bg-white hover:bg-emerald-50 text-emerald-700 font-bold text-[11px] flex items-center gap-1 transition cursor-pointer shadow-2xs"
                        title="Tandai Sudah Dibayar"
                      >
                        <Check class="w-3.5 h-3.5" />
                        <span>Bayar</span>
                      </button>
                    </div>
                  </td>
                </tr>

                <tr v-if="paginatedInvoices.length === 0 && !isLoading">
                  <td colspan="8" class="px-6 py-12 text-center text-slate-400">
                    <FileText class="w-12 h-12 mx-auto mb-2 opacity-30 text-slate-400" />
                    <p class="font-bold text-sm text-slate-600">Belum ada faktur penerimaan pembelian (GRN)</p>
                    <p class="text-xs text-slate-400 mt-0.5">Klik tombol "+ Penerimaan GRN Baru" untuk mencatat barang masuk</p>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <!-- Pagination Bar -->
          <div class="px-4 py-2.5 border-t border-slate-200 bg-slate-50 shrink-0">
            <PaginationBar
              v-model:currentPage="invPage"
              v-model:pageSize="invPageSize"
              :totalItems="filteredInvoices.length"
            />
          </div>
        </div>

        <!-- TAB 2: MASTER PEMASOK / SUPPLIER -->
        <div v-else-if="activeTab === 'suppliers'" class="bg-white border border-slate-300 rounded-xl flex flex-col h-full shadow-xs overflow-hidden">
          <!-- Filter Controls -->
          <div class="px-4 py-3 border-b border-slate-200 flex items-center justify-between gap-3 bg-slate-50/70 shrink-0">
            <div class="relative flex-1 max-w-md">
              <Search class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-slate-400" />
              <input
                v-model="supplierSearchQuery"
                type="text"
                placeholder="Cari kode, nama pemasok, kontak person, telepon..."
                class="w-full pl-9 pr-3.5 py-1.5 rounded-lg border border-slate-300 bg-white text-xs text-slate-800 placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-emerald-500 transition"
              />
            </div>
            <div class="text-xs text-slate-500 font-bold">
              Total Pemasok: <span class="font-mono text-slate-900">{{ suppliers.length }} Vendor</span>
            </div>
          </div>

          <!-- Suppliers Table -->
          <div class="flex-1 overflow-auto min-h-0">
            <table class="w-full text-left text-xs text-slate-700">
              <thead class="bg-slate-50 border-b border-slate-200 text-[11px] font-bold text-slate-500 uppercase tracking-wider sticky top-0 z-10">
                <tr>
                  <th class="px-4 py-2.5">Kode</th>
                  <th class="px-4 py-2.5">Nama Pemasok / PT</th>
                  <th class="px-4 py-2.5">Kontak Person</th>
                  <th class="px-4 py-2.5">No. Telepon</th>
                  <th class="px-4 py-2.5 text-center">Term Pembayaran</th>
                  <th class="px-4 py-2.5">Alamat Kantor</th>
                  <th class="px-4 py-2.5 text-center">Aksi</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-slate-200">
                <tr
                  v-for="supp in paginatedSuppliers"
                  :key="supp.id"
                  class="hover:bg-emerald-50/30 transition group"
                >
                  <td class="px-4 py-3 font-mono font-bold text-slate-900 whitespace-nowrap">
                    {{ supp.code }}
                  </td>
                  <td class="px-4 py-3 font-bold text-slate-900">
                    {{ supp.name }}
                  </td>
                  <td class="px-4 py-3 text-slate-700">
                    {{ supp.contact_person || '-' }}
                  </td>
                  <td class="px-4 py-3 font-mono text-slate-600">
                    {{ supp.phone || '-' }}
                  </td>
                  <td class="px-4 py-3 text-center font-bold text-slate-800">
                    <span class="px-2 py-0.5 rounded bg-slate-100 border border-slate-300 font-mono">
                      {{ supp.term_of_payment_days }} Hari
                    </span>
                  </td>
                  <td class="px-4 py-3 text-slate-600 max-w-xs truncate">
                    {{ supp.address || '-' }}
                  </td>
                  <td class="px-4 py-3 text-center whitespace-nowrap">
                    <div class="flex items-center justify-center gap-1.5">
                      <button
                        type="button"
                        @click="openEditSupplierModal(supp)"
                        class="p-1.5 rounded-lg border border-slate-300 bg-white hover:bg-slate-100 text-amber-700 transition cursor-pointer shadow-2xs"
                        title="Edit Data Pemasok"
                      >
                        <Edit2 class="w-3.5 h-3.5" />
                      </button>
                      <button
                        type="button"
                        @click="handleDeleteSupplier(supp)"
                        class="p-1.5 rounded-lg border border-slate-300 bg-white hover:bg-rose-50 text-rose-600 transition cursor-pointer shadow-2xs hover:border-rose-300"
                        title="Hapus Pemasok"
                      >
                        <Trash2 class="w-3.5 h-3.5" />
                      </button>
                    </div>
                  </td>
                </tr>

                <tr v-if="paginatedSuppliers.length === 0 && !isLoading">
                  <td colspan="7" class="px-6 py-12 text-center text-slate-400">
                    <Building2 class="w-12 h-12 mx-auto mb-2 opacity-30 text-slate-400" />
                    <p class="font-bold text-sm text-slate-600">Tidak ada pemasok ditemukan</p>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <!-- Pagination Bar -->
          <div class="px-4 py-2.5 border-t border-slate-200 bg-slate-50 shrink-0">
            <PaginationBar
              v-model:currentPage="suppPage"
              v-model:pageSize="suppPageSize"
              :totalItems="filteredSuppliers.length"
            />
          </div>
        </div>

        <!-- TAB 3: PESANAN PEMBELIAN (PO) -->
        <div v-else-if="activeTab === 'orders'" class="bg-white border border-slate-300 rounded-xl flex flex-col h-full shadow-xs overflow-hidden">
          <div class="px-4 py-3 border-b border-slate-200 flex items-center justify-between bg-slate-50/70 shrink-0">
            <div class="font-bold text-xs text-slate-700">Daftar Purchase Order (PO) Aktif</div>
            <div class="text-xs text-slate-500 font-mono">{{ purchaseOrders.length }} PO Tercatat</div>
          </div>

          <div class="flex-1 overflow-auto min-h-0">
            <table class="w-full text-left text-xs text-slate-700">
              <thead class="bg-slate-50 border-b border-slate-200 text-[11px] font-bold text-slate-500 uppercase tracking-wider sticky top-0 z-10">
                <tr>
                  <th class="px-4 py-2.5">No. PO</th>
                  <th class="px-4 py-2.5">Pemasok</th>
                  <th class="px-4 py-2.5">Tanggal Order</th>
                  <th class="px-4 py-2.5">Estimasi Tiba</th>
                  <th class="px-4 py-2.5 text-right">Nilai Total PO</th>
                  <th class="px-4 py-2.5 text-center">Status</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-slate-200 font-mono">
                <tr v-for="po in purchaseOrders" :key="po.id" class="hover:bg-slate-50">
                  <td class="px-4 py-3 font-bold text-slate-900">{{ po.po_number }}</td>
                  <td class="px-4 py-3 font-sans font-bold text-slate-800">{{ po.supplier_name }}</td>
                  <td class="px-4 py-3 text-slate-600">{{ formatDate(po.order_date) }}</td>
                  <td class="px-4 py-3 text-slate-600">{{ formatDate(po.expected_delivery) }}</td>
                  <td class="px-4 py-3 text-right font-black text-slate-900">{{ formatRupiah(po.total_amount) }}</td>
                  <td class="px-4 py-3 text-center">
                    <span class="px-2 py-0.5 rounded text-[10px] font-bold bg-slate-100 text-slate-800 border border-slate-300">
                      {{ po.status }}
                    </span>
                  </td>
                </tr>
                <tr v-if="purchaseOrders.length === 0">
                  <td colspan="6" class="px-6 py-12 text-center text-slate-400 font-sans">
                    <Clock class="w-12 h-12 mx-auto mb-2 opacity-30 text-slate-400" />
                    <p class="font-bold text-sm text-slate-600">Belum ada dokumen Purchase Order (PO)</p>
                    <p class="text-xs text-slate-400 mt-0.5">PO baru dapat dibuat saat stok produk menipis di bawah ambang minimum</p>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>

      <!-- 4. Modal Buat GRN Baru -->
      <div
        v-if="showGrnModal"
        class="fixed inset-0 z-50 bg-slate-900/60 backdrop-blur-xs flex items-center justify-center p-4"
        @click.self="showGrnModal = false"
      >
        <div class="bg-white rounded-2xl shadow-2xl border border-slate-300 w-full max-w-4xl overflow-hidden flex flex-col max-h-[92vh] animate-in fade-in zoom-in-95 duration-150">
          <!-- Header -->
          <div class="px-6 py-4 bg-slate-900 text-white flex items-center justify-between shrink-0">
            <div class="flex items-center gap-3">
              <div class="w-9 h-9 rounded-xl bg-blue-500/20 border border-blue-400/40 flex items-center justify-center">
                <Truck class="w-5 h-5 text-blue-400" />
              </div>
              <div>
                <div class="text-xs font-bold text-slate-400 uppercase tracking-wider">Formulir Penerimaan Barang Masuk</div>
                <div class="text-base font-black tracking-tight">Catat Faktur Pembelian (GRN) Baru</div>
              </div>
            </div>
            <button
              type="button"
              @click="showGrnModal = false"
              class="text-slate-400 hover:text-white p-1 rounded-lg hover:bg-slate-800 transition cursor-pointer"
            >
              <X class="w-5 h-5" />
            </button>
          </div>

          <!-- Body -->
          <div class="p-6 overflow-y-auto space-y-5 flex-1 min-h-0 text-xs">
            <!-- Header Inputs -->
            <div class="grid grid-cols-1 sm:grid-cols-3 gap-3.5 bg-slate-50 p-4 rounded-xl border border-slate-200">
              <div>
                <label class="block font-bold text-slate-700 mb-1">Pemasok / Supplier *</label>
                <select
                  v-model="grnForm.supplier_id"
                  class="w-full px-3 py-2 rounded-lg border border-slate-300 bg-white font-sans text-xs focus:ring-2 focus:ring-blue-500 focus:outline-none font-bold text-slate-800"
                >
                  <option v-for="s in suppliers" :key="s.id" :value="s.id">
                    {{ s.name }} ({{ s.code }})
                  </option>
                </select>
              </div>

              <div>
                <label class="block font-bold text-slate-700 mb-1">No. Surat Jalan / Faktur Supplier *</label>
                <input
                  v-model="grnForm.supplier_invoice_number"
                  type="text"
                  placeholder="Contoh: SJ-2026/09/8812"
                  class="w-full px-3 py-2 rounded-lg border border-slate-300 bg-white font-mono text-xs focus:ring-2 focus:ring-blue-500 focus:outline-none"
                />
              </div>

              <div>
                <label class="block font-bold text-slate-700 mb-1">Gudang Penerima *</label>
                <select
                  v-model="grnForm.warehouse_id"
                  class="w-full px-3 py-2 rounded-lg border border-slate-300 bg-white font-sans text-xs focus:ring-2 focus:ring-blue-500 focus:outline-none font-bold text-slate-800"
                >
                  <option v-for="w in warehouses" :key="w.id" :value="w.id">
                    {{ w.name }} ({{ w.code }})
                  </option>
                </select>
              </div>

              <div>
                <label class="block font-bold text-slate-700 mb-1">Tanggal Faktur *</label>
                <input
                  v-model="grnForm.invoice_date"
                  type="date"
                  class="w-full px-3 py-2 rounded-lg border border-slate-300 bg-white font-sans text-xs focus:ring-2 focus:ring-blue-500 focus:outline-none"
                />
              </div>

              <div>
                <label class="block font-bold text-slate-700 mb-1">Tanggal Jatuh Tempo *</label>
                <input
                  v-model="grnForm.due_date"
                  type="date"
                  class="w-full px-3 py-2 rounded-lg border border-slate-300 bg-white font-sans text-xs focus:ring-2 focus:ring-blue-500 focus:outline-none"
                />
              </div>

              <div>
                <label class="block font-bold text-slate-700 mb-1">Catatan Pengiriman</label>
                <input
                  v-model="grnForm.notes"
                  type="text"
                  placeholder="Catatan surat jalan..."
                  class="w-full px-3 py-2 rounded-lg border border-slate-300 bg-white font-sans text-xs focus:ring-2 focus:ring-blue-500 focus:outline-none"
                />
              </div>
            </div>

            <!-- Items Table -->
            <div>
              <div class="flex items-center justify-between mb-2">
                <div class="font-bold text-slate-800 uppercase tracking-wide">
                  Daftar Barang Masuk yang Diterima
                </div>
                <button
                  type="button"
                  @click="addGrnItem"
                  class="px-2.5 py-1 rounded-lg bg-blue-50 hover:bg-blue-100 text-blue-700 border border-blue-200 font-bold text-xs flex items-center gap-1 transition cursor-pointer"
                >
                  <Plus class="w-3.5 h-3.5" />
                  <span>Tambah Baris Produk</span>
                </button>
              </div>

              <div class="border border-slate-200 rounded-xl overflow-hidden">
                <table class="w-full text-left text-xs text-slate-700">
                  <thead class="bg-slate-100 text-[10px] font-bold text-slate-500 uppercase border-b border-slate-200">
                    <tr>
                      <th class="px-3 py-2">Produk</th>
                      <th class="px-3 py-2 w-28 text-right">Qty Masuk</th>
                      <th class="px-3 py-2 w-36 text-right">Harga Beli Satuan (Rp)</th>
                      <th class="px-3 py-2 w-32 text-right">Diskon (Rp)</th>
                      <th class="px-3 py-2 text-right w-36">Subtotal</th>
                      <th class="px-3 py-2 text-center w-12">Hapus</th>
                    </tr>
                  </thead>
                  <tbody class="divide-y divide-slate-100">
                    <tr v-for="(it, idx) in grnForm.items" :key="idx" class="hover:bg-slate-50/80">
                      <!-- Product Selector -->
                      <td class="px-3 py-2">
                        <select
                          v-model="it.product_id"
                          @change="onProductSelected(idx, it.product_id)"
                          class="w-full px-2 py-1.5 rounded-lg border border-slate-300 bg-white font-sans text-xs focus:ring-2 focus:ring-blue-500 focus:outline-none font-bold"
                        >
                          <option value="" disabled>-- Pilih Produk --</option>
                          <option v-for="p in products" :key="p.id" :value="p.id">
                            {{ p.name }} ({{ p.sku }}) - {{ p.unit_code }}
                          </option>
                        </select>
                      </td>

                      <!-- Qty -->
                      <td class="px-3 py-2">
                        <input
                          v-model.number="it.received_qty"
                          type="number"
                          step="any"
                          min="0.001"
                          class="w-full px-2 py-1.5 rounded-lg border border-slate-300 text-right font-mono text-xs focus:ring-2 focus:ring-blue-500 focus:outline-none font-bold"
                        />
                      </td>

                      <!-- Price -->
                      <td class="px-3 py-2">
                        <input
                          v-model.number="it.unit_price"
                          type="number"
                          step="100"
                          min="0"
                          class="w-full px-2 py-1.5 rounded-lg border border-slate-300 text-right font-mono text-xs focus:ring-2 focus:ring-blue-500 focus:outline-none"
                        />
                      </td>

                      <!-- Discount -->
                      <td class="px-3 py-2">
                        <input
                          v-model.number="it.discount_amount"
                          type="number"
                          step="100"
                          min="0"
                          class="w-full px-2 py-1.5 rounded-lg border border-slate-300 text-right font-mono text-xs focus:ring-2 focus:ring-blue-500 focus:outline-none"
                        />
                      </td>

                      <!-- Subtotal -->
                      <td class="px-3 py-2 text-right font-mono font-black text-slate-900 whitespace-nowrap">
                        {{
                          formatRupiah(
                            Math.max(0, (Number(it.received_qty) || 0) * (Number(it.unit_price) || 0) - (Number(it.discount_amount) || 0))
                          )
                        }}
                      </td>

                      <!-- Remove -->
                      <td class="px-3 py-2 text-center">
                        <button
                          type="button"
                          @click="removeGrnItem(idx)"
                          :disabled="grnForm.items.length <= 1"
                          class="p-1 rounded text-rose-500 hover:bg-rose-50 disabled:opacity-30 cursor-pointer"
                        >
                          <Trash2 class="w-3.5 h-3.5" />
                        </button>
                      </td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>

            <!-- Total Ringkasan -->
            <div class="flex justify-end">
              <div class="w-72 bg-blue-50/60 p-4 rounded-xl border border-blue-200 font-mono space-y-1.5">
                <div class="flex justify-between text-slate-600 text-xs">
                  <span>Total Tagihan GRN:</span>
                  <span class="font-black text-slate-900 text-sm">{{ formatRupiah(grnSubtotal) }}</span>
                </div>
                <div class="text-[10px] text-blue-700 font-sans italic">
                  * Otomatis menambah stok gudang & mencatat jurnal hutang dagang supplier
                </div>
              </div>
            </div>
          </div>

          <!-- Footer -->
          <div class="px-6 py-3.5 bg-slate-50 border-t border-slate-200 flex items-center justify-end gap-2.5 shrink-0">
            <button
              type="button"
              @click="showGrnModal = false"
              :disabled="isSavingGrn"
              class="px-4 py-2 rounded-lg border border-slate-300 bg-white hover:bg-slate-100 text-slate-700 font-bold text-xs transition cursor-pointer"
            >
              Batal
            </button>
            <button
              type="button"
              @click="handleSaveGrn"
              :disabled="isSavingGrn"
              class="px-5 py-2 rounded-lg bg-blue-600 hover:bg-blue-700 text-white font-bold text-xs flex items-center gap-2 shadow-xs transition active:scale-95 cursor-pointer disabled:opacity-50"
            >
              <RefreshCw v-if="isSavingGrn" class="w-4 h-4 animate-spin" />
              <Check v-else class="w-4 h-4" />
              <span>Simpan Penerimaan GRN</span>
            </button>
          </div>
        </div>
      </div>

      <!-- 5. Modal Detail Faktur GRN -->
      <div
        v-if="showDetailModal"
        class="fixed inset-0 z-50 bg-slate-900/60 backdrop-blur-xs flex items-center justify-center p-4"
        @click.self="showDetailModal = false"
      >
        <div class="bg-white rounded-2xl shadow-2xl border border-slate-300 w-full max-w-3xl overflow-hidden flex flex-col max-h-[90vh] animate-in fade-in zoom-in-95 duration-150">
          <div class="px-6 py-4 bg-slate-900 text-white flex items-center justify-between shrink-0">
            <div class="flex items-center gap-3">
              <div class="w-9 h-9 rounded-xl bg-blue-500/20 border border-blue-400/40 flex items-center justify-center">
                <FileText class="w-5 h-5 text-blue-400" />
              </div>
              <div>
                <div class="text-xs font-bold text-slate-400 uppercase tracking-wider">Detail Faktur Penerimaan (GRN)</div>
                <div class="text-base font-black tracking-tight flex items-center gap-2">
                  <span>{{ selectedInvoice?.invoice_number }}</span>
                  <span
                    v-if="selectedInvoice?.payment_status === 'PAID'"
                    class="text-[10px] bg-emerald-500/20 text-emerald-300 px-2 py-0.5 rounded border border-emerald-400/30 font-bold"
                  >
                    LUNAS
                  </span>
                  <span
                    v-else
                    class="text-[10px] bg-amber-500/20 text-amber-300 px-2 py-0.5 rounded border border-amber-400/30 font-bold"
                  >
                    BELUM LUNAS
                  </span>
                </div>
              </div>
            </div>
            <button
              type="button"
              @click="showDetailModal = false"
              class="text-slate-400 hover:text-white p-1 rounded-lg hover:bg-slate-800 transition cursor-pointer"
            >
              <X class="w-5 h-5" />
            </button>
          </div>

          <div class="p-6 overflow-y-auto space-y-5 flex-1 min-h-0 text-xs">
            <div v-if="isLoadingDetail" class="py-12 text-center text-slate-400">
              <RefreshCw class="w-8 h-8 mx-auto mb-2 text-blue-500 animate-spin" />
              <p class="font-bold">Memuat rincian faktur...</p>
            </div>

            <div v-else-if="selectedInvoice" class="space-y-5">
              <div class="grid grid-cols-2 sm:grid-cols-4 gap-3 bg-slate-50 p-3.5 rounded-xl border border-slate-200">
                <div>
                  <div class="text-[10px] font-bold text-slate-400 uppercase">Pemasok (Supplier)</div>
                  <div class="font-bold text-slate-800 mt-0.5">{{ selectedInvoice.supplier_name }}</div>
                </div>
                <div>
                  <div class="text-[10px] font-bold text-slate-400 uppercase">Faktur / SJ Supplier</div>
                  <div class="font-mono font-bold text-slate-800 mt-0.5">{{ selectedInvoice.supplier_invoice_number }}</div>
                </div>
                <div>
                  <div class="text-[10px] font-bold text-slate-400 uppercase">Tanggal Faktur</div>
                  <div class="font-bold text-slate-800 mt-0.5">{{ formatDate(selectedInvoice.invoice_date) }}</div>
                </div>
                <div>
                  <div class="text-[10px] font-bold text-slate-400 uppercase">Jatuh Tempo</div>
                  <div class="font-bold text-rose-700 mt-0.5">{{ formatDate(selectedInvoice.due_date) }}</div>
                </div>
              </div>

              <div>
                <div class="text-xs font-bold text-slate-800 mb-2 uppercase tracking-wide">
                  Item Barang yang Diterima
                </div>
                <div class="border border-slate-200 rounded-xl overflow-hidden">
                  <table class="w-full text-left text-xs text-slate-700">
                    <thead class="bg-slate-100 text-[10px] font-bold text-slate-500 uppercase border-b border-slate-200">
                      <tr>
                        <th class="px-3 py-2">Produk</th>
                        <th class="px-3 py-2 text-right">Qty Diterima</th>
                        <th class="px-3 py-2 text-right">Harga Satuan Beli</th>
                        <th class="px-3 py-2 text-right">Diskon</th>
                        <th class="px-3 py-2 text-right">Subtotal</th>
                      </tr>
                    </thead>
                    <tbody class="divide-y divide-slate-100 font-mono">
                      <tr v-for="(it, idx) in selectedInvoice.items" :key="idx" class="hover:bg-slate-50/80">
                        <td class="px-3 py-2.5 font-sans font-bold text-slate-900">
                          {{ it.product_name }}
                          <div class="text-[10px] font-normal text-slate-400 font-mono">{{ it.product_sku }}</div>
                        </td>
                        <td class="px-3 py-2.5 text-right font-bold text-slate-800">
                          {{ it.received_qty }} {{ it.unit_code }}
                        </td>
                        <td class="px-3 py-2.5 text-right text-slate-600">
                          {{ formatRupiah(it.unit_price) }}
                        </td>
                        <td class="px-3 py-2.5 text-right text-rose-600">
                          {{ Number(it.discount_amount) > 0 ? `-${formatRupiah(it.discount_amount)}` : '-' }}
                        </td>
                        <td class="px-3 py-2.5 text-right font-black text-slate-900">
                          {{ formatRupiah(it.subtotal) }}
                        </td>
                      </tr>
                    </tbody>
                  </table>
                </div>
              </div>

              <div class="flex justify-end">
                <div class="w-80 bg-slate-50 p-4 rounded-xl border border-slate-200 space-y-2 font-mono">
                  <div class="flex justify-between text-slate-600">
                    <span>Subtotal:</span>
                    <span>{{ formatRupiah(selectedInvoice.subtotal) }}</span>
                  </div>
                  <div class="flex justify-between text-slate-600" v-if="Number(selectedInvoice.discount_amount) > 0">
                    <span>Diskon:</span>
                    <span>-{{ formatRupiah(selectedInvoice.discount_amount) }}</span>
                  </div>
                  <div class="flex justify-between text-slate-600" v-if="Number(selectedInvoice.tax_amount) > 0">
                    <span>PPN:</span>
                    <span>{{ formatRupiah(selectedInvoice.tax_amount) }}</span>
                  </div>
                  <div class="h-px bg-slate-300 my-1"></div>
                  <div class="flex justify-between text-slate-900 font-extrabold text-sm">
                    <span class="font-sans">Total Tagihan:</span>
                    <span>{{ formatRupiah(selectedInvoice.total_amount) }}</span>
                  </div>
                  <div class="flex justify-between text-emerald-700 font-bold">
                    <span>Sudah Dibayar:</span>
                    <span>{{ formatRupiah(selectedInvoice.paid_amount) }}</span>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <div class="px-6 py-3.5 bg-slate-50 border-t border-slate-200 flex items-center justify-between shrink-0">
            <button
              v-if="selectedInvoice && selectedInvoice.payment_status !== 'PAID'"
              type="button"
              @click="handlePayInvoice(selectedInvoice)"
              class="px-4 py-2 rounded-lg bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-xs flex items-center gap-2 shadow-xs transition active:scale-95 cursor-pointer"
            >
              <Check class="w-4 h-4" />
              <span>Tandai Lunas Sekarang</span>
            </button>
            <div v-else></div>

            <button
              type="button"
              @click="showDetailModal = false"
              class="px-4 py-2 rounded-lg border border-slate-300 bg-white hover:bg-slate-100 text-slate-700 font-bold text-xs transition cursor-pointer"
            >
              Tutup
            </button>
          </div>
        </div>
      </div>

      <!-- 6. Modal Form Supplier (Tambah / Edit) -->
      <div
        v-if="showSupplierModal"
        class="fixed inset-0 z-50 bg-slate-900/60 backdrop-blur-xs flex items-center justify-center p-4"
        @click.self="showSupplierModal = false"
      >
        <div class="bg-white rounded-2xl shadow-2xl border border-slate-300 w-full max-w-md overflow-hidden animate-in fade-in zoom-in-95 duration-150">
          <div class="p-6">
            <div class="flex items-center gap-3.5 mb-4">
              <div class="w-11 h-11 rounded-xl bg-emerald-50 border border-emerald-300 flex items-center justify-center shrink-0">
                <Building2 class="w-6 h-6 text-emerald-600" />
              </div>
              <div>
                <h3 class="text-base font-black text-slate-900">
                  {{ isEditingSupplier ? 'Edit Data Pemasok' : 'Tambah Pemasok Baru' }}
                </h3>
                <p class="text-xs text-slate-500">Katalog vendor distributor ritel</p>
              </div>
            </div>

            <div class="space-y-3 text-xs">
              <div>
                <label class="block font-bold text-slate-700 mb-1">Kode Pemasok *</label>
                <input
                  v-model="supplierForm.code"
                  type="text"
                  class="w-full px-3 py-2 rounded-lg border border-slate-300 font-mono text-xs focus:ring-2 focus:ring-emerald-500 focus:outline-none"
                  placeholder="SUP-001"
                />
              </div>

              <div>
                <label class="block font-bold text-slate-700 mb-1">Nama Pemasok / Perusahaan *</label>
                <input
                  v-model="supplierForm.name"
                  type="text"
                  class="w-full px-3 py-2 rounded-lg border border-slate-300 text-xs focus:ring-2 focus:ring-emerald-500 focus:outline-none"
                  placeholder="PT Indomarco / Distributor..."
                />
              </div>

              <div class="grid grid-cols-2 gap-2.5">
                <div>
                  <label class="block font-bold text-slate-700 mb-1">Kontak Person</label>
                  <input
                    v-model="supplierForm.contact_person"
                    type="text"
                    class="w-full px-3 py-2 rounded-lg border border-slate-300 text-xs focus:ring-2 focus:ring-emerald-500 focus:outline-none"
                    placeholder="Nama PIC / Sales"
                  />
                </div>
                <div>
                  <label class="block font-bold text-slate-700 mb-1">No. Telepon / WA</label>
                  <input
                    v-model="supplierForm.phone"
                    type="text"
                    class="w-full px-3 py-2 rounded-lg border border-slate-300 font-mono text-xs focus:ring-2 focus:ring-emerald-500 focus:outline-none"
                    placeholder="0812-..."
                  />
                </div>
              </div>

              <div>
                <label class="block font-bold text-slate-700 mb-1">Term of Payment (Hari)</label>
                <input
                  v-model.number="supplierForm.term_of_payment_days"
                  type="number"
                  min="0"
                  class="w-full px-3 py-2 rounded-lg border border-slate-300 font-mono text-xs focus:ring-2 focus:ring-emerald-500 focus:outline-none"
                  placeholder="14"
                />
              </div>

              <div>
                <label class="block font-bold text-slate-700 mb-1">Alamat Kantor / Gudang</label>
                <textarea
                  v-model="supplierForm.address"
                  rows="2"
                  class="w-full px-3 py-2 rounded-lg border border-slate-300 text-xs focus:ring-2 focus:ring-emerald-500 focus:outline-none"
                  placeholder="Alamat lengkap supplier..."
                ></textarea>
              </div>
            </div>
          </div>

          <div class="px-6 py-3.5 bg-slate-50 border-t border-slate-200 flex items-center justify-end gap-2.5">
            <button
              type="button"
              @click="showSupplierModal = false"
              :disabled="isSavingSupplier"
              class="px-4 py-2 rounded-lg border border-slate-300 bg-white hover:bg-slate-100 text-slate-700 font-bold text-xs transition cursor-pointer"
            >
              Batal
            </button>
            <button
              type="button"
              @click="handleSaveSupplier"
              :disabled="isSavingSupplier"
              class="px-4 py-2 rounded-lg bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-xs flex items-center gap-2 shadow-xs transition active:scale-95 cursor-pointer disabled:opacity-50"
            >
              <RefreshCw v-if="isSavingSupplier" class="w-4 h-4 animate-spin" />
              <Check v-else class="w-4 h-4" />
              <span>Simpan Pemasok</span>
            </button>
          </div>
        </div>
      </div>
    </div>
  </AdminLayout>
</template>
