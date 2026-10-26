import { defineStore } from 'pinia'
import { ref } from 'vue'
import axios from 'axios'

export interface InitialSetupConfig {
  store_name: string
  store_tagline: string
  store_logo_url: string | null
  store_address: string
  store_phone: string
  pairing_code: string
  cash_rounding_mode: string
  main_outlet_id: string | null
  main_outlet_code: string
  main_outlet_name: string
  main_outlet_address: string | null
  main_outlet_phone: string | null
  admin_full_name: string
  admin_username: string
}

export interface InitialSetupPayload {
  store_name: string
  store_tagline?: string | null
  store_logo_url?: string | null
  store_address: string
  store_phone: string
  pairing_code: string
  main_outlet_name: string
  main_outlet_address?: string | null
  main_outlet_phone?: string | null
  include_dummy_products: boolean
  admin_full_name?: string | null
  admin_password?: string | null
  supervisor_pin?: string | null
  cash_rounding_mode?: string
}

export const useInitialSetupStore = defineStore('initialSetup', () => {
  const isSetupCompleted = ref<boolean>(true)
  const completedAt = ref<string | null>(null)
  const hasExistingSales = ref<boolean>(false)
  const dummyProductsCount = ref<number>(0)
  const currentConfig = ref<InitialSetupConfig | null>(null)
  const isLoading = ref<boolean>(false)
  const isSaving = ref<boolean>(false)
  const lastError = ref<string | null>(null)

  function getBaseUrls(): string[] {
    const urls: string[] = []

    // 1. PRIORITAS UTAMA: Jika diakses melalui browser web, gunakan window.location.origin
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
  }

  async function fetchSetupStatus(): Promise<boolean> {
    isLoading.value = true
    lastError.value = null
    for (const base of getBaseUrls()) {
      try {
        const res = await axios.get(`${base}/api/v1/system/setup-status`, { timeout: 3000 })
        if (res.data?.success && res.data?.data) {
          const d = res.data.data
          isSetupCompleted.value = !!d.is_setup_completed
          completedAt.value = d.completed_at || null
          hasExistingSales.value = !!d.has_existing_sales
          dummyProductsCount.value = Number(d.dummy_products_count || 0)
          if (d.current_config) {
            currentConfig.value = d.current_config
          }
          return isSetupCompleted.value
        }
      } catch (err: any) {
        // Continue to next fallback base url
      }
    }
    isLoading.value = false
    return isSetupCompleted.value
  }

  async function executeInitialSetup(payload: InitialSetupPayload): Promise<{ success: boolean; message: string }> {
    isSaving.value = true
    lastError.value = null
    let lastErr = 'Gagal terhubung ke host server.'

    for (const base of getBaseUrls()) {
      try {
        const res = await axios.post(`${base}/api/v1/system/initial-setup`, payload, {
          timeout: 10000,
          headers: { 'Content-Type': 'application/json' },
        })
        if (res.data?.success) {
          isSetupCompleted.value = true
          await fetchSetupStatus()
          return {
            success: true,
            message: res.data.message || 'Konfigurasi awal berhasil disimpan!',
          }
        } else {
          lastErr = res.data?.message || lastErr
        }
      } catch (err: any) {
        lastErr = err.response?.data?.message || err.message || lastErr
      }
    }

    isSaving.value = false
    lastError.value = lastErr
    throw new Error(lastErr)
  }

  return {
    isSetupCompleted,
    completedAt,
    hasExistingSales,
    dummyProductsCount,
    currentConfig,
    isLoading,
    isSaving,
    lastError,
    fetchSetupStatus,
    executeInitialSetup,
  }
})
