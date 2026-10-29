<script setup lang="ts">
import { ref, onMounted } from 'vue'
import {
  X,
  Database,
  Download,
  Upload,
  CheckCircle2,
  XCircle,
  FileArchive,
  RefreshCw,
  ShieldAlert,
  Folder,
  FolderOpen,
  RotateCcw,
} from 'lucide-vue-next'
import { useBackupStore } from '@/stores/backupStore'
import { useDirectorySettingsStore } from '@/stores/directorySettingsStore'
import { useDialog } from '@/composables/useDialog'

defineProps<{
  modelValue: boolean
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
  (e: 'restore-success'): void
}>()

const backupStore = useBackupStore()
const dirStore = useDirectorySettingsStore()
const dialog = useDialog()

onMounted(async () => {
  await dirStore.init()
})

const activeTab = ref<'EXPORT' | 'RESTORE'>('EXPORT')
const exportNotes = ref('')
const selectedFile = ref<File | null>(null)
const confirmChecked = ref(false)
const fileInputRef = ref<HTMLInputElement | null>(null)

function closeModal() {
  if (backupStore.isExporting || backupStore.isRestoring) return
  emit('update:modelValue', false)
  backupStore.clearState()
  selectedFile.value = null
  confirmChecked.value = false
  exportNotes.value = ''
}

async function handleExport() {
  try {
    await backupStore.exportBackupZip(exportNotes.value.trim() || undefined)
  } catch (err: any) {
    // Error is set in store
  }
}

async function handleFileSelected(e: Event) {
  const target = e.target as HTMLInputElement
  if (!target.files || target.files.length === 0) return

  const file = target.files[0]
  if (!file.name.toLowerCase().endsWith('.zip')) {
    dialog.alert('Harap pilih file dengan ekstensi .zip hasil ekspor cadangan Notane Server.', 'warning')
    if (fileInputRef.value) fileInputRef.value.value = ''
    return
  }

  selectedFile.value = file
  confirmChecked.value = false

  try {
    await backupStore.inspectBackupFile(file)
  } catch (err: any) {
    // Error is set in store
  }
}

function resetSelectedFile() {
  selectedFile.value = null
  confirmChecked.value = false
  backupStore.clearState()
  if (fileInputRef.value) fileInputRef.value.value = ''
}

async function handleRestore() {
  if (!selectedFile.value) return
  if (!confirmChecked.value) {
    dialog.alert('Harap centang kotak persetujuan konfirmasi risiko sebelum memulihkan database.', 'warning')
    return
  }

  const ok = await dialog.confirm({
    title: 'Konfirmasi Pemulihan Database',
    message: 'PERINGATAN: Proses pemulihan database akan mengganti seluruh data operasional toko saat ini. Seluruh transaksi kasir yang belum tercadangkan akan hilang. Apakah Anda benar-benar yakin ingin melanjutkan pemulihan data ini?',
    confirmText: 'Ya, Pulihkan Sekarang',
    cancelText: 'Batal',
    type: 'danger',
  })
  if (!ok) return

  try {
    await backupStore.restoreDatabase(selectedFile.value)
    emit('restore-success')
  } catch (err: any) {
    // Error is set in store
  }
}

function formatDate(iso: string) {
  try {
    return new Date(iso).toLocaleString('id-ID', {
      dateStyle: 'medium',
      timeStyle: 'medium',
    })
  } catch {
    return iso
  }
}
</script>

