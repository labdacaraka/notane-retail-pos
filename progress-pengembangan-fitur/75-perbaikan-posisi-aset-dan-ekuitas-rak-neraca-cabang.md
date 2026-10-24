# Progress Fitur #75: Perbaikan Neraca Cabang & Rekening Antar-Kantor (RAK) sebagai Ekuitas Cabang

## Latar Belakang & Masalah
Pada tab **Posisi Aset & Neraca (Balance Sheet)** di halaman Pembukuan Akuntansi (`/admin/accounting`), ketika pengguna memfilter cabang penerima seperti **Cabang Tegal Besar** dan **Gudang Utama Toko**:
1. **Total Aktiva bernilai 0**, padahal akun aset lancar riil memiliki saldo (misal: Bank Rp 100.000 dan Persediaan Rp 28.000 di Cabang Tegal Besar; Kas Rp 10.000 dan Bank Rp 75.000 di Gudang Utama).
2. **Total Ekuitas bernilai 0** dan tidak ada akun yang tampil di sisi kanan (Pasiva).
3. Laporan tetap menunjukkan status *Balanced* karena `0 == 0`, namun menyembunyikan posisi aset sebenarnya dari cabang tersebut.

## Akar Masalah (Root Cause Analysis)
1. **Perlakuan Akun 1901 (Rekening Antar-Kantor / RAK):**
   - RAK terdaftar pada bagan akun (`chart_of_accounts`) dengan kategori `ASSET` (kategori 1).
   - Ketika cabang menerima transfer kas atau barang dari Kantor Pusat / Cabang Pengirim:
     - Cabang Penerima menjurnal: **Debit Aset (Kas/Bank/Persediaan)** dan **Kredit RAK (1901)**.
     - Saldo RAK cabang penerima bersaldo **Kredit** (`debit - credit < 0`, misal `-Rp 128.000`).
   - Pada saat query kalkulasi neraca dijalankan:
     - Query aset mengambil seluruh akun berkategori `ASSET` (`SUM(jl.debit - jl.credit)`).
     - RAK dimasukkan ke dalam `non_current_assets` dengan nilai negatif `-Rp 128.000`.
     - Akibatnya: `current_assets (Rp 128.000) + non_current_assets (-Rp 128.000) = Total Aset Rp 0`.
   - Sementara itu, query Ekuitas hanya mengambil akun berkategori `EQUITY`, sehingga RAK bersaldo kredit tidak masuk ke sisi Ekuitas (`Total Ekuitas = Rp 0`).
2. **Penyajian UI pada Kolom Aktiva:**
   - Seluruh daftar aset (`balanceSheet.asset_details`) sebelumnya ditampilkan di bawah satu sub-header `A. ASET LANCAR`, tanpa memisahkan kelompok aset lancar dan aset tidak lancar / RAK.

## Solusi & Implementasi (Standar Akuntansi Kantor Pusat & Cabang)
1. **Backend (`notane-server/src-tauri/src/services/accounting_service.rs`):**
   - Mendeteksi akun RAK (`code.starts_with("19")`):
     - Jika saldo RAK cabang bernilai **Kredit** (`amt < 0`):
       - Nilai ini adalah **Modal Kerja / Penempatan dari Kantor Pusat** (Ekuitas Cabang).
       - Dikecualikan dari Total Aset (tidak mengurangi aset riil).
       - Dimasukkan ke dalam `equity_details` dan `recorded_equity` sebesar nilai absolutnya (`amt.abs()`) dengan label `Rekening Antar-Kantor (Modal / Penempatan dari Kantor Pusat)`.
     - Jika saldo RAK bernilai **Debit** (`amt > 0`, seperti pada Kantor Pusat / Flagship Store pengirim):
       - Nilai ini merupakan piutang/investasi pada cabang lain, tetap disajikan pada sisi **Aset Tidak Lancar & Antar-Kantor** (`non_current_assets`).
     - Pada laporan konsolidasian (Semua Cabang / `outlet_id = null`):
       - Seluruh transaksi RAK resiprokal antar-cabang saling mengeliminasi (*self-netting* menjadi `0`), sehingga tidak muncul baik di Aset maupun Ekuitas.
2. **Frontend Client (`notane-client/src/pages/AccountingPage.vue`):**
   - Menambahkan computed filter `currentAssetDetails` (akun `11xx` dan `12xx`) dan `nonCurrentAssetDetails` (akun selain `11xx` dan `12xx`).
   - Memisahkan tampilan Kolom Kiri menjadi:
     - **A. ASET LANCAR (CURRENT ASSETS)**
     - **B. ASET TIDAK LANCAR & ANTAR-KANTOR (NON-CURRENT ASSETS)** (ditampilkan jika ada saldo non-lancar)
   - Memperbarui kartu ringkasan Aset untuk menyajikan porsi aset lancar dan aset tidak lancar/antar-kantor.
   - Menambahkan fallback tampilan informatif jika belum ada modal/ekuitas tercatat.

## Hasil Verifikasi
1. **Cabang Tegal Besar (`31da5fa6-a635-4ed0-91c8-05a4a7164973`):**
   - Total Aktiva: **Rp 128.000** (Bank Rp 100.000, Persediaan Rp 28.000)
   - Total Pasiva: **Rp 128.000** (Ekuitas RAK dari Kantor Pusat Rp 128.000)
   - Status: **Balanced ✓**
2. **Gudang Utama Toko (`b0000000-0000-0000-0000-000000000002`):**
   - Total Aktiva: **Rp 85.000** (Kas Rp 10.000, Bank Rp 75.000)
   - Total Pasiva: **Rp 85.000** (Ekuitas RAK dari Kantor Pusat Rp 85.000)
   - Status: **Balanced ✓**
3. **Notane Retail Flagship Store (Kantor Pusat):**
   - Total Aktiva: **Rp 1.016.355** (Termasuk Aset Antar-Kantor Rp 213.000)
   - Total Pasiva: **Rp 1.016.355** (Kewajiban Rp 1.155 + Ekuitas Rp 1.015.200)
   - Status: **Balanced ✓**
4. **Semua Cabang (Konsolidasian):**
   - Total Aktiva: **Rp 1.026.355**
   - Total Pasiva: **Rp 1.026.355**
   - Status: **Balanced ✓** (RAK tereliminasi 100% resiprokal).
