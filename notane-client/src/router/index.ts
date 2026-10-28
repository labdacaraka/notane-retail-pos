import { createRouter, createWebHistory } from 'vue-router'
import PosRegister from '@/pages/PosRegister.vue'
import Login from '@/pages/Login.vue'
import CustomerDisplay from '@/pages/CustomerDisplay.vue'
import AdminDashboard from '@/pages/AdminDashboard.vue'
import ProductsPage from '@/pages/ProductsPage.vue'
import PrintLabelsPage from '@/pages/PrintLabelsPage.vue'
import CustomersPage from '@/pages/CustomersPage.vue'
import VouchersPage from '@/pages/VouchersPage.vue'
import FlashSalesPage from '@/pages/FlashSalesPage.vue'
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
import ExpensesPage from '@/pages/ExpensesPage.vue'
import PromotionsPage from '@/pages/PromotionsPage.vue'
const routes = [
  {
    path: '/server-host',
    alias: '/server',
    redirect: '/',
  },
  {
    path: '/',
    name: 'pos',
    component: PosRegister,
    meta: { requiresAuth: true, permission: ['sales:create', 'pos:checkout'] },
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
    meta: { requiresAuth: true, permission: ['sales:read', 'report:view'] },
  },
  {
    path: '/admin/purchases',
    name: 'admin-purchases',
    component: PurchasesPage,
    meta: { requiresAuth: true, permission: ['purchasing:read', 'purchasing:write'] },
  },
  {
    path: '/admin/outlets',
    name: 'admin-outlets',
    component: OutletsPage,
    meta: { requiresAuth: true, permission: ['outlets:read', 'outlets:manage', 'master:read'] },
  },
  {
    path: '/admin/products',
    name: 'admin-products',
    component: ProductsPage,
    meta: { requiresAuth: true, permission: ['products:read', 'master:read'] },
  },
  {
    path: '/admin/price-labels',
    name: 'admin-price-labels',
    component: PrintLabelsPage,
    meta: { requiresAuth: true, permission: ['products:read', 'master:read'], requiredFeature: 'label_printing' },
  },
  {
    path: '/admin/customers',
    name: 'admin-customers',
    component: CustomersPage,
    meta: { requiresAuth: true, permission: ['customers:read', 'customers:manage', 'master:read'], requiredFeature: 'membership' },
  },
  {
    path: '/admin/vouchers',
    name: 'admin-vouchers',
    component: VouchersPage,
    meta: { requiresAuth: true, permission: ['vouchers:read', 'vouchers:manage', 'master:read'], requiredFeature: 'promotions' },
  },
  {
    path: '/admin/flash-sales',
    name: 'admin-flash-sales',
    component: FlashSalesPage,
    meta: { requiresAuth: true, permission: ['promotions:read', 'master:read'], requiredFeature: 'promotions' },
  },
  {
    path: '/admin/promotions',
    name: 'admin-promotions',
    component: PromotionsPage,
    meta: { requiresAuth: true, permission: ['promotions:read', 'master:read'], requiredFeature: 'promotions' },
  },
  {
    path: '/admin/parameters',
    name: 'admin-parameters',
    component: ParametersPage,
    meta: { requiresAuth: true, permission: ['settings:manage', 'settings:update', 'master:write'] },
  },
  {
    path: '/admin/inventory',
    name: 'admin-inventory',
    component: InventoryPage,
    meta: { requiresAuth: true, permission: ['inventory:read', 'inventory:write'] },
  },
  {
    path: '/admin/accounting',
    name: 'admin-accounting',
    component: AccountingPage,
    meta: { requiresAuth: true, permission: ['accounting:read', 'accounting:write'], requiredFeature: 'accounting' },
  },
  {
    path: '/admin/expenses',
    name: 'admin-expenses',
    component: ExpensesPage,
    meta: { requiresAuth: true, permission: ['expenses:read', 'expenses:categories'], requiredFeature: 'expenses' },
  },
  {
    path: '/admin/audit-logs',
    name: 'admin-audit-logs',
    component: AuditLogPage,
    meta: { requiresAuth: true, permission: ['audit:read', 'report:view'], requiredFeature: 'audit_log' },
  },
  {
    path: '/admin/receipt-settings',
    name: 'admin-receipt-settings',
    component: ReceiptSettingsPage,
    meta: { requiresAuth: true, permission: ['settings:read', 'settings:update', 'settings:manage'] },
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
    meta: { requiresAuth: true, permission: ['users:read', 'users:manage'] },
  },
]

const router = createRouter({
  history: createWebHistory(),
  routes,
})

router.beforeEach(async (to, _from, next) => {
  const token = localStorage.getItem('notane_token')
  if (to.meta.requiresAuth && !token) {
    return next({ name: 'login' })
  }

  // Feature License Verification (Zero Trust Frontend Guard)
  const reqFeature = to.meta.requiredFeature as string | undefined
  if (reqFeature) {
    const { useLicenseStore } = await import('@/stores/licenseStore')
    const licStore = useLicenseStore()
    if (!licStore.licenseInfo.is_valid && licStore.licenseInfo.status === 'UNLICENSED') {
      await licStore.fetchLicenseInfo()
    }
    if (!licStore.hasFeature(reqFeature)) {
      console.warn(`[LicenseGuard] Akses ditolak: Modul '${reqFeature}' tidak diaktifkan pada lisensi server.`)
      const featureNames: Record<string, string> = {
        membership: 'Pelanggan & Membership',
        accounting: 'Akuntansi & Pembukuan',
        promotions: 'Promosi Retail, Voucher & Flash Sale',
        label_printing: 'Cetak Label Harga',
        expenses: 'Biaya & Pengeluaran Kas',
        audit_log: 'Jejak Audit Keamanan',
      }
      const featName = featureNames[reqFeature] || reqFeature
      alert(`Fitur Belum Diaktifkan: Modul '${featName}' memerlukan lisensi tambahan pada server toko Anda. Silakan hubungi Provider Notane untuk mengaktifkan fitur ini.`)
      return next(false)
    }
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
