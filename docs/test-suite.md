# Test Suite

Spec reference: All module specs in `openspec/changes/implementation-p1-core-mvp/specs/` — every scenario from `product-module/spec.md`, `cart-module/spec.md`, `customer-module/spec.md`, `order-module/spec.md`, and `foundation/spec.md` is covered.

## Summary

125 tests across 7 integration test files + 8 E2E test files + 5 unit test source files, covering all 20 API endpoints + health check with 100% endpoint coverage and 92.12% line coverage.

| Test file | Count | What it covers |
|---|---|---|
| `src/error.rs` (unit) | 9 | All AppError variants: status codes, type strings, code strings |
| `src/config.rs` (unit) | 2 | Config loading, defaults |
| `src/db.rs` (unit) | 4 | PG pool creation, migrations, ping |
| `src/seed.rs` (unit) | 6 | Seed data counts, idempotency, published status, bindings, ranks |
| `src/lib.rs` (unit) | 2 | Health check, app state build |
| `tests/product_test.rs` | 23 | All 8 product endpoints: CRUD, variants, duplicate handle/SKU, filtering, pagination, soft delete |
| `tests/cart_test.rs` | 11 | All 6 cart endpoints: create, get, update, add/update/remove items, completed guard |
| `tests/customer_test.rs` | 10 | All 3 customer endpoints: register, get/update profile, auth header |
| `tests/order_test.rs` | 11 | All 3 order endpoints: complete cart, get/list orders, error cases, atomicity |
| `tests/seed_flow_test.rs` | 2 | E2E smoke: browse→cart→checkout, customer order history |
| `tests/health_test.rs` | 1 | Health endpoint |
| `tests/contract_test.rs` | 34 | Response shape validation (10), error contract (10), HTTP method audit (3), validation (7), CORS preflight (1), pagination cap (1), metadata (1), deny unknown fields (1) |
| `tests/e2e/` | 8 | Live HTTP E2E: guest checkout, customer lifecycle, admin CRUD, cart guards, errors, response shapes |

## Test Infrastructure

### `tests/common/mod.rs`

Provides `setup_test_app()` — connects to `toko_test` PostgreSQL database via `DATABASE_URL`, runs migrations, cleans all tables, builds AppState with all 5 repositories, and returns `(Router, AppDb)`. Every integration test uses this.

### Per-file helpers

Each test file defines:
- `body_json(resp) -> Value` — extracts JSON response body
- `request(method, uri, payload) -> Request<Body>` — builds HTTP requests with correct content-type headers
- Domain-specific seed helpers (e.g., `create_sample_product`, `create_cart_with_item`)

## Endpoint Coverage Matrix

| # | Method | Path | Test(s) |
|---|---|---|---|
| 1 | POST | `/admin/products` | `test_admin_create_product_success`, `_validation_failure`, `_duplicate_handle`, `_no_options_no_variants`, `_reuse_handle_after_soft_delete` |
| 2 | GET | `/admin/products` | `test_admin_list_products`, `_pagination`, `_with_deleted` |
| 3 | GET | `/admin/products/{id}` | `test_admin_get_product`, `_not_found` |
| 4 | POST | `/admin/products/{id}` | `test_admin_update_product`, `_not_found`, `_partial`, `test_http_method_post_for_product_update` |
| 5 | DELETE | `/admin/products/{id}` | `test_admin_delete_product`, `_not_found` |
| 6 | POST | `/admin/products/{id}/variants` | `test_admin_add_variant`, `_product_not_found`, `_validation_failure`, `_duplicate_sku` |
| 7 | GET | `/store/products` | `test_store_list_published_only` |
| 8 | GET | `/store/products/{id}` | `test_store_get_published_product`, `_deleted_product_returns_404` |
| 9 | POST | `/store/carts` | `test_store_create_cart_with_defaults`, `_with_email`, `_validation_failure` |
| 10 | GET | `/store/carts/{id}` | `test_cart_get_response_format` |
| 11 | POST | `/store/carts/{id}` | `test_http_method_post_for_cart_update` |
| 12 | POST | `/store/carts/{id}/line-items` | `test_cart_full_flow` (step 2), `_add_same_variant_merges_quantity` |
| 13 | POST | `/store/carts/{id}/line-items/{lid}` | `test_cart_full_flow` (step 3) |
| 14 | DELETE | `/store/carts/{id}/line-items/{lid}` | `test_cart_full_flow` (step 5) |
| 15 | POST | `/store/carts/{id}/complete` | `test_complete_cart_creates_order`, `_empty_cart_rejected`, `_already_completed_rejected`, `_nonexistent_cart`, `_returns_conflict_error_format`, `_and_payment_are_atomic` |
| 16 | GET | `/store/orders` | `test_list_orders_by_customer`, `_without_auth_rejected` |
| 17 | GET | `/store/orders/{id}` | `test_get_order_by_id`, `_not_found` |
| 18 | POST | `/store/customers` | `test_register_customer_success`, `_duplicate_email`, `_missing_email`, `_invalid_email` |
| 19 | GET | `/store/customers/me` | `test_get_profile_with_valid_header`, `_without_header`, `_not_found` |
| 20 | POST | `/store/customers/me` | `test_update_customer_profile`, `_without_header`, `test_http_method_post_for_customer_update` |
| + | GET | `/health` | `test_health_check_ok` |

