# Modul 31: Cetak Label Harga Produk Masal (Multi-Outlet & Multi-Barcode)

## 📌 Ringkasan Fitur
Modul **Cetak Label Harga Masal** (`/admin/price-labels`) dirancang untuk mempermudah operasional toko, minimarket, dan supermarket dalam memperbarui label harga di rak pajang (*shelf talker / price tag*) maupun mencetak stiker barcode produk yang ditempel langsung ke kemasan barang.

Fitur ini secara khusus menjamin **akurasi harga terkini yang sinkron dengan outlet yang dipilih** (Multi-Outlet Pricing), sehingga perbedaan harga antar cabang/outlet tidak akan salah cetak.

---

## 🎯 Fitur & Kemampuan Utama

### 1. Sinkronisasi Harga Sesuai Outlet Terpilih
- Dropdown selector outlet di bagian atas.
- Ketika outlet dipilih, query backend `GET /api/v1/products?outlet_id=<outlet_id>` dipanggil.
- Backend secara otomatis melakukan `COALESCE(pop.base_selling_price, p.base_selling_price)` dari tabel `product_outlet_prices` sehingga harga yang tampil dan dicetak adalah **harga aktif toko/outlet yang bersangkutan**, bukan harga global jika ada penyesuaian cabang.
- Terdapat badge indikator status harga outlet terpilih untuk transparansi kasir / store manager.

### 2. Multi-Barcode & Varian Kemasan
- Menampilkan barcode produk utama.
- Menghasilkan **Barcode Vektor SVG Code-128B Standar** resolusi tinggi (`src/utils/barcodeSvg.ts`) tanpa dependensi pustaka luar.
- Barcode dapat dipindai (*scannable*) oleh berbagai scanner barcode optik (CCD / Laser 1D) maupun smartphone scanner.

### 3. Pilihan Ukuran & Format Label Fleksibel
Mendukung 3 ukuran standar retail:
1. **Label Rak Supermarket (Shelf Tag 65 × 35 mm)**:
   - Menampilkan: Nama Toko, Nama Produk, SKU / Kategori, Barcode SVG + teks angka, Harga Jumbo tebal, Satuan Dasar (`/ PCS`, `/ DUS`, dll.), dan Tanggal Cetak.
2. **Sticker Produk / Barcode (40 × 25 mm)**:
   - Format kompak untuk ditempel langsung di kemasan produk.
   - Barcode SVG scannable, nama produk, harga, dan kode SKU.
3. **Mini Jewelry / Cosmetics Tag (30 × 20 mm)**:
   - Khusus barang kecil (aksesoris, kosmetik, perkakas kecil).

### 4. Pilihan Format Kertas
- **Thermal Roll (Continuous)**: Cocok untuk printer barcode kasir / stiker continuous roll (misal: Xprinter, Zebra, Postek, Panda, Iware).
- **Sheet Grid A4**: Cocok untuk kertas stiker label plano / kertas HVS A4 yang dicetak dengan printer kantor biasa / laserjet, tersusun rapi dalam grid responsif.

### 5. Quick Bulk Action & Seleksi Cepat
- Seleksi per produk atau pilih semua hasil filter (*Select All Filtered*).
- Tombol aksi masal:
  - **Semua Qty = 1**: Setel 1 lembar label per produk terpilih.
  - **Semua Qty = 2**: Setel 2 lembar per produk (cocok untuk rak ganda / cadangan).
  - **Sesuai Stok**: Otomatis mengatur lembar cetak sesuai stok tercatat produk.
  - **Reset**: Batalkan semua seleksi dengan satu klik.

### 6. Kustomisasi Informasi Label (Live Toggle)
Pengguna dapat memilih informasi apa saja yang ingin dimunculkan pada stiker melalui checkbox konfigurasi:
- Tampilkan Nama Toko (bisa disesuaikan teksnya)
- Tampilkan Barcode Scannable
- Tampilkan Satuan Produk
- Tampilkan Tanggal Cetak (berguna untuk audit perubahan harga rak)
- Tampilkan SKU Produk

### 7. WYSIWYG Live Preview & Print CSS Presisi
- Tampilan pratinjau langsung di layar menyerupai hasil cetakan asli.
- Aturan cetak CSS `@media print` presisi:
  - `@page { margin: 3mm; size: auto; }`
  - Isolasi elemen cetak (menyembunyikan sidebar, navbar, tombol, dan form input).
  - Penghindaran potong stiker (`page-break-inside: avoid; break-inside: avoid;`).

---

## 🛠️ File Terkait
1. `src/utils/barcodeSvg.ts` — Engine pembuat SVG Code-128 murni.
2. `src/pages/PrintLabelsPage.vue` — Tampilan utama & logic cetak label harga masal.
3. `src/router/index.ts` — Rute `/admin/price-labels` dengan guard permission `master:read`.
4. `src/layouts/AdminLayout.vue` — Menu sidebar "Cetak Label Harga".
5. `src/pages/ProductsPage.vue` — Tombol shortcut pintas di ribbon atas katalog produk.
