# Dokumentasi Fitur #41: Pengaturan Konversi Poin ke Rupiah, Batas Persentase, dan Multi-Payment Kasir

## 1. Ringkasan Eksekutif
Fitur ini mengimplementasikan sistem loyalitas pelanggan yang fleksibel dan aman:
1. **Pengaturan Konversi Poin ke Rupiah**: Pengelola toko dapat mengatur nilai tukar poin (misal 1 poin = Rp 100), minimal belanja untuk mendapatkan 1 poin (misal Rp 10.000 = 1 poin), dan minimal poin untuk penukaran.
2. **Pengaturan Batas Persentase Maksimal Pemakaian Poin**: Pengelola toko dapat membatasi maksimal persentase penggunaan poin terhadap total belanja (misal maksimal 50% atau 70%), sehingga pelanggan tidak dapat membayar 100% menggunakan poin jika kebijakan toko menghendaki adanya pembayaran uang riil.
3. **Pembayaran Multi-Payment Berbarengan di Kasir (POS)**: Pada saat checkout, kasir dapat menerapkan poin member (sebagian atau sesuai batas maksimal) dan sisa tagihan dibayarkan berbarengan menggunakan metode pembayaran lain seperti Tunai (Cash), QRIS, Transfer Bank, atau Kartu Debit/Kredit (EDC).
4. **Akuntansi & Auto-Journaling**: Nilai penukaran poin dicatat secara otomatis ke dalam bagan akun Chart of Accounts (CoA) `4106` (*Potongan Penjualan - Penukaran Poin Reward*) dengan mekanisme double-entry seimbang tanpa mengganggu saldo kas/bank.
5. **Pemulihan Poin Saat Void**: Jika transaksi kasir dibatalkan/di-void, poin yang telah dipotong otomatis dikembalikan ke saldo member dengan riwayat ledger `CORRECTION`.

---

## 2. Perubahan Database & Migrasi
File migrasi: [src-tauri/migrations/027_loyalty_point_settings_and_rules.sql](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src-tauri/migrations/027_loyalty_point_settings_and_rules.sql)
- **Tabel `loyalty_settings`**:
  - `id`: Primary key (singleton).
  - `is_enabled`: Flag aktif/nonaktif sistem poin loyalty.
  - `point_value_rupiah`: Nilai 1 poin dalam satuan Rupiah (default: Rp 100).
  - `min_spend_per_point`: Nominal belanja untuk menghasilkan 1 poin (default: Rp 10.000).
  - `max_point_usage_percent`: Batas persentase maksimal pemakaian poin per transaksi (0 - 100%, default: 50%).
  - `min_redeem_points`: Minimal saldo poin untuk penukaran (default: 1 poin).
- **Chart of Accounts (CoA)**:
  - Menyisipkan akun kontra-pendapatan `4106` (*Potongan Penjualan - Penukaran Poin Reward*).
- Didaftarkan pada Batch 17 di [src-tauri/src/db/mod.rs](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src-tauri/src/db/mod.rs).

---

## 3. Backend Implementation (Rust)
1. **Model Pengaturan**:
   - [src-tauri/src/models/settings.rs](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src-tauri/src/models/settings.rs): Struktur data `LoyaltySettings` dan DTO `UpdateLoyaltySettingsRequest`.
2. **API Endpoint Pengaturan Master**:
   - [src-tauri/src/api/master.rs](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src-tauri/src/api/master.rs):
     - `GET /api/v1/settings/loyalty` (mengambil konfigurasi aktif).
     - `PUT /api/v1/settings/loyalty` (memperbarui konfigurasi, diproteksi permission `settings:manage`).
