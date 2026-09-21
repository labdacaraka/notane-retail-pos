-- ============================================================================
-- MIGRATION 008: USER OUTLETS ACCESS CONTROL
-- Mengatur hak akses multi-lokasi / outlet per akun pengguna
-- ============================================================================

-- 1. Buat tabel user_outlets
CREATE TABLE IF NOT EXISTS user_outlets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    outlet_id UUID NOT NULL REFERENCES outlets(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, outlet_id)
);

CREATE INDEX IF NOT EXISTS idx_user_outlets_user_id ON user_outlets(user_id);
CREATE INDEX IF NOT EXISTS idx_user_outlets_outlet_id ON user_outlets(outlet_id);

-- 2. Berikan akses default awal untuk user yang sudah ada ke outlet yang berstatus aktif
-- Agar akun lama tidak terkunci secara tiba-tiba
INSERT INTO user_outlets (user_id, outlet_id)
SELECT u.id, o.id
FROM users u
CROSS JOIN outlets o
WHERE o.is_active = TRUE
ON CONFLICT (user_id, outlet_id) DO NOTHING;
