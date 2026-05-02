ALTER TABLE orders ADD COLUMN fulfillment_status TEXT NOT NULL DEFAULT 'not_fulfilled'
    CHECK (fulfillment_status IN ('not_fulfilled', 'fulfilled', 'shipped', 'canceled'));
ALTER TABLE orders ADD COLUMN shipped_at TIMESTAMPTZ;

ALTER TABLE payment_records ADD COLUMN captured_at TIMESTAMPTZ;

CREATE INDEX idx_orders_fulfillment_status ON orders (fulfillment_status) WHERE deleted_at IS NULL;