**Coverage: 20/20 endpoints + health = 100%**

## E2E Tests (Task 16)

Located in `tests/e2e/`. Run against live `axum::serve` with `reqwest::Client`. See `docs/test-e2e.md` for full documentation.

| Test | Steps | Endpoints covered |
|------|-------|-------------------|
| `test_e2e_guest_checkout_flow` | 9 | health, product list/detail, cart create/add-item/complete |
| `test_e2e_customer_lifecycle` | 8 | customer register/get/update, cart create/add-item/complete, order list/detail |
| `test_e2e_admin_product_crud` | 10 | admin product create/list/get/update/publish/add-variant/delete, store GET |
| `test_e2e_admin_product_with_variants` | 4 | admin product create with options/variants, calculated_price, option combo |
| `test_e2e_cart_update_and_delete` | 7 | cart create/add-item/update/delete-line-item, empty cart complete |
| `test_e2e_cart_completed_guards` | 5 | cart complete, update completed (409), add item completed (409) |
| `test_e2e_error_responses` | 7 | 404, 422, 400, 401 error scenarios |
| `test_e2e_response_shapes` | 5 | Product, Cart, Order, Customer, Error contract shapes |

## Contract Tests (10.6)

Located in `tests/contract_test.rs`. Each test validates that a response contains the expected top-level and nested field keys. For example:
- Product response must contain `{product: {id, title, handle, status, options, variants, ...}}`
- Cart response must contain `{cart: {id, currency_code, items, item_total, total, ...}}`
- Order complete response must contain `{type, order: {...}, payment: {...}}`

Uses a `assert_has_fields(value, &[...])` helper rather than exact JSON matching, since field values (IDs, timestamps) are dynamic.

## Error Contract Tests (10.7)

Located in `tests/contract_test.rs`. Validates that every error response matches the OAS Error schema:

```json
{
  "code": "invalid_request_error" | "invalid_state_error" | "api_error" | "unknown_error",
  "type": "not_found" | "invalid_data" | "duplicate_error" | "unauthorized" | "unexpected_state" | "database_error",
  "message": "..."
}
```

Uses `assert_oas_error(body, expected_type, expected_code)` which checks:
1. Exactly 3 fields (code, type, message) — no extra fields
2. All values are strings
3. Values match the expected error type and code

Covered error paths:

| Status | Type | Code | Trigger |
|---|---|---|---|
| 400 | `invalid_data` | `invalid_request_error` | Empty title, invalid email |
| 401 | `unauthorized` | `unknown_error` | Missing X-Customer-Id |
| 404 | `not_found` | `invalid_request_error` | Non-existent product/cart/order |
| 409 | `unexpected_state` | `invalid_state_error` | Empty cart completion, completed cart update |
| 422 | `duplicate_error` | `invalid_request_error` | Duplicate handle, duplicate email |

## HTTP Method Convention (12.7)

Three dedicated tests verify that updates use POST (not PUT):
- `test_http_method_post_for_product_update` — `POST /admin/products/{id}` for partial update
- `test_http_method_post_for_customer_update` — `POST /store/customers/me` for profile update
- `test_http_method_post_for_cart_update` — `POST /store/carts/{id}` for cart update

This matches Medusa's convention where both create and update operations use POST.

## CORS Preflight (foundation spec)

`test_cors_preflight_headers` verifies that OPTIONS requests return `access-control-allow-origin` and `access-control-allow-methods` headers, matching the foundation spec requirement "CORS headers present".

## Verification Record

Cross-referenced every `#### Scenario:` across all 5 module specs against the test suite. All scenarios are covered:

| Spec | Scenarios | All covered? |
|---|---|---|
| product-module | 14 | Yes |
| cart-module | 12 | Yes |
| customer-module | 7 | Yes |
| order-module | 7 | Yes |
| foundation (testable) | 6 | Yes |

Fixes applied during verification:
- Added `test_admin_add_variant_duplicate_sku` — was missing (product-module spec scenario "Add variant with duplicate SKU")
- Added `test_cors_preflight_headers` — was missing (foundation spec scenario "CORS headers present")

## Running

```bash
make docker-up    # Start PG (auto-creates toko_test + toko_e2e)
make test-pg      # Integration tests against PostgreSQL
make test-e2e     # E2E tests only
make test-e2e-pg  # All tests (integration + E2E)
make lint         # cargo clippy -- -D warnings
make cov          # cargo llvm-cov --summary-only
```

All tests require `--test-threads=1` for DB isolation. Makefile targets handle this automatically.
