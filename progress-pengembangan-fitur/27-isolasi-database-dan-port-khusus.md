# 27. Isolasi Database PostgreSQL & Port Khusus Anti-Konflik

## Latar Belakang Masalah
Sebelumnya, sistem menggunakan port-port umum:
1. **Port 5432**: Port default global PostgreSQL. Jika komputer host toko/kantor sudah menjalankan server PostgreSQL lokal lain, aplikasi akan mengalami crash atau konflik kepemilikan database.
2. **Port 8080**: Port umum web server (Apache Tomcat, Spring Boot, Jenkins, dev server proxy).
3. **Port 8081**: Port umum yang sering digunakan untuk tooling/developer server.

Kondisi tersebut menyebabkan risiko tinggi tabrakan port (*port conflict*) ketika aplikasi dijalankan pada komputer yang sudah memiliki service aktif.

---

## Solusi & Arsitektur yang Diterapkan

### 1. Migrasi ke Port Khusus Terisolasi (Dedicated Non-Standard Ports)
Seluruh port yang digunakan oleh Notane Retail POS telah dipindahkan ke rentang port khusus yang aman dari bentrokan:
- **PostgreSQL Database**: Dari port umum `5432` menjadi **`15432`** (Port Terisolasi). Port `5432` milik komputer host kini **100% bebas** dan tidak disentuh sama sekali oleh Notane Retail POS.
- **Axum POS Server (REST & WebSocket)**: Dari port umum `8080` menjadi **`18080`**.
- **Host Control Channel (Standby Channel)**: Dari port umum `8081` menjadi **`18081`**.

### 2. Modul Embedded PostgreSQL Manager (`EmbeddedPgManager`)
Dibuat modul Rust [`src-tauri/src/db/embedded.rs`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src-tauri/src/db/embedded.rs) yang bertindak sebagai pengelola siklus hidup database terisolasi:
- **Pengecekan Port Otomatis**: Memeriksa ketersediaan koneksi pada port `15432`.
- **Auto-Provisioning**: Jika server PostgreSQL terisolasi belum berjalan, aplikasi secara mandiri menyiapkan instance terisolasi dalam direktori data aplikasi lokal (`dirs::data_local_dir()/notane-retail-pos/embedded_pg/data`), tanpa mengotori path sistem operasi atau registry host.
- **Graceful Shutdown**: Menghentikan proses database secara aman saat aplikasi ditutup.

### 3. Sinkronisasi Seluruh Konfigurasi Sistem
- **Docker / Podman Compose**:
  File [`docker-compose.yml`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/docker-compose.yml) memetakan host port `15432:5432`. Container hanya mengikat port `15432` pada host machine, membebaskan port `5432`.
- **Konfigurasi Lingkungan (`.env` & `.env.example`)**:
  - `DATABASE_URL=postgres://postgres:postgres@127.0.0.1:15432/notane_pos`
  - `PORT=18080`
- **Vite Dev Server Proxy**:
  [`vite.config.ts`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/vite.config.ts) mengarahkan proxy `/api` dan `/ws` ke `http://127.0.0.1:18080`.
- **LAN Discovery & Store**:
  [`discoveryService.ts`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src/services/discoveryService.ts) dan [`networkStore.ts`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src/stores/networkStore.ts) menginisialisasi kandidat dan cache ke port `18080` & `18081`, lengkap dengan auto-migrasi jika pengguna sebelumnya menyimpan nilai port lama di local storage.
- **UI & Panel Kontrol**:
  [`Login.vue`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src/pages/Login.vue), [`ServerSelector.vue`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src/components/common/ServerSelector.vue), [`NetworkConnectionModal.vue`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src/components/common/NetworkConnectionModal.vue), dan [`ClientConnectionPage.vue`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src/pages/ClientConnectionPage.vue) menampilkan dan memvalidasi port baru.
- **Installer Windows & Setup Scripts**:
  [`installer/windows/setup.iss`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/installer/windows/setup.iss), [`scripts/setup.sh`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/scripts/setup.sh), dan [`scripts/setup.ps1`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/scripts/setup.ps1) telah dimutakhirkan ke port `15432`.

---

## Hasil Pengujian
1. Verifikasi `lsof` pada sistem membuktikan:
   - Port `5432` (Host Postgres default): **100% BEBAS (Free)**.
   - Port `8080` (Common web server): **100% BEBAS (Free)**.
   - Port `15432` (Notane Postgres Isolated): **Aktif & Terhubung**.
   - Port `18080` (Axum REST & WebSocket): **Aktif Melayani Kasir**.
   - Port `18081` (Host Control Standby): **Aktif**.
2. Pengujian interaktif antarmuka peramban:
   - Status server berhasil ditransisikan dari `STOPPED` ke `RUNNING`.
   - Data produk, pengguna, dan skema migrasi database terbaca utuh tanpa error.
