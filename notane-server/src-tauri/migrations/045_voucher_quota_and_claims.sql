-- Migration 045: Voucher Quota & Claim Tracking
ALTER TABLE vouchers ADD COLUMN IF NOT EXISTS quota INT NOT NULL DEFAULT 0;
ALTER TABLE vouchers ADD COLUMN IF NOT EXISTS used_count INT NOT NULL DEFAULT 0;

-- Indeks untuk efisiensi query kedaluwarsa voucher
CREATE INDEX IF NOT EXISTS idx_vouchers_status_valid_until ON vouchers (status, valid_until);

-- Permission untuk melihat riwayat klaim voucher
INSERT INTO permissions (code, description)
VALUES ('vouchers:claims', 'Melihat riwayat transaksi klaim penggunaan kupon voucher diskon')
ON CONFLICT (code) DO NOTHING;

-- Berikan izin ke SUPERADMIN, STORE_MANAGER, dan SUPERVISOR
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r, permissions p
WHERE r.name IN ('SUPERADMIN', 'STORE_MANAGER', 'SUPERVISOR')
  AND p.code = 'vouchers:claims'
ON CONFLICT DO NOTHING;
