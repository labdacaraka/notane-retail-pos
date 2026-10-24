# Dokumen Hasil Verifikasi & Rekomendasi Fitur Tambahan Aplikasi Kasir Ritel (Standar Minimarket / Indomaret / Alfamart)

*Dokumen ini telah direvisi setelah dilakukan audit menyeluruh pada seluruh modul sistem dan penjelajahan langsung pada antarmuka admin panel (`http://localhost:5175/admin`).*

---

## 1. Hasil Audit & Koreksi Fitur yang Ternyata SUDAH TERSEDIA di Sistem

Setelah dilakukan penelusuran mendalam terhadap kode sumber dan antarmuka web, beberapa fitur yang sebelumnya diduga belum ada, **ternyata telah terimplementasi dengan baik** di dalam aplikasi:

| Fitur | Status Temuan di Aplikasi | Lokasi & Implementasi Aktif |
|---|---|---|
| **Stock Opname Fisik** | **SUDAH ADA** | Terletak di menu **Stok & Opname** (`/admin/inventory` - Tab 1). Terdapat tabel perbandingan stok sistem vs fisik aktual, kalkulasi selisih otomatis, dan tombol *"Simpan & Bukukan Jurnal Opname"* yang langsung mencatat jurnal penyesuaian persediaan ke akuntansi. |
| **Pencatatan Barang Rusak / Expired** | **SUDAH ADA** | Terintegrasi di modul penyesuaian stok (`/admin/inventory`) dengan master kode alasan `ADJ_EXP` (*Barang Kedaluwarsa / Rusak*) dan `ADJ_OPNAME` (*Penyesuaian Fisik Stock Opname*). |
| **Integrasi Barcode Timbangan Digital (Fresh Produce)** | **SUDAH ADA** | Sistem sudah memiliki opsi `is_weight_embedded` di Master Produk (`/admin/products`), parser barcode timbangan EAN-13 (prefix `20`/`21`/`22`) di backend, dan keranjang kasir (`CartTable.vue`) mendukung kuantitas desimal 3 digit (`step="0.001"` kg). |
| **Peringatan Stok Minimum (*Low Stock Alert*)** | **SUDAH ADA** | Setiap produk memiliki kolom `min_stock` di Master Produk (`/admin/products`), badge penanda warna amber/merah saat stok menipis, dan filter produk kategori `LOW` (Stok Rendah). |
| **Tiering Member & Loyalitas Poin** | **SUDAH ADA** | Terletak di menu **Pelanggan & Member** (`/admin/customers`). Sudah tersedia manajemen grup tier (Regular, Member, Wholesale, Platinum), aturan konversi poin ke rupiah, batas maksimal pembayaran poin, dan modul **Voucher Diskon** (`/admin/vouchers`). |
| **Pembatalan Transaksi (*Void*)** | **SUDAH ADA** | Terletak di menu **Transaksi Penjualan** (`/admin/sales`). Transaksi dapat dibatalkan dengan proteksi PIN Supervisor, mengembalikan stok ke gudang (*RETURN_IN*), dan mencatat jurnal pembalik akuntansi. |

---

## 2. Fitur yang BENAR-BENAR BELUM ADA (Gap Analysis Standar Toko Ritel Modern)

Berikut adalah daftar fitur yang **murni belum tersedia** di sistem dan sangat krusial bagi operasional toko ritel modern berantai (seperti Indomaret / Alfamart):

---

### A. Prioritas 1: Kritis untuk Meja Kasir & Operasional Harian (High Priority)

#### 1. Mesin Promo Otomatis Ritel (*Promotion Engine: Buy X Get Y & Tebus Murah*)
* **Kondisi Saat Ini:** Sistem saat ini baru mendukung diskon manual kasir (dengan PIN supervisor), harga khusus member, voucher kode, dan flash sale berdasarkan jam.
* **Kebutuhan Standar Minimarket:** Sistem promo otomatis yang langsung terhitung di keranjang kasir tanpa input manual kasir:
  - **Beli X Gratis Y (*Buy X Get Y*):** Contoh: Beli 2 Teh Botol Gratis 1, Beli 1 Kopi ABC Gratis 1 Gula Pasir.
  - **Tebus Murah (*Purchase with Purchase / PWP*):** Contoh: Belanja minimal Rp 50.000 di luar rokok, berhak membeli Minyak Goreng 1L seharga Rp 10.000.
  - **Paket Hemat / Bundling:** Contoh: Beli Roti Sari Roti + Susu Ultra Milk otomatis dipotong diskon paket Rp 2.500.
* **Manfaat:** Mendorong penjualan impulsif dan meningkatkan rata-rata belanja konsumen (*Average Basket Size*) seperti program JSM minimarket.

---

