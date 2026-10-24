# Progress 58: Kelengkapan Seluruh Action Permissions Berdasarkan Endpoint API & Pemetaan RBAC Role

## Ringkasan Perubahan
Menyempurnakan seluruh sistem otorisasi dan kontrol akses (*Role-Based Access Control / RBAC*) dengan prinsip **1 Aksi / Endpoint = 1 Permission** di seluruh endpoint API Notane:
- Melakukan audit komprehensif pada seluruh 16 file API handler di backend Rust (`src-tauri/src/api/*.rs`).
- Mendaftarkan permission granular baru untuk seluruh operasi CRUD (`create`, `read`, `update`, `delete`, `void`, `approve`, `adjust`, dsb).
- Memetakan seluruh permissions ke dalam peran default sistem (**SUPERADMIN**, **STORE_MANAGER**, **SUPERVISOR**, **CASHIER**, **WAREHOUSE**) melalui migrasi database terintegrasi `043_complete_granular_action_permissions.sql`.
- Menyelaraskan middleware autentikasi dan otorisasi API backend dengan fallback yang aman (*fail-closed* & *backward compatible*).
- Memperbarui antarmuka pengguna back-office (`UsersPage.vue`) sehingga seluruh 27 modul fungsional memiliki label bahasa Indonesia yang ramah dan ikon Iconify yang selaras.

---

## 1. Pemetaan Modul & Action Permissions Lengkap

### A. Penjualan & Transaksi POS (`sales` & `pos`)
- **`sales:read`**: Melihat daftar, ringkasan, riwayat nota, dan analitik transaksi penjualan kasir.
- **`sales:create`**: Melakukan checkout dan pencatatan transaksi penjualan baru di meja kasir.
- **`sales:void`**: Otorisasi pembatalan atau void nota transaksi penjualan.
- **`sales:discount`**: Pemberian potongan diskon manual atau override harga item penjualan.
- **`sales:open_drawer`**: Otorisasi membuka laci meja kasir manual tanpa transaksi belanja.
- **`sales:all_ownership`**: Melihat seluruh transaksi penjualan dari semua staf dan kasir (tanpa batas kepemilikan data).

### B. Retur Penjualan Konsumen (`sales_returns`)
- **`sales_returns:read`**: Melihat data dan riwayat permohonan retur penjualan konsumen.
- **`sales_returns:create`**: Membuat permohonan retur penjualan dan penukaran barang konsumen di meja kasir.
- **`sales_returns:approve`**: Otorisasi persetujuan retur penjualan konsumen dan pengembalian uang.
- **`sales_returns:reject`**: Otorisasi penolakan permohonan retur penjualan konsumen.

### C. Shift Kasir & Uang Fisik Laci (`shift` & `cash_drops`)
- **`shift:read`**: Melihat riwayat, status shift aktif, dan ringkasan kas laci shift kasir.
- **`shift:open`**: Membuka register shift kasir baru.
- **`shift:close`**: Menutup shift kasir dan rekonsiliasi kas (blind settlement).
- **`cash_drops:read`**: Melihat riwayat serah terima penarikan kas laci meja kasir.
- **`cash_drops:create`**: Melakukan penarikan kas laci meja kasir ke brankas/supervisor.

### D. Pembelian & Pengadaan (`purchasing`)
- **`purchasing:read`**: Melihat data pesanan pembelian (PO), bukti terima barang (GRN), dan faktur supplier.
- **`purchasing:create`**: Membuat pesanan pembelian (PO), bukti terima barang (GRN), dan faktur supplier baru.
- **`purchasing:update`**: Mengubah pesanan pembelian dan memproses penerimaan barang (GRN) ke faktur pembelian.
- **`purchasing:pay`**: Melakukan pelunasan dan pembayaran utang faktur pembelian supplier.
- **`purchasing:void`**: Otorisasi pembatalan atau void PO, bukti penerimaan barang (GRN), dan faktur pembelian.
- **`purchasing:all_ownership`**: Melihat seluruh data pembelian dan faktur supplier dari semua staf.

### E. Stok & Mutasi Inventori (`inventory`)
- **`inventory:read`**: Melihat data saldo stok, mutasi kartu stok, dan riwayat mutasi barang.
- **`inventory:adjust`**: Melakukan penyesuaian stok opname manual.
- **`inventory:transfer`**: Membuat mutasi pengiriman dan konfirmasi penerimaan transfer stok antar gerai.
- **`inventory:transfer_cancel`**: Otorisasi pembatalan mutasi pengiriman transfer stok barang.
- **`inventory:costing`**: Mengatur konfigurasi metode perhitungan HPP (FIFO/Moving Average).
- **`inventory:all_ownership`**: Melihat seluruh mutasi dan transfer stok dari semua staf.

### F. Pengeluaran & Setoran Modal (`expenses` & `capital`)
- **`expenses:read`**: Melihat daftar, ringkasan, dan rincian transaksi pengeluaran operasional.
- **`expenses:create`**: Mencatat transaksi pengeluaran kas operasional dan petty cash.
- **`expenses:update`**: Mengubah data kategori atau keterangan transaksi pengeluaran kas.
- **`expenses:delete`**: Membatalkan atau menghapus transaksi pengeluaran operasional.
- **`expenses:categories`**: Mengelola master kategori akun pengeluaran operasional.
- **`expenses:all_ownership`**: Melihat seluruh transaksi pengeluaran dari semua staf.
- **`capital:read`**: Melihat riwayat dan ringkasan transaksi setoran modal pemilik.
- **`capital:create`**: Mencatat transaksi setoran modal usaha ke kas/bank.
- **`capital:delete`**: Membatalkan transaksi setoran modal usaha.

