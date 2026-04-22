CREATE TABLE customers (
    id TEXT PRIMARY KEY,
    first_name TEXT,
    last_name TEXT,
    email TEXT NOT NULL,
    phone TEXT,
    company_name TEXT,
    has_account BOOLEAN NOT NULL DEFAULT FALSE,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);

CREATE TABLE customer_addresses (
    id TEXT PRIMARY KEY,
    customer_id TEXT NOT NULL REFERENCES customers(id) ON DELETE CASCADE,
    address_name TEXT,
    first_name TEXT,
    last_name TEXT,
    company TEXT,
    address_1 TEXT,
    address_2 TEXT,
    city TEXT,
    province TEXT,
    postal_code TEXT,
    country_code TEXT,
    phone TEXT,
    is_default_shipping BOOLEAN NOT NULL DEFAULT FALSE,
    is_default_billing BOOLEAN NOT NULL DEFAULT FALSE,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ
);

CREATE INDEX idx_customer_addresses_customer_id ON customer_addresses (customer_id);
CREATE UNIQUE INDEX uq_customer_default_shipping ON customer_addresses (customer_id) WHERE is_default_shipping = TRUE AND deleted_at IS NULL;
CREATE UNIQUE INDEX uq_customer_default_billing ON customer_addresses (customer_id) WHERE is_default_billing = TRUE AND deleted_at IS NULL;
CREATE UNIQUE INDEX uq_customers_email ON customers (email, has_account) WHERE deleted_at IS NULL;
