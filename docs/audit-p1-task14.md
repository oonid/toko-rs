# Second Audit: P1 Compatibility vs vendor/medusa/

**Date**: 2026-04-09
**Scope**: End-to-end audit of toko-rs P1 (proposal, design, implementation) against `vendor/medusa/`
**Status**: All P1 findings resolved (14a–14f). See "Resolution Status" at bottom.

---

## Audit Dimensions

| # | Dimension | Findings |
|---|---|---|
| 1 | API route coverage | 21 toko-rs endpoints vs ~55 Medusa store endpoints + ~20 admin product sub-routes |
| 2 | Data model field coverage | ~52 missing P1 fields/models across 6 entities |
| 3 | Business logic correctness | 5 P1 bugs, 8 known divergences, 15 P2 deferred |
| 4 | Validation / middleware / error handling | 5 gaps |
| 5 | Store API response shapes | 10 breakages that crash Medusa frontends |

---

## A. API Route Coverage

### toko-rs Endpoints (21 total)

| # | Method | Path | Medusa Match |
|---|---|---|---|
| 1 | POST | `/admin/products` | MATCH |
| 2 | GET | `/admin/products` | MATCH |
| 3 | GET | `/admin/products/{id}` | MATCH |
| 4 | POST | `/admin/products/{id}` | MATCH |
| 5 | DELETE | `/admin/products/{id}` | MATCH |
| 6 | POST | `/admin/products/{id}/variants` | MATCH |
| 7 | GET | `/store/products` | MATCH |
| 8 | GET | `/store/products/{id}` | MATCH |
| 9 | POST | `/store/carts` | MATCH |
| 10 | GET | `/store/carts/{id}` | MATCH |
| 11 | POST | `/store/carts/{id}` | MATCH |
| 12 | POST | `/store/carts/{id}/line-items` | MATCH |
| 13 | POST | `/store/carts/{id}/line-items/{line_id}` | MATCH |
| 14 | DELETE | `/store/carts/{id}/line-items/{line_id}` | MATCH |
| 15 | POST | `/store/carts/{id}/complete` | MATCH |
| 16 | GET | `/store/orders` | MATCH |
| 17 | GET | `/store/orders/{id}` | MATCH |
| 18 | POST | `/store/customers` | MATCH |
| 19 | GET | `/store/customers/me` | MATCH |
| 20 | POST | `/store/customers/me` | MATCH |
| 21 | GET | `/health` | toko-rs-specific |

### Missing Medusa Store Route Groups (P2+)

| Domain | Endpoints |
|---|---|
| Regions | 2 |
| Currencies | 2 |
| Collections | 2 |
| Product Categories | 2 |
| Product Tags | 2 |
| Product Types | 2 |
| Product Variants | 2 |
| Shipping Options | 2 |
| Return Reasons | 2 |
| Returns | 1 |
| Payment Providers | 1 |
| Payment Collections | 2+ |
| Locales | 1 |
| Cart shipping/promotions/taxes | 5 |
| Customer addresses | 5 |
| Order transfers | 4 |

### Missing Medusa Admin Product Sub-Routes (P2+)

| Endpoint | Method |
|---|---|
| `/admin/products/{id}/variants` | GET (list) |
| `/admin/products/{id}/variants/{variant_id}` | GET, POST (update), DELETE |
| `/admin/products/{id}/variants/batch` | POST |
| `/admin/products/{id}/options` | GET, POST |
| `/admin/products/{id}/options/{option_id}` | GET, POST (update), DELETE |
| `/admin/products/batch` | POST |
| `/admin/products/export` | POST |
| `/admin/products/import[s]` | POST |

---

## B. Data Model Field Coverage

### Cart (`src/cart/models.rs` vs `vendor/medusa/packages/modules/cart/src/models/cart.ts`)

