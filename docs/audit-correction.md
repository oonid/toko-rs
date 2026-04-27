# Audit Corrections: Medusa Compatibility

Completed 2026-04-08. Tasks 4a–4f done.
Post-implementation audit fixes: 2026-04-08. Tasks 7a–7f done.
Data integrity + spec reconciliation: 2026-04-08. Tasks 7d.1–7d.2, 7e.1–7e.2 done.
Post-audit response shape verification: 2026-04-09. Tasks 12a.1–12a.3 done.
Post-audit error handling divergence fixes: 2026-04-09. Tasks 12b.1–12b.2 done.
Post-audit database schema gap fixes: 2026-04-09. Tasks 12c.1–12c.3 done.
Post-audit missing index additions: 2026-04-09. Tasks 12d.1–12d.3 done.
Post-audit verification pass: 2026-04-09. Tasks 12e.1–12e.4 done.
Second audit P1 business logic fixes: 2026-04-09. Tasks 14a.1–14a.7 done.
Second audit customer address schema + response stubs: 2026-04-09. Tasks 14f.1–14f.6 done.
Second audit input validation fixes: 2026-04-09. Tasks 14b.1–14b.4 done.
Second audit P1 response shape stubs: 2026-04-09. Tasks 14c.1–14c.7 done.
Second audit P1 middleware/security fixes: 2026-04-09. Tasks 14d.1–14d.2 done.
E2E integration test suite: 2026-04-10. Tasks 16a–16f done.

## Source

Audit against:
- `vendor/medusa/packages/core/framework/src/http/middlewares/error-handler.ts`
- `vendor/medusa/packages/modules/product/src/models/` (product.ts, product-option.ts, product-option-value.ts, product-variant.ts)
- `specs/store.oas.yaml` Error schema

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

## 4b. Database Schema Alignment with Medusa Models

### Pivot table rename: `product_variant_options` → `product_variant_option`

Medusa's `ProductVariant.options` relation declares `pivotTable: "product_variant_option"`
(singular). toko-rs was using `product_variant_options` (plural).

**Files changed:**
- `migrations/001_products.sql` — CREATE TABLE name
- `migrations/sqlite/001_products.sql` — CREATE TABLE name
- `src/product/repository.rs:345` — INSERT INTO statement
- `src/product/repository.rs:392` — SELECT JOIN statement

### SQLite products.handle: column UNIQUE → partial unique index

The PG migration correctly used a partial unique constraint:
```sql
CONSTRAINT uq_products_handle UNIQUE (handle) WHERE deleted_at IS NULL
```

The SQLite migration used a column-level `UNIQUE` which does not respect `deleted_at`:
```sql
handle TEXT NOT NULL UNIQUE,  -- blocks handle re-use after soft delete
```

**Fix:** Removed column-level UNIQUE, added partial unique index (supported by SQLite 3.8+):
```sql
handle TEXT NOT NULL,
-- ...
CREATE UNIQUE INDEX uq_products_handle ON products (handle) WHERE deleted_at IS NULL;
```

**Bug demonstrated:** `test_admin_create_product_reuse_handle_after_soft_delete` — create
product → soft-delete → create new product with same title (handle auto-generated) → was
returning 422, now returns 200.

### Missing unique indexes on product_options and product_option_values

Medusa defines two partial unique indexes that toko-rs was missing:

| Medusa index name | Columns | Condition |
|---|---|---|
| `IDX_option_product_id_title_unique` | `(product_id, title)` | `WHERE deleted_at IS NULL` |
| `IDX_option_value_option_id_unique` | `(option_id, value)` | `WHERE deleted_at IS NULL` |

These prevent creating two options with the same title on one product, or two option values
with the same value under one option. Added to both PG and SQLite migrations.

### Complete index inventory (001_products)

