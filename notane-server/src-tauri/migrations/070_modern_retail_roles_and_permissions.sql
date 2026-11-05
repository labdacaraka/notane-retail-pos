-- ============================================================================
-- MIGRASI 070: STANDARISASI PERAN & PERMISSION INDUSTRI RETAIL MODERN
-- Menambahkan peran-peran resmi ritel modern: ASSISTANT_MANAGER, ACCOUNTING,
-- PURCHASING, INTERNAL_AUDIT, dan MERCHANDISER.
-- Melengkapi & mengaudit seluruh matriks perizinan untuk menjamin prinsip Zero-Trust
-- dan Segregation of Duties (Pemisahan Fungsi).
-- ============================================================================

-- 1. DAFTARKAN SEMUA PERAN (ROLES) STANDAR INDUSTRI RETAIL MODERN
INSERT INTO roles (name, description, is_active) VALUES
('SUPERADMIN', 'Akses penuh tanpa batas ke seluruh modul back-office, lisensi, dan konfigurasi sistem', TRUE),
('STORE_MANAGER', 'Manajer Toko dengan akses pengawasan menyeluruh, laporan laba rugi, dan otorisasi tingkat tinggi', TRUE),
('ASSISTANT_MANAGER', 'Asisten Manajer Toko untuk operasional harian gerai, supervisi kasir, dan penanganan stok', TRUE),
('SUPERVISOR', 'Supervisor Gerai dengan wewenang otorisasi kasir (void, diskon, buka laci), approval retur, dan opname', TRUE),
('CASHIER', 'Kasir Operasional untuk transaksi penjualan kasir, buka/tutup shift, setor kas, dan penukaran poin', TRUE),
('WAREHOUSE', 'Staf Gudang & Logistik untuk penerimaan barang (GRN), kartu stok, mutasi transfer, dan opname fisik', TRUE),
('PURCHASING', 'Staf Pengadaan / Buyer untuk pesanan pembelian (PO), faktur supplier, kontrol utang, dan retur pembelian', TRUE),
('ACCOUNTING', 'Staf Akuntansi & Keuangan untuk pembukuan jurnal, mutasi kas bank, pelunasan utang piutang, dan tutup buku', TRUE),
('INTERNAL_AUDIT', 'Auditor Internal & Pengawas Mutu untuk investigasi jejak audit, selisih stok (shrinkage), dan kepatuhan SOP', TRUE),
('MERCHANDISER', 'Merchandiser & Promosi untuk penataan katalog produk, barcode, harga bertingkat, banner layar, dan voucher', TRUE)
ON CONFLICT (name) DO UPDATE SET
    description = EXCLUDED.description,
    is_active = TRUE;


-- 2. PASTIKAN SELURUH KAMUS PERMISSION TERDAFTAR LENGKAP DI DATABASE
INSERT INTO permissions (code, description) VALUES
-- Penjualan & Kasir (Sales & POS)
('pos:checkout', 'Melakukan transaksi penjualan checkout kasir'),
('pos:void', 'Otorisasi pembatalan barang atau nota kasir (Void)'),
('pos:discount', 'Pemberian potongan diskon manual atau price override di kasir'),
('pos:open_drawer', 'Otorisasi membuka laci kasir manual tanpa transaksi belanja'),
('pos:hold_recall', 'Menahan dan memanggil kembali transaksi nota tertunda di kasir'),
('sales:read', 'Melihat daftar, ringkasan, riwayat nota, dan data penjualan kasir'),
('sales:create', 'Melakukan checkout dan pencatatan transaksi penjualan baru'),
('sales:void', 'Otorisasi pembatalan atau void nota transaksi penjualan'),
('sales:discount', 'Pemberian diskon manual atau potongan harga pada transaksi penjualan'),
('sales:open_drawer', 'Otorisasi membuka laci meja kasir tanpa transaksi belanja'),
('sales:export', 'Mengekspor laporan riwayat transaksi penjualan kasir ke format file Excel (.xlsx)'),
('sales:all_ownership', 'Melihat dan mengelola transaksi penjualan dari semua kasir/pengguna di outlet'),

