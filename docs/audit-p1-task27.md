# P1 Audit — Task 27: Test Contract Audit & Master Checklist Cleanup

**Date**: 2026-04-27
**Type**: Quality Gate / Structural Audit
**Scope**: Every test (197) + master checklist accuracy

---

## 1. Test Inventory

### Breakdown by file

| File | Tests | Category |
|------|-------|----------|
| src/lib.rs (unit) | 36 | INFRA — db, config, error, types, seed internals |
| src/main.rs (unit) | 0 | — |
| tests/cart_test.rs | 25 | BEHAVIOR + CONTRACT |
| tests/contract_test.rs | 37 | CONTRACT (error shapes, HTTP methods, CORS, response shapes) |
| tests/customer_test.rs | 13 | BEHAVIOR + CONTRACT |
| tests/order_test.rs | 23 | BEHAVIOR + CONTRACT + INFRA |
| tests/product_test.rs | 52 | BEHAVIOR + CONTRACT |
| tests/health_test.rs | 1 | INFRA |
| tests/seed_flow_test.rs | 2 | E2E/BEHAVIOR |
| tests/e2e/*.rs | 8 | E2E (full HTTP via reqwest) |
| **Total** | **197** | |

### By category

| Category | Count | Description |
|----------|-------|-------------|
| CONTRACT | ~20 | Verify response JSON matches Medusa OAS |
| BEHAVIOR | ~83 | Verify business logic correctness |
| INFRA | ~41 | Unit tests for db/config/error/types internals |
| E2E | ~7 | Full HTTP server tests via reqwest |

### 114 integration/E2E tests (excluding 41 infra + 36 unit + 1 health + 36 seed internals overlap)

---

## 2. Critical Test Issues

### 2A. Misleading Test Names (2)

| Test | File | Problem |
|-------|------|---------|
| `test_cart_update_line_item_quantity_zero_rejected` | cart_test.rs:125 | Name implies 400 rejection, but asserts **200 OK** (item is removed). Should be `...quantity_zero_removes_item`. |
| `test_concurrent_cart_completion_only_one_succeeds` | order_test.rs:666 | Name implies one fails, but asserts **both return 200** (idempotent). Should be `...completion_is_idempotent`. |

### 2B. Zero Contract Coverage for Recently-Added Fields (6)

These fields were added in T26 but have **zero API-level test verification**:

| Field | Model Location | What's Missing |
|-------|---------------|----------------|
| `option: {id, title}` nested object | `VariantOptionValue` (product/models.rs:107-112) | Every test only checks `["value"]`, never `["option"]["id"]` or `["option"]["title"]` |
| `currency_code` in `calculated_price` | `CalculatedPrice` (product/models.rs:99) | Contract test checks only 3 of 4 fields, skips `currency_code` |
| `credit_line_total/subtotal/tax_total` | `OrderWithItems`/`CartWithItems` | Added fields never asserted in any test |
| `discount_subtotal` | `OrderWithItems`/`CartWithItems` | Same — zero verification |
| `cart_id` in order response | `Order` (order/models.rs:9) | Only verified via raw SQL (`SELECT COUNT(*)`), never in API JSON |
| `shipping_address`/`billing_address` in order JSON | `Order` | Only raw SQL verification, never `order["shipping_address"]` |
| `email` in order response | `Order` | Cart email propagation to order never verified in JSON |

### 2C. Non-HTTP Tests in Integration Test Files (2)

| Test | File | Problem |
|-------|------|---------|
| `test_payment_repo_create_and_find` | order_test.rs:479 | Constructs `PaymentRepository` directly — repo-layer unit test, not HTTP |
| `test_cart_complete_error_response_type` | order_test.rs:541 | Constructs `CartCompleteResponse` Rust structs directly — serde test, not HTTP |

### 2D. Duplicate Tests (3)

| Tests | Overlap |
|-------|---------|
| `test_complete_empty_cart_rejected` + `test_complete_empty_cart_returns_bad_request_format` + `test_error_400_empty_cart_completion` | All test the same empty-cart-completion behavior |
| `test_complete_already_completed_cart_is_idempotent` ≡ `test_cart_completion_retry_returns_same_order` | Both verify sequential retry returns same order_id |
| `test_error_response_format` (product_test.rs) ⊂ contract_test.rs OAS error tests | Subset of contract error shape tests |

### 2E. Weak Assertion Tests — Status Code Only (11)

These tests only check `resp.status()` and never inspect the response body:

1. `test_store_create_cart_validation_failure` (cart_test.rs)
2. `test_admin_get_product_not_found` (product_test.rs)
3. `test_admin_add_variant_product_not_found` (product_test.rs)
4. `test_admin_add_variant_validation_failure` (product_test.rs)
5. `test_admin_update_product_not_found` (product_test.rs)
6. `test_admin_delete_product_not_found` (product_test.rs)
7. `test_get_order_not_found` (order_test.rs)
8. `test_register_customer_invalid_email` (customer_test.rs)
9. `test_admin_get_variant_not_found` (product_test.rs)
10. `test_admin_delete_variant_not_found` (product_test.rs)
11. `test_admin_list_variants_product_not_found` (product_test.rs)

---

## 3. Audit Master Checklist Issues

### 3A. Audit Reversals — Later Audit Undid Earlier Fix (4)

| Original | Reversal | What |
|----------|----------|------|
| T24 BUG-5 (#25): `range(min=1)` for quantity | T26 BUG-1 (#27): reverted to `range(min=0)` | Quantity validation |
| T22 S1 (#95): `#[serde(skip)]` on 9 types | T23 S3,S4: removed from Product + Customer | deleted_at visibility |
| T18 S9 (#82): "oli vs ordli" deferred | T25 BUG-1 (#26): fixed to ordli | Line item prefix |
| T18 S10 (#83): "limit 20 vs 50" deferred | T20 F5 (#73): changed to 50 | Default pagination |

### 3B. Stale Entries (6)

| Entry | Problem |
|-------|---------|
| #25 (Bugs) | Describes `range(min=1)` which was reverted by #27 |
| #82 (Deferred) | "oli vs ordli" listed as divergence but actually fixed |
| #83 (Deferred) | "limit 20 vs 50" listed as divergence but actually fixed |
| #95 (Deferred) | Says `#[serde(skip)]` on 9 types, but 2 were reversed by T23 |
| Section 5 #23 | Says "kept on 7 other types" but code has 9 `#[serde(skip)]` on deleted_at |
| Section 2 #35 | Says "7 fields" added to CartWithItems, actual count is 6 |

### 3C. Numbering Collisions (23 numbers, ~70 collisions)

The `#` column restarts mid-table in Section 5 (jumps back to 21), and Section 2 starts from 16. Result: numbers 16-40, 67-68, 74-75 each refer to 2-4 completely different fixes depending on which section you read. The `#` column is unusable as a unique identifier.

### 3D. Total Count Wrong

| Section | Claimed | Actual Rows |
|---------|---------|-------------|
| 1. Bugs Fixed | 33 | 29 |
| 2. Response Shape | 21 | 20 |
| 3. Input/Validation | 11 | 9 |
| 4. Error Handling | 11 | 11 |
| 5. DB Schema | 31 | 29 |
| 6. Business Logic | 10 | 9 |
| 7. Config/Infra | 4 | 4 |
| **Total** | **121** | **111** |

Off by 10 (9.1% inflation).

### 3E. Scope Creep — Not P1 Medusa Compat (~10 entries)

| Entry | What | Why Not P1 |
|-------|------|-----------|
| #53 | create_product/add_variant transactional | Same API response either way |
| #57 | 13 SQLite performance indexes | Performance, not API behavior |
| #59 | display_id UNIQUE race handling | Internal race condition |
| #65 | Atomic UPDATE...RETURNING for sequences | Internal race condition |
| #74 | AppConfig defaults for HOST/PORT/RUST_LOG | Developer experience |
| #76 | CORS config (was permissive) | Security hardening |
| #77 | SQLite feature flag support | Internal multi-DB infra |
| #46 | PG error code 40001 mapping | DB-specific internal handling |
| #89 (deferred) | N+1 query pattern | Performance |
| #90 (deferred) | Generic DB constraint messages | Developer experience |

These are legitimate improvements but don't affect Medusa frontend compatibility.

---

## 4. Recommendations

### 4A. Fix Tests (the "contracts")

| Priority | Action | Subtasks |
|----------|--------|----------|
| HIGH | Fix 2 misleading names | 27a.1–27a.2 |
| HIGH | Add 6 missing contract assertions for T26 fields | 27b.1–27b.6 |
| MEDIUM | Remove 2 duplicate tests | 27c.1–27c.2 |
| MEDIUM | Upgrade 11 weak tests with body assertions | 27d.1–27d.2 |

### 4B. Fix Master Checklist

| Priority | Action | Subtasks |
|----------|--------|----------|
| HIGH | Introduce globally unique IDs (B-, S-, I-, E-, D-, L-, C- prefixes) | 27e.1 |
| HIGH | Mark 4 reversals, fix 6 stale entries | 27e.2–27e.4 |
| HIGH | Recount total to 111 | 27e.5 |
| MEDIUM | Tag ~10 non-P1 entries as [INTERNAL] | 27e.6 |
| MEDIUM | Eliminate number collisions via unique IDs | 27e.7 |

### 4C. Structural Proposal for Master Checklist

Replace the current 7-section table structure with a single flat table:

```
| ID | Source | Category | Finding | Fix | Status | Section |
```

Where `ID` is globally unique, `Category` is BUG/SHAPE/INPUT/ERROR/DB/LOGIC/CONFIG, and `Status` is FIXED/REVERTED/DEFERRED/INTERNAL. This eliminates all numbering collisions and makes the document usable as a cross-reference.

---

## 5. Impact Assessment

- **197 tests pass** but **6 contract fields have zero coverage** — the tests don't actually verify T26's API changes
- **121 claimed fixes** but **actual count is 111** — the checklist overstates by 10%
- **4 reversals undocumented** — future auditors may re-introduce reverted fixes
- **23 numbering collisions** — the checklist cannot be used as a reliable cross-reference

Fixing these issues would bring the test suite and documentation in line with the actual codebase state, making them true "contracts" as intended.
