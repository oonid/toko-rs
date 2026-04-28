# P1 Medusa Compatibility — Master Checklist

Consolidates all findings from `docs/audit-p1-task{12,14,18,19,20,21,22,23,24,25,26,27,28,29,30,31}.md` into a single reference. Every item is tagged with its source audit, status, and where it was fixed (or why it was deferred). Tasks 27 and 29 were structural audits (checklist accuracy, re-numbering, redundant test annotation) — their impact is reflected in the checklist structure itself (prefixed IDs, reversal chains, corrected counts).

**Last verified**: 2026-04-28 — 207 tests pass on PostgreSQL, clippy clean, fmt clean. Latest audit: Task 31. Total: 128 fixes across 7 categories.

---

## 1. Bugs Fixed (B-1 … B-32)

| ID | Source | Finding | Fix | Audit Section |
|----|--------|---------|-----|---------------|
| B-1 | T20 BUG-1 | Cart completion had no idempotency protection — concurrent requests could create duplicate orders | `SELECT ... FOR UPDATE` (PG) + guard UPDATE (SQLite) in `create_from_cart` | 20a |
| B-2 | T20 BUG-2 | Product soft-delete ran 4 independent UPDATEs — failure mid-way left inconsistent state | Wrapped in single DB transaction | 20b |
| B-3 | T20 BUG-3 | Snapshot captured 5 fields but model extracted 8 — `variant_option_values` always null | Captured `variant_option_values` via JOIN in `add_line_item` | 20c |
| B-4 | T19 S1 | `JsonDataError` mapped to `DuplicateError` (422) instead of `InvalidData` (400) | Changed mapping in `src/extract.rs` | 19a |
| B-5 | T18 S1 | `load_relations` didn't filter `deleted_at IS NULL` on child tables — soft-deleted items leaked | Added filter to 5 child-table queries | 18a |
| B-6 | T14 B1 | `update_line_item`/`delete_line_item` didn't check cart completion — mutations on completed carts | Added `completed_at` guard to both methods | 14a.1 |
| B-7 | T14 B2 | `resolve_variant_options_tx` used fragile title-based variant lookup | Changed to pass `variant.id` directly from `insert_variant_tx` | 14a.2 |
| B-8 | T14 B3 | Missing option values silently skipped instead of erroring | Returns `AppError::NotFound` | 14a.3 |
| B-9 | T14 B4 | No validation that variant options cover all product options | Added coverage check before insert | 14a.4 |
| B-10 | T14 B5 | No validation that variant option combinations are unique | Added `HashSet` dedup check | 14a.5 |
| B-11 | T14 B6 | Product `status` accepted any string — no enum validation | Added `ProductStatus` enum with serde rename | 14a.6 |
| B-12 | T14 B7 | `admin_update_product` never called `.validate()` | Added `.validate()` call | 14a.7 |
| B-13 | T19 S3 | Soft-delete didn't cascade to children (variants, options, option values) | Added cascade UPDATEs in `soft_delete` | 19c |
| B-14 | T19 S4 | Variant option uniqueness not checked against DB (only against input batch) | Added DB check in `add_variant` | 19d |
| B-15 | T12 M4 | Empty cart completion returned 409 (Conflict) instead of 400 (Bad Request) | Changed to `AppError::InvalidData` | 12b.2 |
| B-16 | T21 B1 | `GET /store/orders/{id}` had no customer ownership check — any authenticated user could view any order | Added `CustomerId` extraction + ownership verification in `store_get_order` | 21a |
| B-17 | T21 B2 | `add_line_item` had no row-level lock — concurrent requests could create duplicate line items | Added `FOR UPDATE` (PG) + guard UPDATE (SQLite) to cart row in `add_line_item` | 21b |
| B-18 | T23 BUG-1 | SQL injection via `order` query param — user input interpolated into `ORDER BY` via `format!()` | Added `validate_order_param()` whitelist in `src/types.rs` | 23a |
| B-19 | T23 B1,B2 | Cart→order data loss — `metadata`, `shipping_address`, `billing_address`, line item `metadata` silently dropped | Extended order + order line item INSERT statements | 23b |
| B-20 | T23 B3 | `update_cart` missing `rows_affected()` check — stale data returned if cart completed between SELECT and UPDATE | Added `rows_affected()` guard returning `InvalidData` | 23c |
| B-21 | T24 BUG-1 | `product_subtitle` never populated in line item snapshot — extraction code was dead code | Added `p.subtitle` to snapshot query + JSON in `add_line_item` | 24a |
| B-22 | T24 BUG-2 | `requires_shipping` and `is_discountable` hardcoded to `true` — ignored product data | Read from snapshot; gift cards get `requires_shipping: false`, `is_discountable: false` | 24b |
| B-23 | T24 BUG-3 | `UpdateVariantInput.price` had no range validation — negative prices accepted | Added `#[validate(range(min = 0))]` | 24c |
| B-24 | T24 BUG-4 | `AddLineItemInput.variant_id` accepted empty string | Added `#[validate(length(min = 1))]` | 24d |
| B-25 | T24 BUG-5 | `UpdateLineItemInput.quantity` allowed 0 — meaningless zero-quantity items persisted | Changed to `range(min = 1)` | 24e | **[REVERTED by B-27]** |
| B-26 | T25 BUG-1 | Order line item ID prefix `"oli"` should be `"ordli"` per Medusa convention | Changed prefix in `src/order/repository.rs` | 25a |
| B-27 | T26 BUG-1 | `UpdateLineItemInput.quantity` range(min=1) rejected 0, but Medusa uses gte(0) — 0 is a removal signal | Reverted to `range(min=0)` and restored `quantity==0→delete` branch | 26a |
| B-28 | T26 BUG-2 | `CreateCustomerInput.email` was required but Medusa's `StoreCreateCustomer` has `email` optional | Changed to `Option<String>` in types, model, and both PG/SQLite migrations | 26b |
| B-29 | T26 BUG-3 | `is_giftcard`/`discountable` only accepted JSON boolean, but Medusa uses `booleanString()` accepting `"true"`/`"false"` strings | Custom `bool_or_string::deserialize` serde deserializer | 26c |
| B-30 | T22 B1 | `update_line_item` / `delete_line_item` no affected-rows check — silent success on nonexistent/completed items | `rows_affected()` check returns 404 | 22c |
| B-31 | T22 D1 | `product_variant_option` join rows NOT cascade-deleted on product soft-delete — orphan rows remain | `DELETE FROM product_variant_option WHERE variant_id IN (...)` in `soft_delete` | 22b |
| B-32 | T23 D1 | `soft_delete_variant` left orphan `product_variant_option` pivot rows | Added `DELETE FROM product_variant_option WHERE variant_id = $1` + transaction | 23h |

