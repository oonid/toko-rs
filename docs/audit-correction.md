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

## 12a. Post-Audit Response Shape Verification

Completed 2026-04-09.

**Audit source**: `docs/audit-p1-task12.md` — comprehensive comparison of toko-rs P1 implementation against Medusa vendor reference.

### Context

The audit report (`docs/audit-p1-task12.md`) identified three HIGH-severity response shape incompatibilities (H1, H2, H3). Upon inspection, all three were already correctly implemented in the codebase — the `tasks.md` had them as unchecked `[ ]` despite the code already matching Medusa's response shapes. This section documents the verification and strengthens the contract tests with negative assertions.

### 12a.1: Line item DELETE response — `{ id, object, deleted, parent }`

**Status**: Already implemented correctly.

The `LineItemDeleteResponse` type in `src/cart/types.rs:42-48` already returns the Medusa-compatible shape:

```rust
pub struct LineItemDeleteResponse {
    pub id: String,
    pub object: String,        // "line-item"
    pub deleted: bool,          // true
    pub parent: CartWithItems,  // the updated cart
}
```

The handler in `src/cart/routes.rs:74-85` constructs this correctly. The contract test `test_contract_line_item_delete_response_shape` verifies all 4 fields including nested `parent` shape.

**Medusa reference**: `StoreLineItemDeleteResponse` = `DeleteResponseWithParent<"line-item", StoreCartResponse>` → `{ id, object: "line-item", deleted: true, parent: StoreCart }`.

### 12a.2: Cart complete response — `{ type: "order", order }` only

**Status**: Already implemented correctly. Strengthened with negative assertion.

The `CartCompleteResponse` type in `src/order/types.rs:19-24` has exactly 2 fields:

```rust
pub struct CartCompleteResponse {
    #[serde(rename = "type")]
    pub response_type: String,  // "order"
    pub order: OrderWithItems,
}
```

No `payment` field exists. The audit report noted that a prior version had `payment` as a top-level field, but the current implementation does not.

**Contract test strengthened**: `test_contract_order_complete_response_shape` now asserts:
- Exactly 2 top-level keys (`type`, `order`)
- `payment` key is NOT present

**Medusa reference**: `StoreCompleteCartResponse` success case = `{ type: "order", order: StoreOrder }`. The error case `{ type: "cart", cart, error }` requires `payment_session` table (deferred to P2).

### 12a.3: Order GET response — `{ order }` only

**Status**: Already implemented correctly. Strengthened with negative assertion.

The `OrderResponse` type in `src/order/types.rs:6-9` has exactly 1 field:

```rust
pub struct OrderResponse {
    pub order: OrderWithItems,
}
```

No `payment` field exists. The order object is a valid subset of Medusa's `StoreOrder` — missing optional fields (`payment_collections`, `fulfillments`, `shipping_methods`) that depend on deferred tables.

**Contract test strengthened**: `test_contract_order_detail_response_shape` now asserts:
- Exactly 1 top-level key (`order`)
- `payment` key is NOT present

**Medusa reference**: `StoreOrderResponse` = `{ order: StoreOrder }` where `payment_collections` is optional (`?`) on `StoreOrder`.

### Files Changed

| # | File | Change |
|---|---|---|
| 12a.1 | N/A | Already implemented — `LineItemDeleteResponse` in `src/cart/types.rs:42-48`, handler in `src/cart/routes.rs:74-85` |
| 12a.2 | `tests/contract_test.rs` | Added negative assertion: cart complete response has exactly 2 keys, no `payment` |
| 12a.3 | `tests/contract_test.rs` | Added negative assertion: order detail response has exactly 1 key, no `payment` |
| — | `openspec/changes/implementation-p1-core-mvp/tasks.md` | Marked 12a.1–12a.3 as `[x]` |
| — | `docs/audit-correction.md` | Added section 12a |

### TDD Record (12a)

1. **RED**: Added negative assertions to 2 contract tests (`test_contract_order_complete_response_shape`, `test_contract_order_detail_response_shape`) — assertions assert `payment` is absent and exact key count
2. **GREEN**: Assertions pass immediately — code was already correct. No production code changes needed.
3. **Verify**: 104 tests pass, clippy clean, zero warnings

---

## 12b. Post-Audit Error Handling Divergence Fixes

Completed 2026-04-09.

**Audit source**: `docs/audit-p1-task12.md` findings M1 and M4.

### Context

The audit identified two error handling divergences between toko-rs and Medusa:

1. **M1**: `AppError::Conflict.error_type()` returned `"unexpected_state"` instead of `"conflict"`. Medusa's error handler (`error-handler.ts:47-49`) maps `MedusaError.Types.CONFLICT` to `type: "conflict"`. The previous audit (Task 7a.1) had intentionally changed FROM `"conflict"` TO `"unexpected_state"` based on the spec table, but direct comparison with Medusa's source shows the spec table was incorrect.

