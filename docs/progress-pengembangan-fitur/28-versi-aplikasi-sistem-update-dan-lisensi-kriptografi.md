# 28. Versi Aplikasi, Sistem Update (OTA One-Click), dan Lisensi Kriptografi Anti-Cracking

## Latar Belakang & Kebutuhan
Untuk melindungi aplikasi Notane Retail POS Enterprise dari pembajakan software, keygen tiruan, patching biner, serta reverse engineering, sistem membutuhkan arsitektur lisensi dan pembaruan terpadu yang memenuhi kriteria:
1. **Keamanan Kriptografi Asimetris**: Mustahil dibuatkan keygen atau sertifikat palsu meskipun kode klien didekompilasi.
2. **Node-Locking Hardware ID (HWID)**: Lisensi terikat secara fisik ke spesifikasi mesin klien (*motherboard, CPU, MAC, UUID sistem*).
3. **Anti-Clock-Rollback**: Mencegah kecurangan pemunduran jam sistem operasi komputer klien demi memperpanjang masa aktif.
4. **Sistem Pembaruan Sekali Klik (OTA One-Click Update)**:
   - Terintegrasi langsung dengan status lisensi aktif.
   - Memperbarui seluruh kebutuhan aplikasi: eksekusi otomatis migrasi database baru (*Laravel-style*), penggantian bundle front-end, dan pembaruan binary backend.
5. **Panduan & Standar Server Provider**: Dokumentasi teknis lengkap beserta kode server backend penyedia (*provider*) untuk penerbitan lisensi dan hosting bundle rilis.

---

## Solusi & Arsitektur yang Diterapkan

### 1. Kriptografi Asimetris Ed25519 & Kunci Master
Sistem mengadopsi standar enkripsi modern asimetris Ed25519:
- **Client Binary (Rust)**: **Hanya menyimpan Master Public Key** (`779b7d3fb80a25e9503bf7b85ffbd226ed9a4b8ff9de844872dd3a19357bb213`). Binary klien tidak memiliki kemampuan menerbitkan kunci baru, hanya memvalidasi.
- **Provider Server**: Menyimpan **Master Private Key** (`40c97328401a31ed63b6ecd07bf1ea35a731e775cab79c730582b49069b0a5b9`) di environment variabel cloud yang terisolasi.
- **Node-Locking**: Algoritma hashing SHA-256 sidik jari hardware menghasilkan ID unik perangkat (misal: `NOTANE-HWID-224B-3497-2F1C-F899`). Token lisensi dari komputer A ditolak jika disalin ke komputer B (`HARDWARE_MISMATCH`).
- **Anti-Clock Tampering**: `app_licenses.last_monotonic_time` mencatat waktu sistem berjalan. Jika terdeteksi waktu mundur drastis, status beralih ke `CLOCK_TAMPERED`.

### 2. Skema Database & Migrasi Terintegrasi
Dibuat file migrasi baru: [`src-tauri/migrations/012_license_system.sql`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src-tauri/migrations/012_license_system.sql):
- **Tabel `app_licenses`**: Menyimpan token sertifikat mentah, tanda tangan digital, hardware ID, metadata pemilik, tanggal kedaluwarsa, batas terminal, dan timestamp waktu monotonik.
- **Tabel `app_update_history`**: Audit trail riwayat instalasi pembaruan aplikasi beserta nomor versi dan status migrasi.
- **Auto-Migration Runner**: Ditambahkan ke daftar registrasi di [`src-tauri/src/db/mod.rs`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src-tauri/src/db/mod.rs). Migrasi dieksekusi secara otomatis saat backend dijalankan.

### 3. Modul Backend Rust (`license_service` & `update_service`)
- [`src-tauri/src/services/license_service.rs`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src-tauri/src/services/license_service.rs):
  - Ekstraksi HWID mesin.
  - Verifikasi tanda tangan digital Ed25519 (`verify_strict`).
  - Evaluasi status lisensi: `Active`, `GracePeriod`, `Expired`, `HardwareMismatch`, `ClockTampered`, `Unlicensed`.
- [`src-tauri/src/services/update_service.rs`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src-tauri/src/services/update_service.rs):
  - Manajemen versi aplikasi (`v1.1.0 Enterprise`).
  - Verifikasi hak pembaruan berdasarkan lisensi aktif (*license gating*).
  - Pengecekan versi dan unduhan paket OTA.