#### 2. Retur Penjualan Parsial Konsumen di Layar Kasir POS (*Customer Partial Return / Exchange*)
* **Kondisi Saat Ini:** Fitur yang ada adalah *Void Transaksi Penuh* di menu admin penjualan. Tidak bisa melakukan retur jika konsumen hanya ingin mengembalikan 1 dari 5 barang yang dibeli.
* **Kebutuhan Standar Minimarket:** Tombol menu di layar kasir POS:
  - Kasir memasukkan nomor faktur nota struk belanja konsumen sebelumnya.
  - Kasir memilih barang mana yang cacat/rusak/salah beli dan memasukkan jumlah retur.
  - Validasi otorisasi PIN Supervisor / Asisten Toko (MOD).
  - Kasir memilih opsi: **Kembalikan Uang Tunai (*Refund*)** atau **Tukar Barang Sejenis (*Exchange*)**.
  - Cetak Struk Bukti Retur Penjualan.
* **Manfaat:** Standar pelayanan pelanggan wajib toko ritel modern tanpa harus membatalkan keseluruhan transaksi lama.

---

#### 3. Penarikan Uang Laci Kasir ke Brankas (*Cash Drop / Pick Up Kasir*)
* **Kondisi Saat Ini:** Shift kasir mencatat modal awal dan settlement akhir shift. Selama shift berjalan, uang tunai menumpuk di laci kasir meja depan.
* **Kebutuhan Standar Minimarket:** 
  - Notifikasi peringatan otomatis di layar kasir ketika saldo tunai di laci melewati ambang batas aman (misalnya laci melebihi Rp 2.000.000).
  - Menu *"Setor Uang / Cash Drop"*: Kasir menyerahkan sebagian uang tunai (misal Rp 1.500.000) ke kepala toko/supervisor untuk disimpan di brankas belakang.
  - Sistem mencetak slip tanda terima penyerahan kas bertandatangan kasir dan supervisor.
* **Manfaat:** Mencegah kerugian finansial akibat risiko pencurian, perampokan, atau selisih uang di meja depan.

---

### B. Prioritas 2: Efisiensi Pengadaan & Kontrol Kualitas Produk (Medium Priority)

#### 4. Pelacakan Tanggal Kedaluwarsa di Penerimaan Barang (*Expired Date Tracking & Notifikasi FEFO*)
* **Kondisi Saat Ini:** Tabel database `product_batches` sudah memiliki kolom `expiry_date`, namun pada form Penerimaan Barang (GRN di menu `/admin/purchases`) belum ada input tanggal kedaluwarsa per batch barang masuk.
* **Kebutuhan Standar Minimarket:**
  - Form input tanggal kedaluwarsa (*Expired Date*) saat staf menerima faktur barang dari pemasok/supplier.
  - Widget / Laporan peringatan dini di dashboard: **Daftar Barang Mendekati ED** (H-60, H-30, H-14, H-7) agar staf toko segera memajang barang tersebut di rak depan (*First-Expired First-Out / FEFO*) atau mengembalikan (*retur expired*) ke supplier sebelum basi.
* **Manfaat:** Menghindari denda sidak BPOM, melindungi konsumen dari produk basi, dan menekan kerugian barang rusak.

---

#### 5. Pembuatan Draf PO Otomatis Berdasarkan Stok Rendah (*Auto-Generate Reorder PO*)
* **Kondisi Saat Ini:** Sistem sudah menandai produk yang stoknya `<= min_stock`, namun pembuatan Pesanan Pembelian (PO di `/admin/purchases`) masih harus memilih barang satu per satu secara manual.
* **Kebutuhan Standar Minimarket:**
  - Tombol *"Tarik Barang Stok Rendah / Auto Reorder"* di modul PO.
  - Sistem otomatis menyusun draf pesanan pembelian berisi daftar semua barang yang berada di bawah titik minimum stok beserta kuantitas saran order ke supplier terkait.
* **Manfaat:** Menghemat waktu staf pengadaan toko dan mencegah kekosongan barang terlaris (*Out of Stock*).

---

#### 6. Pembulatan Kembalian Kasir & Opsi Donasi (*Cash Rounding & Donation Policy*)
* **Kondisi Saat Ini:** Total belanja dan kembalian tunai dihitung sampai ke pecahan rupiah terkecil (misal Rp 15.340), sementara uang fisik logam pecahan kecil sering langka di kasir.
* **Kebutuhan Standar Minimarket:**
  - Opsi pengaturan pembulatan kasir (misal: pembulatan ke bawah kelipatan Rp 100 terdekat).
  - Opsi penawaran donasi resmi kemanusiaan di kasir (misal sisa Rp 60 didonasikan dengan persetujuan konsumen) yang tercatat rapi pada struk dan akun penampungan akuntansi.
