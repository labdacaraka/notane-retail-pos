import { defineStore } from 'pinia'
import { ref } from 'vue'
import { apiClient } from '../services/api'

export interface ProviderStatus {
  is_server_running: boolean
  port: number
  external_url: string
  total_clients: number
  total_licenses: number
  active_licenses: number
  total_devices: number
  total_releases: number
  master_public_key: string
}

export interface Client {
  id: string
  client_name: string
  company_name: string
  email: string
  phone?: string
  address?: string
  created_at: string
  updated_at: string
}

export interface License {
  id: string
  license_key: string
  client_id: string
  client_name?: string
  tier: 'BASIC' | 'PRO' | 'ENTERPRISE'
  max_terminals: number
  status: 'ACTIVE' | 'SUSPENDED' | 'EXPIRED' | 'REVOKED'
  issued_at: string
  expires_at: string
  grace_period_days: number
  features: string
  notes?: string
  active_terminals?: number
  created_at: string
  updated_at: string
}

export interface DeviceActivation {
  id: string
  license_id: string
  license_key?: string
  client_name?: string
  hardware_id: string
  device_name?: string
  ip_address?: string
  os_info?: string
  app_version?: string
  activated_at: string
  last_ping_at: string
  is_active: number
}

export interface AppRelease {
  id: string
  version: string
  release_name: string
  changelog: string
  release_date: string
  download_url: string
  checksum_sha256: string
  signature_ed25519: string
  is_mandatory: number
  min_license_tier: string
  has_db_migration: number
  is_published: number
  created_at: string
}

export interface AuditLog {
  id: string
  license_key?: string
  hardware_id?: string
  action: string
  ip_address?: string
  status: string
  details?: string
  created_at: string
}

export interface ProviderSettings {
  server_port: number
  auto_start_server: boolean
  master_private_key: string
  master_public_key: string
}

export interface ManualSignResult {
  license_token_json: string
  signature_hex: string
  public_key_hex: string
}

export interface PaginationParams {
  page?: number
  page_size?: number
  sort_by?: string
  sort_order?: 'asc' | 'desc'
  search?: string
  status?: string
  tier?: string
}

export interface PaginatedResponse<T> {
  items: T[]
  page: number
  page_size: number
  total_items: number
  total_pages: number
}

