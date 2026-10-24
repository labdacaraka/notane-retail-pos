# Progress 35: Integrasi Chart of Accounts (CoA) Persediaan Per Outlet, Lifecycle Otomatis Tambah/Hapus Outlet, dan Modul Transfer Stok Antar Outlet

## Latar Belakang & Kebutuhan
Sebelumnya, seluruh pergerakan persediaan ritel (pembelian GRN, penyesuaian opname, dan penjualan kasir POS) dicatat ke satu akun persediaan umum yaitu `1201 - Persediaan Barang Dagang Retail`. Hal ini menyebabkan keterbatasan:
1. Tidak dapat melihat nilai neraca persediaan barang yang terisolasi dan spesifik di masing-masing gerai/outlet/gudang.
2. Ketika membuat outlet baru, akun akuntansi tidak otomatis terbentuk, dan sebaliknya saat menghapus outlet tidak ada sinkronisasi akun.
3. Belum tersedianya sistem mutasi transfer stok antar outlet/gudang yang mencatat perpindahan stok fisik sekaligus pembukuan jurnal akuntansi berpasangan (*double-entry bookkeeping*) secara otomatis.

---

## Solusi & Perubahan yang Dilakukan

### 1. Database Schema & Migrasi Terintegrasi (`020_outlet_coa_and_stock_transfer.sql`)
- **Relasi Dua Arah antara `outlets` dan `chart_of_accounts`**:
  - Kolom `chart_of_accounts.outlet_id UUID REFERENCES outlets(id) ON DELETE CASCADE`.
  - Kolom `outlets.inventory_account_id UUID REFERENCES chart_of_accounts(id) ON DELETE SET NULL`.
- **Hierarki Akun Persediaan**:
  - Akun `1201` diset sebagai akun induk/header (`is_header = TRUE`).
  - Dijalankan migrasi otomatis (*backfill*) untuk seluruh outlet yang ada dengan format kode `1201-{KODE_OUTLET}` (contoh: `1201-OUT-001`, `1201-OUT-002`, `1201-WH-001`, `1201-WH-002`) dan nama `Persediaan - {NAMA_OUTLET}` dengan kategori Aset Lancar.
- **Tabel Dokumen Mutasi Transfer Stok**:
  - `stock_transfers`: `id`, `transfer_number`, `from_outlet_id`, `to_outlet_id`, `transfer_date`, `status`, `notes`, `total_amount`, `created_by`, `created_at`.
  - `stock_transfer_items`: `id`, `transfer_id`, `product_id`, `quantity`, `cost_price`, `subtotal`.

### 2. Backend Rust: Lifecycle Otomatis Tambah & Hapus Outlet (`api/master.rs`)
- **Pembuatan Outlet Baru (`create_outlet_handler`)**:
  - Setiap penambahan outlet baru otomatis membuat sub-akun CoA persediaan di bawah `1201` dengan kode `1201-{KODE_OUTLET}` dan nama `Persediaan - {NAMA_OUTLET}`.
  - Akun tersebut langsung dikaitkan ke `inventory_account_id` outlet terkait dalam satu transaksi database atomik.
- **Pembaruan Outlet (`update_outlet_handler`)**:
  - Mengubah nama atau status aktif outlet otomatis menyinkronkan nama dan status aktif akun CoA-nya.
- **Penghapusan Outlet Bersih & Aman (`delete_outlet_handler`)**:
  - **Proteksi Integritas**: Dilarang menghapus jika outlet sudah memiliki transaksi penjualan, sisa stok, riwayat transfer stok, atau riwayat jurnal akuntansi (disarankan dinonaktifkan).
  - Jika outlet bersih (belum ada mutasi), penghapusan outlet akan secara otomatis menghapus sub-akun CoA persediaannya tanpa meninggalkan *orphan record*.

### 3. Backend Rust: Alur Pembelian & Penjualan Menggunakan CoA Outlet Spesifik
- **Penerimaan Pembelian GRN (`inventory_service.rs`)**:
  - Mendebit akun CoA persediaan milik outlet penerima barang:
    - **DEBIT**: `Persediaan - [Outlet Penerima]` (misal `1201-OUT-001`)
    - **KREDIT**: `2101 - Hutang Usaha Supplier`
- **Penjualan Kasir POS (`pos_service.rs`)**:
  - Mengkredit akun persediaan milik outlet tempat kasir bertransaksi:
    - **DEBIT**: `5101 - Beban Pokok Penjualan (HPP)`
    - **KREDIT**: `Persediaan - [Outlet Transaksi Kasir]`

