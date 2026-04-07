## Context

toko-rs is a Rust single-binary headless e-commerce backend inspired by MedusaJS (TypeScript/Node.js). It targets API compatibility with Medusa's store and admin endpoints while being a standalone Rust application. The P1 Core MVP covers the essential Browse → Cart → Checkout flow.

**Implementation reference**: `vendor/medusa/` is a git submodule tracking the Medusa `develop` branch. It serves as the authoritative source for:
- **API contracts**: OpenAPI 3.0 base schemas in `vendor/medusa/www/utils/generated/oas-output/base/` (copied to `specs/`). Per-endpoint operation specs (JSDoc) in `vendor/medusa/www/utils/generated/oas-output/operations/`.
- **Model definitions**: TypeScript models in `vendor/medusa/packages/modules/*/src/models/` — used to verify field coverage and relationship structure.
- **Validation schemas**: Zod validators in `vendor/medusa/packages/medusa/src/api/*/validators.ts` — used to derive Rust request validation.
- **Route handlers**: Implementation patterns in `vendor/medusa/packages/medusa/src/api/*/route.ts` — used to understand response wrapping and error handling.

**Current state**: Phase 0 (scaffold) complete. Phase 1-A (product module) is **partially implemented**: models, types, and the `create_product` repository method work; all 8 routes are registered but 7 of 8 handlers are 501 stubs. The `generate_entity_id()` and `generate_handle()` utilities exist in `types.rs` but are not yet used by the product repository. Variant-to-option pivot writes are not yet implemented. The cart module has stub files with basic create/line-item logic working. Order, customer, and payment modules have only `mod.rs` stubs.

**Known P1 divergences from Medusa** (by design, not bugs):

| Area | Medusa | toko-rs P1 | Rationale |
|---|---|---|---|
| Variant pricing | Separate Pricing module: `AdminCreateVariantPrice[]` with currency_code, amount, min/max qty, rules | Single `price: i64` column on product_variants | P1 is single-currency; Pricing module is P2 |
| Cart/Order addresses | Separate `Address` table with FK relationship | Inline JSON column on cart/order row | Dormant in P1; activates as JSON in P2 |
| Line item snapshot | 12 denormalized columns (product_title, variant_sku, etc.) directly on line item | Single `snapshot` JSON column | Structural simplification; same data captured |
| Order versioning | OrderLineItem (static snapshot) + OrderItem (mutable fulfillment tracking per version) | Single `order_line_items` table | P1 has no order edits/claims/exchanges |
| Error `code` field | 3-field error: `code` + `type` + `message` | Must match (3 fields) | Required for OAS compliance — see error-handling spec |
| List filtering | 15+ filter params with operator maps ($eq, $in, $like, etc.) and $and/$or | 5 basic params (offset, limit, order, fields, with_deleted) | P1 MVP; expand in P2 |
| Auth | JWT tokens + session cookies | X-Customer-Id header stub | P1 simplification |

**Constraints**:
- Single binary, no microservices
- **PostgreSQL-primary**: All SQL written for PostgreSQL (`$1, $2` placeholders, `timestamptz`, `jsonb`). SQLite used for local development via a thin placeholder adapter. Medusa uses PostgreSQL — toko-rs targets the same.
- Medusa-compatible JSON response shapes (not HTTP status codes alone)
- Error responses MUST match the 3-field schema (`code`, `type`, `message`) from `specs/store.oas.yaml`
- Rust edition 2021, MSRV 1.85
- No authentication beyond `X-Customer-Id` header stub (P2 concern)
- Docker Compose for integration testing against PostgreSQL

## Goals / Non-Goals

**Goals:**
- Complete all 20 P1 endpoints with Medusa-compatible API contracts
- Implement cart line item management with product snapshot
- Implement cart-to-order atomic transaction
- Customer registration and profile management
- Full integration test suite
- Idempotent seed data

**Non-Goals:**
- Authentication/JWT (P2)
- Shipping methods and calculation (P2, needs new tables)
- Payment provider integration (P2, manual provider only in P1)
- Tax calculation (P3)
- Image/file upload
- Admin authentication
- WebSocket/real-time updates
- Multi-currency pricing (P1 uses single price field)

## Decisions

### 1. Single crate with module folders
Each domain (product, cart, order, customer, payment) is a folder under `src/` with consistent structure: `mod.rs`, `models.rs`, `repository.rs`, `routes.rs`, `types.rs`. This mirrors Medusa's module organization without Cargo workspace overhead.

**Alternative considered**: Cargo workspace with separate crates. Rejected because 11 tables and 20 endpoints don't justify the compilation complexity. Module boundaries are sufficient, and if needed later, each folder can become its own crate with minimal structural change.

### 2. Single repository per module (no dual SQLite/Postgres repos)
Each module has exactly ONE repository struct using `PgPool`. No `SqlitePool`, no enum dispatch, no `#[cfg]` guards. PostgreSQL is the primary and only target for production queries.

