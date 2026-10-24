-- ============================================================================
-- Migration 041: Supervisor Biometric Authentication & Settings
-- 1. Tambah kolom konfigurasi biometrik pada receipt_settings
-- 2. Tambah kolom kredensial biometrik & face recognition pada users
-- 3. Daftarkan permissions RBAC baru untuk biometrik & map ke roles
-- ============================================================================

-- 1. Konfigurasi Biometrik Toko pada receipt_settings
ALTER TABLE receipt_settings
    ADD COLUMN IF NOT EXISTS supervisor_biometric_mode VARCHAR(30) NOT NULL DEFAULT 'OFF',
    ADD COLUMN IF NOT EXISTS supervisor_biometric_policy VARCHAR(30) NOT NULL DEFAULT 'OPTIONAL',
    ADD COLUMN IF NOT EXISTS face_match_threshold NUMERIC(5, 2) NOT NULL DEFAULT 75.00;

-- 2. Kredensial Biometrik Supervisor pada tabel users
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS biometric_credential_id VARCHAR(500),
    ADD COLUMN IF NOT EXISTS biometric_public_key TEXT,
    ADD COLUMN IF NOT EXISTS face_descriptor TEXT,
    ADD COLUMN IF NOT EXISTS face_photo_preview TEXT,
    ADD COLUMN IF NOT EXISTS has_biometric BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS has_face_recognition BOOLEAN NOT NULL DEFAULT FALSE;

-- Index untuk percepat lookup kredensial biometrik
CREATE INDEX IF NOT EXISTS idx_users_biometric_cred ON users(biometric_credential_id);

-- 3. Pendaftaran Permissions Baru
INSERT INTO permissions (code, description) VALUES
('settings:biometric', 'Mengatur preferensi dan kebijakan sistem biometrik supervisor toko'),
('supervisor:biometric_enroll', 'Mendaftarkan, memperbarui, atau mereset data biometrik supervisor'),
('supervisor:biometric_override', 'Melakukan otorisasi approval supervisor menggunakan sidik jari atau wajah')
ON CONFLICT (code) DO NOTHING;

-- Petakan ke SUPERADMIN & STORE_MANAGER
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p
WHERE r.name IN ('SUPERADMIN', 'STORE_MANAGER') AND p.code IN (
    'settings:biometric',
    'supervisor:biometric_enroll',
    'supervisor:biometric_override'
)
ON CONFLICT DO NOTHING;

-- Petakan ke SUPERVISOR
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p
WHERE r.name = 'SUPERVISOR' AND p.code IN (
    'supervisor:biometric_enroll',
    'supervisor:biometric_override'
)
ON CONFLICT DO NOTHING;
