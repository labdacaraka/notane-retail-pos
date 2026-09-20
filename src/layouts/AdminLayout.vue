<script setup lang="ts">
import { useRouter, useRoute } from 'vue-router'
import { LogOut, ChevronDown, Check, X, Server, Monitor } from 'lucide-vue-next'
import { useAuthStore } from '@/stores/authStore'
import { useNetworkStore } from '@/stores/networkStore'
import NetworkConnectionModal from '@/components/common/NetworkConnectionModal.vue'
import { ref, computed, onMounted } from 'vue'

const router = useRouter()
const route = useRoute()
const authStore = useAuthStore()
const networkStore = useNetworkStore()
const showNetworkModal = ref(false)
const showRoleModal = ref(false)

// 1. Grup Menu Operasional & Master Data Bisnis
const operationalMenus = [
  { path: '/admin', name: 'Dashboard Hub', icon: 'fluent-color:building-store-24' },
  { path: '/admin/sales', name: 'Penjualan', icon: 'fluent-color:receipt-24' },
  { path: '/admin/purchases', name: 'Pembelian', icon: 'streamline-ultimate-color:delivery-truck-cargo' },
  { path: '/admin/products', name: 'Master Produk', icon: 'streamline-ultimate-color:warehouse-cart-packages-2' },
  { path: '/admin/customers', name: 'Pelanggan & Member', icon: 'fluent-color:people-team-24' },
  { path: '/admin/vouchers', name: 'Voucher & Promo', icon: 'fluent-color:gift-card-24' },
  { path: '/admin/inventory', name: 'Stok & Opname', icon: 'streamline-ultimate-color:warehouse-storage-2' },
  { path: '/admin/accounting', name: 'Jurnal & Laba Rugi', icon: 'streamline-ultimate-color:accounting-calculator-1' },
]

// 2. Grup Menu Pengaturan & Keamanan
const configMenus = [
  { path: '/admin/users', name: 'Pengguna & Hak Akses', icon: 'fluent-color:lock-shield-24' },
  { path: '/admin/receipt-settings', name: 'Desain Struk', icon: 'fluent-color:document-text-24' },
  { path: '/admin/parameters', name: 'Parameter Retail', icon: 'fluent-color:settings-24' },
  { path: '/admin/audit-logs', name: 'Jejak Audit', icon: 'fluent-color:shield-checkmark-24' },
]

// 3. Menu Jaringan Beradaptasi Berdasarkan Node Role (Server vs Client)
const networkMenu = computed(() => {
  if (networkStore.isServerNode) {
    return {
      path: '/admin/server-control',
      name: 'Kendali Server Host',
      icon: 'fluent-color:database-24',
      badge: 'SERVER',
    }
  } else {
    return {
      path: '/admin/client-connection',
      name: 'Koneksi Server LAN',
      icon: 'fluent-color:wifi-24',
      badge: 'CLIENT',
    }
  }
})

onMounted(() => {
  networkStore.fetchHostStatus()
})

function handleLogout() {
  authStore.logout()
  router.push('/login')
}

function handleSelectRole(role: 'SERVER' | 'CLIENT') {
  networkStore.setNodeRole(role)
  showRoleModal.value = false
  // Jika sedang berada di halaman server-control tapi pindah ke client, redirect ke client-connection
  if (role === 'CLIENT' && route.path === '/admin/server-control') {
    router.push('/admin/client-connection')
  } else if (role === 'SERVER' && route.path === '/admin/client-connection') {
    router.push('/admin/server-control')
  }
}
</script>

