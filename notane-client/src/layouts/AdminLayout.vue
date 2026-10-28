<script setup lang="ts">
import { useRouter, useRoute } from 'vue-router'
import { LogOut } from 'lucide-vue-next'
import { useAuthStore } from '@/stores/authStore'
import { useNetworkStore } from '@/stores/networkStore'
import { useLocationStore } from '@/stores/locationStore'
import { useStoreBrandingStore } from '@/stores/storeBrandingStore'
import { useDialog } from '@/composables/useDialog'
import { useLicenseStore } from '@/stores/licenseStore'
import UserProfileSecurityModal from '@/components/common/UserProfileSecurityModal.vue'
import ClientUpdateModal from '@/components/common/ClientUpdateModal.vue'
import ThemeToggle from '@/components/common/ThemeToggle.vue'
import { ref, computed, onMounted } from 'vue'

const router = useRouter()
const route = useRoute()
const authStore = useAuthStore()
const networkStore = useNetworkStore()
const locationStore = useLocationStore()
const storeBrandingStore = useStoreBrandingStore()
const dialog = useDialog()
const licenseStore = useLicenseStore()
const showSecurityModal = ref(false)
const showLocationDropdown = ref(false)
const showClientUpdateModal = ref(false)

const isDisconnected = computed(() => {
  if (networkStore.nodeRole === 'SERVER') return false
  return networkStore.connectionStatus !== 'CONNECTED'
})
const isConnectionConfigPage = computed(() => route.path === '/admin/client-connection')

// 1. Grup Menu Operasional & Master Data Bisnis
const rawOperationalMenus = [
  { path: '/admin', name: 'Dashboard Hub', icon: 'fluent-color:apps-24' },
  { path: '/admin/outlets', name: 'Master Outlet', icon: 'fluent-color:building-store-24', permission: ['outlets:read', 'outlets:manage', 'master:read'] },
  { path: '/admin/sales', name: 'Penjualan', icon: 'fluent-color:receipt-24', permission: ['sales:read', 'report:view'] },
  { path: '/admin/purchases', name: 'Pembelian', icon: 'streamline-ultimate-color:delivery-truck-cargo', permission: ['purchasing:read', 'purchasing:write'] },
  { path: '/admin/products', name: 'Master Produk', icon: 'streamline-ultimate-color:warehouse-cart-packages-2', permission: ['products:read', 'master:read'] },
  { path: '/admin/price-labels', name: 'Cetak Label Harga', icon: 'streamline-ultimate-color:tags-1', permission: ['products:read', 'master:read'], requiredFeature: 'label_printing' },
  { path: '/admin/customers', name: 'Pelanggan & Member', icon: 'fluent-color:people-team-24', permission: ['customers:read', 'customers:manage', 'master:read'], requiredFeature: 'membership' },
  { path: '/admin/vouchers', name: 'Voucher & Promo', icon: 'fluent-color:gift-card-24', permission: ['vouchers:read', 'vouchers:manage', 'master:read'], requiredFeature: 'promotions' },
  { path: '/admin/flash-sales', name: 'Flash Sale', icon: 'flat-color-icons:flash-on', permission: ['promotions:read', 'master:read'], requiredFeature: 'promotions' },
  { path: '/admin/promotions', name: 'Promosi Retail', icon: 'streamline-ultimate-color:discount', permission: ['promotions:read', 'master:read'], requiredFeature: 'promotions' },
  { path: '/admin/inventory', name: 'Stok & Opname', icon: 'streamline-ultimate-color:warehouse-storage-2', permission: ['inventory:read', 'inventory:write'] },
  { path: '/admin/accounting', name: 'Jurnal & Laba Rugi', icon: 'streamline-ultimate-color:accounting-calculator-1', permission: ['accounting:read', 'accounting:write'], requiredFeature: 'accounting' },
  { path: '/admin/expenses', name: 'Biaya & Pengeluaran', icon: 'streamline-ultimate-color:money-bag-dollar', permission: ['expenses:read', 'expenses:categories', 'accounting:read'], requiredFeature: 'expenses' },
]

