# Task 24: Ninth Audit — P1 Medusa Compatibility Full Audit (Post-100 Fixes)

**Source**: Comprehensive 6-dimension audit against `vendor/medusa/` at develop branch.
**Reconciled against**: `docs/audit-master-checklist.md` (100 prior fixes confirmed).
**Date**: 2026-04-24
**Status**: Findings identified, pending implementation.

---

## Audit Methodology

Six parallel audit streams compared all 25 P1 endpoints against Medusa vendor source:

1. **Product models & routes**: Field-by-field comparison of Product, ProductVariant, ProductOption, ProductOptionValue, CalculatedPrice, ImageStub, and all input/response types
2. **Cart/Order/Customer models & routes**: Field-by-field comparison of Cart, CartLineItem, Order, OrderLineItem, Customer, CustomerAddress and all input/response types
3. **Error handling**: Error response format, `code`/`type`/`message` fields, HTTP status codes, Medusa error handler behavior
4. **Input validation**: Edge cases, empty strings, negative values, missing validations
5. **Business logic**: Snapshot population, line item dedup, cart completion flow, hardcoded defaults
6. **P2 gap documentation**: Architectural differences requiring schema changes or new modules

All findings cross-referenced against 100-item master checklist to avoid duplicates.

---

## Findings Summary

| Severity | Count |
|----------|-------|
| BUG (correctness) | 5 |
| MEDIUM (validation gaps) | 3 |
| LOW (documentation) | 2 |
| **Total actionable P1** | **10** |
| P2 architectural gaps (documented) | ~30 |

---

## Actionable P1 Findings

### BUG-1: `variant_barcode` and `product_subtitle` never populated on line items

**Severity**: BUG
**Files**: `src/cart/models.rs:52,58`, `src/order/models.rs:54,60`, `src/cart/repository.rs:150-206`
**Medusa**: `packages/modules/cart/src/models/line-item.ts` — `variant_barcode`, `product_subtitle` are persisted fields.

Both `CartLineItem` and `OrderLineItem` declare `variant_barcode: Option<String>` and `product_subtitle: Option<String>` with `#[sqlx(skip)]`. The `from_items()` methods attempt to extract them from `snapshot["variant_barcode"]` and `snapshot["product_subtitle"]` respectively. However:

- The snapshot JSON at `src/cart/repository.rs:199-206` never includes `variant_barcode` or `product_subtitle`
- The SQL query at `src/cart/repository.rs:150-158` selects `p.title`, `p.description`, `p.handle` but NOT `p.subtitle`
- The `product_variants` table has no `barcode` column at all
- The extraction code in `from_items()` is dead code — these fields are always `None`

**Fix**:
1. Add `p.subtitle as product_subtitle` to the snapshot query
2. Add `"product_subtitle"` to the snapshot JSON
3. Add a `barcode TEXT` column to `product_variants` (if P1) or document as P2
4. For now, at minimum: populate `product_subtitle` from the existing `products.subtitle` column

---

### BUG-2: `requires_shipping` and `is_discountable` hardcoded on line items

**Severity**: BUG
**Files**: `src/cart/models.rs:119-120`, `src/order/models.rs:125-126`
**Medusa**: `packages/modules/cart/src/models/line-item.ts:28-29` — persisted DB columns with per-item values.

```rust
item.requires_shipping = true;
item.is_discountable = true;
```

Medusa reads these from the product/variant data. Digital goods should have `requires_shipping: false`, gift cards should have `is_discountable: false`. toko-rs now has `is_giftcard` and `discountable` columns on products (Task 23).

**Fix**: Read `requires_shipping` and `is_discountable` from the snapshot data during `from_items()`. For `is_discountable`, use the product's `discountable` field. For `requires_shipping`, default to `true` until shipping profiles are implemented (P2).

---

### BUG-3: `UpdateVariantInput.price` has no range validation — negative prices accepted

**Severity**: BUG
**Files**: `src/product/types.rs:88`
**Compare with**: `src/product/types.rs:63` — `CreateProductVariantInput.price` has `#[validate(range(min = 0))]`

```rust
pub struct UpdateVariantInput {
    pub price: Option<i64>,  // no #[validate(range(min = 0))]
}
```

A client can `POST /admin/products/{id}/variants/{variant_id}` with `{"price": -9999}` and it will be persisted.

**Fix**: Add `#[validate(range(min = 0, message = "Price cannot be negative"))]` to `UpdateVariantInput.price`.

---

### BUG-4: `AddLineItemInput.variant_id` accepts empty string

**Severity**: BUG
**Files**: `src/cart/types.rs:27`

```rust
pub struct AddLineItemInput {
    pub variant_id: String,  // no length validation
}
```

Sending `{"variant_id": "", "quantity": 1}` passes validation but fails at the DB level with an unhelpful 404 error.

**Fix**: Add `#[validate(length(min = 1, message = "variant_id is required"))]` to `variant_id`.

---

### BUG-5: `UpdateLineItemInput.quantity` allows 0 — should remove item

