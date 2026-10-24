-- ============================================================================
-- MIGRATION 014: FIX PRODUCT IMAGE STORAGE
-- Hapus index yang tidak valid (tidak bisa mengindeks base64/text besar)
-- Image disimpan sebagai file path/URL, bukan base64 di DB
-- ============================================================================

-- Hapus index yang menyebabkan error "index row too large"
DROP INDEX IF EXISTS idx_products_image_url;

-- Bersihkan data image_url lama yang berupa base64 (terlalu besar untuk index)
-- Set ke NULL jika nilainya adalah data URL base64
UPDATE products
SET image_url = NULL
WHERE image_url LIKE 'data:image/%';

-- Tidak perlu index pada image_url karena ini bukan field yang di-search/join
-- image_url hanya diakses via SELECT product by ID atau list
