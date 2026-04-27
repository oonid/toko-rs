# Task 26: Eleventh Audit — P1 Medusa Compatibility Deep Audit (Post-108 Fixes)

**Source**: Comprehensive 6-dimension audit against `vendor/medusa/` at develop branch.
**Reconciled against**: `docs/audit-master-checklist.md` (108 prior fixes confirmed).
**Date**: 2026-04-26
**Status**: Findings identified, pending implementation.

---

## Audit Methodology

Six parallel audit streams compared all 25 P1 endpoints against Medusa vendor source:

1. **Route signatures**: HTTP method + path + response wrapper for all 25 endpoints against Medusa route files (`packages/medusa/src/api/{admin,store}/*/route.ts`)
2. **Response shapes**: Field-by-field comparison of all entity models against Medusa query-config files (`query-config.ts`, `helpers.ts`, MikroORM models)
3. **Input types & validation**: All 10 input types compared against Medusa Zod validators (field presence, type, constraints, strictness)
4. **DB schema**: All 12 tables (PG + SQLite migrations) compared against Medusa MikroORM model definitions (`packages/modules/*/src/models/`)
5. **Error handling**: Error variant → HTTP status → `type` → `code` mapping against Medusa error handler
6. **Business logic**: Cart completion, line item dedup, product soft-delete, variant creation, customer registration, cart update, order list — compared against Medusa workflows in `packages/core-flows/src/`

All findings cross-referenced against 108-item master checklist to avoid duplicates.

---

## Findings Summary

| Severity | Count |
|----------|-------|
| BUG (correctness) | 3 |
| HIGH (structural gaps) | 4 |
| MEDIUM (functional gaps) | 9 |
| LOW (cosmetic/deferred) | ~20 |
| **Total actionable P1** | **8** |
| P2 / deferred | ~28 |

---

## Actionable P1 Findings

### BUG-1: `UpdateLineItemInput.quantity` rejects 0 — Medusa allows it (regression from T24e/T25b)

**Severity**: BUG
**Files**: `src/cart/types.rs:36`, `src/cart/repository.rs:293-297`
**Medusa**: `vendor/medusa/packages/medusa/src/api/store/carts/validators.ts:83` — `z.number().gte(0)` with comment "can be 0 to remove item from cart"

T24e changed `UpdateLineItemInput.quantity` from accepting 0 to `range(min = 1)`. T25b then removed the `if input.quantity == 0 { return self.delete_line_item(...) }` branch as "dead code." However, **Medusa explicitly allows quantity=0** as a removal signal — the validator is `gte(0)` not `gt(0)`.

This is a regression: Medusa SDK clients that send `quantity: 0` to remove items will get 400 errors instead of deletion.

**Fix**:
1. Revert `range(min = 1)` back to `range(min = 0)` on `UpdateLineItemInput.quantity`
2. Restore the `if input.quantity == 0 { return self.delete_line_item(...) }` branch
3. Update `test_cart_full_flow` to use quantity=0 again instead of DELETE

---

### BUG-2: `CreateCustomerInput.email` is required — Medusa has it optional

**Severity**: BUG
**Files**: `src/customer/types.rs:8-14`
**Medusa**: `vendor/medusa/packages/medusa/src/api/store/customers/validators.ts:7` — `email: z.string().email().nullish()`

toko-rs declares `email: String` (required), but Medusa's `StoreCreateCustomer` has `email: z.string().email().nullish()` — explicitly optional and nullable. This means Medusa clients can create customers without an email, but toko-rs rejects such requests.

**Fix**: Change `email: String` to `email: Option<String>` in `CreateCustomerInput`. Update repository to handle `None` (set email to NULL in DB; the UNIQUE constraint only applies to non-null, non-deleted emails). Update test that asserts email is required.

---

### BUG-3: `is_giftcard`/`discountable` only accept JSON boolean — Medusa accepts string "true"/"false"

