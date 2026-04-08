## 1. Phase 0 — Project Scaffold (DONE)

- [x] 0.1 Initialize Git workspace with Cargo.toml (edition 2021, MSRV 1.85)
- [x] 0.2 Add Medusa source as vendor submodule (`vendor/medusa/`, `develop` branch, OAS specs verified identical to `vendor/medusa/www/utils/generated/oas-output/base/`)
- [x] 0.3 Configure dependencies: axum 0.8, sqlx 0.8, tokio, serde, validator, ulid, slug, dotenvy, thiserror, chrono, tracing
- [x] 0.4 Write foundation files: config.rs, db.rs, error.rs, types.rs
- [x] 0.5 Write SQL migrations: 001_products, 002_customers, 003_carts, 004_orders, 005_payments, 006_idempotency
- [x] 0.6 Write main.rs skeleton: tracing init, config load, DB pool, migrations, Axum router, health check
- [x] 0.7 Create Makefile with dev, test, check, lint, fmt, seed, clean-db targets
- [x] 0.8 Create .env and .env.example with DATABASE_URL, HOST, PORT, RUST_LOG
- [x] 0.9 Verify `cargo test` passes (6 tests: 3 product + 3 cart)
- [x] 0.10 Add CORS middleware (CorsLayer::permissive) to app_router
- [x] 0.11 Add graceful shutdown (with_graceful_shutdown + SIGINT/SIGTERM handler)
- [x] 0.12 Health check probes DB connectivity via db::ping() — returns "degraded"/"disconnected" on failure
- [x] 0.13 FindParams defaults: offset=0 (serde default), limit=50 (serde default fn)
- [x] 0.14 Zero compiler warnings — removed unused imports (delete, Arc), declared `cfg(coverage)` check-cfg in Cargo.toml; 6 tests passing

## 2. Phase 1-A — Product Module (DONE)

- [x] 2.1 Define models: Product, ProductOption, ProductOptionValue, ProductVariant, ProductWithRelations, ProductOptionWithValues, ProductVariantWithOptions, VariantOptionValue
- [x] 2.2 Define request/response types: AdminCreateProductRequest, AdminUpdateProductRequest, ProductResponse, ProductListResponse, DeleteResponse
- [x] 2.3 Implement repository: create (transactional with options/variants)
- [x] 2.4 Implement routes: all 8 routes registered in router
- [x] 2.5 Wire product routes into Axum router with AppState
- [x] 2.6 Write integration tests for product create (2 tests: success + validation)
- [x] 2.7 Verify route stubs respond (replaced by full implementation tests)
- [x] 2.8 Implement `admin_list_products` route handler — paginated with offset/limit/order/with_deleted
- [x] 2.9 Implement `admin_get_product` route handler — find_by_id with options/variants/variant_options join
- [x] 2.10 Implement `admin_update_product` route handler — COALESCE partial update pattern
- [x] 2.11 Implement `admin_delete_product` route handler — soft delete returning Medusa DeleteResponse
- [x] 2.12 Implement `admin_add_variant` route handler — insert variant with option binding resolution
- [x] 2.13 Implement `store_list_products` route handler — filters `status = 'published' AND deleted_at IS NULL`
- [x] 2.14 Implement `store_get_product` route handler — find_published_by_id, 404 for draft/deleted
- [x] 2.15 Fix handle generation: use `types::generate_handle()` (slug crate)
- [x] 2.16 Use `types::generate_entity_id()` for all ID generation
- [x] 2.17 ULID casing: lowercase (matches ulid crate default, spec updated to `[0-9a-z]{26}`)
- [x] 2.18 Add duplicate handle detection: SQLite UNIQUE violation mapped to AppError::DuplicateError
- [x] 2.19 Implement `find_by_id` in repository — product + options + option_values + variants + variant_options
- [x] 2.20 Implement `list` in repository — paginated with offset, limit, order, with_deleted
- [x] 2.21 Implement `list_published` in repository — status='published' AND deleted_at IS NULL
- [x] 2.22 Implement `update` in repository — COALESCE pattern for partial updates
- [x] 2.23 Implement `soft_delete` in repository — set deleted_at = CURRENT_TIMESTAMP
- [x] 2.24 Implement `add_variant` in repository — insert variant with option binding via product_variant_options pivot

## 2b. Database Refactor — PostgreSQL-Primary (DONE)

