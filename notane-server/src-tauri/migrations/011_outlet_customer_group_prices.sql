-- ============================================================================
-- MIGRATION 011: OUTLET CUSTOMER GROUP PRICES (HARGA MEMBER PER OUTLET)
-- Harga khusus member/grup pelanggan yang spesifik per outlet/cabang.
-- Prioritas lookup: outlet+group > global group > outlet base > global base
-- ============================================================================

CREATE TABLE IF NOT EXISTS outlet_customer_group_prices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    outlet_id UUID NOT NULL REFERENCES outlets(id) ON DELETE CASCADE,
    customer_group_id INT NOT NULL REFERENCES customer_groups(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    special_price NUMERIC(18, 4) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id),
    CONSTRAINT uk_outlet_group_product UNIQUE (outlet_id, customer_group_id, product_id)
);

CREATE INDEX IF NOT EXISTS idx_outlet_cgp_lookup
    ON outlet_customer_group_prices(product_id, outlet_id, customer_group_id);

CREATE INDEX IF NOT EXISTS idx_outlet_cgp_outlet
    ON outlet_customer_group_prices(outlet_id);
