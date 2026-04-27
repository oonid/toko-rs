# P1 Audit — Task 28: Response Shape Gap Fixes

**Date**: 2026-04-27
**Type**: Compatibility Audit
**Scope**: Full comparison of toko-rs response shapes against Medusa `vendor/medusa/` source

---

## 1. Methodology

Compared every field in toko-rs's Rust models against:
- **Medusa DML models**: `vendor/medusa/packages/modules/{product,cart,order,customer}/src/models/*.ts`
- **HTTP types**: `vendor/medusa/packages/core/types/src/http/{product,cart,order,customer}/common.ts`
- **Store query-configs**: `vendor/medusa/packages/medusa/src/api/store/{products,carts,orders,customers}/query-config.ts`
- **Admin query-configs**: `vendor/medusa/packages/medusa/src/api/admin/{products,customers}/query-config.ts`
- **Store validators**: `vendor/medusa/packages/medusa/src/api/store/{carts,customers,orders}/validators.ts`

P1 scope filter: only fields that affect the core **Browse → Cart → Checkout** flow. Fields dependent on P2 modules (regions, inventory, shipping, promotions, pricing, collections, types) are excluded unless they can be added as zero-cost nullable stubs.

---

## 2. Findings Catalogue

### 2a. Product

| # | Finding | Field | Medusa Context | Severity | P1? | Status |
|---|---------|-------|----------------|----------|-----|--------|
| P-1 | Missing `collection_id` key | Product response | Store + Admin defaults | MEDIUM | Stub | **T28c** |
| P-2 | Missing `type_id` key | Product response | Store + Admin defaults | MEDIUM | Stub | **T28c** |
| P-3 | Missing `weight` column | Product | Store + Admin defaults | MEDIUM | No | P2 — needs shipping module |
| P-4 | Missing `length` column | Product | Store + Admin defaults | MEDIUM | No | P2 — needs shipping module |
| P-5 | Missing `height` column | Product | Store + Admin defaults | MEDIUM | No | P2 — needs shipping module |
| P-6 | Missing `width` column | Product | Store + Admin defaults | MEDIUM | No | P2 — needs shipping module |
| P-7 | Missing `hs_code` column | Product | Store + Admin defaults | LOW | No | P2 — customs |
| P-8 | Missing `origin_country` column | Product | Store + Admin defaults | LOW | No | P2 — customs |
| P-9 | Missing `mid_code` column | Product | Store + Admin defaults | LOW | No | P2 — customs |
| P-10 | Missing `material` column | Product | Store + Admin defaults | LOW | No | P2 — product attribute |
| P-11 | Missing `external_id` column | Product | Admin defaults | LOW | No | P2 — ERP integration |
| P-12 | Extra `status` on store response | Product | Not in Medusa store defaults | LOW | No | Harmless — store filters to published |
| P-13 | Extra `deleted_at` on store response | Product | Not in Medusa store defaults | LOW | No | Always null on store queries |
| P-14 | Extra `metadata` on store response | Product | Not in Medusa store defaults | LOW | No | Harmless extra |
| P-15 | Missing `images` on create input | CreateProductInput | Medusa accepts `[{url}]` | MEDIUM | No | Image upload is non-goal per design doc |

### 2b. ProductVariant

| # | Finding | Field | Medusa Context | Severity | P1? | Status |
|---|---------|-------|----------------|----------|-----|--------|
| PV-1 | Missing `barcode` column | Variant | Store + Admin defaults | MEDIUM | No | Admin-only, flows through snapshot |
| PV-2 | Missing `ean` column | Variant | Store + Admin defaults | LOW | No | P2 — admin identifier |
| PV-3 | Missing `upc` column | Variant | Store + Admin defaults | LOW | No | P2 — admin identifier |
| PV-4 | Missing `allow_backorder` column | Variant | Store + Admin defaults | HIGH | No | P2 — needs inventory module |
| PV-5 | Missing `manage_inventory` column | Variant | Store + Admin defaults | HIGH | No | P2 — needs inventory module |
| PV-6 | Missing `thumbnail` column | Variant | Store + Admin defaults | MEDIUM | No | P2 — variant-level images (2.11.2) |
| PV-7 | Missing dimension columns | Variant | Store + Admin defaults | MEDIUM | No | P2 — shipping module |
| PV-8..PV-11 | Missing customs columns | Variant | Store + Admin defaults | LOW | No | P2 |
| PV-13 | Extra flat `price` field | Variant response | Not in Medusa variant shape | LOW | No | **KNOWN** — Decision 13 |

### 2c. ProductImage

| # | Finding | Field | Severity | P1? | Status |
|---|---------|-------|----------|-----|--------|
| IMG-1 | Missing `id` | MEDIUM | No | **KNOWN** — #103 deferred P2 |
| IMG-2 | Missing `rank` | MEDIUM | No | **KNOWN** — #103 deferred P2 |

### 2d. Cart / CartLineItem

