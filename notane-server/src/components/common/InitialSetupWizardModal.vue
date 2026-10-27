<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import {
  Sparkles,
  Store,
  Building2,
  Database,
  ShieldCheck,
  CheckCircle2,
  ArrowRight,
  ArrowLeft,
  Upload,
  Trash2,
  Shuffle,
  Eye,
  EyeOff,
  AlertCircle,
  X,
  ShoppingBag,
  Check,
  RefreshCw,
} from 'lucide-vue-next'
import { useInitialSetupStore } from '@/stores/initialSetupStore'
import { useStoreBrandingStore } from '@/stores/storeBrandingStore'
import { useNetworkStore } from '@/stores/networkStore'

const props = defineProps<{
  modelValue: boolean
  forceFirstRun?: boolean
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
  (e: 'completed'): void
}>()

const setupStore = useInitialSetupStore()
const brandingStore = useStoreBrandingStore()
const networkStore = useNetworkStore()

const currentStep = ref<number>(1)
const totalSteps = 5

// Form State
const form = ref({
  store_name: '',
  store_tagline: '',
  store_logo_url: null as string | null,
  main_outlet_name: '',
  store_address: '',
  store_phone: '',
  include_dummy_products: false,
  cash_rounding_mode: 'NONE',
  admin_full_name: '',
  admin_password: '',
  supervisor_pin: '',
  pairing_code: '',
})

const showPassword = ref(false)
const isUploadingLogo = ref(false)
const logoInputRef = ref<HTMLInputElement | null>(null)
const isSubmitting = ref(false)
const submissionError = ref<string | null>(null)
const isSuccessCelebration = ref(false)

// Populate initial data from currentConfig or defaults
function initFormData() {
  const cfg = setupStore.currentConfig
  form.value.store_name = cfg?.store_name || brandingStore.storeName || 'NOTANE RETAIL STORE'
  form.value.store_tagline = cfg?.store_tagline || brandingStore.storeTagline || 'Pusat Belanja Retail Modern & Terpercaya'
  form.value.store_logo_url = cfg?.store_logo_url || brandingStore.storeLogoUrl || null
  form.value.main_outlet_name = cfg?.main_outlet_name || 'Notane Retail Flagship Store'
  form.value.store_address = cfg?.store_address || 'Jl. Sudirman No. 123, Jakarta Pusat'
  form.value.store_phone = cfg?.store_phone || '021-5551234'
  form.value.pairing_code = cfg?.pairing_code || networkStore.hostStatus.pairing_code || 'NOTANE-8899'
  form.value.cash_rounding_mode = cfg?.cash_rounding_mode || 'NONE'
  form.value.admin_full_name = cfg?.admin_full_name || 'Administrator Utama'
  form.value.admin_password = ''
  form.value.supervisor_pin = ''
  form.value.include_dummy_products = setupStore.dummyProductsCount > 0
}

watch(
  () => props.modelValue,
  (open) => {
    if (open) {
      currentStep.value = 1
      isSuccessCelebration.value = false
      submissionError.value = null
      initFormData()
    }
  },
  { immediate: true }
)

const isFirstRun = computed(() => {
  return props.forceFirstRun || !setupStore.isSetupCompleted
})

// Step validations
const isStep1Valid = computed(() => {
  return form.value.store_name.trim().length >= 3
})

const isStep2Valid = computed(() => {
  return form.value.main_outlet_name.trim().length >= 3 && form.value.store_address.trim().length >= 5
})

const isStep3Valid = computed(() => true)

const isStep4Valid = computed(() => {
  if (form.value.pairing_code.trim().length < 4) return false
  if (form.value.admin_password.trim() && form.value.admin_password.trim().length < 6) return false
  if (form.value.supervisor_pin.trim() && form.value.supervisor_pin.trim().length < 4) return false
  return true
})

function nextStep() {
  submissionError.value = null
  if (currentStep.value === 1 && !isStep1Valid.value) return
  if (currentStep.value === 2 && !isStep2Valid.value) return
  if (currentStep.value === 3 && !isStep3Valid.value) return
  if (currentStep.value === 4 && !isStep4Valid.value) return

  if (currentStep.value < totalSteps) {
    currentStep.value++
  }
}

function prevStep() {
  submissionError.value = null
  if (currentStep.value > 1) {
    currentStep.value--
  }
}

async function handleLogoUpload(e: Event) {
  const target = e.target as HTMLInputElement
  if (target.files && target.files[0]) {
    const file = target.files[0]
    isUploadingLogo.value = true
    try {
      const url = await brandingStore.uploadLogoFile(file)
      form.value.store_logo_url = url
    } catch (err: any) {
      alert('Gagal mengupload logo: ' + err.message)
    } finally {
      isUploadingLogo.value = false
    }
  }
}

function removeLogo() {
  form.value.store_logo_url = null
  if (logoInputRef.value) logoInputRef.value.value = ''
}

function generateRandomPairing() {
  const chars = 'ABCDEFGHJKLMNPQRSTUVWXYZ23456789'
  let code = 'NOTANE-'
  for (let i = 0; i < 4; i++) {
    code += chars.charAt(Math.floor(Math.random() * chars.length))
  }
  form.value.pairing_code = code
}

