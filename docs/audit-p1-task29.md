# Audit Report — Task 29: Checklist Re-Numbering & Redundant Test Annotation

**Date**: 2026-04-27
**Scope**: Master checklist structural cleanup + test redundancy annotation
**Status**: Completed

---

## 1. Summary

Two structural problems had accumulated across 12 sequential audits:

1. **Master checklist** had 47 numbering collisions, 2 stale entries, 4 miscategorized entries, and an incorrect total count.
2. **Test suite** had ~30 clearly redundant tests (14%) where the same scenario was tested by another test with equal or greater assertion coverage.

This audit fixes both without changing any runtime behavior.

---

## 2. Master Checklist Re-Numbering

### Problem

The old checklist used per-section numbering (`#1`–`#29` in Section 1, `#16`–`#38` in Section 2, etc.). This created 47 collisions across sections (e.g., Section 5 entries #21–#29 collided with Section 2). The deferred section (Section 8) contained 4 entries that had fixes applied (#95–#98) and 2 entries that were already marked "Fixed" but still listed (#82, #83).

### Solution

Replaced all per-section `#` numbers with globally unique prefixed IDs:

| Prefix | Section | Count |
|--------|---------|-------|
| B- | Bugs Fixed | 32 |
| S- | Response Shape Fixes | 25 |
| V- | Input / Validation Fixes | 11 |
| E- | Error Handling Fixes | 12 |
| D- | Database Schema Fixes | 25 |
| L- | Business Logic Fixes | 9 |
| C- | Configuration & Infrastructure | 4 |
| **Total** | | **118** |

### Corrections Applied

| Issue | Fix |
|-------|-----|
| Total claimed 116, actual 114 in Sections 1-7 | Corrected to 118 (114 + 4 moved from deferred) |
| Section 5 #23 (deleted_at visibility) → actually Response Shape | Moved to S-25 |
| Section 5 #24 (cart state 409→400) → actually Error Handling | Moved to E-12 |
| Section 5 #25 (orphan pivot rows) → actually Bug | Moved to B-32 |
| Section 5 #26 (option coverage) → actually Validation | Moved to V-11 |
| Section 8 #95 (deleted_at skip) → fix applied | Moved to S-24 |
| Section 8 #96 (orphan pivot rows) → fix applied | Moved to B-31 |
| Section 8 #97 (affected-rows check) → fix applied | Moved to B-30 |
| Section 8 #98 (deny_unknown_fields) → fix applied | Moved to V-10 |
| Section 8 #82 (ordli prefix) → duplicate of B-26 | Removed |
| Section 8 #83 (pagination limit) → duplicate of L-7 | Removed |
| B-6/L-2 added 409 guards → E-12 changed to 400 | Added superseded notes |
| Numbering collision note in Section 5 | Eliminated (all IDs now unique) |

### New Reference Tables

Added two new reference tables to the checklist:
- **Audit Reversal Chains**: Documents 4 cases where a later audit reversed an earlier fix
- **Superseded Entries**: Documents entries whose behavior was later changed by another fix

---

## 3. Redundant Test Annotation

### Methodology

Every test across 11 files (163 test functions + 50 lib unit tests = 213 total) was catalogued and cross-referenced. A test was marked `// REDUNDANT:` when:

1. It tests the exact same endpoint + scenario as another test
2. The other test provides equal or greater assertion coverage
3. Both tests run at the same layer (integration or E2E)

The key principle: **contract tests are the canonical TDD truth references against `vendor/medusa/`**. Domain tests that assert the same Medusa shape/error with weaker assertions are redundant. But tests that assert unique scenarios, different error codes, different endpoints, or different behavioral aspects are NOT redundant even if they appear similar.

### Re-Audit: 6 Annotations Corrected

After the initial pass (36 marked), a second pass examined every REDUNDANT annotation by reading both test bodies side-by-side. Found 6 annotations that were incorrect:

| Test | Why NOT Redundant |
|------|-------------------|
| `contract_test::test_variant_option_value_not_found_rejected` (404) | Different error code (404 vs 400), different scenario (nonexistent option VALUE vs missing COVERAGE), different endpoint (`create_product` vs `add_variant`) |
| `contract_test::test_variant_missing_option_coverage_rejected` | Different input pattern (partial: specifies Size not Color vs NO options at all) |
| `contract_test::test_variant_duplicate_option_combination_rejected` (400) | Different endpoint (`create_product` vs `add_variant`), different error code (400 vs 422), different dedup (HashSet vs DB) |
| `contract_test::test_contract_product_list_response_shape` | Tests Medusa field names (shape); product_test tests pagination behavior — different aspects |
| `contract_test::test_contract_delete_response_shape` | Canonical `{id, object, deleted}` shape reference; product_test adds GET-after-delete — complementary |
| `cart_test::test_cart_get_response_format` | Has unique assertions: `completed_at` is null, `deleted_at` is absent — not checked by contract test |

Action taken: removed 6 REDUNDANT annotations. For the cart test, migrated its unique assertions into `test_contract_cart_response_shape` so the canonical contract test is the single truth reference.

### Final Tests Marked Redundant (30 of 213 — 14%)

| File | Count | Redundancy Reason |
|------|-------|-------------------|
| `health_test.rs` | 1 | Shape covered by `contract_test::test_contract_health_response_shape` |
| `seed_flow_test.rs` | 2 | Flows covered by e2e tests and `cart_test::test_cart_full_flow` |
| `product_test.rs` | 6 | Error/404/422 overlaps with `contract_test` error tests |
| `customer_test.rs` | 4 | Error/shape overlaps with `contract_test` error and shape tests |
| `cart_test.rs` | 4 | Completed-cart guard overlaps with `contract_test` and e2e |
| `order_test.rs` | 5 | Error/shape overlaps with `contract_test` error and shape tests |
| `e2e/*.rs` | 8 | E2E flows overlap with integration tests (same flows, different transport) |
| **Total** | **30** | **14% of 213 tests** |

### Not Marked (intentionally)

The following overlapping tests were NOT marked because they provide unique value:

- **`order_test::test_get_order_by_id`** — asserts order-specific fields (display_id, items, totals) beyond shape
- **`order_test::test_list_orders_by_customer`** — asserts pagination + filtering beyond shape
- **`cart_test::test_store_create_cart_with_defaults`** — tests default behavior (currency, empty items)
- **`cart_test::test_cart_item_total_computed`** — tests computation logic, not just shape
- **`cart_test::test_cart_get_response_format`** — asserts `completed_at: null` and no `deleted_at` (migrated to contract test)
- **All snapshot field tests** (4 cart + 1 order) — each tests unique field extraction logic
- **All concurrency tests** (3) — unique race condition coverage
- **`contract_test::test_contract_product_list_response_shape`** — canonical Medusa field name reference
- **`contract_test::test_contract_delete_response_shape`** — canonical DeleteResponse shape reference
- **`contract_test::test_variant_*`** (3 tests) — each tests a different error path than the product_test counterparts

### Contract Test Strengthened

`test_contract_cart_response_shape` now also asserts:
- `completed_at` is `null` (new cart not completed)
- `deleted_at` is absent (not serialized)

This migrates the unique value from `test_cart_get_response_format` into the canonical contract test.

---

## 4. Verification

- 213 tests pass on PostgreSQL
- `cargo clippy -- -D warnings` — clean
- `cargo fmt --check` — clean
- No runtime behavior changed — only comments, documentation, and 2 new assertions in contract test

---

## 5. Impact Assessment

| Metric | Before | After |
|--------|--------|-------|
| Checklist numbering collisions | 47 | 0 |
| Stale entries in deferred | 6 | 0 |
| Miscategorized entries | 4 | 0 |
| Total fix count accuracy | Wrong (claimed 116) | Correct (118) |
| Tests marked redundant | 0 | 30 (14%) |
| Incorrect REDUNDANT annotations | N/A | 0 (6 caught and removed) |
| Contract test assertions added | N/A | +2 (completed_at null, deleted_at absent) |
| Tests deleted | 0 | 0 |
| Runtime behavior changed | No | No |

The 30 redundant tests remain in the suite and continue to pass. They are marked for future cleanup (delete or merge) but are not removed in this task to avoid changing test count or coverage metrics.
