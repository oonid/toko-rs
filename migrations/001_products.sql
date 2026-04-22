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
    deleted_at TIMESTAMPTZ
);

CREATE TABLE product_options (
    id TEXT PRIMARY KEY,
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);

CREATE TABLE product_option_values (
    id TEXT PRIMARY KEY,
    option_id TEXT NOT NULL REFERENCES product_options(id) ON DELETE CASCADE,
    value TEXT NOT NULL,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);

CREATE TABLE product_variants (
    id TEXT PRIMARY KEY,
    product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    sku TEXT,
    price BIGINT NOT NULL DEFAULT 0,
    variant_rank BIGINT NOT NULL DEFAULT 0,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);

CREATE TABLE product_variant_option (
    id TEXT PRIMARY KEY,
    variant_id TEXT NOT NULL REFERENCES product_variants(id) ON DELETE CASCADE,
    option_value_id TEXT NOT NULL REFERENCES product_option_values(id) ON DELETE CASCADE,
    CONSTRAINT uq_product_variant_option UNIQUE (variant_id, option_value_id)
);

CREATE INDEX idx_products_status ON products (status) WHERE deleted_at IS NULL;
CREATE UNIQUE INDEX uq_products_handle ON products (handle) WHERE deleted_at IS NULL;
CREATE INDEX idx_product_options_product_id ON product_options (product_id);
CREATE UNIQUE INDEX uq_product_options_product_id_title ON product_options (product_id, title) WHERE deleted_at IS NULL;
CREATE INDEX idx_product_option_values_option_id ON product_option_values (option_id);
CREATE UNIQUE INDEX uq_product_option_values_option_id_value ON product_option_values (option_id, value) WHERE deleted_at IS NULL;
CREATE INDEX idx_product_variants_product_id ON product_variants (product_id) WHERE deleted_at IS NULL;
CREATE UNIQUE INDEX uq_product_variants_sku ON product_variants (sku) WHERE deleted_at IS NULL AND sku IS NOT NULL;
CREATE INDEX idx_product_variants_id_product_id ON product_variants (id, product_id) WHERE deleted_at IS NULL;
