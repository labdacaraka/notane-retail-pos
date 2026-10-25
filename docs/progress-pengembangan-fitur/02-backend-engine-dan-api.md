# Progress Log 02: Backend Core Engine & REST / WebSocket API (Rust Axum)

- **Tanggal Penyelesaian:** 2026-09-20
- **Status:** Selesai (Completed)
- **Komponen:** Rust Backend, Axum HTTP Server, Tokio Async, SQLx PostgreSQL, Argon2id, JWT, WebSockets

---

## 1. Ringkasan Fitur Backend yang Telah Selesai

### A. Keamanan & Autentikasi (`/api/v1/auth`)
- **Argon2id Hashing & Verification:** Implementasi modul hashing standar industri di `src-tauri/src/utils/hash.rs` untuk keamanan kata sandi pengguna dan PIN otorisasi supervisor.
- **JWT Middleware & Token Management (`utils/jwt.rs`):** Menghasilkan dan memvalidasi JWT token yang memuat identitas pengguna, peran (RBAC), dan ID terminal kasir.
- **In-Place Supervisor PIN Approval (`POST /api/v1/auth/supervisor-approve`):** Memverifikasi PIN supervisor secara instan tanpa memaksa kasir logout saat melakukan tindakan sensitif (Void, Diskon manual, Buka Laci).
- **Audit Trail 360° Service (`services/audit_service.rs`):** Rekam jejak seluruh aksi kritis dengan payload lama vs payload baru di tabel `audit_logs`.

### B. Modul Operasional Kasir & Shift (`/api/v1/shifts` & `/api/v1/sales`)
- **Buka Shift (`POST /api/v1/shifts/open`):** Mencatat terminal kasir aktif, jam buka, dan modal awal (*opening cash/floating cash*).
- **Tutup Shift / Blind Settlement (`POST /api/v1/shifts/close`):** Kasir menginput hitungan fisik uang tunai di laci (*blind count*). Sistem menghitung komparasi dengan hitungan sistem, menghitung *discrepancy* (selisih lebih/kurang kas), dan **otomatis membentuk jurnal akuntansi selisih kas** (1101 Kas vs 4201 Pendapatan Lebih Kas / 5201 Beban Kurang Kas).
- **Pencarian Produk Cepat (`GET /api/v1/products/barcode/:code`):** Latensi rendah, mendukung barcode standar EAN-13, SKU, dan **Weight-Embedded Barcode parser** (`src-tauri/src/utils/barcode.rs`) untuk produk timbangan buah/sayur/daging elektrik.
- **Atomic Checkout POS (`POST /api/v1/sales/checkout`) [VITAL ENGINE]:**
  1. Dibungkus dalam satu **PostgreSQL Database Transaction**.
  2. Mengunci baris stok gudang (`FOR UPDATE`).
  3. Memotong kuantitas stok di `warehouse_stocks`.
  4. Mencatat riwayat mutasi kartu stok ketat di `stock_mutations` dengan referensi `SALES`.
  5. Menghitung dan mengakumulasi poin loyalitas pelanggan (1 poin per Rp 10.000) dan mencatat mutasi di `customer_point_ledgers`.
  6. Memvalidasi dan menerapkan voucher potongan diskon.
  7. Menyimpan nota transaksi ke `sales`, `sale_items`, dan `sale_payments` (dukungan Split Payment: Tunai, QRIS, EDC, Poin).
  8. **Otomatisasi Penjurnalan Double-Entry Seimbang (`journal_entries` & `journal_lines`):**
     - *Debet:* Kas Operasional Kasir (1101) / Bank QRIS (1102)
     - *Debet:* Beban Pokok Penjualan HPP (5101)
     - *Kredit:* Persediaan Barang Dagang (1201)
     - *Kredit:* Pendapatan Penjualan Retail (4101)
     - *Kredit:* Hutang Pajak PPN (2102) jika ada pajak
- **Pembatalan Nota / Void (`POST /api/v1/sales/:id/void`):** Memvalidasi PIN supervisor, memulihkan stok gudang, membalik mutasi stok (*RETURN_IN*), membalik poin pelanggan, dan membentuk **Jurnal Pembalik (Reversal Journal)**.

### C. Modul Inventori & Pengadaan (`/api/v1/inventory`)
- **Penerimaan Faktur Barang Masuk (GRN) (`POST /api/v1/inventory/grn`):** Menambah saldo stok gudang, mencatat mutasi `PURCHASE_GRN`, memperbarui harga beli rata-rata tertimbang (*Weighted Moving Average - WMA*), dan mencatat jurnal Persediaan (1201) vs Hutang Usaha Supplier (2101).
- **Stock Opname / Penyesuaian Fisik (`POST /api/v1/inventory/adjustments`):** Mengkalkulasi selisih stok sistem vs fisik aktual, memperbarui stok, dan mencatat jurnal Beban Selisih Persediaan (5301).
- **Kartu Stok Real-Time (`GET /api/v1/inventory/stock-ledger/:product_id`):** Melacak jejak mutasi kuantitas masuk, keluar, dan saldo berjalan produk.

### D. Modul Akuntansi & Pelaporan Keuangan (`/api/v1/reports` & `/api/v1/accounting`)
- **Laporan Laba Rugi Real-Time (`GET /api/v1/reports/income-statement`):** Mengkalkulasi Pendapatan Bersih, HPP (COGS), Laba Kotor, Beban Operasional, dan Laba Bersih secara real-time dari data jurnal General Ledger.
- **Laporan Z-Report Shift Kasir (`GET /api/v1/reports/shift-z-report/:shift_id`):** Ringkasan transaksi, pembagian metode pembayaran (Tunai, QRIS, EDC), modal kasir, expected cash, dan selisih kas.
- **Buku Jurnal Umum & Bagan Akun (COA):** Endpoint untuk monitoring jurnal dan akun akuntansi.

### E. Integrasi Real-Time LAN (`/ws/v1/events`)
- Handler WebSocket dua arah menggunakan broadcast channel Tokio untuk sinkronisasi instan perubahan stok, harga produk, dan status shift antar mesin kasir.
