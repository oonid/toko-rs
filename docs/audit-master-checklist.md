# P1 Medusa Compatibility — Master Checklist

Consolidates all findings from `docs/audit-p1-task{12,14,18,19,20}.md` and `docs/audit-correction.md` into a single reference. Every item is tagged with its source audit, status, and where it was fixed (or why it was deferred).

**Last verified**: 2026-04-22 — 199 tests pass on SQLite + PostgreSQL, clippy clean, fmt clean.

---

## 1. Bugs Fixed

| # | Source | Finding | Fix | Audit-Correction Section |
|---|--------|---------|-----|--------------------------|
| 1 | T20 BUG-1 | Cart completion had no idempotency protection — concurrent requests could create duplicate orders | `SELECT ... FOR UPDATE` (PG) + guard UPDATE (SQLite) in `create_from_cart` | 20a |
| 2 | T20 BUG-2 | Product soft-delete ran 4 independent UPDATEs — failure mid-way left inconsistent state | Wrapped in single DB transaction | 20b |
| 3 | T20 BUG-3 | Snapshot captured 5 fields but model extracted 8 — `variant_option_values` always null | Captured `variant_option_values` via JOIN in `add_line_item` | 20c |
| 4 | T19 S1 | `JsonDataError` mapped to `DuplicateError` (422) instead of `InvalidData` (400) | Changed mapping in `src/extract.rs` | 19a |
| 5 | T18 S1 | `load_relations` didn't filter `deleted_at IS NULL` on child tables — soft-deleted items leaked | Added filter to 5 child-table queries | 18a |
| 6 | T14 B1 | `update_line_item`/`delete_line_item` didn't check cart completion — mutations on completed carts | Added `completed_at` guard to both methods | 14a.1 |
| 7 | T14 B2 | `resolve_variant_options_tx` used fragile title-based variant lookup | Changed to pass `variant.id` directly from `insert_variant_tx` | 14a.2 |
| 8 | T14 B3 | Missing option values silently skipped instead of erroring | Returns `AppError::NotFound` | 14a.3 |
| 9 | T14 B4 | No validation that variant options cover all product options | Added coverage check before insert | 14a.4 |
| 10 | T14 B5 | No validation that variant option combinations are unique | Added `HashSet` dedup check | 14a.5 |
| 11 | T14 B6 | Product `status` accepted any string — no enum validation | Added `ProductStatus` enum with serde rename | 14a.6 |
| 12 | T14 B7 | `admin_update_product` never called `.validate()` | Added `.validate()` call | 14a.7 |
| 13 | T19 S3 | Soft-delete didn't cascade to children (variants, options, option values) | Added cascade UPDATEs in `soft_delete` | 19c |
| 14 | T19 S4 | Variant option uniqueness not checked against DB (only against input batch) | Added DB check in `add_variant` | 19d |
| 15 | T12 M4 | Empty cart completion returned 409 (Conflict) instead of 400 (Bad Request) | Changed to `AppError::InvalidData` | 12b.2 |
| 16 | T21 B1 | `GET /store/orders/{id}` had no customer ownership check — any authenticated user could view any order | Added `CustomerId` extraction + ownership verification in `store_get_order` | 21a |
| 17 | T21 B2 | `add_line_item` had no row-level lock — concurrent requests could create duplicate line items | Added `FOR UPDATE` (PG) + guard UPDATE (SQLite) to cart row in `add_line_item` | 21b |

---

## 2. Response Shape Fixes (Medusa Frontend Compatibility)

