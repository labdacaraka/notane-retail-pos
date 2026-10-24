<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { X, Calendar, Lock, Printer } from 'lucide-vue-next'
import { useShiftStore } from '@/stores/shiftStore'
import { useAuthStore } from '@/stores/authStore'
import { useLocationStore } from '@/stores/locationStore'
import { api } from '@/services/api'
import SupervisorAuthSection from '@/components/pos/SupervisorAuthSection.vue'
import SearchableSelect from '@/components/common/SearchableSelect.vue'

const props = defineProps<{
  show: boolean
  mode: 'OPEN' | 'CLOSE'
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'success'): void
}>()

const shiftStore = useShiftStore()
const authStore = useAuthStore()
const locationStore = useLocationStore()

const openingCash = ref<number>(100000)
const actualCash = ref<number>(0)
const supervisors = ref<Array<{ id: string; username: string; full_name: string; role_name: string }>>([])
const selectedSupervisorId = ref<string>('')
const selectedOutletId = ref<string>('')
const supervisorPin = ref<string>('')
const supervisorAuthToken = ref<string>('')
const isSubmitting = ref<boolean>(false)
const errorMsg = ref<string>('')
const zReport = ref<any>(null)

async function loadSupervisors() {
  try {
    const res = await api.get('/auth/supervisors')
    if (res.data?.success && Array.isArray(res.data.data)) {
      supervisors.value = res.data.data
      if (supervisors.value.length > 0 && !selectedSupervisorId.value) {
        selectedSupervisorId.value = supervisors.value[0].id
      }
    }
  } catch (err) {
    console.error('Gagal memuat supervisor:', err)
  }
}

async function loadOutlets() {
  try {
    await locationStore.fetchAccessibleOutlets()
  } catch (err) {
    console.error('Gagal memuat outlet yang dapat diakses:', err)
  }
}

function handleOutletChange(newOutletId: string) {
  selectedOutletId.value = newOutletId
  selectedSupervisorId.value = ''
  supervisorPin.value = ''
  supervisorAuthToken.value = ''
}

onMounted(() => {
  loadSupervisors()
  loadOutlets()
})

watch(
  () => props.show,
  (val) => {
    if (val) {
      errorMsg.value = ''
      supervisorPin.value = ''
      supervisorAuthToken.value = ''
      loadSupervisors()
      loadOutlets()
    }
  }
)

function formatCurrency(val: number): string {
  return new Intl.NumberFormat('id-ID', {
    style: 'currency',
    currency: 'IDR',
    minimumFractionDigits: 0,
    maximumFractionDigits: 0,
  }).format(val)
}

async function handleOpenShift() {
  if (!selectedOutletId.value) {
    errorMsg.value = 'Cabang / Lokasi shift kasir wajib dipilih'
    return
  }
  if (openingCash.value < 0) {
    errorMsg.value = 'Modal awal kasir tidak boleh negatif'
    return
  }
  if (!supervisorPin.value && !supervisorAuthToken.value) {
    errorMsg.value = 'Otorisasi supervisor (Biometrik atau PIN) wajib dilakukan'
    return
  }

  isSubmitting.value = true
  errorMsg.value = ''

  try {
    await shiftStore.openShift(
      authStore.terminalId,
      openingCash.value,
      selectedSupervisorId.value || undefined,
      supervisorPin.value || undefined,
      selectedOutletId.value,
      supervisorAuthToken.value || undefined
    )
    if (locationStore.currentOutletId !== selectedOutletId.value) {
      await locationStore.switchLocation(selectedOutletId.value)
    }
    emit('success')
    emit('close')
  } catch (err: any) {
    errorMsg.value = err.response?.data?.message || err.message || 'Gagal membuka shift'
  } finally {
    isSubmitting.value = false
  }
}

async function handleCloseShift() {
  if (!supervisorPin.value && !supervisorAuthToken.value) {
    errorMsg.value = 'Otorisasi supervisor (Biometrik atau PIN) wajib dilakukan'
    return
  }

  isSubmitting.value = true
  errorMsg.value = ''

  try {
    const res = await shiftStore.closeShift(
      actualCash.value,
      selectedSupervisorId.value || undefined,
      supervisorPin.value || undefined,
      supervisorAuthToken.value || undefined
    )
    zReport.value = res
    emit('success')
  } catch (err: any) {
    errorMsg.value = err.response?.data?.message || err.message || 'Gagal menutup shift'
  } finally {
    isSubmitting.value = false
  }
}

