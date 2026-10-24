-- ============================================================================
-- SEED DATA AWAL: SISTEM POS RETAIL ENTERPRISE
-- ============================================================================

-- 1. Roles
INSERT INTO roles (name, description) VALUES
('SUPERADMIN', 'Akses penuh ke seluruh sistem dan konfigurasi back-office'),
('STORE_MANAGER', 'Manajer Toko dengan akses laporan dan otorisasi tingkat tinggi'),
('SUPERVISOR', 'Supervisor gerai dengan hak otorisasi void, diskon, dan buka laci'),
('CASHIER', 'Kasir operasional untuk transaksi penjualan POS'),
('WAREHOUSE', 'Staf gudang untuk penerimaan barang dan stock opname')
ON CONFLICT (name) DO NOTHING;

-- 2. Permissions
INSERT INTO permissions (code, description) VALUES
('pos:checkout', 'Melakukan transaksi penjualan checkout'),
('pos:void', 'Otorisasi pembatalan barang atau nota (Void)'),
('pos:discount', 'Pemberian diskon manual atau price override'),
('pos:open_drawer', 'Membuka cash drawer tanpa transaksi'),
('pos:hold_recall', 'Menahan dan memanggil transaksi tertunda'),
('shift:open', 'Membuka shift kasir baru'),
('shift:close', 'Melakukan blind settlement tutup shift'),
('shift:view', 'Melihat riwayat shift kasir'),
('inventory:read', 'Melihat data stok dan kartu stok'),
('inventory:write', 'Menerima barang masuk dan mutasi stok'),
('inventory:adjust', 'Melakukan penyesuaian stock opname'),
('purchasing:read', 'Melihat data PO dan invoice supplier'),
('purchasing:write', 'Membuat PO dan Good Receipt Note'),
('accounting:read', 'Melihat bagan akun, jurnal, dan laba rugi'),
('accounting:write', 'Membuat entri jurnal manual'),
('report:view', 'Mengakses seluruh laporan analitik'),
('master:read', 'Melihat master data produk dan pelanggan'),
('master:write', 'Mengubah master data produk, harga, dan supplier'),
('users:manage', 'Mengelola akun staf dan matriks hak akses peran'),
('settings:manage', 'Mengatur konfigurasi sistem toko dan desain cetak struk'),
('audit:read', 'Melihat log aktivitas sistem dan jejak audit keamanan')
ON CONFLICT (code) DO NOTHING;

-- 3. Role Permissions Mapping
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p WHERE r.name = 'SUPERADMIN'
ON CONFLICT DO NOTHING;

INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p 
WHERE r.name = 'SUPERVISOR' AND p.code IN (
    'pos:checkout', 'pos:void', 'pos:discount', 'pos:open_drawer', 'pos:hold_recall',
    'shift:open', 'shift:close', 'shift:view', 'inventory:read', 'master:read', 'report:view'
)
ON CONFLICT DO NOTHING;

INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p 
WHERE r.name = 'CASHIER' AND p.code IN (
    'pos:checkout', 'pos:hold_recall', 'shift:open', 'shift:close', 'master:read'
)
ON CONFLICT DO NOTHING;

INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p 
WHERE r.name = 'WAREHOUSE' AND p.code IN (
    'inventory:read', 'inventory:write', 'inventory:adjust', 'purchasing:read', 'purchasing:write', 'master:read'
)
ON CONFLICT DO NOTHING;

-- 4. Initial Users
-- Password hash & PIN hash menggunakan format Argon2id standar:
-- admin123 hash: $argon2id$v=19$m=65536,t=2,p=1$7x2lY9qI72uP6+ZzK0w4EA$X21qV+89eXqS5M72p4nKqjZ2g1rG2w3s4d5f6g7h8i9
-- 7. Initial Users
-- Passwords:
-- admin: admin123 (PIN: 123456)
-- spv: spv123 (PIN: 123456)
-- kasir1: kasir123 (PIN: 111111)
INSERT INTO users (role_id, username, password_hash, pin_hash, full_name, email, phone)
SELECT 
    r.id,
    'admin',
    '$argon2id$v=19$m=19456,t=2,p=1$XnId5RRM/tk8BDPJssaHpA$mOYRhiZhM5/oCvIwVP8hTREGluAnqBXKonJtFTjFPFw',
    '$argon2id$v=19$m=19456,t=2,p=1$S1yuWPST6JXi2aYgXsKNlA$s+pjjBPS+dDVHDfc1Scezjr8tXMUTBSLmstjitjfhmU',
    'Administrator Utama',
    'admin@notane.retail',
    '081234567890'