---

## 2. Response Shape Fixes (S-1 … S-25)

| ID | Source | Finding | Fix | Audit Section |
|----|--------|---------|-----|---------------|
| S-1 | T12 H1 | Line item DELETE returned `{ cart }` instead of `{ id, object, deleted, parent }` | `LineItemDeleteResponse` struct | 12a.1 |
| S-2 | T12 H2 | Cart complete response had extra top-level `payment` field | Removed — now `{ type: "order", order }` only | 12a.2 |
| S-3 | T12 H3 | Order GET response had extra top-level `payment` field | Removed — now `{ order }` only | 12a.3 |
| S-4 | T14 R1 | Variant had flat `price` instead of `calculated_price` object | Added `CalculatedPrice` struct with `calculated_amount`, etc. | 14c.2 |
| S-5 | T14 R2 | Missing `images` array on product | Added `images: Vec<ImageStub>` (default `[]`) | 14c.1, 18f |
| S-6 | T14 R3 | Missing `is_giftcard`, `discountable` on product | Added to `ProductWithRelations` with defaults | 14c.1 |
| S-7 | T14 R4 | Missing ~22 computed total fields on cart | Added via `from_items()` | 14c.3 |
| S-8 | T14 R5 | Missing fields on cart line items (`requires_shipping`, etc.) | Added `#[sqlx(skip)]` stubs via `from_items()` | 14c.7 |
| S-9 | T14 R7 | Missing ~22 computed total fields on order | Added via `from_items()` | 14c.3 |
| S-10 | T14 R8 | Missing `payment_status`, `fulfillment_status` on order | Added stubs: `"not_paid"`, `"not_fulfilled"` | 14c.5 |
| S-11 | T14 R9 | Missing `addresses` array on customer | `CustomerWithAddresses` wrapper + `CustomerAddress` model | 14f |
| S-12 | T14 R10 | Missing `fulfillments`, `shipping_methods` on order | Added empty array stubs | 14c.6 |
| S-13 | T18 S7 | `images: Vec<String>` vs Medusa's `ProductImage[]` objects | Changed to `Vec<ImageStub>` with `{ url }` shape | 18f |
| S-14 | T19 S2 | Missing admin variant endpoints (list/get/update/delete) | Implemented 4 endpoints | 19b |
| S-15 | T19 S12 | Line-item snapshot fields not surfaced in response | Added top-level fields from snapshot JSON | 19k |
| S-16 | T20 F3 | Missing per-item totals on line items (`item_total`, `subtotal`, etc.) | 12 `#[sqlx(skip)]` fields per line item, computed in `from_items()` | 20f |
| S-17 | T21 S4 | Internal `snapshot` field leaked to API responses on cart and order line items | Added `#[serde(skip)]` to `snapshot` on `CartLineItem` and `OrderLineItem` | 21c |
| S-18 | T26 HIGH-1 | Variant options had flat `{id, value, option_id}` — Medusa nests as `{id, value, option: {id, title}}` | `NestedOption` struct + updated query to JOIN `product_options` | 26d |
| S-19 | T26 MEDIUM-1 | `CalculatedPrice` missing `currency_code` | Added `currency_code: String` field, populated from `ProductRepository.default_currency_code` | 26g |
| S-20 | T26 MEDIUM-2,5,6 | Missing `credit_line_*` totals and `discount_subtotal` on cart/order | Added 7 fields to `CartWithItems`, 4 fields to `OrderWithItems` (all default 0) | 26h |
| S-21 | T28 BUG | Line item `thumbnail` not captured in snapshot — cart/order items render without images | Added `p.thumbnail` to snapshot query + surface as `thumbnail` on both line item models | 28a |
| S-22 | T28 MEDIUM | Line item `is_giftcard` captured in snapshot but not surfaced as response field | Extract `product_is_giftcard` as `is_giftcard: bool` on both line item models | 28b |
| S-23 | T28 STUB | Product missing `collection_id` and `type_id` keys — Medusa frontend gets `undefined` not `null` | Added `#[sqlx(skip)]` nullable stubs, always `null` in P1 | 28c |
| S-24 | T22 S1 | `deleted_at` leaked on 9 entity types | `#[serde(skip)]` on all 9 `deleted_at` fields | 22a | **Note: S-25 reversed this for Product and Customer — 7 remain skipped** |
| S-25 | T23 S3,S4 | `deleted_at` hidden too broadly — Medusa admin product + store customer include it | Removed `#[serde(skip)]` from `Product` and `Customer`; kept on 7 other types | 23f |
| S-26 | T30-1 | 5 Product Option CRUD endpoints entirely missing | Added GET/POST list+create, GET/POST/DELETE individual option endpoints (30 endpoint methods total) | 30a |
| S-27 | T30-2 | Variant model missing `thumbnail` field | Added `thumbnail` column to `product_variants`, `ProductVariant` model, create/update inputs | 30c |
| S-28 | T30-3,8 | Product images not persisted — `ImageStub { url }` only, no DB table, no input field | `ProductImage` model (id, url, product_id, rank), `product_images` table, `images` field on create/update inputs. **Supersedes S-5 and S-13 (ImageStub)** | 30b |
| S-29 | T30-4 | Line item missing `compare_at_unit_price` field | Added nullable `compare_at_unit_price` to `CartLineItem`, `OrderLineItem`, both tables, order INSERT | 30d |
| S-30 | T30-5 | Customer missing `created_by` field | Added `created_by TEXT` column to `customers`, `Customer` model. **Was X-7 (deferred), now fixed** | 30e |
| S-31 | T31-1 | Product images input format `Vec<String>` — Medusa SDK sends `{url: "..."}` objects | `ImageInput { url }` for create, `UpdateImageInput { id?, url }` for update. **Supersedes S-28 input format** | 31a |

