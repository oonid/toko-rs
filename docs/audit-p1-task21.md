# Audit P1 Task 21: Sixth Audit — P1 Medusa Compatibility Verification

**Date**: 2026-04-22
**Scope**: Full 6-dimension audit of toko-rs P1 against `vendor/medusa/` at develop branch, reconciled against `docs/audit-master-checklist.md` (77 prior fixes).
**Methodology**: 6 parallel audit streams, each comparing toko-rs source line-by-line against Medusa TypeScript source.
**Previous audits**: Task 12 (first), Task 14 (second), Task 18 (third), Task 19 (fourth), Task 20 (fifth).

---

## Executive Summary

All **24 P1 endpoints** match Medusa on method and path. All **77 prior fixes** from the master checklist are confirmed correct in the codebase. This audit found **2 new MEDIUM bugs**, **6 new HIGH findings**, **9 MEDIUM findings**, and **5 LOW findings** not caught by previous audits.

### Finding Summary

| Severity | Count | New Areas |
|----------|-------|-----------|
| BUG (MEDIUM) | 2 | Order ownership bypass, cart add_line_item race condition |
| HIGH | 6 | Untyped addresses, untyped fulfillments/shipping, simplified calculated_price, `snapshot`/`has_account` leaked, `orders.status` missing "draft" CHECK, `payment_records` missing `deleted_at` |
| MEDIUM | 9 | `deny_unknown_fields` over-strict on 5 types, `Option` vs `nullish` on 30+ fields, `CreateProductInput.status` missing default, conflict message override, missing `order.detail`/`order.summary`, DB constraint message specificity, `payment_records.provider` vs `provider_id`, cart update/delete missing FOR UPDATE, `customer.email` nullable mismatch |
| LOW | 5 | Extra `deleted_at` on store responses, N+1 queries, product update not transactional, seed not transactional, `variant_rank` nullable mismatch |

---

## Dimension 1: Route Paths & HTTP Methods

### CONFIRMED: All 24 endpoints match exactly

| # | Method | Path | Match |
|---|--------|------|-------|
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

31 additional Medusa endpoints within scope are intentionally missing (P2: batch ops, inventory, options CRUD, addresses, transfers, promotions, taxes, shipping, product-variants).

---

## Dimension 2: Response Shapes

### S1: Cart/Order addresses are untyped `Json<Value>` blobs (HIGH)

**Location**: `src/cart/models.rs:12-14`, `src/order/models.rs:13-16`

Medusa returns typed `StoreCartAddress` / `StoreOrderAddress` objects with 14+ named fields (`first_name`, `last_name`, `address_1`, `city`, `country_code`, etc.). toko-rs returns opaque JSON — consumers cannot rely on field names.

**Status**: Known architectural divergence (Decision 5 in design.md — inline JSONB vs separate table). Not a P1 regression but limits storefront compatibility.

### S2: Order `fulfillments` and `shipping_methods` are untyped (HIGH)

**Location**: `src/order/models.rs:116-117`

Both are `Vec<serde_json::Value>` (always empty `vec![]`). Medusa returns typed `StoreOrderFulfillment[]` and `StoreOrderShippingMethod[]` with 15-20+ fields each.

**Status**: P1 stub — documented. Empty arrays prevent TypeError on `.map()` / `.length`.

### S3: Variant `calculated_price` severely simplified (HIGH)

**Location**: `src/product/models.rs:91-95`

toko-rs: 3 fields (`calculated_amount`, `original_amount`, `is_calculated_price_tax_inclusive`).
Medusa: 17+ fields including `id`, `currency_code`, nested `calculated_price{}` / `original_price{}` sub-objects, tax variants.

**Status**: P1 simplification — documented. The 3 core fields prevent `undefined` crashes.

### S4: Internal `snapshot` field leaked to API responses (HIGH — NEW)

**Location**: `src/cart/models.rs:33`, `src/order/models.rs:35`

Both `CartLineItem` and `OrderLineItem` expose `pub snapshot: Option<sqlx::types::Json<serde_json::Value>>` without `#[serde(skip)]`. This internal DB field leaks into every cart and order response. Medusa does not have a `snapshot` field — it uses individual columns.

**Impact**: Clients see an unexpected `snapshot` key containing raw JSON. Information disclosure of internal field structure.

**Fix**: Add `#[serde(skip)]` to `snapshot` on both models.

### S5: `has_account` on Customer store response — admin-only field leaked (HIGH — NEW)

**Location**: `src/customer/models.rs:13`

`pub has_account: bool` is present on `Customer`, which is flattened into `CustomerWithAddresses` via `#[serde(flatten)]`. Medusa's `StoreCustomer` does NOT include `has_account` — it only appears on `AdminCustomer`.

