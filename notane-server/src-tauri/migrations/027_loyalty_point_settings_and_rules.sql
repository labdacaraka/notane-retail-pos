-- 027_loyalty_point_settings_and_rules.sql
-- Pengaturan konversi poin loyalty, batas persentase pemakaian poin, dan akun CoA potongan poin

CREATE TABLE IF NOT EXISTS loyalty_settings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    is_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    point_value_rupiah NUMERIC(15, 2) NOT NULL DEFAULT 100.00,       -- 1 Poin = Rp 100 (nilai saat ditukarkan)
    min_spend_per_point NUMERIC(15, 2) NOT NULL DEFAULT 10000.00,    -- Belanja Rp 10.000 dapat 1 Poin
    max_point_usage_percent NUMERIC(5, 2) NOT NULL DEFAULT 50.00,    -- Maksimal pemakaian poin (% dari total tagihan)
    min_redeem_points INT NOT NULL DEFAULT 1,                        -- Minimal poin untuk dapat ditukarkan
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Seed baris default pengaturan loyalty jika belum ada
INSERT INTO loyalty_settings (
    id, is_enabled, point_value_rupiah, min_spend_per_point, max_point_usage_percent, min_redeem_points
) VALUES (
    'd1000000-0000-0000-0000-000000000001'::uuid,
    TRUE,
    100.00,
    10000.00,
    50.00,
    1
) ON CONFLICT DO NOTHING;

-- Tambahkan Akun CoA 4106: Potongan Penjualan - Penukaran Poin Reward (Contra-Revenue)
INSERT INTO chart_of_accounts (id, account_category_id, code, name, is_header, is_active)
VALUES ('a1000000-0000-0000-0000-000000000017'::uuid, 4, '4106', 'Potongan Penjualan - Penukaran Poin Reward', false, true)
ON CONFLICT (code) DO UPDATE SET
    name = EXCLUDED.name,
    account_category_id = EXCLUDED.account_category_id,
    is_active = true;