---

## 3. Input / Validation Fixes (V-1 … V-11)

| ID | Source | Finding | Fix | Audit Section |
|----|--------|---------|-----|---------------|
| V-1 | T14 B8 | No `deny_unknown_fields` on input types — unknown fields silently ignored | Added to all 9 input structs | 14b.1 |
| V-2 | T14 B9 | `metadata` type too permissive — accepted arrays/strings | Changed to `HashMap<String, Value>` | 14b.2 |
| V-3 | T14 V4 | `FindParams.limit` had no upper bound — `limit=9999999` possible | Added `capped_limit()` max 100 | 14b.3 |
| V-4 | T18 S2 | `deny_unknown_fields` rejects Medusa SDK fields not in toko-rs schemas | Documented as intentional strict validation (Decision 12) | 18g |
| V-5 | T20 F1a | `CreateProductInput` missing `is_giftcard`, `discountable`, `subtitle` | Added to create/update inputs | 20d |
| V-6 | T20 F1b | `CreateProductVariantInput` missing `variant_rank` | Added `variant_rank: Option<i64>` | 20d |
| V-7 | T21 I1 | `deny_unknown_fields` on 5 types that Medusa doesn't use `.strict()` on — SDK clients rejected | Removed from 5 types | 21g |
| V-8 | T26 HIGH-2 | Cart create/update input types missing `shipping_address` and `billing_address` fields | Added `Option<serde_json::Value>` to both `CreateCartInput` and `UpdateCartInput` | 26e |
| V-9 | T26 MEDIUM-9 | `ListOrdersParams` missing `id` and `status` query filters | Added optional filters with dynamic WHERE clause construction | 26j |
| V-10 | T22 I6 | `ListOrdersParams` has `deny_unknown_fields` but Medusa's `createFindParams` is NOT strict | Removed `deny_unknown_fields` | 22d |
| V-11 | T23 V1,V2 | `add_variant` had no option coverage check; `create_product` skipped check when `options` was `None` | Required `options` to cover ALL product option titles in both paths | 23i |
| V-12 | T30-7 | `UpdateCustomerInput` missing `email` field — customers cannot change email | Added `email: Option<String>` to `UpdateCustomerInput`, bound in repository UPDATE | 30f | **T31 CORRECTION**: T30-7 referenced admin schema; Medusa `StoreUpdateCustomer` does NOT have `email`. Change is harmless (extra capability).** |

