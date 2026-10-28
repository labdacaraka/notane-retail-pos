-- 064_license_resilience_and_sync_logs.sql
-- Caching lisensi terenkripsi (Dual-Storage Disk + DB), pelacakan sinkronisasi berkala, toleransi offline (grace period), dan log audit provider

CREATE TABLE IF NOT EXISTS license_encrypted_cache (
    id INT PRIMARY KEY DEFAULT 1 CHECK (id = 1),
    encrypted_vault BYTEA NOT NULL,
    hardware_id VARCHAR(255) NOT NULL,
    license_key VARCHAR(100),
    last_monotonic_time BIGINT NOT NULL DEFAULT 0,
    activated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_sync_at TIMESTAMPTZ,
    last_sync_status VARCHAR(50) NOT NULL DEFAULT 'INITIALIZED',
    consecutive_failures INT NOT NULL DEFAULT 0,
    last_failure_reason TEXT,
    last_failed_sync_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS license_sync_logs (
    id BIGSERIAL PRIMARY KEY,
    sync_type VARCHAR(50) NOT NULL, -- 'SCHEDULED_PING', 'MANUAL_SYNC', 'ACTIVATION'
    status VARCHAR(50) NOT NULL,    -- 'SUCCESS', 'UNREACHABLE', 'FAILED', 'REVOKED'
    http_status INT,
    latency_ms INT,
    provider_url TEXT NOT NULL,
    message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_license_sync_logs_created_at ON license_sync_logs (created_at DESC);

-- Registrasi Hak Akses RBAC Sesuai Zero-Trust Protocol
INSERT INTO permissions (slug, name, module, description, created_at)
VALUES 
    ('system:license_sync', 'Sinkronisasi Lisensi Provider', 'system', 'Mengizinkan sinkronisasi manual status lisensi toko dengan server provider', CURRENT_TIMESTAMP),
    ('system:license_logs', 'Lihat Riwayat Lisensi', 'system', 'Melihat log riwayat verifikasi dan sinkronisasi server lisensi', CURRENT_TIMESTAMP)
ON CONFLICT (slug) DO NOTHING;

INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r, permissions p
WHERE r.name IN ('SUPERADMIN', 'ADMIN') AND p.slug IN ('system:license_sync', 'system:license_logs')
ON CONFLICT DO NOTHING;

