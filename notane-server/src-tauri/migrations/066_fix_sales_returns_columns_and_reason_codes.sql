-- Migration 066: Fix sales returns schema alignment, refund method, and default reason codes
ALTER TABLE sales_returns ADD COLUMN IF NOT EXISTS refund_method VARCHAR(30) NOT NULL DEFAULT 'CASH';
ALTER TABLE sales_returns ADD COLUMN IF NOT EXISTS refund_amount NUMERIC(18, 4) NOT NULL DEFAULT 0.0000;

-- Kolom pelacak status retur pada transaksi penjualan
ALTER TABLE sales ADD COLUMN IF NOT EXISTS return_status VARCHAR(20) NOT NULL DEFAULT 'NONE';
ALTER TABLE sales ADD COLUMN IF NOT EXISTS total_refund_amount NUMERIC(18, 4) NOT NULL DEFAULT 0.0000;

-- Daftarkan akun CoA 4107 untuk Retur Penjualan Konsumen
INSERT INTO chart_of_accounts (id, account_category_id, code, name, is_header, is_system, is_active, created_at, updated_at)
VALUES (gen_random_uuid(), 4, '4107', 'Retur Penjualan Konsumen (Sales Returns)', false, true, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
ON CONFLICT (code) DO NOTHING;

-- Pastikan reason_code RET_DEFECT terdaftar
INSERT INTO reason_codes (id, type, code, description)
VALUES (gen_random_uuid(), 'RETURN_IN', 'RET_DEFECT', 'Barang Cacat / Rusak / Expired')
ON CONFLICT (code) DO NOTHING;

