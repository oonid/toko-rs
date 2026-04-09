# P1 Core MVP — Post-Audit Compatibility Report (Task 12)

**Date**: 2026-04-09  
**Scope**: Full comparison of toko-rs P1 implementation against Medusa vendor reference (`vendor/medusa/`)  
**Audited**: All 21 endpoints, all 12 migration files, all model/type definitions, error handling, response shapes  

## Audit Methodology

### Sources Compared

| Source | Location | Purpose |
|---|---|---|
| toko-rs routes | `src/*/routes.rs` | Endpoint definitions, handlers, middleware |
| toko-rs types | `src/*/types.rs`, `src/*/models.rs` | Request/response shapes, DB models |
| toko-rs migrations | `migrations/*.sql`, `migrations/sqlite/*.sql` | DB schema (PG + SQLite) |
| toko-rs errors | `src/error.rs` | Error response format and mappings |
| Medusa route handlers | `vendor/medusa/packages/medusa/src/api/*/route.ts` | Request/response contracts, validators |
| Medusa validators | `vendor/medusa/packages/medusa/src/api/*/validators.ts` | Input validation schemas |
| Medusa query configs | `vendor/medusa/packages/medusa/src/api/*/query-config.ts` | Default fields, response shapes |
| Medusa middlewares | `vendor/medusa/packages/medusa/src/api/*/middlewares.ts` | Auth, validation chains |
| Medusa models | `vendor/medusa/packages/modules/*/src/models/*.ts` | DB column definitions, relationships, indexes |
| Medusa error handler | `vendor/medusa/packages/core/framework/src/http/middlewares/error-handler.ts` | Error format and status mapping |
| Medusa errors | `vendor/medusa/packages/core/utils/src/common/errors.ts` | Error type constants |
| OAS Error schema | `specs/store.oas.yaml` | Canonical error field definitions |

### Audit Process

1. **Endpoint audit**: Catalogued all 21 toko-rs endpoints (method, path, request body, response shape, middleware, errors) and compared against Medusa's route files
2. **Response shape audit**: Read all toko-rs model/type Rust files to determine exact JSON serialization, compared against Medusa's TypeScript response types and query configs
3. **DB schema audit**: Read all 12 migration files (PG + SQLite), compared column-for-column against Medusa's TypeScript model decorators (columns, indexes, relationships, cascades)
4. **Error handling audit**: Read toko-rs `error.rs` and compared against Medusa's error-handler.ts switch statement and OAS Error schema enums
5. **Business logic audit**: Reviewed repository implementations for edge cases, atomicity, and constraint handling

---

## Findings

### HIGH Severity (breaks Medusa client compatibility)

#### H1. Line item DELETE response shape mismatch

**Location**: `src/cart/routes.rs:74-80`

| Aspect | toko-rs | Medusa |
|---|---|---|
| Handler returns | `Json<CartResponse>` → `{ cart: CartWithItems }` | `StoreLineItemDeleteResponse` → `{ id, object: "line-item", deleted: true, parent: StoreCart }` |

**Impact**: A Medusa frontend checking `response.deleted === true` or `response.object === "line-item"` will fail. The toko-rs response has no `deleted`, `object`, or `id` fields at the top level.

**Evidence**: Medusa's `store/carts/[id]/line-items/[line_id]/route.ts` returns `StoreLineItemDeleteResponse` which is `DeleteResponseWithParent<"line-item", StoreCartResponse>`. The handler at line 36-62 explicitly constructs `{ id, object: "line-item", deleted: true, parent: cart }`.

---

#### H2. Cart complete response has extra top-level `payment`

**Location**: `src/order/types.rs:22-27`, `src/order/routes.rs:23-37`

| Aspect | toko-rs | Medusa |
|---|---|---|
| Response type | `CartCompleteResponse { response_type, order, payment }` | `StoreCompleteCartResponse` — discriminated union |
| Success shape | `{ type: "order", order: ..., payment: PaymentRecord }` | `{ type: "order", order: StoreOrder }` |
| Error shape | Not supported | `{ type: "cart", cart: StoreCart, error: { message, name, type } }` |

**Impact**: 
1. Extra `payment` field breaks clients expecting strict `{ type, order }` shape
2. `payment` is `PaymentRecord` (required), not `Option` — cannot represent the error case

