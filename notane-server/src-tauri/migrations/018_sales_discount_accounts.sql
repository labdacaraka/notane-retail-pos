-- Migration 018: Add Sales Discount Accounts to Chart of Accounts
-- Account category 4 is REVENUE (Contra-revenue accounts with normal balance DEBIT)

INSERT INTO chart_of_accounts (id, account_category_id, code, name, is_header, is_active)
VALUES
  ('a1000000-0000-0000-0000-000000000013'::uuid, 4, '4102', 'Potongan Penjualan - Flash Sale', false, true),
  ('a1000000-0000-0000-0000-000000000014'::uuid, 4, '4103', 'Potongan Penjualan - Diskon Member', false, true),
  ('a1000000-0000-0000-0000-000000000015'::uuid, 4, '4104', 'Potongan Penjualan - Diskon Voucher', false, true),
  ('a1000000-0000-0000-0000-000000000016'::uuid, 4, '4105', 'Potongan Penjualan - Diskon Kasir / Item', false, true)
ON CONFLICT (code) DO UPDATE SET
  name = EXCLUDED.name,
  account_category_id = EXCLUDED.account_category_id,
  is_active = true;
