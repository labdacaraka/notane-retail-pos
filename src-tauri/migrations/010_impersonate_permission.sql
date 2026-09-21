-- 010_impersonate_permission.sql
-- Menambahkan permission untuk mode impersonate pengguna

INSERT INTO permissions (code, description) VALUES
('users:impersonate', 'Mode Impersonate: Masuk langsung sebagai pengguna lain tanpa login/logout')
ON CONFLICT (code) DO NOTHING;

-- Berikan ke SUPERADMIN secara default
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p 
WHERE r.name = 'SUPERADMIN' AND p.code = 'users:impersonate'
ON CONFLICT DO NOTHING;