async function handleSubmitSetup() {
  isSubmitting.value = true
  submissionError.value = null

  try {
    await setupStore.executeInitialSetup({
      store_name: form.value.store_name.trim(),
      store_tagline: form.value.store_tagline.trim() || null,
      store_logo_url: form.value.store_logo_url,
      store_address: form.value.store_address.trim(),
      store_phone: form.value.store_phone.trim(),
      pairing_code: form.value.pairing_code.trim().toUpperCase(),
      main_outlet_name: form.value.main_outlet_name.trim(),
      main_outlet_address: form.value.store_address.trim(),
      main_outlet_phone: form.value.store_phone.trim(),
      include_dummy_products: form.value.include_dummy_products,
      admin_full_name: form.value.admin_full_name.trim() || null,
      admin_password: form.value.admin_password.trim() || null,
      supervisor_pin: form.value.supervisor_pin.trim() || null,
      cash_rounding_mode: form.value.cash_rounding_mode,
    })

    // Sync to store branding local state
    brandingStore.storeName = form.value.store_name.trim()
    brandingStore.storeTagline = form.value.store_tagline.trim()
    brandingStore.storeLogoUrl = form.value.store_logo_url

    // Sync pairing code in network store and localStorage immediately
    const newPairing = form.value.pairing_code.trim().toUpperCase()
    networkStore.hostStatus.pairing_code = newPairing
    networkStore.pairingCode = newPairing
    localStorage.setItem('notane_pairing_code', newPairing)
    localStorage.setItem('notane_node_role', 'SERVER')
    localStorage.setItem('notane_device_type', 'HOST')
    localStorage.setItem('notane_device_id', 'HOST-SERVER')

    isSuccessCelebration.value = true
    emit('completed')
  } catch (err: any) {
    submissionError.value = err.message || 'Terjadi kesalahan saat menyimpan pengaturan.'
  } finally {
    isSubmitting.value = false
  }
}

function finishWizard() {
  localStorage.setItem('notane_initial_setup_prompted', 'true')
  emit('update:modelValue', false)
}

function handleDismiss() {
  localStorage.setItem('notane_initial_setup_prompted', 'true')
  emit('update:modelValue', false)
}

async function handleSkipWithDefaults() {
  initFormData()
  await handleSubmitSetup()
}
</script>