**Severity**: BUG
**Files**: `src/product/types.rs:32-49` (`CreateProductInput`), `src/product/types.rs:71-84` (`UpdateProductInput`)
**Medusa**: `vendor/medusa/packages/medusa/src/api/admin/products/validators.ts` — uses `booleanString()` which accepts both `true`/`false` (boolean) AND `"true"`/`"false"` (string).

Form-encoded requests or legacy clients passing `is_giftcard: "true"` (string) are rejected by toko-rs's `Option<bool>` deserialization. Medusa's `booleanString()` Zod validator explicitly handles this.

**Fix**: Add a custom serde deserializer for `is_giftcard` and `discountable` that accepts both `true`/`false` (bool) and `"true"`/`"false"` (string). Alternatively, use `#[serde(deserialize_with = "deserialize_bool_or_string")]`.

---

### HIGH-1: Variant `options` flat `{id, value, option_id}` vs Medusa nested `{id, value, option: {id, title}}`

**Severity**: HIGH
**Files**: `src/product/models.rs:101-106` (`VariantOptionValue`)
**Medusa**: `vendor/medusa/packages/modules/product/src/models/product-option-value.ts` — has `option` BelongsTo relation loaded by `*variants.options`

Medusa returns:
```json
"options": [{ "id": "optval_...", "value": "Red", "option": { "id": "opt_...", "title": "Color" } }]
```

toko-rs returns:
```json
"options": [{ "id": "optval_...", "value": "Red", "option_id": "opt_..." }]
```

Clients accessing `variant.options[0].option.title` will get `undefined`. This breaks Medusa frontend rendering of variant option labels.

**Fix**: Add `option: Option<NestedOption>` field to `VariantOptionValue` where `NestedOption { id: String, title: String }`. Populate from `load_relations` query which already joins `product_options`.

---

### HIGH-2: Cart `CreateCartInput`/`UpdateCartInput` missing address fields — addresses never settable

**Severity**: HIGH
**Files**: `src/cart/types.rs:7-24`
**Medusa**: `vendor/medusa/packages/medusa/src/api/store/carts/validators.ts:15-29` (CreateCart), `validators.ts:54-66` (UpdateCart)

Medusa accepts `shipping_address` and `billing_address` on both create and update cart. The cart model has `shipping_address`/`billing_address` JSONB columns in the DB, but neither `CreateCartInput` nor `UpdateCartInput` exposes them. Result: orders are always created with NULL addresses.

**Fix**: Add `shipping_address: Option<serde_json::Value>` and `billing_address: Option<serde_json::Value>` to both `CreateCartInput` and `UpdateCartInput`. Write to the existing JSONB columns. Add validation that address values are objects (not arrays/strings).

---

### HIGH-3: No idempotency on cart completion — client retry creates duplicate order

**Severity**: HIGH
**Files**: `src/order/repository.rs:18` (`create_from_cart`)
**Medusa**: `vendor/medusa/packages/core-flows/src/cart/workflows/complete-cart.ts:316-339` — queries `order_cart` link for existing order before creating.

The `SELECT ... FOR UPDATE` prevents concurrent requests but does not protect against retries after commit. If a client times out and retries, a second order is created.

**Fix**: After acquiring the cart lock and before creating the order, query `orders` for an existing order with `id IN (SELECT order_id FROM order_line_items WHERE ...)` or simpler: add a `cart_id` column to `orders` table and check for existing order before creating. Alternatively, use the `_sequences` table to track cart→order mapping.

---

### HIGH-4: Admin variant uses `calculated_price` instead of `prices[]` array

**Severity**: HIGH
**Files**: `src/product/models.rs:94-99` (`CalculatedPrice`)
**Medusa**: `vendor/medusa/packages/medusa/src/api/admin/products/query-config.ts` — `defaultAdminProductFields` includes `*variants.prices`

Medusa admin returns `prices: [{id, amount, currency_code, min_quantity, max_quantity, variant_id, rules}]`. toko-rs returns `calculated_price: {calculated_amount, original_amount, ...}` which is a store-only concept. Admin clients expecting `variant.prices` will fail.

