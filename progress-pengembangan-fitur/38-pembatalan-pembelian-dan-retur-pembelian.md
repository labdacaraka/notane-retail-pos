# Progress Pengembangan Fitur: Pembatalan Dokumen Pembelian & Menu Retur Pembelian

## 1. Ringkasan Fitur
Menanggapi kebutuhan alur operasional pengadaan barang (procurement):
1. **Pembatalan Dokumen Pembelian (Purchase Document Cancellations)**:
   - **Pembatalan Purchase Order (PO)**: Pembatalan pesanan barang yang belum selesai diterima.
   - **Pembatalan Penerimaan Barang Fisik (GRN - Goods Receipt Note)**: Membatalkan penerimaan barang jika faktur belum diterbitkan. Sistem secara otomatis mengembalikan (mengurangi) stok fisik di gudang/outlet terkait, mencatat mutasi stok pembalik `PURCHASE_GRN_CANCEL`, mengembalikan kuantitas diterima pada item PO, memulihkan status PO ke `PENDING` atau `PARTIAL`, serta membalikkan jurnal akuntansi Persediaan vs Unbilled GRN.
   - **Pembatalan Faktur Pembelian (Purchase Invoice)**: Membatalkan tagihan pembelian yang belum dibayar (`UNPAID`). Sistem membalikkan status dokumen GRN rujukan kembali ke `UNINVOICED`, membalikkan kuantitas tertagih di item GRN, dan membalikkan jurnal akuntansi Hutang Usaha & Pajak Masukan vs Unbilled GRN.

2. **Menu Retur Pembelian ke Supplier (Purchase Returns)**:
   - Disediakan **Tab Retur Pembelian** khusus di halaman Pengadaan & Pembelian (`PurchasesPage.vue`).
   - Mampu menangani 3 metode pengembalian/kompensasi retur:
     - `DEBT_REDUCTION`: Otomatis memotong saldo hutang faktur pembelian terkait.
     - `CASH_REFUND`: Pengembalian kas/transfer langsung dari supplier (Jurnal Debet Kas `1101`).
     - `REPLACEMENT`: Penggantian fisik barang / kredit toko.
   - Mengurangi stok fisik barang dagang di gudang/outlet terkait dan mencatat mutasi stok keluar bertipe `PURCHASE_RETURN`.
   - Menghasilkan jurnal akuntansi otomatis berpasangan: Debet Hutang Usaha (`2101`) atau Kas (`1101`) vs Kredit Persediaan Barang Dagang (`1201`).

---

## 2. Struktur Database & Skema Migrasi

### File Migrasi: `src-tauri/migrations/024_purchase_returns_and_cancellations.sql`
- **Tabel `purchase_returns`**:
  - `id`: UUID (Primary Key)
  - `return_number`: VARCHAR (Unik, misal: `RET-20260930-XXXX`)
  - `supplier_id`: UUID (Referensi ke `suppliers.id`)
  - `outlet_id`: UUID (Referensi ke `outlets.id`)
  - `warehouse_id`: UUID (Referensi ke `outlets.id`)
  - `invoice_id`: UUID (Referensi opsional ke `purchase_invoices.id`)
  - `goods_receipt_id`: UUID (Referensi opsional ke `goods_receipts.id`)
  - `return_date`: DATE
  - `status`: VARCHAR (`COMPLETED`, `CANCELLED`)
  - `refund_method`: VARCHAR (`DEBT_REDUCTION`, `CASH_REFUND`, `REPLACEMENT`)
  - `reason`: TEXT (Alasan retur, misal: cacat pabrik, rusak, kadaluarsa, salah kirim)
  - `subtotal`: NUMERIC(18, 4)
  - `tax_amount`: NUMERIC(18, 4)
  - `total_amount`: NUMERIC(18, 4)
  - `notes`: TEXT