**Why we can't fully match Medusa here**: Medusa nests `payment_collections` (a separate table) inside the order via `query.graph()`. The order detail workflow forcibly adds `payment_collections.*` to query fields. However, toko-rs P1 does not implement `payment_collections` or `payment_session` tables — the payment module is collapsed to a single `payment_records` table (see schema mapping in `docs/database-foundation.md`). The `{ type: "cart", cart, error }` error case requires payment session logic which also depends on deferred tables.

**P1 fix**: Remove top-level `payment`, return `{ type: "order", order }` only. `payment_collections` is optional (`?`) in Medusa's `StoreOrder` TypeScript type, so omitting it is valid. The error case is deferred to P2.

**Evidence**: Medusa's `store/carts/[id]/complete/route.ts` returns either `{ type: "order", order }` on success or `{ type: "cart", cart, error }` on payment/inventory failure. Payment collections are accessed via `order.payment_collections`.

---

#### H3. Order GET response has extra top-level `payment`

**Location**: `src/order/types.rs:7-11`, `src/order/routes.rs:60-67`

| Aspect | toko-rs | Medusa |
|---|---|---|
| Response | `{ order: OrderWithItems, payment: PaymentRecord \| null }` | `{ order: StoreOrder }` |

**Impact**: Extra `payment` key at top level. Medusa clients looking for `payment_collections` inside the order won't find it; clients not expecting `payment` at the root will see unexpected data.

**Why we can't nest `payment_collections`**: Medusa's `StoreOrder` has `payment_collections?: StorePaymentCollection[]` (optional). Medusa's order detail workflow forcibly injects `"payment_collections.*"` into query fields and its aggregate status functions iterate over it without null checks. However, toko-rs P1 does not implement the `payment_collections` / `payment_session` / `payment` tables (collapsed to `payment_records` — see schema mapping in `docs/database-foundation.md`). The order object will be a valid subset of `StoreOrder`.

**P1 fix**: Remove top-level `payment`, return `{ order }` only.

**Evidence**: Medusa's `store/orders/[id]/route.ts` returns `StoreOrderResponse` which is `{ order: StoreOrder }`.

---

### MEDIUM Severity (semantic divergence, not immediately breaking)

#### M1. Conflict error `type` field mismatch

**Location**: `src/error.rs:58`

| Aspect | toko-rs | Medusa |
|---|---|---|
| `AppError::Conflict` type | `"unexpected_state"` | `"conflict"` |
| `AppError::Conflict` code | `"invalid_state_error"` | `"invalid_state_error"` |
| HTTP status | 409 | 409 |

**Impact**: The OAS enum includes `"conflict"` as a valid type value. Medusa's error handler at `error-handler.ts:47-49` maps `CONFLICT` to `type: "conflict"`. toko-rs remaps to `"unexpected_state"`. A client branching on `error.type === "conflict"` won't match.

**Note**: The previous audit (Task 7a.1) intentionally changed FROM `"conflict"` TO `"unexpected_state"` based on the spec table. However, direct comparison with Medusa's error-handler.ts shows the spec table was incorrect — Medusa DOES use `type: "conflict"` for 409 responses. This should be reverted.

**Evidence**: 
- `vendor/medusa/packages/core/utils/src/common/errors.ts:16`: `CONFLICT: "conflict"`
- `vendor/medusa/packages/core/framework/src/http/middlewares/error-handler.ts:47-49`: `case MedusaError.Types.CONFLICT: statusCode = 409; errObj.code = INVALID_STATE_ERROR;`

---

#### M2. SQLite email uniqueness stricter than PG/Medusa

**Location**: `migrations/sqlite/002_customers.sql:5`

| Aspect | PG | SQLite |
|---|---|---|
| Constraint | `UNIQUE (email, has_account) WHERE deleted_at IS NULL` | `email TEXT UNIQUE NOT NULL` |
| Same email, guest+registered | Allowed | Blocked |
| Soft-deleted rows excluded | Yes | No |

**Impact**: No practical impact in P1 (no guest customer path), but blocks guest checkout in P2. Tests run against SQLite so any future guest+registered test would fail.

