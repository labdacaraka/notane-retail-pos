import axios from 'axios'

export function getProviderApiBase(): string {
  const envUrl = ((import.meta as any).env?.VITE_PROVIDER_API_URL as string)?.trim()
  if (envUrl) return envUrl

  if (typeof window !== 'undefined') {
    // Port 5177 adalah Vite dev server; arahkan langsung ke backend port 19090
    if (window.location.port === '5177' || (import.meta as any).env?.DEV) {
      return 'http://127.0.0.1:19090'
    }
    if (window.location.origin && !window.location.origin.includes('tauri://')) {
      return window.location.origin
    }
  }
  return 'http://127.0.0.1:19090'
}

export const PROVIDER_API_BASE = getProviderApiBase()

export const apiClient = axios.create({
  baseURL: getProviderApiBase(),
  timeout: 15000,
  headers: {
    'Content-Type': 'application/json',
  },
})

// Attach session token automatically and ensure current baseURL is used
apiClient.interceptors.request.use((config) => {
  config.baseURL = getProviderApiBase()
  const token = localStorage.getItem('notane_manager_session_token')
  if (token) {
    config.headers.Authorization = `Bearer ${token}`
  }
  return config
})

// Intercept errors
apiClient.interceptors.response.use(
  (response) => response,
  (error) => {
    const errorMsg =
      error.response?.data?.error ||
      error.response?.data?.details ||
      error.response?.data?.message ||
      error.message ||
      'Terjadi kesalahan pada server provider'
    return Promise.reject(new Error(errorMsg))
  }
)

export default apiClient
