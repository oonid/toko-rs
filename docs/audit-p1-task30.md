# Task 30: P1 Re-Audit Against Updated Medusa Vendor

**Date**: 2026-04-28
**Medusa vendor**: `0303d7f30b` (latest develop branch)
**Scope**: P1 only (Browse → Cart → Checkout = 25 endpoint methods)

## Methodology

1. Read all Medusa vendor source for P1 endpoints (admin/store products, options, variants, cart, order, customer)
2. Read all toko-rs models, types, routes, migrations, error handling
3. Compare response shapes, input validators, HTTP methods/paths, business logic
4. Triage each finding as: **P1 FIX**, **P2 DEFER**, or **BY-DESIGN**

## Triage Policy

- **P1 FIX**: Missing field/endpoint that breaks P1 Browse→Cart→Checkout AND does not require a P2 module
- **P2 DEFER**: Missing field/feature that depends on a P2 module (regions, inventory, shipping, promotions, payment, tax, sales channels, product types/collections/tags, admin auth)
- **BY-DESIGN**: Intentional divergence for P1 scope

---

## P1 FIX (8 findings)

### T30-1 | 5 Product Option endpoints entirely missing
- **Medusa has**:
  - `GET /admin/products/{id}/options` → `{product_options, count, offset, limit}`
  - `POST /admin/products/{id}/options` → `{product: Product}` (create option)
  - `GET /admin/products/{id}/options/{option_id}` → `{product_option: Option}`
  - `POST /admin/products/{id}/options/{option_id}` → `{product: Product}` (update option)
  - `DELETE /admin/products/{id}/options/{option_id}` → `{id, object: "product_option", deleted: true, parent: Product}`
- **toko-rs has**: None. Options can only be created inline during `POST /admin/products`
- **Impact**: 3 of the original 14 product endpoint methods are missing (create/update/delete option). Medusa also has list and get for options (5 total).
- **Files**: `src/product/routes.rs`, `src/product/repository.rs`, `src/product/types.rs`
- **Source**: `vendor/medusa/packages/medusa/src/api/admin/products/[id]/options/route.ts` and `[option_id]/route.ts`

### T30-2 | Variant model missing `thumbnail` field
- **Medusa**: `ProductVariant` has `thumbnail` field (since v2.11.2)
- **toko-rs**: Missing from `ProductVariant` model, `product_variants` table, `CreateProductVariantInput`, `UpdateVariantInput`
- **Impact**: Variant-level thumbnail images cannot be displayed in storefronts
- **Files**: `src/product/models.rs:54`, `migrations/001_products.sql:37`, `src/product/types.rs:66,99`
- **Source**: `vendor/medusa/packages/modules/product/src/models/product-variant.ts`

### T30-3 | Product images not persisted
- **Medusa**: `ProductImage` model with `{id, url, rank, metadata}`, stored in `image` table with `product_images` join table. Product response includes `*images` in default fields.
- **toko-rs**: `ImageStub { url }` only. No `product_images` table. Repository hardcodes `images: vec![]` at `src/product/repository.rs:827`. No `images` field in `CreateProductInput` or `UpdateProductInput`.
- **Impact**: Product images are always empty in API responses. Frontends cannot display product images.
- **Files**: `src/product/models.rs:70-72,80`, `src/product/repository.rs:827`, `migrations/001_products.sql`
- **Source**: `vendor/medusa/packages/modules/product/src/models/product-image.ts`

### T30-4 | Line item missing `compare_at_unit_price` field
- **Medusa**: `LineItem` has `compare_at_unit_price` for "was $X, now $Y" pricing
- **toko-rs**: Missing from `CartLineItem` and `OrderLineItem` models, `cart_line_items` and `order_line_items` tables
- **Impact**: Strike-through pricing ("sale price") not available in Browse→Cart→Checkout
- **Files**: `src/cart/models.rs:25`, `src/order/models.rs:27`, `migrations/003_carts.sql:15`, `migrations/004_orders.sql:25`
- **Source**: Medusa `StoreCartLineItem` type

