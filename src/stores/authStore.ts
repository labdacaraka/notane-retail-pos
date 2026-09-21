import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { api } from '@/services/api'
import { useLocationStore } from '@/stores/locationStore'
import type { UserProfile } from '@/types'

export interface ImpersonationBackup {
  originalToken: string
  originalUser: UserProfile
  impersonatedAt: string
}

export const useAuthStore = defineStore('auth', () => {
  const token = ref<string | null>(localStorage.getItem('notane_token'))
  const user = ref<UserProfile | null>(
    localStorage.getItem('notane_user')
      ? JSON.parse(localStorage.getItem('notane_user')!)
      : null
  )
  const terminalId = ref<string>(localStorage.getItem('notane_terminal') || 'KASIR-01')

  const impersonatorBackup = ref<ImpersonationBackup | null>(
    localStorage.getItem('notane_impersonator_backup')
      ? JSON.parse(localStorage.getItem('notane_impersonator_backup')!)
      : null
  )

  const isAuthenticated = computed(() => !!token.value)
  const isSupervisor = computed(() =>
    ['SUPERVISOR', 'STORE_MANAGER', 'SUPERADMIN'].includes(user.value?.role_name || '')
  )
  const isImpersonating = computed(() => !!impersonatorBackup.value)
  const impersonator = computed(() => impersonatorBackup.value?.originalUser || null)

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
      impersonatorBackup.value = null
      localStorage.removeItem('notane_impersonator_backup')
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
    impersonatorBackup.value = null
    localStorage.removeItem('notane_token')
    localStorage.removeItem('notane_user')
    localStorage.removeItem('notane_impersonator_backup')
  }

  async function impersonateUser(targetUserId: string) {
    if (!token.value || !user.value) {
      throw new Error('Sesi tidak valid')
    }

    // Simpan akun asli sebelum impersonate pertama kali
    if (!impersonatorBackup.value) {
      const backup: ImpersonationBackup = {
        originalToken: token.value,
        originalUser: { ...user.value },
        impersonatedAt: new Date().toISOString(),
      }
      impersonatorBackup.value = backup
      localStorage.setItem('notane_impersonator_backup', JSON.stringify(backup))
    }

    try {
      const res = await api.post(`/users/${targetUserId}/impersonate`)
      if (res.data?.success && res.data?.data) {
        const { token: targetToken, user: targetUser } = res.data.data
        token.value = targetToken
        user.value = targetUser
        localStorage.setItem('notane_token', targetToken)
        localStorage.setItem('notane_user', JSON.stringify(targetUser))

        // Refresh location outlets
        const locationStore = useLocationStore()
        await locationStore.fetchAccessibleOutlets()

        return res.data
      } else {
        throw new Error(res.data?.message || 'Gagal memulai mode impersonate')
      }
    } catch (err) {
      throw err
    }
  }

  async function stopImpersonating() {
    if (!impersonatorBackup.value) return

    const backup = impersonatorBackup.value
    token.value = backup.originalToken
    user.value = backup.originalUser
    localStorage.setItem('notane_token', backup.originalToken)
    localStorage.setItem('notane_user', JSON.stringify(backup.originalUser))
    localStorage.removeItem('notane_impersonator_backup')
    impersonatorBackup.value = null

    // Refresh location outlets
    const locationStore = useLocationStore()
    await locationStore.fetchAccessibleOutlets()
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

  function hasOutletAccess(outletId: string): boolean {
    if (!user.value) return false
    if (user.value.role_name === 'SUPERADMIN') return true
    const allowed = user.value.outlet_ids || []
    return allowed.includes(outletId)
  }

  return {
    token,
    user,
    terminalId,
    impersonatorBackup,
    isImpersonating,
    impersonator,
    isAuthenticated,
    isSupervisor,
    login,
    logout,
    impersonateUser,
    stopImpersonating,
    fetchCurrentUser,
    hasPermission,
    hasOutletAccess,
  }
})

