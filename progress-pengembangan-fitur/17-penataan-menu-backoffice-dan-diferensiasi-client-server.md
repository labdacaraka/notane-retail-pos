# Progress 17: Penataan Menu Back-Office & Diferensiasi Tampilan Client vs Server

## 1. Latar Belakang & Masalah
Sebelumnya, navigasi Command Ribbon di Back-Office mencampurkan menu pengelolaan bisnis ritel dengan pengaturan teknis `Server Host & Device` dalam satu deretan datar tanpa pengelompokan. Hal ini membingungkan bagi pengguna karena:
1. Meskipun aplikasi Notane Retail POS dikembangkan dalam satu basis kode (single codebase repository) untuk Tauri dan Web, komputer di toko ritel memiliki dua peran yang berbeda:
   - **Komputer Server Host (Master Node)**: Menjalankan database PostgreSQL dan engine backend Axum di port 8080, mengontrol siklus hidup daemon server, serta memantau perangkat kasir lain.
   - **Komputer Client Kasir (Station Node)**: Bertindak sebagai terminal kasir / tablet yang terhubung ke Server Host melalui jaringan LAN lokal, dan tidak menjalankan daemon server lokal.
2. Pada komputer Client Kasir, munculnya menu "Server Host & Device" dengan tombol Mulai/Hentikan Server dan acak Pairing Code sangat tidak tepat, karena Client tidak bertindak sebagai server.

---

## 2. Solusi & Implementasi Teknis

### A. Konsep Node Role (Peran Perangkat) pada Store Pinia
- **File**: `src/stores/networkStore.ts`
- **State Baru**:
  - `nodeRole: 'SERVER' | 'CLIENT'` (persisten di `localStorage.getItem('notane_node_role') || 'SERVER'`).
- **Getters**:
  - `isServerNode`: Mengecek apakah perangkat ini bertindak sebagai Server Host Utama.
  - `isClientNode`: Mengecek apakah perangkat ini bertindak sebagai Terminal Client Kasir.
- **Actions**:
  - `setNodeRole(role)`: Menyimpan preferensi peran perangkat dan otomatis menyesuaikan target host jika kembali ke Server mode (`127.0.0.1:8080`).

### B. Penataan Ulang Bilah Navigasi Ribbon (`AdminLayout.vue`)
Menu bilah atas (Ribbon) ditata ulang secara rapi ke dalam tiga kelompok fungsional dengan garis pemisah (divider) yang elegan:
1. **Grup 1: Operasional & Master Data Bisnis**:
   - `Dashboard Hub` (`/admin`)
   - `Master Produk` (`/admin/products`)
   - `Pelanggan & Member` (`/admin/customers`)
   - `Voucher & Promo` (`/admin/vouchers`)
   - `Stok & Opname` (`/admin/inventory`)
   - `Jurnal & Laba Rugi` (`/admin/accounting`)
2. **Grup 2: Desain & Konfigurasi Toko**:
   - `Desain Struk` (`/admin/receipt-settings`)
   - `Parameter Retail` (`/admin/parameters`)
   - `Jejak Audit` (`/admin/audit-logs`)
3. **Grup 3: Jaringan & Infrastruktur (Diferensiasi Client vs Server)**:
   - **Jika Perangkat Bertindak Sebagai SERVER**:
     - Menampilkan menu **Kendali Server Host** (`/admin/server-control`, ikon server exchange, badge ungu `SERVER`).
     - Mengarahkan ke pusat kendali daemon server port 8080, monitor kasir aktif, dan kode pairing.
   - **Jika Perangkat Bertindak Sebagai CLIENT**:
     - Menampilkan menu **Koneksi Server LAN** (`/admin/client-connection`, ikon signal square, badge biru `CLIENT`).
     - Mengarahkan ke formulir penyambungan terminal kasir ke IP Server Host LAN.

