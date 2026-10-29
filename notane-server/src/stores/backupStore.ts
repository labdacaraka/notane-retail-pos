import { defineStore } from 'pinia'
import axios from 'axios'
import { useNetworkStore } from './networkStore'
import { useDirectorySettingsStore } from './directorySettingsStore'

export interface TableBackupSummary {
  table_name: string
  record_count: number
}

export interface BackupManifest {
  app_name: string
  server_version: string
  min_server_version: string
  schema_version: number
  exported_at: string
  database_name: string
  total_tables: number
  total_records: number
  sql_file: string
  checksum_sha256: string
  os: string
  arch: string
  notes?: string
  tables_summary?: TableBackupSummary[]
}

export interface CompatibilityCheckResult {
  is_compatible: boolean
  current_server_version: string
  current_schema_version: number
  required_min_version: string
  backup_server_version: string
  backup_schema_version: number
  error_message?: string | null
  warning_message?: string | null
  manifest: BackupManifest
}

export interface RestoreSummary {
  success: boolean
  message: string
  total_tables: number
  total_records: number
  restored_at: string
  manifest: BackupManifest
}

export const useBackupStore = defineStore('backup', {
  state: () => ({
    isExporting: false,
    isInspecting: false,
    isRestoring: false,
    inspectionResult: null as CompatibilityCheckResult | null,
    restoreResult: null as RestoreSummary | null,
    errorMessage: '',
    successMessage: '',
    lastSavedPath: '',
    lastSavedFolder: '',
  }),

  actions: {
    async exportBackupZip(notes?: string): Promise<{ filename: string; filePath: string; folderPath: string }> {
      const networkStore = useNetworkStore()
      const dirStore = useDirectorySettingsStore()
      this.isExporting = true
      this.errorMessage = ''
      this.successMessage = ''

      try {
        const url = `${networkStore.controlUrl}/api/v1/system/database/backup`
        const response = await axios.post(
          url,
          {},
          {
            params: notes ? { notes } : undefined,
            responseType: 'blob',
            timeout: 120000,
          }
        )

        // Dapatkan nama file dari Content-Disposition atau buat default
        const contentDisposition = response.headers['content-disposition'] || ''
        let filename = 'notane_database_backup.zip'
        const match = contentDisposition.match(/filename="?([^"]+)"?/)
        if (match && match[1]) {
          filename = match[1]
        } else {
          const timestamp = new Date().toISOString().replace(/[-:T.]/g, '').slice(0, 14)
          filename = `notane_backup_${timestamp}.zip`
        }

        const blob = new Blob([response.data], { type: 'application/zip' })
        const saveRes = await dirStore.saveBackupFile(filename, blob)

        this.lastSavedPath = saveRes.filePath
        this.lastSavedFolder = saveRes.folderPath

        const folderMsg = saveRes.isNativeSave
          ? `Tersimpan di direktori: ${saveRes.folderPath}`
          : 'Tersimpan di folder Unduhan (Downloads)'

        this.successMessage = `Berkas cadangan '${filename}' berhasil diekspor. ${folderMsg}`

        return {
          filename,
          filePath: saveRes.filePath,
          folderPath: saveRes.folderPath,
        }
      } catch (err: any) {
        if (err.response && err.response.data instanceof Blob) {
          try {
            const text = await err.response.data.text()
            const parsed = JSON.parse(text)
            this.errorMessage = parsed.message || 'Gagal mengekspor database'
          } catch {
            this.errorMessage = 'Gagal mengekspor database'
          }
        } else {
          this.errorMessage = err.response?.data?.message || err.message || 'Gagal mengekspor database'
        }
        throw new Error(this.errorMessage)
      } finally {
        this.isExporting = false
      }
    },

    async inspectBackupFile(file: File): Promise<CompatibilityCheckResult> {
      const networkStore = useNetworkStore()
      this.isInspecting = true
      this.errorMessage = ''
      this.inspectionResult = null

      try {
        const url = `${networkStore.controlUrl}/api/v1/system/database/inspect`
        const buffer = await file.arrayBuffer()

        const response = await axios.post(url, buffer, {
          headers: {
            'Content-Type': 'application/octet-stream',
          },
          timeout: 60000,
        })

        if (response.data && response.data.success && response.data.data) {
          this.inspectionResult = response.data.data
          return response.data.data
        } else {
          throw new Error(response.data?.message || 'Gagal membaca informasi kompatibilitas berkas cadangan')
        }
      } catch (err: any) {
        const msg = err.response?.data?.message || err.message || 'Gagal memeriksa berkas cadangan'
        this.errorMessage = msg
        throw new Error(msg)
      } finally {
        this.isInspecting = false
      }
    },

    async restoreDatabase(file: File): Promise<RestoreSummary> {
      const networkStore = useNetworkStore()
      this.isRestoring = true
      this.errorMessage = ''
      this.restoreResult = null

      try {
        const url = `${networkStore.controlUrl}/api/v1/system/database/restore`
        const buffer = await file.arrayBuffer()

        const response = await axios.post(url, buffer, {
          headers: {
            'Content-Type': 'application/octet-stream',
          },
          timeout: 180000, // 3 minutes timeout for restoring
        })

        if (response.data && response.data.success && response.data.data) {
          this.restoreResult = response.data.data
          this.successMessage = response.data.message || 'Database berhasil dipulihkan'
          return response.data.data
        } else {
          throw new Error(response.data?.message || 'Gagal memulihkan database')
        }
      } catch (err: any) {
        const msg = err.response?.data?.message || err.message || 'Gagal memulihkan database'
        this.errorMessage = msg
        throw new Error(msg)
      } finally {
        this.isRestoring = false
      }
    },

    clearState() {
      this.inspectionResult = null
      this.restoreResult = null
      this.errorMessage = ''
      this.successMessage = ''
    },
  },
})
