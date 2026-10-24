# Progress 48: Fitur Pencatatan Setoran Modal Usaha (Capital Deposits) & Otomasi Jurnal Ekuitas

## Ringkasan Fitur
Menambahkan sistem pencatatan dan pengelolaan **Setoran Modal Usaha (Capital Deposits)** yang terintegrasi langsung dengan pembukuan akuntansi berbasis *double-entry* dan Bagan Akun (COA). Fitur ini memungkinkan pemilik usaha (owner / investor) untuk mencatat penambahan modal kas/bank ke perusahaan atau cabang tertentu secara transparan, akuntabel, dan rapi.

---

## 1. Database & Migrasi (`034_capital_deposits.sql`)
1. **Tabel `capital_deposits`**:
   - `id`: UUID Primary Key
   - `deposit_no`: Nomor bukti transaksi unik (`CAP-YYYYMMDD-XXXX`)
   - `deposit_date`: Tanggal transaksi setoran
   - `outlet_id`: Opsional (kantor pusat atau gerai cabang penerima)
   - `investor_name`: Nama pemilik/investor penyetor modal
   - `amount`: Nominal modal disetor (`NUMERIC(18, 4)`)
   - `payment_account_id`: Rekening Kas/Bank penerima (Debit)
   - `equity_account_id`: Akun Ekuitas/Modal (Kredit)
   - `journal_entry_id`: Relasi ke jurnal umum yang terbit otomatis
   - `reference_no`: Bukti transfer/slip bank
   - `notes`: Catatan peruntukan modal
   - `status`: `COMPLETED` atau `CANCELLED`
   - `cancellation_reason`: Alasan pembatalan
   - `cancelled_by` & `cancelled_at`: Riwayat pembatalan
   - `reversal_journal_entry_id`: Relasi ke jurnal pembalik saat dibatalkan
2. **Akun Standar COA & Permission Seeding**:
   - Akun Ekuitas: `3101` (Modal Pemilik Disetor)
   - Registrasi granular permissions: `capital:read`, `capital:create`, `capital:delete`
   - Mapping default ke role `super_admin`, `admin`, dan `cashier` (read-only)

---

## 2. Backend Rust Engine
1. **Model Domain (`src-tauri/src/models/capital.rs`)**:
   - `CapitalDeposit`, `CapitalDepositDetail`, `CreateCapitalDepositRequest`, `CancelCapitalDepositRequest`, `CapitalSummaryResponse`.
2. **Layanan Akuntansi (`src-tauri/src/services/capital_service.rs`)**:
   - `create_capital_deposit`: Membuat transaksi, menerbitkan nomor `CAP-...`, membukukan jurnal umum otomatis:
     - `Debit`: Akun Kas / Bank Penerima (`payment_account_id`)
     - `Kredit`: Akun Ekuitas / Modal Disetor (`equity_account_id`)
   - `cancel_capital_deposit`: Membatalkan setoran modal dan menerbitkan Jurnal Pembalik (*reversing entry*):
     - `Debit`: Akun Ekuitas / Modal Disetor
     - `Kredit`: Akun Kas / Bank Penerima
   - `get_capital_deposits`: Filter paginasi, pencarian keyword, status, dan outlet.
   - `get_capital_summary`: Ringkasan metrik total modal masuk, bulan ini, tahun ini, dan frekuensi setoran.
3. **API Endpoints (`src-tauri/src/api/capital.rs` & `src-tauri/src/api/mod.rs`)**:
   - `GET /api/v1/capital/deposits`
   - `POST /api/v1/capital/deposits`
   - `POST /api/v1/capital/deposits/{id}/cancel`
   - `GET /api/v1/capital/summary`
   - Terintegrasi dengan Axum router dan Auth Guard.

---

## 3. Frontend Vue 3 + TypeScript
1. **Definisi Types (`src/types/index.ts`)**:
   - `CapitalDepositDetail`, `CreateCapitalDepositPayload`, `CancelCapitalDepositPayload`, `CapitalSummaryResponse`.
2. **Service API (`src/services/api.ts`)**:
   - `api.getCapitalDeposits(params)`
   - `api.getCapitalSummary(outletId)`
   - `api.createCapitalDeposit(payload)`
   - `api.cancelCapitalDeposit(id, payload)`
3. **Tampilan Antarmuka Halaman Akuntansi (`src/pages/AccountingPage.vue`)**:
   - Tab baru: **Setoran Modal (Capital)** dengan ikon koin.
   - **4 KPI Card**: Total Modal Masuk, Setoran Bulan Ini, Setoran Tahun Ini, Frekuensi Transaksi.
   - **Filter & Pencarian**: Pencarian nama investor, nomor bukti, referensi transfer, dan filter status (Semua / Selesai / Dibatalkan).
   - **Tabel Transaksi**: Menampilkan tanggal, pemilik, outlet, rekening kas/bank, akun ekuitas, nominal, status, dan aksi.
   - **Modal Formulir Setoran Modal**: Pilihan akun kas/bank, akun ekuitas, nominal, tanggal, outlet, serta preview simulasi jurnal double-entry otomatis.
   - **Modal Pembatalan Transaksi**: Meminta alasan pembatalan dan memberikan peringatan pembuatan jurnal pembalik.
   - **Modal Bukti Kas Masuk (Print Voucher)**: Kuitansi resmi tanda terima setoran modal usaha siap cetak dengan format thermal/kertas standar dan kolom tanda tangan penyetor & bagian keuangan.

---

## 4. Validasi & Pengujian
- `cargo check`: Berhasil tanpa error (`exit code 0`).
- `npx vue-tsc --noEmit`: Berhasil tanpa error (`exit code 0`).
