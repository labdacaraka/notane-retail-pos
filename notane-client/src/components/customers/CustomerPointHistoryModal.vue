<script setup lang="ts">
import { ref, watch, computed } from 'vue'
import {
  X,
  Coins,
  Award,
  ArrowUpRight,
  ArrowDownLeft,
  Sliders,
  RefreshCw,
  PlusCircle,
  MinusCircle,
  User,
  Phone,
  AlertCircle,
  CheckCircle2,
  Sparkles,
} from 'lucide-vue-next'
import { api } from '@/services/api'
import { useAuthStore } from '@/stores/authStore'
import { useDialog } from '@/composables/useDialog'
import { formatQty } from '@/utils/formatters'
import PaginationBar from '@/components/common/PaginationBar.vue'
import type { Customer, CustomerPointHistoryResponse } from '@/types'

const props = defineProps<{
  show: boolean
  customer: Customer | null
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'updated'): void
}>()

const authStore = useAuthStore()
const dialog = useDialog()

const isLoading = ref<boolean>(false)
const historyData = ref<CustomerPointHistoryResponse | null>(null)
const filterType = ref<string>('ALL')
const currentPage = ref<number>(1)
const pageSize = ref<number>(5)
const errorMessage = ref<string>('')
const successMessage = ref<string>('')

// Penyesuaian poin modal state
const showAdjustModal = ref<boolean>(false)
const isSubmittingAdjust = ref<boolean>(false)
const adjustForm = ref<{
  adjustment_type: 'ADD' | 'DEDUCT'
  points: number
  notes: string
}>({
  adjustment_type: 'ADD',
  points: 10,
  notes: '',
})

const canManagePoints = computed(() => {
  return authStore.hasPermission(['customers:manage', 'loyalty:manage', 'master:write'])
})

async function fetchPointHistory() {
  if (!props.customer?.id) return
  isLoading.value = true
  errorMessage.value = ''
  try {
    const res = await api.get(`/customers/${props.customer.id}/point-ledgers`, {
      params: {
        page: currentPage.value,
        page_size: pageSize.value,
        transaction_type: filterType.value === 'ALL' ? undefined : filterType.value,
      },
    })
    if (res.data.success) {
      historyData.value = res.data.data
    } else {
      errorMessage.value = res.data.message || 'Gagal memuat riwayat poin'
    }
  } catch (err: any) {
    console.error('Error fetching point history:', err)
    errorMessage.value = err.response?.data?.message || 'Gagal memuat riwayat mutasi poin'
  } finally {
    isLoading.value = false
  }
}

watch(
  () => props.show,
  (val) => {
    if (val && props.customer?.id) {
      currentPage.value = 1
      filterType.value = 'ALL'
      errorMessage.value = ''
      successMessage.value = ''
      showAdjustModal.value = false
      fetchPointHistory()
    }
  }
)

function handleFilterChange(type: string) {
  filterType.value = type
  currentPage.value = 1
  fetchPointHistory()
}

function openAdjustModal() {
  adjustForm.value = {
    adjustment_type: 'ADD',
    points: 10,
    notes: '',
  }
  showAdjustModal.value = true
}

