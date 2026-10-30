<script setup lang="ts">
import { computed, onMounted } from 'vue'
import { useRouter } from 'vue-router'
import AdminLayout from '@/layouts/AdminLayout.vue'
import { useAuthStore } from '@/stores/authStore'
import { useStoreBrandingStore } from '@/stores/storeBrandingStore'
import { useLicenseStore } from '@/stores/licenseStore'
import { useDialog } from '@/composables/useDialog'
import { ArrowUpRight } from 'lucide-vue-next'

const router = useRouter()
const authStore = useAuthStore()
const storeBrandingStore = useStoreBrandingStore()
const licenseStore = useLicenseStore()
const dialog = useDialog()

onMounted(() => {
  storeBrandingStore.fetchBranding().catch(() => {})
  licenseStore.fetchLicenseInfo().catch(() => {})
})

function handleTileClick(tile: any) {
  if (tile.isLocked) {
    dialog.alert(
      `Fitur "${tile.title}" belum diaktifkan pada lisensi server toko Anda. Silakan hubungi Provider Notane Retail POS untuk mengaktifkan lisensi modul ini.`,
      'warning'
    )
    return
  }
  router.push(tile.path)
}

interface HubTile {
  title: string
  desc: string
  icon: string
  path: string
  color: string
  badge: string
  permission?: string[]
  requiredFeature?: string
  isLocked?: boolean
}

interface HubSection {
  category: string
  tiles: HubTile[]
}

