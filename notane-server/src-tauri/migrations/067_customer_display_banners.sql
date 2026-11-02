-- ============================================================================
-- Migration 067: Customer Display Promotional Banners
-- Menambahkan tabel banner promosi untuk Layar Hadap Pelanggan (Customer Display),
-- pendaftaran hak akses RBAC, dan pemetaan ke default roles.
-- ============================================================================

-- 1. Buat Tabel customer_display_banners
CREATE TABLE IF NOT EXISTS customer_display_banners (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title VARCHAR(255) NOT NULL,
    subtitle TEXT,
    image_url TEXT NOT NULL,
    brand_name VARCHAR(150),
    badge_text VARCHAR(50) DEFAULT 'PROMO SPESIAL',
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    display_order INT NOT NULL DEFAULT 0,
    start_date DATE,
    end_date DATE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    updated_by UUID REFERENCES users(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_cust_display_banners_active 
    ON customer_display_banners(is_active, display_order ASC, created_at DESC);

-- Trigger auto-update timestamp
DROP TRIGGER IF EXISTS trg_update_timestamp_cust_display_banners ON customer_display_banners;
CREATE TRIGGER trg_update_timestamp_cust_display_banners
    BEFORE UPDATE ON customer_display_banners
    FOR EACH ROW EXECUTE FUNCTION update_timestamp_column();

-- 2. Pendaftaran Permissions Baru
INSERT INTO permissions (code, description) VALUES
('display_banners:read', 'Melihat daftar banner promosi dan tayangan di Customer Display'),
('display_banners:create', 'Menambahkan banner promosi baru untuk Customer Display'),
('display_banners:update', 'Mengubah pengaturan, urutan, dan masa berlaku banner promosi'),
('display_banners:delete', 'Menghapus banner promosi dari sistem Customer Display')
ON CONFLICT (code) DO NOTHING;

-- 3. Petakan Hak Akses ke Default Roles
-- SUPERADMIN & STORE_MANAGER: Hak Penuh
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p
WHERE r.name IN ('SUPERADMIN', 'STORE_MANAGER') 
  AND p.code IN (
      'display_banners:read',
      'display_banners:create',
      'display_banners:update',
      'display_banners:delete'
  )
ON CONFLICT DO NOTHING;

-- SUPERVISOR & CASHIER: Hak Melihat (Read)
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p
WHERE r.name IN ('SUPERVISOR', 'CASHIER') 
  AND p.code = 'display_banners:read'
ON CONFLICT DO NOTHING;

