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
      name: 'Notane Retail POS Enterprise',
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
      provider_server_url: 'https://license.notaneretail.com',
    } as LicenseInfo,

    providerConfig: {
      current_url: 'http://127.0.0.1:19090',
      has_env_override: false,
      default_local_url: 'http://127.0.0.1:19090',
      default_production_url: 'https://license.notaneretail.com',
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
    getBaseUrl(): string {
      if (typeof window !== 'undefined' && window.location?.origin) {
        // Jika dibuka dari Vite dev client port 5173/5175, arahkan ke target netStore
        if (window.location.port === '5173' || window.location.port === '5175') {
          const netStore = useNetworkStore()
          return netStore.serverUrl
        }
        return window.location.origin
      }
      const netStore = useNetworkStore()
      return netStore.serverUrl
    },

    async fetchVersionInfo() {
      try {
        const res = await axios.get(`${this.getBaseUrl()}/api/v1/system/version`, { timeout: 3000 })
        if (res.data) {
          this.versionInfo = res.data
        }
      } catch (_) {}
    },

    async fetchProviderConfig(): Promise<ProviderConfig | null> {
      try {
        const res = await axios.get(`${this.getBaseUrl()}/api/v1/system/provider-config`, { timeout: 3000 })
        if (res.data) {
          this.providerConfig = res.data
          return res.data
        }
      } catch (_) {}
      return null
    },

    async saveProviderConfig(url: string, isCustom: boolean): Promise<boolean> {
      this.isLoading = true
      this.errorMessage = ''
      this.successMessage = ''
      try {
        const res = await axios.post(`${this.getBaseUrl()}/api/v1/system/provider-config`, {
          provider_url: url.trim(),
          is_custom: isCustom,
        }, { timeout: 5000 })

        if (res.data?.status === 'success') {
          this.successMessage = res.data.message || 'Konfigurasi Provider berhasil disimpan!'
          await this.fetchProviderConfig()
          await this.fetchLicenseInfo()
          return true
        }
        return false
      } catch (err: any) {
        this.errorMessage = err.response?.data?.message || err.message || 'Gagal menyimpan konfigurasi provider'
        return false
      } finally {
        this.isLoading = false
      }
    },

    async testProviderConnection(url?: string): Promise<ProviderConnectionTestResult> {
      this.isTestingConnection = true
      this.testResult = null
      try {
        const res = await axios.post(`${this.getBaseUrl()}/api/v1/system/provider-config/test`, {
          provider_url: url ? url.trim() : undefined,
        }, { timeout: 6000 })

        this.testResult = res.data
        return res.data
      } catch (err: any) {
        const errData: ProviderConnectionTestResult = err.response?.data || {
          success: false,
          status: 'error',
          target_url: url || this.providerConfig.current_url,
          message: err.message || 'Gagal terhubung ke provider',
        }
        this.testResult = errData
        return errData
      } finally {
        this.isTestingConnection = false
      }
    },

    async activateLicenseOnline(licenseKey: string): Promise<boolean> {
      this.isActivatingOnline = true
      this.errorMessage = ''
      this.successMessage = ''
      try {
        const res = await axios.post(`${this.getBaseUrl()}/api/v1/system/license/activate-online`, {
          license_key: licenseKey.trim().toUpperCase(),
        }, { timeout: 12000 })

        if (res.data?.status === 'success') {
          this.successMessage = res.data.message || 'Lisensi berhasil diaktivasi secara online!'
          await this.fetchLicenseInfo()
          useNetworkStore().fetchHostStatus()
          return true
        } else {
          this.errorMessage = res.data?.message || 'Aktivasi online gagal'
          return false
        }
      } catch (err: any) {
        this.errorMessage = err.response?.data?.message || err.message || 'Gagal mengaktivasi lisensi secara online'
        return false
      } finally {
        this.isActivatingOnline = false
      }
    },

    async fetchLicenseInfo() {
      this.isLoading = true
      try {
        const res = await axios.get(`${this.getBaseUrl()}/api/v1/system/license`, { timeout: 4000 })
        if (res.data) {
          this.licenseInfo = res.data
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
          useNetworkStore().fetchHostStatus()
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

    async checkForUpdates(): Promise<UpdateInfo | null> {
      this.isCheckingUpdate = true
      this.errorMessage = ''
      try {
        const res = await axios.post(`${this.getBaseUrl()}/api/v1/system/updates/check`, {}, { timeout: 7000 })
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
