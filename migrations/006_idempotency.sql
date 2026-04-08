CREATE TABLE idempotency_keys (
    key TEXT PRIMARY KEY,
    response_id TEXT NOT NULL,
    response_type TEXT NOT NULL DEFAULT 'order',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_idempotency_keys_response_id ON idempotency_keys (response_id);
