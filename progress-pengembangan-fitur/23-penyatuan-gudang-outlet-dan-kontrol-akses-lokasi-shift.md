# 23 - Penyatuan Gudang ke Tabel Outlets, Hak Akses Multi-Lokasi, dan Penguncian Shift Kasir

**Tanggal**: 2026-09-21  
**Status**: ✅ SELESAI  
**Kategori**: POS, Back-Office & Database Architecture

---

## Ringkasan Implementasi

Pembaruan arsitektural menyeluruh pada struktur organisasi multi-lokasi sistem POS:
1. **Penyatuan Tabel Gudang & Outlet**: Tabel `warehouses` dilebur ke dalam tabel `outlets` menggunakan kolom boolean `is_warehouse`. Pola ini memfasilitasi relasi multi-cabang di mana 1 gudang pusat/suplai dapat melayani banyak gerai toko/outlet ritel.
2. **Kontrol Akses Multi-Lokasi Pengguna (`user_outlets`)**: Setiap akun staf/kasir dapat dibatasi hanya boleh mengakses outlet/cabang tertentu. Akun dengan peran `SUPERADMIN` secara otomatis memiliki akses penuh ke seluruh lokasi.
3. **Location Switcher Back-Office**: Pengguna back-office dapat beralih lokasi kerja secara langsung melalui switcher di header navigasi, sehingga katalog dan stok barang otomatis menyesuaikan lokasi aktif.
4. **Penguncian Lokasi Shift Kasir POS**: Kasir diwajibkan memilih lokasi cabang saat membuka shift (`open_shift`). Setelah dibuka, lokasi kasir terkunci dan seluruh perhitungan stok dan harga produk saat pemindaian otomatis merujuk ke gudang penyuplai cabang shift tersebut.

---

## Fitur yang Diimplementasikan

### 1. Migrasi Skema Database
- **`005_outlets_and_permissions.sql`**: Master outlet toko, penugasan `manager_id`, `default_warehouse_id`, tabel relasi `outlet_supervisors`, serta permission `outlets:read` dan `outlets:manage`.
- **`006_decouple_warehouses_and_link_outlets.sql`**: Menghubungkan transaksi `sales`, `purchase_orders`, `purchase_invoices` langsung ke tabel `outlets`.
- **`007_unify_warehouses_into_outlets.sql`**: Menyatukan tabel `warehouses` ke dalam tabel `outlets` dengan kolom `is_warehouse BOOLEAN`, memperbarui seluruh Foreign Key (`warehouse_stocks`, `stock_batches`, `stock_mutations`, dll.), dan menyediakan backward-compatible VIEW `warehouses`.
- **`008_user_outlets_access_control.sql`**: Membuat tabel relasi `user_outlets (id, user_id, outlet_id)` untuk pembatasan akses outlet per pengguna.

### 2. Backend Engine (Rust Axum)
- **`GET /api/v1/me/outlets`**: Mengembalikan daftar outlet yang diizinkan untuk pengguna yang sedang login (semua lokasi untuk `SUPERADMIN`, atau lokasi terpilih pada tabel `user_outlets`).
- **Pembaruan Payload Pengguna (`api/users.rs`)**:
  - `POST /api/v1/users` & `PUT /api/v1/users/{id}` menerima array `outlet_ids` untuk menyinkronkan hak akses cabang pengguna.
  - `GET /api/v1/users` mengembalikan `outlet_ids` dan array objek `outlets` lengkap.
- **Validasi Hak Akses Kasir pada Shift (`services/pos_service.rs`)**:
  - Memverifikasi apakah kasir memiliki izin ke `outlet_id` yang dipilih saat membuka shift. Permintaan ditolak jika user tidak berizin.
  - Enriching struct `CashierShift` dengan metadata `outlet_id`, `outlet_name`, `outlet_code`, dan `default_warehouse_id`.

### 3. Frontend Store & UI Back-Office
- **`locationStore.ts`**: Mengelola state outlet aktif, daftar outlet yang dapat diakses pengguna, filter cabang toko vs gudang, dan sinkronisasi ke `cartStore.setOutlet()`.
- **Location Switcher Navbar (`AdminLayout.vue`)**: Dropdown pemilihan lokasi di header admin yang menampilkan nama dan kode outlet aktif.
- **Master Pengguna (`UsersPage.vue`)**:
  - Kolom **"Akses Lokasi"** di tabel pengguna dengan badge *"Semua Lokasi"* untuk SUPERADMIN atau pil kode outlet yang diizinkan.
  - Bagian **"Hak Akses Outlet & Lokasi"** di form modal pengguna dengan tombol cepat *"Pilih Semua"* dan *"Kosongkan"*, serta checklist outlet interaktif.

### 4. Kasir POS & Shift Aktif
- **Modal Buka Shift (`ShiftModal.vue`)**:
  - Kasir memilih **Cabang / Lokasi Shift Kasir** dari daftar cabang yang diizinkan baginya.
- **Header POS (`PosLayout.vue`)**:
  - Menampilkan badge lokasi cabang yang terkunci: `📍 [KODE-OUTLET] Nama Outlet (Shift Aktif)`.
  - Kasir tidak dapat mengganti lokasi selama shift aktif berjalan.
- **Katalog & Stok Kasir (`cartStore.ts`)**:
  - Pemindaian barcode otomatis terhubung ke gudang penyuplai shift kasir (`/products/barcode/:barcode?warehouse_id=...`).

---

## File yang Dibuat / Dimodifikasi

1. **Database Migrations**:
   - `src-tauri/migrations/005_outlets_and_permissions.sql` [BARU]
   - `src-tauri/migrations/006_decouple_warehouses_and_link_outlets.sql` [BARU]
   - `src-tauri/migrations/007_unify_warehouses_into_outlets.sql` [BARU]
   - `src-tauri/migrations/008_user_outlets_access_control.sql` [BARU]
   - `src-tauri/migrations/002_seed_data.sql` [MODIFIKASI]
   - `src-tauri/src/db/mod.rs` [MODIFIKASI]
2. **Backend**:
   - `src-tauri/src/models/outlet.rs` [BARU]
   - `src-tauri/src/models/shift.rs` [MODIFIKASI]
   - `src-tauri/src/api/outlets.rs` [BARU]
   - `src-tauri/src/api/users.rs` [MODIFIKASI]
   - `src-tauri/src/api/mod.rs` [MODIFIKASI]
   - `src-tauri/src/services/pos_service.rs` [MODIFIKASI]
3. **Frontend**:
   - `src/stores/locationStore.ts` [BARU]
   - `src/stores/shiftStore.ts` [MODIFIKASI]
   - `src/stores/cartStore.ts` [MODIFIKASI]
   - `src/types/index.ts` [MODIFIKASI]
   - `src/pages/OutletsPage.vue` [BARU]
   - `src/pages/UsersPage.vue` [MODIFIKASI]
   - `src/components/pos/ShiftModal.vue` [MODIFIKASI]
   - `src/layouts/AdminLayout.vue` [MODIFIKASI]
   - `src/layouts/PosLayout.vue` [MODIFIKASI]
   - `src/router/index.ts` [MODIFIKASI]
