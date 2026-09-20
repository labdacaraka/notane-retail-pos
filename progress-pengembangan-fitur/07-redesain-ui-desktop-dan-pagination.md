# Progres 07: Redesain UI Desktop Native & Penerapan Sistem Paging Menyeluruh

Dokumen ini mencatat pencapaian refaktor antarmuka aplikasi dari gaya tata letak web (sidebar vertikal sisi kiri) menjadi tampilan khas aplikasi desktop retail/ERP modern dengan **Top Ribbon Tiles ("kotak-kotak di atas")** dan **Central Launcher Hub ("kotak-kotak di tengah")**, serta penerapan **komponen pagination mandiri di seluruh tabel data**.

---

## 1. Latar Belakang & Kebutuhan Pengguna

1. **Karakteristik Aplikasi Desktop:**
   - Aplikasi desktop native POS / Retail ERP (seperti SAP, Microsoft Dynamics, atau POS Desktop modern) umumnya tidak menggunakan sidebar vertikal web yang menyita lebar layar secara permanen.
   - Menu navigasi aplikasi desktop umumnya berbentuk:
     - **Ribbon Tiles Horizontal di bagian atas** (kotak-kotak menu fungsional dengan ikon dan indikator aktif).
     - **Central Dashboard Hub di bagian tengah** (kotak-kotak kartu launcher modular per departemen/kategori modul).
2. **Efisiensi Ruang Layar (Full-Width Data Workspace):**
   - Retail POS mengelola tabel data yang padat (Barcode, SKU, Nama Produk, HPP, Harga Jual, Stok, Satuan, TOP Distributor, Kontak, dll.). Menghilangkan sidebar kiri langsung membebaskan ~256px lebar layar horizontal sehingga tabel dapat dibaca dengan lega tanpa horizontal scrolling yang mengganggu kasir/operator.
3. **Pencegahan Beban Rendering Data (Pagination Mandatory):**
   - Data produk, mutasi kartu stok, jurnal umum, dan log audit di toko ritel dapat mencapai puluhan hingga ratusan ribu baris.
   - Diperlukan mekanisme pagination di seluruh tabel agar UI tetap ringan, instan, dan responsif.

---

## 2. Rincian Perubahan UI Desktop

### A. Redesain Tata Letak Desktop (`src/layouts/AdminLayout.vue`)
- **Penghapusan Sidebar Kiri:** Seluruh panel vertikal web kiri dihapus.
- **Desktop Top Application Bar:**
  - Logo merek & branding Notane Retail POS.
  - Quick action launcher "Buka Kasir POS (F1)" dengan badge status aktif.
  - Status konektivitas jaringan (Online / LAN Local).
  - Profil operator kasir aktif dan tombol logout aman.
- **Top Command Ribbon ("Kotak-kotak di Atas"):**
  - Baris kartu navigasi modular horizontal di bagian atas halaman dengan icon, label, deskripsi ringkas, dan glowing active dot:
    1. **Hub Utama** (`/admin`) - Launcher desktop.
    2. **Produk** (`/admin/products`) - Katalog & barcode master.
    3. **Parameter** (`/admin/parameters`) - Satuan, kategori, merek, supplier.
    4. **Pelanggan** (`/admin/customers`) - Member, poin loyalitas, limit kredit.
    5. **Voucher** (`/admin/vouchers`) - Diskon, kupon, promo kasir.
    6. **Inventori** (`/admin/inventory`) - Opname fisik, GRN penerimaan, kartu stok.
    7. **Akuntansi** (`/admin/accounting`) - Laba rugi, jurnal umum, bagan akun.
    8. **Audit Trail** (`/admin/audit-log`) - Rekam jejak supervisor & kasir.
- **Area Kerja Penuh (Full-Width Canvas):**
  - Konten utama memiliki ruang 100% lebar layar dengan scrollbar terisolasi dan padding yang proporsional untuk operasional desktop.

### B. Central Modular Launcher Hub (`src/pages/AdminDashboard.vue` - `/admin`)
- Menyediakan beranda desktop berformat **"Kotak-kotak di Tengah"** dengan modular cards:
  - **Seksi Master Data Enterprise:**
    - *Master Produk & Harga*: Manajemen katalog, barcode, multi-satuan & harga bertingkat.
    - *Parameter Retail*: Master satuan (UoM), kategori bertingkat, merek & supplier.
    - *Pelanggan & Member*: Grup pelanggan, limit piutang, poin reward loyalti.
    - *Voucher & Promosi*: Kupon diskon kasir, kuota penggunaan & validitas waktu.
  - **Seksi Operasional & Rantai Pasok:**
    - *Terminal Kasir POS*: Buka layar register kasir transaksi penjualan langsung.
    - *Inventori & Kartu Stok*: Stock opname fisik, penerimaan faktur supplier, kartu stok.
  - **Seksi Keuangan & Tata Kelola:**
    - *Pembukuan & Laporan*: Jurnal otomatis double-entry, laba rugi, bagan akun COA.
    - *Jejak Audit Keamanan*: Rekam jejak 360° aksi sensitif kasir & otorisasi supervisor.

