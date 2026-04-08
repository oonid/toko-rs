CREATE TABLE _sequences (
    name TEXT PRIMARY KEY,
    value INTEGER NOT NULL DEFAULT 0
);

INSERT INTO _sequences (name, value) VALUES ('order_display_id', 0);

CREATE TABLE orders (
    id TEXT PRIMARY KEY,
    display_id INTEGER NOT NULL UNIQUE,
    customer_id TEXT REFERENCES customers(id) ON DELETE SET NULL,
    email TEXT,
    currency_code TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'completed', 'canceled', 'requires_action', 'archived')),
    shipping_address JSONB,
    billing_address JSONB,
    metadata JSONB,
    canceled_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);

CREATE TABLE order_line_items (
    id TEXT PRIMARY KEY,
    order_id TEXT NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    quantity INTEGER NOT NULL,
    unit_price INTEGER NOT NULL,
    variant_id TEXT REFERENCES product_variants(id) ON DELETE SET NULL,
    product_id TEXT REFERENCES products(id) ON DELETE SET NULL,
    snapshot JSONB,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);

CREATE INDEX idx_orders_customer_id ON orders (customer_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_orders_display_id ON orders (display_id);
CREATE INDEX idx_order_line_items_order_id ON order_line_items (order_id);
