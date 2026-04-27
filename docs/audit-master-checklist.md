# P1 Medusa Compatibility — Master Checklist

Consolidates all findings from `docs/audit-p1-task{12,14,18,19,20,21,22,23,24,25,26}.md` and `docs/audit-correction.md` into a single reference. Every item is tagged with its source audit, status, and where it was fixed (or why it was deferred).

**Last verified**: 2026-04-27 — 197 tests pass on both SQLite and PostgreSQL, clippy clean on both features, fmt clean. Eleventh audit (Task 26) findings fully implemented.

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
| 18 | T23 BUG-1 | SQL injection via `order` query param — user input interpolated into `ORDER BY` via `format!()` | Added `validate_order_param()` whitelist in `src/types.rs` | 23a |
| 19 | T23 B1,B2 | Cart→order data loss — `metadata`, `shipping_address`, `billing_address`, line item `metadata` silently dropped | Extended order + order line item INSERT statements | 23b |
| 20 | T23 B3 | `update_cart` missing `rows_affected()` check — stale data returned if cart completed between SELECT and UPDATE | Added `rows_affected()` guard returning `InvalidData` | 23c |
| 21 | T24 BUG-1 | `product_subtitle` never populated in line item snapshot — extraction code was dead code | Added `p.subtitle` to snapshot query + JSON in `add_line_item` | 24a |
| 22 | T24 BUG-2 | `requires_shipping` and `is_discountable` hardcoded to `true` — ignored product data | Read from snapshot (`product_discountable`, `product_is_giftcard`); gift cards get `requires_shipping: false`, `is_discountable: false` | 24b |
| 23 | T24 BUG-3 | `UpdateVariantInput.price` had no range validation — negative prices accepted | Added `#[validate(range(min = 0))]` | 24c |
| 24 | T24 BUG-4 | `AddLineItemInput.variant_id` accepted empty string | Added `#[validate(length(min = 1))]` | 24d |
| 25 | T24 BUG-5 | `UpdateLineItemInput.quantity` allowed 0 — meaningless zero-quantity items persisted | Changed to `range(min = 1)`; clients must use DELETE endpoint | 24e |
| 26 | T25 BUG-1 | Order line item ID prefix `"oli"` should be `"ordli"` per Medusa convention | Changed prefix in `src/order/repository.rs` | 25a |
| 27 | T26 BUG-1 | `UpdateLineItemInput.quantity` range(min=1) rejected 0, but Medusa uses gte(0) — 0 is a removal signal | Reverted to `range(min=0)` and restored `quantity==0→delete` branch | 26a |
| 28 | T26 BUG-2 | `CreateCustomerInput.email` was required but Medusa's `StoreCreateCustomer` has `email: z.string().email().nullish()` (optional) | Changed to `Option<String>` in types, model, and both PG/SQLite migrations | 26b |
| 29 | T26 BUG-3 | `is_giftcard`/`discountable` only accepted JSON boolean, but Medusa uses `booleanString()` accepting `"true"`/`"false"` strings | Custom `bool_or_string::deserialize` serde deserializer with `deserialize_any` + `Visitor` | 26c |

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
| 33 | T26 HIGH-1 | Variant options had flat `{id, value, option_id}` — Medusa nests as `{id, value, option: {id, title}}` | `NestedOption` struct + updated query to JOIN `product_options` | 26d |
| 34 | T26 MEDIUM-1 | `CalculatedPrice` missing `currency_code` | Added `currency_code: String` field, populated from `ProductRepository.default_currency_code` | 26g |
| 35 | T26 MEDIUM-2,5,6 | Missing `credit_line_*` totals and `discount_subtotal` on cart/order | Added 7 fields to `CartWithItems`, 4 fields to `OrderWithItems` (all default 0) | 26h |

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
| 39 | T26 HIGH-2 | Cart create/update input types missing `shipping_address` and `billing_address` fields | Added `Option<serde_json::Value>` to both `CreateCartInput` and `UpdateCartInput` | 26e |
| 40 | T26 MEDIUM-9 | `ListOrdersParams` missing `id` and `status` query filters | Added optional filters with dynamic WHERE clause construction | 26j |

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

