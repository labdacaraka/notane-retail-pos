-- Migration 023: Add Detailed Purchase Accounts to Chart of Accounts
-- 5102: Potongan & Diskon Pembelian (Contra-COGS, normal balance CREDIT)
-- 1104: Pajak Masukan / PPN Masukan (Asset, normal balance DEBIT)
-- 5103: Biaya Angkut Pembelian / Ongkir Pengadaan (Expense / COGS addition)
-- 5104: Retur Pembelian (Contra-COGS, normal balance CREDIT)

INSERT INTO chart_of_accounts (id, account_category_id, code, name, is_header, is_active, created_at, updated_at)
SELECT gen_random_uuid(), 5, '5102', 'Potongan & Diskon Pembelian', false, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM chart_of_accounts WHERE code = '5102');

INSERT INTO chart_of_accounts (id, account_category_id, code, name, is_header, is_active, created_at, updated_at)
SELECT gen_random_uuid(), 1, '1104', 'Pajak Masukan (PPN Masukan / VAT In)', false, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM chart_of_accounts WHERE code = '1104');

INSERT INTO chart_of_accounts (id, account_category_id, code, name, is_header, is_active, created_at, updated_at)
SELECT gen_random_uuid(), 5, '5103', 'Biaya Angkut Pembelian (Freight-in)', false, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM chart_of_accounts WHERE code = '5103');

INSERT INTO chart_of_accounts (id, account_category_id, code, name, is_header, is_active, created_at, updated_at)
SELECT gen_random_uuid(), 5, '5104', 'Retur Pembelian (Purchase Returns)', false, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM chart_of_accounts WHERE code = '5104');
