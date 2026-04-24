# Task 25: Tenth Audit — P1 Medusa Compatibility Deep Audit (Post-105 Fixes)

**Source**: End-to-end data flow audit + migration consistency audit against `vendor/medusa/`.
**Reconciled against**: `docs/audit-master-checklist.md` (105 prior fixes confirmed).
**Date**: 2026-04-24
**Status**: Findings identified, pending implementation.

---

## Audit Methodology

Two deep-dive audit streams after 9 prior audit rounds:

1. **Cart→Order data flow**: Read every line of `src/cart/repository.rs`, `src/order/repository.rs`, `src/cart/models.rs`, `src/order/models.rs`, `src/seed.rs`. Compared snapshot construction, field mapping, and `from_items()` extraction against Medusa's `LineItem` model, `prepareLineItemData`, `completeCartWorkflow`.
2. **Migration consistency**: Read all 10 migration files (5 PG + 5 SQLite) and all 5 model/repository pairs. Checked PG/SQLite parity, model-schema alignment, INSERT/UPDATE coverage, and constraint correctness.

---

## Findings Summary

| Severity | Count |
|----------|-------|
| BUG | 2 |
| HIGH | 3 |
| MEDIUM | 3 |
| LOW | 5 |
| **Total actionable P1** | **5** |
| P2 / deferred | 8 |

---

## Actionable P1 Findings

### BUG-1: Order line item ID prefix `"oli"` instead of Medusa's `"ordli"`

**Files**: `src/order/repository.rs:92`
**Medusa**: `vendor/medusa/packages/modules/order/src/models/line-item.ts:6` — `prefix: "ordli"`

```rust
let item_id = generate_entity_id("oli");  // should be "ordli"
```

Any Medusa tooling that validates ID prefixes will fail to recognize `"oli_"` as order line items. The cart prefix `"cali"` correctly matches Medusa.

**Fix**: Change `"oli"` to `"ordli"`.

---

### HIGH-1: Dead `quantity == 0 → delete` branch in `update_line_item`

**Files**: `src/cart/repository.rs:295-297`, `src/cart/types.rs:36`

The validator rejects `quantity = 0` before reaching the repository, making this branch unreachable dead code:

```rust
// repository.rs:295-297 — unreachable
if input.quantity == 0 {
    return self.delete_line_item(cart_id, line_id).await;
}
```

Should be removed to avoid confusion.

---

### HIGH-2: No CHECK constraints on monetary/quantity columns

**Files**: `migrations/001_products.sql:42`, `migrations/003_carts.sql:19-20`, `migrations/004_orders.sql:28-29`, `migrations/005_payments.sql:4`

Rust validates ranges at the application layer, but direct DB access can insert negative prices, zero/negative quantities, and negative payment amounts. Both PG and SQLite support CHECK constraints and already use them for status enums.

**Fix**: Add `CHECK (price >= 0)` to `product_variants.price`, `CHECK (quantity > 0)` to line item tables, `CHECK (unit_price >= 0)`, `CHECK (amount >= 0)` to payments.

---

### MEDIUM-1: `is_tax_inclusive` hardcoded to `false`

**Files**: `src/cart/models.rs:119`, `src/order/models.rs:125`
**Medusa**: `vendor/medusa/.../prepare-line-item-data.ts:169` — reads from pricing calculation

Currently correct since toko-rs has no tax engine, but should read from a stored value rather than hardcoding, for forward compatibility.

---

### MEDIUM-2: COALESCE update pattern prevents clearing nullable fields

**Files**: `src/product/repository.rs:252-260`, `src/customer/repository.rs:71-78`, `src/cart/repository.rs:88-90`

`COALESCE($N, column)` means passing `null` keeps the old value. There is no way for API consumers to explicitly clear a nullable field (e.g., remove a phone number, unset a description). The sentinel pattern should differentiate "not provided" from "set to null."

---

## P2 / Deferred Items (Documented)

These are known architectural gaps documented in prior audits:

| # | Gap | Reason |
|---|-----|--------|
| P2-1 | Orders permanently stuck at `pending` — no status transition methods | P2 operations module |
| P2-2 | Payments permanently stuck at `pending` — no capture/refund | P2 payment module |
| P2-3 | `customer_addresses` table exists but has no write paths | P2 address CRUD |
| P2-4 | Cart/order `shipping_address`/`billing_address` always NULL | P2 address management |
| P2-5 | `variant_barcode` always null — no `barcode` column on `product_variants` | P2 product enrichment |
| P2-6 | `is_giftcard` not a separate line item field in API response | P2 line item model |
| P2-7 | Missing `subtitle`, `thumbnail`, `product_type` on line items | P2 line item enrichment |
| P2-8 | `is_custom_price`, `compare_at_unit_price` not captured | P2 pricing module |

---

## Already Correct (Verified)

| Area | Status |
|------|--------|
| PG/SQLite migration parity | ✅ All columns, constraints, indexes match |
| Model-schema alignment | ✅ All fields backed by columns (or properly `#[sqlx(skip)]`) |
| `product_subtitle` populated in snapshot | ✅ Fixed in T24a |
| `is_discountable`/`requires_shipping` from product data | ✅ Fixed in T24b |
| Price validation on create/update variant | ✅ Fixed in T24c |
| `variant_id` length validation | ✅ Fixed in T24d |
| `quantity: 0` rejection | ✅ Fixed in T24e |
| Seed data uses schema defaults for `is_giftcard`/`discountable` | ✅ Correct |

---

## Implementation Checklist

### 25a. Fix order line item ID prefix (BUG-1)
- [ ] 25a.1 Change `"oli"` to `"ordli"` in `src/order/repository.rs:92`
- [ ] 25a.2 Update any test assertions matching `"oli_"` prefix
- [ ] 25a.3 Run full test suite

### 25b. Remove dead `quantity == 0` branch (HIGH-1)
- [ ] 25b.1 Remove `if input.quantity == 0` branch from `update_line_item`
- [ ] 25b.2 Run full test suite

### 25c. Add CHECK constraints on monetary/quantity columns (HIGH-2)
- [ ] 25c.1 Add `CHECK (price >= 0)` to `product_variants.price` in both PG and SQLite
- [ ] 25c.2 Add `CHECK (quantity > 0)` to `cart_line_items.quantity` and `order_line_items.quantity`
- [ ] 25c.3 Add `CHECK (unit_price >= 0)` to both line item tables
- [ ] 25c.4 Add `CHECK (amount >= 0)` to `payment_records.amount`
- [ ] 25c.5 Run full test suite

### 25d. Read `is_tax_inclusive` from snapshot (MEDIUM-1)
- [ ] 25d.1 Add `"is_tax_inclusive": false` to snapshot JSON (forward-compatible)
- [ ] 25d.2 Update `from_items()` to read from snapshot with `false` default
- [ ] 25d.3 Run full test suite

### 25e. Verification pass
- [ ] 25e.1 Run full test suite on SQLite
- [ ] 25e.2 Run full test suite on PostgreSQL
- [ ] 25e.3 Run `cargo clippy -- -D warnings` on both features
- [ ] 25e.4 Run `cargo fmt --check`
- [ ] 25e.5 Update `docs/audit-master-checklist.md`