- [x] 2b.1 Remove dual `SqliteProductRepository` / `PostgresProductRepository` pattern — consolidate to single `ProductRepository` using `SqlitePool` (PG adapter deferred; single-repo pattern established)
- [x] 2b.2 Remove dual `SqliteCartRepository` / `PostgresCartRepository` pattern — consolidate to single `CartRepository`
- [x] 2b.3 Remove `DatabaseRepo` enum dispatch in `db.rs` — replace with `Repositories` struct holding individual repo instances
- [x] 2b.4 Remove all `#[cfg(not(coverage))]` / `#[cfg(coverage)]` guards from repositories and Cargo.toml
- [x] 2b.5 SQLite adapter: repos use `?` placeholders directly; PG migration path uses `$N` placeholders in `migrations/pg/` (placeholder translation not needed — separate migration sets)
- [x] 2b.6 Create `docker-compose.yml` with PostgreSQL 16 service for integration testing
- [x] 2b.7 PostgreSQL-primary migrations in `migrations/` — `timestamptz`, `jsonb`, `BOOLEAN`, partial unique indexes, CHECK constraints
- [x] 2b.8 SQLite-compatible migrations in `migrations/sqlite/` for in-memory test path
- [x] 2b.9 Update `AppState` to hold `Arc<Repositories>` with individual repo structs (no enum dispatch)
- [x] 2b.10 Update test infrastructure (`tests/common/mod.rs`) to use `Repositories` struct
- [x] 2b.11 Fix variant-to-option pivot: persist variant option bindings to `product_variant_options` table during create_product and add_variant
- [x] 2b.12 Fix error response: add `code` field to match 3-field OAS Error schema (`code`, `type`, `message`)
- [x] 2b.13 Verify all existing tests still pass after refactor — 41 tests, clippy clean, 92.42% coverage
- [x] 2b.14 Add Makefile docker targets: `docker-up`, `docker-down`, `test-pg`, `cov`

## 3. Phase 1-D — Customer Module (DONE)

- [x] 3.1 Define customer models: Customer struct with id, first_name, last_name, email, phone, has_account, metadata, timestamps
- [x] 3.2 Define customer request/response types: CreateCustomerInput (email required), UpdateCustomerInput (partial), CustomerResponse
- [x] 3.3 Implement customer repository: create (with duplicate email detection via UNIQUE violation), find_by_id, update (COALESCE partial)
- [x] 3.4 Implement customer routes: POST /store/customers, GET /store/customers/me, POST /store/customers/me
- [x] 3.5 Implement X-Customer-Id header extraction as Axum middleware (from_fn) using Extension to inject CustomerId
- [x] 3.6 Wire customer repository into AppState (Repositories.customer)
- [x] 3.7 Wire customer routes into main router
- [x] 3.8 Write integration tests: 10 tests — register success, duplicate email, missing email, invalid email, get profile, get without header, get not found, update profile, update without header, response format

## 4. Audit Fixes — Medusa Compatibility Corrections

### 4a. Error handling alignment with Medusa error handler

- [x] 4a.1 Fix `error_code()` mapping in `src/error.rs`: align with Medusa's error-handler.ts — most error types should NOT override `code`; only `duplicate_error` → `invalid_request_error`, `database_error` → `api_error`, and unknown → `unknown_error` have explicit overrides — **audit confirmed current code mappings are already correct per OAS enum; no change needed**
- [x] 4a.2 Fix `DuplicateError` HTTP status: 409 → 422 (Medusa maps `duplicate_error` to 422, not 409)
- [x] 4a.3 Fix `UnexpectedState` HTTP status: 409 → 500 (Medusa default for `unexpected_state`; documented divergence for future cart conflict scenarios in `docs/audit-correction.md`)
- [x] 4a.4 Update error unit tests in `src/error.rs` to verify corrected mappings — also updated integration tests in `tests/product_test.rs` and `tests/customer_test.rs`; 51 tests pass, clippy clean

### 4b. Database schema alignment with Medusa models

