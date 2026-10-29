-- Migration 065: Add rounding_amount column to sales table for cashier cash rounding support
ALTER TABLE sales ADD COLUMN IF NOT EXISTS rounding_amount NUMERIC(18,4) NOT NULL DEFAULT 0.0000;

