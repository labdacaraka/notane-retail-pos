<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import AdminLayout from '@/layouts/AdminLayout.vue'
import { Icon } from '@iconify/vue'
import { ArrowLeft } from 'lucide-vue-next'
import { api } from '@/services/api'
import { useAuthStore } from '@/stores/authStore'

const router = useRouter()
const authStore = useAuthStore()

// ============================================================================
// TABS
// ============================================================================
type Tab = 'users' | 'roles' | 'permissions'
const activeTab = ref<Tab>('users')

// ============================================================================
// USERS
// ============================================================================
const users = ref<any[]>([])
const roles = ref<any[]>([])
const permissions = ref<any[]>([])
const loadingUsers = ref(false)
const loadingRoles = ref(false)

const userSearch = ref('')
const userFilterRole = ref<number | null>(null)
const userFilterActive = ref<'all' | 'active' | 'inactive'>('active')

const showUserModal = ref(false)
const editingUser = ref<any | null>(null)
const allOutlets = ref<any[]>([])
const userForm = ref({
  username: '',
  full_name: '',
  password: '',
  confirm_password: '',
  pin: '',
  role_id: 0,
  email: '',
  phone: '',
  outlet_ids: [] as string[],
})

const showResetPwdModal = ref(false)
const resetTarget = ref<any | null>(null)
const resetForm = ref({ new_password: '', confirm_password: '' })

// Modal Ganti PIN Khusus (Admin atau Pengguna)
const showChangePinModal = ref(false)
const changePinTarget = ref<any | null>(null)
const changePinForm = ref({ new_pin: '', confirm_pin: '' })
const changePinSaving = ref(false)
const changePinError = ref('')

// Cek apakah pengguna saat ini berhak mengubah PIN pengguna lain
const canChangePin = computed(() => {
  return authStore.user?.role_name === 'SUPERADMIN' || 
         authStore.user?.permissions?.includes('users:change_pin') ||
         authStore.user?.permissions?.includes('users:manage')
})

const isSelectedRoleSuperadmin = computed(() => {
  const r = roles.value.find(role => role.id === userForm.value.role_id)
  return r?.name === 'SUPERADMIN'
})

function toggleUserOutlet(outletId: string) {
  const idx = userForm.value.outlet_ids.indexOf(outletId)
  if (idx > -1) {
    userForm.value.outlet_ids.splice(idx, 1)
  } else {
    userForm.value.outlet_ids.push(outletId)
  }
}

function selectAllUserOutlets() {
  userForm.value.outlet_ids = allOutlets.value.map(o => o.id)
}

function clearAllUserOutlets() {
  userForm.value.outlet_ids = []
}

const filteredUsers = computed(() => {
  let list = users.value
  if (userSearch.value) {
    const q = userSearch.value.toLowerCase()
    list = list.filter(u =>
      u.full_name.toLowerCase().includes(q) ||
      u.username.toLowerCase().includes(q) ||
      (u.email || '').toLowerCase().includes(q)
    )
  }
  if (userFilterRole.value) {
    list = list.filter(u => u.role_id === userFilterRole.value)
  }
  if (userFilterActive.value !== 'all') {
    list = list.filter(u => userFilterActive.value === 'active' ? u.is_active : !u.is_active)
  }
  return list
})

async function fetchUsers() {
  loadingUsers.value = true
  try {
    const res = await api.get("/users")
    users.value = res.data.data || []
  } catch (e) { console.error(e) }
  finally { loadingUsers.value = false }
}

async function fetchRoles() {
  loadingRoles.value = true
  try {
    const res = await api.get("/roles")
    roles.value = res.data.data || []
  } catch (e) { console.error(e) }
  finally { loadingRoles.value = false }
}

async function fetchOutlets() {
  try {
    const res = await api.get("/outlets")
    allOutlets.value = res.data?.data || []
  } catch (e) { console.error(e) }
}

async function fetchPermissions() {
  try {
    const res = await api.get("/permissions")
    permissions.value = res.data.data || []
  } catch (e) { console.error(e) }
}

function openCreateUser() {
  editingUser.value = null
  userForm.value = { 
    username: '', 
    full_name: '', 
    password: '', 
    confirm_password: '', 
    pin: '', 
    role_id: roles.value[0]?.id || 0, 
    email: '', 
    phone: '',
    outlet_ids: allOutlets.value.map(o => o.id),
  }
  showUserModal.value = true
}

function openEditUser(user: any) {
  editingUser.value = user
  userForm.value = { 
    username: user.username, 
    full_name: user.full_name, 
    password: '', 
    confirm_password: '', 
    pin: '', 
    role_id: user.role_id, 
    email: user.email || '', 
    phone: user.phone || '',
    outlet_ids: (user.outlet_ids || []).slice(),
  }
  showUserModal.value = true
}

const userSaving = ref(false)
const userError = ref('')

async function saveUser() {
  userError.value = ''
  if (!editingUser.value && userForm.value.password !== userForm.value.confirm_password) {
    userError.value = 'Konfirmasi password tidak cocok!'
    return
  }
  userSaving.value = true
  try {
    if (editingUser.value) {
      await api.put(`/users/${editingUser.value.id}`, {
        full_name: userForm.value.full_name,
        role_id: userForm.value.role_id,
        email: userForm.value.email || null,
        phone: userForm.value.phone || null,
        outlet_ids: userForm.value.outlet_ids,
      })
      toast('Pengguna berhasil diperbarui', 'success')
    } else {
      if (userForm.value.pin.length < 4) { userError.value = 'PIN minimal 4 digit'; return }
      await api.post("/users", {
        username: userForm.value.username,
        full_name: userForm.value.full_name,
        password: userForm.value.password,
        pin: userForm.value.pin,
        role_id: userForm.value.role_id,
        email: userForm.value.email || null,
        phone: userForm.value.phone || null,
        outlet_ids: userForm.value.outlet_ids,
      })
      toast('Pengguna baru berhasil dibuat', 'success')
    }
    showUserModal.value = false
    await fetchUsers()
  } catch (e: any) {
    userError.value = e.response?.data?.message || e.message
  } finally {
    userSaving.value = false
  }
}

async function toggleUserActive(user: any) {
  const newStatus = !user.is_active
  await api.put(`/users/${user.id}/toggle-active`, { is_active: newStatus })
  user.is_active = newStatus
  toast(newStatus ? 'Pengguna diaktifkan kembali' : 'Pengguna dinonaktifkan', 'info')
}

function openResetPwd(user: any) {
  resetTarget.value = user
  resetForm.value = { new_password: '', confirm_password: '' }
  resetError.value = ''
  showResetPwdModal.value = true
}

const resetSaving = ref(false)
const resetError = ref('')

async function doResetPassword() {
  resetError.value = ''
  if (resetForm.value.new_password !== resetForm.value.confirm_password) {
    resetError.value = 'Konfirmasi password tidak cocok!'
    return
  }
  if (resetForm.value.new_password.length < 6) {
    resetError.value = 'Password minimal 6 karakter!'
    return
  }
  resetSaving.value = true
  try {
    await api.post(`/users/${resetTarget.value.id}/change-password`, {
      new_password: resetForm.value.new_password,
    })
    toast('Password berhasil direset', 'success')
    showResetPwdModal.value = false
  } catch (e: any) {
    resetError.value = e.response?.data?.message || e.message
  } finally {
    resetSaving.value = false
  }
}

