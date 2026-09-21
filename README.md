# Notane Retail POS Enterprise 🛍️🚀

**Notane Retail POS Enterprise** adalah aplikasi kasir & ERP ritel hybrid modern berperforma tinggi yang dibangun dengan arsitektur **Hybrid Desktop & Distributed Server**. Aplikasi ini menggabungkan kecepatan native desktop menggunakan **Tauri v2 + Rust** dan antarmuka interaktif yang elegan menggunakan **Vue 3 + Tailwind CSS**.

Sistem dirancang untuk mendukung operasional ritel tunggal maupun multi-kasir:
- **Server Host (PC Utama / Kasir 1)**: Berfungsi sebagai server backend utama (REST API, WebSocket, PostgreSQL) sekaligus dapat menjalankan antarmuka POS lokal atau berjalan sebagai background tray daemon.
- **Client (PC Kasir Tambahan / Thin Client)**: Mesin kasir cabang/counter lain terhubung ke Server Host secara realtime melalui jaringan LAN/Wi-Fi menggunakan mDNS Discovery/Server Selector, dilengkapi Offline Storage.
- **Server-Only Mode (Daemon / Headless)**: Dapat dideploy sebagai backend service murni di server Linux/Docker/Cloud tanpa GUI.

```
                        +-----------------------------+
                        |       Jaringan Toko         |
                        |   (Router / Switch LAN)     |
                        +-----------------------------+
                                       |
             +-------------------------+-------------------------+
             |                                                   |
             v                                                   v
  +-----------------------+                           +-----------------------+
  |      MODE SERVER      |                           |      MODE CLIENT      |
  |  (PC Utama / Kasir 1) |                           |  (PC Kasir Tambahan)  |
  +-----------------------+                           +-----------------------+
  | * System Tray Only    |                           | * UI Kasir (Vue 3)    |
  | * Axum Web Server     | <=== REST & WebSocket === | * Server Selector     |
  | * PostgreSQL          |      (Port: 8080 LAN)     | * Offline Storage     |
  | * mDNS Broadcaster    |                           | * mDNS Discovery      |
  +-----------------------+                           +-----------------------+
```

---

## 🌟 Fitur Utama

1. **POS Front-Office Kasir**
   - Transaksi kasir cepat dengan barcode scanner & searchable product selector.
   - Fitur Hold / Recall transaksi belanja pelanggan.
   - Manajemen Shift Kasir (Buka Shift, Rekonsiliasi Kas, Tutup Shift).
   - Otorisasi Supervisor PIN untuk tindakan sensitif (void, diskon manual khusus).
   - Integrasi cetak struk thermal via ESC/POS (Bluetooth / Serial / Network).
   - Customer Display Mirroring (menampilkan keranjang ke layar kedua via WebSocket realtime).

2. **Back-Office & ERP Ritel**
   - **Master Data**: Produk multi-satuan, barcode, kategori, rak gudang, pelanggan, pemasok.
   - **Manajemen Inventaris**: Penyesuaian stok (*Stock Adjustment*), kartu stok (*Stock Movement Audit*).
   - **Pengadaan & Pembelian (Purchase Orders)**: Pencatatan faktur barang masuk dan histori vendor.
   - **Akuntansi & Keuangan**: Buku besar, jurnal umum otomatis, dan laporan arus kas/laba-rugi.
   - **Pengaturan Struk & Profil Toko**: Kustomisasi header/footer struk, QRIS, logo, dan profil toko tersimpan di database.
   - **Manajemen Pengguna & RBAC**: Pengaturan Hak Akses (Administrator, Kasir, Supervisor, Inventory Manager, Akuntan).

3. **Infrastruktur Jaringan & Kontrol Server**
   - **Dual Port Engine**: Port utama (8080) untuk REST/WebSocket POS dan Port Kontrol (8081) untuk remote restart/stop service.
   - Fitur Pairing Token & Persistent Reconnection untuk kasir client jika jaringan sempat terputus.

---

## 🛠️ Tech Stack & Arsitektur

