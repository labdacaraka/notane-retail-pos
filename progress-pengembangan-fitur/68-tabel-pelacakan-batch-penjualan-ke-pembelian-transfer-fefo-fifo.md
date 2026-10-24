# Progress Pengembangan Fitur 68: Tabel Pemetaan Penjualan ke Pembelian/Transfer Stok & Pelacakan FEFO/FIFO

## 1. Ringkasan Kebutuhan
Menyediakan sistem pelacakan (traceability) dan pemetaan granular untuk setiap item yang dijual/di-checkout di POS terhadap sumber fisik kedatangannya, baik dari Pembelian (GRN / Faktur Pembelian) maupun Transfer Stok antar outlet. Hal ini memastikan metode HPP dan alokasi stok **FEFO** (*First Expired, First Out*) dan **FIFO** (*First In, First Out*) terkontrol, terdokumentasi, dan dapat diaudit secara akurat.

---

## 2. Struktur Database (Migration 048)
File migrasi: `notane-server/src-tauri/migrations/048_sale_item_stock_batches_and_fefo_tracking.sql`

### A. Penyempurnaan `product_stock_batches`
- Ditambahkan kolom `expiry_date DATE` (opsional untuk barang dengan kedaluwarsa).
- Ditambahkan kolom `source_document_number VARCHAR(100)` (nomor PO, GRN, No Faktur Pembelian, atau No Transfer Stok).
- Indeks prioritas FEFO/FIFO:
  ```sql
  CREATE INDEX idx_stock_batches_fefo_priority 
  ON product_stock_batches (product_id, outlet_id, expiry_date ASC NULLS LAST, received_at ASC, id ASC)
  WHERE remaining_qty > 0;
  ```

### B. Tabel Baru `sale_item_stock_batches`
Menyimpan rincian pecahan alokasi batch yang terpotong untuk setiap item transaksi:
- `id UUID PRIMARY KEY DEFAULT gen_random_uuid()`
- `sale_id UUID NOT NULL REFERENCES sales(id) ON DELETE CASCADE`
- `sale_item_id UUID NOT NULL REFERENCES sale_items(id) ON DELETE CASCADE`
- `product_id UUID NOT NULL REFERENCES products(id) ON DELETE CASCADE`
- `outlet_id UUID NOT NULL REFERENCES outlets(id) ON DELETE CASCADE`
- `batch_id UUID REFERENCES product_stock_batches(id) ON DELETE SET NULL`
- `batch_number VARCHAR(100) NOT NULL`
- `quantity NUMERIC(14, 4) NOT NULL`
- `cost_price NUMERIC(18, 4) NOT NULL`
- `subtotal_cost NUMERIC(18, 4) NOT NULL`
- `source_reference_type VARCHAR(50) NOT NULL` (`PURCHASE_GRN`, `PURCHASE_INVOICE`, `TRANSFER`, `INITIAL_STOCK`, `STOCK_ADJUSTMENT`)
- `source_reference_id UUID`
- `source_document_number VARCHAR(100)`
- `expiry_date DATE`
- `allocation_method VARCHAR(20) NOT NULL` (`FEFO`, `FIFO`, `FALLBACK`)
- `created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP`

### C. Registrasi Hak Akses (RBAC)
- Permission `inventory:batch_tracking` didaftarkan di tabel `permissions`.
- Diberikan secara otomatis kepada role `SUPERADMIN` dan `SUPERVISOR`.

---

## 3. Logika Backend & Services

1. **`costing_service.rs`:**
   - **Penerimaan Barang (`recalculate_and_apply_cost_on_receipt_ext`)**: Selalu mencatat batch fisik di `product_stock_batches` terlepas dari metode HPP yang dipilih (FIFO, WMA, atau Replacement Cost), sehingga jejak fisik batch barang selalu tercatat.
   - **Alokasi Batch (`allocate_and_consume_stock_batches`)**:
     - Mengambil batch aktif dengan urutan `expiry_date ASC NULLS LAST, received_at ASC, id ASC`.
     - Jika memiliki `expiry_date`, ditandai sebagai metode alokasi `FEFO`. Jika tidak, `FIFO`.
     - Mengembalikan `Vec<BatchDeductionDetail>` berisi rincian kuantitas, harga pokok per pecahan batch, dan referensi dokumen asal.
     - `consume_fifo_batches` dijadikan wrapper backward-compatible.

2. **`pos_service.rs`:**
   - **Checkout POS**:
     - Memotong batch fisik secara atomik dalam transaksi `tx`.
     - Menyimpan rincian alokasi potongan stok ke tabel `sale_item_stock_batches`.
   - **Void Sale (Pembatalan Transaksi)**:
     - Mengembalikan `remaining_qty` secara otomatis pada masing-masing baris batch di `product_stock_batches` sesuai catatan di `sale_item_stock_batches`.
   - **Audit API**:
     - Endpoint `GET /api/v1/sales/{id}/stock-batches` untuk melihat seluruh alokasi batch stok pada nota.
     - Respons `GET /api/v1/sales/{id}` diperkaya dengan array `stock_batches`.

---

## 4. Hasil Pengujian Verifikasi
1. **Pemeriksaan Skema Database**: Tabel `sale_item_stock_batches` dan indeks `idx_stock_batches_fefo_priority` berhasil dibuat di database PostgreSQL port 15432.
2. **Kompilasi Rust**: `cargo check` dan `cargo build` berhasil 100% tanpa error.
3. **Simulasi Checkout**:
   - Transaksi nota `INV-20261013-5DE6A6F9` berhasil memotong 2 unit dari batch `SALDO-AWAL`.
   - Tabel `sale_item_stock_batches` terisi secara otomatis dengan data lengkap (`batch_id`, `quantity = 2`, `cost_price = 5500`, `allocation_method = FIFO`).
   - Sisa batch `remaining_qty` berkurang dari 78 menjadi 76.
4. **Simulasi Pembatalan (Void)**:
   - Dilakukan void dengan supervisor PIN `123456`.
   - Kuantitas batch `remaining_qty` otomatis kembali utuh menjadi 78.
