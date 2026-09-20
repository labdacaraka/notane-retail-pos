# Progress Log 01: Arsitektur Sistem & Skema Database PostgreSQL

- **Tanggal Penyelesaian:** 2026-09-20
- **Status:** Selesai (Completed)
- **Komponen:** Arsitektur Monorepo, Database DDL Schema, Seeder Awal, Toolchain

---

## 1. Ringkasan Pekerjaan
Telah diinisialisasi struktur monorepo standar enterprise retail sesuai spesifikasi dokumen perencanaan [perencanaan_aplikasi_kasir_retail.md](../perencanaan/perencanaan_aplikasi_kasir_retail.md):
- **Frontend:** Vue 3, TypeScript, Vite, Pinia, Tailwind CSS, Lucide Icons.
- **Backend:** Rust, Tauri v2, Axum, SQLx, Tokio, Argon2id, JWT.
- **Database:** PostgreSQL dengan skema DDL 25+ tabel terintegrasi dan sistem migrasi otomatis (*embedded migrations*).

---

## 2. Rincian Skema Database (25+ Tabel Terpadu)
File migrasi telah disusun dan diintegrasikan secara terpusat:
- **`src-tauri/migrations/001_initial_schema.sql`**:
  1. *Keamanan & Peran:* `roles`, `permissions`, `role_permissions`, `users`, `audit_logs`, trigger otomatis `update_timestamp_column()`.
  2. *Master Data & Produk:* `brands`, `payment_methods`, `reason_codes`, `outlets`, `categories`, `units`, `products`, `product_units` (konversi bertingkat PCS/BOX/DUS), `product_barcodes` (multi-barcode per SKU), `product_price_tiers` (grosir bertingkat).
  3. *Inventori & Gudang:* `warehouses`, `warehouse_stocks`, `stock_batches` (FIFO/LIFO), `stock_mutations` (kartu stok ketat), `stock_adjustments`, `stock_adjustment_items`.
  4. *Pengadaan (Purchasing):* `suppliers`, `purchase_orders`, `purchase_order_items`, `purchase_invoices` (GRN), `purchase_invoice_items`.
  5. *POS & Pelanggan:* `customer_groups`, `customer_group_prices`, `customers`, `customer_point_ledgers`, `vouchers`, `cashier_shifts`, `sales`, `sale_items`, `sale_payments`.
  6. *Akuntansi Double-Entry:* `account_categories`, `chart_of_accounts`, `journal_entries` (dengan check constraint `total_debit = total_credit`), `journal_lines` (check constraint debit/kredit valid).

- **`src-tauri/migrations/002_seed_data.sql`**:
  1. Peran standar: `SUPERADMIN`, `STORE_MANAGER`, `SUPERVISOR`, `CASHIER`, `WAREHOUSE`.
  2. Permissions lengkap: `pos:checkout`, `pos:void`, `pos:discount`, `shift:open`, `shift:close`, `inventory:read`, `inventory:adjust`, `accounting:read`, dll.
  3. Akun default: `admin` (pass: `admin123`, PIN: `123456`), `spv` (pass: `spv123`, PIN: `123456`), `kasir1` (pass: `kasir123`, PIN: `111111`).
  4. Master outlet `OUT-001` & gudang `WH-001`.
  5. Bagan Akun Retail: 1101 (Kas Toko), 1102 (Bank & QRIS), 1201 (Persediaan), 2101 (Hutang Dagang), 2102 (PPN), 4101 (Penjualan), 5101 (HPP), 5201/4201 (Selisih Kas), 5301 (Selisih Persediaan).
  6. Demo Produk retail: Indomie Goreng (dengan multi-satuan DUS=40 PCS dan tier price), Susu Ultra Cokelat, Apel Fuji Segar (barcode timbangan elektrik `200001000000`), dan Voucher `HEMAT10K`.
