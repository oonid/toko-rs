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

---

## Implementation History (from audit-correction.md)

## Changes Made

### DuplicateError: 409 → 422

Medusa maps `duplicate_error` to HTTP 422 (`invalid_request_error` response), not 409 Conflict.
The `code` field override `"invalid_request_error"` was already correct.

**Before:**
```
DuplicateError → 409 Conflict, code: "invalid_request_error"
```

**After:**
```
DuplicateError → 422 Unprocessable Entity, code: "invalid_request_error"
```

**References:**
- `src/error.rs:38` — status_code() match arm
- `tests/product_test.rs:81` — duplicate handle test
- `tests/customer_test.rs:62` — duplicate email test

### UnexpectedState: 409 → 500

Medusa maps `unexpected_state` to HTTP 500 (falls through to the default case in the error
handler switch). The `code` field `"invalid_state_error"` is only used by Medusa for
QueryRunner-related conflicts (409), not for general unexpected state.

**Before:**
```
UnexpectedState → 409 Conflict, code: "invalid_state_error"
```

**After:**
```
UnexpectedState → 500 Internal Server Error, code: "invalid_state_error"
```

**References:**
- `src/error.rs:39` — status_code() match arm

## Final Error Mapping Table

| toko-rs Variant | HTTP Status | `type` | `code` | Medusa Reference |
|---|---|---|---|---|
| `NotFound` | 404 | `not_found` | `invalid_request_error` | Medusa: 404, code pass-through |
| `InvalidData` | 400 | `invalid_data` | `invalid_request_error` | Medusa: 400, code pass-through |
| `DuplicateError` | **422** | `duplicate_error` | `invalid_request_error` | Medusa: 422, code override to `invalid_request_error` |
| `Conflict` | **409** | **`conflict`** | `invalid_state_error` | Medusa: 409, `type: "conflict"` per error-handler.ts. Updated in 12b.1. |
| `Unauthorized` | 401 | `unauthorized` | `unknown_error` | Medusa: 401, code pass-through |
| `UnexpectedState` | **500** | `unexpected_state` | `invalid_state_error` | Medusa: 500 default, code pass-through |
| `DatabaseError` | 500 | `database_error` | `api_error` | Medusa: 500, message sanitized |
| `MigrationError` | 500 | **`database_error`** | `api_error` | Same category as DatabaseError, message sanitized |

## Code Field Design Decision

Medusa's `code` field is optional — many error types pass through whatever code was set at
throw time (often `undefined`, omitted from JSON). toko-rs always includes `code` in the
response body because:

1. The OAS Error schema defines `code` as a required field with enum values:
   `[invalid_state_error, invalid_request_error, api_error, unknown_error]`
2. Always-present code is simpler for API consumers to handle

The chosen values follow this logic:
- `"invalid_request_error"` — client sent a bad/contradictory request (not found, invalid data, duplicate)
- `"api_error"` — server-side infrastructure failure (database, migration)
- `"invalid_state_error"` — state conflict (unexpected state)
- `"unknown_error"` — unrecognized error category (unauthorized in P1; auth is a stub)

## Known P1 Divergences from Medusa

### Unauthorized code value

Medusa passes through the original `code` for unauthorized errors (often undefined).
toko-rs uses `"unknown_error"` since the P1 auth stub doesn't warrant a specific code.
This may change when JWT auth is implemented (P2).

## TDD Record

1. **RED**: Updated 3 unit tests in `src/error.rs` and 2 integration tests to expect 422/500
2. **GREEN**: Changed 2 match arms in `status_code()` — `DuplicateError` → `UNPROCESSABLE_ENTITY`, `UnexpectedState` → `INTERNAL_SERVER_ERROR`
3. **Verify**: 51 tests pass, clippy clean


---

## Implementation History (from audit-correction.md)

## 7a. Post-Implementation Audit — Error Handling Spec Fixes

Source: comprehensive audit comparing implementation against `specs/error-handling/spec.md` and the Medusa vendor reference at `vendor/medusa/`.

### 7a.1: `AppError::Conflict` type: `"conflict"` → `"unexpected_state"`

The spec's error-handling/spec.md defines the allowed `type` values as: `not_found`,
`invalid_data`, `duplicate_error`, `unauthorized`, `unexpected_state`, `database_error`,
`unknown_error`. The value `"conflict"` was not in this enum.

The spec's error table explicitly maps cart state conflicts (completed cart, empty cart
completion) to `type: "unexpected_state"`, `code: "invalid_state_error"`, HTTP 409.

**Before:**
```
Conflict → 409, type: "conflict", code: "invalid_state_error"
```

**After:**
```
Conflict → 409, type: "unexpected_state", code: "invalid_state_error"
```

**References:**
- `src/error.rs:58` — error_type() match arm
- `tests/cart_test.rs:439` — completed cart update error assertion
- `tests/order_test.rs:122` — empty cart completion error assertion