2. **M4**: Empty cart completion returned `AppError::Conflict` (HTTP 409). An empty cart is an invalid request (client error), not a conflict (concurrent modification). Should be 400.

### 12b.1: `AppError::Conflict.error_type()` — `"unexpected_state"` → `"conflict"`

**Before:**
```
Conflict → 409, type: "unexpected_state", code: "invalid_state_error"
```

**After:**
```
Conflict → 409, type: "conflict", code: "invalid_state_error"
```

**Affected error sites** (all use `AppError::Conflict`):
- `src/order/repository.rs:30` — cart already completed
- `src/order/repository.rs:117` — display_id race condition
- `src/cart/repository.rs:90` — cannot update completed cart
- `src/cart/repository.rs:128` — cannot add item to completed cart

**Medusa evidence**:
- `vendor/medusa/packages/core/utils/src/common/errors.ts:16`: `CONFLICT: "conflict"`
- `vendor/medusa/packages/core/framework/src/http/middlewares/error-handler.ts:47-49`: `case MedusaError.Types.CONFLICT: statusCode = 409; errObj.code = INVALID_STATE_ERROR;`

### 12b.2: Empty cart completion — `Conflict` (409) → `InvalidData` (400)

**Before:**
```
Empty cart completion → 409 Conflict, type: "unexpected_state", code: "invalid_state_error"
```

**After:**
```
Empty cart completion → 400 Bad Request, type: "invalid_data", code: "invalid_request_error"
```

An empty cart is semantically an invalid request — the client should not attempt to complete a cart with no items. A 409 Conflict implies concurrent modification or state race, which is not the case here.

**Note**: Already-completed cart completion (line 30) and the completed-cart guards in cart repository remain as `AppError::Conflict` (409) — those are genuine state conflicts.

### Updated Error Mapping Table (post 12b)

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

### Files Changed

| # | File | Change |
|---|---|---|
| 12b.1 | `src/error.rs:58` | `error_type()` match arm: `"unexpected_state"` → `"conflict"` |
| 12b.1 | `src/error.rs:170` | Unit test assertion updated |
| 12b.1 | `tests/cart_test.rs:441` | Completed cart update: `"unexpected_state"` → `"conflict"` |
| 12b.1 | `tests/contract_test.rs:689-694` | Completed cart contract: `"unexpected_state"` → `"conflict"` |
| 12b.2 | `src/order/repository.rs:41` | `AppError::Conflict` → `AppError::InvalidData` |
| 12b.2 | `tests/order_test.rs:127,129` | Empty cart: 409→400, `"unexpected_state"`→`"invalid_data"` |
| 12b.2 | `tests/order_test.rs:364-393` | Renamed test + changed assertions: 409→400, `"unexpected_state"`→`"invalid_data"`, `"invalid_state_error"`→`"invalid_request_error"` |
| 12b.2 | `tests/contract_test.rs:637-662` | Renamed test: `test_error_409_empty_cart_completion` → `test_error_400_empty_cart_completion`. Changed: 409→400, `"unexpected_state"`→`"invalid_data"`, `"invalid_state_error"`→`"invalid_request_error"` |
| — | `openspec/changes/implementation-p1-core-mvp/tasks.md` | Marked 12b.1–12b.2 as `[x]` |
| — | `docs/audit-correction.md` | Added section 12b |

### TDD Record (12b)

1. **RED**: Updated `src/error.rs:58` (production code for 12b.1 — single line, immediate effect). Updated all test assertions for both 12b.1 and 12b.2 in one pass. Ran `cargo test` — 1 failure confirmed (`test_error_400_empty_cart_completion` still getting 409 from production code).
2. **GREEN**: Changed `src/order/repository.rs:41` from `AppError::Conflict` to `AppError::InvalidData`.
3. **Verify**: 104 tests pass, clippy clean, zero warnings

---

## 12c. Post-Audit Database Schema Gap Fixes

Completed 2026-04-09.

**Audit source**: `docs/audit-p1-task12.md` findings M2, M3, L4.

### 12c.1: SQLite `customers.email` uniqueness — column-level → partial composite index

**Before (SQLite):**
```sql
email TEXT UNIQUE NOT NULL
```

**After (SQLite):**
```sql
email TEXT NOT NULL,
-- ...
CREATE UNIQUE INDEX uq_customers_email ON customers (email, has_account) WHERE deleted_at IS NULL;
```

Now matches the PG migration which has:
```sql
CONSTRAINT uq_customers_email UNIQUE (email, has_account) WHERE deleted_at IS NULL
```

