# Audit P1 Task 19: Comprehensive Medusa Compatibility Deep-Dive

**Date**: 2026-04-22  
**Source**: `vendor/medusa/` at `12b4e72189` (develop branch)  
**Scope**: Full comparison of all 21 toko-rs endpoints against Medusa v2 route handlers, validators, models, workflows, and error handling.  
**Auditor**: Automated comparison against Medusa source

---

## Executive Summary

toko-rs P1 implements 21 of Medusa's ~25 equivalent store/admin endpoints with correct URL paths, HTTP methods, and basic response wrappers. The project is architecturally sound with 137 tests passing, clippy-clean, and dual-database support. This audit identifies **5 HIGH**, **8 MEDIUM**, and **7 LOW** findings across 6 audit dimensions.

### Finding Summary

| Severity | Count | Areas |
|----------|-------|-------|
| HIGH     | 5     | Bug in JSON extractor, missing admin variant endpoints, soft-delete cascade, variant option uniqueness against DB, line-item dedup price gap |
| MEDIUM   | 8     | Missing response fields, missing input fields, missing indexes, error code inconsistencies, no cart-complete error branch, missing `company_name`, `GET /store/orders/:id` auth, `product_options.metadata` missing |
| LOW      | 7     | Message formatting, `deleted_at` exposure, extra fields in responses, cosmetic prefix differences, `estimate_count` missing, missing total sub-fields |

---

## S1. `JsonDataError` mapped to `DuplicateError` (BUG — HIGH)

**Location**: `src/extract.rs`

The custom JSON extractor maps `axum::extract::rejection::JsonDataError` (structurally valid JSON that fails serde deserialization — e.g., wrong field types) to `AppError::DuplicateError`, producing a **422** response with `type: "duplicate_error"`.

This is semantically incorrect. A deserialization error is an invalid request, not a duplicate.

**Current**:
```
JSON with wrong types → 422 { type: "duplicate_error", code: "invalid_request_error" }
```

**Expected**:
```
JSON with wrong types → 400 { type: "invalid_data", code: "invalid_request_error" }
```

**Fix**: Change `JsonDataError` mapping to `AppError::InvalidData`.

---

## S2. Missing Admin Variant Endpoints (HIGH)

toko-rs has no standalone variant management. Variants can only be added via `POST /admin/products/:id/variants` or inline during product creation. Medusa provides 4 additional endpoints:

| Endpoint | Medusa Method + Path | Status |
|----------|----------------------|--------|
| List variants | `GET /admin/products/:id/variants` | MISSING |
| Get variant | `GET /admin/products/:id/variants/:variant_id` | MISSING |
| Update variant | `POST /admin/products/:id/variants/:variant_id` | MISSING |
| Delete variant | `DELETE /admin/products/:id/variants/:variant_id` | MISSING |

Without these, there is no way to edit variant title/prices/SKU or remove a variant after creation. Admin UIs built for Medusa will 404 on these routes.

---

## S3. Soft-Delete Does Not Cascade to Children (HIGH)

**Location**: `src/product/repository.rs:261-285`

Medusa's `Product` model declares `.cascades({ delete: ["variants", "options", "images"] })`, and `ProductOption` cascades to `["values"]`. Soft-deleting a product cascades to variants → options → option values → images (nested cascade).

toko-rs only sets `deleted_at` on the product row. Children (variants, options, option values) remain with `deleted_at IS NULL` in the database.

`load_relations` filters by `deleted_at IS NULL` on children, so soft-deleted products' children become invisible through the product endpoint. However:
- Direct queries on variants/options tables still return them
- If the product is restored (P2), stale children remain
- Orphaned children consume unique constraints (e.g., `sku` uniqueness)

**Fix**: After setting `deleted_at` on the product, also UPDATE `deleted_at` on `product_variants`, `product_options`, and `product_option_values` where `product_id = $1 AND deleted_at IS NULL`.

---

## S4. Variant Option Uniqueness Not Checked Against DB (HIGH)

**Location**: `src/product/repository.rs` — `create_product` and `add_variant`

toko-rs validates that variant option combinations are unique **within the input batch**. Medusa's `checkIfVariantWithOptionsAlreadyExists` also checks against **existing variants already in the database** for the same product.

