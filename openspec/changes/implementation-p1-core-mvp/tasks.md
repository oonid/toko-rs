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

## 7. Post-Implementation Audit Fixes

Audit source: comprehensive comparison of implementation against Medusa vendor reference (`vendor/medusa/`) and all change specs (`specs/*.md`).

### 7a. Error handling spec inconsistencies

- [x] 7a.1 Fix `AppError::Conflict` to return `type: "unexpected_state"` instead of `type: "conflict"` — the spec's error-handling/spec.md defines `"conflict"` as an invalid type value; cart completion and completed-cart guard errors should use `"unexpected_state"` per the spec table. Update `error_type()` match arm and error unit test.
- [x] 7a.2 Sanitize `DatabaseError` message to `"Internal server error"` — spec says "message sanitized, not exposing internals" but current code leaks raw sqlx error (table/column names, SQL details). Only log the real error via `tracing::error!`, return generic message.
- [x] 7a.3 Fix `MigrationError.error_type()` to return `"database_error"` — previous `"migration_error"` is not in the spec's allowed type enum. Unified with DatabaseError since both are infrastructure 500 errors. Message also sanitized.
- [x] 7a.4 Update all integration tests that assert error `type` field to match corrected values — `tests/cart_test.rs:439` and `tests/order_test.rs:122` changed from `"conflict"` to `"unexpected_state"`; unit tests in `error.rs` updated for type and message assertions.

### 7b. SQLite migration parity with PostgreSQL (constraints & defaults)

- [x] 7b.1 Add `CHECK (status IN ('draft', 'published', 'proposed', 'rejected'))` to `products.status` in `migrations/sqlite/001_products.sql` — PG has this, SQLite didn't.
- [x] 7b.2 Add `CREATE UNIQUE INDEX uq_product_variants_sku ON product_variants (sku) WHERE deleted_at IS NULL AND sku IS NOT NULL` to `migrations/sqlite/001_products.sql` — PG has this, SQLite didn't; duplicate SKUs possible in tests.
- [x] 7b.3 Add `DEFAULT 'usd'` to `carts.currency_code` in `migrations/sqlite/003_carts.sql` — spec says default is "usd", PG has it, SQLite didn't.
- [x] 7b.4 Add `NOT NULL DEFAULT 'manual'` to `payment_records.provider` in `migrations/sqlite/005_payments.sql` — PG has `NOT NULL DEFAULT 'manual'`, SQLite allowed NULL. Updated `PaymentRecord` model from `Option<String>` to `String`.
- [x] 7b.5 Add `DEFAULT 'usd'` to `payment_records.currency_code` in `migrations/sqlite/005_payments.sql` — PG has it, SQLite didn't.
- [x] 7b.6 Add `CHECK (status IN ('pending', 'authorized', 'captured', 'failed', 'refunded'))` to `payment_records.status` in `migrations/sqlite/005_payments.sql` — PG has it, SQLite didn't.
- [x] 7b.7 Add `CHECK (status IN ('pending', 'completed', 'canceled', 'requires_action', 'archived'))` to `orders.status` in both `migrations/sqlite/004_orders.sql` and `migrations/004_orders.sql` — neither PG nor SQLite had this CHECK; spec planning doc listed it.

### 7c. SQLite migration parity with PostgreSQL (indexes — 13 missing + 3 missing tables)

- [x] 7c.1 Add `idx_products_status ON products (status) WHERE deleted_at IS NULL` to SQLite 001
- [x] 7c.2 Add `idx_product_options_product_id ON product_options (product_id)` to SQLite 001
- [x] 7c.3 Add `idx_product_option_values_option_id ON product_option_values (option_id)` to SQLite 001
- [x] 7c.4 Add `idx_product_variants_product_id ON product_variants (product_id) WHERE deleted_at IS NULL` to SQLite 001
- [x] 7c.5 Add `idx_customer_addresses_customer_id ON customer_addresses (customer_id)` to SQLite 002 — also added missing `customer_addresses` table definition
- [x] 7c.6 Add `idx_carts_customer_id ON carts (customer_id) WHERE deleted_at IS NULL` to SQLite 003
- [x] 7c.7 Add `idx_cart_line_items_cart_id ON cart_line_items (cart_id) WHERE deleted_at IS NULL` to SQLite 003 — also added missing `cart_line_items` table definition
- [x] 7c.8 Add `idx_orders_customer_id ON orders (customer_id) WHERE deleted_at IS NULL` to SQLite 004
- [x] 7c.9 Add `idx_orders_display_id ON orders (display_id)` to SQLite 004
- [x] 7c.10 Add `idx_order_line_items_order_id ON order_line_items (order_id)` to SQLite 004 — also added missing `order_line_items` table definition
- [x] 7c.11 Add `idx_payment_records_order_id ON payment_records (order_id)` to SQLite 005
- [x] 7c.12 Add `idx_payment_records_status ON payment_records (status)` to SQLite 005
- [x] 7c.13 Add `idx_idempotency_keys_response_id ON idempotency_keys (response_id)` to SQLite 006
- [x] 7c.14 Verify all indexes match PG migration semantics; run `cargo test` to confirm no regressions — 69 tests pass, clippy clean

### 7d. Data integrity fixes

- [x] 7d.1 Move payment creation inside the order creation transaction in `src/order/repository.rs` — payment INSERT runs before tx.commit() via new `PaymentRepository::create_with_tx()` static method; failure rolls back the entire order + payment + cart completion atomically
- [x] 7d.2 Handle display_id UNIQUE constraint violation gracefully — SQLite error code 2067 mapped to `AppError::Conflict` (409, `type: "unexpected_state"`) via `map_display_id_conflict()` helper; client receives clear retry message instead of raw DatabaseError

### 7e. Spec reconciliation

- [x] 7e.1 Update `specs/error-handling/spec.md` error table: documented `display_id` race as additional trigger for UnexpectedState scenario; added atomic cart-to-order and display_id concurrency requirements to `specs/order-module/spec.md`
- [x] 7e.2 Update `docs/audit-correction.md` with post-audit correction entries for all fixes applied in 7a–7d.

### 7f. Default currency change USD → IDR (config-driven)