**Fix**: Add `prices: Vec<PriceStub>` to `ProductVariant` (or `ProductVariantWithOptions`) where `PriceStub { id: String, amount: i64, currency_code: String }`. Populate with a single-element array from the variant's `price` column and the cart's/default currency. Keep `calculated_price` for store responses only (or add it conditionally).

---

### MEDIUM-1: Store `calculated_price` missing `currency_code` field

**Severity**: MEDIUM
**Files**: `src/product/models.rs:94-99`
**Medusa**: `vendor/medusa/packages/medusa/src/api/store/products/helpers.ts:97-119`

Medusa's store `calculated_price` includes `currency_code`. toko-rs omits it. Clients that display currency alongside price have no way to know which currency the amount is in.

**Fix**: Add `currency_code: String` to `CalculatedPrice`. Populate from cart or default currency.

---

### MEDIUM-2: Cart/Order use `gift_card_total`/`gift_card_tax_total` instead of `credit_line_*`

**Severity**: MEDIUM
**Files**: `src/cart/models.rs:112-113`, `src/order/models.rs:99-100`
**Medusa**: `vendor/medusa/packages/medusa/src/api/store/carts/query-config.ts` — `credit_line_total`, `credit_line_subtotal`, `credit_line_tax_total`

Medusa uses `credit_line_*` naming. toko-rs uses `gift_card_*`. A Medusa frontend looking for `credit_line_total` gets `undefined`.

**Fix**: Add `credit_line_total: i64`, `credit_line_subtotal: i64`, `credit_line_tax_total: i64` to both `CartWithItems` and `OrderWithItems`. Keep `gift_card_*` as aliases or remove them. All values default to 0 in P1.

---

### MEDIUM-3: No guest-to-registered customer upgrade path

**Severity**: MEDIUM
**Files**: `src/customer/repository.rs:18-46`
**Medusa**: `vendor/medusa/packages/core-flows/src/customer/steps/validate-customer-account-creation.ts:43-61`

Medusa allows upgrading a guest customer (`has_account=false`) to a registered customer when they sign up with the same email. toko-rs rejects any duplicate email with a unique constraint violation. If a customer placed an order as a guest with email X, they cannot register with email X.

**Fix**: Before INSERT, query for existing customer with same email. If found with `has_account=false`, update to `has_account=true` instead of rejecting. If found with `has_account=true`, reject as duplicate.

---

### MEDIUM-4: Variant SKU uniqueness includes soft-deleted variants

**Severity**: MEDIUM
**Files**: `src/product/repository.rs:699-708`
**Medusa**: Unique index `IDX_product_variant_sku_unique` has `WHERE deleted_at IS NULL`

toko-rs's unique index correctly has `WHERE deleted_at IS NULL AND sku IS NOT NULL`, so the DB allows reusing SKUs from soft-deleted variants. However, the error message says "Variant with SKU '...' already exists" without clarifying whether it's from a soft-deleted variant. If the application checks before insert, it might block reuse.

**Fix**: Verify that the unique constraint error path correctly allows soft-deleted SKU reuse (it should since the DB constraint is correct). Improve the error message if it catches a non-deleted duplicate vs a query-level false positive.

---

### MEDIUM-5: Cart missing `discount_subtotal`, `shipping_discount_total`, `original_shipping_discount_total`

**Severity**: MEDIUM
**Files**: `src/cart/models.rs:87-114`
**Medusa**: `vendor/medusa/packages/medusa/src/api/store/carts/query-config.ts`

Medusa's `defaultStoreCartFields` includes `discount_subtotal`, `shipping_discount_total`, `original_shipping_discount_total` in addition to `discount_total` and `discount_tax_total` that toko-rs already has.

**Fix**: Add these 3 fields to `CartWithItems`, initialized to 0.

---

### MEDIUM-6: Order missing `summary` field, `discount_subtotal`, `credit_line_*` fields

**Severity**: MEDIUM
**Files**: `src/order/models.rs:89-120`
**Medusa**: `vendor/medusa/packages/medusa/src/api/store/orders/query-config.ts`

