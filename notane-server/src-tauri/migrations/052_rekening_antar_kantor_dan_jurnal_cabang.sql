-- ============================================================================
-- MIGRATION 052: Rekening Antar-Kantor (RAK) & Jurnal Akuntansi Per Cabang
-- 1. Mendaftarkan Rekening Antar-Kantor (RAK / Inter-Branch) di Bagan Akun (COA 1901)
-- 2. Menambahkan sub-akun RAK per-outlet dan kolom rak_account_id pada tabel outlets
-- 3. Mendaftarkan hak akses (permission) accounting:fund_transfer ke sistem RBAC
-- 4. Backfill outlet_id pada seluruh jurnal historis (GRN, Invoice, Sales VOID, Shift, Opname, Retur)
-- ============================================================================

-- 1. Tambah kolom rak_account_id ke tabel outlets jika belum ada
ALTER TABLE outlets 
ADD COLUMN IF NOT EXISTS rak_account_id UUID REFERENCES chart_of_accounts(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_outlets_rak_account_id ON outlets(rak_account_id);

-- 2. Buat Akun Induk RAK (1901) dan sub-akun untuk setiap outlet yang ada
DO $$
DECLARE
    v_parent_rak_id UUID;
    v_asset_cat_id INTEGER := 1; -- 1 = ASSET
    r RECORD;
    v_new_sub_coa_id UUID;
    v_coa_code VARCHAR(30);
BEGIN
    -- Cek atau buat akun induk RAK 1901
    SELECT id INTO v_parent_rak_id FROM chart_of_accounts WHERE code = '1901';
    IF v_parent_rak_id IS NULL THEN
        INSERT INTO chart_of_accounts (
            id, account_category_id, parent_id, code, name, description,
            is_header, is_active, is_system, outlet_id, created_at, updated_at
        ) VALUES (
            gen_random_uuid(),
            v_asset_cat_id,
            NULL,
            '1901',
            'Rekening Antar-Kantor (RAK / Inter-Branch Clearing)',
            'Akun kliring dan resiprokal antar cabang untuk transfer stok, transfer dana, dan mutasi silang cabang',
            TRUE,
            TRUE,
            TRUE,
            NULL,
            CURRENT_TIMESTAMP,
            CURRENT_TIMESTAMP
        )
        RETURNING id INTO v_parent_rak_id;
    ELSE
        UPDATE chart_of_accounts 
        SET is_header = TRUE, is_system = TRUE, name = 'Rekening Antar-Kantor (RAK / Inter-Branch Clearing)' 
        WHERE id = v_parent_rak_id;
    END IF;

    -- Generate sub-akun RAK per outlet
    FOR r IN SELECT id, code, name, is_active FROM outlets LOOP
        v_coa_code := '1901-' || UPPER(TRIM(r.code));

        INSERT INTO chart_of_accounts (
            id, account_category_id, parent_id, code, name, description,
            is_header, is_active, is_system, outlet_id, created_at, updated_at
        ) VALUES (
            gen_random_uuid(),
            v_asset_cat_id,
            v_parent_rak_id,
            v_coa_code,
            'RAK - ' || TRIM(r.name),
            'Rekening Antar-Kantor untuk ' || TRIM(r.name),
            FALSE,
            r.is_active,
            TRUE,
            r.id,
            CURRENT_TIMESTAMP,
            CURRENT_TIMESTAMP
        )
        ON CONFLICT (code) DO UPDATE 
        SET outlet_id = r.id, 
            name = 'RAK - ' || TRIM(r.name), 
            is_system = TRUE,
            parent_id = v_parent_rak_id
        RETURNING id INTO v_new_sub_coa_id;

        UPDATE outlets SET rak_account_id = v_new_sub_coa_id WHERE id = r.id;
    END LOOP;
END $$;

-- 3. Daftarkan permission baru: accounting:fund_transfer
INSERT INTO permissions (code, description, created_at)
VALUES (
    'accounting:fund_transfer',
    'Melakukan transfer dana dan kas antar cabang dengan pencatatan otomatis Rekening Antar-Kantor (RAK)',
    CURRENT_TIMESTAMP
)
ON CONFLICT (code) DO NOTHING;

-- Berikan izin ke role SUPERADMIN dan STORE_MANAGER secara default
INSERT INTO role_permissions (role_id, permission_id)
SELECT r.id, p.id
FROM roles r, permissions p
WHERE r.name IN ('SUPERADMIN', 'STORE_MANAGER') 
  AND p.code = 'accounting:fund_transfer'
ON CONFLICT DO NOTHING;

-- 4. Backfill outlet_id pada seluruh jurnal historis yang masih NULL
-- 4.1 PURCHASE_GRN dari goods_receipts
UPDATE journal_entries je
SET outlet_id = gr.outlet_id
FROM goods_receipts gr
WHERE je.reference_id = gr.id 
  AND je.outlet_id IS NULL 
  AND gr.outlet_id IS NOT NULL;

-- 4.2 PURCHASE_INVOICE & PURCHASE dari purchase_invoices
UPDATE journal_entries je
SET outlet_id = pi.outlet_id
FROM purchase_invoices pi
WHERE je.reference_id = pi.id 
  AND je.outlet_id IS NULL 
  AND pi.outlet_id IS NOT NULL;

-- 4.3 VOID dan SALES dari sales
UPDATE journal_entries je
SET outlet_id = s.outlet_id
FROM sales s
WHERE je.reference_id = s.id 
  AND je.outlet_id IS NULL 
  AND s.outlet_id IS NOT NULL;

-- 4.4 SHIFT_DISCREPANCY dari cashier_shifts
UPDATE journal_entries je
SET outlet_id = cs.outlet_id
FROM cashier_shifts cs
WHERE je.reference_id = cs.id 
  AND je.outlet_id IS NULL 
  AND cs.outlet_id IS NOT NULL;

-- 4.5 INVENTORY_DISCREPANCY / OPNAME dari stock_adjustments
UPDATE journal_entries je
SET outlet_id = sa.warehouse_id
FROM stock_adjustments sa
WHERE je.reference_id = sa.id 
  AND je.outlet_id IS NULL 
  AND sa.warehouse_id IS NOT NULL;

-- 4.6 PURCHASE_RETURN dari purchase_returns
UPDATE journal_entries je
SET outlet_id = pr.warehouse_id
FROM purchase_returns pr
WHERE je.reference_id = pr.id 
  AND je.outlet_id IS NULL 
  AND pr.warehouse_id IS NOT NULL;

-- 4.7 Jurnal pengiriman transfer stok lama dari stock_transfers
UPDATE journal_entries je
SET outlet_id = st.from_outlet_id
FROM stock_transfers st
WHERE je.reference_id = st.id 
  AND je.outlet_id IS NULL 
  AND je.journal_number LIKE 'JRN-TRF-%'
  AND st.from_outlet_id IS NOT NULL;

UPDATE journal_entries je
SET outlet_id = st.to_outlet_id
FROM stock_transfers st
WHERE je.reference_id = st.id 
  AND je.outlet_id IS NULL 
  AND je.journal_number LIKE 'JRN-RCV-TRF-%'
  AND st.to_outlet_id IS NOT NULL;

UPDATE journal_entries je
SET outlet_id = st.from_outlet_id
FROM stock_transfers st
WHERE je.reference_id = st.id 
  AND je.outlet_id IS NULL 
  AND je.journal_number LIKE 'JRN-CNL-TRF-%'
  AND st.from_outlet_id IS NOT NULL;

-- 4.8 Jurnal Saldo Awal Persediaan dari reference_id (outlet_id)
UPDATE journal_entries je
SET outlet_id = je.reference_id
WHERE je.reference_type = 'INITIAL_STOCK'
  AND je.outlet_id IS NULL
  AND je.reference_id IN (SELECT id FROM outlets);
