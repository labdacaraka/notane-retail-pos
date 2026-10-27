# Progress 52: Zero-Database License Provider URL & Anti-Tamper Hardening

## 1. Latar Belakang & Masalah
Sebelumnya, alamat server provider lisensi disimpan di tabel database PostgreSQL `system_provider_configs`. Pendekatan ini memiliki celah keamanan serius (*vulnerability*):
- Pengguna yang memiliki akses database via DB Manager (seperti DBeaver, TablePlus, Navicat, pgAdmin, atau CLI `psql`) dapat memanipulasi kolom `config_value` untuk mengarahkan URL provider lisensi ke server palsu (*rogue provider*) yang mereka kendalikan.
- Hal ini dapat membahayakan validasi lisensi offline/online dan integritas pembaruan sistem.

## 2. Solusi & Keputusan Arsitektur
Diterapkan pendekatan **Zero-Database Exposure & Binary Hardening**:
1. **Eliminasi Tabel Database:**
   - Tabel `system_provider_configs` dihapus total dari database PostgreSQL (`DROP TABLE IF EXISTS system_provider_configs`).
   - File migrasi `040_provider_server_config.sql` diperbarui agar memastikan tabel tersebut tidak dibuat/disimpan di database.
2. **Hardcoded Official URL di Binary Rust:**
   - URL produksi resmi (`https://lisensi.notane.id`) dikunci di level kompilasi binary Rust (`OFFICIAL_PRODUCTION_PROVIDER_URL`).
   - Mode debug menggunakan `http://127.0.0.1:19090` (`DEFAULT_LOCAL_PROVIDER_URL`).
3. **Environment Variable Override untuk Development / Container:**
   - Menggunakan `PROVIDER_SERVER_URL` atau `NOTANE_PROVIDER_SERVER_URL` jika didefinisikan di lingkungan sistem / container Docker/Podman (misal: `http://host.containers.internal:19090`).
   - Nilai ini dibaca secara murni via memori/lingkungan sistem (`std::env::var`), tanpa ada pembacaan atau penyimpanan ke PostgreSQL.
4. **Endpoint API Rust & Axum:**
   - `GET /api/v1/system/provider-config`: Mengembalikan status URL aktif, status override env var, dan mode binary. Tidak ada query database.
   - `POST /api/v1/system/provider-config`: Mengembalikan `400 Bad Request` dengan pesan keamanan bahwa URL tidak dapat diubah via database atau form web UI.
   - `POST /api/v1/system/provider-config/test`: Melakukan pengujian sambungan (*ping latency*) langsung ke URL provider aktif.
5. **UI Web Manager:**
   - Mengubah tab **Server Provider** pada `LicenseUpdateModal.vue` menjadi *read-only display* dengan kartu alamat URL aktif, indikator mode, kartu penjelasan keamanan *Anti-Tampering*, dan tombol **"Uji Koneksi Provider"**.

## 3. Hasil Pengujian
- Database PostgreSQL: Tabel `system_provider_configs` telah dihapus.
- Backend Rust: `cargo check` lulus tanpa error.
- API Endpoint: `curl -s http://127.0.0.1:15438/api/v1/system/provider-config/test` berhasil merespons `200 OK` dengan latency ~6ms.
- Frontend: `vue-tsc && vite build` sukses dan telah dideploy ke container `notane-riang-market-server`.
