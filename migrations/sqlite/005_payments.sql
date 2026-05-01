CREATE TABLE payment_records (
    id TEXT PRIMARY KEY,
    order_id TEXT NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    amount INTEGER NOT NULL CHECK (amount >= 0),
    currency_code TEXT NOT NULL DEFAULT 'idr',
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'authorized', 'captured', 'failed', 'refunded', 'canceled')),
    provider TEXT NOT NULL DEFAULT 'manual',
    metadata JSON,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at DATETIME
);

CREATE INDEX idx_payment_records_order_id ON payment_records (order_id);
CREATE INDEX idx_payment_records_status ON payment_records (status);
CREATE INDEX idx_payment_records_provider ON payment_records (provider);