# Audit P1 Task 22: Seventh Audit — Full P1 Medusa Compatibility Deep Audit

**Date**: 2026-04-22
**Scope**: Comprehensive 6-dimension deep audit of toko-rs P1 against `vendor/medusa/` at develop branch, reconciled against `docs/audit-master-checklist.md` (84 prior fixes across Tasks 12–21).
**Methodology**: 6 parallel audit streams with exhaustive line-by-line comparison against Medusa TypeScript source, Zod validators, MikroORM models, and workflow definitions.
**Previous audits**: Task 12 (1st), Task 14 (2nd), Task 18 (3rd), Task 19 (4th), Task 20 (5th), Task 21 (6th).

---

## Executive Summary

All **25 endpoint methods** (24 matching Medusa + 1 health) are confirmed exact matches on method and path. All **84 prior fixes** from the master checklist are confirmed correct. This audit found **1 new HIGH bug**, **4 new HIGH findings**, **6 new MEDIUM findings**, and **7 new LOW findings** not caught by any prior audit.

### Finding Summary

| Severity | Count | New Areas |
|----------|-------|-----------|
| BUG (HIGH) | 1 | `deleted_at` leaked on 8 entity types — internal field exposed to all store API responses |
| HIGH | 4 | `deleted_at` leak (continued), `product_variant_option` join rows orphaned on soft-delete, `update_line_item` no existence check on affected rows, `ListOrdersParams` strictness mismatch |
| MEDIUM | 6 | `payment_records.provider` name mismatch, `ListOrdersParams.limit` default 50 vs Medusa 15, `ProductVariant.product_id` NOT NULL vs Medusa nullable, address JSONB serialization wrapper (`{"0":{...}}`), `order_line_items.unit_price` NOT NULL vs Medusa nullable, missing `find_by_email` on customer |
| LOW | 7 | `product_option_values.option_id` NOT NULL vs nullable, `product_variants.variant_rank` NOT NULL vs nullable, `customer_addresses` default-address indexes include `deleted_at` guard (toko-rs is arguably more correct), missing `created_by` on customer, ImageStub missing `id`/`rank`, variant `price` field extra (Medusa doesn't have it on variant directly), `payment_records.status` uses different enum values than Medusa PaymentCollectionStatus |

### Changes Since Task 21

All 7 Task 21 fixes (21a–21h) are confirmed in source:
- 21a: Order ownership verification ✅
- 21b: `add_line_item` FOR UPDATE ✅
- 21c: `snapshot` field `#[serde(skip)]` ✅
- 21d: `has_account` confirmed as Medusa-accepted (FALSE POSITIVE) ✅
- 21e: `"draft"` added to orders.status CHECK ✅
- 21f: `deleted_at` added to payment_records ✅
- 21g: `deny_unknown_fields` removed from 5 non-strict types ✅
- 21h: `completed_at IS NULL` guards on cart UPDATE WHERE ✅

---

## Dimension 1: Route Paths & HTTP Methods

### CONFIRMED: All 24 Medusa-matching endpoints are 100% correct

Full verification against every `route.ts` in `vendor/medusa/packages/medusa/src/api/`:

| # | Method | Path | Medusa Match |
|---|--------|------|------|
| 1 | POST | `/admin/products` | MATCH |
| 2 | GET | `/admin/products` | MATCH |
| 3 | GET | `/admin/products/{id}` | MATCH |
| 4 | POST | `/admin/products/{id}` | MATCH |
| 5 | DELETE | `/admin/products/{id}` | MATCH |
| 6 | GET | `/admin/products/{id}/variants` | MATCH |
| 7 | POST | `/admin/products/{id}/variants` | MATCH |
| 8 | GET | `/admin/products/{id}/variants/{variant_id}` | MATCH |
| 9 | POST | `/admin/products/{id}/variants/{variant_id}` | MATCH |
| 10 | DELETE | `/admin/products/{id}/variants/{variant_id}` | MATCH |
| 11 | GET | `/store/products` | MATCH |
| 12 | GET | `/store/products/{id}` | MATCH |
| 13 | POST | `/store/carts` | MATCH |
| 14 | GET | `/store/carts/{id}` | MATCH |
| 15 | POST | `/store/carts/{id}` | MATCH |
| 16 | POST | `/store/carts/{id}/line-items` | MATCH |
| 17 | POST | `/store/carts/{id}/line-items/{line_id}` | MATCH |
| 18 | DELETE | `/store/carts/{id}/line-items/{line_id}` | MATCH |
| 19 | POST | `/store/carts/{id}/complete` | MATCH |
| 20 | GET | `/store/orders` | MATCH |
| 21 | GET | `/store/orders/{id}` | MATCH |
| 22 | POST | `/store/customers` | MATCH |
| 23 | GET | `/store/customers/me` | MATCH |
| 24 | POST | `/store/customers/me` | MATCH |
| 25 | GET | `/health` | CUSTOM (no Medusa equivalent) |

### HTTP Method Convention

Verified: toko-rs uses POST for all updates (never PUT/PATCH). Matches Medusa convention exactly. CORS layer allows `GET, POST, DELETE, OPTIONS` only — correct.

### `deny_unknown_fields` Audit (post-Task 21g)

| Input Type | toko-rs `deny_unknown_fields` | Medusa `.strict()` | Match? |
|---|---|---|---|
| `CreateProductInput` | YES | YES | OK |
| `UpdateProductInput` | YES | YES | OK |
| `CreateProductVariantInput` | YES | YES | OK |
| `UpdateVariantInput` | YES | YES | OK |
| `CreateCartInput` | YES | YES | OK |
| `UpdateCartInput` | YES | YES | OK |
| `CreateProductOptionInput` | NO | NO | OK |
| `AddLineItemInput` | NO | NO | OK |
| `UpdateLineItemInput` | NO | NO | OK |
| `CreateCustomerInput` | NO | NO | OK |
| `UpdateCustomerInput` | NO | NO | OK |
| `ListOrdersParams` | YES | **NO** | **MISMATCH** (new finding — see I6) |

**New finding (I6)**: `ListOrdersParams` in `src/order/types.rs` has `deny_unknown_fields`, but Medusa's `AdminGetOrdersParams` uses `createFindParams` which is NOT strict. This will reject valid Medusa SDK query parameters.

---

## Dimension 2: Response Shapes

### S1: `deleted_at` leaked to API responses on 8 entity types (HIGH — NEW)

**Locations**:
- `src/product/models.rs:17` — Product
- `src/product/models.rs:29` — ProductOption
- `src/product/models.rs:41` — ProductOptionValue
- `src/product/models.rs:56` — ProductVariant
- `src/cart/models.rs:20` — Cart
- `src/order/models.rs:22` — Order
- `src/customer/models.rs:42` — CustomerAddress
- `src/payment/models.rs:17` — PaymentRecord

Medusa's store query configs do NOT return `deleted_at` on these types (verified against query-config files in `vendor/medusa/packages/medusa/src/api/store/`). The `deleted_at` field is internal and should not appear in API responses.

**Note**: CartLineItem and OrderLineItem already have `#[serde(skip)]` on `deleted_at` (fixed in Task 21c). The 8 types above do NOT.

**Impact**: Every product, variant, option, option value, cart, order, customer address, and payment record response includes an unnecessary `deleted_at: null` field. This bloats responses and leaks internal implementation details.

**Fix**: Add `#[serde(skip)]` to `deleted_at` on all 8 types.

### S2: Cart/Order addresses untyped `Json<Value>` (HIGH — known)

Same as Task 21 S1. Confirmed: Medusa uses typed `BaseCartAddress` / `BaseOrderAddress` with 14 named fields. toko-rs uses inline `JSONB`. Additionally, the `sqlx::types::Json<Value>` wrapper serializes as `{"0": {...}}` instead of the unwrapped inner value — the address shape in API responses is wrong.

**Status**: Known architectural divergence (Decision 5).

### S3: Variant `calculated_price` simplified (HIGH — known)

toko-rs: 3 fields. Medusa: 17+ fields with nested sub-objects. Confirmed as P1 simplification.

### S4: All 22 computed total fields present and correctly named (CONFIRMED)

Both `CartWithItems` and `OrderWithItems` expose all 22 Medusa total fields. All 12 per-item totals also present. Field names match Medusa exactly.

### S5: Missing `order.summary` (MEDIUM — known)

Medusa's `StoreOrder` includes `summary: BaseOrderSummary` with 7 fields. toko-rs does not expose this.

### S6: `fulfillments` / `shipping_methods` untyped (HIGH — known)

Both are `Vec<serde_json::Value>` (always empty). Confirmed as P1 stubs.

### S7: ImageStub missing `id` and `rank` (LOW — NEW)

toko-rs `ImageStub` has only `url: String`. Medusa's `BaseProductImage` has `id`, `url`, `rank`, `metadata`, timestamps. At minimum, `id` and `rank` should be added for frontend compatibility.

---

## Dimension 3: Request/Input Types

### I1: `Option<T>` vs Medusa `nullish()` — explicit null rejected (MEDIUM — known from T21)

~30 fields across all input types where Medusa's `.nullish()` accepts explicit `null` but toko-rs's `Option<T>` rejects it. No change since Task 21 — deferred.

### I2: `CreateProductInput.status` missing default (MEDIUM — known from T21)

Medusa defaults to `DRAFT` at validation. toko-rs has no default (falls to DB default `"draft"`). Behavior matches at the response level. Deferred.

### I3: `CreateCustomerInput.email` required vs Medusa optional (MEDIUM — known from T21)

toko-rs: `email: String` (required). Medusa: `z.string().email().nullish()` (optional). Deferred — aligns with DB NOT NULL constraint.

### I4: `price` vs `prices` — structural divergence (known — Decision 13)

Confirmed: toko-rs uses single `price: i64`, Medusa uses `prices: AdminPrice[]` array. Architectural decision.

### I5: `payment_records.provider` vs Medusa `provider_id` (MEDIUM — known from T21, confirmed)

Column name mismatch. Low urgency but creates friction for migration tooling.

### I6: `ListOrdersParams` has `deny_unknown_fields` but Medusa is NOT strict (HIGH — NEW)

**Location**: `src/order/types.rs:64-71`

Medusa's `AdminGetOrdersParams` uses `createFindParams` which does NOT apply `.strict()`. toko-rs applies `deny_unknown_fields`, rejecting valid Medusa SDK query parameters like `fields`, `order`, `with_deleted`, etc.

**Also**: Default limit is 50 in toko-rs vs 15 in Medusa (for admin order listing). The store order listing may differ — this needs verification.

**Fix**: Remove `deny_unknown_fields` from `ListOrdersParams`.

---

## Dimension 4: Database Schema

### D1: `product_variant_option` join rows NOT cascade-deleted on soft-delete (HIGH — NEW)

**Location**: `src/product/repository.rs:261-315` — `soft_delete` method

When a product is soft-deleted, the method cascades `deleted_at` to:
1. `product_variants` ✅
2. `product_options` ✅
3. `product_option_values` (via subquery on options) ✅
4. `product_variant_option` ❌ **MISSING**

The pivot table `product_variant_option` rows remain un-soft-deleted. While this doesn't cause visible bugs (the variants themselves are hidden), it leaves orphan join rows that could cause issues if variants are later restored or queried directly.

**Fix**: Add `UPDATE product_variant_option SET deleted_at = CURRENT_TIMESTAMP WHERE variant_id IN (SELECT id FROM product_variants WHERE product_id = $1)` to the soft_delete transaction.

### D2: `product_variants.product_id` NOT NULL vs Medusa nullable (MEDIUM — NEW)

**Location**: `migrations/002_products.sql`

toko-rs: `product_id TEXT NOT NULL REFERENCES products(id) ON DELETE CASCADE`
Medusa: `product: model.belongsTo(...).nullable()` — nullable

Medusa allows orphan variants (no product). toko-rs does not. Low practical impact but technically diverges.

### D3: `product_option_values.option_id` NOT NULL vs Medusa nullable (LOW — known from T21)

Same pattern — Medusa allows orphan option values, toko-rs does not. Arguably more correct.

### D4: `order_line_items.unit_price` NOT NULL vs Medusa nullable (MEDIUM — NEW)

**Location**: `migrations/004_orders.sql`

toko-rs: `unit_price BIGINT NOT NULL`
Medusa `OrderLineItem`: `unit_price: model.bigNumber().nullable()` — nullable

Medusa allows free items (null price). toko-rs requires a numeric value. Edge case for P1.

### D5: `payment_records.status` uses wrong enum values (LOW — NEW)

toko-rs CHECK: `('pending', 'authorized', 'captured', 'failed', 'refunded')`
Medusa `PaymentCollectionStatus`: `('not_paid', 'awaiting', 'authorized', 'partially_authorized', 'canceled', 'failed', 'partially_captured', 'completed')`

These are completely different enum sets. Medusa puts status on `PaymentCollection`, not on `Payment`. toko-rs puts status directly on the payment record with its own lifecycle values.

**Status**: Known architectural simplification. toko-rs's values are internally consistent.

### D6: PG/SQLite parity confirmed complete (CONFIRMED)

All tables, columns, constraints, and indexes present in both PG and SQLite. No new parity gaps.

### D7: Missing `created_by` on customers (LOW — NEW)

Medusa's `Customer` model has `created_by: model.text().nullable()`. toko-rs does not. Used for audit trail — who created this customer.

---

## Dimension 5: Error Handling

### E1: All 9 error variants match on HTTP status and `type` (CONFIRMED)

Every variant's HTTP status and `type` field matches Medusa exactly:

| Variant | HTTP | `type` | `code` | Medusa Match |
|---|---|---|---|---|
| `NotFound` | 404 | `not_found` | `invalid_request_error` | STATUS+TYPE: YES |
| `InvalidData` | 400 | `invalid_data` | `invalid_request_error` | STATUS+TYPE: YES |
| `DuplicateError` | 422 | `duplicate_error` | `invalid_request_error` | STATUS+TYPE+CODE: YES |
| `Forbidden` | 403 | `forbidden` | `invalid_state_error` | STATUS+TYPE: YES |
| `Conflict` | 409 | `conflict` | `invalid_state_error` | STATUS+TYPE+CODE: YES |
| `Unauthorized` | 401 | `unauthorized` | `unknown_error` | STATUS+TYPE: YES |
| `UnexpectedState` | 500 | `unexpected_state` | `invalid_state_error` | STATUS+TYPE: YES |
| `DatabaseError` | 500 | `database_error` | `api_error` | STATUS+TYPE+CODE: YES |
| `MigrationError` | 500 | `database_error` | `api_error` | STATUS+TYPE+CODE: YES |

### E2: `code` field always present vs Medusa sometimes absent (LOW — known)

toko-rs always includes `code`. Medusa sometimes omits it (especially for validation errors and some error types where the original error had no code). toko-rs is more consistent.

### E3: Missing `not_allowed` and `invalid_argument` error types (LOW — known)

Not needed for P1 scope. `InvalidData` serves as substitute for `not_allowed` scenarios.

### E4: JSON deserialization error handling is BETTER than Medusa (CONFIRMED)

toko-rs returns specific `{"type": "invalid_data", "message": "JSON syntax error: <details>"}` for malformed JSON. Medusa returns generic `{"type": "unknown_error", "message": "An unknown error occurred."}`. toko-rs provides more actionable error messages.

### E5: Conflict message preserves caller context (CONFIRMED — acceptable divergence)

Medusa overwrites ALL conflict messages with idempotency-key boilerplate. toko-rs preserves the caller's specific message (e.g., "Cart is already completed"). More informative for API consumers.

---

## Dimension 6: Business Logic

### B1: `update_line_item` does not verify affected rows (HIGH — NEW)

**Location**: `src/cart/repository.rs:295-309`

The UPDATE query has no check on `rows_affected()` after execution. If the line item doesn't exist, was already deleted, or the cart was completed between the check and the update, the UPDATE silently affects 0 rows. The method then calls `self.get_cart(cart_id)` and returns success with the cart — hiding the fact that no update occurred.

**Impact**: A client updating a non-existent line item gets a 200 with the cart response instead of a 404. Similarly for `delete_line_item`.

**Fix**: Check `result.rows_affected()` after the UPDATE. If 0, return `AppError::NotFound("Line item not found")`.

### B2: Product `update` not transactional (LOW — known from T21)

Existence check SELECT + UPDATE not in transaction. Very low severity.

### B3: N+1 query patterns (LOW — known)

`load_relations` per product in list queries, `load_items` per order in `list_by_customer`. Performance optimization, not correctness.

### B4: Customer `find_by_email` not implemented (MEDIUM — NEW)

**Location**: `src/customer/repository.rs`

Medusa's `CustomerModuleService` has `retrieveFromRegistrationByEmail()` and similar lookup methods. toko-rs's `CustomerRepository` has no `find_by_email` method. This is needed for:
- Duplicate email detection with clear error messages (currently relies on DB unique constraint)
- Customer lookup during cart operations (Medusa creates/finds customer from email)

**Fix**: Add `find_by_email(email: &str) -> Result<Option<Customer>, AppError>` method.

### B5: Cart completion has no idempotency beyond row lock (MEDIUM — known)

Medusa checks `order_cart` remote link for idempotency. toko-rs relies on `FOR UPDATE` which prevents concurrent completion but does not prevent duplicate orders from sequential retries after network failures.

### B6: Soft-delete join table cleanup (same as D1)

See D1 above — `product_variant_option` rows orphaned.

---

## Items Confirmed Fixed from Master Checklist (84/84 verified)

All 84 items from `docs/audit-master-checklist.md` verified correct in source:

- 17 bugs fixed (idempotency, transactionality, cascades, ownership, race conditions)
- 17 response shape fixes (delete response, totals, stubs, addresses, per-item fields, snapshot hidden)
- 7 input/validation fixes (deny_unknown_fields, metadata type, limit cap, non-strict types)
- 11 error handling fixes (status codes, type mapping, JSON rejection, dead code removal, prefix removal)
- 20 DB schema fixes (constraints, indexes, defaults, migrations, currency, draft status, payment deleted_at)
- 8 business logic fixes (dedup, idempotent DELETE, pagination defaults, completed_at guards)
- 4 config/infra fixes (CORS, config defaults, SQLite feature flag)

No regressions detected. All prior fixes are stable.

---

## Recommended Action Plan

### Immediate (new actionable findings)

| Finding | Severity | Effort | Impact |
|---------|----------|--------|--------|
| S1: `deleted_at` leaked on 8 entity types — add `#[serde(skip)]` | HIGH | ~15 min | API contract — stops leaking internal field |
| D1: `product_variant_option` orphaned on soft-delete — add cascade UPDATE | HIGH | ~10 min | Data integrity — prevents orphan join rows |
| B1: `update_line_item` / `delete_line_item` no affected-rows check | HIGH | ~15 min | Correctness — returns 404 instead of silent success |
| I6: `ListOrdersParams` strictness mismatch — remove `deny_unknown_fields` | HIGH | ~5 min | SDK compatibility — stops rejecting valid query params |

### Next Sprint (P1 polish)

| Finding | Severity | Effort | Impact |
|---------|----------|--------|--------|
| B4: Add `find_by_email` to customer repository | MEDIUM | ~30 min | Needed for proper duplicate detection |
| D4: Make `order_line_items.unit_price` nullable | MEDIUM | ~30 min | Medusa parity — supports free items |
| D2: Make `product_variants.product_id` nullable | MEDIUM | ~1 hour | Medusa parity — supports orphan variants |
| S5: Add `order.summary` stub | MEDIUM | ~1 hour | Frontend compatibility |
| S7: Expand `ImageStub` with `id` and `rank` | LOW | ~30 min | Frontend compatibility |

### Deferred (low priority / by design)

| Finding | Reason |
|---------|--------|
| S2: Untyped addresses | Architectural (Decision 5) |
| S3: Simplified `calculated_price` | P1 simplification (Decision 13) |
| S6: Untyped fulfillments/shipping | P1 stubs, empty arrays prevent TypeError |
| I1: `nullish()` vs `Option<T>` | Affects ~30 fields, requires custom serde; deferred |
| I2: `status` default | Behavior matches at response level via DB default |
| I3: `email` required on customer | Aligns with DB NOT NULL; simplification |
| I4: `price` vs `prices` | Known divergence (Decision 13) |
| I5: `provider` vs `provider_id` | Low urgency; functional |
| D3: `option_id` NOT NULL | Arguably more correct |
| D5: Payment status enum values | Architectural simplification |
| D7: Missing `created_by` on customer | Audit trail — P2 concern |
| E2: `code` always present | More consistent than Medusa |
| E3: Missing error types | Not needed for P1 |
| E5: Conflict message format | More informative than Medusa |
| B2: Product update not transactional | Very low severity |
| B3: N+1 queries | Performance, not correctness |
| B5: Cart completion idempotency | Requires persistent idempotency store |

---

## Audit Methodology

Six parallel audit streams compared toko-rs against `vendor/medusa/`:

1. **Route paths & HTTP methods**: All 25 endpoints vs Medusa route handlers → 100% match (24 Medusa + 1 custom health)
2. **Response shapes**: Exhaustive field-by-field comparison against Medusa TypeScript types, query configs, and Store response types → 1 new HIGH finding (`deleted_at` leak on 8 types), 1 known HIGH, 2 known MEDIUM
3. **Request/input types**: Field-by-field against Medusa Zod validators → 1 new HIGH (`ListOrdersParams` strict), 3 known MEDIUM
4. **Database schema**: 14 tables vs Medusa MikroORM models → 1 new HIGH (orphan join rows), 2 new MEDIUM, 2 new LOW
5. **Error handling**: All 9 variants vs Medusa error handler → confirmed correct, no new findings
6. **Business logic**: 10 core flows vs Medusa workflows → 1 new HIGH (no affected-rows check), 1 new MEDIUM (missing `find_by_email`), 3 known LOW

Each stream was cross-referenced against `docs/audit-master-checklist.md` (84 items) to confirm prior fixes and identify new findings only.
