-- ============================================================================
-- MIGRATION 047: ADD PRIMARY SUPPLIER TO PRODUCTS TABLE
-- ============================================================================

-- 1. Tambahkan kolom primary_supplier_id ke tabel products
ALTER TABLE products 
ADD COLUMN IF NOT EXISTS primary_supplier_id UUID REFERENCES suppliers(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_products_primary_supplier_id ON products(primary_supplier_id);

-- 2. Isi default primary_supplier_id dari riwayat pembelian PO terbaru (jika ada)
UPDATE products p
SET primary_supplier_id = sub.supplier_id
FROM (
    SELECT DISTINCT ON (poi.product_id) poi.product_id, po.supplier_id
    FROM purchase_order_items poi
    JOIN purchase_orders po ON poi.po_id = po.id
    ORDER BY poi.product_id, po.order_date DESC, po.created_at DESC
) sub
WHERE p.id = sub.product_id AND p.primary_supplier_id IS NULL;
