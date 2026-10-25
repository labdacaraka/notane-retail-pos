# Dokumentasi Progress: Multi-Outlet Pricing, Mode Impersonate, Hak Akses Kasir, dan Realtime Customer Display

Dokumen ini mencatat pembaruan dan implementasi fitur-fitur enterprise baru pada Notane Retail POS:
1. Multi-Outlet Pricing (Harga Jual & HPP Berbeda Tiap Cabang/Toko) dengan isolasi hak akses lokasi.
2. Mode Impersonate Pengguna (`users:impersonate`) untuk audit dan supervisi tanpa perlu login-logout berulang.
3. Proteksi tombol kasir di back-office berdasarkan izin RBAC `pos:checkout`.
4. Arsitektur IPC Native & Realtime Customer Display (Layar Hadap Pelanggan) berbasis `BroadcastChannel`.

---

## 1. Multi-Outlet Pricing (`product_outlet_prices`)

### 1.1 Skema Database & Migrasi
- Berkas migrasi: `src-tauri/migrations/009_product_outlet_prices.sql`.
- Tabel `product_outlet_prices`:
  - `id`: UUID (Primary Key).
  - `outlet_id`: UUID (Foreign Key ke `outlets.id` ON DELETE CASCADE).
  - `product_id`: UUID (Foreign Key ke `products.id` ON DELETE CASCADE).
  - `cost_price`: NUMERIC(18, 4) (HPP khusus outlet tersebut).
  - `base_selling_price`: NUMERIC(18, 4) (Harga jual dasar khusus outlet tersebut).
  - `created_at`, `updated_at`, `created_by`, `updated_by`.
  - Constraint: `UNIQUE(outlet_id, product_id)`.
  - Index pencarian cepat: `idx_prod_outlet_prices_lookup` dan `idx_prod_outlet_prices_outlet`.
- Didaftarkan pada sistem migrasi otomatis embedded di `src-tauri/src/db/mod.rs`.

### 1.2 Backend API & Handler
- CRUD Handler di `src-tauri/src/api/master.rs`:
  - `GET /api/v1/products/{id}/outlet-prices`: Mengambil daftar harga outlet untuk produk tertentu.
  - `POST /api/v1/products/{id}/outlet-prices`: Menyimpan / memperbarui harga per outlet (UPSERT).
  - `DELETE /api/v1/products/outlet-prices/{id}`: Menghapus pengaturan harga khusus outlet (kembali ke harga dasar global).
- Resolusi Harga di POS:
  - `pos_service.rs` pada pencarian barcode otomatis menimpa `base_selling_price` dan `cost_price` jika ditemukan record harga khusus pada outlet transaksi.
  - `master.rs` pada endpoint `GET /api/v1/products`: Ditambahkan parameter query `outlet_id` dan `limit`. Melakukan `LEFT JOIN product_outlet_prices` dan menggunakan `COALESCE(pop.base_selling_price, p.base_selling_price)` sehingga katalog barang di halaman kasir langsung menyajikan harga yang sesuai dengan outlet aktif.

### 1.3 Kontrol Akses Berdasarkan Hak Akses Pengguna
- Di frontend `src/pages/ProductsPage.vue`:
  - Modal "Harga Per Outlet" hanya menampilkan daftar toko/cabang yang diizinkan untuk pengguna saat ini (berdasarkan relasi `user_accessible_outlets` dari `locationStore`).
  - Pengguna dengan izin 1 lokasi tidak dapat melihat maupun mengubah harga outlet lain yang bukan wewenangnya.
- Di frontend `src/components/pos/BarcodeBar.vue` & `src/components/pos/ProductSearchModal.vue`:
  - Pindai barcode dan dropdown autocomplete mengirimkan `outlet_id` aktif dari store, memastikan harga yang tampil di pencarian dan saat ditambahkan ke keranjang konsisten dengan outlet yang sedang beroperasi.

---

## 2. Mode Impersonate Pengguna (`users:impersonate`)

### 2.1 Skema & Izin Keamanan
- Berkas migrasi: `src-tauri/migrations/010_impersonate_permission.sql`.
- Menambahkan kode permission baru: `users:impersonate` (Kategori Pengguna & Keamanan).
- Otomatis di-grant kepada role `SUPER_ADMIN` dan `ADMINISTRATOR`.