| # | Source | Finding | Fix | Section |
|---|--------|---------|-----|---------|
| 16 | T12 H1 | Line item DELETE returned `{ cart }` instead of `{ id, object, deleted, parent }` | `LineItemDeleteResponse` struct (already implemented, verified) | 12a.1 |
| 17 | T12 H2 | Cart complete response had extra top-level `payment` field | Removed — now `{ type: "order", order }` only | 12a.2 |
| 18 | T12 H3 | Order GET response had extra top-level `payment` field | Removed — now `{ order }` only | 12a.3 |
| 19 | T14 R1 | Variant had flat `price` instead of `calculated_price` object | Added `CalculatedPrice` struct with `calculated_amount`, etc. | 14c.2 |
| 20 | T14 R2 | Missing `images` array on product | Added `images: Vec<ImageStub>` (default `[]`) | 14c.1, 18f |
| 21 | T14 R3 | Missing `is_giftcard`, `discountable` on product | Added to `ProductWithRelations` with defaults | 14c.1 |
| 22 | T14 R4 | Missing ~22 computed total fields on cart | Added via `from_items()` | 14c.3 |
| 23 | T14 R5 | Missing fields on cart line items (`requires_shipping`, etc.) | Added `#[sqlx(skip)]` stubs via `from_items()` | 14c.7 |
| 24 | T14 R7 | Missing ~22 computed total fields on order | Added via `from_items()` | 14c.3 |
| 25 | T14 R8 | Missing `payment_status`, `fulfillment_status` on order | Added stubs: `"not_paid"`, `"not_fulfilled"` | 14c.5 |
| 26 | T14 R9 | Missing `addresses` array on customer | `CustomerWithAddresses` wrapper + `CustomerAddress` model | 14f |
| 27 | T14 R10 | Missing `fulfillments`, `shipping_methods` on order | Added empty array stubs | 14c.6 |
| 28 | T18 S7 | `images: Vec<String>` vs Medusa's `ProductImage[]` objects | Changed to `Vec<ImageStub>` with `{ url }` shape | 18f |
| 29 | T19 S2 | Missing admin variant endpoints (list/get/update/delete) | Implemented 4 endpoints | 19b |
| 30 | T19 S12 | Line-item snapshot fields not surfaced in response | Added top-level fields from snapshot JSON | 19k |
| 31 | T20 F3 | Missing per-item totals on line items (`item_total`, `subtotal`, etc.) | 12 `#[sqlx(skip)]` fields per line item, computed in `from_items()` | 20f |
| 32 | T21 S4 | Internal `snapshot` field leaked to API responses on cart and order line items | Added `#[serde(skip)]` to `snapshot` on `CartLineItem` and `OrderLineItem` | 21c |

---

## 3. Input Type & Validation Fixes

| # | Source | Finding | Fix | Section |
|---|--------|---------|-----|---------|
| 32 | T14 B8 | No `deny_unknown_fields` on input types — unknown fields silently ignored | Added to all 9 input structs | 14b.1 |
| 33 | T14 B9 | `metadata` type too permissive — accepted arrays/strings | Changed to `HashMap<String, Value>` | 14b.2 |
| 34 | T14 V4 | `FindParams.limit` had no upper bound — `limit=9999999` possible | Added `capped_limit()` max 100 | 14b.3 |
| 35 | T18 S2 | `deny_unknown_fields` rejects Medusa SDK fields not in toko-rs schemas | Documented as intentional strict validation (Decision 12) | 18g |
| 36 | T20 F1a | `CreateProductInput` missing `is_giftcard`, `discountable`, `subtitle` | Added to create/update inputs | 20d |
| 37 | T20 F1b | `CreateProductVariantInput` missing `variant_rank` | Added `variant_rank: Option<i64>` | 20d |
| 38 | T21 I1 | `deny_unknown_fields` on 5 types that Medusa doesn't use `.strict()` on — SDK clients rejected | Removed from `CreateProductOptionInput`, `AddLineItemInput`, `UpdateLineItemInput`, `CreateCustomerInput`, `UpdateCustomerInput` | 21g |

---

## 4. Error Handling Fixes

| # | Source | Finding | Fix | Section |
|---|--------|---------|-----|---------|
| 38 | T4a | `DuplicateError` returned 409 instead of 422 | Changed to 422 | 4a |
| 39 | T4a | `UnexpectedState` returned 409 instead of 500 | Changed to 500 | 4a |
| 40 | T4a | `DatabaseError` message leaked raw sqlx error details | Sanitized to `"Internal server error"` | 7a.2 |
| 41 | T4a | `MigrationError` type was `"migration_error"` (not in spec enum) | Changed to `"database_error"` | 7a.3 |
| 42 | T7a | `Conflict` type was `"unexpected_state"` (spec table was wrong) | Changed to `"conflict"` per Medusa source | 12b.1 |
| 43 | T14 V2 | No `Forbidden` (403) error variant | Added `AppError::Forbidden` | 14b.4 |
| 44 | T14 V3 | No structured SQLite error code mapping | Added `map_sqlite_constraint()` | 14d.2 |
| 45 | T18 S6 | JSON deserialization errors bypassed AppError — inconsistent error shapes | Custom `Json<T>` extractor in `src/extract.rs` | 18d |
| 46 | T18 S6 | PG error code `40001` (serialization failure) not mapped to Conflict | Added mapping via `is_serialization_failure()` | 18e |
| 47 | T20 F2 | `ValidationError` variant was dead code (never used anywhere) | Removed from enum | 20e |
| 48 | T20 F4 | Error messages prefixed: `"Not Found: ..."`, `"Duplicate Error: ..."` | Removed all prefixes from `#[error(...)]` attrs | 20g |