-- Retur Penjualan (Sales Returns)
('sales_returns:read', 'Melihat data dan riwayat permohonan retur penjualan konsumen'),
('sales_returns:create', 'Membuat permohonan retur penjualan dan penukaran barang konsumen di kasir'),
('sales_returns:approve', 'Otorisasi persetujuan retur penjualan konsumen dan pengembalian uang'),
('sales_returns:reject', 'Otorisasi penolakan permohonan retur penjualan konsumen'),

-- Shift Kasir & Setor Kas Laci (Shifts & Cash Drops)
('shift:open', 'Membuka sesi shift kasir baru'),
('shift:close', 'Melakukan tutup shift dan pencatatan kas fisik (settlement)'),
('shift:view', 'Melihat riwayat shift kasir'),
('shift:read', 'Melihat status shift aktif dan ringkasan kas laci meja kasir'),
('cash_drops:read', 'Melihat riwayat serah terima penarikan kas laci meja kasir'),
('cash_drops:create', 'Melakukan penarikan kas laci meja kasir ke brankas/supervisor'),

-- Persediaan & Pergudangan (Inventory & Warehouse)
('inventory:read', 'Melihat data stok, kartu stok, dan riwayat pergerakan barang'),
('inventory:write', 'Mencatat mutasi barang masuk, keluar, dan penerimaan stok'),
('inventory:adjust', 'Melakukan penyesuaian stock opname fisik toko/gudang'),
('inventory:transfer', 'Membuat mutasi pengiriman dan konfirmasi penerimaan transfer stok antar outlet'),
('inventory:transfer_cancel', 'Otorisasi pembatalan mutasi transfer stok barang'),
('inventory:batch_tracking', 'Melihat pelacakan alokasi batch stok penjualan ke pembelian dan transfer stok (FEFO/FIFO)'),
('inventory:all_ownership', 'Mengakses seluruh data persediaan di semua gudang dan outlet cabang'),

-- Pembelian & Pengadaan (Purchasing & Procurement)
('purchasing:read', 'Melihat data pesanan pembelian (PO), penerimaan barang (GRN), dan faktur supplier'),
('purchasing:create', 'Membuat pesanan pembelian (PO), bukti terima barang (GRN), dan faktur supplier baru'),
('purchasing:update', 'Mengubah pesanan pembelian dan memproses penerimaan barang ke faktur pembelian'),
('purchasing:write', 'Menulis dan memproses dokumen transaksi pembelian'),
('purchasing:void', 'Otorisasi pembatalan atau void pesanan pembelian, GRN, dan faktur pembelian'),
('purchasing:pay', 'Melakukan pelunasan dan pembayaran utang faktur pembelian supplier'),
('purchasing:export', 'Mengekspor data pesanan pembelian (PO), GRN, faktur, dan retur supplier ke Excel (.xlsx)'),
('purchasing:all_ownership', 'Mengakses seluruh dokumen pembelian dari seluruh outlet cabang'),

-- Biaya Operasional & Pendapatan Lain (Expenses & Other Income)
('expenses:read', 'Melihat riwayat pencatatan transaksi biaya dan beban operasional toko'),
('expenses:create', 'Mencatat transaksi pengeluaran biaya dan kas operasional toko'),
('expenses:update', 'Mengubah data kategori atau keterangan transaksi pengeluaran operasional'),
('expenses:delete', 'Membatalkan atau menghapus transaksi pengeluaran operasional'),
('expenses:categories', 'Mengelola master kategori beban dan pos pengeluaran operasional'),
('expenses:export', 'Mengekspor data transaksi pengeluaran kas operasional toko ke Excel (.xlsx)'),
('expenses:all_ownership', 'Melihat dan mengelola transaksi beban pengeluaran dari seluruh outlet'),
('other_income:read', 'Melihat daftar transaksi pendapatan dan pemasukan lain-lain di luar penjualan retail'),
('other_income:create', 'Mencatat transaksi pemasukan non-operasional/pendapatan lain ke kas atau bank'),
('other_income:delete', 'Membatalkan atau menghapus transaksi pendapatan lain yang keliru'),
('other_income:categories', 'Mengelola master kategori pendapatan lain-lain'),

