# Progress Pengembangan Fitur: Pemisahan Ketat Mode Server Host vs Client Terminal, Penyimpanan Offline & Selektor Server LAN

**Tanggal:** 21 September 2026  
**Status:** Selesai & Terverifikasi  
**Terkait:** Mode Server (Headless Host), Mode Client (POS & Back-Office), LAN Discovery (mDNS), Offline Storage  

---

## 1. Latar Belakang & Masalah
1. **Kegagalan Login Jaringan Awal (Network Error):**
   - Saat menjalankan frontend, pengguna mendapati pesan "Network Error" karena backend server belum dijalankan.
   - Perlu adanya panduan alur, script start server mandiri, serta validasi status koneksi sebelum kasir melakukan login.
2. **Topologi Jaringan Toko yang Ketat (Server Host vs Client):**
   - **Mode Server (PC Utama / Kasir 1):** Dijalankan murni sebagai server engine background/system tray (Axum Web Server, PostgreSQL, mDNS Broadcaster). PC Utama/Host **tidak dapat** digunakan sebagai kasir maupun back-office untuk menjaga integritas database dan performa throughput transaksi.
   - **Mode Client (PC Kasir Tambahan / Terminal):** UI Kasir (Vue 3), Server Selector (mDNS discovery + manual IP), Offline Storage, Login Kasir & Back-Office.
3. **Pembersihan Komponen Mode Switcher:**
   - Masih tersisa tombol *"MODE: CLIENT KASIR (STATION)"* dan *"Mode Client Station"* di bilah navigasi atas serta halaman koneksi client yang membingungkan alur kerja operator. Seluruh switch tersebut harus dihilangkan dari antarmuka terminal/client.

---

## 2. Rincian Pekerjaan & Perubahan Teknis

### A. Backend & Konfigurasi Server Host
- **NPM Script `npm run server`:** Menambahkan perintah `cargo run --manifest-path src-tauri/Cargo.toml -- --server-only` ke `package.json` untuk menjalankan daemon Axum & PostgreSQL secara langsung.
- **Docker Compose:** Penyediaan `docker-compose.yml` untuk stack PostgreSQL dan dependensi lokal.
- **Pembaruan Panduan `README.md`:** Mendokumentasikan topologi toko LAN, panduan startup server daemon, dan diagram alur koneksi client.

### B. Penyimpanan Offline & Sinkronisasi
- **`src/services/offlineStorage.ts`:**
  - Caching katalog produk lokal (`IndexedDB` / `localStorage`).
  - Buffer antrean transaksi offline saat koneksi LAN terputus sementara.
  - Sinkronisasi otomatis saat koneksi ke Server Host tersambung kembali.

### C. Penemuan Server Jaringan (LAN Discovery & Selector)
- **`src/services/discoveryService.ts`:**
  - Layanan pemindaian mDNS & ping probe untuk mendeteksi Server Host di subnet LAN yang sama.
- **`src/components/common/ServerSelector.vue`:**
  - Komponen pemilihan Server Host dengan fitur *Scan Jaringan LAN (mDNS)*, input IP/Port manual, pairing code, dan uji latensi (ping).
  - Integrasi sebelum form login kasir.

### D. Redesain & Pemisahan UI (Login & Layar Kasir)
- **`src/pages/Login.vue`:**
  - Memisahkan tampilan login:
    - **Mode Server:** Menampilkan Panel Kontrol Server murni (status port 8080/8081, PostgreSQL, mDNS broadcast IP, pairing code, tabel connected clients, tombol start/stop/minimize ke tray). Tanpa form login operasional kasir.
    - **Mode Client:** Menampilkan Server Selector, status Offline Storage, dan form autentikasi kasir/back-office.
  - Perbaikan layout scrolling container (`index.html` dan `Login.vue`).

### E. Pembersihan Sisa Mode Switcher
- **`src/layouts/AdminLayout.vue`:**
  - Menghapus tombol ganti mode *"MODE: CLIENT KASIR (STATION) / MODE: SERVER HOST (MASTER)"* di header bilah atas.
  - Menghapus modal pop-up pemilihan peran perangkat (`showRoleModal`).
  - Menyederhanakan indikator kanan atas murni sebagai status koneksi ke host LAN (`Ke Host (..ms)` / `Terputus dari Host`).
  - Menghapus ribbon badge pembeda Server vs Client.
- **`src/pages/ClientConnectionPage.vue`:**
  - Menghapus badge *"Mode Client Station"* di samping header judul.
  - Menghapus kartu panduan dan tombol *"Beralih ke Mode Server Host"*.
- **`src/layouts/PosLayout.vue`:**
  - Mengubah role badge menjadi identitas terminal kasir murni: `💻 Terminal [KASIR-01]`.
- **`src/pages/AdminDashboard.vue`:**
  - Menghapus percabangan server node dan mengubah tile menjadi `Koneksi Server LAN` dengan badge `Koneksi Server`.

---

## 3. Hasil Pengujian & Verifikasi
1. **Kompilasi TypeScript & Bundler:**
   - Dijalankan via `vue-tsc && vite build`: Lolos 100% tanpa error ketik maupun unused variables/imports (`dist/index.html` ter-bundle bersih).
2. **Pengujian Tampilan Browser:**
   - Dilakukan verifikasi langsung di `http://localhost:5175/admin/client-connection`.
   - Header bilah atas, judul halaman, dan footer terbebas dari tulisan *"Mode Client Kasir"* dan opsi switcher.
