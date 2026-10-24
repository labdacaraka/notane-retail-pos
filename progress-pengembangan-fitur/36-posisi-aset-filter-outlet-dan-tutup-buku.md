# Progress Fitur 36: Laporan Posisi Aset (Neraca), Filter Per Outlet, dan Sistem Tutup Buku Akuntansi

## 1. Ringkasan Kebutuhan
1. **Posisi Aset (Neraca / Balance Sheet)**:
   - Menyajikan Aktiva (Aset Lancar, Kas, Bank, Piutang, Persediaan Barang) dan Pasiva (Kewajiban & Ekuitas).
   - Memastikan prinsip dasar akuntansi terpenuhi: $\text{Total Aset} = \text{Total Kewajiban} + \text{Total Ekuitas}$.
   - Laba/rugi periode berjalan yang belum ditutup buku diperhitungkan secara otomatis pada baris `3301 - Laba (Rugi) Periode Berjalan`.
2. **Filter Tiap Outlet & Konsolidasian**:
   - Seluruh laporan akuntansi (Laba Rugi, Posisi Aset/Neraca, Buku Jurnal Umum, Riwayat Tutup Buku) dapat difilter per outlet (misal: Toko Pusat, Cabang 1) atau "Semua Cabang / Konsolidasi".
   - Persediaan barang dagang (`1301`) dihitung per saldo nilai stok gudang/outlet terkait.
   - Jurnal transaksi penjualan dan transfer stok memiliki referensi `outlet_id`.
3. **Sistem Tutup Buku Periode (Period Closing)**:
   - Menutup akun nominal (Pendapatan `4xxx` dan Beban `5xxx`, `6xxx`) ke akun penampung (*3901 - Ikhtisar Laba Rugi*), lalu memindahkan laba/rugi bersih ke *3201 - Laba Ditahan (Retained Earnings)*.
   - Membuat jurnal penutup resmi bertipe `CLOSING_ENTRY`.
4. **Proteksi Periode Tertutup (Locked Period Protection & Standar Akuntansi)**:
   - Sesuai kaidah SAK/PSAK, ketika periode telah ditutup, seluruh transaksi masa lampau ($\le \text{period\_end}$) **TIDAK DAPAT** dibuat, diubah, atau dihapus (*locked*).
   - Validasi ketat diterapkan di backend engine (`pos_service`, `inventory_service`, `accounting_service`).
   - Koreksi transaksi masa lampau dilakukan via:
     - **Jurnal Penyesuaian Manual Periode Berjalan**: Untuk koreksi retrospektif sesuai PSAK 25.
     - **Buka Kembali Buku (Reopen Period)**: Khusus otorisasi manajerial/akuntan dengan justifikasi jejak audit (*audit trail*).

---

## 2. Rincian Implementasi

### A. Database Migration (`021_accounting_period_closing_and_balance_sheet.sql`)
1. Menambahkan kolom `outlet_id UUID REFERENCES outlets(id)` pada tabel `journal_entries` lengkap dengan index.
2. Menambahkan akun standar akuntansi ke bagan akun (`chart_of_accounts`):
   - `3201`: Laba Ditahan (*Retained Earnings*) - Ekuitas
   - `3901`: Ikhtisar Laba Rugi (*Income Summary*) - Ekuitas (Penampung Sementara)
3. Membuat tabel `accounting_closings`:
   - `id`, `outlet_id` (NULL jika penutupan entitas konsolidasi), `closing_number`, `period_start`, `period_end`, `closed_at`, `closed_by`, `status` (`CLOSED` / `REOPENED`), `notes`, `reopened_at`, `reopened_by`, `reopen_reason`.
   - Index unik dan index tanggal aktif untuk performa pencarian `check_transaction_date_allowed`.

### B. Backend Rust Engine
1. **`models/accounting.rs`**:
   - `BalanceSheetReport`, `BalanceSheetSection`, `BalanceSheetAccountItem`.
   - `AccountingClosing`, `CreateClosingRequest`, `ReopenClosingRequest`.
   - `ManualJournalRequest`, `ManualJournalLineRequest`.