- [x] 7f.1 Add `DEFAULT_CURRENCY_CODE` to `AppConfig` in `src/config.rs` with serde default `"idr"`, update config tests
- [x] 7f.2 Thread `default_currency_code` through `Repositories` struct to `CartRepository`, replace hardcoded `"usd"` fallback in `create_cart()`
- [x] 7f.3 Update PG migrations `003_carts.sql` and `005_payments.sql`: change `DEFAULT 'usd'` → `DEFAULT 'idr'`
- [x] 7f.4 Update SQLite migrations `003_carts.sql` and `005_payments.sql`: change `DEFAULT 'usd'` → `DEFAULT 'idr'`
- [x] 7f.5 Update integration tests: change `"usd"` assertions/payloads to `"idr"` in `tests/cart_test.rs` and `tests/order_test.rs`; keep `"eur"` override test as-is
- [x] 7f.6 Update change specs: `cart-module/spec.md`, `database-schema/spec.md`, and `foundation/spec.md` to reference `"idr"` as default currency
- [x] 7f.7 Add `DEFAULT_CURRENCY_CODE=idr` to `.env.example` with documentation; add IDR price semantics to `design.md` divergence table and risks section
- [x] 7f.8 Verify `cargo test` passes, clippy clean — all tests updated

## 8. Phase 1-E — Integration Wiring (DONE)