**Evidence**: Medusa's `vendor/medusa/packages/modules/customer/src/models/customer.ts` defines `@Index({ name: "...", on: ["email", "has_account"], unique: true, where: "deleted_at IS NULL" })`.

---

#### M3. `_sequences` table created but unused

**Location**: `migrations/004_orders.sql` (creates table), `src/order/repository.rs:46-49` (was using `MAX+1`)

The migration creates `_sequences` with seed `('order_display_id', 0)`, but `create_from_cart` was using `SELECT COALESCE(MAX(display_id), 0) + 1 FROM orders` instead.

**Fix**: Adopted `_sequences` for `display_id` generation. Replaced `MAX(display_id)+1` with atomic `UPDATE _sequences SET value = value + 1 WHERE name = 'order_display_id' RETURNING value` inside the transaction. This eliminates the race condition between SELECT and INSERT under concurrent requests. The `_sequences` row is updated atomically within the same transaction as the order INSERT — no gap between reading and writing the sequence value.

---

#### M4. Empty cart completion returns 409 instead of 400

**Location**: `src/order/repository.rs:43-44`

Empty cart completion returns `AppError::Conflict("Cannot complete an empty cart")` → HTTP 409 with `type: "unexpected_state"`. This is semantically incorrect — an empty cart is an invalid request (client error), not a conflict (concurrent modification). Medusa would return a validation error.

---

### LOW Severity (by-design simplifications or minor gaps)

#### L1. Missing indexes on `cart_line_items`

Medusa defines:
- `IDX_line_item_variant_id ON cart_line_items (variant_id) WHERE deleted_at IS NULL AND variant_id IS NOT NULL`
- `IDX_line_item_product_id ON cart_line_items (product_id) WHERE deleted_at IS NULL AND product_id IS NOT NULL`

toko-rs only indexes `cart_id`. Under load with many carts referencing the same variant, the cross-cart variant lookup in `add_line_item` would benefit from these indexes.

#### L2. Missing composite index on `product_variant`

Medusa has `IDX_product_variant_id_product_id ON product_variant (id, product_id) WHERE deleted_at IS NULL`. toko-rs doesn't have this composite index.

#### L3. Missing index on `carts.currency_code`

Medusa has `IDX_cart_curency_code ON cart (currency_code) WHERE deleted_at IS NULL`. toko-rs doesn't index this column.

#### L4. `product_variant_option` pivot has no unique constraint

The pivot table has `variant_id` and `option_value_id` columns but no `UNIQUE(variant_id, option_value_id)` constraint. Duplicate pivot rows are possible if the application code doesn't guard against them.

#### L5. Missing entities (P2+ scope)

The following Medusa entities are not implemented in P1 (by design):
- `product_images`, `product_tags`, `product_collections`, `product_categories` — product module extensions
- `cart_shipping_method`, `cart_line_item_adjustment`, `cart_line_item_tax_line` — shipping and tax
- `order_shipping_method`, `order_change`, `order_return`, `order_exchange`, `order_claim` — fulfillment lifecycle
- `customer_groups` — segmentation
- `payment_sessions`, `payment_collections` — payment workflow

#### L6. No admin auth on `/admin/*` routes

All admin endpoints accept unauthenticated requests. P1 simplification (X-Customer-Id header only for store routes).

#### L7. Missing cart fields

Medusa carts include `region_id`, `sales_channel_id`, `locale`, `shipping_methods`, `promotions` — all P2 scope.

---

## What's Correct (Confirmed Compatible)

