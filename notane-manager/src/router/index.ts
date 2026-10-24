import { createRouter, createWebHashHistory } from 'vue-router'
import { useAuthStore } from '../stores/authStore'
import DashboardView from '../pages/DashboardView.vue'
import ClientsView from '../pages/ClientsView.vue'
import LicensesView from '../pages/LicensesView.vue'
import OfflineSignerView from '../pages/OfflineSignerView.vue'
import DevicesView from '../pages/DevicesView.vue'
import ReleasesView from '../pages/ReleasesView.vue'
import AuditLogsView from '../pages/AuditLogsView.vue'
import SettingsView from '../pages/SettingsView.vue'
import LoginView from '../pages/LoginView.vue'

const routes = [
  { path: '/login', name: 'login', component: LoginView, meta: { public: true } },
  { path: '/', name: 'dashboard', component: DashboardView },
  { path: '/clients', name: 'clients', component: ClientsView },
  { path: '/licenses', name: 'licenses', component: LicensesView },
  { path: '/signer', name: 'signer', component: OfflineSignerView },
  { path: '/devices', name: 'devices', component: DevicesView },
  { path: '/releases', name: 'releases', component: ReleasesView },
  { path: '/audit', name: 'audit', component: AuditLogsView },
  { path: '/settings', name: 'settings', component: SettingsView },
]

export const router = createRouter({
  history: createWebHashHistory(),
  routes,
})

router.beforeEach(async (to, from, next) => {
  const authStore = useAuthStore()

  // Verify session validity
  if (authStore.sessionToken) {
    await authStore.checkSession()
  }

  const isPublic = to.meta.public === true

  if (!isPublic && !authStore.isAuthenticated) {
    return next({ path: '/login', query: { redirect: to.fullPath } })
  }

  if (isPublic && authStore.isAuthenticated && to.name === 'login') {
    return next({ path: '/' })
  }

  next()
})
