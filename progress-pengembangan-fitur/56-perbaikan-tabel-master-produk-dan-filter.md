# Progress 56: Perbaikan Tabel Master Produk, Tombol Aksi, Filter Kategori, dan Sinkronisasi Outlet Aktif

## Ringkasan Perbaikan
Memperbaiki antarmuka master produk di `/admin/products`:
1. **Keselarasan Kolom Tabel (Alignment)**: Mengatasi ketidaksesuaian kolom di mana sel `Satuan Dasar` hilang di `<tbody>`, yang sebelumnya menyebabkan data HPP, Harga Jual, Stok Fisik, dan Aksi bergeser satu kolom ke kiri.
2. **Kerapian Tombol Aksi**: Mengelompokkan tombol aksi per baris menjadi rapi dan terorganisir (`Batch`, `Barcode`, Dropdown `Harga` untuk opsi harga khusus outlet/grup/member, tombol icon `Edit`, dan tombol icon `Hapus`).
3. **Filter Kategori Searchable ke API**: Mengganti select native dengan komponen `SearchableSelect` yang terhubung ke endpoint `/categories?search=...`, serta memperbarui backend Rust agar mendukung parameter pencarian kategori.
4. **Sinkronisasi Harga dan Stok Berdasarkan Outlet Global**: Memastikan harga dan stok produk selalu memprioritaskan outlet aktif yang dipilih di pojok kiri atas, memperbarui query fallback harga outlet, serta me-reload data secara reaktif saat outlet diganti.

## Detail Implementasi

### 1. Backend (`notane-server/src-tauri/src/api/master.rs` & `notane-client/src-tauri/src/api/master.rs`)
- **Pencarian Kategori**: Menambahkan struct `CategoryQuery { search: Option<String> }` pada `list_categories_handler` agar mendukung pencarian kategori berdasarkan nama atau kode via SQL `ILIKE`.
- **Resolusi Harga Outlet**: Memperbaiki fallback harga outlet di `list_products_handler`:
  - `COALESCE(NULLIF(pop.cost_price, 0), p.cost_price) as cost_price`
  - `COALESCE(NULLIF(pop.base_selling_price, 0), p.base_selling_price) as base_selling_price`
  Jika outlet belum memiliki override harga khusus, otomatis mengambil harga default produk master.

### 2. Frontend (`notane-client/src/pages/ProductsPage.vue` & `notane-server/src/pages/ProductsPage.vue`)
- **Tabel Master Produk**:
  - Menambahkan sel `<td>` untuk **Satuan Dasar** (`prod.unit_code`) di urutan ke-5 sesuai `<thead>`.
  - Header kolom `Harga Jual` dan `Stok Fisik` menampilkan kode outlet aktif (`[OUT-001]` atau `[WH-001]`).
- **Tombol Aksi**:
  - Menyediakan dropdown menu **`Harga`** yang memuat opsi:
    - *Harga Outlet / Cabang*
    - *Harga Kelompok Pelanggan*
    - *Harga Member per Outlet*
  - Tombol aksi quick icon yang ringkas untuk `Edit` dan `Hapus`.
- **Filter Kategori**:
  - Mengintegrasikan `SearchableSelect` dengan `api-endpoint="/categories"`, placeholder pencarian, dan tombol clear selection (`allow-clear`).
- **Reaktivitas Outlet Global**:
  - Memastikan `onMounted` memeriksa dan menunggu `fetchAccessibleOutlets()` jika outlet belum terinisialisasi.
  - Memastikan parameter query `outlet_id` dikirimkan secara akurat berdasarkan `locationStore.currentOutletId`.
  - Sinkronisasi otomatis saat event `notane-location-changed` terpanggil.

## Hasil Pengujian Browser E2E
- Kolom tabel tampil presisi 100% (semua 9 kolom sejajar sempurna).
- Menu popup dropdown `Harga` berfungsi mulus dan dapat ditutup saat klik di luar.
- Pencarian kategori `Sayur` memfilter ke `Sayur & Buah Segar`, pencarian `Minuman` memfilter ke `Makanan & Minuman`, dan tombol clear selection mereset tabel kembali normal.
- Penggantian outlet aktif di pojok kiri atas (ke `[WH-001]` dan kembali ke `[OUT-001]`) secara instan merefresh harga dan stok fisik sesuai data riil outlet tersebut.
