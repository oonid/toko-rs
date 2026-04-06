CREATE TABLE customers (
    id TEXT PRIMARY KEY,
    first_name TEXT,
    last_name TEXT,
    email TEXT UNIQUE NOT NULL,
    phone TEXT,
    has_account BOOLEAN NOT NULL DEFAULT FALSE,
    metadata JSON,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at DATETIME
);

CREATE TABLE customer_addresses (
    id TEXT PRIMARY KEY,
    customer_id TEXT NOT NULL REFERENCES customers(id) ON DELETE CASCADE,
    address_name TEXT,
    first_name TEXT,
    last_name TEXT,
    company TEXT,
    address_1 TEXT NOT NULL,
    address_2 TEXT,
    city TEXT,
    state_province TEXT,
    postal_code TEXT,
    country_code TEXT NOT NULL,
    phone TEXT,
    metadata JSON,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at DATETIME
);