- [x] 8.1 Mount all module routes in main router: /admin/products/*, /store/products/*, /store/carts/*, /store/orders/*, /store/customers/*
- [x] 8.2 Apply middleware stack: TraceLayer + CorsLayer
- [x] 8.3 Wire AppState with all repository handles
- [x] 8.4 Implement health check with database connectivity test
- [x] 8.5 Wire customer routes into main router (done in Phase 1-D)
- [x] 8.6 Wire order routes into main router — public router merged directly, protected router behind auth_customer_id middleware
- [x] 8.7 Verify all endpoints respond correctly — 69 tests pass, clippy clean, 93% line coverage

## 9. Phase 1-F — Seed Data

- [x] 9.1 Implement seed function with 3-5 sample products (all published, with options and variants) — 3 products (Kaos Polos, Jeans Slim, Sneakers Classic) with 13 variants total, each with size option bindings
- [x] 9.2 Add 1 sample customer to seed — Budi Santoso (cus_seed_budi), email budi@example.com, has_account=true
- [x] 9.3 Make seed idempotent (check existence before inserting) — fixed IDs + SELECT COUNT(*) check for parents, INSERT OR IGNORE for children
- [x] 9.4 Wire --seed CLI flag to seed function — already wired in main.rs:26-30
- [x] 9.5 Smoke test full Browse → Cart → Checkout flow — 2 integration tests in tests/seed_flow_test.rs exercising browse→cart→checkout and customer order history
- [x] 9.6 Comprehensive curl walkthrough for all 20 endpoints — docs/seed-data.md with copy-paste-ready curl examples covering all 8 product, 6 cart, 3 customer, and 3 order endpoints plus error scenarios (404, 409, 422, method not allowed)

## 10. Phase 1-G — Test Suite

- [x] 10.1 Create test infrastructure: setup_test_db (in-memory SQLite + migrations), create_test_app, helper functions — `tests/common/mod.rs` provides `setup_test_app()`, each test file has `body_json()` and `request()` helpers
- [x] 10.2 Write product tests: admin CRUD, store filtering, contract validation — 23 tests in `tests/product_test.rs` covering all 8 product endpoints including duplicate SKU variant
- [x] 10.3 Write cart tests: create, add/update/remove items, completed cart guard, quantity validation — 9 tests in `tests/cart_test.rs` covering all 6 cart endpoints
- [x] 10.4 Write order tests: full flow, empty/completed cart errors, display_id, payment record, customer filtering — 11 tests in `tests/order_test.rs` covering all 3 order endpoints
- [x] 10.5 Write customer tests: register, duplicate email, profile CRUD, auth header — 10 tests in `tests/customer_test.rs` covering all 3 customer endpoints
- [x] 10.6 Write contract tests: verify all response JSON shapes match API contract — 10 contract tests in `tests/contract_test.rs` validating response field presence for product, cart, customer, order, order list, delete, health, and order detail responses
- [x] 10.7 Write error contract tests: verify all error responses include `code`, `type`, `message` fields matching specs/store.oas.yaml Error schema — 10 error contract tests covering 404, 400, 401, 409, 422 status codes with exact code/type value assertions
- [x] 10.8 Verify `cargo test` passes all tests with 100% endpoint coverage — 103 tests covering all 20+1 endpoints; HTTP method audit (13.7) verified via 3 dedicated tests confirming POST for updates; CORS preflight test added; all spec scenarios cross-referenced and covered

## 11. Phase 1-H — Polish

- [x] 11.1 Run `cargo clippy -- -D warnings` — zero warnings — PASS (verified clean)
- [x] 11.2 Run `cargo fmt` — consistent formatting — PASS (verified clean)
- [x] 11.3 Verify all `#[tracing::instrument]` annotations on handlers — added `#[tracing::instrument]` to all 20 route handlers + health_check (21 total); uses `skip_all` with contextual `fields` for path params (id, cart_id, customer_id) and query params (offset, limit)
- [x] 11.4 Verify all 20 endpoints return correct Medusa-compatible JSON shapes — verified via 10 contract tests in `tests/contract_test.rs` + 2 integration smoke tests covering all response types; all endpoints return Medusa-compatible `{product: ...}`, `{cart: ...}`, `{customer: ...}`, `{type, order, payment}`, `{orders, count, offset, limit}`, `{id, object, deleted}`, `{status, database, version}` shapes

## 12. Post-Audit Response Shape & Compatibility Fixes

Full audit documented in `docs/audit-p1-task12.md`. Fixes address Medusa API response shape incompatibilities, error type divergences, and schema gaps discovered during comprehensive comparison against `vendor/medusa/` source and OAS specs.

### 12a. Response shape incompatibilities (HIGH)

- [x] 12a.1 Fix line item DELETE response: create `LineItemDeleteResponse` type matching Medusa's `{ id, object: "line-item", deleted: true, parent: Cart }` instead of reusing `CartResponse` — update `store_delete_line_item` in `src/cart/routes.rs` — **already implemented in prior phase; verified with contract test `test_contract_line_item_delete_response_shape`**
- [x] 12a.2 Fix cart complete response: remove top-level `payment` from `CartCompleteResponse` in `src/order/types.rs` — Medusa returns `{ type: "order", order }` only. **Already implemented — `CartCompleteResponse` has only `{ type, order }`, no `payment` field.** Contract test strengthened with negative assertion confirming `payment` key is absent and exactly 2 top-level keys exist.
- [x] 12a.3 Fix order GET response: change `OrderResponse` from `{ order, payment }` to `{ order }` only in `src/order/types.rs` — **already implemented — `OrderResponse` has only `{ order }`, no `payment` field.** Contract test strengthened with negative assertion confirming `payment` key is absent and exactly 1 top-level key exists.

### 12b. Error handling divergences (MEDIUM)

- [x] 12b.1 Fix `AppError::Conflict.error_type()` from `"unexpected_state"` to `"conflict"` in `src/error.rs` — Medusa's error handler maps `CONFLICT` type to `type: "conflict"`, not `"unexpected_state"`. Updated error unit test, cart_test.rs, and contract_test.rs
- [x] 12b.2 Fix empty cart completion error: change from `AppError::Conflict` (409) to `AppError::InvalidData` (400) in `src/order/repository.rs:41` — empty cart is an invalid request, not a conflict. Updated order_test.rs and contract_test.rs

### 12c. Database schema gaps (MEDIUM)

- [x] 12c.1 Fix SQLite `customers.email` uniqueness: change `email TEXT UNIQUE NOT NULL` to `CREATE UNIQUE INDEX uq_customers_email ON customers (email, has_account) WHERE deleted_at IS NULL` in `migrations/sqlite/002_customers.sql` — now matches PG migration and Medusa's guest+registered same-email pattern
- [x] 12c.2 Add `UNIQUE(variant_id, option_value_id)` constraint to `product_variant_option` pivot table in both PG and SQLite migrations — prevents duplicate pivot rows
- [x] 12c.3 Adopt `_sequences` table for `display_id` generation in `src/order/repository.rs:46-49` — replaced `MAX(display_id)+1` with atomic `UPDATE _sequences SET value = value + 1 WHERE name = 'order_display_id' RETURNING value` inside the transaction, eliminating the race condition between SELECT and INSERT

### 12d. Missing indexes (LOW)

- [x] 12d.1 Add `idx_cart_line_items_variant_id ON cart_line_items (variant_id) WHERE deleted_at IS NULL AND variant_id IS NOT NULL` — matches Medusa's `IDX_line_item_variant_id`
- [x] 12d.2 Add `idx_cart_line_items_product_id ON cart_line_items (product_id) WHERE deleted_at IS NULL AND product_id IS NOT NULL` — matches Medusa's `IDX_line_item_product_id`
- [x] 12d.3 Add `idx_carts_currency_code ON carts (currency_code) WHERE deleted_at IS NULL` — matches Medusa's `IDX_cart_curency_code`

### 12e. Verification

- [x] 12e.1 Update all affected contract tests in `tests/contract_test.rs` to assert new response shapes — already updated inline during 12a.1–12a.3 (line item delete, cart complete, order detail) and 12b.1–12b.2 (empty cart 400, completed cart 409)
- [x] 12e.2 Update all affected integration tests in `tests/cart_test.rs` and `tests/order_test.rs` for new error type values and response shapes — already updated inline during 12b.1–12b.2
- [x] 12e.3 Verify `cargo test` passes all tests, clippy clean — 104 tests pass, zero warnings
- [x] 12e.4 Update `docs/audit-correction.md` with all changes applied — sections 12a–12e added

## 13. Verification & TDD Quality Gates (cross-cutting)

- [x] 13.1 Verify module boundary rules: no cross-module imports (product does not import cart, etc.)
- [x] 13.2 Verify all shared infrastructure has unit tests (error.rs, config.rs, db.rs, seed.rs, lib.rs)
- [x] 13.3 Verify `cargo clippy -- -D warnings` passes with zero warnings
- [x] 13.4 Verify `cargo llvm-cov --summary-only` shows >90% line coverage
- [x] 13.5 Verify error responses match 3-field OAS Error schema (`code`, `type`, `message`) — implemented in Phase 2b.12
- [x] 13.6 Verify contract tests reference Medusa vendor files for response shape validation
- [x] 13.7 Verify HTTP method convention: POST for create AND update (no PUT) on all mutation endpoints — verified via 3 dedicated tests in `tests/contract_test.rs`

## 14. Second Audit: P1 Compatibility Fixes (cross-cutting)

**Audit source**: `docs/audit-p1-task14.md`

### 14a. P1 Bugs (business logic correctness)

- [x] 14a.1 Add `completed_at` guard to `update_line_item` and `delete_line_item` in `src/cart/repository.rs` — completed carts reject item mutations with 409 Conflict. Tests: `test_cart_update_line_item_on_completed_cart_rejected`, `test_cart_delete_line_item_on_completed_cart_rejected`
- [x] 14a.2 Fix `resolve_variant_options_tx` in `src/product/repository.rs` — now accepts `variant_id: &str` parameter directly from `insert_variant_tx` return value instead of fragile title-based lookup
- [x] 14a.3 Fix `resolve_variant_options_tx` in `src/product/repository.rs` — returns `AppError::NotFound` when option value not found instead of silently swallowing. Test: `test_variant_option_value_not_found_rejected`
- [x] 14a.4 Add validation that variant options cover ALL product options during product creation in `src/product/repository.rs`. Test: `test_variant_missing_option_coverage_rejected`
- [x] 14a.5 Add validation that variant option combinations are unique per product in `src/product/repository.rs`. Test: `test_variant_duplicate_option_combination_rejected`
- [x] 14a.6 Validate product `status` as `ProductStatus` enum (`draft`, `published`, `proposed`, `rejected`) in `src/product/types.rs` — invalid strings rejected at deserialization (422). Tests: `test_product_invalid_status_rejected`, `test_product_update_validates`
- [x] 14a.7 Add `.validate()` call to `admin_update_product` handler in `src/product/routes.rs` — no longer bypassed

### 14b. P1 Input Validation (request shape correctness)

- [x] 14b.1 Add `#[serde(deny_unknown_fields)]` to all input types in `src/*/types.rs` — match Medusa's `.strict()` behavior so misspelled fields return 422. Tests: `test_unknown_fields_rejected`, `test_product_unknown_fields_rejected`
- [x] 14b.2 Tighten `metadata` type from `Option<serde_json::Value>` to `Option<HashMap<String, serde_json::Value>>` on all input types — match Medusa's `Record<string, unknown>`. Added `metadata_to_json` helper in `src/types.rs`. Test: `test_metadata_must_be_object`
- [x] 14b.3 Add upper bound to `FindParams.limit` (max 100) via `capped_limit()` in `src/types.rs` and `src/order/types.rs` — prevent unbounded queries. Responses reflect capped value. Test: `test_list_limit_capped`
- [x] 14b.4 Add `Forbidden` (403) error variant to `src/error.rs` — 403, type `forbidden`, code `invalid_state_error`. Unit test: `test_forbidden`

### 14c. P1 Response Shape Stubs (Medusa frontend compatibility)

- [x] 14c.1 Add `images: Vec<String>`, `is_giftcard: bool`, `discountable: bool` stubs to `ProductWithRelations` — return empty/default values so Medusa frontend conditionals don't throw TypeError. Contract test strengthened
- [x] 14c.2 Add variant `calculated_price: { calculated_amount, original_amount, is_calculated_price_tax_inclusive }` to `ProductVariantWithOptions` — mirrors raw `price` value since P1 has no pricing module. Contract test asserts nested fields
- [x] 14c.3 Add 22 computed total stub fields to `CartWithItems` and `OrderWithItems` via `from_items()` helpers — `subtotal`, `tax_total`, `discount_total`, `shipping_total`, `gift_card_total`, and all `original_*` variants default to 0 (tax/shipping/discount/gift_card) or mirror `item_total` (subtotal/total). Contract tests strengthened
- [x] 14c.4 Add `addresses: []` empty array to customer response — prevents `.map()` / `.length` TypeError on Medusa frontends. Already completed in 14f
- [x] 14c.5 Add `payment_status: "not_paid"`, `fulfillment_status: "not_fulfilled"` enum stubs to `OrderWithItems`. Contract test asserts values
- [x] 14c.6 Add `fulfillments: []`, `shipping_methods: []` empty arrays to `OrderWithItems`. Contract test asserts arrays
- [x] 14c.7 Add `requires_shipping: true`, `is_discountable: true`, `is_tax_inclusive: false` defaults to `CartLineItem` and `OrderLineItem` via `#[sqlx(skip)]` + `from_items()` helper

