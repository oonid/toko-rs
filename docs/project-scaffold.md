# Phase 0 — Project Scaffold

## Overview

The project scaffold establishes the foundational layer that all domain modules (product, cart, order, customer, payment) build upon. It handles configuration, database connectivity, error handling, HTTP server lifecycle, and shared utilities.

## File Structure

```
toko-rs/
├── .env                          # Runtime config (gitignored)
├── .env.example                  # Config template (committed)
├── .gitignore
├── .rustfmt.toml                 # rustfmt config
├── Cargo.toml                    # Dependencies and MSRV 1.85
├── Makefile                      # Development commands
├── migrations/
│   ├── 001_products.sql          # products, product_options, product_option_values, product_variants, product_variant_options
│   ├── 002_customers.sql         # customers, customer_addresses
│   ├── 003_carts.sql             # carts, cart_line_items
│   ├── 004_orders.sql            # _sequences, orders, order_line_items
│   ├── 005_payments.sql          # payment_records
│   └── 006_idempotency.sql       # idempotency_keys
├── specs/
│   ├── store.oas.yaml            # Medusa Store OpenAPI base schema (from vendor/medusa/)
│   └── admin.oas.yaml            # Medusa Admin OpenAPI base schema (from vendor/medusa/)
├── vendor/medusa/                # Git submodule — MedusaJS implementation reference
└── src/
    ├── main.rs                   # Server entrypoint, graceful shutdown, signal handling
    ├── lib.rs                    # AppState, app_router, CORS, health check, module declarations
    ├── config.rs                 # AppConfig — env var loading via envy + dotenvy
    ├── db.rs                     # AppDb enum, DatabaseRepo enum, create_db, run_migrations, ping
    ├── error.rs                  # AppError enum — Medusa-compatible error responses
    ├── types.rs                  # Shared utilities: generate_entity_id, generate_handle, FindParams
    ├── seed.rs                   # --seed CLI flag handler
    ├── product/                  # Domain module
    ├── cart/                     # Domain module
    ├── order/                    # Domain module (stub)
    ├── customer/                 # Domain module (stub)
    └── payment/                  # Domain module (stub)
```

## Components

### Configuration (`src/config.rs`)

Loads environment variables via `envy` with `.env` file support via `dotenvy`.

| Variable | Type | Default | Description |
|---|---|---|---|
| `DATABASE_URL` | String | — | SQLite or PostgreSQL connection string |
| `HOST` | String | `0.0.0.0` | Server bind address |
| `PORT` | u16 | `3000` | Server bind port |
| `RUST_LOG` | String | — | Tracing filter (e.g., `toko_rs=debug`) |

### Database Layer (`src/db.rs`)

- **`AppDb`** — Enum wrapping `SqlitePool` or `PgPool`, selected at runtime from `DATABASE_URL` prefix.
- **`DatabaseRepo`** — Enum dispatch with per-module repository handles. Delegates method calls to the correct backend. *Will be refactored in Phase 2b to single-repo pattern with PgPool.*
- **`create_db(url)`** — Creates pool + repo based on URL scheme (`sqlite://` vs `postgres://`).
- **`run_migrations(db)`** — Executes `sqlx::migrate!("./migrations")` against the pool.
- **`ping(db)`** — Executes `SELECT 1` to verify database connectivity. Used by health check.

### Error Handling (`src/error.rs`)

Maps domain errors to Medusa-compatible JSON responses. Currently emits 2 fields (`type`, `message`); the `code` field will be added in Phase 2b.

| Variant | HTTP Status | `type` value |
|---|---|---|
| `NotFound` | 404 | `not_found` |
| `InvalidData` | 400 | `invalid_data` |
| `DuplicateError` | 409 | `duplicate_error` |
| `Unauthorized` | 401 | `unauthorized` |
| `UnexpectedState` | 409 | `unexpected_state` |
| `DatabaseError` | 500 | `database_error` |
| `MigrationError` | 500 | `migration_error` |

### Shared Utilities (`src/types.rs`)

- **`generate_entity_id(prefix)`** — Generates `{prefix}_{ULID}` for all entity IDs. All repositories should use this instead of inline `format!()`.
- **`generate_handle(title)`** — Generates URL-safe handles via the `slug` crate. Handles unicode and special characters.
- **`FindParams`** — Query parameter struct for paginated list endpoints. Defaults: `offset=0`, `limit=50`.

### HTTP Server (`src/main.rs` + `src/lib.rs`)

**Startup sequence:**
1. Load config from environment
2. Initialize tracing subscriber (`EnvFilter` + fmt layer)
3. Create DB pool and run migrations
4. Handle `--seed` flag (insert sample data and exit)
5. Build `AppState` and `app_router`
6. Bind TCP listener and serve with graceful shutdown

**Middleware stack** (applied in `app_router`):
- `TraceLayer` — HTTP request/response tracing via tower-http
- `CorsLayer::permissive()` — Allow all origins, methods, headers (development mode)

**Graceful shutdown** — Handles SIGINT (Ctrl+C) and SIGTERM. In-flight requests complete before the server stops.

### Health Check

`GET /health` — Probes database connectivity via `db::ping()`. Returns:

```json
// Healthy
{"status": "ok", "database": "connected", "version": "0.1.0"}

// Database unreachable
{"status": "degraded", "database": "disconnected", "version": "0.1.0"}
```

### Database Schema (Migrations)

6 migration files create **14 tables** total:

| Migration | Tables |
|---|---|
| `001_products.sql` | `products`, `product_options`, `product_option_values`, `product_variants`, `product_variant_options` |
| `002_customers.sql` | `customers`, `customer_addresses` |
| `003_carts.sql` | `carts`, `cart_line_items` |
| `004_orders.sql` | `_sequences`, `orders`, `order_line_items` |
| `005_payments.sql` | `payment_records` |
| `006_idempotency.sql` | `idempotency_keys` |

Current DDL targets **SQLite** (DATETIME, JSON types). PostgreSQL-primary DDL (timestamptz, jsonb) rewrite is scheduled for Phase 2b.

## Makefile

| Target | Command |
|---|---|
| `dev` | `cargo run` |
| `test` | `cargo test` |
| `check` | `cargo check` |
| `lint` | `cargo clippy -- -D warnings` |
| `fmt` | `cargo fmt` |
| `seed` | `cargo run -- --seed` |
| `clean-db` | `rm -f toko.db` |

Docker targets (`docker-up`, `docker-down`, `test-pg`) will be added in Phase 2b when `docker-compose.yml` is created.

## Dependencies

**Runtime** (15 crates): axum, sqlx, tokio, serde, serde_json, validator, ulid, slug, dotenvy, thiserror, chrono, tracing, tracing-subscriber, tower, tower-http, envy

**Dev** (4 crates): reqwest, serial_test, wiremock, assert-json-diff

## Build Quality

- **Zero compiler warnings** — Unused imports removed; `cfg(coverage)` declared in `Cargo.toml [lints.rust]` to suppress unexpected_cfgs warnings.
- **6 integration tests** passing (3 product + 3 cart).

## Medusa Reference

- **Submodule**: `vendor/medusa/` tracks the `develop` branch
- **OpenAPI specs**: `specs/store.oas.yaml` and `specs/admin.oas.yaml` are byte-identical copies from `vendor/medusa/www/utils/generated/oas-output/base/`
- **Model definitions**: `vendor/medusa/packages/modules/*/src/models/`
- **Migration reference**: `vendor/medusa/packages/modules/*/src/migrations/`
- **Validation schemas**: `vendor/medusa/packages/medusa/src/api/*/validators.ts`
