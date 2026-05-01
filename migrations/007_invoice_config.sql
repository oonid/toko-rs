CREATE TABLE invoice_config (
    id TEXT PRIMARY KEY,
    company_name TEXT NOT NULL,
    company_address TEXT NOT NULL,
    company_phone TEXT NOT NULL,
    company_email TEXT NOT NULL,
    company_logo TEXT,
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