2. **`services/accounting_service.rs`**:
   - `get_balance_sheet(pool, as_of_date, outlet_id)`: Perhitungan saldo rill + persediaan stok per outlet + laba berjalan dinamis.
   - `check_transaction_date_allowed(pool, tx_date, outlet_id)`: Validasi blokir transaksi pada periode tertutup.
   - `perform_accounting_closing(...)`: Pembuatan transaksi tutup buku dan penjurnalan debit pendapatan & kredit beban ke laba ditahan.
   - `reopen_accounting_closing(...)`: Pembatalan jurnal closing dan pencatatan jejak audit pembukaan kembali.
   - `create_manual_journal_entry(...)`: Fasilitas jurnal penyesuaian manual debit/kredit seimbang.
   - Filter `outlet_id` pada `get_income_statement` dan `list_journal_entries`.
3. **`services/inventory_service.rs` & `pos_service.rs`**:
   - Pemasangan `check_transaction_date_allowed` pada `checkout`, `void_sale`, `process_stock_transfer`, `create_purchase_order`, `process_goods_receipt`, dan `process_stock_adjustment`.
   - Penyertaan `outlet_id` saat membukukan jurnal penjualan.
4. **`api/reports.rs`**:
   - `GET /api/v1/reports/balance-sheet`
   - `GET /api/v1/accounting/closings`
   - `POST /api/v1/accounting/closings`
   - `POST /api/v1/accounting/closings/{id}/reopen`
   - `POST /api/v1/accounting/journal-entries`

### C. Frontend Back-Office (`src/pages/AccountingPage.vue`)
- **Filter Bar Lengkap**: Dropdown Outlet, Preset Rentang Tanggal ("Hari Ini", "Bulan Ini", "Bulan Lalu", "Tahun Ini"), Custom Date Range, dan As-Of Date picker.
- **Tab Posisi Aset & Neraca (Balance Sheet)**:
  - Tampilan dua kolom rapi: **Aktiva (Aset Lancar & Nilai Persediaan)** vs **Pasiva (Kewajiban & Ekuitas + Laba Berjalan)**.
  - Badge Status Keseimbangan: `Balanced ✓` dengan indikator selisih matematis.
- **Tab Tutup Buku Periode**:
  - Banner panduan kaidah PSAK tentang tata cara penutupan buku & penguncian periode.
  - Tabel riwayat penutupan dengan status aktif (`CLOSED` / `REOPENED`), tanggal cut-off, dan pembuat.
  - Tombol aksi `+ Tutup Buku Baru` dan tombol `Buka Kembali (Reopen)` berjejak audit.
- **Modal Jurnal Penyesuaian Manual**:
  - Formulir multi-baris debit & kredit dinamis dengan verifikasi `Total Debit == Total Kredit`.
- **Tab Buku Jurnal Umum & Laba Rugi**:
  - Terintegrasi penuh dengan filter outlet dan badge tipe jurnal (`CLOSING_ENTRY`, `MANUAL_ADJUSTMENT`, `SALE`, dll).

---

## 3. Hasil Pengujian
1. **Pengujian Keseimbangan Neraca**:
   - `GET /api/v1/reports/balance-sheet?as_of_date=2026-09-26` mengembalikan `is_balanced: true` dengan total aset sama persis dengan total kewajiban + ekuitas.
2. **Pengujian Tutup Buku**:
   - Eksekusi tutup buku periode Agustus 2026 sukses mencatatkan jurnal penutup penyeimbang akun pendapatan dan beban ke akun `3201`.
3. **Pengujian Proteksi Periode Tertutup**:
   - Transaksi transfer stok atau penerimaan barang dengan tanggal dalam periode tertutup (misal `15-08-2026`) berhasil **DITOLAK** dengan pesan error akuntansi standar.
4. **Pengujian Buka Kembali (Reopen)**:
   - Endpoint `POST /api/v1/accounting/closings/{id}/reopen` berhasil membuka kembali periode dan membatalkan jurnal penutup disertai jejak alasan audit.
5. **Pengujian Frontend Build**:
   - `npm run build` selesai tanpa error (type check `vue-tsc` dan bundle vite sukses).
