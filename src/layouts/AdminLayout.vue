<script setup lang="ts">
import { useRouter, useRoute } from 'vue-router'
import { LogOut } from 'lucide-vue-next'
import { useAuthStore } from '@/stores/authStore'
import { useNetworkStore } from '@/stores/networkStore'
import NetworkConnectionModal from '@/components/common/NetworkConnectionModal.vue'
import UserProfileSecurityModal from '@/components/common/UserProfileSecurityModal.vue'
import { useLocationStore } from '@/stores/locationStore'
import { ref, computed, onMounted } from 'vue'

const router = useRouter()
const route = useRoute()
const authStore = useAuthStore()
const networkStore = useNetworkStore()
const locationStore = useLocationStore()
const showNetworkModal = ref(false)
const showSecurityModal = ref(false)
const showLocationDropdown = ref(false)

// 1. Grup Menu Operasional & Master Data Bisnis
const rawOperationalMenus = [
  { path: '/admin', name: 'Dashboard Hub', icon: 'fluent-color:building-store-24' },
  { path: '/admin/outlets', name: 'Master Outlet', icon: 'fluent-color:building-shop-24', permission: ['outlets:read', 'master:read'] },
  { path: '/admin/sales', name: 'Penjualan', icon: 'fluent-color:receipt-24', permission: 'report:view' },
  { path: '/admin/purchases', name: 'Pembelian', icon: 'streamline-ultimate-color:delivery-truck-cargo', permission: 'purchasing:read' },
  { path: '/admin/products', name: 'Master Produk', icon: 'streamline-ultimate-color:warehouse-cart-packages-2', permission: 'master:read' },
  { path: '/admin/customers', name: 'Pelanggan & Member', icon: 'fluent-color:people-team-24', permission: 'master:read' },
  { path: '/admin/vouchers', name: 'Voucher & Promo', icon: 'fluent-color:gift-card-24', permission: 'master:read' },
  { path: '/admin/inventory', name: 'Stok & Opname', icon: 'streamline-ultimate-color:warehouse-storage-2', permission: 'inventory:read' },
  { path: '/admin/accounting', name: 'Jurnal & Laba Rugi', icon: 'streamline-ultimate-color:accounting-calculator-1', permission: 'accounting:read' },
]

const operationalMenus = computed(() => {
  return rawOperationalMenus.filter(m => !m.permission || authStore.hasPermission(m.permission))
})

// 2. Grup Menu Pengaturan & Keamanan
const rawConfigMenus = [
  { path: '/admin/users', name: 'Pengguna & Hak Akses', icon: 'fluent-color:lock-shield-24', permission: 'users:manage' },
  { path: '/admin/receipt-settings', name: 'Desain Struk', icon: 'fluent-color:document-text-24', permission: 'settings:manage' },
  { path: '/admin/parameters', name: 'Parameter Retail', icon: 'fluent-color:settings-24', permission: ['settings:manage', 'master:write'] },
  { path: '/admin/audit-logs', name: 'Jejak Audit', icon: 'fluent-color:shield-checkmark-24', permission: ['audit:read', 'report:view'] },
]

const configMenus = computed(() => {
  return rawConfigMenus.filter(m => !m.permission || authStore.hasPermission(m.permission))
})

// 3. Menu Jaringan LAN Terminal Client
const networkMenu = {
  path: '/admin/client-connection',
  name: 'Koneksi Server LAN',
  icon: 'fluent-color:wifi-24',
}

onMounted(() => {
  networkStore.fetchHostStatus()
  authStore.fetchCurrentUser()
  locationStore.fetchAccessibleOutlets()
})

function handleLogout() {
  authStore.logout()
  router.push('/login')
}
</script>

