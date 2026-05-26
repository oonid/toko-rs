CREATE TABLE event_outbox (
    id            TEXT PRIMARY KEY,
    event_name    TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id   TEXT NOT NULL,
    payload       TEXT NOT NULL DEFAULT '{}',
    created_at    TEXT NOT NULL DEFAULT (datetime('now')),
    processed_at  TEXT
);

CREATE INDEX idx_event_outbox_unprocessed
    ON event_outbox (created_at ASC)
    WHERE processed_at IS NULL;

CREATE INDEX idx_event_outbox_resource
    ON event_outbox (resource_type, resource_id);

CREATE TABLE webhook_subscriptions (
    id         TEXT PRIMARY KEY,
    url        TEXT NOT NULL,
    events     TEXT NOT NULL DEFAULT '[]',
    secret     TEXT NOT NULL,
    enabled    INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_webhook_subscriptions_enabled
    ON webhook_subscriptions (enabled)
    WHERE enabled = 1;