FROM roles r WHERE r.name = 'SUPERADMIN'
ON CONFLICT (username) DO NOTHING;

INSERT INTO users (role_id, username, password_hash, pin_hash, full_name, email, phone)
SELECT 
    r.id,
    'spv',
    '$argon2id$v=19$m=19456,t=2,p=1$pqOQROjc75D8mwWXSf8+pA$nBKZIDb5UvIjQEigU3Tljc3APzrxYHYlw51XanbKY+Q',
    '$argon2id$v=19$m=19456,t=2,p=1$S1yuWPST6JXi2aYgXsKNlA$s+pjjBPS+dDVHDfc1Scezjr8tXMUTBSLmstjitjfhmU',
    'Supervisor Toko',
    'spv@notane.retail',
    '081234567891'
FROM roles r WHERE r.name = 'SUPERVISOR'
ON CONFLICT (username) DO NOTHING;

INSERT INTO users (role_id, username, password_hash, pin_hash, full_name, email, phone)
SELECT 
    r.id,
    'kasir1',
    '$argon2id$v=19$m=19456,t=2,p=1$dX4rSaxA0g69YxayRr6PHA$lz+Zzw2zDwpnGQ7Ge3d40HF3qzAivNb642nFw7FsQn4',
    '$argon2id$v=19$m=19456,t=2,p=1$UjO4qtBP7yP3qdBJVNgJAw$cH1aIH+1k9at9bBgdmGE1OnTEH7o3go+DcJNaG9DrrA',
    'Siti Kasir Utama',
    'kasir1@notane.retail',
    '081234567892'
FROM roles r WHERE r.name = 'CASHIER'
ON CONFLICT (username) DO NOTHING;

-- 5. Outlets & Warehouses
INSERT INTO outlets (id, code, name, address, phone) VALUES
('b0000000-0000-0000-0000-000000000001'::uuid, 'OUT-001', 'Notane Retail Flagship Store', 'Jl. Sudirman No. 123, Jakarta', '021-5551234')
ON CONFLICT (code) DO NOTHING;

INSERT INTO outlets (id, code, name, is_warehouse) VALUES
('b0000000-0000-0000-0000-000000000002'::uuid, 'WH-001', 'Gudang Utama Toko', TRUE)
ON CONFLICT (code) DO NOTHING;

INSERT INTO warehouses (id, outlet_id, code, name) VALUES
('b0000000-0000-0000-0000-000000000002'::uuid, 'b0000000-0000-0000-0000-000000000001'::uuid, 'WH-001', 'Gudang Utama Toko')
ON CONFLICT (code) DO NOTHING;

-- 6. Categories
INSERT INTO categories (id, code, name) VALUES
('c0000000-0000-0000-0000-000000000001'::uuid, 'CAT-FNB', 'Makanan & Minuman'),
('c0000000-0000-0000-0000-000000000002'::uuid, 'CAT-GROC', 'Sembako & Kebutuhan Pokok'),
('c0000000-0000-0000-0000-000000000003'::uuid, 'CAT-CARE', 'Perawatan & Kebersihan'),
('c0000000-0000-0000-0000-000000000004'::uuid, 'CAT-FRESH', 'Sayur & Buah Segar')
ON CONFLICT (code) DO NOTHING;

-- 7. Units
INSERT INTO units (id, code, name, is_fractional) VALUES
('d0000000-0000-0000-0000-000000000001'::uuid, 'PCS', 'Pieces / Satuan', false),
('d0000000-0000-0000-0000-000000000002'::uuid, 'BOX', 'Box / Kotak', false),
('d0000000-0000-0000-0000-000000000003'::uuid, 'DUS', 'Dus / Karton', false),
('d0000000-0000-0000-0000-000000000004'::uuid, 'KG', 'Kilogram (Timbangan)', true)
ON CONFLICT (code) DO NOTHING;