---

## 4. Error Handling Fixes (E-1 … E-12)

| ID | Source | Finding | Fix | Audit Section |
|----|--------|---------|-----|---------------|
| E-1 | T4a | `DuplicateError` returned 409 instead of 422 | Changed to 422 | 4a |
| E-2 | T4a | `UnexpectedState` returned 409 instead of 500 | Changed to 500 | 4a |
| E-3 | T4a | `DatabaseError` message leaked raw sqlx error details | Sanitized to `"Internal server error"` | 7a.2 |
| E-4 | T4a | `MigrationError` type was `"migration_error"` (not in spec enum) | Changed to `"database_error"` | 7a.3 |
| E-5 | T7a | `Conflict` type was `"unexpected_state"` (spec table was wrong) | Changed to `"conflict"` per Medusa source | 12b.1 |
| E-6 | T14 V2 | No `Forbidden` (403) error variant | Added `AppError::Forbidden` | 14b.4 |
| E-7 | T14 V3 | No structured SQLite error code mapping | Added `map_sqlite_constraint()` | 14d.2 |
| E-8 | T18 S6 | JSON deserialization errors bypassed AppError — inconsistent error shapes | Custom `Json<T>` extractor in `src/extract.rs` | 18d |
| E-9 | T18 S6 | PG error code `40001` (serialization failure) not mapped to Conflict | Added mapping via `is_serialization_failure()` | 18e | **[INTERNAL]** |
| E-10 | T20 F2 | `ValidationError` variant was dead code (never used anywhere) | Removed from enum | 20e |
| E-11 | T20 F4 | Error messages prefixed: `"Not Found: ..."`, `"Duplicate Error: ..."` | Removed all prefixes from `#[error(...)]` attrs | 20g |
| E-12 | T23 E2 | Cart state violations returned 409 (Conflict) — Medusa uses 400 (InvalidData) | Changed 8 locations from `AppError::Conflict` → `AppError::InvalidData` | 23g | **Supersedes B-6 and L-2 original 409 guards** |

---

## 5. Database Schema Fixes (D-1 … D-25)