### G. Rekening Bank Usaha (`bank`)
- **`bank:read`**: Melihat daftar rekening bank penerimaan usaha.
- **`bank:create`**: Menambahkan rekening bank penerimaan usaha baru.
- **`bank:update`**: Mengubah informasi rekening bank penerimaan usaha.
- **`bank:delete`**: Menonaktifkan atau menghapus rekening bank penerimaan.

### H. Promosi, Voucher & Flash Sale (`promotions` & `vouchers`)
- **`promotions:read`**: Melihat daftar program promo otomatis ritel dan flash sale.
- **`promotions:create`**: Membuat program promo ritel baru atau jadwal event flash sale.
- **`promotions:update`**: Mengubah konfigurasi program promo atau event flash sale.
- **`promotions:delete`**: Menghapus atau menghentikan program promo dan flash sale.
- **`vouchers:read`**: Melihat daftar kupon voucher diskon dan memeriksa validitas kode voucher kasir.
- **`vouchers:create`**: Membuat kode kupon voucher diskon promosi toko baru.
- **`vouchers:update`**: Mengubah syarat, nominal, dan ketentuan kupon voucher diskon.
- **`vouchers:delete`**: Menghapus kupon voucher diskon promosi toko.

### I. Loyalitas & Poin Member (`loyalty`)
- **`loyalty:read`**: Melihat konfigurasi loyalitas dan saldo poin member pelanggan.
- **`loyalty:manage`**: Mengatur nilai konversi rupiah poin, rasio perolehan, dan batas penukaran.
- **`loyalty:adjust`**: Melakukan koreksi atau penyesuaian manual saldo poin member pelanggan.
- **`loyalty:redeem`**: Otorisasi penggunaan dan penukaran saldo poin reward belanja di kasir.

### J. Master Data Produk, Pelanggan, Supplier, Kategori, Merek, Satuan, Outlet
- **`products:read`**, **`products:create`**, **`products:update`**, **`products:delete`**, **`products:import`**, **`products:barcodes`**, **`products:outlet_prices`**, **`products:customer_group_prices`**, **`products:price_tiers`**
- **`customers:read`**, **`customers:create`**, **`customers:update`**, **`customers:delete`**
- **`suppliers:read`**, **`suppliers:create`**, **`suppliers:update`**, **`suppliers:delete`**
- **`categories:read`**, **`categories:create`**, **`categories:update`**, **`categories:delete`**
- **`brands:read`**, **`brands:create`**, **`brands:update`**, **`brands:delete`**
- **`units:read`**, **`units:create`**, **`units:update`**, **`units:delete`**
- **`outlets:read`**, **`outlets:create`**, **`outlets:update`**, **`outlets:delete`**

### K. Akuntansi, Laporan & Pengaturan Sistem
- **`accounting:read`**, **`accounting:create`**, **`accounting:update`**, **`accounting:close_period`**, **`accounting:reopen_period`**, **`accounting:all_ownership`**
- **`report:read`**, **`report:view`**
- **`settings:read`**, **`settings:update`**, **`settings:biometric`**
- **`users:read`**, **`users:create`**, **`users:update`**, **`users:delete`**, **`users:change_password`**, **`users:change_pin`**, **`users:impersonate`**
- **`supervisor:biometric_enroll`**, **`supervisor:biometric_override`**

---

## 2. Pemetaan Hak Akses per Peran Default (*Default Roles*)

| Role | Total Hak Akses | Cakupan Otorisasi & Tanggung Jawab |
| :--- | :---: | :--- |
| **`SUPERADMIN`** | **124 Hak Akses (100%)** | Akses mutlak ke seluruh fitur, modul, konfigurasi back-office, database, dan pengguna. |
| **`STORE_MANAGER`** | **53 Hak Akses** | Manajemen operasional toko, seluruh transaksi (`all_ownership`), persetujuan retur, inventori, pembelian, promosi, voucher, kas operasional, master data, dan laporan analitik lengkap. |
| **`SUPERVISOR`** | **34 Hak Akses** | Otorisasi meja kasir (`sales:void`, `sales:discount`, `sales:open_drawer`), persetujuan retur konsumen (`sales_returns:approve`), transfer stok gerai, pencatatan biaya kas kecil, shift, dan biometrik override. |
| **`CASHIER`** | **12 Hak Akses** | Fokus operasional kasir: checkout (`sales:create`), riwayat penjualan kasir (`sales:read`), pengajuan retur (`sales_returns:create`), buka/tutup shift, tarik kas laci, klaim poin reward, dan cek voucher diskon. |
| **`WAREHOUSE`** | **8 Hak Akses** | Penerimaan barang (GRN), pesanan pembelian supplier (PO), penyesuaian stok opname, mutasi transfer stok, dan katalog barang. |

---

## 3. Verifikasi & Pengujian
- **Migrasi Database**: `043_complete_granular_action_permissions` dieksekusi sukses saat server start (Batch 4).
- **Kompilasi Rust**: Lulus uji kompilasi `cargo check` pada `notane-server` dan `notane-client` dengan 0 error.
- **Antarmuka Pengguna (UI)**: Halaman Manajemen Pengguna (`http://localhost:5175/admin/users`) pada tab **Peran & Hak Akses** menampilkan matriks izin yang lengkap, rapi, dan mudah dikonfigurasi per peran.
