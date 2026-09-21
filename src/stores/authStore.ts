import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { api } from '@/services/api'
import type { UserProfile } from '@/types'

export const useAuthStore = defineStore('auth', () => {
  const token = ref<string | null>(localStorage.getItem('notane_token'))
  const user = ref<UserProfile | null>(
    localStorage.getItem('notane_user')
      ? JSON.parse(localStorage.getItem('notane_user')!)
      : null
  )
  const terminalId = ref<string>(localStorage.getItem('notane_terminal') || 'KASIR-01')

  const isAuthenticated = computed(() => !!token.value)
  const isSupervisor = computed(() =>
    ['SUPERVISOR', 'STORE_MANAGER', 'SUPERADMIN'].includes(user.value?.role_name || '')
  )

  async function login(username: string, password: string, terminal?: string) {
    if (terminal) {
      terminalId.value = terminal
      localStorage.setItem('notane_terminal', terminal)
    }

    const res = await api.post('/auth/login', {
      username,
      password,
      terminal_id: terminalId.value,
    })

    if (res.data.success) {
      token.value = res.data.data.token
      user.value = res.data.data.user
      localStorage.setItem('notane_token', res.data.data.token)
      localStorage.setItem('notane_user', JSON.stringify(res.data.data.user))
      return res.data.data
    } else {
      throw new Error(res.data.message || 'Login gagal')
    }
  }

  function logout() {
    token.value = null
    user.value = null
    localStorage.removeItem('notane_token')
    localStorage.removeItem('notane_user')
  }

  async function fetchCurrentUser() {
    if (!token.value) return null
    try {
      const res = await api.get('/auth/me')
      if (res.data?.success && res.data?.data) {
        user.value = res.data.data
        localStorage.setItem('notane_user', JSON.stringify(res.data.data))
        return res.data.data
      }
    } catch (err) {
      console.error('Failed to refresh user profile:', err)
    }
    return null
  }

  function hasPermission(perm: string | string[]): boolean {
    if (!user.value) return false
    if (user.value.role_name === 'SUPERADMIN') return true
    const perms = user.value.permissions || []
    if (Array.isArray(perm)) {
      return perm.some(p => perms.includes(p))
    }
    return perms.includes(perm)
  }

  return {
    token,
    user,
    terminalId,
    isAuthenticated,
    isSupervisor,
    login,
    logout,
    fetchCurrentUser,
    hasPermission,
  }
})