const hubSections = computed(() => {
  const allSections: HubSection[] = [
    {
      category: 'DATA MASTER TOKO',
      tiles: [
        {
          title: 'Master Outlet & Gerai',
          desc: 'Manajemen cabang outlet, penugasan Store Manager, Supervisor penanggung jawab & gudang',
          icon: 'fluent-color:building-store-24',
          path: '/admin/outlets',
          color: 'from-amber-500 to-orange-600',
          badge: 'Outlet & Toko',
          permission: ['outlets:read', 'outlets:manage', 'master:read'],
        },
        {
          title: 'Master Produk & SKU',
          desc: 'Katalog barang multi-satuan, barcode scanner, tiering harga grosir & HPP',
          icon: 'streamline-ultimate-color:warehouse-cart-packages-2',
          path: '/admin/products',
          color: 'from-amber-400 to-amber-600',
          badge: 'Katalog Barang',
          permission: ['products:read', 'master:read'],
        },
        {
          title: 'Cetak Label Harga',
          desc: 'Desain label barcode rak display, cetak stiker harga & label promo',
          icon: 'streamline-ultimate-color:tags-1',
          path: '/admin/price-labels',
          color: 'from-amber-400 to-amber-600',
          badge: 'Label Barcode',
          permission: ['products:read', 'master:read'],
          requiredFeature: 'label_printing',
        },
        {
          title: 'Pelanggan & Member',
          desc: 'Registrasi pelanggan, tier membership, saldo reward poin & piutang',
          icon: 'fluent-color:people-team-24',
          path: '/admin/customers',
          color: 'from-blue-400 to-indigo-600',
          badge: 'Loyalti Member',
          permission: ['customers:read', 'customers:manage', 'master:read'],
          requiredFeature: 'membership',
        },
        {
          title: 'Voucher & Promo',
          desc: 'Kupon promo diskon persentase, nominal tetap & tanggal berlaku',
          icon: 'fluent-color:gift-card-24',
          path: '/admin/vouchers',
          color: 'from-rose-400 to-pink-600',
          badge: 'Promosi',
          permission: ['vouchers:read', 'vouchers:manage', 'master:read'],
          requiredFeature: 'promotions',
        },
        {
          title: 'Flash Sale Diskon',
          desc: 'Manajemen event flash sale jam tertentu, batas kuota & hitung mundur',
          icon: 'flat-color-icons:flash-on',
          path: '/admin/flash-sales',
          color: 'from-orange-500 to-rose-600',
          badge: 'Flash Sale',
          permission: ['promotions:read', 'master:read'],
          requiredFeature: 'promotions',
        },
        {
          title: 'Promosi Retail',
          desc: 'Aturan diskon otomatis bertingkat, tebus murah, dan beli X gratis Y',
          icon: 'streamline-ultimate-color:discount',
          path: '/admin/promotions',
          color: 'from-rose-500 to-red-600',
          badge: 'Diskon & Promo',
          permission: ['promotions:read', 'master:read'],
          requiredFeature: 'promotions',
        },
        {
          title: 'Parameter Retail',
          desc: 'Satuan barang (termasuk desimal/timbangan), kategori, merek & distributor',
          icon: 'fluent-color:settings-24',
          path: '/admin/parameters',
          color: 'from-teal-400 to-emerald-600',
          badge: 'Konfigurasi',
          permission: ['settings:manage', 'settings:update', 'master:write'],
        },
      ],
    },
    {
      category: 'TRANSAKSI & OPERASIONAL BISNIS',
      tiles: [
        {
          title: 'Transaksi Penjualan',
          desc: 'Riwayat penjualan kasir, cetak ulang struk, dan pembatalan nota (void)',
          icon: 'fluent-color:receipt-24',
          path: '/admin/sales',
          color: 'from-amber-400 to-amber-600',
          badge: 'Penjualan',
          permission: ['sales:read', 'report:view'],
        },
        {
          title: 'Pengadaan & Pembelian',
          desc: 'Penerimaan barang masuk, tagihan supplier, pesanan pembelian & data pemasok',
          icon: 'streamline-ultimate-color:delivery-truck-cargo',
          path: '/admin/purchases',
          color: 'from-blue-500 to-indigo-600',
          badge: 'Pembelian',
          permission: ['purchasing:read', 'purchasing:write'],
        },
        {
          title: 'Layar Kasir POS',
          desc: 'Buka transaksi kasir penjualan POS',
          icon: 'streamline-ultimate-color:shopping-cart-full',
          path: '/',
          color: 'from-emerald-400 to-teal-600',
          badge: 'Kasir',
          permission: ['sales:create', 'pos:checkout'],
        },
        {
          title: 'Stok & Stock Opname',
          desc: 'Kartu stok, mutasi gudang, opname fisik barang & penyesuaian selisih',
          icon: 'streamline-ultimate-color:warehouse-storage-2',
          path: '/admin/inventory',
          color: 'from-orange-500 to-amber-600',
          badge: 'Gudang',
          permission: ['inventory:read', 'inventory:write'],
        },
        {
          title: 'Biaya & Pengeluaran',
          desc: 'Pencatatan kas operasional, petty cash, kategori beban biaya & laporan pengeluaran',
          icon: 'streamline-ultimate-color:money-bag-dollar',
          path: '/admin/expenses',
          color: 'from-rose-500 to-amber-600',
          badge: 'Beban Operasional',
          permission: ['expenses:read', 'expenses:categories', 'accounting:read'],
          requiredFeature: 'expenses',
        },
      ],
    },
    {
      category: 'JARINGAN & PENGATURAN SISTEM',
      tiles: [
        {
          title: 'Koneksi Server LAN',
          desc: 'Pengaturan IP server host, port, kode pairing & uji latensi koneksi kasir ke server',
          icon: 'fluent-color:wifi-24',
          path: '/admin/client-connection',
          color: 'from-blue-500 to-cyan-600',
          badge: 'Koneksi Server',
          permission: undefined,
        },
        {
          title: 'Pengaturan Struk Kasir',
          desc: 'Identitas toko, alamat, no telp, NPWP, pesan footer, dan format thermal printer',
          icon: 'fluent-color:document-text-24',
          path: '/admin/receipt-settings',
          color: 'from-purple-500 to-indigo-600',
          badge: 'Struk Kasir',
          permission: ['settings:read', 'settings:update', 'settings:manage'],
        },
      ],
    },
    {
      category: 'KEAMANAN & MANAJEMEN SISTEM',
      tiles: [
        {
          title: 'Pengguna & Hak Akses',
          desc: 'Kelola akun staf kasir, peran jabatan, dan izin akses fitur',
          icon: 'fluent-color:lock-shield-24',
          path: '/admin/users',
          color: 'from-indigo-500 to-purple-700',
          badge: 'Hak Akses',
          permission: ['users:read', 'users:manage'],
        },
        {
          title: 'Jurnal & Laba Rugi',
          desc: 'Buku jurnal umum dan laporan laba rugi toko',
          icon: 'streamline-ultimate-color:accounting-calculator-1',
          path: '/admin/accounting',
          color: 'from-emerald-500 to-teal-700',
          badge: 'Akuntansi',
          permission: ['accounting:read', 'accounting:write'],
          requiredFeature: 'accounting',
        },
        {
          title: 'Jejak Audit',
          desc: 'Riwayat aktivitas kasir, izin supervisor, dan log sistem',
          icon: 'fluent-color:shield-checkmark-24',
          path: '/admin/audit-logs',
          color: 'from-slate-600 to-slate-800',
          badge: 'Jejak Audit',
          permission: ['audit:read', 'report:view'],
          requiredFeature: 'audit_log',
        },
      ],
    },
  ]

  return allSections
    .map((sec) => ({
      ...sec,
      tiles: sec.tiles
        .filter((t) => !t.permission || authStore.hasPermission(t.permission))
        .map((t) => ({
          ...t,
          isLocked: Boolean(t.requiredFeature && !licenseStore.hasFeature(t.requiredFeature)),
        })),
    }))
    .filter((sec) => sec.tiles.length > 0)
})
</script>

