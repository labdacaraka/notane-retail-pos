-- ============================================================================
-- Migration 037: Retail Enterprise Features (Prioritas 1 & Prioritas 2)
-- 1. Promotion Engine (Buy X Get Y, Tebus Murah PWP, Combo Bundling)
-- 2. Customer Sales Returns & Exchanges (Retur Parsial Konsumen di Layar Kasir)
-- 3. Cash Drops / Pick Up (Setor Kas Laci Kasir ke Brankas)
-- 4. Expiry Date & Batch Tracking pada Goods Receipt (Penerimaan Barang)
-- 5. Cash Rounding & Donation Policy serta Batas Uang Tunai Laci Kasir
-- 6. Pendaftaran Permissions & Mapping ke Role RBAC
-- ============================================================================

-- 1. Tabel Promosi Otomatis (Promotion Engine)
CREATE TABLE IF NOT EXISTS promotions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code VARCHAR(50) UNIQUE NOT NULL,
    name VARCHAR(150) NOT NULL,
    promo_type VARCHAR(50) NOT NULL, -- BUY_X_GET_Y, TEBUS_MURAH, COMBO_BUNDLE
    start_date TIMESTAMPTZ NOT NULL,
    end_date TIMESTAMPTZ NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    min_purchase_amount NUMERIC(18, 4) NOT NULL DEFAULT 0.0000,
    outlet_id UUID REFERENCES outlets(id) ON DELETE CASCADE, -- NULL = berlaku di semua outlet
    priority INT NOT NULL DEFAULT 1,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

CREATE TABLE IF NOT EXISTS promotion_rules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    promotion_id UUID NOT NULL REFERENCES promotions(id) ON DELETE CASCADE,
    buy_product_id UUID REFERENCES products(id) ON DELETE CASCADE,
    buy_quantity NUMERIC(14, 4) NOT NULL DEFAULT 1.0000,
    reward_product_id UUID REFERENCES products(id) ON DELETE CASCADE,
    reward_quantity NUMERIC(14, 4) NOT NULL DEFAULT 1.0000,
    reward_discount_percent NUMERIC(5, 2) NOT NULL DEFAULT 100.00, -- 100% = gratis
    reward_special_price NUMERIC(18, 4), -- Misal tebus murah seharga Rp 10.000
    bundle_discount_amount NUMERIC(18, 4) NOT NULL DEFAULT 0.0000,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_promotions_active_dates ON promotions(is_active, start_date, end_date);
CREATE INDEX IF NOT EXISTS idx_promotions_outlet_id ON promotions(outlet_id);
CREATE INDEX IF NOT EXISTS idx_promotion_rules_promotion_id ON promotion_rules(promotion_id);

-- 2. Tabel Retur Penjualan Konsumen (Customer Sales Returns)
CREATE TABLE IF NOT EXISTS sales_returns (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    return_number VARCHAR(50) UNIQUE NOT NULL,
    sale_id UUID NOT NULL REFERENCES sales(id) ON DELETE RESTRICT,
    outlet_id UUID REFERENCES outlets(id) ON DELETE RESTRICT,
    shift_id UUID REFERENCES cashier_shifts(id) ON DELETE SET NULL,
    cashier_id UUID NOT NULL REFERENCES users(id),
    supervisor_id UUID NOT NULL REFERENCES users(id),
    return_type VARCHAR(30) NOT NULL DEFAULT 'REFUND', -- REFUND, EXCHANGE
    total_refund_amount NUMERIC(18, 4) NOT NULL DEFAULT 0.0000,
    reason_id UUID REFERENCES reason_codes(id),
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS sales_return_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sales_return_id UUID NOT NULL REFERENCES sales_returns(id) ON DELETE CASCADE,
    sale_item_id UUID NOT NULL REFERENCES sale_items(id) ON DELETE RESTRICT,
    product_id UUID NOT NULL REFERENCES products(id) ON DELETE RESTRICT,
    quantity NUMERIC(14, 4) NOT NULL,
    unit_price NUMERIC(18, 4) NOT NULL,
    discount_amount NUMERIC(18, 4) NOT NULL DEFAULT 0.0000,
    subtotal NUMERIC(18, 4) NOT NULL,
    condition VARCHAR(30) NOT NULL DEFAULT 'GOOD', -- GOOD, DAMAGED
    restock BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_sales_returns_sale_id ON sales_returns(sale_id);
CREATE INDEX IF NOT EXISTS idx_sales_returns_outlet_id ON sales_returns(outlet_id);
CREATE INDEX IF NOT EXISTS idx_sales_return_items_return_id ON sales_return_items(sales_return_id);

-- Akun CoA Retur Penjualan (Kontra Pendapatan: 4103)
INSERT INTO chart_of_accounts (id, account_category_id, code, name, is_header, is_system, is_active, created_at, updated_at)
SELECT gen_random_uuid(), 4, '4103', 'Retur Penjualan Konsumen (Sales Returns)', false, true, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM chart_of_accounts WHERE code = '4103');

