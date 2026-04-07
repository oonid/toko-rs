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

**Phase 0 known gaps** (to be addressed in Phase 2b):
- Migrations are SQLite DDL (DATETIME, JSON). PostgreSQL-primary DDL (timestamptz, jsonb) rewrite is Phase 2b.7
- No CORS middleware yet. Phase 2b or Phase 1-E
- No graceful shutdown yet. Phase 2b or Phase 1-E
- Health check doesn't test DB connectivity (always returns "connected")
- FindParams has no default values (offset=0, limit=20)
- Makefile missing docker-up, docker-down, test-pg targets
- 21 compiler warnings (cfg(coverage) guards, unused imports)

## 2. Phase 1-A — Product Module (PARTIAL)

- [x] 2.1 Define models: Product, ProductOption, ProductOptionValue, ProductVariant, ProductWithRelations, ProductOptionWithValues, ProductVariantWithOptions, VariantOptionValue
- [x] 2.2 Define request/response types: AdminCreateProductRequest, AdminUpdateProductRequest, ProductResponse, ProductListResponse, DeleteResponse
- [x] 2.3 Implement repository: create (transactional with options/variants) — **only create_product implemented**
- [x] 2.4 Implement routes: all 8 routes registered in router
- [x] 2.5 Wire product routes into Axum router with AppState
- [x] 2.6 Write integration tests for product create (2 tests: success + validation)
- [x] 2.7 Verify route stubs respond (1 test covering all 7 stub endpoints)
- [ ] 2.8 Implement `admin_list_products` route handler (currently 501 stub)
- [ ] 2.9 Implement `admin_get_product` route handler (currently 501 stub) — requires repo `find_by_id` with options/variants join
- [ ] 2.10 Implement `admin_update_product` route handler (currently 501 stub) — requires repo `update` method
- [ ] 2.11 Implement `admin_delete_product` route handler (currently 501 stub) — requires repo `soft_delete` method, returns `DeleteResponse`
- [ ] 2.12 Implement `admin_add_variant` route handler (currently 501 stub) — requires repo `add_variant` method
- [ ] 2.13 Implement `store_list_products` route handler (currently 501 stub) — filters `status = 'published' AND deleted_at IS NULL`
- [ ] 2.14 Implement `store_get_product` route handler (currently 501 stub) — same as admin get but 404 for draft/deleted
- [ ] 2.15 Fix handle generation: use `types::generate_handle()` (slug crate) instead of inline `replace(" ", "-")`
- [ ] 2.16 Use `types::generate_entity_id()` instead of inline `format!("{}_{}", prefix, ...)` for all ID generation
- [ ] 2.17 Fix ULID casing: spec says `[0-9A-Z]` but code lowercases. Align spec to match Medusa (lowercase) or keep uppercase.
- [ ] 2.18 Add duplicate handle detection: catch SQLite UNIQUE violation and return `AppError::DuplicateError` instead of raw `DatabaseError`
- [ ] 2.19 Implement `find_by_id` in repository — query product + options + option_values + variants + variant_options in a single fetch or join
- [ ] 2.20 Implement `list` in repository — paginated with `offset`, `limit`, `order`, `with_deleted` support
- [ ] 2.21 Implement `list_published` in repository — same as list but `WHERE status = 'published' AND deleted_at IS NULL`
- [ ] 2.22 Implement `update` in repository — COALESCE pattern for partial updates
- [ ] 2.23 Implement `soft_delete` in repository — set `deleted_at = CURRENT_TIMESTAMP`
- [ ] 2.24 Implement `add_variant` in repository — insert variant with option binding resolution

## 2b. Database Refactor — PostgreSQL-Primary (blocks all subsequent phases)

- [ ] 2b.1 Remove dual `SqliteProductRepository` / `PostgresProductRepository` pattern — consolidate to single `ProductRepository` using `PgPool` with `$1, $2` placeholders
- [ ] 2b.2 Remove dual `SqliteCartRepository` / `PostgresCartRepository` pattern — consolidate to single `CartRepository`
- [ ] 2b.3 Remove `DatabaseRepo` enum dispatch in `db.rs` — replace with simple struct holding individual repo instances
- [ ] 2b.4 Remove all `#[cfg(not(coverage))]` / `#[cfg(coverage)]` guards from repositories
- [ ] 2b.5 Add SQLite placeholder adapter: translate `$1, $2, $3` → `?` for in-memory test path (single function, applied at query execution)
- [ ] 2b.6 Create `docker-compose.yml` with PostgreSQL 16 service for integration testing
- [ ] 2b.7 Rewrite migrations to PostgreSQL-primary DDL (`timestamptz`, `jsonb`, `BOOLEAN`, quoted identifiers) aligned with Medusa's `vendor/medusa/packages/modules/*/src/migrations/`
- [ ] 2b.8 Maintain SQLite-compatible migration files for in-memory test path (separate directory or adapter)
- [ ] 2b.9 Update `AppState` to hold `PgPool` + individual repo structs (no enum dispatch)
- [ ] 2b.10 Update test infrastructure (`tests/common/mod.rs`) to use SQLite in-memory with placeholder adapter
- [ ] 2b.11 Fix variant-to-option pivot: persist variant option bindings to `product_variant_options` table during create_product and add_variant
- [ ] 2b.12 Fix error response: add `code` field to match 3-field OAS Error schema (`code`, `type`, `message`)
- [ ] 2b.13 Verify all existing tests still pass after refactor

