-- 071_tax_rate_settings.sql
-- Menambahkan konfigurasi tarif pajak toko (PPN), status pengaktifan pemungutan pajak, dan metode pajak inklusif pada tabel receipt_settings

ALTER TABLE receipt_settings
    ADD COLUMN IF NOT EXISTS enable_tax BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS default_tax_rate NUMERIC(5, 2) NOT NULL DEFAULT 11.00,
    ADD COLUMN IF NOT EXISTS tax_inclusive BOOLEAN NOT NULL DEFAULT FALSE;

-- Pastikan baris konfigurasi yang ada memiliki nilai default yang valid
UPDATE receipt_settings
SET 
    enable_tax = COALESCE(enable_tax, TRUE),
    default_tax_rate = COALESCE(default_tax_rate, 11.00),
    tax_inclusive = COALESCE(tax_inclusive, FALSE)
WHERE id IS NOT NULL;
