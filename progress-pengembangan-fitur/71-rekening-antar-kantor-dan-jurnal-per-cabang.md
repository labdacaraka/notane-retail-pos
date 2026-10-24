# 71. Rekening Antar-Kantor (RAK) & Pelaporan Jurnal Akuntansi Per-Cabang

## Latar Belakang & Kebutuhan
Untuk mendukung operasional multi-cabang/outlet yang akuntabel dan mematuhi prinsip akuntansi berterima umum (PSAK / IFRS):
1. **Laporan Per-Cabang yang Berimbang (*Balanced Double-Entry*)**:
   Setiap transaksi jurnal di menu Akuntansi (`http://localhost:5175/admin/accounting`) harus dapat difilter per cabang/outlet, di mana laporan Laba Rugi, Neraca, dan Buku Jurnal per-outlet wajib seimbang (Debet = Kredit).
2. **Rekening Antar-Kantor (RAK / Inter-Branch Clearing)**:
   Transaksi yang melibatkan lebih dari satu cabang (seperti transfer stok antar outlet, transfer dana/kas operasional antar cabang) membutuhkan rekening penyeimbang antar-kantor agar:
   - Cabang Pengirim mencatat pengeluaran aset dan mendebit RAK (piutang antar-kantor).
   - Cabang Penerima mencatat penerimaan aset dan mengkredit RAK (hutang antar-kantor).
   - Pada tingkat konsolidasi (seluruh cabang), saldo rekening RAK saling meniadakan (*zero out* / net nol).

---

## Solusi & Arsitektur yang Diterapkan

### 1. Database Layer (`migrations/052_rekening_antar_kantor_dan_jurnal_cabang.sql`)
- **Master Header RAK**: Mendaftarkan akun header COA `1901` (*Rekening Antar-Kantor (RAK / Inter-Branch Clearing)*, Kategori ASSET, `is_header = TRUE`, `is_system = TRUE`).
- **Sub-Akun RAK Otomatis per Outlet**: Membuat sub-akun RAK `1901-<KODE_OUTLET>` untuk setiap outlet yang ada dan menghubungkannya via kolom `outlets.rak_account_id`.
- **RBAC Permission**: Mendaftarkan permission `accounting:fund_transfer` dan menetapkannya ke role `SUPERADMIN` dan `STORE_MANAGER`.
- **Backfill Historis `journal_entries`**: Mengaitkan 100% data historis header jurnal yang belum memiliki `outlet_id` dari tabel relasinya (`sales`, `goods_receipts`, `purchase_invoices`, `cashier_shifts`, `stock_adjustments`, `purchase_returns`, `stock_transfers`).

### 2. Backend Inventory Service & POS Service
- **Transfer Stok Antar Outlet (`create_stock_transfer`, `receive_stock_transfer`, `cancel_stock_transfer`)**:
  - Kini membuat **dua jurnal voucher berpasangan**:
    - **Cabang Pengirim (`from_outlet_id`)**: Debit `RAK Pengirim`, Kredit `Persediaan Pengirim`.
    - **Cabang Penerima (`to_outlet_id`)**: Debit `Persediaan Penerima` (atau In-Transit), Kredit `RAK Penerima`.
  - Pembatalan transfer stok membalik kedua jurnal menggunakan RAK masing-masing cabang.
- **Konsistensi `outlet_id` pada Semua Transaksi Jurnal**:
  - Mengikat `outlet_id` pada jurnal penerimaan barang (GRN), faktur pembelian, pembayaran hutang, penyesuaian stok opname, retur pembelian, selisih kas kasir (*shift discrepancy*), dan pembatalan transaksi POS (*void*).

### 3. Backend Accounting Service & API
- **Endpoint Transfer Dana Antar Cabang (`POST /api/v1/accounting/fund-transfers`)**:
  - Divalidasi dengan permission `accounting:fund_transfer` atau `accounting:write`.
  - Memeriksa proteksi penutupan buku (periode terkunci) untuk kedua cabang.
  - Membukukan 2 jurnal berimbang secara atomik dalam satu transaksi database:
    - Voucher Pengirim: Debit RAK Pengirim, Kredit Kas/Bank Pengirim.
    - Voucher Penerima: Debit Kas/Bank Penerima, Kredit RAK Penerima.
- **Penyempurnaan Filter Laporan**:
  - Kueri `list_journal_entries`, `get_income_statement`, dan `get_balance_sheet` mendukung filter per cabang dengan klausa: `(je.outlet_id = '{oid}' OR (je.outlet_id IS NULL AND coa.outlet_id = '{oid}'))`.

