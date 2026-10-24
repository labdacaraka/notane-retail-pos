-- ============================================================================
-- MIGRASI 029: PERMISSION LENGKAP UNTUK AKSI PEMBELIAN, PRODUK, BARCODE, HARGA & INVENTORI
-- ============================================================================

-- 1. Tambahkan permission untuk setiap aksi spesifik (Granular Action Permissions)
INSERT INTO permissions (code, description) VALUES
-- Purchasing (Pembelian) Actions
('purchasing:void', 'Otorisasi pembatalan atau void PO, penerimaan barang (GRN), dan faktur pembelian'),
('purchasing:pay', 'Melakukan pelunasan dan pembayaran utang faktur pembelian supplier'),

-- Product Pricing, Barcodes & Import Actions
('products:outlet_prices', 'Mengatur dan mengubah harga jual khusus produk per outlet gerai'),
('products:customer_group_prices', 'Mengatur harga khusus grup pelanggan dan member VIP per outlet'),
('products:barcodes', 'Menambah, mengubah, menghapus multi-barcode produk dan barcode utama'),
('products:price_tiers', 'Mengatur harga grosir bertingkat (price tiers) berdasarkan volume kuantitas'),
('products:import', 'Mengimpor data katalog produk dan mengunggah foto produk secara masal'),

-- Customers & Customer Groups
('customers:manage', 'Mengelola data master pelanggan, limit kredit piutang, dan grup member'),

-- Vouchers & Promotions
('vouchers:manage', 'Mengelola master kupon voucher diskon dan promosi toko'),

-- Inventory Transfers & Cancellations
('inventory:transfer', 'Membuat mutasi pengiriman dan konfirmasi penerimaan transfer stok antar outlet'),
('inventory:transfer_cancel', 'Otorisasi pembatalan mutasi transfer stok barang'),

-- Accounting Period Closing & Reopen
('accounting:close_period', 'Melakukan proses tutup buku periode akuntansi dan penguncian laporan'),
('accounting:reopen_period', 'Otorisasi pembukaan kembali periode akuntansi yang telah ditutup')
ON CONFLICT (code) DO NOTHING;

-- 2. Petakan ke Role SUPERADMIN (Seluruh Hak Akses)
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p
WHERE r.name = 'SUPERADMIN' AND p.code IN (
    'purchasing:void', 'purchasing:pay',
    'products:outlet_prices', 'products:customer_group_prices', 'products:barcodes',
    'products:price_tiers', 'products:import',
    'customers:manage', 'vouchers:manage',
    'inventory:transfer', 'inventory:transfer_cancel',
    'accounting:close_period', 'accounting:reopen_period'
)
ON CONFLICT DO NOTHING;

-- 3. Petakan ke Role STORE_MANAGER
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p
WHERE r.name = 'STORE_MANAGER' AND p.code IN (
    'purchasing:void', 'purchasing:pay',
    'products:outlet_prices', 'products:customer_group_prices', 'products:barcodes',
    'products:price_tiers', 'products:import',
    'customers:manage', 'vouchers:manage',
    'inventory:transfer', 'inventory:transfer_cancel',
    'accounting:close_period', 'accounting:reopen_period'
)
ON CONFLICT DO NOTHING;

-- 4. Petakan ke Role SUPERVISOR
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p
WHERE r.name = 'SUPERVISOR' AND p.code IN (
    'purchasing:void', 'purchasing:pay',
    'products:outlet_prices', 'products:customer_group_prices', 'products:barcodes',
    'products:price_tiers',
    'customers:manage', 'vouchers:manage',
    'inventory:transfer', 'inventory:transfer_cancel'
)
ON CONFLICT DO NOTHING;

-- 5. Petakan ke Role WAREHOUSE (Gudang & Logistik)
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p
WHERE r.name = 'WAREHOUSE' AND p.code IN (
    'products:barcodes',
    'inventory:transfer'
)
ON CONFLICT DO NOTHING;
