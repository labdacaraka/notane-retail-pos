# Progress Log 05: Laporan Akhir & Hasil Pengujian Sistem (Testing & Verification)

- **Tanggal Penyelesaian:** 2026-09-20
- **Status:** Selesai (Completed)
- **Komponen:** Pengujian End-to-End, Integrasi POS Kasir, Akuntansi Terpadu, Kartu Stok

---

## 1. Ringkasan Eksekusi Pengujian (End-to-End Verification)

Sistem Point of Sales Retail Enterprise **Notane Retail POS** telah selesai dibangun secara menyeluruh dan memenuhi 100% persyaratan yang tertera pada [perencanaan_aplikasi_kasir_retail.md](../perencanaan/perencanaan_aplikasi_kasir_retail.md).

### Skenario Pengujian yang Divalidasi:
1. **Autentikasi & Otorisasi RBAC:**
   - Login kasir operasional (`kasir1`) dan administrator (`admin`) dengan enkripsi kata sandi standar Argon2id.
   - Hak akses pengguna terkendali berbasis matriks permissions di database.
   - Otorisasi Supervisor Langsung (*In-Place PIN*) berhasil memvalidasi tindakan sensitif (*Void Item*, Buka Laci, Diskon) tanpa memaksa kasir keluar dari layar transaksi.

2. **Operasional Kasir (Shift Kasir):**
   - Kasir membuka shift dengan modal awal (*opening cash* Rp 100.000).
   - Penutupan shift kasir menggunakan metode **Blind Settlement**: kasir menginput hitungan uang fisik tunai tanpa mengetahui ekspektasi sistem. Sistem membandingkan selisih (*discrepancy*) dan otomatis mencatat entri jurnal selisih kas (Beban Kurang Kas / Pendapatan Lebih Kas).
   - Pencetakan ringkasan shift (*Z-Report*).

3. **Pemindaian Barcode & Produk Timbangan Elektrik:**
   - Pemindaian instan barcode standar EAN-13 (Indomie Goreng, Susu Ultra).
   - Pemindaian **Weight-Embedded Barcode** (Apel Fuji timbangan elektrik dengan barcode format `2000010012500` -> terdeteksi otomatis sebagai produk timbangan seberat 1.250 kg).
   - Konversi satuan multi-UoM (misal 1 DUS = 40 PCS Indomie Goreng dengan penyesuaian harga dan stok otomatis).

4. **Transaksi Pembayaran & Split Payment:**
   - Dukungan pembayaran campuran (*Split Payment*): Tunai + QRIS / Kartu EDC / Poin Member.
   - Tombol nominal cepat uang pecahan tunai (Uang Pas, +10k, +20k, +50k, +100k) dan kalkulasi kembalian akurat.
   - Preview dan pencetakan struk belanja thermal 80mm standar POS kasir.

5. **Pengendalian Stok Ketat (Inventory Control):**
   - Transaksi checkout secara atomik mengurangi saldo stok di gudang dan merekam mutasi kartu stok (*stock_mutations*).
   - Fitur Stock Opname fisik toko membandingkan stok aktual vs sistem dan membukukan jurnal selisih persediaan (*shrinkage*).
   - Fitur Penerimaan Barang Masuk (GRN) mencatat faktur distributor dan memperbarui harga pokok HPP secara *Weighted Moving Average (WMA)*.

6. **Otomatisasi Pembukuan Double-Entry (General Ledger):**
   - Setiap penjualan berhasil secara otomatis membentuk jurnal akuntansi seimbang (*balanced*):
     - *Debet:* Kas Operasional / Bank QRIS Settlement
     - *Debet:* Beban Pokok Penjualan (HPP / COGS)
     - *Kredit:* Persediaan Barang Dagang
     - *Kredit:* Pendapatan Penjualan Retail POS
     - *Kredit:* Hutang Pajak PPN
   - Penarikan Laporan Laba Rugi (*Income Statement*) real-time yang langsung menyajikan Penjualan Bersih, Total HPP, Laba Kotor, dan Laba Bersih.

7. **Jejak Audit Keamanan 360° (Audit Trail):**
   - Seluruh tindakan sensitif terekam di tabel `audit_logs` lengkap dengan timestamp, terminal, pelaku kasir, supervisor yang mengotorisasi, serta nilai lama vs nilai baru.

---

## 2. Petunjuk Menjalankan Aplikasi

### Menjalankan Database PostgreSQL:
```bash
podman start notane-postgres
```

### Menjalankan Backend Rust Axum:
```bash
cd src-tauri
cargo run -- --server-only
```
*Server API REST & WebSockets akan berjalan di `http://127.0.0.1:8080`.*

### Menjalankan Frontend Vue 3 (Development):
```bash
npm run dev
```
*Aplikasi web kasir dapat diakses di `http://localhost:5175`.*

### Menjalankan Aplikasi Desktop (Tauri v2):
```bash
npm run tauri dev
```
*Membuka window desktop native Notane Retail POS.*

---

## 3. Akun Uji Coba Default
| Peran | Username | Password | PIN Supervisor |
| :--- | :--- | :--- | :--- |
| **Superadmin** | `admin` | `admin123` | `123456` |
| **Supervisor Toko** | `spv` | `spv123` | `123456` |
| **Kasir 1** | `kasir1` | `kasir123` | `111111` |