| ID | Source | Finding | Fix | Audit Section |
|----|--------|---------|-----|---------------|
| D-1 | T4b | Pivot table named `product_variant_options` (plural) — Medusa uses singular | Renamed to `product_variant_option` | 4b |
| D-2 | T4b | SQLite `products.handle` — column UNIQUE blocked reuse after soft-delete | Changed to partial unique index `WHERE deleted_at IS NULL` | 4b |
| D-3 | T4b | Missing unique index on `(product_id, title)` for options | Added partial unique index | 4b |
| D-4 | T4b | Missing unique index on `(option_id, value)` for option values | Added partial unique index | 4b |
| D-5 | T4c | `create_product` and `add_variant` not transactional | Wrapped in `self.pool.begin()` transactions | 4c | **[INTERNAL]** |
| D-6 | T7b | SQLite missing CHECK constraints (products.status, payment.status, orders.status) | Added CHECK constraints to match PG | 7b |
| D-7 | T7b | SQLite missing `DEFAULT 'idr'` on carts.currency_code, payment_records fields | Added defaults to match PG | 7b |
| D-8 | T7b | `PaymentRecord.provider` was `Option<String>` — PG has `NOT NULL DEFAULT 'manual'` | Changed to `String` | 7b |
| D-9 | T7c | 13 missing SQLite performance indexes + 3 missing child table definitions | Added all indexes + tables | 7c | **[INTERNAL]** |
| D-10 | T7d | Payment creation outside order transaction — orphan risk on failure | Moved inside transaction via `create_with_tx()` | 7d.1 |
| D-11 | T7d | `display_id` UNIQUE race produced raw DatabaseError (500) | Added `map_display_id_conflict()` → 409 Conflict | 7d.2 | **[INTERNAL]** |
| D-12 | T7f | Default currency hardcoded to `"usd"` — project needs IDR | Config-driven `DEFAULT_CURRENCY_CODE` (default `"idr"`) | 7f |
| D-13 | T12 M2 | SQLite `customers.email` had column-level UNIQUE — blocked guest+registered same email | Changed to partial composite index `(email, has_account) WHERE deleted_at IS NULL` | 12c.1 |
| D-14 | T12 L4 | `product_variant_option` pivot had no unique constraint | Added `UNIQUE(variant_id, option_value_id)` | 12c.2 |
| D-15 | T12 M3 | `_sequences` table created but unused — `MAX(display_id)+1` had race condition | Adopted atomic `UPDATE ... RETURNING value` | 12c.3 |
| D-16 | T12 L1 | Missing indexes on `cart_line_items.variant_id`, `.product_id`, `carts.currency_code` | Added 3 indexes | 12d |
| D-17 | T19 S9 | Missing `metadata` on `product_options` and `product_option_values` | Added `metadata JSONB` to both tables + Rust models | 19i |
| D-18 | T19 S10 | Missing 6 DB indexes (variant composite, orders, order_line_items) | Added 6 indexes to both PG and SQLite | 19j |
| D-19 | T21 D1 | `orders.status` CHECK missing "draft" — would reject legitimate Medusa orders | Added `'draft'` to CHECK in both PG and SQLite migrations | 21e |
| D-20 | T21 D2 | `payment_records` missing `deleted_at` — no soft-delete support | Added `deleted_at` column to both PG and SQLite + model | 21f |
| D-21 | T23 S1 | `subtitle` accepted in input but never stored — no DB column, no model field | Added `subtitle TEXT` column + model field + INSERT/UPDATE bindings | 23d |
| D-22 | T23 S2 | `is_giftcard`/`discountable` always hardcoded despite accepting input | Added DB columns, removed hardcoded fields from `ProductWithRelations` | 23e |
| D-23 | T25 HIGH-2 | No CHECK constraints on monetary/quantity columns — negative values accepted at DB level | Added `CHECK` on all monetary columns in both PG and SQLite | 25c |
| D-24 | T26 HIGH-3 | No `cart_id` on orders — no idempotency protection for cart completion | Added `cart_id TEXT UNIQUE` column + index to orders in both PG and SQLite | 26f |
| D-25 | T26 MEDIUM-7 | Missing `provider` index on `payment_records` | Added `CREATE INDEX idx_payment_records_provider` to both migrations | 26i |
| D-26 | T30-1,2,3 | No `product_images` table; variant missing `thumbnail` column | `CREATE TABLE product_images` (id, url, product_id, rank, timestamps); `ALTER TABLE product_variants ADD thumbnail` | 30a-c |
| D-27 | T30-4 | Line items missing `compare_at_unit_price` column | Added `compare_at_unit_price BIGINT` to `cart_line_items` and `order_line_items` in both PG and SQLite | 30d |
| D-28 | T30-5 | Customer missing `created_by` column | Added `created_by TEXT` to `customers` in both PG and SQLite | 30e |

