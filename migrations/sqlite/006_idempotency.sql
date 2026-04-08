-- Idempotency keys used primarily for the checkout process to prevent double-charging or dual-order creation
CREATE TABLE idempotency_keys (
  key TEXT PRIMARY KEY,
  response_id TEXT NOT NULL, -- The ID of the resource returned (e.g. order_id)
  response_type TEXT NOT NULL DEFAULT 'order',
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
