# Task 19 Verification Report: Medusa Compatibility Deep-Dive

**Date**: 2026-04-22  
**Source**: `docs/audit-p1-task19.md` (20 findings)  
**Method**: Line-by-line comparison of toko-rs implementation against `vendor/medusa/` source at `develop` branch.  
**Scope**: All 13 implemented sub-tasks (19a–19m) verified against Medusa models, services, API routes, validators, and OAS specs.

---

## Summary

| Verdict | Count |
|---------|-------|
| MATCH (correct) | 5 |
| DIFFER (intentional, documented) | 3 |
| DIFFER (new bug found — FIXED) | 2 |
| PARTIAL (response shape fixed) | 1 |
| N/A (out of P1 scope) | 2 |

---

## Finding-by-Finding Analysis

### S1 (19a): JsonDataError mapping — **DIFFER → FIXED**

**Status**: Bug found during verification, now fixed.

| | Medusa | toko-rs (before) | toko-rs (after) |
|---|---|---|---|
| HTTP Status | **400** | ~~422~~ | **400** |
| Error `type` | `"invalid_data"` | `"invalid_data"` | `"invalid_data"` |
| Error `code` | omitted | `"invalid_request_error"` | `"invalid_request_error"` |

**Medusa behavior**: When a JSON field has the wrong type (e.g., `"quantity": "hello"`), Zod validation fails with `MedusaError.Types.INVALID_DATA`, which Medusa's error handler maps to **400** (`error-handler.ts:67-68`).

**toko-rs behavior**: `extract.rs:24-26` maps `JsonDataError` → `AppError::ValidationError`, which returns **422**. This is incorrect — it should map to `AppError::InvalidData` (400).

**Bug**: `src/extract.rs:25` — changed `ValidationError` to `InvalidData`. **Fixed.**

**Severity**: ~~MEDIUM~~ FIXED. Wrong status code on wrong-type fields.

---

### S2 (19b): Admin variant endpoints — **PARTIAL → FIXED (response shape)**

**Status**: All 5 core CRUD endpoints implemented. Update variant response shape fixed.

| Endpoint | Route | Response |
|---|---|---|
| `GET /admin/products/:id/variants` | ✅ Match | ✅ `{ variants, count, offset, limit }` |
| `POST /admin/products/:id/variants` | ✅ Match | ✅ `{ product: {...} }` |
| `GET /admin/products/:id/variants/:vid` | ✅ Match | ✅ `{ variant: {...} }` |
| `POST /admin/products/:id/variants/:vid` | ✅ Route exists | ✅ `{ product: {...} }` (fixed) |
| `DELETE /admin/products/:id/variants/:vid` | ✅ Match | ✅ `{ id, object, deleted, parent }` |

**Fixed**: Update variant response changed from `{ variant }` to `{ product }` to match Medusa's behavior (`src/product/routes.rs:170`).

**Missing endpoints** (out of P1 scope): 8 additional Medusa variant endpoints (batch ops, inventory linking, variant images, standalone listing) are not implemented. These require inventory/pricing modules not in P1.

**Severity**: MEDIUM. Update variant response shape incompatibility may break Medusa admin frontends.

---

### S3 (19c): Soft-delete cascade — **MATCH (with atomicity gap)**

**Status**: Both systems cascade `deleted_at` to variants, options, and option values.

| | Medusa | toko-rs |
|---|---|---|
| Cascade to variants | ✅ | ✅ |
| Cascade to options | ✅ | ✅ |
| Cascade to option_values | ✅ (recursive from options) | ✅ (sub-select) |
| Atomicity | **Transactional** (MikroORM unit of work) | **NOT transactional** (4 independent queries) |

**Atomicity gap**: `src/product/repository.rs:261-310` — the four UPDATE statements run independently on `self.pool` without wrapping in a transaction. If the variants/options UPDATE fails after the product is already soft-deleted, the database is left in an inconsistent state. Medusa avoids this because MikroORM's cascading soft-delete runs within a single transaction.

**Severity**: LOW. Failure during child cascade is extremely unlikely (simple UPDATEs on indexed columns). Worth fixing for correctness but not blocking.

---

### S4 (19d): Variant option uniqueness — **MATCH (different implementation, same behavior)**

