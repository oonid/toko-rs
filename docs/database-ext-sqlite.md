# SQLite Extension

## Overview

toko-rs supports SQLite as an optional compile-time backend via Cargo feature flag. PostgreSQL remains the default and recommended production backend. SQLite is useful for:

- **Local development** without Docker/PostgreSQL
- **Integration testing** against an in-memory database
- **Embedded/single-user deployments**

## Feature Flag

```bash
# PostgreSQL (default)
cargo build

# SQLite
cargo build --features sqlite --no-default-features
```

The binary is compiled for exactly one backend. There is no runtime backend switching.

### Cargo.toml

```toml
[features]
default = ["postgres"]
postgres = ["sqlx/postgres"]
sqlite = ["sqlx/sqlite"]
```

## Architecture

### Type Aliases (src/db.rs)

Only `src/db.rs` uses `#[cfg]` guards. All other files use generic type aliases:

| Alias | PostgreSQL (default) | SQLite |
|---|---|---|
| `DbPool` | `PgPool` | `SqlitePool` |
| `DbPoolOptions` | `PgPoolOptions` | `SqlitePoolOptions` |
| `DbDatabase` | `sqlx::Postgres` | `sqlx::Sqlite` |
| `DbTransaction<'a>` | `sqlx::Transaction<'a, Postgres>` | `sqlx::Transaction<'a, Sqlite>` |

### cfg Scope

`#[cfg]` guards touch only these locations in `src/db.rs`:

1. Type alias definitions (5 aliases)
2. `create_db()` — pool construction (SQLite uses `max_connections(1)` + `PRAGMA foreign_keys = ON`)
3. `run_migrations()` — selects `./migrations/` (PG) or `./migrations/sqlite/` (SQLite)
4. Error code constants — `unique_violation_code()`, `fk_violation_code()`, `not_null_violation_code()`

All 5 repository files, all route handlers, and all test files use `DbPool`/`DbTransaction` with zero cfg guards.

## SQL Compatibility

The same SQL queries work on both backends because:

| Feature | PostgreSQL | SQLite | Notes |
|---|---|---|---|
| Parameter placeholders | `$1, $2, ...` | `$1, $2, ...` | sqlx normalizes to `?` for SQLite |
| `RETURNING *` | Yes | Yes (3.35+) | Bundled libsqlite3-sys ships 3.39+ |
| `ON CONFLICT DO NOTHING` | Yes | Yes (3.24+) | Used in seed.rs (14 occurrences) |
| `CURRENT_TIMESTAMP` | Yes | Yes | Replaces PG-only `now()` |
| `JSONB` columns | Native | Stored as TEXT | Application-layer JSON; no queries use JSON operators |
| `BOOLEAN` | Native | INTEGER (0/1) | sqlx handles mapping |
| Partial unique indexes | Yes | Yes (3.8+) | `WHERE deleted_at IS NULL` |
| Cascading FKs | Yes | Yes (with PRAGMA) | `PRAGMA foreign_keys = ON` set on connect |
| `UPDATE ... RETURNING` | Yes | Yes (3.35+) | Used for `_sequences` display_id generation |

### SQL Changes for Portability

| Change | Files | Occurrences |
|---|---|---|
| `now()` → `CURRENT_TIMESTAMP` | product, cart, customer, order repos | 9 |

## Migration Differences

### PostgreSQL (`./migrations/`)

- `TIMESTAMPTZ` columns with `DEFAULT now()`
- `JSONB` columns
- `BOOLEAN` columns
- `BIGINT` for i64 columns
- `CHECK (status IN (...))` constraints
- `CREATE UNIQUE INDEX ... WHERE` for partial unique constraints
- Performance indexes with `WHERE deleted_at IS NULL` filters

### SQLite (`./migrations/sqlite/`)