<template>
  <div class="h-screen flex flex-col bg-slate-100 text-slate-900 overflow-hidden font-sans select-none" @click="showLocationDropdown = false">
    <!-- 1. TOP NATIVE DESKTOP TITLE BAR -->
    <header class="bg-white border-b border-slate-300 px-5 py-2 flex items-center justify-between shadow-sm z-30 shrink-0">
      <!-- Left: Brand & Outlet Info + Role Switcher Badge -->
      <div class="flex items-center gap-4">
        <div class="flex items-center gap-3">
          <img src="/logo.png" alt="Notane Logo" class="w-8 h-8 rounded-lg shadow-sm border border-slate-200 object-contain hover:scale-105 transition" />
          <div>
            <div class="font-extrabold text-sm text-slate-900 leading-none tracking-wide flex items-center gap-1.5">
              <span>NOTANE RETAIL POS</span>
              <span class="text-[9px] bg-emerald-100 text-emerald-800 font-mono px-1.5 py-0.5 rounded border border-emerald-300 font-bold">ENTERPRISE</span>
            </div>
            <!-- Interactive Location Switcher -->
            <div class="relative mt-1" @click.stop>
              <button
                type="button"
                @click="showLocationDropdown = !showLocationDropdown"
                class="text-[11px] text-slate-700 bg-slate-100 hover:bg-amber-50 hover:text-amber-900 border border-slate-300 hover:border-amber-300 px-2 py-0.5 rounded-md font-bold flex items-center gap-1.5 transition cursor-pointer shadow-2xs"
                title="Klik untuk memilih / mengganti lokasi outlet aktif"
              >
                <Icon icon="streamline-ultimate-color:shop-sign-bag" class="w-3.5 h-3.5 shrink-0" />
                <span class="font-mono text-amber-800 text-[10px]">[{{ locationStore.currentOutlet?.code || 'OUT' }}]</span>
                <span class="max-w-[140px] truncate">{{ locationStore.currentOutlet?.name || 'Pilih Lokasi' }}</span>
                <Icon icon="fluent:chevron-down-12-filled" class="w-2.5 h-2.5 text-slate-400 ml-0.5" />
              </button>

              <!-- Dropdown Menu Pilihan Lokasi -->
              <div
                v-if="showLocationDropdown"
                class="absolute left-0 top-full mt-1 w-64 bg-white rounded-xl shadow-xl border border-slate-200 py-1.5 z-50 text-xs animate-fadeIn"
              >
                <div class="px-3 py-1 text-[10px] font-bold text-slate-400 uppercase tracking-wider border-b border-slate-100 flex items-center justify-between">
                  <span>Lokasi Akses Anda</span>
                  <span class="font-mono text-slate-500">{{ locationStore.accessibleOutlets.length }} Lokasi</span>
                </div>
                <div class="max-h-56 overflow-y-auto divide-y divide-slate-100">
                  <button
                    v-for="loc in locationStore.accessibleOutlets"
                    :key="loc.id"
                    type="button"
                    @click="locationStore.switchLocation(loc.id); showLocationDropdown = false"
                    class="w-full px-3 py-2 text-left flex items-center justify-between hover:bg-amber-50/80 transition cursor-pointer"
                    :class="{ 'bg-amber-50 font-bold text-amber-900': loc.id === locationStore.currentOutletId }"
                  >
                    <div class="flex items-center gap-2 min-w-0">
                      <Icon
                        :icon="loc.is_warehouse ? 'fluent-color:building-warehouse-24' : 'fluent-color:building-shop-24'"
                        class="w-4 h-4 shrink-0"
                      />
                      <div class="truncate">
                        <div class="leading-tight truncate text-xs">{{ loc.name }}</div>
                        <div class="text-[10px] text-slate-400 font-mono mt-0.5">
                          {{ loc.code }} • {{ loc.is_warehouse ? 'Gudang' : 'Toko' }}
                        </div>
                      </div>
                    </div>
                    <Icon
                      v-if="loc.id === locationStore.currentOutletId"
                      icon="fluent:checkmark-circle-16-filled"
                      class="w-4 h-4 text-emerald-600 shrink-0"
                    />
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>

      </div>

      <!-- Right: Cashier POS Switcher & User Operator Info -->
      <div class="flex items-center gap-3">
        <button
          @click="router.push('/')"
          class="bg-emerald-600 hover:bg-emerald-700 text-white px-3.5 py-1.5 rounded-lg text-xs font-bold flex items-center gap-2 shadow-sm border border-emerald-700 transition active:scale-95 cursor-pointer"
        >
          <Icon icon="streamline-ultimate-color:shopping-cart-full" class="w-4 h-4" />
          <span>Buka Layar Kasir POS (F1)</span>
        </button>

        <!-- Server Host Connection Status Button -->
        <button
          type="button"
          @click="router.push('/admin/client-connection')"
          class="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg border text-[11px] font-bold cursor-pointer transition-all hover:opacity-90 active:scale-95"
          :class="networkStore.connectionStatus === 'CONNECTED'
            ? 'bg-emerald-50 text-emerald-800 border-emerald-300'
            : 'bg-rose-50 text-rose-800 border-rose-300'"
          title="Status Koneksi Terminal ke Server Host LAN. Klik untuk konfigurasi koneksi"
        >
          <span
            class="w-2 h-2 rounded-full"
            :class="networkStore.connectionStatus === 'CONNECTED' ? 'bg-emerald-500 animate-ping' : 'bg-rose-500'"
          ></span>
          <span>
            {{ networkStore.connectionStatus === 'CONNECTED'
              ? `Ke Host (${networkStore.latencyMs !== null ? `${networkStore.latencyMs}ms` : 'Aktif'})`
              : 'Terputus dari Host' }}
          </span>
        </button>

        <!-- User Operator Badge (Klik untuk Ganti Password / PIN Mandiri) -->
        <div class="h-7 border-l border-slate-300 mx-1"></div>

        <button
          type="button"
          @click="showSecurityModal = true"
          class="flex items-center gap-2 bg-slate-50 hover:bg-slate-100 border border-slate-300 hover:border-indigo-400 px-3 py-1 rounded-lg transition-all cursor-pointer shadow-2xs group"
          title="Klik untuk ubah Password atau PIN akun Anda"
        >
          <div class="w-6 h-6 rounded-md bg-emerald-100 border border-emerald-300 group-hover:bg-indigo-100 group-hover:border-indigo-300 flex items-center justify-center text-emerald-800 group-hover:text-indigo-800 font-black text-xs transition">
            {{ authStore.user?.full_name?.charAt(0) || 'U' }}
          </div>
          <div class="text-left">
            <div class="text-xs font-bold text-slate-800 group-hover:text-indigo-950 leading-none">{{ authStore.user?.full_name }}</div>
            <div class="text-[9px] text-slate-500 font-mono uppercase leading-none mt-0.5 flex items-center gap-1">
              <span>{{ authStore.user?.role_name }}</span>
              <Icon icon="flat-color-icons:key" class="w-3 h-3 opacity-90" />
            </div>
          </div>
        </button>

        <button
          @click="handleLogout"
          class="text-slate-500 hover:text-rose-600 p-1.5 rounded-lg hover:bg-slate-100 border border-slate-200 transition cursor-pointer"
          title="Keluar / Logout"
        >
          <LogOut class="w-4 h-4" />
        </button>
      </div>
    </header>

    <!-- Modal Mandiri Ganti Password & PIN -->
    <UserProfileSecurityModal
      :show="showSecurityModal"
      @close="showSecurityModal = false"
    />

    <!-- 2. TOP DESKTOP COMMAND TILES RIBBON ("Kotak-Kotak di Atas") -->
    <nav class="bg-slate-200/90 border-b border-slate-300 px-3 py-2 shrink-0 overflow-x-auto shadow-inner">
      <div class="flex items-center gap-2 min-w-max">
        <!-- GROUP 1: OPERASIONAL & MASTER DATA BISNIS -->
        <div class="flex items-center gap-2">
          <router-link
            v-for="item in operationalMenus"
            :key="item.path"
            :to="item.path"
            class="group flex flex-col items-center justify-center px-3.5 py-1.5 rounded-lg transition-all duration-150 relative border select-none min-w-[86px] h-[62px] text-center gap-1 cursor-pointer"
            :class="route.path === item.path
              ? 'bg-amber-100 text-amber-950 border-amber-500 shadow-sm ring-2 ring-amber-400/50 scale-[1.02]'
              : 'bg-white hover:bg-slate-50 text-slate-700 hover:text-slate-900 border-slate-300 shadow-xs hover:border-amber-300 active:scale-95'"
          >
            <Icon
              :icon="item.icon"
              class="w-6 h-6 transition-transform duration-150 group-hover:scale-110 drop-shadow-xs"
            />
            <span
              class="text-[10px] leading-tight tracking-tight font-bold whitespace-nowrap"
              :class="route.path === item.path ? 'text-amber-950 font-extrabold' : 'text-slate-700 group-hover:text-slate-900'"
            >
              {{ item.name }}
            </span>
            <div
              v-if="route.path === item.path"
              class="w-4 h-0.5 bg-amber-600 rounded-full"
            ></div>
          </router-link>
        </div>

        <!-- DIVIDER ANTAR KELOMPOK -->
        <div v-if="configMenus.length > 0" class="h-10 w-px bg-slate-300 mx-1"></div>

        <!-- GROUP 2: PENGATURAN & AUDIT -->
        <div v-if="configMenus.length > 0" class="flex items-center gap-2">
          <router-link
            v-for="item in configMenus"
            :key="item.path"
            :to="item.path"
            class="group flex flex-col items-center justify-center px-3.5 py-1.5 rounded-lg transition-all duration-150 relative border select-none min-w-[86px] h-[62px] text-center gap-1 cursor-pointer"
            :class="route.path === item.path
              ? 'bg-amber-100 text-amber-950 border-amber-500 shadow-sm ring-2 ring-amber-400/50 scale-[1.02]'
              : 'bg-white hover:bg-slate-50 text-slate-700 hover:text-slate-900 border-slate-300 shadow-xs hover:border-amber-300 active:scale-95'"
          >
            <Icon
              :icon="item.icon"
              class="w-6 h-6 transition-transform duration-150 group-hover:scale-110 drop-shadow-xs"
            />
            <span
              class="text-[10px] leading-tight tracking-tight font-bold whitespace-nowrap"
              :class="route.path === item.path ? 'text-amber-950 font-extrabold' : 'text-slate-700 group-hover:text-slate-900'"
            >
              {{ item.name }}
            </span>
            <div
              v-if="route.path === item.path"
              class="w-4 h-0.5 bg-amber-600 rounded-full"
            ></div>
          </router-link>
        </div>

        <!-- DIVIDER ANTAR KELOMPOK -->
        <!-- DIVIDER ANTAR KELOMPOK -->
        <div class="h-10 w-px bg-slate-300 mx-1"></div>

        <!-- GROUP 3: KONEKSI JARINGAN LAN -->
        <div class="flex items-center gap-2">
          <router-link
            :to="networkMenu.path"
            class="group flex flex-col items-center justify-center px-3.5 py-1.5 rounded-lg transition-all duration-150 relative border select-none min-w-[96px] h-[62px] text-center gap-1 cursor-pointer"
            :class="route.path === networkMenu.path
              ? 'bg-blue-100 text-blue-950 border-blue-500 shadow-sm ring-2 ring-blue-400/50 scale-[1.02]'
              : 'bg-white hover:bg-slate-50 text-slate-700 hover:text-slate-900 border-slate-300 shadow-xs hover:border-blue-300 active:scale-95'"
          >
            <Icon
              :icon="networkMenu.icon"
              class="w-6 h-6 transition-transform duration-150 group-hover:scale-110 drop-shadow-xs"
            />
            <span
              class="text-[10px] leading-tight tracking-tight font-bold whitespace-nowrap"
              :class="route.path === networkMenu.path ? 'font-extrabold text-blue-950' : 'text-slate-700 group-hover:text-slate-900'"
            >
              {{ networkMenu.name }}
            </span>
            <div
              v-if="route.path === networkMenu.path"
              class="w-4 h-0.5 rounded-full bg-blue-600"
            ></div>
          </router-link>
        </div>
      </div>
    </nav>

    <!-- 3. FULL-WIDTH DESKTOP WORKSPACE AREA -->
    <main class="flex-1 overflow-y-auto bg-slate-100">
      <slot />
    </main>

    <!-- Modal Konfigurasi Jaringan Lama (jika dipanggil) -->
    <NetworkConnectionModal v-model="showNetworkModal" />
  </div>
</template>
