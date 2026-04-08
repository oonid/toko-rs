CREATE TABLE payment_records (
    id TEXT PRIMARY KEY,
    order_id TEXT NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    amount INTEGER NOT NULL,
    currency_code TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending', -- pending, authorized, captured, canceled
    provider TEXT, -- e.g., 'stripe', 'manual'
    metadata JSON,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