### 7a.2: `DatabaseError` message: raw leak → `"Internal server error"`

The spec scenario says: `"message": "Internal server error" (message sanitized, not exposing internals)`.
The previous implementation returned `e.to_string()` which included raw sqlx error text
(table/column names, connection details, SQL fragments).

**Before:**
```
DatabaseError → 500, message: "error with configuration: cfg fail"
```

**After:**
```
DatabaseError → 500, message: "Internal server error"
```

The real error is still logged via `tracing::error!()` for server-side debugging.

**References:**
- `src/error.rs:83` — IntoResponse message match arm

### 7a.3: `MigrationError` type: `"migration_error"` → `"database_error"`

The value `"migration_error"` is not in the spec's allowed `type` enum. Since migration
errors are the same category as database errors (infrastructure failures, 500 status),
the type is unified to `"database_error"`.

**Before:**
```
MigrationError → 500, type: "migration_error", code: "api_error"
```

**After:**
```
MigrationError → 500, type: "database_error", code: "api_error"
```

Message is also sanitized to `"Internal server error"` (same as DatabaseError).

**References:**
- `src/error.rs:60` — error_type() match arm
- `src/error.rs:88` — IntoResponse message match arm

### TDD Record (7a)

1. **RED**: Updated 3 unit tests in `src/error.rs` (type + message assertions), 2 integration
   tests (`cart_test.rs`, `order_test.rs`) — 5 tests fail
2. **GREEN**: Changed 2 match arms in `error_type()`, 2 message constructions in `IntoResponse`
3. **Verify**: 69 tests pass, clippy clean

---

---

## Implementation History (from audit-correction.md)

## 4e. Configuration Defaults

### 4e.1: AppConfig defaults

Added serde default functions for `HOST`, `PORT`, `RUST_LOG`:

| Field | Default | Spec requirement |
|---|---|---|
| `host` | `"0.0.0.0"` | Yes |
| `port` | `3000` | Yes |
| `rust_log` | `"toko_rs=debug,tower_http=debug"` | Yes |

`database_url` remains required (no default — must be explicitly configured).

**Test:** `test_defaults_when_not_set` — removes HOST/PORT/RUST_LOG env vars, loads config,
asserts defaults. Uses `serial_test` to prevent env var race conditions.

### 4e.2: FindParams limit default 50 → 20

Changed `default_limit()` in `src/types.rs` from 50 to 20 to match Medusa's default list
pagination. Existing tests that rely on limit use explicit values or are unaffected.

**Files changed:** `src/config.rs`, `src/types.rs`

---

## 4f. Spec Reconciliation

Updated `specs/foundation/spec.md` "Module boundary rules" requirement to document the P1
exception for cross-module SQL joins:

> **P1 exception**: A module MAY issue SQL queries that JOIN against another module's tables
> when needed for data enrichment (e.g., cart → product_variants). This matches `design.md`
> Decision 8.

Added a new scenario:
```
Scenario: Cross-module SQL joins are permitted in P1
WHEN the cart module needs to look up variant prices
THEN it issues direct SQL JOIN without importing crate::product::* types
```

This reconciles the spec with the design doc and existing implementation.

---

---

## Implementation History (from audit-correction.md)

## 7f. Default Currency USD → IDR (Config-Driven)

Completed 2026-04-08.

### Context

toko-rs is developed primarily for the Indonesian market. The default currency should reflect this by using IDR (Indonesian Rupiah) instead of USD. Rather than a simple find-and-replace of the hardcoded `"usd"` string, the change introduces a `DEFAULT_CURRENCY_CODE` configuration variable that can be overridden via environment variables — aligning with Medusa's pattern of deriving the default currency from region configuration, but simplified for P1 which has no region concept.

### Design Decision: Config-driven default

**Approach chosen**: `DEFAULT_CURRENCY_CODE` environment variable with serde default `"idr"`, threaded through `AppConfig` → `Repositories` → `CartRepository`.

**Alternatives considered**:
- Hardcode `"idr"` everywhere — simpler but requires code changes for any future market.
- Region-based lookup (Medusa pattern) — over-engineering for P1's single-currency scope.

The config-driven approach means a deployment in another market only requires changing one environment variable, no code changes.

### IDR Price Semantics

The `price` integer column is unit-agnostic — it stores a numeric value with no inherent scale. For IDR:

- **Storage**: Integer values. Fractional amounts are permitted (e.g., `1500` represents Rp1,500; `15` represents Rp15). Percentage-based calculations (tax, discounts) may produce fractional results like Rp1.5, which are stored as-is.
- **Display formatting**: Thousands use comma separator (`2500` → `Rp2,500`). Fractions use dot (`3/2` → `Rp1.5`).
- **No sub-unit convention**: Unlike USD (cents = dollars × 100), IDR has no practical sub-unit. The integer value is the face value. This is documented in `design.md` as a known P1 simplification.

