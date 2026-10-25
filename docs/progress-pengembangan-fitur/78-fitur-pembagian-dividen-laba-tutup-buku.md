# Laporan Pengembangan: 78 - Fitur Pembagian Dividen Laba dari Rekening Akun Tutup Buku

## 1. Latar Belakang & Kebutuhan Bisnis
Dalam siklus akuntansi standar (PSAK/SAK), hasil usaha suatu entitas bisnis dihitung pada setiap akhir periode akuntansi melalui proses **Tutup Buku (Period-End Closing)**:
1. Seluruh akun nominal (Pendapatan/Revenue dan Beban/Expenses) ditutup ke *Ikhtisar Laba Rugi*.
2. Selisih laba/rugi bersih (*Net Income*) dipindahkan secara resmi ke akun ekuitas **3201 - Laba Ditahan (Retained Earnings)**.
3. Hak para pemilik usaha atau pemegang saham atas laba bersih yang terkumpul dibagikan dalam bentuk **Dividen**.

Untuk menjaga kepatuhan dan integritas pembukuan:
- **Dividen hanya boleh dialokasikan setelah periode buku resmi ditutup** (`is_closed = true`).
- Pembagian dividen langsung memotong saldo rekening ekuitas **Laba Ditahan (3201)** dan mengkredit rekening Kas/Bank pencairan (`11xx`).
- Sistem harus menolak penarikan dividen jika melebihi saldo laba ditahan yang tersedia (*fail-closed safeguard*).
- Sistem mendukung pencatatan pembagian dividen ke banyak penerima (*multi-shareholder allocation*) dengan persentase kepemilikan dan nominal masing-masing.

---

## 2. Arsitektur & Skema Database

### A. Migrasi SQL (`migrations/057_dividend_distributions.sql`)
1. **Tabel `dividend_distributions`**:
   - `id UUID PRIMARY KEY DEFAULT gen_random_uuid()`
   - `distribution_number VARCHAR(50) NOT NULL UNIQUE` (format: `DIV-YYYYMM-XXXX`)
   - `period_id UUID NOT NULL REFERENCES accounting_periods(id)`
   - `distribution_date DATE NOT NULL`
   - `total_amount NUMERIC(15, 2) NOT NULL CHECK (total_amount > 0)`
   - `retained_earnings_account_id UUID NOT NULL REFERENCES chart_of_accounts(id)` (Akun `3201`)
   - `payment_account_id UUID NOT NULL REFERENCES chart_of_accounts(id)` (Kas/Bank)
   - `journal_entry_id UUID REFERENCES journal_entries(id) ON DELETE SET NULL`
   - `recipient_name VARCHAR(150) NOT NULL`
   - `notes TEXT`
   - `recipients_data JSONB` (Array of shareholder allocations: name, percentage, nominal, notes)
   - `created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()`
   - `created_by UUID REFERENCES users(id)`

2. **Bagan Akun (COA)**:
   - Terintegrasi langsung dengan akun `3201 (Laba Ditahan / Retained Earnings)` dan `3301 (Dividen / Pembagian Laba)` pada Kategori 3 (EQUITY).

3. **Registrasi Izin & RBAC (Zero-Trust Security)**:
   - `accounting:dividend:read` -> Melihat daftar dividen & ringkasan saldo laba ditahan.
   - `accounting:dividend:create` -> Melakukan distribusi dividen dan penjurnalan otomatis.
   - `accounting:dividend:delete` -> Membatalkan dividen dan membalik (*reversal*) jurnal umum terkait.
   - Diberikan ke role `SUPERADMIN` secara otomatis.

---

## 3. Backend Implementation (Rust Axum & SQLx)

1. **Model DTO (`src/models/dividend.rs`)**:
   - `DividendDistribution`: Representasi lengkap row distribusi dengan join nama periode, kode COA, dan nomor jurnal.
   - `CreateDividendRequest`: Payload pembuatan dividen dengan validasi nominal, akun kas/bank, dan array penerima.
   - `DividendRecipientItem`: Data tiap pemegang saham (`name`, `share_percentage`, `amount`, `notes`).
   - `ClosedPeriodForDividend`: Data periode tutup buku yang eligible (status `is_closed = true`, laba bersih, dan sisa laba).
   - `DividendSummary`: Ringkasan saldo laba ditahan akun `3201`, total dividen yang sudah dibagikan, dan periode yang eligible.

