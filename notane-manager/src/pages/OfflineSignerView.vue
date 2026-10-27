<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { useProviderStore, type ManualSignResult } from '../stores/providerStore'
import {
  FileSignature,
  KeyRound,
  Cpu,
  Copy,
  Check,
  ShieldCheck,
  AlertCircle,
  Sparkles,
} from 'lucide-vue-next'

const route = useRoute()
const providerStore = useProviderStore()

const licenseKey = ref('')
const hardwareId = ref('')
const isGenerating = ref(false)
const signResult = ref<ManualSignResult | null>(null)
const copiedToken = ref(false)
const copiedSignature = ref(false)
const validationError = ref('')

onMounted(async () => {
  await providerStore.fetchLicenses()
  if (route.query.key) {
    licenseKey.value = String(route.query.key)
  } else if (providerStore.licenses.length > 0) {
    licenseKey.value = providerStore.licenses[0].license_key
  }
})

async function handleGenerate() {
  validationError.value = ''
  if (!licenseKey.value.trim()) {
    validationError.value = 'Nomor lisensi wajib diisi'
    return
  }
  if (!hardwareId.value.trim()) {
    validationError.value = 'Hardware ID mesin server (notane-server) wajib diisi'
    return
  }

  isGenerating.value = true
  const res = await providerStore.signTokenManual(licenseKey.value.trim(), hardwareId.value.trim())
  isGenerating.value = false

  if (res) {
    signResult.value = res
  }
}

async function copyToken() {
  if (!signResult.value) return
  await navigator.clipboard.writeText(signResult.value.license_token_json)
  copiedToken.value = true
  setTimeout(() => (copiedToken.value = false), 2000)
}

async function copySignature() {
  if (!signResult.value) return
  await navigator.clipboard.writeText(signResult.value.signature_hex)
  copiedSignature.value = true
  setTimeout(() => (copiedSignature.value = false), 2000)
}
</script>