### C. Indikator Peran & Modal Pengubah Peran (Node Mode Switcher)
- Di bagian kiri atas header bar, ditambahkan tombol indikator peran interaktif:
  - Mode Server: `[ 🖥️ MODE: SERVER HOST (MASTER) ▾ ]` (warna ungu).
  - Mode Client: `[ 💻 MODE: CLIENT KASIR (STATION) ▾ ]` (warna biru).
- Tombol ini membuka dialog modal **"Pilih Peran Perangkat Ini (Node Mode)"** yang menjelaskan perbedaan kedua peran dan memungkinkan pengguna beralih dengan satu klik.

### D. Halaman Baru: `ClientConnectionPage.vue`
- **Route**: `/admin/client-connection`
- Khusus bagi komputer yang bertindak sebagai Client Kasir untuk mengelola koneksi ke Server Host:
  - Kartu Status Real-Time: Status sambungan (`ONLINE`/`OFFLINE`), latensi ping (ms), dan alamat Server Host.
  - Formulir Sambungan: Input IP Server Host, Port, ID Terminal (`KASIR-01`, dll.), Nama Perangkat, dan Kode Pencocokan (Pairing Code).
  - Tombol Aksi: *Uji Sambungan (Ping Server)*, *Sambungkan ke Server*, dan *Putuskan Sambungan*.
  - Sakelar Sambung Otomatis (Auto-Reconnect).

### E. Integrasi Halaman Server Host (`ServerControlPage.vue`)
- Dibungkus dengan `AdminLayout` sehingga tetap menampilkan pita menu navigasi desktop yang konsisten.
- Dilengkapi badge penanda khusus: `Mode Server Host (Master)`.

### F. Pembaruan Launcher Hub di `AdminDashboard.vue`
- Kartu launcher pada kategori *Operasional & Pergudangan* beradaptasi secara otomatis:
  - Jika Server: Menampilkan tile **Kendali Server Host** (`Mode Server Host`).
  - Jika Client: Menampilkan tile **Koneksi Server LAN** (`Mode Client Station`).

---

## 3. Hasil Pengujian & Verifikasi
1. **Pengujian Kompilasi**:
   - `npm run build`: Kompilasi TypeScript dan bundel Vite selesai dengan status 0 error.
2. **Pengujian Otomatis Subagent Browser**:
   - **Mode Server Host**:
     - Header menampilkan badge ungu `MODE: SERVER HOST (MASTER)` dan status `Host :8080 (Aktif)`.
     - Ribbon menampilkan grup terstruktur dengan `Kendali Server Host [SERVER]`.
     - Screenshot tersimpan: `server_mode_dashboard_1789878974966.png`.
   - **Beralih ke Mode Client Kasir**:
     - Mengklik tombol switcher mode dan memilih `Mode Client Kasir (Station Node)`.
     - UI seketika bertransformasi: badge menjadi biru `MODE: CLIENT KASIR (STATION)`.
     - Ribbon dan Dashboard Hub secara dinamis mengganti kendali server menjadi `Koneksi Server LAN [CLIENT]`.
     - Screenshot tersimpan: `client_mode_dashboard_1789879011142.png`.
   - **Pengujian Sambungan Client ke Host**:
     - Membuka halaman `/admin/client-connection`.
     - Mengklik *Sambungkan ke Server* -> status berubah menjadi `TERHUBUNG (ONLINE)` dengan latensi `0 ms`.
     - Tombol status di bilah atas otomatis berubah hijau `Ke Host (0ms)`.
     - Screenshot tersimpan: `client_connection_page_1789879094594.png`.
   - **Beralih Kembali ke Server Host**:
     - Mengklik switcher mode dan memilih `Mode Server Host (Master Node)`.
     - Dialihkan kembali ke `/admin/server-control`.
     - Terminal `KASIR-01` langsung terdeteksi dan tercatat secara real-time pada tabel *Monitor Perangkat & Terminal Kasir Aktif*.
     - Screenshot tersimpan: `server_mode_control_1789879140690.png`.
