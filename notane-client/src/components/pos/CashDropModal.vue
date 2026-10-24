<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { api } from '@/services/api'
import type { CashDropStatusSummary } from '@/types'
import { Icon } from '@iconify/vue'
import SupervisorAuthSection from '@/components/pos/SupervisorAuthSection.vue'
import { useLocationStore } from '@/stores/locationStore'

const locationStore = useLocationStore()

const props = defineProps<{
  modelValue: boolean
  shiftId: string
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', val: boolean): void
  (e: 'drop-completed', result: any): void
}>()

const summary = ref<CashDropStatusSummary | null>(null)
const isLoading = ref(false)
const isSubmitting = ref(false)
const errorMsg = ref('')
const successMsg = ref('')

const dropAmount = ref<number>(500000)
const selectedSupervisorId = ref('')
const supervisorPin = ref('')
const supervisorAuthToken = ref('')
const notes = ref('')

async function fetchSummary() {
  if (!props.shiftId) return
  isLoading.value = true
  errorMsg.value = ''
  try {
    const res = await api.get('/pos/cash-drops/current-shift')
    if (res.data?.success) {
      summary.value = res.data.data
    }
  } catch (err: any) {
    errorMsg.value = err.response?.data?.message || 'Gagal memuat status kas laci'
  } finally {
    isLoading.value = false
  }
}

function setQuickAmount(amt: number) {
  dropAmount.value = amt
}

function formatRupiah(num: number): string {
  return new Intl.NumberFormat('id-ID', {
    style: 'currency',
    currency: 'IDR',
    maximumFractionDigits: 0,
  }).format(num)
}

async function handleConfirmDrop() {
  if (dropAmount.value <= 0) {
    errorMsg.value = 'Nominal setor kas harus lebih besar dari 0'
    return
  }

  if (!supervisorPin.value.trim() && !supervisorAuthToken.value.trim()) {
    errorMsg.value = 'Otorisasi Supervisor (Biometrik atau PIN) wajib dilakukan'
    return
  }

  isSubmitting.value = true
  errorMsg.value = ''
  successMsg.value = ''

  try {
    const res = await api.post('/pos/cash-drops', {
      shift_id: props.shiftId,
      amount: dropAmount.value,
      supervisor_id: selectedSupervisorId.value || undefined,
      supervisor_pin: supervisorPin.value.trim() || undefined,
      supervisor_auth_token: supervisorAuthToken.value || undefined,
      notes: notes.value || undefined,
    })

    if (res.data?.success) {
      successMsg.value = res.data.message || 'Setor kas laci berhasil dicatat!'
      emit('drop-completed', res.data.data)
      await fetchSummary()
      supervisorPin.value = ''
      notes.value = ''
      setTimeout(() => {
        closeModal()
      }, 1500)
    } else {
      errorMsg.value = res.data?.message || 'Gagal mencatat cash drop'
    }
  } catch (err: any) {
    errorMsg.value = err.response?.data?.message || 'Gagal memproses setor kas. Pastikan PIN supervisor benar.'
  } finally {
    isSubmitting.value = false
  }
}

function closeModal() {
  emit('update:modelValue', false)
  errorMsg.value = ''
  successMsg.value = ''
  supervisorPin.value = ''
}

onMounted(() => {
  if (props.modelValue) {
    fetchSummary()
  }
})
</script>