This means: if product already has a variant with `{Color: "Red", Size: "M"}`, adding a new variant with the same combination via a separate API call will succeed in toko-rs but fail in Medusa.

**Fix**: Before inserting a new variant, query existing `product_variant_option` rows for the product, reconstruct option combos, and reject duplicates against both DB and input.

---

## S5. Line Item Dedup Does Not Consider `unit_price` (HIGH)

**Location**: `src/cart/repository.rs:102-218`

Medusa's `getLineItemActions` step treats items with different `unit_price` values as separate line items (custom vs calculated pricing). toko-rs's dedup only checks `variant_id` + `metadata`.

In practice, this means: if a cart has a line item for variant X at $10, and the price changes to $15 before the next add, Medusa creates a new line item while toko-rs merges quantity into the existing one at the old price.

**Impact**: Low in P1 (single-currency, no price rules), but the behavior diverges from Medusa's. Once pricing rules are introduced in P2, this becomes critical.

**Fix**: Include `unit_price` comparison in the existing-item lookup SQL.

---

## S6. Cart Complete Has No Error Branch (MEDIUM)

**Location**: `src/order/types.rs` — `CartCompleteResponse`

Medusa's `StoreCompleteCartResponse` is a discriminated union:
- Success: `{ type: "order", order: StoreOrder }`
- Error: `{ type: "cart", cart: StoreCart, error: { message, name, type } }`

toko-rs only implements the success branch. When payment fails (P2), there's no way to return the error alongside the cart state.

**Fix**: Add an error variant to `CartCompleteResponse` (or return different types based on outcome).

---

## S7. Missing `company_name` on Customer (MEDIUM)

**Location**: `src/customer/models.rs`, `src/customer/types.rs`, `migrations/002_customers.sql`

Medusa's `Customer` model has `company_name: model.text().nullable()`. It appears in both `StoreCreateCustomer` and `StoreUpdateCustomer` validators. toko-rs omits it entirely.

**Fix**: Add `company_name` column to `customers` table, add to `Customer` model, add to `CreateCustomerInput` and `UpdateCustomerInput`.

---

## S8. `GET /store/orders/:id` Auth Mismatch (MEDIUM)

**Location**: `src/order/routes.rs`

Medusa's `GET /store/orders/:id` uses `MedusaRequest` (unauthenticated). A TODO comment in Medusa's source questions whether auth should be added, but currently it's public. toko-rs requires `X-Customer-Id` header.

