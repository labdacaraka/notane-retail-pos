-- 038_store_branding_logo.sql
-- Menambahkan kolom store_logo_url pada tabel receipt_settings untuk kustomisasi logo dan branding toko pada aplikasi POS, Customer Display, dan Back-Office

ALTER TABLE receipt_settings
    ADD COLUMN IF NOT EXISTS store_logo_url TEXT;

UPDATE receipt_settings
SET store_logo_url = NULL
WHERE store_logo_url IS NULL;
