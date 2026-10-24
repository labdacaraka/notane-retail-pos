-- ============================================================================
-- MIGRATION 044: USER CURRENT OUTLET PERSONALIZATION
-- Menyimpan preferensi dan sesi lokasi outlet aktif yang dipersonalisasi per pengguna
-- ============================================================================

-- 1. Tambah kolom current_outlet_id pada tabel users
ALTER TABLE users ADD COLUMN IF NOT EXISTS current_outlet_id UUID REFERENCES outlets(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_users_current_outlet_id ON users(current_outlet_id);

-- 2. Inisialisasi outlet aktif awal berdasarkan outlet yang diakses pengguna
-- Prioritaskan toko fisik (is_warehouse = FALSE) jika ada
UPDATE users u
SET current_outlet_id = (
    SELECT o.id 
    FROM user_outlets uo
    JOIN outlets o ON uo.outlet_id = o.id
    WHERE uo.user_id = u.id AND o.is_active = TRUE
    ORDER BY o.is_warehouse ASC, o.code ASC
    LIMIT 1
)
WHERE u.current_outlet_id IS NULL;

-- 3. Jika pengguna adalah superadmin dan belum memiliki relasi terdaftar di user_outlets,
-- pasangkan ke outlet aktif pertama yang ada di sistem
UPDATE users u
SET current_outlet_id = (
    SELECT id 
    FROM outlets 
    WHERE is_active = TRUE 
    ORDER BY is_warehouse ASC, code ASC 
    LIMIT 1
)
WHERE u.current_outlet_id IS NULL;
