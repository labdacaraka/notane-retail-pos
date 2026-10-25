# 30 — Solusi Produk Berganti Barcode (Multi-Barcode) & Alur Pembelian (GRN)

**Tanggal**: 2026-09-26  
**Status**: ✅ SELESAI & TERVERIFIKASI PENUH (Full-stack end-to-end)

---

## 1. Latar Belakang & Masalah Bisnis

Di industri ritel, produsen atau supplier kerap mengganti desain kemasan dan kode barcode produk (misal: edisi khusus, perubahan ukuran kemasan, pergantian nomor registrasi distributor).
Di toko, seringkali masih terdapat stok kemasan lama, sementara stok kiriman baru memiliki barcode yang berbeda.

Sebelum fitur ini:
- Toko terpaksa membuat master produk baru (yang memecah histori stok dan laporan penjualan), ATAU
- Mengubah barcode master produk yang menyebabkan kasir tidak bisa men-scan kemasan lama yang masih ada di rak pajang.

---

## 2. Solusi Teknis & Arsitektur

### A. Konsep Multi-Barcode (Barcode Utama + Barcode Alias)
- **Primary Barcode (`products.barcode`)**: Barcode utama yang digunakan sebagai identitas default produk pada label harga dan struk belanja.
- **Additional / Alias Barcodes (`product_barcodes`)**: Tabel relasional yang menyimpan seluruh riwayat barcode lama maupun barcode alternatif (dus/karton, varian supplier, dsb).
- **Universal Barcode Lookup**:
  - Endpoint Kasir POS (`GET /products/barcode/:code`) mencari di `products.barcode`, `products.sku`, dan seluruh barcode di `product_barcodes`.
  - Jika kasir men-scan kemasan lama ATAU kemasan baru, kasir tetap mendeteksi produk yang sama secara realtime dan memasukkannya ke keranjang belanja.

---

## 3. Perubahan Backend (Rust)

### src-tauri/src/api/master.rs
- Menambahkan endpoint RESTful:
  1. `GET /api/v1/products/:id/barcodes` — Mengambil barcode utama dan seluruh daftar barcode alias.
  2. `POST /api/v1/products/:id/barcodes` — Mendaftarkan barcode baru ke produk dengan validasi keunikan lintas produk. Mendukung opsi `set_as_primary: true` (menukar barcode utama & mengarsipkan barcode lama ke alias).
  3. `DELETE /api/v1/products/:id/barcodes/:barcode_id` — Menghapus barcode alias.
  4. `PUT /api/v1/products/:id/barcodes/:barcode_id/set-primary` — Mengubah barcode alias menjadi Barcode Utama, dan barcode utama sebelumnya otomatis disimpan sebagai barcode alias.
  5. `POST /api/v1/products/barcode-scan` — Endpoint pengecekan cepat barcode:
     - Mengembalikan `{ found: true, is_alias: bool, product: {...} }` jika barcode sudah dikenal.
     - Mengembalikan `{ found: false, scanned_barcode: "..." }` jika barcode baru dan belum terdaftar.
- Memperbarui `list_products_handler`:
  - Menyertakan kolom `barcode_count` (jumlah barcode tambahan aktif).
  - Search filter otomatis mencakup `product_barcodes.barcode`.

### src-tauri/src/models/inventory.rs & inventory_service.rs
- Menambahkan field opsional `new_barcode: Option<String>` dan `new_barcode_desc: Option<String>` pada `GoodsReceiptItemInput`.
- Di fungsi `process_goods_receipt`, jika ada barang masuk yang memiliki `new_barcode`, sistem secara otomatis mendaftarkannya ke tabel `product_barcodes`.

---

## 4. Perubahan Frontend (Vue 3 / TypeScript)

