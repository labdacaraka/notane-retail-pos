# Progress 77: Initial Setup Wizard Saat Instalasi Notane Server

## Ringkasan Fitur
Membuat fitur **Initial Setup Wizard (Panduan Konfigurasi Awal Toko)** yang muncul secara otomatis ketika `notane-server` pertama kali diinstal atau dijalankan oleh pengguna, serta dapat dibuka kembali sewaktu-waktu melalui tombol **Setup Awal Toko** di Header maupun menu Branding Toko pada antarmuka Server Host.

Wizard ini dirancang khusus untuk mempermudah pemilik bisnis / teknisi retail mengkonfigurasi entitas toko dari nol secara terstruktur dalam 5 langkah intuitif, termasuk opsi penting pembersihan data demo (dummy products) atau menyimpannya untuk keperluan pengujian.

---

## 5 Tahapan Wizard Setup

### 1. Branding & Identitas Toko
- **Nama Toko** (Wajib, minimal 3 karakter): Nama resmi ritel/usaha.
- **Slogan / Tagline**: Pesan atau sub-judul yang tercetak di struk belanja.
- **Logo Toko**: Drag-and-drop & uploader logo dengan pratinjau langsung. Terhubung ke endpoint upload server `/api/v1/system/branding/upload-logo`.

### 2. Cabang Utama & Kontak
- **Nama Cabang Utama** (Wajib): Menamai outlet utama/pusat (misal: "Cabang Pusat", "Flagship Store").
- **Alamat Toko & Cabang**: Alamat fisik toko yang dicetak pada struk belanja dan kop laporan keuangan.
- **Nomor Telepon / WhatsApp**: Nomor kontak layanan pelanggan kasir dan struk.

### 3. Opsi Database & Data Demo (Dummy Products)
Memberikan pilihan interaktif yang sangat jelas dengan kartu visual:
- **Pilihan A: Database Bersih (Siap Jualan - Rekomendasi untuk Toko Baru)**
  - Menghapus otomatis produk contoh dummy (Indomie Goreng, Ultra Milk, Apel Fuji), mutasi stok awal demo, kartu stok, dan jurnal pembukuan terkait demo.
  - Memastikan toko siap diisi produk dan stok riil tanpa data sampah.
  - Aman: Jika sistem mendeteksi sudah ada transaksi penjualan pada produk demo tersebut di toko yang sudah berjalan, penghapusan dicegah secara atomik.
- **Pilihan B: Muat Data Demo / Dummy Produk (Untuk Belajar / Eksplorasi)**
  - Mempertahankan produk contoh, barcode contoh, dan saldo stok dummy untuk pelatihan staf kasir dan simulasi penjualan.

### 4. Keamanan & Jaringan Terminal (LAN)
- **Nama Lengkap Administrator**: Nama penanggung jawab superadmin toko.
- **Password Baru Akun Admin**: Penggantian password akun `admin` superadmin dengan enkripsi Argon2id.
- **PIN Otorisasi Supervisor**: PIN angka 6-digit untuk otorisasi diskon, void, dan hold di kasir POS.
- **Kode Pairing Client LAN**: Kode sandi berpasangan terminal POS kasir ke Server Host dengan tombol generator **Acak Kode**.
- **Aturan Pembulatan Kasir**: Konfigurasi pembulatan uang tunai (Ke atas Rp 100, Ke bawah Rp 100, Matematika Rp 100, atau Nonaktif).

### 5. Ringkasan & Konfirmasi Setup
- Pratinjau menyeluruh seluruh pengaturan sebelum dieksekusi secara permanen.
- Tombol **Terapkan & Luncurkan Notane Server** dengan animasi spinner loading dan feedback konfirmasi sukses.

---

## Detail Perubahan Teknis

### 1. Database & Migrasi
- File: [`notane-server/src-tauri/migrations/056_initial_setup_wizard_settings.sql`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/notane-server/src-tauri/migrations/056_initial_setup_wizard_settings.sql)
  - Menambahkan kolom `is_initial_setup_completed` (`BOOLEAN DEFAULT FALSE`) dan `initial_setup_completed_at` (`TIMESTAMPTZ`) pada tabel `receipt_settings`.
  - Otomatis menandai toko yang sudah memiliki riwayat penjualan riil (`sales`) sebagai `is_initial_setup_completed = true` agar instalasi lama tidak terganggu.
  - Terdaftar di [`notane-server/src-tauri/src/db/mod.rs`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/notane-server/src-tauri/src/db/mod.rs).