- [x] 4b.1 Rename pivot table `product_variant_options` → `product_variant_option` (singular) in both PG/SQLite migrations and 2 SQL queries in `src/product/repository.rs` — matches Medusa's `pivotTable: "product_variant_option"`
- [x] 4b.2 Fix SQLite `products.handle` constraint: replaced column-level `UNIQUE` with `CREATE UNIQUE INDEX uq_products_handle ON products (handle) WHERE deleted_at IS NULL` — now matches PG migration behavior; new test `test_admin_create_product_reuse_handle_after_soft_delete` verifies handle re-use after delete
- [x] 4b.3 Add missing unique index `uq_product_options_product_id_title ON product_options (product_id, title) WHERE deleted_at IS NULL` per Medusa `IDX_option_product_id_title_unique` — applied to both PG and SQLite
- [x] 4b.4 Add missing unique index `uq_product_option_values_option_id_value ON product_option_values (option_id, value) WHERE deleted_at IS NULL` per Medusa `IDX_option_value_option_id_unique` — applied to both PG and SQLite
- [x] 4b.5 Apply 4b.2–4b.4 to both PG and SQLite migration sets — 52 tests pass, clippy clean

### 4c. Product repository transactional safety

- [x] 4c.1 Wrap `create_product` and `add_variant` in `self.pool.begin()` transactions — refactored `insert_variant` and `resolve_variant_options` into static `insert_variant_tx`/`resolve_variant_options_tx` methods that accept `&mut Transaction`; failure mid-way now rolls back cleanly

### 4d. Cart module pre-existing fixes

- [x] 4d.1 Add `item_total` and `total` computed fields to `CartWithItems` — computed as `sum(quantity * unit_price)` in `get_cart()`, initialized to 0 in `create_cart()`; test `test_cart_item_total_computed` verifies 3x$10 → total=3000
- [x] 4d.2 Add completed-cart guard to `update_cart` — checks `completed_at IS NOT NULL`, returns 409 `Conflict`; added `AppError::Conflict` variant (`type: "conflict"`, code: `"invalid_state_error"`, status: 409) matching Medusa's conflict error type; test `test_cart_update_completed_cart_rejected` verifies
- [x] 4d.3 Fix `store_complete_cart` stub — returns `AppError::Conflict("Cart completion is not yet implemented")` with proper JSON body instead of bare `StatusCode::NOT_IMPLEMENTED`

### 4e. Configuration defaults

- [x] 4e.1 Add serde defaults to `AppConfig`: `HOST` → `"0.0.0.0"`, `PORT` → `3000`, `RUST_LOG` → `"toko_rs=debug,tower_http=debug"`; test `test_defaults_when_not_set` verifies with `serial_test` guard
- [x] 4e.2 Change `FindParams.limit` default: 50 → 20 to match Medusa's default pagination

### 4f. Spec reconciliation

- [x] 4f.1 Update `specs/foundation/spec.md` module boundary rule — added "P1 exception for cross-module SQL joins" with new scenario documenting cart → product_variants JOIN pattern
- [x] 4f.2 Verify all tests pass after audit fixes — 56 tests pass, clippy clean

## 5. Phase 1-B — Cart Module (DONE)

- [x] 5.1 Define cart models: Cart, CartLineItem, CartWithItems — CartWithItems includes computed `item_total` and `total` fields; `CartLineItem` includes `snapshot` JSON for variant info capture
- [x] 5.2 Define cart request/response types: CreateCartInput (currency_code defaults to "usd"), UpdateCartInput, AddLineItemInput (variant_id required, quantity >= 1), UpdateLineItemInput (quantity >= 0), CartResponse wraps CartWithItems
- [x] 5.3 Implement cart repository: `create_cart` (insert + empty items), `get_cart` (with items + computed totals), `update_cart` (COALESCE partial + completed-cart guard → 409), `mark_completed` (for Phase 1-C)
- [x] 5.4 Implement line item repository: `add_line_item` (variant lookup via SQL JOIN + snapshot + merge-same-variant quantity), `update_line_item` (soft-delete at qty 0), `delete_line_item` (soft-delete)
- [x] 5.5 Implement cart validation: completed-cart guard on `update_cart` and `add_line_item` → 409 Conflict; `AppError::Conflict` variant added matching Medusa's `"conflict"` error type
- [x] 5.6 Implement cart routes: POST /store/carts, GET /store/carts/:id, POST /store/carts/:id — all return `CartResponse` with computed totals
- [x] 5.7 Implement line item routes: POST /store/carts/:id/line-items, POST /store/carts/:id/line-items/:line_id, DELETE /store/carts/:id/line-items/:line_id — POST /store/carts/:id/complete returns 409 stub for Phase 1-C
- [x] 5.8 Wire cart repository into AppState — `CartRepository` in `Repositories` struct, initialized in `create_db()`
- [x] 5.9 Wire cart routes into main router — `app_router()` merges `cart::routes::router()`
- [x] 5.10 Write integration tests: 9 tests — create with defaults, create with email, validation, full flow (13 steps), same-variant merge, computed totals, completed cart rejected (update), completed cart rejected (add item), response format contract