| Komponen | Teknologi |
| :--- | :--- |
| **Desktop Core / Native** | [Tauri v2](https://tauri.app/) (Rust) |
| **Backend Engine & API** | [Rust](https://www.rust-lang.org/) + [Axum](https://github.com/tokio-rs/axum) (HTTP/WebSocket Server) |
| **Database ORM & Migrasi** | [PostgreSQL](https://www.postgresql.org/) + [SQLx](https://github.com/launchbadge/sqlx) (Auto-embedded Migrations) |
| **Frontend Framework** | [Vue 3](https://vuejs.org/) (Composition API, `<script setup>`) + [TypeScript](https://www.typescriptlang.org/) |
| **State Management** | [Pinia](https://pinia.vuejs.org/) |
| **Styling & UI Design** | [Tailwind CSS](https://tailwindcss.com/) + Fluent Color Icons / Lucide Icons |
| **Build & Tooling** | [Vite](https://vitejs.dev/) |

---

## 📋 Prasyarat Sistem (Prerequisites)

Sebelum menginstal dan menjalankan proyek di PC atau server baru, pastikan perangkat telah memiliki:

1. **Node.js**: Versi `>= 18.x` (disarankan Node.js 20 LTS) & `npm`
2. **Rust & Cargo**: Versi stable terbaru (install via `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
3. **PostgreSQL Database**: Versi `>= 14.x`
4. **Dependensi Sistem Native Tauri (sesuai OS)**:
   - **macOS**: Xcode Command Line Tools (`xcode-select --install`)
   - **Linux (Ubuntu/Debian)**:
     ```bash
     sudo apt update
     sudo apt install -y libwebkit2gtk-4.1-dev build-essential curl wget file \
       libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
     ```
   - **Windows**: Microsoft C++ Build Tools & WebView2 Runtime.

---

## 🚀 Panduan Instalasi Cepat (One-Click Developer Setup)

Bagi developer baru, Anda **tidak perlu menginstal dan mengonfigurasi PostgreSQL secara manual**. Proyek ini telah dilengkapi dengan skrip otomasi dan Docker Compose.

### 1. Clone Repository & Jalankan Otomasi Setup
```bash
git clone git@github.com:labdacaraka/notane-retail-pos.git
cd notane-retail-pos

# Jalankan skrip setup otomatis (membuat .env, menginstal npm, dan menyiapkan database)
npm run setup
```

### 2. Mengaktifkan Database (Zero-Config)
Jika Anda menggunakan Docker, Anda cukup menjalankan:
```bash
npm run db:up
```
> Database PostgreSQL 16 akan otomatis berjalan di port `5432` dengan konfigurasi siap pakai (`postgres:postgres@127.0.0.1:5432/notane_pos`).
> Untuk mematikan database, jalankan `npm run db:down`.

*(Opsional: Jika PC Anda sudah memiliki instalasi PostgreSQL lokal, skrip `npm run setup` akan otomatis mendeteksinya).*

### 3. Menjalankan Aplikasi

Terdapat dua cara untuk menjalankan aplikasi selama tahap pengembangan:

#### Opsi A: Aplikasi Desktop Lengkap (Direkomendasikan - Satu Perintah)
```bash
npm run tauri dev
```
> Tauri akan otomatis mengompilasi backend Rust (Axum API di port 8080) sekaligus meluncurkan frontend Vite dalam jendela desktop native.

#### Opsi B: Mode Web Browser (`http://localhost:5173`)
Frontend memerlukan backend API yang aktif di port `8080` untuk proses autentikasi dan database. Jalankan pada **2 terminal terpisah**:

1. **Terminal 1 (Jalankan Backend Server):**
   ```bash
   npm run server
   ```
   *(Menjalankan backend Rust Axum di port 8080 & port kontrol 8081 tanpa membuka jendela desktop)*.

2. **Terminal 2 (Jalankan Frontend):**
   ```bash
   npm run dev
   ```
   Buka browser di `http://localhost:5173`.

> ⚠️ **Catatan Penting (Network Error saat Login)**: Jika Anda hanya menjalankan `npm run dev` tanpa menjalankan backend server (`npm run server` atau `npm run tauri dev`), proses login akan menampilkan pesan *Network Error* karena browser tidak dapat terhubung ke port 8080.

---

### 🔑 Akun & Kredensial Default (Seed Data)

Setelah database dan backend berjalan, migrasi awal akan otomatis menyiapkan akun demo siap pakai:

| Peran (Role) | Username | Password | PIN Otorisasi | Hak Akses Utama |
| :--- | :--- | :--- | :--- | :--- |
| **Administrator** | `admin` | `admin123` | `123456` | Akses penuh sistem, master data, user & RBAC, laporan |
| **Supervisor** | `spv` | `spv123` | `123456` | Operasional kasir, otorisasi void/diskon, audit |
| **Kasir** | `kasir1` | `kasir123` | `111111` | Buka/tutup shift, transaksi POS, hold/recall |

*(Pada halaman login, Anda juga dapat mengklik tombol shortcut **"Admin"**, **"Supervisor"**, atau **"Kasir"** untuk auto-fill username & password).*

---

## 🖥️ Panduan Deploy di Server / Headless Server (Backend Mode)

Jika Anda ingin menempatkan **Notane Retail POS** di server pusat (Dedicated Server/VPS/PC Server Kantor) tanpa memerlukan GUI/tampilan desktop:

### 1. Build Backend Binary (Server Only)
Masuk ke direktori backend `src-tauri` dan lakukan kompilasi:
```bash
cd src-tauri
cargo build --release
```
Binary server akan terbuat di `src-tauri/target/release/notane-retail-pos`.

### 2. Jalankan Mode `--server-only`
```bash
./target/release/notane-retail-pos --server-only
```
Pada mode ini, Rust Axum akan otomatis:
- Menghubungkan & menjalankan migrasi PostgreSQL.
- Menjalankan **API POS & WebSocket** di port `8080` (0.0.0.0:8080).
- Menjalankan **Host Control Channel** di port `8081` (0.0.0.0:8081).
- Berjalan sebagai background daemon tanpa memanggil WebView/GUI.

### 3. Menjadikan Systemd Service (Linux Server)
Buat file service di `/etc/systemd/system/notane-pos.service`:
```ini
[Unit]
Description=Notane Retail POS Server Daemon
After=network.target postgresql.service

[Service]
Type=simple
User=posuser
WorkingDirectory=/opt/notane-retail-pos/src-tauri
Environment="DATABASE_URL=postgres://posuser:password@127.0.0.1:5432/notane_pos"
Environment="PORT=8080"
Environment="RUST_LOG=info"
ExecStart=/opt/notane-retail-pos/src-tauri/target/release/notane-retail-pos --server-only
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```
Aktifkan dan jalankan service:
```bash
sudo systemctl daemon-reload
sudo systemctl enable notane-pos
sudo systemctl start notane-pos
```

---

## 📦 Build Installer Desktop (Distribution)
 
 ### 1. Installer Standar Tauri
 Untuk mendistribusikan installer aplikasi ke komputer kasir (format `.msi`/`.exe` di Windows, `.dmg` di macOS, `.deb`/`.AppImage` di Linux):
 
 ```bash
 npm run tauri build
 ```
 Hasil installer akan tersedia di folder:
 `src-tauri/target/release/bundle/`
 
 ### 2. Installer Satu-Klik untuk Pengguna Awam (Opsi A: Bundled Silent PostgreSQL)
 Untuk pengguna toko/kasir awam yang menginstal aplikasi di komputer baru tanpa perlu tahu cara menginstal PostgreSQL secara terpisah:
 Gunakan skrip Inno Setup yang tersedia di [installer/windows/](installer/windows/README.md).
 Installer ini akan otomatis memasang aplikasi sekaligus mengonfigurasi engine database PostgreSQL secara *silent* di balik layar. Pengguna akhir cukup klik *"Next > Install > Finish"*.

---

## 🔐 Akun Bawaan (Default Seed Credentials)

Saat database pertama kali diinisialisasi otomatis oleh migrasi, akun awal yang tersedia:

| Role | Username / ID | Password Default | Akses |
| :--- | :--- | :--- | :--- |
| **Administrator** | `admin` | `admin123` | Akses penuh seluruh modul ERP & POS |
| **Kasir** | `kasir` | `kasir123` | Akses operasional Register POS & Shift |

*Harap segera mengganti kata sandi default setelah instalasi di lingkungan produksi melalui menu Manajemen Pengguna.*

---

## 📁 Struktur Direktori Proyek

```text
notane-retail-pos/
├── perencanaan/                 # Dokumen proses bisnis & requirement arsitektur
├── progress-pengembangan-fitur/ # Dokumentasi tahapan implementasi fitur lengkap
├── public/                      # Asset statis, logo, favicon
├── src/                         # Frontend Vue 3 (SPA)
│   ├── assets/                  # CSS, font, logo cutouts
│   ├── components/              # Komponen modular (POS, Select2, modals, cart)
│   ├── layouts/                 # Layout (PosLayout & AdminLayout)
│   ├── pages/                   # Halaman (Register POS, Users, Products, Accounting, dll)
│   ├── router/                  # Definisi Vue Router
│   ├── services/                # Axios API client & WebSocket sync service
│   ├── stores/                  # Pinia State Management (Cart, Auth, Shift, Network)
│   └── types/                   # TypeScript interface & types
├── src-tauri/                   # Backend Rust & Tauri Configuration
│   ├── migrations/              # Skema database SQLx PostgreSQL
│   ├── src/
│   │   ├── api/                 # Endpoint REST Axum & WebSocket handler
│   │   ├── db/                  # SQLx Database pool & migration runner
│   │   ├── models/              # Rust structs model (Sales, Product, User, dll)
│   │   ├── services/            # Business logic (POS, Inventory, Accounting)
│   │   └── utils/               # Hashing Argon2, JWT token, ESC/POS generator
│   ├── Cargo.toml               # Dependensi crate Rust
│   └── tauri.conf.json          # Konfigurasi window & desktop Tauri
├── .env.example                 # Template konfigurasi environment
├── package.json                 # Skrip build & dependensi NPM
└── vite.config.ts               # Konfigurasi Vite bundler
```

---

## 📄 Lisensi
Hak Cipta © 2026 **Labda Caraka / Notane Retail**. Seluruh hak cipta dilindungi undang-undang.