-- Reason Codes untuk Retur Konsumen
INSERT INTO reason_codes (id, type, code, description) VALUES
('f1000000-0000-0000-0000-000000000005'::uuid, 'RETURN_IN', 'RET_DEFECT', 'Barang Cacat / Rusak / Expired'),
('f1000000-0000-0000-0000-000000000006'::uuid, 'RETURN_IN', 'RET_EXCHANGE', 'Konsumen Tukar Barang / Salah Beli')
ON CONFLICT (id) DO NOTHING;

-- 3. Tabel Setor Kas Laci Meja Kasir (Cash Drops / Pick Up)
CREATE TABLE IF NOT EXISTS cash_drops (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    drop_number VARCHAR(50) UNIQUE NOT NULL,
    shift_id UUID NOT NULL REFERENCES cashier_shifts(id) ON DELETE RESTRICT,
    outlet_id UUID REFERENCES outlets(id) ON DELETE RESTRICT,
    cashier_id UUID NOT NULL REFERENCES users(id),
    supervisor_id UUID NOT NULL REFERENCES users(id),
    amount NUMERIC(18, 4) NOT NULL,
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_cash_drops_shift_id ON cash_drops(shift_id);
CREATE INDEX IF NOT EXISTS idx_cash_drops_outlet_id ON cash_drops(outlet_id);

-- 4. Tambahkan expiry_date & batch_number pada goods_receipt_items
ALTER TABLE goods_receipt_items
    ADD COLUMN IF NOT EXISTS expiry_date DATE,
    ADD COLUMN IF NOT EXISTS batch_number VARCHAR(100);

-- 5. Tambahkan pengaturan pembulatan kas, donasi, & batas kas laci pada receipt_settings
ALTER TABLE receipt_settings
    ADD COLUMN IF NOT EXISTS cash_rounding_mode VARCHAR(30) NOT NULL DEFAULT 'NONE',
    ADD COLUMN IF NOT EXISTS max_cash_drawer_limit NUMERIC(18, 4) NOT NULL DEFAULT 2000000.0000;

-- Akun CoA Titipan Donasi Konsumen (2104)
INSERT INTO chart_of_accounts (id, account_category_id, code, name, is_header, is_system, is_active, created_at, updated_at)
SELECT gen_random_uuid(), 2, '2104', 'Titipan Donasi Konsumen Kasir', false, true, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM chart_of_accounts WHERE code = '2104');

-- 6. Pendaftaran Permissions Baru
INSERT INTO permissions (code, description) VALUES
('promotions:read', 'Melihat daftar promo otomatis ritel'),
('promotions:create', 'Membuat program promo baru (Buy X Get Y, Tebus Murah, Bundling)'),
('promotions:update', 'Mengubah konfigurasi program promo'),
('promotions:delete', 'Menghapus program promo'),
('sales_returns:read', 'Melihat data dan riwayat retur penjualan konsumen'),
('sales_returns:create', 'Memproses retur penjualan dan penukaran barang konsumen di kasir'),
('cash_drops:read', 'Melihat riwayat penarikan kas laci meja kasir'),
('cash_drops:create', 'Melakukan serah terima setor kas laci kasir ke brankas/supervisor')
ON CONFLICT (code) DO NOTHING;

-- Petakan ke SUPERADMIN & STORE_MANAGER
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p
WHERE r.name IN ('SUPERADMIN', 'STORE_MANAGER') AND p.code IN (
    'promotions:read', 'promotions:create', 'promotions:update', 'promotions:delete',
    'sales_returns:read', 'sales_returns:create',
    'cash_drops:read', 'cash_drops:create'
)
ON CONFLICT DO NOTHING;

-- Petakan ke SUPERVISOR
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p
WHERE r.name = 'SUPERVISOR' AND p.code IN (
    'promotions:read',
    'sales_returns:read', 'sales_returns:create',
    'cash_drops:read', 'cash_drops:create'
)
ON CONFLICT DO NOTHING;

-- Petakan ke CASHIER
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p
WHERE r.name = 'CASHIER' AND p.code IN (
    'promotions:read',
    'sales_returns:create',
    'cash_drops:create'
)
ON CONFLICT DO NOTHING;
