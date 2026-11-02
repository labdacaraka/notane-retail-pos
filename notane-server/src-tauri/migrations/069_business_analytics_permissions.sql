-- Migration: 069_business_analytics_permissions.sql
-- Pendaftaran hak akses menu statistik dan analitik bisnis

INSERT INTO permissions (code, description)
VALUES (
    'analytics:read',
    'Mengakses dasbor eksekutif, grafik tren finansial, metrik penjualan, pembelian, beban, dan laba operasional'
)
ON CONFLICT (code) DO NOTHING;

-- Berikan hak akses ke role SUPERADMIN, STORE_MANAGER, dan SUPERVISOR
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r, permissions p
WHERE r.name IN ('SUPERADMIN', 'STORE_MANAGER', 'SUPERVISOR')
  AND p.code = 'analytics:read'
ON CONFLICT DO NOTHING;

