-- ============================================================================
-- MIGRASI 028: PERMISSIONS UNTUK FITUR LOYALITAS DAN POIN MEMBER
-- ============================================================================

-- 1. Tambahkan permission untuk loyalty & point rewards
INSERT INTO permissions (code, description) VALUES
('loyalty:manage', 'Mengatur konfigurasi konversi poin, batas persentase, dan aturan loyalitas'),
('loyalty:redeem', 'Otorisasi penukaran poin reward loyalitas di kasir'),
('loyalty:read', 'Melihat konfigurasi program loyalitas dan saldo poin member')
ON CONFLICT (code) DO NOTHING;

-- 2. Petakan ke Role SUPERADMIN (Seluruh Hak Akses)
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p
WHERE r.name = 'SUPERADMIN' AND p.code IN ('loyalty:manage', 'loyalty:redeem', 'loyalty:read')
ON CONFLICT DO NOTHING;

-- 3. Petakan ke Role STORE_MANAGER
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p
WHERE r.name = 'STORE_MANAGER' AND p.code IN ('loyalty:manage', 'loyalty:redeem', 'loyalty:read')
ON CONFLICT DO NOTHING;

-- 4. Petakan ke Role SUPERVISOR
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p
WHERE r.name = 'SUPERVISOR' AND p.code IN ('loyalty:read', 'loyalty:redeem')
ON CONFLICT DO NOTHING;

-- 5. Petakan ke Role CASHIER
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p
WHERE r.name = 'CASHIER' AND p.code IN ('loyalty:read', 'loyalty:redeem')
ON CONFLICT DO NOTHING;