-- Kas, Bank & Modal Usaha (Cash, Bank & Capital)
('capital:read', 'Melihat riwayat transaksi setoran dan penarikan modal pemilik usaha'),
('capital:create', 'Mencatat transaksi penambahan modal disetor ke kas atau rekening bank usaha'),
('capital:delete', 'Membatalkan atau menghapus entri transaksi setoran modal'),
('bank:read', 'Melihat daftar rekening kas dan bank usaha beserta saldo mutasi'),
('bank:create', 'Menambahkan rekening bank usaha baru'),
('bank:update', 'Mengubah informasi nomor rekening dan nama bank usaha'),
('bank:delete', 'Menonaktifkan rekening kas dan bank usaha'),

-- Promosi, Voucher, Banner & Loyalitas (Promotions, Vouchers & Loyalty)
('promotions:read', 'Melihat daftar promosi diskon retail, flash sale, dan promo bertingkat'),
('promotions:create', 'Membuat promosi diskon, flash sale, atau bundling baru'),
('promotions:update', 'Mengubah jadwal, besaran diskon, dan produk yang dipromosikan'),
('promotions:delete', 'Menghapus atau membatalkan promosi toko'),
('promotions:write', 'Menulis dan mengatur aturan promosi toko'),
('vouchers:read', 'Melihat daftar kupon voucher diskon dan memeriksa validitas kode voucher kasir'),
('vouchers:create', 'Membuat kode kupon voucher diskon promosi toko baru'),
('vouchers:update', 'Mengubah syarat, nominal, dan ketentuan kupon voucher diskon'),
('vouchers:delete', 'Menghapus kupon voucher diskon promosi toko'),
('vouchers:manage', 'Mengelola master kupon voucher diskon dan alokasi kuota promosi'),
('vouchers:claims', 'Melihat dan memverifikasi data klaim kupon voucher pelanggan'),
('display_banners:read', 'Melihat daftar banner promosi pada layar kedua (Customer Display Screen)'),
('display_banners:create', 'Mengunggah banner promosi baru untuk layar kedua kasir'),
('display_banners:update', 'Mengubah status aktif dan urutan tampilan banner promosi layar kedua'),
('display_banners:delete', 'Menghapus banner promosi layar kedua kasir'),
('loyalty:read', 'Melihat informasi program loyalitas, riwayat poin, dan kartu pelanggan'),
('loyalty:manage', 'Mengatur rasio perolehan poin belanja dan aturan penukaran poin member'),
('loyalty:adjust', 'Melakukan koreksi atau penyesuaian manual saldo poin member pelanggan'),
('loyalty:redeem', 'Memproses penukaran poin member menjadi potongan belanja di kasir'),