**Why this matters**: Medusa allows the same email for both a guest and a registered customer (differentiated by `has_account`). The previous column-level `UNIQUE` blocked this. The partial composite index also excludes soft-deleted rows, allowing email reuse after deletion.

**Medusa evidence**: `vendor/medusa/packages/modules/customer/src/models/customer.ts` defines `@Index({ name: "...", on: ["email", "has_account"], unique: true, where: "deleted_at IS NULL" })`.

### 12c.2: `product_variant_option` pivot — composite unique constraint

**Before (both PG and SQLite):**
```sql
CREATE TABLE product_variant_option (
    id TEXT PRIMARY KEY,
    variant_id TEXT NOT NULL REFERENCES product_variants(id) ON DELETE CASCADE,
    option_value_id TEXT NOT NULL REFERENCES product_option_values(id) ON DELETE CASCADE
);
```

**After (both PG and SQLite):**
```sql
CREATE TABLE product_variant_option (
    id TEXT PRIMARY KEY,
    variant_id TEXT NOT NULL REFERENCES product_variants(id) ON DELETE CASCADE,
    option_value_id TEXT NOT NULL REFERENCES product_option_values(id) ON DELETE CASCADE,
    CONSTRAINT uq_product_variant_option UNIQUE (variant_id, option_value_id)
);
```

Prevents duplicate pivot rows where the same variant is bound to the same option value twice. The application code in `src/product/repository.rs` uses `resolve_variant_options_tx` which inserts one row per binding, but without this constraint a bug or race could produce duplicates.

### 12c.3: `_sequences` table adopted for `display_id` generation

**Before:**
```sql
SELECT COALESCE(MAX(display_id), 0) + 1 FROM orders
```

**After:**
```sql
UPDATE _sequences SET value = value + 1 WHERE name = 'order_display_id' RETURNING value
```

The `_sequences` table was created in migrations but never used — `create_from_cart` was using `MAX(display_id)+1` which has a race window between SELECT and INSERT under concurrent requests. The `map_display_id_conflict()` handler partially mitigated this by catching SQLite error code 2067, but produced a 409 error instead of seamless sequencing.

The atomic `UPDATE ... RETURNING` runs inside the same transaction as the order INSERT — there is no gap between reading and writing the sequence value. Two concurrent transactions will serialize on the `_sequences` row lock (SQLite database-level locking), so the second transaction always sees the incremented value.

**Files changed:**
- `src/order/repository.rs:46-49` — replaced `SELECT COALESCE(MAX(display_id), 0) + 1 FROM orders` with `UPDATE _sequences SET value = value + 1 WHERE name = 'order_display_id' RETURNING value`

**Note**: `map_display_id_conflict()` is retained as a safety net for the `display_id` UNIQUE constraint on the orders table.

### Files Changed

| # | File | Change |
|---|---|---|
| 12c.1 | `migrations/sqlite/002_customers.sql` | Removed column-level `UNIQUE`, added `CREATE UNIQUE INDEX uq_customers_email ON customers (email, has_account) WHERE deleted_at IS NULL` |
| 12c.2 | `migrations/001_products.sql` | Added `CONSTRAINT uq_product_variant_option UNIQUE (variant_id, option_value_id)` to `product_variant_option` table |
| 12c.2 | `migrations/sqlite/001_products.sql` | Same as PG |
| 12c.3 | `migrations/004_orders.sql` | Removed `_sequences` table DDL and seed INSERT |
| 12c.3 | `src/order/repository.rs:46-49` | Replaced `SELECT COALESCE(MAX(display_id), 0) + 1 FROM orders` with `UPDATE _sequences SET value = value + 1 WHERE name = 'order_display_id' RETURNING value` |
| — | `openspec/changes/implementation-p1-core-mvp/tasks.md` | Marked 12c.1–12c.3 as `[x]` |
| — | `docs/audit-correction.md` | Added section 12c |

### TDD Record (12c)

1. **RED**: N/A — migration-only changes; existing tests produce valid data. Constraints add safety net for edge cases not yet exercised by tests (guest+registered same email, duplicate pivot rows).
2. **GREEN**: Applied all 3 migration fixes across 5 files.
3. **Verify**: 104 tests pass, clippy clean, zero warnings

---

## 12d. Post-Audit Missing Index Additions

Completed 2026-04-09.

**Audit source**: `docs/audit-p1-task12.md` findings L1, L3.

### Context

