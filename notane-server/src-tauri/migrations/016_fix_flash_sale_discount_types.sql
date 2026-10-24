-- Migration 016: Normalisasi discount_type pada flash_sale_items
-- Rename DISCOUNT_AMOUNT -> NOMINAL_CUT untuk konsistensi antara frontend dan backend

UPDATE flash_sale_items
SET discount_type = 'NOMINAL_CUT'
WHERE discount_type = 'DISCOUNT_AMOUNT';

-- Tambahkan constraint CHECK agar kedepannya discount_type selalu valid
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.table_constraints
        WHERE table_name = 'flash_sale_items'
          AND constraint_name = 'flash_sale_items_discount_type_check'
    ) THEN
        ALTER TABLE flash_sale_items
        ADD CONSTRAINT flash_sale_items_discount_type_check
        CHECK (discount_type IN ('PERCENTAGE', 'FIXED_PRICE', 'NOMINAL_CUT'));
    END IF;
END $$;
