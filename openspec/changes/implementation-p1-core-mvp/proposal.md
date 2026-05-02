## Why

toko-rs is a Rust single-binary headless e-commerce backend inspired by MedusaJS. The P1 Core MVP delivers the essential Browse → Cart → Checkout flow with Medusa-compatible API contracts, enabling frontend clients built for Medusa to work against toko-rs with zero modification. This change implements the entire P1 scope — from project scaffold through a fully tested 20-endpoint backend.

**Architecture inspiration**: MedusaJS uses a micro-kernel architecture with three layers — kernel (`packages/core/`), domain modules (`packages/modules/`), and composition root (`packages/medusa/`). Modules are isolated service packages that own models, services, repositories, and migrations but contain NO route handlers. toko-rs adapts this pattern to a single Rust crate with per-module folders and enforced module boundary rules. See `design.md` Decisions 8–10 for the P1 simplifications.

**Development methodology**: All implementation follows TDD (Test-Driven Development). Spec scenarios (WHEN/THEN format) serve as test contracts that are written before implementation. Coverage target: >90% line coverage (`cargo llvm-cov`).

## What Changes

- **Project scaffold**: Single Rust crate with axum + sqlx + SQLite, tracing, migrations, config
- **12-table database schema**: products, product_options, product_option_values, product_variants, product_variant_options (pivot), product_images, customers, customer_addresses, carts, cart_line_items, orders, order_line_items, payment_records, _sequences. This is a simplification of Medusa's 40+ table schema — see `docs/database.md` for the full Medusa-to-toko-rs table mapping (which tables are implemented, collapsed into columns, or deferred to P2+). Invoice config stored as env vars (not a table). `idempotency_keys` table removed (dead code).
- **20 Admin API endpoints**: Product CRUD + variant/option management (12), customer list+get (2), cart list (1), order cancel+complete+fulfill+ship+capture-payment (5), invoice config get/update (2), order invoice view (1)
- **14 Store API endpoints**: Product browsing, cart management, cart-to-order completion, order viewing, customer registration/profile
- **Medusa-compatible error format**: `{"code": "...", "type": "...", "message": "..."}` per the Error schema in `specs/store.oas.yaml` and `specs/admin.oas.yaml` (copied from `vendor/medusa/`)
- **Medusa-compatible response patterns**: Root wrapper (`{"product": {...}}`), list pagination (`{"products": [...], "count", "offset", "limit"}`)
- **HTTP method convention**: POST for both create and update (no PUT), matching Medusa's convention
- **Variant-to-option wiring persisted**: Variant option bindings written to `product_variant_options` pivot table during product/variant creation
- **Module boundary rules**: Modules must not import other domain modules; only shared infrastructure (`types.rs`, `error.rs`, `db.rs`) may be imported across module boundaries
- **Integration test suite**: Full endpoint coverage with contract validation against Medusa response shapes
- **Contract testing methodology**: Spec scenarios map 1:1 to test cases; response JSON verified against Medusa reference
- **Seed data**: Idempotent sample data for development
- **Medusa vendor reference**: `vendor/medusa/` submodule as implementation authority; `specs/` holds copied OpenAPI base schemas from `vendor/medusa/www/utils/generated/oas-output/base/`

## Capabilities

### New Capabilities

- `product-module`: Product, variant, option, and option-value CRUD for admin and store APIs (8 endpoints)
- `cart-module`: Cart creation, line item management, cart completion flow (7 store endpoints), and admin cart list (1 admin endpoint)
- `order-module`: Order generation from cart completion, order listing and detail retrieval (3 store endpoints), and admin order cancel/complete/fulfill/ship/capture-payment (5 admin endpoints). Order fulfillment_status persisted as column (not_fulfilled → fulfilled → shipped → canceled). Payment capture updates payment_records.status. Invoice enriched with payment info.
- `customer-module`: Customer registration and profile management (3 store endpoints), and admin customer list+get (2 admin endpoints)
- `invoice-module`: Invoice issuer config from env vars (2 admin endpoints) and on-the-fly invoice generation from order data (1 admin endpoint) — text-based, no PDF (P1)
- `database-schema`: 14-table schema (products, product_options, product_option_values, product_variants, product_variant_options, product_images, customers, customer_addresses, carts, cart_line_items, orders, order_line_items, payment_records, _sequences) with soft delete, prefixed ULID IDs, and JSON metadata fields. Invoice config via env vars. `idempotency_keys` removed.
- `error-handling`: Medusa-compatible error types mapped to HTTP status codes
- `foundation`: Config, DB pool, migrations, tracing, health check, seed data, Makefile
- `testing`: Contract testing methodology with TDD workflow, >90% coverage target, spec-to-test traceability

### Modified Capabilities

(None — this is the initial implementation)

## Impact

- **New binary**: Single `toko-rs` executable serving both admin and store APIs on port 3000
- **Database**: SQLite file `toko.db` in working directory (development), PostgreSQL-ready via sqlx AnyPool
- **Dependencies**: 15 runtime crates (axum, sqlx, tokio, serde, validator, ulid, slug, dotenvy, thiserror, chrono, tracing, tower-http, tower, serde_json) + 4 dev crates
- **No breaking changes**: Greenfield implementation
- **Reference sources**: `vendor/medusa/` (git submodule, `develop` branch) — MedusaJS source as implementation reference. `specs/store.oas.yaml` and `specs/admin.oas.yaml` — OpenAPI 3.0 base schemas copied from `vendor/medusa/www/utils/generated/oas-output/base/`. Per-endpoint operation specs in `vendor/medusa/www/utils/generated/oas-output/operations/`. Medusa model definitions in `vendor/medusa/packages/modules/*/src/models/`