- **Tabel `purchase_return_items`**:
  - `id`: UUID (Primary Key)
  - `purchase_return_id`: UUID (Referensi ke `purchase_returns.id`)
  - `product_id`: UUID (Referensi ke `products.id`)
  - `returned_qty`: NUMERIC(14, 4)
  - `unit_price`: NUMERIC(18, 4)
  - `subtotal`: NUMERIC(18, 4)
  - `notes`: TEXT

---

## 3. Alur & Jurnal Akuntansi (Double-Entry Bookkeeping)

| Operasi | Akun Debet | Akun Kredit | Keterangan |
|---|---|---|---|
| **Terima Barang (GRN)** | `1201` Persediaan Barang Dagang | `2103` Hutang Penerimaan Belum Tertagih (Unbilled GRN) | Nilai barang fisik diterima di gudang |
| **Batal Terima Barang (GRN Cancel)** | `2103` Hutang Penerimaan Belum Tertagih | `1201` Persediaan Barang Dagang | Membalikkan jurnal persediaan dan Unbilled GRN |
| **Terbitkan Faktur Pembelian** | `2103` Hutang Penerimaan Belum Tertagih<br>`1104` Pajak Masukan (PPN) | `2101` Hutang Usaha<br>`5102` Potongan/Diskon Pembelian | Mengakui hutang resmi kepada supplier |
| **Batal Faktur Pembelian** | `2101` Hutang Usaha<br>`5102` Potongan/Diskon Pembelian | `2103` Hutang Penerimaan Belum Tertagih<br>`1104` Pajak Masukan (PPN) | Membalikkan hutang dan memulihkan status unbilled GRN |
| **Retur Pembelian (Potong Hutang)** | `2101` Hutang Usaha | `1201` Persediaan Barang Dagang | Mengurangi hutang ke supplier dan mengurangi stok persediaan |
| **Retur Pembelian (Kas Kembali)** | `1101` Kas / Bank | `1201` Persediaan Barang Dagang | Menerima pengembalian uang dari supplier dan mengurangi stok |

---

## 4. Endpoint Backend API

1. `POST /api/v1/inventory/goods-receipts/{id}/cancel`
   - Membatalkan penerimaan barang (GRN).
   - Validasi: GRN belum pernah diterbitkan faktur pembelian.
2. `POST /api/v1/inventory/purchase-invoices/{id}/cancel`
   - Membatalkan faktur pembelian.
   - Validasi: Faktur belum pernah dibayar (`paid_amount == 0`).
3. `GET /api/v1/inventory/purchase-returns`
   - Mengambil daftar dokumen retur pembelian dengan filter pencarian, filter status, dan pagination.
4. `POST /api/v1/inventory/purchase-returns`
   - Mencatat pengembalian barang baru, memotong stok fisik gudang, membuat mutasi stok `PURCHASE_RETURN`, dan mencatat jurnal akuntansi.
5. `GET /api/v1/inventory/purchase-returns/{id}`
   - Mengambil rincian lengkap dokumen retur beserta item barang yang dikembalikan.

---

## 5. Antarmuka Pengguna (UI)

- **Navigasi Tab Terstruktur di `PurchasesPage.vue`**:
  1. *Pesanan Pembelian (PO)*
  2. *Penerimaan Barang (GRN)* (Tersedia tombol **Batalkan GRN**)
  3. *Faktur Pembelian & Hutang* (Tersedia tombol **Batalkan Faktur** & **Retur Barang**)
  4. *Retur Pembelian* (Tabel lengkap daftar retur & Tombol **+ Buat Retur Pembelian**)
  5. *Master Pemasok (Supplier)*
- **Modal Buat Retur Pembelian**:
  - Pemilihan pemasok, outlet/gudang, tanggal retur, metode kompensasi, dan alasan retur.
  - Multi-baris item barang retur dengan validasi kuantitas > 0, harga beli, kalkulasi otomatis subtotal dan total nilai retur.
  - Informasi ringkasan otomasi stok & jurnal akuntansi.
- **Modal Detail Retur Pembelian**:
  - Menampilkan header informasi retur, tabel rincian barang yang diretur, dan total nilai kompensasi.
