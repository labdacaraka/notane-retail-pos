import { defineStore } from 'pinia'
import { ref } from 'vue'
import * as XLSX from 'xlsx'
import { api, getApiBaseUrl } from '@/services/api'
import { useDialogStore } from './dialogStore'

export interface ExportJob {
  id: string
  title: string
  module: string
  filename: string
  status: 'QUEUED' | 'PROCESSING' | 'COMPLETED' | 'FAILED'
  progress: number
  recordCount: number
  createdAt: Date
  completedAt?: Date
  errorMessage?: string
  blobUrl?: string
  downloadUrl?: string
  rawWorkbook?: XLSX.WorkBook
}

export interface QueueExportOptions {
  title: string
  module: 'sales' | 'purchases' | 'expenses' | 'products'
  filenamePrefix: string
  fetchData: () => Promise<any[]>
  transformToRows: (data: any[]) => { sheetName: string; rows: any[]; headers?: string[] }[]
  onSuccessMessage?: string
}

export interface ServerExportOptions {
  module: string
  title: string
  filterParams: Record<string, any>
  outletId?: string
  onSuccessMessage?: string
}

export const useExportStore = defineStore('export', () => {
  const dialog = useDialogStore()
  const jobs = ref<ExportJob[]>([])
  const isWidgetOpen = ref<boolean>(false)

  function playSuccessChime() {
    try {
      const AudioContext = window.AudioContext || (window as any).webkitAudioContext
      if (!AudioContext) return
      const ctx = new AudioContext()
      const osc = ctx.createOscillator()
      const gain = ctx.createGain()
      osc.type = 'sine'
      osc.frequency.setValueAtTime(587.33, ctx.currentTime) // D5
      osc.frequency.exponentialRampToValueAtTime(880, ctx.currentTime + 0.15) // A5
      gain.gain.setValueAtTime(0.12, ctx.currentTime)
      gain.gain.exponentialRampToValueAtTime(0.01, ctx.currentTime + 0.3)
      osc.connect(gain)
      gain.connect(ctx.destination)
      osc.start()
      osc.stop(ctx.currentTime + 0.35)
    } catch {
      // Best-effort audio notification
    }
  }

  function triggerDownloadUrl(url: string, filename: string) {
    const link = document.createElement('a')
    link.href = url
    link.download = filename
    document.body.appendChild(link)
    link.click()
    document.body.removeChild(link)
  }

  function updateJob(jobId: string, updates: Partial<ExportJob>) {
    const job = jobs.value.find((j) => j.id === jobId)
    if (job) {
      Object.assign(job, updates)
    }
  }

  // =========================================================================
  // SERVER-SIDE BACKGROUND EXPORT QUEUE (SKALA BESAR / JUTAAN DATA)
  // =========================================================================
  async function queueServerExport(opts: ServerExportOptions): Promise<string> {
    try {
      const filterParams = { ...opts.filterParams }
      if (opts.outletId && !filterParams.outlet_id) {
        filterParams.outlet_id = opts.outletId
      }

      // 1. Kirim permintaan ekspor ke antrean server
      const res = await api.post('/exports', {
        module: opts.module,
        title: opts.title,
        filter_params: filterParams,
      })

      if (!res.data?.success || !res.data?.data) {
        throw new Error(res.data?.message || 'Gagal menjadwalkan ekspor di server.')
      }

      const serverJob = res.data.data
      const localJob: ExportJob = {
        id: serverJob.id,
        title: serverJob.title,
        module: serverJob.module,
        filename: serverJob.filename,
        status: 'PROCESSING',
        progress: 15,
        recordCount: 0,
        createdAt: new Date(),
        downloadUrl: `${getApiBaseUrl()}/exports/${serverJob.id}/download`,
      }

      jobs.value.unshift(localJob)

      dialog.showToast(
        `Ekspor "${opts.title}" sedang diproses di server latar belakang...`,
        'info',
        4000
      )

      // 2. State & Cleanup Controllers
      let hasCompleted = false
      let tickerTimer: any = null
      let pollTimer: any = null
      let wsListener: any = null

      const completeJob = (totalRows: number) => {
        if (hasCompleted) return
        hasCompleted = true
        if (tickerTimer) clearInterval(tickerTimer)
        if (pollTimer) clearInterval(pollTimer)
        if (wsListener && typeof window !== 'undefined') {
          window.removeEventListener('notane-ws-message', wsListener)
        }

        // Update reaktif langsung ke proxy di dalam jobs.value
        updateJob(serverJob.id, {
          status: 'COMPLETED',
          progress: 100,
          recordCount: totalRows,
          completedAt: new Date(),
        })

        // Unduh otomatis file Excel jadi dari server
        const downloadUrl = `${getApiBaseUrl()}/exports/${serverJob.id}/download`
        triggerDownloadUrl(downloadUrl, serverJob.filename)
        playSuccessChime()

        const msg =
          opts.onSuccessMessage ||
          `Ekspor selesai! File "${serverJob.filename}" berhasil diunduh (${totalRows} data).`
        dialog.showToast(msg, 'success', 6000)
      }

      const failJob = (errorMsg: string) => {
        if (hasCompleted) return
        hasCompleted = true
        if (tickerTimer) clearInterval(tickerTimer)
        if (pollTimer) clearInterval(pollTimer)
        if (wsListener && typeof window !== 'undefined') {
          window.removeEventListener('notane-ws-message', wsListener)
        }

        updateJob(serverJob.id, {
          status: 'FAILED',
          errorMessage: errorMsg,
        })

        dialog.showToast(
          `Ekspor "${opts.title}" gagal: ${errorMsg}`,
          'danger',
          6000
        )
      }

      // Animasi visual progress aktif (bergerak dari 15% naik bertahap ke 92%)
      tickerTimer = setInterval(() => {
        if (hasCompleted) return
        const cur = jobs.value.find((j) => j.id === serverJob.id)
        if (cur && cur.status === 'PROCESSING' && cur.progress < 90) {
          const step = Math.floor(Math.random() * 6) + 4
          cur.progress = Math.min(90, cur.progress + step)
        }
      }, 350)

      // Listener WebSocket instan jika event dikirim dari server
      if (typeof window !== 'undefined') {
        wsListener = (event: any) => {
          const data = event.detail
          if (data && data.type === 'EXPORT_JOB_COMPLETED' && data.job_id === serverJob.id) {
            completeJob(data.total_rows || 0)
          }
        }
        window.addEventListener('notane-ws-message', wsListener)
      }

      // Polling setiap 750ms untuk mengecek status riil di server
      let pollAttempts = 0
      const maxAttempts = 300 // Timeout ~3.75 menit

      pollTimer = setInterval(async () => {
        if (hasCompleted) return
        pollAttempts++

        try {
          const statusRes = await api.get(`/exports/${serverJob.id}/status`)
          if (statusRes.data?.success && statusRes.data?.data) {
            const current = statusRes.data.data

            if (current.status === 'COMPLETED') {
              completeJob(current.total_rows || current.processed_rows || 0)
            } else if (current.status === 'FAILED') {
              failJob(current.error_message || 'Terjadi kesalahan saat memproses ekspor di server.')
            } else if (current.total_rows && current.total_rows > 0) {
              const realPercent = Math.round((current.processed_rows / current.total_rows) * 100)
              const cur = jobs.value.find((j) => j.id === serverJob.id)
              if (cur && cur.status === 'PROCESSING') {
                cur.progress = Math.max(cur.progress, Math.min(95, realPercent))
                cur.recordCount = current.processed_rows || 0
              }
            }
          }
        } catch (err: any) {
          console.warn('Gagal memeriksa status ekspor server:', err)
        }

        if (pollAttempts >= maxAttempts) {
          failJob('Waktu tunggu pemrosesan server habis (timeout).')
        }
      }, 750)

      return serverJob.id
    } catch (err: any) {
      console.error('Gagal membuat antrean ekspor server:', err)
      const errorMsg = err.response?.data?.message || err.message || 'Gagal memulai ekspor di server'
      dialog.showToast(`Gagal memulai ekspor: ${errorMsg}`, 'danger', 6000)
      throw err
    }
  }

  // =========================================================================
  // CLIENT-SIDE ASYNCHRONOUS EXPORT FALLBACK (UNTUK DATA RINGAN)
  // =========================================================================
  async function queueExport(opts: QueueExportOptions): Promise<string> {
    const timestamp = new Date().toISOString().replace(/[-:T]/g, '').slice(0, 14)
    const filename = `${opts.filenamePrefix}_${timestamp}.xlsx`
    const jobId = `job_${Date.now()}_${Math.random().toString(36).substring(2, 7)}`

    const job: ExportJob = {
      id: jobId,
      title: opts.title,
      module: opts.module,
      filename,
      status: 'PROCESSING',
      progress: 15,
      recordCount: 0,
      createdAt: new Date(),
    }

    jobs.value.unshift(job)

    dialog.showToast(
      `Ekspor ${opts.title} sedang diproses di belakang layar...`,
      'info',
      4000
    )

    setTimeout(async () => {
      try {
        job.progress = 35
        const rawData = await opts.fetchData()

        job.recordCount = rawData.length
        job.progress = 70

        if (rawData.length === 0) {
          job.status = 'COMPLETED'
          job.progress = 100
          job.completedAt = new Date()
          dialog.showToast(
            `Tidak ada data ditemukan untuk diekspor pada filter ini (${opts.title}).`,
            'warning',
            5000
          )
          return
        }

        const sheetsData = opts.transformToRows(rawData)
        const wb = XLSX.utils.book_new()

        for (const s of sheetsData) {
          const ws = s.headers
            ? XLSX.utils.json_to_sheet(s.rows, { header: s.headers })
            : XLSX.utils.json_to_sheet(s.rows)
          XLSX.utils.book_append_sheet(wb, ws, s.sheetName || 'Data')
        }

        job.progress = 90

        const wbout = XLSX.write(wb, { bookType: 'xlsx', type: 'array' })
        const blob = new Blob([wbout], {
          type: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
        })
        const blobUrl = URL.createObjectURL(blob)

        job.blobUrl = blobUrl
        job.rawWorkbook = wb
        job.status = 'COMPLETED'
        job.progress = 100
        job.completedAt = new Date()

        triggerDownloadUrl(blobUrl, filename)
        playSuccessChime()

        const successMsg =
          opts.onSuccessMessage ||
          `Ekspor selesai! File "${filename}" berhasil diunduh (${job.recordCount} data).`
        dialog.showToast(successMsg, 'success', 6000)
      } catch (err: any) {
        console.error('Export Error:', err)
        job.status = 'FAILED'
        job.errorMessage = err.message || 'Gagal memproses file ekspor'
        dialog.showToast(
          `Gagal mengekspor ${opts.title}: ${job.errorMessage}`,
          'danger',
          6000
        )
      }
    }, 50)

    return jobId
  }

  function redownloadJob(job: ExportJob) {
    if (job.downloadUrl) {
      triggerDownloadUrl(job.downloadUrl, job.filename)
      dialog.showToast(`Mengunduh ulang "${job.filename}" dari server...`, 'info', 3000)
    } else if (job.blobUrl) {
      triggerDownloadUrl(job.blobUrl, job.filename)
      dialog.showToast(`Mengunduh ulang "${job.filename}"...`, 'info', 3000)
    } else if (job.rawWorkbook) {
      XLSX.writeFile(job.rawWorkbook, job.filename)
      dialog.showToast(`Mengunduh ulang "${job.filename}"...`, 'info', 3000)
    }
  }

  function removeJob(jobId: string) {
    const idx = jobs.value.findIndex((j) => j.id === jobId)
    if (idx !== -1) {
      if (jobs.value[idx].blobUrl) {
        URL.revokeObjectURL(jobs.value[idx].blobUrl!)
      }
      jobs.value.splice(idx, 1)
    }
  }

  function clearCompletedJobs() {
    for (const j of jobs.value) {
      if (j.blobUrl) URL.revokeObjectURL(j.blobUrl)
    }
    jobs.value = jobs.value.filter((j) => j.status === 'PROCESSING')
  }

  return {
    jobs,
    isWidgetOpen,
    queueServerExport,
    queueExport,
    redownloadJob,
    removeJob,
    clearCompletedJobs,
  }
})