Medusa returns an `order.summary` object with computed totals. Also includes `discount_subtotal` and `credit_line_*` fields that toko-rs doesn't have.

**Fix**: Add `summary: OrderSummary` struct with `subtotal`, `total`, `discount_total`, `shipping_total`, `tax_total` (all defaulting to computed values). Add `discount_subtotal` and `credit_line_*` fields.

---

### MEDIUM-7: `payment_records.provider` column missing index

**Severity**: MEDIUM
**Files**: `migrations/005_payments.sql`, `migrations/sqlite/005_payments.sql`
**Medusa**: `IDX_payment_provider_id` on `provider_id`

Medusa indexes the provider column. toko-rs does not.

**Fix**: Add `CREATE INDEX idx_payment_records_provider ON payment_records (provider)` to both PG and SQLite migrations.

---

### MEDIUM-8: `product_option_values.option_id` NOT NULL — Medusa allows nullable

**Severity**: MEDIUM
**Files**: `migrations/001_products.sql:30`, `migrations/sqlite/001_products.sql:30`
**Medusa**: `vendor/medusa/packages/modules/product/src/models/product-option-value.ts` — `belongsTo ProductOption` with `.nullable()`

Medusa permits orphan option values without an option. toko-rs requires every value to belong to an option. Low practical impact but diverges from Medusa's data model.

**Fix**: Change `option_id TEXT NOT NULL` to `option_id TEXT` (nullable) in both PG and SQLite migrations. Update Rust model accordingly.

---

### MEDIUM-9: `ListOrdersParams` missing `id` and `status` filter fields

**Severity**: MEDIUM
**Files**: `src/order/types.rs:65`
**Medusa**: `vendor/medusa/packages/medusa/src/api/store/orders/validators.ts:13` — `id` and `status` filter fields

Medusa allows filtering orders by ID and status. toko-rs only supports `offset`, `limit`, `order`, and `fields`.

**Fix**: Add `id: Option<String>` and `status: Option<String>` to `ListOrdersParams`. Update repository query to apply filters.

---

## P2 / Deferred Items (Documented)

These are known architectural gaps documented in prior audits or newly identified:

| # | Gap | Reason |
|---|-----|--------|
| P2-1 | Pricing Module (`prices[]` array, multi-currency) | Requires separate pricing module |
| P2-2 | Inventory reservation on order creation | Requires inventory module |
| P2-3 | Shipping validation + shipping methods on orders | Requires shipping module |
| P2-4 | Tax lines/adjustments on order items | Requires tax module |
| P2-5 | Promotion usage registration | Requires promotion module |
| P2-6 | Event emission (order placed, product deleted, etc.) | Requires event system |
| P2-7 | Auth identity linking / session-based auth | Requires auth module |
| P2-8 | Order-cart link for cross-module queries | Requires link module |
| P2-9 | Payment authorization + capture + refund | Requires payment provider integration |
| P2-10 | Guest customer concept (`has_account=false`) | Requires auth module for guest detection |
| P2-11 | Cart refresh/recalculation after mutations (tax, promotions) | Requires tax + promotion modules |
| P2-12 | Region support (`region_id`, currency switching) | Requires region module |
| P2-13 | Sales channel support | Requires sales channel module |
| P2-14 | Product type, collection, tags, categories relations | Requires additional tables |
| P2-15 | Order versioning / edits (`version` column, `OrderItem` split) | Requires order operations module |
| P2-16 | Payment collection architecture (session → collection → payment) | Architectural simplification |
| P2-17 | `customer_id` directly settable on cart (security concern) | Requires auth module |
| P2-18 | Missing 11 product scalar fields (weight, dimensions, hs_code, etc.) | P2 product enrichment |
| P2-19 | Missing 14 variant scalar fields (barcode, ean, upc, manage_inventory, etc.) | P2 product enrichment |
| P2-20 | Addresses as raw JSONB vs Medusa's structured Address table | P2 address refactor |
| P2-21 | Order `custom_display_id`, `region_id`, `locale`, `no_notification` columns | P2 order enrichment |
| P2-22 | Cart `locale`, `sales_channel_id` fields | P2 multi-locale |
| P2-23 | Cart line item `compare_at_unit_price`, nested `product`/`variant` objects | P2 pricing + relations |
| P2-24 | Admin product batch endpoint (`/admin/products/batch`) | P2 batch operations |
| P2-25 | `calculated_price` missing tax-adjusted amount fields | Requires tax module |
| P2-26 | Product options expose extra fields beyond Medusa's `["id", "title"]` | Harmless extension |
| P2-27 | Customer `default_billing_address_id`/`default_shipping_address_id` not in Medusa | Harmless extension |
| P2-28 | `docs/database.md` stale — lists `subtitle`, `is_giftcard`, `discountable`, `company_name` as "Dropped" but they're now present | Doc update |

