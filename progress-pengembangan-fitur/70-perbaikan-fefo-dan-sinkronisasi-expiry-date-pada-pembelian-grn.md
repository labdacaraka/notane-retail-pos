# 70. Perbaikan Prioritas FEFO & Sinkronisasi Expiry Date pada Penerimaan Barang (GRN)

## Ringkasan Masalah & Investigasi
Saat pengujian pembelian (GRN) dengan 2 batch barang yang memiliki tanggal expired berbeda pada produk barcode `8992761000493`:
- Batch 1: Nomor Batch `123`, Qty 5, Expired `2026-10-31` (GRN-20261013045142-6574)
- Batch 2: Nomor Batch `124`, Qty 10, Expired `2026-10-15` (GRN-20261013045335-C183)

Saat checkout di POS, sistem justru memotong batch `SALDO-AWAL` / batch yang masuk lebih awal dan bukan batch `124` yang tanggal kedaluwarsanya lebih dekat (`2026-10-15`).

### Akar Penyebab Masalah:
1. **`inventory_service::create_goods_receipt`**: Saat rincian barang GRN diproses, pemanggilan kalkulasi HPP & batch stok memanggil `recalculate_and_apply_cost_on_receipt(...)` yang mengabaikan `expiry_date` dan `batch_number` (selalu mengoper `None` untuk expiry dan nomor GRN untuk batch). Akibatnya, pada tabel `product_stock_batches`, kedua batch tersebut tersimpan dengan `expiry_date = NULL`.
2. **Prioritas Pengurutan FEFO**: Kueri alokasi batch mengurutkan berdasarkan `ORDER BY expiry_date ASC NULLS LAST, received_at ASC, id ASC`. Karena semua batch memiliki `expiry_date = NULL`, pengurutan otomatis beralih ke `received_at ASC` (FIFO tanggal masuk), sehingga batch yang masuk duluan yang terpilih.
3. **Status Costing Method Produk**: Produk warisan seed data memiliki `costing_method = 'WMA'`, sehingga tidak langsung mewarisi setting global `FIFO` yang dipilih pada Pengaturan Parameter.

---

## Solusi Perbaikan

### 1. Backend Service (`inventory_service.rs` & `models/inventory.rs`)
- Pemanggilan rekalkulasi HPP & batch pada GRN kini menggunakan `recalculate_and_apply_cost_on_receipt_ext` dengan mengoper:
  - `actual_batch_num`: Menggunakan `it.batch_number` jika diisi oleh pengguna (misal `124`), atau fallback ke nomor GRN.
  - `source_document_number`: Nomor GRN resmi.
  - `expiry_date`: Tanggal kedaluwarsa resmi dari baris penerimaan barang.
- Pembelian langsung (`create_goods_receipt_invoice_direct_tx`) juga telah diselaraskan dengan menambahkan field `expiry_date` dan `batch_number` pada `GoodsReceiptItemInput`.

### 2. Database Migration (`051_backfill_grn_batches_expiry_and_product_costing.sql`)
- Menjalankan sinkronisasi / backfill data batch yang sudah ada:
  ```sql
  UPDATE product_stock_batches psb
  SET 
      expiry_date = gri.expiry_date,
      batch_number = COALESCE(NULLIF(gri.batch_number, ''), psb.batch_number),
      source_document_number = gr.grn_number
  FROM goods_receipt_items gri
  JOIN goods_receipts gr ON gri.goods_receipt_id = gr.id
  WHERE psb.reference_type = 'PURCHASE_GRN'
    AND psb.reference_id = gr.id
    AND psb.product_id = gri.product_id
    AND (psb.expiry_date IS NULL OR psb.expiry_date != gri.expiry_date);

  UPDATE products SET costing_method = 'GLOBAL' WHERE costing_method = 'WMA';
  ```

---

## Hasil Pengujian & Verifikasi
1. **Urutan Batch FEFO**:
   - Batch `124` (Expired `2026-10-15`): Prioritas #1 (Paling atas)
   - Batch `123` (Expired `2026-10-31`): Prioritas #2
   - Batch `SALDO-AWAL` (Tanpa expired / NULLS LAST): Prioritas #3
2. **Uji Checkout POS**:
   - Transaksi checkout 2 unit produk `8992761000493` (`INV-20261013-B39BCDCF`) berhasil mengeksekusi alokasi **FEFO** dari batch **`124`**.
   - Sisa stok batch `124` berkurang dari `10` menjadi `8`.
   - Riwayat pelacakan POS pada modal Batch mencatat faktur, kasir, kuantitas, dan metode alokasi `FEFO` secara presisi.