-- Master Produk & Katalog (Products & Master Data)
('products:read', 'Melihat katalog produk, daftar harga, dan ketersediaan stok barang'),
('products:create', 'Menambahkan produk dan varian barang baru ke dalam katalog master'),
('products:update', 'Mengubah informasi produk, satuan, harga jual, dan spesifikasi barang'),
('products:delete', 'Menghapus atau menonaktifkan produk dari katalog master barang'),
('products:barcodes', 'Menambah, mengubah, menghapus multi-barcode produk dan cetak barcode'),
('products:outlet_prices', 'Mengatur dan mengubah harga jual khusus produk per outlet gerai'),
('products:customer_group_prices', 'Mengatur harga khusus grup pelanggan dan member VIP per outlet'),
('products:price_tiers', 'Mengatur harga grosir bertingkat (price tiers) berdasarkan volume kuantitas'),
('products:import', 'Mengimpor data katalog produk dan mengunggah foto produk secara masal'),
('products:export', 'Mengekspor katalog master produk ke format file Excel (.xlsx)'),
('customers:read', 'Melihat data profil pelanggan, grup member, limit piutang, dan riwayat belanja'),
('customers:create', 'Mendaftarkan pelanggan baru atau grup klasifikasi member pelanggan'),
('customers:update', 'Mengubah informasi kontak, data profil, dan limit kredit pelanggan'),
('customers:delete', 'Menghapus atau menonaktifkan data pelanggan dari master sistem'),
('customers:manage', 'Mengelola data master pelanggan, limit kredit piutang, dan grup member'),
('suppliers:read', 'Melihat data master pemasok, kontak perwakilan, dan riwayat faktur supplier'),
('suppliers:create', 'Menambahkan data pemasok/supplier baru ke dalam sistem'),
('suppliers:update', 'Mengubah informasi alamat, telepon, dan syarat pembayaran supplier'),
('suppliers:delete', 'Menghapus data pemasok/supplier dari master sistem'),
('categories:read', 'Melihat daftar kategori klasifikasi produk barang'),
('categories:create', 'Menambahkan kategori klasifikasi produk baru'),
('categories:update', 'Mengubah nama dan informasi kategori produk'),
('categories:delete', 'Menghapus kategori produk dari master sistem'),
('brands:read', 'Melihat daftar merek/brand manufaktur produk'),
('brands:create', 'Menambahkan merek/brand produk baru'),
('brands:update', 'Mengubah nama dan informasi merek/brand produk'),
('brands:delete', 'Menghapus merek/brand produk dari master sistem'),
('units:read', 'Melihat daftar satuan ukuran dan konversi takaran barang'),
('units:create', 'Menambahkan satuan ukuran barang baru'),
('units:update', 'Mengubah nama dan simbol satuan ukuran barang'),
('units:delete', 'Menghapus satuan ukuran barang dari master sistem'),
('outlets:read', 'Melihat daftar gerai cabang, profil toko, dan alamat outlet'),
('outlets:create', 'Menambahkan gerai cabang atau outlet toko baru'),
('outlets:update', 'Mengubah alamat, telepon, dan informasi konfigurasi outlet toko'),
('outlets:delete', 'Menonaktifkan atau menghapus data gerai cabang outlet toko'),
('outlets:manage', 'Mengelola master gerai cabang dan konfigurasi operasional toko'),
('master:read', 'Melihat master data katalog dan konfigurasi umum sistem'),
('master:write', 'Mengubah master data katalog dan parameter umum sistem'),

-- Akuntansi & Jurnal Umum (Accounting & GL)
('accounting:read', 'Melihat bagan akun (COA), buku besar, neraca saldo, dan laba rugi'),
('accounting:create', 'Mencatat entri jurnal akuntansi manual dan menambah akun bagan akun (COA) baru'),
('accounting:update', 'Mengubah bagan akun (COA) dan parameter pembukuan akuntansi'),
('accounting:write', 'Membuat entri jurnal manual dan mutasi pembukuan'),
('accounting:close_period', 'Melakukan proses tutup buku periode akuntansi dan penguncian laporan'),
('accounting:reopen_period', 'Otorisasi pembukaan kembali periode akuntansi yang telah ditutup'),
('accounting:periods:read', 'Melihat status pembukaan dan penutupan periode kalender akuntansi'),
('accounting:periods:close', 'Melakukan penutupan operasional (Soft-Close) pada periode akuntansi'),
('accounting:periods:reopen', 'Membuka kembali periode akuntansi yang telah ditutup untuk revisi jurnal'),
('accounting:hard_close', 'Melakukan penutupan permanen (Hard-Close) periode akuntansi (Audit Final)'),
('accounting:fund_transfer', 'Melakukan mutasi pemindahan dana antar kantor cabang (Rekening Antar Kantor)'),
('accounting:dividend:read', 'Melihat riwayat pembagian dividen laba ditahan kepada pemilik usaha'),
('accounting:dividend:create', 'Mencatat transaksi penetapan dan pembayaran dividen dari laba bersih periode'),
('accounting:dividend:delete', 'Membatalkan pencatatan pembagian dividen yang keliru'),
('accounting:all_ownership', 'Mengakses seluruh jurnal dan laporan keuangan dari seluruh outlet cabang'),

-- Laporan & Analitik Bisnis (Reports & Analytics)
('report:read', 'Melihat seluruh laporan penjualan, pembelian, stok, dan kas operasional'),
('report:view', 'Mengakses menu laporan analitik dan ringkasan operasional toko'),
('analytics:read', 'Mengakses dasbor eksekutif, grafik tren finansial, metrik penjualan, pembelian, beban, dan laba operasional'),

