# Progress 72: Perbaikan Fitur Transfer Kas / Bank & Migrasi RAK

## Masalah yang Ditemukan
Saat pengguna mencoba fitur transfer kas/bank di halaman akuntansi (`http://localhost:5175/admin/accounting`), terjadi error:
1. **Error Database**: `Database error: error returned from database: column "rak_account_id" does not exist`
   - **Penyebab**: File migrasi `052_rekening_antar_kantor_dan_jurnal_cabang.sql` yang menambahkan kolom `rak_account_id` pada tabel `outlets` serta inisialisasi akun COA RAK (1901) sudah dibuat di direktori migrations, namun belum didaftarkan di array `run_embedded_migrations` pada `notane-server/src-tauri/src/db/mod.rs`. Akibatnya migrasi tersebut belum dieksekusi ke PostgreSQL.
2. **Keterbatasan Logika Bisnis Transfer**:
   - Backend `create_fund_transfer` dan modal di `AccountingPage.vue` sebelumnya melarang transfer bila outlet asal sama dengan outlet tujuan (`Outlet asal dan outlet tujuan tidak boleh sama`).
   - Padahal dalam operasional kasir & retail harian, mutasi internal kas/bank pada outlet yang sama (misalnya: setoran kas laci kasir `1101` ke rekening bank settlement `1102`) adalah transaksi yang sangat umum, terutama bagi toko dengan 1 cabang (single-outlet).

---

## Solusi & Implementasi

### 1. Registrasi Migrasi 052 (`notane-server/src-tauri/src/db/mod.rs`)
Menambahkan `("052_rekening_antar_kantor_dan_jurnal_cabang", include_str!("../../migrations/052_rekening_antar_kantor_dan_jurnal_cabang.sql"))` ke dalam array migrasi otomatis pada startup server sehingga kolom `outlets.rak_account_id` dan COA RAK dibuat secara otomatis.

### 2. Peningkatan Layanan Backend (`notane-server/src-tauri/src/services/accounting_service.rs`)
Memperbarui fungsi `create_fund_transfer`:
- Mengubah validasi: yang dilarang adalah `from_account_id == to_account_id` (tidak boleh transfer ke akun yang sama), bukan outlet yang sama.
- **Mode Transfer Internal Kas / Bank (`from_outlet_id == to_outlet_id`)**:
  - Menghasilkan 1 jurnal umum seimbang (`JRN-FND-INT-...`) pada outlet tersebut:
    - **Debit**: Akun Kas / Bank Tujuan
    - **Credit**: Akun Kas / Bank Asal
- **Mode Transfer Antar-Cabang (`from_outlet_id != to_outlet_id`)**:
  - Menghasilkan 2 jurnal berpasangan seimbang melalui akun kliring Rekening Antar-Kantor (RAK) per masing-masing cabang.

### 3. Pembaruan UI Frontend (`notane-client/src/pages/AccountingPage.vue`)
- Mengubah label tombol menjadi `+ Transfer Kas / Bank` dan judul modal menjadi `Transfer Kas / Bank & Antar-Cabang`.
- Mendukung default pemilihan outlet untuk single-outlet maupun multi-outlet (`to_outlet_id: defaultTo || defaultFrom`).
- Memberikan indikator dinamis pada modal:
  - *Mode Internal Kas/Bank*: "Transfer Kas/Bank Internal (Debit/Kredit langsung pada cabang yang sama)".
  - *Mode Antar-Cabang*: "Transfer Antar-Cabang (Otomatis menggunakan kliring Rekening Antar-Kantor / RAK)".
- Validasi tombol submit diperbarui agar hanya aktif jika `from_account_id !== to_account_id`.

---

## Hasil Pengujian & Verifikasi
1. **Kompilasi**: `cargo build --bin notane-retail-server` dan `vue-tsc --noEmit` berhasil tanpa error (0 errors).
2. **Migrasi**: Kolom `rak_account_id` dan akun RAK (1901) berhasil diinisialisasi pada PostgreSQL.
3. **Pengujian Transfer Internal**:
   - Membukukan transfer Rp 50.000 dari *Kas Operasional Laci Kasir* (1101) ke *Bank & Piutang QRIS Settlement* (1102) di cabang yang sama.
   - Response API mengembalikan status `200 OK` dengan nomor jurnal `JRN-FND-INT-...`.
   - Jurnal umum tercatat seimbang: Debit 50.000, Kredit 50.000.
4. **Pengujian Transfer Antar-Cabang (RAK)**:
   - Membukukan transfer Rp 75.000 dari *Notane Retail Flagship Store* ke *Gudang Utama Toko*.
   - Terbentuk 2 jurnal RAK berpasangan: `JRN-FND-OUT-...` dan `JRN-FND-IN-...` dengan saldo seimbang.

---

## Penambahan Validasi Hak Akses Outlet Pengirim & Ketersediaan Saldo Rekening

### 1. Pembatasan Hak Akses Outlet Pengirim (Zero-Trust & Fail-Closed)
- **Frontend**:
  - Dropdown *Cabang Asal* pada modal transfer kas/bank dihubungkan ke endpoint `/me/outlets`, sehingga hanya menampilkan lokasi/cabang yang diizinkan untuk akun pengguna yang sedang login (`user_outlets`).
  - Pemilihan default cabang asal mengutamakan lokasi yang diakses pengguna.
- **Backend**:
  - Pada `create_fund_transfer_handler` di [reports.rs](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/notane-server/src-tauri/src/api/reports.rs) dan `create_fund_transfer` di [accounting_service.rs](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/notane-server/src-tauri/src/services/accounting_service.rs), dilakukan verifikasi otorisasi: pengguna non-superadmin wajib terdaftar di `user_outlets` untuk cabang `from_outlet_id` (kecuali memiliki hak `accounting:all_ownership`). Jika tidak, permintaan transfer ditolak dengan pesan: *"Akses ditolak: Anda tidak memiliki wewenang pada cabang pengirim tersebut."*.

### 2. Validasi Ketersediaan Saldo Rekening Kas/Bank Pengirim
- **Backend API & Service**:
  - Disediakan endpoint `GET /api/v1/accounting/accounts/:id/balance?outlet_id=...` dan fungsi `get_account_balance` di backend untuk menghitung saldo berjalan (*current balance*) akun aset/kas/bank secara real-time berdasarkan riwayat jurnal debet/kredit pada cabang tersebut.
  - Sebelum mutasi dicatat, sistem memverifikasi bahwa saldo rekening asal > 0 dan `nominal transfer <= saldo kas/bank asal`. Jika saldo tidak mencukupi, transaksi dibatalkan dengan pesan informatif: *"Nominal transfer (Rp ...) melebihi saldo rekening kas/bank asal yang tersedia (Saldo saat ini: Rp ...)"* atau *"Saldo rekening kas/bank asal tidak mencukupi"*.
- **Frontend Real-time UX**:
  - Menampilkan badge saldo real-time (`Saldo: Rp ...`) saat rekening asal dipilih.
  - Menampilkan peringatan interaktif jika saldo kosong atau nominal transfer melampaui saldo yang ada.
  - Tombol submit dinonaktifkan otomatis bila saldo rekening asal tidak mencukupi atau nominal melebihi batas saldo.