**Severity**: BUG
**Files**: `src/cart/types.rs:35`
**Medusa**: Setting quantity to 0 triggers line item removal.

```rust
pub struct UpdateLineItemInput {
    #[validate(range(min = 0))]
    pub quantity: i64,
}
```

Quantity 0 persists a zero-quantity line item in the cart, which is meaningless. Medusa treats this as removal.

**Fix**: Either (a) change to `range(min = 1)` and reject 0, or (b) in the route handler, if `quantity == 0`, call the delete endpoint instead.

---

### MEDIUM-1: `UpdateProductInput` and `UpdateVariantInput` missing `deny_unknown_fields`

**Severity**: MEDIUM
**Files**: `src/product/types.rs:70,83`

`CreateProductInput` and `CreateProductVariantInput` have `#[serde(deny_unknown_fields)]`, but the update variants do not. Typos in update payloads are silently ignored instead of returning a 400 error.

**Fix**: Add `#[serde(deny_unknown_fields)]` to both `UpdateProductInput` and `UpdateVariantInput`.

---

### MEDIUM-2: No validation on empty-string `title`/`sku` in `UpdateProductInput`

**Severity**: MEDIUM
**Files**: `src/product/types.rs:71-81`

`UpdateProductInput.title` and `UpdateProductInput.sku` have no validation. The repository uses `COALESCE(NULLIF($1, ''), ...)` which silently no-ops on empty strings. Medusa would return a validation error.

**Fix**: For fields where empty string is invalid, add `#[validate(length(min = 1))]` or handle in the validate() method.

---

### MEDIUM-3: `product_variant_option` rows not cleaned when option value is soft-deleted

**Severity**: MEDIUM
**Files**: N/A (no option value soft-delete endpoint exists)

While Task 23h fixed cascade cleanup for variant soft-delete, there is no equivalent cleanup when an option value is soft-deleted. If an option value is deleted, any `product_variant_option` rows referencing it become orphan references. This is a gap but not actionable until option CRUD endpoints exist (P2).

**Decision**: Defer to P2 when option CRUD is implemented.

---

## Previously Known / Deferred Items Reconfirmed

The following were re-verified as documented P2 gaps:

| # | Gap | Master Checklist # | Reason |
|---|-----|-------------------|--------|
| A1 | Multi-currency pricing (`prices[]` vs `price: i64`) | #93 | Architectural — requires pricing module |
| A2 | Missing `region_id`, shipping methods, promotions | #93 | P2 checkout flow |
| A3 | Addresses as JSONB vs typed tables | #93 | Schema migration required |
| A4 | Missing product dimension fields (width, height, etc.) | #93 | P2 product enrichment |
| A5 | Missing variant fields (barcode, ean, upc, etc.) | #93 | P2 product enrichment |
| A6 | Images always empty (`product_images` table missing) | #93 | P2 media management |
| A7 | `CalculatedPrice` is passthrough (no pricing engine) | #93 | P2 pricing module |
| A8 | Missing `fields`/`expand` query parameter support | #93 | P2 API flexibility |
| A9 | Error `code` field always present vs Medusa omitting it | #86 | Intentional (more consistent) |
| A10 | No `Idempotency-Key` header support | #93 | P2 resilience |
| A11 | `deleted_at` hidden on option/variant/option-value types | T22 decision | Intentional for P1 |
| A12 | No order status transitions (always "pending") | #93 | P2 operations |
| A13 | No inventory checks on add-to-cart | #93 | P2 inventory module |
| A14 | No address CRUD endpoints | #93 | P2 customer management |
| A15 | No store variant endpoints | #93 | P2 store API |
| A16 | No product option CRUD endpoints | #93 | P2 product management |

---

## What's Already Correct (Verified)

| Area | Status |
|------|--------|
| All HTTP methods match Medusa | ✅ POST for updates, DELETE for deletes |
| Response wrapper keys match | ✅ `{ product }`, `{ cart }`, `{ order }`, etc. |
| Delete response shapes match | ✅ `{ id, object, deleted }` |
| Pagination shape matches | ✅ `{ items, count, offset, limit }` |
| Error status codes correct | ✅ 400, 401, 404, 409, 422, 500 |
| Completed-cart guards return 400 | ✅ Fixed in T23g |
| SQL injection clean | ✅ `validate_order_param()` whitelist (T23a) |
| Cart→order field copy | ✅ Fixed in T23b |
| Cart line item dedup (variant + price + metadata) | ✅ Arguably better than Medusa's |
| `deny_unknown_fields` on create types | ✅ |
| `variant_rank` on create/update variant | ✅ |
| `subtitle`, `is_giftcard`, `discountable` stored | ✅ Fixed in T23d, T23e |
| `deleted_at` visible on Product and Customer | ✅ Fixed in T23f |
| Pivot cleanup on variant soft-delete | ✅ Fixed in T23h |
| Option coverage validation | ✅ Fixed in T23i |

---

## Implementation Checklist

