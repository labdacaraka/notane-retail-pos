-- ============================================================================
-- MIGRASI 068: SISTEM TRANSAKSI PENDAPATAN LAIN-LAIN (OTHER INCOME TRANSACTIONS)
-- ============================================================================

-- 1. Permission Registrasi (Mandatory RBAC)
INSERT INTO permissions (code, description) VALUES
('other_income:read', 'Melihat daftar, ringkasan, dan rincian transaksi pendapatan lain-lain'),
('other_income:create', 'Mencatat transaksi penerimaan kas pendapatan lain-lain'),
('other_income:delete', 'Membatalkan atau void transaksi pendapatan lain-lain'),
('other_income:categories', 'Mengelola master kategori akun pendapatan lain-lain')
ON CONFLICT (code) DO NOTHING;

-- Petakan ke Role SUPERADMIN
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p
WHERE r.name = 'SUPERADMIN' AND p.code IN (
    'other_income:read', 'other_income:create', 'other_income:delete', 'other_income:categories'
)
ON CONFLICT DO NOTHING;

-- Petakan ke Role STORE_MANAGER
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p
WHERE r.name = 'STORE_MANAGER' AND p.code IN (
    'other_income:read', 'other_income:create', 'other_income:delete', 'other_income:categories'
)
ON CONFLICT DO NOTHING;

-- Petakan ke Role SUPERVISOR
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p
WHERE r.name = 'SUPERVISOR' AND p.code IN (
    'other_income:read', 'other_income:create'
)
ON CONFLICT DO NOTHING;

-- Petakan ke Role CASHIER
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p
WHERE r.name = 'CASHIER' AND p.code IN (
    'other_income:read', 'other_income:create'
)
ON CONFLICT DO NOTHING;


-- 2. Chart of Accounts Tambahan untuk Pendapatan Lain-lain (REVENUE - Category 4)
INSERT INTO chart_of_accounts (id, account_category_id, code, name, is_header, is_system, is_active, created_at, updated_at)
SELECT gen_random_uuid(), 4, '4401', 'Pendapatan Sewa & Kerjasama Promosi', false, true, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM chart_of_accounts WHERE code = '4401');

INSERT INTO chart_of_accounts (id, account_category_id, code, name, is_header, is_system, is_active, created_at, updated_at)
SELECT gen_random_uuid(), 4, '4402', 'Pendapatan Jasa Titip & Komisi Konsinyasi', false, true, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM chart_of_accounts WHERE code = '4402');

INSERT INTO chart_of_accounts (id, account_category_id, code, name, is_header, is_system, is_active, created_at, updated_at)
SELECT gen_random_uuid(), 4, '4403', 'Pendapatan Penjualan Limbah & Kardus Toko', false, true, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM chart_of_accounts WHERE code = '4403');

INSERT INTO chart_of_accounts (id, account_category_id, code, name, is_header, is_system, is_active, created_at, updated_at)
SELECT gen_random_uuid(), 4, '4499', 'Pendapatan Non-Operasional & Lain-lain', false, true, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM chart_of_accounts WHERE code = '4499');


-- 3. Tabel Kategori Pendapatan Lain-lain (Other Income Categories)
CREATE TABLE IF NOT EXISTS other_income_categories (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code VARCHAR(50) UNIQUE NOT NULL,
    name VARCHAR(150) NOT NULL,
    account_id UUID REFERENCES chart_of_accounts(id) ON DELETE SET NULL,
    description TEXT,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Seed Default Kategori Pendapatan Lain-lain
INSERT INTO other_income_categories (id, code, name, account_id, description) VALUES
('b2000000-0000-0000-0000-000000000001'::uuid, 'SEWA_PROMOSI', 'Sewa Tempat & Banner Promosi Toko', (SELECT id FROM chart_of_accounts WHERE code = '4401' LIMIT 1), 'Sewa space booth, banner promosi brand, display rak promo'),
('b2000000-0000-0000-0000-000000000002'::uuid, 'KOMISI', 'Komisi, Jastip & Konsinyasi Toko', (SELECT id FROM chart_of_accounts WHERE code = '4402' LIMIT 1), 'Bagi hasil konsinyasi barang UMKM, komisi jasa titip atau layanan'),
('b2000000-0000-0000-0000-000000000003'::uuid, 'LIMBAH', 'Penjualan Kardus & Limbah Toko', (SELECT id FROM chart_of_accounts WHERE code = '4403' LIMIT 1), 'Hasil penjualan kardus bekas packing, karung, palet, dan limbah plastik toko'),
('b2000000-0000-0000-0000-000000000004'::uuid, 'BUNGA_BANK', 'Bunga Rekening Bank & Jasa Giro', (SELECT id FROM chart_of_accounts WHERE code = '4301' LIMIT 1), 'Pendapatan bunga tabungan dan jasa giro rekening operasional toko'),
('b2000000-0000-0000-0000-000000000005'::uuid, 'LAIN_LAIN', 'Pendapatan Non-Operasional Lainnya', (SELECT id FROM chart_of_accounts WHERE code = '4499' LIMIT 1), 'Penerimaan pendapatan di luar operasional utama toko')
ON CONFLICT (code) DO NOTHING;


-- 4. Tabel Transaksi Pendapatan Lain-lain (Other Income Transactions)
CREATE TABLE IF NOT EXISTS other_income_transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    transaction_no VARCHAR(50) UNIQUE NOT NULL,
    outlet_id UUID NOT NULL REFERENCES outlets(id) ON DELETE RESTRICT,
    category_id UUID REFERENCES other_income_categories(id) ON DELETE SET NULL,
    income_account_id UUID NOT NULL REFERENCES chart_of_accounts(id) ON DELETE RESTRICT,
    payment_account_id UUID NOT NULL REFERENCES chart_of_accounts(id) ON DELETE RESTRICT,
    amount NUMERIC(18, 4) NOT NULL CHECK (amount > 0),
    transaction_date DATE NOT NULL DEFAULT CURRENT_DATE,
    payer_name VARCHAR(150),
    notes TEXT,
    reference_no VARCHAR(100),
    attachment_url TEXT,
    shift_id UUID REFERENCES cashier_shifts(id) ON DELETE SET NULL,
    journal_entry_id UUID REFERENCES journal_entries(id) ON DELETE SET NULL,
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    cancelled_by UUID REFERENCES users(id) ON DELETE SET NULL,
    cancelled_at TIMESTAMPTZ,
    cancellation_reason TEXT,
    status VARCHAR(20) NOT NULL DEFAULT 'COMPLETED' CHECK (status IN ('COMPLETED', 'CANCELLED')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Indeks Kinerja
CREATE INDEX IF NOT EXISTS idx_other_income_transactions_outlet_date ON other_income_transactions(outlet_id, transaction_date DESC);
CREATE INDEX IF NOT EXISTS idx_other_income_transactions_shift ON other_income_transactions(shift_id);
CREATE INDEX IF NOT EXISTS idx_other_income_transactions_status ON other_income_transactions(status);
CREATE INDEX IF NOT EXISTS idx_other_income_transactions_category ON other_income_transactions(category_id);

