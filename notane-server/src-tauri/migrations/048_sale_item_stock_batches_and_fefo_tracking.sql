-- ============================================================================
-- MIGRATION 048: SALE ITEM STOCK BATCHES AND FEFO/FIFO TRACEABILITY
-- ============================================================================

-- 1. Penambahan kolom expiry_date dan source_document_number pada product_stock_batches
ALTER TABLE product_stock_batches
ADD COLUMN IF NOT EXISTS expiry_date DATE,
ADD COLUMN IF NOT EXISTS source_document_number VARCHAR(100);

-- Indeks untuk prioritas alokasi FEFO (tanggal kedaluwarsa terdekat) lalu FIFO (tanggal masuk terlama)
CREATE INDEX IF NOT EXISTS idx_stock_batches_fefo_priority 
    ON product_stock_batches (product_id, outlet_id, expiry_date ASC NULLS LAST, received_at ASC, id ASC)
    WHERE remaining_qty > 0;

-- 2. Pembuatan tabel relasi pelacakan penjualan ke sumber batch stok (Pembelian / Transfer)
CREATE TABLE IF NOT EXISTS sale_item_stock_batches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sale_id UUID NOT NULL REFERENCES sales(id) ON DELETE CASCADE,
    sale_item_id UUID NOT NULL REFERENCES sale_items(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    outlet_id UUID NOT NULL REFERENCES outlets(id) ON DELETE CASCADE,
    batch_id UUID REFERENCES product_stock_batches(id) ON DELETE SET NULL,
    batch_number VARCHAR(100) NOT NULL,
    quantity NUMERIC(14, 4) NOT NULL,
    cost_price NUMERIC(18, 4) NOT NULL,
    subtotal_cost NUMERIC(18, 4) NOT NULL,
    source_reference_type VARCHAR(50) NOT NULL, -- 'PURCHASE_GRN', 'PURCHASE_INVOICE', 'TRANSFER', 'INITIAL_STOCK', 'STOCK_ADJUSTMENT'
    source_reference_id UUID,
    source_document_number VARCHAR(100), -- Nomor PO, GRN, No Faktur Pembelian, No Transfer Stok
    expiry_date DATE,
    allocation_method VARCHAR(20) NOT NULL DEFAULT 'FIFO', -- 'FEFO', 'FIFO', 'MANUAL'
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_sale_item_stock_batches_sale_id ON sale_item_stock_batches(sale_id);
CREATE INDEX IF NOT EXISTS idx_sale_item_stock_batches_sale_item_id ON sale_item_stock_batches(sale_item_id);
CREATE INDEX IF NOT EXISTS idx_sale_item_stock_batches_product_id ON sale_item_stock_batches(product_id);
CREATE INDEX IF NOT EXISTS idx_sale_item_stock_batches_batch_id ON sale_item_stock_batches(batch_id);
CREATE INDEX IF NOT EXISTS idx_sale_item_stock_batches_source ON sale_item_stock_batches(source_reference_type, source_reference_id);

-- 3. Registrasi Hak Akses (RBAC) untuk audit dan pelacakan batch stok penjualan
INSERT INTO permissions (code, description) VALUES
('inventory:batch_tracking', 'Melihat pelacakan alokasi batch stok penjualan ke pembelian dan transfer stok')
ON CONFLICT (code) DO NOTHING;

-- Berikan akses ke SUPERADMIN dan SUPERVISOR
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r, permissions p
WHERE r.name IN ('SUPERADMIN', 'SUPERVISOR')
  AND p.code = 'inventory:batch_tracking'
ON CONFLICT DO NOTHING;
