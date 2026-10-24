-- ============================================================================
-- MIGRATION 031: PRODUCT COSTING METHODS AND STOCK BATCHES (FIFO / WMA / REPLACEMENT COST)
-- ============================================================================

-- 1. Penambahan konfigurasi penyesuaian harga jual & markup pada produk
ALTER TABLE products 
ADD COLUMN IF NOT EXISTS auto_adjust_selling_price BOOLEAN NOT NULL DEFAULT FALSE,
ADD COLUMN IF NOT EXISTS markup_percent NUMERIC(8, 2) DEFAULT NULL;

-- 2. Kolom harga beli terakhir per outlet untuk audit HPP
ALTER TABLE product_outlet_prices
ADD COLUMN IF NOT EXISTS last_purchase_price NUMERIC(18, 4) DEFAULT NULL;

-- 3. Tabel Batch Stok Produk untuk pelacakan FIFO (First In, First Out)
CREATE TABLE IF NOT EXISTS product_stock_batches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    product_id UUID NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    outlet_id UUID NOT NULL REFERENCES outlets(id) ON DELETE CASCADE,
    batch_number VARCHAR(100) NOT NULL,
    initial_qty NUMERIC(14, 4) NOT NULL,
    remaining_qty NUMERIC(14, 4) NOT NULL,
    cost_price NUMERIC(18, 4) NOT NULL,
    reference_type VARCHAR(50) NOT NULL, -- 'INITIAL_STOCK', 'PURCHASE_GRN', 'PURCHASE_INVOICE', 'TRANSFER'
    reference_id UUID,
    received_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_stock_batches_fifo 
    ON product_stock_batches (product_id, outlet_id, received_at ASC, id ASC);

CREATE INDEX IF NOT EXISTS idx_stock_batches_remaining 
    ON product_stock_batches (product_id, outlet_id) 
    WHERE remaining_qty > 0;

-- 4. Inisialisasi batch awal dari stok yang sudah ada di sistem agar FIFO langsung siap dipakai
INSERT INTO product_stock_batches (
    product_id, outlet_id, batch_number, initial_qty, remaining_qty, cost_price, reference_type, received_at
)
SELECT 
    ws.product_id,
    ws.warehouse_id,
    'SALDO-AWAL',
    ws.quantity,
    ws.quantity,
    COALESCE(pop.cost_price, p.cost_price, 0),
    'INITIAL_STOCK',
    CURRENT_TIMESTAMP
FROM warehouse_stocks ws
JOIN products p ON ws.product_id = p.id
LEFT JOIN product_outlet_prices pop ON pop.product_id = ws.product_id AND pop.outlet_id = ws.warehouse_id
WHERE ws.quantity > 0;