- `DATETIME` columns with `DEFAULT CURRENT_TIMESTAMP`
- `TEXT` columns storing JSON
- `INTEGER` columns (0/1) for booleans
- `INTEGER` for i64 columns (SQLite integers are 64-bit)
- Same `CHECK` constraints (supported since 3.25+)
- Same partial unique indexes (supported since 3.8+)
- Same performance indexes

## Error Codes

Error detection uses backend-specific constraint codes, abstracted behind helper functions:

| Constraint | PG Code | SQLite Code | Helper |
|---|---|---|---|
| Unique violation | `23505` | `2067` | `db::is_unique_violation()` |
| FK violation | `23503` | `787` | `db::is_fk_violation()` |
| Not-null violation | `23502` | `1299` | `db::is_not_null_violation()` |

## Running

### Local Development (SQLite file)

```bash
cargo build --features sqlite --no-default-features
DATABASE_URL=sqlite:toko.db cargo run
```

### Testing (SQLite in-memory)

```bash
DATABASE_URL="sqlite::memory:" cargo test --features sqlite --no-default-features -- --test-threads=1
```

Or via Makefile:

```bash
make test-sqlite
```

### Full Suite (Both Backends)

```bash
make test-all
```

This runs `make test-pg` followed by `make test-sqlite`.

## Limitations

- **Single connection**: SQLite pool uses `max_connections(1)` for write safety. Not suitable for concurrent write workloads.
- **No `JSONB` operators**: JSON columns are stored as TEXT. Raw SQL queries using `->>`, `@>`, or other JSONB operators would need adaptation. toko-rs doesn't use these in P1.
- **No advisory locks**: PostgreSQL advisory locks (`pg_advisory_lock`) are not available. Used for idempotency in P2.
- **Database-level locking**: SQLite locks the entire database during writes. Under concurrent access, writes serialize automatically but may timeout.
- **Migration tooling**: `sqlx::migrate!()` is compile-time resolved. Each backend has its own migration directory. Schema changes must be applied to both.

---

## Implementation History (from audit-correction.md)

## 7b. Post-Implementation Audit — SQLite Migration Parity with PostgreSQL

Source: comprehensive audit comparing all SQLite migrations against their PG counterparts.

### Summary of Changes

| # | Migration | Column | SQLite Before | SQLite After |
|---|---|---|---|---|
| 7b.1 | 001_products | `status` | `TEXT NOT NULL DEFAULT 'draft'` | + `CHECK (status IN ('draft','published','proposed','rejected'))` |
| 7b.2 | 001_products | `sku` unique | (none) | `CREATE UNIQUE INDEX uq_product_variants_sku ON product_variants (sku) WHERE deleted_at IS NULL AND sku IS NOT NULL` |
| 7b.3 | 003_carts | `currency_code` | `TEXT NOT NULL` | `TEXT NOT NULL DEFAULT 'usd'` (later changed to `'idr'` in 7f) |
| 7b.4 | 005_payments | `provider` | `TEXT` (nullable, no default) | `TEXT NOT NULL DEFAULT 'manual'` |
| 7b.5 | 005_payments | `currency_code` | `TEXT NOT NULL` | `TEXT NOT NULL DEFAULT 'usd'` (later changed to `'idr'` in 7f) |
| 7b.6 | 005_payments | `status` | `TEXT NOT NULL DEFAULT 'pending'` | + `CHECK (status IN ('pending','authorized','captured','failed','refunded'))` |
| 7b.7 | 004_orders (PG+SQLite) | `status` | `TEXT NOT NULL DEFAULT 'pending'` | + `CHECK (status IN ('pending','completed','canceled','requires_action','archived'))` |

### Model change: PaymentRecord.provider

Updated `src/payment/models.rs`: `provider: Option<String>` → `provider: String`.
The repository always inserts `'manual'` so this is a no-op in practice but the type
now matches the `NOT NULL` constraint.

### Constraint parity verification

All constraints now match between PG and SQLite:

| Constraint | PG | SQLite |
|---|---|---|
| products.status CHECK | Yes | Yes |
| product_variants.sku UNIQUE partial | Yes | Yes |
| carts.currency_code DEFAULT 'idr' | Yes | Yes |
| orders.status CHECK | Yes | Yes |
| payment_records.status CHECK | Yes | Yes |
| payment_records.provider NOT NULL DEFAULT | Yes | Yes |
| payment_records.currency_code DEFAULT 'idr' | Yes | Yes |

### TDD Record (7b)

1. **RED**: N/A — existing tests already produce valid data; constraints add safety net only
2. **GREEN**: Applied all 7 migration fixes + 1 model type fix. No test changes needed.
3. **Verify**: 69 tests pass, clippy clean

---

## 7c. SQLite Migration Index Parity with PostgreSQL (13 indexes + 3 missing tables)

Completed 2026-04-08.

### Context

During the 7b audit (constraint parity), performance indexes were deferred to a separate task. This section adds all 13 missing SQLite performance indexes to match their PG counterparts. During implementation, 3 additional issues were discovered: the SQLite migrations for `customer_addresses` (002), `cart_line_items` (003), and `order_line_items` (004) were missing their child table definitions entirely — the tables had been defined inline in code but never added to the SQLite migration files. The indexes referencing these tables exposed the gap.

### Discovery: Missing child table definitions in SQLite migrations

The following SQLite migrations were missing child table CREATE TABLE statements that existed in their PG counterparts:

| Migration | Missing table | Impact |
|---|---|---|
| `002_customers.sql` | `customer_addresses` | Table never created in SQLite; addresses endpoints would fail |
| `003_carts.sql` | `cart_line_items` | Table never created in SQLite; cart line item operations would fail |
| `004_orders.sql` | `order_line_items` | Table never created in SQLite; order item retrieval would fail |

These tables worked in tests because sqlx's migration runner for SQLite was previously using the PG migration path or the tables were being created by test setup. After the 2b refactor consolidated to the `migrations/sqlite/` path, these tables were never added to the SQLite-specific migration files.

### Index Additions

All indexes now match between PG and SQLite:

| # | Index | Table | Columns | Partial? | Migration |
|---|---|---|---|---|---|
| 7c.1 | `idx_products_status` | `products` | `(status)` | `WHERE deleted_at IS NULL` | sqlite/001 |
| 7c.2 | `idx_product_options_product_id` | `product_options` | `(product_id)` | No | sqlite/001 |
| 7c.3 | `idx_product_option_values_option_id` | `product_option_values` | `(option_id)` | No | sqlite/001 |
| 7c.4 | `idx_product_variants_product_id` | `product_variants` | `(product_id)` | `WHERE deleted_at IS NULL` | sqlite/001 |
| 7c.5 | `idx_customer_addresses_customer_id` | `customer_addresses` | `(customer_id)` | No | sqlite/002 |
| 7c.6 | `idx_carts_customer_id` | `carts` | `(customer_id)` | `WHERE deleted_at IS NULL` | sqlite/003 |
| 7c.7 | `idx_cart_line_items_cart_id` | `cart_line_items` | `(cart_id)` | `WHERE deleted_at IS NULL` | sqlite/003 |
| 7c.8 | `idx_orders_customer_id` | `orders` | `(customer_id)` | `WHERE deleted_at IS NULL` | sqlite/004 |
| 7c.9 | `idx_orders_display_id` | `orders` | `(display_id)` | No | sqlite/004 |
| 7c.10 | `idx_order_line_items_order_id` | `order_line_items` | `(order_id)` | No | sqlite/004 |
| 7c.11 | `idx_payment_records_order_id` | `payment_records` | `(order_id)` | No | sqlite/005 |
| 7c.12 | `idx_payment_records_status` | `payment_records` | `(status)` | No | sqlite/005 |
| 7c.13 | `idx_idempotency_keys_response_id` | `idempotency_keys` | `(response_id)` | No | sqlite/006 |

### Complete Index Inventory (post 7c)

#### 001_products