### 2. Backend API Handlers (Rust / Axum)
- File: [`notane-server/src-tauri/src/api/system.rs`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/notane-server/src-tauri/src/api/system.rs)
  - `GET /api/v1/system/setup-status`: Mengecek apakah wizard sudah pernah diselesaikan, mendeteksi jumlah dummy produk, mendeteksi riwayat transaksi, dan mengembalikan konfigurasi identitas saat ini.
  - `POST /api/v1/system/initial-setup`: Menjalankan transaksi atomik:
    - Validasi fail-closed (panjang nama toko, outlet, password, pairing code).
    - Update `receipt_settings` (nama toko, alamat, telepon, tagline, logo, mode pembulatan, pairing code).
    - Update data outlet utama pada tabel `outlets`.
    - Pembersihan dummy products secara bersih (`stock_mutations`, `stock_batches`, `journal_entries`, `products`) jika `include_dummy_products == false`.
    - Update kredensial admin dan supervisor PIN menggunakan hash Argon2id yang aman.
    - Sinkronisasi memory state pairing code pada running server.
    - Pencatatan log audit sistem (`system_audit_logs`).

### 3. Frontend Store & Komponen UI (Vue 3 + TypeScript + Tailwind)
- File: [`notane-server/src/stores/initialSetupStore.ts`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/notane-server/src/stores/initialSetupStore.ts)
  - Pinia store untuk reaktivitas status wizard, pengecekan `fetchSetupStatus()`, dan pemanggilan `executeInitialSetup()`.
- File: [`notane-server/src/components/common/InitialSetupWizardModal.vue`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/notane-server/src/components/common/InitialSetupWizardModal.vue)
  - Modal dialog interaktif 5 langkah dengan stepper visual, validasi real-time, uploader logo toko, kartu pemilih mode database, dan dialog konfirmasi sukses.
- File: [`notane-server/src/pages/ServerHostApp.vue`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/notane-server/src/pages/ServerHostApp.vue)
  - Integrasi deteksi otomatis saat mount: jika `is_setup_completed == false`, wizard langsung terbuka di tengah layar.
  - Menambahkan tombol "Setup Awal Toko" pada header navigasi.
  - Menambahkan tautan "Buka Wizard Setup Lengkap" pada panel Branding Toko.

---

## Perbaikan: Eliminasi Penolakan Pairing Code pada Host Server
- **Akar Masalah:**
  - Sebelumnya antarmuka `notane-server` (Host Manager) menggunakan store jaringan yang sama dengan client dan mengidentifikasi dirinya sebagai `CASHIER` dengan kode pairing lama dari cache `localStorage`. Ketika pengguna mengganti kode pairing via Wizard (misal menjadi `NOTANEKAYA`), backend menolak koneksi WebSocket local host dengan alert `Penolakan Koneksi Host: Kode Pencocokan (Pairing Code) salah...`.
- **Solusi yang Diterapkan:**
  1. **Backend Rust (`ws.rs`):**
     - Membuka parameter `is_host: Option<bool>`.
     - Bypass validasi pairing code untuk koneksi internal dari Host Manager (`is_host == true`, `device_type == "HOST"`, atau `device_id == "HOST-SERVER"`).
     - Menjaga koneksi Host Manager tetap utuh saat siaran `FORCE_DISCONNECT` dikirimkan ke terminal client kasir luar.
     - Menyaring perangkat HOST agar tidak muncul di daftar perangkat kasir terhubung (`list_devices`) dan tidak menghitung HOST ke dalam total client.
  2. **Frontend Host UI (`networkStore.ts` & `InitialSetupWizardModal.vue`):**
     - Menetapkan peran `nodeRole: 'SERVER'`, `deviceType: 'HOST'`, `deviceId: 'HOST-SERVER'`.
     - Mengirim parameter `is_host: 'true'` saat membuka WebSocket.
     - Otomatis menyinkronkan `pairing_code` terbaru dari backend ke local state dan `localStorage` saat fetch status maupun setelah wizard selesai disimpan.
     - Mengabaikan sinyal alert penolakan host (`CONNECTION_REJECTED` & `FORCE_DISCONNECT`) pada UI Host Server.

---

## Verifikasi & Pengujian
1. **Pemeriksaan Kompilasi & Tipe:**
   - Server Rust `cargo check` & `cargo build` berhasil (0 error).
   - Frontend Vue `vue-tsc && vite build` berhasil (0 error).
2. **Pengujian Browser Subagent (End-to-End):**
   - Mengakses UI Server Host di `http://localhost:5176`.
   - Menguji alur dari Langkah 1 hingga Langkah 5.
   - Menguji penggantian kode pairing menjadi `NOTANEKAYA` via Wizard dan reload browser.
   - Terverifikasi 100% bebas dari pesan penolakan koneksi host, server berstatus **SERVER AKTIF**, dan menampilkan kode pairing `NOTANEKAYA`.