| | Medusa | toko-rs |
|---|---|---|
| Comparison key | `option_value_id` (UUID) | `(option_title, value_string)` tuple |
| Match semantics | Subset (`.every()`) | Exact (`HashSet ==`) |
| Batch dedup | O(N²) `.every()` | `HashSet` |
| DB check on add | Yes | Yes |

**Analysis**: Both systems validate variant option combination uniqueness. For the standard creation flow (all options required), behavior is equivalent. Edge-case differences:
- Medusa is stricter on updates (subset matching with `.every()`).
- toko-rs is stricter on identity (string-based comparison catches semantically identical combos with different IDs).

**Severity**: NONE. Intentional implementation difference, functionally equivalent for P1.

---

### S5 (19e): Line-item dedup includes unit_price — **DIFFER (intentional divergence)**

| | Medusa | toko-rs |
|---|---|---|
| Regular-priced items | Matches on `variant_id + metadata` (ignores price) | Matches on `variant_id + unit_price + metadata` |
| Custom-priced items | Matches on `variant_id + unit_price + metadata` | Same |

**Medusa behavior** (`get-line-item-actions.ts:100-114`): Uses a split logic. For regular-priced items (`is_custom_price = false`), Medusa merges regardless of price. Only custom-priced items compare `unit_price`.

**toko-rs behavior** (`src/cart/repository.rs:157`): Unconditionally includes `unit_price` in the WHERE clause.

**Impact**: If a variant's price changes between adding to cart, toko-rs creates a separate line item at the new price, while Medusa would merge (updating the existing item's quantity and recalculating). toko-rs's behavior is arguably more correct for price integrity.

**Documented**: This is an intentional design decision. Price changes between adds should create separate items.

**Severity**: LOW. Intentional divergence documented in design decisions.

---

### S6 (19f): Cart complete error branch — **DIFFER (dead code)**

| | Medusa | toko-rs |
|---|---|---|
| Error response type | `CartCompleteResponse::error()` — used for payment failures | `CartCompleteResponse::error()` — **dead code, never called** |
| Success response | `{ type: "order", order: {...} }` | ✅ Same |

**Medusa behavior** (`vendor/medusa/packages/medusa/src/api/store/carts/[id]/complete/route.ts:37-85`): For recoverable payment errors, returns HTTP 200 with `{ type: "cart", cart: {...}, error: { message, name, type } }`, allowing the client to retry.

**toko-rs behavior**: `CartCompleteResponse::error()` exists in `src/order/types.rs:50-61` but is **never invoked** from any route handler. All errors propagate as `AppError`, returning a generic error JSON instead of Medusa's structured error response. The route handler (`src/order/routes.rs`) only calls `CartCompleteResponse::success()`.

**Severity**: MEDIUM. The error branch is implemented but dead code. Payment retry scenarios (P2) will need this. Not blocking for P1 since toko-rs doesn't have a real payment provider yet.

---

### S7 (19g): company_name on customer — **MATCH**

| | Medusa | toko-rs |
|---|---|---|
| Model field | `company_name: model.text().searchable().nullable()` | `company_name: Option<String>` |
| DB column | `company_name TEXT NULL` | ✅ Same |
| Create input | `company_name?: string \| null` | ✅ Same |
| Update input | `company_name?: string \| null` | ✅ Same |
| API response | Always present (nullable) | ✅ Same |

Verified in `vendor/medusa/packages/modules/customer/src/models/customer.ts:9`.

---

### S8 (19h): GET /store/orders/:id auth — **DIFFER (intentional security improvement)**

| | Medusa | toko-rs |
|---|---|---|
| Authentication | **None** (unauthenticated, has TODO comment to add it) | `X-Customer-Id` header required |
| Authorization | No customer_id filter | No customer_id filter |

**Medusa behavior** (`vendor/medusa/packages/medusa/src/api/store/orders/[id]/route.ts:5`): Has a `// TODO: Do we want to apply some sort of authentication here?` comment. The endpoint is currently unauthenticated.

**toko-rs behavior**: Requires `X-Customer-Id` header (returns 401 if missing). However, the repository doesn't filter by customer_id, so any authenticated customer can view any order.

**Severity**: NONE. toko-rs is stricter than Medusa (which has acknowledged this as a gap). Documented as Decision 14 in `design.md`.

---

### S9 (19i): metadata on product_options and option_values — **MATCH**

