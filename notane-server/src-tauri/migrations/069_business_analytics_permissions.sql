-- Migration: 069_business_analytics_permissions.sql
-- Pendaftaran hak akses menu statistik dan analitik bisnis

INSERT INTO permissions (slug, name, module, description, created_at)
VALUES (
    'analytics:read',
    'Melihat Statistik & Analitik Bisnis',
    'report',
    'Mengakses dasbor eksekutif, grafik tren finansial, metrik penjualan, pembelian, beban, dan laba operasional',
    CURRENT_TIMESTAMP
)
ON CONFLICT (slug) DO NOTHING;

-- Berikan hak akses ke role super_admin, store_manager, dan supervisor
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r, permissions p
WHERE r.name IN ('super_admin', 'SUPERADMIN', 'STORE_MANAGER', 'store_manager', 'SUPERVISOR', 'supervisor')
  AND p.slug = 'analytics:read'
ON CONFLICT DO NOTHING;

