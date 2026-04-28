# Phase 1-B: Cart Module

Completed 2026-04-08. All 10 tasks done (5.1–5.10).

## Endpoints

| Method | Path | Handler | Description |
|---|---|---|---|
| POST | `/store/carts` | `store_create_cart` | Create cart (defaults: currency_code=usd) |
| GET | `/store/carts/:id` | `store_get_cart` | Get cart with items + computed totals |
| POST | `/store/carts/:id` | `store_update_cart` | Update email/customer_id/metadata |
| POST | `/store/carts/:id/line-items` | `store_add_line_item` | Add line item (variant lookup + snapshot) |
| POST | `/store/carts/:id/line-items/:line_id` | `store_update_line_item` | Update quantity (0 = soft-delete) |
| DELETE | `/store/carts/:id/line-items/:line_id` | `store_delete_line_item` | Soft-delete line item |
| POST | `/store/carts/:id/complete` | `store_complete_cart` | Stub — returns 409 (Phase 1-C) |

## Architecture

```
src/cart/
  mod.rs          — pub mod exports
  models.rs       — Cart, CartLineItem, CartWithItems
  types.rs        — CreateCartInput, UpdateCartInput, AddLineItemInput, UpdateLineItemInput, CartResponse
  repository.rs   — CartRepository (SqlitePool)
  routes.rs       — Axum route handlers
```

### Models

- **Cart**: id (cart_ prefix), customer_id, email, currency_code, shipping_address (JSON), billing_address (JSON), metadata (JSON), completed_at, timestamps, soft-delete
- **CartLineItem**: id (cali_ prefix), cart_id, title, quantity, unit_price (integer cents), variant_id, product_id, snapshot (JSON), metadata (JSON), timestamps, soft-delete
- **CartWithItems**: flattens Cart fields + `items: Vec<CartLineItem>` + computed `item_total` and `total`

### Response Shape

```json
{
  "cart": {
    "id": "cart_01ARZ3...",
    "customer_id": null,
    "email": "buyer@example.com",
    "currency_code": "usd",
    "items": [
      {
        "id": "cali_01ARZ3...",
        "title": "Test Product",
        "quantity": 2,
        "unit_price": 1000,
        "variant_id": "variant_...",
        "product_id": "prod_...",
        "snapshot": {
          "product_title": "Test Product",
          "variant_title": "Small",
          "variant_sku": "TEST-S"
        }
      }
    ],
    "item_total": 2000,
    "total": 2000,
    "completed_at": null,
    "created_at": "2026-04-08T...",
    "updated_at": "2026-04-08T..."
  }
}
```

## Key Behaviors

### Line Item Merge
Adding a line item with a `variant_id` that already exists in the cart merges quantities
instead of creating a duplicate. The snapshot is taken from the first insertion.

### Completed Cart Guard
`update_cart` and `add_line_item` check `completed_at IS NOT NULL` and return 409 Conflict
if the cart is completed. This uses `AppError::Conflict` which maps to:
```json
{"type": "conflict", "code": "invalid_state_error", "message": "Cannot ... completed cart"}
```

### Computed Totals
`item_total = sum(quantity * unit_price)` for all non-deleted items. `total = item_total`.
Computed in `get_cart()` on every read.

### Cross-Module SQL Join
`add_line_item` performs a direct SQL JOIN against `product_variants` and `products` tables
to look up price, product_title, variant_title, and variant_sku. This is the documented
P1 exception to module boundary rules (see `specs/foundation/spec.md`).

### mark_completed
`CartRepository::mark_completed()` sets `completed_at = CURRENT_TIMESTAMP`. Used by the
order module (Phase 1-C) during cart-to-order conversion.

## Tests (9 total)

| Test | Spec Scenario |
|---|---|
| `test_store_create_cart_with_defaults` | `{}` body → currency_code "usd", totals 0 |
| `test_store_create_cart_with_email` | currency_code + email |
| `test_store_create_cart_validation_failure` | Invalid email → 400 |
| `test_cart_full_flow` | 13-step flow: create, add, update qty, update cart, delete, qty=0, 404s, stub, validation |
| `test_cart_add_same_variant_merges_quantity` | 2+3 → single item with qty=5 |
| `test_cart_item_total_computed` | Empty→0, add 3x$10→3000 |
| `test_cart_update_completed_cart_rejected` | Completed cart update → 409 |
| `test_cart_add_item_to_completed_cart_rejected` | Completed cart add item → 409 |
| `test_cart_get_response_format` | Contract: all fields present with correct types |

---

## Implementation History (from audit-correction.md)

## 4d. Cart Module Pre-existing Fixes

### 4d.1: Computed `item_total` and `total` fields

The cart spec requires `item_total = sum(quantity * unit_price)` and `total = item_total` on
every cart response. Added `item_total: i64` and `total: i64` to `CartWithItems`, computed in
`get_cart()` and initialized to 0 in `create_cart()`.

**Test:** `test_cart_item_total_computed` — creates cart (total=0), adds 3x$10 item
(total=3000).

### 4d.2: Completed-cart guard on `update_cart`

`update_cart` now checks `completed_at IS NOT NULL` before applying mutations. Returns 409
`Conflict` error.

**Test:** `test_cart_update_completed_cart_rejected` — creates cart, sets `completed_at` via
raw SQL, attempts update, asserts 409 with `type: "conflict"`.

### 4d.3: Complete-cart stub returns JSON error

Changed `store_complete_cart` from returning bare `StatusCode::NOT_IMPLEMENTED` to returning
`AppError::Conflict("Cart completion is not yet implemented")`. This produces proper JSON:
```json
{"code": "invalid_state_error", "type": "conflict", "message": "Conflict: Cart completion is not yet implemented"}
```

### New `Conflict` error variant

Added `AppError::Conflict(String)` to `src/error.rs`:
- HTTP 409 Conflict
- `type: "conflict"`
- `code: "invalid_state_error"`

This maps to Medusa's `"conflict"` error type (409 with `code: "invalid_state_error"`), used
for QueryRunner conflicts and cart state conflicts.

**Files changed:** `src/error.rs`, `src/cart/models.rs`, `src/cart/repository.rs`,
`src/cart/routes.rs`, `tests/cart_test.rs`