**Fix**: Either skip the field with `#[serde(skip)]` on the store response, or remove it from the flattened struct.

### S6: All 22 computed total fields present and correctly named

Cart (`CartWithItems`) and Order (`OrderWithItems`) both have all 22 total fields: `item_total`, `item_subtotal`, `item_tax_total`, `total`, `subtotal`, `tax_total`, `discount_total`, `discount_tax_total`, `shipping_total`, `shipping_subtotal`, `shipping_tax_total`, `original_total`, `original_subtotal`, `original_tax_total`, `original_item_total`, `original_item_subtotal`, `original_item_tax_total`, `original_shipping_total`, `original_shipping_subtotal`, `original_shipping_tax_total`, `gift_card_total`, `gift_card_tax_total`.

All 12 per-item totals also present on line items.

### S7: Missing `order.summary` and `order_line_item.detail` (MEDIUM)

Medusa's `StoreOrder` includes `summary: BaseOrderSummary` with 7 fields (`pending_difference`, `paid_total`, `refunded_total`, etc.) and each line item has `detail: BaseOrderItemDetail` with fulfillment quantities (`fulfilled_quantity`, `shipped_quantity`, `delivered_quantity`).

**Status**: P1-borderline — summary is important for payment reconciliation but depends on fulfillment tracking.

---

## Dimension 3: Request/Input Types

### I1: `deny_unknown_fields` over-strict on 5 types (MEDIUM — NEW)

These Medusa validators do NOT use `.strict()`, but toko-rs applies `deny_unknown_fields`:

| toko-rs Type | Medusa Equiv | Medusa `.strict()`? | Issue |
|---|---|---|---|
| `CreateProductOptionInput` | `CreateProductOption` | **NO** | Rejects fields Medusa accepts |
| `AddLineItemInput` | `StoreAddCartLineItem` | **NO** | Rejects fields Medusa accepts |
| `UpdateLineItemInput` | `StoreUpdateCartLineItem` | **NO** | Rejects fields Medusa accepts |
| `CreateCustomerInput` | `StoreCreateCustomer` | **NO** | Rejects fields Medusa accepts |
| `UpdateCustomerInput` | `StoreUpdateCustomer` | **NO** | Rejects fields Medusa accepts |

**Impact**: Medusa SDK clients sending extra fields (e.g., `additional_data`) get 422 errors.

**Fix**: Remove `deny_unknown_fields` from these 5 types, or accept all known Medusa fields with `Option<T>`.

### I2: `Option<T>` vs Medusa `nullish()` — cannot set explicit null (MEDIUM — NEW)

Medusa's `nullish()` allows both "absent" and "explicitly null". toko-rs's `Option<T>` with serde defaults treats explicit `null` as a deserialization error. A Medusa client sending `"subtitle": null` will be rejected.

**Affected**: ~30 fields across all input types.

**Impact**: Medusa SDK clients that explicitly set fields to `null` to clear them will get 400 errors.

### I3: `CreateProductInput.status` missing default (MEDIUM — NEW)

Medusa: `statusEnum.nullish().default(ProductStatus.DRAFT)` — defaults to `"draft"`.
toko-rs: `status: Option<ProductStatus>` — no default, falls to DB default `"draft"`.

**Impact**: The response returns `status: "draft"` correctly because the DB default is set, but the *behavior path* differs — Medusa sets it at validation, toko-rs at DB insert.

### I4: `price` (scalar i64) vs `prices` (array) — structural divergence

Confirmed as known P1 simplification (Decision 13). toko-rs uses single `price: i64`; Medusa uses `prices: AdminPrice[]` array for multi-currency. Not a new finding.

### I5: `CreateCustomerInput.email` required vs Medusa optional (MEDIUM — NEW)

toko-rs: `email: String` (required).
Medusa: `email: z.string().email().nullish()` (optional).

A valid Medusa request without email will be rejected by toko-rs. This blocks guest registration without email.

---

## Dimension 4: Database Schema

### D1: `orders.status` CHECK constraint missing "draft" (HIGH — NEW)

**Location**: `migrations/004_orders.sql`, `migrations/sqlite/004_orders.sql`

Medusa's `OrderStatus` includes `"draft"`. toko-rs CHECK: `('pending','completed','canceled','requires_action','archived')` — missing `'draft'`.

**Fix**: Add `'draft'` to both PG and SQLite CHECK constraints.

### D2: `payment_records` missing `deleted_at` column (HIGH — NEW)

**Location**: `migrations/005_payments.sql`, `migrations/sqlite/005_payments.sql`