---

## Already Correct (Verified)

| Area | Status |
|------|--------|
| All 25 route HTTP methods + paths | ✅ Match Medusa exactly |
| All 24 response top-level wrappers | ✅ `{product}`, `{cart}`, `{order}`, etc. |
| Error mapping table (8 variants) | ✅ Correct HTTP/type/code |
| `deny_unknown_fields` placement | ✅ Present exactly where Medusa uses `.strict()` |
| Cart completion `FOR UPDATE` lock | ✅ Prevents concurrent duplicates |
| Cart completed-at guards (8 locations) | ✅ Returns `InvalidData` (400) |
| Line item dedup (variant_id + unit_price + metadata) | ✅ Matches Medusa behavior |
| Product soft-delete cascade (variants + options + option_values + pivot) | ✅ Transactional |
| Idempotent product delete (already-deleted → 200) | ✅ Matches Medusa |
| Variant option coverage validation | ✅ All options must be covered |
| Variant option combination uniqueness | ✅ Checked against DB |
| Cart line item delete response `{id, object, deleted, parent}` | ✅ Matches Medusa |
| Order ownership verification (`GET /store/orders/:id`) | ✅ **Exceeds** Medusa (has TODO) |
| Order list scoped by `customer_id` | ✅ Matches Medusa |
| PG/SQLite migration parity (all constraints, indexes) | ✅ Exact match |
| All 6 status enum CHECK constraints | ✅ Exact match with Medusa enums |
| Order line item ID prefix `"ordli"` | ✅ Fixed in T25a |
| `is_tax_inclusive` reads from snapshot | ✅ Fixed in T25d |
| All CHECK constraints on monetary/quantity columns | ✅ Fixed in T25c |
| Cart→order field copy (metadata, addresses, line item metadata) | ✅ Fixed in T23b |
| SQL injection protection (ORDER BY whitelist) | ✅ Fixed in T23a |

---

## Implementation Checklist

### 26a. Revert quantity=0 removal — restore Medusa-compatible behavior (BUG-1)
- [ ] 26a.1 Revert `UpdateLineItemInput.quantity` from `range(min = 1)` back to `range(min = 0)`
- [ ] 26a.2 Restore `if input.quantity == 0 { return self.delete_line_item(...) }` branch in `update_line_item`
- [ ] 26a.3 Update `test_cart_full_flow` to use quantity=0 instead of DELETE
- [ ] 26a.4 Update test `test_cart_update_line_item_quantity_zero_rejected` to verify quantity=0 deletes item
- [ ] 26a.5 Run full test suite

### 26b. Make `CreateCustomerInput.email` optional (BUG-2)
- [ ] 26b.1 Change `email: String` to `email: Option<String>` in `CreateCustomerInput`
- [ ] 26b.2 Update repository `create` to handle `None` (insert NULL)
- [ ] 26b.3 Update email-required test to verify email is now optional
- [ ] 26b.4 Add test: create customer without email succeeds
- [ ] 26b.5 Run full test suite

### 26c. Accept string "true"/"false" for `is_giftcard`/`discountable` (BUG-3)
- [ ] 26c.1 Add `deserialize_bool_or_string` helper in `src/types.rs`
- [ ] 26c.2 Apply to `is_giftcard` and `discountable` on `CreateProductInput` and `UpdateProductInput`
- [ ] 26c.3 Add test: `is_giftcard: "true"` (string) accepted
- [ ] 26c.4 Add test: `is_giftcard: true` (boolean) still works
- [ ] 26c.5 Run full test suite

