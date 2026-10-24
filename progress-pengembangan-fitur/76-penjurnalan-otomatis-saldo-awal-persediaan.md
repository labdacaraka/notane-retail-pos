# Dokumentasi Fitur: 76. Penjurnalan Otomatis Saldo Awal Persediaan (Initial Stock)

## 1. Latar Belakang Masalah
Sebelum perbaikan ini:
1. Pada instalasi baru atau migrasi database awal (`002_seed_data.sql`), data produk demo dimasukkan dengan stok fisik di `warehouse_stocks` dan `product_stock_batches` tanpa membukukan jurnal akuntansi saldo awal (`INITIAL_STOCK`).
2. Ketika transaksi penjualan atau pengiriman transfer terjadi di POS, sistem akuntansi perpetual mencatat pengurangan persediaan (`Kredit: 1201 Persediaan Barang Dagang Retail`) dan pengakuan HPP (`Debit: 5101 HPP`).
3. Akibat tidak adanya saldo debit pembuka di akun 1201, saldo buku besar akun 1201 menjadi **minus** (misal: `-Rp 68.300` di Notane Retail Flagship Store).
4. Selain pada instalasi server baru (seed data), pembuatan produk baru dengan stok awal (`create_product_handler`) maupun impor produk via Excel/CSV (`import_products_handler`) di runtime belum membuat entri jurnal akuntansi berpasangan secara otomatis.

---

## 2. Solusi & Perubahan yang Diterapkan

### A. Migrasi Database `055_journal_entry_for_initial_stocks.sql`
- Membuat migrasi yang mengevaluasi seluruh outlet yang memiliki persediaan awal dari `product_stock_batches` (`reference_type = 'INITIAL_STOCK'`) atau sisa kuantitas di `warehouse_stocks` yang belum memiliki jurnal `INITIAL_STOCK`.
- Menghitung total nilai persediaan awal berdasarkan `initial_qty * cost_price`.
- Membukukan otomatis jurnal double-entry:
  - **Debit**: `1201 - Persediaan Barang Dagang Retail` (akun persediaan outlet terkait) sebesar total nilai persediaan awal.
  - **Kredit**: `3101 - Modal Awal Pemilik` (akun ekuitas modal disetor pemilik) sebesar total nilai persediaan awal.
- Sesuai arsitektur *Dimensional Accounting* (Migrasi 054), identitas cabang dicatat pada detail transaksi `journal_lines.outlet_id`.

### B. Pembaruan Seed Data Instalasi Baru (`002_seed_data.sql` & `052_rekening_antar_kantor_dan_jurnal_cabang.sql`)
- Menambahkan entri jurnal saldo awal demo (`JRN-INIT-OUT-001`) di `002_seed_data.sql` bernilai **Rp 1.676.000** (150 Indomie @ Rp 2.800 + 80 Ultra Milk @ Rp 5.500 + 25.5 Apel Fuji @ Rp 32.000).
- Menambahkan aturan backfill `INITIAL_STOCK` di `052_rekening_antar_kantor_dan_jurnal_cabang.sql` agar saat migrasi lama dijalankan di instalasi server baru, kolom dimensi cabang tertaut secara sempurna.

### C. Penjurnalan Otomatis Dinamis di Server (`master.rs`)
Menambahkan fungsi pembantu `record_initial_stock_journal`:
1. **Tambah Produk Baru (`create_product_handler`)**:
   - Jika `payload.initial_stock > 0` dan `cost_price > 0`, sistem langsung menerbitkan nomor jurnal `JRN-INIT-<TIMESTAMP>-<RANDOM>`, mendebit akun persediaan outlet terkait (`1201`) dan mengkredit modal pemilik (`3101`).
2. **Impor Produk Excel/CSV (`import_products_handler`)**:
   - Merekam record batch FIFO di `product_stock_batches` dengan `reference_type = 'INITIAL_STOCK'`.
   - Menjalankan `record_initial_stock_journal` untuk setiap produk baru yang memiliki stok awal dan harga pokok.

---

## 3. Hasil Verifikasi Neraca (Balance Sheet)

Pengecekan laporan posisi keuangan / neraca setelah migrasi 055 diterapkan:

| Filter Neraca | Total Aktiva (Aset) | Total Pasiva (Kewajiban + Ekuitas) | Saldo Akun 1201 | Status Keseimbangan |
|---|---|---|---|---|
| **Notane Retail Flagship Store** | Rp 2.692.355 | Rp 2.692.355 | **+Rp 1.607.700** | **Balanced ✓** |
| **Cabang Tegal Besar** | Rp 128.000 | Rp 128.000 | **+Rp 28.000** | **Balanced ✓** |
| **Konsolidasian (Semua Cabang)** | Rp 2.702.355 | Rp 2.702.355 | **+Rp 1.635.700** | **Balanced ✓** |

Persamaan dasar akuntansi `Aktiva = Kewajiban + Ekuitas` terpenuhi 100% dan saldo persediaan tidak lagi minus di cabang mana pun.
