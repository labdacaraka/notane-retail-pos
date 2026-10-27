import axios from 'axios'

export function getApiBaseUrl(): string {
  let raw = (localStorage.getItem('notane_target_host') || '127.0.0.1').trim()
  if (raw.includes('tauri') || raw.startsWith('tauri:')) {
    raw = '127.0.0.1'
    localStorage.setItem('notane_target_host', '127.0.0.1')
  }
  const port = localStorage.getItem('notane_target_port') || '18080'
  if (raw.startsWith('http://') || raw.startsWith('https://')) {
    return `${raw.replace(/\/$/, '')}/api/v1`
  }
  return `http://${raw}:${port}/api/v1`
}

export const api = axios.create({
  baseURL: getApiBaseUrl(),
  headers: {
    'Content-Type': 'application/json',
  },
})

api.interceptors.request.use((config) => {
  config.baseURL = getApiBaseUrl()

  const pairingCode = localStorage.getItem('notane_pairing_code')
  if (pairingCode) {
    config.headers['X-Pairing-Code'] = pairingCode
  }

  const token = localStorage.getItem('notane_token')
  if (token) {
    config.headers.Authorization = `Bearer ${token}`
  }
  return config
})

api.interceptors.response.use(
  (response) => response,
  (error) => {
    if (error.response?.status === 401) {
      if (error.response?.data?.code === 'INVALID_PAIRING_CODE') {
        localStorage.setItem('notane_connection_intent', 'DISCONNECTED')
      } else {
        // Clear token if expired
        localStorage.removeItem('notane_token')
      }
    }
    return Promise.reject(error)
  }
)
