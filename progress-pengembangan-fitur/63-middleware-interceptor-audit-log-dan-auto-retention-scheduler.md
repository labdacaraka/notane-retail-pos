# 63. Middleware Interceptor Audit Log & Scheduler Pembersihan Otomatis

## Latar Belakang & Masalah
Untuk keperluan akuntabilitas, keamanan transaksi, dan audit forensik 360°, seluruh tindakan pengguna ke server API POS perlu dilacak secara komprehensif tanpa harus menuliskan fungsi logging manual di setiap controller atau route. Di samping itu, agar ukuran basis data tidak membengkak tanpa batas seiring waktu, diperlukan konfigurasi retensi hari (misal 30 hari, 60 hari, dsb.) dan background scheduler otomatis yang membersihkan log kadaluwarsa secara berkala.

---

## Solusi & Arsitektur yang Diterapkan

### 1. Intersepsi Middleware Zero-Trust (Axum Web Engine)
File: `notane-server/src-tauri/src/api/audit_middleware.rs`
- Middleware `audit_log_middleware` mengintersepsi setiap request API HTTP yang masuk ke `/api/v1/*`.
- **Informasi yang Ditangkap:**
  - **Siapa (Pelaksana):** Mengurai token Bearer JWT secara kriptografis (`user_id`, `username`, `role_name`, `terminal_id`). Jika anonim atau proses login, mengekstrak username dari payload request serta header `X-Terminal-Id`, `X-Device-Name`, `X-Forwarded-For`, dan `User-Agent`.
  - **Dimana (Rute & Modul):** Metode HTTP (`GET`, `POST`, `PUT`, `DELETE`), endpoint URI lengkap (termasuk query parameters), dan nama modul/entitas (`pos`, `auth`, `products`, `vouchers`, `promotions`, `inventory`, dsb.).
  - **Kapan (Timestamp & Latensi):** Waktu pencatatan UTC presisi dan durasi waktu eksekusi (`execution_time_ms`).
  - **Data yang Dikirim (Payload Body & Parameter):** Membaca byte payload request dengan aman (tanpa merusak downstream body streaming) dan menyimpannya dalam format JSON.
  - **Sanitasi Kredensial:** Seluruh field sensitif (`password`, `pin`, `supervisor_pin`, `token`, `secret`, `pairing_code`) otomatis disensor (`***`).
  - **Hasil & Status:** Menyimpan HTTP status code respon (misal `200`, `201`, `400`, `401`, `500`).
- **Pencatatan Asinkron:** Penyimpanan record dilakukan di background task `tokio::spawn` sehingga tidak menambah latensi pada response pengguna.
- **Pengecualian Rekursif:** Endpoint internal seperti `/auth/audit-logs`, `/ws/*`, upload aset gambar `/uploads/*`, dan `/ping` dikecualikan agar tidak menimbulkan looping log.

### 2. Skema Basis Data & Konfigurasi Retensi
File Migrasi: `notane-server/src-tauri/migrations/046_audit_log_system_and_retention.sql`
- **Tabel `audit_logs`:**
  - Menambahkan kolom `http_method VARCHAR(10)`, `endpoint TEXT`, `status_code INT`, `execution_time_ms INT`, `user_agent TEXT`, `user_name VARCHAR(100)`, dan `role_name VARCHAR(50)`.
  - Menambahkan indeks pada `created_at DESC`, `user_id`, `entity_name`, dan `http_method`.
- **Tabel `receipt_settings`:**
  - Menambahkan kolom `audit_log_retention_days INT NOT NULL DEFAULT 30` (nilai `0` berarti simpan selamanya).
- **Permissions RBAC:**
  - `audit:manage` (Mengatur retensi log audit dan konfigurasi pelacakan).
  - `audit:cleanup` (Membersihkan riwayat log audit sistem secara manual).

### 3. Background Scheduler Pembersihan Otomatis
File: `notane-server/src-tauri/src/api/ws.rs` & `notane-server/src-tauri/src/services/audit_service.rs`
- Pada background scheduler loop (interval 5 menit), ditambahkan pemeriksaan retensi:
  ```sql
  DELETE FROM audit_logs 
  WHERE created_at < CURRENT_TIMESTAMP - ($1 || ' days')::INTERVAL
  ```
- Jika ada log yang dihapus, server memancarkan broadcast WebSocket `AUDIT_LOGS_PURGED` sehingga client langsung memperbarui data tampilan.

### 4. Antarmuka Manajemen & Audit Trail 360° (Client Vue 3)
File: `notane-client/src/pages/AuditLogPage.vue` (`http://localhost:5175/admin/audit-logs`)
- **Pencarian & Multi-Filter:**
  - Pencarian fleksibel berdasarkan nama pengguna, aksi, terminal, atau rute endpoint.
  - Filter metode HTTP (Semua, GET, POST, PUT, DELETE).
  - Filter status respon (Semua, 2xx Sukses, 4xx/5xx Error).
  - Filter modul (POS, Otentikasi, Produk, Stok, Voucher, Promosi, dsb.).
- **Modal Detail Aktivitas:**
  - Menampilkan ringkasan identitas pelaksana, peran, IP, durasi eksekusi, serta penampil JSON payload rapi dengan tombol salin (copy payload).
- **Modal Pengaturan Retensi:**
  - Pilihan cepat masa retensi (7, 14, 30, 60, 90, 180, 365 hari, atau permanen).
  - Statistik jumlah total log dan tanggal log terlama.
  - Opsi pembersihan manual fleksibel:
    1. **Berdasarkan Umur:** Menghapus log yang lebih tua dari hari yang dipilih (misal > 1 hari, > 7 hari, > 30 hari). Jika belum ada data yang melebihi batas hari, sistem memberikan notifikasi informatif yang jelas.
    2. **Kosongkan Seluruh Riwayat:** Tombol khusus untuk menghapus seluruh jejak log seketika (reset 0) dengan konfirmasi keamanan.