<template>
  <div class="h-screen flex flex-col bg-slate-100 text-slate-900 overflow-hidden font-sans select-none">
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
            <div class="text-[10px] text-slate-500 font-mono mt-0.5 flex items-center gap-1.5">
              <Icon icon="streamline-ultimate-color:shop-sign-bag" class="w-3.5 h-3.5" />
              <span>Notane Flagship Store (OUT-001)</span>
            </div>
          </div>
        </div>

        <div class="h-6 w-px bg-slate-300"></div>

        <!-- Node Mode Indicator & Switcher Button -->
        <button
          type="button"
          @click="showRoleModal = true"
          class="flex items-center gap-1.5 px-2.5 py-1 rounded-lg border text-[11px] font-bold transition shadow-2xs cursor-pointer active:scale-95"
          :class="networkStore.isServerNode
            ? 'bg-purple-50 hover:bg-purple-100 text-purple-900 border-purple-300'
            : 'bg-blue-50 hover:bg-blue-100 text-blue-900 border-blue-300'"
          title="Klik untuk beralih antara Mode Server Host dan Mode Client Kasir"
        >
          <Server v-if="networkStore.isServerNode" class="w-3.5 h-3.5 text-purple-700" />
          <Monitor v-else class="w-3.5 h-3.5 text-blue-700" />
          <span class="tracking-wide">
            {{ networkStore.isServerNode ? 'MODE: SERVER HOST (MASTER)' : 'MODE: CLIENT KASIR (STATION)' }}
          </span>
          <ChevronDown class="w-3 h-3 opacity-60" />
        </button>
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

        <!-- Dynamic Server / Network Status Button -->
        <button
          v-if="networkStore.isServerNode"
          type="button"
          @click="router.push('/admin/server-control')"
          class="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg border text-[11px] font-bold cursor-pointer transition-all hover:opacity-90 active:scale-95"
          :class="networkStore.hostStatus.is_running
            ? 'bg-emerald-50 text-emerald-800 border-emerald-300'
            : 'bg-rose-50 text-rose-800 border-rose-300'"
          title="Status Daemon Server Host Port 8080. Klik untuk ke Pengendali Server Host"
        >
          <span
            class="w-2 h-2 rounded-full"
            :class="networkStore.hostStatus.is_running ? 'bg-emerald-500 animate-ping' : 'bg-rose-500'"
          ></span>
          <span>{{ networkStore.hostStatus.is_running ? `Host :${networkStore.hostStatus.port} (Aktif)` : 'Host Daemon Mati' }}</span>
        </button>

        <button
          v-else
          type="button"
          @click="router.push('/admin/client-connection')"
          class="flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg border text-[11px] font-bold cursor-pointer transition-all hover:opacity-90 active:scale-95"
          :class="networkStore.connectionStatus === 'CONNECTED'
            ? 'bg-emerald-50 text-emerald-800 border-emerald-300'
            : 'bg-rose-50 text-rose-800 border-rose-300'"
          title="Status Koneksi Terminal Kasir ke Server Host LAN. Klik untuk konfigurasi koneksi"
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

        <!-- User Operator Badge -->
        <div class="h-7 border-l border-slate-300 mx-1"></div>

        <div class="flex items-center gap-2 bg-slate-50 border border-slate-300 px-3 py-1 rounded-lg">
          <div class="w-6 h-6 rounded-md bg-emerald-100 border border-emerald-300 flex items-center justify-center text-emerald-800 font-black text-xs">
            {{ authStore.user?.full_name?.charAt(0) || 'U' }}
          </div>
          <div class="text-left">
            <div class="text-xs font-bold text-slate-800 leading-none">{{ authStore.user?.full_name }}</div>
            <div class="text-[9px] text-slate-500 font-mono uppercase leading-none mt-0.5">{{ authStore.user?.role_name }}</div>
          </div>
        </div>

        <button
          @click="handleLogout"
          class="text-slate-500 hover:text-rose-600 p-1.5 rounded-lg hover:bg-slate-100 border border-slate-200 transition cursor-pointer"
          title="Keluar / Logout"
        >
          <LogOut class="w-4 h-4" />
        </button>
      </div>
    </header>

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
        <div class="h-10 w-px bg-slate-300 mx-1"></div>

        <!-- GROUP 2: PENGATURAN & AUDIT -->
        <div class="flex items-center gap-2">
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
        <div class="h-10 w-px bg-slate-300 mx-1"></div>

        <!-- GROUP 3: INFRASTRUKTUR / JARINGAN (DIFERENSIASI CLIENT VS SERVER) -->
        <div class="flex items-center gap-2">
          <router-link
            :to="networkMenu.path"
            class="group flex flex-col items-center justify-center px-3.5 py-1.5 rounded-lg transition-all duration-150 relative border select-none min-w-[96px] h-[62px] text-center gap-1 cursor-pointer"
            :class="route.path === networkMenu.path
              ? (networkStore.isServerNode
                  ? 'bg-purple-100 text-purple-950 border-purple-500 shadow-sm ring-2 ring-purple-400/50 scale-[1.02]'
                  : 'bg-blue-100 text-blue-950 border-blue-500 shadow-sm ring-2 ring-blue-400/50 scale-[1.02]')
              : (networkStore.isServerNode
                  ? 'bg-purple-50/60 hover:bg-purple-50 text-purple-900 border-purple-200 shadow-xs active:scale-95'
                  : 'bg-blue-50/60 hover:bg-blue-50 text-blue-900 border-blue-200 shadow-xs active:scale-95')"
          >
            <Icon
              :icon="networkMenu.icon"
              class="w-6 h-6 transition-transform duration-150 group-hover:scale-110 drop-shadow-xs"
            />
            <div class="flex items-center gap-1">
              <span
                class="text-[10px] leading-tight tracking-tight font-bold whitespace-nowrap"
                :class="route.path === networkMenu.path ? 'font-extrabold' : 'text-slate-700 group-hover:text-slate-900'"
              >
                {{ networkMenu.name }}
              </span>
              <span
                class="text-[8px] font-mono font-bold px-1 rounded"
                :class="networkStore.isServerNode ? 'bg-purple-200 text-purple-800' : 'bg-blue-200 text-blue-800'"
              >
                {{ networkMenu.badge }}
              </span>
            </div>
            <div
              v-if="route.path === networkMenu.path"
              class="w-4 h-0.5 rounded-full"
              :class="networkStore.isServerNode ? 'bg-purple-600' : 'bg-blue-600'"
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

    <!-- MODAL PEMILIHAN PERAN PERANGKAT (NODE MODE SWITCHER) -->
    <div
      v-if="showRoleModal"
      class="fixed inset-0 z-50 flex items-center justify-center bg-slate-900/50 backdrop-blur-xs p-4"
    >
      <div class="bg-white border-2 border-slate-400 shadow-2xl rounded-xl w-full max-w-lg overflow-hidden flex flex-col font-sans">
        <!-- Title Bar -->
        <div class="bg-slate-800 text-white px-5 py-3 flex items-center justify-between">
          <div class="flex items-center gap-2 font-bold text-xs tracking-wide uppercase">
            <Server class="w-4 h-4 text-purple-400" />
            <span>Pilih Peran Perangkat Ini (Node Mode)</span>
          </div>
          <button @click="showRoleModal = false" class="text-slate-400 hover:text-white transition">
            <X class="w-4 h-4" />
          </button>
        </div>

        <div class="p-6 space-y-4">
          <p class="text-xs text-slate-600 leading-relaxed">
            Aplikasi Notane Retail POS dibangun dalam 1 basis kode yang fleksibel. Anda dapat menentukan apakah komputer ini bertindak sebagai <strong>Server Utama</strong> toko atau sebagai <strong>Terminal Kasir (Client)</strong>.
          </p>

          <div class="space-y-3">
            <!-- Option 1: Server Host (Master Node) -->
            <div
              @click="handleSelectRole('SERVER')"
              class="border-2 rounded-xl p-4 flex items-start gap-3.5 cursor-pointer transition"
              :class="networkStore.isServerNode
                ? 'border-purple-600 bg-purple-50/60 shadow-xs'
                : 'border-slate-200 hover:border-slate-300 bg-white'"
            >
              <div
                class="w-10 h-10 rounded-lg flex items-center justify-center shrink-0"
                :class="networkStore.isServerNode ? 'bg-purple-600 text-white' : 'bg-purple-100 text-purple-700'"
              >
                <Server class="w-5 h-5" />
              </div>
              <div class="flex-1">
                <div class="flex items-center justify-between">
                  <div class="text-xs font-bold text-slate-900">Mode Server Host (Master Node)</div>
                  <span
                    v-if="networkStore.isServerNode"
                    class="text-[9px] bg-purple-200 text-purple-900 font-mono font-bold px-1.5 py-0.5 rounded flex items-center gap-1"
                  >
                    <Check class="w-3 h-3" /> AKTIF
                  </span>
                </div>
                <p class="text-[11px] text-slate-500 mt-1 leading-normal">
                  Komputer ini adalah server toko utama yang menjalankan database PostgreSQL dan backend Axum di port 8080 untuk melayani terminal-terminal kasir lain di jaringan toko.
                </p>
              </div>
            </div>

            <!-- Option 2: Client Station (Terminal Kasir) -->
            <div
              @click="handleSelectRole('CLIENT')"
              class="border-2 rounded-xl p-4 flex items-start gap-3.5 cursor-pointer transition"
              :class="networkStore.isClientNode
                ? 'border-blue-600 bg-blue-50/60 shadow-xs'
                : 'border-slate-200 hover:border-slate-300 bg-white'"
            >
              <div
                class="w-10 h-10 rounded-lg flex items-center justify-center shrink-0"
                :class="networkStore.isClientNode ? 'bg-blue-600 text-white' : 'bg-blue-100 text-blue-700'"
              >
                <Monitor class="w-5 h-5" />
              </div>
              <div class="flex-1">
                <div class="flex items-center justify-between">
                  <div class="text-xs font-bold text-slate-900">Mode Client Kasir (Station Node)</div>
                  <span
                    v-if="networkStore.isClientNode"
                    class="text-[9px] bg-blue-200 text-blue-900 font-mono font-bold px-1.5 py-0.5 rounded flex items-center gap-1"
                  >
                    <Check class="w-3 h-3" /> AKTIF
                  </span>
                </div>
                <p class="text-[11px] text-slate-500 mt-1 leading-normal">
                  Komputer ini hanya bertindak sebagai terminal kasir / tablet pelanggan yang terhubung ke Server Host melalui jaringan LAN lokal (Wi-Fi/Ethernet).
                </p>
              </div>
            </div>
          </div>
        </div>

        <!-- Footer -->
        <div class="bg-slate-50 border-t border-slate-200 px-6 py-3 flex justify-end">
          <button
            type="button"
            @click="showRoleModal = false"
            class="px-4 py-1.5 bg-slate-200 hover:bg-slate-300 text-slate-800 text-xs font-bold rounded-lg transition cursor-pointer"
          >
            Tutup
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
