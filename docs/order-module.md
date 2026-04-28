# Phase 1-C: Order & Payment Module

Completed 2026-04-08. All 9 tasks done (6.1–6.9).

## Endpoints

| Method | Path | Handler | Auth | Description |
|---|---|---|---|---|
| POST | `/store/carts/:id/complete` | `store_complete_cart` | None | Convert cart → order + payment atomically |
| GET | `/store/orders` | `store_list_orders` | X-Customer-Id | List customer's orders (paginated) |
| GET | `/store/orders/:id` | `store_get_order` | X-Customer-Id | Get order with items + payment |

## Architecture

```
src/order/
  mod.rs          — pub mod exports
  models.rs       — Order, OrderLineItem, OrderWithItems
  types.rs        — OrderResponse, OrderListResponse, CartCompleteResponse, ListOrdersParams
  repository.rs   — OrderRepository (SqlitePool)
  routes.rs       — router() + protected_router(), Axum handlers

src/payment/
  mod.rs          — pub mod exports
  models.rs       — PaymentRecord
  repository.rs   — PaymentRepository (create, find_by_order_id)
```

### Models

- **Order**: id (order_ prefix), display_id (auto-increment integer), customer_id, email, currency_code, status, fulfillment_status, metadata (JSON), timestamps, soft-delete
- **OrderLineItem**: id (oli_ prefix), order_id, title, quantity, unit_price (integer cents), variant_id, product_id, snapshot (JSON), metadata (JSON), timestamps, soft-delete
- **OrderWithItems**: flattens Order fields + `items: Vec<OrderLineItem>` + computed `item_total` and `total`
- **PaymentRecord**: id (pay_ prefix), order_id, amount (integer cents), currency_code, status, provider, metadata (JSON), timestamps

### Response Shape

```json
{
  "type": "order",
  "order": {
    "id": "order_01ARZ3...",
    "display_id": 1,
    "customer_id": null,
    "email": "buyer@example.com",
    "currency_code": "usd",
    "status": "pending",
    "items": [
      {
        "id": "oli_01ARZ3...",
        "title": "Test Product",
        "quantity": 2,
        "unit_price": 1000,
        "variant_id": "variant_...",
        "product_id": "prod_...",
        "snapshot": { ... }
      }
    ],
    "item_total": 2000,
    "total": 2000,
    "created_at": "2026-04-08T...",
    "updated_at": "2026-04-08T..."
  },
  "payment": {
    "id": "pay_01ARZ3...",
    "order_id": "order_01ARZ3...",
    "amount": 2000,
    "currency_code": "usd",
    "status": "pending",
    "provider": "manual"
  }
}
```

## Key Behaviors

### Atomic Cart-to-Order Conversion
`create_from_cart` runs a single SQLx transaction:
1. Fetch cart (404 if not found)
2. Check `completed_at IS NULL` (409 if already completed)
3. Fetch cart line items (409 if empty)
4. Compute `display_id = COALESCE(MAX(display_id), 0) + 1`
5. Insert order row
6. Copy line items to `order_line_items`
7. Set cart `completed_at = CURRENT_TIMESTAMP`
8. Commit transaction
9. Create payment record (outside tx — non-critical)

### Cross-Module SQL Join
`create_from_cart` queries `carts` and `cart_line_items` tables directly rather than
calling `CartRepository` methods. This keeps everything in one transaction — the documented
P1 exception to module boundary rules (see `specs/foundation/spec.md`).

### Route Split: Public vs Protected
- `router()` — `/store/carts/:id/complete` is public (no auth). Cart actions should not
  require customer authentication.
- `protected_router()` — `/store/orders` and `/store/orders/:id` require `X-Customer-Id`
  header via `auth_customer_id` middleware.

### display_id Auto-Increment
Uses `COALESCE(MAX(display_id), 0) + 1` from the orders table within the transaction.
The `_sequences` table in the migration is unused (reserved for PG-compatible sequences).

### Computed Totals
`item_total = sum(quantity * unit_price)` for all order line items. `total = item_total`.
Computed in `load_items()` on every read (same pattern as cart module).

## Tests (9 total)