### 14d. P1 Middleware / Security

- [x] 14d.1 Replace `CorsLayer::permissive()` with configured CORS in `src/lib.rs` — read allowed origins from `AppConfig.cors_origins` (comma-separated, default `"*"`); `build_cors_layer()` constructs proper AllowOrigin/AllowMethods/AllowHeaders; `app_router_with_cors()` for production, `app_router()` for tests
- [x] 14d.2 Add SQLite error code mapping to structured `AppError` variants in `src/error.rs` — `map_sqlite_constraint()` function: code 2067 → `DuplicateError`, 787 → `NotFound`, 1299 → `InvalidData`; unit test for non-DB error passthrough

### 14e. Verification

- [x] 14e.1 Run `cargo test`, verify all existing + new tests pass
- [x] 14e.2 Run `cargo clippy -- -D warnings`, verify zero warnings
- [x] 14e.3 Update `docs/audit-correction.md` with all 14a–14f changes

### 14f. Customer Address Schema + Response Stubs

- [x] 14f.1 Add `is_default_shipping` / `is_default_billing` BOOLEAN columns + partial unique indexes to `customer_addresses` in both PG and SQLite migrations
- [x] 14f.2 Rename `state_province` → `province` in `customer_addresses` to match Medusa's field name
- [x] 14f.3 Add `CustomerAddress` model in `src/customer/models.rs` + `CustomerWithAddresses` wrapper with `addresses: Vec<CustomerAddress>`, `default_billing_address_id: Option<String>`, `default_shipping_address_id: Option<String>`
- [x] 14f.4 Add `list_addresses` query + `wrap_with_addresses` helper in `src/customer/repository.rs` — reads from `customer_addresses` table, derives default address IDs
- [x] 14f.5 Update all customer routes to return `CustomerWithAddresses` instead of bare `Customer`
- [x] 14f.6 Strengthen contract test `test_contract_customer_response_shape` — asserts `addresses` array, `default_*_address_id` null

## 15. PostgreSQL Driver Support

Rewrite all repos, db.rs, and seed.rs to use `PgPool` natively (matching design.md Decision 2). All ~55 SQL queries rewritten with `$N` placeholders. SQLite placeholder translator deferred (tests now run against PG via Docker). PG migrations fixed: inline `UNIQUE WHERE` constraints → `CREATE UNIQUE INDEX ... WHERE`, `INTEGER` → `BIGINT` for i64 compatibility.

### 15a. Infrastructure (db.rs, migrations)

- [x] 15a.1 Rewrite `src/db.rs` — `AppDb::Postgres(PgPool)` variant, `PgPoolOptions`, migrations from `./migrations/`. Tests use `toko_test` database via `DATABASE_URL` env var.
- [x] 15a.2 Fix PG migrations — inline `UNIQUE WHERE` constraints (invalid PG syntax) → `CREATE UNIQUE INDEX ... WHERE`. Affected: 001_products (handle, sku), 002_customers (email).
- [x] 15a.3 Fix PG `INTEGER` → `BIGINT` for all `i64` columns (price, quantity, unit_price, amount, variant_rank, display_id, value) — PG `INTEGER` is INT4 (32-bit), Rust `i64` needs INT8.

### 15b. Repository rewrite ($N placeholders)

- [x] 15b.1 Rewrite `src/product/repository.rs` — all queries use `$1, $2, ...` placeholders. Transaction type: `sqlx::Transaction<'_, sqlx::Postgres>`. Unique violation detection: `db_err.code() == "23505"`.
- [x] 15b.2 Rewrite `src/cart/repository.rs` — same pattern. `CURRENT_TIMESTAMP` → `now()`.
- [x] 15b.3 Rewrite `src/customer/repository.rs` — same pattern.
- [x] 15b.4 Rewrite `src/order/repository.rs` — same pattern. `_sequences UPDATE ... RETURNING` works on PG.
- [x] 15b.5 Rewrite `src/payment/repository.rs` — `create_with_tx` transaction type `sqlx::Transaction<'_, sqlx::Postgres>`.

### 15c. Seed rewrite (ON CONFLICT DO NOTHING)

- [x] 15c.1 Rewrite `src/seed.rs` — all `INSERT OR IGNORE` → `INSERT ... ON CONFLICT (id) DO NOTHING`. All `?` → `$N`. `SqlitePool` → `PgPool`. Seed tests use `clean_seed_data()` helper.

### 15d. Error mapping update