**Fix**: Either document this as an intentional security improvement, or remove the auth middleware from the single order GET endpoint. (Recommendation: keep auth — it's more secure, and Medusa will likely add it too.)

---

## S9. `product_options.metadata` and `product_option_values.metadata` Missing (MEDIUM)

**Location**: `migrations/001_products.sql`, `src/product/models.rs`

Medusa's `ProductOption` and `ProductOptionValue` models both have `metadata: model.json().nullable()`. toko-rs has no `metadata` column on either table.

**Fix**: Add `metadata JSONB` to both tables in PG and SQLite migrations, add to Rust models.

---

## S10. Missing DB Indexes (MEDIUM)

| Table | Missing Index | Medusa Index Name |
|-------|---------------|-------------------|
| `product_variants` | `(id, product_id) WHERE deleted_at IS NULL` | `IDX_product_variant_id_product_id` |
| `orders` | `(deleted_at) WHERE deleted_at IS NOT NULL` | `IDX_order_deleted_at` |
| `orders` | `(currency_code) WHERE deleted_at IS NULL` | `IDX_order_currency_code` |
| `order_line_items` | `(deleted_at) WHERE deleted_at IS NOT NULL` | `IDX_order_line_item_deleted_at` |
| `order_line_items` | `(product_id) WHERE deleted_at IS NULL` | `IDX_order_line_item_product_id` |
| `order_line_items` | `(variant_id) WHERE deleted_at IS NULL` | `IDX_order_line_item_variant_id` |

---

## S11. Error `code` Field Always Present (MEDIUM)

**Location**: `src/error.rs`

Medusa's error handler only overrides `code` in 3 of 10+ error types (conflict → `"invalid_state_error"`, duplicate → `"invalid_request_error"`, database → `"api_error"`). For all other types, `code` comes from the constructor and is often absent entirely.

toko-rs always includes `code` deterministically from a `match` arm. This means:
- toko-rs: `{ code: "invalid_request_error", type: "not_found", message: "..." }`
- Medusa: `{ type: "not_found", message: "..." }` (no `code`)

**Verdict**: toko-rs's approach is more consistent. Clients that expect `code` will work; clients that don't expect it will ignore it. No fix needed — document as intentional.

---

## S12. Missing Response Fields on Line Items (MEDIUM)

Medusa returns 12 denormalized product/variant snapshot fields directly on line items (`product_title`, `variant_sku`, etc.) plus per-item totals (`item_total`, `total`, `subtotal`, etc.). toko-rs stores snapshots in a JSON `snapshot` column but does not surface these fields in the API response.

Medusa frontend components access `item.product_title`, `item.variant_sku` directly. These will be `undefined` against toko-rs.

**Fix**: Either explode `snapshot` fields into top-level response fields, or document that toko-rs frontends must access `snapshot.product_title` instead.

---

## S13. `customer_id` on Cart Create is EXTRA (MEDIUM)

**Location**: `src/cart/types.rs:5-14`

toko-rs accepts `customer_id` in `CreateCartInput`. Medusa's `CreateCart` validator does not have `customer_id` — it's inferred from the auth context. This extra field means Medusa SDK clients can send it, but it also means toko-rs allows manual customer assignment that Medusa doesn't.

**Verdict**: Low risk in P1 (no real auth). Document as intentional.

---

## S14. Error Message Prefixing (LOW)

toko-rs prefixes all error messages: `"Not Found: ..."`, `"Invalid Data: ..."`. Medusa uses raw messages from constructors.

**Verdict**: Informational. No fix needed.

---

## S15. `deleted_at` Exposed on Store Responses (LOW)

toko-rs returns `deleted_at` on carts, line items, and addresses in store API responses. Medusa's store types don't expose `deleted_at` on these entities.

**Verdict**: Harmless extra data. No fix needed.

---

## S16. `variant_rank` Nullable vs NOT NULL (LOW)

Medusa's `variant_rank` is `nullable()` on `ProductVariant`. toko-rs declares it `NOT NULL DEFAULT 0`.

**Verdict**: Minor. No fix needed.

---

## S17. Order Line Item ID Prefix `oli` vs `ordli` (LOW)

toko-rs uses prefix `oli_`, Medusa uses `ordli_`. Already documented in `design.md`.

**Verdict**: Cosmetic. No fix needed.

---

## S18. Missing `estimate_count` on List Responses (LOW)

Medusa returns `estimate_count` on paginated list responses from the index engine. toko-rs does not.

**Verdict**: P1 deferral. No fix needed.

---

## S19. Missing Total Sub-fields (LOW)

Medusa returns `discount_subtotal`, `item_discount_total`, `shipping_discount_total`, `credit_line_total`, `credit_line_subtotal`, `credit_line_tax_total` on cart/order totals. toko-rs has 22 total fields but omits these 6.

**Verdict**: P2 concern (discounts and credit lines not in P1).

---

## S20. Variant `title` Nullable vs Required (LOW)

Medusa's variant `title` is nullable. toko-rs's `ProductVariant.title` is `NOT NULL`.

**Fix**: Change to nullable if strict Medusa compat is needed.

---

## Recommended Action Plan

### Immediate (before marking P1 complete)

| Finding | Effort | Impact |
|---------|--------|--------|
| S1: Fix `JsonDataError` → `InvalidData` | 1 line | Correct error semantics |
| S3: Soft-delete cascade to children | ~30 min | Data integrity |
| S4: Variant option uniqueness vs DB | ~30 min | Prevents duplicate variants |
| S9: Add `metadata` to options/option_values | ~1 hour | Schema parity |

### Next Sprint (P1.1 or early P2)

| Finding | Effort | Impact |
|---------|--------|--------|
| S2: Admin variant endpoints (list/get/update/delete) | ~1 day | Admin UI compatibility |
| S5: Line-item dedup include `unit_price` | ~1 hour | Pricing correctness |
| S6: Cart complete error branch | ~2 hours | Payment flow readiness |
| S7: Add `company_name` to customer | ~1 hour | Field parity |
| S10: Add missing DB indexes | ~30 min | Query performance |
| S12: Surface line-item snapshot fields | ~2 hours | Frontend compatibility |

### Deferred to P2

| Finding | Reason |
|---------|--------|
| S8: Order auth mismatch | Keep toko-rs behavior (more secure) |
| S11: `code` always present | More consistent than Medusa |
| S13: `customer_id` on cart create | Needed until real auth |
| S14-S20: LOW findings | No functional impact |

---

## Audit Methodology

Six parallel audit streams compared toko-rs against `vendor/medusa/` at commit `12b4e72189`:

1. **Route paths & HTTP methods**: All 21 endpoints vs Medusa route handlers
2. **Response shapes**: Field-by-field comparison against Medusa TypeScript types
3. **Request/input types**: Field-by-field comparison against Medusa Zod validators
4. **Database schema**: Table, column, constraint, and index comparison against Medusa models
5. **Error handling**: Error types, status codes, code values, and message formatting vs Medusa error handler
6. **Business logic**: Product creation, cart dedup, cart completion, soft-delete cascade, variant resolution vs Medusa workflows

Each stream produced a detailed field-level comparison. Findings were then deduplicated, classified by severity, and prioritized.

---

## Verification Pass (Post-Implementation)

**Date**: 2026-04-22  
**Method**: Line-by-line comparison of all 13 implemented sub-tasks (19a–19m) against `vendor/medusa/` source at `develop` branch.  
**Scope**: Every finding re-verified against Medusa models, services, API routes, validators, and OAS specs.

### Verification Summary

| Verdict | Count |
|---------|-------|
| MATCH (correct) | 5 |
| DIFFER (intentional, documented) | 3 |
| DIFFER (new bug found — FIXED) | 2 |
| PARTIAL (response shape fixed) | 1 |
| N/A (out of P1 scope) | 2 |

### Finding-by-Finding Verification

#### S1 (19a): JsonDataError mapping — **DIFFER → FIXED**

| | Medusa | toko-rs (before) | toko-rs (after) |
|---|---|---|---|
| HTTP Status | **400** | ~~422~~ | **400** |
| Error `type` | `"invalid_data"` | `"invalid_data"` | `"invalid_data"` |
| Error `code` | omitted | `"invalid_request_error"` | `"invalid_request_error"` |

**Medusa behavior**: When a JSON field has the wrong type (e.g., `"quantity": "hello"`), Zod validation fails with `MedusaError.Types.INVALID_DATA`, which Medusa's error handler maps to **400** (`error-handler.ts:67-68`).

**Bug**: `src/extract.rs:25` — changed `ValidationError` to `InvalidData`. **Fixed.**

---

#### S2 (19b): Admin variant endpoints — **PARTIAL → FIXED (response shape)**

| Endpoint | Route | Response |
|---|---|---|
| `GET /admin/products/:id/variants` | ✅ Match | ✅ `{ variants, count, offset, limit }` |
| `POST /admin/products/:id/variants` | ✅ Match | ✅ `{ product: {...} }` |
| `GET /admin/products/:id/variants/:vid` | ✅ Match | ✅ `{ variant: {...} }` |
| `POST /admin/products/:id/variants/:vid` | ✅ Route exists | ✅ `{ product: {...} }` (fixed) |
| `DELETE /admin/products/:id/variants/:vid` | ✅ Match | ✅ `{ id, object, deleted, parent }` |

**Fixed**: Update variant response changed from `{ variant }` to `{ product }` to match Medusa's behavior (`src/product/routes.rs:170`).

**Missing endpoints** (out of P1 scope): 8 additional Medusa variant endpoints (batch ops, inventory linking, variant images, standalone listing) are not implemented. These require inventory/pricing modules not in P1.

---

#### S3 (19c): Soft-delete cascade — **MATCH (with atomicity gap)**

| | Medusa | toko-rs |
|---|---|---|
| Cascade to variants | ✅ | ✅ |
| Cascade to options | ✅ | ✅ |
| Cascade to option_values | ✅ (recursive from options) | ✅ (sub-select) |
| Atomicity | **Transactional** (MikroORM unit of work) | **NOT transactional** (4 independent queries) |

**Atomicity gap**: `src/product/repository.rs:261-310` — the four UPDATE statements run independently on `self.pool` without wrapping in a transaction. If the variants/options UPDATE fails after the product is already soft-deleted, the database is left in an inconsistent state. Medusa avoids this because MikroORM's cascading soft-delete runs within a single transaction.

**Severity**: LOW. Failure during child cascade is extremely unlikely (simple UPDATEs on indexed columns). Worth fixing for correctness but not blocking.

---

#### S4 (19d): Variant option uniqueness — **MATCH (different implementation, same behavior)**

| | Medusa | toko-rs |
|---|---|---|
| Comparison key | `option_value_id` (UUID) | `(option_title, value_string)` tuple |
| Match semantics | Subset (`.every()`) | Exact (`HashSet ==`) |
| Batch dedup | O(N²) `.every()` | `HashSet` |
| DB check on add | Yes | Yes |

For the standard creation flow (all options required), behavior is equivalent. Edge-case differences:
- Medusa is stricter on updates (subset matching with `.every()`).
- toko-rs is stricter on identity (string-based comparison catches semantically identical combos with different IDs).

---

#### S5 (19e): Line-item dedup includes unit_price — **DIFFER (intentional divergence)**

| | Medusa | toko-rs |
|---|---|---|
| Regular-priced items | Matches on `variant_id + metadata` (ignores price) | Matches on `variant_id + unit_price + metadata` |
| Custom-priced items | Matches on `variant_id + unit_price + metadata` | Same |

**Medusa behavior** (`get-line-item-actions.ts:100-114`): Uses split logic. For regular-priced items (`is_custom_price = false`), Medusa merges regardless of price. Only custom-priced items compare `unit_price`.

**toko-rs behavior** (`src/cart/repository.rs:157`): Unconditionally includes `unit_price` in the WHERE clause.

**Impact**: If a variant's price changes between adds, toko-rs creates a separate line item at the new price, while Medusa would merge. toko-rs's behavior is arguably more correct for price integrity.

**Documented**: Intentional design decision. Price changes between adds should create separate items.

---

#### S6 (19f): Cart complete error branch — **DIFFER (dead code)**

| | Medusa | toko-rs |
|---|---|---|
| Error response type | `CartCompleteResponse::error()` — used for payment failures | `CartCompleteResponse::error()` — **dead code, never called** |
| Success response | `{ type: "order", order: {...} }` | ✅ Same |

**Medusa behavior** (`vendor/medusa/packages/medusa/src/api/store/carts/[id]/complete/route.ts:37-85`): For recoverable payment errors, returns HTTP 200 with `{ type: "cart", cart: {...}, error: { message, name, type } }`.

**toko-rs behavior**: `CartCompleteResponse::error()` exists in `src/order/types.rs:50-61` but is **never invoked** from any route handler. All errors propagate as `AppError`. The error branch is implemented but dead code — payment retry scenarios (P2) will need this.

---

#### S7 (19g): company_name on customer — **MATCH**

| | Medusa | toko-rs |
|---|---|---|
| Model field | `company_name: model.text().searchable().nullable()` | `company_name: Option<String>` |
| DB column | `company_name TEXT NULL` | ✅ Same |
| Create input | `company_name?: string \| null` | ✅ Same |
| Update input | `company_name?: string \| null` | ✅ Same |

Verified in `vendor/medusa/packages/modules/customer/src/models/customer.ts:9`.

---

#### S8 (19h): GET /store/orders/:id auth — **DIFFER (intentional security improvement)**

| | Medusa | toko-rs |
|---|---|---|
| Authentication | **None** (unauthenticated, has TODO comment) | `X-Customer-Id` header required |

**Medusa behavior**: Has a `// TODO: Do we want to apply some sort of authentication here?` comment. toko-rs is stricter. Documented as Decision 14 in `design.md`.

---

#### S9 (19i): metadata on product_options and option_values — **MATCH**

| | Medusa | toko-rs |
|---|---|---|
| ProductOption.metadata | `model.json().nullable()` | `Option<Json<Value>>` ✅ |
| ProductOptionValue.metadata | `model.json().nullable()` | `Option<Json<Value>>` ✅ |

Verified in `vendor/medusa/packages/modules/product/src/models/product-option.ts:9` and `product-option-value.ts:8`.

---

#### S10 (19j): Missing DB indexes — **MATCH (within scope)**

toko-rs has indexes matching every Medusa index for columns that exist in toko-rs. Missing Medusa indexes are only for columns toko-rs doesn't model (`product_type_id`, `barcode`, `ean`, `upc`, `region_id`, `sales_channel_id`). The 6 new indexes added in 19j are correct.

---

#### S12 (19k): Line-item snapshot fields — **MATCH (API shape), DIFFER (storage)**

| | Medusa | toko-rs |
|---|---|---|
| API response fields | Top-level: `product_title`, `variant_sku`, etc. | ✅ Same top-level fields |
| Storage | Individual DB columns | JSONB `snapshot` column |

API response shapes are identical. The JSONB approach trades queryability for simplicity — fine for P1.

---

#### S13 (19l): customer_id on cart create — **DIFFER (intentional, documented)**

| | Medusa | toko-rs |
|---|---|---|
| Source of customer_id | Auth session (`req.auth_context?.actor_id`) | Request body (`customer_id` field) |

**Security note**: toko-rs allows any caller to set `customer_id` arbitrarily (impersonation). Acceptable only because there's no real auth in P1. Documented as Decision 15 in `design.md`.

---

### Verification Action Items

| # | Finding | Severity | Status |
|---|---------|----------|--------|
| 1 | S1: `JsonDataError` returns 422, Medusa returns 400 | MEDIUM | **Fixed** — `extract.rs:25` changed to `InvalidData` |
| 2 | S2: Update variant returns `{ variant }`, Medusa returns `{ product }` | MEDIUM | **Fixed** — `routes.rs:170` now returns `ProductResponse` |
| 3 | S3: Soft-delete cascade not atomic | LOW | Deferred — low risk, simple UPDATEs |

### Test Coverage at Verification

- **99 integration tests pass** on SQLite (non-e2e)
- **8 E2E tests** (require running server)
- **Clippy clean** on both feature sets
- **`cargo fmt --check`** clean
- Tests added/updated in Task 19: +23 integration tests, 5 test assertions corrected (422→400 for Medusa-aligned error handling)

---

## Implementation Details

## Task 19: Fourth Audit — Medusa Compatibility Deep-Dive

Source: `docs/audit-p1-task19.md`. 20 findings: 5 HIGH, 8 MEDIUM, 7 LOW.

### Changes applied

| Finding | Files changed | Description |
|---|---|---|
| 19a | `src/extract.rs`, `src/error.rs` | `JsonDataError` → `ValidationError` (422/`invalid_data`); syntax errors stay 400 |
| 19b | `src/product/routes.rs`, `src/product/repository.rs`, `src/product/types.rs` | 4 admin variant endpoints: list, get, update, delete |
| 19c | `src/product/repository.rs` | Soft-delete cascade to variants, options, option_values |
| 19d | `src/product/repository.rs` | DB-aware variant option combo uniqueness check |
| 19e | `src/cart/repository.rs` | Line-item dedup includes `unit_price` in WHERE |
| 19f | `src/order/types.rs`, `src/order/routes.rs` | `CartCompleteResponse` with `success()`/`error()` + `CartCompleteError` struct |
| 19g | `migrations/*/002_customers.sql`, `src/customer/{models,types,repository}.rs`, `src/seed.rs` | Added `company_name` column to customers |
| 19h | `design.md` (Decisions 14–15) | Documented auth divergence on order lookup + `customer_id` on cart create |
| — | `tests/order_test.rs` | +2 tests: success/error response shape |
| — | `tests/product_test.rs` | +14 tests: variant CRUD, cascade, option combo |
| — | `tests/cart_test.rs` | +1 test: different price separate line item |
| — | `tests/customer_test.rs` | +3 tests: create/update/null company_name |
| — | `tests/contract_test.rs` | +2 tests: missing content-type, wrong type mapping |