### 24a. Populate `product_subtitle` in line item snapshot (BUG-1 partial)
- [ ] 24a.1 Add `p.subtitle as product_subtitle` to snapshot query in `src/cart/repository.rs:150-158`
- [ ] 24a.2 Add `"product_subtitle": product_subtitle` to snapshot JSON at `src/cart/repository.rs:199-206`
- [ ] 24a.3 Handle `None` case (subtitle is `Option<String>`) in snapshot construction
- [ ] 24a.4 Add test verifying `product_subtitle` appears in cart line item response
- [ ] 24a.5 Run full test suite

### 24b. Read `is_discountable` and `requires_shipping` from product data (BUG-2)
- [ ] 24b.1 Add `"is_discountable"` and `"requires_shipping"` to snapshot JSON, read from product fields
- [ ] 24b.2 Update `from_items()` in both cart and order models to read from snapshot
- [ ] 24b.3 Default `requires_shipping` to `true`, read `is_discountable` from product's `discountable` column
- [ ] 24b.4 Add test: gift card product has `is_discountable: false` on line item
- [ ] 24b.5 Run full test suite

### 24c. Add price validation to `UpdateVariantInput` (BUG-3)
- [ ] 24c.1 Add `#[validate(range(min = 0, message = "Price cannot be negative"))]` to `UpdateVariantInput.price`
- [ ] 24c.2 Add test: negative price in update variant rejected with 400
- [ ] 24c.3 Run full test suite

### 24d. Add `variant_id` length validation to `AddLineItemInput` (BUG-4)
- [ ] 24d.1 Add `#[validate(length(min = 1, message = "variant_id is required"))]` to `AddLineItemInput.variant_id`
- [ ] 24d.2 Add test: empty variant_id rejected with 400
- [ ] 24d.3 Run full test suite

### 24e. Handle `quantity: 0` in update line item (BUG-5)
- [ ] 24e.1 Change `UpdateLineItemInput.quantity` validation to `range(min = 1)` OR add handler logic for quantity 0 → delete
- [ ] 24e.2 Add test: quantity 0 rejected or triggers removal
- [ ] 24e.3 Run full test suite

### 24f. Add `deny_unknown_fields` to update input types (MEDIUM-1)
- [ ] 24f.1 Add `#[serde(deny_unknown_fields)]` to `UpdateProductInput`
- [ ] 24f.2 Add `#[serde(deny_unknown_fields)]` to `UpdateVariantInput`
- [ ] 24f.3 Add tests: unknown fields in update payloads rejected with 400
- [ ] 24f.4 Run full test suite

### 24g. Verification pass
- [ ] 24g.1 Run full test suite on SQLite
- [ ] 24g.2 Run full test suite on PostgreSQL
- [ ] 24g.3 Run `cargo clippy -- -D warnings` on both features
- [ ] 24g.4 Run `cargo fmt --check`
- [ ] 24g.5 Update `docs/audit-master-checklist.md` with new fixes

---

## Architectural P2 Gaps (Documented, Not Actionable in P1)

These findings document fundamental schema/architecture differences with Medusa that require P2 planning:

| # | Gap | Impact |
|---|-----|--------|
| P2-1 | Multi-currency pricing (`prices[]` array vs single `price: i64`) | Multi-currency, price rules, regional pricing impossible |
| P2-2 | Missing `product_images` table with `rank` | No image storage or management |
| P2-3 | Missing `region_id`, `shipping_methods`, `ShippingMethod` table | No shipping cost calculation, no shipping method selection |
| P2-4 | Addresses as JSONB vs dedicated `cart_address` table | No address validation, no structured queries, no tax calc |
| P2-5 | Missing product dimension fields (width, height, length, weight, etc.) | Shipping calculation, customs, POS integrations blocked |
| P2-6 | Missing variant fields (barcode, ean, upc, thumbnail, etc.) | POS, inventory integrations blocked |
| P2-7 | Missing `CalculatedPrice` fields (currency_code, tax amounts, price lists) | Price ambiguity, no tax-inclusive display |
| P2-8 | No `fields`/`expand` query parameter support | No sparse fieldsets, always returns full objects |
| P2-9 | Error `code` field always present | Medusa omits for some error types — intentional divergence |
| P2-10 | No `Idempotency-Key` header support | Double-ordering on client retry possible |
| P2-11 | No order status transitions (always "pending") | No cancel, archive, complete flows |
| P2-12 | No inventory checks on add-to-cart | Items can be added regardless of stock |
| P2-13 | No address CRUD endpoints | Customer address management impossible |
| P2-14 | No store variant endpoints | Store cannot list/get variants directly |
| P2-15 | No product option CRUD endpoints | Cannot add/update/delete options on existing products |
| P2-16 | No promotion/tax routes | No promo codes, no tax recalculation |
| P2-17 | `deleted_at` hidden on option/variant/option-value | Intentional P1 decision — P2 may expose selectively |
| P2-18 | Missing `sales_channel_id`, `locale` on cart/order | No multi-channel, no i18n |
| P2-19 | Missing `collection`, `type`, `tags`, `categories` tables | No product categorization, no storefront navigation |
| P2-20 | `has_account` hardcoded to TRUE | Should default to FALSE, set TRUE only on auth link |
