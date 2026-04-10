# Testing

Spec reference: All module specs in `openspec/changes/implementation-p1-core-mvp/specs/` — every scenario from `product-module/spec.md`, `cart-module/spec.md`, `customer-module/spec.md`, `order-module/spec.md`, and `foundation/spec.md` is covered.

## Summary

129 tests across 7 integration test files + 8 E2E test files + 7 unit test source files, covering all 20 API endpoints + health check with 100% endpoint coverage, 93.89% line coverage, 90.30% region coverage.

| Test file | Count | What it covers |
|---|---|---|
| `src/error.rs` (unit) | 9 | All AppError variants: status codes, type strings, code strings |
| `src/config.rs` (unit) | 3 | Config loading, defaults, CORS origins |
| `src/db.rs` (unit) | 4 | PG pool creation, migrations, ping |
| `src/seed.rs` (unit) | 6 | Seed data counts, idempotency, published status, bindings, ranks |
| `src/lib.rs` (unit) | 4 | Health check, app state build, CORS wildcard/specific |
| `tests/product_test.rs` | 23 | All 8 product endpoints: CRUD, variants, duplicate handle/SKU, filtering, pagination, soft delete |
| `tests/cart_test.rs` | 11 | All 6 cart endpoints: create, get, update, add/update/remove items, completed guard |
| `tests/customer_test.rs` | 10 | All 3 customer endpoints: register, get/update profile, auth header |
| `tests/order_test.rs` | 12 | All 3 order endpoints + payment repo direct test |
| `tests/seed_flow_test.rs` | 2 | E2E smoke: browse→cart→checkout, customer order history |
| `tests/health_test.rs` | 1 | Health endpoint |
| `tests/contract_test.rs` | 34 | Response shape validation (10), error contract (10), HTTP method audit (3), validation (7), CORS preflight (1), pagination cap (1), metadata (1), deny unknown fields (1) |
| `tests/e2e/` | 8 | Live HTTP E2E: guest checkout, customer lifecycle, admin CRUD, cart guards, errors, response shapes |

---

## Test Infrastructure

### Integration Tests

`tests/common/mod.rs` provides `setup_test_app()`:
1. Connects to `toko_test` via `DATABASE_URL` env var
2. Runs all 6 PG migrations (`./migrations/`)
3. Calls `clean_all_tables()` to wipe all data (preserves schema)
4. Returns `(Router, AppDb)` for test use

`clean_all_tables()` deletes all rows from: `payment_records`, `order_line_items`, `orders`, `cart_line_items`, `carts`, `customer_addresses`, `customers`, `product_variant_option`, `product_option_values`, `product_options`, `product_variants`, `products`, `idempotency_keys`. Also resets `_sequences` to 0.

### E2E Tests

Located in `tests/e2e/`. Run against live `axum::serve` with `reqwest::Client`.

`tests/e2e/common/mod.rs` provides `setup_e2e()`:
1. Resolve database URL from `E2E_DATABASE_URL` env var
2. Create PG pool via `create_db()`
3. Run migrations via `run_migrations()`
4. `clean_all_tables()` — wipe all data
5. `seed()` — insert 3 products + 1 customer
6. Bind `TcpListener` to `127.0.0.1:0` (random port)
7. Start `axum::serve` in `tokio::spawn`
8. Return `E2eContext { base_url, client, pool, server }`

On drop, `E2eContext::drop()` aborts the server task.

### E2eContext Helpers

| Method | Description |
|--------|-------------|
| `get(path)` | GET request |
| `post_json(path, body)` | POST with JSON body |
| `post_json_with_header(path, body, name, value)` | POST with custom header |
| `get_with_header(path, name, value)` | GET with custom header |
| `delete(path)` | DELETE request |
| `body(resp)` | Parse response JSON |
| `url(path)` | Build full URL |

### Databases

| Database | Purpose | Isolation |
|----------|---------|-----------|
| `toko` | Production / manual testing | — |
| `toko_test` | Integration tests (tower::oneshot) | `clean_all_tables()` per test |
| `toko_e2e` | E2E tests (live HTTP) | `clean_all_tables()` per test |

