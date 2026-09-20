<script setup lang="ts">
import { ref, onMounted, computed } from 'vue'
import AdminLayout from '@/layouts/AdminLayout.vue'
import PaginationBar from '@/components/common/PaginationBar.vue'
import {
  Ticket,
  Plus,
  Search,
  Percent,
  Edit2,
  Trash2,
  CheckCircle2,
} from 'lucide-vue-next'
import { api } from '@/services/api'

const vouchers = ref<any[]>([])
const isLoading = ref<boolean>(false)
const searchQuery = ref<string>('')
const statusFilter = ref<string>('')

// Pagination state
const currentPage = ref<number>(1)
const pageSize = ref<number>(10)

const showVoucherModal = ref<boolean>(false)
const isEditing = ref<boolean>(false)
const currentVoucherId = ref<string | null>(null)

const voucherForm = ref({
  code: '',
  description: '',
  discount_type: 'PERCENTAGE', // PERCENTAGE, FIXED_AMOUNT
  discount_value: 10,
  min_purchase_amount: 0,
  max_discount_amount: null as number | null,
  valid_from: '',
  valid_until: '',
  status: 'ACTIVE',
})

async function fetchVouchers() {
  isLoading.value = true
  try {
    const res = await api.get('/vouchers')
    if (res.data.success) {
      vouchers.value = res.data.data
    }
  } catch (err) {
    console.error('Error fetching vouchers:', err)
  } finally {
    isLoading.value = false
  }
}

const filteredVouchers = computed(() => {
  return vouchers.value.filter((v) => {
    const matchSearch =
      !searchQuery.value ||
      v.code.toLowerCase().includes(searchQuery.value.toLowerCase()) ||
      (v.description && v.description.toLowerCase().includes(searchQuery.value.toLowerCase()))

    const matchStatus = !statusFilter.value || v.status === statusFilter.value
    return matchSearch && matchStatus
  })
})

const paginatedVouchers = computed(() => {
  const start = (currentPage.value - 1) * pageSize.value
  return filteredVouchers.value.slice(start, start + pageSize.value)
})

function openCreateModal() {
  isEditing.value = false
  currentVoucherId.value = null

  const now = new Date()
  const nextMonth = new Date()
  nextMonth.setDate(now.getDate() + 30)

  voucherForm.value = {
    code: `PROMO-${Date.now().toString().slice(-4)}`,
    description: '',
    discount_type: 'PERCENTAGE',
    discount_value: 10,
    min_purchase_amount: 50000,
    max_discount_amount: 20000,
    valid_from: now.toISOString().slice(0, 16),
    valid_until: nextMonth.toISOString().slice(0, 16),
    status: 'ACTIVE',
  }
  showVoucherModal.value = true
}

function openEditModal(v: any) {
  isEditing.value = true
  currentVoucherId.value = v.id
  voucherForm.value = {
    code: v.code,
    description: v.description || '',
    discount_type: v.discount_type,
    discount_value: Number(v.discount_value),
    min_purchase_amount: Number(v.min_purchase_amount || 0),
    max_discount_amount: v.max_discount_amount ? Number(v.max_discount_amount) : null,
    valid_from: v.valid_from ? new Date(v.valid_from).toISOString().slice(0, 16) : '',
    valid_until: v.valid_until ? new Date(v.valid_until).toISOString().slice(0, 16) : '',
    status: v.status || 'ACTIVE',
  }
  showVoucherModal.value = true
}

async function handleSaveVoucher() {
  if (!voucherForm.value.code) {
    alert('Kode voucher wajib diisi')
    return
  }

  const payload = {
    ...voucherForm.value,
    valid_from: new Date(voucherForm.value.valid_from).toISOString(),
    valid_until: new Date(voucherForm.value.valid_until).toISOString(),
  }

  try {
    if (isEditing.value && currentVoucherId.value) {
      await api.put(`/vouchers/${currentVoucherId.value}`, payload)
    } else {
      await api.post('/vouchers', payload)
    }
    showVoucherModal.value = false
    fetchVouchers()
  } catch (err: any) {
    alert(err.response?.data?.message || 'Gagal menyimpan voucher')
  }
}

async function handleDeleteVoucher(id: string, code: string) {
  if (!confirm(`Nonaktifkan voucher "${code}"?`)) return
  try {
    await api.delete(`/vouchers/${id}`)
    fetchVouchers()
  } catch (err: any) {
    alert(err.response?.data?.message || 'Gagal menonaktifkan voucher')
  }
}