---

## 5. Database Schema Fixes

| # | Source | Finding | Fix | Section |
|---|--------|---------|-----|---------|
| 49 | T4b | Pivot table named `product_variant_options` (plural) — Medusa uses singular | Renamed to `product_variant_option` | 4b |
| 50 | T4b | SQLite `products.handle` — column UNIQUE blocked reuse after soft-delete | Changed to partial unique index `WHERE deleted_at IS NULL` | 4b |
| 51 | T4b | Missing unique index on `(product_id, title)` for options | Added partial unique index | 4b |
| 52 | T4b | Missing unique index on `(option_id, value)` for option values | Added partial unique index | 4b |
| 53 | T4c | `create_product` and `add_variant` not transactional | Wrapped in `self.pool.begin()` transactions | 4c |
| 54 | T7b | SQLite missing CHECK constraints (products.status, payment.status, orders.status) | Added CHECK constraints to match PG | 7b |
| 55 | T7b | SQLite missing `DEFAULT 'idr'` on carts.currency_code, payment_records fields | Added defaults to match PG | 7b |
| 56 | T7b | `PaymentRecord.provider` was `Option<String>` — PG has `NOT NULL DEFAULT 'manual'` | Changed to `String` | 7b |
| 57 | T7c | 13 missing SQLite performance indexes + 3 missing child table definitions | Added all indexes + `customer_addresses`, `cart_line_items`, `order_line_items` tables | 7c |
| 58 | T7d | Payment creation outside order transaction — orphan risk on failure | Moved inside transaction via `create_with_tx()` | 7d.1 |
| 59 | T7d | `display_id` UNIQUE race produced raw DatabaseError (500) | Added `map_display_id_conflict()` → 409 Conflict | 7d.2 |
| 60 | T7f | Default currency hardcoded to `"usd"` — project needs IDR | Config-driven `DEFAULT_CURRENCY_CODE` (default `"idr"`) | 7f |
| 61 | T12 M2 | SQLite `customers.email` had column-level UNIQUE — blocked guest+registered same email | Changed to partial composite index `(email, has_account) WHERE deleted_at IS NULL` | 12c.1 |
| 62 | T12 L4 | `product_variant_option` pivot had no unique constraint | Added `UNIQUE(variant_id, option_value_id)` | 12c.2 |
| 63 | T12 M3 | `_sequences` table created but unused — `MAX(display_id)+1` had race condition | Adopted atomic `UPDATE ... RETURNING value` | 12c.3 |
| 64 | T12 L1 | Missing indexes on `cart_line_items.variant_id`, `.product_id`, `carts.currency_code` | Added 3 indexes | 12d |
| 65 | T19 S9 | Missing `metadata` on `product_options` and `product_option_values` | Added `metadata JSONB` to both tables + Rust models | 19i |
| 66 | T19 S10 | Missing 6 DB indexes (variant composite, orders, order_line_items) | Added 6 indexes to both PG and SQLite | 19j |
| 67 | T21 D1 | `orders.status` CHECK missing "draft" — would reject legitimate Medusa orders | Added `'draft'` to CHECK in both PG and SQLite migrations | 21e |
| 68 | T21 D2 | `payment_records` missing `deleted_at` — no soft-delete support | Added `deleted_at` column to both PG and SQLite + model | 21f |

---

## 6. Business Logic Fixes

| # | Source | Finding | Fix | Section |
|---|--------|---------|-----|---------|
| 67 | T4d | Cart had no computed `item_total` and `total` fields | Added computed fields in `get_cart()` | 4d.1 |
| 68 | T4d | No completed-cart guard on `update_cart` | Added 409 guard | 4d.2 |
| 69 | T18 S3 | Double-soft-delete returned 404 (Medusa returns 200 idempotent) | Check if already-deleted, return success | 18b |
| 70 | T18 S4 | Line item dedup ignored metadata (Medusa checks deep-equality) | Added metadata comparison — different metadata creates separate item | 18c |
| 71 | T19 S5 | Line item dedup didn't consider `unit_price` (Medusa does for custom pricing) | Added `unit_price` to WHERE clause | 19e |
| 72 | T19 S7 | Missing `company_name` on customer | Added column, model, input fields, tests | 19g |
| 73 | T20 F5 | Default pagination limit was 20 — Medusa uses 50 | Changed `default_limit()` to return 50 | 20h |
| 74 | T21 B3 | Cart `update_cart`/`update_line_item`/`delete_line_item` UPDATE had no `completed_at IS NULL` guard — race condition with concurrent completion | Added `AND completed_at IS NULL` (or subquery equivalent) to all 3 UPDATE WHERE clauses | 21h |

