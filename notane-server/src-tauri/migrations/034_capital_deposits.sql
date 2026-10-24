-- ============================================================================
-- MIGRASI 034: TRANSAKSI SETORAN MODAL USAHA & EKUITAS PEMILIK (CAPITAL DEPOSITS)
-- ============================================================================

-- 1. Permission Registrasi (Mandatory RBAC)
INSERT INTO permissions (code, description) VALUES
('capital:read', 'Melihat daftar riwayat, ringkasan, dan rincian transaksi setoran modal pemilik'),
('capital:create', 'Mencatat transaksi penambahan dan setoran modal usaha ke kas/bank'),
('capital:delete', 'Membatalkan transaksi setoran modal usaha')
ON CONFLICT (code) DO NOTHING;

-- Petakan ke Role SUPERADMIN
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p
WHERE r.name = 'SUPERADMIN' AND p.code IN (
    'capital:read', 'capital:create', 'capital:delete'
)
ON CONFLICT DO NOTHING;

-- Petakan ke Role STORE_MANAGER
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p
WHERE r.name = 'STORE_MANAGER' AND p.code IN (
    'capital:read', 'capital:create', 'capital:delete'
)
ON CONFLICT DO NOTHING;


-- 2. Chart of Accounts Tambahan untuk Ekuitas Pemilik (Category 3 = EQUITY)
INSERT INTO chart_of_accounts (id, account_category_id, code, name, is_header, is_active)
VALUES 
('a1000000-0000-0000-0000-000000000031'::uuid, 3, '3102', 'Tambahan Modal Disetor (Paid-in Capital)', false, true),
('a1000000-0000-0000-0000-000000000032'::uuid, 3, '3103', 'Prive / Penarikan Modal Pemilik (Drawings)', false, true)
ON CONFLICT (code) DO NOTHING;


-- 3. Tabel Transaksi Setoran Modal (Capital Deposits)
CREATE TABLE IF NOT EXISTS capital_deposits (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    deposit_no VARCHAR(50) UNIQUE NOT NULL,
    outlet_id UUID REFERENCES outlets(id) ON DELETE SET NULL,
    deposit_date DATE NOT NULL DEFAULT CURRENT_DATE,
    investor_name VARCHAR(150) NOT NULL,
    amount NUMERIC(18, 4) NOT NULL CHECK (amount > 0),
    payment_account_id UUID NOT NULL REFERENCES chart_of_accounts(id) ON DELETE RESTRICT,
    equity_account_id UUID NOT NULL REFERENCES chart_of_accounts(id) ON DELETE RESTRICT,
    reference_no VARCHAR(100),
    notes TEXT,
    attachment_url TEXT,
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
CREATE INDEX IF NOT EXISTS idx_capital_deposits_outlet_date ON capital_deposits(outlet_id, deposit_date DESC);
CREATE INDEX IF NOT EXISTS idx_capital_deposits_status ON capital_deposits(status);
CREATE INDEX IF NOT EXISTS idx_capital_deposits_payment ON capital_deposits(payment_account_id);
CREATE INDEX IF NOT EXISTS idx_capital_deposits_equity ON capital_deposits(equity_account_id);
