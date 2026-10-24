-- ============================================================================
-- MIGRASI 043: KELENGKAPAN SELURUH ACTION PERMISSIONS SISTEM NOTANE (1 ACTION = 1 PERMISSION)
-- Menjamin setiap aksi (create, read, update, delete/void) di seluruh endpoint API
-- memiliki permission database yang berpasangan dan dipetakan ke role default.
-- ============================================================================

-- 1. DAFTARKAN SEMUA PERMISSION GRANULAR BERDASARKAN AKSI & ENDPOINT
INSERT INTO permissions (code, description) VALUES
-- Penjualan POS (Sales)
('sales:read', 'Melihat daftar, ringkasan, riwayat nota, dan analitik transaksi penjualan kasir'),
('sales:create', 'Melakukan checkout dan pencatatan transaksi penjualan baru di kasir'),
('sales:void', 'Otorisasi pembatalan atau void nota transaksi penjualan'),
('sales:discount', 'Pemberian potongan diskon manual atau override harga item penjualan'),
('sales:open_drawer', 'Otorisasi membuka laci meja kasir manual tanpa transaksi belanja'),

-- Retur Penjualan (Sales Returns)
('sales_returns:read', 'Melihat data dan riwayat permohonan retur penjualan konsumen'),
('sales_returns:create', 'Membuat permohonan retur penjualan dan penukaran barang konsumen di kasir'),
('sales_returns:approve', 'Otorisasi persetujuan retur penjualan konsumen dan pengembalian uang'),
('sales_returns:reject', 'Otorisasi penolakan permohonan retur penjualan konsumen'),

-- Shift Kasir (Cashier Shifts)
('shift:read', 'Melihat riwayat, status shift aktif, dan ringkasan kas laci shift kasir'),

-- Tarik Tunai Kas Laci (Cash Drops)
('cash_drops:read', 'Melihat riwayat serah terima penarikan kas laci meja kasir'),
('cash_drops:create', 'Melakukan penarikan kas laci meja kasir ke brankas/supervisor'),

-- Pembelian & Pengadaan (Purchasing)
('purchasing:create', 'Membuat pesanan pembelian (PO), bukti terima barang (GRN), dan faktur supplier baru'),
('purchasing:update', 'Mengubah pesanan pembelian dan memproses penerimaan barang (GRN) ke faktur pembelian'),

-- Pengeluaran Operasional (Expenses)
('expenses:update', 'Mengubah data kategori atau keterangan transaksi pengeluaran kas operasional'),

-- Promosi, Voucher & Diskon (Promotions & Vouchers)
('vouchers:read', 'Melihat daftar kupon voucher diskon dan memeriksa validitas kode voucher kasir'),
('vouchers:create', 'Membuat kode kupon voucher diskon promosi toko baru'),
('vouchers:update', 'Mengubah syarat, nominal, dan ketentuan kupon voucher diskon'),
('vouchers:delete', 'Menghapus kupon voucher diskon promosi toko'),

-- Loyalitas & Poin Member (Loyalty)
('loyalty:adjust', 'Melakukan koreksi atau penyesuaian manual saldo poin member pelanggan'),

-- Master Produk (Products)
('products:read', 'Melihat katalog produk, daftar harga, dan ketersediaan stok barang'),
('products:create', 'Menambahkan produk dan varian barang baru ke dalam katalog master'),
('products:update', 'Mengubah informasi produk, satuan, harga jual, dan spesifikasi barang'),
('products:delete', 'Menghapus atau menonaktifkan produk dari katalog master barang'),

-- Master Pelanggan & Member (Customers)
('customers:read', 'Melihat data profil pelanggan, grup member, limit piutang, dan riwayat belanja'),
('customers:create', 'Mendaftarkan pelanggan baru atau grup klasifikasi member pelanggan'),
('customers:update', 'Mengubah informasi kontak, data profil, dan limit kredit pelanggan'),
('customers:delete', 'Menghapus atau menonaktifkan data pelanggan dari master sistem'),

-- Master Pemasok / Supplier (Suppliers)
('suppliers:read', 'Melihat data master pemasok, kontak perwakilan, dan riwayat faktur supplier'),
('suppliers:create', 'Menambahkan data pemasok/supplier baru ke dalam sistem'),
('suppliers:update', 'Mengubah informasi alamat, telepon, dan syarat pembayaran supplier'),
('suppliers:delete', 'Menghapus data pemasok/supplier dari master sistem'),