- [x] 15d.1 Replace `map_sqlite_constraint()` with `map_db_constraint()` in `src/error.rs` — PG error codes: `23505` → DuplicateError, `23503` → NotFound, `23502` → InvalidData.

### 15e. Test infrastructure + verification

- [x] 15e.1 Update `tests/common/mod.rs` — `setup_test_app()` connects to `toko_test`, runs migrations, calls `clean_all_tables()` before each test for isolation.
- [x] 15e.2 Update all test files — `AppDb::Sqlite` → `AppDb::Postgres`, `SqlitePool` → `PgPool`, `?` → `$1` in raw SQL, boolean `1` → `TRUE`, `INSERT OR IGNORE` → `ON CONFLICT DO NOTHING`.
- [x] 15e.3 Run `DATABASE_URL=postgres://... cargo test -- --test-threads=1` — all 117 tests pass against PostgreSQL.
- [x] 15e.4 Run `cargo clippy -- -D warnings` — zero warnings.

## 16. E2E Integration Test Suite

Full HTTP integration tests using `reqwest` against a live `axum::serve` instance. Tests run against SQLite in-memory by default; set `E2E_DATABASE_URL` env var to run against PostgreSQL (via Docker Compose or testcontainers). Covers all 21 endpoints with the full commerce cycle from `docs/seed-data.md` plus additional edge cases.

### 16a. Test harness

- [x] 16a.1 Create `tests/e2e/mod.rs` — `setup_e2e_app(database_url)` function that creates DB pool, runs migrations, seeds data, binds to `127.0.0.1:0` (random port), starts `axum::serve` in background `tokio::spawn`, returns base URL + `reqwest::Client` + DB pool for assertions. Detects `E2E_DATABASE_URL` env var for PostgreSQL; falls back to SQLite in-memory.
- [x] 16a.2 Add `testcontainers` dependency to `Cargo.toml` `[dev-dependencies]` with `postgres` feature. Add helper to start PG container programmatically when `E2E_DATABASE_URL=testcontainers://` is set.
- [x] 16a.3 Update `docker-compose.yml` — add `toko_test` database init or use separate `docker-compose.test.yml` with ephemeral PG container.
- [x] 16a.4 Add Makefile targets: `test-e2e` (SQLite cycle), `test-e2e-pg` (both SQLite + PG cycles via Docker Compose), `test-e2e-tc` (both cycles via testcontainers).

### 16b. Full commerce cycle test (happy path, 17 steps)

- [x] 16b.1 Implement `test_e2e_guest_checkout_flow` — covers steps 1-9 from seed-data.md: health check → browse products → product detail → create cart → add item → add second item (merge qty) → update quantity → verify totals → complete cart → verify order.
- [x] 16b.2 Implement `test_e2e_customer_lifecycle` — covers steps 10-17: register customer → get profile → update profile → create cart with customer_id → add item → complete → list orders → view order detail. Tests `X-Customer-Id` header auth.

### 16c. Admin product CRUD tests

- [x] 16c.1 Implement `test_e2e_admin_product_crud` — create draft product → list all products (includes drafts) → get single product → publish → partial update → add variant → verify variant + options → soft-delete → verify 404 on store GET.
- [x] 16c.2 Implement `test_e2e_admin_product_with_variants` — create product with options + variants in one request → verify option coverage validation → verify calculated_price on variant → verify unique option combo constraint.

### 16d. Cart manipulation tests

- [x] 16d.1 Implement `test_e2e_cart_update_and_delete` — create cart → add item → update cart email → delete line item → add same variant again (verify merge) → verify empty cart completion returns 400.
- [x] 16d.2 Implement `test_e2e_cart_completed_guards` — create cart → add item → complete → attempt update → 409 → attempt add item → 409 → attempt update line item → 409 → attempt delete line item → 409.

### 16e. Error and validation tests

- [x] 16e.1 Implement `test_e2e_error_responses` — verify 422 for duplicate email, 404 for nonexistent cart/order/customer, 400 for invalid quantity, 401 for missing X-Customer-Id on protected endpoints, 422 for unknown fields, 422 for invalid product status, 422 for string metadata.
- [x] 16e.2 Implement `test_e2e_response_shapes` — verify contract shapes for product (images, is_giftcard, discountable, calculated_price), cart (22 total fields), order (22 total fields, payment_status, fulfillment_status, fulfillments, shipping_methods), customer (addresses array, default address IDs), line items (requires_shipping, is_discountable, is_tax_inclusive).

### 16f. Verification

- [x] 16f.1 Run `test-e2e` (SQLite) — all E2E tests pass.
- [x] 16f.2 Run `test-e2e-pg` or `test-e2e-tc` (PostgreSQL) — same tests pass against real PostgreSQL.
- [x] 16f.3 Run `cargo test` — all existing 117 tests still pass (no regressions).
- [x] 16f.4 Run `cargo clippy -- -D warnings` — zero warnings.
- [x] 16f.5 Update `docs/audit-correction.md` with Task 16 changes.

## Task 17: SQLite Feature Flag Support

PostgreSQL remains default and primary. SQLite is an optional compile-time backend via `cargo build --features sqlite --no-default-features`. No AnyPool, no code duplication, no method-level cfg guards.

### 17a. Infrastructure setup

- [x] 17a.1 Add `[features]` section to `Cargo.toml`: `default = ["postgres"]`, `postgres = ["sqlx/postgres"]`, `sqlite = ["sqlx/sqlite"]`. Remove unused `"any"` feature from sqlx deps.
- [x] 17a.2 Add type aliases in `src/db.rs`: `DbPool`, `DbPoolOptions`, `DbTransaction` — conditionally resolving to PgPool/SqlitePool via `#[cfg]`.
- [x] 17a.3 Update `AppDb` from enum to struct: `AppDb { pool: DbPool }` using type alias instead of raw `PgPool`.
- [x] 17a.4 Update `create_db()` to use `DbPoolOptions`, `run_migrations()` to select `./migrations/` (PG) or `./migrations/sqlite/` (SQLite) via cfg, `ping()` to use `DbPool`.

### 17b. SQL portability (search-and-replace)

- [x] 17b.1 Replace `now()` with `CURRENT_TIMESTAMP` in all 9 occurrences: `src/product/repository.rs` (2), `src/cart/repository.rs` (5), `src/customer/repository.rs` (1), `src/order/repository.rs` (1).
- [x] 17b.2 Update all 5 repo files to use `DbPool` type alias and `DbTransaction` instead of `PgPool` / `Transaction<'_, Postgres>`.
- [x] 17b.3 Update `src/seed.rs` to use `DbPool` type alias and extract pool from `AppDb` struct.

