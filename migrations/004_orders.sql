CREATE TABLE _sequences (
    name TEXT PRIMARY KEY,
    value BIGINT NOT NULL DEFAULT 0
);

INSERT INTO _sequences (name, value) VALUES ('order_display_id', 0);

CREATE TABLE orders (
    id TEXT PRIMARY KEY,
    display_id BIGINT NOT NULL UNIQUE,
    cart_id TEXT UNIQUE,
    customer_id TEXT REFERENCES customers(id) ON DELETE SET NULL,
    email TEXT,
    currency_code TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('draft', 'pending', 'completed', 'canceled', 'requires_action', 'archived')),
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
    quantity BIGINT NOT NULL CHECK (quantity > 0),
    unit_price BIGINT NOT NULL CHECK (unit_price >= 0),
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
CREATE INDEX idx_orders_deleted_at ON orders (deleted_at) WHERE deleted_at IS NOT NULL;
CREATE INDEX idx_orders_currency_code ON orders (currency_code) WHERE deleted_at IS NULL;
CREATE INDEX idx_orders_cart_id ON orders (cart_id) WHERE cart_id IS NOT NULL;
CREATE INDEX idx_order_line_items_order_id ON order_line_items (order_id);
CREATE INDEX idx_order_line_items_deleted_at ON order_line_items (deleted_at) WHERE deleted_at IS NOT NULL;
CREATE INDEX idx_order_line_items_product_id ON order_line_items (product_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_order_line_items_variant_id ON order_line_items (variant_id) WHERE deleted_at IS NULL;