---

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
| 15 | POST | `/store/carts/{id}/complete` | `test_complete_cart_creates_order`, `_empty_cart_rejected`, `_already_completed_rejected`, `_nonexistent_cart`, `_and_payment_are_atomic` |
| 16 | GET | `/store/orders` | `test_list_orders_by_customer`, `_without_auth_rejected` |
| 17 | GET | `/store/orders/{id}` | `test_get_order_by_id`, `_not_found` |
| 18 | POST | `/store/customers` | `test_register_customer_success`, `_duplicate_email`, `_missing_email`, `_invalid_email` |
| 19 | GET | `/store/customers/me` | `test_get_profile_with_valid_header`, `_without_header`, `_not_found` |
| 20 | POST | `/store/customers/me` | `test_update_customer_profile`, `_without_header`, `test_http_method_post_for_customer_update` |
| + | GET | `/health` | `test_health_check_ok` |

**Coverage: 20/20 endpoints + health = 100%**

---

## E2E Test Coverage

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

---

## Contract Tests

Located in `tests/contract_test.rs`. Each test validates response field keys using `assert_has_fields(value, &[...])`.

### Response Shape Validation (10 tests)

Product, cart, order, customer, error response shapes verified against Medusa OAS.

### Error Contract Tests (10 tests)

Validates every error response matches the OAS Error schema (`code`, `type`, `message`):

| Status | Type | Code | Trigger |
|---|---|---|---|
| 400 | `invalid_data` | `invalid_request_error` | Empty title, invalid email |
| 401 | `unauthorized` | `unknown_error` | Missing X-Customer-Id |
| 404 | `not_found` | `invalid_request_error` | Non-existent product/cart/order |
| 409 | `unexpected_state` | `invalid_state_error` | Empty cart completion, completed cart update |
| 422 | `duplicate_error` | `invalid_request_error` | Duplicate handle, duplicate email |

### HTTP Method Convention (3 tests)

Verifies updates use POST (not PUT), matching Medusa's convention.

### CORS Preflight (1 test)

Verifies OPTIONS requests return correct CORS headers.

---

## Spec Verification Record

Cross-referenced every `#### Scenario:` across all 5 module specs against the test suite. All scenarios covered:

| Spec | Scenarios | All covered? |
|---|---|---|
| product-module | 14 | Yes |
| cart-module | 12 | Yes |
| customer-module | 7 | Yes |
| order-module | 7 | Yes |
| foundation (testable) | 6 | Yes |

---

## Running Tests

```bash
make docker-up    # Start PG (auto-creates toko_test + toko_e2e)
make test-pg      # Integration tests against PostgreSQL
make test-sqlite  # Integration tests against SQLite in-memory
make test-all     # Both PG and SQLite
make test-e2e     # E2E tests only
make test-e2e-pg  # All tests (integration + E2E)
make lint         # cargo clippy -- -D warnings
make cov          # cargo llvm-cov --summary-only
```

All tests require `--test-threads=1` for DB isolation. Makefile targets handle this automatically.

### SQLite Tests

The full test suite (129 tests) runs against SQLite in-memory with `--features sqlite --no-default-features`:

```bash
DATABASE_URL="sqlite::memory:" cargo test --features sqlite --no-default-features -- --test-threads=1
```

Or via Makefile:

```bash
make test-sqlite
```

SQLite tests use the same test infrastructure as PG tests — `tests/common/mod.rs` reads `DATABASE_URL` and routes to the appropriate backend. The `run_migrations()` call in `src/db.rs` selects `./migrations/sqlite/` when the `sqlite` feature is active.

### Testcontainers

Set `E2E_DATABASE_URL=testcontainers://` to start a fresh PG container programmatically per test run. Requires Docker daemon.

### Coverage

```
Line Coverage:  93.89%
Region Coverage: 90.30%
```

Run with: `make cov` or `cargo llvm-cov --summary-only -- --test-threads=1`
