-- 003_receipt_settings.sql
-- Tabel konfigurasi profil toko dan desain struk thermal kasir

CREATE TABLE IF NOT EXISTS receipt_settings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    outlet_id UUID REFERENCES outlets(id) ON DELETE CASCADE,
    store_name VARCHAR(150) NOT NULL DEFAULT 'NOTANE RETAIL STORE',
    store_tagline VARCHAR(200) DEFAULT 'Pusat Belanja Retail Modern & Terpercaya',
    store_address TEXT NOT NULL DEFAULT 'Jl. Sudirman No. 123, Jakarta Pusat',
    store_phone VARCHAR(50) NOT NULL DEFAULT '021-5551234 / 0812-3456-7890',
    store_website VARCHAR(100) DEFAULT 'www.notane.id',
    tax_id VARCHAR(50) DEFAULT 'NPWP: 01.234.567.8-901.000',
    header_note TEXT DEFAULT 'Selamat Datang di Notane Retail',
    footer_note_1 TEXT DEFAULT 'Terima Kasih Atas Kunjungan Anda',
    footer_note_2 TEXT DEFAULT 'Barang yang sudah dibeli tidak dapat ditukar/dikembalikan',
    social_media VARCHAR(100) DEFAULT 'IG: @notaneretail • CS: 0812-3456-7890',
    show_cashier_name BOOLEAN NOT NULL DEFAULT TRUE,
    show_customer_info BOOLEAN NOT NULL DEFAULT TRUE,
    show_loyalty_points BOOLEAN NOT NULL DEFAULT TRUE,
    show_tax_info BOOLEAN NOT NULL DEFAULT TRUE,
    paper_width VARCHAR(10) NOT NULL DEFAULT '58mm',
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Seed row default profil toko untuk outlet utama
INSERT INTO receipt_settings (
    id,
    outlet_id,
    store_name,
    store_tagline,
    store_address,
    store_phone,
    store_website,
    tax_id,
    header_note,
    footer_note_1,
    footer_note_2,
    social_media,
    show_cashier_name,
    show_customer_info,
    show_loyalty_points,
    show_tax_info,
    paper_width
) VALUES (
    'e0000000-0000-0000-0000-000000000001'::uuid,
    'b0000000-0000-0000-0000-000000000001'::uuid,
    'NOTANE RETAIL STORE',
    'Pusat Belanja Retail Modern & Terpercaya',
    'Jl. Sudirman No. 123, Jakarta Pusat',
    '021-5551234 / 0812-3456-7890',
    'www.notane.id',
    'NPWP: 01.234.567.8-901.000',
    'Selamat Datang di Notane Retail',
    'Terima Kasih Atas Kunjungan Anda',
    'Barang yang sudah dibeli tidak dapat ditukar/dikembalikan',
    'IG: @notaneretail • CS: 0812-3456-7890',
    true,
    true,
    true,
    true,
    '58mm'
) ON CONFLICT (id) DO NOTHING;
