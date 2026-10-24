-- ============================================================
-- Migration 036: Lock All System COA & Bank Accounts (Bank Penerimaan)
-- 1. Mengunci secara total semua akun COA bawaan sistem, termasuk:
--    - Pajak PPN Masukan (1104) dan PPN Keluaran (2102)
--    - Seluruh akun persediaan outlet (1201-%)
--    - Akun persediaan in-transit (1202)
--    - Akun hutang penerimaan belum ditagih (2103)
--    - Akun laba ditahan (3201) & ikhtisar laba rugi (3901)
--    - Akun diskon, retur, & biaya pembelian (5102, 5103, 5104)
--    - Seluruh akun bawaan lainnya
-- 2. Membuat tabel `bank_accounts` untuk CRUD Rekening Bank Penerimaan Khusus
-- 3. Mendaftarkan permissions `bank:*` ke sistem RBAC
-- ============================================================

-- 1. Kunci SEMUA akun COA yang ada saat ini sebagai akun sistem terlindungi
UPDATE chart_of_accounts
SET is_system = TRUE;

-- 2. Buat tabel master `bank_accounts` (Bank Penerimaan Usaha)
CREATE TABLE IF NOT EXISTS bank_accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    bank_name VARCHAR(100) NOT NULL,
    account_number VARCHAR(50) NOT NULL,
    account_holder VARCHAR(100) NOT NULL,
    branch VARCHAR(100),
    outlet_id UUID REFERENCES outlets(id) ON DELETE SET NULL,
    coa_id UUID REFERENCES chart_of_accounts(id) ON DELETE RESTRICT,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    is_default BOOLEAN NOT NULL DEFAULT FALSE,
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_bank_accounts_outlet_id ON bank_accounts(outlet_id);
CREATE INDEX IF NOT EXISTS idx_bank_accounts_coa_id ON bank_accounts(coa_id);

-- 3. Inisialisasi Bank Penerimaan default dari akun 1102 jika tabel masih kosong
DO $$
DECLARE
    v_coa_1102_id UUID;
BEGIN
    SELECT id INTO v_coa_1102_id FROM chart_of_accounts WHERE code = '1102' LIMIT 1;
    IF v_coa_1102_id IS NOT NULL AND NOT EXISTS (SELECT 1 FROM bank_accounts) THEN
        INSERT INTO bank_accounts (
            bank_name,
            account_number,
            account_holder,
            branch,
            outlet_id,
            coa_id,
            is_active,
            is_default,
            notes
        ) VALUES (
            'Bank Operasional Utama',
            'REK-UTAMA-01',
            'PT Notane Retail',
            'Kantor Pusat',
            NULL,
            v_coa_1102_id,
            TRUE,
            TRUE,
            'Rekening bank penerimaan utama untuk settlement QRIS, EDC, dan transfer kasir.'
        );
    END IF;
END $$;

-- 4. Daftarkan Hak Akses (Permissions) untuk modul Bank Penerimaan
INSERT INTO permissions (code, description) VALUES
('bank:read', 'Melihat daftar rekening bank penerimaan usaha'),
('bank:create', 'Menambahkan rekening bank penerimaan usaha baru'),
('bank:update', 'Mengubah informasi rekening bank penerimaan usaha'),
('bank:delete', 'Menonaktifkan atau menghapus rekening bank penerimaan')
ON CONFLICT (code) DO NOTHING;

-- Berikan hak akses ke role SUPERADMIN dan ADMIN secara default
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r, permissions p
WHERE r.name IN ('SUPERADMIN', 'ADMIN')
  AND p.code IN ('bank:read', 'bank:create', 'bank:update', 'bank:delete')
ON CONFLICT DO NOTHING;