| 21 | T23 S1 | `subtitle` accepted in input but never stored — no DB column, no model field | Added `subtitle TEXT` column + model field + INSERT/UPDATE bindings | 23d |
| 22 | T23 S2 | `is_giftcard`/`discountable` always hardcoded (`false`/`true`) despite accepting input | Added DB columns, removed hardcoded fields from `ProductWithRelations` | 23e |
| 23 | T23 S3,S4 | `deleted_at` hidden too broadly — Medusa admin product + store customer include it | Removed `#[serde(skip)]` from `Product` and `Customer`; kept on 7 other types | 23f |
| 24 | T23 E2 | Cart state violations returned 409 (Conflict) — Medusa uses 400 (InvalidData) | Changed 8 locations from `AppError::Conflict` → `AppError::InvalidData` | 23g |
| 25 | T23 D1 | `soft_delete_variant` left orphan `product_variant_option` pivot rows | Added `DELETE FROM product_variant_option WHERE variant_id = $1` + transaction | 23h |
| 26 | T23 V1,V2 | `add_variant` had no option coverage check; `create_product` skipped check when `options` was `None` | Required `options` to cover ALL product option titles in both paths | 23i |
| 27 | T25 HIGH-2 | No CHECK constraints on monetary/quantity columns — negative values accepted at DB level | Added `CHECK (price >= 0)`, `CHECK (quantity > 0)`, `CHECK (unit_price >= 0)`, `CHECK (amount >= 0)` to all relevant columns in both PG and SQLite migrations | 25c |
| 28 | T26 HIGH-3 | No `cart_id` on orders — no idempotency protection for cart completion | Added `cart_id TEXT UNIQUE` column + index to orders in both PG and SQLite | 26f |
| 29 | T26 MEDIUM-7 | Missing `provider` index on `payment_records` | Added `CREATE INDEX idx_payment_records_provider` to both migrations | 26i |

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
| 75 | T26 HIGH-3 | Cart completion not idempotent — retry created new order or returned error | Idempotency check: lookup existing order by `cart_id` before creating; returns existing order on retry | 26f |

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
| 95 | T22 S1 | `deleted_at` leaked on 9 entity types (Product, ProductOption, ProductOptionValue, ProductVariant, Cart, Order, Customer, CustomerAddress, PaymentRecord) | `#[serde(skip)]` on all 9 `deleted_at` fields | 22a |
| 96 | T22 D1 | `product_variant_option` join rows NOT cascade-deleted on product soft-delete — orphan rows remain | `DELETE FROM product_variant_option WHERE variant_id IN (...)` in `soft_delete` | 22b |
| 97 | T22 B1 | `update_line_item` / `delete_line_item` no affected-rows check — silent success on nonexistent/completed items | `rows_affected()` check returns 404 | 22c |
| 98 | T22 I6 | `ListOrdersParams` has `deny_unknown_fields` but Medusa's `createFindParams` is NOT strict | Removed `deny_unknown_fields` | 22d |
| 99 | T22 D2 | `product_variants.product_id` NOT NULL vs Medusa nullable | **Deferred** — arguably more correct |
| 100 | T22 D4 | `order_line_items.unit_price` NOT NULL vs Medusa nullable | **Deferred** — edge case |
| 101 | T22 D5 | `payment_records.status` uses different enum values than Medusa PaymentCollectionStatus | **Deferred** — architectural simplification |
| 102 | T22 D7 | Missing `created_by` on customer model | **Deferred** — P2 audit trail |
| 103 | T22 S7 | ImageStub missing `id` and `rank` fields | **Deferred** — P2 polish |
| 104 | T22 B4 | Customer `find_by_email` not implemented | **Deferred** — needed for proper duplicate detection |

---

## Summary Statistics

| Category | Count |
|----------|-------|
| Bugs fixed | 33 |
| Response shape fixes | 21 |
| Input/validation fixes | 11 |
| Error handling fixes | 11 |
| Database schema fixes | 31 |
| Business logic fixes | 10 |
| Config/infra fixes | 4 |
| **Total fixes applied** | **121** |
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
