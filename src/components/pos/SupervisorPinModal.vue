<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue'
import { ShieldAlert, X, Check } from 'lucide-vue-next'
import { api } from '@/services/api'

const props = defineProps<{
  show: boolean
  title?: string
  description?: string
  actionType?: string
}>()

const emit = defineEmits<{
  (e: 'close'): void
  (e: 'approved', supervisor: { id: string; name: string }): void
}>()

const supervisors = ref<Array<{ id: string; username: string; full_name: string; role_name: string }>>([])
const selectedSupervisorId = ref<string>('')
const pin = ref<string>('')
const isChecking = ref<boolean>(false)
const errorMessage = ref<string>('')
const inputRef = ref<HTMLInputElement | null>(null)

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

function focusInput() {
  setTimeout(() => inputRef.value?.focus(), 50)
}

function appendPin(digit: string) {
  if (pin.value.length < 6) {
    pin.value += digit
  }
}

function clearPin() {
  pin.value = ''
  errorMessage.value = ''
}

async function verifyPin() {
  if (pin.value.length < 4) {
    errorMessage.value = 'Masukkan minimal 4 digit PIN Supervisor'
    return
  }

  isChecking.value = true
  errorMessage.value = ''

  try {
    const res = await api.post('/auth/supervisor-approve', {
      supervisor_id: selectedSupervisorId.value || undefined,
      pin: pin.value,
      action: props.actionType || 'SUPERVISOR_OVERRIDE',
    })

    if (res.data.success) {
      emit('approved', {
        id: res.data.data.supervisor_id,
        name: res.data.data.supervisor_name,
      })
      clearPin()
    } else {
      errorMessage.value = res.data.message || 'PIN Supervisor salah'
      pin.value = ''
    }
  } catch (err: any) {
    errorMessage.value = err.response?.data?.message || 'PIN Supervisor salah atau tidak memiliki otorisasi'
    pin.value = ''
  } finally {
    isChecking.value = false
  }
}

function handleKeyDown(e: KeyboardEvent) {
  if (e.key === 'Escape' && props.show) {
    emit('close')
  } else if (e.key === 'Enter' && props.show) {
    verifyPin()
  }
}

onMounted(() => {
  window.addEventListener('keydown', handleKeyDown)
  loadSupervisors()
  focusInput()
})

watch(
  () => props.show,
  (val) => {
    if (val) {
      clearPin()
      loadSupervisors()
      focusInput()
    }
  }
)

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeyDown)
})
</script>

<template>
  <div
    v-if="show"
    class="fixed inset-0 z-50 bg-slate-900/50 backdrop-blur-xs flex items-center justify-center p-4 select-none animate-fadeIn"
  >
    <div class="bg-white border border-slate-300 w-full max-w-sm rounded-2xl shadow-2xl overflow-hidden text-slate-900">
      <!-- Header -->
      <div class="bg-slate-100 p-4 border-b border-slate-300 flex items-center justify-between">
        <div class="flex items-center gap-2 text-amber-700">
          <ShieldAlert class="w-5 h-5" />
          <h3 class="font-bold text-sm text-slate-900">{{ title || 'Otorisasi Supervisor' }}</h3>
        </div>
        <button @click="emit('close')" class="text-slate-400 hover:text-slate-700 p-1">
          <X class="w-4 h-4" />
        </button>
      </div>

      <!-- Body -->
      <div class="p-5 space-y-3.5">
        <p class="text-xs text-slate-600 text-center">
          {{ description || 'Aksi ini memerlukan persetujuan Supervisor Toko. Pilih Supervisor dan masukkan PIN di bawah ini:' }}
        </p>

        <!-- Supervisor Selector -->
        <div>
          <label class="block text-xs font-bold text-slate-700 mb-1">Supervisor Penanggung Jawab:</label>
          <select
            v-model="selectedSupervisorId"
            class="w-full bg-slate-50 border border-slate-300 rounded-xl px-3 py-2 text-xs text-slate-900 font-sans focus:bg-white focus:border-amber-600 focus:outline-none"
          >
            <option v-for="spv in supervisors" :key="spv.id" :value="spv.id">
              {{ spv.full_name }} ({{ spv.role_name }})
            </option>
            <option v-if="supervisors.length === 0" value="">
              -- Memuat Supervisor --
            </option>
          </select>
        </div>

        <!-- PIN Input Field -->
        <div class="relative">
          <input
            ref="inputRef"
            v-model="pin"
            type="password"
            maxlength="6"
            placeholder="••••••"
            class="w-full bg-slate-50 border-2 border-amber-500/70 rounded-xl py-3 text-center text-3xl tracking-widest font-mono text-slate-900 focus:bg-white focus:outline-none focus:border-amber-600 focus:ring-4 focus:ring-amber-500/20"
            @keyup.enter="verifyPin"
          />
        </div>

        <!-- Numeric Keypad -->
        <div class="grid grid-cols-3 gap-2 pt-1">
          <button
            v-for="n in 9"
            :key="n"
            @click="appendPin(n.toString())"
            class="bg-slate-100 hover:bg-slate-200 text-slate-900 font-mono text-lg font-bold py-3 rounded-xl border border-slate-300 transition active:scale-95 shadow-xs"
          >
            {{ n }}
          </button>
          <button
            @click="clearPin"
            class="bg-rose-50 hover:bg-rose-100 text-rose-700 font-bold py-3 rounded-xl border border-rose-300 transition active:scale-95 text-xs shadow-xs"
          >
            HAPUS
          </button>
          <button
            @click="appendPin('0')"
            class="bg-slate-100 hover:bg-slate-200 text-slate-900 font-mono text-lg font-bold py-3 rounded-xl border border-slate-300 transition active:scale-95 shadow-xs"
          >
            0
          </button>
          <button
            @click="verifyPin"
            :disabled="isChecking"
            class="bg-emerald-600 hover:bg-emerald-700 text-white font-bold py-3 rounded-xl border border-emerald-700 transition active:scale-95 flex items-center justify-center gap-1 text-xs shadow-xs"
          >
            <Check class="w-4 h-4" />
            <span>OK</span>
          </button>
        </div>

        <!-- Error Msg -->
        <div v-if="errorMessage" class="bg-rose-50 border border-rose-300 p-2.5 rounded-xl text-rose-700 text-xs text-center font-medium">
          ⚠️ {{ errorMessage }}
        </div>
      </div>
    </div>
  </div>
</template>