## 3. Phase 1-D — Customer Module

- [ ] 3.1 Define customer models: Customer struct with id, first_name, last_name, email, phone, has_account, metadata, timestamps
- [ ] 3.2 Define customer request/response types: StoreCreateCustomerRequest, StoreUpdateCustomerRequest, CustomerResponse
- [ ] 3.3 Implement customer repository (single repo, PgPool): create, find_by_id, find_by_email, update with duplicate email check
- [ ] 3.4 Implement customer routes: POST /store/customers, GET /store/customers/me, POST /store/customers/me
- [ ] 3.5 Implement X-Customer-Id header extraction middleware for /me endpoints
- [ ] 3.6 Wire customer repository into AppState
- [ ] 3.7 Wire customer routes into main router
- [ ] 3.8 Write integration tests: register, duplicate email, get profile, update profile, missing header

## 4. Phase 1-B — Cart Module

- [ ] 4.1 Define cart models: Cart, CartLineItem, CartWithItems, LineItemSnapshot
- [ ] 4.2 Define cart request/response types: StoreCreateCartRequest, StoreUpdateCartRequest, StoreAddLineItemRequest, StoreUpdateLineItemRequest, CartResponse
- [ ] 4.3 Implement cart repository (single repo, PgPool): create, find_by_id (with items + computed totals), update, mark_completed
- [ ] 4.4 Implement line item repository: add_line_item (with variant lookup + snapshot), update_line_item (soft delete at qty 0), remove_line_item
- [ ] 4.5 Implement cart validation: check not completed before mutations
- [ ] 4.6 Implement cart routes: POST /store/carts, GET /store/carts/:id, POST /store/carts/:id
- [ ] 4.7 Implement line item routes: POST /store/carts/:id/line-items, POST /store/carts/:id/line-items/:line_id, DELETE /store/carts/:id/line-items/:line_id
- [ ] 4.8 Wire cart repository into AppState
- [ ] 4.9 Wire cart routes into main router
- [ ] 4.10 Write integration tests: create cart, add item, update quantity, remove item, invalid variant, completed cart mutation, quantity validation

## 5. Phase 1-C — Order Module

- [ ] 5.1 Define payment model: PaymentRecord struct with id, order_id, amount, currency_code, status, provider, metadata, timestamps
- [ ] 5.2 Implement payment repository (single repo, PgPool): create, find_by_order_id
- [ ] 5.3 Define order models: Order, OrderLineItem, OrderWithItems
- [ ] 5.4 Define order response types: OrderResponse, OrderListResponse, CartCompleteResponse
- [ ] 5.5 Implement order repository: create_from_cart (atomic transaction with display_id auto-increment, item copy, payment creation, cart completion), find_by_id, list_by_customer
- [ ] 5.6 Implement order routes: POST /store/carts/:id/complete, GET /store/orders, GET /store/orders/:id
- [ ] 5.7 Wire order and payment repositories into AppState
- [ ] 5.8 Wire order routes into main router
- [ ] 5.9 Write integration tests: full flow (cart → order), empty cart completion, completed cart re-completion, display_id increment, payment record verification, order list by customer

## 6. Phase 1-E — Integration Wiring

- [ ] 6.1 Mount all module routes in main router: /admin/products/*, /store/products/*, /store/carts/*, /store/orders/*, /store/customers/*
- [ ] 6.2 Apply middleware stack: TraceLayer + CorsLayer
- [ ] 6.3 Wire AppState with all repository handles
- [ ] 6.4 Implement health check with database connectivity test
- [ ] 6.5 Verify all 20 endpoints respond correctly

## 7. Phase 1-F — Seed Data

- [ ] 7.1 Implement seed function with 3-5 sample products (all published, with options and variants)
- [ ] 7.2 Add 1 sample customer to seed
- [ ] 7.3 Make seed idempotent (check existence before inserting)
- [ ] 7.4 Wire --seed CLI flag to seed function
- [ ] 7.5 Smoke test full Browse → Cart → Checkout flow via curl

## 8. Phase 1-G — Test Suite

- [ ] 8.1 Create test infrastructure: setup_test_db (in-memory SQLite + migrations), create_test_app, helper functions
- [ ] 8.2 Write product tests: admin CRUD, store filtering, contract validation
- [ ] 8.3 Write cart tests: create, add/update/remove items, completed cart guard, quantity validation
- [ ] 8.4 Write order tests: full flow, empty/completed cart errors, display_id, payment record, customer filtering
- [ ] 8.5 Write customer tests: register, duplicate email, profile CRUD, auth header
- [ ] 8.6 Write contract tests: verify all response JSON shapes match API contract using assert-json-diff
- [ ] 8.7 Write error contract tests: verify all error responses include `code`, `type`, `message` fields matching specs/store.oas.yaml Error schema
- [ ] 8.8 Verify `cargo test` passes all tests with 100% endpoint coverage

## 9. Phase 1-H — Polish

- [ ] 9.1 Run `cargo clippy -- -D warnings` — zero warnings
- [ ] 9.2 Run `cargo fmt` — consistent formatting
- [ ] 9.3 Verify all `#[tracing::instrument]` annotations on handlers
- [ ] 9.4 Verify all 20 endpoints return correct Medusa-compatible JSON shapes
