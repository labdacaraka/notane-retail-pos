-- ============================================================================
-- MIGRATION: 021_accounting_period_closing_and_balance_sheet.sql
-- Description: Menambahkan outlet_id ke journal_entries, akun Laba Ditahan & Ikhtisar L/R,
--              serta tabel accounting_closings untuk sistem tutup buku akuntansi.
-- ============================================================================

-- 1. Tambah kolom outlet_id ke tabel journal_entries jika belum ada
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'journal_entries' AND column_name = 'outlet_id'
    ) THEN
        ALTER TABLE journal_entries ADD COLUMN outlet_id UUID REFERENCES outlets(id) ON DELETE SET NULL;
        CREATE INDEX IF NOT EXISTS idx_journal_entries_outlet_id ON journal_entries(outlet_id);
    END IF;
END $$;

-- 2. Pastikan akun penutup standar (Laba Ditahan & Ikhtisar Laba Rugi) tersedia
-- Kategori 3 = EQUITY (Normal Balance: CREDIT)
INSERT INTO chart_of_accounts (id, account_category_id, code, name, is_header, is_active, created_at, updated_at)
SELECT gen_random_uuid(), 3, '3201', 'Laba Ditahan (Retained Earnings)', false, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM chart_of_accounts WHERE code = '3201');

INSERT INTO chart_of_accounts (id, account_category_id, code, name, is_header, is_active, created_at, updated_at)
SELECT gen_random_uuid(), 3, '3901', 'Ikhtisar Laba Rugi (Income Summary)', false, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM chart_of_accounts WHERE code = '3901');

-- 3. Tabel Riwayat Tutup Buku Akuntansi (Accounting Period Closings)
CREATE TABLE IF NOT EXISTS accounting_closings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    closing_number VARCHAR(50) UNIQUE NOT NULL,
    period_start DATE NOT NULL,
    period_end DATE NOT NULL,
    closing_date DATE NOT NULL,
    outlet_id UUID REFERENCES outlets(id) ON DELETE SET NULL, -- NULL = Konsolidasi Seluruh Cabang
    total_revenue NUMERIC(18, 4) NOT NULL DEFAULT 0,
    total_expense NUMERIC(18, 4) NOT NULL DEFAULT 0,
    net_income NUMERIC(18, 4) NOT NULL DEFAULT 0,
    journal_entry_id UUID REFERENCES journal_entries(id) ON DELETE SET NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'CLOSED', -- 'CLOSED', 'REOPENED'
    notes TEXT,
    reopened_at TIMESTAMPTZ,
    reopened_by UUID REFERENCES users(id),
    reopen_reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id)
);

CREATE INDEX IF NOT EXISTS idx_accounting_closings_status ON accounting_closings(status);
CREATE INDEX IF NOT EXISTS idx_accounting_closings_dates ON accounting_closings(period_start, period_end);
CREATE INDEX IF NOT EXISTS idx_accounting_closings_outlet ON accounting_closings(outlet_id);
