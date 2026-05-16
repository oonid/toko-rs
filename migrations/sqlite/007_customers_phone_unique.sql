-- Soft-delete duplicate phone records before adding constraint (keep earliest created_at).
-- Uses correlated subquery instead of DISTINCT ON (PostgreSQL syntax not supported in SQLite).
UPDATE customers
SET deleted_at = CURRENT_TIMESTAMP
WHERE id NOT IN (
    SELECT c2.id FROM customers c2
    WHERE c2.phone = customers.phone
      AND c2.phone IS NOT NULL
      AND c2.deleted_at IS NULL
    ORDER BY c2.created_at ASC
    LIMIT 1
)
AND phone IS NOT NULL
AND deleted_at IS NULL;

-- Add unique partial index (SQLite supports partial indexes since 3.8.9).
CREATE UNIQUE INDEX uq_customers_phone
ON customers (phone)
WHERE deleted_at IS NULL AND phone IS NOT NULL;
