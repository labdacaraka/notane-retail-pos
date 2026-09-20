<script setup lang="ts">
import { ref } from 'vue'
import { X, Calendar, Lock, Printer } from 'lucide-vue-next'
import { useShiftStore } from '@/stores/shiftStore'
import { useAuthStore } from '@/stores/authStore'

defineProps<{
  show: boolean
  mode: 'OPEN' | 'CLOSE'
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'success'): void
}>()

const shiftStore = useShiftStore()
const authStore = useAuthStore()

const openingCash = ref<number>(100000)
const actualCash = ref<number>(0)
const supervisorPin = ref<string>('')
const isSubmitting = ref<boolean>(false)
const errorMsg = ref<string>('')
const zReport = ref<any>(null)

function formatCurrency(val: number): string {
  return new Intl.NumberFormat('id-ID', {
    style: 'currency',
    currency: 'IDR',
    minimumFractionDigits: 0,
    maximumFractionDigits: 0,
  }).format(val)
}

async function handleOpenShift() {
  if (openingCash.value < 0) {
    errorMsg.value = 'Modal awal kasir tidak boleh negatif'
    return
  }

  isSubmitting.value = true
  errorMsg.value = ''

  try {
    await shiftStore.openShift(authStore.terminalId, openingCash.value)
    emit('success')
    emit('close')
  } catch (err: any) {
    errorMsg.value = err.response?.data?.message || err.message || 'Gagal membuka shift'
  } finally {
    isSubmitting.value = false
  }
}

async function handleCloseShift() {
  isSubmitting.value = true
  errorMsg.value = ''

  try {
    const res = await shiftStore.closeShift(actualCash.value, supervisorPin.value || undefined)
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

        <div>
          <label class="block text-xs font-bold text-slate-600 mb-1">
            PIN Supervisor (Opsional)
          </label>
          <input
            v-model="supervisorPin"
            type="password"
            placeholder="PIN jika diperlukan approval"
            class="w-full bg-slate-50 border border-slate-300 rounded-xl px-3 py-2 text-sm text-slate-900 font-mono focus:bg-white focus:border-emerald-600 focus:outline-none"
          />
        </div>

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
      class="bg-white border border-slate-300 w-full max-w-sm rounded-2xl shadow-2xl p-4 flex flex-col items-center space-y-4 text-slate-900"
    >
      <div class="bg-white text-black font-mono text-xs p-4 rounded border border-slate-300 shadow-sm w-full">
        <div class="text-center pb-2 border-b border-dashed border-gray-400">
          <h4 class="font-bold text-sm">LAPORAN PENUTUPAN SHIFT (Z-REPORT)</h4>
          <p class="text-[10px]">NOTANE RETAIL STORE</p>
        </div>

        <div class="py-2 border-b border-dashed border-gray-400 text-[10px] space-y-0.5">
          <div class="flex justify-between">
            <span>Kasir: {{ zReport.cashier_name }}</span>
            <span>Terminal: {{ zReport.shift.terminal_id }}</span>
          </div>
          <div class="flex justify-between">
            <span>Buka: {{ new Date(zReport.shift.opened_at).toLocaleTimeString() }}</span>
            <span>Tutup: {{ new Date(zReport.shift.closed_at).toLocaleTimeString() }}</span>
          </div>
        </div>

        <div class="py-2 border-b border-dashed border-gray-400 text-[10px] space-y-1">
          <div class="flex justify-between">
            <span>Modal Awal Kas:</span>
            <span>{{ formatCurrency(zReport.shift.opening_cash) }}</span>
          </div>
          <div class="flex justify-between">
            <span>Penjualan Tunai:</span>
            <span>{{ formatCurrency(zReport.total_cash_received) }}</span>
          </div>
          <div class="flex justify-between">
            <span>Penjualan QRIS:</span>
            <span>{{ formatCurrency(zReport.total_qris_received) }}</span>
          </div>
          <div class="flex justify-between">
            <span>Penjualan EDC:</span>
            <span>{{ formatCurrency(zReport.total_card_received) }}</span>
          </div>
          <div class="flex justify-between font-bold border-t border-gray-300 pt-1">
            <span>Total Penjualan:</span>
            <span>{{ formatCurrency(zReport.total_net_sales) }}</span>
          </div>
        </div>

        <div class="py-2 border-b border-dashed border-gray-400 text-[10px] space-y-1 font-bold">
          <div class="flex justify-between">
            <span>Seharusnya di Laci:</span>
            <span>{{ formatCurrency(zReport.expected_cash_in_drawer) }}</span>
          </div>
          <div class="flex justify-between">
            <span>Hitungan Fisik Kasir:</span>
            <span>{{ formatCurrency(zReport.actual_cash_counted) }}</span>
          </div>
          <div
            class="flex justify-between"
            :class="zReport.discrepancy < 0 ? 'text-red-600' : zReport.discrepancy > 0 ? 'text-green-600' : ''"
          >
            <span>Selisih (Discrepancy):</span>
            <span>{{ formatCurrency(zReport.discrepancy) }} ({{ zReport.discrepancy < 0 ? 'Kurang Kas' : zReport.discrepancy > 0 ? 'Lebih Kas' : 'Sesuai' }})</span>
          </div>
        </div>

        <div class="text-center pt-2 text-[9px] text-gray-500">
          Pencatatan shift telah dibukukan otomatis ke General Ledger.
        </div>
      </div>

      <div class="flex items-center gap-2 w-full">
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