Every other table in toko-rs has `deleted_at TIMESTAMPTZ` for soft-delete support. `payment_records` does not. Medusa's `Payment` model also supports soft-delete.

**Fix**: Add `deleted_at TIMESTAMPTZ` (PG) / `deleted_at DATETIME` (SQLite) to payment_records.

### D3: `payment_records.provider` vs Medusa `provider_id` (MEDIUM — NEW)

Column name differs from Medusa's `provider_id`. Low urgency but creates friction for any migration tooling.

### D4: `customers.email` NOT NULL vs Medusa nullable (MEDIUM — NEW)

Medusa's `email` is `nullable()` — supports guest customers without email. toko-rs is `NOT NULL`. Blocks guest checkout where email is not provided.

**Status**: Known P1 simplification. The DB constraint aligns with the required `email` in `CreateCustomerInput`.

### D5: `product_option_values.option_id` NOT NULL vs Medusa nullable

Medusa's belongsTo is `.nullable()`. toko-rs is NOT NULL. Arguably more correct (every value belongs to an option), but technically diverges.

**Status**: Low — no practical impact.

### D6: `product_variants.variant_rank` NOT NULL vs Medusa nullable

Medusa: `model.number().default(0).nullable()`. toko-rs: `NOT NULL DEFAULT 0`.

**Status**: Low — functionally identical.

### D7: PG/SQLite parity confirmed complete

All tables, columns, constraints, and indexes are present in both PG and SQLite variants. No remaining gaps.

---

## Dimension 5: Error Handling

### E1: All 9 error variants match Medusa on HTTP status and `type` (CONFIRMED)

| Variant | HTTP | `type` | `code` | Medusa Match |
|---|---|---|---|---|
| `NotFound` | 404 | `not_found` | `invalid_request_error` | MATCH |
| `InvalidData` | 400 | `invalid_data` | `invalid_request_error` | MATCH |
| `DuplicateError` | 422 | `duplicate_error` | `invalid_request_error` | MATCH |
| `Forbidden` | 403 | `forbidden` | `invalid_state_error` | MATCH |
| `Conflict` | 409 | `conflict` | `invalid_state_error` | MATCH |
| `Unauthorized` | 401 | `unauthorized` | `unknown_error` | MATCH |
| `UnexpectedState` | 500 | `unexpected_state` | `invalid_state_error` | MATCH |
| `DatabaseError` | 500 | `database_error` | `api_error` | MATCH |
| `MigrationError` | 500 | `database_error` | `api_error` | MATCH |

### E2: Conflict message override differs from Medusa (MEDIUM — NEW)

Medusa's error handler forcibly replaces ALL `conflict` type messages with: `"The request conflicted with another request. You may retry the request with the provided Idempotency-Key."`

toko-rs passes through caller-supplied messages (e.g., `"Cannot add items to a completed cart"`). More informative but diverges from Medusa's exact text.

### E3: Missing error types in toko-rs (LOW)

toko-rs has no equivalents for:
- `not_allowed` (400) — Medusa uses for permission-denied scenarios
- `payment_authorization_error` (422) — P2 when payment providers added
- `invalid_argument` (500) — internal programming errors

**Status**: Not needed in P1. `InvalidData` serves as substitute.

### E4: DB constraint messages generic vs Medusa's contextual (MEDIUM — NEW)

Medusa's `formatException` parses PG error `detail` field to produce: `"Product with title Blue Shirt already exists"`.
toko-rs's `map_db_constraint` produces: `"A record with this value already exists"`.

**Status**: Known limitation — requires PG-specific error detail parsing. Same as prior F7 finding.

---

## Dimension 6: Business Logic

### B1: `GET /store/orders/{id}` missing customer ownership verification (BUG — MEDIUM — NEW)

**Location**: `src/order/routes.rs:56-63`

The handler extracts only `Path(id)` and calls `find_by_id`. It does NOT extract `CustomerId` from the auth middleware extension, and does NOT verify `order.customer_id == customer.id`.

**Impact**: Any authenticated customer can view any order by ID — order enumeration vulnerability.

**Fix**: Extract `Extension(customer): Extension<CustomerId>`, verify `order.customer_id == Some(customer.id)`.

### B2: `add_line_item` race condition without row-level lock (BUG — MEDIUM — NEW)

**Location**: `src/cart/repository.rs` — `add_line_item`

The dedup SELECT + INSERT/UPDATE runs without `SELECT ... FOR UPDATE`. Two concurrent requests for the same cart + variant could both read "no existing item" and both INSERT, creating duplicate line items.