### 2.2 Backend Impersonation
- Handler `POST /api/v1/users/{id}/impersonate` di `src-tauri/src/api/users.rs`:
  - Validasi hak akses (`users:impersonate` atau Super Admin).
  - Verifikasi target user aktif dan bukan diri sendiri.
  - Menghasilkan token JWT baru dengan claims `impersonated_by: Option<Uuid>`.
  - Mencatat aktivitas impersonate ke tabel audit log.

### 2.3 Frontend Impersonation UX
- Store `src/stores/authStore.ts`:
  - State `isImpersonating` dan `impersonator` (menyimpan sesi akun asli).
  - Fungsi `impersonateUser(targetId)` dan `stopImpersonating()`.
- Banner Notifikasi Melayang:
  - Ditambahkan di `src/layouts/AdminLayout.vue` dan `src/layouts/PosLayout.vue`.
  - Menampilkan nama akun yang sedang diimpersonate beserta perannya, dengan tombol *"Kembali ke Akun Asli"* yang selalu terlihat.
- Perbaikan Icon:
  - Menggunakan icon terdaftar yang valid: `fluent-color:people-sync-20` / `fluent-color:people-sync-24` dan `flat-color-icons:leave`.
- Pembersihan UI:
  - Tombol "Kembali" yang redundan pada header `src/pages/UsersPage.vue` telah dibersihkan.

---

## 3. Perlindungan Tombol Kasir POS

- Tombol *"Buka Layar Kasir POS (F1)"* di header back-office (`src/layouts/AdminLayout.vue`) kini dilindungi dengan `v-if="authStore.hasPermission('pos:checkout')"`.
- Pengguna yang masuk dengan role tanpa akses kasir (misal Auditor, Purchasing, Staf Gudang) tidak lagi melihat tombol tersebut, mencegah klik sia-sia atau kebingungan pengguna.

---

## 4. Realtime Customer Display (Layar Hadap Pelanggan)

### 4.1 Tantangan & Akar Masalah
- Pada lingkungan desktop native Tauri v2 (WebView2 di Windows), pemanggilan `window.open` standar diabaikan oleh webview runtime sehingga klik tombol tidak membuka jendela apa pun.
- Antara jendela utama (Kasir) dan jendela kedua (Customer Display), instance memori JavaScript/Pinia terisolasi satu sama lain, sehingga data keranjang tidak tersinkronisasi tanpa saluran komunikasi IPC.

### 4.2 Solusi Multi-Window Desktop Native
- `src-tauri/tauri.conf.json`:
  - Didaftarkan secondary window `customer-display` dengan konfigurasi:
    - `url`: `/customer-display`
    - `width`: 1024, `height`: 600, `resizable`: true, `visible`: false (preloaded).
- `src-tauri/src/tauri_cmds/mod.rs` & `src-tauri/src/lib.rs`:
  - Dibuat command Tauri `open_customer_display_window`:
    - Mengambil window `customer-display` yang ada, memunculkannya (`show()`), unminimize, dan set focus.
    - Jika jendela telah ditutup (destroyed), otomatis dibangun kembali melalui `WebviewWindowBuilder`.

### 4.3 Saluran Sinkronisasi Realtime (`customerDisplayChannel.ts`)
- Berkas baru: `src/services/customerDisplayChannel.ts`.
- Menggunakan arsitektur `BroadcastChannel` berlatensi sub-milidetik dengan replikasi storage-event mirroring (`localStorage`).
- Bekerja secara universal:
  1. Multi-window desktop native Tauri pada monitor kedua.
  2. Jendela browser terpisah pada monitor kedua.
  3. Tablet / smartphone pelanggan yang membuka URL `/customer-display` melalui LAN.