| Index | Type | PG | SQLite |
|---|---|---|---|
| `uq_products_handle` | partial unique | Yes | Yes |
| `uq_product_variants_sku` | partial unique (nullable) | Yes | Yes |
| `uq_product_options_product_id_title` | partial unique | Yes | Yes |
| `uq_product_option_values_option_id_value` | partial unique | Yes | Yes |
| `idx_products_status` | partial | Yes | Yes |
| `idx_product_options_product_id` | plain | Yes | Yes |
| `idx_product_option_values_option_id` | plain | Yes | Yes |
| `idx_product_variants_product_id` | partial | Yes | Yes |

#### 002_customers

| Index | Type | PG | SQLite |
|---|---|---|---|
| `uq_customers_email` | partial unique | Yes | column-level UNIQUE (P1 design decision) |
| `idx_customer_addresses_customer_id` | plain | Yes | Yes |

#### 003_carts

| Index | Type | PG | SQLite |
|---|---|---|---|
| `idx_carts_customer_id` | partial | Yes | Yes |
| `idx_cart_line_items_cart_id` | partial | Yes | Yes |

#### 004_orders

| Index | Type | PG | SQLite |
|---|---|---|---|
| `idx_orders_customer_id` | partial | Yes | Yes |
| `idx_orders_display_id` | plain | Yes | Yes |
| `idx_order_line_items_order_id` | plain | Yes | Yes |

#### 005_payments

| Index | Type | PG | SQLite |
|---|---|---|---|
| `idx_payment_records_order_id` | plain | Yes | Yes |
| `idx_payment_records_status` | plain | Yes | Yes |

#### 006_idempotency

| Index | Type | PG | SQLite |
|---|---|---|---|
| `idx_idempotency_keys_response_id` | plain | Yes | Yes |

### Files Changed

- `migrations/sqlite/001_products.sql` — 4 indexes added
- `migrations/sqlite/002_customers.sql` — `customer_addresses` table added + 1 index
- `migrations/sqlite/003_carts.sql` — `cart_line_items` table added + 2 indexes
- `migrations/sqlite/004_orders.sql` — `order_line_items` table added + 3 indexes
- `migrations/sqlite/005_payments.sql` — 2 indexes added
- `migrations/sqlite/006_idempotency.sql` — 1 index added

No code changes. No test changes. No PG migration changes.

### TDD Record (7c)

1. **RED**: `cargo test` failed after adding indexes that referenced non-existent tables — 4 test failures exposed 3 missing table definitions (`cart_line_items`, `order_line_items`, `customer_addresses`)
2. **GREEN**: Added all 3 missing table definitions + 13 indexes across 6 migration files
3. **Verify**: 69 tests pass, clippy clean, zero warnings

---

---

## Implementation History (from audit-correction.md)

## 17. SQLite Feature Flag Support

Completed 2026-04-10.

### Context

Task 17 adds SQLite as an optional compile-time backend via Cargo feature flag. PostgreSQL remains the default and primary backend. SQLite is selected at compile time with `--features sqlite --no-default-features`. The implementation uses type aliases in `src/db.rs` to avoid code duplication — no method-level `#[cfg]` guards on repository code.

### 17a. Infrastructure setup

| # | File | Change |
|---|---|---|
| 17a.1 | `Cargo.toml` | Added `[features]` section: `default = ["postgres"]`, `postgres = ["sqlx/postgres"]`, `sqlite = ["sqlx/sqlite"]`. Removed unused `"any"` feature. |
| 17a.2 | `src/db.rs` | Type aliases: `DbPool`, `DbPoolOptions`, `DbDatabase`, `DbTransaction` via `#[cfg]` |
| 17a.3 | `src/db.rs` | `AppDb` changed from enum `AppDb::Postgres(PgPool)` to struct `AppDb { pool: DbPool }` — only one backend compiled at a time |
| 17a.4 | `src/db.rs` | `create_db()` uses `DbPoolOptions`, cfg-gated pool construction (SQLite: `max_connections(1)`, `PRAGMA foreign_keys = ON`). `run_migrations()` cfg-gated migration path. |

