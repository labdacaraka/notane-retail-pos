# Fitur: Metode Perhitungan HPP Dinamis & Pelacakan Batch Stok per Outlet

## 1. Latar Belakang & Kebutuhan Bisnis
Dalam operasional bisnis ritel multi-outlet:
1. Harga Beli Pokok (HPP / `cost_price`) tidak bersifat statis dan memiliki nilai yang berbeda di setiap outlet atau gudang tergantung biaya logistik, supplier lokal, serta fluktuasi harga kulakan.
2. Setiap pembelian / kulakan baru dapat memiliki harga beli yang lebih murah atau lebih mahal daripada stok lama yang masih tersimpan di rak atau gudang.
3. Bisnis membutuhkan fleksibilitas metode penilaian persediaan yang dapat diatur secara terpusat (Global) maupun dikhususkan per produk:
   - **Rata-rata Tertimbang (Weighted Moving Average / WMA)**: Menggabungkan nilai sisa stok lama dengan stok masuk baru untuk menghitung HPP rata-rata proporsional.
   - **FIFO (First In, First Out)**: Mencatat setiap kulakan ke dalam *batch lot* tersendiri dan mengonsumsi batch tertua saat penjualan kasir/POS untuk menghitung HPP riil per transaksi.
   - **Harga Ganti (Replacement Cost)**: Menggunakan harga kulakan terbaru sebagai HPP aktif secara langsung, dengan opsi otomatis memperbarui harga jual dasar (`base_selling_price`) berdasarkan target margin/markup keuntungan.

---

## 2. Arsitektur & Skema Database

### A. Migrasi Database
1. **`031_product_costing_methods_and_batches.sql`**:
   - Menambahkan kolom `auto_adjust_selling_price BOOLEAN NOT NULL DEFAULT FALSE` dan `markup_percent NUMERIC(8, 2)` pada tabel `products`.
   - Menambahkan kolom `last_purchase_price NUMERIC(18, 4)` pada tabel `product_outlet_prices`.
   - Membuat tabel `product_stock_batches`:
     - `id UUID PRIMARY KEY DEFAULT gen_random_uuid()`
     - `product_id UUID REFERENCES products(id)`
     - `outlet_id UUID REFERENCES outlets(id)`
     - `warehouse_id UUID REFERENCES warehouses(id)`
     - `batch_no VARCHAR(100) NOT NULL`
     - `initial_qty NUMERIC(18, 4) NOT NULL`
     - `remaining_qty NUMERIC(18, 4) NOT NULL`
     - `unit_cost_price NUMERIC(18, 4) NOT NULL`
     - `received_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP`
     - `expiry_date DATE`
     - `source_doc_type VARCHAR(50)` (GRN, PO, INITIAL_STOCK, TRANSFER_IN, dll.)
     - `source_doc_id UUID`
     - `status VARCHAR(20) DEFAULT 'ACTIVE'` ('ACTIVE', 'DEPLETED', 'EXPIRED')
   - Indexing: `idx_stock_batches_fifo` pada `(product_id, outlet_id, received_at ASC, id ASC)` dan index parsial untuk status aktif.
   - Seed otomatis batch awal dari `warehouse_stocks` yang sudah ada sebelumnya.

2. **`032_inventory_costing_settings_and_global_fallback.sql`**:
   - Menambahkan tabel konfigurasi global `inventory_costing_settings`:
     - `default_costing_method VARCHAR(20) NOT NULL DEFAULT 'WMA'` ('WMA', 'FIFO', 'REPLACEMENT_COST')
     - `auto_adjust_selling_price_on_replacement BOOLEAN NOT NULL DEFAULT FALSE`
     - `default_markup_percent NUMERIC(8, 2) DEFAULT 25.00`
   - Memperbarui `costing_method` pada tabel `products` agar mendukung nilai `'GLOBAL'` (mengikuti setting toko pusat).

---

## 3. Implementasi Backend (Rust & Tauri)

### A. Layanan Costing (`src-tauri/src/services/costing_service.rs`)
1. **`resolve_effective_costing_method`**:
   - Memeriksa metode HPP produk. Jika nilainya `'GLOBAL'`, maka mengambil metode default dari `inventory_costing_settings`.
2. **`recalculate_and_apply_cost_on_receipt`**:
   - Dipanggil saat penerimaan barang (Goods Receipt Note, Penerimaan Pembelian, atau Transfer Masuk).
   - **WMA**:
     $$\text{HPP Baru} = \frac{(\text{Sisa Stok Lama} \times \text{HPP Lama}) + (\text{Qty Masuk} \times \text{Harga Beli Baru})}{\text{Sisa Stok Lama} + \text{Qty Masuk}}$$
   - **FIFO**:
     - Membuat record batch lot baru di `product_stock_batches`.
     - Mengatur HPP aktif outlet ke harga unit batch tertua yang masih memiliki `remaining_qty > 0`.
   - **Replacement Cost**:
     - Mengubah HPP aktif outlet langsung menjadi `incoming_unit_cost`.
     - Jika fitur `auto_adjust_selling_price` aktif, harga jual dasar dihitung ulang:
       - Menggunakan `markup_percent` yang ditentukan: $\text{Harga Jual Baru} = \text{HPP Baru} \times (1 + \frac{\text{Markup}}{100})$.
       - Atau mempertahankan rasio profit margin sebelumnya jika markup tidak ditentukan.
   - Menyimpan hasil HPP ke `product_outlet_prices` (spesifik outlet) dan memperbarui `products.cost_price` sebagai fallback global.