- Di `src/stores/cartStore.ts`:
  - Ditambahkan **deep watcher** yang memantau perubahan `items`, `selectedCustomer`, `voucherCode`, dan `voucherDiscount`.
  - Setiap kali kasir memindai barang, mengubah kuantitas, menerapkan diskon, atau memilih member, data keranjang langsung disiarkan ke Customer Display seketika itu juga.
  - Menanggapi sinyal `REQUEST_SYNC` saat layar pelanggan baru saja dibuka agar langsung menampilkan kondisi keranjang terkini tanpa menunggu pemindaian berikutnya.
- Di `src/components/pos/QuickPayModal.vue`:
  - Menyiarkan event `broadcastPaymentSuccess` saat transaksi checkout berhasil, menyertakan no. faktur, nominal bayar, metode bayar, dan nominal kembalian.

### 4.4 Redesain Layar Pelanggan (`src/pages/CustomerDisplay.vue`)
- Desain modern dark mode dengan tipografi tegas dan rasio kontras tinggi yang nyaman dilihat pelanggan dari kejauhan.
- **3 Kondisi Tampilan Reaktif**:
  1. **Kondisi Standby (Keranjang Kosong)**:
     - Sambutan selamat datang, logo toko, dan indikator status kesiapan kasir melayani.
  2. **Kondisi Belanja Aktif (Item Terpindai)**:
     - Daftar belanjaan transparan dengan sorotan khusus *"Baru Dipindai"* pada barang teratas.
     - Rincian kuantitas, satuan, harga per unit, diskon per barang, dan subtotal.
     - Badge pelanggan member jika terdaftar.
     - Rincian hemat diskon, informasi PPN, dan angka Total Pembayaran berukuran raksasa.
  3. **Kondisi Transaksi Lunas**:
     - Banner animasi perayaan transaksi sukses.
     - Rincian total belanja dan nominal yang dibayarkan.
     - **Kartu Kembalian Raksasa** yang mencolok dengan informasi uang kembalian yang harus diterima pelanggan.

---

## 5. Ringkasan File yang Dimodifikasi & Dibuat

| File | Status | Keterangan |
|---|---|---|
| `src-tauri/migrations/009_product_outlet_prices.sql` | Dibuat | Skema tabel harga produk per outlet |
| `src-tauri/migrations/010_impersonate_permission.sql` | Dibuat | Skema permission mode impersonate |
| `src/services/customerDisplayChannel.ts` | Dibuat | Saluran realtime sync & IPC window Customer Display |
| `src-tauri/tauri.conf.json` | Dimodifikasi | Konfigurasi secondary window `customer-display` |
| `src-tauri/src/tauri_cmds/mod.rs` | Dimodifikasi | Command IPC `open_customer_display_window` |
| `src-tauri/src/lib.rs` | Dimodifikasi | Pendaftaran command IPC baru |
| `src-tauri/src/api/master.rs` | Dimodifikasi | CRUD harga outlet & resolusi harga di list produk |
| `src-tauri/src/api/users.rs` | Dimodifikasi | Endpoint `POST /api/v1/users/{id}/impersonate` |
| `src-tauri/src/db/mod.rs` | Dimodifikasi | Registrasi migrasi skema database 009 & 010 |
| `src/stores/cartStore.ts` | Dimodifikasi | Deep watcher realtime sync ke Customer Display & filter outlet |
| `src/stores/authStore.ts` | Dimodifikasi | State & method impersonate user |
| `src/pages/CustomerDisplay.vue` | Dimodifikasi | Redesain lengkap layar hadap pelanggan realtime |
| `src/pages/ProductsPage.vue` | Dimodifikasi | Modal & isolasi pengaturan harga per outlet |
| `src/pages/UsersPage.vue` | Dimodifikasi | Tombol impersonate & pembersihan tombol kembali |
| `src/components/pos/BarcodeBar.vue` | Dimodifikasi | Pindai barcode & dropdown harga spesifik outlet |
| `src/components/pos/ProductSearchModal.vue` | Dimodifikasi | Pencarian produk harga spesifik outlet |
| `src/components/pos/QuickPayModal.vue` | Dimodifikasi | Broadcast payment success & kembalian ke customer display |
| `src/layouts/AdminLayout.vue` | Dimodifikasi | Proteksi tombol kasir & banner impersonate |
| `src/layouts/PosLayout.vue` | Dimodifikasi | Tombol Customer Display native & banner impersonate |
