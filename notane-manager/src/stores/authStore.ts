import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { apiClient } from '../services/api'

export interface AdminUser {
  id: string
  username: string
  full_name: string
  role: string
  last_login_at?: string
}

export interface AuthResponse {
  user: AdminUser
  token: string
  permissions: string[]
}

export const useAuthStore = defineStore('auth', () => {
  const tokenKey = 'notane_manager_session_token'
  const userKey = 'notane_manager_user'

  const sessionToken = ref<string | null>(localStorage.getItem(tokenKey) || null)
  const currentUser = ref<AdminUser | null>(
    localStorage.getItem(userKey) ? JSON.parse(localStorage.getItem(userKey)!) : null
  )

  const permissions = ref<string[]>([
    'client:read', 'client:create', 'client:update', 'client:delete',
    'license:read', 'license:create', 'license:update', 'license:revoke',
    'device:read', 'device:unbind',
    'release:read', 'release:create', 'release:delete',
    'server:control', 'audit:read',
  ])

  const isAuthenticated = computed(() => !!sessionToken.value && !!currentUser.value)

  async function invokeTauri<T>(cmd: string, args: Record<string, any> = {}): Promise<T> {
    if (typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__) {
      const { invoke } = await import('@tauri-apps/api/core')
      return invoke<T>(cmd, args)
    }

    // Fallback for browser dev mode
    console.warn(`[Browser Dev Mode] Auth IPC '${cmd}' simulated.`)
    if (cmd === 'login_admin' || cmd === 'login_with_pin') {
      const mockUser: AdminUser = {
        id: 'usr_master_admin',
        username: 'admin',
        full_name: 'Master Administrator',
        role: 'super_admin',
        last_login_at: new Date().toISOString(),
      }
      return {
        user: mockUser,
        token: 'mock-dev-token-2026',
        permissions: permissions.value,
      } as T
    }
    return true as T
  }

  async function login(username: string, password: string): Promise<{ success: boolean; message?: string }> {
    try {
      if (typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__) {
        const { invoke } = await import('@tauri-apps/api/core')
        const res = await invoke<AuthResponse>('login_admin', {
          payload: { username: username.trim(), password: password.trim() }
        })
        if (res && res.token) {
          sessionToken.value = res.token
          currentUser.value = res.user
          permissions.value = res.permissions || permissions.value
          localStorage.setItem(tokenKey, res.token)
          localStorage.setItem(userKey, JSON.stringify(res.user))
          return { success: true }
        }
      } else {
        const { data } = await apiClient.post<AuthResponse>('/api/v1/admin/auth/login', {
          username: username.trim(),
          password: password.trim(),
        })
        if (data && data.token) {
          sessionToken.value = data.token
          currentUser.value = data.user
          permissions.value = data.permissions || permissions.value
          localStorage.setItem(tokenKey, data.token)
          localStorage.setItem(userKey, JSON.stringify(data.user))
          return { success: true }
        }
      }
      return { success: false, message: 'Gagal autentikasi' }
    } catch (err: any) {
      return { success: false, message: err?.message || 'Username atau password salah' }
    }
  }

  async function loginWithPin(pin: string): Promise<{ success: boolean; message?: string }> {
    try {
      if (typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__) {
        const { invoke } = await import('@tauri-apps/api/core')
        const res = await invoke<AuthResponse>('login_with_pin', {
          payload: { pin: pin.trim() }
        })
        if (res && res.token) {
          sessionToken.value = res.token
          currentUser.value = res.user
          permissions.value = res.permissions || permissions.value
          localStorage.setItem(tokenKey, res.token)
          localStorage.setItem(userKey, JSON.stringify(res.user))
          return { success: true }
        }
      } else {
        const { data } = await apiClient.post<AuthResponse>('/api/v1/admin/auth/pin', {
          pin: pin.trim(),
        })
        if (data && data.token) {
          sessionToken.value = data.token
          currentUser.value = data.user
          permissions.value = data.permissions || permissions.value
          localStorage.setItem(tokenKey, data.token)
          localStorage.setItem(userKey, JSON.stringify(data.user))
          return { success: true }
        }
      }
      return { success: false, message: 'PIN tidak valid' }
    } catch (err: any) {
      return { success: false, message: err?.message || 'PIN keamanan salah' }
    }
  }

  async function checkSession(): Promise<boolean> {
    if (!sessionToken.value) return false
    try {
      if (typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__) {
        const { invoke } = await import('@tauri-apps/api/core')
        const isValid = await invoke<boolean>('verify_admin_session', { token: sessionToken.value })
        if (!isValid) {
          logout()
          return false
        }
      } else {
        await apiClient.get('/api/v1/admin/auth/verify')
      }
      return true
    } catch (_) {
      logout()
      return false
    }
  }

  async function logout() {
    if (sessionToken.value) {
      try {
        if (typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__) {
          const { invoke } = await import('@tauri-apps/api/core')
          await invoke<boolean>('logout_admin', { token: sessionToken.value })
        } else {
          await apiClient.post('/api/v1/admin/auth/logout')
        }
      } catch (_) {}
    }
    sessionToken.value = null
    currentUser.value = null
    localStorage.removeItem(tokenKey)
    localStorage.removeItem(userKey)
  }

  async function changeCredentials(payload: {
    old_password?: string
    new_password?: string
    new_pin?: string
  }): Promise<{ success: boolean; message?: string }> {
    try {
      if (typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__) {
        const { invoke } = await import('@tauri-apps/api/core')
        await invoke<boolean>('change_admin_credentials', { payload })
      } else {
        await apiClient.post('/api/v1/admin/auth/change-credentials', payload)
      }
      return { success: true }
    } catch (err: any) {
      return { success: false, message: err?.message || 'Gagal mengubah kata sandi' }
    }
  }

  function hasPermission(permissionSlug: string): boolean {
    if (currentUser.value?.role === 'super_admin') return true
    return permissions.value.includes(permissionSlug)
  }

  return {
    currentUser,
    sessionToken,
    permissions,
    isAuthenticated,
    login,
    loginWithPin,
    checkSession,
    logout,
    changeCredentials,
    hasPermission,
  }
})
