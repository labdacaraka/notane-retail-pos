-- ============================================================================
-- MIGRASI 005: OUTLETS ENHANCEMENT & PERMISSION-BASED AUTH
-- ============================================================================

-- 1. Tambahkan permission baru untuk otorisasi & manajemen outlet
INSERT INTO permissions (code, description) VALUES
('auth:supervisor', 'Hak Otorisasi Supervisor Kasir & Operasional POS (Void, Approval, Buka/Tutup Register)'),
('auth:store_manager', 'Hak Otorisasi & Tanggung Jawab Store Manager Toko'),
('outlets:read', 'Melihat Master Data Outlet Gerai Toko'),
('outlets:manage', 'Mengelola Master Data Outlet dan Penugasan Staf Toko')
ON CONFLICT (code) DO NOTHING;

-- 2. Petakan permission baru ke role yang sesuai
-- SUPERADMIN mendapatkan seluruh permission baru
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p 
WHERE r.name = 'SUPERADMIN' AND p.code IN ('auth:supervisor', 'auth:store_manager', 'outlets:read', 'outlets:manage')
ON CONFLICT DO NOTHING;

-- STORE_MANAGER mendapatkan hak store manager, supervisor, dan melihat/mengelola outlet
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p 
WHERE r.name = 'STORE_MANAGER' AND p.code IN ('auth:supervisor', 'auth:store_manager', 'outlets:read', 'outlets:manage')
ON CONFLICT DO NOTHING;

-- SUPERVISOR mendapatkan hak supervisor dan melihat outlet
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p 
WHERE r.name = 'SUPERVISOR' AND p.code IN ('auth:supervisor', 'outlets:read')
ON CONFLICT DO NOTHING;

-- 3. Tambahkan kolom manager_id dan default_warehouse_id ke tabel outlets jika belum ada
ALTER TABLE outlets ADD COLUMN IF NOT EXISTS manager_id UUID REFERENCES users(id);
ALTER TABLE outlets ADD COLUMN IF NOT EXISTS default_warehouse_id UUID REFERENCES warehouses(id);

-- 4. Buat tabel relasi outlet_supervisors untuk menampung multi-supervisor per outlet
CREATE TABLE IF NOT EXISTS outlet_supervisors (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    outlet_id UUID NOT NULL REFERENCES outlets(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    CONSTRAINT uk_outlet_user_supervisor UNIQUE (outlet_id, user_id)
);

-- 5. Seed default: set store manager & default warehouse untuk outlet default jika ada
DO $$
DECLARE
    v_outlet_id UUID;
    v_spv_id UUID;
    v_admin_id UUID;
    v_wh_id UUID;
BEGIN
    SELECT id INTO v_outlet_id FROM outlets WHERE code = 'OUT-001' LIMIT 1;
    SELECT id INTO v_spv_id FROM users WHERE username = 'spv' LIMIT 1;
    SELECT id INTO v_admin_id FROM users WHERE username = 'admin' LIMIT 1;
    SELECT id INTO v_wh_id FROM warehouses WHERE code = 'WH-001' LIMIT 1;

    IF v_outlet_id IS NOT NULL THEN
        -- Set manager_id jika masih null
        UPDATE outlets 
        SET manager_id = COALESCE(manager_id, v_admin_id),
            default_warehouse_id = COALESCE(default_warehouse_id, v_wh_id)
        WHERE id = v_outlet_id;

        -- Tugaskan spv dan admin ke outlet_supervisors OUT-001
        IF v_spv_id IS NOT NULL THEN
            INSERT INTO outlet_supervisors (outlet_id, user_id)
            VALUES (v_outlet_id, v_spv_id)
            ON CONFLICT DO NOTHING;
        END IF;

        IF v_admin_id IS NOT NULL THEN
            INSERT INTO outlet_supervisors (outlet_id, user_id)
            VALUES (v_outlet_id, v_admin_id)
            ON CONFLICT DO NOTHING;
        END IF;
    END IF;
END $$;