-- Pengguna & Hak Akses (Users & Security)
('users:read', 'Melihat daftar akun staf, profil pengguna, daftar peran (roles), dan matriks perizinan'),
('users:create', 'Mendaftarkan akun staf pengguna baru atau membuat peran (role) baru'),
('users:update', 'Mengubah data akun staf pengguna, peran penugasan, dan hak akses permissions peran'),
('users:delete', 'Menonaktifkan atau menghapus akun staf pengguna dan peran dari sistem'),
('users:change_password', 'Mengubah atau mereset kata sandi login staf pengguna lain'),
('users:change_pin', 'Mengubah atau mereset nomor PIN supervisor/kasir untuk otorisasi cepat'),
('users:impersonate', 'Masuk sementara sebagai pengguna staf lain untuk keperluan audit/bantuan teknis'),
('users:manage', 'Mengelola seluruh akun staf, penetapan cabang, dan matriks hak akses peran'),
('supervisor:biometric_override', 'Melakukan otorisasi transaksi kasir (void, diskon, buka laci) dengan sidik jari biometrik'),
('supervisor:biometric_enroll', 'Mendaftarkan data sidik jari biometrik staf supervisor toko'),
('settings:read', 'Melihat pengaturan sistem toko, format struk belanja, dan kebijakan operasional'),
('settings:update', 'Mengubah konfigurasi toko, format struk kasir, logo, dan preferensi sistem'),
('settings:manage', 'Mengatur konfigurasi sistem toko dan desain cetak struk'),
('settings:biometric', 'Mengonfigurasi setelan perangkat pembaca biometrik dan ambang batas sensitivitas'),
('audit:read', 'Melihat log aktivitas sistem, jejak audit keamanan, dan rekaman perubahan data'),
('audit:manage', 'Mengonfigurasi kebijakan retensi log audit dan arsip keamanan'),
('audit:cleanup', 'Membersihkan arsip log audit yang telah melampaui batas retensi kedaluwarsa'),
('database:backup', 'Mengekspor cadangan database lengkap ke berkas ZIP (SQL & Manifest Kompatibilitas)'),
('database:restore', 'Memulihkan database dari berkas ZIP cadangan SQL setelah validasi kompatibilitas versi'),
('system:license_sync', 'Melakukan sinkronisasi lisensi retail dengan server lisensi Notane'),
('system:license_logs', 'Melihat riwayat log koneksi dan ketahanan verifikasi lisensi offline')
ON CONFLICT (code) DO UPDATE SET
    description = EXCLUDED.description;


-- ============================================================================
-- 3. PEMETAAN PERMISSION KE PERAN (ROLE-BASED ACCESS CONTROL)
-- ============================================================================

-- A. Role: SUPERADMIN (Akses Penuh 100% Tanpa Pengecualian)
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r, permissions p
WHERE UPPER(r.name) = 'SUPERADMIN'
ON CONFLICT DO NOTHING;


