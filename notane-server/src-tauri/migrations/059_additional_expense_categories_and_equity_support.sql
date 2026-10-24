-- =========================================================================
-- Migration 059: Additional CoA for Adjustments & Extended Expense Categories
-- 1. Tambah CoA Akuntansi untuk Penyesuaian Toko:
--    - 1301: Aset Tetap & Peralatan Toko (ASSET)
--    - 1302: Akumulasi Penyusutan Peralatan Toko (ASSET)
--    - 1401: Sewa & Beban Dibayar Dimuka (ASSET)
--    - 2105: Beban Akrual & Hutang Biaya Operasional (LIABILITY)
--    - 4301: Pendapatan Bunga Bank & Jasa Giro (REVENUE)
--    - 5410: Beban Penyusutan Aset Tetap Toko (EXPENSE)
--    - 5501: Beban Administrasi Bank, EDC & QRIS (EXPENSE)
-- 2. Tambah Kategori Pengeluaran Toko Lengkap:
--    - PROMOSI: Promosi, Iklan & Brosur Toko (5406)
--    - BIAYA_BANK: Biaya Admin Bank & MDR QRIS (5501)
--    - RETRIBUSI: Iuran Lingkungan, Retribusi & Parkir (5407)
--    - PRIVE: Prive / Pengambilan Kas Pribadi Pemilik (3103)
-- =========================================================================

-- 1. Tambah CoA yang diperlukan
INSERT INTO chart_of_accounts (id, account_category_id, code, name, is_header, is_system, is_active, created_at, updated_at)
SELECT gen_random_uuid(), 1, '1301', 'Aset Tetap & Peralatan Toko', false, true, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM chart_of_accounts WHERE code = '1301');

INSERT INTO chart_of_accounts (id, account_category_id, code, name, is_header, is_system, is_active, created_at, updated_at)
SELECT gen_random_uuid(), 1, '1302', 'Akumulasi Penyusutan Peralatan Toko', false, true, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM chart_of_accounts WHERE code = '1302');

INSERT INTO chart_of_accounts (id, account_category_id, code, name, is_header, is_system, is_active, created_at, updated_at)
SELECT gen_random_uuid(), 1, '1401', 'Sewa & Beban Dibayar Dimuka', false, true, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM chart_of_accounts WHERE code = '1401');

INSERT INTO chart_of_accounts (id, account_category_id, code, name, is_header, is_system, is_active, created_at, updated_at)
SELECT gen_random_uuid(), 2, '2105', 'Beban Akrual & Hutang Biaya Operasional', false, true, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM chart_of_accounts WHERE code = '2105');

INSERT INTO chart_of_accounts (id, account_category_id, code, name, is_header, is_system, is_active, created_at, updated_at)
SELECT gen_random_uuid(), 4, '4301', 'Pendapatan Bunga Bank & Jasa Giro', false, true, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM chart_of_accounts WHERE code = '4301');

INSERT INTO chart_of_accounts (id, account_category_id, code, name, is_header, is_system, is_active, created_at, updated_at)
SELECT gen_random_uuid(), 5, '5410', 'Beban Penyusutan Aset Tetap Toko', false, true, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM chart_of_accounts WHERE code = '5410');

INSERT INTO chart_of_accounts (id, account_category_id, code, name, is_header, is_system, is_active, created_at, updated_at)
SELECT gen_random_uuid(), 5, '5501', 'Beban Administrasi Bank, EDC & QRIS', false, true, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM chart_of_accounts WHERE code = '5501');

-- Pastikan 5406, 5407, 3103 ada
INSERT INTO chart_of_accounts (id, account_category_id, code, name, is_header, is_system, is_active, created_at, updated_at)
SELECT gen_random_uuid(), 5, '5406', 'Beban Pemasaran, Iklan & Promosi', false, true, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM chart_of_accounts WHERE code = '5406');

INSERT INTO chart_of_accounts (id, account_category_id, code, name, is_header, is_system, is_active, created_at, updated_at)
SELECT gen_random_uuid(), 5, '5407', 'Beban Kebersihan, Keamanan & Sampah', false, true, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM chart_of_accounts WHERE code = '5407');

-- 2. Tambah Kategori Pengeluaran Toko Lengkap
INSERT INTO expense_categories (id, code, name, account_id, description, is_active)
SELECT 
    gen_random_uuid(), 
    'PROMOSI', 
    'Promosi, Iklan & Brosur Toko', 
    (SELECT id FROM chart_of_accounts WHERE code = '5406' LIMIT 1), 
    'Cetak banner, spanduk diskon, brosur promo, dan iklan sosmed', 
    true
WHERE NOT EXISTS (SELECT 1 FROM expense_categories WHERE code = 'PROMOSI');

INSERT INTO expense_categories (id, code, name, account_id, description, is_active)
SELECT 
    gen_random_uuid(), 
    'BIAYA_BANK', 
    'Biaya Admin Bank & MDR QRIS', 
    (SELECT id FROM chart_of_accounts WHERE code = '5501' LIMIT 1), 
    'Biaya transfer antar bank, potongan transaksi QRIS, atau biaya admin EDC', 
    true
WHERE NOT EXISTS (SELECT 1 FROM expense_categories WHERE code = 'BIAYA_BANK');

INSERT INTO expense_categories (id, code, name, account_id, description, is_active)
SELECT 
    gen_random_uuid(), 
    'RETRIBUSI', 
    'Iuran Lingkungan, Retribusi & Parkir', 
    (SELECT id FROM chart_of_accounts WHERE code = '5407' LIMIT 1), 
    'Iuran keamanan, sampah pasar, parkir bongkar muat, retribusi resmi', 
    true
WHERE NOT EXISTS (SELECT 1 FROM expense_categories WHERE code = 'RETRIBUSI');

INSERT INTO expense_categories (id, code, name, account_id, description, is_active)
SELECT 
    gen_random_uuid(), 
    'PRIVE', 
    'Prive / Pengambilan Kas Pribadi Pemilik', 
    (SELECT id FROM chart_of_accounts WHERE code = '3103' LIMIT 1), 
    'Penarikan uang tunai oleh pemilik toko dari kasir/bank untuk keperluan pribadi', 
    true
WHERE NOT EXISTS (SELECT 1 FROM expense_categories WHERE code = 'PRIVE');