function openChangePin(user: any) {
  changePinTarget.value = user
  changePinForm.value = { new_pin: '', confirm_pin: '' }
  changePinError.value = ''
  showChangePinModal.value = true
}

async function doChangePin() {
  changePinError.value = ''
  const pin = changePinForm.value.new_pin.trim()
  const confirmPin = changePinForm.value.confirm_pin.trim()

  if (pin !== confirmPin) {
    changePinError.value = 'Konfirmasi PIN tidak cocok!'
    return
  }
  if (pin.length < 4 || pin.length > 6 || !/^\d+$/.test(pin)) {
    changePinError.value = 'PIN harus berupa 4-6 digit angka!'
    return
  }

  changePinSaving.value = true
  try {
    await api.post(`/users/${changePinTarget.value.id}/change-pin`, {
      new_pin: pin,
    })
    toast(`PIN untuk @${changePinTarget.value.username} berhasil diubah`, 'success')
    showChangePinModal.value = false
  } catch (e: any) {
    changePinError.value = e.response?.data?.message || e.message
  } finally {
    changePinSaving.value = false
  }
}

// ============================================================================
// ROLES & PERMISSION MATRIX
// ============================================================================
const selectedRole = ref<any | null>(null)
const rolePermissions = ref<any[]>([])
const loadingRolePerms = ref(false)
const savingRolePerms = ref(false)

const showRoleModal = ref(false)
const editingRole = ref<any | null>(null)
const roleForm = ref({ name: '', description: '' })
const roleSaving = ref(false)
const roleError = ref('')

async function selectRole(role: any) {
  selectedRole.value = role
  loadingRolePerms.value = true
  try {
    const res = await api.get(`/roles/${role.id}/permissions`)
    rolePermissions.value = res.data.data || []
  } catch (e) { console.error(e) }
  finally { loadingRolePerms.value = false }
}

function togglePermission(perm: any) {
  perm.has_permission = !perm.has_permission
}

async function saveRolePermissions() {
  if (!selectedRole.value) return
  savingRolePerms.value = true
  try {
    const ids = rolePermissions.value.filter(p => p.has_permission).map(p => p.id)
    await api.put(`/roles/${selectedRole.value.id}/permissions`, { permission_ids: ids })
    toast(`Hak akses "${selectedRole.value.name}" berhasil disimpan`, 'success')
    await fetchRoles()
    await authStore.fetchCurrentUser()
  } catch (e: any) {
    toast(e.response?.data?.message || e.message, 'error')
  } finally {
    savingRolePerms.value = false
  }
}

// Group permissions by module prefix
const groupedPermissions = computed(() => {
  const groups: Record<string, any[]> = {}
  rolePermissions.value.forEach(p => {
    const [module] = p.code.split(':')
    if (!groups[module]) groups[module] = []
    groups[module].push(p)
  })
  return groups
})

const permissionModuleIcons: Record<string, string> = {
  pos: 'fluent-color:receipt-24',
  shift: 'fluent-color:clock-24',
  inventory: 'streamline-ultimate-color:warehouse-storage-2',
  purchasing: 'streamline-ultimate-color:delivery-truck-cargo',
  accounting: 'streamline-ultimate-color:accounting-calculator-1',
  report: 'fluent-color:data-bar-vertical-ascending-24',
  master: 'fluent-color:database-24',
  users: 'fluent-color:lock-shield-24',
  settings: 'fluent-color:settings-24',
  audit: 'fluent-color:shield-checkmark-24',
}

const permissionModuleLabels: Record<string, string> = {
  pos: 'Kasir & Transaksi POS',
  shift: 'Manajemen Shift',
  inventory: 'Stok & Inventori',
  purchasing: 'Pembelian & Pengadaan',
  accounting: 'Akuntansi & Jurnal',
  report: 'Laporan & Analitik',
  master: 'Master Data',
  users: 'Pengguna & Hak Akses',
  settings: 'Pengaturan Sistem & Struk',
  audit: 'Jejak Audit Keamanan',
}

function openCreateRole() {
  editingRole.value = null
  roleForm.value = { name: '', description: '' }
  showRoleModal.value = true
}

function openEditRole(role: any) {
  editingRole.value = role
  roleForm.value = { name: role.name, description: role.description || '' }
  showRoleModal.value = true
}

async function saveRole() {
  roleError.value = ''
  roleSaving.value = true
  try {
    if (editingRole.value) {
      await api.put(`/roles/${editingRole.value.id}`, roleForm.value)
      toast('Role berhasil diperbarui', 'success')
    } else {
      await api.post("/roles", roleForm.value)
      toast('Role baru berhasil dibuat', 'success')
    }
    showRoleModal.value = false
    await fetchRoles()
  } catch (e: any) {
    roleError.value = e.response?.data?.message || e.message
  } finally {
    roleSaving.value = false
  }
}

async function deleteRole(role: any) {
  if (!confirm(`Hapus/nonaktifkan role "${role.name}"?`)) return
  try {
    await api.delete(`/roles/${role.id}`)
    toast('Role dinonaktifkan', 'info')
    if (selectedRole.value?.id === role.id) { selectedRole.value = null; rolePermissions.value = [] }
    await fetchRoles()
  } catch (e: any) {
    toast(e.response?.data?.message || e.message, 'error')
  }
}

// ============================================================================
// TOAST
// ============================================================================
const toasts = ref<{ id: number; msg: string; type: string }[]>([])
let toastId = 0

function toast(msg: string, type: 'success' | 'error' | 'info' = 'info') {
  const id = ++toastId
  toasts.value.push({ id, msg, type })
  setTimeout(() => { toasts.value = toasts.value.filter(t => t.id !== id) }, 3500)
}

// Role color helper
const roleColors: Record<string, { bg: string; text: string; border: string }> = {
  SUPERADMIN: { bg: 'bg-purple-100', text: 'text-purple-800', border: 'border-purple-400' },
  STORE_MANAGER: { bg: 'bg-blue-100', text: 'text-blue-800', border: 'border-blue-400' },
  SUPERVISOR: { bg: 'bg-amber-100', text: 'text-amber-800', border: 'border-amber-400' },
  CASHIER: { bg: 'bg-emerald-100', text: 'text-emerald-800', border: 'border-emerald-400' },
  WAREHOUSE: { bg: 'bg-orange-100', text: 'text-orange-800', border: 'border-orange-400' },
}

function getRoleColor(name: string) {
  return roleColors[name] || { bg: 'bg-slate-100', text: 'text-slate-800', border: 'border-slate-400' }
}

// ============================================================================
// INIT
// ============================================================================
onMounted(async () => {
  await Promise.all([fetchUsers(), fetchRoles(), fetchPermissions(), fetchOutlets()])
})
</script>