-- B. Role: STORE_MANAGER (Kepala Toko / General Store Manager)
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r, permissions p
WHERE UPPER(r.name) = 'STORE_MANAGER' AND p.code IN (
    -- Penjualan & Otorisasi POS
    'pos:checkout', 'pos:void', 'pos:discount', 'pos:open_drawer', 'pos:hold_recall',
    'sales:read', 'sales:create', 'sales:void', 'sales:discount', 'sales:open_drawer', 'sales:export', 'sales:all_ownership',
    'sales_returns:read', 'sales_returns:create', 'sales_returns:approve', 'sales_returns:reject',
    
    -- Shift & Kas
    'shift:read', 'shift:open', 'shift:close', 'shift:view',
    'cash_drops:read', 'cash_drops:create',

    -- Pembelian & Inventori Lengkap
    'purchasing:read', 'purchasing:create', 'purchasing:update', 'purchasing:write', 'purchasing:void', 'purchasing:pay', 'purchasing:export', 'purchasing:all_ownership',
    'inventory:read', 'inventory:write', 'inventory:adjust', 'inventory:transfer', 'inventory:transfer_cancel', 'inventory:batch_tracking', 'inventory:all_ownership',

    -- Biaya, Pendapatan Lain, Kas & Bank
    'expenses:read', 'expenses:create', 'expenses:update', 'expenses:delete', 'expenses:categories', 'expenses:export', 'expenses:all_ownership',
    'other_income:read', 'other_income:create', 'other_income:delete', 'other_income:categories',
    'capital:read',
    'bank:read', 'bank:create', 'bank:update',

    -- Promosi, Voucher, Banner & Loyalty
    'promotions:read', 'promotions:create', 'promotions:update', 'promotions:delete', 'promotions:write',
    'vouchers:read', 'vouchers:create', 'vouchers:update', 'vouchers:delete', 'vouchers:manage', 'vouchers:claims',
    'display_banners:read', 'display_banners:create', 'display_banners:update', 'display_banners:delete',
    'loyalty:read', 'loyalty:manage', 'loyalty:adjust', 'loyalty:redeem',

    -- Master Data Lengkap
    'products:read', 'products:create', 'products:update', 'products:delete', 'products:barcodes', 'products:outlet_prices', 'products:customer_group_prices', 'products:price_tiers', 'products:import', 'products:export',
    'customers:read', 'customers:create', 'customers:update', 'customers:delete', 'customers:manage',
    'suppliers:read', 'suppliers:create', 'suppliers:update', 'suppliers:delete',
    'categories:read', 'categories:create', 'categories:update', 'categories:delete',
    'brands:read', 'brands:create', 'brands:update', 'brands:delete',
    'units:read', 'units:create', 'units:update', 'units:delete',
    'outlets:read', 'outlets:update', 'outlets:manage',
    'master:read', 'master:write',

    -- Laporan & Analitik Bisnis
    'report:read', 'report:view',
    'analytics:read',

    -- Akuntansi Operasional Toko
    'accounting:read', 'accounting:close_period', 'accounting:periods:read', 'accounting:periods:close', 'accounting:all_ownership',

    -- Supervisi & Keamanan Toko
    'supervisor:biometric_override', 'supervisor:biometric_enroll',
    'users:read', 'users:change_pin',
    'settings:read', 'settings:update', 'settings:manage',
    'audit:read',
    'database:backup'
)
ON CONFLICT DO NOTHING;


-- C. Role: ASSISTANT_MANAGER (Asisten Kepala Toko / Deputy Store Manager)
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r, permissions p
WHERE UPPER(r.name) = 'ASSISTANT_MANAGER' AND p.code IN (
    -- Penjualan & Otorisasi POS
    'pos:checkout', 'pos:void', 'pos:discount', 'pos:open_drawer', 'pos:hold_recall',
    'sales:read', 'sales:create', 'sales:void', 'sales:discount', 'sales:open_drawer', 'sales:export', 'sales:all_ownership',
    'sales_returns:read', 'sales_returns:create', 'sales_returns:approve', 'sales_returns:reject',

    -- Shift & Kas
    'shift:read', 'shift:open', 'shift:close', 'shift:view',
    'cash_drops:read', 'cash_drops:create',

    -- Inventori & Penerimaan Barang
    'inventory:read', 'inventory:write', 'inventory:adjust', 'inventory:transfer', 'inventory:transfer_cancel', 'inventory:batch_tracking',
    'purchasing:read', 'purchasing:create', 'purchasing:update', 'purchasing:write', 'purchasing:export',

    -- Operasional Kas & Biaya Harian
    'expenses:read', 'expenses:create', 'expenses:categories', 'expenses:export',
    'other_income:read', 'other_income:create', 'other_income:categories',
    'bank:read',

    -- Promosi, Display & Member
    'promotions:read', 'promotions:create', 'promotions:update',
    'vouchers:read', 'vouchers:create', 'vouchers:update', 'vouchers:claims',
    'display_banners:read', 'display_banners:create', 'display_banners:update',
    'loyalty:read', 'loyalty:redeem',

    -- Master Data Operasional
    'products:read', 'products:barcodes', 'products:outlet_prices', 'products:price_tiers', 'products:export',
    'customers:read', 'customers:create', 'customers:update',
    'suppliers:read',
    'categories:read', 'brands:read', 'units:read', 'outlets:read',
    'master:read',

    -- Laporan & Analitik
    'report:read', 'report:view',
    'analytics:read',

    -- Supervisi & Otorisasi
    'supervisor:biometric_override', 'supervisor:biometric_enroll',
    'users:read', 'users:change_pin'
)
ON CONFLICT DO NOTHING;