* **Manfaat:** Menghilangkan perdebatan antara kasir dan pembeli seputar ketiadaan uang kembalian receh.

---

#### 7. Pemindaian Cepat Stock Opname per-Rak (*Barcode Scanner Cycle Count Mode*)
* **Kondisi Saat Ini:** Modul opname di `/admin/inventory` menampilkan tabel seluruh produk sekaligus dan mengharuskan staf mencari nama produk di tabel.
* **Kebutuhan Standar Minimarket:**
  - Mode *"Scan Cepat Opname"* di halaman inventori: staf cukup mengarahkan barcode scanner ke barang di rak lorong display, kuantitas fisik bertambah otomatis `+1` per scan atau input kuantitas angka langsung.
  - Riwayat audit opname tersimpan rapi per tanggal dan per penanggung jawab.
* **Manfaat:** Mempercepat proses opname harian rak toko tanpa mengganggu antrean kasir.

---

### C. Prioritas 3: Layanan Tambahan Ritel Modern (Value-Added Services)

#### 8. Layanan Loket PPOB & Pembayaran Tagihan Terintegrasi (*Biller / Top-Up Service*)
* **Kebutuhan Standar Minimarket:** Integrasi API agregator pembayaran tagihan pihak ketiga di layar kasir (Token PLN, Pulsa, BPJS, PDAM, Top Up e-Money/GoPay/ShopeePay).
* **Manfaat:** Memposisikan toko sebagai pusat transaksi terpadu (*One-Stop Solution*) bagi masyarakat sekitar dan memberikan pendapatan komisi tambahan (*Fee-Based Income*).

---

## 3. Matriks Rekomendasi Fitur Tambahan & Status Pengerjaan

| No | Nama Fitur | Kategori Modul | Prioritas | Status Implementasi |
|---|---|---|---|---|
| **1** | **Mesin Promo Otomatis (Buy X Get Y & Tebus Murah)** | Kasir POS & Promo | **Prioritas 1** | ✅ **SELESAI** (`/promotions`, `CartTable`, engine evaluasi promo real-time) |
| **2** | **Retur Parsial Konsumen di Layar Kasir** | Kasir POS | **Prioritas 1** | ✅ **SELESAI** (`SalesReturnModal.vue`, otorisasi PIN SPV, restock/waste, jurnal COA 4103) |
| **3** | **Setor Kas Laci Meja Kasir (*Cash Drop / Pick Up*)** | Kasir POS & Keuangan | **Prioritas 1** | ✅ **SELESAI** (`CashDropModal.vue`, limit brankas, rekonsiliasi shift otomatis) |
| **4** | **Input Expired Date di Penerimaan Barang & Alert FEFO** | Pembelian & Gudang | **Prioritas 2** | ✅ **SELESAI** (Input ED & Batch di GRN, tab Alert FEFO di `/admin/inventory`) |
| **5** | **Auto-Generate Draf PO dari Stok Minimum** | Pengadaan & Pembelian | **Prioritas 2** | ✅ **SELESAI** (Tombol ⚡ Tarik Stok Menipis di Modal Buat PO `/admin/purchases`) |
| **6** | **Pembulatan Kembalian Kasir & Opsi Donasi** | Kasir POS & Pengaturan | **Prioritas 2** | ✅ **SELESAI** (Tab 4 di `/admin/receipt-settings` & widget donasi di `QuickPayModal.vue`) |
| **7** | **Mode Scan Cepat Barcode pada Stock Opname Rak** | Inventori & Stok | **Prioritas 2** | ✅ **SELESAI** (Toggle Fast Scan Mode + scanner audio di `/admin/inventory`) |
| **8** | **Layanan Pembayaran PPOB & Top-Up** | Kasir POS & Integrasi | **Prioritas 3** | ⏳ Rencana Integrasi API Partner Masa Depan |

---

## 4. Kesimpulan Akhir & Hasil Pengujian

Seluruh fitur tambahan **Prioritas 1** dan **Prioritas 2** telah berhasil diimplementasikan 100% pada seluruh layer:
1. **Database:** Migrasi batch `037_retail_features_p1_p2.sql` (tabel promo, sales returns, cash drops, kolom ED batch GRN, kolom receipt cash rounding, COA 4103 & 2104, role permissions RBAC).
2. **Backend Rust:** Handler Axum API Zero-Trust dengan validasi semantik DTO strongly-typed, auto-jurnal akuntansi berpasangan, dan perhitungan shift cash reconciliation.
3. **Frontend Vue 3:** UI Back-Office dan POS Register telah teruji bebas error type checking TypeScript (`vue-tsc` exit code 0) dan diverifikasi secara visual melalui browser live.

Aplikasi **Notane Retail POS & ERP** kini telah memenuhi standar operasional minimarket modern setara Indomaret & Alfamart.
