-- ============================================================================
-- MIGRATION 015: FLASH SALE PROMOTIONS & RETAIL DISCOUNT ENGINE
-- Mendukung program flash sale multi-outlet, batas kuota, time-window,
-- dan perincian akumulasi diskon (flash sale, member, voucher) di struk.
-- ============================================================================

-- 1. Tabel Master Program Flash Sale
CREATE TABLE IF NOT EXISTS flash_sales (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(150) NOT NULL,
    description TEXT,
    start_date DATE NOT NULL,
    end_date DATE NOT NULL,
    is_all_day BOOLEAN NOT NULL DEFAULT true,
    start_time TIME,
    end_time TIME,
    is_active BOOLEAN NOT NULL DEFAULT true,
    is_stackable_with_member BOOLEAN NOT NULL DEFAULT false, -- Standar ritel: default false untuk menjaga margin HPP
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id)
);

CREATE INDEX IF NOT EXISTS idx_flash_sales_dates ON flash_sales(start_date, end_date, is_active);

-- 2. Item-Item Produk yang Ikut Serta dalam Flash Sale
CREATE TABLE IF NOT EXISTS flash_sale_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    flash_sale_id UUID NOT NULL REFERENCES flash_sales(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    discount_type VARCHAR(20) NOT NULL DEFAULT 'FIXED_PRICE', -- FIXED_PRICE, PERCENTAGE, NOMINAL_CUT
    discount_value NUMERIC(18, 4) NOT NULL,
    max_qty_per_transaction NUMERIC(14, 4), -- Maksimal kuota per struk kasir
    promo_quota NUMERIC(14, 4),             -- Total alokasi kuota unit promo
    sold_qty NUMERIC(14, 4) NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uq_flash_sale_product UNIQUE(flash_sale_id, product_id)
);

CREATE INDEX IF NOT EXISTS idx_flash_sale_items_product ON flash_sale_items(product_id);

-- 3. Target Outlet yang Berhak Menerapkan Flash Sale Ini
CREATE TABLE IF NOT EXISTS flash_sale_outlets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    flash_sale_id UUID NOT NULL REFERENCES flash_sales(id) ON DELETE CASCADE,
    outlet_id UUID NOT NULL REFERENCES outlets(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uq_flash_sale_outlet UNIQUE(flash_sale_id, outlet_id)
);

CREATE INDEX IF NOT EXISTS idx_flash_sale_outlets_outlet ON flash_sale_outlets(outlet_id);

-- 4. Penyesuaian Kolom Audit Diskon di Tabel sale_items
ALTER TABLE sale_items 
    ADD COLUMN IF NOT EXISTS flash_sale_id UUID REFERENCES flash_sales(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS original_price NUMERIC(18, 4),
    ADD COLUMN IF NOT EXISTS flash_sale_discount NUMERIC(18, 4) NOT NULL DEFAULT 0.0000,
    ADD COLUMN IF NOT EXISTS member_discount NUMERIC(18, 4) NOT NULL DEFAULT 0.0000;

-- 5. Penyesuaian Kolom Akumulasi Penghematan di Tabel sales
ALTER TABLE sales
    ADD COLUMN IF NOT EXISTS total_flash_sale_discount NUMERIC(18, 4) NOT NULL DEFAULT 0.0000,
    ADD COLUMN IF NOT EXISTS total_member_discount NUMERIC(18, 4) NOT NULL DEFAULT 0.0000,
    ADD COLUMN IF NOT EXISTS total_savings_amount NUMERIC(18, 4) NOT NULL DEFAULT 0.0000;

-- 6. Hak Akses (Permissions) untuk Promosi / Flash Sale
INSERT INTO permissions (code, description)
VALUES 
    ('promotions:read', 'Melihat daftar program flash sale dan event diskon'),
    ('promotions:write', 'Membuat, mengedit, dan menghentikan program flash sale')
ON CONFLICT (code) DO NOTHING;

-- Berikan permission ke role SUPERADMIN dan STORE_MANAGER
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r
CROSS JOIN permissions p
WHERE r.name IN ('SUPERADMIN', 'STORE_MANAGER')
  AND p.code IN ('promotions:read', 'promotions:write')
ON CONFLICT DO NOTHING;