**For local development and testing**, two options are supported:
- **SQLite in-memory** (`sqlite::memory:`): Used by integration tests. A thin helper translates `$1, $2, $3` placeholders to `?` at query preparation time. This avoids duplicating every query.
- **PostgreSQL via Docker**: Used for full compatibility testing. `docker-compose.yml` provides a PostgreSQL 16 container. `DATABASE_URL=postgres://...` runs against it directly.

**Previous approach (rejected)**: Enum dispatch with `DatabaseRepo { Sqlite { product, cart }, Postgres { product, cart } }` where every repo method was duplicated — once with `?` placeholders, once with `$N` placeholders. This doubled maintenance cost per module and left Postgres implementations as untested stubs. With 5 modules planned, this would produce ~60 duplicated method bodies and ~30 `#[cfg]` guards.

### 3. PostgreSQL-native DDL with Medusa alignment
Migrations are written for PostgreSQL first (matching Medusa's DDL: `timestamptz`, `jsonb`, `BOOLEAN`, quoted identifiers). SQLite-compatible alternatives are provided in a separate migration directory for the in-memory test adapter.

**Key DDL differences documented**:

| Feature | PostgreSQL (primary) | SQLite (dev/test) |
|---|---|---|
| Timestamps | `timestamptz DEFAULT now()` | `DATETIME DEFAULT CURRENT_TIMESTAMP` |
| JSON | `jsonb` | `TEXT` (stores JSON string) |
| Boolean | `BOOLEAN` | `INTEGER` (0/1) |
| Placeholders | `$1, $2, $3` | `?` (translated by adapter) |
| RETURNING | `RETURNING *` | `RETURNING *` (SQLite 3.35+) |
| Partial indexes | `WHERE deleted_at IS NULL` | `WHERE deleted_at IS NULL` |
| Cascading FK | `ON DELETE CASCADE` | `ON DELETE CASCADE` (with `PRAGMA foreign_keys`) |

**Medusa reference**: `vendor/medusa/packages/modules/*/src/migrations/` contains the authoritative PostgreSQL DDL. toko-rs migrations follow the same naming conventions (`product`, `product_variant`, `product_option`, `product_option_value`) and index patterns.

### 4. Application-managed timestamps and IDs
- **IDs**: Prefixed ULID via `ulid` crate (`prod_01JX...`, `cart_01JX...`)
- **Timestamps**: SQL `DEFAULT CURRENT_TIMESTAMP` on insert, explicit `updated_at = CURRENT_TIMESTAMP` on update
- **Soft delete**: `deleted_at` column set to timestamp, filtered by `WHERE deleted_at IS NULL`

### 5. JSON metadata and snapshots
Metadata fields (`metadata`) stored as `TEXT` containing JSON. Cart line items include a `snapshot` JSON field capturing product_title, variant_title, variant_sku at add-time. This frozen snapshot preserves data integrity even if the product is later modified.

**Medusa comparison**: Medusa denormalizes 12 product/variant fields directly onto `LineItem` (product_title, product_description, variant_sku, variant_title, variant_option_values, etc.). toko-rs collapses these into a single `snapshot` JSON column containing the same data. This is structurally different but functionally equivalent.

### 6. Cart completion as atomic transaction
Cart-to-order conversion is a single SQL transaction that:
1. Validates cart state (exists, not completed, has items)
2. Generates order with `display_id = MAX(display_id) + 1`
3. Copies cart line items to order line items
4. Creates payment record (status: pending, provider: manual)
5. Marks cart as completed

**Medusa comparison**: Medusa uses a complex workflow engine with multiple steps (create order, update inventory, create payment collection, etc.). toko-rs collapses this into a single transaction. Medusa's `OrderItem` (fulfillment tracking) is not replicated — P1 uses only `order_line_items` (static snapshot).

### 7. P1 auth stub: X-Customer-Id header
Customer-scoped endpoints (`/store/customers/me`, `/store/orders`) use `X-Customer-Id` header for identification. No JWT, no password hashing. This is a deliberate P1 simplification replaced by full auth in P2.

## Risks / Trade-offs

- **Placeholder adapter for SQLite tests**: Translating `$N` → `?` at runtime adds a thin layer of indirection for the in-memory SQLite test path. Mitigation: the adapter is a single function, and integration tests against PostgreSQL (via Docker) serve as the authoritative validation.
- **Single price field**: Medusa uses a pricing module with multi-currency price sets. P1 collapses this to a single `price` integer (cents) on `product_variants`. Breaking change if multi-currency is needed later. Mitigation: price is an integer field that can be migrated to a foreign key.
- **No admin auth**: Admin endpoints are fully open. Acceptable for development/demo. Must add auth before production.
- **Cart line item snapshot not updated on product change**: By design — snapshots freeze state at add-time. If product price changes, existing cart items keep old price. This matches Medusa behavior.
- **Docker dependency for full PG testing**: Integration tests can run against SQLite in-memory without Docker, but full PostgreSQL compatibility requires `docker compose up`. Mitigation: CI pipeline runs both; local dev can use either.
