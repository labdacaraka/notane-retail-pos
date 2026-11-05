# PANDUAN PENGGUNAAN SISTEM NOTANE RETAIL POS
**Solusi Point of Sale & Back-Office Enterprise Retail Modern**

Selamat datang di dokumentasi resmi panduan operasional **Notane Retail POS**. Panduan ini disusun secara sistematis per bab dan per peran (*Role-Based Access Control*) untuk memandu seluruh staf operasional toko—mulai dari Kasir, Supervisor, Manajer, Gudang, Akuntansi, hingga Administrator Utama.

---

## 📚 DAFTAR ISI PANDUAN

| No | Modul / Peran (*Role*) | Dokumen Panduan | Cakupan Tanggung Jawab Utama |
|:---:|---|---|---|
| **00** | **Notane Retail Server** | [00_PANDUAN_NOTANE_SERVER.md](./00_PANDUAN_NOTANE_SERVER.md) | Pusat data toko, status server, pairing code, backup/restore, dan pembaruan sistem. |
| **01** | **Kasir (CASHIER)** | [01_ROLE_CASHIER.md](./01_ROLE_CASHIER.md) | Buka shift, scan barcode, hold/recall, transaksi multi-pembayaran, cetak struk, dan tutup shift. |
| **02** | **Supervisor (SUPERVISOR)** | [02_ROLE_SUPERVISOR.md](./02_ROLE_SUPERVISOR.md) | Otorisasi void kasir, diskon manual, buka laci kasir, approval retur, opname harian, dan cash drop. |
| **03** | **Asisten Manajer (ASSISTANT_MANAGER)** | [03_ROLE_ASSISTANT_MANAGER.md](./03_ROLE_ASSISTANT_MANAGER.md) | Supervisi shift toko, penerimaan barang (GRN), transfer stok cabang, dan reset PIN kasir. |
| **04** | **Kepala Toko (STORE_MANAGER)** | [04_ROLE_STORE_MANAGER.md](./04_ROLE_STORE_MANAGER.md) | Dasbor analitik bisnis, multi-harga outlet, flash sale, tutup buku periode, dan audit operasional. |
| **05** | **Gudang & Logistik (WAREHOUSE)** | [05_ROLE_WAREHOUSE.md](./05_ROLE_WAREHOUSE.md) | Penerimaan barang GRN, kartu stok, pelacakan batch FEFO (kedaluwarsa), dan cetak label barcode. |
| **06** | **Pengadaan (PURCHASING)** | [06_ROLE_PURCHASING.md](./06_ROLE_PURCHASING.md) | Pembuatan pesanan pembelian (PO), faktur supplier, kontrol utang dagang, dan retur pembelian. |
| **07** | **Akuntansi & Keuangan (ACCOUNTING)** | [07_ROLE_ACCOUNTING.md](./07_ROLE_ACCOUNTING.md) | Bagan akun (COA), jurnal umum, mutasi kas/bank, biaya operasional, dan tutup buku akuntansi. |
| **08** | **Auditor Internal (INTERNAL_AUDIT)** | [08_ROLE_INTERNAL_AUDIT.md](./08_ROLE_INTERNAL_AUDIT.md) | Penelusuran jejak audit (Audit Logs), investigasi void kasir, selisih stok (*shrinkage*), dan margin laba. |
| **09** | **Merchandiser (MERCHANDISER)** | [09_ROLE_MERCHANDISER.md](./09_ROLE_MERCHANDISER.md) | Master produk & varian, banner layar pelanggan (*Customer Display*), voucher kupon, dan member. |
| **10** | **Super Admin (SUPERADMIN)** | [10_ROLE_SUPERADMIN.md](./10_ROLE_SUPERADMIN.md) | Manajemen staf pengguna, matriks hak akses peran (RBAC), pengaturan global, dan pemulihan data. |
| **ALL** | **Kamus Fitur Seluruh Menu** | [PANDUAN_FITUR_BERDASARKAN_MENU.md](./PANDUAN_FITUR_BERDASARKAN_MENU.md) | Panduan lengkap eksplorasi seluruh tombol, aksi, dan kapabilitas sistem berdasarkan menu aplikasi. |

---

## 🏛️ ARSITEKTUR CLIENT-SERVER RETAIL MODERN

Sistem Notane Retail beroperasi menggunakan arsitektur *Zero-Trust Local Server*:
1. **Notane Retail Server (Komputer Utama / Server Toko):**
   - Menjalankan basis data PostgreSQL terenkripsi di jaringan lokal toko (LAN/Wi-Fi).
   - Menyediakan API terpusat untuk sinkronisasi harga, stok, kartu member, dan pembukuan.
   - Tidak membutuhkan koneksi internet stabil (100% *Offline-First*).
2. **Notane Retail POS Client (Komputer Kasir / Terminal Back-Office):**
   - Dipasang di setiap meja kasir atau laptop back-office manajer.
   - Terhubung secara aman ke Notane Server menggunakan **Kode Pairing Toko**.
   - Dilengkapi penyimpanan lokal (*IndexedDB cache*) sehingga kasir tetap bisa melayani transaksi belanja meskipun kabel LAN terlepas.

---

## 🔑 DAFTAR AKUN DEFAULT UNTUK PELATIHAN

Seluruh pengguna bawaan (*default seed*) dikonfigurasi dengan kredensial standar awal:

| Username | Role | Nama Petugas | Password Awal | PIN Otorisasi |
|---|---|---|:---:|:---:|
| `admin` | **SUPERADMIN** | Administrator Utama | `admin123` | `123456` |
| `manager1` | **STORE_MANAGER** | Eko Prasetyo | `admin123` | `123456` |
| `asst_mgr` | **ASSISTANT_MANAGER** | Budi Santoso | `admin123` | `123456` |
| `spv` | **SUPERVISOR** | Supervisor Toko | `admin123` | `123456` |
| `kasir1` | **CASHIER** | Siti Kasir Utama | `admin123` | `111111` |
| `gudang1` | **WAREHOUSE** | Agus Pratama | `admin123` | `123456` |
| `buyer1` | **PURCHASING** | Dewi Lestari | `admin123` | `123456` |
| `akunting1` | **ACCOUNTING** | Fajar Nugraha | `admin123` | `123456` |
| `audit1` | **INTERNAL_AUDIT** | Hendro Wibowo | `admin123` | `123456` |
| `merch1` | **MERCHANDISER** | Rina Melati | `admin123` | `123456` |

> [!IMPORTANT]
> Segera ubah password dan PIN default setelah sistem pertama kali dipasang di toko operasional melalui menu **Pengguna & Hak Akses**.