-- Master Kategori Produk (Categories)
('categories:read', 'Melihat daftar kategori klasifikasi produk barang'),
('categories:create', 'Menambahkan kategori klasifikasi produk baru'),
('categories:update', 'Mengubah nama dan informasi kategori produk'),
('categories:delete', 'Menghapus kategori produk dari master sistem'),

-- Master Merek Produk (Brands)
('brands:read', 'Melihat daftar merek/brand manufaktur produk'),
('brands:create', 'Menambahkan merek/brand produk baru'),
('brands:update', 'Mengubah nama dan informasi merek/brand produk'),
('brands:delete', 'Menghapus merek/brand produk dari master sistem'),

-- Master Satuan Ukuran (Units)
('units:read', 'Melihat daftar satuan ukuran dan konversi takaran barang'),
('units:create', 'Menambahkan satuan ukuran barang baru'),
('units:update', 'Mengubah nama dan simbol satuan ukuran barang'),
('units:delete', 'Menghapus satuan ukuran barang dari master sistem'),

-- Master Outlet & Gerai Toko (Outlets)
('outlets:create', 'Menambahkan gerai cabang atau outlet toko baru'),
('outlets:update', 'Mengubah alamat, telepon, dan informasi konfigurasi outlet toko'),
('outlets:delete', 'Menonaktifkan atau menghapus data gerai cabang outlet toko'),

-- Pengaturan Sistem & Struk (Settings)
('settings:read', 'Melihat pengaturan sistem toko, format struk belanja, dan kebijakan operasional'),
('settings:update', 'Mengubah konfigurasi toko, format struk kasir, logo, dan preferensi sistem'),

-- Akuntansi & Jurnal Umum (Accounting)
('accounting:create', 'Mencatat entri jurnal akuntansi manual dan menambah akun bagan akun (COA) baru'),
('accounting:update', 'Mengubah bagan akun (COA) dan parameter pembukuan akuntansi'),

-- Laporan & Analitik (Reports)
('report:read', 'Melihat seluruh laporan analitik penjualan, laba rugi, dan performa bisnis'),

-- Pengguna & Hak Akses (Users)
('users:read', 'Melihat daftar akun staf, profil pengguna, daftar peran (roles), dan matriks perizinan'),
('users:create', 'Mendaftarkan akun staf pengguna baru atau membuat peran (role) baru'),
('users:update', 'Mengubah data akun staf pengguna, peran penugasan, dan hak akses permissions peran'),
('users:delete', 'Menonaktifkan atau menghapus akun staf pengguna dan peran dari sistem'),
('users:change_password', 'Mengubah atau mereset kata sandi login staf pengguna lain')
ON CONFLICT (code) DO NOTHING;


-- ============================================================================
-- 2. PEMETAAN PERMISSION KE PERAN (ROLE-BASED ACCESS CONTROL)
-- ============================================================================

-- A. Role SUPERADMIN: Mendapatkan SEMUA HAK AKSES (100% Seluruh Permissions)
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r, permissions p
WHERE r.name = 'SUPERADMIN'
ON CONFLICT DO NOTHING;

