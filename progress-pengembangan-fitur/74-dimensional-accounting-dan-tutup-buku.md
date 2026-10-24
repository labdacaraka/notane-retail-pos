# Progress 74: Refactoring Dimensional Accounting & Sistem Tutup Buku (Soft & Hard Close)

## 1. Ringkasan Implementasi
Refactoring arsitektur pembukuan dari model redundansi Chart of Accounts (CoA per-cabang) ke pendekatan modern **Dimensional Accounting**, serta penerapan sistem tutup buku bertingkat (**Soft Close** berbasis penguncian periode transaksi dan **Hard Close** tutup buku akhir tahun fiskal).

---

## 2. Fase 1: Restrukturisasi Skema Database & Migrasi (Migration 054)
- **Tabel `chart_of_accounts` (Master CoA Universal):**
  - Kolom `outlet_id` telah dihapus.
  - Seluruh sub-akun per-cabang redundan (misal `1201-OUT-001`, `1901-OUT-001`) direkonsiliasi dan direpetakan ke akun master terpusat `1201 (Persediaan Barang Dagang Retail)` dan `1901 (Rekening Antar-Kantor / RAK)`.
  - Master CoA kini bersifat universal tunggal untuk seluruh cabang.
- **Tabel `journal_entries` (Header Jurnal):**
  - Kolom `outlet_id` dihapus dari header.
  - Kolom `is_closing_entry: BOOLEAN NOT NULL DEFAULT FALSE` ditambahkan untuk menandai jurnal penutupan buku.
  - Sebelum drop kolom, histori `outlet_id` dari header disalin ke baris detail `journal_lines`.
- **Tabel `journal_lines` (Detail Baris Jurnal - Dimensi Cabang):**
  - Menambahkan kolom `outlet_id: UUID REFERENCES outlets(id)`.
  - Setiap baris debet/kredit membawa dimensi cabang yang bertanggung jawab, memungkinkan satu entri jurnal mencatat transaksi lintas cabang (multi-branch entries).
- **Tabel `accounting_periods` (Soft Close):**
  - Tabel baru: `id, name, start_date, end_date, is_closed, closed_at, closed_by, reopened_at, reopened_by, reopen_reason, created_at, updated_at`.
- **RBAC Permissions Baru:**
  - `accounting:periods:read`
  - `accounting:periods:close`
  - `accounting:periods:reopen`
  - `accounting:hard_close`

---

## 3. Fase 2: Laporan Keuangan Cabang vs Konsolidasi & Sifat Self-Canceling RAK
- **Laporan Laba Rugi (`Income Statement`) & Neraca (`Balance Sheet`):**
  - **Laporan Per-Cabang:** Menggunakan filter `WHERE jl.outlet_id = 'BRANCH_ID'`.
  - **Laporan Konsolidasi:** Menghapus klausul `WHERE jl.outlet_id`, menjumlahkan saldo seluruh cabang secara otomatis.
- **Mekanisme Self-Canceling RAK (1901):**
  - Pada transaksi transfer kas/dana atau stok antar-cabang:
    - Cabang Asal mendebit RAK (1901) dengan `outlet_id = Cabang_A`.
    - Cabang Tujuan mengkredit RAK (1901) dengan `outlet_id = Cabang_B`.
  - Pada laporan keuangan konsolidasi seluruh cabang, saldo debet dan kredit akun 1901 otomatis saling meniadakan (*self-netting* menjadi Rp 0), tanpa memerlukan eliminasi manual yang rumit.

---

## 4. Fase 3: Mekanisme Tutup Buku (Soft Close & Hard Close)
- **Soft Close (Penguncian Periode Transaksi):**
  - Fungsi validasi `check_transaction_date_allowed` di backend secara fail-closed memeriksa apakah tanggal transaksi berada dalam rentang `accounting_periods` yang `is_closed = TRUE`.
  - Jika berada dalam periode terkunci, request mutasi (Penjualan POS, Pembelian, Biaya/Pengeluaran, Setor Modal, Jurnal Penyesuaian) langsung ditolak dengan pesan:
    > *"Transaksi ditolak: Tanggal transaksi (DD-MM-YYYY) berada dalam periode akuntansi yang terkunci / ditutup (Nama Periode - Tanggal s/d Tanggal)."*
  - Fitur **Reopen Period** memungkinkan pembukaan kunci dengan pencatatan alasan audit (*audit trail*).
- **Hard Close (Tutup Buku Akhir Tahun):**
  - Endpoint `POST /api/v1/accounting/hard-close`.
  - Menghitung total saldo seluruh akun Pendapatan (Revenue - Kategori 4) dan Beban (Expense - Kategori 5) pada tahun fiskal tersebut.
  - Membuat jurnal penutup resmi (`is_closing_entry = TRUE`), mendebit seluruh akun Pendapatan, mengkredit seluruh akun Beban sehingga saldo nominal menjadi 0.
  - Memindahkan selisih Laba/Rugi Bersih ke akun **Laba Ditahan (3201)** pada kelompok Ekuitas.

---

## 5. Pembaruan Antarmuka Frontend (`AccountingPage.vue`)
1. **Tab Bagan Akun (COA):**
   - Kolom "Outlet / Lingkup" diubah menjadi "Cakupan Akun" dengan badge `Universal (Semua Cabang)`.
   - Modal Tambah & Edit Akun menghapus dropdown outlet dan menambahkan catatan integritas Master Universal.
2. **Tab Buku Jurnal Umum (Ledger):**
   - Setiap baris transaksi (*journal lines*) menampilkan badge cabang dimensi (`line.outlet_name`).
   - Header entri penutup menampilkan badge `Tutup Buku`.
3. **Tab Tutup Buku Periode (Closings):**
   - Menambahkan tabel **Periode Akuntansi & Kontrol Kunci Transaksi (Soft Close)** lengkap dengan indikator status terkunci/terbuka, riwayat pengunci, dan tombol *Kunci Periode* / *Buka Kunci*.
   - Tombol **+ Tambah Periode Baru** dengan modal formulir nama & tanggal cut-off.
   - Tombol **Tutup Buku Akhir Tahun (Hard Close)** dengan modal konfirmasi tahun fiskal, catatan, dan ringkasan kaidah pemindahan saldo nominal ke Laba Ditahan (3201).
