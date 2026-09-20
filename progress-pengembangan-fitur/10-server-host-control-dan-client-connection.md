# Progress Log 10: Implementasi Konsep Host Server Control & Client Connection

**Tanggal**: 20 September 2026  
**Status**: Selesai (Completed & Verified)  
**Fitur**: Server Host Start & Stop, Connected Devices Monitor, Client Connect / Disconnect, LAN Host Configuration & Heartbeat Latency

---

## 1. Latar Belakang & Kebutuhan
Dalam operasional kasir retail multi-perangkat (multi-device terminal), dibutuhkan arsitektur jaringan lokal (LAN) yang fleksibel:
1. **Sisi Host Server (Back-Office / Komputer Server Toko)**:
   - Pengendali penuh untuk memulai (*Start*) dan menghentikan (*Stop*) background daemon server.
   - Kemampuan mendeteksi alamat IP lokal (LAN) secara otomatis (contoh `192.168.1.7:8080`) untuk dibagikan ke terminal-terminal kasir lain.
   - Pemantau perangkat aktif (*Connected Devices Monitor*) secara real-time yang mencatat identitas perangkat, tipe (Kasir POS, Layar Pelanggan, Tablet), alamat IP, kasir yang login, waktu koneksi, serta latensi/heartbeat.
   - Kemampuan memutuskan koneksi perangkat tertentu secara paksa (*Kick Client*).
2. **Sisi Terminal Klien (Client POS Terminal)**:
   - Antarmuka untuk melakukan *Connect* dan *Disconnect* ke Host Server.
   - Konfigurasi IP Host & Port target server LAN secara dinamis tanpa mengubah kode.
   - Fitur uji coba latensi (*Ping Test*) sebelum menyambungkan.
   - Indikator status koneksi interaktif di header kasir (Hijau/Aktif beserta latensi ms, Merah/Terputus, Kuning/Menghubungkan).
   - Pengaturan *Auto-Reconnect* jika koneksi terputus tiba-tiba.

---

## 2. Perubahan & Implementasi Teknis

### Backend (Rust Axum & WebSocket)
1. **`src-tauri/src/api/ws.rs`**:
   - Struktur data `ConnectedDevice` & `ServerStatus`.
   - Thread-safe in-memory `DeviceRegistry` (`Arc<RwLock<HashMap<String, ConnectedDevice>>>`).
   - Deteksi IP LAN otomatis lintas platform menggunakan `UdpSocket`.
   - Penanganan query handshake WebSocket (`device_id`, `device_name`, `device_type`, `user_name`).
   - Heartbeat 5 detik ping/pong serta penanganan pemutusan koneksi otomatis saat socket ditutup.
   - Penanganan pesan `FORCE_DISCONNECT` untuk memutus perangkat yang di-kick oleh host.
2. **`src-tauri/src/api/system.rs`**:
   - `GET /api/v1/system/health`: Cek kesehatan server & latensi.
   - `GET /api/v1/system/status`: Status operasional server, port, uptime, dan daftar IP host.
   - `GET /api/v1/system/devices`: Daftar seluruh perangkat yang sedang terhubung.
   - `POST /api/v1/system/devices/{id}/disconnect`: Memutuskan koneksi perangkat target.
   - `POST /api/v1/system/server/start`: Mengubah status server menjadi aktif/berjalan.
   - `POST /api/v1/system/server/stop`: Menghentikan server dan membroadcast sinyal `SERVER_STOPPED`.
3. **`src-tauri/src/api/mod.rs`**:
   - Mendaftarkan rute `/api/v1/system` pada router Axum.
4. **`src-tauri/src/main.rs`**:
   - Memasukkan parameter port ke pembuatan router.

### Frontend (Vue 3, Pinia, TypeScript, Retro Desktop UI)
1. **`src/stores/networkStore.ts`**:
   - Pinia store pengelola siklus hidup koneksi client & state server host.
   - Menyimpan preferensi target host, port, terminal ID di `localStorage`.
   - Menjalankan background heartbeat & auto-reconnection loop.
   - Menghubungkan broadcast event WebSocket (`DEVICE_CONNECTED`, `DEVICE_DISCONNECTED`, `SERVER_STOPPED`, `SERVER_STARTED`, `FORCE_DISCONNECT`).
2. **`src/services/api.ts`**:
   - Menyesuaikan `baseURL` Axios secara dinamis dari `localStorage` agar seluruh request REST API otomatis diarahkan ke server host yang dipilih klien kasir.
3. **`src/components/common/NetworkConnectionModal.vue`**:
   - Dialog konfigurasi jaringan modal retro desktop light theme.
   - Form isian IP host, port, Terminal ID, nama perangkat, tipe terminal, dan auto-reconnect.
   - Tombol *Uji Latensi (Ping Test)* dengan estimasi waktu respon (ms).
   - Tombol *Sambungkan (Connect)* dan *Putuskan (Disconnect)*.
4. **`src/pages/ServerControlPage.vue`**:
   - Halaman Back-Office `/admin/server-control`.
   - Banner kontrol Start/Stop daemon server.
   - Kartu URL IP LAN Host dengan tombol *Salin URL*.
   - Kartu metrik jumlah terminal aktif dan uptime server.
   - Tabel spreadsheet real-time *Monitor Perangkat & Terminal Kasir Aktif* dengan tombol *Putuskan*.
5. **`src/layouts/PosLayout.vue` & `src/layouts/AdminLayout.vue`**:
   - Badge indikator jaringan interaktif di header (klik untuk membuka modal koneksi).
   - Menambahkan menu *Server Host & Device* pada pita navigasi ribbon admin.
6. **`src/pages/AdminDashboard.vue`**:
   - Menambahkan kartu launcher modul *Server Host & Device*.

---

## 3. Hasil Pengujian & Verifikasi
- **Cargo Check**: Selesai dengan `0 warnings` dan `0 errors`.
- **NPM Build**: Selesai sukses (`dist/assets` terkompilasi penuh).
- **REST Endpoints Testing**:
  - `GET /api/v1/system/health`: Respons `200 OK`.
  - `GET /api/v1/system/status`: Respons sukses menampilkan IP `192.168.1.7` & `127.0.0.1`.
  - `POST /api/v1/system/server/stop` & `start`: Respons instan mengubah status daemon.
- **Browser End-to-End Simulation**:
  - Berhasil membuka halaman Host `/admin/server-control`.
  - Berhasil melakukan *Ping Test* dari kasir (latensi 10 ms).
  - Berhasil menghubungkan `KASIR-01` ke host.
  - Tabel Monitor di host langsung menampilkan status `KASIR-01` sebagai `ONLINE` secara real-time.
  - Berhasil mengeksekusi *kick/putuskan* koneksi perangkat.