<template>
  <div
    v-if="modelValue"
    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-950/80 backdrop-blur-md select-none transition-all duration-300 animate-in fade-in"
  >
    <div
      class="bg-white rounded-3xl shadow-2xl border border-slate-200 w-full max-w-3xl overflow-hidden flex flex-col max-h-[92vh] relative"
    >
      <!-- Wizard Modal Header -->
      <div class="px-8 pt-7 pb-5 border-b border-slate-100 bg-slate-50/50 flex items-center justify-between">
        <div class="flex items-center gap-3.5">
          <div
            class="w-12 h-12 rounded-2xl bg-gradient-to-tr from-indigo-600 to-violet-500 text-white flex items-center justify-center shadow-md shadow-indigo-500/20"
          >
            <Sparkles class="w-6 h-6 animate-pulse" />
          </div>
          <div>
            <div class="flex items-center gap-2">
              <h2 class="text-xl font-black text-slate-900 tracking-tight">Setup Awal Toko & Server</h2>
              <span
                class="px-2.5 py-0.5 rounded-full text-[10px] font-black uppercase tracking-wider bg-indigo-100 text-indigo-700 font-mono"
              >
                Initial Wizard
              </span>
            </div>
            <p class="text-xs text-slate-500 font-medium mt-0.5">
              Panduan konfigurasi wajib untuk memulai sistem Notane Retail POS
            </p>
          </div>
        </div>

        <button
          type="button"
          @click="handleDismiss"
          class="p-2 text-slate-400 hover:text-slate-700 hover:bg-slate-100 rounded-xl transition-colors cursor-pointer"
          title="Tutup Wizard"
        >
          <X class="w-5 h-5" />
        </button>
      </div>

      <!-- Stepper Indicator -->
      <div v-if="!isSuccessCelebration" class="px-8 py-3.5 bg-slate-50 border-b border-slate-100">
        <div class="flex items-center justify-between">
          <!-- Step 1 -->
          <div class="flex items-center gap-2">
            <div
              :class="[
                'w-7 h-7 rounded-full text-xs font-black flex items-center justify-center transition-all',
                currentStep === 1
                  ? 'bg-indigo-600 text-white shadow-xs ring-4 ring-indigo-100'
                  : currentStep > 1
                  ? 'bg-emerald-500 text-white'
                  : 'bg-slate-200 text-slate-500',
              ]"
            >
              <Check v-if="currentStep > 1" class="w-4 h-4 stroke-[3]" />
              <span v-else>1</span>
            </div>
            <span
              :class="[
                'text-xs font-bold hidden sm:inline',
                currentStep === 1 ? 'text-indigo-600' : currentStep > 1 ? 'text-slate-800' : 'text-slate-400',
              ]"
            >
              Branding
            </span>
          </div>

          <div class="flex-1 h-0.5 mx-2 bg-slate-200 overflow-hidden">
            <div
              class="h-full bg-emerald-500 transition-all duration-300"
              :style="{ width: currentStep > 1 ? '100%' : '0%' }"
            ></div>
          </div>

          <!-- Step 2 -->
          <div class="flex items-center gap-2">
            <div
              :class="[
                'w-7 h-7 rounded-full text-xs font-black flex items-center justify-center transition-all',
                currentStep === 2
                  ? 'bg-indigo-600 text-white shadow-xs ring-4 ring-indigo-100'
                  : currentStep > 2
                  ? 'bg-emerald-500 text-white'
                  : 'bg-slate-200 text-slate-500',
              ]"
            >
              <Check v-if="currentStep > 2" class="w-4 h-4 stroke-[3]" />
              <span v-else>2</span>
            </div>
            <span
              :class="[
                'text-xs font-bold hidden sm:inline',
                currentStep === 2 ? 'text-indigo-600' : currentStep > 2 ? 'text-slate-800' : 'text-slate-400',
              ]"
            >
              Cabang
            </span>
          </div>

          <div class="flex-1 h-0.5 mx-2 bg-slate-200 overflow-hidden">
            <div
              class="h-full bg-emerald-500 transition-all duration-300"
              :style="{ width: currentStep > 2 ? '100%' : '0%' }"
            ></div>
          </div>

          <!-- Step 3 -->
          <div class="flex items-center gap-2">
            <div
              :class="[
                'w-7 h-7 rounded-full text-xs font-black flex items-center justify-center transition-all',
                currentStep === 3
                  ? 'bg-indigo-600 text-white shadow-xs ring-4 ring-indigo-100'
                  : currentStep > 3
                  ? 'bg-emerald-500 text-white'
                  : 'bg-slate-200 text-slate-500',
              ]"
            >
              <Check v-if="currentStep > 3" class="w-4 h-4 stroke-[3]" />
              <span v-else>3</span>
            </div>
            <span
              :class="[
                'text-xs font-bold hidden sm:inline',
                currentStep === 3 ? 'text-indigo-600' : currentStep > 3 ? 'text-slate-800' : 'text-slate-400',
              ]"
            >
              Data Awal
            </span>
          </div>

          <div class="flex-1 h-0.5 mx-2 bg-slate-200 overflow-hidden">
            <div
              class="h-full bg-emerald-500 transition-all duration-300"
              :style="{ width: currentStep > 3 ? '100%' : '0%' }"
            ></div>
          </div>

          <!-- Step 4 -->
          <div class="flex items-center gap-2">
            <div
              :class="[
                'w-7 h-7 rounded-full text-xs font-black flex items-center justify-center transition-all',
                currentStep === 4
                  ? 'bg-indigo-600 text-white shadow-xs ring-4 ring-indigo-100'
                  : currentStep > 4
                  ? 'bg-emerald-500 text-white'
                  : 'bg-slate-200 text-slate-500',
              ]"
            >
              <Check v-if="currentStep > 4" class="w-4 h-4 stroke-[3]" />
              <span v-else>4</span>
            </div>
            <span
              :class="[
                'text-xs font-bold hidden sm:inline',
                currentStep === 4 ? 'text-indigo-600' : currentStep > 4 ? 'text-slate-800' : 'text-slate-400',
              ]"
            >
              Akses Admin
            </span>
          </div>

          <div class="flex-1 h-0.5 mx-2 bg-slate-200 overflow-hidden">
            <div
              class="h-full bg-emerald-500 transition-all duration-300"
              :style="{ width: currentStep > 4 ? '100%' : '0%' }"
            ></div>
          </div>

          <!-- Step 5 -->
          <div class="flex items-center gap-2">
            <div
              :class="[
                'w-7 h-7 rounded-full text-xs font-black flex items-center justify-center transition-all',
                currentStep === 5
                  ? 'bg-indigo-600 text-white shadow-xs ring-4 ring-indigo-100'
                  : 'bg-slate-200 text-slate-500',
              ]"
            >
              <span>5</span>
            </div>
            <span
              :class="[
                'text-xs font-bold hidden sm:inline',
                currentStep === 5 ? 'text-indigo-600' : 'text-slate-400',
              ]"
            >
              Tinjauan
            </span>
          </div>
        </div>
      </div>

      <!-- Wizard Body / Content -->
      <div class="flex-1 overflow-y-auto p-8 space-y-6">
        <!-- Error Alert -->
        <div
          v-if="submissionError"
          class="p-4 rounded-2xl bg-rose-50 border border-rose-200 text-rose-800 text-xs flex items-start gap-3"
        >
          <AlertCircle class="w-5 h-5 text-rose-600 shrink-0 mt-0.5" />
          <div class="flex-1">
            <p class="font-bold">Gagal Menyimpan Setup Awal</p>
            <p class="mt-0.5">{{ submissionError }}</p>
          </div>
        </div>

        <!-- ================= STEP 1: BRANDING & IDENTITAS TOKO ================= -->
        <div v-if="currentStep === 1 && !isSuccessCelebration" class="space-y-6 animate-in fade-in duration-200">
          <div class="border-b border-slate-100 pb-3">
            <h3 class="text-base font-extrabold text-slate-900 flex items-center gap-2">
              <Store class="w-5 h-5 text-indigo-600" />
              1. Identitas Usaha & Logo Toko
            </h3>
            <p class="text-xs text-slate-500 mt-0.5">
              Identitas ini akan tampil pada kop struk belanja, header kasir POS, dan layar terminal.
            </p>
          </div>

          <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
            <!-- Left: Logo Uploader & Preview -->
            <div class="flex flex-col items-center justify-center p-5 rounded-2xl bg-slate-50 border border-slate-200 text-center">
              <label class="text-xs font-black text-slate-700 uppercase tracking-wider mb-3">Logo Toko</label>
              
              <div class="relative group w-32 h-32 rounded-2xl border-2 border-dashed border-slate-300 bg-white flex items-center justify-center overflow-hidden shadow-inner mb-3">
                <img
                  v-if="form.store_logo_url"
                  :src="form.store_logo_url"
                  alt="Logo Toko"
                  class="w-full h-full object-contain p-2"
                />
                <div v-else class="text-slate-400 flex flex-col items-center gap-1.5 p-3">
                  <Store class="w-8 h-8 opacity-40" />
                  <span class="text-[10px] font-bold">Tanpa Logo</span>
                </div>

                <div
                  v-if="isUploadingLogo"
                  class="absolute inset-0 bg-white/80 backdrop-blur-xs flex items-center justify-center"
                >
                  <RefreshCw class="w-6 h-6 animate-spin text-indigo-600" />
                </div>
              </div>

              <input
                ref="logoInputRef"
                type="file"
                accept="image/png,image/jpeg,image/webp,image/svg+xml"
                class="hidden"
                @change="handleLogoUpload"
              />

              <div class="flex items-center gap-2">
                <button
                  type="button"
                  @click="logoInputRef?.click()"
                  class="px-3 py-1.5 rounded-xl bg-indigo-50 hover:bg-indigo-100 text-indigo-700 text-xs font-bold border border-indigo-200 flex items-center gap-1.5 transition-colors"
                >
                  <Upload class="w-3.5 h-3.5" />
                  {{ form.store_logo_url ? 'Ganti Logo' : 'Upload Logo' }}
                </button>
                <button
                  v-if="form.store_logo_url"
                  type="button"
                  @click="removeLogo"
                  class="p-1.5 rounded-xl bg-rose-50 hover:bg-rose-100 text-rose-600 border border-rose-200 transition-colors"
                  title="Hapus Logo"
                >
                  <Trash2 class="w-3.5 h-3.5" />
                </button>
              </div>
              <p class="text-[10px] text-slate-400 mt-2">Mendukung PNG, JPG, WebP (Maks 2MB)</p>
            </div>

            <!-- Right: Store Inputs -->
            <div class="md:col-span-2 space-y-4">
              <div>
                <label class="block text-xs font-black text-slate-700 uppercase tracking-wider mb-1.5">
                  Nama Toko / Bisnis Retail <span class="text-rose-500">*</span>
                </label>
                <input
                  v-model="form.store_name"
                  type="text"
                  placeholder="Contoh: Toko Sembako Berkah, Riang Market"
                  class="w-full px-4 py-2.5 rounded-xl border border-slate-300 text-sm font-bold text-slate-900 focus:outline-hidden focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500 transition-all shadow-xs"
                />
                <p v-if="!isStep1Valid" class="text-[11px] text-rose-600 font-medium mt-1">
                  Nama toko wajib diisi (minimal 3 karakter).
                </p>
              </div>

              <div>
                <label class="block text-xs font-black text-slate-700 uppercase tracking-wider mb-1.5">
                  Slogan / Motto Toko
                </label>
                <input
                  v-model="form.store_tagline"
                  type="text"
                  placeholder="Contoh: Belanja Hemat, Lengkap & Terpercaya Setiap Hari"
                  class="w-full px-4 py-2.5 rounded-xl border border-slate-300 text-sm text-slate-800 focus:outline-hidden focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500 transition-all shadow-xs"
                />
              </div>

              <div class="p-3.5 rounded-2xl bg-indigo-50/70 border border-indigo-100 text-xs text-indigo-900 flex items-start gap-2.5">
                <Sparkles class="w-4 h-4 text-indigo-600 shrink-0 mt-0.5" />
                <p class="leading-relaxed">
                  Nama dan slogan toko akan otomatis disinkronkan ke seluruh komputer kasir (LAN Client) secara real-time.
                </p>
              </div>
            </div>
          </div>
        </div>

        <!-- ================= STEP 2: CABANG UTAMA & KONTAK ================= -->
        <div v-if="currentStep === 2 && !isSuccessCelebration" class="space-y-6 animate-in fade-in duration-200">
          <div class="border-b border-slate-100 pb-3">
            <h3 class="text-base font-extrabold text-slate-900 flex items-center gap-2">
              <Building2 class="w-5 h-5 text-indigo-600" />
              2. Cabang Utama & Kontak Lokasi Toko
            </h3>
            <p class="text-xs text-slate-500 mt-0.5">
              Notane mendukung multi-cabang. Di sini kita menentukan identitas cabang pusat (Headquarter) pertama Anda.
            </p>
          </div>

          <div class="space-y-4">
            <div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
              <div class="sm:col-span-2">
                <label class="block text-xs font-black text-slate-700 uppercase tracking-wider mb-1.5">
                  Nama Cabang Utama <span class="text-rose-500">*</span>
                </label>
                <input
                  v-model="form.main_outlet_name"
                  type="text"
                  placeholder="Contoh: Toko Pusat Sudirman, Cabang Utama"
                  class="w-full px-4 py-2.5 rounded-xl border border-slate-300 text-sm font-bold text-slate-900 focus:outline-hidden focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500 transition-all shadow-xs"
                />
              </div>

              <div>
                <label class="block text-xs font-black text-slate-700 uppercase tracking-wider mb-1.5">
                  Kode Cabang
                </label>
                <input
                  type="text"
                  value="OUT-001 (Pusat)"
                  disabled
                  class="w-full px-4 py-2.5 rounded-xl border border-slate-200 bg-slate-100 text-sm font-mono font-bold text-slate-500 cursor-not-allowed"
                />
              </div>
            </div>

            <div>
              <label class="block text-xs font-black text-slate-700 uppercase tracking-wider mb-1.5">
                Alamat Fisik Toko <span class="text-rose-500">*</span>
              </label>
              <textarea
                v-model="form.store_address"
                rows="2"
                placeholder="Contoh: Jl. Jenderal Sudirman No. 123, Kel. Gambir, Jakarta Pusat"
                class="w-full px-4 py-2.5 rounded-xl border border-slate-300 text-sm text-slate-800 focus:outline-hidden focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500 transition-all shadow-xs resize-none"
              ></textarea>
              <p class="text-[11px] text-slate-400 mt-0.5">Alamat ini akan dicetak pada bagian kop atas struk kasir.</p>
            </div>

            <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <div>
                <label class="block text-xs font-black text-slate-700 uppercase tracking-wider mb-1.5">
                  Nomor Telepon / WhatsApp Toko
                </label>
                <input
                  v-model="form.store_phone"
                  type="text"
                  placeholder="Contoh: 021-5551234 / 0812-3456-7890"
                  class="w-full px-4 py-2.5 rounded-xl border border-slate-300 text-sm text-slate-800 focus:outline-hidden focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500 transition-all shadow-xs"
                />
              </div>

              <div>
                <label class="block text-xs font-black text-slate-700 uppercase tracking-wider mb-1.5">
                  Aturan Pembulatan Kas Kasir
                </label>
                <select
                  v-model="form.cash_rounding_mode"
                  class="w-full px-4 py-2.5 rounded-xl border border-slate-300 text-sm text-slate-800 focus:outline-hidden focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500 transition-all shadow-xs bg-white"
                >
                  <option value="NONE">Tanpa Pembulatan (Sesuai Nominal)</option>
                  <option value="ROUND_100">Bulatkan ke Rp 100 terdekat</option>
                  <option value="FLOOR_500">Bulatkan ke Bawah Rp 500</option>
                  <option value="CEIL_500">Bulatkan ke Atas Rp 500</option>
                </select>
              </div>
            </div>
          </div>
        </div>

        <!-- ================= STEP 3: OPSI DATA AWAL & PRODUK DUMMY ================= -->
        <div v-if="currentStep === 3 && !isSuccessCelebration" class="space-y-6 animate-in fade-in duration-200">
          <div class="border-b border-slate-100 pb-3">
            <h3 class="text-base font-extrabold text-slate-900 flex items-center gap-2">
              <Database class="w-5 h-5 text-indigo-600" />
              3. Inisialisasi Database: Data Bersih vs Data Contoh
            </h3>
            <p class="text-xs text-slate-500 mt-0.5">
              Tentukan apakah Anda ingin memulai toko dengan database kosong bersih atau menyertakan data contoh demo.
            </p>
          </div>

          <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <!-- Option A: Clean Database (Recommended) -->
            <div
              @click="form.include_dummy_products = false"
              :class="[
                'p-5 rounded-3xl border-2 cursor-pointer transition-all flex flex-col justify-between relative overflow-hidden select-none',
                !form.include_dummy_products
                  ? 'border-emerald-500 bg-emerald-50/40 shadow-md ring-2 ring-emerald-500/20'
                  : 'border-slate-200 bg-white hover:border-slate-300 hover:bg-slate-50/50',
              ]"
            >
              <div class="space-y-3">
                <div class="flex items-center justify-between">
                  <div class="w-10 h-10 rounded-2xl bg-emerald-100 text-emerald-700 flex items-center justify-center font-bold">
                    <CheckCircle2 class="w-5 h-5" />
                  </div>
                  <span class="px-2.5 py-0.5 rounded-full text-[10px] font-black uppercase bg-emerald-100 text-emerald-800">
                    Direkomendasikan
                  </span>
                </div>

                <div>
                  <h4 class="text-sm font-black text-slate-900">Database Bersih (Siap Jualan)</h4>
                  <p class="text-xs text-slate-500 mt-1 leading-relaxed">
                    Menghapus seluruh produk dummy/contoh. Sistem akan bersih sehingga Anda siap menginput produk dan barcode asli toko Anda.
                  </p>
                </div>

                <div class="pt-3 border-t border-emerald-100/80 space-y-1.5 text-[11px] text-slate-600">
                  <div class="flex items-center gap-2 text-emerald-700 font-semibold">
                    <Check class="w-3.5 h-3.5" /> Tanpa produk dummy yang mengotori laporan
                  </div>
                  <div class="flex items-center gap-2 text-emerald-700 font-semibold">
                    <Check class="w-3.5 h-3.5" /> Kategori & Satuan umum tetap tersedia
                  </div>
                  <div class="flex items-center gap-2 text-emerald-700 font-semibold">
                    <Check class="w-3.5 h-3.5" /> Bagan Akun (COA) akuntansi siap pakai
                  </div>
                </div>
              </div>

              <div class="mt-4 flex items-center justify-between pt-2">
                <span class="text-xs font-bold text-slate-700">Pilih Mode Bersih</span>
                <div
                  :class="[
                    'w-5 h-5 rounded-full border flex items-center justify-center transition-all',
                    !form.include_dummy_products
                      ? 'border-emerald-600 bg-emerald-600 text-white'
                      : 'border-slate-300 bg-white',
                  ]"
                >
                  <Check v-if="!form.include_dummy_products" class="w-3 h-3 stroke-[3]" />
                </div>
              </div>
            </div>

            <!-- Option B: Include Dummy Data -->
            <div
              @click="form.include_dummy_products = true"
              :class="[
                'p-5 rounded-3xl border-2 cursor-pointer transition-all flex flex-col justify-between relative overflow-hidden select-none',
                form.include_dummy_products
                  ? 'border-indigo-500 bg-indigo-50/40 shadow-md ring-2 ring-indigo-500/20'
                  : 'border-slate-200 bg-white hover:border-slate-300 hover:bg-slate-50/50',
              ]"
            >
              <div class="space-y-3">
                <div class="flex items-center justify-between">
                  <div class="w-10 h-10 rounded-2xl bg-indigo-100 text-indigo-700 flex items-center justify-center font-bold">
                    <ShoppingBag class="w-5 h-5" />
                  </div>
                  <span class="px-2.5 py-0.5 rounded-full text-[10px] font-black uppercase bg-indigo-100 text-indigo-800">
                    Untuk Demo
                  </span>
                </div>

                <div>
                  <h4 class="text-sm font-black text-slate-900">Muat Data Demo / Contoh Produk</h4>
                  <p class="text-xs text-slate-500 mt-1 leading-relaxed">
                    Memuat 3 produk contoh (Indomie, Ultra Milk, Apel timbangan) beserta stok awal, harga tier, dan voucher promo untuk mencoba transaksi.
                  </p>
                </div>

                <div class="pt-3 border-t border-indigo-100/80 space-y-1.5 text-[11px] text-slate-600">
                  <div class="flex items-center gap-2 text-indigo-700 font-semibold">
                    <Check class="w-3.5 h-3.5" /> 3 Produk demo siap scan barcode
                  </div>
                  <div class="flex items-center gap-2 text-indigo-700 font-semibold">
                    <Check class="w-3.5 h-3.5" /> Saldo awal persediaan otomatis terjurnal
                  </div>
                  <div class="flex items-center gap-2 text-indigo-700 font-semibold">
                    <Check class="w-3.5 h-3.5" /> Cocok untuk pelatihan kasir baru
                  </div>
                </div>
              </div>

              <div class="mt-4 flex items-center justify-between pt-2">
                <span class="text-xs font-bold text-slate-700">Pilih Data Demo</span>
                <div
                  :class="[
                    'w-5 h-5 rounded-full border flex items-center justify-center transition-all',
                    form.include_dummy_products
                      ? 'border-indigo-600 bg-indigo-600 text-white'
                      : 'border-slate-300 bg-white',
                  ]"
                >
                  <Check v-if="form.include_dummy_products" class="w-3 h-3 stroke-[3]" />
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- ================= STEP 4: AKUN ADMINISTRATOR & KEAMANAN ================= -->
        <div v-if="currentStep === 4 && !isSuccessCelebration" class="space-y-6 animate-in fade-in duration-200">
          <div class="border-b border-slate-100 pb-3">
            <h3 class="text-base font-extrabold text-slate-900 flex items-center gap-2">
              <ShieldCheck class="w-5 h-5 text-indigo-600" />
              4. Akun Administrator & Keamanan Jaringan
            </h3>
            <p class="text-xs text-slate-500 mt-0.5">
              Atur kredensial akun pemilik toko dan kode rahasia pairing untuk menghubungkan komputer kasir.
            </p>
          </div>

          <div class="space-y-4">
            <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <div>
                <label class="block text-xs font-black text-slate-700 uppercase tracking-wider mb-1.5">
                  Nama Pemilik / Administrator
                </label>
                <input
                  v-model="form.admin_full_name"
                  type="text"
                  placeholder="Contoh: Budi Santoso"
                  class="w-full px-4 py-2.5 rounded-xl border border-slate-300 text-sm text-slate-900 focus:outline-hidden focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500 transition-all shadow-xs"
                />
              </div>

              <div>
                <label class="block text-xs font-black text-slate-700 uppercase tracking-wider mb-1.5">
                  Username Superadmin
                </label>
                <input
                  type="text"
                  value="admin"
                  disabled
                  class="w-full px-4 py-2.5 rounded-xl border border-slate-200 bg-slate-100 text-sm font-mono font-bold text-slate-500 cursor-not-allowed"
                />
              </div>
            </div>

            <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
              <div>
                <label class="block text-xs font-black text-slate-700 uppercase tracking-wider mb-1.5 flex items-center justify-between">
                  <span>Password Baru Superadmin</span>
                  <span class="text-[10px] text-slate-400 font-normal lowercase">(kosongkan jika default: admin123)</span>
                </label>
                <div class="relative">
                  <input
                    v-model="form.admin_password"
                    :type="showPassword ? 'text' : 'password'"
                    placeholder="Minimal 6 karakter"
                    class="w-full pl-4 pr-10 py-2.5 rounded-xl border border-slate-300 text-sm text-slate-900 focus:outline-hidden focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500 transition-all shadow-xs"
                  />
                  <button
                    type="button"
                    @click="showPassword = !showPassword"
                    class="absolute right-3 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-600"
                  >
                    <EyeOff v-if="showPassword" class="w-4 h-4" />
                    <Eye v-else class="w-4 h-4" />
                  </button>
                </div>
              </div>

              <div>
                <label class="block text-xs font-black text-slate-700 uppercase tracking-wider mb-1.5 flex items-center justify-between">
                  <span>PIN Supervisor Kasir</span>
                  <span class="text-[10px] text-slate-400 font-normal lowercase">(kosongkan jika default: 123456)</span>
                </label>
                <input
                  v-model="form.supervisor_pin"
                  type="password"
                  maxlength="6"
                  placeholder="6 Digit PIN (Otorisasi Kasir)"
                  class="w-full px-4 py-2.5 rounded-xl border border-slate-300 text-sm text-slate-900 focus:outline-hidden focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500 transition-all shadow-xs tracking-widest font-mono"
                />
              </div>
            </div>

            <!-- Pairing Code Box -->
            <div class="p-4 rounded-2xl bg-slate-50 border border-slate-200 space-y-2">
              <label class="block text-xs font-black text-slate-700 uppercase tracking-wider">
                Kode Pairing Kasir LAN (Client Pairing Code) <span class="text-rose-500">*</span>
              </label>
              <div class="flex items-center gap-2">
                <input
                  v-model="form.pairing_code"
                  type="text"
                  placeholder="Contoh: NOTANE-8899"
                  class="flex-1 px-4 py-2.5 rounded-xl border border-slate-300 text-sm font-mono font-black text-indigo-700 uppercase tracking-wider focus:outline-hidden focus:ring-2 focus:ring-indigo-500 focus:border-indigo-500 transition-all shadow-xs bg-white"
                />
                <button
                  type="button"
                  @click="generateRandomPairing"
                  class="px-3 py-2.5 rounded-xl bg-white hover:bg-slate-100 text-slate-700 border border-slate-200 text-xs font-bold flex items-center gap-1.5 transition-colors shadow-xs"
                >
                  <Shuffle class="w-3.5 h-3.5 text-indigo-600" />
                  Acak Kode
                </button>
              </div>
              <p class="text-[11px] text-slate-500">
                Komputer kasir akan memasukkan kode ini saat pertama kali terhubung ke Notane Server di jaringan lokal.
              </p>
            </div>
          </div>
        </div>

        <!-- ================= STEP 5: TINJAUAN & KONFIRMASI ================= -->
        <div v-if="currentStep === 5 && !isSuccessCelebration" class="space-y-6 animate-in fade-in duration-200">
          <div class="border-b border-slate-100 pb-3">
            <h3 class="text-base font-extrabold text-slate-900 flex items-center gap-2">
              <Sparkles class="w-5 h-5 text-indigo-600" />
              5. Ringkasan & Konfirmasi Peluncuran
            </h3>
            <p class="text-xs text-slate-500 mt-0.5">
              Silakan periksa kembali konfigurasi awal sebelum diterapkan ke database Notane Server.
            </p>
          </div>

          <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <!-- Review Card 1: Toko & Cabang -->
            <div class="p-4 rounded-2xl bg-slate-50 border border-slate-200 space-y-3">
              <div class="flex items-center gap-2 text-xs font-black text-slate-700 uppercase tracking-wider">
                <Store class="w-4 h-4 text-indigo-600" /> Profil Toko & Cabang
              </div>
              <div class="space-y-1.5 text-xs">
                <div class="flex justify-between py-1 border-b border-slate-200/60">
                  <span class="text-slate-500">Nama Toko:</span>
                  <span class="font-bold text-slate-900">{{ form.store_name }}</span>
                </div>
                <div class="flex justify-between py-1 border-b border-slate-200/60">
                  <span class="text-slate-500">Slogan:</span>
                  <span class="font-medium text-slate-700 truncate max-w-[180px]">{{ form.store_tagline || '-' }}</span>
                </div>
                <div class="flex justify-between py-1 border-b border-slate-200/60">
                  <span class="text-slate-500">Logo:</span>
                  <span :class="form.store_logo_url ? 'text-emerald-600 font-bold' : 'text-slate-400'">
                    {{ form.store_logo_url ? '✓ Terpasang' : 'Tanpa Logo' }}
                  </span>
                </div>
                <div class="flex justify-between py-1 border-b border-slate-200/60">
                  <span class="text-slate-500">Cabang Utama:</span>
                  <span class="font-bold text-indigo-700">{{ form.main_outlet_name }}</span>
                </div>
                <div class="flex justify-between py-1">
                  <span class="text-slate-500">No. Telepon:</span>
                  <span class="font-mono text-slate-800">{{ form.store_phone }}</span>
                </div>
              </div>
            </div>

            <!-- Review Card 2: Database & Akses -->
            <div class="p-4 rounded-2xl bg-slate-50 border border-slate-200 space-y-3">
              <div class="flex items-center gap-2 text-xs font-black text-slate-700 uppercase tracking-wider">
                <Database class="w-4 h-4 text-indigo-600" /> Database & Kredensial
              </div>
              <div class="space-y-1.5 text-xs">
                <div class="flex justify-between py-1 border-b border-slate-200/60">
                  <span class="text-slate-500">Mode Database:</span>
                  <span
                    :class="[
                      'px-2 py-0.5 rounded-full text-[10px] font-bold uppercase',
                      !form.include_dummy_products
                        ? 'bg-emerald-100 text-emerald-800'
                        : 'bg-indigo-100 text-indigo-800',
                    ]"
                  >
                    {{ !form.include_dummy_products ? 'Bersih (Siap Jualan)' : 'Sertakan Data Demo' }}
                  </span>
                </div>
                <div class="flex justify-between py-1 border-b border-slate-200/60">
                  <span class="text-slate-500">Kode Pairing Kasir:</span>
                  <span class="font-mono font-black text-indigo-700">{{ form.pairing_code }}</span>
                </div>
                <div class="flex justify-between py-1 border-b border-slate-200/60">
                  <span class="text-slate-500">Admin Utama:</span>
                  <span class="font-bold text-slate-900">{{ form.admin_full_name }}</span>
                </div>
                <div class="flex justify-between py-1 border-b border-slate-200/60">
                  <span class="text-slate-500">Password Admin:</span>
                  <span class="font-medium text-slate-700">
                    {{ form.admin_password ? 'Password Baru Diterapkan' : 'Default (admin123)' }}
                  </span>
                </div>
                <div class="flex justify-between py-1">
                  <span class="text-slate-500">PIN Supervisor:</span>
                  <span class="font-medium text-slate-700">
                    {{ form.supervisor_pin ? 'PIN Baru Diterapkan' : 'Default (123456)' }}
                  </span>
                </div>
              </div>
            </div>
          </div>

          <div class="p-4 rounded-2xl bg-emerald-50 border border-emerald-200 text-xs text-emerald-900 flex items-start gap-3">
            <CheckCircle2 class="w-5 h-5 text-emerald-600 shrink-0 mt-0.5" />
            <div>
              <p class="font-bold">Semua Data Siap Diterapkan!</p>
              <p class="mt-0.5 text-emerald-800 leading-relaxed">
                Klik tombol <strong>"Terapkan & Luncurkan Notane Server"</strong> di bawah untuk mengeksekusi konfigurasi ke database.
              </p>
            </div>
          </div>
        </div>

        <!-- ================= CELEBRATION / SUCCESS SCREEN ================= -->
        <div v-if="isSuccessCelebration" class="py-8 text-center space-y-6 animate-in zoom-in-95 duration-300">
          <div
            class="w-20 h-20 rounded-full bg-emerald-100 text-emerald-600 flex items-center justify-center mx-auto shadow-lg shadow-emerald-500/20"
          >
            <CheckCircle2 class="w-10 h-10" />
          </div>

          <div class="space-y-1.5">
            <h3 class="text-2xl font-black text-slate-900 tracking-tight">Selamat! Toko Anda Siap Beroperasi</h3>
            <p class="text-sm text-slate-500 max-w-md mx-auto">
              Konfigurasi <strong>{{ form.store_name }}</strong> berhasil disimpan ke database. Notane Server sekarang aktif melayani terminal kasir.
            </p>
          </div>

          <div class="max-w-md mx-auto p-4 rounded-2xl bg-slate-50 border border-slate-200 text-left space-y-2 text-xs">
            <div class="flex justify-between">
              <span class="text-slate-500">Toko:</span>
              <span class="font-bold text-slate-900">{{ form.store_name }}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-slate-500">Cabang:</span>
              <span class="font-bold text-indigo-700">{{ form.main_outlet_name }}</span>
            </div>
            <div class="flex justify-between">
              <span class="text-slate-500">Kode Pairing:</span>
              <span class="font-mono font-black text-indigo-600">{{ form.pairing_code }}</span>
            </div>
          </div>

          <div class="flex items-center justify-center gap-3 pt-4">
            <button
              type="button"
              @click="finishWizard"
              class="px-6 py-3 rounded-2xl bg-indigo-600 hover:bg-indigo-700 text-white font-extrabold text-sm shadow-md shadow-indigo-600/20 transition-all flex items-center gap-2"
            >
              Buka Host Manager
              <ArrowRight class="w-4 h-4" />
            </button>
          </div>
        </div>
      </div>

      <!-- Wizard Modal Footer -->
      <div
        v-if="!isSuccessCelebration"
        class="px-8 py-4 border-t border-slate-100 bg-slate-50/60 flex items-center justify-between"
      >
        <button
          v-if="currentStep > 1"
          type="button"
          @click="prevStep"
          class="px-4 py-2.5 rounded-xl border border-slate-300 bg-white hover:bg-slate-100 text-slate-700 text-xs font-bold transition-colors flex items-center gap-1.5 shadow-xs cursor-pointer"
        >
          <ArrowLeft class="w-4 h-4" />
          Sebelumnya
        </button>
        <button
          v-else
          type="button"
          @click="handleDismiss"
          class="px-4 py-2.5 rounded-xl border border-slate-200 bg-white hover:bg-slate-100 text-slate-500 text-xs font-semibold transition-colors flex items-center gap-1.5 shadow-xs cursor-pointer"
        >
          Nanti Saja
        </button>

        <div class="flex items-center gap-3">
          <button
            v-if="currentStep === 1 && isFirstRun"
            type="button"
            @click="handleSkipWithDefaults"
            :disabled="isSubmitting"
            class="px-4 py-2.5 rounded-xl border border-indigo-200 bg-indigo-50 hover:bg-indigo-100 text-indigo-700 text-xs font-bold transition-all flex items-center gap-1.5 cursor-pointer shadow-xs"
            title="Gunakan pengaturan toko bawaan tanpa mengisi wizard"
          >
            <Sparkles class="w-3.5 h-3.5 text-indigo-600" />
            Lewati (Pakai Bawaan)
          </button>

          <button
            v-if="currentStep < totalSteps"
            type="button"
            @click="nextStep"
            class="px-5 py-2.5 rounded-xl bg-indigo-600 hover:bg-indigo-700 text-white text-xs font-extrabold transition-all shadow-md shadow-indigo-600/20 flex items-center gap-1.5 cursor-pointer"
          >
            Lanjutkan
            <ArrowRight class="w-4 h-4" />
          </button>

          <button
            v-else
            type="button"
            @click="handleSubmitSetup"
            :disabled="isSubmitting"
            class="px-6 py-2.5 rounded-xl bg-emerald-600 hover:bg-emerald-700 disabled:opacity-50 text-white text-xs font-black transition-all shadow-md shadow-emerald-600/20 flex items-center gap-2"
          >
            <RefreshCw v-if="isSubmitting" class="w-4 h-4 animate-spin" />
            <Sparkles v-else class="w-4 h-4" />
            {{ isSubmitting ? 'Menerapkan Konfigurasi...' : 'Terapkan & Luncurkan Notane Server' }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