3. **`consume_fifo_batches`**:
   - Dipanggil saat transaksi penjualan di kasir/POS (`pos_service.rs`).
   - Melakukan locking baris batch (`SELECT ... FOR UPDATE ORDER BY received_at ASC, id ASC`) pada batch aktif produk di outlet terkait.
   - Mengurangi `remaining_qty` secara berurutan hingga kuantitas terjual terpenuhi.
   - Mengembalikan total HPP riil transaksi dan nilai HPP unit tertimbang dari batch yang terpakai.
   - Mengubah status batch menjadi `'DEPLETED'` jika `remaining_qty <= 0`.
   - Memperbarui HPP aktif di `product_outlet_prices` ke harga batch tertua berikutnya.

### B. Integrasi Endpoint & Master API
1. **Pengaturan Global HPP (`src-tauri/src/api/master.rs`)**:
   - `GET /api/v1/settings/inventory-costing`: Mengambil konfigurasi metode HPP toko.
   - `PUT /api/v1/settings/inventory-costing`: Menyimpan konfigurasi metode HPP toko (memerlukan izin `master:write`).
2. **Batch Stok Produk**:
   - `GET /api/v1/products/{id}/batches`: Menampilkan daftar riwayat dan status batch lot produk per outlet.
3. **Penerimaan Barang**:
   - Diintegrasikan ke alur penerimaan langsung (`process_goods_receipt`), alur PO (`create_goods_receipt`), dan transfer stok (`receive_stock_transfer`) di `src-tauri/src/services/inventory_service.rs`.
4. **Penjualan Kasir POS**:
   - Diintegrasikan ke alur checkout POS di `src-tauri/src/services/pos_service.rs` untuk menghitung `sale_items.unit_cost_price`, `stock_mutations`, dan jurnal pembukuan HPP persediaan.

---

## 4. Implementasi Frontend (Vue 3 + TypeScript)

### A. Pengaturan HPP di Master Parameter (`src/pages/ParametersPage.vue`)
- Tab khusus: **Metode HPP (Costing)**.
- Pilihan metode default toko:
  1. *Rata-rata Tertimbang (WMA)*.
  2. *FIFO (First In, First Out)*.
  3. *Harga Ganti (Replacement Cost)*.
- Konfigurasi default penyesuaian harga jual otomatis dan target markup persen jika metode *Replacement Cost* dipilih.
- Menyimpan pengaturan dengan validasi dan hak akses `master:write`.

### B. Manajemen Produk (`src/pages/ProductsPage.vue`)
- **Tampilan Daftar Produk**:
  - Kolom HPP dilengkapi badge status metode biaya:
    - `Global (WMA / FIFO / Harga Ganti)` dengan ikon bola dunia, menandakan produk mengikuti pengaturan terpusat.
    - `Khusus: FIFO`, `Khusus: Harga Ganti`, atau `Khusus: Rata-rata` jika produk memiliki konfigurasi tersendiri.
  - Tombol **"Batch"** di setiap baris produk untuk membuka modal riwayat batch lot stok (nomor batch, tanggal masuk, outlet, sisa stok, HPP unit, status).
- **Header Toolbar**:
  - Tombol shortcut indikator: **HPP Global: [Metode Aktif]** yang mengarahkan langsung ke halaman Master Parameter.
- **Modal Tambah & Edit Produk**:
  - Pilihan metode perhitungan HPP:
    - *Ikuti Pengaturan Global (Standar Toko)*
    - *Khusus Produk Ini: Rata-rata Tertimbang*
    - *Khusus Produk Ini: FIFO*
    - *Khusus Produk Ini: Harga Ganti*
  - Opsi dinamis untuk penyesuaian harga jual otomatis dan persentase markup keuntungan.

---

## 5. Verifikasi & Pengujian
- **Pemeriksaan Tipe TypeScript (`vue-tsc --noEmit`)**: Lulus tanpa error (Exit code: 0).
- **Pemeriksaan Kompilasi Rust (`cargo check`)**: Lulus tanpa error/warning fatal (Exit code: 0).
- **Integritas Keamanan & Arsitektur**:
  - Validasi schema di Rust backend dengan zero-panic guarantee (`Result<T, E>`).
  - Hak akses otorisasi (`master:write` dan session guard) diterapkan pada semua mutasi data biaya dan parameter sistem.
