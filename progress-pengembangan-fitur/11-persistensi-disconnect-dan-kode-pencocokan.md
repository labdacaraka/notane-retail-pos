# Progress Log 11: Persistensi Status Disconnect & Sistem Kode Pencocokan (Pairing Code)

**Tanggal**: 20 September 2026  
**Status**: Selesai (Completed & Verified)  
**Fitur**: Persistensi Disconnect saat Refresh, Fleksibilitas URL VPS/Cloud Domain, dan Sistem Kode Pencocokan (Pairing Code) untuk Keamanan Akses Host

---

## 1. Masalah & Solusi yang Diterapkan

### A. Masalah Persistensi Disconnect saat Refresh
- **Permasalahan Sebelumnya**: Ketika kasir menekan tombol *Putuskan (Disconnect)* lalu melakukan refresh browser, aplikasi langsung melakukan auto-connect kembali karena status *intent* pengguna tidak disimpan. Hal ini menyulitkan jika kasir ingin bekerja offline atau sedang berganti server host (misalnya ke server VPS).
- **Solusi**:
  - Diterapkan penyimpanan status *connection intent* (`notane_connection_intent`) di `localStorage` dengan nilai `'CONNECTED'` atau `'DISCONNECTED'`.
  - Jika kasir sengaja menekan tombol *Putuskan (Disconnect)*, intent disetel ke `'DISCONNECTED'`.
  - Pada `onMounted` (`PosLayout.vue`), auto-reconnect **hanya** berjalan jika `connectionIntent === 'CONNECTED'`. Jika `'DISCONNECTED'`, aplikasi tetap dalam kondisi terputus saat di-refresh.

### B. Dukungan Koneksi VPS / Domain Cloud
- Input target server di `NetworkConnectionModal.vue` dan `networkStore.ts` kini mendukung format fleksibel:
  - IP LAN lokal (contoh: `192.168.1.100` atau `127.0.0.1` dengan port `8080`).
  - Domain VPS / Cloud (contoh: `pos.tokoku.com` atau `https://pos.tokoku.com`).
  - Penyesuaian protokol WebSocket otomatis (`ws://` untuk HTTP standar dan `wss://` untuk HTTPS/SSL).
  - Panggilan REST API via Axios otomatis menyertakan header `X-Pairing-Code`.

### C. Sistem Kode Pencocokan (Pairing Code Security)
- **Tujuan**: Mencegah perangkat asing yang tidak dikenal mengakses database atau transaksi kasir, terutama saat server host di-deploy di VPS publik atau jaringan terbuka.
- **Sisi Host (`/admin/server-control`)**:
  - Menampilkan panel khusus **Kode Pencocokan Perangkat (Pairing Code)**.
  - Kode bawaan default (misal: `NOTANE-8899`) atau acak.
  - Tombol **"Salin Kode"** untuk kemudahan operasional.
  - Tombol **"Acak Kode Baru (Regenerate)"** (`POST /api/v1/system/pairing-code/regenerate`) yang menghasilkan kode baru secara instan tanpa memutus perangkat yang sudah terverifikasi sebelumnya.
- **Sisi Client Kasir (`NetworkConnectionModal.vue`)**:
  - Input wajib **Kode Pencocokan (Pairing Code)**.
  - Verifikasi pra-koneksi melalui `POST /api/v1/system/pairing-code/verify`.
  - Handshake WebSocket otomatis mengirimkan parameter `pairing_code`.
  - Jika kode salah: Server menolak upgrade/koneksi dengan pesan: `"Kode pencocokan salah! Perangkat tidak diizinkan terhubung ke server ini."`
  - Jika kode benar: Server mengizinkan koneksi, perangkat ditandai sebagai **`● ONLINE (✓ Terverifikasi)`** pada tabel monitor host.

---

## 2. Pengujian & Verifikasi End-to-End
1. **Verifikasi Persistensi Disconnect**:
   - Status awal kasir: Terhubung (`LAN Host`).
   - Kasir mengklik *Putuskan Koneksi (Disconnect)* -> status berubah menjadi *Terputus dari Host Server* (Merah).
   - Browser di-refresh (`http://localhost:5175/`) -> Terverifikasi status **TETAP TERPUTUS** dan tidak melakukan auto-reconnect.