<template>
  <AdminLayout>
    <div class="h-full flex flex-col bg-slate-100 overflow-hidden font-sans">
      <!-- Toast notifications -->
      <div class="fixed top-4 right-4 z-[9999] flex flex-col gap-2 pointer-events-none">
        <TransitionGroup name="toast">
          <div
            v-for="t in toasts" :key="t.id"
            class="px-4 py-3 rounded-xl shadow-lg text-sm font-semibold pointer-events-auto flex items-center gap-2 border"
            :class="{
              'bg-emerald-50 text-emerald-800 border-emerald-300': t.type === 'success',
              'bg-rose-50 text-rose-800 border-rose-300': t.type === 'error',
              'bg-blue-50 text-blue-800 border-blue-300': t.type === 'info',
            }"
          >
            <Icon :icon="t.type === 'success' ? 'fluent-color:checkmark-circle-24' : t.type === 'error' ? 'fluent-color:dismiss-circle-24' : 'fluent-color:checkmark-circle-24'" class="w-5 h-5" />
            {{ t.msg }}
          </div>
        </TransitionGroup>
      </div>

      <!-- Page Header & Action Bar -->
      <div class="bg-white border-b border-slate-300 px-6 py-3.5 flex flex-wrap items-center justify-between gap-4 shadow-xs shrink-0">
        <div class="flex items-center gap-3">
          <!-- Tombol Back / Kembali ke Dashboard Hub -->
          <button
            type="button"
            @click="router.push('/admin')"
            class="px-3 py-2 rounded-lg bg-slate-50 hover:bg-slate-100 border border-slate-300 text-slate-700 hover:text-slate-900 transition shadow-xs flex items-center gap-1.5 text-xs font-bold cursor-pointer active:scale-95"
            title="Kembali ke Dashboard Hub"
          >
            <ArrowLeft class="w-4 h-4 text-slate-600" />
            <span>Kembali</span>
          </button>

          <div class="h-6 w-px bg-slate-300"></div>

          <div class="w-10 h-10 rounded-xl bg-purple-100 border border-purple-200 flex items-center justify-center text-purple-700 shadow-xs">
            <Icon icon="fluent-color:lock-shield-24" class="w-6 h-6" />
          </div>
          <div>
            <div class="flex items-center gap-2">
              <h1 class="text-base font-extrabold text-slate-900 tracking-tight">Manajemen Pengguna & Hak Akses</h1>
              <span class="text-[10px] bg-purple-50 text-purple-700 border border-purple-200 font-bold px-2 py-0.5 rounded-full flex items-center gap-1 font-mono">
                <Icon icon="fluent-color:shield-checkmark-24" class="w-3 h-3" />
                RBAC Enterprise
              </span>
            </div>
            <p class="text-xs text-slate-500 mt-0.5">
              Kelola pengguna sistem, peran (roles), dan matriks hak akses keamanan (RBAC).
            </p>
          </div>
        </div>

        <!-- Navigation Tabs -->
        <div class="flex items-center gap-1 bg-slate-100 p-1 rounded-xl border border-slate-300 shadow-inner">
          <button
            v-for="tab in [
              { key: 'users', label: 'Pengguna', icon: 'fluent-color:person-24', count: users.length },
              { key: 'roles', label: 'Roles & Permissions', icon: 'fluent-color:shield-checkmark-24', count: roles.length },
            ]"
            :key="tab.key"
            type="button"
            @click="activeTab = tab.key as Tab"
            class="flex items-center gap-2 px-4 py-1.5 text-xs font-bold rounded-lg transition-all cursor-pointer"
            :class="activeTab === tab.key
              ? 'bg-white text-indigo-700 shadow-xs border border-slate-200'
              : 'text-slate-600 hover:text-slate-900 hover:bg-slate-200/60 border border-transparent'"
          >
            <Icon :icon="tab.icon" class="w-4 h-4" />
            <span>{{ tab.label }}</span>
            <span
              class="px-1.5 py-0.5 rounded-full text-[10px] font-bold"
              :class="activeTab === tab.key ? 'bg-indigo-100 text-indigo-800' : 'bg-slate-200 text-slate-600'"
            >
              {{ tab.count }}
            </span>
          </button>
        </div>
      </div>

    <!-- ================================================================== -->
    <!-- TAB: USERS -->
    <!-- ================================================================== -->
    <div v-if="activeTab === 'users'" class="flex-1 flex flex-col overflow-hidden p-5 gap-4">
      <!-- Toolbar -->
      <div class="flex items-center gap-3 bg-white p-3 rounded-xl border border-slate-200 shadow-sm">
        <!-- Search -->
        <div class="relative flex-1 max-w-xs">
          <Icon icon="fluent-color:building-government-search-24" class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" />
          <input
            v-model="userSearch"
            type="text"
            placeholder="Cari nama, username, email..."
            class="w-full pl-9 pr-3 py-2 text-sm border border-slate-200 rounded-lg focus:outline-none focus:border-indigo-400 focus:ring-2 focus:ring-indigo-100"
          />
        </div>

        <!-- Filter Role -->
        <select v-model="userFilterRole"
          class="px-3 py-2 text-sm border border-slate-200 rounded-lg focus:outline-none focus:border-indigo-400 bg-white"
        >
          <option :value="null">Semua Role</option>
          <option v-for="r in roles" :key="r.id" :value="r.id">{{ r.name }}</option>
        </select>

        <!-- Filter Active -->
        <div class="flex rounded-lg border border-slate-200 overflow-hidden">
          <button v-for="opt in [{ v:'active', l:'Aktif' }, { v:'inactive', l:'Nonaktif' }, { v:'all', l:'Semua' }]"
            :key="opt.v"
            @click="userFilterActive = opt.v as any"
            class="px-3 py-2 text-xs font-semibold transition"
            :class="userFilterActive === opt.v
              ? 'bg-indigo-600 text-white'
              : 'bg-white text-slate-600 hover:bg-slate-50'"
          >{{ opt.l }}</button>
        </div>

        <div class="flex-1" />

        <button @click="openCreateUser"
          class="flex items-center gap-2 bg-indigo-600 hover:bg-indigo-700 text-white px-4 py-2 rounded-lg text-sm font-bold shadow-sm transition active:scale-95 cursor-pointer"
        >
          <Icon icon="fluent-color:person-add-24" class="w-4 h-4" />
          Tambah Pengguna
        </button>
      </div>

      <!-- Users Table -->
      <div class="flex-1 bg-white rounded-xl border border-slate-200 shadow-sm overflow-hidden flex flex-col">
        <div class="overflow-auto flex-1">
          <table class="w-full text-sm">
            <thead class="bg-slate-50 border-b border-slate-200 sticky top-0">
              <tr>
                <th class="text-left px-4 py-3 font-semibold text-slate-600 text-xs uppercase tracking-wide">Pengguna</th>
                <th class="text-left px-4 py-3 font-semibold text-slate-600 text-xs uppercase tracking-wide">Role</th>
                <th class="text-left px-4 py-3 font-semibold text-slate-600 text-xs uppercase tracking-wide">Akses Lokasi</th>
                <th class="text-left px-4 py-3 font-semibold text-slate-600 text-xs uppercase tracking-wide">Kontak</th>
                <th class="text-left px-4 py-3 font-semibold text-slate-600 text-xs uppercase tracking-wide">Status</th>
                <th class="text-left px-4 py-3 font-semibold text-slate-600 text-xs uppercase tracking-wide">Dibuat</th>
                <th class="text-right px-4 py-3 font-semibold text-slate-600 text-xs uppercase tracking-wide">Aksi</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-100">
              <tr v-if="loadingUsers">
                <td colspan="7" class="text-center py-16 text-slate-400">
                  <Icon icon="fluent-color:arrow-clockwise-dashes-24" class="w-8 h-8 mx-auto animate-spin" />
                  <div class="mt-2 text-sm">Memuat data pengguna...</div>
                </td>
              </tr>
              <tr v-else-if="filteredUsers.length === 0">
                <td colspan="7" class="text-center py-16 text-slate-400">
                  <Icon icon="fluent-color:person-available-24" class="w-12 h-12 mx-auto opacity-50" />
                  <div class="mt-2 text-sm font-medium">Tidak ada pengguna ditemukan</div>
                </td>
              </tr>
              <tr
                v-for="user in filteredUsers"
                :key="user.id"
                class="hover:bg-slate-50 transition-colors"
                :class="!user.is_active ? 'opacity-60' : ''"
              >
                <td class="px-4 py-3">
                  <div class="flex items-center gap-3">
                    <div class="w-9 h-9 rounded-xl flex items-center justify-center text-white font-black text-sm shadow-sm"
                      :class="user.is_active ? 'bg-gradient-to-br from-indigo-500 to-purple-600' : 'bg-slate-400'"
                    >
                      {{ user.full_name.charAt(0) }}
                    </div>
                    <div>
                      <div class="font-bold text-slate-900">{{ user.full_name }}</div>
                      <div class="text-xs text-slate-400 font-mono">@{{ user.username }}</div>
                    </div>
                  </div>
                </td>
                <td class="px-4 py-3">
                  <span class="inline-flex items-center px-2 py-1 rounded-lg text-xs font-bold border"
                    :class="[getRoleColor(user.role_name).bg, getRoleColor(user.role_name).text, getRoleColor(user.role_name).border]"
                  >
                    {{ user.role_name }}
                  </span>
                </td>
                <td class="px-4 py-3">
                  <div v-if="user.role_name === 'SUPERADMIN'">
                    <span class="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] font-bold bg-purple-50 text-purple-700 border border-purple-200">
                      <Icon icon="fluent-color:shield-keyhole-24" class="w-3.5 h-3.5" />
                      Semua Lokasi
                    </span>
                  </div>
                  <div v-else-if="user.outlets && user.outlets.length > 0" class="flex flex-wrap gap-1 max-w-xs">
                    <span
                      v-for="o in user.outlets"
                      :key="o.id"
                      class="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[11px] font-medium border"
                      :class="o.is_warehouse ? 'bg-amber-50 text-amber-800 border-amber-200' : 'bg-blue-50 text-blue-800 border-blue-200'"
                      :title="`${o.code} - ${o.name} (${o.is_warehouse ? 'Gudang' : 'Toko'})`"
                    >
                      <Icon :icon="o.is_warehouse ? 'fluent-color:building-retail-shield-24' : 'fluent-color:building-shop-24'" class="w-3 h-3" />
                      {{ o.name }}
                    </span>
                  </div>
                  <div v-else>
                    <span class="inline-flex items-center px-2 py-0.5 rounded text-[11px] text-rose-500 bg-rose-50 border border-rose-200 italic">
                      Tidak ada akses
                    </span>
                  </div>
                </td>
                <td class="px-4 py-3">
                  <div class="text-slate-700 text-xs">{{ user.email || '-' }}</div>
                  <div class="text-slate-400 text-xs font-mono">{{ user.phone || '-' }}</div>
                </td>
                <td class="px-4 py-3">
                  <span class="inline-flex items-center gap-1 px-2 py-1 rounded-full text-xs font-bold"
                    :class="user.is_active ? 'bg-emerald-100 text-emerald-700' : 'bg-rose-100 text-rose-700'"
                  >
                    <span class="w-1.5 h-1.5 rounded-full" :class="user.is_active ? 'bg-emerald-500' : 'bg-rose-500'" />
                    {{ user.is_active ? 'Aktif' : 'Nonaktif' }}
                  </span>
                </td>
                <td class="px-4 py-3 text-xs text-slate-400">
                  {{ new Date(user.created_at).toLocaleDateString('id-ID', { day: '2-digit', month: 'short', year: 'numeric' }) }}
                </td>
                <td class="px-4 py-3">
                  <div class="flex items-center justify-end gap-1">
                    <button @click="openEditUser(user)" title="Edit Data" class="p-1.5 rounded-lg hover:bg-indigo-50 text-indigo-600 transition cursor-pointer">
                      <Icon icon="fluent-color:edit-24" class="w-4 h-4" />
                    </button>
                    <button @click="openResetPwd(user)" title="Ganti / Reset Password" class="p-1.5 rounded-lg hover:bg-amber-50 text-amber-600 transition cursor-pointer">
                      <Icon icon="fluent-color:person-key-24" class="w-4 h-4" />
                    </button>
                    <button
                      v-if="canChangePin"
                      @click="openChangePin(user)"
                      title="Ganti PIN Otorisasi"
                      class="p-1.5 rounded-lg hover:bg-purple-50 text-purple-600 transition cursor-pointer"
                    >
                      <Icon icon="streamline-ultimate-color:dial-pad" class="w-4 h-4" />
                    </button>
                    <button @click="toggleUserActive(user)" :title="user.is_active ? 'Nonaktifkan' : 'Aktifkan kembali'"
                      class="p-1.5 rounded-lg transition cursor-pointer"
                      :class="user.is_active ? 'hover:bg-rose-50 text-rose-500' : 'hover:bg-emerald-50 text-emerald-600'"
                    >
                      <Icon :icon="user.is_active ? 'fluent-color:person-available-24' : 'fluent-color:person-available-24'" class="w-4 h-4" />
                    </button>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
        <!-- Footer -->
        <div class="border-t border-slate-100 px-4 py-2 bg-slate-50 text-xs text-slate-500 flex items-center justify-between">
          <span>Menampilkan <strong>{{ filteredUsers.length }}</strong> dari <strong>{{ users.length }}</strong> pengguna</span>
          <span>{{ users.filter(u => u.is_active).length }} aktif · {{ users.filter(u => !u.is_active).length }} nonaktif</span>
        </div>
      </div>
    </div>

    <!-- ================================================================== -->
    <!-- TAB: ROLES & PERMISSIONS -->
    <!-- ================================================================== -->
    <div v-if="activeTab === 'roles'" class="flex-1 flex overflow-hidden p-5 gap-4">
      <!-- Left: Role List -->
      <div class="w-72 flex flex-col gap-3 shrink-0">
        <div class="flex items-center justify-between">
          <h2 class="font-bold text-slate-700 text-sm">Daftar Role</h2>
          <button @click="openCreateRole"
            class="flex items-center gap-1 bg-indigo-600 hover:bg-indigo-700 text-white px-2.5 py-1.5 rounded-lg text-xs font-bold transition active:scale-95 cursor-pointer"
          >
            <Icon icon="fluent-color:add-circle-24" class="w-3.5 h-3.5" />
            Buat Role
          </button>
        </div>

        <div class="flex flex-col gap-2 overflow-y-auto">
          <div
            v-for="role in roles"
            :key="role.id"
            @click="selectRole(role)"
            class="bg-white rounded-xl border-2 p-3 cursor-pointer transition-all hover:shadow-md"
            :class="selectedRole?.id === role.id
              ? 'border-indigo-500 shadow-md ring-2 ring-indigo-200'
              : 'border-slate-200 hover:border-indigo-300'"
          >
            <div class="flex items-start justify-between gap-2">
              <div class="flex-1">
                <div class="flex items-center gap-2">
                  <span class="font-bold text-sm text-slate-900">{{ role.name }}</span>
                  <span v-if="!role.is_active" class="text-[9px] bg-rose-100 text-rose-700 px-1 rounded font-bold">NONAKTIF</span>
                </div>
                <p class="text-xs text-slate-500 mt-0.5 line-clamp-2">{{ role.description || 'Tidak ada deskripsi' }}</p>
                <div class="flex items-center gap-3 mt-2 text-xs text-slate-400">
                  <span class="flex items-center gap-1">
                    <Icon icon="fluent-color:person-24" class="w-3.5 h-3.5" />
                    {{ role.user_count }} pengguna
                  </span>
                  <span class="flex items-center gap-1">
                    <Icon icon="fluent-color:person-key-24" class="w-3.5 h-3.5" />
                    {{ role.permission_count }} hak akses
                  </span>
                </div>
              </div>
              <div class="flex flex-col gap-1">
                <button @click.stop="openEditRole(role)" class="p-1 rounded hover:bg-indigo-50 text-indigo-500 transition cursor-pointer">
                  <Icon icon="fluent-color:edit-24" class="w-3.5 h-3.5" />
                </button>
                <button @click.stop="deleteRole(role)" class="p-1 rounded hover:bg-rose-50 text-rose-400 transition cursor-pointer" :disabled="role.name === 'SUPERADMIN'">
                  <Icon icon="fluent-color:wrench-24" class="w-3.5 h-3.5" />
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Right: Permission Matrix -->
      <div class="flex-1 bg-white rounded-xl border border-slate-200 shadow-sm overflow-hidden flex flex-col">
        <div v-if="!selectedRole" class="flex-1 flex flex-col items-center justify-center text-slate-400 gap-3">
          <Icon icon="fluent-color:shield-checkmark-24" class="w-16 h-16 opacity-40" />
          <div class="text-sm font-medium">Pilih Role di sebelah kiri untuk mengatur hak akses</div>
        </div>

        <template v-else>
          <div class="border-b border-slate-100 px-5 py-3 flex items-center justify-between">
            <div>
              <h2 class="font-extrabold text-slate-900 flex items-center gap-2">
                <Icon icon="fluent-color:shield-checkmark-24" class="w-5 h-5" />
                Hak Akses: {{ selectedRole.name }}
              </h2>
              <p class="text-xs text-slate-500 mt-0.5">{{ selectedRole.description }}</p>
            </div>
            <div class="flex items-center gap-2">
              <span class="text-xs text-slate-500">
                {{ rolePermissions.filter(p => p.has_permission).length }} / {{ rolePermissions.length }} aktif
              </span>
              <button
                @click="saveRolePermissions"
                :disabled="savingRolePerms || selectedRole.name === 'SUPERADMIN'"
                class="flex items-center gap-2 bg-emerald-600 hover:bg-emerald-700 disabled:opacity-50 text-white px-4 py-2 rounded-lg text-sm font-bold transition active:scale-95 cursor-pointer"
              >
                <Icon icon="fluent-color:arrow-clockwise-dashes-24" class="w-4 h-4" />
                {{ savingRolePerms ? 'Menyimpan...' : 'Simpan Perubahan' }}
              </button>
            </div>
          </div>

          <div v-if="selectedRole.name === 'SUPERADMIN'" class="flex-1 flex items-center justify-center">
            <div class="text-center text-slate-500 p-8">
              <Icon icon="fluent-color:star-24" class="w-16 h-16 mx-auto mb-3" />
              <h3 class="font-bold text-slate-700">SUPERADMIN memiliki SEMUA hak akses</h3>
              <p class="text-sm mt-1">Role ini tidak dapat dimodifikasi hak aksesnya. Seluruh fitur sistem selalu tersedia.</p>
            </div>
          </div>

          <div v-else-if="loadingRolePerms" class="flex-1 flex items-center justify-center">
            <Icon icon="fluent-color:arrow-clockwise-dashes-24" class="w-8 h-8 animate-spin text-indigo-500" />
          </div>

          <div v-else class="flex-1 overflow-y-auto p-5">
            <div class="grid grid-cols-1 gap-4">
              <div v-for="(perms, module) in groupedPermissions" :key="module" class="bg-slate-50 rounded-xl border border-slate-200 overflow-hidden">
                <!-- Module Header -->
                <div class="flex items-center gap-3 px-4 py-3 bg-slate-100 border-b border-slate-200">
                  <Icon :icon="permissionModuleIcons[module] || 'fluent-color:settings-24'" class="w-5 h-5" />
                  <h3 class="font-bold text-slate-700 text-sm">{{ permissionModuleLabels[module] || module.toUpperCase() }}</h3>
                  <span class="ml-auto text-xs text-slate-400">{{ perms.filter(p => p.has_permission).length }}/{{ perms.length }} aktif</span>
                </div>
                <!-- Permissions -->
                <div class="p-3 grid grid-cols-1 gap-2">
                  <label
                    v-for="perm in perms"
                    :key="perm.id"
                    class="flex items-start gap-3 p-3 rounded-lg border-2 cursor-pointer transition-all"
                    :class="perm.has_permission
                      ? 'bg-indigo-50 border-indigo-300 shadow-sm'
                      : 'bg-white border-slate-200 hover:border-indigo-200'"
                  >
                    <div class="relative mt-0.5">
                      <input
                        type="checkbox"
                        :checked="perm.has_permission"
                        @change="togglePermission(perm)"
                        class="sr-only"
                      />
                      <div class="w-5 h-5 rounded-md border-2 flex items-center justify-center transition-all"
                        :class="perm.has_permission ? 'bg-indigo-600 border-indigo-600' : 'border-slate-300 bg-white'"
                      >
                        <Icon v-if="perm.has_permission" icon="fluent-color:checkmark-circle-24" class="w-3 h-3 text-white" />
                      </div>
                    </div>
                    <div class="flex-1 min-w-0">
                      <div class="font-bold text-xs font-mono" :class="perm.has_permission ? 'text-indigo-700' : 'text-slate-500'">
                        {{ perm.code }}
                      </div>
                      <div class="text-xs text-slate-600 mt-0.5">{{ perm.description }}</div>
                    </div>
                  </label>
                </div>
              </div>
            </div>
          </div>
        </template>
      </div>
    </div>

    <!-- ================================================================== -->
    <!-- MODAL: CREATE/EDIT USER -->
    <!-- ================================================================== -->
    <Teleport to="body">
      <Transition name="modal">
        <div v-if="showUserModal" class="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center p-4">
          <div class="bg-white rounded-2xl shadow-2xl w-full max-w-lg">
            <!-- Header -->
            <div class="flex items-center gap-3 p-5 border-b border-slate-100">
              <div class="w-10 h-10 rounded-xl bg-gradient-to-br from-indigo-500 to-purple-600 flex items-center justify-center">
                <Icon :icon="editingUser ? 'fluent-color:edit-24' : 'fluent-color:person-add-24'" class="w-5 h-5" />
              </div>
              <div>
                <h3 class="font-extrabold text-slate-900">{{ editingUser ? 'Edit Pengguna' : 'Tambah Pengguna Baru' }}</h3>
                <p class="text-xs text-slate-500">{{ editingUser ? `Mengubah data: ${editingUser.full_name}` : 'Buat akun pengguna baru ke sistem' }}</p>
              </div>
              <button @click="showUserModal = false" class="ml-auto p-2 hover:bg-slate-100 rounded-lg transition cursor-pointer">
                <Icon icon="fluent-color:cloud-dismiss-24" class="w-4 h-4" />
              </button>
            </div>

            <div class="p-5 space-y-4 max-h-[60vh] overflow-y-auto">
              <div v-if="userError" class="flex items-center gap-2 bg-rose-50 text-rose-700 text-sm p-3 rounded-lg border border-rose-200">
                <Icon icon="fluent-color:error-circle-24" class="w-4 h-4 shrink-0" />
                {{ userError }}
              </div>

              <div class="grid grid-cols-2 gap-4">
                <div class="col-span-2">
                  <label class="block text-xs font-bold text-slate-600 mb-1">Nama Lengkap <span class="text-rose-500">*</span></label>
                  <input v-model="userForm.full_name" type="text" placeholder="Contoh: Budi Santoso"
                    class="w-full px-3 py-2 border border-slate-200 rounded-lg text-sm focus:outline-none focus:border-indigo-400 focus:ring-2 focus:ring-indigo-100" />
                </div>
                <div :class="editingUser ? 'col-span-2' : 'col-span-1'">
                  <label class="block text-xs font-bold text-slate-600 mb-1">Username <span class="text-rose-500">*</span></label>
                  <input v-model="userForm.username" type="text" placeholder="budi.santoso" :disabled="!!editingUser"
                    class="w-full px-3 py-2 border border-slate-200 rounded-lg text-sm focus:outline-none focus:border-indigo-400 focus:ring-2 focus:ring-indigo-100 disabled:bg-slate-100 disabled:text-slate-400" />
                </div>
                <div v-if="!editingUser">
                  <label class="block text-xs font-bold text-slate-600 mb-1">Role <span class="text-rose-500">*</span></label>
                  <select v-model="userForm.role_id"
                    class="w-full px-3 py-2 border border-slate-200 rounded-lg text-sm focus:outline-none focus:border-indigo-400 bg-white">
                    <option v-for="r in roles" :key="r.id" :value="r.id">{{ r.name }}</option>
                  </select>
                </div>
                <div v-if="editingUser" class="col-span-2">
                  <label class="block text-xs font-bold text-slate-600 mb-1">Role <span class="text-rose-500">*</span></label>
                  <select v-model="userForm.role_id"
                    class="w-full px-3 py-2 border border-slate-200 rounded-lg text-sm focus:outline-none focus:border-indigo-400 bg-white">
                    <option v-for="r in roles" :key="r.id" :value="r.id">{{ r.name }}</option>
                  </select>
                </div>
              </div>

              <!-- Password fields: only for new user -->
              <template v-if="!editingUser">
                <div class="border-t border-slate-100 pt-3">
                  <p class="text-xs font-bold text-slate-500 mb-3 uppercase tracking-wide">Kredensial Akses</p>
                  <div class="grid grid-cols-2 gap-4">
                    <div>
                      <label class="block text-xs font-bold text-slate-600 mb-1">Password <span class="text-rose-500">*</span></label>
                      <input v-model="userForm.password" type="password" placeholder="Min 6 karakter"
                        class="w-full px-3 py-2 border border-slate-200 rounded-lg text-sm focus:outline-none focus:border-indigo-400 focus:ring-2 focus:ring-indigo-100" />
                    </div>
                    <div>
                      <label class="block text-xs font-bold text-slate-600 mb-1">Konfirmasi Password <span class="text-rose-500">*</span></label>
                      <input v-model="userForm.confirm_password" type="password" placeholder="Ulangi password"
                        class="w-full px-3 py-2 border border-slate-200 rounded-lg text-sm focus:outline-none focus:border-indigo-400 focus:ring-2 focus:ring-indigo-100" />
                    </div>
                    <div class="col-span-2">
                      <label class="block text-xs font-bold text-slate-600 mb-1">PIN Supervisor (4-6 digit) <span class="text-rose-500">*</span></label>
                      <input v-model="userForm.pin" type="password" placeholder="PIN numerik untuk otorisasi supervisor" maxlength="6"
                        class="w-full px-3 py-2 border border-slate-200 rounded-lg text-sm focus:outline-none focus:border-indigo-400 focus:ring-2 focus:ring-indigo-100" />
                    </div>
                  </div>
                </div>
              </template>

              <!-- Contact info -->
              <div class="border-t border-slate-100 pt-3">
                <p class="text-xs font-bold text-slate-500 mb-3 uppercase tracking-wide">Informasi Kontak (Opsional)</p>
                <div class="grid grid-cols-2 gap-4">
                  <div>
                    <label class="block text-xs font-bold text-slate-600 mb-1">Email</label>
                    <input v-model="userForm.email" type="email" placeholder="email@example.com"
                      class="w-full px-3 py-2 border border-slate-200 rounded-lg text-sm focus:outline-none focus:border-indigo-400 focus:ring-2 focus:ring-indigo-100" />
                  </div>
                  <div>
                    <label class="block text-xs font-bold text-slate-600 mb-1">No. HP</label>
                    <input v-model="userForm.phone" type="tel" placeholder="08xxxxxxxxxx"
                      class="w-full px-3 py-2 border border-slate-200 rounded-lg text-sm focus:outline-none focus:border-indigo-400 focus:ring-2 focus:ring-indigo-100" />
                  </div>
                </div>
              </div>

              <!-- Outlet Access Section -->
              <div class="border-t border-slate-100 pt-3">
                <div class="flex items-center justify-between mb-2">
                  <div>
                    <p class="text-xs font-bold text-slate-700 uppercase tracking-wide flex items-center gap-1.5">
                      <Icon icon="fluent-color:building-shop-24" class="w-4 h-4" />
                      Hak Akses Outlet & Lokasi
                    </p>
                    <p class="text-[11px] text-slate-500">Tentukan outlet atau gudang yang diizinkan untuk dikelola/dibuka oleh akun ini.</p>
                  </div>
                  <div v-if="!isSelectedRoleSuperadmin" class="flex items-center gap-2">
                    <button
                      type="button"
                      @click="selectAllUserOutlets"
                      class="text-[11px] font-bold text-indigo-600 hover:text-indigo-800 transition cursor-pointer"
                    >
                      Pilih Semua
                    </button>
                    <span class="text-slate-300">|</span>
                    <button
                      type="button"
                      @click="clearAllUserOutlets"
                      class="text-[11px] font-bold text-rose-500 hover:text-rose-700 transition cursor-pointer"
                    >
                      Kosongkan
                    </button>
                  </div>
                </div>

                <!-- Info if role is SUPERADMIN -->
                <div v-if="isSelectedRoleSuperadmin" class="p-3 bg-purple-50 border border-purple-200 rounded-xl text-xs text-purple-800 flex items-start gap-2.5">
                  <Icon icon="fluent-color:shield-keyhole-24" class="w-5 h-5 shrink-0 mt-0.5" />
                  <div>
                    <strong class="font-bold">Akses Penuh Seluruh Cabang:</strong>
                    <div class="text-[11px] text-purple-700 mt-0.5">
                      Pengguna dengan role SUPERADMIN secara otomatis memiliki hak akses penuh ke seluruh outlet cabang kasir dan gudang inventaris di seluruh sistem.
                    </div>
                  </div>
                </div>

                <!-- Outlets checklist grid -->
                <div v-else class="space-y-2 mt-2">
                  <div v-if="allOutlets.length === 0" class="text-xs text-slate-400 italic py-2">
                    Belum ada outlet terdaftar di sistem.
                  </div>
                  <div v-else class="grid grid-cols-1 sm:grid-cols-2 gap-2 max-h-48 overflow-y-auto pr-1">
                    <div
                      v-for="outlet in allOutlets"
                      :key="outlet.id"
                      @click="toggleUserOutlet(outlet.id)"
                      class="flex items-center gap-2.5 p-2.5 rounded-xl border transition cursor-pointer select-none"
                      :class="userForm.outlet_ids.includes(outlet.id)
                        ? 'bg-indigo-50/70 border-indigo-300 shadow-xs'
                        : 'bg-white border-slate-200 hover:bg-slate-50'"
                    >
                      <input
                        type="checkbox"
                        :checked="userForm.outlet_ids.includes(outlet.id)"
                        class="w-4 h-4 text-indigo-600 rounded border-slate-300 focus:ring-indigo-400 cursor-pointer pointer-events-none"
                      />
                      <div class="flex-1 min-w-0">
                        <div class="text-xs font-bold text-slate-900 truncate flex items-center gap-1.5">
                          <span>{{ outlet.name }}</span>
                          <span class="text-[10px] font-mono font-normal text-slate-500">({{ outlet.code }})</span>
                        </div>
                        <div class="text-[10px] flex items-center gap-1 mt-0.5" :class="outlet.is_warehouse ? 'text-amber-700' : 'text-blue-600'">
                          <span class="w-1.5 h-1.5 rounded-full" :class="outlet.is_warehouse ? 'bg-amber-500' : 'bg-blue-500'"></span>
                          {{ outlet.is_warehouse ? 'Gudang Pusat / Suplai' : 'Toko Retail & POS' }}
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <div class="p-5 border-t border-slate-100 flex justify-end gap-2">
              <button @click="showUserModal = false" class="px-4 py-2 text-sm font-semibold text-slate-600 hover:bg-slate-100 rounded-lg transition cursor-pointer">
                Batal
              </button>
              <button @click="saveUser" :disabled="userSaving"
                class="flex items-center gap-2 bg-indigo-600 hover:bg-indigo-700 disabled:opacity-50 text-white px-5 py-2 rounded-lg text-sm font-bold shadow-sm transition cursor-pointer"
              >
                <Icon icon="fluent-color:arrow-clockwise-dashes-24" class="w-4 h-4" />
                {{ userSaving ? 'Menyimpan...' : (editingUser ? 'Simpan Perubahan' : 'Buat Pengguna') }}
              </button>
            </div>
          </div>
        </div>
      </Transition>

      <!-- MODAL: RESET PASSWORD -->
      <Transition name="modal">
        <div v-if="showResetPwdModal" class="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center p-4">
          <div class="bg-white rounded-2xl shadow-2xl w-full max-w-md">
            <div class="flex items-center gap-3 p-5 border-b border-slate-100">
              <div class="w-10 h-10 rounded-xl bg-gradient-to-br from-amber-500 to-orange-600 flex items-center justify-center">
                <Icon icon="fluent-color:person-key-24" class="w-5 h-5" />
              </div>
              <div>
                <h3 class="font-extrabold text-slate-900">Reset Password</h3>
                <p class="text-xs text-slate-500">{{ resetTarget?.full_name }} (@{{ resetTarget?.username }})</p>
              </div>
              <button @click="showResetPwdModal = false" class="ml-auto p-2 hover:bg-slate-100 rounded-lg transition cursor-pointer">
                <Icon icon="fluent-color:cloud-dismiss-24" class="w-4 h-4" />
              </button>
            </div>
            <div class="p-5 space-y-4">
              <div v-if="resetError" class="flex items-center gap-2 bg-rose-50 text-rose-700 text-sm p-3 rounded-lg border border-rose-200">
                <Icon icon="fluent-color:error-circle-24" class="w-4 h-4 shrink-0" />
                {{ resetError }}
              </div>
              <div>
                <label class="block text-xs font-bold text-slate-600 mb-1">Password Baru <span class="text-rose-500">*</span></label>
                <input v-model="resetForm.new_password" type="password" placeholder="Min 6 karakter"
                  class="w-full px-3 py-2 border border-slate-200 rounded-lg text-sm focus:outline-none focus:border-amber-400 focus:ring-2 focus:ring-amber-100" />
              </div>
              <div>
                <label class="block text-xs font-bold text-slate-600 mb-1">Konfirmasi Password Baru <span class="text-rose-500">*</span></label>
                <input v-model="resetForm.confirm_password" type="password" placeholder="Ulangi password baru"
                  class="w-full px-3 py-2 border border-slate-200 rounded-lg text-sm focus:outline-none focus:border-amber-400 focus:ring-2 focus:ring-amber-100" />
              </div>
            </div>
            <div class="p-5 border-t border-slate-100 flex justify-end gap-2">
              <button @click="showResetPwdModal = false" class="px-4 py-2 text-sm font-semibold text-slate-600 hover:bg-slate-100 rounded-lg transition cursor-pointer">Batal</button>
              <button @click="doResetPassword" :disabled="resetSaving"
                class="flex items-center gap-2 bg-amber-600 hover:bg-amber-700 disabled:opacity-50 text-white px-5 py-2 rounded-lg text-sm font-bold shadow-sm transition cursor-pointer"
              >
                <Icon icon="fluent-color:person-key-24" class="w-4 h-4" />
                {{ resetSaving ? 'Menyimpan...' : 'Simpan Password Baru' }}
              </button>
            </div>
          </div>
        </div>
      </Transition>

      <!-- MODAL: GANTI PIN PENGGUNA -->
      <Transition name="modal">
        <div v-if="showChangePinModal" class="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center p-4">
          <div class="bg-white rounded-2xl shadow-2xl w-full max-w-md">
            <div class="flex items-center gap-3 p-5 border-b border-slate-100">
              <div class="w-10 h-10 rounded-xl bg-purple-100 border border-purple-200 flex items-center justify-center shadow-xs">
                <Icon icon="streamline-ultimate-color:dial-pad" class="w-6 h-6" />
              </div>
              <div>
                <h3 class="font-extrabold text-slate-900">Ubah PIN Otorisasi Pengguna</h3>
                <p class="text-xs text-slate-500">{{ changePinTarget?.full_name }} (@{{ changePinTarget?.username }})</p>
              </div>
              <button @click="showChangePinModal = false" class="ml-auto p-2 hover:bg-slate-100 rounded-lg transition cursor-pointer">
                <Icon icon="fluent-color:cloud-dismiss-24" class="w-4 h-4" />
              </button>
            </div>
            <div class="p-5 space-y-4">
              <div v-if="changePinError" class="flex items-center gap-2 bg-rose-50 text-rose-700 text-sm p-3 rounded-lg border border-rose-200">
                <Icon icon="fluent-color:error-circle-24" class="w-4 h-4 shrink-0" />
                {{ changePinError }}
              </div>
              <div>
                <label class="block text-xs font-bold text-slate-600 mb-1">PIN Baru (4-6 Digit Angka) <span class="text-rose-500">*</span></label>
                <input
                  v-model="changePinForm.new_pin"
                  type="password"
                  maxlength="6"
                  placeholder="Contoh: 123456"
                  class="w-full px-3 py-2 border border-slate-200 rounded-lg text-sm font-mono tracking-widest focus:outline-none focus:border-purple-400 focus:ring-2 focus:ring-purple-100"
                />
                <p class="text-[11px] text-slate-400 mt-1">Digunakan untuk otorisasi supervisor di POS (Void, Diskon, Buka Laci).</p>
              </div>
              <div>
                <label class="block text-xs font-bold text-slate-600 mb-1">Konfirmasi PIN Baru <span class="text-rose-500">*</span></label>
                <input
                  v-model="changePinForm.confirm_pin"
                  type="password"
                  maxlength="6"
                  placeholder="Ulangi PIN baru"
                  class="w-full px-3 py-2 border border-slate-200 rounded-lg text-sm font-mono tracking-widest focus:outline-none focus:border-purple-400 focus:ring-2 focus:ring-purple-100"
                />
              </div>
            </div>
            <div class="p-5 border-t border-slate-100 flex justify-end gap-2">
              <button @click="showChangePinModal = false" class="px-4 py-2 text-sm font-semibold text-slate-600 hover:bg-slate-100 rounded-lg transition cursor-pointer">Batal</button>
              <button @click="doChangePin" :disabled="changePinSaving"
                class="flex items-center gap-2 bg-purple-600 hover:bg-purple-700 disabled:opacity-50 text-white px-5 py-2 rounded-lg text-sm font-bold shadow-sm transition cursor-pointer"
              >
                <Icon icon="fluent-color:arrow-clockwise-dashes-24" class="w-4 h-4" />
                {{ changePinSaving ? 'Menyimpan...' : 'Simpan PIN Baru' }}
              </button>
            </div>
          </div>
        </div>
      </Transition>

      <!-- MODAL: CREATE/EDIT ROLE -->
      <Transition name="modal">
        <div v-if="showRoleModal" class="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 flex items-center justify-center p-4">
          <div class="bg-white rounded-2xl shadow-2xl w-full max-w-md">
            <div class="flex items-center gap-3 p-5 border-b border-slate-100">
              <div class="w-10 h-10 rounded-xl bg-gradient-to-br from-violet-500 to-indigo-600 flex items-center justify-center">
                <Icon icon="fluent-color:shield-checkmark-24" class="w-5 h-5" />
              </div>
              <div>
                <h3 class="font-extrabold text-slate-900">{{ editingRole ? 'Edit Role' : 'Buat Role Baru' }}</h3>
                <p class="text-xs text-slate-500">{{ editingRole ? `Mengubah role: ${editingRole.name}` : 'Tambahkan role pengguna baru' }}</p>
              </div>
              <button @click="showRoleModal = false" class="ml-auto p-2 hover:bg-slate-100 rounded-lg transition cursor-pointer">
                <Icon icon="fluent-color:cloud-dismiss-24" class="w-4 h-4" />
              </button>
            </div>
            <div class="p-5 space-y-4">
              <div v-if="roleError" class="flex items-center gap-2 bg-rose-50 text-rose-700 text-sm p-3 rounded-lg border border-rose-200">
                <Icon icon="fluent-color:error-circle-24" class="w-4 h-4 shrink-0" />
                {{ roleError }}
              </div>
              <div>
                <label class="block text-xs font-bold text-slate-600 mb-1">Nama Role <span class="text-rose-500">*</span></label>
                <input v-model="roleForm.name" type="text" placeholder="Contoh: BRANCH_MANAGER"
                  class="w-full px-3 py-2 border border-slate-200 rounded-lg text-sm focus:outline-none focus:border-violet-400 focus:ring-2 focus:ring-violet-100 uppercase" />
                <p class="text-[10px] text-slate-400 mt-1">Nama role akan diubah menjadi huruf kapital secara otomatis</p>
              </div>
              <div>
                <label class="block text-xs font-bold text-slate-600 mb-1">Deskripsi</label>
                <textarea v-model="roleForm.description" rows="3" placeholder="Jelaskan fungsi dan tanggung jawab role ini..."
                  class="w-full px-3 py-2 border border-slate-200 rounded-lg text-sm focus:outline-none focus:border-violet-400 focus:ring-2 focus:ring-violet-100 resize-none" />
              </div>
            </div>
            <div class="p-5 border-t border-slate-100 flex justify-end gap-2">
              <button @click="showRoleModal = false" class="px-4 py-2 text-sm font-semibold text-slate-600 hover:bg-slate-100 rounded-lg transition cursor-pointer">Batal</button>
              <button @click="saveRole" :disabled="roleSaving"
                class="flex items-center gap-2 bg-violet-600 hover:bg-violet-700 disabled:opacity-50 text-white px-5 py-2 rounded-lg text-sm font-bold shadow-sm transition cursor-pointer"
              >
                <Icon icon="fluent-color:arrow-clockwise-dashes-24" class="w-4 h-4" />
                {{ roleSaving ? 'Menyimpan...' : (editingRole ? 'Simpan' : 'Buat Role') }}
              </button>
            </div>
          </div>
        </div>
      </Transition>
    </Teleport>
    </div>
  </AdminLayout>
</template>

<style scoped>
.toast-enter-active, .toast-leave-active { transition: all 0.3s ease; }
.toast-enter-from { opacity: 0; transform: translateY(-10px) scale(0.95); }
.toast-leave-to { opacity: 0; transform: translateX(20px); }

.modal-enter-active, .modal-leave-active { transition: all 0.2s ease; }
.modal-enter-from, .modal-leave-to { opacity: 0; transform: scale(0.95); }
</style>