<template>
  <div class="max-w-4xl space-y-6">
    <!-- Intro Card -->
    <div class="p-6 rounded-2xl glass-panel border border-brand-500/20 bg-gradient-to-r from-slate-900 to-brand-950/20">
      <div class="flex items-start gap-4">
        <div class="p-3 rounded-xl bg-brand-500/10 text-brand-400 border border-brand-500/20 shrink-0">
          <FileSignature class="w-6 h-6" />
        </div>
        <div>
          <h2 class="text-base font-bold text-white flex items-center gap-2">
            Aktivasi Lisensi Manual
            <Sparkles class="w-4 h-4 text-brand-400" />
          </h2>
          <p class="text-xs text-slate-300 mt-1 leading-relaxed">
            Digunakan untuk komputer toko yang tidak terhubung ke internet. Masukkan nomor lisensi dan kode perangkat untuk menghasilkan kode aktivasi toko.
          </p>
        </div>
      </div>
    </div>

    <!-- Generator Input Form -->
    <div class="p-6 rounded-2xl glass-panel border border-slate-800 space-y-5">
      <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
        <!-- License Key Selection / Input -->
        <div>
          <label class="block text-xs font-semibold text-slate-300 mb-1 flex items-center gap-1.5">
            <KeyRound class="w-3.5 h-3.5 text-brand-400" />
            <span>Nomor Lisensi *</span>
          </label>
          <div class="space-y-2">
            <input
              v-model="licenseKey"
              type="text"
              placeholder="Contoh: NTN-ENT-2026-X899"
              class="w-full px-3 py-2.5 bg-slate-950 border border-slate-800 rounded-xl text-xs font-mono text-white focus:outline-none focus:border-brand-500"
            />
            <div v-if="providerStore.licenses.length > 0" class="flex flex-wrap gap-1.5 text-[11px]">
              <span class="text-slate-500">Pilih cepat:</span>
              <button
                v-for="lic in providerStore.licenses.slice(0, 3)"
                :key="lic.id"
                type="button"
                @click="licenseKey = lic.license_key"
                class="px-2 py-0.5 rounded bg-slate-800 hover:bg-slate-700 text-slate-300 font-mono transition-colors"
              >
                {{ lic.license_key }}
              </button>
            </div>
          </div>
        </div>

        <!-- Hardware ID (HWID) Input -->
        <div>
          <label class="block text-xs font-semibold text-slate-300 mb-1 flex items-center gap-1.5">
            <Cpu class="w-3.5 h-3.5 text-blue-400" />
            <span>Hardware ID Mesin Server (notane-server) *</span>
          </label>
          <input
            v-model="hardwareId"
            type="text"
            placeholder="Contoh: NOTANE-HWID-A1B2-C3D4-E5F6-7890"
            class="w-full px-3 py-2.5 bg-slate-950 border border-slate-800 rounded-xl text-xs font-mono text-white focus:outline-none focus:border-brand-500"
          />
          <span class="text-[11px] text-slate-500 mt-1 block">
            Salin Hardware ID dari menu Lisensi pada aplikasi Notane Server toko (Bukan komputer kasir client).
          </span>
        </div>
      </div>

      <div v-if="validationError" class="p-3 rounded-xl bg-rose-500/10 border border-rose-500/30 text-rose-400 text-xs flex items-center gap-2">
        <AlertCircle class="w-4 h-4 shrink-0" />
        <span>{{ validationError }}</span>
      </div>

      <div class="pt-2 flex justify-end">
        <button
          @click="handleGenerate"
          :disabled="isGenerating"
          class="px-5 py-2.5 bg-brand-600 hover:bg-brand-500 disabled:opacity-50 text-white rounded-xl text-xs font-bold transition-all shadow-lg shadow-brand-600/30 flex items-center gap-2"
        >
          <ShieldCheck class="w-4 h-4" />
          <span>{{ isGenerating ? 'Membuat Kode...' : 'Buat Kode Aktivasi Toko' }}</span>
        </button>
      </div>
    </div>

    <!-- Sign Result Output -->
    <div v-if="signResult" class="p-6 rounded-2xl glass-panel border border-brand-500/30 space-y-5 animate-in fade-in duration-200">
      <div class="flex items-center justify-between pb-3 border-b border-slate-800">
        <div class="flex items-center gap-2">
          <div class="p-1.5 rounded-lg bg-emerald-500/20 text-emerald-400">
            <ShieldCheck class="w-4 h-4" />
          </div>
          <span class="font-bold text-sm text-white">Kode Aktivasi Berhasil Dibuat!</span>
        </div>
        <span class="text-[11px] font-mono text-brand-400 bg-brand-500/10 px-2 py-0.5 rounded">
          Tanda Tangan Terverifikasi
        </span>
      </div>

      <!-- Output 1: License Token JSON -->
      <div>
        <div class="flex items-center justify-between mb-1.5">
          <label class="text-xs font-semibold text-slate-300">1. Kode Lisensi Toko</label>
          <button
            @click="copyToken"
            class="px-2.5 py-1 rounded-lg bg-slate-800 hover:bg-slate-700 text-brand-400 text-xs font-semibold flex items-center gap-1 transition-colors"
          >
            <component :is="copiedToken ? Check : Copy" class="w-3.5 h-3.5" />
            <span>{{ copiedToken ? 'Tersalin!' : 'Salin Kode' }}</span>
          </button>
        </div>
        <textarea
          readonly
          :value="signResult.license_token_json"
          rows="3"
          class="w-full p-3 bg-slate-950 border border-slate-800 rounded-xl font-mono text-xs text-slate-300 select-all focus:outline-none"
        ></textarea>
      </div>

      <!-- Output 2: Cryptographic Signature Hex -->
      <div>
        <div class="flex items-center justify-between mb-1.5">
          <label class="text-xs font-semibold text-slate-300">2. Kode Tanda Tangan Lisensi</label>
          <button
            @click="copySignature"
            class="px-2.5 py-1 rounded-lg bg-slate-800 hover:bg-slate-700 text-brand-400 text-xs font-semibold flex items-center gap-1 transition-colors"
          >
            <component :is="copiedSignature ? Check : Copy" class="w-3.5 h-3.5" />
            <span>{{ copiedSignature ? 'Tersalin!' : 'Salin Tanda Tangan' }}</span>
          </button>
        </div>
        <input
          readonly
          :value="signResult.signature_hex"
          class="w-full p-3 bg-slate-950 border border-slate-800 rounded-xl font-mono text-xs text-brand-300 select-all focus:outline-none"
        />
      </div>

      <!-- Instruction Note -->
      <div class="p-3.5 rounded-xl bg-slate-900 border border-slate-800 text-xs text-slate-400 space-y-1">
        <div class="font-semibold text-slate-200">Cara Aktivasi di Komputer Toko:</div>
        <ol class="list-decimal list-inside space-y-0.5 text-slate-400">
          <li>Buka aplikasi Notane Server atau Notane POS, lalu buka menu <strong>Lisensi</strong>.</li>
          <li>Pilih tab <strong>Aktivasi Manual</strong>, lalu tempelkan <strong>Kode Lisensi Toko</strong> dan <strong>Kode Tanda Tangan Lisensi</strong> di atas.</li>
          <li>Klik <strong>Simpan Lisensi Toko</strong>. Aplikasi akan memverifikasi keamanan secara otomatis tanpa perlu internet.</li>
        </ol>
      </div>
    </div>
  </div>
</template>
