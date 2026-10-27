import { defineStore } from 'pinia'
import axios from 'axios'
import { useNetworkStore } from './networkStore'

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
}

export interface AppVersionInfo {
  name: string
  version: string
  build_date: string
  target_os: string
  target_arch: string
  is_production: boolean
}

export interface ProviderConfig {
  current_url: string
  has_env_override: boolean
  default_local_url: string
  default_production_url: string
  is_production: boolean
}

export interface ProviderConnectionTestResult {
  success: boolean
  status: string
  target_url: string
  latency_ms?: number
  service?: string
  message: string
}

export const useLicenseStore = defineStore('license', {
  state: () => ({
    versionInfo: {
      name: 'Notane Retail Server',
      version: '1.1.0',
      build_date: '2026-09-26',
      target_os: 'unknown',
      target_arch: 'unknown',
      is_production: true,
    } as AppVersionInfo,

    licenseInfo: {
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
    } as LicenseInfo,

    providerConfig: {
      current_url: 'http://127.0.0.1:19090',
      has_env_override: false,
      default_local_url: 'http://127.0.0.1:19090',
      default_production_url: 'https://lisensi.notane.id',
      is_production: false,
    } as ProviderConfig,

    testResult: null as ProviderConnectionTestResult | null,
    updateInfo: null as UpdateInfo | null,
    isLoading: false,
    isTestingConnection: false,
    isActivatingOnline: false,
    isCheckingUpdate: false,
    isApplyingUpdate: false,
    errorMessage: '',
    successMessage: '',
  }),

  actions: {
    getCandidateUrls(): string[] {
      const urls: string[] = []

      // 1. PRIORITAS UTAMA: Jika diakses via Web Browser, gunakan origin browser saat ini!
      if (typeof window !== 'undefined' && window.location?.origin) {
        const orig = window.location.origin
        if (!orig.startsWith('tauri://') && !orig.includes('tauri.localhost') && !orig.includes(':517')) {
          urls.push(orig)
        }
      }

      const targetHost = (localStorage.getItem('notane_target_host') || '127.0.0.1').trim()
      const targetPort = localStorage.getItem('notane_target_port') || '18080'
      const controlPort = String(Number(targetPort) + 1)

      if (!urls.includes(`http://${targetHost}:${targetPort}`)) urls.push(`http://${targetHost}:${targetPort}`)
      if (!urls.includes(`http://${targetHost}:${controlPort}`)) urls.push(`http://${targetHost}:${controlPort}`)
      if (!urls.includes('http://127.0.0.1:18080')) urls.push('http://127.0.0.1:18080')
      if (!urls.includes('http://127.0.0.1:18081')) urls.push('http://127.0.0.1:18081')

      return urls
    },

    getBaseUrl(): string {
      const list = this.getCandidateUrls()
      return list[0] || 'http://127.0.0.1:18080'
    },

    async fetchVersionInfo() {
      for (const base of this.getCandidateUrls()) {
        try {
          const res = await axios.get(`${base}/api/v1/system/version`, { timeout: 3000 })
          if (res.data) {
            this.versionInfo = res.data
            return
          }
        } catch (_) {}
      }
    },

    async fetchProviderConfig(): Promise<ProviderConfig | null> {
      for (const base of this.getCandidateUrls()) {
        try {
          const res = await axios.get(`${base}/api/v1/system/provider-config`, { timeout: 3000 })
          if (res.data) {
            this.providerConfig = res.data
            return res.data
          }
        } catch (_) {}
      }
      return null
    },

    async saveProviderConfig(url: string, isCustom: boolean): Promise<boolean> {
      this.isLoading = true
      this.errorMessage = ''
      this.successMessage = ''
      let lastErr = 'Gagal menyimpan konfigurasi provider'
      for (const base of this.getCandidateUrls()) {
        try {
          const res = await axios.post(`${base}/api/v1/system/provider-config`, {
            provider_url: url.trim(),
            is_custom: isCustom,
          }, { timeout: 5000 })

          if (res.data?.status === 'success') {
            this.successMessage = res.data.message || 'Konfigurasi Provider berhasil disimpan!'
            await this.fetchProviderConfig()
            await this.fetchLicenseInfo()
            this.isLoading = false
            return true
          }
        } catch (err: any) {
          lastErr = err.response?.data?.message || err.message || lastErr
        }
      }
      this.errorMessage = lastErr
      this.isLoading = false
      return false
    },

    async testProviderConnection(url?: string): Promise<ProviderConnectionTestResult> {
      this.isTestingConnection = true
      this.testResult = null
      let lastErrResult: ProviderConnectionTestResult = {
        success: false,
        status: 'error',
        target_url: url || this.providerConfig.current_url,
        message: 'Gagal terhubung ke provider',
      }
      for (const base of this.getCandidateUrls()) {
        try {
          const res = await axios.post(`${base}/api/v1/system/provider-config/test`, {
            provider_url: url ? url.trim() : undefined,
          }, { timeout: 6000 })

          this.testResult = res.data
          this.isTestingConnection = false
          return res.data
        } catch (err: any) {
          lastErrResult = err.response?.data || {
            success: false,
            status: 'error',
            target_url: url || this.providerConfig.current_url,
            message: err.message || 'Gagal terhubung ke provider',
          }
        }
      }
      this.testResult = lastErrResult
      this.isTestingConnection = false
      return lastErrResult
    },

    async activateLicenseOnline(licenseKey: string): Promise<boolean> {
      this.isActivatingOnline = true
      this.errorMessage = ''
      this.successMessage = ''
      let lastErr = 'Gagal mengaktivasi lisensi secara online'
      for (const base of this.getCandidateUrls()) {
        try {
          const res = await axios.post(`${base}/api/v1/system/license/activate-online`, {
            license_key: licenseKey.trim().toUpperCase(),
          }, { timeout: 12000 })

          if (res.data?.status === 'success') {
            this.successMessage = res.data.message || 'Lisensi berhasil diaktivasi secara online!'
            await this.fetchLicenseInfo()
            useNetworkStore().fetchHostStatus()
            this.isActivatingOnline = false
            return true
          } else {
            lastErr = res.data?.message || 'Aktivasi online gagal'
          }
        } catch (err: any) {
          lastErr = err.response?.data?.message || err.message || lastErr
        }
      }
      this.errorMessage = lastErr
      this.isActivatingOnline = false
      return false
    },

    async fetchLicenseInfo() {
      this.isLoading = true
      let lastErr = 'Gagal memuat status lisensi dari server lokal'
      for (const base of this.getCandidateUrls()) {
        try {
          const res = await axios.get(`${base}/api/v1/system/license`, { timeout: 4000 })
          if (res.data) {
            this.licenseInfo = res.data
            this.isLoading = false
            return
          }
        } catch (err: any) {
          lastErr = err.message || lastErr
        }
      }
      this.errorMessage = lastErr
      this.isLoading = false
    },

    async activateLicense(token: string, signature: string): Promise<boolean> {
      this.isLoading = true
      this.errorMessage = ''
      this.successMessage = ''
      let lastErr = 'Gagal mengaktivasi lisensi'
      for (const base of this.getCandidateUrls()) {
        try {
          const res = await axios.post(`${base}/api/v1/system/license/activate`, {
            token: token.trim(),
            signature: signature.trim(),
          }, { timeout: 8000 })

          if (res.data?.status === 'success') {
            this.successMessage = res.data.message || 'Lisensi berhasil diaktivasi!'
            await this.fetchLicenseInfo()
            useNetworkStore().fetchHostStatus()
            this.isLoading = false
            return true
          } else {
            lastErr = res.data?.message || 'Aktivasi gagal'
          }
        } catch (err: any) {
          lastErr = err.response?.data?.message || err.message || lastErr
        }
      }
      this.errorMessage = lastErr
      this.isLoading = false
      return false
    },

    async checkForUpdates(): Promise<UpdateInfo | null> {
      this.isCheckingUpdate = true
      this.errorMessage = ''
      let lastErr = 'Gagal memeriksa pembaruan'
      for (const base of this.getCandidateUrls()) {
        try {
          const res = await axios.post(`${base}/api/v1/system/updates/check`, {}, { timeout: 7000 })
          if (res.data) {
            this.updateInfo = res.data
            this.isCheckingUpdate = false
            return res.data
          }
        } catch (err: any) {
          lastErr = err.response?.data?.message || err.message || lastErr
        }
      }
      this.errorMessage = lastErr
      this.isCheckingUpdate = false
      return null
    },

    async applyOneClickUpdate(): Promise<boolean> {
      if (!this.updateInfo?.download_url) return false
      this.isApplyingUpdate = true
      let lastErr = 'Gagal menerapkan pembaruan'
      for (const base of this.getCandidateUrls()) {
        try {
          const res = await axios.post(`${base}/api/v1/system/updates/apply`, {
            download_url: this.updateInfo.download_url,
            checksum_sha256: this.updateInfo.checksum_sha256 || '',
            signature: this.updateInfo.signature || '',
            target_version: this.updateInfo.latest_version,
          }, { timeout: 15000 })

          if (res.data?.status === 'success') {
            this.successMessage = res.data.message
            this.isApplyingUpdate = false
            return true
          }
        } catch (err: any) {
          lastErr = err.response?.data?.message || err.message || lastErr
        }
      }
      this.errorMessage = lastErr
      this.isApplyingUpdate = false
      return false
    },
  },
})
