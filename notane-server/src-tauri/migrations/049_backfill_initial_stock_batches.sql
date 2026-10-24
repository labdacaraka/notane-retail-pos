-- ============================================================================
-- MIGRATION 049: BACKFILL INITIAL STOCK BATCHES FOR EXISTING WAREHOUSE STOCKS
-- ============================================================================

-- Masukkan batch saldo awal untuk seluruh produk yang memiliki stok fisik di warehouse_stocks
-- namun belum memiliki record di product_stock_batches agar modal aksi batch dan FEFO/FIFO tracking terisi lengkap
INSERT INTO product_stock_batches (
    id,
    product_id,
    outlet_id,
    batch_number,
    initial_qty,
    remaining_qty,
    cost_price,
    reference_type,
    reference_id,
    source_document_number,
    expiry_date,
    received_at,
    created_at
)
SELECT 
    gen_random_uuid() AS id,
    ws.product_id,
    ws.warehouse_id AS outlet_id,
    'SALDO-AWAL' AS batch_number,
    ws.quantity AS initial_qty,
    ws.quantity AS remaining_qty,
    COALESCE(pop.cost_price, p.cost_price, 0) AS cost_price,
    'INITIAL_STOCK' AS reference_type,
    NULL::UUID AS reference_id,
    'SALDO-AWAL' AS source_document_number,
    NULL::DATE AS expiry_date,
    ws.created_at AS received_at,
    CURRENT_TIMESTAMP AS created_at
FROM warehouse_stocks ws
JOIN products p ON ws.product_id = p.id
LEFT JOIN outlets o ON ws.warehouse_id = o.id
LEFT JOIN product_outlet_prices pop ON pop.outlet_id = ws.warehouse_id AND pop.product_id = ws.product_id
WHERE ws.quantity > 0
  AND NOT EXISTS (
      SELECT 1 
      FROM product_stock_batches psb 
      WHERE psb.product_id = ws.product_id 
        AND psb.outlet_id = ws.warehouse_id
  );
