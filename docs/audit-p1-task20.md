# Audit P1 Task 20: P1 Medusa Compatibility Re-Audit

**Date**: 2026-04-22
**Scope**: P1 Core MVP only — 25 endpoint methods across product (12), cart (7), order (3), customer (3)
**Source**: `vendor/medusa/` at develop branch
**Reference**: `openspec/changes/implementation-p1-core-mvp/`

---

## Executive Summary

All 25 P1 endpoints have correct URL paths and HTTP methods. The audit found **3 bugs**, **4 HIGH findings**, and **5 MEDIUM findings** within P1 scope. All findings are for code that exists and is actively used in P1 — no P2-scoped features (pricing module, inventory, shipping, tax, events, admin orders, admin customers, etc.) are included.

### Finding Summary

| Severity | Count | Areas |
|----------|-------|-------|
| BUG | 3 | Cart completion idempotency, soft-delete transactionality, snapshot fields never captured |
| HIGH | 4 | Missing input fields on P1 endpoints, error type gaps, missing per-item totals |
| MEDIUM | 5 | Error message prefixing, pagination defaults, N+1 queries, dead code, generic constraint messages |

---

## BUG-1: Cart Completion Has No Idempotency Protection

**Location**: `src/order/repository.rs:18-108`

The `create_from_cart` method uses a transaction but has no check for whether an order already exists for this cart. If the request is retried (network timeout, client retry), a second order is created.

**Medusa behavior**: Checks `order_cart` link before creating. Returns existing order on retry.

**toko-rs behavior**: The `_sequences` table uses `UPDATE ... RETURNING value` which is atomic, so `display_id` won't collide. But the cart's `completed_at` is only set at the end of the transaction, so two concurrent transactions could both pass the `completed_at.is_some()` check on line 29 and both create orders.

**Fix**: Use `SELECT ... FOR UPDATE` on the cart row within the transaction to serialize concurrent completions:
```sql
SELECT * FROM carts WHERE id = $1 AND deleted_at IS NULL FOR UPDATE
```

---

## BUG-2: Product Soft-Delete Is Not Transactional

**Location**: `src/product/repository.rs:261-309`

The `soft_delete` method runs 4 independent UPDATE statements against `self.pool` (not within a transaction). If the `product_options` or `product_option_values` UPDATE fails after `products` and `product_variants` are already soft-deleted, the database is left in an inconsistent state.

Compare with `create_product` (line 18) which correctly wraps everything in `self.pool.begin()`.

**Fix**: Wrap all 4 statements in a single transaction, same pattern as `create_product`.

---

## BUG-3: Snapshot Captures 5 Fields but Model Extracts 8

**Location**: `src/cart/repository.rs:147-153` (capture) vs `src/cart/models.rs:98-128` (extraction)

The snapshot JSON in `add_line_item` captures only 5 fields:
```json
{ "product_title", "product_description", "product_handle", "variant_title", "variant_sku" }
```

But `CartWithItems::from_items` (line 98-128) and `OrderWithItems::from_items` try to extract 8 fields including:
- `variant_barcode` — always `None`
- `product_subtitle` — always `None`
- `variant_option_values` — always `None`

These 3 fields will always be `null` in API responses. Medusa captures all of these in the snapshot.

**Note**: `variant_barcode` doesn't exist on `product_variants` in toko-rs (no barcode column), so it can't be captured. But `variant_option_values` CAN be captured — the data is available from the variant's option values.

**Fix**: Capture `variant_option_values` in the snapshot JSON during `add_line_item`.

---

## F1: Missing Commonly-Used Input Fields on P1 Endpoints (HIGH)

### F1a: `CreateProductInput` missing `is_giftcard` and `discountable`

| Missing Field | Medusa Type | Default | toko-rs |
|---|---|---|---|
| `is_giftcard` | `boolean, default false` | `false` | Hardcoded in DB as `DEFAULT false` |
| `discountable` | `boolean, default true` | `true` | Hardcoded in DB as `DEFAULT true` |
| `subtitle` | `string.nullish()` | null | Not in DB schema |

Both `is_giftcard` and `discountable` exist as columns in toko-rs's `products` table. The DB defaults match Medusa's. But because `CreateProductInput` uses `#[serde(deny_unknown_fields)]`, a Medusa SDK sending `"discountable": true` will get a **400 error**.

**Impact**: Medusa admin SDKs and UIs send these fields by default. Every create/update will fail.

**Fix**: Add `is_giftcard: Option<bool>` and `discountable: Option<bool>` to `CreateProductInput` and `UpdateProductInput`. Use the values in the INSERT, falling back to DB defaults when `None`.