2. **Service Layer (`src/services/dividend_service.rs`)**:
   - `get_dividend_summary`: Mengkalkulasi saldo riil akun `3201` via `SUM(credit - debit)` jurnal penutup, serta menghitung laba bersih tiap periode terkunci.
   - `create_dividend`:
     - Memverifikasi periode terkait berstatus `is_closed = true`. Jika belum ditutup, melempar pesan validasi error.
     - Memverifikasi total dividen tidak melebihi saldo akun `3201` saat ini.
     - Memverifikasi akun pembayaran aktif dan berkategori ASSET (Kas/Bank).
     - Menghasilkan nomor transaksi urut `DIV-YYYYMM-XXXX`.
     - Membukukan jurnal berpasangan otomatis:
       - **Debet**: `3201 - Laba Ditahan` (Rp Total Dividen)
       - **Kredit**: `Akun Kas/Bank` (Rp Total Dividen)
     - Menyimpan record distribusi beserta rincian pemegang saham di `recipients_data` JSONB.
     - Mencatat jejak audit trail (*Audit Log*).
   - `cancel_dividend`:
     - Membalik (*reversal*) jurnal umum berpasangan.
     - Menghapus record distribusi dividen.
     - Mencatat pembatalan ke audit log.

3. **API Endpoints (`src/api/dividend.rs`)**:
   - `GET /api/v1/dividends` (`accounting:dividend:read`)
   - `GET /api/v1/dividends/summary` (`accounting:dividend:read`)
   - `GET /api/v1/dividends/{id}` (`accounting:dividend:read`)
   - `POST /api/v1/dividends` (`accounting:dividend:create`)
   - `POST /api/v1/dividends/{id}/cancel` (`accounting:dividend:delete`)

---

## 4. Frontend Implementation (Vue 3 Desktop)

1. **Navigasi Tab Akuntansi (`notane-client/src/pages/AccountingPage.vue`)**:
   - Penambahan tab baru **"Pembagian Dividen (Laba)"** berdampingan langsung dengan tab **"Tutup Buku Periode (Closings)"**.
   - Dilengkapi badge hitung total transaksi dividen.
   - Dukungan lazy-loading server-side on-demand caching.

2. **Koneksi Cepat pada Tab Tutup Buku**:
   - Pada baris periode yang berstatus `TERKUNCI (SOFT CLOSED)`, ditambahkan tombol aksi **"Bagi Dividen"** yang langsung membuka modal formulir dengan periode tersebut otomatis terpilih.

3. **Tampilan Tab Dividen Laba**:
   - **Banner Panduan PSAK**: Menjelaskan hubungan fundamental laba ditahan dengan periode tutup buku.
   - **3 Kartu Ringkasan (KPI)**:
     1. Saldo Akun Laba Ditahan (3201) Tersedia.
     2. Total Dividen Telah Dibagikan.
     3. Jumlah Periode Tutup Buku Siap Dividen.
   - **Filter & Pencarian**: Filter periode tutup buku, kolom cari no. transaksi/penerima/catatan.
   - **Tabel Distribusi Dividen**: No. Distribusi, Tanggal, Periode Terkait, Rekening Pencairan, Total Dividen, Alokasi Penerima, Tombol Pintas Nomor Jurnal, dan Aksi (Detail/Voucher & Batalkan).

4. **Modal Interaktif**:
   - **Modal Pembagian Dividen Baru**:
     - Pilihan periode tutup buku dengan tampilan laba bersih.
     - Pilihan rekening kas/bank pencairan.
     - Input total nominal dividen dengan validasi real-time saldo akun 3201.
     - Tabel dinamis alokasi pemegang saham (Nama, Porsi %, Nominal Rp, Catatan).
     - Pratinjau otomatis jurnal umum seimbang (*Debit: 3201*, *Kredit: Kas/Bank*).
   - **Modal Bukti Dividen (Voucher)**: Menampilkan voucher resmi siap cetak (*Print*).
   - **Modal Konfirmasi Pembatalan**: Dialog pencegahan data hilang dengan penjelasan reversal jurnal.

---

## 5. Pengujian & Verifikasi

1. **Kompilasi Rust Server**:
   - `cargo check --bin notane-retail-server` lolos tanpa error.
   - Server daemon berjalan aktif di port 18080/18081.
2. **Kompilasi Frontend Client**:
   - `vue-tsc && vite build` lolos 100% tanpa error TypeScript.
3. **End-to-End Browser Testing**:
   - Berhasil membuka halaman `/admin/accounting`.
   - Berhasil berpindah ke tab "Pembagian Dividen (Laba)".
   - Nilai saldo Laba Ditahan (3201) dan periode tutup buku terbaca dengan benar dari database server.
   - Modal formulir pembagian dividen terbuka dengan pratinjau jurnal dan alokasi penerima yang reaktif.
