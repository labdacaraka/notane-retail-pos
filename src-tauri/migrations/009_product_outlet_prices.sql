-- ============================================================================
-- MIGRATION 009: PRODUCT OUTLET PRICES (MULTI-OUTLET PRICING)
-- Tabel harga jual dasar dan HPP (cost_price) per outlet/cabang
-- ============================================================================

CREATE TABLE IF NOT EXISTS product_outlet_prices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    outlet_id UUID NOT NULL REFERENCES outlets(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    cost_price NUMERIC(18, 4) NOT NULL DEFAULT 0.0000,
    base_selling_price NUMERIC(18, 4) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id),
    UNIQUE(outlet_id, product_id)
);

CREATE INDEX IF NOT EXISTS idx_prod_outlet_prices_lookup ON product_outlet_prices(product_id, outlet_id);
CREATE INDEX IF NOT EXISTS idx_prod_outlet_prices_outlet ON product_outlet_prices(outlet_id);
