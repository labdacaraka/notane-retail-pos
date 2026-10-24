-- ============================================================================
-- MIGRATION 026: REMOVE DEFAULT WAREHOUSE FROM OUTLETS & ENFORCE SINGLE LOCATION STOCKS
-- Setiap outlet/gudang mengelola stoknya sendiri secara mandiri tanpa akumulasi gudang bawaan
-- ============================================================================

-- 1. Konsolidasikan stok fisik dari WH-001 (Gudang Utama Toko) ke OUT-001 (Notane Retail Flagship Store)
--    karena sebelumnya stok outlet disimpan di default_warehouse_id ini.
INSERT INTO warehouse_stocks (warehouse_id, product_id, quantity, updated_at)
SELECT 'b0000000-0000-0000-0000-000000000001', ws.product_id, ws.quantity, CURRENT_TIMESTAMP
FROM warehouse_stocks ws
WHERE ws.warehouse_id = 'b0000000-0000-0000-0000-000000000002'
ON CONFLICT (warehouse_id, product_id)
DO UPDATE SET quantity = warehouse_stocks.quantity + EXCLUDED.quantity;

-- Setel stok di WH-001 menjadi 0 agar tidak terduplikasi
UPDATE warehouse_stocks 
SET quantity = 0, updated_at = CURRENT_TIMESTAMP
WHERE warehouse_id = 'b0000000-0000-0000-0000-000000000002';

-- 2. Hapus foreign key dan kolom default_warehouse_id dari tabel outlets
ALTER TABLE outlets DROP CONSTRAINT IF EXISTS fk_outlets_default_warehouse;
ALTER TABLE outlets DROP COLUMN IF EXISTS default_warehouse_id;

-- 3. Hapus tabel perantara outlet_warehouses jika ada
DROP TABLE IF EXISTS outlet_warehouses CASCADE;
