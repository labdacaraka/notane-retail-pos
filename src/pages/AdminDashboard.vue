<script setup lang="ts">
import { computed } from 'vue'
import { useRouter } from 'vue-router'
import AdminLayout from '@/layouts/AdminLayout.vue'
import { useAuthStore } from '@/stores/authStore'
import { ArrowUpRight } from 'lucide-vue-next'

const router = useRouter()
const authStore = useAuthStore()

const hubSections = computed(() => {
  const allSections = [
    {
      category: 'MASTER DATA ENTERPRISE',
      tiles: [
        {
          title: 'Master Outlet & Gerai',
          desc: 'Manajemen cabang outlet, penugasan Store Manager, Supervisor penanggung jawab & gudang',
          icon: 'fluent-color:building-store-24',
          path: '/admin/outlets',
          color: 'from-amber-500 to-orange-600',
          badge: 'Multi-Outlet',
          permission: 'master:read',
        },
        {
          title: 'Master Produk & SKU',
          desc: 'Katalog barang multi-satuan, barcode scanner, tiering harga grosir & HPP',
          icon: 'streamline-ultimate-color:warehouse-cart-packages-2',
          path: '/admin/products',
          color: 'from-amber-400 to-amber-600',
          badge: 'Katalog Barang',
          permission: 'master:read',
        },
        {
          title: 'Pelanggan & Member',
          desc: 'Registrasi pelanggan, tier membership, saldo reward poin & piutang',
          icon: 'fluent-color:people-team-24',
          path: '/admin/customers',
          color: 'from-blue-400 to-indigo-600',
          badge: 'Loyalti Member',
          permission: 'master:read',
        },
        {
          title: 'Voucher & Promo',
          desc: 'Kupon promo diskon persentase, nominal tetap & tanggal berlaku',
          icon: 'fluent-color:gift-card-24',
          path: '/admin/vouchers',
          color: 'from-rose-400 to-pink-600',
          badge: 'Promosi',
          permission: 'master:read',
        },
        {
          title: 'Parameter Retail',
          desc: 'Satuan UoM pecahan/desimal, kategori barang, merek & distributor',
          icon: 'fluent-color:settings-24',
          path: '/admin/parameters',
          color: 'from-teal-400 to-emerald-600',
          badge: 'Konfigurasi',
          permission: ['settings:manage', 'master:write'],
        },
      ],
    },
    {
      category: 'TRANSAKSI & OPERASIONAL BISNIS',
      tiles: [
        {
          title: 'Transaksi Penjualan',
          desc: 'Rekapitulasi penjualan kasir, riwayat faktur POS, cetak ulang struk thermal & void',
          icon: 'fluent-color:receipt-24',
          path: '/admin/sales',
          color: 'from-amber-400 to-amber-600',
          badge: 'Sales Audit',
          permission: 'report:view',
        },
        {
          title: 'Pengadaan & Pembelian',
          desc: 'Penerimaan faktur masuk (GRN), tagihan hutang supplier, PO & data pemasok',
          icon: 'streamline-ultimate-color:delivery-truck-cargo',
          path: '/admin/purchases',
          color: 'from-blue-500 to-indigo-600',
          badge: 'Procurement',
          permission: 'purchasing:read',
        },
        {
          title: 'Layar Kasir POS',
          desc: 'Buka terminal register kasir penjualan keyboard-driven F1-F12',
          icon: 'streamline-ultimate-color:shopping-cart-full',
          path: '/',
          color: 'from-emerald-400 to-teal-600',
          badge: 'Front-Office POS',
          permission: 'pos:checkout',
        },
        {
          title: 'Stok & Stock Opname',
          desc: 'Kartu stok, mutasi gudang, stock opname fisik & penyesuaian selisih',
          icon: 'streamline-ultimate-color:warehouse-storage-2',
          path: '/admin/inventory',
          color: 'from-orange-500 to-amber-600',
          badge: 'Warehouse & Opname',
          permission: 'inventory:read',
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
          badge: 'Thermal Receipt',
          permission: 'settings:manage',
        },
      ],
    },
    {
      category: 'KEAMANAN & MANAJEMEN SISTEM',
      tiles: [
        {
          title: 'Pengguna & Hak Akses',
          desc: 'Manajemen akun staf, assignment role, dan konfigurasi matriks hak akses RBAC per modul',
          icon: 'fluent-color:lock-shield-24',
          path: '/admin/users',
          color: 'from-indigo-500 to-purple-700',
          badge: 'RBAC Security',
          permission: 'users:manage',
        },
        {
          title: 'Jurnal & Laba Rugi',
          desc: 'Buku jurnal umum double-entry & laporan laba rugi real-time',
          icon: 'streamline-ultimate-color:accounting-calculator-1',
          path: '/admin/accounting',
          color: 'from-emerald-500 to-teal-700',
          badge: 'Accounting Ledger',
          permission: 'accounting:read',
        },
        {
          title: 'Jejak Audit 360°',
          desc: 'Perekaman log audit keamanan, otorisasi supervisor & aktivitas sistem',
          icon: 'fluent-color:shield-checkmark-24',
          path: '/admin/audit-logs',
          color: 'from-slate-600 to-slate-800',
          badge: 'Security Logs',
          permission: ['audit:read', 'report:view'],
        },
      ],
    },
  ]

  return allSections
    .map((sec) => ({
      ...sec,
      tiles: sec.tiles.filter((t) => !t.permission || authStore.hasPermission(t.permission)),
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
          <img src="/logo.png" alt="Notane Logo" class="w-16 h-16 rounded-xl shadow-sm border border-slate-200 shrink-0 hover:scale-105 transition" />
          <div class="space-y-1">
            <div class="flex items-center gap-2 text-emerald-700 text-xs font-mono font-bold uppercase tracking-wider">
              <Icon icon="streamline-ultimate-color:shop-sign-bag" class="w-4 h-4" />
              <span>Pusat Kendali Back-Office ERP</span>
            </div>
            <h1 class="text-2xl font-black text-slate-900 tracking-tight">
              Menu Utama <span class="text-notane-gradient">Notane Retail POS</span>
            </h1>
            <p class="text-xs text-slate-600 max-w-xl">
              Pilih modul navigasi di bawah ini untuk mengelola master data produk, member pelanggan, kupon promosi diskon, stok gudang, pembukuan akuntansi, dan audit trail.
            </p>
          </div>
        </div>

        <button
          @click="router.push('/')"
          class="bg-emerald-600 hover:bg-emerald-700 text-white font-bold text-xs px-5 py-3 rounded-xl shadow-sm flex items-center gap-2.5 transition active:scale-95 border border-emerald-700 cursor-pointer"
        >
          <Icon icon="streamline-ultimate-color:shopping-cart-full" class="w-5 h-5" />
          <span>Luncurkan Kasir POS (F1)</span>
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
              @click="router.push(tile.path)"
              class="group bg-white hover:bg-slate-50/80 border border-slate-300 hover:border-emerald-500 p-5 rounded-xl shadow-sm hover:shadow-md transition duration-150 cursor-pointer flex flex-col justify-between relative overflow-hidden active:scale-95"
            >
              <!-- Top Color Strip -->
              <div class="absolute top-0 left-0 right-0 h-1.5 bg-gradient-to-r" :class="tile.color"></div>

              <div>
                <!-- Tile Header -->
                <div class="flex items-center justify-between mb-4">
                  <div class="w-12 h-12 rounded-xl flex items-center justify-center bg-slate-50 border border-slate-200 group-hover:border-amber-400 group-hover:bg-amber-50/50 shadow-xs transition group-hover:scale-105">
                    <Icon :icon="tile.icon" class="w-7 h-7 drop-shadow-xs" />
                  </div>
                  <span class="text-[9px] font-mono font-bold text-slate-600 bg-slate-100 px-2.5 py-1 rounded-md border border-slate-200">
                    {{ tile.badge }}
                  </span>
                </div>

                <!-- Tile Title & Description -->
                <h3 class="text-sm font-bold text-slate-900 group-hover:text-emerald-700 transition flex items-center justify-between">
                  <span>{{ tile.title }}</span>
                  <ArrowUpRight class="w-4 h-4 opacity-0 group-hover:opacity-100 text-emerald-600 transition-opacity" />
                </h3>
                <p class="text-xs text-slate-500 mt-1 leading-relaxed">
                  {{ tile.desc }}
                </p>
              </div>

              <!-- Footer CTA -->
              <div class="pt-3 mt-4 border-t border-slate-200 flex items-center justify-between text-[11px] text-slate-500 group-hover:text-slate-700 font-mono">
                <span>Buka Modul</span>
                <span class="font-bold text-emerald-700">&rarr;</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  </AdminLayout>
</template>
