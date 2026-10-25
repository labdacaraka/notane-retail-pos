# Progress 40: Eliminasi Default Warehouse dan Isolasi Pengecekan Stok Per Outlet

## 1. Latar Belakang Masalah
Sebelumnya, sistem masih memiliki sisa logika relasi hierarki antara toko ritel (`outlets`) dan gudang default (`default_warehouse_id` / tabel `outlet_warehouses`). Hal ini menyebabkan sejumlah inkonsistensi:
1. Pengecekan dan query stok produk (seperti pada `GET /api/v1/products`) menggunakan klausa `IN (SELECT COALESCE(o.default_warehouse_id, o.id) ... UNION SELECT id ...)` yang mengakumulasikan kuantitas stok outlet dengan gudang penyuplainya.
2. Ketika kasir atau fitur transfer stok melakukan validasi dan pemotongan stok aktual pada ID outlet toko, terjadi kegagalan (misalnya pesan *"Stok produk di outlet asal tidak mencukupi. Tersedia: 0"*), padahal di tabel tampilan produk stok tampak ada karena diakumulasikan dari gudang default.
3. Sebenarnya konsep gudang terpisah tidak berlaku atau setiap outlet/gudang adalah unit penampung stok independen tersendiri, sehingga kolom `default_warehouse_id` pada outlet tidak lagi diperlukan.

---

## 2. Rincian Perubahan yang Diterapkan

### A. Database Migration (`026_remove_default_warehouse_from_outlets.sql`)
1. **Konsolidasi Stok Fisik**:
   - Memindahkan seluruh stok riil yang sebelumnya tersimpan pada gudang awal default `WH-001` (`b0000000-0000-0000-0000-000000000002`) ke toko utama `OUT-001` (`b0000000-0000-0000-0000-000000000001`) via `ON CONFLICT (warehouse_id, product_id) DO UPDATE`.
   - Mengosongkan kuantitas di `WH-001` menjadi `0` untuk menghindari duplikasi stok.
2. **Pembersihan Skema**:
   - Menghapus foreign key constraint `fk_outlets_default_warehouse` pada tabel `outlets`.
   - Menghapus kolom `default_warehouse_id` dari tabel `outlets`.
   - Menghapus tabel relasi `outlet_warehouses`.
3. **Pencatatan Migrasi**:
   - Didaftarkan pada daftar migrasi terintegrasi `src-tauri/src/db/mod.rs` dan sukses dieksekusi ke PostgreSQL.

### B. Backend Rust Engine
1. **Query Stok Terisolasi (`src-tauri/src/api/master.rs`)**:
   - Mengubah query kalkulasi stok pada `list_products_handler`:
     ```sql
     LEFT JOIN warehouse_stocks ws ON p.id = ws.product_id AND ws.warehouse_id = '{oid}'
     ```
     Stok produk kini 100% murni dihitung HANYA dari outlet/gudang yang diminta (`outlet_id`), tanpa akumulasi dengan gudang mana pun.
   - Menghapus `default_warehouse_id` dari payload create/update outlet serta endpoint detail outlet (`get_outlet_handler`).
2. **POS Service (`src-tauri/src/services/pos_service.rs`)**:
   - Pada `open_shift` dan `get_current_shift`, tidak lagi membaca atau mengisi `default_warehouse_id`.
   - Pada `find_product_for_pos` (scan barcode kasir), pengecekan stok murni membaca `warehouse_stocks` dari `outlet_id` kasir.
   - Pada `checkout`, pemotongan inventori langsung menargetkan `target_warehouse_id = outlet_id`.
3. **Inventory Service (`src-tauri/src/services/inventory_service.rs`)**:
   - Pengecekan ketersediaan stok transfer antar-outlet maupun pembatalan transfer langsung membaca dan memotong saldo fisik dari `from_outlet_id`.
4. **Model Structs**:
   - Menghapus field `default_warehouse_id` dari struct `CashierShift` (`src-tauri/src/models/shift.rs`).

### C. Frontend Pinia Stores & UI Components
1. **`src/types/index.ts`**:
   - Menghapus `default_warehouse_id` dari tipe `CashierShift` dan `Outlet`.
2. **`src/stores/cartStore.ts` & `shiftStore.ts`**:
   - Menghapus state `defaultWarehouseId` dan key `notane_pos_warehouse_id`.
   - Menyelaraskan scanner kasir agar selalu menyematkan `outlet_id` kasir yang aktif.
3. **`src/stores/locationStore.ts`**:
   - Menghapus `default_warehouse_id` dari antarmuka `AccessibleOutlet` dan fungsi perpindahan outlet.
4. **`src/components/pos/QuickPayModal.vue`**:
   - Memastikan payload checkout menyertakan `outlet_id` dan `warehouse_id` yang merujuk pada outlet kasir aktif.
5. **`src/pages/OutletsPage.vue`**:
   - Menghapus kolom header dan baris tabel *"Gudang Suplai"*.
   - Menghapus dropdown pemilihan gudang penyuplai stok dari modal formulir outlet.
   - Membersihkan payload simpan data dari `default_warehouse_id`.
6. **`src/pages/InventoryPage.vue`**:
   - Menyelaraskan sinkronisasi lokasi inventori langsung menggunakan `locationStore.currentOutlet.id`.

---

## 3. Hasil Pengujian & Verifikasi
1. **Kompilasi Rust Backend**: `cargo check` dan `cargo build` berhasil (0 errors).
2. **Eksekusi Migrasi Database**: Migrasi `026_remove_default_warehouse_from_outlets` sukses dieksekusi pada startup server.
3. **Frontend Build & TypeScript**: `npm run build` (`vue-tsc && vite build`) selesai dengan status sukses tanpa kesalahan tipe.
4. **Verifikasi API Stok Terisolasi**:
   - `GET /api/v1/products?outlet_id=b0000000-0000-0000-0000-000000000001`: Produk *Indomie Goreng Spesial 85g* menunjukkan stok `138.0000` (tersimpan langsung di `OUT-001`).
   - `GET /api/v1/products?outlet_id=b0000000-0000-0000-0000-000000000002`: Gudang `WH-001` menunjukkan stok `0` (tidak ada akumulasi silang).
   - `GET /api/v1/outlets`: Data outlet bersih tanpa atribut `default_warehouse_id`.
