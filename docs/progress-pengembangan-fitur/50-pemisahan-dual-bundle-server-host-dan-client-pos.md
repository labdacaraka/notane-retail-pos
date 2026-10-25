# Progress 50: Pemisahan Sistem Menjadi 2 Aplikasi Terpisah (Notane Server Host & Notane POS Client)

## 1. Latar Belakang & Kebutuhan
Sebelumnya, sistem Notane Retail mengemas fungsi Server Host dan fungsi Kasir (Client) dalam satu aplikasi tunggal dengan pergantian mode internal. Untuk memberikan kejelasan operasional di toko ritel, keandalan uptime, serta kemudahan instalasi di berbagai topologi toko, sistem kini dipisahkan secara struktural menjadi **2 aplikasi mandiri**:
1. **Notane Retail Server (Host Engine / Pusat Data)**:
   - Dijalankan di komputer server / komputer utama toko.
   - Mengelola engine database terisolasi PostgreSQL (Port 15432), Server API Axum (Port 18080), Control Channel Port 18081, WebSocket broadcasting, dan fitur Watchdog Self-Healing otomatis.
   - Menyediakan antarmuka dashboard monitoring server, status port, IP host LAN, kode pairing toko, serta daftar terminal kasir yang sedang aktif terhubung.
2. **Notane Retail POS (Client Kasir / Terminal)**:
   - Dijalankan di komputer meja kasir.
   - Antarmuka kasir murni, cepat, dan terfokus pada transaksi jual-beli, scan barcode, printer struk, dan laci uang.
   - Terhubung ke server host via alamat IP lokal (`127.0.0.1` jika berada dalam satu PC yang sama, atau IP LAN seperti `192.168.1.35` untuk PC kasir cabang).

---

## 2. Arsitektur Dual Binary (Tauri + Rust)

Dalam file [`src-tauri/Cargo.toml`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src-tauri/Cargo.toml), didaftarkan dua target executable:
```toml
[[bin]]
name = "notane-retail-pos"
path = "src/main.rs"

[[bin]]
name = "notane-retail-server"
path = "src/bin/server.rs"
```

### Konfigurasi Bundle Tauri:
- **Client Kasir:** Menggunakan [`src-tauri/tauri.conf.json`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src-tauri/tauri.conf.json) dengan window title *"Notane Retail POS Enterprise"* dan default route `/login`.
- **Server Host:** Menggunakan [`src-tauri/tauri.server.conf.json`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src-tauri/tauri.server.conf.json) dengan window title *"Notane Retail Server - Host Manager"* dan default route `/server-host`.

---

## 3. Skenario Operasional Penggunaan

### Skenario A: 1 Komputer Merangkap Server & Kasir (Single PC)
Bila toko hanya memiliki 1 komputer untuk melayani kasir sekaligus menyimpan database:
1. Pengguna membuka aplikasi **Notane Server** dari Desktop. Server host otomatis berjalan (PostgreSQL port 15432 dan API port 18080).
2. Di komputer yang sama, pengguna membuka aplikasi **Notane POS**.
3. Notane POS langsung terhubung secara instan ke server di `http://127.0.0.1:18080`.
4. Pengguna dapat langsung bertransaksi tanpa kendala.

### Skenario B: Server Terpisah + Banyak Komputer Kasir (Multi-Terminal LAN)
Bila toko memiliki komputer backoffice/server dan 2-5 meja kasir di depan:
1. Komputer Server menjalankan aplikasi **Notane Server**. Di layar tampil alamat IP Host LAN (contoh: `http://192.168.1.35:18080`) dan Kode Pairing (contoh: `NOTANE-8899`).
2. Di setiap komputer kasir (Kasir 1, Kasir 2, dst.), cukup buka aplikasi **Notane POS**.
3. Di layar sambungkan Notane POS, masukkan alamat IP server (`http://192.168.1.35:18080`) dan kode pairing.
4. Seluruh kasir terhubung secara real-time ke satu pusat data.

---

## 4. Inno Setup One-Click Installer (Windows)

Pada file installer [`installer/windows/setup.iss`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/installer/windows/setup.iss), installer secara otomatis memaketkan kedua executable dan membuat 2 shortcut di Desktop pengguna:
1. **Notane POS (Kasir)** -> Menjalankan `notane-retail-pos.exe`
2. **Notane Server (Pusat Data)** -> Menjalankan `notane-retail-server.exe`

