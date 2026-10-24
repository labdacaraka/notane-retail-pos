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

    updateInfo: null as UpdateInfo | null,
    isLoading: false,
    isCheckingUpdate: false,
    isApplyingUpdate: false,
    errorMessage: '',
    successMessage: '',
  }),

  actions: {
    getBaseUrl(): string {
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
