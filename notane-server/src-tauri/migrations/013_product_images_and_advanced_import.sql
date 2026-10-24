-- ============================================================================
-- MIGRATION 013: PRODUCT PHOTO & ADVANCED IMPORT SUPPORT
-- Menambahkan kolom image_url pada tabel products untuk menyimpan foto produk
-- ============================================================================

ALTER TABLE products ADD COLUMN IF NOT EXISTS image_url TEXT;

CREATE INDEX IF NOT EXISTS idx_products_image_url ON products(image_url) WHERE image_url IS NOT NULL;
