CREATE TABLE products (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    handle TEXT NOT NULL,
    description TEXT,
    status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'published', 'proposed', 'rejected')),
    thumbnail TEXT,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ,
    CONSTRAINT uq_products_handle UNIQUE (handle) WHERE deleted_at IS NULL
);

CREATE TABLE product_options (
    id TEXT PRIMARY KEY,
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);

CREATE TABLE product_option_values (
    id TEXT PRIMARY KEY,
    option_id TEXT NOT NULL REFERENCES product_options(id) ON DELETE CASCADE,
    value TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);

CREATE TABLE product_variants (
    id TEXT PRIMARY KEY,
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    sku TEXT,
    price INTEGER NOT NULL DEFAULT 0,
    variant_rank INTEGER NOT NULL DEFAULT 0,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ,
    CONSTRAINT uq_product_variants_sku UNIQUE (sku) WHERE deleted_at IS NULL AND sku IS NOT NULL
);

CREATE TABLE product_variant_options (
    id TEXT PRIMARY KEY,
    variant_id TEXT NOT NULL REFERENCES product_variants(id) ON DELETE CASCADE,
    option_value_id TEXT NOT NULL REFERENCES product_option_values(id) ON DELETE CASCADE
);

CREATE INDEX idx_products_status ON products (status) WHERE deleted_at IS NULL;
CREATE INDEX idx_product_options_product_id ON product_options (product_id);
CREATE INDEX idx_product_option_values_option_id ON product_option_values (option_id);
CREATE INDEX idx_product_variants_product_id ON product_variants (product_id) WHERE deleted_at IS NULL;