| Area | Status | Details |
|---|---|---|
| **Endpoint paths** (21 routes) | Match | All paths match Medusa's route structure |
| **HTTP methods** | Match | POST for create+update, DELETE for soft-delete, GET for reads |
| **Error schema** `{code, type, message}` | Match | Field names and ordering match Medusa's error-handler.ts |
| **Error code values** (except Conflict) | Match | `invalid_request_error`, `api_error`, `unknown_error`, `invalid_state_error` |
| **Error type values** (except Conflict) | Match | `not_found`, `invalid_data`, `duplicate_error`, `unauthorized`, `unexpected_state`, `database_error` |
| **Product response** `{product: ...}` | Match | Correct wrapper |
| **Product list** `{products, count, offset, limit}` | Match | Correct pagination wrapper |
| **Product delete** `{id, object, deleted}` | Match | Correct delete confirmation |
| **Cart response** `{cart: ...}` | Match | Correct wrapper (except line item DELETE) |
| **Customer response** `{customer: ...}` | Match | Correct wrapper |
| **Order list** `{orders, count, offset, limit}` | Match | Correct pagination wrapper |
| **Soft-delete** via `deleted_at` | Match | Matches Medusa pattern |
| **Prefixed ULID IDs** | Match | `prod_`, `variant_`, `cart_`, `cali_`, `order_`, `cus_` prefixes |
| **`product_variant_option` pivot name** | Match | Matches Medusa's `pivotTable: "product_variant_option"` |
| **Partial unique on `products.handle`** | Match | `WHERE deleted_at IS NULL` |
| **Same-variant quantity merge** | Match | Merges quantity when adding duplicate variant |
| **Completed-cart guard** | Match | Rejects update/add-item on completed carts |
| **Atomic cart-to-order transaction** | Match | Single SQL tx for order+items+payment+cart completion |
| **Cart line item `snapshot` JSON** | Match (P1 equivalent) | Replaces Medusa's 12 denormalized columns |
| **Default pagination limit** | Match | 20 (matches Medusa store default) |
| **ID casing** | Match | Lowercase ULID |
| **Product status CHECK** | Match | `('draft','published','proposed','rejected')` |
| **Order status CHECK** | Match | `('pending','completed','canceled','requires_action','archived')` |
| **Payment status CHECK** | Match | `('pending','authorized','captured','failed','refunded')` |
| **Default currency** | Match (config-driven) | IDR via `DEFAULT_CURRENCY_CODE` |

---

## Proposed Fixes (Task 12)

### 12a. Response shape incompatibilities (HIGH)

| Task | Fix | Files Changed |
|---|---|---|
| 12a.1 | Create `LineItemDeleteResponse { id, object, deleted, parent }` for line item DELETE | `src/cart/types.rs`, `src/cart/routes.rs` |
| 12a.2 | Remove top-level `payment` from `CartCompleteResponse`; return `{ type: "order", order }` only. `payment_collections` not nestable in P1 (deferred table). Error case `{ type: "cart" }` also deferred | `src/order/types.rs`, `src/order/routes.rs` |
| 12a.3 | Change `OrderResponse` to `{ order }` only. Order object is a valid subset of `StoreOrder` (missing optional `payment_collections`, `fulfillments`, `shipping_methods` from deferred tables) | `src/order/types.rs`, `src/order/routes.rs` |

### 12b. Error handling divergences (MEDIUM)

| Task | Fix | Files Changed |
|---|---|---|
| 12b.1 | Change `AppError::Conflict.error_type()` from `"unexpected_state"` to `"conflict"` | `src/error.rs`, all test files asserting conflict type |
| 12b.2 | Change empty cart error from `AppError::Conflict` to `AppError::InvalidData` | `src/order/repository.rs` |

### 12c. Database schema gaps (MEDIUM)

| Task | Fix | Files Changed |
|---|---|---|
| 12c.1 | Fix SQLite email uniqueness to match PG partial composite unique | `migrations/sqlite/002_customers.sql` |
| 12c.2 | Add `UNIQUE(variant_id, option_value_id)` to pivot table | `migrations/001_products.sql`, `migrations/sqlite/001_products.sql` |
| 12c.3 | Adopt `_sequences` for display_id: replace `MAX(display_id)+1` with atomic `UPDATE _sequences SET value = value + 1 RETURNING value` | `src/order/repository.rs` |

### 12d. Missing indexes (LOW)

| Task | Fix | Files Changed |
|---|---|---|
| 12d.1 | Add variant_id index on cart_line_items | Both `003_carts.sql` migrations |
| 12d.2 | Add product_id index on cart_line_items | Both `003_carts.sql` migrations |
| 12d.3 | Add currency_code index on carts | Both `003_carts.sql` migrations |

### 12e. Verification

| Task | Fix |
|---|---|
| 12e.1 | Update contract tests for new response shapes |
| 12e.2 | Update integration tests for new error type values and response shapes |
| 12e.3 | Verify `cargo test` passes, clippy clean |
| 12e.4 | Update `docs/audit-correction.md` |
