## ADDED Requirements

### Requirement: PostgreSQL-primary schema design
All DDL SHALL be written for PostgreSQL as the primary database, matching Medusa's schema conventions from `vendor/medusa/packages/modules/*/src/migrations/`. Column types use `timestamptz`, `jsonb`, `BOOLEAN`, `TEXT`. A separate SQLite-compatible migration path is maintained for in-memory testing.

#### Scenario: PostgreSQL migration creates tables with correct types
- **WHEN** migrations are run against a PostgreSQL database
- **THEN** tables use `timestamptz` for timestamps, `jsonb` for JSON fields, `BOOLEAN` for boolean fields, matching Medusa's DDL conventions

### Requirement: Product module tables
The system SHALL create 4 tables plus 1 pivot table for the product module:
- `products`: id (TEXT PK, prefix `prod_`), title (TEXT NOT NULL), handle (TEXT NOT NULL, UNIQUE where deleted_at IS NULL), description, status (TEXT CHECK draft|published|proposed|rejected, default draft), thumbnail, metadata (JSONB), timestamps (created_at TIMESTAMPTZ, updated_at TIMESTAMPTZ, deleted_at TIMESTAMPTZ)
- `product_options`: id (TEXT PK, prefix `opt_`), product_id (FK → products CASCADE), title (TEXT NOT NULL), timestamps
- `product_option_values`: id (TEXT PK, prefix `optval_`), option_id (FK → product_options CASCADE), value (TEXT NOT NULL), timestamps
- `product_variants`: id (TEXT PK, prefix `variant_`), product_id (FK → products CASCADE), title (TEXT NOT NULL), sku (TEXT, UNIQUE where deleted_at IS NULL AND sku NOT NULL), price (INTEGER, cents, default 0), variant_rank (INTEGER, default 0), metadata (JSONB), timestamps
- `product_variant_options` (pivot): id (TEXT PK), variant_id (FK → product_variants CASCADE), option_value_id (FK → product_option_values CASCADE)

#### Scenario: Product tables created on startup
- **WHEN** the server starts and runs migrations
- **THEN** all 4 product tables and the pivot table exist with correct columns, constraints, and indexes

### Requirement: Customer module tables
The system SHALL create 2 tables for the customer module:
- `customers`: id (TEXT PK, prefix `cus_`), first_name, last_name, email (TEXT UNIQUE NOT NULL), phone, has_account (BOOLEAN default false), metadata (JSONB), timestamps. **Note**: Current migration uses `email TEXT UNIQUE NOT NULL` rather than the Medusa partial unique `(email, has_account) WHERE deleted_at IS NULL`. This P1 simplification prevents duplicate emails entirely. Phase 2b migration to PostgreSQL will align with Medusa's partial unique index pattern.
- `customer_addresses`: id (TEXT PK, prefix `cuaddr_`), customer_id (FK → customers CASCADE), address_name, first_name, last_name, company, address_1 (TEXT NOT NULL), address_2, city, state_province, postal_code, country_code (TEXT NOT NULL), phone, metadata (JSONB), timestamps. **Dormant in P1** (table exists, no endpoint writes to it). **Note**: `is_default_shipping` and `is_default_billing` columns are intentionally deferred to P2 when address management endpoints activate.

#### Scenario: Customer tables created on startup
- **WHEN** the server starts and runs migrations
- **THEN** both customer tables exist with correct columns and the unique index on email+has_account

### Requirement: Cart module tables
The system SHALL create 2 tables for the cart module:
- `carts`: id (TEXT PK, prefix `cart_`), customer_id (FK → customers), email, currency_code (TEXT default idr), shipping_address (JSONB, dormant in P1), billing_address (JSONB, dormant in P1), metadata (JSONB), completed_at, timestamps
- `cart_line_items`: id (TEXT PK, prefix `cali_`), cart_id (FK → carts CASCADE), title (TEXT NOT NULL), quantity (INTEGER default 1), unit_price (INTEGER NOT NULL), variant_id, product_id, snapshot (JSONB), metadata (JSONB), timestamps

#### Scenario: Cart tables created on startup
- **WHEN** the server starts and runs migrations
- **THEN** both cart tables exist with correct columns, indexes, and dormant JSON fields

### Requirement: Order module tables
The system SHALL create 2 tables plus 1 sequence table for the order module:
- `_sequences`: name (TEXT PK), value (INTEGER NOT NULL DEFAULT 0) — application-managed auto-increment sequences. Pre-seeded with `order_display_id = 0`.
- `orders`: id (TEXT PK, prefix `order_`), display_id (INTEGER NOT NULL UNIQUE), customer_id (FK → customers), email, currency_code (TEXT NOT NULL), status (TEXT NOT NULL default pending), shipping_address (JSONB, dormant in P1), billing_address (JSONB, dormant in P1), metadata (JSONB), canceled_at, timestamps
- `order_line_items`: id (TEXT PK, prefix `oli_`), order_id (FK → orders CASCADE), title (TEXT NOT NULL), quantity (INTEGER NOT NULL), unit_price (INTEGER NOT NULL), variant_id, product_id, snapshot (JSONB), metadata (JSONB), timestamps

#### Scenario: Order tables created on startup
- **WHEN** the server starts and runs migrations
- **THEN** _sequences, orders, and order_line_items tables exist with correct columns. _sequences contains `order_display_id` row.

### Requirement: Idempotency table
The system SHALL create 1 table for idempotency key tracking:
- `idempotency_keys`: key (TEXT PK), response_id (TEXT NOT NULL), response_type (TEXT NOT NULL default 'order'), created_at. Used to prevent double-order creation on retry.

#### Scenario: Idempotency table created on startup
- **WHEN** the server starts and runs migrations
- **THEN** the idempotency_keys table exists with correct columns

### Requirement: Payment table
The system SHALL create 1 table for payments:
- `payment_records`: id (TEXT PK, prefix `pay_`), order_id (FK → orders), amount (INTEGER NOT NULL), currency_code (TEXT default idr), status (TEXT CHECK pending|authorized|captured|failed|refunded, default pending), provider (TEXT default manual), metadata (JSONB), created_at, updated_at

#### Scenario: Payment table created on startup
- **WHEN** the server starts and runs migrations
- **THEN** the payment_records table exists with indexes on order_id and status

### Requirement: All IDs use prefixed ULID format
The system SHALL generate entity IDs as `{prefix}_{ULID}` where the prefix is entity-specific (prod_, opt_, optval_, variant_, cart_, cali_, order_, oli_, cus_, cuaddr_, pay_). If an ID is provided by the caller, it SHALL be used as-is.

#### Scenario: Generated ID format
- **WHEN** a new product is created without providing an ID
- **THEN** the product ID matches the pattern `prod_[0-9a-z]{26}` (lowercase ULID, matching the `ulid` crate's default `.to_string().to_lowercase()` output used in the current implementation)

### Requirement: Soft delete pattern
All entities with a `deleted_at` column SHALL support soft deletion by setting `deleted_at` to the current timestamp. All list queries SHALL exclude soft-deleted records by default (WHERE deleted_at IS NULL). The `with_deleted` query parameter MAY override this filter.

#### Scenario: Soft-deleted entity excluded from list
- **WHEN** a product is soft-deleted and a list query is made without `with_deleted`
- **THEN** the deleted product does not appear in results