## 6. Phase 1-C — Order Module (DONE)

- [x] 6.1 Define payment model: PaymentRecord struct with id, order_id, amount, currency_code, status, provider, metadata, timestamps
- [x] 6.2 Implement payment repository (single repo, SqlitePool): create, find_by_order_id
- [x] 6.3 Define order models: Order, OrderLineItem, OrderWithItems
- [x] 6.4 Define order response types: OrderResponse, OrderListResponse, CartCompleteResponse
- [x] 6.5 Implement order repository: create_from_cart (atomic transaction with display_id auto-increment, item copy, payment creation, cart completion), find_by_id, list_by_customer
- [x] 6.6 Implement order routes: POST /store/carts/:id/complete (public), GET /store/orders (auth), GET /store/orders/:id (auth)
- [x] 6.7 Wire order and payment repositories into AppState
- [x] 6.8 Wire order routes into main router (public + protected split)
- [x] 6.9 Write integration tests: 9 tests — cart→order, empty/completed/nonexistent cart errors, display_id increment, get by id, list by customer, auth guard

## 7. Phase 1-E — Integration Wiring

- [x] 7.1 Mount all module routes in main router: /admin/products/*, /store/products/*, /store/carts/*, /store/orders/*, /store/customers/*
- [x] 7.2 Apply middleware stack: TraceLayer + CorsLayer
- [x] 7.3 Wire AppState with all repository handles
- [x] 7.4 Implement health check with database connectivity test
- [x] 7.5 Wire customer routes into main router (done in Phase 1-D)
- [x] 7.6 Wire order routes into main router — public router merged directly, protected router behind auth_customer_id middleware
- [x] 7.7 Verify all endpoints respond correctly — 69 tests pass, clippy clean, 93% line coverage

## 8. Phase 1-F — Seed Data

- [ ] 8.1 Implement seed function with 3-5 sample products (all published, with options and variants)
- [ ] 8.2 Add 1 sample customer to seed
- [ ] 8.3 Make seed idempotent (check existence before inserting)
- [ ] 8.4 Wire --seed CLI flag to seed function
- [ ] 8.5 Smoke test full Browse → Cart → Checkout flow via curl

## 9. Phase 1-G — Test Suite

- [ ] 9.1 Create test infrastructure: setup_test_db (in-memory SQLite + migrations), create_test_app, helper functions
- [ ] 9.2 Write product tests: admin CRUD, store filtering, contract validation
- [ ] 9.3 Write cart tests: create, add/update/remove items, completed cart guard, quantity validation
- [ ] 9.4 Write order tests: full flow, empty/completed cart errors, display_id, payment record, customer filtering
- [ ] 9.5 Write customer tests: register, duplicate email, profile CRUD, auth header
- [ ] 9.6 Write contract tests: verify all response JSON shapes match API contract using assert-json-diff
- [ ] 9.7 Write error contract tests: verify all error responses include `code`, `type`, `message` fields matching specs/store.oas.yaml Error schema
- [ ] 9.8 Verify `cargo test` passes all tests with 100% endpoint coverage

## 10. Phase 1-H — Polish

- [ ] 10.1 Run `cargo clippy -- -D warnings` — zero warnings
- [ ] 10.2 Run `cargo fmt` — consistent formatting
- [ ] 10.3 Verify all `#[tracing::instrument]` annotations on handlers
- [ ] 10.4 Verify all 20 endpoints return correct Medusa-compatible JSON shapes

## 11. Architecture & TDD Quality Gates (cross-cutting)

- [x] 11.1 Verify module boundary rules: no cross-module imports (product does not import cart, etc.)
- [x] 11.2 Verify all shared infrastructure has unit tests (error.rs, config.rs, db.rs, seed.rs, lib.rs)
- [x] 11.3 Verify `cargo clippy -- -D warnings` passes with zero warnings
- [x] 11.4 Verify `cargo llvm-cov --summary-only` shows >90% line coverage
- [x] 11.5 Verify error responses match 3-field OAS Error schema (`code`, `type`, `message`) — implemented in Phase 2b.12
- [ ] 11.6 Verify contract tests reference Medusa vendor files for response shape validation
- [ ] 11.7 Verify HTTP method convention: POST for create AND update (no PUT) on all mutation endpoints