### A. Komponen Baru: `src/components/products/ProductBarcodesModal.vue`
Modal interaktif dan intuitif untuk mengelola multi-barcode produk:
- Menampilkan barcode utama saat ini dengan indikator status aktif.
- Formulir scan/tambah barcode baru dengan opsi: *"Jadikan langsung sebagai Barcode Utama (barcode lama otomatis jadi alias)"*.
- Tabel daftar barcode tambahan/alias lengkap dengan catatan (misal: "Kemasan Baru 2026"), tanggal pendaftaran, tombol **"Jadikan Utama"** (Star) dan tombol **"Hapus"** (Trash).
- Dilengkapi edukasi visual mengenai alur barcode kemasan.

### B. Master Produk: `src/pages/ProductsPage.vue`
- Kolom Barcode di tabel produk menampilkan barcode utama dan tombol badge interaktif `+X Barcode`.
- Tombol aksi **Barcode** pada setiap baris produk untuk membuka modal manajemen multi-barcode.
- Modal di-teleport ke `body` untuk performa render optimal.

### C. Menu Pengadaan / Penerimaan Barang (GRN): `src/pages/PurchasesPage.vue`
Alur lengkap terintegrasi saat menerima barang datang dari supplier:
1. **Quick Barcode Scanner Box**:
   - Kolom scan barcode kemasan barang masuk.
   - Jika barcode **Dikenal**: barang otomatis ditambahkan ke daftar GRN (atau Qty bertambah +1 jika sudah ada) dengan notifikasi sukses.
   - Jika barcode **Belum Dikenal (Kemasan Baru)**:
     - Membuka **Modal Dialog Tautkan Barcode Baru**:
       - Menampilkan kode barcode fisik yang di-scan.
       - Pilihan memilih produk yang sesuai dari daftar master produk.
       - Input catatan kemasan (misal: "Kemasan Baru dari Supplier Sumber Berkah").
       - Opsi "Jadikan barcode ini sebagai Barcode Utama".
       - Tombol **"Tautkan Barcode & Tambah ke GRN"**: otomatis mendaftarkan barcode tersebut ke backend dan langsung memasukkan barang tersebut ke form penerimaan GRN!
2. **Tombol Kelola Multi-Barcode per Baris GRN**:
   - Staf dapat melihat barcode saat ini dan mengklik tombol "Kelola Multi-Barcode" di setiap baris barang masuk untuk melihat atau mengubah barcode secara langsung on-the-spot.

---

## 5. Hasil Pengujian & Verifikasi API

| Kasus Uji | Permintaan / Aksi | Hasil Respon | Status |
|---|---|---|---|
| **Ambil Multi Barcode** | `GET /products/:id/barcodes` | Mengembalikan primary barcode & list alias | ✅ Berhasil |
| **Tambah Barcode Baru** | `POST /products/:id/barcodes` | Barcode `8992026001001` tersimpan sebagai alias | ✅ Berhasil |
| **Scan di Kasir POS** | `GET /products/barcode/8992026001001` | Mendeteksi produk `"Teh Hijau Botol Import"` | ✅ Berhasil |
| **Pencarian Master Produk** | `GET /products?search=8992026001001` | Produk ditemukan dengan `barcode_count: 1` | ✅ Berhasil |
| **Tukar Barcode Utama** | `PUT /products/:id/barcodes/:id/set-primary` | Barcode baru jadi utama, barcode lama otomatis jadi alias | ✅ Berhasil |
| **Scan Kemasan Lama di Kasir** | `GET /products/barcode/8999990001` | Tetap mendeteksi produk yang sama di kasir | ✅ Berhasil |
| **Scan Kemasan Baru di Kasir** | `GET /products/barcode/8992026001001` | Tetap mendeteksi produk yang sama di kasir | ✅ Berhasil |
| **Deteksi Barcode Baru di GRN** | `POST /products/barcode-scan` | Mengembalikan `found: false` untuk barcode belum terdaftar | ✅ Berhasil |
| **Kompilasi Frontend** | `npm run build` | Exit code 0, 1745 modules transformed | ✅ Berhasil |
| **Kompilasi Backend** | `cargo build` | Finished dev profile (unoptimized + debuginfo) | ✅ Berhasil |
