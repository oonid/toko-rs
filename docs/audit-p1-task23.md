# Task 23: Eighth Audit — P1 Medusa Compatibility Deep Audit

**Source**: Comprehensive 6-dimension audit against `vendor/medusa/` at develop branch.
**Reconciled against**: `docs/audit-master-checklist.md` (89 prior fixes confirmed).
**Date**: 2026-04-23
**Status**: Findings identified, pending implementation.

---

## Audit Methodology

Six parallel audit streams compared all 25 P1 endpoints against Medusa vendor source:

1. **Routes & handlers**: HTTP methods, paths, status codes, middleware, query params
2. **Response shapes**: Field-by-field comparison against Medusa query configs and model types
3. **Input types & validation**: Request schemas, field types, strictness, accepted-but-dropped fields
4. **Database schema**: Column-by-column comparison against Medusa entity definitions
5. **Error handling**: Error types, codes, status codes vs Medusa's error handler
6. **Business logic**: Workflow correctness, atomicity, idempotency, edge cases

All findings cross-referenced against 89-item master checklist to avoid duplicates.

---

## Findings Summary

| Severity | Count |
|----------|-------|
| BUG (correctness/security) | 1 |
| HIGH | 4 |
| MEDIUM | 11 |
| LOW | 10 |
| **Total actionable** | **26** |

Many HIGH/MEDIUM schema findings (missing columns for `subtitle`, `is_giftcard`, `discountable`, barcode, etc.) are architectural P2 gaps documented here for planning but **not** proposed for immediate fix. The **actionable P1 findings** that should be fixed are:

- **BUG-1**: SQL injection via `order` query param
- **B1**: Cart metadata/address not copied to order on completion
- **B2**: Cart line item metadata not copied to order line items
- **B3**: `update_cart` missing `rows_affected()` check — silent 200 on concurrent completion
- **S1**: `subtitle` accepted in input but silently dropped (no DB column)
- **S2**: `is_giftcard`/`discountable` accepted in input but always return hardcoded defaults
- **S3**: Customer `deleted_at` incorrectly hidden — Medusa store query config includes it
- **S4**: Admin product `deleted_at` incorrectly hidden — Medusa admin query config includes it
- **E1**: `map_db_constraint` is dead code — FK/NOT NULL violations leak as 500
- **E2**: Cart state violations return 409 (Conflict) instead of 400 (InvalidData)
- **D1**: `soft_delete_variant` leaves orphaned `product_variant_option` pivot rows
- **V1**: `add_variant` allows partial option coverage (no "all options" check)
- **V2**: `create_product` option coverage validation skipped when variant `options` is `None`

---

## BUG: SQL Injection via `order` Query Parameter

**Severity**: BUG (security)
**Files**: `src/product/repository.rs:168-175`, `:201-204`, `:389-393`
**Endpoints**: `GET /admin/products`, `GET /store/products`, `GET /admin/products/:id/variants`

`FindParams.order: Option<String>` is interpolated directly into SQL via `format!()` with zero sanitization:

```rust
let order = params.order.as_deref().unwrap_or("p.created_at DESC");
let query_sql = format!(
    "SELECT * FROM products p {} ORDER BY {} LIMIT $1 OFFSET $2",
    where_clause, order  // user-controlled string
);
```

While sqlx's prepared-statement model prevents multi-statement injection, subquery injection through ORDER BY is feasible. Three repository methods share this pattern: `list`, `list_published`, `list_variants`.

**Fix**: Validate `order` against a whitelist of allowed column+direction pairs (e.g., `{"id", "title", "created_at", "updated_at", "status"}` × `{ASC, DESC}`).

---

## HIGH Findings

### B1. Cart metadata/address not copied to order on completion

**Files**: `src/order/repository.rs:70-84`
**Medusa**: `completeCartWorkflow` explicitly copies `shipping_address`, `billing_address`, `metadata` from cart to order.
**toko-rs**: INSERT only copies `customer_id`, `email`, `currency_code`. The `orders` table has `metadata`, `shipping_address`, `billing_address` columns — they are left NULL.

### B2. Cart line item metadata not copied to order line items

**Files**: `src/order/repository.rs:89-106`
**Medusa**: `prepareLineItemData` maps `metadata: item?.metadata ?? {}` into order line items.
**toko-rs**: INSERT copies `title`, `quantity`, `unit_price`, `variant_id`, `product_id`, `snapshot` but not `metadata`. The `order_line_items` table has a `metadata` column.

### B3. `update_cart` missing `rows_affected()` check

