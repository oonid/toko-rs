# Task 31: P1 Full Re-Audit Against Medusa Vendor

**Date**: 2026-04-28
**Medusa vendor**: `0303d7f30b` (latest develop branch)
**Scope**: P1 only (Browse → Cart → Checkout = 30 endpoint methods)
**Status**: Findings identified

## Methodology

1. Read all Medusa vendor route handlers, Zod validators, query configs, middleware, and model definitions for all 30 P1 endpoints across admin/products, store/products, store/carts, store/orders, store/customers
2. Read all toko-rs routes, models, types, repository, and error handling code
3. Compare every HTTP method/path, request body shape, response wrapper shape, field names, field visibility, pagination defaults, error shapes, and input validation
4. Cross-reference against `docs/audit-master-checklist.md` (127 prior fixes)
5. Triage each finding as: **P1 FIX**, **P2 DEFER**, **BY-DESIGN**, or **CORRECTION**

## Triage Policy

- **P1 FIX**: Missing field/shape that breaks Medusa SDK compatibility in the Browse→Cart→Checkout flow AND does not require a P2 module
- **P2 DEFER**: Missing field/feature that depends on a P2 module
- **BY-DESIGN**: Intentional divergence for P1 scope
- **CORRECTION**: Prior audit finding was based on incorrect analysis

---

## P1 FIX (1 finding)

### T31-1 | Product images input format mismatch — `Vec<String>` vs `Vec<{url: String}>`

- **Medusa**: `CreateProduct` accepts `images: Array<{url: string}>` (objects with url field). `UpdateProduct` accepts `images: Array<{id?: string, url: string}>` (objects with optional id and url).
- **toko-rs**: `CreateProductInput.images: Option<Vec<String>>` and `UpdateProductInput.images: Option<Vec<String>>` — plain URL strings
- **Impact**: A Medusa SDK sends `{"images": [{"url": "https://..."}]}`. Serde fails to deserialize `{url: "..."}` as a plain `String` → **400 error**. Image management via any Medusa-compatible client is broken.
- **Source**: `vendor/medusa/packages/medusa/src/api/admin/products/validators.ts` — `CreateProduct` uses `images: z.array(z.object({ url: z.string() })).optional()`
- **Files**: `src/product/types.rs` (input types), `src/product/repository.rs` (url extraction)
- **Effort**: Low — change input types to `Vec<ImageInput>` structs, update repository to extract `.url`

---

## CORRECTION (1 finding)

### T31-C1 | T30-7 audit correction — `StoreUpdateCustomer` does NOT have `email`

- **T30-7 claim**: "Medusa: `UpdateCustomerDTO` includes `email`" → added `email: Option<String>` to `UpdateCustomerInput`
- **Actual**: T30-7 referenced the **admin** `UpdateCustomer` schema (which has `email`), not the **store** `StoreUpdateCustomer` schema (which does NOT have `email`)
- **Verified**: `StoreUpdateCustomer` in `vendor/medusa/packages/medusa/src/api/store/customers/validators.ts` is `{company_name?, first_name?, last_name?, phone?, metadata?}` — no `email`
- **Action**: No code change needed. Having `email` on store update is an extra capability, not a break. The V-12 master checklist entry's rationale is inaccurate but the change itself is harmless.

---

## CONFIRMED MATCH — 30 endpoints verified

Every endpoint was verified against the Medusa vendor source:

