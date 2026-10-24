# Progress Pengembangan Fitur #39: Alur Transfer Stok Antar-Outlet / Gudang Mirip Pembelian (In-Transit & Penerimaan Fisik)

**Tanggal:** 30 September 2026  
**Status:** Selesai & Terverifikasi  
**Terkait:** Alur Distribusi Logistik Antar Cabang, Double-Entry Bookkeeping Persediaan Dalam Perjalanan (`1202`), Verifikasi Fisik Kedatangan Barang, Pembatalan Transfer.

---

## 1. Latar Belakang & Kebutuhan

Sebelum pembaruan ini, transfer stok antar-outlet/gudang langsung memindahkan stok seketika (`COMPLETED`) tanpa memperhitungkan jeda waktu pengiriman fisik armada di jalan (*in-transit*). Kebutuhan operasional retail mensyaratkan:
1. **Pemisahan Tahap Pengiriman (*Outbound Dispatch*) dan Penerimaan Fisik (*Inbound Receipt*)**:
   - Mirip dengan siklus pembelian barang (PO -> Penerimaan Fisik GRN -> Pembayaran), transfer antar outlet/gudang memerlukan proses pengiriman oleh outlet asal, pelacakan status armada di perjalanan (*In-Transit*), dan verifikasi jumlah fisik yang diterima oleh staf outlet tujuan.
2. **Akuntansi Persediaan Dalam Perjalanan (*Goods in Transit Account*)**:
   - Saat barang meninggalkan outlet pengirim, nilai persediaan outlet pengirim berkurang, namun belum boleh masuk ke persediaan outlet tujuan sampai barang diverifikasi. Selama di jalan, nilai barang dicatat pada akun aset **`1202 - Persediaan Dalam Perjalanan (In-Transit)`**.
   - Ketika barang tiba dan diverifikasi di outlet tujuan, akun `1202` dikreditkan dan akun persediaan outlet tujuan (`1201-{OUTLET}`) didebitkan.
   - Apabila terdapat selisih fisik rusak/hilang di jalan, selisih langsung dibukukan ke akun Beban Kerusakan/Penyusutan Persediaan (`5301`).
3. **Mekanisme Pembatalan Transfer (*Cancellation*)**:
   - Jika armada pengiriman batal berangkat, transfer berstatus `IN_TRANSIT` dapat dibatalkan oleh pihak pengirim. Stok barang seketika dikembalikan ke outlet asal dengan jurnal pembalik (*reversing journal*).
4. **Opsi Transfer Langsung (*Immediate Transfer*)**:
   - Tetap dipertahankan untuk kebutuhan perpindahan stok cepat intra-lokasi atau transfer darurat yang tidak memerlukan armada perjalanan.

---

## 2. Perubahan Database & Migrasi

### File Migrasi: `025_stock_transfer_workflow_and_in_transit.sql`
1. **Akun CoA Baru**:
   - Menambahkan akun `1202 - Persediaan Dalam Perjalanan (In-Transit)` bertipe Aset Lancar (`account_category_id = 1`).
2. **Kolom Baru pada Tabel `stock_transfers`**:
   - `received_date` (DATE): Tanggal barang fisik diterima di outlet tujuan.
   - `received_by` (UUID): ID pengguna penerima barang.
   - `received_at` (TIMESTAMPTZ): Waktu penerimaan fisik dikonfirmasi.
   - `shipped_at` (TIMESTAMPTZ): Waktu barang dikirimkan dari outlet asal.
   - `cancellation_reason` (TEXT): Alasan pembatalan jika transfer dibatalkan saat in-transit.
   - `cancelled_by` (UUID): ID pengguna yang membatalkan.
   - `cancelled_at` (TIMESTAMPTZ): Waktu pembatalan.
3. **Kolom Baru pada Tabel `stock_transfer_items`**:
   - `received_quantity` (NUMERIC(15,4)): Jumlah fisik aktual yang diterima di outlet tujuan.
   - `notes` (TEXT): Catatan per item (misal kondisi kemasan, kerusakan saat diterima).

---

## 3. Backend Rust Implementation

### Model & Request DTO (`src-tauri/src/models/inventory.rs`)
- `CreateStockTransferRequest`: Ditambahkan field `is_immediate: Option<bool>`.
- `ReceiveStockTransferRequest`: DTO penerimaan barang transfer (`receive_date`, `notes`, `items: Vec<ReceiveStockTransferItemInput>`).
- `CancelStockTransferRequest`: DTO pembatalan transfer (`reason`).
- `StockTransferFilter`: Mendukung filter status (`IN_TRANSIT`, `COMPLETED`, `CANCELLED`) serta filter arah transfer (`to_outlet_id`).

### Endpoint REST API (`src-tauri/src/api/inventory.rs`)
- `POST /api/v1/inventory/transfers`: Membuat pengiriman transfer stok (2 tahap secara default atau instan jika `is_immediate: true`).
- `POST /api/v1/inventory/transfers/{id}/receive`: Menerima barang fisik transfer di outlet tujuan.
- `POST /api/v1/inventory/transfers/{id}/cancel`: Membatalkan pengiriman barang transfer dan mengembalikan stok asal.
- `GET /api/v1/inventory/transfers`: Daftar transfer dengan status, kuantitas dikirim vs diterima, nama penerima/pembatal.
- `GET /api/v1/inventory/transfers/{id}`: Detail transfer lengkap beserta seluruh jurnal pembukuan (*multi-journal entries*) sepanjang siklus transfer.

