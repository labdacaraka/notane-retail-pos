-- Migration: 006_decouple_warehouses_and_link_outlets.sql
-- Decouple 1-to-1 relationship between warehouses and outlets so 1 warehouse can supply multiple outlets.
-- Link all commercial transactions (sales, cashier_shifts, purchase_orders, purchase_invoices) directly to outlets.

-- 1. Warehouses: Make outlet_id nullable so warehouses can be standalone/central/shared
ALTER TABLE warehouses ALTER COLUMN outlet_id DROP NOT NULL;

-- 2. Supply relationship mapping: outlet_warehouses (Many-to-Many)
CREATE TABLE IF NOT EXISTS outlet_warehouses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    outlet_id UUID NOT NULL REFERENCES outlets(id) ON DELETE CASCADE,
    warehouse_id UUID NOT NULL REFERENCES warehouses(id) ON DELETE CASCADE,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uk_outlet_warehouse UNIQUE (outlet_id, warehouse_id)
);

-- Seed existing default_warehouse_id into outlet_warehouses
INSERT INTO outlet_warehouses (outlet_id, warehouse_id, is_default)
SELECT id, default_warehouse_id, TRUE
FROM outlets
WHERE default_warehouse_id IS NOT NULL
ON CONFLICT (outlet_id, warehouse_id) DO NOTHING;

-- 3. Cashier Shifts: Connect to outlet
ALTER TABLE cashier_shifts ADD COLUMN IF NOT EXISTS outlet_id UUID REFERENCES outlets(id);

UPDATE cashier_shifts
SET outlet_id = (SELECT id FROM outlets WHERE is_active = TRUE ORDER BY created_at ASC LIMIT 1)
WHERE outlet_id IS NULL;

-- 4. Sales: Connect to outlet (primary transaction entity)
ALTER TABLE sales ADD COLUMN IF NOT EXISTS outlet_id UUID REFERENCES outlets(id);

UPDATE sales s
SET outlet_id = w.outlet_id
FROM warehouses w
WHERE s.warehouse_id = w.id AND s.outlet_id IS NULL AND w.outlet_id IS NOT NULL;

UPDATE sales
SET outlet_id = (SELECT id FROM outlets ORDER BY created_at ASC LIMIT 1)
WHERE outlet_id IS NULL;

ALTER TABLE sales ALTER COLUMN outlet_id SET NOT NULL;
ALTER TABLE sales ALTER COLUMN warehouse_id DROP NOT NULL;

-- 5. Purchase Orders: Connect to outlet
ALTER TABLE purchase_orders ADD COLUMN IF NOT EXISTS outlet_id UUID REFERENCES outlets(id);

UPDATE purchase_orders po
SET outlet_id = w.outlet_id
FROM warehouses w
WHERE po.warehouse_id = w.id AND po.outlet_id IS NULL AND w.outlet_id IS NOT NULL;

UPDATE purchase_orders
SET outlet_id = (SELECT id FROM outlets ORDER BY created_at ASC LIMIT 1)
WHERE outlet_id IS NULL;

ALTER TABLE purchase_orders ALTER COLUMN warehouse_id DROP NOT NULL;

-- 6. Purchase Invoices: Connect to outlet
ALTER TABLE purchase_invoices ADD COLUMN IF NOT EXISTS outlet_id UUID REFERENCES outlets(id);

UPDATE purchase_invoices pi
SET outlet_id = w.outlet_id
FROM warehouses w
WHERE pi.warehouse_id = w.id AND pi.outlet_id IS NULL AND w.outlet_id IS NOT NULL;

UPDATE purchase_invoices
SET outlet_id = (SELECT id FROM outlets ORDER BY created_at ASC LIMIT 1)
WHERE outlet_id IS NULL;

ALTER TABLE purchase_invoices ALTER COLUMN warehouse_id DROP NOT NULL;

-- Indexing for reporting performance
CREATE INDEX IF NOT EXISTS idx_sales_outlet_id ON sales(outlet_id);
CREATE INDEX IF NOT EXISTS idx_cashier_shifts_outlet_id ON cashier_shifts(outlet_id);
CREATE INDEX IF NOT EXISTS idx_purchase_orders_outlet_id ON purchase_orders(outlet_id);
CREATE INDEX IF NOT EXISTS idx_purchase_invoices_outlet_id ON purchase_invoices(outlet_id);
CREATE INDEX IF NOT EXISTS idx_outlet_warehouses_outlet ON outlet_warehouses(outlet_id);
CREATE INDEX IF NOT EXISTS idx_outlet_warehouses_warehouse ON outlet_warehouses(warehouse_id);
