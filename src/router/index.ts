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

const routes = [
  {
    path: '/',
    name: 'pos',
    component: PosRegister,
    meta: { requiresAuth: true },
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
    meta: { requiresAuth: true },
  },
  {
    path: '/admin/sales',
    name: 'admin-sales',
    component: SalesPage,
    meta: { requiresAuth: true },
  },
  {
    path: '/admin/purchases',
    name: 'admin-purchases',
    component: PurchasesPage,
    meta: { requiresAuth: true },
  },
  {
    path: '/admin/products',
    name: 'admin-products',
    component: ProductsPage,
    meta: { requiresAuth: true },
  },
  {
    path: '/admin/customers',
    name: 'admin-customers',
    component: CustomersPage,
    meta: { requiresAuth: true },
  },
  {
    path: '/admin/vouchers',
    name: 'admin-vouchers',
    component: VouchersPage,
    meta: { requiresAuth: true },
  },
  {
    path: '/admin/parameters',
    name: 'admin-parameters',
    component: ParametersPage,
    meta: { requiresAuth: true },
  },
  {
    path: '/admin/inventory',
    name: 'admin-inventory',
    component: InventoryPage,
    meta: { requiresAuth: true },
  },
  {
    path: '/admin/accounting',
    name: 'admin-accounting',
    component: AccountingPage,
    meta: { requiresAuth: true },
  },
  {
    path: '/admin/audit-logs',
    name: 'admin-audit-logs',
    component: AuditLogPage,
    meta: { requiresAuth: true },
  },
  {
    path: '/admin/receipt-settings',
    name: 'admin-receipt-settings',
    component: ReceiptSettingsPage,
    meta: { requiresAuth: true },
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
    meta: { requiresAuth: true },
  },
]

const router = createRouter({
  history: createWebHistory(),
  routes,
})

router.beforeEach((to, _from, next) => {
  const token = localStorage.getItem('notane_token')
  if (to.meta.requiresAuth && !token) {
    next({ name: 'login' })
  } else {
    next()
  }
})

export default router
