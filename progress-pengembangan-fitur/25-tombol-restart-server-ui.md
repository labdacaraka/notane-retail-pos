# 25 - Tombol Restart Server di UI & Auto Migration Reload

**Tanggal**: 2026-09-25  
**Status**: ✅ SELESAI  

---

## Latar Belakang & Masalah
Sebelumnya, ketika ada migrasi skema database baru atau pembaruan konfigurasi, pengguna harus mematikan backend secara manual lewat terminal (`pkill` dan menjalankan ulang binary Rust). Belum ada tombol ataupun API untuk melakukan restart layanan POS langsung dari antarmuka Web/UI.

---

## Solusi & Arsitektur Implementasi

### 1. Backend Rust Axum (Dual-Port Engine)
- **File**: [`src-tauri/src/api/system.rs`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src-tauri/src/api/system.rs)
- Menambahkan route baru:
  `POST /api/v1/system/server/restart` (tersedia di Port Kontrol `8081` & Port POS `8080`).
- **Alur Kerja `restart_server`**:
  1. Menghentikan listener socket POS lama pada port `8080` via `abort_server()`.
  2. Mengosongkan daftar terminal tersambung (`state.devices.write().await.clear()`) dan menyiarkan sinyal `SERVER_RESTARTED`.
  3. Menjalankan migrasi database terintegrasi otomatis (`run_embedded_migrations(&state.pool)`).
  4. Membangkitkan kembali layanan Axum POS di port `8080` via `spawn_pos_server()`.
  5. Port Kontrol `8081` tetap aktif tanpa terputus, sehingga status response HTTP berhasil dikembalikan ke frontend tanpa timeout.

### 2. State Management Pinia
- **File**: [`src/stores/networkStore.ts`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src/stores/networkStore.ts)
- Menambahkan method `restartHostServer(): Promise<boolean>` yang memanggil endpoint `/server/restart` pada Control Channel (8081) dan otomatis menyegarkan status host serta daftar perangkat terhubung.

### 3. UI/UX Pengendali Server
- **File**: [`src/pages/ServerControlPage.vue`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src/pages/ServerControlPage.vue)
  - Ditambahkan tombol **"Muat Ulang (Restart)"** warna amber di panel aksi server (antara tombol *Mulai* dan *Hentikan*).
  - Dilengkapi dialog konfirmasi aman in-app (`ConfirmModal`) untuk mencegah klik tidak sengaja saat kasir aktif.
- **File**: [`src/pages/Login.vue`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src/pages/Login.vue)
  - Ditambahkan tombol **"Restart"** dengan animasi putar (*spin*) saat proses berlangsung pada kartu Server Host di halaman login (mode server).

---

## Pengujian & Verifikasi
- Endpoint `POST http://localhost:8081/api/v1/system/server/restart` diuji:
  - Berhasil mengembalikan `{"is_running":true,"message":"Host Server berhasil dimuat ulang (Restarted)","status":"success"}`.
  - Log server membuktikan seluruh 11 migrasi dieksekusi ulang secara otomatis dan server POS 8080 kembali `Listening`.
- `npm run build` dan `cargo build`: 100% lolos tanpa error.
