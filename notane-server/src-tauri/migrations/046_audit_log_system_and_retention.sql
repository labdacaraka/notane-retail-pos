-- Migration 046: Audit Log Interceptor, Request Tracking, and Auto-Retention System

-- 1. Penambahan Kolom Pelacakan HTTP Request pada Tabel audit_logs
ALTER TABLE audit_logs ALTER COLUMN action TYPE VARCHAR(100);
ALTER TABLE audit_logs ALTER COLUMN entity_name TYPE VARCHAR(100);
ALTER TABLE audit_logs ALTER COLUMN terminal_id DROP NOT NULL;
ALTER TABLE audit_logs ALTER COLUMN terminal_id SET DEFAULT 'SYSTEM';

ALTER TABLE audit_logs ADD COLUMN IF NOT EXISTS http_method VARCHAR(10);
ALTER TABLE audit_logs ADD COLUMN IF NOT EXISTS endpoint TEXT;
ALTER TABLE audit_logs ADD COLUMN IF NOT EXISTS status_code INT DEFAULT 200;
ALTER TABLE audit_logs ADD COLUMN IF NOT EXISTS execution_time_ms INT DEFAULT 0;
ALTER TABLE audit_logs ADD COLUMN IF NOT EXISTS user_agent TEXT;
ALTER TABLE audit_logs ADD COLUMN IF NOT EXISTS user_name VARCHAR(100);
ALTER TABLE audit_logs ADD COLUMN IF NOT EXISTS role_name VARCHAR(50);

-- 2. Index untuk Performa Query dan Scheduler Pembersihan Retensi
CREATE INDEX IF NOT EXISTS idx_audit_logs_created_at ON audit_logs(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_audit_logs_user_id ON audit_logs(user_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_entity_name ON audit_logs(entity_name);
CREATE INDEX IF NOT EXISTS idx_audit_logs_http_method ON audit_logs(http_method);

-- 3. Penambahan Kolom Retensi Log pada Tabel receipt_settings (Default 30 Hari, 0 = Selamanya)
ALTER TABLE receipt_settings ADD COLUMN IF NOT EXISTS audit_log_retention_days INT NOT NULL DEFAULT 30;

-- 4. Pendaftaran Permissions Baru untuk Manajemen & Pembersihan Audit Log
INSERT INTO permissions (code, description) VALUES
('audit:manage', 'Mengatur retensi log audit dan konfigurasi pelacakan aktivitas'),
('audit:cleanup', 'Menghapus atau membersihkan riwayat log audit sistem secara manual')
ON CONFLICT (code) DO NOTHING;

-- Berikan akses ke SUPERADMIN
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p 
WHERE r.name = 'SUPERADMIN' AND p.code IN ('audit:manage', 'audit:cleanup')
ON CONFLICT DO NOTHING;
