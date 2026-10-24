-- ============================================================================
-- MIGRATION 055: Automatic Journal Entries for Initial Stock (Saldo Awal Persediaan)
-- Membukukan otomatis jurnal akuntansi saldo awal persediaan barang dagang
-- (Debit: 1201 Persediaan Barang Dagang, Kredit: 3101 Modal Awal Pemilik)
-- untuk seluruh outlet yang memiliki saldo awal persediaan namun belum dijurnal.
-- ============================================================================

DO $$
DECLARE
    v_capital_account_id UUID;
    v_journal_id UUID;
    v_journal_number VARCHAR(50);
    r RECORD;
BEGIN
    -- 1. Ambil akun Modal Awal Pemilik (3101)
    SELECT id INTO v_capital_account_id FROM chart_of_accounts WHERE code = '3101' LIMIT 1;
    IF v_capital_account_id IS NULL THEN
        SELECT id INTO v_capital_account_id FROM chart_of_accounts WHERE account_category_id = 3 ORDER BY code ASC LIMIT 1;
    END IF;

    -- 2. Periksa setiap outlet yang memiliki stok awal namun belum memiliki jurnal INITIAL_STOCK
    FOR r IN 
        WITH initial_items AS (
            -- Dari product_stock_batches reference_type = 'INITIAL_STOCK'
            SELECT 
                psb.outlet_id,
                SUM(psb.initial_qty * psb.cost_price) AS total_val
            FROM product_stock_batches psb
            WHERE psb.reference_type = 'INITIAL_STOCK'
            GROUP BY psb.outlet_id

            UNION ALL

            -- Fallback untuk produk di warehouse_stocks yang belum ada batch
            SELECT 
                ws.warehouse_id AS outlet_id,
                SUM(ws.quantity * COALESCE(pop.cost_price, p.cost_price, 0)) AS total_val
            FROM warehouse_stocks ws
            JOIN products p ON ws.product_id = p.id
            LEFT JOIN product_outlet_prices pop ON pop.outlet_id = ws.warehouse_id AND pop.product_id = ws.product_id
            WHERE ws.quantity > 0
              AND NOT EXISTS (
                  SELECT 1 FROM product_stock_batches psb 
                  WHERE psb.outlet_id = ws.warehouse_id AND psb.product_id = ws.product_id
              )
            GROUP BY ws.warehouse_id
        )
        SELECT 
            o.id AS outlet_id,
            o.code AS outlet_code,
            o.name AS outlet_name,
            COALESCE(o.inventory_account_id, (SELECT id FROM chart_of_accounts WHERE code = '1201' LIMIT 1)) AS inv_account_id,
            COALESCE(SUM(ii.total_val), 0) AS total_initial_value
        FROM outlets o
        JOIN initial_items ii ON o.id = ii.outlet_id
        WHERE NOT EXISTS (
            SELECT 1 
            FROM journal_entries je 
            WHERE je.reference_type = 'INITIAL_STOCK' 
              AND je.reference_id = o.id
        )
        AND NOT EXISTS (
            SELECT 1 
            FROM journal_entries je
            JOIN journal_lines jl ON jl.journal_id = je.id
            WHERE je.reference_type = 'INITIAL_STOCK'
              AND jl.outlet_id = o.id
        )
        GROUP BY o.id, o.code, o.name, o.inventory_account_id
    LOOP
        IF r.total_initial_value > 0 AND v_capital_account_id IS NOT NULL AND r.inv_account_id IS NOT NULL THEN
            v_journal_id := gen_random_uuid();
            v_journal_number := 'JRN-INIT-' || UPPER(TRIM(r.outlet_code));

            -- 1. Insert header jurnal
            INSERT INTO journal_entries (
                id, journal_number, journal_date, reference_type, reference_id, description,
                total_debit, total_credit, is_closing_entry, created_at, updated_at
            ) VALUES (
                v_journal_id,
                v_journal_number,
                CURRENT_DATE,
                'INITIAL_STOCK',
                r.outlet_id,
                'Saldo Awal Persediaan Barang Dagang - ' || TRIM(r.outlet_name),
                r.total_initial_value,
                r.total_initial_value,
                FALSE,
                CURRENT_TIMESTAMP,
                CURRENT_TIMESTAMP
            )
            ON CONFLICT (journal_number) DO NOTHING;

            -- 2. Line Debit: Persediaan Barang Dagang (1201) dengan dimensi cabang outlet_id
            INSERT INTO journal_lines (
                id, journal_id, account_id, debit, credit, memo, outlet_id, created_at, updated_at
            ) VALUES (
                gen_random_uuid(),
                v_journal_id,
                r.inv_account_id,
                r.total_initial_value,
                0,
                'Saldo Awal Persediaan Barang Dagang di ' || TRIM(r.outlet_name),
                r.outlet_id,
                CURRENT_TIMESTAMP,
                CURRENT_TIMESTAMP
            );

            -- 3. Line Kredit: Modal Awal Pemilik (3101) dengan dimensi cabang outlet_id
            INSERT INTO journal_lines (
                id, journal_id, account_id, debit, credit, memo, outlet_id, created_at, updated_at
            ) VALUES (
                gen_random_uuid(),
                v_journal_id,
                v_capital_account_id,
                0,
                r.total_initial_value,
                'Penyerahan Modal Berupa Persediaan Barang Dagang Awal di ' || TRIM(r.outlet_name),
                r.outlet_id,
                CURRENT_TIMESTAMP,
                CURRENT_TIMESTAMP
            );
        END IF;
    END LOOP;
END $$;
