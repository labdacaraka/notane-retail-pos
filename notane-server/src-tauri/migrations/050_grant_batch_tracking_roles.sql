-- ============================================================================
-- MIGRATION 050: GRANT INVENTORY BATCH TRACKING PERMISSION TO STORE ROLES
-- ============================================================================

INSERT INTO permissions (code, description) VALUES
('inventory:batch_tracking', 'Melihat pelacakan alokasi batch stok penjualan ke pembelian dan transfer stok')
ON CONFLICT (code) DO NOTHING;

-- Berikan akses ke SUPERADMIN, SUPERVISOR, STORE_MANAGER, dan WAREHOUSE
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r, permissions p
WHERE r.name IN ('SUPERADMIN', 'SUPERVISOR', 'STORE_MANAGER', 'WAREHOUSE')
  AND p.code = 'inventory:batch_tracking'
ON CONFLICT DO NOTHING;