<template>
  <div v-if="modelValue" class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/60 backdrop-blur-sm animate-fade-in">
    <div class="bg-white rounded-2xl shadow-2xl border border-slate-200 w-full max-w-2xl max-h-[92vh] flex flex-col overflow-hidden">
      <!-- Header -->
      <div class="px-6 py-4 bg-gradient-to-r from-amber-600 via-amber-500 to-orange-600 text-white flex items-center justify-between">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-xl bg-white/20 flex items-center justify-center backdrop-blur-sm">
            <Icon icon="fluent:vault-24-filled" class="text-2xl" />
          </div>
          <div>
            <h2 class="text-lg font-black tracking-wide">Setor Kas Laci (Cash Drop / Pick Up)</h2>
            <p class="text-xs text-amber-100">Amankan uang kas berlebih ke brankas/supervisor toko</p>
          </div>
        </div>
        <button @click="closeModal" class="p-2 rounded-lg hover:bg-white/20 transition-colors text-white">
          <Icon icon="fluent:dismiss-24-regular" class="text-xl" />
        </button>
      </div>

      <!-- Body -->
      <div class="p-6 overflow-y-auto flex-1 space-y-5">
        <!-- Status Kas Laci Real-Time -->
        <div v-if="summary" class="grid grid-cols-1 sm:grid-cols-3 gap-3">
          <div class="p-3.5 bg-slate-50 border border-slate-200 rounded-xl text-center">
            <span class="text-[11px] font-bold text-slate-500 uppercase block">Kas Laci Sekarang</span>
            <span class="text-base font-black text-slate-800">{{ formatRupiah(summary.current_drawer_cash) }}</span>
          </div>

          <div class="p-3.5 bg-slate-50 border border-slate-200 rounded-xl text-center">
            <span class="text-[11px] font-bold text-slate-500 uppercase block">Batas Maksimal Laci</span>
            <span class="text-base font-black text-slate-800">{{ formatRupiah(summary.max_drawer_limit) }}</span>
          </div>

          <div class="p-3.5 bg-slate-50 border border-slate-200 rounded-xl text-center">
            <span class="text-[11px] font-bold text-slate-500 uppercase block">Total Disetor Shift Ini</span>
            <span class="text-base font-black text-amber-700">{{ formatRupiah(summary.total_dropped) }}</span>
          </div>
        </div>

        <!-- Warning Limit Banner -->
        <div v-if="summary?.should_drop" class="p-3.5 bg-amber-50 border-2 border-amber-400 rounded-xl flex items-start gap-3 text-xs text-amber-950 animate-pulse">
          <Icon icon="fluent:warning-24-filled" class="text-2xl text-amber-600 shrink-0" />
          <div>
            <b class="text-amber-900 block text-sm">Peringatan: Kas Laci Telah Melampaui Batas!</b>
            Uang fisik di laci saat ini ({{ formatRupiah(summary.current_drawer_cash) }}) telah melewati batas keamanan kasir ({{ formatRupiah(summary.max_drawer_limit) }}). Segera lakukan penyetoran ke supervisor untuk menjaga keamanan kas.
          </div>
        </div>

        <!-- Form Input Drop Amount -->
        <div class="space-y-3">
          <label class="block text-xs font-bold text-slate-700 uppercase">Nominal yang Disetor ke Brankas</label>
          <div class="relative">
            <span class="absolute left-4 top-1/2 -translate-y-1/2 font-black text-lg text-slate-400">Rp</span>
            <input
              v-model.number="dropAmount"
              type="number"
              step="10000"
              min="10000"
              class="w-full pl-12 pr-4 py-3 text-xl font-black text-slate-900 bg-slate-50 border-2 border-slate-300 rounded-xl focus:bg-white focus:border-amber-500 focus:outline-none"
            />
          </div>

          <!-- Quick Amount Buttons -->
          <div class="grid grid-cols-4 gap-2">
            <button
              v-for="amt in [500000, 1000000, 1500000, 2000000]"
              :key="amt"
              type="button"
              @click="setQuickAmount(amt)"
              class="py-2 px-1 text-xs font-bold rounded-lg border transition"
              :class="dropAmount === amt ? 'bg-amber-600 text-white border-amber-600' : 'bg-slate-100 text-slate-700 border-slate-200 hover:bg-slate-200'"
            >
              {{ formatRupiah(amt) }}
            </button>
          </div>
        </div>

        <!-- Catatan & Supervisor PIN -->
        <div class="space-y-3">
          <div>
            <label class="block text-xs font-bold text-slate-700 uppercase mb-1">Catatan / Alasan Penyetoran (Opsional)</label>
            <input
              v-model="notes"
              type="text"
              placeholder="Misal: Penyetoran kas siang jam 12 / uang pecahan besar..."
              class="w-full px-3 py-2 text-xs border border-slate-300 rounded-xl bg-white focus:ring-2 focus:ring-amber-500 focus:outline-none"
            />
          </div>

          <div class="p-3 bg-amber-50/70 border border-amber-300 rounded-xl">
            <SupervisorAuthSection
              v-model:model-value-supervisor-id="selectedSupervisorId"
              v-model:model-value-pin="supervisorPin"
              v-model:model-value-auth-token="supervisorAuthToken"
              :outlet-id="locationStore.currentOutletId"
              action-type="CASH_DROP"
              label="Otorisasi Supervisor Kasir (Setor Kas)"
            />
          </div>
        </div>

        <!-- Alert messages -->
        <div v-if="errorMsg" class="p-3 bg-rose-50 border border-rose-200 rounded-xl text-xs font-bold text-rose-700 flex items-center gap-2">
          <Icon icon="fluent:error-circle-24-filled" class="text-lg" />
          <span>{{ errorMsg }}</span>
        </div>

        <div v-if="successMsg" class="p-3 bg-emerald-50 border border-emerald-200 rounded-xl text-xs font-bold text-emerald-700 flex items-center gap-2">
          <Icon icon="fluent:checkmark-circle-24-filled" class="text-lg" />
          <span>{{ successMsg }}</span>
        </div>

        <!-- Riwayat Cash Drop Shift Ini -->
        <div v-if="summary && summary.drops.length > 0" class="pt-2">
          <h4 class="text-xs font-bold text-slate-700 uppercase mb-2">Riwayat Setoran Kas Shift Ini ({{ summary.drops.length }}x)</h4>
          <div class="border border-slate-200 rounded-xl divide-y divide-slate-100 max-h-36 overflow-y-auto text-xs">
            <div v-for="d in summary.drops" :key="d.id" class="p-2.5 flex items-center justify-between">
              <div>
                <span class="font-bold text-slate-800">{{ d.drop_number }}</span>
                <span class="text-slate-500 block text-[11px]">SPV: {{ d.supervisor_name || 'Supervisor' }} &bull; {{ new Date(d.created_at).toLocaleTimeString('id-ID') }}</span>
              </div>
              <span class="font-black text-amber-700">{{ formatRupiah(d.amount) }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Footer Buttons -->
      <div class="px-6 py-4 bg-slate-50 border-t border-slate-200 flex items-center justify-between">
        <button
          type="button"
          @click="closeModal"
          class="px-4 py-2 border border-slate-300 text-slate-700 hover:bg-slate-100 font-bold text-xs rounded-xl transition"
        >
          Batal
        </button>

        <button
          type="button"
          @click="handleConfirmDrop"
          :disabled="isSubmitting || dropAmount <= 0 || !supervisorPin.trim()"
          class="px-6 py-2.5 bg-amber-600 hover:bg-amber-700 disabled:opacity-50 text-white font-black text-sm rounded-xl flex items-center gap-2 shadow-lg shadow-amber-600/30 transition"
        >
          <Icon v-if="isSubmitting" icon="fluent:spinner-ios-20-regular" class="animate-spin text-lg" />
          <Icon v-else icon="fluent:arrow-download-24-filled" class="text-lg" />
          <span>Setor Kas Sekarang ({{ formatRupiah(dropAmount) }})</span>
        </button>
      </div>
    </div>
  </div>
</template>
