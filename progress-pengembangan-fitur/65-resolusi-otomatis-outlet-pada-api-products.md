# Progress 65: Resolusi Otomatis Outlet pada API Products dan Dropdown Tanpa Redundansi Payload

## Latar Belakang
Sebelumnya saat mengakses halaman Master Produk (`ProductsPage.vue`) serta dropdown pencarian produk kasir (`ProductSearchModal.vue`, `BarcodeBar.vue`), frontend masih menyertakan parameter `outlet_id: activeOutletId` di query request API. Hal ini membebani komunikasi client-server dan rawan inkonsistensi jika state lokasi di sisi browser belum sinkron. Padahal server backend (`notane-server`) telah memiliki konteks otentikasi user dan sesi shift aktif.

## Solusi & Perubahan yang Diterapkan

### 1. Backend (`notane-server`)
- Mengintegrasikan fungsi utilitas `resolve_effective_outlet_id(&pool, &headers, query.outlet_id)` ke dalam:
  - `list_products_handler` di [notane-server/src-tauri/src/api/master.rs](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/notane-server/src-tauri/src/api/master.rs):
    - Menerima `headers: HeaderMap` untuk mengekstrak claims sesi JWT.
    - Menghitung outlet efektif dengan urutan prioritas:
      1. Nilai eksplisit `query.outlet_id` jika disediakan (untuk kasus spesifik seperti pemilihan gudang asal transfer atau PO khusus).
      2. Outlet dari shift kasir aktif yang sedang terbuka (`cashier_shifts`).
      3. Preferensi lokasi outlet yang tersimpan pada akun pengguna (`users.current_outlet_id`).
      4. Fallback ke outlet ritel aktif pertama di database.
    - Menerapkan `effective_outlet_id` pada query join harga outlet (`product_outlet_prices`) dan stok gudang (`warehouse_stocks`), sehingga perhitungan HPP, harga jual toko, dan sisa stok kuantitas selalu akurat.
  - `barcode_lookup_handler` di [notane-server/src-tauri/src/api/pos.rs](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/notane-server/src-tauri/src/api/pos.rs):
    - Menggunakan `resolve_effective_outlet_id` sehingga barcode scanning tidak gagal jika `outlet_id` tidak disertakan di query params.

### 2. Frontend (`notane-client`)
- **`ProductsPage.vue`**:
  - Menghapus parameter `outlet_id: activeOutletId` pada pemanggilan `api.get('/products')`. Halaman master produk kini mengambil data produk bersih tanpa mengirim `outlet_id`.
- **`ProductSearchModal.vue` & `BarcodeBar.vue`**:
  - Menghapus query `outlet_id` pada pencarian produk cepat dan dropdown kasir. Backend otomatis mengaitkan produk dengan open shift kasir atau lokasi aktif.
- **`InventoryPage.vue`**:
  - Menghapus query `outlet_id` pada `loadInitialData()` produk awal.
  - Mempertahankan `extra-params="{ outlet_id: transferForm.from_outlet_id }"` pada bagian transfer stok gudang karena merupakan case khusus untuk mengecek stok outlet asal pengiriman.
- **`PurchasesPage.vue`**:
  - Mempertahankan `extra-params="poForm.outlet_id ? { outlet_id: poForm.outlet_id } : {}"` pada baris item PO, di mana backend akan memprioritaskan outlet PO jika dipilih, atau fallback otomatis ke outlet aktif jika belum dipilih.

## Hasil Pengujian & Verifikasi
- Pengujian API langsung:
  ```bash
  curl -s "http://127.0.0.1:18080/api/v1/products?limit=2"
  ```
  Menghasilkan status `success: true` lengkap dengan informasi harga dan kuantitas stok produk (`stock_qty`) yang teresolusi secara otomatis.
- Kompilasi backend `cargo check` dan `cargo build` berhasil bersih.
- Build frontend `npm run build` (`vue-tsc && vite build`) di `notane-client` selesai dengan kode 0 (sukses).
- Daemon server host berjalan aktif di port 18080 dan 18081.