| Field | Medusa | toko-rs | Status |
|---|---|---|---|
| `id` | `model.id(prefix:"cart")` | `String` | OK |
| `region_id` | `model.text().nullable()` | — | **MISSING** |
| `customer_id` | `model.text().nullable()` | `Option<String>` | OK |
| `sales_channel_id` | `model.text().nullable()` | — | MISSING P2 |
| `email` | `model.text().nullable()` | `Option<String>` | OK |
| `currency_code` | `model.text()` | `String` | OK |
| `metadata` | `model.json().nullable()` | `Option<Json<Value>>` | OK |
| `completed_at` | `model.dateTime().nullable()` | `Option<DateTime<Utc>>` | OK |
| `shipping_address` | `hasOne(Address)` separate table | `Option<Json<Value>>` inline | **INCOMPATIBLE** |
| `billing_address` | `hasOne(Address)` separate table | `Option<Json<Value>>` inline | **INCOMPATIBLE** |

### CartLineItem (`src/cart/models.rs` vs `vendor/medusa/packages/modules/cart/src/models/line-item.ts`)

| Field | Medusa | toko-rs | Status |
|---|---|---|---|
| `title` | `model.text()` | `String` | OK |
| `quantity` | `model.number()` | `i64` | OK |
| `unit_price` | `model.bigNumber()` | `i64` | **TYPE MISMATCH** (bigNumber vs int) |
| `variant_id` | `model.text().nullable()` | `Option<String>` | OK |
| `product_id` | `model.text().nullable()` | `Option<String>` | OK |
| `product_title` | `model.text().nullable()` | — | **MISSING** (in snapshot instead) |
| `variant_sku` | `model.text().nullable()` | — | **MISSING** (in snapshot instead) |
| `variant_title` | `model.text().nullable()` | — | **MISSING** (in snapshot instead) |
| `variant_option_values` | `model.json().nullable()` | — | **MISSING** (in snapshot instead) |
| `requires_shipping` | `model.boolean().default(true)` | — | **MISSING** |
| `is_discountable` | `model.boolean().default(true)` | — | MISSING P2 |
| `is_giftcard` | `model.boolean().default(false)` | — | MISSING P2 |
| `is_tax_inclusive` | `model.boolean().default(false)` | — | **MISSING** |
| `compare_at_unit_price` | `model.bigNumber().nullable()` | — | MISSING P2 |
| `snapshot` | — | `Option<Json<Value>>` | toko-rs-specific (replaces 12 columns) |

### Order (`src/order/models.rs` vs `vendor/medusa/packages/modules/order/src/models/order.ts`)

| Field | Medusa | toko-rs | Status |
|---|---|---|---|
| `display_id` | `model.autoincrement()` | `i64` (via `_sequences`) | OK |
| `version` | `model.number().default(1)` | — | **MISSING** (order change system) |
| `status` | `model.enum(OrderStatus)` | `String` (CHECK constraint) | **INCOMPATIBLE** (untyped) |
| `shipping_address` | `hasOne(OrderAddress)` | `Option<Json<Value>>` inline | **INCOMPATIBLE** |
| `billing_address` | `hasOne(OrderAddress)` | `Option<Json<Value>>` inline | **INCOMPATIBLE** |

**Missing entirely**: `OrderItem` (fulfillment tracking per version — `fulfilled_quantity`, `shipped_quantity`, `delivered_quantity`)

### Product (`src/product/models.rs` vs `vendor/medusa/packages/modules/product/src/models/product.ts`)

| Field | Medusa | toko-rs | Status |
|---|---|---|---|
| `is_giftcard` | `model.boolean().default(false)` | — | **MISSING** |
| `discountable` | `model.boolean().default(true)` | — | **MISSING** |
| `status` | `model.enum(ProductStatus)` | `String` | **INCOMPATIBLE** (untyped) |

### ProductVariant (`src/product/models.rs` vs `vendor/medusa/packages/modules/product/src/models/product-variant.ts`)

| Field | Medusa | toko-rs | Status |
|---|---|---|---|
| `barcode` | `model.text().nullable()` | — | **MISSING** |
| `allow_backorder` | `model.boolean().default(false)` | — | **MISSING** |
| `manage_inventory` | `model.boolean().default(true)` | — | **MISSING** |
| `price` | **DOES NOT EXIST** | `i64` | **INCOMPATIBLE** (Medusa uses Pricing module) |

### Payment (`src/payment/models.rs` vs `vendor/medusa/packages/modules/payment/src/models/payment.ts`)

