# Progress Pengembangan Fitur: Menu Transaksi Penjualan & Pengadaan Pembelian (Back-Office)

**Tanggal:** 20 September 2026  
**Status:** Selesai (Completed & Verified via Automated Browser Test)

---

## 1. Latar Belakang & Kebutuhan Bisnis
Berdasarkan dokumen arsitektur dan rancangan basis data di `perencanaan/perencanaan_aplikasi_kasir_retail.md`, terdapat tabel inti transaksi yang belum memiliki antarmuka (UI/UX) dan menu operasional di Back-Office:
1. **Modul Penjualan (`sales`, `sale_items`, `sale_payments`):**
   - Riwayat transaksi kasir POS, rincian produk yang dibeli beserta harga satuan & diskon, metode pembayaran (tunai, QRIS, kartu), rekap KPI omzet ritel, cetak ulang struk thermal tanpa dialog, dan pembatalan transaksi (*void*).
2. **Modul Pengadaan & Pembelian (`purchase_invoices`, `purchase_invoice_items`, `suppliers`, `purchase_orders`):**
   - Penerimaan faktur barang masuk / *Goods Receipt Note* (GRN), penambahan stok gudang otomatis, penghitungan harga pokok rata-rata tertimbang (WMA), pencatatan tagihan hutang dagang supplier, pelunasan pembayaran, serta master katalog pemasok (*vendor directory*).

---

## 2. Implementasi Backend & API Layer (Rust Axum + SQLx)

### A. Modul Penjualan (`src-tauri/src/services/pos_service.rs` & `src-tauri/src/api/pos.rs`)
1. **`GET /api/v1/sales/stats`:**
   - Menghitung statistik penjualan real-time: `total_revenue`, `total_transactions`, `total_items_sold`, `total_discount`, dan `avg_basket_size` (ATV).
2. **`GET /api/v1/sales`:**
   - Menampilkan riwayat transaksi penjualan dengan *JOIN* ke tabel `customers`, `cashier_shifts`, dan `users` untuk menyajikan nama kasir, nomor terminal register, serta nama & nomor telepon pelanggan.
3. **`GET /api/v1/sales/:id`:**
   - Menampilkan rincian faktur lengkap beserta relasi `sale_items` (nama produk, SKU, barcode, satuan UoM, harga beli HPP, harga jual, diskon, subtotal) dan `sale_payments` (nama metode bayar, kategori, nominal, nomor referensi).
4. **`POST /api/v1/sales/:id/void`:**
   - Melakukan pembatalan transaksi dengan otorisasi PIN supervisor, pengembalian kuantitas stok barang ke gudang, dan pembalikan jurnal akuntansi.

### B. Modul Pengadaan & Pembelian (`src-tauri/src/services/inventory_service.rs` & `src-tauri/src/api/inventory.rs`)
1. **`GET /api/v1/inventory/purchases`:**
   - Mengambil seluruh daftar faktur pembelian barang masuk (`purchase_invoices`) beserta nama supplier, nama gudang, tanggal faktur, tanggal jatuh tempo, total tagihan, dan status pembayaran (`UNPAID` / `PAID`).
2. **`GET /api/v1/inventory/purchases/:id`:**
   - Mengambil detail faktur GRN beserta daftar baris item produk yang diterima (`purchase_invoice_items`), jumlah kuantitas, harga beli satuan, dan diskon.
3. **`POST /api/v1/inventory/purchases/:id/pay`:**
   - Mengubah status faktur pembelian menjadi `PAID` (lunas).
4. **`POST /api/v1/inventory/grn`:**
   - Memproses penerimaan barang fisik dari supplier (*Goods Receipt Note*), menambah stok gudang (`warehouse_stocks`), mencatat mutasi stok (`stock_mutations`), memperbarui HPP rata-rata produk (`cost_price`), dan membukukan jurnal persediaan vs hutang dagang supplier.
5. **`GET /api/v1/inventory/purchase-orders`:**
   - Menampilkan daftar *Purchase Order* (PO) ke supplier.
6. **`GET /api/v1/suppliers`, `POST`, `PUT`, `DELETE`:**
   - CRUD lengkap master data pemasok/distributor ritel.

---

## 3. Implementasi Frontend (Vue 3 + Tailwind CSS + Pinia)

