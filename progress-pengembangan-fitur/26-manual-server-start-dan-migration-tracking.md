# 26. Penyesuaian Standby Mode Server Host & Migration Tracking Table

## Ringkasan Penyesuaian

Berdasarkan kebutuhan:
1. **Host Server Standby on Launch (Manual Start)**:
   - Komputer client kasir atau perangkat yang hanya ingin bertindak sebagai terminal tidak perlu otomatis membuka port server host 8080 saat aplikasi dibuka.
   - Host server kini berada dalam mode **SIAGA (STOPPED)** saat aplikasi di-launch. Pengguna dapat menyalakan server dengan menekan tombol **"Nyalakan Server"** di Panel Kontrol Server (`/admin/server-control` maupun di halaman Login Mode Server).
   - Port kontrol ringan (8081) tetap standby melayani status dan perintah start/stop/restart tanpa membebani resource POS.
   - Opsi CLI `--auto-start` tetap disediakan jika ingin dijalankan sebagai dedicated headless server daemon yang langsung aktif tanpa GUI.

2. **Database Migration Tracking Table (`_migrations`)**:
   - Sebelumnya, setiap kali aplikasi di-restart, fungsi migrasi mengeksekusi ulang seluruh file SQL (termasuk `002_seed_data.sql` yang mereset quantity stok demo pada `warehouse_stocks`).
   - Sekarang, sistem migrasi mengadopsi mekanisme tabel tracking mirip **Laravel Migrations** menggunakan tabel `_migrations`:
     ```sql
     CREATE TABLE IF NOT EXISTS _migrations (
         id SERIAL PRIMARY KEY,
         migration VARCHAR(255) NOT NULL UNIQUE,
         batch INT NOT NULL,
         applied_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
     );
     ```
   - Setiap file migrasi yang telah dieksekusi akan dicatat ke dalam `_migrations`.
   - Pada start berikutnya, file yang sudah tercatat diabaikan (`Database skema sudah mutakhir (tidak ada migrasi baru yang perlu dijalankan)`).
   - Database tidak lagi ter-reset dari 0 saat start aplikasi, dan perubahan data/stok pengguna tetap utuh.

## File yang Diubah
- [`src-tauri/src/db/mod.rs`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src-tauri/src/db/mod.rs): Penambahan skema `_migrations` dan pengecekan batch migrasi.
- [`src-tauri/src/main.rs`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src-tauri/src/main.rs): Default port 8080 siaga / stopped saat launch; hanya auto-start bila argumen `--auto-start` diberikan.
- [`src-tauri/src/api/ws.rs`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src-tauri/src/api/ws.rs): Inisialisasi awal `AppState::is_running` menjadi `false`.
- [`src/stores/networkStore.ts`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src/stores/networkStore.ts): Inisialisasi awal `hostStatus.is_running = false`.
