-- ============================================================================
-- MIGRASI 063: PERMISSION CADANGAN & PEMULIHAN DATABASE LENGKAP (BACKUP & RESTORE)
-- Fitur: Ekspor Database SQL ke ZIP (dengan Manifest Versi) dan Impor Pemulihan
-- ============================================================================

INSERT INTO permissions (code, description) VALUES
('database:backup', 'Mengekspor cadangan database lengkap ke berkas ZIP (SQL & Manifest Kompatibilitas)'),
('database:restore', 'Memulihkan database dari berkas ZIP cadangan SQL setelah validasi kompatibilitas versi')
ON CONFLICT (code) DO UPDATE SET
    description = EXCLUDED.description;

-- Petakan ke role tingkat manajerial & superadmin
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r
CROSS JOIN permissions p
WHERE r.name IN ('super_admin', 'admin', 'store_manager', 'general_manager')
  AND p.code IN ('database:backup', 'database:restore')
ON CONFLICT DO NOTHING;
