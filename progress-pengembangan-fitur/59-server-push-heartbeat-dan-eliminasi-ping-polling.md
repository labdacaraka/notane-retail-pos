# Progress 59: Implementasi Server Push Heartbeat Scheduler & Eliminasi Polling Jaringan

## Ringkasan Perubahan
Memperbaiki arsitektur deteksi status koneksi antara aplikasi Host (`notane-server`) dan Terminal Kasir (`notane-client`) dari sistem **HTTP Polling** yang membebani jaringan menjadi arsitektur **Server-Driven Push & Client Subscription** berbasis WebSocket:
- **Background Heartbeat Scheduler (Rust Tokio Engine):** Mengimplementasikan penjadwalan otomatis di background daemon `notane-server` yang memancarkan sinyal detak (*heartbeat beacon*) setiap 2 detik ke seluruh client yang mensubscribe.
- **Push & Subscribe Event Architecture:** Terminal client cukup membuka koneksi WebSocket sekali dan mendengarkan event `HOST_HEARTBEAT` yang dikirim oleh server host.
- **Eliminasi Total HTTP Polling:** Menghapus interval polling berulang pada `ServerRecoveryBanner.vue`, `networkStore.ts`, dan `Login.vue`.
- **Passive Watchdog Zero-Overhead:** Klien menggunakan pengawas pasif berbasis timer 7 detik yang di-reset setiap kali sinyal detak server diterima, memberikan kepastian status *real-time* tanpa membuat request HTTP berkala.

---

## 1. Latar Belakang Masalah Polling Sebelumnya
Sebelum perbaikan ini:
1. **Beban Request HTTP Berulang (Polling):** Komponen seperti `ServerRecoveryBanner.vue` menjalankan `setInterval` setiap 4 detik untuk melakukan `axios.get('/api/v1/system/health')`, sementara store jaringan dan modul lain melakukan request HTTP ping berkala.
2. **Heartbeat Scheduler Mati Mendadak:** Pada implementasi sebelumnya di `spawn_pos_server`, loop beacon langsung berhenti (*break*) pada iterasi pertama karena memeriksa status `is_running` yang belum bernilai `true` saat fungsi pertama kali dipanggil.
3. **Redundansi Handshake Koneksi:** Terminal kasir melakukan HTTP ping dan validasi kode pairing secara serial melalui HTTP sebelum akhirnya membuka WebSocket.

---

## 2. Implementasi Teknis & Solusi

### A. Sisi Server Host (`notane-server` / Rust Tokio Core)
1. **Background Heartbeat Scheduler (`AppState::start_heartbeat_scheduler`):**
   - Menggunakan `tokio::time::interval` berdurasi 2 detik dengan `MissedTickBehavior::Skip`.
   - Menghindari penghentian (*break*) saat server berhenti sementara; loop tetap siaga di background dan langsung melanjutkan siaran ketika status `is_running` kembali aktif.
   - Menggunakan `AtomicBool` untuk memastikan hanya ada 1 background scheduler yang berjalan per instance server (idempoten & thread-safe).
   - Memancarkan payload lengkap:
     ```json
     {
       "type": "HOST_HEARTBEAT",
       "timestamp": 1728741234567,
       "server_time": "2026-10-12T14:30:00Z",
       "connected_devices": 3,
       "is_running": true,
       "uptime_seconds": 120,
       "pairing_code": "NOTANE-8899"
     }
     ```
2. **Instant Handshake Beacon:**
   - Saat terminal kasir baru terhubung ke WebSocket `/ws/v1/events`, server langsung mengirimkan `HOST_HEARTBEAT` pertama seketika itu juga (latensi < 1ms), sehingga client tidak perlu menunggu 2 detik untuk mengetahui status aktif server.

### B. Sisi Client Terminal (`notane-client` / Vue 3 + Pinia)
1. **Langganan Sinyal (Subscribe Flow) di `networkStore.ts`:**
   - Metode `connect()` langsung menginisiasi koneksi WebSocket `initWebSocket()`.
   - Parameter autentikasi (`device_id`, `device_name`, `device_type`, `pairing_code`) dikirimkan langsung saat handshake WebSocket query params.
   - `checkConnection()` tidak lagi memanggil `axios.get` atau `pingServer()`; jika status store sudah `CONNECTED` via subscription aktif, fungsi langsung mengembalikan `true`.
2. **Passive Watchdog:**
   - Klien hanya mendengarkan sinyal detak broadcast. Jika dalam 7 detik (toleransi 3 siklus detak) tidak ada detak yang masuk, client secara otomatis menandai status `DISCONNECTED` dan memicu penjadwalan auto-reconnect.
3. **Pembersihan Polling pada `ServerRecoveryBanner.vue`:**
   - Menghapus `checkInterval = setInterval(performHealthCheck, 4000)` dan request HTTP `/health`.
   - Menggantinya dengan status reaktif murni `computed(() => netStore.connectionIntent === 'CONNECTED' && netStore.connectionStatus !== 'CONNECTED')`.
   - Banner darurat muncul dan menghilang secara reaktif berdasarkan event socket push tanpa membebani CPU atau jaringan.
4. **Pembaruan Halaman Login (`Login.vue`):**
   - Menghubungkan indikator online/offline langsung ke reaktivitas WebSocket subscription tanpa polling HTTP.

---

## 3. Hasil Pengujian & Verifikasi
- **Cargo Check (`notane-server` & `notane-client`):** 0 error.
- **NPM Build (`notane-client` & `notane-server`):** Selesai sukses dan menghasilkan bundle dist tanpa error kompilasi TypeScript (`vue-tsc`).
- **Efisiensi Jaringan:** Mengeliminasi seluruh traffic HTTP polling berkala (0 request `/health` berulang per menit saat kasir standby).