| # | Endpoint | Method | Path | Response wrapper | Status |
|---|----------|--------|------|-----------------|--------|
| 1 | Admin create product | POST | /admin/products | `{product}` | MATCH |
| 2 | Admin list products | GET | /admin/products | `{products, count, offset, limit}` | MATCH |
| 3 | Admin get product | GET | /admin/products/:id | `{product}` | MATCH |
| 4 | Admin update product | POST | /admin/products/:id | `{product}` | MATCH |
| 5 | Admin delete product | DELETE | /admin/products/:id | `{id, object, deleted}` | MATCH |
| 6 | Admin list variants | GET | /admin/products/:id/variants | `{variants, count, offset, limit}` | MATCH |
| 7 | Admin get variant | GET | /admin/products/:id/variants/:vid | `{variant}` | MATCH |
| 8 | Admin create variant | POST | /admin/products/:id/variants | `{product}` | MATCH |
| 9 | Admin update variant | POST | /admin/products/:id/variants/:vid | `{product}` | MATCH |
| 10 | Admin delete variant | DELETE | /admin/products/:id/variants/:vid | `{id, object, deleted, parent}` | MATCH |
| 11 | Admin list options | GET | /admin/products/:id/options | `{product_options, count, offset, limit}` | MATCH |
| 12 | Admin get option | GET | /admin/products/:id/options/:oid | `{product_option}` | MATCH |
| 13 | Admin create option | POST | /admin/products/:id/options | `{product}` | MATCH |
| 14 | Admin update option | POST | /admin/products/:id/options/:oid | `{product}` | MATCH |
| 15 | Admin delete option | DELETE | /admin/products/:id/options/:oid | `{id, object, deleted, parent}` | MATCH |
| 16 | Store list products | GET | /store/products | `{products, count, offset, limit}` | MATCH |
| 17 | Store get product | GET | /store/products/:id | `{product}` | MATCH |
| 18 | Store create cart | POST | /store/carts | `{cart}` | MATCH |
| 19 | Store get cart | GET | /store/carts/:id | `{cart}` | MATCH |
| 20 | Store update cart | POST | /store/carts/:id | `{cart}` | MATCH |
| 21 | Store add line item | POST | /store/carts/:id/line-items | `{cart}` | MATCH |
| 22 | Store update line item | POST | /store/carts/:id/line-items/:lid | `{cart}` | MATCH |
| 23 | Store delete line item | DELETE | /store/carts/:id/line-items/:lid | `{id, object, deleted, parent}` | MATCH |
| 24 | Store complete cart | POST | /store/carts/:id/complete | `{type, order}` | MATCH |
| 25 | Store list orders | GET | /store/orders | `{orders, count, offset, limit}` | MATCH |
| 26 | Store get order | GET | /store/orders/:id | `{order}` | MATCH |
| 27 | Store register customer | POST | /store/customers | `{customer}` | MATCH |
| 28 | Store get customer me | GET | /store/customers/me | `{customer}` | MATCH |
| 29 | Store update customer me | POST | /store/customers/me | `{customer}` | MATCH |
| 30 | Health check | GET | /health | `{status, ...}` | MATCH |

---

## KEY FIELD-LEVEL VERIFICATIONS

| Area | Verified | Status |
|------|----------|--------|
| All ~30 cart/order computed totals | CartWithItems, OrderWithItems | MATCH |
| Line item snapshot fields (12) | product_title, variant_sku, thumbnail, etc. | MATCH |
| Variant calculated_price | {calculated_amount, original_amount, is_calculated_price_tax_inclusive, currency_code} | MATCH |
| Variant nested options | {id, value, option: {id, title}} | MATCH |
| Product is_giftcard, discountable | Both input and response | MATCH |
| Product collection_id, type_id stubs | Always null, present in response | MATCH |
| Customer created_by, has_account, deleted_at | All visible in response | MATCH |
| Line item compare_at_unit_price | On both cart and order line items | MATCH |
| Cart completed_at visible, deleted_at hidden | Correct visibility | MATCH |
| Error response shape | {type, message, code} on all errors | MATCH |
| Pagination default limit=50, capped at 100 | Matches Medusa | MATCH |
| deny_unknown_fields on strict schemas | Matches Medusa's `.strict()` | MATCH |
| ProductImage model (id, url, rank) | Correct shape, no product_id in response | MATCH |
| ProductOption/Value with metadata | Both tables and models | MATCH |
| Product status enum | draft, proposed, published, rejected | MATCH |
| Cart line items ordered by created_at | Deterministic ordering | MATCH |
| Store products filter: published only | Correctly filtered | MATCH |
| Customer address model | All fields match Medusa | MATCH |

---

## NO NEW P2 DEFERRALS

All previously deferred items (X-1 through X-14) remain accurate. No new P2 findings discovered.

---

## Summary

| Category | Count |
|----------|-------|
| **P1 FIX** | 1 (T31-1: images input format) |
| **Corrections** | 1 (T31-C1: store update email) |
| **Match verified** | 30/30 endpoints + ~30 field-level checks |
| **New P2 deferrals** | 0 |

**Previous audits (127 fixes across T12–T30) are solid.** The codebase has one remaining P1 compatibility gap: the images input format. Everything else matches.
