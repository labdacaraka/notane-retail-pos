-- ============================================================
-- Migration 035: COA System Lock
-- Menambahkan kolom `is_system` ke chart_of_accounts untuk
-- melindungi akun-akun bawaan yang terhubung ke logika sistem
-- dari perubahan yang tidak sengaja oleh pengguna.
--
-- Akun dengan is_system = TRUE:
--   - TIDAK DAPAT diedit nama/deskripsi/status via UI
--   - TIDAK DAPAT dinonaktifkan
--   - Hanya super-admin melalui migrasi yang bisa mengubahnya
-- ============================================================

-- 1. Tambah kolom is_system (default FALSE agar aman untuk data existing)
ALTER TABLE chart_of_accounts
    ADD COLUMN IF NOT EXISTS is_system BOOLEAN NOT NULL DEFAULT FALSE;

-- 2. Tandai semua akun bawaan sistem.
--    Semua akun yang di-seed oleh sistem menggunakan pola UUID:
--    a1000000-0000-0000-0000-xxxxxxxxxxxx
--    Pattern ini eksklusif untuk akun sistem, bukan akun buatan user.
UPDATE chart_of_accounts
SET is_system = TRUE
WHERE id::text LIKE 'a1000000-0000-0000-0000-%';

-- 3. Tambah index untuk mempercepat query filtering is_system
CREATE INDEX IF NOT EXISTS idx_coa_is_system
    ON chart_of_accounts (is_system);

-- 4. Tambah check constraint agar akun sistem tidak bisa di-deactivate
--    via SQL langsung (defense-in-depth tambahan)
ALTER TABLE chart_of_accounts
    DROP CONSTRAINT IF EXISTS chk_system_account_always_active;
ALTER TABLE chart_of_accounts
    ADD CONSTRAINT chk_system_account_always_active
    CHECK (NOT (is_system = TRUE AND is_active = FALSE));
