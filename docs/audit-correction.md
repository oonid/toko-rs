# Audit Corrections: Medusa Compatibility

Completed 2026-04-08. Tasks 4a–4f done.
Post-implementation audit fixes: 2026-04-08. Tasks 7a.1–7a.4, 7b.1–7b.7, 7f.1–7f.8 done.

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
| `Conflict` | **409** | **`unexpected_state`** | `invalid_state_error` | Medusa: 409, spec table row: `unexpected_state` |
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

## 7f. Default Currency Change USD → IDR (Config-Driven)

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
| `Conflict` | 409 | `unexpected_state` | `invalid_state_error` |
| `Unauthorized` | 401 | `unauthorized` | `unknown_error` |
| `UnexpectedState` | 500 | `unexpected_state` | `invalid_state_error` |
| `DatabaseError` | 500 | `database_error` | `api_error` |
| `MigrationError` | 500 | `database_error` | `api_error` |
