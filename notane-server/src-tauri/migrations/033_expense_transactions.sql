-- ============================================================================
-- MIGRASI 033: SISTEM TRANSAKSI PENGELUARAN & BIAYA OPERASIONAL (EXPENSE TRANSACTIONS)
-- ============================================================================

-- 1. Permission Registrasi (Mandatory RBAC)
INSERT INTO permissions (code, description) VALUES
('expenses:read', 'Melihat daftar, ringkasan, dan rincian transaksi pengeluaran operasional'),
('expenses:create', 'Mencatat transaksi pengeluaran kas operasional dan petty cash'),
('expenses:delete', 'Membatalkan atau menghapus transaksi pengeluaran operasional'),
('expenses:categories', 'Mengelola master kategori akun pengeluaran operasional')
ON CONFLICT (code) DO NOTHING;

-- Petakan ke Role SUPERADMIN
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p
WHERE r.name = 'SUPERADMIN' AND p.code IN (
    'expenses:read', 'expenses:create', 'expenses:delete', 'expenses:categories'
)
ON CONFLICT DO NOTHING;

-- Petakan ke Role STORE_MANAGER
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p
WHERE r.name = 'STORE_MANAGER' AND p.code IN (
    'expenses:read', 'expenses:create', 'expenses:delete', 'expenses:categories'
)
ON CONFLICT DO NOTHING;

-- Petakan ke Role SUPERVISOR
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p
WHERE r.name = 'SUPERVISOR' AND p.code IN (
    'expenses:read', 'expenses:create'
)
ON CONFLICT DO NOTHING;

-- Petakan ke Role CASHIER (Agar kasir dapat mencatat kas keluar / petty cash langsung dari laci kasir)
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id FROM roles r, permissions p
WHERE r.name = 'CASHIER' AND p.code IN (
    'expenses:read', 'expenses:create'
)
ON CONFLICT DO NOTHING;


-- 2. Chart of Accounts Tambahan untuk Pengeluaran Operasional & Kas Kecil
-- Akun Kas Kecil (ASSET - Category 1)
INSERT INTO chart_of_accounts (id, account_category_id, code, name, is_header, is_active)
VALUES 
('a1000000-0000-0000-0000-000000000021'::uuid, 1, '1104', 'Kas Kecil Operasional Toko (Petty Cash)', false, true)
ON CONFLICT (code) DO NOTHING;

-- Akun Beban Operasional (EXPENSE - Category 5)
INSERT INTO chart_of_accounts (id, account_category_id, code, name, is_header, is_active)
VALUES
('a1000000-0000-0000-0000-000000000022'::uuid, 5, '5401', 'Beban Gaji & Upah Karyawan', false, true),
('a1000000-0000-0000-0000-000000000023'::uuid, 5, '5402', 'Beban Listrik, Air & Internet', false, true),
('a1000000-0000-0000-0000-000000000024'::uuid, 5, '5403', 'Beban Sewa Gedung & Tempat', false, true),
('a1000000-0000-0000-0000-000000000025'::uuid, 5, '5404', 'Beban Perlengkapan Toko & Plastik', false, true),
('a1000000-0000-0000-0000-000000000026'::uuid, 5, '5405', 'Beban Pemeliharaan, Servis & Perbaikan', false, true),
('a1000000-0000-0000-0000-000000000027'::uuid, 5, '5406', 'Beban Pemasaran, Iklan & Promosi', false, true),
('a1000000-0000-0000-0000-000000000028'::uuid, 5, '5407', 'Beban Kebersihan, Keamanan & Sampah', false, true),
('a1000000-0000-0000-0000-000000000029'::uuid, 5, '5408', 'Beban Konsumsi & Keperluan Kantor (ATK)', false, true),
('a1000000-0000-0000-0000-000000000030'::uuid, 5, '5499', 'Beban Operasional Lain-lain', false, true)
ON CONFLICT (code) DO NOTHING;


