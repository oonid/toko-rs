# Database Test Infrastructure

## Overview

All 125 integration, unit, and E2E tests run against PostgreSQL 16 via Docker Compose. Tests are no longer in-memory SQLite — they exercise real database behavior including type constraints, partial unique indexes, and `RETURNING` clauses.

## Prerequisites

```bash
docker compose up -d          # Start PostgreSQL 16 container
docker compose exec postgres pg_isready -U postgres  # Verify ready
```

The `docker-compose.yml` provisions PostgreSQL 16 on port 5432 with:
- User: `postgres`
- Password: `postgres`
- Production DB: `toko`
- Test DB: `toko_test` (created manually or by test runner)

## Test Databases

| Database | Purpose | Created by |
|---|---|---|
| `toko` | Production / manual testing | `docker-compose.yml` |
| `toko_test` | Integration tests | `scripts/init-dbs.sh` (auto on first `docker compose up`) |
| `toko_e2e` | E2E tests (live HTTP) | `scripts/init-dbs.sh` (auto on first `docker compose up`) |

Create `toko_test` and `toko_e2e` databases (automatic with `docker compose up -d`):

```bash
docker compose up -d
# databases created automatically via scripts/init-dbs.sh
```

## Running Tests

```bash
# All tests against PostgreSQL (serial execution required for DB isolation)
DATABASE_URL=postgres://postgres:postgres@localhost:5432/toko_test \
  cargo test -- --test-threads=1

# Specific test binary
DATABASE_URL=postgres://postgres:postgres@localhost:5432/toko_test \
  cargo test --test product_test -- --test-threads=1

# Lib tests only (unit tests)
DATABASE_URL=postgres://postgres:postgres@localhost:5432/toko_test \
  cargo test --lib -- --test-threads=1
```

**Important**: Tests must run with `--test-threads=1` because they share the `toko_test` database. Parallel execution causes migration conflicts and data races.

## Test Isolation

`tests/common/mod.rs` provides `setup_test_app()` which:
1. Connects to `toko_test` via `DATABASE_URL` env var
2. Runs all 6 PG migrations (`./migrations/`)
3. Calls `clean_all_tables()` to wipe all data (preserves schema)
4. Returns `(Router, AppDb)` for test use

`clean_all_tables()` deletes all rows from: `payment_records`, `order_line_items`, `orders`, `cart_line_items`, `carts`, `customer_addresses`, `customers`, `product_variant_option`, `product_option_values`, `product_options`, `product_variants`, `products`, `idempotency_keys`. Also resets `_sequences` to 0.

## Migration Details

### PG Migration Fixes (Task 15a.2)

PostgreSQL does not support `WHERE` in inline `UNIQUE` constraints. These were extracted to `CREATE UNIQUE INDEX ... WHERE` statements:

| Table | Constraint | Fix |
|---|---|---|
| `products` | `UNIQUE (handle) WHERE deleted_at IS NULL` | `CREATE UNIQUE INDEX uq_products_handle ON products (handle) WHERE deleted_at IS NULL` |
| `product_variants` | `UNIQUE (sku) WHERE deleted_at IS NULL AND sku IS NOT NULL` | `CREATE UNIQUE INDEX uq_product_variants_sku ON product_variants (sku) WHERE deleted_at IS NULL AND sku IS NOT NULL` |
| `customers` | `UNIQUE (email, has_account) WHERE deleted_at IS NULL` | `CREATE UNIQUE INDEX uq_customers_email ON customers (email, has_account) WHERE deleted_at IS NULL` |

### Type Mapping (Task 15a.3)

PG `INTEGER` is INT4 (32-bit). Rust `i64` requires INT8. All numeric columns changed to `BIGINT`:

| Migration | Columns |
|---|---|
| `001_products.sql` | `product_variants.price`, `product_variants.variant_rank` |
| `003_carts.sql` | `cart_line_items.quantity`, `cart_line_items.unit_price` |
| `004_orders.sql` | `orders.display_id`, `order_line_items.quantity`, `order_line_items.unit_price`, `_sequences.value` |
| `005_payments.sql` | `payment_records.amount` |

### SQL Dialect Changes

| Feature | SQLite (old) | PostgreSQL (new) |
|---|---|---|
| Placeholders | `?` | `$1, $2, ...` |
| Timestamps | `CURRENT_TIMESTAMP` | `now()` |
| Idempotent insert | `INSERT OR IGNORE` | `INSERT ... ON CONFLICT (id) DO NOTHING` |
| Boolean literals | `1` / `0` | `TRUE` / `FALSE` |
| Error codes | 2067 (unique), 787 (FK), 1299 (null) | 23505 (unique), 23503 (FK), 23502 (null) |
| Pool type | `SqlitePool` | `PgPool` |
| Transaction type | `Transaction<'_, sqlx::Sqlite>` | `Transaction<'_, sqlx::Postgres>` |

## Test Files

| File | Tests | What it tests |
|---|---|---|
| `tests/common/mod.rs` | — | `setup_test_app()`, `clean_all_tables()` |
| `tests/health_test.rs` | 1 | Health check endpoint |
| `tests/product_test.rs` | 23 | Admin CRUD, store browse, validation |
| `tests/cart_test.rs` | 11 | Cart lifecycle, completed guards, line items |
| `tests/order_test.rs` | 10 | Order creation, display_id, customer orders |
| `tests/customer_test.rs` | 2 | Customer CRUD |
| `tests/contract_test.rs` | 34 | Response shapes, error formats, validation |
| `tests/seed_flow_test.rs` | 2 | Seed data browse/checkout flow |
| `tests/e2e/` | 8 | E2E live HTTP tests |
| **Total** | **125** | |

## Error Mapping

`src/error.rs` provides `map_db_constraint()` for translating PG error codes:

| PG Code | Name | toko-rs Variant |
|---|---|---|
| `23505` | unique_violation | `DuplicateError` |
| `23503` | foreign_key_violation | `NotFound` |
| `23502` | not_null_violation | `InvalidData` |

Repos also check `db_err.code().as_deref() == Some("23505")` inline for context-specific messages (e.g., "Variant with SKU 'X' already exists").

## Makefile

```bash
make docker-up    # Start PG container (auto-creates toko_test + toko_e2e)
make docker-down  # Stop PG container
make test-pg      # Run integration tests against PG
make test-e2e     # Run E2E tests only
make test-e2e-pg  # Run all tests (integration + E2E)
make test         # cargo test (requires DATABASE_URL set)
```

## SQLite Status

SQLite migrations in `migrations/sqlite/` are preserved but **not currently used**. Re-enabling SQLite support requires:
1. A placeholder translator (`$N` → `?`) in `db.rs`
2. Dual `AppDb` variant handling in `create_db()`
3. Conditional SQL in repos (or the translator)

This is deferred to a future task (noted in design.md Decision 2).

## Test Coverage

```
Line Coverage:  92.12% (2233/2409 lines covered)
Region Coverage: 88.45%
```

Run with: `make cov` or `cargo llvm-cov --summary-only -- --test-threads=1`
