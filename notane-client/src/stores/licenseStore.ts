import { defineStore } from 'pinia'
import axios from 'axios'
import { invoke, isTauri } from '@tauri-apps/api/core'
import { useNetworkStore } from './networkStore'
import { api } from '@/services/api'

export const FEATURE_ALIASES: Record<string, string[]> = {
  label_printing: ['label_printing', 'price_labels', 'print_labels', 'label_printer', 'barcode_labels', 'label'],
  membership: ['membership', 'customers', 'loyalty', 'customer_loyalty', 'member'],
  promotions: ['promotions', 'promotions_engine', 'vouchers', 'flash_sales', 'discounts', 'promo'],
  expenses: ['expenses', 'petty_cash', 'expense', 'operational_expenses'],
  accounting: ['accounting', 'ledger', 'accounting_ledger', 'general_ledger'],
  audit_log: ['audit_log', 'audit_logs', 'audit_trail', 'security_logs', 'audit'],
  multi_outlet: ['multi_outlet', 'outlets', 'branch_management'],
  inventory_batch: ['inventory_batch', 'batch_tracking', 'fefo'],
  remove_branding: ['remove_branding', 'white_label', 'custom_branding'],
}

declare const __APP_VERSION__: string | undefined
const defaultAppVersion = (import.meta as any).env?.VITE_APP_VERSION || (typeof __APP_VERSION__ !== 'undefined' ? __APP_VERSION__ : '1.0.0')

export type LicenseStatus = 'ACTIVE' | 'GRACE_PERIOD' | 'EXPIRED' | 'HARDWARE_MISMATCH' | 'CLOCK_TAMPERED' | 'UNLICENSED'

export interface LicenseInfo {
  status: LicenseStatus
  is_valid: boolean
  license_key: string | null
  client_name: string | null
  tier: string | null
  max_terminals: number
  current_hardware_id: string
  licensed_hardware_id: string | null
  expires_at: string | null
  days_remaining: number | null
  grace_days_remaining: number | null
  features: string[]
  message: string
  provider_server_url: string
  last_sync_at?: string | null
  sync_status?: string | null
  consecutive_sync_failures?: number | null
  offline_grace_days_remaining?: number | null
}

export interface LicenseSyncLogItem {
  id: number
  sync_type: string
  status: string
  http_status: number | null
  latency_ms: number | null
  provider_url: string
  message: string | null
  created_at: string
}

export interface UpdateInfo {
  current_version: string
  has_update: boolean
  latest_version: string
  release_name: string | null
  changelog: string | null
  release_date: string | null
  download_url: string | null
  checksum_sha256: string | null
  signature: string | null
  is_mandatory: boolean
  license_status: LicenseStatus
  can_update: boolean
  message: string
  component?: string
  min_server_version?: string | null
  server_compatible?: boolean
}

export interface AppVersionInfo {
  name: string
  version: string
  build_date: string
  target_os: string
  target_arch: string
  is_production: boolean
}

const LICENSE_CACHE_KEY = 'notane_cached_license_info'

function getCachedLicenseInfo(): LicenseInfo {
  try {
    const raw = localStorage.getItem(LICENSE_CACHE_KEY)
    if (raw) {
      const parsed = JSON.parse(raw)
      if (parsed && typeof parsed === 'object') {
        return {
          status: parsed.status || 'UNLICENSED',
          is_valid: Boolean(parsed.is_valid),
          license_key: parsed.license_key || null,
          client_name: parsed.client_name || null,
          tier: parsed.tier || null,
          max_terminals: Number(parsed.max_terminals) || 1,
          current_hardware_id: parsed.current_hardware_id || 'MEMUAT...',
          licensed_hardware_id: parsed.licensed_hardware_id || null,
          expires_at: parsed.expires_at || null,
          days_remaining: parsed.days_remaining ?? null,
          grace_days_remaining: parsed.grace_days_remaining ?? null,
          features: Array.isArray(parsed.features) ? parsed.features : [],
          message: parsed.message || 'Memuat status lisensi...',
          provider_server_url: parsed.provider_server_url || 'https://lisensi.notane.id',
          last_sync_at: parsed.last_sync_at || null,
          sync_status: parsed.sync_status || null,
          consecutive_sync_failures: parsed.consecutive_sync_failures ?? null,
          offline_grace_days_remaining: parsed.offline_grace_days_remaining ?? null,
        }
      }
    }
  } catch (err) {
    console.warn('[LicenseStore] Gagal membaca cache lisensi lokal:', err)
  }
  return {
    status: 'UNLICENSED',
    is_valid: false,
    license_key: null,
    client_name: null,
    tier: null,
    max_terminals: 1,
    current_hardware_id: 'MEMUAT...',
    licensed_hardware_id: null,
    expires_at: null,
    days_remaining: null,
    grace_days_remaining: null,
    features: [],
    message: 'Memuat status lisensi...',
    provider_server_url: 'https://lisensi.notane.id',
  }
}