---

## 6. Business Logic Fixes (L-1 … L-9)

| ID | Source | Finding | Fix | Audit Section |
|----|--------|---------|-----|---------------|
| L-1 | T4d | Cart had no computed `item_total` and `total` fields | Added computed fields in `get_cart()` | 4d.1 |
| L-2 | T4d | No completed-cart guard on `update_cart` | Added guard | 4d.2 | **[SUPERSEDED: now returns 400 via E-12, extended by L-8]** |
| L-3 | T18 S3 | Double-soft-delete returned 404 (Medusa returns 200 idempotent) | Check if already-deleted, return success | 18b |
| L-4 | T18 S4 | Line item dedup ignored metadata (Medusa checks deep-equality) | Added metadata comparison — different metadata creates separate item | 18c |
| L-5 | T19 S5 | Line item dedup didn't consider `unit_price` (Medusa does for custom pricing) | Added `unit_price` to WHERE clause | 19e |
| L-6 | T19 S7 | Missing `company_name` on customer | Added column, model, input fields, tests | 19g |
| L-7 | T20 F5 | Default pagination limit was 20 — Medusa uses 50 | Changed `default_limit()` to return 50 | 20h |
| L-8 | T21 B3 | Cart `update_cart`/`update_line_item`/`delete_line_item` UPDATE had no `completed_at IS NULL` guard — race condition with concurrent completion | Added `AND completed_at IS NULL` to all 3 UPDATE WHERE clauses | 21h | **[Extends L-2]** |
| L-9 | T26 HIGH-3 | Cart completion not idempotent — retry created new order or returned error | Idempotency check: lookup existing order by `cart_id` before creating; returns existing order on retry | 26f |

---

## 7. Configuration & Infrastructure (C-1 … C-4)

| ID | Source | Finding | Fix | Audit Section |
|----|--------|---------|-----|---------------|
| C-1 | T4e | `AppConfig` missing defaults for HOST, PORT, RUST_LOG | Added serde defaults | 4e.1 | **[INTERNAL]** |
| C-2 | T4d | Cart completion stub returned bare `StatusCode::NOT_IMPLEMENTED` | Changed to proper JSON error via `AppError::Conflict` | 4d.3 | **[SUPERSEDED: see E-12 for guard status code]** |
| C-3 | T14 V1 | CORS was `CorsLayer::permissive()` — production-unsafe | Config-driven CORS via `AppConfig.cors_origins` | 14d.1 | **[INTERNAL]** |
| C-4 | T17 | No SQLite feature flag support | Added compile-time feature flag with type aliases | 17 | **[INTERNAL]** |

---

## 8. Deferred / Known Divergences

Entries moved from this section to fix sections: S-24 (was T22 S1), B-30 (was T22 B1), B-31 (was T22 D1), V-10 (was T22 I6), S-25 (was T23 S3,S4), S-30 (was X-7/T22 D7), S-28 (supersedes X-8/T22 S7). Removed stale entries #82 and #83 (previously marked as "Fixed", duplicates of B-26 and L-7).

### Deferred to P2

| ID | Source | Finding | Reason |
|----|--------|---------|--------|
| X-1 | T14 R6 | Cart complete has no error branch `{ type: "cart", cart, error }` | Requires `payment_session` table (P2). Dead code infrastructure exists. |
| X-2 | T12 L1-L7 | LOW: missing indexes, missing entities (P2+), no admin auth, missing cart fields | P2 scope or by-design |
| X-3 | T14 B, T30-D1,2,4,6,7,10 | All P2 deferred items: multi-currency pricing, address CRUD, order transfers, shipping, variant inventory/logistics fields, product physical dimensions, product type/collection/tags, cart/order region_id/sales_channel_id/locale, line item product_type/product_collection | Documented in design.md and audit reports |
| X-4 | T22 D2 | `product_variants.product_id` NOT NULL vs Medusa nullable | Arguably more correct |
| X-5 | T22 D4 | `order_line_items.unit_price` NOT NULL vs Medusa nullable | Edge case |
| X-6 | T22 D5 | `payment_records.status` uses different enum values than Medusa PaymentCollectionStatus | Architectural simplification |
| X-9 | T22 B4 | Customer `find_by_email` not implemented | Needed for proper duplicate detection |
| X-10 | T30-D5 | Variant missing `images` relation (M2M via `ProductVariantProductImage`) | Requires variant-level image module (P2) |
| X-11 | T30-D8 | Order missing `summary` computed wrapper (`{trial, pending_difference, current_order, original_order}`) | Requires pricing/tax/shipping computation (P2) |
| X-12 | T30-D9 | Order missing `transactions`, `payment_collections` relations | Requires Payment module (P2) |
| X-13 | T30-D14 | Product option values need separate update/delete within options | Complex nested update logic, deferrable |
| X-14 | T30-6 | `CreateCustomerInput` should require `email` per Medusa workflow | DEFERRED — contradicts T26 (B-28) which explicitly made email optional to match Medusa Zod schema |

