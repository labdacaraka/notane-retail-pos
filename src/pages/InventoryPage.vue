<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import AdminLayout from '@/layouts/AdminLayout.vue'
import PaginationBar from '@/components/common/PaginationBar.vue'
import SearchableSelect from '@/components/common/SearchableSelect.vue'
import { ClipboardCheck, Truck } from 'lucide-vue-next'
import { api } from '@/services/api'

const activeTab = ref<'opname' | 'grn' | 'ledger'>('opname')
const products = ref<any[]>([])
const suppliers = ref<any[]>([])
const warehouses = ref<any[]>([])
const reasonCodes = ref<any[]>([])

// Opname Form State & Pagination
const opnameWarehouseId = ref<string>('b0000000-0000-0000-0000-000000000002')
const opnameReasonId = ref<string>('f1000000-0000-0000-0000-000000000003')
const opnameItems = ref<any[]>([])
const opnamePage = ref<number>(1)
const opnamePageSize = ref<number>(10)
const paginatedOpnameItems = computed(() => {
  const start = (opnamePage.value - 1) * opnamePageSize.value
  return opnameItems.value.slice(start, start + opnamePageSize.value)
})

// GRN Form State
const grnSupplierId = ref<string>('')
const grnInvoiceNumber = ref<string>('')

// Ledger State & Pagination
const selectedProductForLedger = ref<string>('')
const ledgerMutations = ref<any[]>([])
const ledgerPage = ref<number>(1)
const ledgerPageSize = ref<number>(10)
const paginatedLedgerMutations = computed(() => {
  const start = (ledgerPage.value - 1) * ledgerPageSize.value
  return ledgerMutations.value.slice(start, start + ledgerPageSize.value)
})

async function loadInitialData() {
  try {
    const [pRes, sRes, wRes, rRes] = await Promise.all([
      api.get('/products'),
      api.get('/suppliers'),
      api.get('/warehouses'),
      api.get('/reason-codes'),
    ])

    if (pRes.data.success) {
      products.value = pRes.data.data
      opnameItems.value = pRes.data.data.map((p: any) => ({
        product_id: p.id,
        name: p.name,
        sku: p.sku,
        unit_code: p.unit_code,
        system_qty: p.stock_qty || 0,
        actual_qty: p.stock_qty || 0,
      }))
      if (pRes.data.data.length > 0) {
        selectedProductForLedger.value = pRes.data.data[0].id
        loadLedger()
      }
    }
    if (sRes.data.success) suppliers.value = sRes.data.data
    if (wRes.data.success) warehouses.value = wRes.data.data
    if (rRes.data.success) reasonCodes.value = rRes.data.data
  } catch (err) {
    console.error(err)
  }
}

async function handleSaveOpname() {
  try {
    const payload = {
      warehouse_id: opnameWarehouseId.value,
      reason_id: opnameReasonId.value,
      notes: 'Stock Opname Fisik Bulanan Toko',
      items: opnameItems.value.map((it) => ({
        product_id: it.product_id,
        actual_qty: it.actual_qty,
      })),
    }

    const res = await api.post('/inventory/adjustments', payload)
    alert(res.data.message || 'Opname berhasil disimpan!')
    loadInitialData()
  } catch (err: any) {
    alert(err.response?.data?.message || 'Gagal menyimpan opname')
  }
}

async function loadLedger() {
  if (!selectedProductForLedger.value) return
  try {
    const res = await api.get(`/inventory/stock-ledger/${selectedProductForLedger.value}`)
    if (res.data.success) {
      ledgerMutations.value = res.data.data
    }
  } catch (err) {
    console.error(err)
  }
}

onMounted(() => {
  loadInitialData()
})
</script>

