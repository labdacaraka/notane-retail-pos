<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useExportStore } from '@/stores/exportStore'
import { useDirectorySettingsStore } from '@/stores/directorySettingsStore'
import {
  FileSpreadsheet,
  Download,
  CheckCircle2,
  AlertCircle,
  RefreshCw,
  X,
  Trash2,
  ChevronDown,
  Folder,
  FolderOpen,
  FolderCheck,
  Settings,
} from 'lucide-vue-next'

const router = useRouter()
const exportStore = useExportStore()
const dirStore = useDirectorySettingsStore()

onMounted(async () => {
  await dirStore.init()
})

const activeProcessingJobs = computed(() =>
  exportStore.jobs.filter((j) => j.status === 'PROCESSING')
)

const completedJobs = computed(() =>
  exportStore.jobs.filter((j) => j.status === 'COMPLETED')
)

const failedJobs = computed(() =>
  exportStore.jobs.filter((j) => j.status === 'FAILED')
)

const hasAnyJobs = computed(() => exportStore.jobs.length > 0)

const activeExportFolder = computed(() => {
  if (dirStore.exportDirectory) return dirStore.exportDirectory
  if (dirStore.systemPaths?.default_export_dir) return dirStore.systemPaths.default_export_dir
  if (dirStore.systemPaths?.downloads) return dirStore.systemPaths.downloads
  return 'Folder Unduhan Sistem'
})

function handleOpenFolder(targetPath?: string) {
  if (targetPath) {
    dirStore.openExportFolder()
  } else {
    dirStore.openExportFolder()
  }
}

function goToSettings() {
  exportStore.isWidgetOpen = false
  router.push('/admin/parameters?tab=storage')
}
</script>