-- 8. Brands
INSERT INTO brands (id, code, name) VALUES
('e0000000-0000-0000-0000-000000000001'::uuid, 'BRD-INDF', 'Indofood'),
('e0000000-0000-0000-0000-000000000002'::uuid, 'BRD-ULTRA', 'Ultrajaya'),
('e0000000-0000-0000-0000-000000000003'::uuid, 'BRD-FRESH', 'Notane Farm Fresh')
ON CONFLICT (code) DO NOTHING;

-- 9. Payment Methods
INSERT INTO payment_methods (id, code, name, category) VALUES
('f0000000-0000-0000-0000-000000000001'::uuid, 'CASH', 'Uang Tunai (Cash)', 'CASH'),
('f0000000-0000-0000-0000-000000000002'::uuid, 'QRIS', 'QRIS Dinamis', 'QRIS'),
('f0000000-0000-0000-0000-000000000003'::uuid, 'EDC_BCA', 'Debit / Kartu Kredit BCA', 'EDC'),
('f0000000-0000-0000-0000-000000000004'::uuid, 'POINTS', 'Poin Reward Member', 'POINTS')
ON CONFLICT (code) DO NOTHING;

-- 10. Reason Codes
INSERT INTO reason_codes (id, type, code, description) VALUES
('f1000000-0000-0000-0000-000000000001'::uuid, 'VOID', 'VOID_ERR', 'Kesalahan Salah Pindai / Dobel Input'),
('f1000000-0000-0000-0000-000000000002'::uuid, 'VOID', 'VOID_CANCEL', 'Pelanggan Membatalkan Pembelian'),
('f1000000-0000-0000-0000-000000000003'::uuid, 'ADJUSTMENT', 'ADJ_OPNAME', 'Penyesuaian Fisik Stock Opname'),
('f1000000-0000-0000-0000-000000000004'::uuid, 'ADJUSTMENT', 'ADJ_EXP', 'Barang Kedaluwarsa / Rusak'),
('f1000000-0000-0000-0000-000000000005'::uuid, 'DRAWER_OPEN', 'DRAWER_MANUAL', 'Buka Laci Manual Untuk Kembalian')
ON CONFLICT (code) DO NOTHING;

-- 11. Customer Groups & Customers
INSERT INTO customer_groups (id, name, description) VALUES
(1, 'REGULAR', 'Pelanggan reguler umum'),
(2, 'MEMBER', 'Anggota member dengan poin loyalitas'),
(3, 'WHOLESALE', 'Pelanggan grosir / reseller')
ON CONFLICT (id) DO NOTHING;

INSERT INTO customers (id, customer_group_id, code, name, phone, reward_points) VALUES
('c1000000-0000-0000-0000-000000000001'::uuid, 1, 'CUST-REGULAR', 'Pelanggan Umum / Guest', '0000000000', 0),
('c1000000-0000-0000-0000-000000000002'::uuid, 2, 'MBR-001', 'Budi Santoso (Member VIP)', '081299887766', 250)
ON CONFLICT (code) DO NOTHING;

-- 12. Suppliers
INSERT INTO suppliers (id, code, name, contact_person, phone, term_of_payment_days) VALUES
('fa000000-0000-0000-0000-000000000001'::uuid, 'SUP-001', 'PT Indomarco Adi Prima', 'Agus Prasetyo', '021-8889991', 30),
('fa000000-0000-0000-0000-000000000002'::uuid, 'SUP-002', 'PT Ultrajaya Milk Distributor', 'Dewi Lestari', '022-7776662', 14)
ON CONFLICT (code) DO NOTHING;