async function handleSaveAdjust() {
  if (!props.customer?.id) return
  if (!canManagePoints.value) {
    dialog.alert('Akses ditolak: Anda tidak memiliki wewenang menyesuaikan poin (customers:manage)', 'warning')
    return
  }

  if (!adjustForm.value.points || adjustForm.value.points <= 0) {
    dialog.alert('Jumlah poin harus lebih besar dari 0', 'warning')
    return
  }

  if (!adjustForm.value.notes.trim()) {
    dialog.alert('Keterangan / alasan penyesuaian poin wajib diisi', 'warning')
    return
  }

  const currentPts = historyData.value?.summary?.current_balance ?? props.customer.reward_points ?? 0
  if (adjustForm.value.adjustment_type === 'DEDUCT' && adjustForm.value.points > currentPts) {
    dialog.alert(`Saldo poin pelanggan (${currentPts}) tidak mencukupi untuk pemotongan ${adjustForm.value.points} poin`, 'danger')
    return
  }

  isSubmittingAdjust.value = true
  errorMessage.value = ''
  successMessage.value = ''

  try {
    const res = await api.post(`/customers/${props.customer.id}/adjust-points`, {
      adjustment_type: adjustForm.value.adjustment_type,
      points: Number(adjustForm.value.points),
      notes: adjustForm.value.notes.trim(),
    })

    if (res.data.success) {
      successMessage.value = res.data.message || 'Poin berhasil disesuaikan'
      showAdjustModal.value = false
      emit('updated')
      await fetchPointHistory()
    } else {
      dialog.alert(res.data.message || 'Gagal menyesuaikan poin', 'danger')
    }
  } catch (err: any) {
    dialog.alert(err.response?.data?.message || 'Gagal menyimpan penyesuaian poin', 'danger')
  } finally {
    isSubmittingAdjust.value = false
  }
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
</script>

<template>
  <div
    v-if="show"
    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/60 backdrop-blur-xs transition-opacity duration-200"
  >
    <div
      class="bg-white rounded-2xl shadow-2xl border border-slate-200 w-full max-w-4xl max-h-[92vh] flex flex-col overflow-hidden animate-in fade-in zoom-in-95 duration-150"
    >
      <!-- HEADER -->
      <div class="px-6 py-4 border-b border-slate-200 bg-linear-to-r from-amber-500/10 via-amber-500/5 to-transparent flex items-center justify-between">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-xl bg-amber-500 text-white flex items-center justify-center shadow-md shadow-amber-500/20">
            <Coins class="w-5 h-5" />
          </div>
          <div>
            <div class="flex items-center gap-2">
              <h2 class="text-base font-bold text-slate-800">Riwayat Mutasi Poin Pelanggan</h2>
              <span
                v-if="customer?.group_name"
                class="px-2 py-0.5 rounded-full text-[10px] font-extrabold uppercase tracking-wide border bg-amber-100 text-amber-800 border-amber-300"
              >
                {{ customer.group_name }}
              </span>
            </div>
            <div class="flex items-center gap-3 text-xs text-slate-500 mt-0.5">
              <span class="font-mono font-bold text-slate-700">{{ customer?.code }}</span>
              <span>•</span>
              <span class="font-semibold text-slate-900">{{ customer?.name }}</span>
              <span v-if="customer?.phone" class="flex items-center gap-1 text-slate-500">
                <span>•</span>
                <Phone class="w-3 h-3" />
                <span>{{ customer.phone }}</span>
              </span>
            </div>
          </div>
        </div>

        <div class="flex items-center gap-2">
          <button
            @click="fetchPointHistory"
            :disabled="isLoading"
            title="Muat Ulang"
            class="p-2 rounded-lg text-slate-500 hover:text-slate-800 hover:bg-slate-100 transition"
          >
            <RefreshCw class="w-4 h-4" :class="{ 'animate-spin': isLoading }" />
          </button>
          <button
            @click="emit('close')"
            class="p-2 rounded-lg text-slate-400 hover:text-slate-600 hover:bg-slate-100 transition"
          >
            <X class="w-5 h-5" />
          </button>
        </div>
      </div>

      <!-- BODY -->
      <div class="flex-1 overflow-y-auto p-6 space-y-6">
        <!-- ALERT MESSAGES -->
        <div v-if="successMessage" class="p-3 bg-emerald-50 border border-emerald-200 text-emerald-800 rounded-xl text-xs flex items-center gap-2">
          <CheckCircle2 class="w-4 h-4 text-emerald-600 shrink-0" />
          <span>{{ successMessage }}</span>
        </div>
        <div v-if="errorMessage" class="p-3 bg-rose-50 border border-rose-200 text-rose-800 rounded-xl text-xs flex items-center gap-2">
          <AlertCircle class="w-4 h-4 text-rose-600 shrink-0" />
          <span>{{ errorMessage }}</span>
        </div>

        <!-- 3 SUMMARY STATS CARDS -->
        <div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
          <!-- Saldo Saat Ini -->
          <div class="bg-linear-to-br from-amber-50 to-amber-100/50 border border-amber-200/80 rounded-xl p-4 shadow-xs">
            <div class="flex items-center justify-between text-amber-700 text-xs font-semibold">
              <span>Saldo Poin Saat Ini</span>
              <Award class="w-4 h-4 text-amber-600" />
            </div>
            <div class="mt-2 flex items-baseline gap-1.5">
              <span class="text-2xl font-black font-mono text-amber-900">
                {{ formatQty(historyData?.summary?.current_balance ?? customer?.reward_points ?? 0) }}
              </span>
              <span class="text-xs font-bold text-amber-700">Poin</span>
            </div>
            <p class="text-[11px] text-amber-600/80 mt-1">Saldo aktif siap digunakan</p>
          </div>

          <!-- Total Diperoleh (Masuk) -->
          <div class="bg-linear-to-br from-emerald-50 to-emerald-100/50 border border-emerald-200/80 rounded-xl p-4 shadow-xs">
            <div class="flex items-center justify-between text-emerald-700 text-xs font-semibold">
              <span>Total Poin Diperoleh</span>
              <ArrowDownLeft class="w-4 h-4 text-emerald-600" />
            </div>
            <div class="mt-2 flex items-baseline gap-1.5">
              <span class="text-2xl font-black font-mono text-emerald-800">
                +{{ formatQty(historyData?.summary?.total_earned ?? 0) }}
              </span>
              <span class="text-xs font-bold text-emerald-600">Poin</span>
            </div>
            <p class="text-[11px] text-emerald-600/80 mt-1">Total reward dari transaksi</p>
          </div>

          <!-- Total Ditukarkan (Keluar) -->
          <div class="bg-linear-to-br from-rose-50 to-rose-100/50 border border-rose-200/80 rounded-xl p-4 shadow-xs">
            <div class="flex items-center justify-between text-rose-700 text-xs font-semibold">
              <span>Total Poin Ditukarkan</span>
              <ArrowUpRight class="w-4 h-4 text-rose-600" />
            </div>
            <div class="mt-2 flex items-baseline gap-1.5">
              <span class="text-2xl font-black font-mono text-rose-800">
                -{{ formatQty(historyData?.summary?.total_redeemed ?? 0) }}
              </span>
              <span class="text-xs font-bold text-rose-600">Poin</span>
            </div>
            <p class="text-[11px] text-rose-600/80 mt-1">Dipakai untuk potongan belanja</p>
          </div>
        </div>

        <!-- FILTER & ACTION TOOLBAR -->
        <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-3 pt-2">
          <!-- Filter Tabs -->
          <div class="flex items-center gap-1.5 bg-slate-100 p-1 rounded-xl text-xs font-semibold overflow-x-auto">
            <button
              @click="handleFilterChange('ALL')"
              class="px-3 py-1.5 rounded-lg transition"
              :class="filterType === 'ALL' ? 'bg-white text-slate-900 shadow-xs' : 'text-slate-600 hover:text-slate-900'"
            >
              Semua
            </button>
            <button
              @click="handleFilterChange('EARNED')"
              class="px-3 py-1.5 rounded-lg transition flex items-center gap-1"
              :class="filterType === 'EARNED' ? 'bg-white text-emerald-700 shadow-xs' : 'text-slate-600 hover:text-slate-900'"
            >
              <span>Poin Masuk</span>
            </button>
            <button
              @click="handleFilterChange('REDEEMED')"
              class="px-3 py-1.5 rounded-lg transition flex items-center gap-1"
              :class="filterType === 'REDEEMED' ? 'bg-white text-rose-700 shadow-xs' : 'text-slate-600 hover:text-slate-900'"
            >
              <span>Poin Keluar</span>
            </button>
            <button
              @click="handleFilterChange('CORRECTION')"
              class="px-3 py-1.5 rounded-lg transition flex items-center gap-1"
              :class="filterType === 'CORRECTION' ? 'bg-white text-blue-700 shadow-xs' : 'text-slate-600 hover:text-slate-900'"
            >
              <span>Koreksi Manual</span>
            </button>
          </div>

          <!-- Adjust Button -->
          <div v-if="canManagePoints" class="flex items-center gap-2">
            <button
              @click="openAdjustModal"
              class="inline-flex items-center gap-1.5 px-3 py-1.5 text-xs font-bold rounded-lg bg-amber-600 hover:bg-amber-700 text-white shadow-xs transition"
            >
              <Sliders class="w-3.5 h-3.5" />
              <span>Sesuaikan Poin Manual</span>
            </button>
          </div>
        </div>

        <!-- TABLE OF POINT LEDGERS -->
        <div class="border border-slate-200 rounded-xl overflow-hidden shadow-xs bg-white">
          <div class="overflow-x-auto">
            <table class="w-full text-left text-xs border-collapse min-w-[700px]">
              <thead class="bg-slate-100 border-b border-slate-200 text-slate-700 font-mono uppercase tracking-wider font-bold text-[10px]">
                <tr>
                  <th class="p-3">Waktu</th>
                  <th class="p-3">Tipe Transaksi</th>
                  <th class="p-3">No. Nota / Referensi</th>
                  <th class="p-3 text-right">Masuk (+)</th>
                  <th class="p-3 text-right">Keluar (-)</th>
                  <th class="p-3 text-right">Saldo Akhir</th>
                  <th class="p-3">Keterangan</th>
                  <th class="p-3">Operator</th>
                </tr>
              </thead>
              <tbody class="divide-y divide-slate-100 font-sans">
                <tr v-if="isLoading">
                  <td colspan="8" class="p-8 text-center text-slate-500">
                    <div class="flex items-center justify-center gap-2">
                      <RefreshCw class="w-4 h-4 animate-spin text-amber-600" />
                      <span>Memuat riwayat transaksi poin...</span>
                    </div>
                  </td>
                </tr>
                <tr v-else-if="!historyData?.ledgers || historyData.ledgers.length === 0">
                  <td colspan="8" class="p-10 text-center text-slate-500">
                    <div class="flex flex-col items-center justify-center gap-2 max-w-sm mx-auto">
                      <div class="w-10 h-10 rounded-full bg-slate-100 flex items-center justify-center text-slate-400">
                        <Coins class="w-5 h-5" />
                      </div>
                      <p class="font-bold text-slate-700 text-sm">Belum Ada Riwayat Poin</p>
                      <p class="text-xs text-slate-400">
                        Belum ada mutasi pendapatan atau penukaran poin yang tercatat untuk member ini.
                      </p>
                    </div>
                  </td>
                </tr>
                <tr
                  v-for="item in historyData?.ledgers"
                  :key="item.id"
                  class="hover:bg-slate-50/80 transition"
                >
                  <!-- Waktu -->
                  <td class="p-3 font-mono text-slate-600 whitespace-nowrap text-[11px]">
                    {{ formatDate(item.created_at) }}
                  </td>

                  <!-- Tipe -->
                  <td class="p-3 whitespace-nowrap">
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

                  <!-- Referensi -->
                  <td class="p-3 font-mono font-bold whitespace-nowrap">
                    <span v-if="item.reference_number" class="text-blue-700 bg-blue-50 px-2 py-0.5 rounded border border-blue-200 text-[11px]">
                      {{ item.reference_number }}
                    </span>
                    <span v-else class="text-slate-400 font-normal">-</span>
                  </td>

                  <!-- Masuk -->
                  <td class="p-3 text-right font-mono font-bold whitespace-nowrap">
                    <span v-if="item.points_in > 0" class="text-emerald-700">
                      +{{ formatQty(item.points_in) }}
                    </span>
                    <span v-else class="text-slate-300 font-normal">-</span>
                  </td>

                  <!-- Keluar -->
                  <td class="p-3 text-right font-mono font-bold whitespace-nowrap">
                    <span v-if="item.points_out > 0" class="text-rose-700">
                      -{{ formatQty(item.points_out) }}
                    </span>
                    <span v-else class="text-slate-300 font-normal">-</span>
                  </td>

                  <!-- Saldo Akhir -->
                  <td class="p-3 text-right font-mono font-black text-slate-800 whitespace-nowrap">
                    {{ formatQty(item.balance) }}
                  </td>

                  <!-- Keterangan -->
                  <td class="p-3 text-slate-600 max-w-[240px] truncate" :title="item.notes || ''">
                    {{ item.notes || '-' }}
                  </td>

                  <!-- Operator -->
                  <td class="p-3 text-slate-500 whitespace-nowrap">
                    <span class="inline-flex items-center gap-1">
                      <User class="w-3 h-3 text-slate-400" />
                      <span>{{ item.created_by_name || item.created_by_username || 'Sistem' }}</span>
                    </span>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <!-- Pagination -->
          <div v-if="historyData?.pagination" class="border-t border-slate-200">
            <PaginationBar
              v-model:currentPage="currentPage"
              v-model:pageSize="pageSize"
              :totalItems="historyData.pagination.total_items"
              :pageSizeOptions="[5, 10, 20, 50]"
              @update:currentPage="fetchPointHistory"
              @update:pageSize="() => { currentPage = 1; fetchPointHistory() }"
            />
          </div>
        </div>
      </div>

      <!-- FOOTER -->
      <div class="px-6 py-3.5 bg-slate-50 border-t border-slate-200 flex items-center justify-between text-xs text-slate-500">
        <div class="flex items-center gap-1.5">
          <Sparkles class="w-4 h-4 text-amber-500" />
          <span>Sistem Poin Terintegrasi POS, Diskon Member & Akuntansi</span>
        </div>
        <button
          @click="emit('close')"
          class="px-4 py-2 bg-white hover:bg-slate-100 text-slate-700 font-bold border border-slate-300 rounded-xl transition shadow-xs"
        >
          Tutup
        </button>
      </div>
    </div>

    <!-- SUB-MODAL PENYESUAIAN POIN MANUAL -->
    <div
      v-if="showAdjustModal"
      class="fixed inset-0 z-60 flex items-center justify-center p-4 bg-slate-900/60 backdrop-blur-xs"
    >
      <div
        class="bg-white rounded-2xl shadow-2xl border border-slate-200 w-full max-w-md overflow-hidden animate-in fade-in zoom-in-95 duration-150"
      >
        <div class="px-5 py-4 border-b border-slate-200 bg-amber-50/50 flex items-center justify-between">
          <div class="flex items-center gap-2 text-amber-900 font-bold text-sm">
            <Sliders class="w-4 h-4 text-amber-600" />
            <span>Koreksi / Penyesuaian Poin Manual</span>
          </div>
          <button
            @click="showAdjustModal = false"
            class="text-slate-400 hover:text-slate-600 p-1 rounded-lg"
          >
            <X class="w-4 h-4" />
          </button>
        </div>

        <form @submit.prevent="handleSaveAdjust" class="p-5 space-y-4">
          <!-- Aksi: Tambah vs Potong -->
          <div>
            <label class="block text-xs font-bold text-slate-700 mb-1.5">Tindakan Penyesuaian</label>
            <div class="grid grid-cols-2 gap-2">
              <button
                type="button"
                @click="adjustForm.adjustment_type = 'ADD'"
                class="flex items-center justify-center gap-2 p-2.5 rounded-xl border text-xs font-bold transition"
                :class="
                  adjustForm.adjustment_type === 'ADD'
                    ? 'bg-emerald-50 border-emerald-400 text-emerald-800 ring-2 ring-emerald-500/20'
                    : 'bg-white border-slate-200 text-slate-600 hover:bg-slate-50'
                "
              >
                <PlusCircle class="w-4 h-4 text-emerald-600" />
                <span>Tambah Poin (+)</span>
              </button>
              <button
                type="button"
                @click="adjustForm.adjustment_type = 'DEDUCT'"
                class="flex items-center justify-center gap-2 p-2.5 rounded-xl border text-xs font-bold transition"
                :class="
                  adjustForm.adjustment_type === 'DEDUCT'
                    ? 'bg-rose-50 border-rose-400 text-rose-800 ring-2 ring-rose-500/20'
                    : 'bg-white border-slate-200 text-slate-600 hover:bg-slate-50'
                "
              >
                <MinusCircle class="w-4 h-4 text-rose-600" />
                <span>Kurangi Poin (-)</span>
              </button>
            </div>
          </div>

          <!-- Jumlah Poin -->
          <div>
            <label class="block text-xs font-bold text-slate-700 mb-1">
              Jumlah Poin
              <span class="text-rose-500">*</span>
            </label>
            <div class="relative">
              <input
                v-model.number="adjustForm.points"
                type="number"
                min="1"
                step="1"
                required
                placeholder="Misal: 50"
                class="w-full px-3 py-2 border border-slate-300 rounded-xl font-mono font-bold text-slate-800 text-sm focus:outline-none focus:ring-2 focus:ring-amber-500"
              />
              <span class="absolute right-3 top-2.5 text-xs font-bold text-slate-400">Poin</span>
            </div>
            <p class="text-[11px] text-slate-500 mt-1">
              Saldo saat ini: <strong class="text-amber-800">{{ formatQty(historyData?.summary?.current_balance ?? customer?.reward_points ?? 0) }} Poin</strong>
            </p>
          </div>

          <!-- Keterangan -->
          <div>
            <label class="block text-xs font-bold text-slate-700 mb-1">
              Alasan / Keterangan Penyesuaian
              <span class="text-rose-500">*</span>
            </label>
            <textarea
              v-model="adjustForm.notes"
              rows="3"
              required
              placeholder="Contoh: Koreksi poin promo ulang tahun, pembatalan komplain pelanggan..."
              class="w-full px-3 py-2 border border-slate-300 rounded-xl text-xs text-slate-800 focus:outline-none focus:ring-2 focus:ring-amber-500"
            ></textarea>
          </div>

          <!-- Modal Actions -->
          <div class="pt-2 flex items-center justify-end gap-2 border-t border-slate-100">
            <button
              type="button"
              @click="showAdjustModal = false"
              class="px-3.5 py-2 text-xs font-bold text-slate-600 hover:bg-slate-100 rounded-xl transition"
            >
              Batal
            </button>
            <button
              type="submit"
              :disabled="isSubmittingAdjust"
              class="px-4 py-2 text-xs font-bold text-white rounded-xl shadow-xs transition"
              :class="
                adjustForm.adjustment_type === 'ADD'
                  ? 'bg-emerald-600 hover:bg-emerald-700'
                  : 'bg-rose-600 hover:bg-rose-700'
              "
            >
              {{ isSubmittingAdjust ? 'Menyimpan...' : 'Simpan Perubahan Poin' }}
            </button>
          </div>
        </form>
      </div>
    </div>
  </div>
</template>