### Logika Bisnis & Akuntansi (`src-tauri/src/services/inventory_service.rs`)
1. **Pengiriman Transfer (Tahap 1 - Outbound)**:
   - Mengurangi stok fisik di outlet asal (`TRANSFER_OUT`).
   - Status diset ke `IN_TRANSIT`.
   - **Jurnal Pembukuan Pengiriman**:
     - **Debit**: `1202 - Persediaan Dalam Perjalanan (In-Transit)`
     - **Kredit**: `1201-{OUT_ASAL} - Persediaan Outlet Asal`
2. **Penerimaan Transfer (Tahap 2 - Inbound Verification)**:
   - Validasi status harus `IN_TRANSIT`.
   - Memasukkan stok fisik ke outlet tujuan sejumlah `received_quantity` (`TRANSFER_IN`).
   - Status diubah menjadi `COMPLETED`.
   - **Jurnal Pembukuan Penerimaan**:
     - **Debit**: `1201-{OUT_TUJUAN} - Persediaan Outlet Tujuan` (sebesar nilai barang yang diterima)
     - **Debit**: `5301 - Beban Kerusakan & Penyusutan Persediaan` (jika ada barang rusak/hilang)
     - **Kredit**: `1202 - Persediaan Dalam Perjalanan (In-Transit)` (sebesar nilai barang in-transit yang diselesaikan)
3. **Pembatalan Transfer (Cancellation)**:
   - Validasi status harus `IN_TRANSIT`.
   - Mengembalikan stok fisik ke outlet asal (`TRANSFER_CANCEL`).
   - Status diubah menjadi `CANCELLED`.
   - **Jurnal Pembukuan Pembalik**:
     - **Debit**: `1201-{OUT_ASAL} - Persediaan Outlet Asal`
     - **Kredit**: `1202 - Persediaan Dalam Perjalanan (In-Transit)`

---

## 4. Frontend Vue UI Implementation (`src/pages/InventoryPage.vue`)

1. **Filter Tab & Counter**:
   - Filter pill status: **Semua**, **Dalam Perjalanan (In-Transit)**, **Menuju Outlet Ini (Inbound)**, **Selesai**, **Dibatalkan**.
   - Badge counter otomatis untuk transfer aktif yang sedang menuju outlet aktif pengguna.
2. **Tabel Transfer Interaktif**:
   - Kolom status dengan badge warna & icon:
     - Amber / Truck: `DALAM PERJALANAN`
     - Emerald / Check: `SELESAI`
     - Rose / Ban: `DIBATALKAN`
   - Kolom kuantitas menampilkan Qty Kirim vs Qty Terima.
   - Tombol Aksi Kontekstual:
     - Tombol **Terima Barang** (hijau) muncul jika transfer berstatus `IN_TRANSIT` dan menuju outlet pengguna.
     - Tombol **Batalkan** (merah) untuk transfer yang masih di jalan.
     - Tombol **Rincian & Surat Jalan** (biru/abu) untuk melihat riwayat audit dan mencetak surat jalan.
3. **Modal Form Pengiriman Baru**:
   - Pilihan metode transfer:
     - *Standar 2-Tahap (Disarankan)*: Kirim -> Armada Jalan -> Terima & Verifikasi Fisik di Tujuan.
     - *Transfer Langsung (Instan)*: Barang langsung diterima tanpa jeda ekspedisi.
4. **Modal Konfirmasi & Verifikasi Penerimaan Barang**:
   - Input tanggal penerimaan fisik.
   - Tabel item dengan kolom jumlah dikirim, input jumlah diterima aktual, tombol "Terima Penuh Semua", serta indikasi jika terdapat selisih.
5. **Modal Pembatalan Transfer**:
   - Input alasan pembatalan dengan peringatan otomatis bahwa stok akan dikembalikan ke gudang/outlet asal.
6. **Modal Detail & Cetak Surat Jalan**:
   - *Workflow Stepper Timeline*: Menampilkan progress visual dari Pengiriman $\rightarrow$ Dalam Perjalanan $\rightarrow$ Diterima/Dibatalkan beserta waktu dan operator penanggung jawab.
   - Riwayat jurnal pembukuan akuntansi ganda (*multi-journal timeline*).
   - Fitur Cetak Surat Jalan Pengiriman Resmi via `window.print()`.

---

## 5. Hasil Verifikasi Sistem

- [x] **Backend Rust Compilation**: `cargo build` sukses tanpa error.
- [x] **Frontend Vue / TypeScript Build**: `npm run build` (`vue-tsc && vite build`) lolos 100% tanpa error.
- [x] **End-to-End API Test - Siklus 2-Tahap**:
  - Pembuatan transfer menghasilkan status `IN_TRANSIT`, stok asal berkurang, jurnal `Dr 1202 / Cr 1201` terbentuk.
  - Penerimaan transfer di outlet tujuan berhasil mengubah status ke `COMPLETED`, stok tujuan bertambah, jurnal `Dr 1201 / Cr 1202` terbentuk.
- [x] **End-to-End API Test - Siklus Pembatalan**:
  - Pembuatan transfer `IN_TRANSIT` berhasil dibatalkan via endpoint `/cancel`.
  - Status berubah ke `CANCELLED`, alasan tercatat, stok asal kembali, jurnal pembalik `Dr 1201 / Cr 1202` terbentuk secara balance.
- [x] **Resolusi Gudang Asal & Default Warehouse**:
  - Memperbaiki validasi pemotongan stok pada outlet yang menggunakan `default_warehouse_id` (seperti *Notane Retail Flagship Store* yang persediaannya berada di *Gudang Utama Toko*).
  - Pengecekan stok otomatis memeriksa stok outlet langsung maupun gudang bawaannya secara sinkron dengan katalog `/products`, sehingga kuantitas di UI dan validasi backend selaras 100%.
  - Dropdown outlet asal/tujuan kini memberikan penanda visual jelas antara `[Outlet]` dan `[Gudang]`.