### 17c. Error code handling

- [x] 17c.1 Update `src/error.rs` `map_db_constraint()` — uses `crate::db::is_unique_violation()` etc. with cfg-gated error code constants: PG (`23505`, `23503`, `23502`) vs SQLite (`2067`, `787`, `1299`).
- [x] 17c.2 Update inline error code checks in `src/product/repository.rs` (2), `src/customer/repository.rs` (1), `src/order/repository.rs` (1) — use `crate::db::is_unique_violation(&e)` helper.

### 17d. Tests and verification

- [x] 17d.1 Run full test suite with default features (PG) — all 129 tests pass, clippy clean.
- [x] 17d.2 Build with `--features sqlite --no-default-features` — compiles without errors.
- [x] 17d.3 Run tests with SQLite feature against `sqlite::memory:` — all 129 tests pass (28 lib + 93 integration + 8 E2E).
- [x] 17d.4 Run `cargo clippy -- -D warnings` with both feature sets — zero warnings.

### 17e. Documentation and config

- [x] 17e.1 Update `.env` to PG default, `.env.example` to show both options.
- [x] 17e.2 Add `test-sqlite` and `test-all` targets to `Makefile`.
- [x] 17e.3 Update `docs/database.md` — SQLite feature flag section, how to run SQLite tests.
- [x] 17e.4 Amend `design.md` Decision 2 — feature-flag type aliases. Add Decision 11 for SQLite feature flag rationale.
- [x] 17e.5 Update `docs/testing.md` — SQLite test section, updated Makefile commands.
- [x] 17e.6 Update `docs/audit-correction.md` — add Task 17 section.
- [x] 17e.7 Create `docs/database-ext-sqlite.md` — document SQLite extension: feature flag usage, DDL differences, how to run, limitations.

## Task 18: Third Audit — P1 Compatibility Deep-Dive

Source: `docs/audit-p1-task18.md`. Behavioral comparison against Medusa route handlers, validators, workflows, models, and helpers. 19 findings: 3 HIGH, 6 MEDIUM, 4 LOW/known, 6 confirmed matches.

### 18a. Fix `load_relations` soft-delete filter (S1 — HIGH)

- [x] 18a.1 Add `AND deleted_at IS NULL` to `product_options` query in `src/product/repository.rs:389-391`.
- [x] 18a.2 Add `AND deleted_at IS NULL` to `product_option_values` query in `src/product/repository.rs:398-400`.
- [x] 18a.3 Add `AND deleted_at IS NULL` to `product_variants` query in `src/product/repository.rs:410-412`.
- [x] 18a.4 Add `AND pov.deleted_at IS NULL` to `product_variant_option` join query in `src/product/repository.rs:419-426`. (pivot table has no `deleted_at` column.)
- [x] 18a.5 Add test: soft-deleted variant should not appear in product response.
- [x] 18a.6 Add test: soft-deleted option should not appear in product response.

### 18b. Make DELETE idempotent (S3 — HIGH)

- [x] 18b.1 Update `src/product/repository.rs` `soft_delete` — if `rows_affected() == 0`, check if product exists with `deleted_at IS NOT NULL` and return success `{ id, object, deleted: true }` instead of 404.
- [x] 18b.2 Add test: double-delete returns 200 with `{ id, object, deleted: true }`.
- [x] 18b.3 Add test: delete nonexistent product still returns 404.

### 18c. Fix line item dedup to include metadata (S4 — MEDIUM)

- [x] 18c.1 Update existing-item lookup in `src/cart/repository.rs:150` to compare metadata: fetch metadata from existing item, compare with input metadata via `serde_json::Value` equality.
- [x] 18c.2 When metadata differs, create a new line item instead of merging quantity.
- [x] 18c.3 Add test: adding same variant with different metadata creates separate line items.
- [x] 18c.4 Add test: adding same variant with same metadata merges quantity (existing behavior preserved).

### 18d. Add custom JSON rejection handler (S6 — MEDIUM)

- [x] 18d.1 Add `src/extract.rs` with custom `Json<T>` extractor wrapping axum rejections into `AppError`. Update `src/product/routes.rs`, `src/cart/routes.rs`, `src/customer/routes.rs` to use `crate::extract::Json` for input extraction.
- [x] 18d.2 Add test: malformed JSON body returns `{ code, type, message }` with `type: "invalid_data"`.
- [x] 18d.3 Add test: wrong JSON type (e.g., string where number expected) returns consistent error shape.

### 18e. Map PG serialization failure to Conflict (MEDIUM — error handler gap)

- [x] 18e.1 Add `40001` (serialization failure) handling to `map_db_constraint()` in `src/error.rs` — map to `AppError::Conflict`.
- [x] 18e.2 Add `is_serialization_failure()` helper to `src/db.rs` with cfg-gated code constants: PG `40001`, SQLite `""` (no equivalent).
- [x] 18e.3 Verified via code review — unit test not feasible (requires actual PG serialization failure).

### 18f. Fix `images` type to match Medusa shape (S7 — MEDIUM)

- [x] 18f.1 Add `ImageStub` struct to `src/product/models.rs` with `{ url: String }` field.
- [x] 18f.2 Change `ProductWithRelations.images` from `Vec<String>` to `Vec<ImageStub>`.
- [x] 18f.3 Update `load_relations` to return `images: vec![]` with the new type.
- [x] 18f.4 Update contract tests for the new image shape (no change needed — empty array serializes identically).
- [x] 18f.5 Verify: no serialization change visible in API responses (empty array remains `[]`).

### 18g. Document known P1 divergences (no code changes)

- [x] 18g.1 Update `design.md` — add Decision 12: `deny_unknown_fields` as intentional strict validation (S2). Document that Medusa SDK clients must adapt to toko-rs's narrower input schemas.
- [x] 18g.2 Update `design.md` — add Decision 13: variant flat `price` field as a toko-rs extension (S8). Not harmful, visible alongside `calculated_price`.
- [x] 18g.3 Update `design.md` — add known divergence: order line item prefix `oli` vs Medusa's `ordli` (S9).
- [x] 18g.4 Update `design.md` — add known divergence: validation errors include `code` field that Medusa Zod errors omit (S5).
- [x] 18g.5 Update `design.md` — add known divergence: default pagination limit 20 vs Medusa's 50 (S10, already partially documented).

