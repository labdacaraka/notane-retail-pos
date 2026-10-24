-- 039_pairing_code_persistence.sql
-- Menambahkan kolom pairing_code pada tabel receipt_settings agar kode pairing toko tersimpan persisten di database

ALTER TABLE receipt_settings
    ADD COLUMN IF NOT EXISTS pairing_code VARCHAR(50) DEFAULT 'NOTANE-8899';

UPDATE receipt_settings
SET pairing_code = 'NOTANE-8899'
WHERE pairing_code IS NULL;