### 4. Frontend Client (`AccountingPage.vue`)
- **Tombol Aksi Global**: Menambahkan tombol `+ Transfer Kas Antar Cabang` di header akuntansi dengan proteksi permission `accounting:fund_transfer`.
- **Modal Transfer Dana Antar Cabang**:
  - Pilihan Cabang Pengirim & Cabang Penerima.
  - Pilihan Akun Kas/Bank per cabang (difilter sesuai outlet terkait).
  - Input Nominal dan Catatan/Referensi.
  - Informasi interaktif mengenai mekanisme RAK standar akuntansi.
- **Filter Cabang**: Filter dropdown Outlet/Cabang langsung menyaring Laba Rugi, Posisi Aset & Neraca, dan Buku Jurnal Umum secara reaktif.

### 5. Penyelesaian Masalah Dropdown Cabang Duplikat & Upgrade SearchableSelect
- **Akar Masalah Duplikasi Cabang**:
  Pada kueri backend `master.rs::list_outlets_handler` dan `get_outlet_handler`, klausul join COA persediaan sebelumnya menggunakan:
  `LEFT JOIN chart_of_accounts coa ON (o.inventory_account_id = coa.id OR coa.outlet_id = o.id)`
  Ketika akun RAK (`1901-<CODE>`) dibuat dengan `coa.outlet_id = o.id`, kondisi `OR coa.outlet_id = o.id` mencocokkan **dua baris COA sekaligus** (akun persediaan `1201` dan akun RAK `1901`), sehingga database mengembalikan setiap cabang sebanyak 2 kali (duplikat).
- **Perbaikan Backend (`master.rs`, `pos_service.rs`, `inventory_service.rs`)**:
  - Mengubah klausul join menjadi spesifik dan deterministik:
    `LEFT JOIN chart_of_accounts coa ON o.inventory_account_id = coa.id`
  - Mengeliminasi duplikasi secara permanen pada level SQL query backend.
- **Upgrade Komponen `SearchableSelect` di Seluruh Modul Akuntansi**:
  - Seluruh elemen dropdown select pada halaman Akuntansi telah diganti dengan komponen `SearchableSelect`:
    1. **Filter Utama Outlet / Cabang**: Memanggil API `/outlets` server-side, searchable by name/code, dengan badge `Toko` / `Gudang`.
    2. **Modal Transfer Kas Antar Cabang**: Cabang Asal, Cabang Tujuan, Kas/Bank Asal, dan Kas/Bank Tujuan semuanya searchable secara instan.
    3. **Modal Jurnal Penyesuaian Manual**: Pemilihan cabang dan akun COA per baris transaksi (`postableCOA`) searchable by code & name.
    4. **Modal Tutup Buku Periode & Setoran Modal Usaha**: Pemilihan cabang dan rekening kas/ekuitas kini searchable.
  - Komponen `SearchableSelect.vue` diperbarui untuk mendukung filter instan real-time baik pada data server-side maupun client-side options.

---

## Hasil Verifikasi & Pengujian
1. **Verifikasi Dropdown Outlet**:
   - `GET /api/v1/outlets` kini mengembalikan tepat 4 cabang tanpa duplikasi:
     - `OUT-001`: Notane Retail Flagship Store (`Toko`)
     - `OUT-002`: Notane Cabang Kelapa Gading (`Toko`)
     - `WH-001`: Gudang Utama Toko (`Gudang`)
     - `WH-002`: Gudang Distribusi Cikarang (`Gudang`)
2. **Pengujian Transfer Dana Antar Cabang**:
   - Transfer dana kas Rp 500.000 dari *Notane Retail Flagship Store* (`OUT-001`) ke *Notane Cabang Kelapa Gading* (`OUT-002`) berhasil dibukukan dengan nomor jurnal:
     - `JRN-FND-OUT-20261013064437-235E`: Debit `1901-OUT-001` (Rp 500.000), Kredit `1101` (Rp 500.000) [Status: **Balanced ✓**]
     - `JRN-FND-IN-20261013064437-B0F2`: Debit `1101` (Rp 500.000), Kredit `1901-OUT-002` (Rp 500.000) [Status: **Balanced ✓**]
3. **Pengecekan Buku Jurnal & Laporan per Cabang**:
   - Filter `OUT-001`: Hanya memuat transaksi dan pendapatan milik *Notane Retail Flagship Store*.
   - Filter `OUT-002`: Memuat penjualan, penerimaan kas, dan voucher RAK masuk milik *Notane Cabang Kelapa Gading* (Laba kotor: Rp 49.900).
   - Filter Konsolidasi (Semua Cabang): Menampilkan seluruh transaksi dengan saldo rekening RAK tereliminasi seimbang (0).
4. **Verifikasi Browser UI**:
   - Seluruh dropdown di browser menampilkan input pencarian ("Cari cabang...", "Cari kode atau nama kas/bank..."), navigasi keyboard, tombol clear selection (×), dan pemanggilan API server-side yang responsif.