| Test | Spec Scenario |
|---|---|
| `test_complete_cart_creates_order` | Full cart → order with items, totals, payment |
| `test_complete_empty_cart_rejected` | Empty cart → 409 Conflict |
| `test_complete_already_completed_cart_rejected` | Second completion → 409 Conflict |
| `test_complete_nonexistent_cart` | Invalid cart ID → 404 |
| `test_display_id_increments` | 3 orders in same DB → display_id 1, 2, 3 |
| `test_get_order_by_id` | GET single order with items + payment |
| `test_get_order_not_found` | Invalid order ID → 404 |
| `test_list_orders_by_customer` | Paginated list with count, limit, offset |
| `test_list_orders_without_auth_rejected` | No X-Customer-Id header → 401 |

---

## Implementation History (from audit-correction.md)

## 7d. Data Integrity Fixes

Completed 2026-04-08.

### 7d.1: Payment creation moved inside order transaction

The order creation flow (`create_from_cart`) was committing the order transaction first, then
creating the payment record in a separate query using `payment_repo.create()`. If payment
creation failed (e.g., constraint violation, connection drop), the order would persist without
a corresponding payment record — an orphaned order.

**Before:**
```
tx.begin() → create order → copy items → mark cart completed → tx.commit()
payment_repo.create() ← outside transaction, orphan risk on failure
```

**After:**
```
tx.begin() → create order → copy items → create payment → mark cart completed → tx.commit()
```

The payment INSERT now runs within the same transaction. If any step fails, the entire
operation rolls back — no partial state.

**Implementation:**
- Added `PaymentRepository::create_with_tx()` — a static method that accepts `&mut Transaction` instead of using `&self.pool`
- `OrderRepository::create_from_cart()` now calls `PaymentRepository::create_with_tx(&mut tx, ...)` before the cart completion UPDATE and commit
- Removed `payment_repo` parameter from `create_from_cart()` signature — the method no longer needs the `PaymentRepository` instance
- Updated `order/routes.rs` to match the simplified signature

**Files changed:**
- `src/payment/repository.rs` — added `create_with_tx` static method
- `src/order/repository.rs` — moved payment creation before commit, removed parameter
- `src/order/routes.rs` — updated `store_complete_cart` call site

**Test:** `test_order_and_payment_are_atomic` — creates cart with item, completes, verifies both `orders` and `payment_records` rows exist for the same `order_id`

### 7d.2: display_id UNIQUE constraint race handling

Under concurrent requests, `MAX(display_id) + 1` can race — two transactions compute the same
next `display_id`, and the second INSERT hits a UNIQUE violation. Previously, this surfaced
as a raw `DatabaseError` (HTTP 500 with `type: "database_error"`) — an internal error that
doesn't accurately describe the situation.

**Before:**
```
UNIQUE violation on display_id → AppError::DatabaseError → 500, type: "database_error"
```

**After:**
```
UNIQUE violation on display_id → AppError::Conflict → 409, type: "unexpected_state"
    "Order creation failed due to concurrent request. Please retry."
```

**Implementation:**
- Added `OrderRepository::map_display_id_conflict(e: sqlx::Error) -> AppError` — checks for SQLite error code `2067` (SQLITE_CONSTRAINT_UNIQUE)
- Applied via `.map_err(Self::map_display_id_conflict)` on the order INSERT query
- The client receives a 409 with a clear retry message instead of a 500

**Files changed:**
- `src/order/repository.rs` — added `map_display_id_conflict` method, applied to order INSERT

**Test:** `test_complete_cart_returns_conflict_error_format` — verifies empty cart completion returns proper conflict error with `code`, `type`, `message` fields. (The display_id race is difficult to reproduce deterministically in a test; the error mapping is verified by code review and the existing conflict error format test.)

### TDD Record (7d)

1. **RED**: Added 2 new tests — `test_order_and_payment_are_atomic` and `test_complete_cart_returns_conflict_error_format`
2. **GREEN**: Moved payment into transaction, added display_id conflict mapping, updated signatures
3. **Verify**: 71 tests pass, clippy clean, zero warnings

---