| Index | PG | SQLite | Medusa reference |
|---|---|---|---|
| `uq_products_handle` partial unique | `CONSTRAINT` | `CREATE UNIQUE INDEX` | `IDX_product_handle_unique` |
| `uq_product_variants_sku` partial unique | `CONSTRAINT` | — (not added, SKU nullable) | `IDX_product_variant_sku_unique` |
| `uq_product_options_product_id_title` partial unique | **Added** | **Added** | `IDX_option_product_id_title_unique` |
| `uq_product_option_values_option_id_value` partial unique | **Added** | **Added** | `IDX_option_value_option_id_unique` |
| `idx_products_status` partial | Yes | — (not needed for SQLite test perf) | `IDX_product_status` |
| `idx_product_options_product_id` | Yes | — | performance index |
| `idx_product_option_values_option_id` | Yes | — | performance index |
| `idx_product_variants_product_id` partial | Yes | — | `IDX_product_variant_product_id` |

### TDD Record (4b)

1. **RED**: `test_admin_create_product_reuse_handle_after_soft_delete` — creates product, soft-deletes, creates again with same title. Failed: 422 (handle unique violation on SQLite)
2. **GREEN**: Fixed all 3 migration issues in one pass — pivot rename, partial unique index, missing indexes. Also fixed 2 SQL references in repository.rs
3. **Verify**: 52 tests pass (1 new), clippy clean

---

## 4c. Product Repository Transactional Safety