<template>
  <div
    v-if="modelValue"
    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/60 backdrop-blur-xs transition-opacity animate-fadeIn"
  >
    <div
      class="bg-white rounded-3xl border border-slate-200 shadow-2xl max-w-2xl w-full overflow-hidden flex flex-col max-h-[92vh] animate-scaleUp"
    >
      <!-- Modal Header -->
      <div class="px-6 py-5 border-b border-slate-100 flex items-center justify-between bg-slate-50/60">
        <div class="flex items-center gap-3">
          <div class="w-10 h-10 rounded-2xl bg-indigo-100 border border-indigo-200 text-indigo-700 flex items-center justify-center shrink-0">
            <Database class="w-5 h-5" />
          </div>
          <div>
            <h3 class="text-base font-black text-slate-900 tracking-tight flex items-center gap-2">
              <span>Cadangan & Pemulihan Database</span>
              <span class="text-[10px] font-mono px-2 py-0.5 rounded-full bg-indigo-100 text-indigo-800 font-bold uppercase">
                SQL Backup (.zip)
              </span>
            </h3>
            <p class="text-xs text-slate-500 mt-0.5">
              Ekspor seluruh data SQL terenkapsulasi zip atau pulihkan ke komputer server ini.
            </p>
          </div>
        </div>

        <button
          type="button"
          @click="closeModal"
          :disabled="backupStore.isExporting || backupStore.isRestoring"
          class="p-2 text-slate-400 hover:text-slate-700 hover:bg-slate-100 rounded-xl transition cursor-pointer disabled:opacity-40"
        >
          <X class="w-5 h-5" />
        </button>
      </div>

      <!-- Tab Switcher -->
      <div class="px-6 pt-4 border-b border-slate-100 flex items-center gap-3 bg-white shrink-0">
        <button
          type="button"
          @click="activeTab = 'EXPORT'"
          class="pb-3 text-xs font-black transition relative flex items-center gap-2 cursor-pointer"
          :class="activeTab === 'EXPORT' ? 'text-indigo-600' : 'text-slate-400 hover:text-slate-700'"
        >
          <Download class="w-4 h-4" />
          <span>Ekspor Cadangan SQL</span>
          <span
            v-if="activeTab === 'EXPORT'"
            class="absolute bottom-0 left-0 right-0 h-0.5 bg-indigo-600 rounded-full"
          ></span>
        </button>

        <button
          type="button"
          @click="activeTab = 'RESTORE'"
          class="pb-3 text-xs font-black transition relative flex items-center gap-2 cursor-pointer"
          :class="activeTab === 'RESTORE' ? 'text-indigo-600' : 'text-slate-400 hover:text-slate-700'"
        >
          <Upload class="w-4 h-4" />
          <span>Impor & Pulihkan Data</span>
          <span
            v-if="activeTab === 'RESTORE'"
            class="absolute bottom-0 left-0 right-0 h-0.5 bg-indigo-600 rounded-full"
          ></span>
        </button>
      </div>

      <!-- Modal Body (Scrollable) -->
      <div class="p-6 overflow-y-auto space-y-5 flex-1">
        <!-- GLOBAL ALERTS -->
        <div
          v-if="backupStore.errorMessage"
          class="p-4 rounded-2xl bg-rose-50 border border-rose-200 text-rose-800 text-xs font-semibold flex items-start gap-3 animate-fadeIn"
        >
          <XCircle class="w-5 h-5 text-rose-600 shrink-0 mt-0.5" />
          <div class="space-y-1">
            <span class="font-bold">Terjadi Kesalahan:</span>
            <p class="leading-relaxed">{{ backupStore.errorMessage }}</p>
          </div>
        </div>

        <div
          v-if="backupStore.successMessage"
          class="p-4 rounded-2xl bg-emerald-50 border border-emerald-200 text-emerald-800 text-xs font-semibold flex items-start justify-between gap-3 animate-fadeIn"
        >
          <div class="flex items-start gap-3">
            <CheckCircle2 class="w-5 h-5 text-emerald-600 shrink-0 mt-0.5" />
            <div class="space-y-1">
              <span class="font-bold">Berhasil:</span>
              <p class="leading-relaxed">{{ backupStore.successMessage }}</p>
            </div>
          </div>
          <button
            v-if="dirStore.isTauri"
            type="button"
            @click="dirStore.openBackupFolder()"
            class="px-3 py-1.5 rounded-xl bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-xs shrink-0 flex items-center gap-1.5 shadow-xs cursor-pointer transition"
          >
            <FolderOpen class="w-4 h-4" />
            <span>Buka Folder</span>
          </button>
        </div>

        <!-- TAB 1: EKSPOR CADANGAN -->
        <div v-if="activeTab === 'EXPORT'" class="space-y-4">
          <div class="bg-indigo-50/50 border border-indigo-100 rounded-2xl p-4 space-y-3">
            <div class="flex items-center gap-2.5 text-indigo-900 font-bold text-xs">
              <FileArchive class="w-4 h-4 text-indigo-600" />
              <span>Format Paket Ekspor Cadangan (.zip)</span>
            </div>
            <p class="text-xs text-indigo-900/80 leading-relaxed">
              Sistem akan membuat berkas arsip <strong>ZIP</strong> berisi file <code>backup.sql</code> lengkap dan <code>manifest.json</code>. File manifest menyertakan informasi versi aplikasi dan nomor skema database untuk menjamin kompatibilitas saat dipulihkan di komputer server lain.
            </p>
            <div class="grid grid-cols-2 sm:grid-cols-3 gap-2 pt-1 text-[11px]">
              <div class="bg-white/80 p-2.5 rounded-xl border border-indigo-100">
                <span class="text-slate-400 block text-[10px]">Cakupan Data</span>
                <span class="font-bold text-slate-800">100% Seluruh Tabel Toko</span>
              </div>
              <div class="bg-white/80 p-2.5 rounded-xl border border-indigo-100">
                <span class="text-slate-400 block text-[10px]">Tipe Berkas</span>
                <span class="font-bold text-slate-800">ZIP (SQL + Manifest)</span>
              </div>
              <div class="bg-white/80 p-2.5 rounded-xl border border-indigo-100">
                <span class="text-slate-400 block text-[10px]">Integritas Data</span>
                <span class="font-bold text-slate-800">SHA-256 Checksum</span>
              </div>
            </div>
          </div>

          <!-- Pengaturan Folder Penyimpanan Cadangan -->
          <div class="bg-slate-50 border border-slate-200 rounded-2xl p-4 space-y-2.5">
            <div class="flex items-center justify-between">
              <label class="block text-xs font-bold text-slate-700 flex items-center gap-1.5">
                <Folder class="w-4 h-4 text-indigo-600" />
                <span>Direktori Tujuan Penyimpanan Cadangan:</span>
              </label>
              <button
                type="button"
                @click="dirStore.resetBackupDirectory()"
                class="text-[11px] text-slate-500 hover:text-slate-800 font-medium flex items-center gap-1 cursor-pointer transition"
              >
                <RotateCcw class="w-3 h-3" />
                <span>Reset</span>
              </button>
            </div>
            <div class="flex items-center gap-2">
              <input
                type="text"
                :value="dirStore.backupDirectory"
                @input="(e: any) => dirStore.setBackupDirectory(e.target.value)"
                placeholder="Direktori penyimpanan..."
                class="flex-1 bg-white border border-slate-300 rounded-xl px-3 py-2 text-xs font-mono text-slate-800 focus:outline-none focus:border-indigo-600"
              />
              <button
                v-if="dirStore.isTauri"
                type="button"
                @click="dirStore.chooseBackupDirectory()"
                class="px-3 py-2 rounded-xl bg-slate-800 hover:bg-slate-900 text-white text-xs font-bold transition flex items-center gap-1.5 shadow-xs cursor-pointer shrink-0"
              >
                <FolderOpen class="w-3.5 h-3.5" />
                <span>Pilih...</span>
              </button>
              <button
                v-if="dirStore.isTauri"
                type="button"
                @click="dirStore.openBackupFolder()"
                class="px-3 py-2 rounded-xl bg-white border border-slate-300 hover:bg-slate-100 text-slate-700 text-xs font-bold transition flex items-center gap-1.5 shadow-2xs cursor-pointer shrink-0"
                title="Buka Folder di File Manager"
              >
                <FolderOpen class="w-3.5 h-3.5 text-indigo-600" />
                <span>Buka</span>
              </button>
            </div>
          </div>

          <div>
            <label class="block text-xs font-bold text-slate-700 mb-1.5">
              Catatan / Deskripsi Cadangan (Opsional)
            </label>
            <textarea
              v-model="exportNotes"
              rows="2"
              placeholder="Contoh: Cadangan data bulanan sebelum tutup buku periode Oktober 2026..."
              class="w-full bg-slate-50 border border-slate-300 rounded-2xl p-3.5 text-xs text-slate-800 focus:bg-white focus:outline-none focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-600 transition"
            ></textarea>
          </div>

          <div class="pt-2">
            <button
              type="button"
              @click="handleExport"
              :disabled="backupStore.isExporting"
              class="w-full py-3 px-4 bg-indigo-600 hover:bg-indigo-700 text-white font-bold text-xs rounded-2xl flex items-center justify-center gap-2 transition active:scale-98 disabled:opacity-50 cursor-pointer shadow-md"
            >
              <RefreshCw v-if="backupStore.isExporting" class="w-4 h-4 animate-spin" />
              <Download v-else class="w-4 h-4" />
              <span>{{ backupStore.isExporting ? 'Memproses Ekspor Database...' : 'Unduh Berkas Cadangan Database (.zip)' }}</span>
            </button>
          </div>
        </div>

        <!-- TAB 2: IMPOR & PULIHKAN -->
        <div v-if="activeTab === 'RESTORE'" class="space-y-4">
          <!-- Danger warning box -->
          <div class="bg-amber-50 border border-amber-200 rounded-2xl p-4 flex items-start gap-3">
            <ShieldAlert class="w-5 h-5 text-amber-600 shrink-0 mt-0.5" />
            <div class="text-xs text-amber-900 leading-relaxed">
              <span class="font-bold">Perhatian Penting:</span>
              <p class="mt-0.5">
                Memulihkan database akan <strong>menghapus dan mengganti seluruh data operasional</strong> di server ini dengan data dari berkas cadangan. Pastikan Anda telah mengunduh cadangan terbaru terlebih dahulu sebelum melakukan impor.
              </p>
            </div>
          </div>

          <!-- File Picker Zone -->
          <div v-if="!selectedFile" class="border-2 border-dashed border-slate-300 hover:border-indigo-400 rounded-3xl p-6 text-center bg-slate-50/50 hover:bg-indigo-50/20 transition cursor-pointer" @click="fileInputRef?.click()">
            <input
              ref="fileInputRef"
              type="file"
              accept=".zip"
              class="hidden"
              @change="handleFileSelected"
            />
            <div class="w-12 h-12 rounded-2xl bg-indigo-50 text-indigo-600 border border-indigo-200 flex items-center justify-center mx-auto mb-3">
              <Upload class="w-6 h-6" />
            </div>
            <h4 class="text-xs font-bold text-slate-800">Pilih Berkas Cadangan (.zip)</h4>
            <p class="text-[11px] text-slate-500 mt-1 max-w-sm mx-auto">
              Klik untuk memilih berkas cadangan zip yang diekspor dari Notane Server.
            </p>
          </div>

          <!-- File Inspect Preview Card -->
          <div v-else class="space-y-4">
            <div class="bg-white border border-slate-200 rounded-2xl p-4 shadow-2xs space-y-3">
              <div class="flex items-center justify-between pb-3 border-b border-slate-100">
                <div class="flex items-center gap-2.5">
                  <FileArchive class="w-5 h-5 text-indigo-600 shrink-0" />
                  <div>
                    <span class="text-xs font-bold text-slate-900 block truncate max-w-xs sm:max-w-md">
                      {{ selectedFile.name }}
                    </span>
                    <span class="text-[10px] text-slate-400">
                      Ukuran: {{ (selectedFile.size / 1024).toFixed(1) }} KB
                    </span>
                  </div>
                </div>

                <button
                  type="button"
                  @click="resetSelectedFile"
                  :disabled="backupStore.isRestoring"
                  class="text-xs text-rose-600 hover:text-rose-800 font-bold px-2 py-1 rounded-lg hover:bg-rose-50 transition cursor-pointer"
                >
                  Ganti File
                </button>
              </div>

              <!-- Loading Inspection -->
              <div v-if="backupStore.isInspecting" class="py-4 text-center text-xs text-slate-500 flex items-center justify-center gap-2">
                <RefreshCw class="w-4 h-4 animate-spin text-indigo-600" />
                <span>Memeriksa kompatibilitas versi berkas cadangan...</span>
              </div>

              <!-- Inspection Result -->
              <div v-else-if="backupStore.inspectionResult" class="space-y-3">
                <!-- Compatibility Badge -->
                <div
                  class="p-3 rounded-xl border flex items-center justify-between"
                  :class="backupStore.inspectionResult.is_compatible
                    ? 'bg-emerald-50 border-emerald-200 text-emerald-900'
                    : 'bg-rose-50 border-rose-200 text-rose-900'"
                >
                  <div class="flex items-center gap-2 text-xs font-bold">
                    <CheckCircle2 v-if="backupStore.inspectionResult.is_compatible" class="w-4 h-4 text-emerald-600" />
                    <XCircle v-else class="w-4 h-4 text-rose-600" />
                    <span>
                      {{ backupStore.inspectionResult.is_compatible
                        ? 'Berkas Cadangan Kompatibel'
                        : 'Tidak Kompatibel (Spesifikasi Tidak Terpenuhi)' }}
                    </span>
                  </div>

                  <span class="text-[10px] font-mono px-2 py-0.5 rounded-full font-extrabold uppercase border"
                    :class="backupStore.inspectionResult.is_compatible
                      ? 'bg-emerald-100 text-emerald-800 border-emerald-300'
                      : 'bg-rose-100 text-rose-800 border-rose-300'"
                  >
                    {{ backupStore.inspectionResult.is_compatible ? 'VALID' : 'REJECTED' }}
                  </span>
                </div>

                <!-- Error explanation if incompatible -->
                <div v-if="!backupStore.inspectionResult.is_compatible" class="bg-rose-100/60 border border-rose-200 p-3 rounded-xl text-xs text-rose-900 leading-relaxed font-medium">
                  <span class="font-bold block mb-0.5">Alasan Penolakan:</span>
                  {{ backupStore.inspectionResult.error_message }}
                </div>

                <!-- Warning notice if any -->
                <div v-if="backupStore.inspectionResult.warning_message" class="bg-amber-50 border border-amber-200 p-3 rounded-xl text-xs text-amber-900 leading-relaxed">
                  <span class="font-bold block mb-0.5">Catatan Versi:</span>
                  {{ backupStore.inspectionResult.warning_message }}
                </div>

                <!-- Manifest details grid -->
                <div class="grid grid-cols-2 gap-2 text-xs">
                  <div class="bg-slate-50 p-2.5 rounded-xl border border-slate-200">
                    <span class="text-[10px] text-slate-400 block font-semibold">Versi Server Pembuat</span>
                    <span class="font-bold text-slate-800 font-mono">v{{ backupStore.inspectionResult.backup_server_version }}</span>
                  </div>
                  <div class="bg-slate-50 p-2.5 rounded-xl border border-slate-200">
                    <span class="text-[10px] text-slate-400 block font-semibold">Versi Server Ini</span>
                    <span class="font-bold text-slate-800 font-mono">v{{ backupStore.inspectionResult.current_server_version }}</span>
                  </div>
                  <div class="bg-slate-50 p-2.5 rounded-xl border border-slate-200">
                    <span class="text-[10px] text-slate-400 block font-semibold">Skema Database</span>
                    <span class="font-bold text-slate-800">v{{ backupStore.inspectionResult.backup_schema_version }}</span>
                  </div>
                  <div class="bg-slate-50 p-2.5 rounded-xl border border-slate-200">
                    <span class="text-[10px] text-slate-400 block font-semibold">Waktu Ekspor</span>
                    <span class="font-bold text-slate-800 text-[11px] truncate block">
                      {{ formatDate(backupStore.inspectionResult.manifest.exported_at) }}
                    </span>
                  </div>
                </div>

                <!-- Totals & Notes -->
                <div class="bg-slate-50 p-3 rounded-xl border border-slate-200 text-xs space-y-1">
                  <div class="flex items-center justify-between text-slate-700">
                    <span>Jumlah Tabel:</span>
                    <span class="font-bold">{{ backupStore.inspectionResult.manifest.total_tables }} Tabel</span>
                  </div>
                  <div class="flex items-center justify-between text-slate-700">
                    <span>Total Baris Data:</span>
                    <span class="font-bold">{{ backupStore.inspectionResult.manifest.total_records.toLocaleString('id-ID') }} Rekor</span>
                  </div>
                  <div v-if="backupStore.inspectionResult.manifest.notes" class="pt-1 text-[11px] text-slate-500 italic border-t border-slate-200 mt-1">
                    "{{ backupStore.inspectionResult.manifest.notes }}"
                  </div>
                </div>

                <!-- Checkbox Confirmation (Only enabled if compatible) -->
                <div v-if="backupStore.inspectionResult.is_compatible" class="pt-2">
                  <label class="flex items-start gap-2.5 text-xs text-slate-800 font-medium cursor-pointer select-none">
                    <input
                      v-model="confirmChecked"
                      type="checkbox"
                      class="mt-0.5 rounded text-indigo-600 focus:ring-indigo-500 w-4 h-4"
                    />
                    <span>
                      Saya telah memverifikasi data di atas dan memahami bahwa proses pemulihan akan menimpa seluruh database server saat ini.
                    </span>
                  </label>
                </div>
              </div>
            </div>

            <!-- Restore Action Button -->
            <div>
              <button
                type="button"
                @click="handleRestore"
                :disabled="
                  backupStore.isRestoring ||
                  !backupStore.inspectionResult ||
                  !backupStore.inspectionResult.is_compatible ||
                  !confirmChecked
                "
                class="w-full py-3 px-4 bg-rose-600 hover:bg-rose-700 disabled:bg-slate-200 disabled:text-slate-400 text-white font-bold text-xs rounded-2xl flex items-center justify-center gap-2 transition active:scale-98 cursor-pointer shadow-md"
              >
                <RefreshCw v-if="backupStore.isRestoring" class="w-4 h-4 animate-spin" />
                <Database v-else class="w-4 h-4" />
                <span>
                  {{
                    backupStore.isRestoring
                      ? 'Sedang Memulihkan Database (Mohon Tunggu)...'
                      : 'Mulai Pemulihan Database Toko'
                  }}
                </span>
              </button>
            </div>
          </div>
        </div>
      </div>

      <!-- Modal Footer -->
      <div class="px-6 py-4 border-t border-slate-100 bg-slate-50/60 flex items-center justify-between text-xs text-slate-500">
        <span>Notane Retail POS Server</span>
        <button
          type="button"
          @click="closeModal"
          :disabled="backupStore.isExporting || backupStore.isRestoring"
          class="px-4 py-2 rounded-xl border border-slate-200 hover:bg-slate-100 text-slate-700 font-bold transition cursor-pointer"
        >
          Tutup
        </button>
      </div>
    </div>
  </div>
</template>
