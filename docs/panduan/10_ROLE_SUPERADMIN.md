# BAB 10: PANDUAN PENGGUNAAN SUPER ADMINISTRATOR (SUPERADMIN)
**System Administration, RBAC Matrix & Enterprise Configuration**

Peran **SUPERADMIN** memegang kendali tertinggi atas konfigurasi teknis, manajemen akun staf, matriks hak akses perizinan (*Role-Based Access Control*), dan pengaturan parameter global bisnis retail.

---

## 1. MANAJEMEN AKUN PENGGUNA & PENUGASAN CABANG

Superadmin berhak menambah akun staf baru, menentukan penugasan cabang gerai toko (*Outlet Assignment*), dan mengatur status keaktifan akun.

![Manajemen Pengguna dan Staf Toko](./images/super_01_users_management.png)

### Elemen Manajemen Pengguna yang Ditandai:
1. **[1] Tombol Tambah Pengguna Baru:** Membuka formulir pendaftaran staf baru.
2. **[2] Filter Peran (*Role Filter*):** Menyaring daftar staf berdasarkan peran tugasnya.
3. **[3] Mode Impersonate (Masuk Sementara):** Fitur diagnostik khusus Superadmin untuk login sementara ke akun staf lain tanpa mengetahui kata sandinya, berguna untuk memvalidasi kendala tampilan yang dialami oleh staf tertentu.

### Langkah Menambahkan Staf Baru:
1. Klik tombol **"Tambah Pengguna"**.
2. Masukkan identitas staf: Nama Lengkap, Username unik, Email, dan Nomor Telepon.
3. Masukkan Kata Sandi login dan PIN otorisasi 6-digit.
4. Pilih **Peran (Role)** yang sesuai dengan deskripsi tugasnya (misal: `CASHIER`, `SUPERVISOR`, `WAREHOUSE`).
5. Centang **Outlet yang Dapat Diakses** (kasir hanya boleh mengakses gerai tempatnya bertugas, sedangkan staf audit dapat mengakses seluruh cabang).
6. Klik **"Simpan Pengguna"**.

---

## 2. PENGATURAN MATRIKS HAK AKSES PERAN (RBAC PERMISSIONS MATRIX)

Notane Retail menerapkan arsitektur perizinan granular di mana setiap tombol, aksi, dan menu dapat diaktifkan atau dinonaktifkan per peran secara mandiri.

![Matriks Hak Akses Peran](./images/super_02_matrix.png)

### Prosedur Mengatur Hak Akses:
1. Masuk ke tab **[2] Roles & Permissions**.
2. Pilih peran yang ingin disesuaikan (misal: `CASHIER` atau `SUPERVISOR`).
3. Modul perizinan dikelompokkan secara rapi:
   - *Kasir & Transaksi POS* (`pos:*`, `sales:*`)
   - *Retur Penjualan Konsumen* (`sales_returns:*`)
   - *Manajemen Shift Kasir* (`shift:*`)
   - *Stok & Inventori* (`inventory:*`)
   - *Pembelian & Pengadaan* (`purchasing:*`)
   - *Pengeluaran & Biaya Operasional* (`expenses:*`)
   - *Akuntansi & Jurnal* (`accounting:*`)
4. Centang atau hilangkan centang pada hak akses yang diinginkan.
5. Klik tombol **[2] "Simpan Hak Akses Peran"**. Seluruh pengguna dengan peran tersebut akan langsung terikat dengan matriks hak akses baru tanpa perlu restart server.

---

## 3. PENGATURAN STRUK, PEMBULATAN, PAJAK & PARAMETER RETAIL

Superadmin mengonfigurasi format struk belanja, pembulatan kembalian kas, kebijakan tarif pajak PPN, serta master klasifikasi retail:

1. **Pengaturan Desain Struk, Pembulatan & Pajak (`/admin/receipt-settings`):**
   - **Profil Toko & Header:** Menentukan nama toko, slogan, alamat gerai, nomor telepon/WhatsApp, dan NPWP toko.
   - **Pesan Footer:** Mengatur pesan keramahan, syarat retur barang membawa struk asli, dan akun media sosial.
   - **Format Cetak Thermal:** Memilih lebar kertas thermal 58mm atau 80mm, menguji cetak thermal (*Uji Cetak*), serta visibilitas barcode dan rincian diskon.
   - **Kebijakan Pembulatan Kas Kasir (*Cash Rounding*):** Menentukan metode pembulatan nominal uang kembalian tunai (Tanpa Pembulatan, Bulatkan Turun Rp 100 / Rp 500, Bulatkan Naik Rp 100 / Rp 500, atau Opsi Donasi Receh ke Akun Titipan Donasi Konsumen 2104).
   - **Ambang Batas Kas Laci Kasir (*Max Cash Drawer Limit*):** Menetapkan limit saldo fisik laci kasir (misal Rp 2.000.000,-) untuk peringatan otomatis setor kas laci (*Cash Drop*).
   - **Kebijakan & Tarif Pajak PPN:** Mengaktifkan/menonaktifkan pemungutan PPN toko (status PKP / non-PKP), mengatur persentase tarif PPN standar toko (11%, 12%, atau 0%), serta opsi harga jual katalog termasuk pajak (*Tax-Inclusive*) atau terpisah (*Tax-Exclusive*).

2. **Parameter Retail & Master Klasifikasi Toko (`/admin/parameters`):**
   - **Satuan Produk (*Units*):** Menambah dan mengelola satuan barang (`PCS`, `PACK`, `BOX`, `KG`, dll.) serta penanda bilangan pecahan (*is_fractional*).
   - **Kategori Produk (*Categories*):** Hierarki kategori dan sub-kategori produk ritel toko.
   - **Merek Produk (*Brands*):** Master daftar nama brand/prinsipal produk.
   - **Pemasok / Vendor (*Suppliers*):** Master data rekanan distributor pemasok barang dagang beserta termin jatuh tempo pembayaran (TOP).
   - **Metode Penilaian HPP Persediaan (*Costing Method*):** Menetapkan metode biaya persediaan (FIFO atau Moving Average / WMA).
   - **Keamanan Biometrik Supervisor:** Konfigurasi pemindai sidik jari (*fingerprint*) USB atau kamera pengenal wajah (*face recognition*) untuk otorisasi supervisor di meja kasir.
   - **Direktori Penyimpanan & Backup:** Konfigurasi path folder lokal penyimpanan berkas cadangan database.