### 4. Backend Rust: Modul Transfer Stok Antar Outlet (`inventory_service.rs` & `api/inventory.rs`)
- **Endpoint**:
  - `GET /api/v1/inventory/transfers` (pencarian, filter outlet asal/tujuan, tanggal, pagination).
  - `POST /api/v1/inventory/transfers` (pemrosesan transfer stok dan penjurnalan otomatis).
  - `GET /api/v1/inventory/transfers/:id` (detail mutasi dan jurnal terkait).
- **Logika Eksekusi Mutasi Atomik**:
  1. Validasi kecukupan stok fisik di outlet asal menggunakan `FOR UPDATE`.
  2. Pengurangan stok di outlet asal dan penambahan stok di outlet tujuan pada tabel `warehouse_stocks`.
  3. Pencatatan mutasi kartu stok `stock_mutations` (tipe `TRANSFER`) di kedua lokasi.
  4. Penyimpanan header `stock_transfers` dan rincian `stock_transfer_items`.
  5. **Pembukuan Otomatis Jurnal Double-Entry**:
     - **DEBIT**: Akun CoA Persediaan Outlet Tujuan (senilai Total Nilai Transfer HPP).
     - **KREDIT**: Akun CoA Persediaan Outlet Asal (senilai Total Nilai Transfer HPP).
     - Selalu seimbang (*balanced*) dan tercatat dengan nomor referensi jurnal `JRN-TRF-...`.

### 5. Frontend Vue: Integrasi UI Lengkap
- **`src/pages/OutletsPage.vue`**:
  - Menampilkan badge kode akun persediaan CoA (`CoA: 1201-{CODE}`) di kartu dan baris outlet.
- **`src/pages/AccountingPage.vue`**:
  - Menambahkan kolom *"Terkait Outlet / Lokasi"* pada tabel Bagan Akun (COA).
- **`src/pages/InventoryPage.vue`**:
  - Tab baru: **"Transfer Stok Antar Outlet"**.
  - Tabel riwayat mutasi dengan pencarian, filter outlet asal & tujuan, status, dan tombol detail.
  - Modal **"Buat Transfer Stok Baru"**:
    - Memilih Outlet Asal dan Outlet Tujuan (divalidasi tidak boleh sama).
    - Memilih produk, kuantitas, catatan, dan auto-validasi kuantitas tidak melebihi stok yang tersedia di outlet asal.
    - Menampilkan kalkulasi total nilai transfer berbasis HPP secara real-time.
  - Modal **"Rincian Dokumen Mutasi Transfer Stok"**:
    - Alur transfer (Outlet Pengirim $\rightarrow$ Outlet Penerima), nomor dokumen, pembuat, tanggal.
    - Tabel rincian barang yang ditransfer.
    - Card Jurnal Akuntansi Otomatis yang menampilkan baris Debit dan Kredit seimbang.

---

## Hasil Pengujian & Verifikasi Otomatis
Seluruh 6 skenario pengujian diverifikasi melalui test suite terintegrasi (`scratch/test_outlet_coa_and_transfer.py`):
1. **Verifikasi CoA Eksisting**: Seluruh outlet (Flagship Store, Kelapa Gading, Gudang Utama, Gudang Cikarang) memiliki sub-CoA persediaan masing-masing.
2. **Koneksi CoA di Laporan Akuntansi**: Seluruh sub-akun terhubung dengan nama outlet masing-masing.
3. **Pembuatan Outlet Baru**: Outlet baru otomatis menghasilkan akun CoA `1201-{KODE}`.
4. **Penghapusan Outlet Baru**: Menghapus outlet yang belum bertransaksi otomatis menghapus akun CoA-nya secara bersih.
5. **Transfer Stok & Jurnal**:
   - Transfer 5 unit Apel Fuji dari `OUT-001` ke `OUT-002`.
   - Stok `OUT-001` berkurang dari 65.5 menjadi 60.5 (-5.0).
   - Stok `OUT-002` bertambah dari 35.5 menjadi 40.5 (+5.0).
   - Jurnal `JRN-TRF-20260926125617-B048` membukukan:
     - Debit `1201-OUT-002` (Kelapa Gading): Rp 68.421,0525
     - Kredit `1201-OUT-001` (Flagship Store): Rp 68.421,0525
     - Status: Seimbang (*Balanced*).
6. **Proteksi Integritas Data**: Outlet dengan riwayat mutasi/transaksi berhasil diblokir dari penghapusan sembarangan dengan status HTTP 400.