### T30-5 | Customer missing `created_by` field
- **Medusa**: `Customer` model has `created_by` field
- **toko-rs**: Missing from `Customer` model and `customers` table
- **Impact**: Cannot track who created a customer record
- **Files**: `src/customer/models.rs:6`, `migrations/002_customers.sql:1`
- **Source**: Medusa `CustomerDTO`

### T30-6 | `CreateCustomerInput` should require `email`
- **Medusa**: Zod validator technically allows null, but the `createCustomersWorkflow` requires email
- **toko-rs**: `email` is `Option<String>` with no server-side enforcement
- **Impact**: Customers can be created without email, breaking order confirmation flow
- **Files**: `src/customer/types.rs:8`
- **Source**: Medusa `createCustomersWorkflow` step

### T30-7 | `UpdateCustomerInput` missing `email` field
- **Medusa**: `UpdateCustomerDTO` includes `email`
- **toko-rs**: Cannot update email via `POST /store/customers/me`
- **Impact**: Customers cannot change their email
- **Files**: `src/customer/types.rs:19`
- **Source**: Medusa `StoreUpdateCustomer`

### T30-8 | `CreateProductInput` / `UpdateProductInput` missing `images` field
- **Medusa**: Create and update product accept `images: Array<{url}>`
- **toko-rs**: No way to set images on create or update
- **Impact**: Even if images were persisted (T30-3), there's no way to set them
- **Files**: `src/product/types.rs:32,78`
- **Source**: Medusa `AdminCreateProduct`, `AdminUpdateProduct`

---

## P2 DEFER (14 findings)

### T30-D1 | Variant model missing inventory/logistics fields
- `barcode, ean, upc, allow_backorder, manage_inventory` → Inventory module (P2)
- `hs_code, origin_country, mid_code, material` → Customs/logistics (P2)
- `weight, length, height, width` → Shipping module (P2)
- **Source**: `vendor/medusa/packages/modules/product/src/models/product-variant.ts`

### T30-D2 | Product model missing physical dimension fields
- `weight, length, height, width, hs_code, origin_country, mid_code, material` → Shipping/customs (P2)
- **Source**: `vendor/medusa/packages/medusa/src/api/store/products/query-config.ts:13-19`

### T30-D3 | Product `type_id` / `collection_id` not persisted
- Both are `#[sqlx(skip)]` in toko-rs model, always `None`
- These are FKs to ProductType / ProductCollection modules (P2)
- **Source**: `src/product/models.rs:18-21`

### T30-D4 | Product missing `type`, `collection`, `tags` relations
- Require ProductType, ProductCollection, ProductTag modules (P2)
- **Source**: Medusa query-config `*type`, `*collection`, `*tags`

### T30-D5 | Variant missing `images` relation
- Many-to-many via `ProductVariantProductImage` (since v2.11.2)
- **Source**: `vendor/medusa/packages/modules/product/src/models/product-variant.ts:38-41`

### T30-D6 | Cart missing `region_id`, `sales_channel_id`, `locale`
- `region_id` → Region module (P2)
- `sales_channel_id` → Sales Channel module (P2)
- `locale` → Simple string but low priority for P1
- **Source**: Medusa `StoreCart` type

### T30-D7 | Order missing `region_id`, `sales_channel_id`, `locale`, `version`, `is_draft_order`
- All require P2 modules or P2 features (order editing, draft orders)
- **Source**: Medusa `StoreOrder` type

### T30-D8 | Order missing `summary` relation wrapper
- Medusa wraps computed totals in a `summary` relation with `{trial, pending_difference, current_order, original_order}`
- Requires pricing/tax/shipping computation (P2 modules)
- **Source**: Medusa `OrderSummaryDTO`

### T30-D9 | Order missing `transactions`, `payment_collections` relations
- Require Payment module (P2)
- **Source**: Medusa `StoreOrder` query config

### T30-D10 | Line item missing `product_type`, `product_type_id`, `product_collection`
- Require ProductType/ProductCollection modules (P2)
- **Source**: Medusa `StoreCartLineItem` type

### T30-D11 | Cart complete soft-error response path not wired
- `CartCompleteResponse::error()` exists but route handler only uses `success()`
- Requires inventory checks (P2 module) to trigger soft errors
- **Source**: `src/order/types.rs:50`, `src/cart/routes.rs` complete handler

