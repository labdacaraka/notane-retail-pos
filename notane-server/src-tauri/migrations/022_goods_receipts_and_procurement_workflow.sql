-- Migration 022: Separate Goods Receipts from Purchase Invoices (Full Procurement Workflow)
-- Alur Pengadaan: Purchase Order (PO) -> Goods Receipt (Penerimaan Barang) -> Purchase Invoice (Faktur Pembelian) -> Payment

-- 1. Create table goods_receipts (Penerimaan Barang Fisik dari PO)
CREATE TABLE IF NOT EXISTS goods_receipts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    grn_number VARCHAR(50) UNIQUE NOT NULL,
    po_id UUID NOT NULL REFERENCES purchase_orders(id) ON DELETE RESTRICT,
    supplier_id UUID NOT NULL REFERENCES suppliers(id) ON DELETE RESTRICT,
    outlet_id UUID REFERENCES outlets(id) ON DELETE RESTRICT,
    delivery_order_number VARCHAR(100) NOT NULL, -- No. Surat Jalan / Surat Pengiriman Supplier
    receive_date DATE NOT NULL,
    status VARCHAR(30) NOT NULL DEFAULT 'UNINVOICED', -- UNINVOICED, PARTIALLY_INVOICED, INVOICED, CANCELLED
    notes TEXT,
    total_received_qty NUMERIC(14, 4) NOT NULL DEFAULT 0.0000,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

-- 2. Create table goods_receipt_items (Rincian Barang yang Diterima Aktual)
CREATE TABLE IF NOT EXISTS goods_receipt_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    goods_receipt_id UUID NOT NULL REFERENCES goods_receipts(id) ON DELETE CASCADE,
    po_item_id UUID REFERENCES purchase_order_items(id) ON DELETE SET NULL,
    product_id UUID NOT NULL REFERENCES products(id) ON DELETE RESTRICT,
    ordered_qty NUMERIC(14, 4) NOT NULL DEFAULT 0.0000,
    received_qty NUMERIC(14, 4) NOT NULL,
    unit_price NUMERIC(18, 4) NOT NULL DEFAULT 0.0000,
    invoiced_qty NUMERIC(14, 4) NOT NULL DEFAULT 0.0000,
    subtotal NUMERIC(18, 4) NOT NULL DEFAULT 0.0000,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

-- 3. Enhance purchase_invoices to reference goods_receipts
ALTER TABLE purchase_invoices 
ADD COLUMN IF NOT EXISTS goods_receipt_id UUID REFERENCES goods_receipts(id) ON DELETE SET NULL;

ALTER TABLE purchase_invoice_items 
ADD COLUMN IF NOT EXISTS goods_receipt_item_id UUID REFERENCES goods_receipt_items(id) ON DELETE SET NULL;

-- 4. Add CoA 2103 for Unbilled Goods Receipts (Hutang Penerimaan Barang Belum Ditagih / Unbilled AP)
INSERT INTO chart_of_accounts (id, account_category_id, code, name, is_header, is_active, created_at, updated_at) 
SELECT gen_random_uuid(), 2, '2103', 'Hutang Penerimaan Barang Belum Ditagih (Unbilled GRN)', false, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
WHERE NOT EXISTS (SELECT 1 FROM chart_of_accounts WHERE code = '2103');

-- 5. Backfill historical purchase_invoices to goods_receipts for seamless consistency
DO $$
DECLARE
    r RECORD;
    v_grn_id UUID;
    v_grn_num VARCHAR(50);
BEGIN
    FOR r IN (
        SELECT pi.* 
        FROM purchase_invoices pi 
        WHERE pi.goods_receipt_id IS NULL AND pi.po_id IS NOT NULL
    ) LOOP
        v_grn_id := gen_random_uuid();
        v_grn_num := 'GRN-LEGACY-' || UPPER(SUBSTRING(r.id::text, 1, 8));
        
        INSERT INTO goods_receipts (
            id, grn_number, po_id, supplier_id, outlet_id, delivery_order_number,
            receive_date, status, notes, total_received_qty, created_at, updated_at, created_by, updated_by
        ) VALUES (
            v_grn_id,
            v_grn_num,
            r.po_id,
            r.supplier_id,
            r.outlet_id,
            r.supplier_invoice_number,
            r.invoice_date,
            'INVOICED',
            'Migrasi otomatis dari data faktur historis',
            (SELECT COALESCE(SUM(received_qty), 0) FROM purchase_invoice_items WHERE invoice_id = r.id),
            r.created_at,
            r.updated_at,
            r.created_by,
            r.updated_by
        ) ON CONFLICT (grn_number) DO NOTHING;

        UPDATE purchase_invoices SET goods_receipt_id = v_grn_id WHERE id = r.id;

        INSERT INTO goods_receipt_items (
            id, goods_receipt_id, po_item_id, product_id, ordered_qty, received_qty,
            unit_price, invoiced_qty, subtotal, created_at, updated_at, created_by, updated_by
        )
        SELECT 
            gen_random_uuid(),
            v_grn_id,
            (SELECT poi.id FROM purchase_order_items poi WHERE poi.po_id = r.po_id AND poi.product_id = pii.product_id LIMIT 1),
            pii.product_id,
            pii.ordered_qty,
            pii.received_qty,
            pii.unit_price,
            pii.received_qty,
            pii.subtotal,
            pii.created_at,
            pii.updated_at,
            pii.created_by,
            pii.updated_by
        FROM purchase_invoice_items pii
        WHERE pii.invoice_id = r.id;
    END LOOP;
END $$;