**Files**: `src/cart/repository.rs:82-100`
**Medusa**: `updateCartWorkflow` acquires a distributed lock before reading the cart.
**toko-rs**: The UPDATE WHERE clause has `AND completed_at IS NULL` (correctly prevents mutation), but if the cart is completed between the SELECT and UPDATE, `rows_affected() == 0` and the code proceeds to `self.get_cart()` returning 200 with stale data. The client gets a misleading success response.

### S1. `subtitle` accepted in input but silently dropped

**Files**: `src/product/types.rs:35`, `src/product/models.rs` (no subtitle field), `migrations/001_products.sql` (no subtitle column)
**Medusa**: `subtitle` is a first-class product field, included in admin and store query config defaults, stored in DB.
**toko-rs**: `CreateProductInput` and `UpdateProductInput` declare `subtitle: Option<String>` but the value is never stored. No DB column, no model field, no INSERT/UPDATE binding.

---

## MEDIUM Findings

### S2. `is_giftcard`/`discountable` accepted but always return hardcoded defaults

**Files**: `src/product/types.rs:41-42`, `src/product/repository.rs:743-750`
**Medusa**: Both fields are stored on the product entity, persisted, and returned as stored.
**toko-rs**: Input accepts both fields. `ProductWithRelations` hardcodes `is_giftcard: false, discountable: true`. No DB columns exist. Creating a product with `is_giftcard: true` returns `is_giftcard: false`.

### S3. Customer `deleted_at` incorrectly hidden in store responses

**Files**: `src/customer/models.rs:18` (`#[serde(skip)]`)
**Medusa**: `defaultStoreCustomersFields` line 10: `"deleted_at"` — explicitly included in store query config.
**toko-rs**: `#[serde(skip)]` hides `deleted_at` from ALL customer responses including store.

### S4. Admin product `deleted_at` incorrectly hidden

**Files**: `src/product/models.rs:17` (`#[serde(skip)]`)
**Medusa**: `defaultAdminProductFields` line 89: `"deleted_at"` — explicitly included in admin query config.
**toko-rs**: `#[serde(skip)]` hides `deleted_at` from ALL product responses including admin. Store product defaults exclude `deleted_at`, so the skip is correct for store but wrong for admin.

### E1. `map_db_constraint` is dead code — constraint violations leak as 500

**Files**: `src/error.rs:84`, all 5 repository files
**Medusa**: `exception-formatter.ts` intercepts all constraint types (23505, 23503, 23502, 40001) at the HTTP middleware layer.
**toko-rs**: `map_db_constraint` is defined and tested but never called. Repositories do ad-hoc mapping:
  - `product`: handles unique only; FK/NOT NULL → raw 500
  - `customer`: handles unique only; FK/NOT NULL → raw 500
  - `order`: handles unique (as Conflict); FK/NOT NULL → raw 500
  - `payment`: no mapping at all; all constraints → raw 500

### E2. Cart state violations return 409 (Conflict) instead of 400 (InvalidData)

**Files**: `src/cart/repository.rs:79,121,136,290,335`, `src/order/repository.rs:33,46`
**Medusa**: `validateCartStep` throws `INVALID_DATA` (400) for "Cart is already completed."
**toko-rs**: Returns `Conflict` (409) in 7 locations. Medusa reserves 409 exclusively for idempotency/concurrency conflicts.

### D1. `soft_delete_variant` leaves orphaned pivot rows

