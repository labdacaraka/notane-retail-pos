-- 030_customer_point_ledgers_indices.sql
-- Optimasi indeks untuk riwayat pendapatan dan penukaran poin pelanggan

CREATE INDEX IF NOT EXISTS idx_customer_point_ledgers_customer_id ON customer_point_ledgers(customer_id);
CREATE INDEX IF NOT EXISTS idx_customer_point_ledgers_created_at ON customer_point_ledgers(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_customer_point_ledgers_reference_id ON customer_point_ledgers(reference_id);
CREATE INDEX IF NOT EXISTS idx_customer_point_ledgers_tx_type ON customer_point_ledgers(transaction_type);
