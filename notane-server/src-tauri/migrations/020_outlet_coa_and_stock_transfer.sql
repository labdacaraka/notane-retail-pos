-- Migration 020: Outlet Chart of Accounts (CoA) & Stock Transfer Module

-- 1. Add outlet_id to chart_of_accounts
ALTER TABLE chart_of_accounts 
ADD COLUMN IF NOT EXISTS outlet_id UUID REFERENCES outlets(id) ON DELETE CASCADE;

CREATE INDEX IF NOT EXISTS idx_chart_of_accounts_outlet_id ON chart_of_accounts(outlet_id);

-- 2. Add inventory_account_id to outlets
ALTER TABLE outlets 
ADD COLUMN IF NOT EXISTS inventory_account_id UUID REFERENCES chart_of_accounts(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_outlets_inventory_account_id ON outlets(inventory_account_id);

-- 3. Set parent account 1201 as header if needed, and auto-generate CoA for existing outlets
DO $$
DECLARE
    r RECORD;
    parent_inv_id UUID;
    new_coa_id UUID;
    coa_code VARCHAR(30);
BEGIN
    SELECT id INTO parent_inv_id FROM chart_of_accounts WHERE code = '1201';
    IF parent_inv_id IS NOT NULL THEN
        UPDATE chart_of_accounts SET is_header = TRUE WHERE id = parent_inv_id;

        FOR r IN SELECT id, code, name, is_active FROM outlets LOOP
            coa_code := '1201-' || UPPER(TRIM(r.code));
            
            -- Insert or get existing account
            INSERT INTO chart_of_accounts (id, account_category_id, parent_id, code, name, is_header, is_active, outlet_id)
            VALUES (gen_random_uuid(), 1, parent_inv_id, coa_code, 'Persediaan - ' || TRIM(r.name), FALSE, r.is_active, r.id)
            ON CONFLICT (code) DO UPDATE SET outlet_id = r.id, name = 'Persediaan - ' || TRIM(r.name)
            RETURNING id INTO new_coa_id;

            UPDATE outlets SET inventory_account_id = new_coa_id WHERE id = r.id;
        END LOOP;
    END IF;
END $$;

-- 4. Create Stock Transfers tables
CREATE TABLE IF NOT EXISTS stock_transfers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    transfer_number VARCHAR(50) UNIQUE NOT NULL,
    from_outlet_id UUID NOT NULL REFERENCES outlets(id),
    to_outlet_id UUID NOT NULL REFERENCES outlets(id),
    transfer_date DATE NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'COMPLETED', -- COMPLETED, CANCELLED
    notes TEXT,
    total_amount NUMERIC(18, 4) NOT NULL DEFAULT 0.0000,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id),
    CONSTRAINT chk_different_outlets CHECK (from_outlet_id <> to_outlet_id)
);

CREATE TABLE IF NOT EXISTS stock_transfer_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    transfer_id UUID NOT NULL REFERENCES stock_transfers(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES products(id),
    quantity NUMERIC(14, 4) NOT NULL,
    cost_price NUMERIC(18, 4) NOT NULL,
    subtotal NUMERIC(18, 4) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_stock_transfers_from ON stock_transfers(from_outlet_id);
CREATE INDEX IF NOT EXISTS idx_stock_transfers_to ON stock_transfers(to_outlet_id);
CREATE INDEX IF NOT EXISTS idx_stock_transfers_date ON stock_transfers(transfer_date);
CREATE INDEX IF NOT EXISTS idx_stock_transfer_items_transfer ON stock_transfer_items(transfer_id);
CREATE INDEX IF NOT EXISTS idx_stock_transfer_items_product ON stock_transfer_items(product_id);