| Field | Medusa | toko-rs | Status |
|---|---|---|---|
| `provider_id` | `model.text()` | `provider: String` | **NAME MISMATCH** |
| `order_id` | **DOES NOT EXIST** | `String` | **INCOMPATIBLE** (Medusa links via PaymentCollection) |
| `status` | On PaymentCollection, not Payment | `String` | **INCOMPATIBLE** |
| `captured_at` | `model.dateTime().nullable()` | — | **MISSING** |
| `canceled_at` | `model.dateTime().nullable()` | — | **MISSING** |

**Missing entirely**: `PaymentCollection` model (Medusa's 3-layer payment architecture)

### Customer (`src/customer/models.rs` vs `vendor/medusa/packages/modules/customer/src/models/customer.ts`)

| Field | Medusa | toko-rs | Status |
|---|---|---|---|
| `email` | `model.text().nullable()` | `String` (non-nullable) | **INCOMPATIBLE** |

---

## C. P1 Bugs (business logic correctness)

### B1: `update_line_item` / `delete_line_item` don't check cart completion

**File**: `src/cart/repository.rs:199-242`

`add_line_item` correctly checks `completed_at.is_some()` and rejects mutations on completed carts. `update_line_item` and `delete_line_item` perform no such check. A client can change quantities or remove items on a completed cart, corrupting cart state.

**Medusa reference**: `validateCartStep` in `update-line-item-in-cart.ts` and `remove-line-item.ts` workflows.

**Fix**: Add the same `completed_at` guard to both methods.

### B2: `resolve_variant_options_tx` uses fragile title-based lookup

**File**: `src/product/repository.rs:321-327`

After `insert_variant_tx` creates a variant, `resolve_variant_options_tx` finds it by:
```sql
SELECT id FROM product_variants WHERE product_id = ? AND title = ? ORDER BY created_at DESC LIMIT 1
```

If two variants share the same title (e.g., "Default"), this returns the wrong variant. The `insert_variant_tx` return value (a `ProductVariant` with the generated ID) is discarded.

**Medusa reference**: `createProducts_` pre-generates variant IDs and attaches options in-memory before bulk create (no separate lookup needed).

**Fix**: Return the `ProductVariant` from `insert_variant_tx`, pass its `id` directly to `resolve_variant_options_tx`.

### B3: `resolve_variant_options_tx` silently swallows missing option values

**File**: `src/product/repository.rs:344-353`

When a `(option_title, value_string)` pair doesn't match any `product_option_values` row, the code silently skips it (`if let Some(val) { ... }` / else: nothing).

**Medusa reference**: `assignOptionsToVariants` throws `MedusaError(INVALID_DATA, "Option value ${val} does not exist for option ${key}")`.

**Fix**: Replace with `AppError::NotFound(...)` or `AppError::InvalidData(...)`.

### B4: No validation that variant options cover all product options

**File**: `src/product/repository.rs`

Medusa's `validateProductCreatePayload` enforces that if a product has options, every variant must provide a value for every option. toko-rs has no such validation.

**Fix**: After iterating variants, validate each variant's `options` map covers all created option titles.

### B5: No validation that variant option combinations are unique

**File**: `src/product/repository.rs`

Medusa's `checkIfVariantWithOptionsAlreadyExists` prevents two variants from having the same option value combination (e.g., two Size=XL, Color=Blue variants). toko-rs has no such check.

**Fix**: Collect option maps before insert and check for duplicates.

### B6: Product `status` not enum-validated

**File**: `src/product/types.rs:15`

`pub status: Option<String>` accepts any string. Medusa uses `z.nativeEnum(ProductStatus)` enforcing `"draft" | "published" | "proposed" | "rejected"`.

**Fix**: Create `ProductStatus` enum and derive `Deserialize` with `#[serde(rename_all = "snake_case")]`.

### B7: `admin_update_product` never calls `.validate()`

**File**: `src/product/routes.rs:66-73`

The update handler deserializes input but never calls `.validate()` on it, bypassing all validation constraints.

**Fix**: Add `.validate()?` call, same as the create handler.

### B8: No `deny_unknown_fields` on input types

**File**: All `src/*/types.rs`

Medusa uses `.strict()` on all Zod schemas (rejects unknown fields). toko-rs silently ignores misspelled fields, giving clients no feedback.

**Fix**: Add `#[serde(deny_unknown_fields)]` to all input structs.

### B9: `metadata` type too permissive

**File**: All `src/*/types.rs`

`metadata: Option<serde_json::Value>` accepts arrays, strings, numbers. Medusa validates `z.record(z.unknown())` (object with string keys only).

**Fix**: Change to `Option<HashMap<String, serde_json::Value>>` or `Option<serde_json::Value>` with custom validation requiring object type.

---

## D. Response Shape Breakages (Medusa frontend compatibility)

These findings describe fields that a Medusa frontend SDK expects but toko-rs doesn't provide. When the frontend accesses these fields, it gets `undefined` instead of the expected type, causing TypeError crashes.

### R1: Variant `price` vs `calculated_price` (CRITICAL)

**toko-rs**: `price: i64` (flat integer)
**Medusa**: `calculated_price: { calculated_amount, original_amount, raw_calculated_amount, is_calculated_price_tax_inclusive, ... }` (nested object)

Frontend code: `variant.calculated_price.calculated_amount` → TypeError

### R2: Missing `images` array on product (CRITICAL)

Medusa returns `images: StoreProductImage[]`. Frontend gallery rendering crashes.

### R3: Missing `is_giftcard`, `discountable` booleans on product (HIGH)

Frontend conditional logic: `if (product.is_giftcard)` → TypeError

### R4: Missing ~20 computed total fields on cart (CRITICAL)

Medusa returns: `subtotal`, `tax_total`, `discount_total`, `gift_card_total`, `shipping_total`, `item_total`, `item_subtotal`, `item_tax_total`, `original_total`, `original_subtotal`, `original_tax_total`, etc.

toko-rs only returns `item_total` and `total`.

Frontend: `cart.subtotal`, `cart.tax_total`, `cart.discount_total` → undefined

### R5: Missing ~20 fields on cart line items (HIGH)

Medusa line items have: `product_title`, `product_subtitle`, `variant_sku`, `variant_title`, `variant_option_values`, `requires_shipping`, `is_discountable`, `is_tax_inclusive`, plus all total fields.

### R6: Cart complete response not a discriminated union (CRITICAL)

**Medusa**: `StoreCompleteCartResponse = { type: "order", order } | { type: "cart", cart, error }`
**toko-rs**: Always returns `{ type: "order", order }` — error case never returns `{ type: "cart", cart, error }`

Frontend error handling: `if (response.type === "cart")` never matches.

### R7: Missing ~25 computed total fields on order (CRITICAL)

Same pattern as cart. Frontend order confirmation page: `order.subtotal`, `order.shipping_total`, `order.tax_total` → undefined

### R8: Missing `payment_status`, `fulfillment_status` on order (CRITICAL)

Medusa returns enum fields. Frontend: `order.payment_status`, `order.fulfillment_status` → undefined

### R9: Missing `addresses` array on customer (CRITICAL)

**Medusa**: `addresses: StoreCustomerAddress[]`
**toko-rs**: No `addresses` field

Frontend: `customer.addresses.map(...)`, `customer.addresses.length` → TypeError

### R10: Missing `fulfillments`, `shipping_methods` on order (HIGH)

Medusa returns these as arrays. Frontend tracking/shipping display breaks.

---

## E. Validation / Middleware / Error Handling Gaps

### V1: CORS is permissive

**File**: `src/lib.rs:37`

`CorsLayer::permissive()` allows all origins, methods, headers. Production-unsafe.

**Fix**: Read allowed origins from `AppConfig`, restrict methods to actual routes.

### V2: No `Forbidden` (403) error variant

**File**: `src/error.rs`

No `AppError::Forbidden` variant exists. Needed for future RBAC.

**Fix**: Add variant mapping to 403, code `invalid_state_error`, type `forbidden`.

### V3: No structured SQLite/Postgres error code mapping

**File**: `src/error.rs`

Medusa's `exception-formatter.ts` converts Postgres error codes (23505 duplicate, 23503 FK violation, 23502 null violation) into human-readable `MedusaError`. toko-rs exposes raw `sqlx::Error` details.

**Fix**: Map common error codes to `AppError::DuplicateError`, `AppError::NotFound`, `AppError::InvalidData`.

### V4: `FindParams.limit` has no upper bound

**File**: `src/types.rs`

A client can pass `limit=9999999` and potentially exhaust memory.

**Fix**: Validate `limit <= 100` (or similar).

### V5: Error message prefixes differ from Medusa

**File**: `src/error.rs`

toko-rs: `"Not Found: gone"`, `"Invalid Data: bad"`
Medusa: `"Order with id ... was not found"`, `"Invalid request"`

Clients parsing error messages will see different formats. Minor — Medusa does not guarantee message format.

---

## F. Known P1 Divergences (by design, documented in design.md)

1. No auth (JWT/session) — `X-Customer-Id` header stub
2. No admin auth — `/admin/*` routes fully open
3. No payment authorization flow — `status='pending', provider='manual'` only
4. No inventory management
5. No promotion/coupon/tax/shipping system
6. No region concept — single `DEFAULT_CURRENCY_CODE`
7. No event bus
8. Single `price: i64` on variant vs Medusa Pricing module
9. Inline JSON addresses vs dedicated address entities
10. `snapshot` JSON column vs 12 denormalized line item columns
11. No `OrderItem` (fulfillment tracking) — only `OrderLineItem`

---

## G. P2 Deferred

- Multi-currency pricing / price lists / Pricing module
- Customer address CRUD endpoints (5 routes)
- Product variant standalone CRUD endpoints (6 routes)
- Product option standalone CRUD endpoints (5 routes)
- Order transfer endpoints (4 routes)
- Cart shipping/promotions/taxes endpoints (5 routes)
- `OrderItem` entity (fulfillment quantity tracking)
- `PaymentCollection` entity (3-layer payment architecture)
- `CustomerAddress` entity (address book)
- Product tags, collections, categories, images, type entities
- Product dimensions and weight fields
- Cart `locale` field and translation system
- Idempotency for cart completion
- Distributed locking for cart operations
- 13 entire Medusa store domains (regions, currencies, collections, categories, shipping options, returns, payment providers, etc.)
- 25+ Medusa admin domains (orders admin, customers admin, fulfillments, promotions, campaigns, price lists, tax, inventory, stock locations, etc.)

---

## Summary Statistics

| Metric | Count |
|---|---|
| P1 Bugs | 9 |
| Response Shape Breakages | 10 |
| Validation/Middleware Gaps | 5 |
| Known Design Divergences | 11 |
| P2 Deferred Items | ~30+ |
| toko-rs endpoints | 21 |
| Medusa matched equivalents | 20 of 21 |
| Medusa store route files NOT covered | ~35 |
| Medusa admin domains NOT covered | ~25+ |

---

## Resolution Status

All findings mapped to Task 14 sub-groups. Corrections documented in `docs/audit-correction.md`.

### Section C — P1 Bugs (B1–B9)

| Finding | Description | Task | Status | audit-correction.md |
|---------|-------------|------|--------|---------------------|
| B1 | `update_line_item`/`delete_line_item` no cart completion check | 14a.1 | **FIXED** | Section 14a |
| B2 | Fragile title-based variant lookup | 14a.2 | **FIXED** | Section 14a |
| B3 | Silently swallows missing option values | 14a.3 | **FIXED** | Section 14a |
| B4 | No option coverage validation | 14a.4 | **FIXED** | Section 14a |
| B5 | No unique option combination check | 14a.5 | **FIXED** | Section 14a |
| B6 | Product `status` not enum-validated | 14a.6 | **FIXED** | Section 14a |
| B7 | `admin_update_product` never calls `.validate()` | 14a.7 | **FIXED** | Section 14a |
| B8 | No `deny_unknown_fields` on input types | 14b.1 | **FIXED** | Section 14b |
| B9 | `metadata` type too permissive | 14b.2 | **FIXED** | Section 14b |

### Section D — Response Shape Breakages (R1–R10)

| Finding | Description | Task | Status | audit-correction.md |
|---------|-------------|------|--------|---------------------|
| R1 | Variant `price` vs `calculated_price` | 14c.2 | **FIXED** — added `CalculatedPrice` struct | Section 14c |
| R2 | Missing `images` array on product | 14c.1 | **FIXED** — `images: Vec<String>` default `[]` | Section 14c |
| R3 | Missing `is_giftcard`, `discountable` | 14c.1 | **FIXED** — defaults `false`/`true` | Section 14c |
| R4 | Missing ~20 computed total fields on cart | 14c.3 | **FIXED** — 22 fields via `from_items()` | Section 14c |
| R5 | Missing ~20 fields on cart line items | 14c.7 | **FIXED** — `#[sqlx(skip)]` stubs | Section 14c |
| R6 | Cart complete not discriminated union | — | **DEFERRED P2** — error case `{ type: "cart", cart, error }` requires `payment_session` table | — |
| R7 | Missing ~25 computed total fields on order | 14c.3 | **FIXED** — 22 fields via `from_items()` | Section 14c |
| R8 | Missing `payment_status`, `fulfillment_status` | 14c.5 | **FIXED** — stubs `"not_paid"`, `"not_fulfilled"` | Section 14c |
| R9 | Missing `addresses` array on customer | 14f.1–14f.6 | **FIXED** — `CustomerWithAddresses` wrapper + `CustomerAddress` model | Section 14f |
| R10 | Missing `fulfillments`, `shipping_methods` on order | 14c.6 | **FIXED** — empty array stubs | Section 14c |

### Section E — Validation / Middleware / Error Handling (V1–V5)

| Finding | Description | Task | Status | audit-correction.md |
|---------|-------------|------|--------|---------------------|
| V1 | CORS is permissive | 14d.1 | **FIXED** — config-driven CORS via `AppConfig.cors_origins` | Section 14d |
| V2 | No `Forbidden` (403) error variant | 14b.4 | **FIXED** — `AppError::Forbidden` added | Section 14b |
| V3 | No structured SQLite error code mapping | 14d.2 | **FIXED** — `map_sqlite_constraint()` in `error.rs` | Section 14d |
| V4 | `FindParams.limit` no upper bound | 14b.3 | **FIXED** — `capped_limit()` max 100 | Section 14b |
| V5 | Error message prefixes differ from Medusa | — | **KNOWN DIVERGENCE** — minor, Medusa does not guarantee message format. Documented in design.md. | — |

### Section B — Data Model Field Coverage

Fields marked **MISSING** or **INCOMPATIBLE** in the data model tables are tracked in two categories:

**P1 stubs (resolved in 14c/14f):**
- `Product.is_giftcard`, `Product.discountable`, `Product.images` → 14c.1
- `ProductVariant.calculated_price` → 14c.2
- `CartLineItem.requires_shipping`, `is_tax_inclusive` → 14c.7
- `CustomerAddress` model + `Customer.addresses` → 14f
- `Order.payment_status`, `fulfillment_status`, `fulfillments`, `shipping_methods` → 14c.5/14c.6
- Cart/Order 22 computed total fields → 14c.3

**P2 deferred (by design):**
- `Cart.region_id`, `sales_channel_id` → region module
- `Cart.shipping_address`/`billing_address` inline vs dedicated table → P2 address entities
- `CartLineItem.product_title`, `variant_sku`, etc. → stored in `snapshot` JSON (by design)
- `CartLineItem.is_giftcard`, `compare_at_unit_price` → P2
- `Order.version` → order change system
- `Order.shipping_address`/`billing_address` inline vs dedicated → P2
- `OrderItem` entity → fulfillment quantity tracking
- `ProductVariant.barcode`, `allow_backorder`, `manage_inventory` → inventory module
- `PaymentRecord` field mismatches → PaymentCollection module
- `Customer.email` nullable → P2 guest checkout refinement

### Verification

- **117 tests pass**, clippy clean, zero warnings
- All Task 14 sub-groups (14a–14f) and Task 13 complete
- Full change log in `docs/audit-correction.md` (sections 14a–14d, 14f, 14c)
