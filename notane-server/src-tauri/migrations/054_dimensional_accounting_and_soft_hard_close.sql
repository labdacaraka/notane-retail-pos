-- ============================================================================
-- Migrasi 054: Dimensional Accounting Schema & Soft/Hard Closing
-- 1. Hapus outlet_id dari master chart_of_accounts (1 set CoA untuk semua cabang)
-- 2. Hapus outlet_id dari header journal_entries (hanya mencatat tanggal & referensi)
-- 3. Tambah outlet_id ke detail journal_lines (Dimensional Accounting per baris mutasi)
-- 4. Tambah flag is_closing_entry ke journal_entries
-- 5. Tabel accounting_periods untuk Soft Close (penguncian bulanan) & Hard Close tahunan
-- ============================================================================

-- Langkah 1: Tambahkan kolom outlet_id ke journal_lines (Detail Transaksi)
ALTER TABLE journal_lines ADD COLUMN IF NOT EXISTS outlet_id UUID REFERENCES outlets(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_journal_lines_outlet_id ON journal_lines(outlet_id);

-- Langkah 2: Salin identitas cabang (outlet_id) dari header journal_entries ke detail journal_lines
UPDATE journal_lines jl
SET outlet_id = je.outlet_id
FROM journal_entries je
WHERE jl.journal_id = je.id 
  AND je.outlet_id IS NOT NULL 
  AND jl.outlet_id IS NULL;

-- Langkah 3: Reorganisasi Akun Persediaan (1201) & RAK (1901) ke Master Universal
DO $$
DECLARE
    v_parent_inv_id UUID;
    v_parent_rak_id UUID;
BEGIN
    SELECT id INTO v_parent_inv_id FROM chart_of_accounts WHERE code = '1201' LIMIT 1;
    SELECT id INTO v_parent_rak_id FROM chart_of_accounts WHERE code = '1901' LIMIT 1;

    -- Remap baris persediaan cabang ke master 1201
    IF v_parent_inv_id IS NOT NULL THEN
        -- Ambil outlet_id dari sub-account jika baris belum memiliki outlet_id
        UPDATE journal_lines jl
        SET outlet_id = coa.outlet_id
        FROM chart_of_accounts coa
        WHERE jl.account_id = coa.id 
          AND coa.outlet_id IS NOT NULL 
          AND jl.outlet_id IS NULL;

        -- Alihkan seluruh journal_lines yang memakai sub-akun persediaan ke master akun 1201
        UPDATE journal_lines jl
        SET account_id = v_parent_inv_id
        FROM chart_of_accounts coa
        WHERE jl.account_id = coa.id 
          AND coa.code LIKE '1201-%';

        -- Pastikan master akun 1201 bukan header dan berstatus sistem agar bisa diposting langsung
        UPDATE chart_of_accounts
        SET is_header = FALSE, is_system = TRUE, parent_id = NULL
        WHERE id = v_parent_inv_id;

        -- Alihkan referensi outlets.inventory_account_id ke akun master 1201
        UPDATE outlets SET inventory_account_id = v_parent_inv_id;
    END IF;

    -- Remap baris RAK cabang ke master 1901
    IF v_parent_rak_id IS NOT NULL THEN
        -- Ambil outlet_id dari sub-account jika baris belum memiliki outlet_id
        UPDATE journal_lines jl
        SET outlet_id = coa.outlet_id
        FROM chart_of_accounts coa
        WHERE jl.account_id = coa.id 
          AND coa.outlet_id IS NOT NULL 
          AND jl.outlet_id IS NULL;

        -- Alihkan seluruh journal_lines yang memakai sub-akun RAK ke master akun 1901
        UPDATE journal_lines jl
        SET account_id = v_parent_rak_id
        FROM chart_of_accounts coa
        WHERE jl.account_id = coa.id 
          AND coa.code LIKE '1901-%';

        -- Pastikan master akun 1901 bukan header dan berstatus sistem agar bisa diposting langsung
        UPDATE chart_of_accounts
        SET is_header = FALSE, is_system = TRUE, parent_id = NULL
        WHERE id = v_parent_rak_id;

        -- Alihkan referensi outlets.rak_account_id ke akun master 1901
        UPDATE outlets SET rak_account_id = v_parent_rak_id;
    END IF;
END $$;

-- Hapus sub-akun persediaan dan RAK duplikat per cabang dari chart_of_accounts
DELETE FROM chart_of_accounts WHERE code LIKE '1201-%' OR code LIKE '1901-%';

-- Langkah 4: Hapus kolom outlet_id dari tabel chart_of_accounts (1 set CoA Master)
DROP INDEX IF EXISTS idx_chart_of_accounts_outlet_id;
ALTER TABLE chart_of_accounts DROP COLUMN IF EXISTS outlet_id;

-- Langkah 5: Tambahkan flag is_closing_entry ke journal_entries
ALTER TABLE journal_entries ADD COLUMN IF NOT EXISTS is_closing_entry BOOLEAN DEFAULT FALSE NOT NULL;
UPDATE journal_entries SET is_closing_entry = TRUE WHERE reference_type = 'CLOSING_ENTRY';

-- Langkah 6: Hapus kolom outlet_id dari header journal_entries
DROP INDEX IF EXISTS idx_journal_entries_outlet_id;
ALTER TABLE journal_entries DROP COLUMN IF EXISTS outlet_id;

-- Langkah 7: Buat tabel accounting_periods untuk Soft Close (Penguncian Periode)
CREATE TABLE IF NOT EXISTS accounting_periods (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL,
    start_date DATE NOT NULL,
    end_date DATE NOT NULL,
    is_closed BOOLEAN NOT NULL DEFAULT FALSE,
    closed_at TIMESTAMPTZ,
    closed_by UUID REFERENCES users(id) ON DELETE SET NULL,
    reopened_at TIMESTAMPTZ,
    reopened_by UUID REFERENCES users(id) ON DELETE SET NULL,
    reopen_reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT uq_accounting_period_dates UNIQUE (start_date, end_date)
);

CREATE INDEX IF NOT EXISTS idx_accounting_periods_dates ON accounting_periods(start_date, end_date);
CREATE INDEX IF NOT EXISTS idx_accounting_periods_is_closed ON accounting_periods(is_closed);

-- Langkah 8: Daftarkan permission RBAC
INSERT INTO permissions (code, description, created_at)
VALUES
    ('accounting:periods:read', 'Melihat daftar dan status penguncian periode akuntansi', CURRENT_TIMESTAMP),
    ('accounting:periods:close', 'Mengunci transaksi pada periode akuntansi tertentu (Soft Close)', CURRENT_TIMESTAMP),
    ('accounting:periods:reopen', 'Membuka kembali kunci periode akuntansi', CURRENT_TIMESTAMP),
    ('accounting:hard_close', 'Melakukan jurnal penutupan tahunan / akhir periode (Hard Close)', CURRENT_TIMESTAMP)
ON CONFLICT (code) DO NOTHING;

INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r
CROSS JOIN permissions p
WHERE r.name = 'SUPERADMIN' AND p.code IN (
    'accounting:periods:read',
    'accounting:periods:close',
    'accounting:periods:reopen',
    'accounting:hard_close'
)
ON CONFLICT (role_id, permission_id) DO NOTHING;