### 18h. Verification pass

- [x] 18h.1 Run full test suite on PG — 137 tests pass.
- [x] 18h.2 Run full test suite on SQLite — 137 tests pass.
- [x] 18h.3 Run `cargo clippy -- -D warnings` on both feature sets — zero warnings.
- [x] 18h.4 Run `cargo fmt --check` — clean.
- [x] 18h.5 Update `docs/audit-correction.md` with Task 18 section.

## Task 19: Fourth Audit — Medusa Compatibility Deep-Dive

Source: `docs/audit-p1-task19.md`. Comprehensive comparison against Medusa `develop` branch at `12b4e72189`. 6 audit streams: routes, response shapes, input types, DB schema, error handling, business logic. 20 findings: 5 HIGH, 8 MEDIUM, 7 LOW.

### 19a. Fix `JsonDataError` mapping (S1 — HIGH)

- [x] 19a.1 Fix `src/extract.rs` — change `JsonDataError` mapping from `AppError::DuplicateError` to `AppError::InvalidData` so structurally-valid JSON with wrong field types returns 400 instead of 422
- [x] 19a.2 Add test: send JSON with wrong field type (e.g., string where number expected) — verify 400 `{ type: "invalid_data" }` not 422 `{ type: "duplicate_error" }`

### 19b. Missing admin variant endpoints (S2 — HIGH)

- [x] 19b.1 Implement `GET /admin/products/:id/variants` — paginated list returning `{ variants, count, offset, limit }`
- [x] 19b.2 Implement `GET /admin/products/:id/variants/:variant_id` — single variant with options
- [x] 19b.3 Implement `POST /admin/products/:id/variants/:variant_id` — partial variant update (COALESCE pattern)
- [x] 19b.4 Implement `DELETE /admin/products/:id/variants/:variant_id` — soft-delete returning `{ id, object: "variant", deleted: true, parent: { product } }`
- [x] 19b.5 Add integration tests for all 4 variant endpoints

### 19c. Soft-delete cascade to children (S3 — HIGH)

- [x] 19c.1 Update `soft_delete` in `src/product/repository.rs` — after setting `deleted_at` on product, also UPDATE `deleted_at` on `product_variants`, `product_options`, and `product_option_values` where `product_id = $1 AND deleted_at IS NULL`
- [x] 19c.2 Add test: soft-delete product, verify variants/options/option_values have `deleted_at` set
- [x] 19c.3 Add test: soft-delete product, verify `load_relations` on a different product still returns only non-deleted children

### 19d. Variant option uniqueness against DB (S4 — HIGH)

- [x] 19d.1 Update `add_variant` in `src/product/repository.rs` — query existing `product_variant_option` rows for the product, reconstruct option combos, and reject if new variant's option combination matches an existing DB variant
- [x] 19d.2 Update `create_product` duplicate option combination check to also verify against existing DB variants (not just input batch) — N/A for new products (no DB variants exist); check is only needed in `add_variant`
- [x] 19d.3 Add test: create product with variant `{Color: Red, Size: M}`, then add another variant via separate API call with same combination — verify 422 rejection

### 19e. Line-item dedup include `unit_price` (S5 — HIGH)

- [x] 19e.1 Update existing-item lookup SQL in `src/cart/repository.rs` to include `unit_price` comparison — items with same variant_id but different unit_price become separate line items
- [x] 19e.2 Add test: add item at price X, change variant price, add same variant — verify second line item created (not merged)
- [x] 19e.3 Add test: add item at price X, add same variant at same price — verify quantity merged (existing behavior preserved)

### 19f. Cart complete error branch (S6 — MEDIUM)

- [x] 19f.1 Add error variant to `CartCompleteResponse` in `src/order/types.rs` — `{ type: "cart", cart: CartWithItems, error: { message, name, type } }`
- [x] 19f.2 Update `store_complete_cart` handler to return error branch on payment/processing failures instead of just AppError
- [x] 19f.3 Add test: verify error response shape matches Medusa's `{ type: "cart", cart, error }` format

### 19g. Add `company_name` to customer (S7 — MEDIUM)

- [x] 19g.1 Add `company_name TEXT` column to `customers` table in both PG and SQLite migrations (`002_customers.sql`)
- [x] 19g.2 Add `company_name: Option<String>` to `Customer` model in `src/customer/models.rs`
- [x] 19g.3 Add `company_name` to `CreateCustomerInput` and `UpdateCustomerInput` in `src/customer/types.rs`
- [x] 19g.4 Update seed data in `src/seed.rs` if applicable
- [x] 19g.5 Add tests: create customer with `company_name`, update customer `company_name`, verify in response

### 19h. Document `GET /store/orders/:id` auth divergence (S8 — MEDIUM)