<template>
  <AdminLayout>
    <div class="max-w-7xl mx-auto p-6 space-y-6 select-none">
      <!-- Welcome Hero Banner -->
      <div class="bg-white border border-slate-300 p-6 rounded-2xl shadow-sm flex flex-wrap items-center justify-between gap-6 relative overflow-hidden">
        <div class="flex items-center gap-5">
          <template v-if="storeBrandingStore.removeBranding">
            <img
              v-if="storeBrandingStore.storeLogoUrl"
              :src="storeBrandingStore.storeLogoUrl"
              :alt="storeBrandingStore.storeName"
              class="w-16 h-16 rounded-xl shadow-sm border border-slate-200 shrink-0 object-contain p-1 bg-white hover:scale-105 transition"
            />
            <div
              v-else
              class="w-16 h-16 rounded-xl shadow-sm border border-slate-200 bg-emerald-50 text-emerald-700 flex items-center justify-center shrink-0"
            >
              <Icon icon="fluent-color:building-store-24" class="w-8 h-8" />
            </div>
          </template>
          <img
            v-else
            src="/logo.png"
            alt="Notane Logo"
            class="w-16 h-16 rounded-xl shadow-sm border border-slate-200 shrink-0 hover:scale-105 transition"
          />

          <div class="space-y-1">
            <div class="flex items-center gap-2 text-emerald-700 text-xs font-mono font-bold uppercase tracking-wider">
              <Icon icon="streamline-ultimate-color:shop-sign-bag" class="w-4 h-4" />
              <span>Menu Administrasi & Pengaturan</span>
            </div>
            <h1 class="text-2xl font-black text-slate-900 tracking-tight">
              <template v-if="storeBrandingStore.removeBranding">
                Menu Utama <span class="text-notane-gradient">{{ storeBrandingStore.storeName || 'Back-Office Retail' }}</span>
              </template>
              <template v-else>
                Menu Utama <span class="text-notane-gradient">Notane Retail POS</span>
              </template>
            </h1>
            <p class="text-xs text-slate-600 max-w-xl">
              Pilih menu di bawah ini untuk mengelola data produk, member pelanggan, kupon promosi, stok gudang, pembukuan akuntansi, dan jejak audit.
            </p>
          </div>
        </div>

        <button
          @click="router.push('/')"
          class="bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-xs px-5 py-3 rounded-xl shadow-sm flex items-center gap-2.5 transition active:scale-95 border border-emerald-700 cursor-pointer"
        >
          <Icon icon="streamline-ultimate-color:shopping-cart-full" class="w-5 h-5" />
          <span>Buka Kasir POS (F1)</span>
          <ArrowUpRight class="w-4 h-4" />
        </button>
      </div>

      <!-- Modular Desktop Tiles Grid ("Kotak-Kotak di Tengah") -->
      <div class="space-y-6">
        <div v-for="(section, idx) in hubSections" :key="idx" class="space-y-3">
          <div class="text-[11px] font-mono font-extrabold text-slate-700 tracking-wider flex items-center gap-2 uppercase">
            <span class="w-2.5 h-2.5 rounded-sm bg-emerald-600"></span>
            <span>{{ section.category }}</span>
          </div>

          <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
            <div
              v-for="tile in section.tiles"
              :key="tile.title"
              @click="handleTileClick(tile)"
              class="group border p-5 rounded-xl shadow-sm transition duration-150 cursor-pointer flex flex-col justify-between relative overflow-hidden active:scale-95"
              :class="tile.isLocked
                ? 'bg-slate-100/70 hover:bg-slate-200/60 border-dashed border-slate-300 opacity-60 hover:opacity-85'
                : 'bg-white hover:bg-slate-50/80 border-slate-300 hover:border-emerald-500 hover:shadow-md'"
              :title="tile.isLocked ? `Fitur ${tile.title} (Perlu Upgrade Lisensi)` : tile.title"
            >
              <!-- Top Color Strip -->
              <div
                class="absolute top-0 left-0 right-0 h-1.5"
                :class="tile.isLocked ? 'bg-slate-300' : 'bg-gradient-to-r ' + tile.color"
              ></div>

              <div>
                <!-- Tile Header -->
                <div class="flex items-center justify-between mb-4">
                  <div
                    class="w-12 h-12 rounded-xl flex items-center justify-center border shadow-xs transition"
                    :class="tile.isLocked
                      ? 'bg-slate-200/60 border-slate-300 grayscale opacity-70'
                      : 'bg-slate-50 border-slate-200 group-hover:border-amber-400 group-hover:bg-amber-50/50 group-hover:scale-105'"
                  >
                    <Icon :icon="tile.icon" class="w-7 h-7 drop-shadow-xs" />
                  </div>

                  <!-- Badge Upgrade vs Normal -->
                  <span
                    v-if="tile.isLocked"
                    class="text-[9px] font-mono font-black text-amber-900 bg-amber-100 border border-amber-300 px-2 py-0.5 rounded-md flex items-center gap-1 shadow-2xs"
                  >
                    <Icon icon="fluent:sparkle-16-filled" class="w-3 h-3 text-amber-600" />
                    Upgrade
                  </span>
                  <span
                    v-else
                    class="text-[9px] font-mono font-bold text-slate-600 bg-slate-100 px-2.5 py-1 rounded-md border border-slate-200"
                  >
                    {{ tile.badge }}
                  </span>
                </div>

                <!-- Tile Title & Description -->
                <h3
                  class="text-sm font-bold transition flex items-center justify-between"
                  :class="tile.isLocked ? 'text-slate-500' : 'text-slate-900 group-hover:text-emerald-700'"
                >
                  <span>{{ tile.title }}</span>
                  <ArrowUpRight
                    v-if="!tile.isLocked"
                    class="w-4 h-4 opacity-0 group-hover:opacity-100 text-emerald-600 transition-opacity"
                  />
                  <Icon
                    v-else
                    icon="fluent:lock-closed-16-filled"
                    class="w-4 h-4 text-amber-600 shrink-0"
                  />
                </h3>
                <p class="text-xs text-slate-500 mt-1 leading-relaxed">
                  {{ tile.desc }}
                </p>
              </div>

              <!-- Footer CTA -->
              <div class="pt-3 mt-4 border-t border-slate-200 flex items-center justify-between text-[11px] font-mono">
                <template v-if="tile.isLocked">
                  <span class="text-amber-800 font-bold flex items-center gap-1">
                    <Icon icon="fluent:lock-closed-16-filled" class="w-3.5 h-3.5 text-amber-600" />
                    Perlu Upgrade Lisensi
                  </span>
                  <span class="font-bold text-amber-700">&rarr;</span>
                </template>
                <template v-else>
                  <span class="text-slate-500 group-hover:text-slate-700">Buka Modul</span>
                  <span class="font-bold text-emerald-700">&rarr;</span>
                </template>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </AdminLayout>
</template>
