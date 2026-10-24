-- ============================================================================
-- Migrasi 057: Dividend Distributions Linked to Accounting Closed Periods
-- 1. Tabel dividend_distributions untuk mencatat pembagian dividen dari Laba Ditahan
-- 2. Memastikan relasi ke accounting_periods (wajib periode tutup buku)
-- 3. Akun CoA 3301 (Dividen / Pembagian Laba)
-- 4. Hak akses RBAC (accounting:dividend:*)
-- ============================================================================

-- Langkah 1: Pastikan akun CoA 3301 (Dividen / Pembagian Laba) tersedia
INSERT INTO chart_of_accounts (id, account_category_id, code, name, is_header, is_active, is_system, created_at, updated_at)
SELECT gen_random_uuid(), 3, '3301', 'Dividen / Pembagian Laba', false, true, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM chart_of_accounts WHERE code = '3301');

-- Langkah 2: Tabel dividend_distributions
CREATE TABLE IF NOT EXISTS dividend_distributions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    distribution_number VARCHAR(50) UNIQUE NOT NULL,
    period_id UUID NOT NULL REFERENCES accounting_periods(id) ON DELETE RESTRICT,
    distribution_date DATE NOT NULL,
    total_amount NUMERIC(18, 4) NOT NULL CHECK (total_amount > 0),
    retained_earnings_account_id UUID NOT NULL REFERENCES chart_of_accounts(id),
    payment_account_id UUID NOT NULL REFERENCES chart_of_accounts(id),
    journal_entry_id UUID REFERENCES journal_entries(id) ON DELETE SET NULL,
    recipient_name VARCHAR(150) NOT NULL,
    notes TEXT,
    recipients_data JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

CREATE INDEX IF NOT EXISTS idx_dividend_distributions_period_id ON dividend_distributions(period_id);
CREATE INDEX IF NOT EXISTS idx_dividend_distributions_date ON dividend_distributions(distribution_date);

-- Langkah 3: Daftarkan permission RBAC
INSERT INTO permissions (code, description, created_at)
VALUES
    ('accounting:dividend:read', 'Melihat daftar dan riwayat pembagian dividen laba', CURRENT_TIMESTAMP),
    ('accounting:dividend:create', 'Memproses pembagian dividen dari laba periode tutup buku', CURRENT_TIMESTAMP),
    ('accounting:dividend:delete', 'Membatalkan transaksi pembagian dividen dan membalikkan jurnal', CURRENT_TIMESTAMP)
ON CONFLICT (code) DO NOTHING;

INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r
CROSS JOIN permissions p
WHERE r.name = 'SUPERADMIN' AND p.code IN (
    'accounting:dividend:read',
    'accounting:dividend:create',
    'accounting:dividend:delete'
)
ON CONFLICT (role_id, permission_id) DO NOTHING;
