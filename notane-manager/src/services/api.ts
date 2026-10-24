import axios from 'axios'

// Provider Server default port is 19090
export const PROVIDER_API_BASE =
  ((import.meta as any).env?.VITE_PROVIDER_API_URL as string) || 'http://127.0.0.1:19090'

export const apiClient = axios.create({
  baseURL: PROVIDER_API_BASE,
  timeout: 15000,
  headers: {
    'Content-Type': 'application/json',
  },
})

// Attach session token automatically
apiClient.interceptors.request.use((config) => {
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