function formatCurrency(val: number | string) {
  return new Intl.NumberFormat('id-ID', {
    style: 'currency',
    currency: 'IDR',
    maximumFractionDigits: 0,
  }).format(Number(val) || 0)
}

function formatDate(isoStr: string) {
  if (!isoStr) return '-'
  const d = new Date(isoStr)
  return d.toLocaleDateString('id-ID', {
    day: '2-digit',
    month: 'short',
    year: 'numeric',
  })
}

onMounted(() => {
  fetchVouchers()
})
</script>

<template>
  <AdminLayout>
    <div class="p-6 space-y-4">
      <!-- Page Header -->
      <div class="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h2 class="text-xl font-bold text-slate-900 flex items-center gap-2.5">
            <Icon icon="streamline-ultimate-color:ticket-1" class="w-6 h-6 drop-shadow-xs" />
            <span>Master Voucher & Promosi Diskon</span>
          </h2>
          <p class="text-xs text-slate-500">
            Kelola kode kupon belanja, potongan persentase, diskon nominal tetap, kuota, dan masa berlaku.
          </p>
        </div>

        <button
          @click="openCreateModal"
          class="bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-xs px-4 py-2 rounded-lg shadow-sm flex items-center gap-2 transition active:scale-95 border border-emerald-700"
        >
          <Plus class="w-4 h-4" />
          <span>Tambah Voucher Baru</span>
        </button>
      </div>

      <!-- KPI Summary Cards -->
      <div class="grid grid-cols-1 sm:grid-cols-3 gap-3">
        <div class="bg-white border border-slate-300 p-3.5 rounded-xl flex items-center gap-3 shadow-sm">
          <div class="w-10 h-10 rounded-lg bg-amber-100 text-amber-800 flex items-center justify-center font-bold">
            <Ticket class="w-5 h-5" />
          </div>
          <div>
            <div class="text-[11px] text-slate-500 uppercase tracking-wider font-bold">Total Voucher</div>
            <div class="text-lg font-black text-slate-900">{{ vouchers.length }} Promo</div>
          </div>
        </div>

        <div class="bg-white border border-slate-300 p-3.5 rounded-xl flex items-center gap-3 shadow-sm">
          <div class="w-10 h-10 rounded-lg bg-emerald-100 text-emerald-800 flex items-center justify-center font-bold">
            <CheckCircle2 class="w-5 h-5" />
          </div>
          <div>
            <div class="text-[11px] text-slate-500 uppercase tracking-wider font-bold">Voucher Aktif</div>
            <div class="text-lg font-black text-emerald-700">
              {{ vouchers.filter(v => v.status === 'ACTIVE').length }} Aktif
            </div>
          </div>
        </div>

        <div class="bg-white border border-slate-300 p-3.5 rounded-xl flex items-center gap-3 shadow-sm">
          <div class="w-10 h-10 rounded-lg bg-purple-100 text-purple-800 flex items-center justify-center font-bold">
            <Percent class="w-5 h-5" />
          </div>
          <div>
            <div class="text-[11px] text-slate-500 uppercase tracking-wider font-bold">Voucher Persentase</div>
            <div class="text-lg font-black text-purple-800">
              {{ vouchers.filter(v => v.discount_type === 'PERCENTAGE').length }} Tipe %
            </div>
          </div>
        </div>
      </div>

      <!-- Filter / Search Bar -->
      <div class="flex flex-wrap items-center gap-3 bg-white border border-slate-300 p-2.5 rounded-xl shadow-sm">
        <div class="relative flex-1 min-w-[240px]">
          <Search class="w-4 h-4 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2" />
          <input
            v-model="searchQuery"
            type="text"
            placeholder="Cari voucher berdasarkan kode promo atau deskripsi..."
            class="w-full bg-slate-50 border border-slate-300 rounded-lg pl-9 pr-4 py-1.5 text-xs text-slate-900 placeholder-slate-400 focus:outline-none focus:bg-white focus:border-emerald-600 font-medium"
          />
        </div>

        <div class="flex items-center gap-2">
          <span class="text-xs text-slate-600 font-semibold">Status:</span>
          <select
            v-model="statusFilter"
            class="bg-slate-50 border border-slate-300 rounded-lg px-3 py-1.5 text-xs text-slate-900 focus:outline-none focus:bg-white focus:border-emerald-600 font-medium"
          >
            <option value="">Semua Status</option>
            <option value="ACTIVE">Aktif (ACTIVE)</option>
            <option value="EXPIRED">Kedaluwarsa (EXPIRED)</option>
          </select>
        </div>
      </div>

      <!-- Table of Vouchers -->
      <div class="bg-white border border-slate-300 rounded-xl overflow-hidden shadow-sm">
        <table class="w-full text-left text-xs border-collapse">
          <thead class="bg-slate-100 border-b border-slate-300 text-slate-700 font-mono uppercase tracking-wider font-bold text-[11px]">
            <tr>
              <th class="p-3.5">Kode Voucher</th>
              <th class="p-3.5">Deskripsi</th>
              <th class="p-3.5">Tipe Diskon</th>
              <th class="p-3.5 text-right">Nilai Potongan</th>
              <th class="p-3.5 text-right">Min Belanja</th>
              <th class="p-3.5">Masa Berlaku</th>
              <th class="p-3.5 text-center">Status</th>
              <th class="p-3.5 text-center">Aksi</th>
            </tr>
          </thead>
          <tbody class="divide-y divide-slate-200 font-sans">
            <tr v-if="isLoading">
              <td colspan="8" class="p-8 text-center text-slate-500">Memuat data voucher...</td>
            </tr>
            <tr v-else-if="filteredVouchers.length === 0">
              <td colspan="8" class="p-8 text-center text-slate-500">Tidak ada voucher ditemukan</td>
            </tr>
            <tr v-for="v in paginatedVouchers" :key="v.id" class="hover:bg-emerald-50/40 transition">
              <td class="p-3.5 font-mono font-bold text-amber-800 flex items-center gap-1.5">
                <Ticket class="w-3.5 h-3.5 text-amber-600" />
                <span>{{ v.code }}</span>
              </td>
              <td class="p-3.5 font-medium text-slate-900 max-w-xs truncate">{{ v.description || '-' }}</td>
              <td class="p-3.5">
                <span
                  class="px-2.5 py-1 rounded-full text-[10px] font-bold uppercase border"
                  :class="v.discount_type === 'PERCENTAGE' ? 'bg-purple-100 text-purple-800 border-purple-300' : 'bg-emerald-100 text-emerald-800 border-emerald-300'"
                >
                  {{ v.discount_type === 'PERCENTAGE' ? 'Persen (%)' : 'Nominal Tetap' }}
                </span>
              </td>
              <td class="p-3.5 text-right font-mono font-bold text-emerald-700">
                {{ v.discount_type === 'PERCENTAGE' ? `${Number(v.discount_value)}%` : formatCurrency(v.discount_value) }}
                <span v-if="v.discount_type === 'PERCENTAGE' && v.max_discount_amount" class="block text-[10px] text-slate-500 font-normal">
                  Maks {{ formatCurrency(v.max_discount_amount) }}
                </span>
              </td>
              <td class="p-3.5 text-right font-mono text-slate-800 font-medium">
                {{ formatCurrency(v.min_purchase_amount) }}
              </td>
              <td class="p-3.5 text-slate-600 font-mono text-[11px]">
                <div>Mulai: {{ formatDate(v.valid_from) }}</div>
                <div>S/D: {{ formatDate(v.valid_until) }}</div>
              </td>
              <td class="p-3.5 text-center">
                <span
                  class="px-2.5 py-0.5 rounded-full text-[10px] font-bold border"
                  :class="v.status === 'ACTIVE' ? 'bg-emerald-100 text-emerald-800 border-emerald-300' : 'bg-rose-100 text-rose-800 border-rose-300'"
                >
                  {{ v.status }}
                </span>
              </td>
              <td class="p-3.5 text-center">
                <div class="flex items-center justify-center gap-1.5">
                  <button
                    @click="openEditModal(v)"
                    title="Edit Voucher"
                    class="p-1.5 rounded-md bg-white hover:bg-slate-100 border border-slate-300 text-blue-600 transition shadow-xs"
                  >
                    <Edit2 class="w-3.5 h-3.5" />
                  </button>
                  <button
                    @click="handleDeleteVoucher(v.id, v.code)"
                    title="Nonaktifkan Voucher"
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
          :totalItems="filteredVouchers.length"
        />
      </div>
    </div>

    <!-- Modal Form: Tambah / Edit Voucher -->
    <div
      v-if="showVoucherModal"
      class="fixed inset-0 z-50 bg-slate-900/50 backdrop-blur-xs flex items-center justify-center p-4 select-none"
    >
      <div class="bg-white border border-slate-300 w-full max-w-lg rounded-xl p-6 space-y-4 shadow-2xl">
        <h3 class="font-bold text-base text-slate-900">
          {{ isEditing ? 'Edit Master Voucher Promo' : 'Buat Voucher Promo Baru' }}
        </h3>

        <div class="space-y-3 text-xs">
          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="block text-slate-700 font-medium mb-1">Kode Voucher / Promo *</label>
              <input
                v-model="voucherForm.code"
                class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono uppercase font-bold tracking-wider focus:bg-white focus:border-emerald-600"
                placeholder="e.g. HEMAT20K"
              />
            </div>

            <div>
              <label class="block text-slate-700 font-medium mb-1">Tipe Diskon</label>
              <select
                v-model="voucherForm.discount_type"
                class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 focus:bg-white focus:border-emerald-600"
              >
                <option value="PERCENTAGE">Persentase (%)</option>
                <option value="FIXED_AMOUNT">Potongan Nominal Tetap (Rp)</option>
              </select>
            </div>
          </div>

          <div>
            <label class="block text-slate-700 font-medium mb-1">Deskripsi Promosi</label>
            <input
              v-model="voucherForm.description"
              class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 focus:bg-white focus:border-emerald-600"
              placeholder="e.g. Diskon 20% Minimal Belanja Rp 100.000"
            />
          </div>

          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="block text-slate-700 font-medium mb-1">
                {{ voucherForm.discount_type === 'PERCENTAGE' ? 'Persen Diskon (%)' : 'Nominal Diskon (Rp)' }} *
              </label>
              <input
                v-model.number="voucherForm.discount_value"
                type="number"
                class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono focus:bg-white focus:border-emerald-600"
                placeholder="0"
              />
            </div>

            <div>
              <label class="block text-slate-700 font-medium mb-1">Maks. Potongan Diskon (Rp)</label>
              <input
                v-model.number="voucherForm.max_discount_amount"
                type="number"
                class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono focus:bg-white focus:border-emerald-600"
                placeholder="Kosongkan bila tidak dibatasi"
              />
            </div>
          </div>

          <div>
            <label class="block text-slate-700 font-medium mb-1">Min. Pembelian Belanja (Rp)</label>
            <input
              v-model.number="voucherForm.min_purchase_amount"
              type="number"
              class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono focus:bg-white focus:border-emerald-600"
              placeholder="0"
            />
          </div>

          <div class="grid grid-cols-2 gap-3">
            <div>
              <label class="block text-slate-700 font-medium mb-1">Mulai Berlaku</label>
              <input
                v-model="voucherForm.valid_from"
                type="datetime-local"
                class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono text-[11px] focus:bg-white focus:border-emerald-600"
              />
            </div>

            <div>
              <label class="block text-slate-700 font-medium mb-1">Berlaku Sampai</label>
              <input
                v-model="voucherForm.valid_until"
                type="datetime-local"
                class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 font-mono text-[11px] focus:bg-white focus:border-emerald-600"
              />
            </div>
          </div>

          <div>
            <label class="block text-slate-700 font-medium mb-1">Status Voucher</label>
            <select
              v-model="voucherForm.status"
              class="w-full bg-slate-50 border border-slate-300 rounded-lg px-3 py-2 text-slate-900 focus:bg-white focus:border-emerald-600"
            >
              <option value="ACTIVE">ACTIVE (Aktif & Bisa Digunakan)</option>
              <option value="EXPIRED">EXPIRED (Nonaktif / Kedaluwarsa)</option>
            </select>
          </div>
        </div>

        <div class="flex justify-end gap-2 pt-4 border-t border-slate-200">
          <button @click="showVoucherModal = false" class="bg-slate-100 hover:bg-slate-200 border border-slate-300 text-slate-700 px-4 py-2 rounded-lg text-xs font-bold">
            Batal
          </button>
          <button @click="handleSaveVoucher" class="bg-emerald-600 hover:bg-emerald-700 border border-emerald-700 text-white px-5 py-2 rounded-lg text-xs font-bold shadow-sm">
            Simpan Voucher
          </button>
        </div>
      </div>
    </div>
  </AdminLayout>
</template>