-- B. Role STORE_MANAGER: Manajemen Operasional Toko, Laporan Lengkap & Supervisi
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r, permissions p
WHERE r.name = 'STORE_MANAGER' AND p.code IN (
    -- Sales & Returns
    'sales:read', 'sales:create', 'sales:void', 'sales:discount', 'sales:open_drawer', 'sales:all_ownership',
    'pos:checkout', 'pos:void', 'pos:discount', 'pos:open_drawer', 'pos:hold_recall',
    'sales_returns:read', 'sales_returns:create', 'sales_returns:approve', 'sales_returns:reject',
    
    -- Shifts & Cash
    'shift:read', 'shift:open', 'shift:close', 'shift:view',
    'cash_drops:read', 'cash_drops:create',

    -- Purchasing & Inventory
    'purchasing:read', 'purchasing:create', 'purchasing:update', 'purchasing:pay', 'purchasing:void', 'purchasing:write', 'purchasing:all_ownership',
    'inventory:read', 'inventory:write', 'inventory:adjust', 'inventory:transfer', 'inventory:transfer_cancel', 'inventory:all_ownership',

    -- Operational Expenses & Capital
    'expenses:read', 'expenses:create', 'expenses:update', 'expenses:delete', 'expenses:categories', 'expenses:all_ownership',
    'capital:read', 'capital:create', 'capital:delete',
    'bank:read', 'bank:create', 'bank:update',

    -- Promotions & Loyalty
    'promotions:read', 'promotions:create', 'promotions:update', 'promotions:delete', 'promotions:write',
    'vouchers:read', 'vouchers:create', 'vouchers:update', 'vouchers:delete', 'vouchers:manage',
    'loyalty:read', 'loyalty:manage', 'loyalty:adjust', 'loyalty:redeem',

    -- Master Data
    'products:read', 'products:create', 'products:update', 'products:delete', 'products:import', 'products:barcodes', 'products:outlet_prices', 'products:customer_group_prices', 'products:price_tiers',
    'customers:read', 'customers:create', 'customers:update', 'customers:delete', 'customers:manage',
    'suppliers:read', 'suppliers:create', 'suppliers:update', 'suppliers:delete',
    'categories:read', 'categories:create', 'categories:update', 'categories:delete',
    'brands:read', 'brands:create', 'brands:update', 'brands:delete',
    'units:read', 'units:create', 'units:update', 'units:delete',
    'outlets:read', 'outlets:update', 'outlets:manage',

    -- Reports & Accounting
    'report:read', 'report:view',
    'accounting:read', 'accounting:create', 'accounting:close_period', 'accounting:all_ownership',

    -- Settings & Staf
    'settings:read', 'settings:update', 'settings:manage',
    'users:read', 'users:change_pin'
)
ON CONFLICT DO NOTHING;

-- C. Role SUPERVISOR: Otorisasi Kasir, Approval Retur, Supervisi Gerai & Penarikan Kas
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r, permissions p
WHERE r.name = 'SUPERVISOR' AND p.code IN (
    -- Sales & Otorisasi Kasir
    'sales:read', 'sales:create', 'sales:void', 'sales:discount', 'sales:open_drawer',
    'pos:checkout', 'pos:void', 'pos:discount', 'pos:open_drawer', 'pos:hold_recall',
    'sales_returns:read', 'sales_returns:create', 'sales_returns:approve', 'sales_returns:reject',
    
    -- Shift & Kas
    'shift:read', 'shift:open', 'shift:close', 'shift:view',
    'cash_drops:read', 'cash_drops:create',

    -- Inventori Cabang & Petty Cash
    'inventory:read', 'inventory:transfer',
    'expenses:read', 'expenses:create',
    'bank:read',

    -- Promo & Member
    'promotions:read',
    'vouchers:read',
    'loyalty:read', 'loyalty:redeem',

    -- Master Data Read-Only
    'products:read',
    'customers:read', 'customers:create', 'customers:update',
    'suppliers:read',
    'categories:read',
    'brands:read',
    'units:read',
    'outlets:read',

    -- Biometrik & Laporan Ringan
    'supervisor:biometric_override',
    'report:read', 'report:view'
)
ON CONFLICT DO NOTHING;

-- D. Role CASHIER: Fokus Transaksi Kasir, Cetak Struk, Poin & Retur Konsumen
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r, permissions p
WHERE r.name = 'CASHIER' AND p.code IN (
    'sales:read', 'sales:create',
    'pos:checkout', 'pos:hold_recall',
    'sales_returns:read', 'sales_returns:create',
    'shift:read', 'shift:open', 'shift:close', 'shift:view',
    'cash_drops:read', 'cash_drops:create',
    'expenses:read', 'expenses:create',
    'products:read',
    'customers:read', 'customers:create',
    'vouchers:read',
    'loyalty:read', 'loyalty:redeem'
)
ON CONFLICT DO NOTHING;

-- E. Role WAREHOUSE: Staf Gudang untuk Stok, Transfer Barang & Penerimaan (GRN)
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r, permissions p
WHERE r.name = 'WAREHOUSE' AND p.code IN (
    'inventory:read', 'inventory:write', 'inventory:adjust', 'inventory:transfer',
    'purchasing:read', 'purchasing:create', 'purchasing:update', 'purchasing:write',
    'products:read', 'products:barcodes',
    'suppliers:read',
    'categories:read',
    'brands:read',
    'units:read'
)
ON CONFLICT DO NOTHING;