function persistLicenseCache(info: LicenseInfo) {
  try {
    localStorage.setItem(LICENSE_CACHE_KEY, JSON.stringify(info))
  } catch (err) {
    console.warn('[LicenseStore] Gagal menyimpan cache lisensi lokal:', err)
  }
}

export const useLicenseStore = defineStore('license', {
  state: () => ({
    clientVersionInfo: {
      name: 'Notane Retail Client (Terminal POS & UI)',
      version: defaultAppVersion,
      build_date: '2026-10-25',
      type: 'UI_ONLY',
    },

    versionInfo: {
      name: 'Notane Retail POS Server',
      version: defaultAppVersion,
      build_date: '2026-09-26',
      target_os: 'unknown',
      target_arch: 'unknown',
      is_production: true,
    } as AppVersionInfo,

    licenseInfo: getCachedLicenseInfo(),

    updateInfo: null as UpdateInfo | null,
    syncLogs: [] as LicenseSyncLogItem[],
    isLoading: false,
    isSyncing: false,
    isCheckingUpdate: false,
    isApplyingUpdate: false,
    errorMessage: '',
    successMessage: '',
  }),

  actions: {
    hasFeature(featureId: string): boolean {
      if (!this.licenseInfo?.is_valid) {
        return false
      }

      // 1. Lisensi tier ENTERPRISE atau ULTIMATE otomatis membuka semua fitur modul backoffice
      const tier = (this.licenseInfo.tier || '').trim().toUpperCase()
      if (tier === 'ENTERPRISE' || tier === 'ULTIMATE' || tier === 'ALL') {
        return true
      }

      const feats = this.licenseInfo.features || []
      if (feats.some((f) => f === '*' || f.toLowerCase() === 'all')) {
        return true
      }

      const req = featureId.trim().toLowerCase()
      // 2. Cocokkan nama eksak
      if (feats.some((f) => f.trim().toLowerCase() === req)) {
        return true
      }

      // 3. Cocokkan variasi alias fitur
      const aliases = FEATURE_ALIASES[req] || []
      return feats.some((f) => {
        const featName = f.trim().toLowerCase()
        return aliases.includes(featName)
      })
    },

    getBaseUrl(): string {
      const netStore = useNetworkStore()
      return netStore.serverUrl
    },

    async initClientVersion() {
      if (isTauri()) {
        try {
          const v = await invoke<string>('get_client_version')
          if (v && typeof v === 'string' && v.trim().length > 0) {
            this.clientVersionInfo.version = v.trim()
          }
        } catch (_) {}
      }
    },

    async fetchVersionInfo() {
      await this.initClientVersion()
      try {
        const res = await api.get('/system/version', { timeout: 3500 })
        if (res.data) {
          this.versionInfo = res.data
        }
      } catch (_) {
        try {
          const res = await axios.get(`${this.getBaseUrl()}/api/v1/system/version`, { timeout: 3000 })
          if (res.data) {
            this.versionInfo = res.data
          }
        } catch (_) {}
      }
    },

    async fetchLicenseInfo() {
      this.isLoading = true
      try {
        // 1. Coba via API Client terstandarisasi (sudah dilengkapi headers & baseUrl)
        let data: any = null
        try {
          const res = await api.get('/system/license', { timeout: 4000 })
          data = res.data?.data ? res.data.data : res.data
        } catch (_) {
          // 2. Fallback via getBaseUrl()
          const res = await axios.get(`${this.getBaseUrl()}/api/v1/system/license`, { timeout: 4000 })
          data = res.data?.data ? res.data.data : res.data
        }

        if (data && typeof data === 'object' && ('status' in data || 'is_valid' in data)) {
          this.licenseInfo = {
            ...data,
            features: Array.isArray(data.features) ? data.features : [],
          }
          if (data.is_valid) {
            persistLicenseCache(this.licenseInfo)
          }
        }
      } catch (err: any) {
        this.errorMessage = err.message || 'Gagal memuat status lisensi dari server lokal'
      } finally {
        this.isLoading = false
      }
    },

    async activateLicense(token: string, signature: string): Promise<boolean> {
      this.isLoading = true
      this.errorMessage = ''
      this.successMessage = ''
      try {
        const res = await axios.post(`${this.getBaseUrl()}/api/v1/system/license/activate`, {
          token: token.trim(),
          signature: signature.trim(),
        }, { timeout: 8000 })

        if (res.data?.status === 'success') {
          this.successMessage = res.data.message || 'Lisensi berhasil diaktivasi!'
          await this.fetchLicenseInfo()
          return true
        } else {
          this.errorMessage = res.data?.message || 'Aktivasi gagal'
          return false
        }
      } catch (err: any) {
        this.errorMessage = err.response?.data?.message || err.message || 'Gagal mengaktivasi lisensi'
        return false
      } finally {
        this.isLoading = false
      }
    },

    async syncLicenseWithProvider(): Promise<{ success: boolean; message: string }> {
      this.isSyncing = true
      this.errorMessage = ''
      this.successMessage = ''
      try {
        const res = await axios.post(`${this.getBaseUrl()}/api/v1/system/license/sync`, {}, { timeout: 8000 })
        if (res.data?.data?.license_info) {
          this.licenseInfo = res.data.data.license_info
          persistLicenseCache(res.data.data.license_info)
        } else {
          await this.fetchLicenseInfo()
        }
        const msg = res.data?.message || 'Sinkronisasi berhasil'
        await this.fetchSyncLogs()
        if (res.data?.status === 'success') {
          this.successMessage = msg
          return { success: true, message: msg }
        } else {
          return { success: false, message: msg }
        }
      } catch (err: any) {
        const msg = err.response?.data?.message || err.message || 'Gagal menghubungi server provider lisensi'
        this.errorMessage = msg
        await this.fetchSyncLogs()
        return { success: false, message: msg }
      } finally {
        this.isSyncing = false
      }
    },

    async fetchSyncLogs(): Promise<void> {
      try {
        const res = await axios.get(`${this.getBaseUrl()}/api/v1/system/license/logs?limit=20`, { timeout: 4000 })
        if (res.data?.data) {
          this.syncLogs = res.data.data
        }
      } catch (_) {}
    },

    async checkForUpdates(component: 'client' | 'server' = 'client'): Promise<UpdateInfo | null> {
      this.isCheckingUpdate = true
      this.errorMessage = ''
      try {
        await this.initClientVersion()
        const currentVersion = component === 'client' ? this.clientVersionInfo.version : this.versionInfo.version
        const res = await axios.post(`${this.getBaseUrl()}/api/v1/system/updates/check`, {
          component,
          current_version: currentVersion,
        }, { timeout: 7000 })
        if (res.data) {
          this.updateInfo = res.data
          return res.data
        }
        return null
      } catch (err: any) {
        this.errorMessage = err.response?.data?.message || err.message || 'Gagal memeriksa pembaruan'
        return null
      } finally {
        this.isCheckingUpdate = false
      }
    },

    async applyClientUiUpdate(): Promise<{ success: boolean; message: string }> {
      if (!this.updateInfo) {
        return { success: false, message: 'Belum ada data pembaruan rilis' }
      }
      if (this.updateInfo.server_compatible === false) {
        return {
          success: false,
          message: `Pembaruan diblokir: Notane Server harus diperbarui minimal ke versi ${this.updateInfo.min_server_version || 'terbaru'} sebelum memperbarui UI Klien.`
        }
      }
      this.isApplyingUpdate = true
      try {
        await new Promise(r => setTimeout(r, 1200))
        this.clientVersionInfo.version = this.updateInfo.latest_version
        this.successMessage = `Antarmuka Notane Client berhasil diperbarui ke versi ${this.updateInfo.latest_version}`
        return { success: true, message: this.successMessage }
      } finally {
        this.isApplyingUpdate = false
      }
    },

    async applyOneClickUpdate(): Promise<boolean> {
      if (!this.updateInfo?.download_url) return false
      this.isApplyingUpdate = true
      try {
        const res = await axios.post(`${this.getBaseUrl()}/api/v1/system/updates/apply`, {
          download_url: this.updateInfo.download_url,
          checksum_sha256: this.updateInfo.checksum_sha256 || '',
          signature: this.updateInfo.signature || '',
          target_version: this.updateInfo.latest_version,
        }, { timeout: 15000 })

        if (res.data?.status === 'success') {
          this.successMessage = res.data.message
          return true
        }
        return false
      } catch (err: any) {
        this.errorMessage = err.response?.data?.message || err.message || 'Gagal menerapkan pembaruan'
        return false
      } finally {
        this.isApplyingUpdate = false
      }
    },
  },
})
