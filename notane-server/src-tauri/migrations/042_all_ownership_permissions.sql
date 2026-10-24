-- ============================================================================
-- MIGRASI 042: ALL OWNERSHIP DATA ACCESS PERMISSIONS
-- Menambahkan permission kontrol kepemilikan data (Ownership-Based Access Control)
-- Jika role memiliki permission ini, bebas melihat data dari semua staf.
-- Jika tidak, hanya dapat melihat data yang dibuat atau diubah oleh staf tersebut.
-- ============================================================================

INSERT INTO permissions (code, description) VALUES
('sales:all_ownership', 'Melihat seluruh transaksi penjualan dari semua staf dan kasir (tanpa batas kepemilikan data)'),
('purchasing:all_ownership', 'Melihat seluruh transaksi pembelian dan faktur supplier dari semua staf (tanpa batas kepemilikan data)'),
('expenses:all_ownership', 'Melihat seluruh transaksi pengeluaran kas dan biaya operasional dari semua staf (tanpa batas kepemilikan data)'),
('inventory:all_ownership', 'Melihat seluruh mutasi, penyesuaian, dan transfer stok dari semua staf (tanpa batas kepemilikan data)'),
('accounting:all_ownership', 'Melihat seluruh entri jurnal akuntansi dan pembukuan dari semua staf (tanpa batas kepemilikan data)')
ON CONFLICT (code) DO NOTHING;

-- 1. Berikan seluruh permission all ownership ke role SUPERADMIN
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p
WHERE r.name = 'SUPERADMIN' AND p.code IN (
    'sales:all_ownership',
    'purchasing:all_ownership',
    'expenses:all_ownership',
    'inventory:all_ownership',
    'accounting:all_ownership'
)
ON CONFLICT DO NOTHING;

-- 2. Berikan permission all ownership ke role STORE_MANAGER
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p
WHERE r.name = 'STORE_MANAGER' AND p.code IN (
    'sales:all_ownership',
    'purchasing:all_ownership',
    'expenses:all_ownership',
    'inventory:all_ownership',
    'accounting:all_ownership'
)
ON CONFLICT DO NOTHING;
