# Phase 2b: Database Foundation

Completed 2026-04-08. All 14 tasks done (2b.1–2b.14).

## Architecture

### Repositories Struct (replaces enum dispatch)

The old `DatabaseRepo` enum with `match self { Sqlite {..} => ..., Postgres {..} => ... }` on every method call was replaced with a simple struct:

```
src/db.rs
  AppDb         — enum holding the pool (currently Sqlite only; Postgres variant added in future)
  Repositories  — struct with individual repo instances (product, cart, ...)
  create_db()   — creates pool + repos
  run_migrations() — runs migration directory matching the pool type
  ping()        — health check query
```

```rust
pub struct Repositories {
    pub product: ProductRepository,
    pub cart: CartRepository,
}
```

### AppState

```rust
pub struct AppState {
    pub db: db::AppDb,           // pool for health check
    pub repos: Arc<db::Repositories>,  // shared across handlers
}
```

Routes access repos directly: `state.repos.product.find_by_id(&id)`. No delegation layer, no enum dispatch.

### Module Boundaries

Each module owns a single repository struct:
- `src/product/repository.rs` — `ProductRepository` (SqlitePool)
- `src/cart/repository.rs` — `CartRepository` (SqlitePool)

No cross-module imports. `db.rs` is the only shared coupling point that wires repos together.

### What Was Removed

| Removed | Reason |
|---|---|
| `SqliteProductRepository` / `PostgresProductRepository` dual structs | Single `ProductRepository` per module |
| `SqliteCartRepository` / `PostgresCartRepository` dual structs | Single `CartRepository` per module |
| `DatabaseRepo` enum with 17 delegate methods | Replaced by `Repositories` struct — routes call repos directly |
| All `#[cfg(coverage)]` / `#[cfg(not(coverage))]` guards | No longer needed without dual-repo stub pattern |
| `cfg(coverage)` in `Cargo.toml [lints.rust]` | No longer referenced |

## Migrations

### Two migration sets

| Directory | Purpose | Dialect |
|---|---|---|
| `migrations/` | **PostgreSQL-primary** (production) | `TIMESTAMPTZ`, `JSONB`, `BOOLEAN`, partial unique indexes, `CHECK` constraints |
| `migrations/sqlite/` | **SQLite** (test/dev in-memory) | `DATETIME`, `TEXT` JSON, `INTEGER` booleans |

### PostgreSQL enhancements over SQLite

- `TIMESTAMPTZ DEFAULT now()` instead of `DATETIME DEFAULT CURRENT_TIMESTAMP`
- `JSONB` instead of `TEXT` (JSON) — supports indexing and operators
- `CHECK (status IN (...))` constraints on status columns
- Partial unique indexes: `UNIQUE (handle) WHERE deleted_at IS NULL` — allows reusing handles after soft-delete
- Strategic indexes on foreign keys and filtered indexes on `deleted_at IS NULL`
- `provider TEXT NOT NULL DEFAULT 'manual'` on payment_records (NOT NULL in PG)

### Tables (11 + 1 pivot + 1 sequence)

| Table | Module | Key columns |
|---|---|---|
| `products` | product | id (TEXT PK), handle (UNIQUE WHERE deleted), status (CHECK) |
| `product_options` | product | FK → products CASCADE |
| `product_option_values` | product | FK → product_options CASCADE |
| `product_variants` | product | sku (UNIQUE WHERE deleted+NOT NULL), price (INTEGER cents) |
| `product_variant_options` | product | Pivot: variant ↔ option_value |
| `customers` | customer | email (UNIQUE WHERE deleted), has_account (BOOLEAN) |
| `customer_addresses` | customer | FK → customers CASCADE |
| `carts` | cart | completed_at (nullable), FK → customers SET NULL |
| `cart_line_items` | cart | FK → carts CASCADE, variant_id FK SET NULL, snapshot JSONB |
| `orders` | order | display_id (UNIQUE), status, FK → customers SET NULL |
| `order_line_items` | order | FK → orders CASCADE, snapshot JSONB |
| `payment_records` | payment | FK → orders CASCADE, status (CHECK) |
| `_sequences` | foundation | name/value pairs for display_id auto-increment |
| `idempotency_keys` | foundation | key → response_id mapping |

## Error Response Format

Now matches 3-field OAS Error schema from `specs/store.oas.yaml`:

```json
{
  "code": "invalid_request_error",
  "type": "not_found",
  "message": "Not Found: Product with id prod_xxx was not found"
}
```

| AppError variant | `code` | `type` | HTTP status |
|---|---|---|---|
| `NotFound` | `invalid_request_error` | `not_found` | 404 |
| `InvalidData` | `invalid_request_error` | `invalid_data` | 400 |
| `DuplicateError` | `invalid_request_error` | `duplicate_error` | 409 |
| `UnexpectedState` | `invalid_state_error` | `unexpected_state` | 409 |
| `Unauthorized` | `unknown_error` | `unauthorized` | 401 |
| `DatabaseError` | `api_error` | `database_error` | 500 |
| `MigrationError` | `api_error` | `migration_error` | 500 |

## Docker Integration

`docker-compose.yml` provides PostgreSQL 16 for full compatibility testing:

```bash
make docker-up    # start PG
make docker-down  # stop PG
make test-pg      # run tests against PostgreSQL
```

## Quality Gates

| Metric | Value |
|---|---|
| Tests | 41 passing |
| Clippy | Zero warnings (`-D warnings`) |
| Line coverage | 92.42% (`cargo llvm-cov`) |
| Warnings | Zero compiler warnings |