<template>
  <AdminLayout>
    <div class="p-8 space-y-6">
      <!-- Header -->
      <div class="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h2 class="text-xl font-bold text-slate-900 flex items-center gap-2.5">
            <Icon icon="streamline-ultimate-color:warehouse-storage-2" class="w-6 h-6 drop-shadow-xs" />
            <span>Inventori, Pengadaan & Kartu Stok</span>
          </h2>
          <p class="text-xs text-slate-600">Pengendalian stok fisik ketat, penerimaan faktur supplier (GRN), dan jejak mutasi kartu stok.</p>
        </div>

        <!-- Navigation Tabs -->
        <div class="flex items-center gap-1.5 bg-slate-200/80 border border-slate-300 p-1 rounded-lg">
          <button
            @click="activeTab = 'opname'"
            class="px-3.5 py-1.5 rounded-md text-xs font-bold transition"
            :class="activeTab === 'opname' ? 'bg-white text-emerald-800 shadow-xs border border-slate-300/80' : 'text-slate-600 hover:text-slate-900'"
          >
            Stock Opname Fisik
          </button>
          <button
            @click="activeTab = 'grn'"
            class="px-3.5 py-1.5 rounded-md text-xs font-bold transition"
            :class="activeTab === 'grn' ? 'bg-white text-emerald-800 shadow-xs border border-slate-300/80' : 'text-slate-600 hover:text-slate-900'"
          >
            Terima Barang (GRN)
          </button>
          <button
            @click="activeTab = 'ledger'"
            class="px-3.5 py-1.5 rounded-md text-xs font-bold transition"
            :class="activeTab === 'ledger' ? 'bg-white text-emerald-800 shadow-xs border border-slate-300/80' : 'text-slate-600 hover:text-slate-900'"
          >
            Kartu Stok (Ledger)
          </button>
        </div>
      </div>

      <!-- Tab: Stock Opname -->
      <div v-if="activeTab === 'opname'" class="space-y-4">
        <div class="bg-white border border-slate-300 rounded-xl p-5 flex items-center justify-between shadow-sm">
          <div>
            <h3 class="font-bold text-sm text-slate-900 flex items-center gap-2">
              <ClipboardCheck class="w-4 h-4 text-emerald-600" />
              Penyesuaian Fisik Toko (Stock Opname)
            </h3>
            <p class="text-xs text-slate-500 mt-0.5">Input jumlah fisik aktual yang dihitung di rak toko.</p>
          </div>
          <button
            @click="handleSaveOpname"
            class="bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-xs px-5 py-2.5 rounded-lg shadow-sm border border-emerald-700 transition active:scale-95"
          >
            Simpan & Bukukan Jurnal Opname
          </button>
        </div>

        <div class="bg-white border border-slate-300 rounded-xl overflow-hidden shadow-sm">
          <table class="w-full text-left text-xs border-collapse">
            <thead class="bg-slate-100 border-b border-slate-300 text-slate-700 font-mono text-[11px] uppercase tracking-wider">
              <tr>
                <th class="py-3.5 px-4">Produk</th>
                <th class="py-3.5 px-4">SKU</th>
                <th class="py-3.5 px-4 text-center">Stok Sistem</th>
                <th class="py-3.5 px-4 text-center w-40">Stok Fisik Aktual</th>
                <th class="py-3.5 px-4 text-center">Selisih (Discrepancy)</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-200 font-mono">
              <tr v-for="item in paginatedOpnameItems" :key="item.product_id" class="hover:bg-emerald-50/40 transition">
                <td class="py-3.5 px-4 font-sans font-bold text-slate-900">{{ item.name }}</td>
                <td class="py-3.5 px-4 text-slate-500">{{ item.sku }}</td>
                <td class="py-3.5 px-4 text-center text-slate-700 font-bold">{{ item.system_qty }} {{ item.unit_code }}</td>
                <td class="py-3.5 px-4 text-center">
                  <input
                    v-model.number="item.actual_qty"
                    type="number"
                    class="w-24 bg-slate-50 border border-slate-300 rounded-md px-2 py-1 text-center text-slate-900 font-bold focus:bg-white focus:border-emerald-600 focus:outline-none"
                  />
                </td>
                <td class="py-3.5 px-4 text-center font-bold" :class="item.actual_qty - item.system_qty < 0 ? 'text-rose-600' : item.actual_qty - item.system_qty > 0 ? 'text-emerald-700' : 'text-slate-400'">
                  {{ item.actual_qty - item.system_qty > 0 ? `+${item.actual_qty - item.system_qty}` : item.actual_qty - item.system_qty }} {{ item.unit_code }}
                </td>
              </tr>
            </tbody>
          </table>
          <PaginationBar
            v-model:currentPage="opnamePage"
            v-model:pageSize="opnamePageSize"
            :totalItems="opnameItems.length"
          />
        </div>
      </div>

      <!-- Tab: Goods Receipt Note (GRN) -->
      <div v-else-if="activeTab === 'grn'" class="space-y-4">
        <div class="bg-white border border-slate-300 rounded-xl p-6 space-y-4 shadow-sm">
          <h3 class="font-bold text-sm text-slate-900 flex items-center gap-2">
            <Truck class="w-4 h-4 text-sky-600" />
            Penerimaan Faktur Barang Masuk dari Supplier (GRN)
          </h3>
          <p class="text-xs text-slate-500">Penerimaan barang fisik otomatis menambah saldo stok dan mencatat jurnal Hutang Usaha.</p>

          <div class="grid grid-cols-1 md:grid-cols-3 gap-4 text-xs pt-2">
            <div>
              <label class="block text-slate-700 font-bold mb-1">Supplier Distributor</label>
              <SearchableSelect
                v-model="grnSupplierId"
                api-endpoint="/suppliers"
                placeholder="-- Pilih Supplier --"
                search-placeholder="Cari nama supplier..."
              />
            </div>
            <div>
              <label class="block text-slate-700 font-bold mb-1">No. Faktur Supplier</label>
              <input v-model="grnInvoiceNumber" placeholder="INV-SUPP-12345" class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono focus:bg-white focus:border-emerald-600 focus:outline-none" />
            </div>
          </div>
        </div>
      </div>

      <!-- Tab: Kartu Stok (Ledger) -->
      <div v-else-if="activeTab === 'ledger'" class="space-y-4">
        <div class="bg-white border border-slate-300 rounded-xl p-5 flex flex-wrap items-center gap-4 shadow-sm">
          <label class="text-xs font-bold text-slate-700">Pilih Produk:</label>
          <div class="w-80">
            <SearchableSelect
              v-model="selectedProductForLedger"
              api-endpoint="/products"
              secondary-key="sku"
              placeholder="-- Cari Produk --"
              search-placeholder="Cari berdasarkan nama atau SKU..."
              @change="loadLedger"
            />
          </div>
        </div>

        <div class="bg-white border border-slate-300 rounded-xl overflow-hidden shadow-sm">
          <table class="w-full text-left text-xs border-collapse">
            <thead class="bg-slate-100 border-b border-slate-300 text-slate-700 font-mono text-[11px] uppercase tracking-wider">
              <tr>
                <th class="py-3.5 px-4">Waktu</th>
                <th class="py-3.5 px-4">Jenis Referensi</th>
                <th class="py-3.5 px-4 text-center text-emerald-700 font-bold">Masuk (In)</th>
                <th class="py-3.5 px-4 text-center text-rose-700 font-bold">Keluar (Out)</th>
                <th class="py-3.5 px-4 text-center text-slate-900 font-bold">Saldo Akhir</th>
                <th class="py-3.5 px-4 text-right">HPP Mutasi</th>
                <th class="py-3.5 px-4">Keterangan</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-200 font-mono">
              <tr v-for="m in paginatedLedgerMutations" :key="m.id" class="hover:bg-emerald-50/40 transition">
                <td class="py-3.5 px-4 text-slate-500">{{ new Date(m.created_at).toLocaleString('id-ID') }}</td>
                <td class="py-3.5 px-4 font-bold text-sky-700">{{ m.reference_type }}</td>
                <td class="py-3.5 px-4 text-center text-emerald-700 font-bold">{{ m.qty_in > 0 ? `+${m.qty_in}` : '-' }}</td>
                <td class="py-3.5 px-4 text-center text-rose-700 font-bold">{{ m.qty_out > 0 ? `-${m.qty_out}` : '-' }}</td>
                <td class="py-3.5 px-4 text-center text-slate-900 font-bold">{{ m.balance }}</td>
                <td class="py-3.5 px-4 text-right text-slate-700 font-bold">Rp {{ m.cost_price?.toLocaleString() }}</td>
                <td class="py-3.5 px-4 font-sans text-slate-600">{{ m.notes || '-' }}</td>
              </tr>
              <tr v-if="ledgerMutations.length === 0">
                <td colspan="7" class="py-12 text-center text-slate-400 font-sans">Belum ada riwayat mutasi kartu stok untuk produk ini.</td>
              </tr>
            </tbody>
          </table>
          <PaginationBar
            v-model:currentPage="ledgerPage"
            v-model:pageSize="ledgerPageSize"
            :totalItems="ledgerMutations.length"
          />
        </div>
      </div>
    </div>
  </AdminLayout>
</template>
