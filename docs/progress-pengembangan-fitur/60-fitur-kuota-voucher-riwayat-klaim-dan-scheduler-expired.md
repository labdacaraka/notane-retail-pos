# 60. Fitur Kuota Voucher, Validasi Checkout Kasir, Riwayat Klaim, dan Scheduler Expired Otomatis

## Tanggal Pelaksanaan
12 Oktober 2026

## Ringkasan Kebutuhan & Solusi
Pengguna meminta pengembangan komprehensif pada fitur Voucher Diskon & Promosi (`/admin/vouchers`):
1. **Penambahan Field Kuota (`quota`)**:
   - Kolom `quota` (kuota pemakaian maksimal voucher) dan `used_count` (jumlah voucher telah diklaim transaksi).
   - Nilai `0` menandakan kuota tanpa batas (*unlimited*). Nilai `> 0` membatasi pemakaian sebanyak angka tersebut.
2. **Validasi Kuota di Halaman Kasir & Backend Checkout**:
   - **Pre-check di Cart Store**: Ketika kasir menerapkan voucher promo pada keranjang belanja, sistem memverifikasi kuota tersisa. Jika habis (`quota > 0 && used_count >= quota`), penambahan ditolak.
   - **Zero-Trust Backend Validation**: Endpoint `/vouchers/validate/{code}` serta proses atomik `create_sale` di `pos_service.rs` memvalidasi kuota dan masa berlaku voucher.
   - **Atomisitas Checkout DB**: Ketika transaksi berhasil disimpan (`commit`), `used_count` bertambah 1 (`used_count = used_count + 1`). Jika kuota tercapai, status voucher otomatis diubah menjadi `USED`.
3. **Tombol Aksi Riwayat Klaim Transaksi**:
   - Di tabel voucher (`VouchersPage.vue`), ditambahkan tombol riwayat klaim dengan icon `History`.
   - Menampilkan modal daftar transaksi kasir yang menggunakan voucher tersebut: Nomor Invoice, Tanggal Transaksi, Nama Kasir, Nama Pelanggan, Nama Outlet, Nilai Potongan Diskon, Total Nilai Transaksi, dan Status.
4. **Sistem Background Scheduler Tiap 5 Menit (Server-Side Auto Expiry)**:
   - Di `notane-server` (`ws.rs`), ditambahkan loop scheduler `start_voucher_expiry_scheduler` yang berjalan setiap 300 detik (5 menit).
   - Otomatis mengeksekusi SQL:
     ```sql
     UPDATE vouchers
     SET status = 'EXPIRED'
     WHERE status = 'ACTIVE' AND valid_until < CURRENT_TIMESTAMP
     ```
   - Mengirim notifikasi WebSocket `VOUCHERS_EXPIRED` jika terdapat voucher yang kedaluwarsa.

---

## Rincian Implementasi

### 1. Skema Database & Migrasi RBAC
- File: `notane-server/src-tauri/migrations/045_voucher_quota_and_claims.sql` dan `notane-client/src-tauri/migrations/045_voucher_quota_and_claims.sql`.
- Menambahkan kolom `quota INT NOT NULL DEFAULT 0` dan `used_count INT NOT NULL DEFAULT 0` pada tabel `vouchers`.
- Menambahkan indeks performa `idx_vouchers_status_valid_until` pada `(status, valid_until)`.
- Menambahkan hak akses `vouchers:claims` ke tabel `permissions` dan menetapkannya secara otomatis ke role `super_admin`, `store_manager`, dan `supervisor`.

### 2. Backend Rust Engine (`notane-server/src-tauri`)
- **[master.rs](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/notane-server/src-tauri/src/api/master.rs)**:
  - `list_vouchers_handler`: Mengembalikan `quota` dan `used_count` pada respons JSON.
  - `create_voucher_handler` & `update_voucher_handler`: Menerima dan menyimpan field `quota` (default `0`), validasi input sanitasi.
  - `validate_voucher_handler`: Mengecek apakah `quota > 0 && used_count >= quota`. Jika habis, mengembalikan error `400 Bad Request` dengan pesan "Kuota voucher promo ini sudah habis terpakai".
  - `get_voucher_claims_handler`: Endpoint `GET /vouchers/{id}/claims` dengan guard `vouchers:claims` atau `vouchers:read`. Mengambil riwayat transaksi dari tabel `sales` yang berelasi dengan voucher tersebut.
- **[pos_service.rs](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/notane-server/src-tauri/src/services/pos_service.rs)**:
  - Dalam transaksi DB penjualan `create_sale`, mengecek kuota voucher dari database.
  - Saat transaksi berhasil dibuat, memperbarui counter voucher:
    ```sql
    UPDATE vouchers 
    SET used_count = used_count + 1,
        status = CASE WHEN quota > 0 AND (used_count + 1) >= quota THEN 'USED' ELSE status END
    WHERE id = $1
    ```
- **[ws.rs](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/notane-server/src-tauri/src/api/ws.rs)**:
  - Background tokio task `start_voucher_expiry_scheduler` yang berjalan setiap 5 menit (interval 300 detik), memeriksa `valid_until < CURRENT_TIMESTAMP` dan mengupdate status menjadi `EXPIRED`.

### 3. Frontend Client POS & Store (`notane-client`)
- **[cartStore.ts](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/notane-client/src/stores/cartStore.ts)**:
  - Validasi kuota saat `applyVoucher(code)`: Jika `v.quota > 0 && (v.used_count || 0) >= v.quota`, tampilkan dialog peringatan bahwa kuota voucher promo sudah habis.
- **[VouchersPage.vue](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/notane-client/src/pages/VouchersPage.vue)**:
  - Kolom **Kuota & Terpakai** di tabel voucher: Menampilkan rasio `used_count / quota`, badge `Habis` jika kuota tercapai, `Sisa N`, atau `Tanpa Batas`.
  - Input **Kuota Pemakaian** di form Tambah/Edit Voucher.
  - Tombol aksi **Riwayat Klaim** (`History` icon) di kolom aksi.
  - Modal **Riwayat Klaim Transaksi Voucher**: Menampilkan riwayat invoice, tanggal, kasir, pelanggan, outlet, potongan, total belanja, dan status transaksi.

---

## Hasil Pengujian & Verifikasi
1. `npm run build` di `notane-client`: **Berhasil (0 error)**.
2. `cargo check` di `notane-server/src-tauri`: **Berhasil (0 error)**.
3. `cargo check` di `notane-client/src-tauri`: **Berhasil (0 error)**.
4. `npm run build` di `notane-server`: **Berhasil (0 error)**.
