-- Soft-delete duplicate phone records before adding constraint (keep earliest created_at).
-- Hard DELETE is avoided because duplicates may be referenced by orders/carts via FK.
UPDATE customers
SET deleted_at = now()
WHERE id NOT IN (
    SELECT DISTINCT ON (phone) id
    FROM customers
    WHERE phone IS NOT NULL AND deleted_at IS NULL
    ORDER BY phone, created_at ASC
)
AND phone IS NOT NULL
AND deleted_at IS NULL;

-- Add unique partial index (soft-deleted rows excluded by WHERE clause).
CREATE UNIQUE INDEX uq_customers_phone
ON customers (phone)
WHERE deleted_at IS NULL AND phone IS NOT NULL;
