# Dokumentasi Fitur #42: Penguatan Validasi Backend & Penegakan Hak Akses (Permission Guards)

## 1. Ringkasan Eksekutif
Sesuai dengan prinsip *Zero Trust pada Frontend* dan *Fail-Closed by Default*, seluruh aksi (action features) pada sistem Notane Retail POS kini memiliki validasi semantik di sisi backend Rust serta penegakan hak akses (permission guards) berbasis token JWT dan tabel hak akses database:
1. **Pendaftaran Hak Akses Database (Migration 028)**:
   - Menambahkan permission baru untuk loyalitas dan poin member:
     - `loyalty:manage`: Mengatur konfigurasi konversi poin, batas persentase, dan aturan loyalitas.
     - `loyalty:redeem`: Otorisasi penukaran poin reward loyalitas di kasir.
     - `loyalty:read`: Melihat konfigurasi program loyalitas dan saldo poin member.
   - Hak akses dipetakan ke role: `SUPERADMIN`, `STORE_MANAGER`, `SUPERVISOR`, dan `CASHIER`.
2. **Penegakan Hak Akses di Seluruh Route & Handler Backend**:
   - `jwt.rs`: Menyediakan fungsi `require_permission` dan `require_any_permission`.
   - `master.rs`: Middleware `master_write_middleware` kini memvalidasi `loyalty:manage` / `settings:manage` untuk aturan loyalitas, dan `master:write` untuk entitas master lainnya.
   - `pos.rs`:
     - Buka Shift (`shift:open`)
     - Tutup Shift (`shift:close`)
     - Cek Shift Aktif (`shift:view` / `pos:checkout`)
     - Checkout Kasir (`pos:checkout`) + penukaran poin (`loyalty:redeem`)
     - Pembatalan Transaksi / Void (`pos:void`)
     - Laporan & Riwayat Nota (`report:view` / `pos:checkout`)
   - `flash_sales.rs`:
     - Buat, ubah, hapus flash sale (`promo:write` / `master:write`)
     - Lihat daftar dan detail promo (`promo:read` / `master:read`)
   - `inventory.rs`:
     - Penyesuaian stok / stock opname (`inventory:adjust`)
     - Pembelian / Purchasing PO, Faktur, Retur (`purchasing:write` & `purchasing:read`)
     - Penerimaan barang GRN dan mutasi transfer stok (`inventory:write` & `inventory:read`)
   - `reports.rs`:
     - Jurnal manual, tutup buku periode (`accounting:write` & `accounting:read`)
     - Ringkasan penjualan, neraca, laba rugi, Z-Report kasir (`report:view`)
   - `users.rs`:
     - Manajemen pengguna, peran, dan matriks hak akses (`users:manage`)
     - Ubah PIN kasir (`pin:manage` / `users:change_pin`)
     - Akses profil staf mandiri (`/me/*`) dengan otentikasi JWT valid
3. **Validasi Semantik & Batasan Input di Sisi Backend**:
   - Pengaturan Poin: Nilai tukar > 0 & <= Rp 1.000.000, minimal belanja >= 0 & <= Rp 1.000.000.000, batas persentase pemakaian poin 0% - 100%, minimal poin >= 0 & <= 1.000.000.
   - POS Checkout: Keranjang belanja tidak boleh kosong, kuantitas item > 0, harga >= 0, rincian pembayaran tidak boleh kosong, nominal pembayaran > 0, penukaran poin wajib memiliki `customer_id`.
   - Buka/Tutup Shift: Modal kas awal dan kas akhir tidak boleh bernilai negatif.
   - Void Nota: PIN supervisor wajib diisi.
   - Inventori: Item GRN, stock opname, dan transfer stok tidak boleh kosong; outlet asal dan tujuan transfer tidak boleh sama.
   - Akuntansi: Baris jurnal manual tidak boleh kosong.
   - Pengguna: Validasi panjang username minimal 3 karakter, kata sandi minimal 6 karakter, PIN minimal 4 digit.
4. **Proteksi UI (Frontend Defense-in-Depth)**:
   - [src/pages/CustomersPage.vue](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src/pages/CustomersPage.vue):
     - Pemeriksaan reaktif `canManageLoyalty` (`loyalty:manage`, `settings:manage`, `master:write`).
     - Tampilan banner informasi mode lihat-saja (*Read-Only*) bagi staf yang tidak memiliki izin ubah.
     - Penonaktifan seluruh input kendali (toggle status, input konversi, slider persentase, preset, minimal poin) dan tombol simpan jika staf tidak memiliki hak akses.
     - Validasi pre-flight form di sisi client sebelum mengirim permintaan ke server.

---

## 2. Pengujian & Bukti Verifikasi
1. **Unauthenticated Request (401)**:
   - `PUT /api/v1/settings/loyalty` tanpa token ditolak dengan respons `{"message":"Autentikasi dibutuhkan. Silakan login.","success":false}`.
   - `POST /api/v1/sales/checkout` tanpa token ditolak dengan respons `{"message":"Autentikasi dibutuhkan. Silakan login.","success":false}`.
2. **Validasi Batasan Nilai Backend (400 Bad Request)**:
   - Persentase poin > 100% (`150%`) ditolak: `{"message":"Batas persentase penggunaan poin harus berada di antara 0% dan 100%","success":false}`.
   - Nilai 1 poin <= 0 (`0`) ditolak: `{"message":"Nilai 1 poin dalam rupiah harus lebih besar dari 0 dan maksimal Rp 1.000.000","success":false}`.
   - Minimal belanja per poin negatif (`-500`) ditolak: `{"message":"Minimal belanja per poin tidak boleh negatif dan maksimal Rp 1.000.000.000","success":false}`.
   - Checkout kasir dengan keranjang kosong ditolak: `{"message":"Keranjang belanja tidak boleh kosong","success":false}`.
   - Buka shift kasir dengan uang modal negatif ditolak: `{"message":"Modal awal kas (opening cash) tidak boleh negatif","success":false}`.
3. **Penyimpanan Pengaturan Terotorisasi (200 OK)**:
   - Permintaan valid menggunakan token akun yang memiliki hak akses berhasil disimpan dan mengembalikan kode status 200 OK.
