# Progress 66: Perbaikan Fitur Auto Reorder Stok Menipis pada Modul Pembelian (Purchase)

## Latar Belakang & Masalah
Saat pengguna membuat Purchase Order baru di halaman Pembelian (`PurchasesPage.vue`) dan menekan tombol **"Tarik Stok Menipis (Auto Reorder)"**, sistem menampilkan error:
```
Database error: error returned from database: column p.primary_supplier_id does not exist
```
Hal ini terjadi karena:
1. Kolom `primary_supplier_id` belum terdaftar pada skema tabel `products` di database PostgreSQL.
2. Query SQL di `get_reorder_suggestions_handler` (`notane-server/src-tauri/src/api/inventory.rs`) merujuk ke kolom `p.primary_supplier_id` dan `p.purchase_price` (yang seharusnya `p.cost_price` dan `product_outlet_prices.last_purchase_price`).
3. Query belum mendukung penentuan stok dan harga per outlet efektif secara otomatis dari konteks sesi/shift pengguna.

## Langkah Perbaikan yang Diterapkan

### 1. Database Migration 047
- Dibuat migrasi baru: [`047_add_primary_supplier_to_products.sql`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/notane-server/src-tauri/migrations/047_add_primary_supplier_to_products.sql)
  - Menambahkan kolom `primary_supplier_id UUID REFERENCES suppliers(id) ON DELETE SET NULL` ke tabel `products`.
  - Menambahkan indeks `idx_products_primary_supplier_id`.
  - Secara cerdas mengisi nilai awal `primary_supplier_id` produk dari riwayat transaksi pembelian (`purchase_orders`) sebelumnya jika sudah pernah ada.
- Mendaftarkan migrasi 047 ke daftar migrasi terintegrasi di [`notane-server/src-tauri/src/db/mod.rs`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/notane-server/src-tauri/src/db/mod.rs).

### 2. Backend Handler (`inventory.rs`)
- Memperbarui `get_reorder_suggestions_handler` di [`notane-server/src-tauri/src/api/inventory.rs`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/notane-server/src-tauri/src/api/inventory.rs):
  - Menggunakan `resolve_effective_outlet_id(&pool, &headers, query.outlet_id).await` agar perhitungan sisa stok gudang (`warehouse_stocks`) dan harga beli terakhir per outlet (`product_outlet_prices`) akurat sesuai outlet yang dituju atau outlet aktif.
  - Memperbaiki kolom harga beli menggunakan fallback `COALESCE(NULLIF(pop.last_purchase_price, 0), NULLIF(pop.cost_price, 0), p.cost_price, 0) as purchase_price`.
  - Memperbaiki logika pencocokan supplier: jika filter supplier disediakan, sistem akan menyertakan produk yang:
    - Memiliki `p.primary_supplier_id` yang sesuai, ATAU
    - Pernah dipesan dari supplier tersebut pada riwayat PO, ATAU
    - Belum pernah memiliki supplier atau riwayat PO manapun (sehingga user tetap bisa mereorder produk stok menipis tersebut ke supplier yang sedang dipilih).

### 3. Frontend (`PurchasesPage.vue`)
- Memperbarui `pullReorderSuggestions()` di [`notane-client/src/pages/PurchasesPage.vue`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/notane-client/src/pages/PurchasesPage.vue):
  - Mengirimkan parameter `outlet_id: poForm.value.outlet_id || undefined` jika outlet pemesan telah dipilih di formulir PO, sehingga perhitungan stok menipis terisolasi pada gudang outlet tersebut.

## Pengujian & Verifikasi
- Migrasi 047 berhasil dieksekusi saat server dijalankan (`Selesai mengeksekusi 1 migrasi baru`).
- Pemanggilan endpoint `GET /api/v1/inventory/reorder-suggestions` via cURL mengembalikan daftar produk dengan stok menipis (`suggested_qty`, `current_stock`, `min_stock`, `purchase_price`) dengan status `success: true`.
- Build backend (`cargo check`, `cargo build`) dan frontend (`npm run build`) berhasil 100% tanpa error.
- Server daemon aktif berjalan di port 18080.