The audit identified three missing performance indexes that Medusa defines on `cart_line_items` and `carts`. These indexes improve query performance when looking up line items by variant or product (used in `add_line_item`'s cross-cart variant lookup) and when filtering carts by currency.

### Indexes Added

| # | Index | Table | Columns | Partial? | Medusa Reference |
|---|---|---|---|---|---|
| 12d.1 | `idx_cart_line_items_variant_id` | `cart_line_items` | `(variant_id)` | `WHERE deleted_at IS NULL AND variant_id IS NOT NULL` | `IDX_line_item_variant_id` |
| 12d.2 | `idx_cart_line_items_product_id` | `cart_line_items` | `(product_id)` | `WHERE deleted_at IS NULL AND product_id IS NOT NULL` | `IDX_line_item_product_id` |
| 12d.3 | `idx_carts_currency_code` | `carts` | `(currency_code)` | `WHERE deleted_at IS NULL` | `IDX_cart_curency_code` |

### Updated Cart Index Inventory (post 12d)

| Index | PG | SQLite |
|---|---|---|
| `idx_carts_customer_id` partial | Yes | Yes |
| `idx_cart_line_items_cart_id` partial | Yes | Yes |
| `idx_cart_line_items_variant_id` partial | **Added** | **Added** |
| `idx_cart_line_items_product_id` partial | **Added** | **Added** |
| `idx_carts_currency_code` partial | **Added** | **Added** |

### Files Changed

| # | File | Change |
|---|---|---|
| 12d.1–12d.3 | `migrations/003_carts.sql` | Added 3 indexes |
| 12d.1–12d.3 | `migrations/sqlite/003_carts.sql` | Added 3 indexes |
| — | `openspec/changes/implementation-p1-core-mvp/tasks.md` | Marked 12d.1–12d.3 as `[x]` |
| — | `docs/audit-correction.md` | Added section 12d |

### TDD Record (12d)

1. **RED**: N/A — migration-only changes; indexes improve performance without affecting query results.
2. **GREEN**: Added 3 indexes to both PG and SQLite `003_carts.sql`.
3. **Verify**: 104 tests pass, clippy clean, zero warnings

---

## 12e. Post-Audit Verification Pass

Completed 2026-04-09.

### Context

After completing tasks 12a–12d, this section verifies that all contract and integration tests are consistent with the changes applied and that the test suite passes cleanly.

### Verification Matrix

| Change | Contract Test | Integration Test | Status |
|---|---|---|---|
| 12a.1 Line item DELETE `{id, object, deleted, parent}` | `test_contract_line_item_delete_response_shape` | `test_cart_full_flow` (step 5) | Consistent |
| 12a.2 Cart complete `{type, order}` only | `test_contract_order_complete_response_shape` (2 keys, no `payment`) | `test_complete_cart_creates_order` | Consistent |
| 12a.3 Order GET `{order}` only | `test_contract_order_detail_response_shape` (1 key, no `payment`) | `test_get_order_by_id` | Consistent |
| 12b.1 Conflict `type: "conflict"` | `test_error_409_completed_cart_update` | `test_cart_update_completed_cart_rejected`, `test_cart_add_item_to_completed_cart_rejected`, `test_complete_already_completed_cart_rejected` | Consistent |
| 12b.2 Empty cart → 400 `invalid_data` | `test_error_400_empty_cart_completion` | `test_complete_empty_cart_rejected`, `test_complete_empty_cart_returns_bad_request_format` | Consistent |
| 12c.1 SQLite email partial index | Migration-only | N/A | No test change needed |
| 12c.2 Pivot unique constraint | Migration-only | N/A | No test change needed |
| 12c.3 `_sequences` adopted | N/A | `test_display_id_increments` | Consistent |
| 12d.1–12d.3 Missing indexes | Migration-only | N/A | No test change needed |

### Test Results

- **104 tests pass** across 10 test suites (unit + integration)
- **Clippy clean** — zero warnings with `-D warnings`
- **No new test failures** introduced by 12a–12d

### Files Changed

| # | File | Change |
|---|---|---|
| — | `openspec/changes/implementation-p1-core-mvp/tasks.md` | Marked 12e.1–12e.4 as `[x]` |
| — | `docs/audit-correction.md` | Added section 12e |

---

## 14a. Second Audit — P1 Business Logic Correctness Fixes

Completed 2026-04-09.

**Audit source**: `docs/audit-p1-task14.md` findings B1–B7.

### 14a.1: Completed cart mutation guards on update/delete line item

**Before**: `update_line_item` and `delete_line_item` in `src/cart/repository.rs` did not check `completed_at`, allowing mutations on completed carts.

**After**: Both methods now fetch the cart, check `completed_at.is_some()`, and return `AppError::Conflict` if completed — matching the existing guard in `add_line_item`.

**Tests added**: `test_cart_update_line_item_on_completed_cart_rejected`, `test_cart_delete_line_item_on_completed_cart_rejected`

### 14a.2: Fix variant option wiring — use ID instead of title lookup

**Before**: `resolve_variant_options_tx` used `SELECT id FROM product_variants WHERE product_id = ? AND title = ? ORDER BY created_at DESC LIMIT 1` to find the just-inserted variant. On duplicate titles (e.g., "Default"), this returned the wrong variant.

**After**: `insert_variant_tx` returns the `ProductVariant` (with generated ID). The caller passes `variant.id` directly to `resolve_variant_options_tx`, which now accepts `variant_id: &str` instead of performing a lookup.

**Medusa reference**: `createProducts_` in `product-module-service.ts:1675-1694` pre-generates IDs and attaches options in-memory.

### 14a.3: Error on missing option values instead of silent skip

**Before**: When a `(option_title, value_string)` pair didn't match any `product_option_values` row, the code silently skipped it with `if let Some(val) { ... }`.

**After**: Returns `AppError::NotFound("Option value 'X' not found for option 'Y'")`.

**Medusa reference**: `assignOptionsToVariants` in `product-module-service.ts:2167-2171` throws `MedusaError(INVALID_DATA, ...)`.

**Test added**: `test_variant_option_value_not_found_rejected`

### 14a.4: Validate variant options cover ALL product options

**Before**: A product with options "Size" and "Color" could have a variant that only specified "Size".

**After**: Before inserting variants, each variant's `options` map is checked against all created option titles. Missing options return `AppError::InvalidData`.

**Medusa reference**: `validateProductCreatePayload` in `product-module-service.ts:1893-1928`.

**Test added**: `test_variant_missing_option_coverage_rejected`

### 14a.5: Validate variant option combinations are unique

**Before**: Two variants with the same Size=XL, Color=Blue would succeed, causing ambiguous add-to-cart resolution.

**After**: Before inserting variants, option maps are collected, sorted, and checked for duplicates. Returns `AppError::InvalidData("Duplicate option combination for variant 'X'")`.

**Medusa reference**: `checkIfVariantsHaveUniqueOptionsCombinations` in `product-module-service.ts:2244-2269`.

**Test added**: `test_variant_duplicate_option_combination_rejected`

### 14a.6: Product `status` as typed enum

**Before**: `status: Option<String>` accepted any string (e.g., "banana").

**After**: `status: Option<ProductStatus>` with `#[serde(rename_all = "snake_case")]` — `Draft`, `Proposed`, `Published`, `Rejected`. Invalid strings are rejected at JSON deserialization (HTTP 422).

**Medusa reference**: `z.nativeEnum(ProductStatus)` in Medusa's product validators.

**Tests added**: `test_product_invalid_status_rejected`, `test_product_update_validates`

### 14a.7: `.validate()` call added to `admin_update_product`

**Before**: `admin_update_product` in `src/product/routes.rs` deserialized input but never called `.validate()`, bypassing all validation constraints.

**After**: Added `payload.validate().map_err(|e| AppError::InvalidData(e.to_string()))?` before the repository call.

### Files Changed

| # | File | Change |
|---|---|---|
| 14a.1 | `src/cart/repository.rs` | Added `completed_at` guard to `update_line_item` and `delete_line_item` |
| 14a.2 | `src/product/repository.rs` | `resolve_variant_options_tx` accepts `variant_id: &str` directly; `insert_variant_tx` return value used by callers |
| 14a.3 | `src/product/repository.rs` | Missing option values now return `AppError::NotFound` |
| 14a.4 | `src/product/repository.rs` | Validates variant options cover all product options |
| 14a.5 | `src/product/repository.rs` | Validates unique option combinations via `HashSet` |
| 14a.6 | `src/product/types.rs` | Added `ProductStatus` enum; `status` fields typed as `Option<ProductStatus>` |
| 14a.6 | `src/product/repository.rs` | `create_product` and `update` use `ProductStatus::as_str()` |
| 14a.7 | `src/product/routes.rs` | Added `.validate()` to `admin_update_product` |
| — | `tests/cart_test.rs` | +2 tests: update/delete line item on completed cart |
| — | `tests/contract_test.rs` | +5 tests: invalid status, update validates, option value not found, missing option coverage, duplicate option combo |
| — | `openspec/changes/implementation-p1-core-mvp/tasks.md` | Marked 14a.1–14a.7 as `[x]` |
| — | `docs/audit-correction.md` | Added section 14a |

### TDD Record (14a)

1. **RED** (14a.1): Added 2 tests for update/delete on completed cart — failed because no guard existed.
2. **GREEN** (14a.1): Added `completed_at` guard to both methods. Tests passed.
3. **RED+GREEN** (14a.2–14a.5): Rewrote `resolve_variant_options_tx` (ID-based + error on missing), added option coverage and uniqueness validation in `create_product`. Added 4 contract tests.
4. **RED+GREEN** (14a.6): Added `ProductStatus` enum, updated all status fields and callers. Added 2 contract tests (invalid status returns 422).
5. **GREEN** (14a.7): Added `.validate()` to update handler.
6. **Verify**: 111 tests pass (was 104, +7 new), clippy clean, zero warnings.

---

## 14f. Second Audit — Customer Address Schema + Response Stubs

Completed 2026-04-09.

**Audit source**: `docs/audit-p1-task14.md` findings R9, R10.

### Context

The `customer_addresses` table existed in migrations but was flagged **dormant** — no Rust model, no repository code, no response fields. A Medusa frontend expects `customer.addresses` to be an array (not `undefined`) and reads `is_default_shipping`/`is_default_billing` on each address.

### 14f.1–14f.2: Schema alignment with Medusa

**Changes to both PG and SQLite `002_customers.sql`:**

1. Added `is_default_shipping BOOLEAN NOT NULL DEFAULT FALSE` and `is_default_billing BOOLEAN NOT NULL DEFAULT FALSE` columns
2. Added partial unique indexes enforcing at most one default shipping and one default billing address per customer:
   ```sql
   CREATE UNIQUE INDEX uq_customer_default_shipping ON customer_addresses (customer_id) WHERE is_default_shipping = TRUE AND deleted_at IS NULL;
   CREATE UNIQUE INDEX uq_customer_default_billing ON customer_addresses (customer_id) WHERE is_default_billing = TRUE AND deleted_at IS NULL;
   ```
3. Renamed `state_province` → `province` to match Medusa's field name
4. Relaxed `address_1` and `country_code` from `NOT NULL` to nullable — matching Medusa's model

### 14f.3–14f.5: Rust model + repository + response wrapper

Added `CustomerAddress` model in `src/customer/models.rs` with all fields matching the migration schema.

Changed `CustomerResponse` to use `CustomerWithAddresses`:
```rust
pub struct CustomerWithAddresses {
    #[serde(flatten)]
    pub customer: Customer,
    pub addresses: Vec<CustomerAddress>,
    pub default_billing_address_id: Option<String>,
    pub default_shipping_address_id: Option<String>,
}
```

Added `list_addresses()` and `wrap_with_addresses()` helper in `src/customer/repository.rs` — reads addresses from DB, derives `default_*_address_id` from the `is_default_*` flags.

All customer routes now return `CustomerWithAddresses` instead of bare `Customer`.

### 14f.6: Contract test strengthened

`test_contract_customer_response_shape` now asserts:
- `addresses` is an array (empty for new customer)
- `default_billing_address_id` is null
- `default_shipping_address_id` is null

### Files Changed

| # | File | Change |
|---|---|---|
| 14f.1 | `migrations/002_customers.sql` | Added `is_default_shipping/billing` columns + partial unique indexes |
| 14f.1 | `migrations/sqlite/002_customers.sql` | Same as PG |
| 14f.2 | Both `002_customers.sql` | Renamed `state_province` → `province`, relaxed nullability |
| 14f.3 | `src/customer/models.rs` | Added `CustomerAddress` struct |
| 14f.3 | `src/customer/types.rs` | Added `CustomerWithAddresses` wrapper, updated `CustomerResponse` |
| 14f.4 | `src/customer/repository.rs` | Added `list_addresses`, `wrap_with_addresses`; all methods return `CustomerWithAddresses` |
| 14f.5 | `src/customer/routes.rs` | No changes needed (uses `CustomerResponse` which wraps `CustomerWithAddresses`) |
| 14f.6 | `tests/contract_test.rs` | Strengthened customer shape assertions |
| — | `openspec/changes/implementation-p1-core-mvp/tasks.md` | Marked 14f.1–14f.6 as `[x]` |
| — | `docs/audit-correction.md` | Added section 14f |
| — | `docs/database-foundation.md` | Updated `customer_addresses` status from **dormant** to **active (read)** |

### TDD Record (14f)

1. **RED**: N/A for schema changes. Contract test assertions added after implementation.
2. **GREEN**: Added model, repository helper, response wrapper. All routes updated.
3. **Verify**: 111 tests pass, clippy clean, zero warnings.

---

## 14b. Second Audit — P1 Input Validation Fixes

Completed 2026-04-09.

**Audit source**: `docs/audit-p1-task14.md` findings B8, B9, V4.

### 14b.1: `#[serde(deny_unknown_fields)]` on all input types

Medusa uses `.strict()` on all Zod schemas — unknown fields are rejected. toko-rs was silently ignoring misspelled fields.

**After**: All 9 input structs across 4 modules now have `#[serde(deny_unknown_fields)]`:
- `CreateCartInput`, `UpdateCartInput`, `AddLineItemInput`, `UpdateLineItemInput` (cart)
- `CreateProductInput`, `CreateProductOptionInput`, `CreateProductVariantInput`, `UpdateProductInput` (product)
- `CreateCustomerInput`, `UpdateCustomerInput` (customer)
- `ListOrdersParams` (order)

Unknown fields now return HTTP 422 with serde's error message.

**Tests added**: `test_unknown_fields_rejected`, `test_product_unknown_fields_rejected`

### 14b.2: `metadata` type tightened to `HashMap<String, Value>`

**Before**: `metadata: Option<serde_json::Value>` — accepts arrays, strings, numbers.
**After**: `metadata: Option<HashMap<String, serde_json::Value>>` — accepts only JSON objects with string keys.

Added `metadata_to_json()` helper in `src/types.rs` to convert `HashMap` → `sqlx::types::Json<serde_json::Value>` at repository bind sites. All 9 bind sites across 3 repositories updated.

**Medusa reference**: `z.record(z.unknown())` in all validators.

**Test added**: `test_metadata_must_be_object`

### 14b.3: `FindParams.limit` capped at 100

**Before**: No upper bound — `limit=9999999` was possible.
**After**: `capped_limit()` method returns `self.limit.min(100)`. Both `FindParams` and `ListOrdersParams` have this method. All list queries use it. Response `limit` field reflects the capped value.

**Test added**: `test_list_limit_capped`

### 14b.4: `Forbidden` (403) error variant

**Added**: `AppError::Forbidden(String)` — HTTP 403, `type: "forbidden"`, `code: "invalid_state_error"`.

Not used by any P1 route (no RBAC yet), but available for P2 auth middleware.

**Test added**: `test_forbidden` (unit test in `src/error.rs`)

### Updated Error Mapping Table (post 14b)

| toko-rs Variant | HTTP Status | `type` | `code` |
|---|---|---|---|
| `NotFound` | 404 | `not_found` | `invalid_request_error` |
| `InvalidData` | 400 | `invalid_data` | `invalid_request_error` |
| `DuplicateError` | 422 | `duplicate_error` | `invalid_request_error` |
| `Forbidden` | **403** | **`forbidden`** | `invalid_state_error` |
| `Conflict` | 409 | `conflict` | `invalid_state_error` |
| `Unauthorized` | 401 | `unauthorized` | `unknown_error` |
| `UnexpectedState` | 500 | `unexpected_state` | `invalid_state_error` |
| `DatabaseError` | 500 | `database_error` | `api_error` |
| `MigrationError` | 500 | `database_error` | `api_error` |

### Files Changed

| # | File | Change |
|---|---|---|
| 14b.1 | `src/cart/types.rs` | `#[serde(deny_unknown_fields)]` on 4 input structs |
| 14b.1 | `src/product/types.rs` | `#[serde(deny_unknown_fields)]` on 4 input structs |
| 14b.1 | `src/customer/types.rs` | `#[serde(deny_unknown_fields)]` on 2 input structs |
| 14b.1 | `src/order/types.rs` | `#[serde(deny_unknown_fields)]` on `ListOrdersParams` |
| 14b.2 | All 4 `types.rs` | `metadata: Option<HashMap<String, serde_json::Value>>` |
| 14b.2 | `src/types.rs` | Added `metadata_to_json()` helper |
| 14b.2 | `src/cart/repository.rs` | 4 bind sites use `metadata_to_json()` |
| 14b.2 | `src/product/repository.rs` | 3 bind sites use `metadata_to_json()` |
| 14b.2 | `src/customer/repository.rs` | 2 bind sites use `metadata_to_json()` |
| 14b.3 | `src/types.rs` | Added `capped_limit()` to `FindParams` |
| 14b.3 | `src/order/types.rs` | Added `capped_limit()` to `ListOrdersParams` |
| 14b.3 | `src/product/repository.rs` | 2 list queries use `capped_limit()` |
| 14b.3 | `src/order/repository.rs` | 1 list query uses `capped_limit()` |
| 14b.3 | `src/product/routes.rs` | Responses return `capped_limit()` |
| 14b.3 | `src/order/routes.rs` | Response returns `capped_limit()` |
| 14b.4 | `src/error.rs` | Added `Forbidden` variant + unit test |
| — | `tests/contract_test.rs` | +4 tests: unknown fields, metadata type, limit cap |
| — | `openspec/changes/implementation-p1-core-mvp/tasks.md` | Marked 14b.1–14b.4 as `[x]` |
| — | `docs/audit-correction.md` | Added section 14b |

### TDD Record (14b)

1. **RED+GREEN** (14b.1): Added `deny_unknown_fields` to all input types. Tests: unknown fields → 422.
2. **RED+GREEN** (14b.2): Changed metadata type + added helper + updated all bind sites. Test: string metadata → 422.
3. **RED+GREEN** (14b.3): Added `capped_limit()` + used in queries/responses. Test: `limit=999999` → response `limit <= 100`.
4. **GREEN** (14b.4): Added `Forbidden` variant + unit test.
5. **Verify**: 117 tests pass (was 111, +6 new), clippy clean, zero warnings.

## 14c. P1 Response Shape Stubs (Medusa frontend compatibility)

### Finding

Audit source: `docs/audit-p1-task14.md`, section "Response Shape Stubs".

| ID | Severity | Finding | Resolution |
|----|----------|---------|------------|
| 14c.1 | HIGH | Product missing `images`, `is_giftcard`, `discountable` fields | Added to `ProductWithRelations` with defaults: `images: []`, `is_giftcard: false`, `discountable: true` |
| 14c.2 | HIGH | Variant missing `calculated_price` | Added `CalculatedPrice` struct + `calculated_price` field mirroring raw `price` |
| 14c.3 | HIGH | Cart/Order missing 22 computed total fields | Added via `from_items()` helpers: subtotal, tax_total, discount_total, etc. |
| 14c.4 | HIGH | Customer missing `addresses` array | Completed in 14f — `CustomerWithAddresses` wrapper |
| 14c.5 | MEDIUM | Order missing `payment_status`, `fulfillment_status` | Added stub enums: `"not_paid"`, `"not_fulfilled"` |
| 14c.6 | MEDIUM | Order missing `fulfillments`, `shipping_methods` arrays | Added empty array stubs |
| 14c.7 | MEDIUM | Line items missing `requires_shipping`, `is_discountable`, `is_tax_inclusive` | Added `#[sqlx(skip)]` defaults via `from_items()` |

### Files Changed

| Task | File | Change |
|------|------|--------|
| 14c.1 | `src/product/models.rs` | `ProductWithRelations`: +images, +is_giftcard, +discountable |
| 14c.2 | `src/product/models.rs` | `ProductVariantWithOptions`: +calculated_price (CalculatedPrice struct) |
| 14c.3 | `src/cart/models.rs` | `CartWithItems`: 22 total fields + `from_items()` |
| 14c.3 | `src/order/models.rs` | `OrderWithItems`: 22 total fields + `from_items()` |
| 14c.5 | `src/order/models.rs` | `OrderWithItems`: +payment_status, +fulfillment_status |
| 14c.6 | `src/order/models.rs` | `OrderWithItems`: +fulfillments, +shipping_methods |
| 14c.7 | `src/cart/models.rs` | `CartLineItem`: +requires_shipping, +is_discountable, +is_tax_inclusive (#[sqlx(skip)]) |
| 14c.7 | `src/order/models.rs` | `OrderLineItem`: +requires_shipping, +is_discountable, +is_tax_inclusive (#[sqlx(skip)]) |
| — | `tests/contract_test.rs` | Strengthened assertions for all stubs |

### TDD Record (14c)

1. **GREEN** (14c.1–14c.7): Added all stub fields with defaults. Contract tests strengthened to assert field presence and default values.
2. **Verify**: 117 tests pass, clippy clean.

---

## 14d. P1 Middleware / Security Fixes

### Finding

Audit source: `docs/audit-p1-task14.md`, section "Middleware / Security".

| ID | Severity | Finding | Resolution |
|----|----------|---------|------------|
| 14d.1 | MEDIUM | `CorsLayer::permissive()` in production allows any origin without restriction | Replaced with config-driven CORS: `AppConfig.cors_origins` (comma-separated, default `"*"` for dev). `build_cors_layer()` constructs proper AllowOrigin/AllowMethods/AllowHeaders. `app_router_with_cors()` for production use in main.rs |
| 14d.2 | MEDIUM | No centralized SQLite error code mapping — repos have ad-hoc `message().contains("UNIQUE")` string matching | Added `map_sqlite_constraint()` in `src/error.rs`: code 2067 → `DuplicateError`, 787 → `NotFound`, 1299 → `InvalidData`. Available for repos to use alongside existing custom-message helpers |

### Files Changed

| Task | File | Change |
|------|------|--------|
| 14d.1 | `src/config.rs` | Added `cors_origins: String` field with default `"*"` |
| 14d.1 | `src/lib.rs` | Added `build_cors_layer()`, `app_router_with_cors()` alongside backward-compat `app_router()` |
| 14d.1 | `src/main.rs` | Uses `app_router_with_cors(state, &config.cors_origins)` |
| 14d.2 | `src/error.rs` | Added `pub fn map_sqlite_constraint(e: sqlx::Error) -> AppError` + unit test |

### TDD Record (14d)

1. **GREEN** (14d.1): Added `cors_origins` config, `build_cors_layer()`, and `app_router_with_cors()`. Existing tests use `app_router()` (permissive backward-compat).
2. **GREEN** (14d.2): Added `map_sqlite_constraint()` + unit test for non-DB error passthrough.
3. **Verify**: 117 tests pass, clippy clean, zero warnings.