`create_product` and `add_variant` were inserting product + options + option values + variants
+ variant option bindings across multiple non-transactional queries. A failure mid-way (e.g.,
duplicate SKU on variant #2) would leave partial data — a product with options but no variants.

**Fix:** Wrapped both methods in `self.pool.begin()` transactions. Refactored `insert_variant`
and `resolve_variant_options` from `&self` methods into static `fn(tx: &mut Transaction)` so
they can run within the transaction context.

**Files changed:**
- `src/product/repository.rs` — `create_product` uses `tx`, `add_variant` uses `tx`, new `insert_variant_tx` and `resolve_variant_options_tx` static methods

**Behavior:** No API-visible change — existing tests continue to pass. The fix prevents
partial data on failure paths.

---

## 4d. Cart Module Pre-existing Fixes

### 4d.1: Computed `item_total` and `total` fields

The cart spec requires `item_total = sum(quantity * unit_price)` and `total = item_total` on
every cart response. Added `item_total: i64` and `total: i64` to `CartWithItems`, computed in
`get_cart()` and initialized to 0 in `create_cart()`.

**Test:** `test_cart_item_total_computed` — creates cart (total=0), adds 3x$10 item
(total=3000).

### 4d.2: Completed-cart guard on `update_cart`

`update_cart` now checks `completed_at IS NOT NULL` before applying mutations. Returns 409
`Conflict` error.

**Test:** `test_cart_update_completed_cart_rejected` — creates cart, sets `completed_at` via
raw SQL, attempts update, asserts 409 with `type: "conflict"`.

### 4d.3: Complete-cart stub returns JSON error

Changed `store_complete_cart` from returning bare `StatusCode::NOT_IMPLEMENTED` to returning
`AppError::Conflict("Cart completion is not yet implemented")`. This produces proper JSON:
```json
{"code": "invalid_state_error", "type": "conflict", "message": "Conflict: Cart completion is not yet implemented"}
```

### New `Conflict` error variant

Added `AppError::Conflict(String)` to `src/error.rs`:
- HTTP 409 Conflict
- `type: "conflict"`
- `code: "invalid_state_error"`

This maps to Medusa's `"conflict"` error type (409 with `code: "invalid_state_error"`), used
for QueryRunner conflicts and cart state conflicts.

**Files changed:** `src/error.rs`, `src/cart/models.rs`, `src/cart/repository.rs`,
`src/cart/routes.rs`, `tests/cart_test.rs`

---

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

## TDD Summary (4a–4f)

| Phase | Tests added | Tests total | Status |
|---|---|---|---|
| 4a. Error handling | 0 new, 3 updated | 51 | Pass |
| 4b. DB schema | 1 new (handle re-use) | 52 | Pass |
| 4c. Transactional safety | 0 new (no API change) | 52 | Pass |
| 4d. Cart fixes | 2 new (totals, completed guard), 1 updated (complete stub) | 55 | Pass |
| 4e. Config defaults | 1 new (defaults test) | 56 | Pass |
| 4f. Spec reconciliation | 0 (spec-only) | 56 | Pass |

**Final: 56 tests pass, clippy clean, zero warnings.**

---

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

## 7d. Data Integrity Fixes

Completed 2026-04-08.

### 7d.1: Payment creation moved inside order transaction

The order creation flow (`create_from_cart`) was committing the order transaction first, then
creating the payment record in a separate query using `payment_repo.create()`. If payment
creation failed (e.g., constraint violation, connection drop), the order would persist without
a corresponding payment record — an orphaned order.

**Before:**
```
tx.begin() → create order → copy items → mark cart completed → tx.commit()
payment_repo.create() ← outside transaction, orphan risk on failure
```

**After:**
```
tx.begin() → create order → copy items → create payment → mark cart completed → tx.commit()
```

The payment INSERT now runs within the same transaction. If any step fails, the entire
operation rolls back — no partial state.

**Implementation:**
- Added `PaymentRepository::create_with_tx()` — a static method that accepts `&mut Transaction` instead of using `&self.pool`
- `OrderRepository::create_from_cart()` now calls `PaymentRepository::create_with_tx(&mut tx, ...)` before the cart completion UPDATE and commit
- Removed `payment_repo` parameter from `create_from_cart()` signature — the method no longer needs the `PaymentRepository` instance
- Updated `order/routes.rs` to match the simplified signature

**Files changed:**
- `src/payment/repository.rs` — added `create_with_tx` static method
- `src/order/repository.rs` — moved payment creation before commit, removed parameter
- `src/order/routes.rs` — updated `store_complete_cart` call site

**Test:** `test_order_and_payment_are_atomic` — creates cart with item, completes, verifies both `orders` and `payment_records` rows exist for the same `order_id`

### 7d.2: display_id UNIQUE constraint race handling

Under concurrent requests, `MAX(display_id) + 1` can race — two transactions compute the same
next `display_id`, and the second INSERT hits a UNIQUE violation. Previously, this surfaced
as a raw `DatabaseError` (HTTP 500 with `type: "database_error"`) — an internal error that
doesn't accurately describe the situation.

**Before:**
```
UNIQUE violation on display_id → AppError::DatabaseError → 500, type: "database_error"
```

**After:**
```
UNIQUE violation on display_id → AppError::Conflict → 409, type: "unexpected_state"
    "Order creation failed due to concurrent request. Please retry."
```

**Implementation:**
- Added `OrderRepository::map_display_id_conflict(e: sqlx::Error) -> AppError` — checks for SQLite error code `2067` (SQLITE_CONSTRAINT_UNIQUE)
- Applied via `.map_err(Self::map_display_id_conflict)` on the order INSERT query
- The client receives a 409 with a clear retry message instead of a 500

**Files changed:**
- `src/order/repository.rs` — added `map_display_id_conflict` method, applied to order INSERT

**Test:** `test_complete_cart_returns_conflict_error_format` — verifies empty cart completion returns proper conflict error with `code`, `type`, `message` fields. (The display_id race is difficult to reproduce deterministically in a test; the error mapping is verified by code review and the existing conflict error format test.)

### TDD Record (7d)

1. **RED**: Added 2 new tests — `test_order_and_payment_are_atomic` and `test_complete_cart_returns_conflict_error_format`
2. **GREEN**: Moved payment into transaction, added display_id conflict mapping, updated signatures
3. **Verify**: 71 tests pass, clippy clean, zero warnings

---

## 7e. Spec Reconciliation

Completed 2026-04-08.

### Context

After the post-implementation audit (7a–7c) and data integrity fixes (7d), the change specs in
`openspec/changes/implementation-p1-core-mvp/specs/` were out of sync with the implementation.
Sections 7a.1 and 7e.1 in `tasks.md` tracked this gap. This section documents the spec updates
made to bring all specs back in line with the code.

### 7e.1: Error handling spec update

**File:** `openspec/changes/implementation-p1-core-mvp/specs/error-handling/spec.md`

Updated the **UnexpectedState error** scenario to include the `display_id` race condition as
a valid trigger:

```diff
-- **WHEN** an invalid state transition is attempted (complete already-completed cart, mutate completed cart)
+- **WHEN** an invalid state transition is attempted (complete already-completed cart, mutate completed cart, display_id race under concurrent order creation)
```

This reflects the new `map_display_id_conflict` handler added in 7d.2 which returns
`AppError::Conflict` (mapped to `type: "unexpected_state"`, HTTP 409) when SQLite detects
error code `2067` (UNIQUE constraint violation on `display_id`).

No changes to the error mapping table itself — the table was already correct after the 7a.1
fix (`Conflict → 409, type: "unexpected_state"`).

### 7e.2: Order module spec update

**File:** `openspec/changes/implementation-p1-core-mvp/specs/order-module/spec.md`

Added two new requirements with scenarios:

#### Atomic cart-to-order conversion

Documents that order creation, line item copy, payment record creation, and cart completion
happen within a single database transaction. If any step fails, all changes roll back — no
orphaned orders or payments persist.

```
Scenario: Order and payment are created atomically
WHEN a cart is completed successfully
THEN both the order and its payment record exist in the database

Scenario: Empty cart completion returns conflict
WHEN a cart with no items is completed
THEN the system returns HTTP 409 with {"code": "invalid_state_error", "type": "unexpected_state", "message": "Cannot complete an empty cart"}
```

This requirement was implicit in the original spec but not explicitly stated. The 7d.1 fix
(payment creation moved inside the transaction) made atomicity a guaranteed property worth
documenting.

#### display_id concurrency handling

Updated the existing **Order display_id auto-increment** requirement to document the
concurrent request behavior:

```diff
-The system SHALL assign `display_id` to each new order as `MAX(display_id) + 1` across all existing orders.
+The system SHALL assign `display_id` to each new order as `MAX(display_id) + 1` across all existing orders.
+Under concurrent requests, if a UNIQUE constraint violation occurs on `display_id`, the system SHALL
+return HTTP 409 with `type: "unexpected_state"` and a message indicating the client should retry.
```

This reflects the `map_display_id_conflict` handler added in 7d.2.

### 7e.3: Audit correction documentation

**File:** `docs/audit-correction.md` (this file)

Added sections 7d and 7e documenting all post-audit corrections:

| Section | Tasks | Description |
|---|---|---|
| 7d. Data Integrity Fixes | 7d.1, 7d.2 | Payment atomicity, display_id race handling |
| 7e. Spec Reconciliation | 7e.1, 7e.2, 7e.3 | Error handling spec, order module spec, this document |

### Files Changed (7e)

| # | File | Change |
|---|---|---|
| 7e.1 | `openspec/changes/implementation-p1-core-mvp/specs/error-handling/spec.md` | Added `display_id` race to UnexpectedState scenario |
| 7e.2 | `openspec/changes/implementation-p1-core-mvp/specs/order-module/spec.md` | Added atomic conversion requirement, display_id concurrency scenario, empty cart conflict scenario |
| 7e.3 | `docs/audit-correction.md` | Added 7d and 7e sections with full before/after, implementation detail, and file references |

### TDD Record (7e)

1. **RED**: N/A — spec-only changes, no code modifications
2. **GREEN**: Updated 2 spec files + this document to match implementation state after 7d
3. **Verify**: 71 tests pass (unchanged), clippy clean

---

> **Sections 12a–12e moved to `docs/audit-p1-task12.md` → "Implementation Details".**

---

> **Sections 14a–14f moved to `docs/audit-p1-task14.md` → "Implementation Details".**

---

> **Section 18 moved to `docs/audit-p1-task18.md` → "Implementation Details".**

---

### Changes

E2E tests run against a live `axum::serve` instance using `reqwest::Client`. Each test gets a fresh server on a random port with a clean database seeded via `run_seed()`.

### Files Created

| File | Purpose |
|------|---------|
| `tests/e2e/main.rs` | Crate root with mod declarations |
| `tests/e2e/common/mod.rs` | `E2eContext` struct, `setup_e2e()`, `clean_all_tables()`, `seed()`, testcontainers helper |
| `tests/e2e/guest_checkout.rs` | 9-step guest browse → cart → checkout flow |
| `tests/e2e/customer_lifecycle.rs` | 8-step register → profile → cart → order history |
| `tests/e2e/admin_products.rs` | CRUD + variant validation |
| `tests/e2e/cart_manipulation.rs` | Update/delete/guards |
| `tests/e2e/errors_validation.rs` | Error response tests (404, 422, 400, 401) |
| `tests/e2e/response_shapes.rs` | Contract shape verification |

### Files Modified

| Task | File | Change |
|------|------|--------|
| 16a.2 | `Cargo.toml` | Added `testcontainers` and `testcontainers-modules` to dev-deps |
| 16a.3 | `docker-compose.yml` | Added `scripts/init-dbs.sh` for auto-creating `toko_test` and `toko_e2e` |
| 16a.3 | `scripts/init-dbs.sh` | Init script for creating test databases |
| 16a.4 | `Makefile` | Added `test-e2e`, `test-e2e-pg` targets; fixed `test-pg` to use `toko_test` |

### Fixes Applied

1. **Partial move error**: `setup_e2e()` used `match app_db` which moved the value. Fixed with `match &app_db` and `.clone()`.
2. **Delete response shape**: `DELETE /store/carts/{id}/line-items/{line_id}` returns `LineItemDeleteResponse { id, object, deleted, parent }` — not `{ cart: { items: [] } }`. Test updated to use `body["parent"]["items"]`.
3. **Unused pool warning**: `#[allow(dead_code)]` on `E2eContext` — the `pool` field is available for direct DB assertions.
4. **Testcontainers support**: `E2E_DATABASE_URL=testcontainers://` triggers programmatic PG container creation.

### Test Counts

| Suite | Tests |
|-------|-------|
| Unit tests (lib) | 25 |
| Integration tests | 92 |
| E2E tests | 8 |
| **Total** | **125** |

All 125 tests pass against PostgreSQL 16, `--test-threads=1`, clippy clean.

## 15-16 Verification Pass (2026-04-10)

### Verification Method

- All 6 PG migrations audited: BIGINT, CREATE UNIQUE INDEX ... WHERE, BOOLEAN, now() — all correct
- All 5 repositories audited: $N placeholders, PgPool, PG error codes — all correct
- `src/error.rs` `map_db_constraint()`: 23505/23503/23502 mapping verified
- `src/seed.rs` all INSERT statements use `ON CONFLICT (id) DO NOTHING`
- All 8 E2E tests verified against spec requirements
- Full test suite: 125 tests pass, clippy clean

### Fixes Applied During Verification

| File | Change | Reason |
|------|--------|--------|
| `src/seed.rs` | Changed `seed_customer()` from `SELECT COUNT(*)` guard to `ON CONFLICT (id) DO NOTHING` | Consistency with all other seed INSERTs; eliminates theoretical TOCTOU race |

### Test Coverage

```
Line Coverage: 92.12% (2233 lines, 176 missed)
Region Coverage: 88.45%
```

Above 90% threshold. Low-coverage files:
- `main.rs` (0%) — binary entry point, not testable
- `payment/repository.rs` (43%) — `create()` standalone and `find_by_order_id()` are unused infrastructure for P2
- `config.rs` (77%) — env var loading paths not exercised in unit tests

## Documentation Consolidation (2026-04-10)

Consolidated 5 docs into 3, eliminating duplicated content and superseded planned designs:

| Before | After | Change |
|---|---|---|
| `database-foundation.md` (533 lines) + `database-test.md` (152 lines) | `database.md` (~240 lines) | Merged schema mapping + architecture + migrations + error format + Docker. Deleted 284 lines of superseded Task 15/16 "planned" designs. |
| `test-suite.md` (157 lines) + `test-e2e.md` (113 lines) | `testing.md` (~180 lines) | Merged test catalogs + endpoint matrix + E2E deep-dive. Deduplicated E2E test table and running instructions. |
| `seed-data.md` (1107 lines) | `seed-data.md` (~1085 lines) | Removed `## Test Coverage` section (now in `testing.md`). |

All cross-references updated in: `design.md`, `proposal.md`, `tasks.md`, `audit-correction.md`, `audit-p1-task12.md`.

---

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


> **Section 19 moved to `docs/audit-p1-task19.md` → "Implementation Details".**
