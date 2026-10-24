# 🚀 PANDUAN DEPLOY SERVER NOTANE RETAIL DI VPS (MULTI-BRAND / MULTI-TENANT)

Dokumen ini menjelaskan cara menjalankan **Notane Retail Server** di server VPS menggunakan **Docker Compose**, sehingga **1 VPS dapat melayani banyak brand toko sekaligus** dengan port yang berbeda dan database yang terisolasi.

---

## 🏗️ 1. Arsitektur Multi-Brand di 1 VPS

Setiap brand toko diisolasi menggunakan **Docker Namespace Project** (`-p notane-<slug>`), sehingga:
- Setiap brand memiliki **Port Web & POS API** sendiri (misal `18080`, `18090`, `18100`, dst.).
- Setiap brand memiliki **Database PostgreSQL** dan volume data terpisah.
- Setiap brand memiliki **Kode Pairing Kasir** dan **JWT Secret** sendiri.
- Admin brand dapat membuka Web Host Manager langsung di browser melalui `http://IP_VPS:PORT`.
- Kasir di toko offline dapat menghubungkan aplikasi `notane-client` ke `http://IP_VPS:PORT` menggunakan kode pairing masing-masing.

```
                                 [ 1 UNIT VPS ]
                                        |
     +----------------------------------+----------------------------------+
     |                                                                     |
[ BRAND 1: Kopi Senja ]                                            [ BRAND 2: Butik Cantika ]
├── Port Web/API: 18080                                            ├── Port Web/API: 18090
├── Port Kontrol: 18081                                            ├── Port Kontrol: 18091
├── Database: notane_kopisenja (Internal: 5432)                   ├── Database: notane_cantika (Internal: 5432)
├── Pairing: KOPI-8899                                             ├── Pairing: BUTIK-2244
└── Data: volume_kopisenja                                         └── Data: volume_cantika
```

> 💡 **Efisiensi Resource RAM**: Karena backend Rust dikompilasi secara murni (Headless / Zero GUI Overhead), setiap instance server hanya mengonsumsi **~20-30 MB RAM**! Satu VPS dengan RAM 2 GB mampu menjalankan **10–15 brand toko** secara stabil.

---

## 🛠️ 2. Persiapan Awal di VPS

1. **Pastikan Docker & Docker Compose sudah terpasang:**
   ```bash
   docker --version
   docker compose version
   ```

2. **Clone repositori ke VPS:**
   ```bash
   git clone git@github.com:labdacaraka/notane-retail-pos.git /opt/notane
   cd /opt/notane
   ```

3. **Build Image Docker Server Pertama Kali:**
   ```bash
   docker compose build
   ```
   *(Image `notane-retail-server:latest` akan dibangun satu kali dan dapat dipakai bersama oleh semua brand)*.

---

## ⚡ 3. Cara Praktis: Menggunakan Brand Manager CLI (Linux & Windows)

Tersedia skrip otomasi untuk kedua sistem operasi VPS:
- **Linux (Ubuntu/Debian)**: [`scripts/brand-manager.sh`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/scripts/brand-manager.sh)
- **Windows VPS (PowerShell)**: [`scripts/brand-manager.ps1`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/scripts/brand-manager.ps1)
- **Atau via NPM (Universal Cross-Platform)**: `npm run brand <command>`

### A. Membuat Brand Baru
**Di Linux (Ubuntu):**
```bash
# Format: ./scripts/brand-manager.sh create <slug> "<nama_brand>" <port_server> [port_db]
./scripts/brand-manager.sh create kopisenja "Kopi Senja" 18080 15432
./scripts/brand-manager.sh create butikcantika "Butik Cantika" 18090 15433
```

**Di Windows Server (PowerShell):**
```powershell
# Format: .\scripts\brand-manager.ps1 create <slug> "<nama_brand>" <port_server> [port_db]
.\scripts\brand-manager.ps1 create kopisenja "Kopi Senja" 18080 15432
.\scripts\brand-manager.ps1 create butikcantika "Butik Cantika" 18090 15433
```

**Universal (NPM Runner):**
```bash
npm run brand create kopisenja "Kopi Senja" 18080 15432
```
*Skrip otomatis menghasilkan file konfigurasi di `brands/<slug>.env` lengkap dengan kode pairing acak dan password database yang aman.*

### B. Menyalakan Server Brand
- **Linux**: `./scripts/brand-manager.sh start kopisenja`
- **Windows**: `.\scripts\brand-manager.ps1 start kopisenja`
- **Universal**: `npm run brand start kopisenja`

### C. Melihat Daftar Brand & Status
- **Linux**: `./scripts/brand-manager.sh list`
- **Windows**: `.\scripts\brand-manager.ps1 list`
- **Universal**: `npm run brand list`

### D. Menghentikan atau Me-restart Brand
- **Hentikan**: `npm run brand stop kopisenja` *(atau `./scripts/brand-manager.sh stop kopisenja` / `.\scripts\brand-manager.ps1 stop kopisenja`)*
- **Restart**: `npm run brand restart kopisenja`
- **Lihat Logs**: `npm run brand logs kopisenja`

---

## 📝 4. Cara Manual Menggunakan Docker Compose Standar

Jika Anda lebih menyukai perintah Docker Compose standar tanpa script:

1. Buat file `brands/kopi-senja.env` dari template [`.env.brand.example`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/.env.brand.example):
   ```env
   COMPOSE_PROJECT_NAME=notane-kopisenja
   BRAND_NAME=Kopi Senja
   SERVER_PORT=18080
   CONTROL_PORT=18081
   POSTGRES_PORT=15432
   POSTGRES_DB=notane_kopisenja
   POSTGRES_USER=kopi_admin
   POSTGRES_PASSWORD=password_aman_kopi
   PAIRING_CODE=KOPI-8899
   JWT_SECRET=rahasia_jwt_kopi_2026
   ```

2. Jalankan container:
   ```bash
   docker compose --env-file brands/kopi-senja.env -p notane-kopisenja up -d
   ```

3. Matikan container:
   ```bash
   docker compose --env-file brands/kopi-senja.env -p notane-kopisenja down
   ```

---

## 💻 5. Menghubungkan Aplikasi Kasir Client (`notane-client`) ke VPS

Pada mesin kasir cabang/toko (Windows / macOS / Tablet):
1. Buka aplikasi kasir **Notane Retail POS**.
2. Pada layar pemilihan server / koneksi:
   - **Host Server IP / Domain**: Masukkan IP Publik VPS Anda (misal `103.187.99.10` atau `kopi.domainanda.com`).
   - **Port**: Masukkan port brand tersebut (misal `18080`).
   - **Kode Pairing**: Masukkan kode pairing brand toko (misal `KOPI-8899`).
3. Tekan **"Hubungkan"**.
4. Kasir terhubung langsung secara realtime via REST & WebSocket ke database brand di VPS!

---

## 🌐 6. Akses Web Manager di Browser

Setiap brand memiliki Host Manager berbasis web yang dapat dibuka langsung di peramban:
- Buka: `http://IP_VPS:PORT_BRAND` (Contoh: `http://103.187.99.10:18080/`)
- Web Host Manager akan menampilkan:
  - Monitoring kasir yang sedang aktif terhubung.
  - Pengaturan kode pairing.
  - Status koneksi database.
  - Tombol start/stop/restart engine server.