const operationalMenus = computed(() => {
  return rawOperationalMenus.filter(m => {
    const passPerm = !m.permission || authStore.hasPermission(m.permission)
    const passFeat = !m.requiredFeature || licenseStore.hasFeature(m.requiredFeature)
    return passPerm && passFeat
  })
})

// 2. Grup Menu Pengaturan & Keamanan
const rawConfigMenus = [
  { path: '/admin/users', name: 'Pengguna & Hak Akses', icon: 'fluent-color:lock-shield-24', permission: ['users:read', 'users:manage'] },
  { path: '/admin/receipt-settings', name: 'Desain Struk', icon: 'fluent-color:document-text-24', permission: ['settings:read', 'settings:update', 'settings:manage'] },
  { path: '/admin/parameters', name: 'Parameter Retail', icon: 'fluent-color:settings-24', permission: ['settings:manage', 'settings:update', 'master:write'] },
  { path: '/admin/audit-logs', name: 'Jejak Audit', icon: 'fluent-color:shield-checkmark-24', permission: ['audit:read', 'report:view'], requiredFeature: 'audit_log' },
]

const configMenus = computed(() => {
  return rawConfigMenus.filter(m => {
    const passPerm = !m.permission || authStore.hasPermission(m.permission)
    const passFeat = !m.requiredFeature || licenseStore.hasFeature(m.requiredFeature)
    return passPerm && passFeat
  })
})

// 3. Menu Jaringan LAN Terminal Client
const networkMenu = {
  path: '/admin/client-connection',
  name: 'Koneksi Server LAN',
  icon: 'fluent-color:wifi-24',
}

onMounted(async () => {
  await networkStore.checkConnection()
  authStore.fetchCurrentUser()
  locationStore.fetchAccessibleOutlets()
  storeBrandingStore.fetchBranding()
  licenseStore.fetchLicenseInfo()
})

function handleLogout() {
  authStore.logout()
  router.push('/login')
}

async function handleExitImpersonation() {
  await authStore.stopImpersonating()
  dialog.toast('Anda telah kembali ke akun asli Anda.', 'info')
}
</script>

