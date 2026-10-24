# 69. Sinkronisasi HPP (Cost Price) Berdasarkan Metode Valuasi pada POS, Master Produk, dan Penjualan

## Ringkasan Perubahan
Memastikan penentuan Beban Pokok Penjualan / Harga Pokok Penjualan (HPP / *Cost Price*) saat transaksi checkout POS, barcode lookup kasir, maupun tabel master produk dan modul lainnya tersinkronisasi secara akurat sesuai metode valuasi persediaan yang dipilih (`FIFO`, `FEFO`, `REPLACEMENT_COST`, atau `WMA` / Moving Average).

---

## 1. Perilaku Berdasarkan Metode Valuasi

| Metode HPP | Sumber HPP Aktif (Tampilan / Lookup) | HPP Transaksi Checkout POS | Alokasi Batch Fisik |
|---|---|---|---|
| **FIFO** | HPP batch stok aktif tertua (`received_at ASC`) dengan `remaining_qty > 0`. Fallback ke `pop.cost_price`. | Dihitung dari batch yang terkonsumsi secara FIFO (`fifo_cogs`). | Ya, tercatat di `sale_item_stock_batches`. HPP aktif produk diperbarui ke batch berikutnya. |
| **FEFO** | HPP batch dengan tanggal kedaluwarsa terdekat (`expiry_date ASC NULLS LAST`). Fallback ke `pop.cost_price`. | Dihitung dari batch yang terkonsumsi secara FEFO (`fifo_cogs`). | Ya, tercatat di `sale_item_stock_batches`. HPP aktif produk diperbarui ke batch berikutnya. |
| **REPLACEMENT_COST** | `pop.last_purchase_price` (Harga beli terakhir per-outlet). Fallback ke `pop.cost_price`. | `repl_cost_base * base_qty` (berdasarkan `last_purchase_price`). | Ya, tetap dialokasikan untuk kartu batch fisik. HPP rata-rata/terakhir tidak ditimpa oleh batch satuan. |
| **WMA** (Moving Average) | `pop.cost_price` (Rata-rata tertimbang bergerak per-outlet). | `pop.cost_price * base_qty`. | Ya, tetap dialokasikan untuk kartu batch fisik. Nilai WMA tidak ditimpa oleh batch satuan. |

---

## 2. Sinkronisasi Multi-Satuan (*Conversion Factor*)
Sebelumnya, jika barang dijual dalam satuan bertingkat (misalnya Dus = 10 Pcs):
- `sale_items.unit_cost_price` sempat menyimpan HPP satuan dasar (Pcs) padahal kolom `quantity` menyimpan jumlah satuan jual (Dus). Hal ini menyebabkan nilai `quantity * unit_cost_price` tidak sinkron dengan `line_cost` dan `sales.total_cost_amount`.

**Solusi Perbaikan:**
- `unit_cost_for_item = (effective_cost_price_base * factor).round_dp(4)` disimpan ke `sale_items.unit_cost_price`.
- `stock_mutations.cost_price` tetap menggunakan `effective_cost_price_base` karena kuantitas mutasi dicatat dalam `base_qty`.
- Saat pembatalan / void nota, mutasi `stock_mutations` dikonversi kembali ke basis unit `cost_per_base = (it.unit_cost_price / factor).round_dp(4)`.

---

## 3. Komponen yang Diperbarui

1. **`notane-server/src-tauri/src/services/pos_service.rs`**:
   - `lookup_product`: Mengambil `last_purchase_price`, mengevaluasi metode efektif (`FIFO`/`FEFO`/`REPLACEMENT_COST`/`WMA`), dan menyajikan HPP aktif yang sesuai untuk layar kasir.
   - `checkout`: Mengambil `last_purchase_price`, menghitung `line_cost`, `effective_cost_price_base`, dan `unit_cost_for_item` sesuai metode produk/global, serta menyimpan pemetaan batch ke `sale_item_stock_batches`.
   - `void_sale`: Menyelaraskan harga modal satuan dasar pada mutasi pengembalian stok saat nota dibatalkan.

2. **`notane-server/src-tauri/src/services/costing_service.rs`**:
   - `allocate_and_consume_stock_batches`: Update HPP aktif pada `product_outlet_prices` dibatasi hanya saat metode adalah `FIFO` atau `FEFO`.

3. **`notane-server/src-tauri/src/api/master.rs`**:
   - `list_products_handler`: Menghitung ekspresi `cost_col` secara dinamis sesuai metode efektif per produk dan konfigurasi global agar seluruh halaman (daftar produk, modal, dropdown belanja, dan stock opname) menampilkan HPP yang sinkron.
