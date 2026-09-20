# Progress 16: Pengaturan Struk & Profil Toko Berbasis Database Back-Office

## 1. Ringkasan Fitur
Menyediakan modul pengelolaan konfigurasi struk belanja dan identitas toko ritel yang tersimpan secara persisten di database PostgreSQL. Modul ini memungkinkan admin back-office untuk mengubah profil toko (nama toko, slogan, alamat, nomor telepon/WhatsApp, website, NPWP/Tax ID), catatan header/footer struk, akun media sosial, serta parameter tata letak kertas cetak thermal (58mm / 80mm) yang langsung berdampak secara real-time pada struk kasir POS.

---

## 2. Arsitektur & Komponen yang Dikembangkan

### A. Database (PostgreSQL Migration)
- **File**: `src-tauri/migrations/003_receipt_settings.sql`
- **Tabel**: `receipt_settings`
  - `id` UUID PRIMARY KEY
  - `outlet_id` UUID (Foreign Key ke `outlets`)
  - `store_name` VARCHAR(150) NOT NULL
  - `store_tagline` VARCHAR(255)
  - `store_address` TEXT NOT NULL
  - `store_phone` VARCHAR(50) NOT NULL
  - `store_website` VARCHAR(100)
  - `tax_id` VARCHAR(50) (NPWP Toko)
  - `header_note` TEXT
  - `footer_note_1` TEXT (Kebijakan retur/garansi)
  - `footer_note_2` TEXT (Layanan pelanggan / CS)
  - `social_media` VARCHAR(100) (IG / TikTok / Sosmed)
  - `paper_width` VARCHAR(10) DEFAULT '58mm'
  - `show_phone` BOOLEAN DEFAULT true
  - `show_address` BOOLEAN DEFAULT true
  - `show_cashier_name` BOOLEAN DEFAULT true
  - `show_barcode` BOOLEAN DEFAULT true
  - `show_tax_summary` BOOLEAN DEFAULT true
  - `created_at` TIMESTAMPTZ, `updated_at` TIMESTAMPTZ
- **Seeding Data Awal**: Disediakan data default toko ritel siap pakai.
- **Embedded Migrations**: Ditambahkan ke `src-tauri/src/db/mod.rs` pada fungsi `run_embedded_migrations()`.

### B. Backend API (Rust Axum)
- **Model Struct**: `src-tauri/src/models/settings.rs`
  - `ReceiptSettings`: Representasi entitas database lengkap dengan serde Serialize/Deserialize dan sqlx FromRow.
  - `UpdateReceiptSettingsRequest`: DTO untuk payload pembaruan dari frontend.
- **Endpoint RESTful**: `src-tauri/src/api/master.rs`
  - `GET /api/v1/settings/receipt`: Mengambil data pengaturan struk toko saat ini.
  - `PUT /api/v1/settings/receipt`: Menyimpan/memperbarui data pengaturan struk ke database dan mencatat riwayat perubahan.

### C. Frontend Back-Office UI (`ReceiptSettingsPage.vue`)
- **Route**: `/admin/receipt-settings`
- **Layout Desktop Ribbon**:
  - Diintegrasikan ke bilah navigasi pita desktop `AdminLayout.vue` dengan ikon warna-warni `streamline-ultimate-color:print-text`.
  - Ditambahkan ke *Operational & Pergudangan Launcher Hub* di `AdminDashboard.vue`.
- **Fitur Halaman**:
  1. **Tab 1 - Profil & Identitas Toko**: Input nama toko, slogan, alamat lengkap, telepon/WA, website, dan nomor NPWP.
  2. **Tab 2 - Pesan Header & Footer**: Input catatan pembuka, ketentuan transaksi baris 1, ucapan & kontak baris 2, dan media sosial.
  3. **Tab 3 - Format Cetak & Kertas**: Pemilihan lebar kertas (58mm vs 80mm) serta sakelar visibilitas (tampilkan alamat, telepon, nama kasir, barcode faktur, rincian pajak & kembalian).
  4. **Pratinjau Real-Time Struk Thermal (Right Column)**: Render kertas struk interaktif dengan font monospace bergaya thermal receipt, garis gerigi potong kertas, format dua kolom yang proporsional, serta pembaruan langsung saat admin mengetik (reactive two-way binding).
  5. **Tombol Uji Cetak Thermal**: Menguji langsung format struk yang sedang diedit ke printer hardware atau simulator tanpa harus berpindah ke layar kasir.
  6. **Tombol Reset Bawaan**: Mengembalikan formulir ke konfigurasi default.

### D. Integrasi POS Cashier Printing (`printerStore.ts` & `escpos.ts`)
- `printerStore.ts`:
  - Menambahkan state `receiptSettings` dan fungsi `fetchReceiptSettings()`.
  - Fungsi `printReceipt()` secara otomatis menggunakan metadata toko, kontak, catatan, dan format kertas dari database `receiptSettings`.
- `escpos.ts`:
  - Memperluas `EscPosOptions` dan generator biner `buildReceiptEscPos()` agar membaca opsi dinamis (`storeTagline`, `taxId`, `storeWebsite`, `headerNote`, `footerNote1`, `footerNote2`, `socialMedia`, `showAddress`, `showPhone`, `showCashierName`, `showBarcode`, dll.).

---

## 3. Hasil Pengujian & Verifikasi
1. **Verifikasi Database**:
   - Skema tabel `receipt_settings` dibuat dan diperiksa pada PostgreSQL container.
   - Operasi `INSERT` dan `UPDATE` berhasil dijalankan melalui Axum endpoint.
2. **Pengujian API**:
   - `GET /api/v1/settings/receipt` menghasilkan HTTP 200 dengan payload JSON lengkap.
   - `PUT /api/v1/settings/receipt` berhasil memperbarui data toko dan mengembalikan status sukses.
3. **Pengujian Browser Subagent**:
   - Mengubah nama toko menjadi `NOTANE FLAGSHIP SUPERSTORE`.
   - Mengubah alamat toko menjadi `Jl. Pajajaran Raya No. 99, Baranangsiang, Kota Bogor`.
   - Mengubah catatan footer struk dan beralih antara lebar kertas 58mm dan 80mm.
   - Memverifikasi pratinjau struk real-time merespons seketika terhadap setiap ketikan karakter.
   - Menekan tombol **Simpan Pengaturan** -> Notifikasi toast hijau sukses muncul.
   - Menekan tombol **Uji Cetak Thermal** -> Job cetak berhasil dikirim ke printer thermal.
   - Merefresh halaman browser -> Data yang baru tersimpan terbukti tetap ada dan termuat dari database PostgreSQL.
4. **Verifikasi Kompilasi Frontend & Backend**:
   - `cargo check`: 0 error, 0 warning.
   - `npm run build`: Kompilasi TypeScript dan Vite bundle berhasil 100%.
