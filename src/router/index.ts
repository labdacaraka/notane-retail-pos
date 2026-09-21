import { createRouter, createWebHistory } from 'vue-router'
import PosRegister from '@/pages/PosRegister.vue'
import Login from '@/pages/Login.vue'
import CustomerDisplay from '@/pages/CustomerDisplay.vue'
import AdminDashboard from '@/pages/AdminDashboard.vue'
import ProductsPage from '@/pages/ProductsPage.vue'
import CustomersPage from '@/pages/CustomersPage.vue'
import VouchersPage from '@/pages/VouchersPage.vue'
import ParametersPage from '@/pages/ParametersPage.vue'
import InventoryPage from '@/pages/InventoryPage.vue'
import AccountingPage from '@/pages/AccountingPage.vue'
import AuditLogPage from '@/pages/AuditLogPage.vue'
import ServerControlPage from '@/pages/ServerControlPage.vue'
import ReceiptSettingsPage from '@/pages/ReceiptSettingsPage.vue'
import ClientConnectionPage from '@/pages/ClientConnectionPage.vue'
import SalesPage from '@/pages/SalesPage.vue'
import PurchasesPage from '@/pages/PurchasesPage.vue'
import UsersPage from '@/pages/UsersPage.vue'
import OutletsPage from '@/pages/OutletsPage.vue'

const routes = [
  {
    path: '/',
    name: 'pos',
    component: PosRegister,
    meta: { requiresAuth: true, permission: 'pos:checkout' },
  },
  {
    path: '/login',
    name: 'login',
    component: Login,
  },
  {
    path: '/customer-display',
    name: 'customer-display',
    component: CustomerDisplay,
  },
  {
    path: '/admin',
    name: 'admin-dashboard',
    component: AdminDashboard,
    meta: { requiresAuth: true },
  },
  {
    path: '/admin/server-control',
    name: 'admin-server-control',
    component: ServerControlPage,
    meta: { requiresAuth: true, permission: 'settings:manage' },
  },
  {
    path: '/admin/sales',
    name: 'admin-sales',
    component: SalesPage,
    meta: { requiresAuth: true, permission: 'report:view' },
  },
  {
    path: '/admin/purchases',
    name: 'admin-purchases',
    component: PurchasesPage,
    meta: { requiresAuth: true, permission: 'purchasing:read' },
  },
  {
    path: '/admin/outlets',
    name: 'admin-outlets',
    component: OutletsPage,
    meta: { requiresAuth: true, permission: ['outlets:read', 'master:read'] },
  },
  {
    path: '/admin/products',
    name: 'admin-products',
    component: ProductsPage,
    meta: { requiresAuth: true, permission: 'master:read' },
  },
  {
    path: '/admin/customers',
    name: 'admin-customers',
    component: CustomersPage,
    meta: { requiresAuth: true, permission: 'master:read' },
  },
  {
    path: '/admin/vouchers',
    name: 'admin-vouchers',
    component: VouchersPage,
    meta: { requiresAuth: true, permission: 'master:read' },
  },
  {
    path: '/admin/parameters',
    name: 'admin-parameters',
    component: ParametersPage,
    meta: { requiresAuth: true, permission: ['settings:manage', 'master:write'] },
  },
  {
    path: '/admin/inventory',
    name: 'admin-inventory',
    component: InventoryPage,
    meta: { requiresAuth: true, permission: 'inventory:read' },
  },
  {
    path: '/admin/accounting',
    name: 'admin-accounting',
    component: AccountingPage,
    meta: { requiresAuth: true, permission: 'accounting:read' },
  },
  {
    path: '/admin/audit-logs',
    name: 'admin-audit-logs',
    component: AuditLogPage,
    meta: { requiresAuth: true, permission: ['audit:read', 'report:view'] },
  },
  {
    path: '/admin/receipt-settings',
    name: 'admin-receipt-settings',
    component: ReceiptSettingsPage,
    meta: { requiresAuth: true, permission: 'settings:manage' },
  },
  {
    path: '/admin/client-connection',
    name: 'admin-client-connection',
    component: ClientConnectionPage,
    meta: { requiresAuth: true },
  },
  {
    path: '/admin/users',
    name: 'admin-users',
    component: UsersPage,
    meta: { requiresAuth: true, permission: 'users:manage' },
  },
]

const router = createRouter({
  history: createWebHistory(),
  routes,
})

router.beforeEach((to, _from, next) => {
  const token = localStorage.getItem('notane_token')
  if (to.meta.requiresAuth && !token) {
    return next({ name: 'login' })
  }

  // Permission Verification
  const requiredPerm = to.meta.permission as string | string[] | undefined
  if (requiredPerm) {
    const userStr = localStorage.getItem('notane_user')
    let user: any = null
    try {
      user = userStr ? JSON.parse(userStr) : null
    } catch {}

    if (user) {
      if (user.role_name === 'SUPERADMIN') {
        return next()
      }

      const perms: string[] = user.permissions || []
      const hasPerm = Array.isArray(requiredPerm)
        ? requiredPerm.some(p => perms.includes(p))
        : perms.includes(requiredPerm)

      if (!hasPerm) {
        console.warn(`[AuthGuard] Akses ditolak untuk rute ${to.path}. Dibutuhkan wewenang:`, requiredPerm)
        // Jika kasir mencoba akses menu admin yang tidak berhak, redirect ke POS
        if (perms.includes('pos:checkout') && to.path.startsWith('/admin')) {
          return next({ path: '/' })
        }
        if (to.path !== '/admin') {
          return next({ path: '/admin' })
        }
      }
    }
  }

  next()
})

export default router
