-- Migration 019: Enhance Purchase Invoices and Purchase Orders
-- Add ordered_qty to purchase_invoice_items to distinguish ordered vs received quantities

ALTER TABLE purchase_invoice_items 
ADD COLUMN IF NOT EXISTS ordered_qty NUMERIC(14, 4) NOT NULL DEFAULT 0.0000;

-- Backfill ordered_qty from received_qty for historical records
UPDATE purchase_invoice_items 
SET ordered_qty = received_qty 
WHERE ordered_qty = 0.0000 AND received_qty > 0.0000;

-- Ensure status values on purchase_orders
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint WHERE conname = 'chk_purchase_orders_status'
    ) THEN
        ALTER TABLE purchase_orders 
        ADD CONSTRAINT chk_purchase_orders_status 
        CHECK (status IN ('PENDING', 'ORDERED', 'PARTIAL', 'COMPLETED', 'CANCELLED'));
    END IF;
EXCEPTION
    WHEN OTHERS THEN
        NULL;
END $$;
