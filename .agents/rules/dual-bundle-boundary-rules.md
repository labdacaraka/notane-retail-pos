# ATURAN BATAS ARSITEKTUR DUAL BUNDLE (NOTANE-SERVER VS NOTANE-CLIENT)

Aturan ini mengikat seluruh aksi AI Agent dalam memodifikasi, menambah, atau mengelola kode di repositori ini.

---

## 1. PEMISAHAN PERAN MUTLAK (SINGLE RESPONSIBILITY)

### A. `notane-server` (Host Engine & Server Pusat Data)
- **Fungsi Utama:** Mengelola database PostgreSQL terisolasi, migrasi skema SQL, REST API (Axum), WebSocket broadcasting, otentikasi & otorisasi, bisnis logic services, dan background schedulers.
- **Frontend (`notane-server/src/pages`):**
  - **HANYA BOLEH BERISI `ServerHostApp.vue`** (Antarmuka host monitoring, status port, IP LAN, pairing code, lisensi token).
  - **DILARANG KERAS** menyalin atau menambahkan halaman POS / Back-Office (seperti `InventoryPage.vue`, `PurchasesPage.vue`, `UsersPage.vue`, `VouchersPage.vue`, dsb.) ke dalam `notane-server/src/pages/`.
- **Backend Rust (`notane-server/src-tauri`):**
  - Mengelola `migrations/`, `api/`, `db/`, `models/`, `services/`, dan `utils/`.

### B. `notane-client` (Terminal Kasir & Client POS)
- **Fungsi Utama:** Antarmuka kasir (POS), master data manajemen ritel, pengaturan toko, dan pelaporan yang mengonsumsi API dari `notane-server`.
- **Frontend (`notane-client/src/pages`):**
  - Berisi seluruh halaman aplikasi kasir dan back-office manajemen ritel (`PosRegister.vue`, `ProductsPage.vue`, `VouchersPage.vue`, dsb.).
- **Backend Rust (`notane-client/src-tauri`):**
  - **HANYA BERFUNGSI SEBAGAI IPC PERANGKAT KERAS LOKAL** (printer thermal ESC/POS, silent raw print label TSPL, laci kasir cash drawer, dan customer display window) via `src/tauri_cmds/`.
  - **DILARANG KERAS** membuat atau menyalin:
    - `notane-client/src-tauri/migrations/`
    - `notane-client/src-tauri/src/api/`
    - `notane-client/src-tauri/src/db/`
    - `notane-client/src-tauri/src/models/`
    - `notane-client/src-tauri/src/services/`
    - `notane-client/src-tauri/src/utils/`
  - Dependensi `Cargo.toml` pada client harus tetap minimal (hanya `tauri`, `tauri-plugin-opener`, `serde`, `serde_json`, `uuid`). Dilarang menambahkan dependensi database seperti `sqlx` atau `postgresql_embedded` ke client.

---

## 2. LARANGAN KERAS OPERASI PENYALINAN (NO BLIND COPYING)

1. **JANGAN PERNAH** menyalin file backend/migrasi dari `notane-server` ke `notane-client`.
2. **JANGAN PERNAH** menyalin file halaman admin/master dari `notane-client` ke `notane-server`.
3. Setiap fitur baru harus diletakkan tepat pada tempatnya:
   - Endpoint/Query/Migrasi baru -> **HANYA di `notane-server`**.
   - Halaman UI / Komponen / Form baru -> **HANYA di `notane-client`**.
   - Perubahan interaksi hardware (printer/laci) -> **di `notane-client/src-tauri/src/tauri_cmds/`**.
