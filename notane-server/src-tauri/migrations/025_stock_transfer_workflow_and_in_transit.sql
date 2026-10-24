-- Migration 025: Stock Transfer Workflow (Dispatch -> In-Transit -> Receive) & In-Transit Account

-- 1. Tambah akun CoA 1202: Persediaan Dalam Perjalanan (In-Transit) jika belum ada
DO $$
DECLARE
    parent_asset_id UUID;
    existing_1202 UUID;
BEGIN
    SELECT id INTO existing_1202 FROM chart_of_accounts WHERE code = '1202';
    IF existing_1202 IS NULL THEN
        SELECT id INTO parent_asset_id FROM chart_of_accounts WHERE code = '1201';
        IF parent_asset_id IS NULL THEN
            SELECT id INTO parent_asset_id FROM chart_of_accounts WHERE code = '1200' OR code = '1000' LIMIT 1;
        END IF;

        INSERT INTO chart_of_accounts (
            id, account_category_id, parent_id, code, name, is_header, is_active, created_at, updated_at
        ) VALUES (
            gen_random_uuid(),
            1, -- Asset
            parent_asset_id,
            '1202',
            'Persediaan Dalam Perjalanan (In-Transit)',
            FALSE,
            TRUE,
            CURRENT_TIMESTAMP,
            CURRENT_TIMESTAMP
        );
    END IF;
END $$;

-- 2. Tambah kolom alur pengiriman dan penerimaan pada stock_transfers
ALTER TABLE stock_transfers ADD COLUMN IF NOT EXISTS received_date DATE;
ALTER TABLE stock_transfers ADD COLUMN IF NOT EXISTS received_by UUID REFERENCES users(id);
ALTER TABLE stock_transfers ADD COLUMN IF NOT EXISTS received_at TIMESTAMPTZ;
ALTER TABLE stock_transfers ADD COLUMN IF NOT EXISTS shipped_at TIMESTAMPTZ;
ALTER TABLE stock_transfers ADD COLUMN IF NOT EXISTS cancellation_reason TEXT;
ALTER TABLE stock_transfers ADD COLUMN IF NOT EXISTS cancelled_by UUID REFERENCES users(id);
ALTER TABLE stock_transfers ADD COLUMN IF NOT EXISTS cancelled_at TIMESTAMPTZ;

-- 3. Tambah kolom received_quantity dan notes pada stock_transfer_items
ALTER TABLE stock_transfer_items ADD COLUMN IF NOT EXISTS received_quantity NUMERIC(14, 4);
ALTER TABLE stock_transfer_items ADD COLUMN IF NOT EXISTS notes TEXT;

-- Pastikan data transfer lama yang COMPLETED memiliki received_quantity = quantity
UPDATE stock_transfer_items
SET received_quantity = quantity
WHERE received_quantity IS NULL;

-- 4. Indeks pencarian status
CREATE INDEX IF NOT EXISTS idx_stock_transfers_status ON stock_transfers(status);