### Changes Made

| # | Area | File(s) | Change |
|---|---|---|---|
| 7f.1 | Config | `src/config.rs` | Added `DEFAULT_CURRENCY_CODE` field with serde default `"idr"`. Updated `test_load_with_env_vars` and `test_defaults_when_not_set` to verify default. |
| 7f.2 | State wiring | `src/cart/repository.rs`, `src/db.rs`, `src/lib.rs`, `src/main.rs`, `tests/common/mod.rs` | `CartRepository` now holds `default_currency_code: String`, set from config. `create_db()` and `build_app_state()` accept the currency code parameter. Hardcoded `"usd"` fallback in `create_cart()` replaced with `self.default_currency_code.clone()`. |
| 7f.3 | PG migrations | `migrations/003_carts.sql`, `migrations/005_payments.sql` | `DEFAULT 'usd'` → `DEFAULT 'idr'` |
| 7f.4 | SQLite migrations | `migrations/sqlite/003_carts.sql`, `migrations/sqlite/005_payments.sql` | `DEFAULT 'usd'` → `DEFAULT 'idr'` |
| 7f.5 | Integration tests | `tests/cart_test.rs`, `tests/order_test.rs` | All `"usd"` assertions and payloads changed to `"idr"`. The `"eur"` override test (`test_store_create_cart_with_email`) left unchanged — it tests the ability to specify an explicit currency code. |
| 7f.6 | Change specs | `specs/cart-module/spec.md`, `specs/database-schema/spec.md`, `specs/foundation/spec.md` | Default currency references updated from `"usd"` to `"idr"`. `DEFAULT_CURRENCY_CODE` added to foundation config requirement. |
| 7f.7 | Docs + config | `.env.example`, `design.md` | Added `DEFAULT_CURRENCY_CODE=idr` with documentation. Added "Default currency" row to design.md divergence table. Added IDR formatting convention to risks section. |

### Files Changed (complete list)

- `src/config.rs` — new field + 2 updated tests
- `src/cart/repository.rs` — `CartRepository` struct + constructor + `create_cart()`
- `src/db.rs` — `create_db()` signature + 3 updated tests
- `src/lib.rs` — `build_app_state()` signature + 2 updated tests
- `src/main.rs` — passes `config.default_currency_code`
- `tests/common/mod.rs` — `setup_test_app()` passes `"idr"`
- `tests/cart_test.rs` — 7 occurrences `"usd"` → `"idr"`
- `tests/order_test.rs` — 5 occurrences `"usd"` → `"idr"`
- `migrations/003_carts.sql` — `DEFAULT 'idr'`
- `migrations/005_payments.sql` — `DEFAULT 'idr'`
- `migrations/sqlite/003_carts.sql` — `DEFAULT 'idr'`
- `migrations/sqlite/005_payments.sql` — `DEFAULT 'idr'`
- `.env.example` — `DEFAULT_CURRENCY_CODE=idr` section
- `openspec/changes/implementation-p1-core-mvp/design.md` — divergence table + risks
- `openspec/changes/implementation-p1-core-mvp/specs/cart-module/spec.md` — default `"idr"`
- `openspec/changes/implementation-p1-core-mvp/specs/database-schema/spec.md` — default `idr`
- `openspec/changes/implementation-p1-core-mvp/specs/foundation/spec.md` — config field
- `openspec/changes/implementation-p1-core-mvp/tasks.md` — section 7f added

### Note on migration edits

Existing migrations were edited directly (no new migration file) because toko-rs has not been deployed — there are no existing databases to preserve. This is a pre-release change.

### TDD Record (7f)

1. **RED**: N/A — all tests that referenced `"usd"` were updated to `"idr"` in the same pass as the code change
2. **GREEN**: All code, migration, and test changes applied atomically
3. **Verify**: 69 tests pass, clippy clean, zero warnings

### Updated Error Mapping Table (post 7a + 7f)

The error mapping table from section 7a is unchanged by 7f — currency is a data concern, not an error handling concern. The final mapping remains:

| toko-rs Variant | HTTP Status | `type` | `code` |
|---|---|---|---|
| `NotFound` | 404 | `not_found` | `invalid_request_error` |
| `InvalidData` | 400 | `invalid_data` | `invalid_request_error` |
| `DuplicateError` | 422 | `duplicate_error` | `invalid_request_error` |
| `Conflict` | 409 | **`conflict`** | `invalid_state_error` |
| `Unauthorized` | 401 | `unauthorized` | `unknown_error` |
| `UnexpectedState` | 500 | `unexpected_state` | `invalid_state_error` |
| `DatabaseError` | 500 | `database_error` | `api_error` |
| `MigrationError` | 500 | `database_error` | `api_error` |

---
