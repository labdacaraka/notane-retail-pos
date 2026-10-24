# Dokumentasi Fitur: Validasi Maksimal Stok POS (Frontend & Backend)

## 1. Ringkasan Permasalahan
Sebelumnya, sistem kasir POS tidak memiliki batasan kuantitas maksimal terhadap stok fisik yang tersedia di outlet/gudang:
- Kasir dapat memindai atau mengetik kuantitas melebihi sisa stok riil.
- Backend langsung melakukan pemotongan stok `quantity = current_stock - qty` tanpa validasi, sehingga kolom `quantity` pada tabel `warehouse_stocks` menjadi **negatif (minus)**.

---

## 2. Solusi Defense-in-Depth

### A. Backend Core (Rust / Axum)
Diimplementasikan pada [`src-tauri/src/services/pos_service.rs`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src-tauri/src/services/pos_service.rs):

1. **Pre-flight Stock Aggregation Validation**:
   - Sebelum transaksi penjualan dibuat atau dicatat ke buku besar:
     - Sistem mengagregasi total kebutuhan stok (`item.quantity * conversion_factor`) dalam satuan dasar (*base unit*) untuk setiap `product_id`.
     - Melakukan penguncian baris database secara aman (`SELECT quantity FROM warehouse_stocks WHERE warehouse_id = $1 AND product_id = $2 FOR UPDATE`).
     - Jika `current_stock < required_qty`, checkout **seketika ditolak** dengan pesan kesalahan informatif:
       `"Stok tidak mencukupi untuk '{name}' (SKU: {sku}). Stok tersedia: {current_stock} {unit}, total diminta: {required_qty} {unit}"`.
2. **Deduction Guard**:
   - Pada saat iterasi pemotongan stok fisik di `warehouse_stocks`, sistem kembali memastikan `current_stock >= base_qty`.
   - Menjamin bahwa nilai `new_balance` tidak akan pernah bernilai negatif dalam kondisi apa pun (fail-closed).

---

### B. Frontend Client (Vue.js + Pinia + TypeScript)

1. **Cart Store ([`src/stores/cartStore.ts`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src/stores/cartStore.ts))**:
   - **`getMaxAllowedQuantity(item: CartItem): number`**: Menghitung batas kuantitas maksimal yang diperbolehkan berdasarkan konversi satuan terpilih (`available_stock / conversion_factor`).
   - **`addProductToCart`**:
     - Menolak penambahan produk jika stok gudang/outlet bernilai 0 (`Stok produk habis`).
     - Menolak penambahan jika `(kuantitas_sekarang + kuantitas_baru) > stok_tersedia`, serta mengunci kuantitas pada batas maksimal stok.
   - **`addProduct`**:
     - Memeriksa ketersediaan stok sebelum memasukkan produk ke keranjang belanja.
   - **`updateQuantity(index, newQty)`**:
     - Otomatis membatasi (*clamp*) kuantitas ke nilai `maxAllowed` jika kasir menginput angka melebihi stok yang ada.
   - **`changeUnit`**:
     - Menyesuaikan kembali kuantitas agar tidak melebihi stok yang tersedia pada satuan baru.

2. **Tabel Keranjang Belanja ([`src/components/pos/CartTable.vue`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src/components/pos/CartTable.vue))**:
   - Menampilkan indikator **`Sisa Stok: X [Satuan]`** di bawah nama barang (berwarna merah saat stok mencapai batas).
   - Menetapkan atribut `:max="getMaxAllowedQuantity(item)"` pada input kuantitas.
   - Menonaktifkan tombol **`+`** (`:disabled="item.quantity >= maxAllowed"`) dengan status kursor `cursor-not-allowed` jika kuantitas telah mencapai stok maksimal.
   - Menampilkan label peringatan **`Maksimal Stok`** tepat di bawah stepper kuantitas.

3. **Autocomplete Pencarian Barcode ([`src/components/pos/BarcodeBar.vue`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src/components/pos/BarcodeBar.vue))**:
   - Menampilkan badge merah **`Stok Habis (0)`** pada daftar produk yang stoknya 0.
   - Mencegah penambahan ke keranjang jika kasir memilih barang yang habis dan memunculkan notifikasi peringatan.

4. **Katalog Modal Pencarian ([`src/components/pos/ProductSearchModal.vue`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src/components/pos/ProductSearchModal.vue))**:
   - Menampilkan status **`Stok Habis (0)`** dan menonaktifkan tombol tambah (`+`) untuk barang yang tidak memiliki stok.
   - Mencegah klik dengan efek visual disabled.

5. **Modal Pembayaran ([`src/components/pos/QuickPayModal.vue`](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/src/components/pos/QuickPayModal.vue))**:
   - Melakukan validasi pra-bayar (*pre-payment check*) pada seluruh item belanja.
   - Mencegah eksekusi pembayaran jika ada item yang melebihi stok dan menginstruksikan kasir untuk menyesuaikan keranjang.

---

## 3. Hasil Pengujian
1. **Pengujian Backend via API (curl)**:
   - Request transaksi dengan kuantitas 100 pada barang berstok 38 menghasilkan penolakan:
     ```json
     {
       "message": "Stok tidak mencukupi untuk 'Kopi Kapal Api 165g' (SKU: SKU-KAPAL-API). Stok tersedia: 38.0000 BOX, total diminta: 100 BOX",
       "success": false
     }
     ```
2. **Pengujian UI Kasir di Browser (E2E)**:
   - Kasir menginputkan angka `999` pada kuantitas `Kopi Kapal Api 165g`.
   - Kuantitas otomatis di-clamp menjadi `38`.
   - Tombol `+` otomatis terkunci (disabled).
   - Muncul keterangan `Maksimal Stok` dan `Sisa Stok: 38 BOX`.
