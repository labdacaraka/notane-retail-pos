-- ============================================================================
-- MIGRATION 051: BACKFILL GRN BATCHES EXPIRY DATE AND DEFAULT PRODUCT COSTING GLOBAL
-- ============================================================================

-- 1. Pastikan produk default menggunakan setting global jika masih WMA dari seed data lama
UPDATE products 
SET costing_method = 'GLOBAL' 
WHERE costing_method = 'WMA';

-- 2. Sinkronkan expiry_date, batch_number, dan source_document_number pada product_stock_batches dari goods_receipt_items
UPDATE product_stock_batches psb
SET 
    expiry_date = gri.expiry_date,
    batch_number = COALESCE(NULLIF(gri.batch_number, ''), psb.batch_number),
    source_document_number = gr.grn_number
FROM goods_receipt_items gri
JOIN goods_receipts gr ON gri.goods_receipt_id = gr.id
WHERE psb.reference_type = 'PURCHASE_GRN'
  AND psb.reference_id = gr.id
  AND psb.product_id = gri.product_id
  AND (psb.expiry_date IS NULL OR psb.expiry_date != gri.expiry_date);