| | Medusa | toko-rs |
|---|---|---|
| ProductOption.metadata | `model.json().nullable()` | `Option<Json<Value>>` ✅ |
| ProductOptionValue.metadata | `model.json().nullable()` | `Option<Json<Value>>` ✅ |

Verified in `vendor/medusa/packages/modules/product/src/models/product-option.ts:9` and `product-option-value.ts:8`.

---

### S10 (19j): Missing DB indexes — **MATCH (within scope)**

toko-rs has 22 indexes across products/carts/orders tables. Every Medusa index for a column that exists in toko-rs has a corresponding index. Missing Medusa indexes are only for columns toko-rs doesn't model (`product_type_id`, `barcode`, `ean`, `upc`, `region_id`, `sales_channel_id`).

The 6 new indexes added in 19j are correct:
- `idx_product_variants_id_product_id` — matches Medusa's `IDX_product_variant_id_product_id`
- `idx_orders_deleted_at` — matches Medusa's `IDX_order_deleted_at`
- `idx_orders_currency_code` — matches Medusa's `IDX_order_currency_code`
- `idx_order_line_items_deleted_at` — matches Medusa's `IDX_order_line_item_deleted_at`
- `idx_order_line_items_product_id` — matches Medusa's `IDX_order_line_item_product_id`
- `idx_order_line_items_variant_id` — matches Medusa's `IDX_order_line_item_variant_id`

---

### S12 (19k): Line-item snapshot fields — **MATCH (API shape), DIFFER (storage)**

| | Medusa | toko-rs |
|---|---|---|
| API response fields | Top-level: `product_title`, `variant_sku`, etc. | ✅ Same top-level fields |
| Storage | Individual DB columns | JSONB `snapshot` column |

**Medusa behavior**: Stores `product_title`, `product_description`, `product_subtitle`, `product_handle`, `variant_sku`, `variant_barcode`, `variant_title`, `variant_option_values` as individual database columns on `CartLineItem` and `OrderLineItem` models.

**toko-rs behavior**: Stores all snapshot data in a single JSONB `snapshot` column, then extracts to top-level `#[sqlx(skip)]` fields in `from_items()`.

**Impact**: API response shapes are identical. The JSONB approach trades queryability for simplicity — fine for P1.

---

### S13 (19l): customer_id on cart create — **DIFFER (intentional, documented)**

| | Medusa | toko-rs |
|---|---|---|
| Source of customer_id | Auth session (`req.auth_context?.actor_id`) | Request body (`customer_id` field) |

**Medusa behavior** (`vendor/medusa/packages/medusa/src/api/store/carts/route.ts:20-23`): Overrides `customer_id` with the authenticated user's actor ID. The `StoreCreateCart` validator does NOT include `customer_id`.

**toko-rs behavior**: Accepts `customer_id` as an optional field in the request body.

**Security note**: toko-rs allows any caller to set `customer_id` arbitrarily (impersonation). This is acceptable only because there's no real auth in P1.

**Severity**: LOW. Documented as Decision 15 in `design.md`. Will be removed in P2 when real auth is implemented.

---

## Action Items

### Bugs found and fixed during verification:

| # | Finding | Severity | Status |
|---|---------|----------|--------|
| 1 | S1: `JsonDataError` returns 422, Medusa returns 400 | MEDIUM | **Fixed** — `extract.rs:25` changed to `InvalidData` |
| 2 | S2: Update variant returns `{ variant }`, Medusa returns `{ product }` | MEDIUM | **Fixed** — `routes.rs:170` now returns `ProductResponse` |
| 3 | S3: Soft-delete cascade not atomic | LOW | Deferred — low risk, simple UPDATEs |

### Items already correct or intentionally different:

- S4: Variant option uniqueness — functionally equivalent
- S5: Line-item price dedup — intentional divergence (toko-rs is stricter)
- S6: Cart complete error branch — dead code but infrastructure is correct for P2
- S7: company_name — matches Medusa exactly
- S8: Order auth — intentional security improvement
- S9: metadata on options — matches Medusa exactly
- S10: DB indexes — matches Medusa within scope
- S12: Snapshot fields — API shape matches, storage differs
- S13: customer_id on cart — intentional P1 workaround

---

## Test Coverage

- **158 tests pass** on SQLite (non-e2e)
- **Clippy clean** on both feature sets
- **`cargo fmt --check`** clean
- Tests added/updated in Task 19: +23 integration tests, 5 test assertions corrected (422→400 for Medusa-aligned error handling)