Medusa avoids this via `acquireLockStep` (distributed lock on `cart_id`).

**Fix**: Add `SELECT ... FOR UPDATE` on the cart row or the existing line item row within the dedup query.

### B3: Cart `update_line_item` / `update_cart` — no FOR UPDATE guard (MEDIUM — NEW)

Completed cart checks are done in separate SELECT statements, but the subsequent UPDATE has no `AND completed_at IS NULL` in its WHERE clause. A concurrent cart completion could slip through.

**Fix**: Add `AND completed_at IS NULL` to UPDATE WHERE clauses, or use FOR UPDATE pattern from order creation.

### B4: Product `update` — existence check + UPDATE not transactional (LOW — NEW)

`update()` verifies the product exists (SELECT) then updates it (UPDATE) without a transaction. Between the two, another request could soft-delete the product.

**Status**: Very low severity — the UPDATE would modify a soft-deleted row which is invisible to normal queries.

### B5: N+1 query patterns (LOW — known)

- `load_relations` per product in list queries
- `load_items` per order in `list_by_customer`

Same as prior F6 finding. Performance optimization, not correctness.

---

## Items Confirmed Fixed from Master Checklist (77/77 verified)

All 77 items from `docs/audit-master-checklist.md` verified correct in source:

- 15 bugs fixed (idempotency, transactionality, cascades, etc.)
- 16 response shape fixes (delete response, totals, stubs, addresses, per-item fields)
- 6 input/validation fixes (deny_unknown_fields, metadata type, limit cap)
- 11 error handling fixes (status codes, type mapping, JSON rejection, dead code removal)
- 18 DB schema fixes (constraints, indexes, defaults, migrations, currency)
- 7 business logic fixes (dedup, idempotent DELETE, pagination defaults)
- 4 config/infra fixes (CORS, config defaults, SQLite feature flag)

No regressions detected.

---

## Recommended Action Plan

### Immediate (new bugs)

| Finding | Effort | Impact |
|---------|--------|--------|
| B1: Order ownership verification | ~15 min | Security — prevents order enumeration |
| B2: `add_line_item` FOR UPDATE | ~30 min | Correctness — prevents duplicate line items |
| S4: Add `#[serde(skip)]` to `snapshot` | ~5 min | API contract — stops leaking internal field |
| S5: Skip `has_account` on store response | ~15 min | API contract — stops leaking admin field |
| D1: Add "draft" to orders.status CHECK | ~10 min | Schema parity — prevents future errors |
| D2: Add `deleted_at` to payment_records | ~10 min | Schema parity — enables soft-delete |

### Next Sprint (P1 polish)

| Finding | Effort | Impact |
|---------|--------|--------|
| I1: Remove `deny_unknown_fields` from 5 non-strict types | ~30 min | SDK compatibility |
| B3: Add completed_at guard to cart UPDATE WHERE | ~1 hour | Correctness |
| S7: Add `order.summary` stub | ~1 hour | Frontend compatibility |
| D3: Rename `provider` → `provider_id` | ~1 hour | Schema parity |
| I2: Accept explicit null via `#[serde(default)]` | ~2 hours | SDK compatibility |

### Deferred (low priority / by design)

| Finding | Reason |
|---------|--------|
| S1: Untyped addresses | Architectural (Decision 5) |
| S2: Untyped fulfillments/shipping | P1 stubs, arrays prevent TypeError |
| S3: Simplified calculated_price | P1 simplification (Decision 13) |
| I4: `price` vs `prices` | Known divergence (Decision 13) |
| E2: Conflict message text | More informative than Medusa |
| E4: Generic DB constraint messages | Same as prior F7, requires PG parsing |
| B4: Product update not transactional | Very low severity |
| B5: N+1 queries | Performance, not correctness |

---

## Audit Methodology

Six parallel audit streams compared toko-rs against `vendor/medusa/`:

1. **Route paths & HTTP methods**: All 24 endpoints vs Medusa route handlers → 100% match
2. **Response shapes**: Field-by-field comparison against Medusa TypeScript types → 3 HIGH, 3 MEDIUM gaps
3. **Request/input types**: Field-by-field comparison against Medusa Zod validators → 3 MEDIUM gaps
4. **Database schema**: 14 tables vs Medusa models → 2 HIGH, 3 MEDIUM schema gaps
5. **Error handling**: All 9 variants vs Medusa error handler → confirmed correct, 2 MEDIUM nuances
6. **Business logic**: 8 core flows vs Medusa workflows → 2 MEDIUM bugs, 1 MEDIUM gap

Each stream was cross-referenced against `docs/audit-master-checklist.md` to confirm prior fixes and identify new findings only.