-- 13. Products & Stocks
INSERT INTO products (
    id, category_id, base_unit_id, sku, barcode, name, brand_id, costing_method, 
    cost_price, base_selling_price, min_stock, max_stock, tax_percent, is_weight_embedded
) VALUES
(
    'fb000000-0000-0000-0000-000000000001'::uuid,
    'c0000000-0000-0000-0000-000000000001'::uuid, -- Makanan
    'd0000000-0000-0000-0000-000000000001'::uuid, -- PCS
    'SKU-INDOMIE-GRG',
    '089686010145',
    'Indomie Goreng Spesial 85g',
    'e0000000-0000-0000-0000-000000000001'::uuid,
    'WMA',
    2800.0000,
    3500.0000,
    20.0000,
    500.0000,
    11.00,
    false
),
(
    'fb000000-0000-0000-0000-000000000002'::uuid,
    'c0000000-0000-0000-0000-000000000001'::uuid, -- Minuman
    'd0000000-0000-0000-0000-000000000001'::uuid, -- PCS
    'SKU-ULTRA-COK-250',
    '899800901022',
    'Ultra Milk Susu UHT Cokelat 250ml',
    'e0000000-0000-0000-0000-000000000002'::uuid,
    'WMA',
    5500.0000,
    7000.0000,
    10.0000,
    200.0000,
    11.00,
    false
),
(
    'fb000000-0000-0000-0000-000000000003'::uuid,
    'c0000000-0000-0000-0000-000000000004'::uuid, -- Buah Segar
    'd0000000-0000-0000-0000-000000000004'::uuid, -- KG
    'SKU-APEL-FUJI',
    '200001000000', -- Barcode prefix timbangan elektrik (2 + 5 digit SKU)
    'Apel Fuji Segar Manis (Timbangan)',
    'e0000000-0000-0000-0000-000000000003'::uuid,
    'WMA',
    32000.0000,
    45000.0000,
    5.0000,
    50.0000,
    0.00,
    true
)
ON CONFLICT (sku) DO NOTHING;

-- Multi-Unit: 1 DUS = 40 PCS Indomie Goreng
INSERT INTO product_units (id, product_id, unit_id, conversion_factor, selling_price, barcode)
VALUES (
    'fc000000-0000-0000-0000-000000000001'::uuid,
    'fb000000-0000-0000-0000-000000000001'::uuid,
    'd0000000-0000-0000-0000-000000000003'::uuid, -- DUS
    40.0000,
    135000.0000,
    '089686010145-DUS'
)
ON CONFLICT (product_id, unit_id) DO NOTHING;

-- Price Tier: Beli Min 5 PCS Indomie Goreng -> Harga Rp 3.300
INSERT INTO product_price_tiers (id, product_id, min_quantity, tier_price)
VALUES (
    'fd000000-0000-0000-0000-000000000001'::uuid,
    'fb000000-0000-0000-0000-000000000001'::uuid,
    5.0000,
    3300.0000
)
ON CONFLICT DO NOTHING;

-- Warehouse Stocks Demo
INSERT INTO warehouse_stocks (warehouse_id, product_id, quantity) VALUES
('b0000000-0000-0000-0000-000000000002'::uuid, 'fb000000-0000-0000-0000-000000000001'::uuid, 150.0000),
('b0000000-0000-0000-0000-000000000002'::uuid, 'fb000000-0000-0000-0000-000000000002'::uuid, 80.0000),
('b0000000-0000-0000-0000-000000000002'::uuid, 'fb000000-0000-0000-0000-000000000003'::uuid, 25.5000)
ON CONFLICT (warehouse_id, product_id) DO UPDATE SET quantity = EXCLUDED.quantity;

-- Voucher Demo
INSERT INTO vouchers (id, code, description, discount_type, discount_value, min_purchase_amount, max_discount_amount, valid_from, valid_until)
VALUES (
    'fe000000-0000-0000-0000-000000000001'::uuid,
    'HEMAT10K',
    'Potongan Langsung Rp 10.000 Belanja Min Rp 50.000',
    'FIXED_AMOUNT',
    10000.0000,
    50000.0000,
    10000.0000,
    CURRENT_TIMESTAMP - INTERVAL '1 day',
    CURRENT_TIMESTAMP + INTERVAL '30 days'
)
ON CONFLICT (code) DO NOTHING;

