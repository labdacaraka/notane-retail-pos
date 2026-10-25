-- ============================================================================
-- MIGRASI 061: ANTREAN EKSPOR SERVER-SIDE LATAR BELAKANG (LARGE-SCALE EXPORT)
-- Menangani ekspor hingga jutaan baris data secara asinkron tanpa risiko timeout
-- ============================================================================

CREATE TABLE IF NOT EXISTS export_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE SET NULL,
    outlet_id UUID REFERENCES outlets(id) ON DELETE SET NULL,
    module VARCHAR(50) NOT NULL, -- 'sales', 'purchases_orders', 'purchases_receipts', 'purchases_invoices', 'purchases_returns', 'expenses', 'products'
    title VARCHAR(150) NOT NULL,
    filter_params JSONB NOT NULL DEFAULT '{}'::jsonb,
    filename VARCHAR(255) NOT NULL,
    file_path TEXT,
    file_size_bytes BIGINT DEFAULT 0,
    total_rows INT DEFAULT 0,
    processed_rows INT DEFAULT 0,
    status VARCHAR(20) NOT NULL DEFAULT 'QUEUED', -- 'QUEUED', 'PROCESSING', 'COMPLETED', 'FAILED'
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_export_jobs_user_status ON export_jobs(user_id, status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_export_jobs_module ON export_jobs(module, created_at DESC);
