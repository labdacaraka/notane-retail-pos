# Progress 57: Implementasi Permission All Ownership untuk Akses Data Transaksi (Ownership-Based RBAC)

## Ringkasan Fitur
Menambahkan sistem kontrol akses berbasis kepemilikan data (*Data Ownership Access Control*) untuk seluruh fitur transaksi:
- **Konsep**: Suatu role yang memiliki izin `all_ownership` bebas melihat seluruh data yang dibuat oleh siapa saja. Jika role **tidak** memiliki izin tersebut, maka query data otomatis difilter di sisi backend Rust sehingga hanya menampilkan data yang dibuat atau diubah oleh user yang sedang login (`created_by = user_id OR updated_by = user_id`).
- **Zero-Trust Security**: Validasi dilakukan secara ketat di sisi backend database query & authorization guard, bukan hanya di antarmuka frontend.

## Daftar Permission All Ownership Baru
1. **`sales:all_ownership`**: Melihat seluruh transaksi penjualan dari semua staf dan kasir (tanpa batas kepemilikan data).
2. **`purchasing:all_ownership`**: Melihat seluruh transaksi pembelian dan faktur supplier dari semua staf (tanpa batas kepemilikan data).
3. **`expenses:all_ownership`**: Melihat seluruh transaksi pengeluaran kas dan biaya operasional dari semua staf (tanpa batas kepemilikan data).
4. **`inventory:all_ownership`**: Melihat seluruh mutasi, penyesuaian, dan transfer stok dari semua staf (tanpa batas kepemilikan data).
5. **`accounting:all_ownership`**: Melihat seluruh entri jurnal akuntansi dan pembukuan dari semua staf (tanpa batas kepemilikan data).

Secara default, permission ini dipetakan ke role **SUPERADMIN** dan **STORE_MANAGER**, serta dapat dikonfigurasi secara granular untuk role lainnya (*SUPERVISOR*, *CASHIER*, dll.) via halaman Manajemen Pengguna & Hak Akses (`/admin/users`).

## Detail Implementasi Teknis

### 1. Database Layer: Migrasi 042 (`042_all_ownership_permissions.sql`)
- Mendaftarkan kelima permission ke tabel `permissions`.
- Memetakan permission baru ke role `SUPERADMIN` dan `STORE_MANAGER`.
- Didaftarkan pada array `migrations` di [notane-server/src-tauri/src/db/mod.rs](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/notane-server/src-tauri/src/db/mod.rs) dan [notane-client/src-tauri/src/db/mod.rs](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/notane-client/src-tauri/src/db/mod.rs).

### 2. Authorization Helper (`jwt.rs`)
- Menambahkan fungsi helper `check_permission(&pool, &claims, perm: &str) -> bool` di [notane-server/src-tauri/src/utils/jwt.rs](file:///Volumes/Data/Bisnis/Notane/NotaneRetail/Codes/notane-retail-pos/notane-server/src-tauri/src/utils/jwt.rs):
  - Mengembalikan `true` secara instan untuk role `SUPERADMIN`.
  - Memeriksa tabel `role_permissions` secara reaktif untuk role lainnya.

### 3. Backend Services & Handlers

#### A. Penjualan (`pos_service.rs` & `api/pos.rs`)
- **`list_sales`**: Menerima `owner_id: Option<Uuid>`. Jika diisi (tidak punya `sales:all_ownership`), menambahkan filter:
  `AND (s.created_by = '{uid}' OR s.updated_by = '{uid}' OR cs.user_id = '{uid}')`.
- **`get_sales_stats`**: Menerima `owner_id: Option<Uuid>` untuk menyaring total omzet, total transaksi, dan total item terjual hanya milik user tersebut.
- **`get_sale_detail`**: Memvalidasi kepemilikan nota (`created_by`, `updated_by`, atau kasir shift). Jika staf lain mencoba mengakses nota tanpa izin, mengembalikan `403 FORBIDDEN`.

#### B. Pembelian (`inventory_service.rs` & `api/inventory.rs`)
- **`list_purchase_invoices`**: Memfilter `(pi.created_by = '{uid}' OR pi.updated_by = '{uid}')` jika tidak memiliki `purchasing:all_ownership`.
- **`get_purchase_invoice_detail`**: Memeriksa otorisasi detail faktur pembelian dan menolak akses jika bukan pemilik data.

#### C. Pengeluaran / Biaya Operasional (`expense_service.rs` & `api/expenses.rs`)
- **`list_expenses`**: Memfilter `(et.created_by = '{uid}' OR et.cancelled_by = '{uid}')` jika tidak memiliki `expenses:all_ownership`.
- **`get_expense_summary`**: Menyaring ringkasan pengeluaran harian, bulanan, dan breakdown kategori sesuai kepemilikan data.
- **`get_expense_by_id`**: Memvalidasi kepemilikan transaksi pengeluaran.

#### D. Transfer Stok & Mutasi (`inventory_service.rs` & `api/inventory.rs`)
- **`list_stock_transfers`**: Memfilter `st.created_by = '{uid}'` jika tidak memiliki `inventory:all_ownership`.

#### E. Akuntansi & Jurnal Umum (`accounting_service.rs` & `api/reports.rs`)
- **`list_journal_entries`**: Memfilter `(je.created_by = '{uid}' OR je.updated_by = '{uid}')` jika tidak memiliki `accounting:all_ownership`.

### 4. Frontend UI: Manajemen Pengguna & Hak Akses (`UsersPage.vue`)
- Menambahkan pemetaan label dan ikon untuk modul `sales`, `expenses`, `products`, `customers`, dan `vouchers` di `permissionModuleLabels` dan `permissionModuleIcons`.
- Permissions tampil rapi dan terorganisir di tab **Roles & Permissions** dan modal konfigurasi role.

### 5. Sinkronisasi Client-Server
- Seluruh perubahan di backend dan frontend tersinkronisasi 100% antara `notane-server` dan `notane-client`.
- Lulus pengujian kompilasi `cargo check` pada kedua modul tanpa error.

## Hasil Verifikasi Browser E2E
- Migrasi `042_all_ownership_permissions` dieksekusi sukses saat server boot.
- Di `/admin/users` tab **Roles & Permissions**, seluruh permission baru tampil aktif dan terkelola dengan baik.
- Di `/admin/sales`, data rekapitulasi penjualan kasir dan ringkasan kartu statistik termuat sempurna.