---

## 3. Komponen Paging & Penerapan Menyeluruh

### A. Komponen Mandiri `PaginationBar.vue` (`src/components/common/PaginationBar.vue`)
Komponen pagination berdesain desktop native dengan spesifikasi:
- **Informasi Rentang Data:** `Menampilkan X - Y dari Z data` secara otomatis dihitung berdasarkan `currentPage` dan `pageSize`.
- **Pengaturan Baris per Halaman (Page Size Selector):** Pilihan 10, 25, 50, dan 100 data per halaman.
- **Navigasi Lengkap:**
  - Tombol Ke Halaman Pertama (`<<`)
  - Tombol Mundur 1 Halaman (`<`)
  - Deretan Tombol Angka Halaman Dinamis (dengan highlight aktif)
  - Tombol Maju 1 Halaman (`>`)
  - Tombol Ke Halaman Terakhir (`>>`)

### B. Daftar Tabel yang Telah Terpasang Pagination:

| No | Halaman | Tab / Entitas Tabel | State Pagination |
|:---|:---|:---|:---|
| 1 | `ProductsPage.vue` | Master Produk, Barcode, SKU & Stok | `currentPage`, `pageSize`, `paginatedProducts` |
| 2 | `CustomersPage.vue` | Data Pelanggan & Member VIP (Tab 1) | `custPage`, `custPageSize`, `paginatedCustomers` |
| 3 | `CustomersPage.vue` | Data Grup Pelanggan & Diskon (Tab 2) | `groupPage`, `groupPageSize`, `paginatedGroups` |
| 4 | `VouchersPage.vue` | Voucher Diskon, Kupon & Promo Kasir | `currentPage`, `pageSize`, `paginatedVouchers` |
| 5 | `ParametersPage.vue` | Master Satuan Barang / UoM (Tab 1) | `unitsPage`, `unitsPageSize`, `paginatedUnits` |
| 6 | `ParametersPage.vue` | Master Kategori Barang (Tab 2) | `categoriesPage`, `categoriesPageSize`, `paginatedCategories` |
| 7 | `ParametersPage.vue` | Master Merek / Prinsipal (Tab 3) | `brandsPage`, `brandsPageSize`, `paginatedBrands` |
| 8 | `ParametersPage.vue` | Master Supplier & Distributor (Tab 4) | `suppliersPage`, `suppliersPageSize`, `paginatedSuppliers` |
| 9 | `InventoryPage.vue` | Stock Opname Fisik Rak Toko | `opnamePage`, `opnamePageSize`, `paginatedOpnameItems` |
| 10 | `InventoryPage.vue` | Kartu Stok Barang (Stock Ledger) | `ledgerPage`, `ledgerPageSize`, `paginatedLedgerMutations` |
| 11 | `AccountingPage.vue` | Buku Jurnal Umum (General Ledger) | `jrnPage`, `jrnPageSize`, `paginatedJournalEntries` |
| 12 | `AccountingPage.vue` | Bagan Akun (Chart of Accounts - COA) | `coaPage`, `coaPageSize`, `paginatedCOA` |
| 13 | `AuditLogPage.vue` | Rekam Jejak Audit Keamanan Kasir | `page`, `pageSize`, `paginatedLogs` |

---

## 4. Hasil Verifikasi & Uji Kompilasi

Perintah verifikasi build:
```bash
npm run build
```
Output:
```
> notane-retail-pos@1.0.0 build
> vue-tsc && vite build

vite v6.4.3 building for production...
✓ 1691 modules transformed.
dist/index.html                   0.88 kB │ gzip:  0.49 kB
dist/assets/index-p9nwdvoR.css   37.85 kB │ gzip:  6.92 kB
dist/assets/index-Da9Qa7Ak.js   341.83 kB │ gzip: 99.60 kB
✓ built in 6.34s
```
- Kompilasi TypeScript (`vue-tsc`) : **0 error**.
- Bundle Vite Production: **Berhasil (1691 modul terkompilasi bersih)**.
