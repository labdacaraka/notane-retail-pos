# Progress Fitur: Pengunci Akun Sistem (System CoA Lock) & Manajemen Rekening Bank Resmi

## Ringkasan Eksekutif
Berdasarkan kebutuhan integritas akuntansi dan pelaporan keuangan:
1. **Pengunci CoA Sistem (System Chart of Accounts Lock):** Seluruh akun perkiraan bawaan (seed/default) yang terhubung dengan otomatisasi jurnal POS, pembelian, retur, beban, setoran modal, dan persediaan dikunci secara absolut dari perubahan dan penghapusan agar tidak merusak sistem pembukuan.
2. **Tab Manajemen Rekening Bank Resmi:** Pengguna yang ingin menambahkan rekening bank usaha untuk transaksi operasional/setoran modal kini memiliki tab khusus **"Rekening Bank"** yang otomatis mengaitkan akun CoA aset lancar bank dengan metadata perbankan resmi (Nama Bank, Nomor Rekening, Atas Nama, Cabang/Kantor).

---

## 1. Arsitektur Pertahanan Berlapis (Defense-in-Depth)

### A. Layer Database (Migration 035 & 036)
- **File:** `src-tauri/migrations/035_coa_system_lock.sql` & `036_lock_all_system_coa_and_bank_accounts.sql`
- Menambahkan kolom `is_system BOOLEAN NOT NULL DEFAULT FALSE` pada tabel `chart_of_accounts`.
- Mengunci seluruh akun sistem bawaan (baik yang menggunakan prefix UUID `a1000000-0000-0000-0000-` maupun yang terdaftar pada migration 002, 018, 021, 022, 023, 025, 027, 033, 034).
- Menambahkan skema tabel `bank_accounts` untuk pencatatan detail rekening bank resmi toko.
- CHECK CONSTRAINT pada database mencegah akun bertanda `is_system = TRUE` dinonaktifkan (`is_active = FALSE`).

### B. Layer Backend (Rust / Actix-Web / Tauri)
- **Model:** `src-tauri/src/models/accounting.rs` menyertakan field `is_system: bool`.
- **Service Guard:** `src-tauri/src/services/accounting_service.rs`
  - Pada fungsi `update_chart_of_account`, mengecek status `is_system`. Jika bernilai `true`, langsung menolak perubahan dengan error: `"Akun sistem terproteksi dan tidak dapat diubah atau dihapus"`.
  - Mengamankan kueri UPDATE dengan klausa ganda `AND is_system = FALSE`.
- **API Rekening Bank:**
  - `src-tauri/src/api/banks.rs` & `src-tauri/src/services/bank_service.rs`
  - Mendukung pembuatan rekening bank baru yang otomatis membuat akun CoA bertipe `asset` (subkategori Kas & Bank) dan menandainya aktif.

### C. Layer Frontend (Vue.js + TypeScript)
- **Komponen:** `src/pages/AccountingPage.vue`
  - **Badge Gembok:** Akun sistem menampilkan lencana `🔒 Sistem` berwarna abu gelap/emerald di samping nama akun.
  - **Tombol Terkunci:** Tombol "Edit" pada baris akun sistem digantikan dengan indikator visual `🔒 Terkunci` berstatus disabled / cursor-not-allowed.
  - **Script Guard:** Fungsi `openEditCoaModal` memblokir pembukaan modal edit jika `coa.is_system === true` dan memunculkan toast peringatan.
  - **Tab Rekening Bank:** Tab baru di sebelah Tab Akun Perkiraan dengan kartu bank visual yang mewah, tombol salin nomor rekening, dan dialog pendaftaran rekening bank baru.
