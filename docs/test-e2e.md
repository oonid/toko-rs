# E2E Test Infrastructure

## Overview

End-to-end tests exercise the full HTTP stack: `reqwest::Client` → live `axum::serve` → PostgreSQL. Each test gets an isolated server on a random port with a clean, seeded database.

## Prerequisites

```bash
docker compose up -d
docker exec toko-rs-postgres-1 psql -U postgres -c "SELECT 1 FROM pg_database WHERE datname = 'toko_e2e'"
```

If `toko_e2e` doesn't exist, restart the container (the init script in `scripts/init-dbs.sh` creates it automatically on first run).

## Running Tests

```bash
# E2E only
make test-e2e

# All tests (integration + E2E)
make test-e2e-pg

# Manual
E2E_DATABASE_URL=postgres://postgres:postgres@localhost:5432/toko_e2e \
  cargo test --test e2e -- --test-threads=1
```

**Important**: `--test-threads=1` is required because all E2E tests share the `toko_e2e` database.

## Testcontainers

Set `E2E_DATABASE_URL=testcontainers://` to start a fresh PG container programmatically per test run. Requires Docker daemon running.

```bash
E2E_DATABASE_URL=testcontainers:// cargo test --test e2e -- --test-threads=1
```

## Architecture

```
tests/e2e/
├── main.rs                 # Crate root (mod declarations)
├── common/
│   └── mod.rs              # E2eContext, setup_e2e(), clean_all_tables(), seed()
├── guest_checkout.rs       # Guest browse → cart → checkout
├── customer_lifecycle.rs   # Register → profile → cart → order history
├── admin_products.rs       # Admin CRUD + variant validation
├── cart_manipulation.rs    # Update/delete/guards
├── errors_validation.rs    # Error response contract tests
└── response_shapes.rs      # Response shape contract verification
```

## Test Isolation

`setup_e2e()` performs these steps per test:

1. Resolve database URL from `E2E_DATABASE_URL` env var
2. Create PG pool via `create_db()`
3. Run migrations via `run_migrations()`
4. `clean_all_tables()` — DELETE all rows from all tables, reset `_sequences`
5. `seed()` — insert 3 products + 1 customer via `run_seed()` (ON CONFLICT DO NOTHING)
6. Bind `TcpListener` to `127.0.0.1:0` (random port)
7. Start `axum::serve` in `tokio::spawn`
8. Return `E2eContext { base_url, client, pool, server }`

On drop, `E2eContext::drop()` aborts the server task.

## Databases

| Database | Purpose | Isolation |
|----------|---------|-----------|
| `toko` | Production / manual testing | — |
| `toko_test` | Integration tests (tower::oneshot) | `clean_all_tables()` per test |
| `toko_e2e` | E2E tests (live HTTP) | `clean_all_tables()` per test |

## E2eContext Helpers

| Method | Description |
|--------|-------------|
| `get(path)` | GET request |
| `post_json(path, body)` | POST with JSON body |
| `post_json_with_header(path, body, name, value)` | POST with custom header |
| `get_with_header(path, name, value)` | GET with custom header |
| `delete(path)` | DELETE request |
| `body(resp)` | Parse response JSON |
| `url(path)` | Build full URL |

## Test Coverage

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

Total: 8 E2E tests covering all 21 endpoints.

## Verification

Verified 2026-04-10:
- All 8 tests pass against PostgreSQL 16
- All spec scenarios covered (guest checkout steps 1-9, customer lifecycle steps 10-17)
- Line item delete correctly uses `body["parent"]` (LineItemDeleteResponse schema)
- `clean_all_tables()` clears 13 tables in FK-safe order
- `seed()` uses `ON CONFLICT (id) DO NOTHING` for all entities
- Testcontainers support via `E2E_DATABASE_URL=testcontainers://`