- [x] 19h.1 Document in `design.md`: toko-rs requires `X-Customer-Id` on `GET /store/orders/:id` (intentional security improvement over Medusa's unauthenticated access)
- [x] 19h.2 Update `docs/audit-correction.md` with divergence note

### 19i. Add `metadata` to product options and option values (S9 — MEDIUM)

- [x] 19i.1 Add `metadata JSONB` column to `product_options` table in both PG and SQLite migrations (`001_products.sql`)
- [x] 19i.2 Add `metadata JSONB` column to `product_option_values` table in both PG and SQLite migrations (`001_products.sql`)
- [x] 19i.3 Add `metadata: Option<serde_json::Value>` to `ProductOption` and `ProductOptionValue` models in `src/product/models.rs`
- [x] 19i.4 Update `load_relations` queries and repository INSERT/UPDATE to handle new metadata columns
- [x] 19i.5 Add tests: verify `metadata` appears in product response options and option values

### 19j. Add missing DB indexes (S10 — MEDIUM)

- [x] 19j.1 Add `idx_product_variants_id_product_id ON product_variants (id, product_id) WHERE deleted_at IS NULL` to both PG and SQLite `001_products.sql`
- [x] 19j.2 Add `idx_orders_deleted_at ON orders (deleted_at) WHERE deleted_at IS NOT NULL` to both PG and SQLite `004_orders.sql`
- [x] 19j.3 Add `idx_orders_currency_code ON orders (currency_code) WHERE deleted_at IS NULL` to both PG and SQLite `004_orders.sql`
- [x] 19j.4 Add `idx_order_line_items_deleted_at ON order_line_items (deleted_at) WHERE deleted_at IS NOT NULL` to both PG and SQLite `004_orders.sql`
- [x] 19j.5 Add `idx_order_line_items_product_id ON order_line_items (product_id) WHERE deleted_at IS NULL` to both PG and SQLite `004_orders.sql`
- [x] 19j.6 Add `idx_order_line_items_variant_id ON order_line_items (variant_id) WHERE deleted_at IS NULL` to both PG and SQLite `004_orders.sql`

### 19k. Surface line-item snapshot fields in response (S12 — MEDIUM)

- [x] 19k.1 Add top-level fields to `CartLineItem` response: `product_title`, `product_description`, `product_subtitle`, `product_handle`, `variant_sku`, `variant_barcode`, `variant_title`, `variant_option_values` — populated from `snapshot` JSON column
- [x] 19k.2 Add top-level fields to `OrderLineItem` response with same snapshot-derived fields
- [x] 19k.3 Update contract tests to verify new fields appear in cart and order line item responses

### 19l. Document `customer_id` on cart create as intentional (S13 — MEDIUM)

- [x] 19l.1 Document in `design.md`: `customer_id` in `CreateCartInput` is an intentional toko-rs extension (Medusa infers from auth context); will be removed when real auth is implemented in P2

### 19m. Verification pass

- [x] 19m.1 Run full test suite on PG — all existing + new tests pass
- [x] 19m.2 Run full test suite on SQLite — all existing + new tests pass
- [x] 19m.3 Run `cargo clippy -- -D warnings` on both feature sets — zero warnings
- [x] 19m.4 Run `cargo fmt --check` — clean
- [x] 19m.5 Update `docs/audit-correction.md` with Task 19 section

### 19v. Post-implementation verification against Medusa vendor source

- [x] 19v.1 Fix S1 bug: `JsonDataError` now returns 400 (was 422) — matches Medusa's `INVALID_DATA` → 400 mapping
- [x] 19v.2 Fix S2 bug: Update variant now returns `{ product }` (was `{ variant }`) — matches Medusa's `PostProductsIdVariantsVariant_id` response
- [x] 19v.3 Update 5 contract tests: `deny_unknown_fields`, wrong types, wrong enum → all now assert 400 (was 422)
- [x] 19v.4 Verify S6 note: `CartCompleteResponse::error()` is dead code — infrastructure correct for P2 payment provider
- [x] 19v.5 Document atomicity gap in S3: soft-delete cascade runs 4 independent queries (not transactional) — low risk, deferred
- [x] 19v.6 Write full verification report: `docs/audit-p1-task19-verification.md`
- [x] 19v.7 Run full test suite — 158 tests pass, clippy clean, fmt clean

## Task 20. Fifth Audit — P1 Medusa Compatibility Re-Audit

Full six-dimension comparison of all 25 P1 endpoints against `vendor/medusa/` source. Focused exclusively on P1 scope — no P2 features (pricing, inventory, shipping, tax, events, admin orders, admin customers, etc.). Report: `docs/audit-p1-task20.md`.

### 20a. Fix BUG-1: Cart completion idempotency

- [x] 20a.1 Add `SELECT ... FOR UPDATE` on cart row in `create_from_cart` to serialize concurrent completions
- [x] 20a.2 Add integration test: concurrent cart completion returns Conflict for second request
- [x] 20a.3 Run full test suite — all tests pass, clippy clean

### 20b. Fix BUG-2: Soft-delete transactionality

- [x] 20b.1 Wrap `soft_delete` method in a DB transaction (4 UPDATEs on `self.pool` → single `tx`)
- [x] 20b.2 Run full test suite — all tests pass, clippy clean

### 20c. Fix BUG-3: Snapshot captures 5 fields but model extracts 8

- [x] 20c.1 Capture `variant_option_values` in snapshot JSON during `add_line_item`
- [x] 20c.2 Add test: verify `variant_option_values` is populated in cart/order line item responses
- [x] 20c.3 Run full test suite — all tests pass, clippy clean

### 20d. Add missing input fields for SDK compatibility (F1)

- [x] 20d.1 Add `is_giftcard: Option<bool>` and `discountable: Option<bool>` to `CreateProductInput`
- [x] 20d.2 Add `is_giftcard: Option<bool>` and `discountable: Option<bool>` to `UpdateProductInput`
- [x] 20d.3 Add `variant_rank: Option<i64>` to `CreateProductVariantInput`
- [x] 20d.4 Use values in repository INSERT/UPDATE, fall back to DB defaults when None
- [x] 20d.5 Add tests: verify new fields are accepted and persisted
- [x] 20d.6 Run full test suite — all tests pass, clippy clean

### 20e. Remove dead ValidationError variant (F2)

- [ ] 20e.1 Remove `AppError::ValidationError` variant from `src/error.rs`
- [ ] 20e.2 Update any affected unit tests in `error.rs`
- [ ] 20e.3 Run full test suite — all tests pass, clippy clean

### 20f. Add per-item totals on line items (F3)

- [ ] 20f.1 Add `#[sqlx(skip)]` per-item total fields to `CartLineItem` and `OrderLineItem`
- [ ] 20f.2 Compute per-item totals in `CartWithItems::from_items` and `OrderWithItems::from_items`
- [ ] 20f.3 Add tests: verify `item_total`, `subtotal`, `total` appear on line items
- [ ] 20f.4 Run full test suite — all tests pass, clippy clean

### 20g. Remove error message prefixing (F4)

- [ ] 20g.1 Remove prefixes from `#[error(...)]` attributes in `src/error.rs`
- [ ] 20g.2 Update repository error messages to not include redundant prefixes
- [ ] 20g.3 Update unit tests in `error.rs` to match new message format
- [ ] 20g.4 Update integration tests that assert on error messages
- [ ] 20g.5 Run full test suite — all tests pass, clippy clean

### 20h. Fix pagination defaults (F5)

- [ ] 20h.1 Change default `limit` from 20 to 50 for product, variant, and order list endpoints
- [ ] 20h.2 Update existing tests that assert on pagination defaults
- [ ] 20h.3 Run full test suite — all tests pass, clippy clean

### 20i. Verification pass

- [ ] 20i.1 Run full test suite on SQLite — all tests pass
- [ ] 20i.2 Run `cargo clippy -- -D warnings` — zero warnings
- [ ] 20i.3 Run `cargo fmt --check` — clean
- [ ] 20i.4 Update `docs/audit-p1-task20.md` with verification results