**Files**: `src/product/repository.rs:520-554`
**Medusa**: `deleteProductVariantsWorkflow` calls `removeRemoteLinkStep` which removes variant's links.
**toko-rs**: Product-level `soft_delete` (item #96) hard-deletes pivot rows, but single-variant `soft_delete_variant` does not.

### V1. `add_variant` allows partial option coverage

**Files**: `src/product/repository.rs:327-365`
**Medusa**: Workflow-level consistency maintained by ORM cascade relationships.
**toko-rs**: `create_product` enforces full coverage (all product options must be present). `add_variant` has no equivalent check. Inconsistency: batch creation validates, individual creation does not.

### V2. `create_product` option coverage validation skipped when `options` is `None`

**Files**: `src/product/repository.rs:94-106`
**toko-rs**: The coverage check is guarded by `if let Some(ref opts) = var_input.options`. If options is `None` (omitted from JSON), the check is skipped entirely. A variant can be created without any option bindings even when the product has defined options.

### N3. Cart completion not idempotent on client retry

**Files**: `src/order/repository.rs:32-34`, `src/order/routes.rs:27`
**Medusa**: `completeCartWorkflow` checks `order_cart` link table before creating a new order — returns existing order on retry.
**toko-rs**: Returns 409 on retry with no order data. The `idempotency_keys` table exists (migration 006) but is unused.

### N10. Customer `has_account` hardcoded to `TRUE`

**Files**: `src/customer/repository.rs:22-46`
**Medusa**: `createCustomerAccountWorkflow` sets `has_account: !!data.input.authIdentityId` — dynamically determines guest vs registered.
**toko-rs**: Always sets `has_account = TRUE`. The composite unique index `uq_customers_email ON (email, has_account) WHERE deleted_at IS NULL` was designed for guest+registered coexistence but guest creation is unreachable.

### S5. Variant `options` response shape differs from Medusa

**Files**: `src/product/models.rs:97-102` (`VariantOptionValue`)
**Medusa**: Variant options include nested `option: { id, title, ... }` alongside `id` and `value`.
**toko-rs**: Returns `{ id, value, option_id }` — requires cross-referencing with parent product's options array to find the option title.

---

## LOW Findings

### L1. `fields` query parameter parsed but never used
`FindParams.fields: Option<String>` is accepted but no repository reads it. Medusa uses `fields` for sparse field selection.

### L2. Count query not transactionally consistent with data query
`SELECT COUNT(*)` and `SELECT * ... LIMIT` execute as separate statements. Under concurrent writes, `count` may differ from actual rows returned.

### L3. Product handle not auto-regenerated when title changes
In `update`, handle is always preserved as-is via `COALESCE(NULLIF(handle, ''), handle)`. Changing the title doesn't update the handle.

### L4. `variant_barcode` always `null` on cart line items
Model field exists but `product_variants` table has no `barcode` column, and snapshot never captures it.

### L5. Line item dedup `fetch_optional` may return wrong row
When multiple items share `variant_id + unit_price` but different metadata, `fetch_optional` returns an arbitrary one, potentially creating duplicates.

### L6. `update_line_item`/`delete_line_item` return `NotFound` instead of `Conflict` in race window
If cart is concurrently completed between the check and UPDATE, the error is `NotFound` (misleading) rather than `Conflict`.

### L7. Missing `NotAllowed` error variant
Medusa defines `NOT_ALLOWED` (type="not_allowed", HTTP 400) used in 28 workflow locations. toko-rs has no equivalent.

### L8. Missing domain-specific error codes
Medusa has `INSUFFICIENT_INVENTORY`, `CART_INCOMPATIBLE_STATE`, etc. toko-rs always returns generic codes.

### L9. Missing payment-specific error types
Medusa defines `PAYMENT_AUTHORIZATION_ERROR` and `PAYMENT_REQUIRES_MORE_ERROR`. P2 scope.

### L10. Order response missing multiple Medusa fields
`summary`, `transactions`, `payment_collections`, `credit_line_total`, `item_discount_total`, line item `detail`/`subtitle`/`thumbnail`/`refundable_total`. All are P2 architecture gaps.

---

## Recommended Task 23 Implementation Checklist

### 23a. Fix SQL injection in `order` query param (BUG-1 — CRITICAL)
- [ ] 23a.1 Add `validate_order_param()` to `src/types.rs` — whitelist allowed column+direction pairs
- [ ] 23a.2 Apply validation in `list`, `list_published`, `list_variants` in `src/product/repository.rs`
- [ ] 23a.3 Add tests: invalid order param returns 400, valid order params work

### 23b. Copy cart fields to order on completion (B1, B2 — HIGH)
- [ ] 23b.1 Add `metadata`, `shipping_address`, `billing_address` to order INSERT in `create_from_cart`
- [ ] 23b.2 Add `metadata` to order line item INSERT in `create_from_cart`
- [ ] 23b.3 Add tests: cart metadata/address preserved in order, line item metadata preserved

### 23c. Add `rows_affected()` check to `update_cart` (B3 — HIGH)
- [ ] 23c.1 Check `result.rows_affected()` after UPDATE in `update_cart` — return 409 if 0
- [ ] 23c.2 Add test: concurrent completion returns 409, not 200

### 23d. Add `subtitle` column to products table (S1 — MEDIUM)
- [ ] 23d.1 Add `subtitle TEXT` to `products` in PG and SQLite migrations
- [ ] 23d.2 Add `subtitle: Option<String>` to `Product` model
- [ ] 23d.3 Bind `subtitle` in `create_product` and `update` repository methods
- [ ] 23d.4 Add tests: subtitle persists and returns in response

### 23e. Add `is_giftcard` and `discountable` columns to products (S2 — MEDIUM)
- [ ] 23e.1 Add `is_giftcard BOOLEAN NOT NULL DEFAULT FALSE` and `discountable BOOLEAN NOT NULL DEFAULT TRUE` to PG and SQLite migrations
- [ ] 23e.2 Update `Product` model with new fields
- [ ] 23e.3 Bind values in `create_product` and `update` repository methods
- [ ] 23e.4 Update `load_relations` to read from DB instead of hardcoding
- [ ] 23e.5 Add tests: values persist and return correctly

### 23f. Fix `deleted_at` visibility — admin vs store split (S3, S4 — MEDIUM)
- [ ] 23f.1 Remove `#[serde(skip)]` from `deleted_at` on `Product` in `src/product/models.rs`
- [ ] 23f.2 Remove `#[serde(skip)]` from `deleted_at` on `Customer` in `src/customer/models.rs`
- [ ] 23f.3 Keep `#[serde(skip)]` on `Cart`, `Order`, `CustomerAddress`, `PaymentRecord`, `ProductOption`, `ProductOptionValue`, `ProductVariant` (not in Medusa store query configs)
- [ ] 23f.4 Update product model to support conditional serialization (admin includes, store excludes)
- [ ] 23f.5 Update tests

### 23g. Change cart state violations from 409 to 400 (E2 — MEDIUM)
- [ ] 23g.1 Change `Conflict` → `InvalidData` for completed-cart guards in `src/cart/repository.rs` (6 locations)
- [ ] 23g.2 Change `Conflict` → `InvalidData` for completed-cart guards in `src/order/repository.rs` (2 locations)
- [ ] 23g.3 Update affected tests
- [ ] 23g.4 Run full test suite

### 23h. Cascade pivot row cleanup on single variant soft-delete (D1 — MEDIUM)
- [ ] 23h.1 Add `DELETE FROM product_variant_option WHERE variant_id = $1` to `soft_delete_variant`
- [ ] 23h.2 Run full test suite

### 23i. Fix variant option coverage validation (V1, V2 — MEDIUM)
- [ ] 23i.1 Add "all options must be covered" check to `add_variant` (matching `create_product` logic)
- [ ] 23i.2 Fix `create_product` to require options when product has defined options (remove `if let Some` guard when `option_titles` is non-empty)
- [ ] 23i.3 Add tests: partial option coverage rejected in `add_variant`
- [ ] 23i.4 Run full test suite

### 23j. Verification pass
- [ ] 23j.1 Run full test suite on SQLite
- [ ] 23j.2 Run full test suite on PostgreSQL
- [ ] 23j.3 Run `cargo clippy -- -D warnings` on both features
- [ ] 23j.4 Run `cargo fmt --check`
- [ ] 23j.5 Update `docs/audit-master-checklist.md`

---

## Architectural P2 Gaps (Documented, Not Actionable in P1)

These findings document fundamental schema/architecture differences with Medusa that require P2 planning:

| # | Gap | Impact |
|---|-----|--------|
| A1 | Order line items: Medusa uses two-table split (OrderLineItem + OrderItem) with fulfillment quantity tracking | Order edits, returns, exchanges not possible |
| A2 | Payments: Medusa uses PaymentCollection → Payment → Capture/Refund hierarchy | Multi-payment orders, partial captures, refunds not supported |
| A3 | Addresses: Medusa uses dedicated typed address tables, toko-rs uses inline JSONB | Address querying/indexing impossible, tax calculation blocked |
| A4 | Products: 13+ missing columns (weight, dimensions, barcode, origin_country, etc.) | Shipping calculation, inventory management, POS integrations blocked |
| A5 | Cart line items: 18 missing snapshot columns | Promotion engine, tax calculation, shipping flags unavailable |
| A6 | Orders: Missing `version` column, `summary` object, `region_id`, `sales_channel_id` | Order versioning, region pricing, channel scoping not supported |
| A7 | Products: Missing `product_type`, `product_collection`, `product_tag`, `product_category` tables | Filtering, categorization, storefront navigation not available |
| A8 | Images: Missing `product_images` table with `rank` and per-variant assignment | Multiple product images not supported |
| A9 | `map_db_constraint` needs to be wired as middleware or called consistently | All constraint types must be caught, not just unique |
| A10 | Idempotency: `idempotency_keys` table exists but is unused | Cart completion not idempotent on client retry |