### F1b: `CreateProductVariantInput` missing `variant_rank`

Medusa allows specifying variant ordering at creation. toko-rs auto-calculates rank via `MAX(rank)+1`. The issue is the same: `deny_unknown_fields` will reject requests containing `variant_rank`.

**Fix**: Add `variant_rank: Option<i64>` to `CreateProductVariantInput`.

---

## F2: `ValidationError` Variant Is Dead Code (HIGH)

**Location**: `src/error.rs:32-33`

`AppError::ValidationError` returns 422 with `type: "invalid_data"`. But it is **never used** anywhere — all validation calls use `AppError::InvalidData` (400). This is confusing because:
- Two different status codes (422 vs 400) for the same error type (`"invalid_data"`)
- The 422 variant is unreachable

**Fix**: Remove `ValidationError` from the enum entirely, or repurpose it for actual validation errors distinct from `InvalidData`.

---

## F3: Missing Per-Item Totals on Line Items (HIGH)

**Location**: `src/cart/models.rs`, `src/order/models.rs`

Medusa returns per-item total fields on each line item: `item_total`, `item_subtotal`, `item_tax_total`, `total`, `subtotal`, `tax_total`, `discount_total`, `discount_tax_total`, `original_total`, `original_subtotal`, `original_tax_total`.

toko-rs returns none of these on individual line items. Only the cart/order-level totals are computed.

For P1 (no tax/discount): `item_total = total = subtotal = quantity * unit_price`, all others = 0.

**Impact**: Medusa storefront components access `item.item_total` directly. These will be `undefined`.

**Fix**: Add `#[sqlx(skip)]` per-item total fields to `CartLineItem` and `OrderLineItem`, compute them in `from_items()`.

---

## F4: Error Message Prefixing (MEDIUM)

**Location**: `src/error.rs` — `#[error("Not Found: {0}")]` etc.

Every toko-rs error message is prefixed: `"Not Found: ..."`, `"Invalid Data: ..."`, `"Duplicate Error: ..."`. Medusa never prefixes.

Medusa examples:
- `"Product with id: prod_01 was not found"`
- `"Product variant with sku: SKU-001, already exists."`

toko-rs examples:
- `"Not Found: Product with id prod_01 was not found"`
- `"Duplicate Error: Variant with SKU 'SKU-001' already exists"`

**Impact**: Clients that parse or match error messages will mismatch. Automated tests against Medusa will fail.

**Fix**: Remove prefixes from `#[error(...)]` attributes, construct messages without prefixes in repository code.

---

## F5: Default Pagination Limit Mismatch (MEDIUM)

| Endpoint | toko-rs | Medusa |
|----------|---------|--------|
| Products list | 20 | 50 |
| Variants list | 20 | 50 |
| Orders list | 20 | 50 |

**Fix**: Change defaults from 20 to 50 for product, variant, and order list endpoints.

---

## F6: N+1 Query Pattern in Order Listing (MEDIUM)

**Location**: `src/order/repository.rs:130-157`

`list_by_customer` loads all orders, then calls `load_items` per order in a loop. For a customer with N orders, this is N+1 queries.

**Fix**: Load all items in a single query filtered by `order_id IN (...)`, group by order.

---

## F7: Generic DB Constraint Messages Lose Context (MEDIUM)

**Location**: `src/error.rs:90-102` — `map_db_constraint()`

Generic fallback messages lose all context:
- Unique violation: `"A record with this value already exists"` (no table/column/value)
- FK violation: `"Referenced record not found"` (no reference detail)
- Not-null violation: `"A required field is missing"` (no field name)

Per-repository methods produce better messages but any constraint not explicitly handled falls through to the generic.

**Fix**: Parse PostgreSQL error `detail`/`column` fields to extract entity/key/value context.

---

## F8: `code` Field Mismatches for Some Error Types (MEDIUM)

| Error Type | toko-rs `code` | Medusa `code` | Issue |
|---|---|---|---|
| Unauthorized | `"unknown_error"` | omitted | Misleading code |
| Forbidden | `"invalid_state_error"` | omitted | Wrong semantics |
| UnexpectedState | `"invalid_state_error"` | omitted | Acceptable |

**Fix**: Consider making `code` optional (omit when Medusa omits it) or use `"invalid_request_error"` for Unauthorized.

---

## Items Explicitly OUT of P1 Scope (Not Findings)

These were identified in the full audit but are **not findings** because they are documented P1 deferrals:

- Missing 61 Medusa endpoints (admin orders, admin customers, promotions, shipping, tax, addresses, order transfers, inventory links, batch ops, import/export)
- No Pricing Module (single `price` column is documented P1 design — Decision 13)
- No Inventory Module
- No Tax Module
- No Shipping Module
- No Promotion Module
- No Event System
- No Workflow Engine
- No Auth Module (X-Customer-Id stub is documented — Decision 7)
- No Service Layer (documented — Decision 9)
- Address architecture (inline JSONB vs separate table — documented)
- Order item structure (single table vs order_line_item + order_item — documented)
- Cross-module FKs (toko-rs has them, Medusa doesn't — documented architectural difference)
- Payment architecture (simplified single table — documented)
- `estimate_count` on list responses (documented P1 deferral)
- `CartCompleteResponse::error()` dead code (documented as P2 infrastructure)
- Line-item dedup includes `unit_price` (documented intentional divergence)
- `customer_id` on cart create (documented — Decision 15)
- `GET /store/orders/:id` auth (documented — Decision 14)

---

## Recommended Action Plan

### Immediate (bugs)

| Finding | Effort | Impact |
|---------|--------|--------|
| BUG-1: Cart completion — add `FOR UPDATE` | ~30 min | Prevents duplicate orders |
| BUG-2: Soft-delete — wrap in transaction | ~30 min | Data consistency |
| BUG-3: Capture `variant_option_values` in snapshot | ~1 hour | Frontend compatibility |

### Next Sprint (P1 polish)

| Finding | Effort | Impact |
|---------|--------|--------|
| F1: Add missing input fields | ~1 hour | SDK/API compatibility |
| F2: Remove dead `ValidationError` variant | ~15 min | Code clarity |
| F3: Add per-item totals | ~2 hours | Frontend compatibility |
| F4: Remove error message prefixes | ~1 hour | API contract alignment |
| F5: Fix pagination defaults (20→50) | ~15 min | Client compatibility |

### Deferred (low priority)

| Finding | Reason |
|---------|--------|
| F6: N+1 query optimization | Performance, not correctness |
| F7: Generic constraint message parsing | Requires PG-specific error handling |
| F8: Conditional `code` field | Minor difference |

---

## Audit Methodology

Six parallel audit streams compared toko-rs against `vendor/medusa/`:

1. **Route paths & HTTP methods**: All 25 toko-rs endpoints vs Medusa route handlers → 100% path/method match on P1 scope
2. **Response shapes**: Field-by-field comparison of all response structs against Medusa TypeScript types
3. **Request/input types**: Field-by-field comparison of all input structs against Medusa Zod validators
4. **Database schema**: 14 toko-rs tables vs matching Medusa models — columns, types, constraints, indexes
5. **Error handling**: All 10 error variants vs Medusa's error handler (13 types, 3 codes)
6. **Business logic**: 8 core flows compared step-by-step

Findings were then filtered to P1 scope only, excluding all documented P2 deferrals.

---

## Verification Results

**Date**: 2026-04-22

### All Findings Fixed

| Finding | Fix Commit | Status |
|---------|------------|--------|
| BUG-1: Cart completion idempotency | `e15944b` | Fixed — `SELECT ... FOR UPDATE` (PG) + guard UPDATE (SQLite) |
| BUG-2: Soft-delete transactionality | `cbf14c0` | Fixed — 4 UPDATEs wrapped in single transaction |
| BUG-3: Snapshot missing `variant_option_values` | `19fcf63` | Fixed — captured via JOIN query during `add_line_item` |
| F1: Missing input fields | `1191a2a` | Fixed — `is_giftcard`, `discountable`, `subtitle`, `variant_rank` accepted |
| F2: Dead `ValidationError` variant | `4b55e5f` | Fixed — removed from enum |
| F3: Per-item totals missing | `cae4c15` | Fixed — 12 `#[sqlx(skip)]` fields per line item, computed in `from_items()` |
| F4: Error message prefixing | `6815e83` | Fixed — prefixes removed from `#[error(...)]` attrs and tests |
| F5: Pagination defaults | `a22d0bc` | Fixed — `default_limit()` returns 50, test assertions updated |

### Deferred Findings (low priority)

| Finding | Reason |
|---------|--------|
| F6: N+1 query optimization | Performance, not correctness |
| F7: Generic constraint message parsing | Requires PG-specific error handling |
| F8: Conditional `code` field | Minor difference |

### Test Suite

| Database | Tests | Result |
|----------|-------|--------|
| SQLite (in-memory) | 164 (34 unit + 130 integration) | All pass |
| PostgreSQL 16 | 164 (34 unit + 130 integration) | All pass |

### Lint & Format

| Check | Result |
|-------|--------|
| `cargo clippy --features sqlite -- -D warnings` | Clean |
| `cargo clippy --features postgres -- -D warnings` | Clean |
| `cargo fmt --check` | Clean |