Database engine PostgreSQL tetap dipasang secara silent tanpa konfigurasi manual pada port 15432 terisolasi.

---

## 5. Perintah CLI / NPM Scripts

| Script NPM | Deskripsi |
| :--- | :--- |
| `npm run dev` | Menjalankan Vite frontend development server (port 5175). |
| `npm run server` | Menjalankan Server Host daemon di background (port 18080 & 18081). |
| `npm run server:app` | Menjalankan GUI Notane Retail Server Host Manager. |
| `npm run client:build` | Membangun installer/binary rilis untuk Notane Retail POS (Client). |
| `npm run server:build` | Membangun installer/binary rilis untuk Notane Retail Server (Host). |
| `npm run build:all` | Membangun kedua installer aplikasi sekaligus. |

---

## 6. Verifikasi & Pengujian
- Kompilasi Rust: `cargo check --manifest-path src-tauri/Cargo.toml --bins` **0 error**.
- Kompilasi TypeScript: `npx vue-tsc --noEmit` **0 error**.
- Endpoint API Host & Control Port:
  - `http://127.0.0.1:18080/api/v1/system/status` -> Aktif (`is_running: true`).
  - `http://127.0.0.1:18081/api/v1/system/status` -> Aktif (`is_running: true`).

---

## 7. Penyempurnaan Antarmuka & Isolasi Lisensi

Berdasarkan tinjauan operasional:
1. **Aplikasi Client Kasir (`Notane POS`):**
   - Menghilangkan switch mode server atau opsi alih mode server di layar Login (`Login.vue`).
   - Menghilangkan menu/tombol status lisensi di header maupun ribbon back-office (`AdminLayout.vue`).
   - Aplikasi Kasir murni berfokus pada terminal kasir, koneksi ke Server Host LAN, dan operasional transaksi toko.
2. **Aplikasi Server Host (`Notane Server`):**
   - Mengubah tampilan antarmuka dari mode gelap (darkmode) ke tema **Light Mode** modern yang bersih dan selaras dengan estetika desktop Notane Retail.
   - Menghapus tautan / tombol "Buka Aplikasi Kasir (POS)" agar komputer server berfokus murni sebagai host manager & pusat data.
   - Mengintegrasikan modul **Lisensi Server & Hardware ID Mesin (Node-Locked Ed25519)** secara terpusat di Host App, lengkap dengan tombol salin HWID, rincian tier/kuota kasir, dan modal aktivasi token provider.

---

## 8. Deteksi Otomatis IP Jaringan LAN & Pemulihan Start Server

1. **Deteksi Otomatis IP Jaringan (Multi-Adapter & Offline-Safe):**
   - Mengintegrasikan crate `local-ip-address` untuk mendeteksi IP riil adapter jaringan lokal (Wi-Fi, Ethernet/LAN) secara offline menggunakan OS network stack langsung tanpa bergantung pada koneksi internet.
   - Server Host secara otomatis menampilkan IP LAN sebenarnya (contoh: `http://192.168.1.35:18080`), bukan lagi `127.0.0.1`.
   - Menambahkan pemilih adapter jaringan otomatis pada kartu info jika komputer memiliki lebih dari 1 adapter aktif (misalnya Wi-Fi dan kabel LAN).
2. **Penyelesaian Error Gagal Start Server:**
   - **Resolusi Konflik Simbol `_EMBED_INFO_PLIST`:** Menghilangkan pemanggilan ganda macro `tauri::generate_context!()` di dalam `lib.rs` yang sebelumnya menyebabkan kegagalan kompilasi binary pada sistem desktop. Parameter context sekarang diteruskan dari masing-masing binary (`main.rs` dan `bin/server.rs`).
   - **Multi-Probing Localhost pada Control Channel:** Memperbarui `networkStore.ts` agar metode kontrol host (`startHostServer`, `stopHostServer`, `restartHostServer`, `fetchHostStatus`) selalu mencoba `127.0.0.1:18081` terlebih dahulu secara andal tanpa terpengaruh oleh variabel localStorage client.
   - **Socket Bind Retry:** Menambahkan toleransi retry 5x pada `spawn_pos_server` di backend Rust untuk menangani status `TIME_WAIT` soket TCP saat start/restart cepat.


