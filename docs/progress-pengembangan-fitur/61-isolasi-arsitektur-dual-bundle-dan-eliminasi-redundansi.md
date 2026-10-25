# 61. Isolasi Arsitektur Dual Bundle & Eliminasi Redundansi Kode

## Tanggal Pelaksanaan
12 Oktober 2026

## Latar Belakang & Masalah
Dalam proses pemisahan repositori Notane Retail menjadi dua aplikasi independen (*Notane Server* dan *Notane Client*), sempat tersisa artefak dan modul duplikasi yang melanggar prinsip pemisahan peran:
1. **Redundansi Backend di Notane Client:**
   - Di dalam `notane-client/src-tauri`, masih terdapat folder `migrations/` dan modul `api/`, `db/`, `models/`, `services/`, serta `utils/`.
   - Hal ini tidak tepat karena `notane-client` bertindak murni sebagai konsumen API HTTP/WebSocket dari server, bukan penyedia database atau runtime Axum server.
2. **Redundansi Frontend di Notane Server:**
   - Di dalam folder `notane-server/src/pages`, sempat tersalin halaman `InventoryPage.vue`, `PurchasesPage.vue`, dan `UsersPage.vue`.
   - Padahal komputer server bertindak murni sebagai host manager dan monitor server dengan satu-satunya antarmuka `ServerHostApp.vue`.

---

## Solusi & Tindakan yang Dilakukan

### 1. Eliminasi Total Modul Server di `notane-client/src-tauri`
- **Menghapus Folder Migrasi & Server Rust:**
  - `notane-client/src-tauri/migrations/` -> **Dihapus permanen**
  - `notane-client/src-tauri/src/api/` -> **Dihapus permanen**
  - `notane-client/src-tauri/src/db/` -> **Dihapus permanen**
  - `notane-client/src-tauri/src/models/` -> **Dihapus permanen**
  - `notane-client/src-tauri/src/services/` -> **Dihapus permanen**
  - `notane-client/src-tauri/src/utils/` -> **Dihapus permanen**
- **Restrukturisasi [`lib.rs`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/notane-client/src-tauri/src/lib.rs):**
  - Menghapus deklarasi `pub mod api;`, `db`, `models`, `services`, `utils`.
  - Hanya menyisakan `pub mod tauri_cmds;` untuk IPC hardware lokal kasir (printer ESC/POS & TSPL label, cash drawer, dan layar customer display).
- **Pemangkasan Dependensi [`Cargo.toml`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/notane-client/src-tauri/Cargo.toml):**
  - Menghapus dependensi berat server (`sqlx`, `postgresql_embedded`, `axum`, `tower`, `argon2`, `jsonwebtoken`, `rust_decimal`, dsb.).
  - Menyisakan hanya dependensi minimal Tauri desktop: `tauri`, `tauri-plugin-opener`, `serde`, `serde_json`, dan `uuid`.

### 2. Eliminasi Halaman Non-Host di `notane-server/src/pages`
- Menghapus seluruh file halaman yang tersalin secara redundant:
  - `notane-server/src/pages/InventoryPage.vue` -> **Dihapus**
  - `notane-server/src/pages/PurchasesPage.vue` -> **Dihapus**
  - `notane-server/src/pages/UsersPage.vue` -> **Dihapus**
- Satu-satunya halaman yang ada di `notane-server/src/pages/` adalah **[`ServerHostApp.vue`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/notane-server/src/pages/ServerHostApp.vue)**.

### 3. Penetapan Aturan Batas Mutlak (AI Agent Rules)
Dibuat berkas aturan tetap di [`.agents/rules/dual-bundle-boundary-rules.md`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/.agents/rules/dual-bundle-boundary-rules.md):
- **Dilarang keras menyalin file antar-proyek:**
  - Migrasi/API/Database/Services -> **Hanya di `notane-server`**.
  - Halaman UI / Komponen / Form POS & Backoffice -> **Hanya di `notane-client`**.
  - Server Pages -> **Hanya `ServerHostApp.vue`**.

---

## Hasil Verifikasi
- **`notane-client/src-tauri` (`cargo check`):** Selesai dalam hitungan detik dengan **0 error & 0 warning**.
- **`notane-client` (`npm run build`):** Selesai dengan status **0 error**.
- **`notane-server/src-tauri` (`cargo check`):** Selesai dengan status **0 error**.
- **`notane-server` (`npm run build`):** Selesai dengan status **0 error**.