### A. Halaman Transaksi Penjualan (`src/pages/SalesPage.vue`)
- **4 Kartu KPI Utama:**
  - Total Omzet Penjualan (Rp)
  - Total Item Terjual (Unit/Kg)
  - Rata-Rata Belanja (*Average Basket Size*)
  - Total Diskon & Potongan (Rp)
- **Tabel Transaksi Interaktif:**
  - Pencarian cepat berdasarkan nomor faktur, kasir, pelanggan, atau catatan.
  - Filter status transaksi: Semua, Selesai (*COMPLETED*), dan Dibatalkan (*VOID*).
  - Badge status warna modern (Emerald untuk selesai, Rose untuk void).
  - Tombol aksi cepat: Pratinjau Detail Faktur (Mata), Cetak Ulang Struk Thermal (Printer), dan Batalkan Transaksi (Void).
- **Modal Rincian Faktur:**
  - Grid metadata transaksi (Waktu, Kasir, Terminal POS, Pelanggan).
  - Tabel rincian produk yang dibeli beserta SKU, Barcode, Satuan UoM, Qty, Harga Satuan, Diskon, dan Subtotal.
  - Rincian metode pembayaran dan kalkulasi kembalian.
  - Integrasi tombol cetak ulang ke `printerStore.printReceipt(detail)` langsung ke printer thermal fisik / virtual simulator tanpa dialog browser.
- **Modal Konfirmasi Void Transaksi:**
  - Verifikasi PIN supervisor (default `123456`) dan pilihan alasan pembatalan.

### B. Halaman Pengadaan & Pembelian (`src/pages/PurchasesPage.vue`)
- **Navigasi 3 Tab:**
  1. *Faktur Pembelian Masuk (GRN)*: Tabel histori penerimaan barang masuk dan tagihan supplier.
  2. *Master Pemasok / Supplier*: Katalog vendor distributor lengkap dengan tombol tambah/edit dan term pembayaran.
  3. *Pesanan Pembelian (PO)*: Pemantauan status pemesanan ke distributor.
- **Modal Penerimaan GRN Baru:**
  - Pilihan pemasok & gudang penerima.
  - Nomor Surat Jalan / Faktur Supplier, tanggal faktur, dan tanggal jatuh tempo.
  - Baris item produk dinamis (tambah/hapus baris barang).
  - Otomatis menghitung subtotal dan total tagihan GRN.
  - Tombol "Tandai Lunas / Bayar" untuk menyelesaikan tagihan supplier.

### C. Integrasi Navigasi & Dashboard Hub
- **`src/router/index.ts`:**
  - Menambahkan route `/admin/sales` (`SalesPage.vue`).
  - Menambahkan route `/admin/purchases` (`PurchasesPage.vue`).
- **`src/layouts/AdminLayout.vue`:**
  - Menambahkan menu *Penjualan* (`streamline-ultimate-color:receipt-bill`) dan *Pembelian* (`streamline-ultimate-color:truck-delivery-1`) pada deretan menu pita atas (*top command ribbon*).
- **`src/pages/AdminDashboard.vue`:**
  - Menambahkan ubin peluncur pada kelompok **TRANSAKSI & OPERASIONAL BISNIS** untuk modul *Transaksi Penjualan* dan *Pengadaan & Pembelian*.

---

## 4. Hasil Verifikasi Otomatis (Browser Subagent)
Pengujian fungsionalitas dan visual dilakukan menyeluruh menggunakan browser subagent:
1. **Navigasi Dashboard & Ribbon:** Menu Penjualan dan Pembelian berhasil tampil rapi dengan ikon warna-warni Streamline Ultimate Color.
2. **Modul Penjualan:**
   - 4 kartu ringkasan KPI berhasil memuat data riwayat transaksi penjualan.
   - Modal detail faktur berhasil dibuka dengan rincian item, harga, dan metode pembayaran.
   - Fitur "Cetak Ulang Struk Thermal" berhasil dieksekusi dengan notifikasi sukses.
3. **Modul Pengadaan (Pembelian):**
   - Tab Faktur Pembelian (GRN), Master Pemasok, dan Pesanan Pembelian (PO) berfungsi responsif.
   - Input penerimaan faktur GRN baru berhasil disimpan dan langsung masuk ke tabel transaksi.
