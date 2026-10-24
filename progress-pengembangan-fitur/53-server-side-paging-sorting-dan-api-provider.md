# Progress 53: Server-Side Paging, Sorting, Filtering & REST API Management Studio

## 1. Latar Belakang & Masalah
Saat mencoba aplikasi Notane Provider Studio (`notane-manager`) pada web browser:
1. Ketika menambah klien/tenant baru, UI menampilkan notifikasi sukses namun data tidak muncul di tabel.
   - **Penyebab:** Pada mode browser, Tauri IPC (`window.__TAURI_INTERNALS__`) bernilai `undefined`. Sebelumnya `invokeTauri` hanya mensimulasikan pemanggilan kosong (`return {} as T`) tanpa persistensi database SQLite nyata.
2. Seluruh tabel administrasi (Klien, Lisensi, Perangkat HWID, Rilis OTA, dan Audit Log) belum memiliki mekanisme **server-side paging**, **server-side sorting**, dan **filtering/searching** yang terintegrasi secara dinamis ke database.

## 2. Solusi & Perubahan Arsitektur

### A. Backend Rust Axum REST API Management (`notane-manager/src-tauri`)
1. **Model Paginasi & Pengurutan (`models/mod.rs`):**
   - Menambahkan `PaginationParams` (`page`, `page_size`, `sort_by`, `sort_order`, `search`, `status`, `tier`).
   - Menambahkan `PaginatedResponse<T>` (`items`, `page`, `page_size`, `total_items`, `total_pages`).
2. **Router Admin Lengkap (`api/admin.rs` & `api/mod.rs`):**
   - **Autentikasi & Sesi:** `/api/v1/admin/auth/login`, `/auth/pin`, `/auth/verify`, `/auth/logout`, `/auth/change-credentials`.
   - **Status & Server:** `/api/v1/admin/status`, `/server/toggle`, `/settings`.
   - **Klien Tenant:** `/api/v1/admin/clients` (GET dengan paging, sorting, search; POST, PUT, DELETE).
   - **Lisensi:** `/api/v1/admin/licenses` (GET dengan paging, sorting, search, filter tier/status; POST, PUT status, DELETE).
   - **Perangkat HWID:** `/api/v1/admin/devices` (GET dengan paging, sorting, search, filter aktif/nonaktif; DELETE unbind).
   - **Rilis OTA:** `/api/v1/admin/releases` (GET dengan paging, sorting, search; POST dengan Ed25519 auto-signature, DELETE).
   - **Audit Logs:** `/api/v1/admin/audit-logs` (GET dengan paging, sorting, search, filter status/aksi).
   - **Offline Signer:** `/api/v1/admin/sign-token` (POST penandatanganan token offline Ed25519).

### B. Frontend Service & Pinia Store (`notane-manager/src`)
1. **Axios Client Terpadu (`services/api.ts`):**
   - Otomatis menghubungkan frontend (port 5177) ke provider API engine (port 19090).
   - Menginjeksi `Authorization: Bearer <token>` pada setiap permintaan.
2. **Upgrade Pinia Store (`stores/providerStore.ts` & `authStore.ts`):**
   - Menyediakan state paginasi (`clientsPagination`, `licensesPagination`, `devicesPagination`, `releasesPagination`, `auditLogsPagination`).
   - Mendukung pemanggilan data berbasis query params server-side.
3. **Komponen Paginasi Reusable (`components/common/PaginationBar.vue`):**
   - Indikator: "Menampilkan {from} - {to} dari {total} data".
   - Pilihan limit per halaman (10, 25, 50, 100).
   - Navigasi tombol Pertama, Sebelumnya, Angka Halaman, Selanjutnya, Terakhir.

### C. Pembaruan Halaman / View
1. **`ClientsView.vue`:**
   - Tabel responsif server-side dengan sortable column header (Nama Klien, Perusahaan, Email, Tanggal Terdaftar).
   - Input pencarian server-side dengan debounce otomatis.
   - Pilihan tampilan: Tabel dan Grid Kartu.
   - Modal Tambah Klien dan Edit Klien.
2. **`LicensesView.vue`:**
   - Server-side sorting (Nomor Lisensi, Klien, Tier, Kuota Kasir, Status, Kadaluarsa).
   - Filter cepat tab status (ALL, ACTIVE, SUSPENDED, EXPIRED) dan dropdown Tier.
   - Integrasi tombol aksi Sign Token Offline, Suspend/Aktifkan, dan Hapus.
3. **`DevicesView.vue`:**
   - Server-side sorting (Hardware ID, Nama Terminal, Lisensi, Klien, Ping Terakhir, Status).
   - Filter status perangkat aktif/nonaktif dan tombol reset/unbind HWID slot.
4. **`ReleasesView.vue`:**
   - Server-side sorting & search untuk katalog rilis OTA.
   - Auto-generate signature Ed25519 dan SHA-256 check.
5. **`AuditLogsView.vue`:**
   - Server-side sorting & search untuk audit log keamanan dan percobaan aktivasi.

## 3. Hasil Pengujian & Verifikasi
1. **Kompilasi Rust:** `cargo build` dan `cargo check` lulus tanpa error.
2. **Kompilasi Frontend:** `npm --prefix notane-manager run build` (`vue-tsc && vite build`) lulus 100% tanpa error.
3. **Uji Endpoint:**
   - `POST /api/v1/admin/auth/login`: Mengembalikan token dan daftar permissions lengkap.
   - `POST /api/v1/admin/clients`: Sukses menyimpan klien "Ahmad Pratama" (PT Riang Jaya Sentosa).
   - `GET /api/v1/admin/clients`: Mengembalikan data terpaginasi (`total_items: 1`).
   - `POST /api/v1/admin/licenses`: Sukses menerbitkan lisensi `NTN-ENT-2026-X2998`.
   - `POST /api/v1/license/activate`: Sukses aktivasi online dari mesin kasir dengan hardware ID `TEST-HWID-MAC-M1-8899`.
   - `GET /api/v1/admin/devices`: Otomatis menampilkan perangkat kasir yang terikat.
   - `GET /api/v1/admin/audit-logs`: Otomatis mencatat riwayat audit aktivasi dengan status `SUCCESS`.
   - `POST /api/v1/admin/sign-token`: Berhasil menerbitkan signature token offline Ed25519.