| # | Finding | Field | Severity | P1? | Status |
|---|---------|-------|----------|-----|--------|
| C-1 | Missing `region_id` column | HIGH | No | P2 — no region module, design doc explicit |
| C-2 | Missing `locale` column | LOW | No | P2 — i18n (since 2.12.3) |
| C-3 | Missing `sales_channel_id` column | LOW | No | P2 — sales channels |
| C-4,C-5 | Missing `shipping/billing_address_id` | LOW | No | **By design** — inline JSON, Decision in design doc |
| C-6 | Missing `item_discount_total` computed | MEDIUM | No | P2 — promotions module |
| C-11 | Missing `region_id` on create input | HIGH | No | P2 — no region to reference |
| C-12 | Missing `items` on create input | MEDIUM | No | Convenience — extra API call acceptable |
| CLI-2 | Missing `thumbnail` on line item | **HIGH** | **Yes** | **T28a** — cart UI has no images |
| CLI-3 | Missing `compare_at_unit_price` | MEDIUM | No | Always null, "Was $null" worse than absent |
| CLI-4 | Missing `is_giftcard` on line item | MEDIUM | **Stub** | **T28b** — free snapshot extraction |
| CLI-5 | Missing `is_custom_price` | LOW | No | P2 — custom pricing |
| CLI-6..CLI-8 | Missing product_type/collection fields | LOW | No | P2 |

### 2e. Order / OrderLineItem

| # | Finding | Field | Severity | P1? | Status |
|---|---------|-------|----------|-----|--------|
| O-1 | Missing `region_id` column | HIGH | No | P2 — no region module |
| O-2 | Missing `version` column | MEDIUM | No | P2 — order edits |
| O-3 | Missing `summary` relation | LOW | No | Architectural diff, same totals |
| O-4 | Missing `custom_display_id` | LOW | No | Niche |
| O-6 | Missing `sales_channel_id` | LOW | No | P2 |
| OLI-2 | Missing `thumbnail` on order line item | **HIGH** | **Yes** | **T28a** — order history has no images |
| OLI-3 | Missing `is_giftcard` on order line item | MEDIUM | **Stub** | **T28b** — free snapshot extraction |
| OLI-4 | Missing `compare_at_unit_price` | MEDIUM | No | Always null |
| OLI-6 | Missing `detail` (fulfillment quantities) | HIGH | No | P2 — design doc: "P1 uses only order_line_items (static snapshot)" |

### 2f. Customer

| # | Finding | Field | Severity | P1? | Status |
|---|---------|-------|----------|-----|--------|
| CU-1 | Missing `created_by` | LOW | No | **KNOWN** — #102 deferred P2 |

---

## 3. Triage Summary

| Category | Count |
|---|---|
| P1 fixes (T28) | 5 (2 thumbnail + 1 is_giftcard + 2 nullable stubs) |
| Rejected (P2 module dependency) | 24 |
| Borderline (low impact, deferred) | 4 |
| Already known in checklist | 4 |
| Harmless extras (no fix needed) | 5 |
| **Total findings** | **42** |

### P1 Fix Justification

| Fix | Why P1 | Cost |
|-----|--------|------|
| Line item `thumbnail` | Medusa store query-config includes `items.thumbnail`. Cart page renders without images. Data exists in DB (`products.thumbnail`), just not captured in snapshot. | Snapshot capture + model field |
| Line item `is_giftcard` | Snapshot already captures `product_is_giftcard`. Medusa models include `is_giftcard: boolean`. Extraction is free. | Model field + extraction |
| Product `collection_id` | Medusa store query-config includes it. Frontend JS gets `null` instead of `undefined`. No new column, no migration. | `#[sqlx(skip)]` field |
| Product `type_id` | Same as `collection_id`. | `#[sqlx(skip)]` field |

### Rejection Rationale for Common P2 Items

- **`region_id`**: Design doc explicitly states "P1 has no region concept; config-driven default is the equivalent." Adding a region_id stub is misleading — it implies region support that doesn't exist.
- **`allow_backorder`/`manage_inventory`**: Meaningful only with inventory module. Adding stubs would cause frontends to change behavior (hide "Add to Cart") based on wrong defaults.
- **`compare_at_unit_price`**: Always null in P1. A storefront rendering "Was $null" is worse than the key being absent.
- **Dimensions/customs**: Shipping module P2. Not rendered on storefronts.

---

## 4. Files Changed

| File | Change |
|------|--------|
| `src/cart/repository.rs` | Add `p.thumbnail` to snapshot query + snapshot JSON |
| `src/cart/models.rs` | Add `thumbnail`, `is_giftcard` to `CartLineItem` |
| `src/order/models.rs` | Add `thumbnail`, `is_giftcard` to `OrderLineItem` |
| `src/product/models.rs` | Add `collection_id`, `type_id` to `Product` |
| `tests/contract_test.rs` | Assert new fields in response shapes |
| `docs/audit-master-checklist.md` | Add T28 entries |