export const useProviderStore = defineStore('provider', () => {
  const status = ref<ProviderStatus>({
    is_server_running: true,
    port: 19090,
    external_url: 'http://127.0.0.1:19090',
    total_clients: 0,
    total_licenses: 0,
    active_licenses: 0,
    total_devices: 0,
    total_releases: 0,
    master_public_key: '779b7d3fb80a25e9503bf7b85ffbd226ed9a4b8ff9de844872dd3a19357bb213',
  })

  // Paginated states
  const clientsPagination = ref<PaginatedResponse<Client>>({
    items: [],
    page: 1,
    page_size: 10,
    total_items: 0,
    total_pages: 1,
  })

  const licensesPagination = ref<PaginatedResponse<License>>({
    items: [],
    page: 1,
    page_size: 10,
    total_items: 0,
    total_pages: 1,
  })

  const devicesPagination = ref<PaginatedResponse<DeviceActivation>>({
    items: [],
    page: 1,
    page_size: 10,
    total_items: 0,
    total_pages: 1,
  })

  const releasesPagination = ref<PaginatedResponse<AppRelease>>({
    items: [],
    page: 1,
    page_size: 10,
    total_items: 0,
    total_pages: 1,
  })

  const auditLogsPagination = ref<PaginatedResponse<AuditLog>>({
    items: [],
    page: 1,
    page_size: 10,
    total_items: 0,
    total_pages: 1,
  })

  // Backward-compatibility shortcuts for views
  const clients = ref<Client[]>([])
  const licenses = ref<License[]>([])
  const devices = ref<DeviceActivation[]>([])
  const releases = ref<AppRelease[]>([])
  const auditLogs = ref<AuditLog[]>([])

  const settings = ref<ProviderSettings>({
    server_port: 19090,
    auto_start_server: true,
    master_private_key: '40c97328401a31ed63b6ecd07bf1ea35a731e775cab79c730582b49069b0a5b9',
    master_public_key: '779b7d3fb80a25e9503bf7b85ffbd226ed9a4b8ff9de844872dd3a19357bb213',
  })

  const isLoading = ref(false)
  const errorMessage = ref('')
  const successMessage = ref('')

  // ----------------------------------------------------------------------------
  // STATUS & SERVER
  // ----------------------------------------------------------------------------
  async function fetchStatus() {
    try {
      const { data } = await apiClient.get<ProviderStatus>('/api/v1/admin/status')
      if (data && data.port) {
        status.value = data
      }
    } catch (e: any) {
      console.warn('fetchStatus via HTTP failed, checking IPC or status fallback', e)
    }
  }

  async function toggleServer(enable: boolean) {
    try {
      const { data } = await apiClient.post<{ is_server_running: boolean }>('/api/v1/admin/server/toggle', { enable })
      status.value.is_server_running = data.is_server_running
      successMessage.value = data.is_server_running
        ? 'Server API Provider berhasil diaktifkan'
        : 'Server API Provider dihentikan'
    } catch (e: any) {
      errorMessage.value = e?.message || 'Gagal mengubah status server'
    }
  }

  // ----------------------------------------------------------------------------
  // CLIENTS
  // ----------------------------------------------------------------------------
  async function fetchClients(params: PaginationParams = {}) {
    isLoading.value = true
    try {
      const queryParams: Record<string, any> = {
        page: params.page || clientsPagination.value.page || 1,
        page_size: params.page_size || clientsPagination.value.page_size || 10,
        sort_by: params.sort_by || 'created_at',
        sort_order: params.sort_order || 'desc',
      }
      if (params.search) queryParams.search = params.search

      const { data } = await apiClient.get<PaginatedResponse<Client>>('/api/v1/admin/clients', {
        params: queryParams,
      })
      if (data && Array.isArray(data.items)) {
        clientsPagination.value = data
        clients.value = data.items
      }
    } catch (e: any) {
      errorMessage.value = e?.message || 'Gagal memuat data klien'
    } finally {
      isLoading.value = false
    }
  }

  async function saveClient(payload: {
    client_name: string
    company_name: string
    email: string
    phone?: string
    address?: string
  }) {
    isLoading.value = true
    try {
      await apiClient.post('/api/v1/admin/clients', payload)
      successMessage.value = 'Klien berhasil didaftarkan'
      await fetchClients()
      await fetchStatus()
      return true
    } catch (e: any) {
      errorMessage.value = e?.message || 'Gagal menyimpan klien'
      return false
    } finally {
      isLoading.value = false
    }
  }

  async function updateClient(
    clientId: string,
    payload: {
      client_name: string
      company_name: string
      email: string
      phone?: string
      address?: string
    }
  ) {
    isLoading.value = true
    try {
      await apiClient.put(`/api/v1/admin/clients/${clientId}`, { id: clientId, ...payload })
      successMessage.value = 'Data klien berhasil diperbarui'
      await fetchClients()
      return true
    } catch (e: any) {
      errorMessage.value = e?.message || 'Gagal memperbarui klien'
      return false
    } finally {
      isLoading.value = false
    }
  }

  async function deleteClient(clientId: string) {
    try {
      await apiClient.delete(`/api/v1/admin/clients/${clientId}`)
      successMessage.value = 'Klien berhasil dihapus'
      await fetchClients()
      await fetchStatus()
      return true
    } catch (e: any) {
      errorMessage.value = e?.message || 'Gagal menghapus klien'
      return false
    }
  }

  // ----------------------------------------------------------------------------
  // LICENSES
  // ----------------------------------------------------------------------------
  async function fetchLicenses(params: PaginationParams = {}) {
    isLoading.value = true
    try {
      const queryParams: Record<string, any> = {
        page: params.page || licensesPagination.value.page || 1,
        page_size: params.page_size || licensesPagination.value.page_size || 10,
        sort_by: params.sort_by || 'created_at',
        sort_order: params.sort_order || 'desc',
      }
      if (params.search) queryParams.search = params.search
      if (params.status) queryParams.status = params.status
      if (params.tier) queryParams.tier = params.tier

      const { data } = await apiClient.get<PaginatedResponse<License>>('/api/v1/admin/licenses', {
        params: queryParams,
      })
      if (data && Array.isArray(data.items)) {
        licensesPagination.value = data
        licenses.value = data.items
      }
    } catch (e: any) {
      errorMessage.value = e?.message || 'Gagal memuat data lisensi'
    } finally {
      isLoading.value = false
    }
  }

  async function generateLicense(payload: {
    client_id: string
    tier: string
    max_terminals: number
    expires_in_days: number
    grace_period_days?: number
    features: string[]
    notes?: string
    custom_key?: string
  }) {
    isLoading.value = true
    try {
      const { data } = await apiClient.post<License>('/api/v1/admin/licenses', payload)
      successMessage.value = `Lisensi ${data.license_key} berhasil diterbitkan`
      await fetchLicenses()
      await fetchStatus()
      return data
    } catch (e: any) {
      errorMessage.value = e?.message || 'Gagal menerbitkan lisensi'
      return null
    } finally {
      isLoading.value = false
    }
  }

  async function updateLicenseStatus(licenseId: string, newStatus: string) {
    try {
      await apiClient.put(`/api/v1/admin/licenses/${licenseId}/status`, { status: newStatus })
      successMessage.value = `Status lisensi diubah menjadi ${newStatus}`
      await fetchLicenses()
      await fetchStatus()
      return true
    } catch (e: any) {
      errorMessage.value = e?.message || 'Gagal mengubah status lisensi'
      return false
    }
  }

  async function deleteLicense(licenseId: string) {
    try {
      await apiClient.delete(`/api/v1/admin/licenses/${licenseId}`)
      successMessage.value = 'Lisensi berhasil dihapus'
      await fetchLicenses()
      await fetchStatus()
      return true
    } catch (e: any) {
      errorMessage.value = e?.message || 'Gagal menghapus lisensi'
      return false
    }
  }

  // ----------------------------------------------------------------------------
  // DEVICES
  // ----------------------------------------------------------------------------
  async function fetchDevices(params: PaginationParams = {}) {
    isLoading.value = true
    try {
      const queryParams: Record<string, any> = {
        page: params.page || devicesPagination.value.page || 1,
        page_size: params.page_size || devicesPagination.value.page_size || 10,
        sort_by: params.sort_by || 'last_ping_at',
        sort_order: params.sort_order || 'desc',
      }
      if (params.search) queryParams.search = params.search
      if (params.status) queryParams.status = params.status

      const { data } = await apiClient.get<PaginatedResponse<DeviceActivation>>('/api/v1/admin/devices', {
        params: queryParams,
      })
      if (data && Array.isArray(data.items)) {
        devicesPagination.value = data
        devices.value = data.items
      }
    } catch (e: any) {
      errorMessage.value = e?.message || 'Gagal memuat aktivasi perangkat'
    } finally {
      isLoading.value = false
    }
  }

  async function unbindDevice(deviceId: string) {
    try {
      await apiClient.delete(`/api/v1/admin/devices/${deviceId}`)
      successMessage.value = 'Tautan perangkat HWID berhasil di-reset'
      await fetchDevices()
      await fetchStatus()
      return true
    } catch (e: any) {
      errorMessage.value = e?.message || 'Gagal melepas tautan perangkat'
      return false
    }
  }

  // ----------------------------------------------------------------------------
  // RELEASES
  // ----------------------------------------------------------------------------
  async function fetchReleases(params: PaginationParams = {}) {
    isLoading.value = true
    try {
      const queryParams: Record<string, any> = {
        page: params.page || releasesPagination.value.page || 1,
        page_size: params.page_size || releasesPagination.value.page_size || 10,
        sort_by: params.sort_by || 'created_at',
        sort_order: params.sort_order || 'desc',
      }
      if (params.search) queryParams.search = params.search

      const { data } = await apiClient.get<PaginatedResponse<AppRelease>>('/api/v1/admin/releases', {
        params: queryParams,
      })
      if (data && Array.isArray(data.items)) {
        releasesPagination.value = data
        releases.value = data.items
      }
    } catch (e: any) {
      errorMessage.value = e?.message || 'Gagal memuat daftar rilis OTA'
    } finally {
      isLoading.value = false
    }
  }

  async function createRelease(payload: {
    version: string
    release_name: string
    changelog: string
    download_url: string
    checksum_sha256: string
    is_mandatory: boolean
    min_license_tier: string
    has_db_migration: boolean
  }) {
    isLoading.value = true
    try {
      const { data } = await apiClient.post<AppRelease>('/api/v1/admin/releases', payload)
      successMessage.value = `Rilis versi ${data.version} berhasil diterbitkan dan ditandatangani`
      await fetchReleases()
      await fetchStatus()
      return true
    } catch (e: any) {
      errorMessage.value = e?.message || 'Gagal menerbitkan rilis'
      return false
    } finally {
      isLoading.value = false
    }
  }

  async function deleteRelease(releaseId: string) {
    try {
      await apiClient.delete(`/api/v1/admin/releases/${releaseId}`)
      successMessage.value = 'Rilis update berhasil dihapus'
      await fetchReleases()
      await fetchStatus()
      return true
    } catch (e: any) {
      errorMessage.value = e?.message || 'Gagal menghapus rilis'
      return false
    }
  }

  // ----------------------------------------------------------------------------
  // AUDIT LOGS
  // ----------------------------------------------------------------------------
  async function fetchAuditLogs(params: PaginationParams = {}) {
    isLoading.value = true
    try {
      const queryParams: Record<string, any> = {
        page: params.page || auditLogsPagination.value.page || 1,
        page_size: params.page_size || auditLogsPagination.value.page_size || 25,
        sort_by: params.sort_by || 'created_at',
        sort_order: params.sort_order || 'desc',
      }
      if (params.search) queryParams.search = params.search
      if (params.status) queryParams.status = params.status

      const { data } = await apiClient.get<PaginatedResponse<AuditLog>>('/api/v1/admin/audit-logs', {
        params: queryParams,
      })
      if (data && Array.isArray(data.items)) {
        auditLogsPagination.value = data
        auditLogs.value = data.items
      }
    } catch (e: any) {
      errorMessage.value = e?.message || 'Gagal memuat log audit lisensi'
    } finally {
      isLoading.value = false
    }
  }

  // ----------------------------------------------------------------------------
  // OFFLINE SIGNER
  // ----------------------------------------------------------------------------
  async function signTokenManual(licenseKey: string, hardwareId: string): Promise<ManualSignResult | null> {
    isLoading.value = true
    try {
      const { data } = await apiClient.post<ManualSignResult>('/api/v1/admin/sign-token', {
        license_key: licenseKey.trim(),
        hardware_id: hardwareId.trim(),
      })
      successMessage.value = 'Token lisensi offline berhasil ditandatangani secara kriptografis'
      return data
    } catch (e: any) {
      errorMessage.value = e?.message || 'Gagal menandatangani token lisensi'
      return null
    } finally {
      isLoading.value = false
    }
  }

  // ----------------------------------------------------------------------------
  // SETTINGS
  // ----------------------------------------------------------------------------
  async function fetchSettings() {
    try {
      const { data } = await apiClient.get<ProviderSettings>('/api/v1/admin/settings')
      if (data) {
        settings.value = data
      }
    } catch (e: any) {
      console.warn('fetchSettings error', e)
    }
  }

  async function saveSettings(newSettings: ProviderSettings) {
    isLoading.value = true
    try {
      await apiClient.post('/api/v1/admin/settings', newSettings)
      settings.value = newSettings
      successMessage.value = 'Pengaturan provider berhasil disimpan'
      await fetchStatus()
      return true
    } catch (e: any) {
      errorMessage.value = e?.message || 'Gagal menyimpan pengaturan'
      return false
    } finally {
      isLoading.value = false
    }
  }

  function clearNotifications() {
    errorMessage.value = ''
    successMessage.value = ''
  }

  return {
    status,
    clients,
    clientsPagination,
    licenses,
    licensesPagination,
    devices,
    devicesPagination,
    releases,
    releasesPagination,
    auditLogs,
    auditLogsPagination,
    settings,
    isLoading,
    errorMessage,
    successMessage,
    clearNotifications,
    fetchStatus,
    toggleServer,
    fetchClients,
    saveClient,
    updateClient,
    deleteClient,
    fetchLicenses,
    generateLicense,
    updateLicenseStatus,
    deleteLicense,
    fetchDevices,
    unbindDevice,
    fetchReleases,
    createRelease,
    deleteRelease,
    fetchAuditLogs,
    signTokenManual,
    fetchSettings,
    saveSettings,
  }
})