### 26d. Add nested `option` object to variant options (HIGH-1)
- [ ] 26d.1 Add `NestedOption { id: String, title: String }` struct to `src/product/models.rs`
- [ ] 26d.2 Add `option: Option<NestedOption>` to `VariantOptionValue`
- [ ] 26d.3 Remove flat `option_id` field (or keep as alias)
- [ ] 26d.4 Populate `option` from `load_relations` query (already joins `product_options`)
- [ ] 26d.5 Update contract tests for new shape
- [ ] 26d.6 Run full test suite

### 26e. Add address fields to cart input types (HIGH-2)
- [ ] 26e.1 Add `shipping_address: Option<serde_json::Value>` to `CreateCartInput` and `UpdateCartInput`
- [ ] 26e.2 Add `billing_address: Option<serde_json::Value>` to both types
- [ ] 26e.3 Write address values to existing JSONB columns in repository
- [ ] 26e.4 Add test: create cart with shipping_address
- [ ] 26e.5 Add test: update cart billing_address
- [ ] 26e.6 Verify address preserved on cart→order completion
- [ ] 26e.7 Run full test suite

### 26f. Add cart completion idempotency check (HIGH-3)
- [ ] 26f.1 Add `cart_id TEXT` column to `orders` table in both PG and SQLite migrations
- [ ] 26f.2 Add `cart_id: Option<String>` to `Order` model
- [ ] 26f.3 In `create_from_cart`: after cart lock, query for existing order with `cart_id` before creating
- [ ] 26f.4 If existing order found, return it instead of creating new one
- [ ] 26f.5 Add test: retry cart completion returns same order
- [ ] 26f.6 Run full test suite

### 26g. Add `currency_code` to `CalculatedPrice` (MEDIUM-1)
- [ ] 26g.1 Add `currency_code: String` to `CalculatedPrice` in `src/product/models.rs`
- [ ] 26g.2 Populate from default currency or cart context
- [ ] 26g.3 Update contract tests
- [ ] 26g.4 Run full test suite

### 26h. Add `credit_line_*` fields and missing cart/order totals (MEDIUM-2, MEDIUM-5, MEDIUM-6)
- [ ] 26h.1 Add `credit_line_total`, `credit_line_subtotal`, `credit_line_tax_total` to `CartWithItems` (all default 0)
- [ ] 26h.2 Add `discount_subtotal`, `shipping_discount_total`, `original_shipping_discount_total` to `CartWithItems` (all default 0)
- [ ] 26h.3 Add same `credit_line_*` fields to `OrderWithItems`
- [ ] 26h.4 Add `discount_subtotal` to `OrderWithItems`
- [ ] 26h.5 Update contract tests
- [ ] 26h.6 Run full test suite

### 26i. Add `provider` index on `payment_records` (MEDIUM-7)
- [ ] 26i.1 Add `CREATE INDEX idx_payment_records_provider ON payment_records (provider)` to PG migration
- [ ] 26i.2 Add same to SQLite migration
- [ ] 26i.3 Run full test suite

### 26j. Add `id` and `status` filters to `ListOrdersParams` (MEDIUM-9)
- [ ] 26j.1 Add `id: Option<String>` and `status: Option<String>` to `ListOrdersParams`
- [ ] 26j.2 Update `list_by_customer` repository query to apply filters
- [ ] 26j.3 Add tests: filter orders by status, filter by id
- [ ] 26j.4 Run full test suite

### 26k. Verification pass
- [ ] 26k.1 Run full test suite on SQLite
- [ ] 26k.2 Run full test suite on PostgreSQL
- [ ] 26k.3 Run `cargo clippy -- -D warnings` on both features
- [ ] 26k.4 Run `cargo fmt --check`
- [ ] 26k.5 Update `docs/audit-master-checklist.md`
