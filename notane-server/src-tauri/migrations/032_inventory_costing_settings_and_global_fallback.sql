-- Migration 032: Pengaturan Global Metode Valuasi HPP & Fallback Produk
-- Memungkinkan pengaturan metode perhitungan HPP secara terpusat (global)
-- sehingga produk yang tidak dispesifikasikan metodenya otomatis mewarisi pengaturan global.

CREATE TABLE IF NOT EXISTS inventory_costing_settings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    default_costing_method VARCHAR(30) NOT NULL DEFAULT 'WMA', -- 'WMA', 'FIFO', 'REPLACEMENT_COST'
    default_auto_adjust_selling_price BOOLEAN NOT NULL DEFAULT FALSE,
    default_markup_percent NUMERIC(8, 2) DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Masukkan data awal pengaturan global jika belum ada
INSERT INTO inventory_costing_settings (default_costing_method, default_auto_adjust_selling_price, default_markup_percent)
SELECT 'WMA', FALSE, 0
WHERE NOT EXISTS (SELECT 1 FROM inventory_costing_settings);

-- Ubah default kolom costing_method pada tabel products menjadi 'GLOBAL'
ALTER TABLE products ALTER COLUMN costing_method SET DEFAULT 'GLOBAL';

-- Ubah semua produk yang sebelumnya default 'WMA' menjadi 'GLOBAL' agar langsung mewarisi setting global
UPDATE products 
SET costing_method = 'GLOBAL' 
WHERE costing_method IS NULL OR costing_method = 'WMA';