-- D. Role: SUPERVISOR (Supervisor Shift / Floor Leader)
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r, permissions p
WHERE UPPER(r.name) = 'SUPERVISOR' AND p.code IN (
    -- Otorisasi Kasir & POS
    'pos:checkout', 'pos:void', 'pos:discount', 'pos:open_drawer', 'pos:hold_recall',
    'sales:read', 'sales:create', 'sales:void', 'sales:discount', 'sales:open_drawer',
    'sales_returns:read', 'sales_returns:create', 'sales_returns:approve', 'sales_returns:reject',

    -- Shift & Kas
    'shift:read', 'shift:open', 'shift:close', 'shift:view',
    'cash_drops:read', 'cash_drops:create',

    -- Penyesuaian Stok & Opname Fisik
    'inventory:read', 'inventory:adjust', 'inventory:transfer', 'inventory:batch_tracking',
    'expenses:read', 'expenses:create',

    -- Promo, Banner & Member
    'promotions:read',
    'vouchers:read', 'vouchers:claims',
    'display_banners:read',
    'loyalty:read', 'loyalty:redeem',

    -- Master Data Read & Barcode
    'products:read', 'products:barcodes',
    'customers:read', 'customers:create', 'customers:update',
    'suppliers:read', 'categories:read', 'brands:read', 'units:read', 'outlets:read',
    'master:read',

    -- Otorisasi Biometrik & Laporan
    'supervisor:biometric_override', 'supervisor:biometric_enroll',
    'report:read', 'report:view',
    'analytics:read'
)
ON CONFLICT DO NOTHING;


-- E. Role: CASHIER (Kasir Penjualan / Front-liner)
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r, permissions p
WHERE UPPER(r.name) = 'CASHIER' AND p.code IN (
    -- Transaksi POS
    'pos:checkout', 'pos:hold_recall',
    'sales:read', 'sales:create',
    'sales_returns:read', 'sales_returns:create',

    -- Shift & Kas Laci
    'shift:read', 'shift:open', 'shift:close', 'shift:view',
    'cash_drops:read', 'cash_drops:create',

    -- Biaya Mendesak & Master Produk/Member
    'expenses:read', 'expenses:create',
    'products:read',
    'customers:read', 'customers:create',
    'vouchers:read', 'vouchers:claims',
    'display_banners:read',
    'loyalty:read', 'loyalty:redeem'
)
ON CONFLICT DO NOTHING;


-- F. Role: WAREHOUSE (Staf Gudang & Logistik / Receiving)
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r, permissions p
WHERE UPPER(r.name) = 'WAREHOUSE' AND p.code IN (
    -- Arus Barang & Stok
    'inventory:read', 'inventory:write', 'inventory:adjust', 'inventory:transfer', 'inventory:transfer_cancel', 'inventory:batch_tracking', 'inventory:all_ownership',

    -- Penerimaan Barang (GRN)
    'purchasing:read', 'purchasing:create', 'purchasing:update', 'purchasing:write', 'purchasing:export',

    -- Master Produk & Barcode
    'products:read', 'products:barcodes', 'products:export',
    'suppliers:read', 'categories:read', 'brands:read', 'units:read',
    'master:read'
)
ON CONFLICT DO NOTHING;


-- G. Role: PURCHASING (Staf Pengadaan / Bagian Pembelian / Buyer)
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r, permissions p
WHERE UPPER(r.name) = 'PURCHASING' AND p.code IN (
    -- Pembelian & Pengadaan Lengkap
    'purchasing:read', 'purchasing:create', 'purchasing:update', 'purchasing:write', 'purchasing:void', 'purchasing:pay', 'purchasing:export', 'purchasing:all_ownership',

    -- Cek Stok & Batch Expiry
    'inventory:read', 'inventory:batch_tracking',

    -- Master Pemasok & Katalog Produk
    'suppliers:read', 'suppliers:create', 'suppliers:update', 'suppliers:delete',
    'products:read', 'products:export',
    'categories:read', 'brands:read', 'units:read', 'outlets:read',
    'master:read',

    -- Laporan
    'report:read', 'report:view',
    'analytics:read'
)
ON CONFLICT DO NOTHING;


