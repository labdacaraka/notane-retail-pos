-- 004_pin_management_permission.sql
-- Tambahkan permission khusus untuk hak otorisasi mengubah PIN pengguna lain

INSERT INTO permissions (code, description) VALUES
('users:change_pin', 'Mengubah atau mereset PIN otorisasi pengguna lain')
ON CONFLICT (code) DO NOTHING;

-- Berikan hak akses users:change_pin secara default ke SUPERADMIN dan STORE_MANAGER
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p 
WHERE r.name IN ('SUPERADMIN', 'STORE_MANAGER') AND p.code = 'users:change_pin'
ON CONFLICT DO NOTHING;