-- 14. Chart of Accounts & Categories (Double-Entry)
INSERT INTO account_categories (id, name, normal_balance) VALUES
(1, 'ASSET', 'DEBIT'),
(2, 'LIABILITY', 'CREDIT'),
(3, 'EQUITY', 'CREDIT'),
(4, 'REVENUE', 'CREDIT'),
(5, 'EXPENSE', 'DEBIT')
ON CONFLICT (name) DO NOTHING;

INSERT INTO chart_of_accounts (id, account_category_id, code, name, is_header) VALUES
('a1000000-0000-0000-0000-000000000001'::uuid, 1, '1101', 'Kas Operasional Laci Kasir', false),
('a1000000-0000-0000-0000-000000000002'::uuid, 1, '1102', 'Bank & Piutang QRIS Settlement', false),
('a1000000-0000-0000-0000-000000000003'::uuid, 1, '1103', 'Piutang Usaha Pelanggan', false),
('a1000000-0000-0000-0000-000000000004'::uuid, 1, '1201', 'Persediaan Barang Dagang Retail', false),
('a1000000-0000-0000-0000-000000000005'::uuid, 2, '2101', 'Hutang Usaha Supplier', false),
('a1000000-0000-0000-0000-000000000006'::uuid, 2, '2102', 'Hutang Pajak PPN Keluaran', false),
('a1000000-0000-0000-0000-000000000007'::uuid, 3, '3101', 'Modal Awal Pemilik', false),
('a1000000-0000-0000-0000-000000000008'::uuid, 4, '4101', 'Pendapatan Penjualan Retail POS', false),
('a1000000-0000-0000-0000-000000000009'::uuid, 4, '4201', 'Pendapatan Selisih Kasir (Lebih Kas)', false),
('a1000000-0000-0000-0000-000000000010'::uuid, 5, '5101', 'Beban Pokok Penjualan (HPP / COGS)', false),
('a1000000-0000-0000-0000-000000000011'::uuid, 5, '5201', 'Beban Selisih Kasir (Kurang Kas)', false),
('a1000000-0000-0000-0000-000000000012'::uuid, 5, '5301', 'Beban Selisih Persediaan (Shrinkage)', false)
ON CONFLICT (code) DO NOTHING;

-- 15. Jurnal Saldo Awal Persediaan Produk Demo (150 Indomie @ 2.800 + 80 Ultra Milk @ 5.500 + 25.5 Apel Fuji @ 32.000 = Rp 1.676.000)
INSERT INTO journal_entries (
    id, journal_number, journal_date, reference_type, reference_id, description,
    total_debit, total_credit, created_at, updated_at
) VALUES (
    'a2000000-0000-0000-0000-000000000001'::uuid,
    'JRN-INIT-OUT-001',
    CURRENT_DATE,
    'INITIAL_STOCK',
    'b0000000-0000-0000-0000-000000000001'::uuid,
    'Saldo Awal Persediaan Barang Dagang - Notane Retail Flagship Store',
    1676000.0000,
    1676000.0000,
    CURRENT_TIMESTAMP,
    CURRENT_TIMESTAMP
)
ON CONFLICT (journal_number) DO NOTHING;

INSERT INTO journal_lines (
    id, journal_id, account_id, debit, credit, memo, created_at, updated_at
) VALUES 
(
    'a3000000-0000-0000-0000-000000000001'::uuid,
    'a2000000-0000-0000-0000-000000000001'::uuid,
    'a1000000-0000-0000-0000-000000000004'::uuid, -- 1201 Persediaan Barang Dagang Retail
    1676000.0000,
    0.0000,
    'Saldo Awal Persediaan Barang Dagang di Notane Retail Flagship Store',
    CURRENT_TIMESTAMP,
    CURRENT_TIMESTAMP
),
(
    'a3000000-0000-0000-0000-000000000002'::uuid,
    'a2000000-0000-0000-0000-000000000001'::uuid,
    'a1000000-0000-0000-0000-000000000007'::uuid, -- 3101 Modal Awal Pemilik
    0.0000,
    1676000.0000,
    'Penyerahan Modal Berupa Persediaan Barang Dagang Awal di Notane Retail Flagship Store',
    CURRENT_TIMESTAMP,
    CURRENT_TIMESTAMP
)
ON CONFLICT DO NOTHING;