<template>
  <div class="h-screen flex flex-col bg-slate-100 text-slate-900 overflow-hidden font-sans select-none" @click="showLocationDropdown = false">
    <!-- Impersonation Active Banner -->
    <div
      v-if="authStore.isImpersonating"
      class="bg-gradient-to-r from-purple-800 via-indigo-800 to-purple-900 text-white px-5 py-2 flex items-center justify-between shadow-md z-40 text-xs shrink-0 border-b border-purple-950"
    >
      <div class="flex items-center gap-2.5 font-medium">
        <span class="p-1 rounded-md bg-purple-600/60 border border-purple-400/40 text-white animate-pulse">
          <Icon icon="fluent-color:people-sync-24" class="w-4 h-4" />
        </span>
        <span>
          Mode Impersonate: Anda sedang login sebagai <strong>{{ authStore.user?.full_name }}</strong> (<span class="font-mono text-purple-200 font-bold">{{ authStore.user?.role_name }}</span>).
        </span>
      </div>

      <button
        @click="handleExitImpersonation"
        class="bg-white/15 hover:bg-white text-white hover:text-purple-950 border border-white/30 px-3 py-1 rounded-lg font-bold text-xs transition flex items-center gap-1.5 cursor-pointer shadow-xs active:scale-95"
      >
        <Icon icon="flat-color-icons:leave" class="w-4 h-4" />
        <span>Kembali ke Akun Asli ({{ authStore.impersonator?.full_name || authStore.impersonator?.username || 'Admin' }})</span>
      </button>
    </div>

    <!-- 1. TOP NATIVE DESKTOP TITLE BAR -->
    <header class="bg-white border-b border-slate-300 px-5 py-2 flex items-center justify-between shadow-sm z-30 shrink-0">
      <!-- Left: Brand & Outlet Info + Role Switcher Badge -->
      <div class="flex items-center gap-4">
        <div class="flex items-center gap-3">
          <!-- Logo Toko Mitra (Eyecatching Store Identity) -->
          <div class="w-10 h-10 rounded-xl bg-slate-50 border border-slate-200 p-1 flex items-center justify-center shadow-xs shrink-0 overflow-hidden group hover:border-slate-300 transition">
            <img
              v-if="storeBrandingStore.storeLogoUrl"
              :src="storeBrandingStore.storeLogoUrl"
              alt="Logo Toko"
              class="w-full h-full object-contain"
            />
            <div
              v-else
              class="w-full h-full rounded-lg bg-gradient-to-br from-indigo-500 to-indigo-700 flex items-center justify-center text-white font-black text-sm"
            >
              {{ (storeBrandingStore.storeName || 'T').charAt(0).toUpperCase() }}
            </div>
          </div>

          <div>
            <div class="flex items-center gap-2">
              <span class="font-black text-sm text-slate-900 leading-none tracking-tight uppercase">
                {{ storeBrandingStore.storeName }}
              </span>

              <!-- Co-branding Badge: Powered by Notane POS (Authenticity Credit) -->
              <div
                v-if="!storeBrandingStore.removeBranding"
                class="flex items-center gap-1 bg-slate-100 hover:bg-slate-200/80 border border-slate-200/80 px-1.5 py-0.5 rounded text-[10px] text-slate-600 font-mono transition"
                title="Produksi & Didukung oleh Notane Retail POS"
              >
                <img src="/logo.png" alt="Notane Logo" class="w-3.5 h-3.5 object-contain rounded" />
                <span class="font-bold">Notane POS</span>
                <span class="text-[8px] bg-emerald-100 text-emerald-800 font-mono px-1 py-0.2 rounded font-bold">PRO</span>
              </div>
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
                        :icon="loc.is_warehouse ? 'streamline-ultimate-color:warehouse-storage-2' : 'fluent-color:building-store-24'"
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
          v-if="authStore.hasPermission('pos:checkout')"
          @click="router.push('/')"
          class="bg-emerald-600 hover:bg-emerald-700 text-white px-3.5 py-1.5 rounded-lg text-xs font-bold flex items-center gap-2 shadow-sm border border-emerald-700 transition active:scale-95 cursor-pointer"
        >
          <Icon icon="streamline-ultimate-color:shopping-cart-full" class="w-4 h-4" />
          <span>Buka Kasir (F1)</span>
        </button>

        <!-- Tombol Update Client UI -->
        <button
          type="button"
          @click="showClientUpdateModal = true"
          class="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg border text-[11px] font-bold cursor-pointer transition-all hover:opacity-90 active:scale-95 bg-cyan-50 text-cyan-800 border-cyan-200 hover:bg-cyan-100"
          title="Periksa pembaruan aplikasi kasir"
        >
          <Icon icon="fluent:arrow-sync-circle-16-filled" class="w-3.5 h-3.5 text-cyan-600" />
          <span>Pembaruan</span>
          <span
            v-if="licenseStore.updateInfo?.has_update && licenseStore.updateInfo?.component === 'client'"
            class="px-1.5 py-0.2 rounded-full text-[9px] bg-cyan-600 text-white font-extrabold animate-pulse"
          >
            Ada Versi Baru
          </span>
        </button>

        <!-- Server Host Connection Status Button -->
        <button
          type="button"
          @click="router.push('/admin/client-connection')"
          class="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg border text-[11px] font-bold cursor-pointer transition-all hover:opacity-90 active:scale-95"
          :class="networkStore.connectionStatus === 'CONNECTED'
            ? 'bg-emerald-50 text-emerald-800 border-emerald-300'
            : 'bg-rose-50 text-rose-800 border-rose-300'"
          title="Status sambungan ke komputer server toko. Klik untuk mengatur sambungan"
        >
          <span
            class="w-2 h-2 rounded-full"
            :class="networkStore.connectionStatus === 'CONNECTED' ? 'bg-emerald-500 animate-ping' : 'bg-rose-500'"
          ></span>
          <span>
            {{ networkStore.connectionStatus === 'CONNECTED'
              ? `Server Toko (${networkStore.latencyMs !== null ? `${networkStore.latencyMs}ms` : 'Terhubung'})`
              : 'Server Terputus' }}
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

        <!-- Switch Dark / Light Mode -->
        <ThemeToggle />

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

    <!-- Modal Pembaruan Notane Client UI -->
    <ClientUpdateModal
      :is-open="showClientUpdateModal"
      @close="showClientUpdateModal = false"
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

    <!-- 3. OFFLINE WARNING STRIP WHEN DISCONNECTED -->
    <div
      v-if="isDisconnected && !isConnectionConfigPage"
      class="bg-rose-600 text-white px-4 py-2 flex items-center justify-between text-xs font-bold shadow-sm z-20 shrink-0"
    >
      <div class="flex items-center gap-2">
        <span class="w-2.5 h-2.5 rounded-full bg-white animate-ping"></span>
        <Icon icon="fluent:warning-24-filled" class="w-4 h-4 text-amber-300" />
        <span>Koneksi ke Server Host Terputus: Semua operasi API & transaksi di-disable untuk mencegah inkonsistensi data.</span>
      </div>
      <div class="flex items-center gap-2">
        <button
          type="button"
          @click="networkStore.checkConnection()"
          class="bg-white/20 hover:bg-white/30 text-white text-[11px] font-bold px-2.5 py-1 rounded border border-white/40 transition cursor-pointer"
        >
          Coba Sambungkan Lagi
        </button>
        <button
          type="button"
          @click="router.push('/admin/client-connection')"
          class="bg-white text-rose-800 hover:bg-rose-50 text-[11px] font-black px-2.5 py-1 rounded shadow-xs transition cursor-pointer"
        >
          Konfigurasi Jaringan
        </button>
      </div>
    </div>

    <!-- 4. FULL-WIDTH DESKTOP WORKSPACE AREA -->
    <main class="flex-1 overflow-y-auto bg-slate-100 relative">
      <!-- Disconnected Interaction Blocker Overlay for non-config pages -->
      <div
        v-if="isDisconnected && !isConnectionConfigPage"
        class="absolute inset-0 z-40 bg-slate-900/40 backdrop-blur-[1px] flex flex-col items-center justify-center p-6 text-center select-none"
      >
        <div class="bg-white rounded-2xl p-6 max-w-md shadow-2xl border border-slate-300 space-y-4">
          <div class="w-14 h-14 rounded-2xl bg-rose-100 text-rose-600 border border-rose-200 flex items-center justify-center mx-auto">
            <Icon icon="streamline-ultimate-color:wifi-off" class="w-8 h-8" />
          </div>
          <div>
            <h3 class="text-base font-black text-slate-900">Koneksi Host Server Terputus</h3>
            <p class="text-xs text-slate-600 mt-1 leading-relaxed">
              Tombol dan interaksi dinonaktifkan sementara karena tidak ada koneksi aktif ke server host database backend.
            </p>
            <div v-if="networkStore.disconnectReason" class="mt-2 p-2 rounded-lg bg-rose-50 border border-rose-200 text-rose-800 text-[11px] font-mono">
              {{ networkStore.disconnectReason }}
            </div>
          </div>
          <div class="flex items-center justify-center gap-2 pt-2">
            <button
              type="button"
              @click="networkStore.checkConnection()"
              class="px-4 py-2 bg-slate-100 hover:bg-slate-200 text-slate-800 rounded-lg text-xs font-bold border border-slate-300 transition cursor-pointer"
            >
              Cek Ulang Koneksi
            </button>
            <button
              type="button"
              @click="router.push('/admin/client-connection')"
              class="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg text-xs font-bold border border-blue-700 shadow-xs transition cursor-pointer"
            >
              Atur Sambungan LAN
            </button>
          </div>
        </div>
      </div>

      <slot />
    </main>

    <!-- Modal Konfigurasi Jaringan Lama (jika dipanggil) -->
  </div>
</template>