2. **Verifikasi Penolakan Kode Salah**:
   - Input kode salah: `WRONG-1234` -> Diklik *Sambungkan ke Server*.
   - Terverifikasi koneksi **DITOLAK** dan muncul alert peringatan error yang jelas di modal.
3. **Verifikasi Persetujuan Kode Benar**:
   - Input kode benar: `NOTANE-8899` -> Diklik *Sambungkan ke Server*.
   - Terverifikasi koneksi **BERHASIL**, status menjadi hijau (`LAN Host`), latensi muncul (10 ms).
4. **Verifikasi Panel Host Server Control & Modal Konfirmasi Stabil**:
   - Membuka `/admin/server-control`.
   - Banner *Kode Pencocokan Perangkat* tampil dengan tombol Salin dan Acak Kode Baru.
   - Tabel monitor mencatat `KASIR-01` dengan status **`● ONLINE (✓ Terverifikasi)`**.
   - **Perbaikan Dialog Hentikan Server**: Mengganti `window.confirm()` bawaan browser yang rentan auto-close/tertutup sendiri akibat timer polling latar belakang menjadi komponen in-app **`ConfirmModal.vue`**. Dialog kini tampil stabil di layar, memiliki tema desktop retro yang seragam, dan hanya tertutup ketika pengguna secara sadar mengklik tombol *Batal* atau *Ya, Hentikan Server*.

### D. Arsitektur Hard Socket Shutdown (Connection Refused) & Host Control Channel
- **Permasalahan**: Pengguna menginginkan bahwa saat tombol *Hentikan Server* ditekan, port 8080 **benar-benar mati total** di level socket sistem operasi (`Connection Refused`, tidak ada respons HTTP 503 sama sekali), namun server tetap dapat dinyalakan kembali dari tombol UI (*Start Server*) tanpa harus restart terminal.
- **Tantangan Arsitektur**:
  - Jika port 8080 dimatikan total di level OS (`drop TcpListener`), browser tidak bisa mengirim request HTTP ke port 8080 untuk menyalakannya kembali.
- **Solusi Arsitektur yang Diterapkan**:
  1. **Pemisahan Port Layanan (8080) & Port Kontrol (8081)**:
     - **Service Port 8080 (POS & Data)**: Menangani seluruh transaksi kasir, master produk, inventori, laporan, dan WebSocket terminal.
     - **Control Channel 8081 (Host Management Daemon)**: Menangani status server, start/stop daemon, regenerasi pairing code, dan monitor perangkat.
  2. **Hard Socket Shutdown pada Port 8080**:
     - Saat admin mengklik *Hentikan Server*, Rust menyiarkan `SERVER_STOPPED` ke seluruh klien, mengirim frame penutup, dan memicu `server_handle.abort()`.
     - `TcpListener` pada port 8080 langsung di-*drop* oleh sistem operasi kernel.
     - Setiap request ke port 8080 (misalnya `curl http://127.0.0.1:8080/api/v1/products`) langsung menghasilkan error:
       ```
       curl: (7) Failed to connect to 127.0.0.1 port 8080: Connection refused
       ```
       Port 8080 **mati total tanpa koneksi sama sekali**.
  3. **Re-binding Dinamis saat Nyalakan Server**:
     - Saat admin mengklik *Nyalakan Server*, permintaan diteruskan ke Control Channel (port 8081 atau via Tauri Native IPC).
     - Rust mengeksekusi `tokio::net::TcpListener::bind("0.0.0.0:8080")` baru dan menjalankan `axum::serve` kembali.
     - Port 8080 langsung aktif kembali (`HTTP 200 OK`).
  4. **Dukungan Dual-Mode Frontend (`networkStore.ts`)**:
     - Mendeteksi `controlUrl` (port 8081) secara otomatis untuk manajemen host.
     - Otomatis memicu pemutusan WebSocket dan status DISCONNECTED saat port 8080 mati.
- **Hasil Pengujian End-to-End**:
  - `curl http://127.0.0.1:8080/api/v1/products` saat server dihentikan -> **`curl: (7) Connection refused`** (Mati total).
  - Klik *Mulai Server* di `/admin/server-control` -> Server rebind instan.
  - `curl http://127.0.0.1:8080/api/v1/products` saat server aktif -> **`HTTP/1.1 200 OK`**.