-- 3. Tabel Kategori Pengeluaran (Expense Categories)
CREATE TABLE IF NOT EXISTS expense_categories (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code VARCHAR(50) UNIQUE NOT NULL,
    name VARCHAR(150) NOT NULL,
    account_id UUID REFERENCES chart_of_accounts(id) ON DELETE SET NULL,
    description TEXT,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Seed Default Kategori Pengeluaran
INSERT INTO expense_categories (id, code, name, account_id, description) VALUES
('b1000000-0000-0000-0000-000000000001'::uuid, 'UTILITAS', 'Listrik, Air & Internet', (SELECT id FROM chart_of_accounts WHERE code = '5402' LIMIT 1), 'Tagihan PLN, PDAM, pulsa, dan wifi toko'),
('b1000000-0000-0000-0000-000000000002'::uuid, 'PERLENGKAPAN', 'Perlengkapan & Plastik Toko', (SELECT id FROM chart_of_accounts WHERE code = '5404' LIMIT 1), 'Plastik kresek, thermal paper kasir, lakban, kotak kemasan'),
('b1000000-0000-0000-0000-000000000003'::uuid, 'KONSUMSI', 'Konsumsi & ATK Toko', (SELECT id FROM chart_of_accounts WHERE code = '5408' LIMIT 1), 'Air galon, kopi, teh, alat tulis kantor kasir'),
('b1000000-0000-0000-0000-000000000004'::uuid, 'PEMELIHARAAN', 'Pemeliharaan & Perbaikan', (SELECT id FROM chart_of_accounts WHERE code = '5405' LIMIT 1), 'Servis AC, lampu putus, perbaikan rak, hardware POS'),
('b1000000-0000-0000-0000-000000000005'::uuid, 'KEBERSIHAN', 'Kebersihan & Keamanan', (SELECT id FROM chart_of_accounts WHERE code = '5407' LIMIT 1), 'Iuran sampah, pembersih lantai, sabun cuci, iuran lingkungan'),
('b1000000-0000-0000-0000-000000000006'::uuid, 'TRANSPORT', 'Transport & Logistik Lokal', (SELECT id FROM chart_of_accounts WHERE code = '5499' LIMIT 1), 'Bensin kurir toko, uang parkir, ongkir jemput barang'),
('b1000000-0000-0000-0000-000000000007'::uuid, 'SEWA', 'Sewa Tempat & Bangunan', (SELECT id FROM chart_of_accounts WHERE code = '5403' LIMIT 1), 'Biaya sewa ruko / lapak gerai toko'),
('b1000000-0000-0000-0000-000000000008'::uuid, 'GAJI', 'Gaji, Bonus & Uang Makan Staf', (SELECT id FROM chart_of_accounts WHERE code = '5401' LIMIT 1), 'Upah lembur, uang makan kasir, tunjangan kerja'),
('b1000000-0000-0000-0000-000000000009'::uuid, 'LAIN_LAIN', 'Biaya Operasional Lain-lain', (SELECT id FROM chart_of_accounts WHERE code = '5499' LIMIT 1), 'Pengeluaran darurat atau operasional kasir lainnya')
ON CONFLICT (code) DO NOTHING;


-- 4. Tabel Transaksi Pengeluaran (Expense Transactions)
CREATE TABLE IF NOT EXISTS expense_transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    transaction_no VARCHAR(50) UNIQUE NOT NULL,
    outlet_id UUID NOT NULL REFERENCES outlets(id) ON DELETE RESTRICT,
    category_id UUID REFERENCES expense_categories(id) ON DELETE SET NULL,
    expense_account_id UUID NOT NULL REFERENCES chart_of_accounts(id) ON DELETE RESTRICT,
    payment_account_id UUID NOT NULL REFERENCES chart_of_accounts(id) ON DELETE RESTRICT,
    amount NUMERIC(18, 4) NOT NULL CHECK (amount > 0),
    transaction_date DATE NOT NULL DEFAULT CURRENT_DATE,
    recipient_name VARCHAR(150),
    notes TEXT,
    reference_no VARCHAR(100),
    attachment_url TEXT,
    shift_id UUID REFERENCES cashier_shifts(id) ON DELETE SET NULL,
    journal_entry_id UUID REFERENCES journal_entries(id) ON DELETE SET NULL,
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    cancelled_by UUID REFERENCES users(id) ON DELETE SET NULL,
    cancelled_at TIMESTAMPTZ,
    cancellation_reason TEXT,
    status VARCHAR(20) NOT NULL DEFAULT 'COMPLETED' CHECK (status IN ('COMPLETED', 'CANCELLED')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Indeks Kinerja
CREATE INDEX IF NOT EXISTS idx_expense_transactions_outlet_date ON expense_transactions(outlet_id, transaction_date DESC);
CREATE INDEX IF NOT EXISTS idx_expense_transactions_shift ON expense_transactions(shift_id);
CREATE INDEX IF NOT EXISTS idx_expense_transactions_status ON expense_transactions(status);
CREATE INDEX IF NOT EXISTS idx_expense_transactions_category ON expense_transactions(category_id);