<template>
  <Teleport to="body">
    <!-- Floating Widget Container di Pojok Kanan Bawah -->
    <div
      v-if="hasAnyJobs"
      class="fixed bottom-6 right-6 z-[9990] flex flex-col items-end gap-2 font-sans select-none"
    >
      <!-- Popover Drawer Riwayat Export -->
      <Transition
        enter-active-class="transform transition ease-out duration-200"
        enter-from-class="opacity-0 translate-y-3 scale-95"
        enter-to-class="opacity-100 translate-y-0 scale-100"
        leave-active-class="transform transition ease-in duration-150"
        leave-from-class="opacity-100 translate-y-0 scale-100"
        leave-to-class="opacity-0 translate-y-3 scale-95"
      >
        <div
          v-if="exportStore.isWidgetOpen"
          class="w-80 sm:w-96 bg-white border border-slate-300 rounded-xl shadow-2xl overflow-hidden flex flex-col max-h-[500px] animate-in fade-in"
        >
          <!-- Title Bar -->
          <div class="px-4 py-3 bg-slate-900 text-white flex items-center justify-between border-b border-slate-800">
            <div class="flex items-center gap-2">
              <div class="p-1 rounded-lg bg-emerald-500/20 text-emerald-400">
                <FileSpreadsheet class="w-4 h-4" />
              </div>
              <div class="text-xs font-bold leading-tight">
                <div>Antrean Ekspor Excel</div>
                <div class="text-[10px] text-slate-400 font-normal">
                  {{ exportStore.jobs.length }} berkas
                </div>
              </div>
            </div>

            <div class="flex items-center gap-1">
              <button
                v-if="completedJobs.length > 0 || failedJobs.length > 0"
                type="button"
                @click="exportStore.clearCompletedJobs()"
                class="text-slate-400 hover:text-white p-1 rounded-lg hover:bg-slate-800 text-[10px] flex items-center gap-1 transition cursor-pointer"
                title="Bersihkan riwayat selesai"
              >
                <Trash2 class="w-3.5 h-3.5" />
                <span>Bersihkan</span>
              </button>
              <button
                type="button"
                @click="exportStore.isWidgetOpen = false"
                class="text-slate-400 hover:text-white p-1 rounded-lg hover:bg-slate-800 transition cursor-pointer"
                title="Tutup Panel"
              >
                <ChevronDown class="w-4 h-4" />
              </button>
            </div>
          </div>

          <!-- Banner Informasi Folder Tujuan Ekspor -->
          <div class="px-3.5 py-2.5 bg-slate-50 border-b border-slate-200 text-xs flex items-center justify-between gap-2">
            <div class="flex items-center gap-2 min-w-0">
              <Folder class="w-4 h-4 text-slate-600 shrink-0" />
              <div class="min-w-0">
                <div class="text-[10px] text-slate-500 font-bold uppercase tracking-wider">
                  Folder Tujuan:
                </div>
                <div class="text-[11px] font-mono text-slate-800 font-medium truncate" :title="activeExportFolder">
                  {{ activeExportFolder }}
                </div>
              </div>
            </div>

            <div class="flex items-center gap-1 shrink-0">
              <button
                v-if="dirStore.isTauri"
                type="button"
                @click="handleOpenFolder()"
                class="px-2 py-1 rounded bg-white border border-slate-300 text-slate-700 hover:bg-slate-100 text-[10px] font-semibold flex items-center gap-1 cursor-pointer transition shadow-2xs"
                title="Buka Folder di Finder / Explorer"
              >
                <FolderOpen class="w-3.5 h-3.5 text-slate-600" />
                <span>Buka</span>
              </button>
              <button
                type="button"
                @click="goToSettings"
                class="p-1 rounded text-slate-400 hover:text-slate-700 hover:bg-slate-200 transition cursor-pointer"
                title="Atur Direktori Penyimpanan"
              >
                <Settings class="w-3.5 h-3.5" />
              </button>
            </div>
          </div>

          <!-- List of Jobs -->
          <div class="overflow-y-auto divide-y divide-slate-100 p-2 space-y-1">
            <div
              v-for="job in exportStore.jobs"
              :key="job.id"
              class="p-3 rounded-xl border transition-all"
              :class="{
                'bg-emerald-50/60 border-emerald-200': job.status === 'COMPLETED',
                'bg-amber-50/60 border-amber-200': job.status === 'PROCESSING',
                'bg-rose-50/60 border-rose-200': job.status === 'FAILED',
              }"
            >
              <div class="flex items-start justify-between gap-2">
                <div class="flex items-start gap-2 min-w-0">
                  <div class="mt-0.5 shrink-0">
                    <RefreshCw
                      v-if="job.status === 'PROCESSING'"
                      class="w-4 h-4 text-amber-600 animate-spin"
                    />
                    <CheckCircle2
                      v-else-if="job.status === 'COMPLETED'"
                      class="w-4 h-4 text-emerald-600"
                    />
                    <AlertCircle v-else class="w-4 h-4 text-rose-600" />
                  </div>
                  <div class="min-w-0 flex-1">
                    <div class="text-xs font-bold text-slate-800 truncate">
                      {{ job.title }}
                    </div>
                    <div class="text-[10px] text-slate-500 font-mono truncate">
                      {{ job.filename }}
                    </div>
                    <div class="text-[10px] text-slate-400 mt-0.5">
                      <span v-if="job.status === 'PROCESSING'" class="text-amber-700 font-medium">
                        Memproses data ({{ job.progress }}%)...
                      </span>
                      <span v-else-if="job.status === 'COMPLETED'" class="text-emerald-700 font-medium">
                        Selesai • {{ job.recordCount }} baris data
                      </span>
                      <span v-else class="text-rose-700 font-medium truncate">
                        Gagal: {{ job.errorMessage }}
                      </span>
                    </div>
                  </div>
                </div>

                <div class="flex items-center gap-1 shrink-0">
                  <button
                    v-if="job.status === 'COMPLETED'"
                    type="button"
                    @click="exportStore.redownloadJob(job)"
                    class="p-1.5 rounded-lg bg-white border border-emerald-300 text-emerald-700 hover:bg-emerald-100 transition shadow-2xs cursor-pointer"
                    title="Unduh Ulang File Excel"
                  >
                    <Download class="w-3.5 h-3.5" />
                  </button>
                  <button
                    type="button"
                    @click="exportStore.removeJob(job.id)"
                    class="p-1.5 rounded-lg text-slate-400 hover:text-rose-600 hover:bg-white transition cursor-pointer"
                    title="Hapus dari riwayat"
                  >
                    <X class="w-3.5 h-3.5" />
                  </button>
                </div>
              </div>

              <!-- Mini Progress Bar jika sedang proses -->
              <div
                v-if="job.status === 'PROCESSING'"
                class="w-full bg-amber-200/80 rounded-full h-1 mt-2.5 overflow-hidden"
              >
                <div
                  class="bg-amber-600 h-1 rounded-full transition-all duration-300"
                  :style="{ width: `${job.progress}%` }"
                ></div>
              </div>

              <!-- Banner Info Lokasi Berkas Selesai -->
              <div
                v-if="job.status === 'COMPLETED'"
                class="mt-2.5 pt-2 border-t border-emerald-200/60 flex items-center justify-between gap-2 text-[10px]"
              >
                <div class="flex items-center gap-1.5 text-emerald-800 min-w-0">
                  <FolderCheck class="w-3.5 h-3.5 text-emerald-600 shrink-0" />
                  <span class="truncate font-mono" :title="job.savedFolderPath || activeExportFolder">
                    {{ job.savedFolderPath || activeExportFolder }}
                  </span>
                </div>
                <button
                  v-if="dirStore.isTauri"
                  type="button"
                  @click="handleOpenFolder(job.savedFolderPath)"
                  class="px-2 py-0.5 rounded bg-white border border-emerald-300 text-emerald-700 hover:bg-emerald-100 font-bold shrink-0 flex items-center gap-1 transition cursor-pointer shadow-2xs"
                >
                  <FolderOpen class="w-3 h-3" />
                  <span>Buka Folder</span>
                </button>
              </div>
            </div>
          </div>
        </div>
      </Transition>

      <!-- Floating Trigger Button / Pill -->
      <button
        type="button"
        @click="exportStore.isWidgetOpen = !exportStore.isWidgetOpen"
        class="flex items-center gap-2 px-3.5 py-2 rounded-xl shadow-lg border transition cursor-pointer"
        :class="[
          activeProcessingJobs.length > 0
            ? 'bg-amber-600 hover:bg-amber-700 text-white border-amber-700'
            : 'bg-slate-900 hover:bg-slate-800 text-white border-slate-700',
        ]"
      >
        <div class="relative">
          <RefreshCw
            v-if="activeProcessingJobs.length > 0"
            class="w-4 h-4 animate-spin"
          />
          <FileSpreadsheet v-else class="w-4 h-4 text-emerald-400" />

          <!-- Badge Counter -->
          <span
            v-if="completedJobs.length > 0 && activeProcessingJobs.length === 0"
            class="absolute -top-1.5 -right-1.5 w-3.5 h-3.5 bg-emerald-500 text-white text-[9px] font-black rounded-full flex items-center justify-center border border-slate-900"
          >
            {{ completedJobs.length }}
          </span>
        </div>

        <div class="text-xs font-bold">
          <span v-if="activeProcessingJobs.length === 1">
            Mengekspor ({{ activeProcessingJobs[0].progress }}%)...
          </span>
          <span v-else-if="activeProcessingJobs.length > 1">
            Mengekspor ({{ activeProcessingJobs.length }} file)...
          </span>
          <span v-else>
            Ekspor Excel ({{ exportStore.jobs.length }})
          </span>
        </div>

        <ChevronDown
          class="w-3.5 h-3.5 text-white/70 transition-transform"
          :class="{ 'rotate-180': exportStore.isWidgetOpen }"
        />
      </button>
    </div>
  </Teleport>
</template>
