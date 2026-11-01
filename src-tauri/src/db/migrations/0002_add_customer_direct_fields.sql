-- 0002_add_customer_direct_fields.sql
-- Add direct customer name + phone fields to invoices table.
-- Existing rows will have NULL for these columns (backward compatible).
ALTER TABLE invoices ADD COLUMN customer_name TEXT;
ALTER TABLE invoices ADD COLUMN customer_phone TEXT;
