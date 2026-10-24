-- ============================================================================
-- MIGRATION 056: Initial Setup Wizard Status & Store Configuration
-- Menyimpan status penyelesaian setup awal (first-run wizard) dan riwayat konfigurasi
-- ============================================================================

ALTER TABLE receipt_settings 
ADD COLUMN IF NOT EXISTS is_initial_setup_completed BOOLEAN NOT NULL DEFAULT FALSE;

ALTER TABLE receipt_settings 
ADD COLUMN IF NOT EXISTS initial_setup_completed_at TIMESTAMPTZ;

-- Jika database sudah memiliki transaksi penjualan (sudah beroperasi aktif), tandai setup awal sudah selesai
UPDATE receipt_settings
SET is_initial_setup_completed = TRUE, 
    initial_setup_completed_at = CURRENT_TIMESTAMP
WHERE EXISTS (SELECT 1 FROM sales LIMIT 1);