function printZReport() {
  window.print()
}

function paymentIcon(category: string): string {
  const cat = (category || '').toUpperCase()
  if (cat.includes('CASH') || cat.includes('TUNAI')) return '💵'
  if (cat.includes('QRIS') || cat.includes('QR')) return '📱'
  if (cat.includes('EDC') || cat.includes('CARD') || cat.includes('DEBIT') || cat.includes('KREDIT')) return '💳'
  if (cat.includes('POINT') || cat.includes('POIN') || cat.includes('REWARD')) return '⭐'
  if (cat.includes('TRANSFER') || cat.includes('BANK')) return '🏦'
  return '💰'
}
</script>

<template>
  <div
    v-if="show"
    class="fixed inset-0 z-50 bg-slate-900/50 backdrop-blur-xs flex items-center justify-center p-4 select-none animate-fadeIn"
  >
    <!-- Open Shift Dialog -->
    <div
      v-if="mode === 'OPEN'"
      class="bg-white border border-slate-300 w-full max-w-md rounded-2xl shadow-2xl overflow-hidden flex flex-col text-slate-900"
    >
      <div class="bg-slate-100 p-4 px-6 border-b border-slate-300 flex items-center justify-between">
        <div class="flex items-center gap-2 text-emerald-700">
          <Calendar class="w-5 h-5" />
          <h3 class="font-bold text-sm text-slate-900">Buka Shift Kasir Baru</h3>
        </div>
        <button @click="emit('close')" class="text-slate-400 hover:text-slate-700 p-1">
          <X class="w-4 h-4" />
        </button>
      </div>

      <div class="p-6 space-y-4">
        <div class="bg-slate-50 p-3 rounded-xl border border-slate-200 space-y-1 text-xs text-slate-600">
          <div>Terminal: <strong class="text-slate-900 font-bold">{{ authStore.terminalId }}</strong></div>
          <div>Kasir: <strong class="text-slate-900 font-bold">{{ authStore.user?.full_name }}</strong></div>
        </div>

        <div>
          <label class="block text-xs font-bold text-slate-700 mb-1">
            Cabang / Lokasi Shift Kasir *
          </label>
          <SearchableSelect
            v-model="selectedOutletId"
            api-endpoint="/me/outlets"
            :options="locationStore.accessibleOutlets"
            value-key="id"
            label-key="name"
            secondary-key="code"
            placeholder="-- Pilih Cabang / Lokasi Shift --"
            search-placeholder="Ketik nama atau kode cabang..."
            :allow-clear="false"
            @change="handleOutletChange"
          />
          <p class="text-[11px] text-slate-500 mt-1">Stok produk dan harga kasir akan disesuaikan otomatis dengan lokasi shift ini.</p>
        </div>

        <div>
          <label class="block text-xs font-bold text-slate-700 mb-1.5">
            Modal Awal Kasir / Floating Cash (Rp)
          </label>
          <input
            v-model.number="openingCash"
            type="number"
            placeholder="100000"
            class="w-full bg-slate-50 border-2 border-emerald-600/70 rounded-xl px-4 py-3 text-2xl font-mono text-slate-900 text-right focus:bg-white focus:outline-none focus:border-emerald-600 font-bold"
          />
          <p class="text-[11px] text-slate-500 mt-1">Uang pecahan kecil untuk modal uang kembalian di laci kasir.</p>
        </div>

        <!-- Supervisor Biometric & PIN Section -->
        <SupervisorAuthSection
          v-model:model-value-supervisor-id="selectedSupervisorId"
          v-model:model-value-pin="supervisorPin"
          v-model:model-value-auth-token="supervisorAuthToken"
          :outlet-id="selectedOutletId"
          action-type="SHIFT_OPEN"
          label="Supervisor Penanggung Jawab Buka Shift"
        />

        <div v-if="errorMsg" class="bg-rose-50 border border-rose-300 p-3 rounded-xl text-rose-700 text-xs font-semibold">
          ⚠️ {{ errorMsg }}
        </div>
      </div>

      <div class="bg-slate-100 p-4 px-6 border-t border-slate-300 flex justify-end gap-2">
        <button
          @click="emit('close')"
          class="bg-slate-200 hover:bg-slate-300 text-slate-700 font-bold px-4 py-2 rounded-xl text-xs border border-slate-300 transition active:scale-95"
        >
          Batal
        </button>
        <button
          @click="handleOpenShift"
          :disabled="isSubmitting"
          class="bg-emerald-600 hover:bg-emerald-700 text-white font-bold px-5 py-2 rounded-xl text-xs shadow-sm border border-emerald-700 transition active:scale-95"
        >
          {{ isSubmitting ? 'Membuka...' : 'Mulai Shift Kasir' }}
        </button>
      </div>
    </div>

    <!-- Close Shift (Blind Settlement) Dialog -->
    <div
      v-else-if="mode === 'CLOSE' && !zReport"
      class="bg-white border border-slate-300 w-full max-w-md rounded-2xl shadow-2xl overflow-hidden flex flex-col text-slate-900"
    >
      <div class="bg-slate-100 p-4 px-6 border-b border-slate-300 flex items-center justify-between">
        <div class="flex items-center gap-2 text-rose-700">
          <Lock class="w-5 h-5" />
          <h3 class="font-bold text-sm text-slate-900">Tutup Shift Kasir (Blind Settlement)</h3>
        </div>
        <button @click="emit('close')" class="text-slate-400 hover:text-slate-700 p-1">
          <X class="w-4 h-4" />
        </button>
      </div>

      <div class="p-6 space-y-4">
        <div class="bg-amber-50 border border-amber-300 rounded-xl p-3 text-xs text-amber-900">
          <strong>Blind Count:</strong> Hitung seluruh uang fisik tunai yang ada di dalam laci kasir sekarang dan masukkan jumlah totalnya.
        </div>

        <div>
          <label class="block text-xs font-bold text-slate-700 mb-1.5">
            Total Uang Fisik di Laci Kasir (Rp)
          </label>
          <input
            v-model.number="actualCash"
            type="number"
            class="w-full bg-slate-50 border-2 border-rose-500/70 rounded-xl px-4 py-3 text-2xl font-mono text-slate-900 text-right focus:bg-white focus:outline-none focus:border-rose-600 font-bold"
          />
        </div>

        <!-- Supervisor Biometric & PIN Section -->
        <SupervisorAuthSection
          v-model:model-value-supervisor-id="selectedSupervisorId"
          v-model:model-value-pin="supervisorPin"
          v-model:model-value-auth-token="supervisorAuthToken"
          :outlet-id="shiftStore.currentShift?.outlet_id || locationStore.currentOutletId"
          action-type="SHIFT_CLOSE"
          label="Supervisor Penanggung Jawab Tutup Shift"
        />

        <div v-if="errorMsg" class="bg-rose-50 border border-rose-300 p-3 rounded-xl text-rose-700 text-xs font-semibold">
          ⚠️ {{ errorMsg }}
        </div>
      </div>

      <div class="bg-slate-100 p-4 px-6 border-t border-slate-300 flex justify-end gap-2">
        <button
          @click="emit('close')"
          class="bg-slate-200 hover:bg-slate-300 text-slate-700 font-bold px-4 py-2 rounded-xl text-xs border border-slate-300 transition active:scale-95"
        >
          Batal
        </button>
        <button
          @click="handleCloseShift"
          :disabled="isSubmitting"
          class="bg-rose-600 hover:bg-rose-700 text-white font-bold px-5 py-2 rounded-xl text-xs shadow-sm border border-rose-700 transition active:scale-95"
        >
          {{ isSubmitting ? 'Memproses...' : 'Tutup Shift & Cetak Z-Report' }}
        </button>
      </div>
    </div>

    <!-- Z-Report Modal Display -->
    <div
      v-else-if="zReport"
      class="bg-white border border-slate-300 w-full max-w-lg rounded-2xl shadow-2xl flex flex-col text-slate-900 overflow-hidden"
    >
      <!-- Header -->
      <div class="bg-slate-800 p-4 text-center text-white">
        <h4 class="font-bold text-sm tracking-wide">📋 LAPORAN PENUTUPAN SHIFT</h4>
        <p class="text-[10px] text-slate-400 mt-0.5">Z-REPORT — NOTANE RETAIL POS</p>
      </div>

      <!-- Scrollable content -->
      <div class="overflow-y-auto max-h-[70vh] p-4 space-y-3">
        <!-- Shift Info -->
        <div class="bg-slate-50 border border-slate-200 rounded-xl p-3 text-[11px] space-y-1">
          <div class="flex justify-between text-slate-700">
            <span class="font-semibold">Kasir</span>
            <span class="text-slate-900 font-bold">{{ zReport.cashier_name }}</span>
          </div>
          <div class="flex justify-between text-slate-700">
            <span class="font-semibold">Terminal</span>
            <span class="text-slate-900">{{ zReport.shift?.terminal_id }}</span>
          </div>
          <div class="flex justify-between text-slate-700">
            <span class="font-semibold">Outlet</span>
            <span class="text-slate-900">{{ zReport.shift?.outlet_name || zReport.outlet_name || '-' }}</span>
          </div>
          <div class="flex justify-between text-slate-700 pt-1 border-t border-slate-200">
            <span class="font-semibold">Mulai Shift</span>
            <span class="text-slate-900">{{ new Date(zReport.shift?.opened_at).toLocaleString('id-ID', { day:'2-digit', month:'short', year:'numeric', hour:'2-digit', minute:'2-digit' }) }}</span>
          </div>
          <div class="flex justify-between text-slate-700">
            <span class="font-semibold">Tutup Shift</span>
            <span class="text-slate-900">{{ new Date(zReport.shift?.closed_at).toLocaleString('id-ID', { day:'2-digit', month:'short', year:'numeric', hour:'2-digit', minute:'2-digit' }) }}</span>
          </div>
          <div class="flex justify-between text-slate-700">
            <span class="font-semibold">Jumlah Transaksi</span>
            <span class="text-slate-900 font-bold">{{ zReport.total_transactions ?? zReport.transaction_count ?? '-' }} nota</span>
          </div>
        </div>

        <!-- Payment Method Breakdown -->
        <div>
          <h5 class="text-[10px] font-bold text-slate-500 uppercase tracking-widest mb-2">Rekapan Penerimaan per Metode Bayar</h5>
          <div class="border border-slate-200 rounded-xl overflow-hidden">
            <!-- Dynamic payment_breakdown if available -->
            <template v-if="zReport.payment_breakdown && zReport.payment_breakdown.length > 0">
              <div
                v-for="(pm, idx) in zReport.payment_breakdown"
                :key="pm.category || pm.method_code || idx"
                class="flex items-center justify-between px-3 py-2 text-[11px]"
                :class="idx % 2 === 0 ? 'bg-white' : 'bg-slate-50'"
              >
                <div class="flex items-center gap-2">
                  <span class="text-base">{{ paymentIcon(pm.category || pm.method_code) }}</span>
                  <span class="text-slate-700 font-medium">{{ pm.method_name || pm.name }}</span>
                  <span v-if="pm.transaction_count" class="text-slate-400 text-[10px]">({{ pm.transaction_count }}x)</span>
                </div>
                <span class="font-bold text-slate-900">{{ formatCurrency(pm.total_amount || pm.amount || 0) }}</span>
              </div>
            </template>

            <!-- Fallback: use individual totals if payment_breakdown not available -->
            <template v-else>
              <div class="flex items-center justify-between px-3 py-2 text-[11px] bg-white">
                <div class="flex items-center gap-2">
                  <span class="text-base">💵</span>
                  <span class="text-slate-700 font-medium">Uang Tunai (Cash)</span>
                </div>
                <span class="font-bold text-slate-900">{{ formatCurrency(zReport.total_cash_received ?? 0) }}</span>
              </div>
              <div class="flex items-center justify-between px-3 py-2 text-[11px] bg-slate-50">
                <div class="flex items-center gap-2">
                  <span class="text-base">📱</span>
                  <span class="text-slate-700 font-medium">QRIS</span>
                </div>
                <span class="font-bold text-slate-900">{{ formatCurrency(zReport.total_qris_received ?? 0) }}</span>
              </div>
              <div class="flex items-center justify-between px-3 py-2 text-[11px] bg-white">
                <div class="flex items-center gap-2">
                  <span class="text-base">💳</span>
                  <span class="text-slate-700 font-medium">Debit / Kredit EDC</span>
                </div>
                <span class="font-bold text-slate-900">{{ formatCurrency(zReport.total_card_received ?? 0) }}</span>
              </div>
              <div v-if="(zReport.total_points_redeemed ?? 0) > 0" class="flex items-center justify-between px-3 py-2 text-[11px] bg-slate-50">
                <div class="flex items-center gap-2">
                  <span class="text-base">⭐</span>
                  <span class="text-slate-700 font-medium">Poin Reward (Redemption)</span>
                </div>
                <span class="font-bold text-slate-900">{{ formatCurrency(zReport.total_points_redeemed ?? 0) }}</span>
              </div>
              <div v-if="(zReport.total_transfer_received ?? 0) > 0" class="flex items-center justify-between px-3 py-2 text-[11px] bg-white">
                <div class="flex items-center gap-2">
                  <span class="text-base">🏦</span>
                  <span class="text-slate-700 font-medium">Transfer Bank</span>
                </div>
                <span class="font-bold text-slate-900">{{ formatCurrency(zReport.total_transfer_received ?? 0) }}</span>
              </div>
            </template>

            <!-- Grand Total row -->
            <div class="flex items-center justify-between px-3 py-2.5 text-[12px] bg-slate-800 text-white">
              <span class="font-bold">Total Penjualan Seluruh Shift</span>
              <span class="font-bold">{{ formatCurrency(zReport.total_net_sales ?? 0) }}</span>
            </div>
          </div>
        </div>

        <!-- Cash Drawer Reconciliation -->
        <div>
          <h5 class="text-[10px] font-bold text-slate-500 uppercase tracking-widest mb-2">Rekonsiliasi Laci Kasir (Cash Drawer)</h5>
          <div class="border border-slate-200 rounded-xl overflow-hidden">
            <div class="flex items-center justify-between px-3 py-2 text-[11px] bg-white">
              <span class="text-slate-600">Modal Awal Shift (Opening Cash)</span>
              <span class="font-semibold text-slate-800">{{ formatCurrency(zReport.shift?.opening_cash ?? 0) }}</span>
            </div>
            <div class="flex items-center justify-between px-3 py-2 text-[11px] bg-slate-50">
              <span class="text-slate-600">+ Penerimaan Tunai Shift</span>
              <span class="font-semibold text-emerald-700">+{{ formatCurrency(zReport.total_cash_received ?? 0) }}</span>
            </div>
            <div class="flex items-center justify-between px-3 py-2.5 text-[11px] bg-amber-50 border-t border-amber-200">
              <span class="text-amber-800 font-bold">= Seharusnya di Laci</span>
              <span class="font-bold text-amber-800">{{ formatCurrency(zReport.expected_cash_in_drawer ?? 0) }}</span>
            </div>
            <div class="flex items-center justify-between px-3 py-2 text-[11px] bg-white">
              <span class="text-slate-600">Hitungan Fisik Kasir</span>
              <span class="font-semibold text-slate-800">{{ formatCurrency(zReport.actual_cash_counted ?? 0) }}</span>
            </div>
            <div
              class="flex items-center justify-between px-3 py-2.5 text-[12px] font-bold border-t-2"
              :class="(zReport.discrepancy ?? 0) < 0
                ? 'bg-red-50 border-red-300 text-red-700'
                : (zReport.discrepancy ?? 0) > 0
                ? 'bg-green-50 border-green-300 text-green-700'
                : 'bg-emerald-50 border-emerald-300 text-emerald-700'"
            >
              <span>Selisih Kas</span>
              <span>
                {{ formatCurrency(Math.abs(zReport.discrepancy ?? 0)) }}
                <span class="text-[10px] ml-1">
                  {{ (zReport.discrepancy ?? 0) < 0 ? '⚠️ KURANG KAS' : (zReport.discrepancy ?? 0) > 0 ? '✅ LEBIH KAS' : '✅ SESUAI' }}
                </span>
              </span>
            </div>
          </div>
        </div>

        <!-- Supervisor note -->
        <p class="text-[10px] text-slate-400 text-center">
          Pencatatan shift telah dibukukan otomatis ke General Ledger.
        </p>
      </div>

      <!-- Footer actions -->
      <div class="p-4 border-t border-slate-200 flex items-center gap-2 bg-slate-50">
        <button
          @click="printZReport"
          class="flex-1 bg-emerald-600 hover:bg-emerald-700 text-white py-2.5 rounded-xl text-xs font-bold flex items-center justify-center gap-1.5 transition active:scale-95 shadow-sm border border-emerald-700"
        >
          <Printer class="w-4 h-4" />
          <span>Cetak Z-Report</span>
        </button>
        <button
          @click="zReport = null; emit('close')"
          class="bg-slate-100 hover:bg-slate-200 border border-slate-300 text-slate-800 px-4 py-2.5 rounded-xl text-xs font-bold transition active:scale-95"
        >
          Tutup
        </button>
      </div>
    </div>
  </div>
</template>
