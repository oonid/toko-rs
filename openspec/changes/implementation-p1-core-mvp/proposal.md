## Why

toko-rs is a Rust single-binary headless e-commerce backend inspired by MedusaJS. The P1 Core MVP delivers the essential Browse → Cart → Checkout flow with Medusa-compatible API contracts, enabling frontend clients built for Medusa to work against toko-rs with zero modification. This change implements the entire P1 scope — from project scaffold through a fully tested 20-endpoint backend.

## What Changes

- **Project scaffold**: Single Rust crate with axum + sqlx + SQLite, tracing, migrations, config
- **11-table database schema**: products, product_options, product_option_values, product_variants, product_variant_options (pivot), carts, cart_line_items, orders, order_line_items, customers, customer_addresses, payment_records
- **6 Admin API endpoints**: Product CRUD + variant management
- **14 Store API endpoints**: Product browsing, cart management, cart-to-order completion, order viewing, customer registration/profile
- **Medusa-compatible error format**: `{"code": "...", "type": "...", "message": "..."}` per the Error schema in `specs/store.oas.yaml` and `specs/admin.oas.yaml` (copied from `vendor/medusa/`)
- **Medusa-compatible response patterns**: Root wrapper (`{"product": {...}}`), list pagination (`{"products": [...], "count", "offset", "limit"}`)
- **Variant-to-option wiring persisted**: Variant option bindings written to `product_variant_options` pivot table during product/variant creation
- **Integration test suite**: Full endpoint coverage with contract validation
- **Seed data**: Idempotent sample data for development
- **Medusa vendor reference**: `vendor/medusa/` submodule as implementation authority; `specs/` holds copied OpenAPI base schemas from `vendor/medusa/www/utils/generated/oas-output/base/`

## Capabilities

### New Capabilities

- `product-module`: Product, variant, option, and option-value CRUD for admin and store APIs (8 endpoints)
- `cart-module`: Cart creation, line item management, and cart completion flow (7 endpoints including complete)
- `order-module`: Order generation from cart completion, order listing and detail retrieval (3 endpoints)
- `customer-module`: Customer registration and profile management (3 endpoints)
- `database-schema`: 11-table + 1-pivot SQLite schema with soft delete, prefixed ULID IDs, and JSON metadata fields
- `error-handling`: Medusa-compatible error types mapped to HTTP status codes
- `foundation`: Config, DB pool, migrations, tracing, health check, seed data, Makefile

### Modified Capabilities

(None — this is the initial implementation)

## Impact

- **New binary**: Single `toko-rs` executable serving both admin and store APIs on port 3000
- **Database**: SQLite file `toko.db` in working directory (development), PostgreSQL-ready via sqlx AnyPool
- **Dependencies**: 15 runtime crates (axum, sqlx, tokio, serde, validator, ulid, slug, dotenvy, thiserror, chrono, tracing, tower-http, tower, serde_json) + 4 dev crates
- **No breaking changes**: Greenfield implementation
- **Reference sources**: `vendor/medusa/` (git submodule, `develop` branch) — MedusaJS source as implementation reference. `specs/store.oas.yaml` and `specs/admin.oas.yaml` — OpenAPI 3.0 base schemas copied from `vendor/medusa/www/utils/generated/oas-output/base/`. Per-endpoint operation specs in `vendor/medusa/www/utils/generated/oas-output/operations/`. Medusa model definitions in `vendor/medusa/packages/modules/*/src/models/`