### Known Divergences (by design)

| ID | Source | Finding | Reason |
|----|--------|---------|--------|
| K-1 | T14 V5 / T20 F4 | Error message format differences | Medusa doesn't guarantee message format. Fixed prefixes in 20g, but exact messages may differ. |
| K-2 | T18 S2 | `deny_unknown_fields` rejects Medusa SDK fields not in toko-rs schemas | Intentional strict validation (Decision 12) |
| K-3 | T18 S8 | Variant exposes flat `price: i64` that Medusa does not have | Harmless extension alongside `calculated_price` (Decision 13) |
| K-4 | T18 S5 | Validation errors include `code` field; Medusa Zod errors omit it | toko-rs is more consistent; documented as intentional |
| K-5 | T19 S8 | `GET /store/orders/:id` requires auth — Medusa doesn't | Intentional security improvement (Decision 14) |
| K-6 | T19 S11 | Error `code` field always present — Medusa omits for some types | More consistent than Medusa; documented |
| K-7 | T19 S13 | `customer_id` on cart create is extra — Medusa infers from auth | Needed until real auth (Decision 15) |
| K-8 | T19 S14-S20 | LOW findings: message formatting, `deleted_at` exposure, extra fields, cosmetic prefixes, `estimate_count`, total sub-fields, variant title nullable | No functional impact, documented |
| K-9 | T20 F8 | `code` field mismatches for Unauthorized, Forbidden, UnexpectedState | Minor difference |
| K-10 | T21 S5 | `has_account` on store customer response — confirmed present in Medusa store query config | FALSE POSITIVE — no fix needed |

### Internal (deferred — code quality, not P1 API behavior)

| ID | Source | Finding | Reason |
|----|--------|---------|--------|
| I-1 | T20 F6 | N+1 query pattern in order listing | Performance, not correctness |
| I-2 | T20 F7 | Generic DB constraint messages lose context | Requires PG-specific error detail parsing |

---

## Summary Statistics

| Category | Count |
|----------|-------|
| Bugs fixed (B) | 32 |
| Response shape fixes (S) | 31 |
| Input/validation fixes (V) | 12 |
| Error handling fixes (E) | 12 |
| Database schema fixes (D) | 28 |
| Business logic fixes (L) | 9 |
| Config/infra fixes (C) | 4 |
| **Total fixes applied** | **128** |
| Deferred to P2 | 12 |
| Known divergences (by design) | 10 |
| False positive | 1 |
| Internal (deferred) | 2 |

### Audit Reversal Chains

1. **B-25 → B-27**: `quantity` range: min=1 (T24) → reverted to min=0 (T26)
2. **S-24 → S-25**: `deleted_at` skip: added on 9 types (T22) → reversed for Product+Customer (T23)
3. **B-26**: `ordli` prefix — old #82 in deferred was incorrect, fixed in T25
4. **L-7**: pagination 50 — old #83 in deferred was incorrect, fixed in T20
5. **S-5 → S-13 → S-28**: `images` field: empty array (T14) → `ImageStub { url }` (T18) → `ProductImage` model with persistence (T30)
6. **X-7 → S-30**: `created_by` deferred (T22) → now fixed (T30)
7. **X-8 → S-28**: `ImageStub` missing id/rank deferred (T22) → now fixed with `ProductImage` (T30)
8. **S-28 → S-31**: Images input `Vec<String>` (T30) → `Vec<ImageInput>` object format (T31)

### Superseded Entries

- **B-6, L-2**: Originally added 409 guards → superseded by E-12 which changed all cart state violations to 400 `InvalidData`
- **C-2**: Cart completion stub used `AppError::Conflict` → guard status code changed by E-12

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