---

## 7. Configuration & Infrastructure

| # | Source | Finding | Fix | Section |
|---|--------|---------|-----|---------|
| 74 | T4e | `AppConfig` missing defaults for HOST, PORT, RUST_LOG | Added serde defaults | 4e.1 |
| 75 | T4d | Cart completion stub returned bare `StatusCode::NOT_IMPLEMENTED` | Changed to proper JSON error via `AppError::Conflict` | 4d.3 |
| 76 | T14 V1 | CORS was `CorsLayer::permissive()` — production-unsafe | Config-driven CORS via `AppConfig.cors_origins` | 14d.1 |
| 77 | T17 | No SQLite feature flag support | Added compile-time feature flag with type aliases | 17 |

---

## 8. Deferred / Known Divergences (No Fix Needed)

| # | Source | Finding | Reason |
|---|--------|---------|--------|
| 78 | T14 R6 | Cart complete has no error branch `{ type: "cart", cart, error }` | Requires `payment_session` table (P2). Dead code infrastructure exists in `CartCompleteResponse::error()`. |
| 79 | T14 V5 / T20 F4 | Error message format differences | Medusa doesn't guarantee message format. Fixed prefixes in 20g, but exact messages may differ. |
| 80 | T18 S2 | `deny_unknown_fields` rejects Medusa SDK fields not in toko-rs schemas | Intentional strict validation (Decision 12) |
| 81 | T18 S8 | Variant exposes flat `price: i64` that Medusa does not have | Harmless extension alongside `calculated_price` (Decision 13) |
| 82 | T18 S9 | Order line item prefix `oli` vs Medusa's `ordli` | Cosmetic, documented in design.md |
| 83 | T18 S10 | Default pagination limit 20 vs Medusa's 50 | **Fixed** in 20h |
| 84 | T18 S5 | Validation errors include `code` field; Medusa Zod errors omit it | toko-rs is more consistent; documented as intentional |
| 85 | T19 S8 | `GET /store/orders/:id` requires auth — Medusa doesn't | Intentional security improvement (Decision 14) |
| 86 | T19 S11 | Error `code` field always present — Medusa omits for some types | More consistent than Medusa; documented |
| 87 | T19 S13 | `customer_id` on cart create is extra — Medusa infers from auth | Needed until real auth (Decision 15) |
| 88 | T19 S14-S20 | LOW findings: message formatting, `deleted_at` exposure, extra fields, cosmetic prefixes, `estimate_count`, total sub-fields, variant title nullable | No functional impact, documented |
| 89 | T20 F6 | N+1 query pattern in order listing | Performance, not correctness |
| 90 | T20 F7 | Generic DB constraint messages lose context | Requires PG-specific error detail parsing |
| 91 | T20 F8 | `code` field mismatches for Unauthorized, Forbidden, UnexpectedState | Minor difference |
| 92 | T12 L1-L7 | LOW: missing indexes, missing entities (P2+), no admin auth, missing cart fields | P2 scope or by-design |
| 93 | T14 B | All P2 deferred items: multi-currency pricing, address CRUD, order transfers, shipping, etc. | Documented in design.md and audit reports |
| 94 | T21 S5 | `has_account` on store customer response — reported as leaked but confirmed present in Medusa store query config | FALSE POSITIVE — no fix needed |

---

## Summary Statistics

| Category | Count |
|----------|-------|
| Bugs fixed | 17 |
| Response shape fixes | 17 |
| Input/validation fixes | 7 |
| Error handling fixes | 11 |
| Database schema fixes | 20 |
| Business logic fixes | 8 |
| Config/infra fixes | 4 |
| **Total fixes applied** | **84** |
| Deferred to P2 | 16 |
| Known divergences (by design) | ~10 |

### Final Error Mapping Table

| Variant | HTTP | `type` | `code` |
|---|---|---|---|
| `NotFound` | 404 | `not_found` | `invalid_request_error` |
| `InvalidData` | 400 | `invalid_data` | `invalid_request_error` |
| `DuplicateError` | 422 | `duplicate_error` | `invalid_request_error` |
| `Forbidden` | 403 | `forbidden` | `invalid_state_error` |
| `Conflict` | 409 | `conflict` | `invalid_state_error` |
| `Unauthorized` | 401 | `unauthorized` | `unknown_error` |
| `UnexpectedState` | 500 | `unexpected_state` | `invalid_state_error` |
| `DatabaseError` | 500 | `database_error` | `api_error` |
| `MigrationError` | 500 | `database_error` | `api_error` |