3. **Logika Transaksi Kasir & Multi-Payment**:
   - [src-tauri/src/services/pos_service.rs](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src-tauri/src/services/pos_service.rs):
     - Membaca pengaturan `loyalty_settings` saat checkout.
     - Memeriksa pembayaran bertipe `POINTS`:
       - Menghitung batas nominal maksimal pemakaian poin berdasarkan `max_point_usage_percent` dari total belanja.
       - Memvalidasi bahwa nominal penukaran poin tidak melebihi batas persentase kebijakan toko.
       - Mengonversi nominal Rupiah ke jumlah poin yang dibutuhkan dan memeriksa kecukupan saldo poin pelanggan.
       - Memotong saldo poin member di database dan mencatat transaksi ke `customer_point_ledgers` dengan status `REDEEMED`.
     - Multi-payment: Menerima rincian pembayaran poin bersamaan dengan metode tunai/non-tunai lainnya dalam satu transaksi.
     - Auto-journaling: Mendebit akun CoA `4106` (*Potongan Penjualan - Penukaran Poin Reward*) untuk porsi poin, mendebit Kas/Bank untuk sisa pembayaran riil, dan mengkredit Penjualan + Pajak.
     - Perhitungan perolehan poin baru (`earned_points`) dihitung dari sisa belanja setelah dikurangi diskon & pemakaian poin.
     - Saat `void_sale`, saldo poin yang ditukar dikembalikan ke akun member dengan entri ledger `CORRECTION`.

---

## 4. Frontend & User Interface (Vue 3 / TypeScript)
1. **Halaman Pengaturan Member & Poin**:
   - [src/pages/CustomersPage.vue](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src/pages/CustomersPage.vue):
     - Tab baru: **"Aturan Poin & Konversi Rupiah"**.
     - Tombol toggle aktif/nonaktif sistem reward poin.
     - Input nilai 1 poin ke Rupiah dan syarat minimal belanja per poin.
     - Slider interaktif batas persentase pemakaian poin (0% - 100%) dengan tombol preset cepat: `25%`, `50%`, `70%`, `100%`.
     - Kotak simulasi interaktif (*Live Preview*) agar kasir/manajer dapat melihat langsung kalkulasi belanja Rp 100.000 dengan kepemilikan 500 poin.
2. **Modal Pembayaran Kasir (QuickPayModal)**:
   - [src/components/pos/QuickPayModal.vue](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src/components/pos/QuickPayModal.vue):
     - Saat memilih pelanggan member, kartu informasi poin muncul secara cerdas.
     - Menampilkan saldo poin yang dimiliki, konversi dalam Rupiah, dan batas maksimal poin yang diperbolehkan untuk transaksi tersebut.
     - Tombol cepat penggunaan poin: `25%`, `50%`, `Maksimal Poin`, atau `Input Manual`.
     - Setelah poin diterapkan, kasir langsung melihat sisa tagihan (*Sisa Tagihan Belanja*) yang harus dilunasi.
     - Kasir dapat langsung melunasi sisa tagihan tersebut menggunakan Tunai (dengan hitungan kembalian otomatis), QRIS, Transfer Bank, atau Kartu EDC.
     - Tersedia tombol `Batal Poin` jika pelanggan berubah pikiran.

---

## 5. Pengujian & Verifikasi
1. **Pengujian Simpan Pengaturan**: Berhasil menyimpan `point_value_rupiah: 100`, `min_spend_per_point: 10000`, `max_point_usage_percent: 50` via API dan UI.
2. **Pengujian Validasi Persentase Maksimal**: Transaksi kasir yang mencoba menukar poin melebihi 50% total tagihan berhasil ditolak oleh backend dengan pesan error: `Penggunaan poin melebihi batas maksimal 50.00% dari total belanja (Maksimal: Rp 3885.00)`.
3. **Pengujian Multi-Payment Berhasil**:
   - Total tagihan: Rp 7.770
   - Poin diterapkan: Rp 3.000 (30 poin)
   - Sisa tagihan dibayar Tunai: Rp 5.000
   - Kembalian: Rp 230
   - Hasil: Invoice `INV-20260930-59805211` sukses diterbitkan, poin terpotong 30 poin, riwayat ledger `REDEEMED` tercatat, dan jurnal akuntansi terbit seimbang.