### T30-D12 | `currency_code` defaults to hardcoded `'idr'` in migration
- Config has `default_currency_code` but migration hardcodes `'idr'`
- Not P2 per se but low priority for Indonesian-market focus
- **Source**: `migrations/003_carts.sql:5`

### T30-D13 | Store order list requires customer auth
- Medusa store order GET is unauthenticated (marked as TODO in their code)
- toko-rs requires `X-Customer-Id` header — actually more secure
- **Source**: `vendor/medusa/packages/medusa/src/api/store/orders/[id]/route.ts:5`

### T30-D14 | Product option values need separate update/delete within options
- Medusa allows updating option values (adding/removing) when updating an option
- Complex nested update logic, deferrable
- **Source**: Medusa `updateProductOptionsWorkflow`

---

## BY-DESIGN (4 findings)

### T30-B1 | Variant pricing uses simplified `calculated_price` instead of `prices[]` array
- Medusa's `prices[]` requires Pricing module (P2)
- toko-rs embeds single `price` on variant and wraps in `CalculatedPrice`
- Acceptable P1 simplification

### T30-B2 | Health endpoint response shape differs
- Medusa: `{status: "ok", health: [...modules]}`
- toko-rs: `{status, database, version}`
- Health is not part of the Browse→Cart→Checkout flow

### T30-B3 | Customer auth uses `X-Customer-Id` header instead of JWT
- Intentional P1 design — no auth provider module
- Will need JWT auth provider in P2

### T30-B4 | Error response shape field ordering differs
- Medusa: `{type, message, code?}`
- toko-rs: `{code, type, message}`
- Both contain equivalent information; JSON key order is not semantically significant

---

## Confirmed MATCH (key items verified correct)

| Area | Detail |
|------|--------|
| Product delete response | `{id, object: "product", deleted: true}` — no parent ✓ |
| Variant delete response | `{id, object: "variant", deleted: true, parent: Product}` ✓ |
| Create/update variant response | `{product: Product}` ✓ |
| Cart CRUD (7 endpoints) | All correct method/path/response ✓ |
| Cart line-item delete | `{id, object: "line-item", deleted: true, parent: Cart}` ✓ |
| Cart complete response | `{type: "order", order: ...}` on success ✓ |
| Order list/get | Correct method/path/response ✓ |
| Customer register/get-me/update-me | Correct method/path/response ✓ |
| Variant nested options | `{id, value, option: {id, title}}` ✓ |
| Customer address model | All fields match Medusa ✓ |
| Cart computed totals | ~20 BigNumber fields all present ✓ |
| Line item snapshot hydration | Correctly populates product/variant fields ✓ |

---

## Summary

| Category | Count |
|----------|-------|
| **P1 FIX** | 8 |
| **P2 DEFER** | 14 |
| **BY-DESIGN** | 4 |
| **MATCH** | ~15 items verified correct |

### P1 Fix Priority Order

1. **T30-1** — Product Option CRUD endpoints (biggest gap: 5 missing endpoints)
2. **T30-3** — Product image persistence (table + join table + input/output)
3. **T30-8** — Product input `images` field (prerequisite for T30-3)
4. **T30-2** — Variant `thumbnail` field
5. **T30-4** — Line item `compare_at_unit_price`
6. **T30-5** — Customer `created_by`
7. **T30-6** — Require `email` in customer create
8. **T30-7** — Add `email` to customer update

### Estimated Effort

| Fix | Effort | Risk |
|-----|--------|------|
| T30-1 Option CRUD | High (5 endpoints, repo methods, types) | Medium |
| T30-3 Image persistence | Medium (new table, join table, repo changes) | Medium |
| T30-2 Variant thumbnail | Low (1 column + model field) | Low |
| T30-4 compare_at_unit_price | Low (2 columns + model fields) | Low |
| T30-5 Customer created_by | Low (1 column + model field) | Low |
| T30-6 Require email | Low (validation change) | Low |
| T30-7 Update email | Low (1 field) | Low |