-- H. Role: ACCOUNTING (Staf Akuntansi & Keuangan / Finance & Tax)
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r, permissions p
WHERE UPPER(r.name) = 'ACCOUNTING' AND p.code IN (
    -- Akuntansi & Jurnal Umum Lengkap
    'accounting:read', 'accounting:create', 'accounting:update', 'accounting:write',
    'accounting:close_period', 'accounting:reopen_period',
    'accounting:periods:read', 'accounting:periods:close', 'accounting:periods:reopen', 'accounting:hard_close',
    'accounting:fund_transfer',
    'accounting:dividend:read', 'accounting:dividend:create', 'accounting:dividend:delete',
    'accounting:all_ownership',

    -- Kas, Bank & Modal
    'bank:read', 'bank:create', 'bank:update', 'bank:delete',
    'capital:read', 'capital:create', 'capital:delete',

    -- Biaya Operasional & Pendapatan Lain
    'expenses:read', 'expenses:create', 'expenses:update', 'expenses:delete', 'expenses:categories', 'expenses:export', 'expenses:all_ownership',
    'other_income:read', 'other_income:create', 'other_income:delete', 'other_income:categories',

    -- Settlement Utang & Piutang
    'purchasing:read', 'purchasing:pay', 'purchasing:export',
    'sales:read', 'sales:export',

    -- Master Data Referensi & Laporan
    'customers:read', 'suppliers:read', 'outlets:read',
    'master:read',
    'report:read', 'report:view',
    'analytics:read'
)
ON CONFLICT DO NOTHING;


-- I. Role: INTERNAL_AUDIT (Auditor Internal / Loss Prevention / Pengawas Mutu & Kepatuhan)
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r, permissions p
WHERE UPPER(r.name) = 'INTERNAL_AUDIT' AND p.code IN (
    -- Jejak Audit Sistem
    'audit:read',

    -- Audit Penjualan, Void, Retur & Kas Laci
    'sales:read', 'sales:export', 'sales:all_ownership',
    'sales_returns:read',
    'shift:read', 'shift:view',
    'cash_drops:read',

    -- Audit Persediaan & Selisih Stok (Shrinkage)
    'inventory:read', 'inventory:batch_tracking', 'inventory:all_ownership',

    -- Audit Pembelian & Penerimaan Barang
    'purchasing:read', 'purchasing:export', 'purchasing:all_ownership',

    -- Audit Biaya Operasional & Keuangan
    'expenses:read', 'expenses:export', 'expenses:all_ownership',
    'other_income:read',
    'bank:read', 'capital:read',
    'accounting:read', 'accounting:periods:read', 'accounting:dividend:read', 'accounting:all_ownership',

    -- Referensi Master Data
    'products:read', 'products:export',
    'customers:read', 'suppliers:read', 'categories:read', 'brands:read', 'units:read', 'outlets:read',
    'master:read',

    -- Laporan & Analitik
    'report:read', 'report:view',
    'analytics:read'
)
ON CONFLICT DO NOTHING;


-- J. Role: MERCHANDISER (Merchandiser / Tim Promosi & Display Toko)
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r, permissions p
WHERE UPPER(r.name) = 'MERCHANDISER' AND p.code IN (
    -- Master Produk, Harga Grosir & Barcode
    'products:read', 'products:create', 'products:update', 'products:barcodes',
    'products:outlet_prices', 'products:customer_group_prices', 'products:price_tiers', 'products:export',
    'categories:read', 'categories:create', 'categories:update',
    'brands:read', 'brands:create', 'brands:update',
    'units:read',
    'master:read',

    -- Promosi, Flash Sale, Voucher & Layar Banner
    'promotions:read', 'promotions:create', 'promotions:update', 'promotions:delete', 'promotions:write',
    'display_banners:read', 'display_banners:create', 'display_banners:update', 'display_banners:delete',
    'vouchers:read', 'vouchers:create', 'vouchers:update', 'vouchers:manage', 'vouchers:claims',

    -- Pelanggan & Loyalty
    'customers:read', 'loyalty:read', 'loyalty:manage',

    -- Stok & Laporan
    'inventory:read', 'inventory:batch_tracking',
    'report:read', 'report:view'
)
ON CONFLICT DO NOTHING;

