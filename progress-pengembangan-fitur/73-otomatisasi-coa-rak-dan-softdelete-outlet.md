# 73. Otomatisasi Chart of Account RAK & Persediaan serta Mekanisme Soft-Delete Outlet

## Ringkasan Perubahan
Modul Master Outlet & Gudang diperbarui untuk memastikan kelengkapan akun keuangan (Chart of Accounts) dan menjaga integritas data akuntansi saat terjadi penonaktifan/penghapusan outlet:
1. **Otomatisasi Akun RAK (Rekening Antar Kantor - 1901) & Persediaan (1201):**
   - Saat membuat outlet/gudang baru, sistem secara otomatis membuat dua akun COA spesifik untuk lokasi tersebut:
     - `1201-<KODE_OUTLET>`: Persediaan - <Nama Outlet> (Aset Lancar)
     - `1901-<KODE_OUTLET>`: RAK - <Nama Outlet> (Aset Lainnya / RAK)
   - Kolom `inventory_account_id` dan `rak_account_id` di tabel `outlets` langsung dihubungkan ke ID akun masing-masing.
   - Saat update nama outlet atau status aktif, nama kedua akun COA tersebut disinkronkan secara otomatis.
2. **Backfill Migrasi Database (053):**
   - Dibuat migrasi `053_sync_missing_outlet_rak_accounts.sql` untuk memastikan parent account `1901` tersedia dan otomatis membuatkan akun RAK bagi outlet lama yang belum memiliki akun RAK (seperti Cabang Tegal Besar / `OUT-002`).
3. **Mekanisme Soft-Delete Aman untuk Integritas Transaksi:**
   - Ketika pengguna menghapus outlet (`DELETE /api/v1/outlets/:id`), sistem memeriksa apakah outlet tersebut sudah memiliki riwayat transaksi:
     - Transaksi Penjualan (`sales`)
     - Mutasi/Transfer Stok (`stock_transfers` sebagai origin atau destination)
     - Faktur Pembelian (`purchase_invoices`)
     - Shift Kasir (`cashier_shifts`)
     - Jurnal Keuangan (`journal_entries` atau `journal_lines`)
   - **Jika terdapat riwayat transaksi:** Sistem tidak melakukan hard delete (yang akan menyebabkan foreign key violation atau merusak audit trail pembukuan), melainkan menerapkan **Soft-Delete** (`is_active = FALSE` pada outlet dan akun-akun COA terkait).
   - **Jika belum memiliki transaksi apapun:** Sistem melakukan penghapusan fisik (hard delete) bersih beserta akun COA yang belum terpakai.
4. **Penyempurnaan Tampilan UI Master Lokasi (`OutletsPage.vue`):**
   - Tabel menampilkan badge informasi kedua COA: `Stok: 1201-...` dan `RAK: 1901-...`.
   - Modal konfirmasi hapus memberikan penjelasan transparan mengenai kebijakan Soft-Delete pada outlet yang memiliki transaksi.

## File yang Dimodifikasi & Ditambahkan
1. `notane-server/src-tauri/migrations/053_sync_missing_outlet_rak_accounts.sql` (Migrasi baru)
2. `notane-server/src-tauri/src/db/mod.rs` (Registrasi migrasi)
3. `notane-server/src-tauri/src/api/master.rs` (Join COA RAK, auto-create RAK, sync saat update, validasi riwayat dan soft-delete)
4. `notane-client/src/pages/OutletsPage.vue` (UI badge Stok & RAK, pesan modal soft-delete)
