# 51 - Aplikasi Management Server Provider Lisensi & Update OTA (Tauri + Rust + Vue 3)

## 📌 Ringkasan Implementasi
Telah selesai dibangun aplikasi **Notane License & Server Management Studio (`notane-manager`)** menggunakan arsitektur **Tauri v2 (Rust Core) + Vue 3 + Tailwind CSS**. 

Aplikasi ini bertindak sebagai **Provider Server / Master Licensing Authority** sesuai spesifikasi pada `PANDUAN_SERVER_PROVIDER_LISENSI_DAN_UPDATE.md`, menangani penerbitan lisensi resmi, penguncian hardware node-locking (HWID), generator tanda tangan kriptografis asimetris Ed25519, serta distribusi paket pembaruan OTA.

---

## 🏗️ Komponen Arsitektur

### 1. Cryptographic Engine (Ed25519)
- **Master Private Key**: `40c97328401a31ed63b6ecd07bf1ea35a731e775cab79c730582b49069b0a5b9`
- **Master Public Key**: `779b7d3fb80a25e9503bf7b85ffbd226ed9a4b8ff9de844872dd3a19357bb213` (tersimpan di klien POS).
- **Asymmetric Signature**: Menandatangani payload lisensi JSON dan checksum bundle update OTA sehingga mustahil dipalsukan/cracked tanpa Private Key.

### 2. Database Engine (Zero-Configuration SQLite / VPS Ready)
- Menggunakan database SQLite lokal (`notane_provider.db`) di direktori aplikasi, sehingga tidak membutuhkan instalasi PostgreSQL/Docker untuk menjalankannya di desktop manager.
- Skema tabel:
  - `roles`, `permissions`, `role_permissions` (RBAC terstruktur).
  - `provider_clients`: Manajemen data klien tenant.
  - `provider_licenses`: Katalog lisensi, kuota terminal, masa aktif, dan status.
  - `provider_device_activations`: Pencatatan HWID node-locking per lisensi.
  - `provider_app_releases`: Repositori versi rilis, link zip, SHA256 checksum, dan Ed25519 signature.
  - `provider_license_audit_logs`: Audit trail aktivitas aktivasi, verifikasi, dan deteksi tampering.
  - `provider_settings`: Konfigurasi port server dan private key.

### 3. Embedded REST API Server (Axum)
- Berjalan secara mandiri pada port **19090** (dapat diaktifkan/dinonaktifkan via tombol UI atau CLI `--headless`).
- Endpoint yang disediakan:
  - `GET /health` : Health check status provider.
  - `GET /api/v1/system/status` : Metrik jumlah lisensi, terminal aktif, dan uptime.
  - `POST /api/v1/license/activate` : Dipanggil oleh client/server untuk mendaftarkan dan memvalidasi HWID, menghasilkan sertifikat ber-signature Ed25519.
  - `POST /api/v1/license/verify` : Pengecekan periodik status lisensi dari terminal kasir.
  - `POST /api/v1/updates/check` : Pengecekan otomatis rilis pembaruan OTA.

### 4. Antarmuka Manajemen (Vue 3 + Tailwind CSS + Lucide)
- **Dashboard**: Live status API provider, KPI metrik, daftar lisensi terbaru, dan ringkasan audit log.
- **Klien Tenant**: Formulir pendaftaran dan pengelolaan perusahaan pemilik outlet.
- **Daftar Lisensi**: Penerbitan lisensi multi-tier (BASIC, PRO, ENTERPRISE), pengaturan kuota terminal, dan aktivasi/suspend status.
- **Offline Signer**: Generator manual untuk kasir tanpa internet. Memasukkan nomor lisensi + HWID, menghasilkan Token JSON + 64-byte Hex Signature siap salin.
- **Hardware Node-Locking**: Monitor HWID mesin kasir yang terikat beserta IP, versi aplikasi, dan tombol *Reset / Unbind Mesin*.
- **Rilis Pembaruan (OTA)**: Publikasi update versi baru, perhitungan checksum SHA-256, dan penandatanganan paket zip.
- **Audit & Keamanan**: Pemantauan log aktivitas aktivasi, verifikasi, dan peringatan tampering.
- **Pengaturan**: Konfigurasi port API provider dan Master Private Key.

---

## 🚀 Perintah Menjalankan Aplikasi

Dari root folder repository:

```bash
# 1. Menjalankan Antarmuka Desktop Tauri GUI
npm run manager:app

# 2. Menjalankan Mode Web Dev di Browser (Port 5177)
npm run manager:dev

# 3. Menjalankan Mode Headless Daemon di VPS / Background (Port 19090)
npm run manager:daemon
```