- **REST Endpoints di Axum** ([`src-tauri/src/api/system.rs`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src-tauri/src/api/system.rs)):
  - `GET /api/v1/system/version`
  - `GET /api/v1/system/license`
  - `POST /api/v1/system/license/activate`
  - `POST /api/v1/system/updates/check`
  - `POST /api/v1/system/updates/apply`

### 4. Antarmuka UI & State Management (Vue 3 / TypeScript)
- **Pinia Store**: [`src/stores/licenseStore.ts`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src/stores/licenseStore.ts) mengelola status lisensi, masa aktif, HWID, dan pengecekan pembaruan.
- **Komponen Modal Terpadu**: [`src/components/common/LicenseUpdateModal.vue`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src/components/common/LicenseUpdateModal.vue):
  - Tab **Status Lisensi**: Copy HWID 1-klik, rincian paket, dan form aktivasi token lisensi resmi.
  - Tab **Pembaruan Sistem (OTA)**: Cek pembaruan ke server provider, changelog rilis, dan tombol "Unduh & Terapkan Pembaruan".
- **Integrasi Navigasi & Header**:
  - [`AdminLayout.vue`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src/layouts/AdminLayout.vue): Lencana status lisensi & versi di header atas, serta tombol navigasi pita (*Ribbon*) "Lisensi & Update".
  - [`ServerControlPage.vue`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src/pages/ServerControlPage.vue): Tombol shortcut "Lisensi & Update" di samping panel kontrol server host.

### 5. Panduan Lengkap Pengembangan Server Provider
Disusun dokumen arsitektur dan kode siap pakai di root proyek:
[`PANDUAN_SERVER_PROVIDER_LISENSI_DAN_UPDATE.md`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/PANDUAN_SERVER_PROVIDER_LISENSI_DAN_UPDATE.md)
- Spesifikasi database provider (PostgreSQL / SQLite).
- Kontrak REST API (`/license/activate`, `/license/verify`, `/updates/check`, `/updates/download`).
- Implementasi penuh server backend Provider menggunakan **Node.js, Express, dan TypeScript** dengan kriptografi `@noble/ed25519`.
- Struktur paket bundle rilis (`update-bundle.zip`) berisi frontend dist, migration SQL, dan backend binary.
- Strategi hardening biner Rust (`opt-level=3`, `lto="fat"`, `strip=true`, `panic="abort"`) dan UPX packer.

---

## Hasil Pengujian & Verifikasi

1. **Auto-Migration Database Saat Startup**:
   ```log
   INFO notane_retail_pos_lib::db: Menjalankan migrasi baru: 012_license_system (Batch 2)...
   INFO notane_retail_pos_lib::db: Migrasi 012_license_system berhasil dieksekusi dan dicatat (Batch 2).
   ```
2. **Verifikasi REST API Endpoint**:
   - `GET /api/v1/system/version` merespons:
     `{"name":"Notane Retail POS Enterprise","version":"1.1.0","build_date":"2026-09-26","target_os":"macos","target_arch":"x86_64","is_production":false}`
   - `GET /api/v1/system/license` merespons HWID unik mesin dan status awal `UNLICENSED`.
3. **Pengujian Kriptografi Ed25519 & Anti-Tampering**:
   - **Aktivasi Valid**: Menggunakan token yang ditandatangani oleh Master Private Key berhasil mengaktifkan lisensi:
     `{"status":"ACTIVE","tier":"ENTERPRISE","client_name":"Toko Sinar Rezeki","days_remaining":366,"is_valid":true}`
   - **Manipulasi/Tampering**: Mengubah isi payload (misal mengubah masa aktif menjadi tahun 2099 atau mengubah nama klien) langsung ditolak oleh backend:
     `"VERIFIKASI GAGAL: Tanda tangan kriptografi lisensi tidak valid atau telah dimodifikasi (Anti-Cracking triggered)!"`
4. **Verifikasi Antarmuka Browser**:
   - Tombol lencana di header menampilkan versi `v1.1.0` dan paket `ENTERPRISE`.
   - Tombol Ribbon "Lisensi & Update" membuka modal `LicenseUpdateModal.vue` dengan tampilan status lisensi aktif, sisa 366 hari, dan panel OTA updater.
