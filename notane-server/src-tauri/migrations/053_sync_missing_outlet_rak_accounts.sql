-- ============================================================================
-- MIGRATION: 053_sync_missing_outlet_rak_accounts.sql
-- Description: Sinkronisasi dan pembuatan otomatis akun CoA RAK (1901)
--              untuk seluruh outlet yang belum memiliki rak_account_id.
-- ============================================================================

DO $$
DECLARE
    r RECORD;
    v_asset_cat_id INT;
    v_parent_rak_id UUID;
    v_coa_code VARCHAR(30);
    v_new_sub_coa_id UUID;
BEGIN
    SELECT id INTO v_asset_cat_id FROM account_categories WHERE name = 'ASSET' LIMIT 1;
    SELECT id INTO v_parent_rak_id FROM chart_of_accounts WHERE code = '1901' LIMIT 1;

    -- Pastikan akun induk RAK 1901 tersedia
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
    END IF;

    -- Sinkronisasi akun RAK untuk seluruh outlet yang belum memiliki rak_account_id
    FOR r IN SELECT id, code, name, is_active FROM outlets WHERE rak_account_id IS NULL LOOP
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