### 17b. SQL portability

| # | File | Change |
|---|---|---|
| 17b.1 | `src/product/repository.rs`, `src/cart/repository.rs`, `src/customer/repository.rs`, `src/order/repository.rs` | `now()` → `CURRENT_TIMESTAMP` in 9 occurrences (both backends support it) |
| 17b.2 | All 5 repo files | `PgPool` → `DbPool`, `Transaction<'_, Postgres>` → `DbTransaction<'_>` |
| 17b.3 | `src/seed.rs` | `sqlx::PgPool` → `DbPool`, `AppDb::Postgres(pool.clone())` → `AppDb { pool: pool.clone() }` |

### 17c. Error code handling

| # | File | Change |
|---|---|---|
| 17c.1 | `src/db.rs` | Added `is_unique_violation()`, `is_fk_violation()`, `is_not_null_violation()` helpers with cfg-gated code constants (PG: 23505/23503/23502, SQLite: 2067/787/1299) |
| 17c.2 | `src/error.rs`, `src/product/repository.rs`, `src/customer/repository.rs`, `src/order/repository.rs` | `map_db_constraint()` and inline checks use helper functions instead of hardcoded PG codes |

### 17d. Tests and verification

| # | Result |
|---|---|
| 17d.1 | 129 PG tests pass, clippy clean |
| 17d.2 | `cargo check --features sqlite --no-default-features` compiles |
| 17d.3 | **129 SQLite tests pass** (28 lib + 93 integration + 8 E2E) via `DATABASE_URL="sqlite::memory:"` |
| 17d.4 | Clippy clean on both feature sets |

**Fixes applied for SQLite test compatibility:**
- `src/db.rs` tests: `test_db_url()` cfg-gated default URL (PG → SQLite)
- `src/seed.rs` tests: `setup_seed_db()` cfg-gated `sqlx::migrate!()` call, `test_db_url()` cfg-gated default
- `tests/common/mod.rs`: default `DATABASE_URL` cfg-gated
- `tests/e2e/common/mod.rs`: default `E2E_DATABASE_URL` cfg-gated

### 17e. Documentation and config

| # | File | Change |
|---|---|---|
| 17e.1 | `.env` | Default changed from SQLite to PG URL |
| 17e.1 | `.env.example` | Shows both PG and SQLite options with comments |
| 17e.2 | `Makefile` | Added `test-sqlite` and `test-all` targets |
| 17e.3 | `docs/database.md` | Added SQLite feature flag section with architecture, error codes, quick start |
| 17e.4 | `design.md` | Decision 2 rewritten for compile-time backend selection. Added Decision 11. Risks section updated. |
| 17e.5 | `docs/testing.md` | Added SQLite test section |
| 17e.6 | `docs/audit-correction.md` | Added Task 17 section (this section) |
| 17e.7 | `docs/database-ext-sqlite.md` | Created — full SQLite extension documentation |

### Key discoveries during implementation

- **sqlx normalizes `$N` placeholders for SQLite automatically** — zero SQL changes needed for parameter syntax
- **SQLite 3.35+ supports `RETURNING *`** — all RETURNING usages work on both backends
- **`ON CONFLICT DO NOTHING`** (14 uses in seed.rs) works on SQLite 3.24+
- **`now()` is PG-only** — replaced with `CURRENT_TIMESTAMP` (9 occurrences across 4 files)
- **`sqlx::migrate!()` macro is compile-time** — two separate invocations behind `#[cfg]` for different migration directories
- **SQLite `:memory:` URL works for all tests** — each test gets its own isolated in-memory database

### TDD Record (17)

1. **RED**: N/A — code changes are infrastructure refactoring, not new features
2. **GREEN**: Applied all cfg-gated type aliases, SQL portability fixes, error code helpers, test infrastructure fixes
3. **Verify**: 129 tests pass on both PG and SQLite, clippy clean on both feature sets

---


