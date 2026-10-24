-- ============================================================================
-- Migrasi 058: Relasi Pembagian Dividen ke Riwayat Tutup Buku (accounting_closings)
-- Soft close (accounting_periods) hanya berfungsi mengunci transaksi,
-- sedangkan penutupan laba/rugi ke Laba Ditahan (3201) terjadi pada Tutup Buku (accounting_closings).
-- ============================================================================

-- 1. Lepas foreign key lama ke accounting_periods
ALTER TABLE dividend_distributions 
    DROP CONSTRAINT IF EXISTS dividend_distributions_period_id_fkey;

-- 2. Rename period_id menjadi closing_id jika belum di-rename
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'dividend_distributions' AND column_name = 'period_id'
    ) THEN
        ALTER TABLE dividend_distributions RENAME COLUMN period_id TO closing_id;
    END IF;
END $$;

-- 3. Tambahkan constraint foreign key ke accounting_closings
ALTER TABLE dividend_distributions 
    DROP CONSTRAINT IF EXISTS dividend_distributions_closing_id_fkey;

ALTER TABLE dividend_distributions 
    ADD CONSTRAINT dividend_distributions_closing_id_fkey 
    FOREIGN KEY (closing_id) REFERENCES accounting_closings(id) ON DELETE RESTRICT;

-- 4. Index untuk performa query pencarian dividen per dokumen tutup buku
CREATE INDEX IF NOT EXISTS idx_dividend_distributions_closing_id ON dividend_distributions(closing_id);
