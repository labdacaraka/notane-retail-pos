-- ============================================================================
-- MIGRATION 007: UNIFY WAREHOUSES INTO OUTLETS TABLE
-- Menyatukan tabel warehouses ke dalam tabel outlets dengan penambahan flag is_warehouse
-- ============================================================================

-- 1. Tambah kolom is_warehouse ke tabel outlets
ALTER TABLE outlets ADD COLUMN IF NOT EXISTS is_warehouse BOOLEAN NOT NULL DEFAULT FALSE;

-- 2. Migrasikan data gudang yang ada di tabel warehouses ke tabel outlets
-- Jika ada id/code yang belum ada di outlets, insert sebagai outlet baru dengan is_warehouse = TRUE
INSERT INTO outlets (id, code, name, is_active, is_warehouse, created_at, updated_at, created_by, updated_by)
SELECT w.id, w.code, w.name, w.is_active, TRUE, w.created_at, w.updated_at, w.created_by, w.updated_by
FROM warehouses w
WHERE NOT EXISTS (
    SELECT 1 FROM outlets o WHERE o.id = w.id OR o.code = w.code
);

-- Untuk gudang yang kodenya atau id-nya sudah ada di outlets, pastikan is_warehouse = TRUE
UPDATE outlets 
SET is_warehouse = TRUE 
WHERE id IN (SELECT id FROM warehouses) OR code IN (SELECT code FROM warehouses);

-- 3. Hapus semua Foreign Key constraints yang mengarah ke tabel warehouses
DO $$
DECLARE
    r RECORD;
BEGIN
    FOR r IN (
        SELECT tc.table_name, tc.constraint_name
        FROM information_schema.table_constraints tc
        JOIN information_schema.constraint_column_usage ccu ON tc.constraint_name = ccu.constraint_name
        WHERE tc.constraint_type = 'FOREIGN KEY' AND ccu.table_name = 'warehouses'
    ) LOOP
        EXECUTE 'ALTER TABLE ' || quote_ident(r.table_name) || ' DROP CONSTRAINT IF EXISTS ' || quote_ident(r.constraint_name);
    END LOOP;
END $$;

-- 4. Pasang ulang Foreign Key constraints agar mereferensikan tabel outlets(id)
DO $$
BEGIN
    -- outlets.default_warehouse_id
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='outlets' AND column_name='default_warehouse_id') THEN
        ALTER TABLE outlets ADD CONSTRAINT fk_outlets_default_warehouse 
            FOREIGN KEY (default_warehouse_id) REFERENCES outlets(id) ON DELETE SET NULL;
    END IF;

    -- warehouse_stocks.warehouse_id
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='warehouse_stocks' AND column_name='warehouse_id') THEN
        ALTER TABLE warehouse_stocks ADD CONSTRAINT fk_warehouse_stocks_outlet 
            FOREIGN KEY (warehouse_id) REFERENCES outlets(id) ON DELETE CASCADE;
    END IF;

    -- stock_batches.warehouse_id
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='stock_batches' AND column_name='warehouse_id') THEN
        ALTER TABLE stock_batches ADD CONSTRAINT fk_stock_batches_outlet 
            FOREIGN KEY (warehouse_id) REFERENCES outlets(id) ON DELETE CASCADE;
    END IF;

    -- stock_mutations
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='stock_mutations' AND column_name='warehouse_id') THEN
        ALTER TABLE stock_mutations ADD CONSTRAINT fk_stock_mutations_outlet 
            FOREIGN KEY (warehouse_id) REFERENCES outlets(id) ON DELETE SET NULL;
    END IF;

    -- stock_adjustments
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='stock_adjustments' AND column_name='warehouse_id') THEN
        ALTER TABLE stock_adjustments ADD CONSTRAINT fk_stock_adjustments_outlet 
            FOREIGN KEY (warehouse_id) REFERENCES outlets(id) ON DELETE SET NULL;
    END IF;

    -- purchase_orders.warehouse_id
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='purchase_orders' AND column_name='warehouse_id') THEN
        ALTER TABLE purchase_orders ADD CONSTRAINT fk_purchase_orders_outlet_wh 
            FOREIGN KEY (warehouse_id) REFERENCES outlets(id) ON DELETE SET NULL;
    END IF;

    -- purchase_invoices.warehouse_id
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='purchase_invoices' AND column_name='warehouse_id') THEN
        ALTER TABLE purchase_invoices ADD CONSTRAINT fk_purchase_invoices_outlet_wh 
            FOREIGN KEY (warehouse_id) REFERENCES outlets(id) ON DELETE SET NULL;
    END IF;

    -- sales.warehouse_id
    IF EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name='sales' AND column_name='warehouse_id') THEN
        ALTER TABLE sales ADD CONSTRAINT fk_sales_outlet_wh 
            FOREIGN KEY (warehouse_id) REFERENCES outlets(id) ON DELETE SET NULL;
    END IF;

    -- outlet_warehouses.warehouse_id
    IF EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name='outlet_warehouses') THEN
        ALTER TABLE outlet_warehouses ADD CONSTRAINT fk_outlet_warehouses_warehouse 
            FOREIGN KEY (warehouse_id) REFERENCES outlets(id) ON DELETE CASCADE;
    END IF;
END $$;

-- 5. Buat index untuk pencarian lokasi gudang pada tabel outlets
CREATE INDEX IF NOT EXISTS idx_outlets_is_warehouse ON outlets(is_warehouse);

-- 6. Hapus tabel fisik warehouses dan ganti dengan VIEW warehouses untuk backward compatibility
DROP TABLE IF EXISTS warehouses CASCADE;

CREATE OR REPLACE VIEW warehouses AS
SELECT 
    id,
    id AS outlet_id,
    code,
    name,
    is_active,
    is_warehouse,
    created_at,
    updated_at,
    created_by,
    updated_by
FROM outlets
WHERE is_warehouse = TRUE;
