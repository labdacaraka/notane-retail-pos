-- ============================================================================
-- MIGRASI 060: PERMISSION EKSPOR EXCEL LATAR BELAKANG
-- Fitur: Penjualan, Pembelian, Pengeluaran Operasional, dan Master Produk
-- ============================================================================

INSERT INTO permissions (code, description) VALUES
('sales:export', 'Mengekspor laporan riwayat transaksi penjualan kasir ke format file Excel (.xlsx)'),
('purchasing:export', 'Mengekspor data pesanan pembelian (PO), penerimaan barang (GRN), faktur, dan retur supplier ke Excel (.xlsx)'),
('expenses:export', 'Mengekspor data transaksi pengeluaran kas operasional toko ke format file Excel (.xlsx)'),
('products:export', 'Mengekspor katalog master produk ke format file Excel (.xlsx) yang kompatibel dengan template import')
ON CONFLICT (code) DO UPDATE SET
    description = EXCLUDED.description;

-- Petakan ke role default
-- 1. Super Admin (role_id = 1) & Store Manager / Admin (role_id = 2) mendapatkan seluruh izin export
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r
CROSS JOIN permissions p
WHERE r.name IN ('super_admin', 'admin', 'store_manager', 'general_manager')
  AND p.code IN ('sales:export', 'purchasing:export', 'expenses:export', 'products:export')
ON CONFLICT DO NOTHING;

-- 2. Staff Akuntansi / Keuangan (Accounting)
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r
CROSS JOIN permissions p
WHERE r.name IN ('accounting', 'finance')
  AND p.code IN ('sales:export', 'purchasing:export', 'expenses:export')
ON CONFLICT DO NOTHING;

-- 3. Staff Pengadaan / Gudang (Purchasing / Warehouse)
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r
CROSS JOIN permissions p
WHERE r.name IN ('purchasing', 'warehouse', 'inventory')
  AND p.code IN ('purchasing:export', 'products:export')
ON CONFLICT DO NOTHING;
