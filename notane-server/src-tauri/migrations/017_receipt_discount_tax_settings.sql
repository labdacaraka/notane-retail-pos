-- 017_receipt_discount_tax_settings.sql
-- Menambahkan opsi pengaturan visibilitas diskon flash sale, member, pajak, dan penghematan pada struk thermal

ALTER TABLE receipt_settings
    ADD COLUMN IF NOT EXISTS show_phone BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS show_address BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS show_barcode BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS show_flash_sale_discount BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS show_member_discount BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS show_tax_summary BOOLEAN NOT NULL DEFAULT TRUE,
    ADD COLUMN IF NOT EXISTS show_savings_summary BOOLEAN NOT NULL DEFAULT TRUE;
