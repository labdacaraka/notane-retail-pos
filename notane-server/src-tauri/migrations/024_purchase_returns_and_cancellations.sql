-- Migration 024: Purchase Returns (Retur Pembelian) & Enhanced Purchase Cancellations

-- 1. Create table purchase_returns (Nota Retur Pembelian ke Supplier)
CREATE TABLE IF NOT EXISTS purchase_returns (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    return_number VARCHAR(50) UNIQUE NOT NULL,
    supplier_id UUID NOT NULL REFERENCES suppliers(id) ON DELETE RESTRICT,
    outlet_id UUID REFERENCES outlets(id) ON DELETE RESTRICT,
    warehouse_id UUID REFERENCES outlets(id) ON DELETE SET NULL,
    invoice_id UUID REFERENCES purchase_invoices(id) ON DELETE SET NULL,
    goods_receipt_id UUID REFERENCES goods_receipts(id) ON DELETE SET NULL,
    return_date DATE NOT NULL,
    status VARCHAR(30) NOT NULL DEFAULT 'COMPLETED', -- COMPLETED, CANCELLED
    refund_method VARCHAR(30) NOT NULL DEFAULT 'DEBT_REDUCTION', -- DEBT_REDUCTION, CASH_REFUND, REPLACEMENT
    reason TEXT NOT NULL, -- Rusak, Cacat, Kadaluarsa, Salah Kirim, dll
    subtotal NUMERIC(18, 4) NOT NULL DEFAULT 0.0000,
    tax_amount NUMERIC(18, 4) NOT NULL DEFAULT 0.0000,
    total_amount NUMERIC(18, 4) NOT NULL DEFAULT 0.0000,
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

-- 2. Create table purchase_return_items (Rincian Item yang Diretur)
CREATE TABLE IF NOT EXISTS purchase_return_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    purchase_return_id UUID NOT NULL REFERENCES purchase_returns(id) ON DELETE CASCADE,
    product_id UUID NOT NULL REFERENCES products(id) ON DELETE RESTRICT,
    returned_qty NUMERIC(14, 4) NOT NULL,
    unit_price NUMERIC(18, 4) NOT NULL DEFAULT 0.0000,
    subtotal NUMERIC(18, 4) NOT NULL DEFAULT 0.0000,
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by UUID REFERENCES users(id),
    updated_by UUID REFERENCES users(id)
);

-- 3. Indexes for fast query lookup
CREATE INDEX IF NOT EXISTS idx_purchase_returns_supplier_id ON purchase_returns(supplier_id);
CREATE INDEX IF NOT EXISTS idx_purchase_returns_outlet_id ON purchase_returns(outlet_id);
CREATE INDEX IF NOT EXISTS idx_purchase_returns_invoice_id ON purchase_returns(invoice_id);
CREATE INDEX IF NOT EXISTS idx_purchase_returns_date ON purchase_returns(return_date);
CREATE INDEX IF NOT EXISTS idx_purchase_return_items_ret_id ON purchase_return_items(purchase_return_id);
CREATE INDEX IF NOT EXISTS idx_purchase_return_items_prod_id ON purchase_return_items(product_id);
