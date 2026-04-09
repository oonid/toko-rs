# Database Foundation

## Medusa-to-toko-rs Schema Mapping

This section maps every Medusa table (`vendor/medusa/packages/modules/*/src/models/`) to its P1 equivalent in toko-rs. Tables are classified as **implemented** (possibly simplified), **collapsed into a column** (Medusa's separate table becomes a JSON field), or **deferred to P2+**.

Referenced by `design.md` (P1 divergences table) and `proposal.md` (schema scope).

### Product Module

| Medusa Table | Medusa Columns (key) | toko-rs Status | toko-rs Table/Column | Simplification |
|---|---|---|---|---|
| `product` | title, handle, subtitle, description, is_giftcard, status, thumbnail, weight, length, height, width, origin_country, hs_code, mid_code, material, discountable, external_id, metadata + 8 relationships | **Implemented** | `products` | Dropped: subtitle, is_giftcard, weight/length/height/width, origin_country, hs_code, mid_code, material, discountable, external_id. Dropped relationships: type, tags, images, collection, categories |
| `product_variant` | title, sku, barcode, ean, upc, allow_backorder, manage_inventory, weight/length/height/width, hs_code, origin_country, mid_code, material, variant_rank, thumbnail, metadata + 3 relationships | **Implemented** | `product_variants` | Dropped: barcode, ean, upc, allow_backorder, manage_inventory, weight/length/height/width, hs_code, origin_country, mid_code, material, thumbnail. Single `price` column replaces Pricing module. Dropped relationships: images |
| `product_option` | title, metadata + product FK | **Implemented** | `product_options` | Exact match |
| `product_option_value` | value, metadata + option FK | **Implemented** | `product_option_values` | Exact match |
| `product_variant_option` (pivot) | variant_id, option_value_id | **Implemented** | `product_variant_option` (pivot) | Exact match |
| `product_image` | url, rank, metadata + product FK + variant M2M | **Deferred P2+** | — | No image support in P1 |
| `product_variant_product_image` (pivot) | variant_id, image_id | **Deferred P2+** | — | Depends on product_image |
| `product_tag` | value, metadata + product M2M | **Deferred P2+** | — | No tagging in P1 |
| `product_tags` (pivot) | product_id, product_tag_id | **Deferred P2+** | — | Depends on product_tag |
| `product_type` | value, metadata | **Deferred P2+** | — | No type classification in P1 |
| `product_collection` | title, handle, metadata | **Deferred P2+** | — | No collections in P1 |
| `product_category` | name, description, handle, mpath, is_active, is_internal, rank, metadata + parent self-ref + product M2M | **Deferred P2+** | — | No category tree in P1 |
| `product_category_product` (pivot) | product_id, product_category_id | **Deferred P2+** | — | Depends on product_category |

### Cart Module

| Medusa Table | Medusa Columns (key) | toko-rs Status | toko-rs Table/Column | Simplification |
|---|---|---|---|---|
| `cart` | region_id, customer_id, sales_channel_id, email, currency_code, locale, metadata, completed_at + shipping/billing address hasOne + items/shipping_methods hasMany | **Implemented** | `carts` | Dropped: region_id, sales_channel_id, locale. Addresses stored inline as JSON (see cart_address below). Dropped relationships: shipping_methods, credit_lines |
| `cart_address` | customer_id, company, first_name, last_name, address_1/2, city, country_code, province, postal_code, phone, metadata | **Collapsed** | `carts.shipping_address` JSON + `carts.billing_address` JSON | Medusa uses a separate table with FK; toko-rs stores address as inline JSONB column. Dormant in P1 (no address endpoints) |
| `cart_line_item` | title, subtitle, thumbnail, quantity, variant_id, product_id, product_title, product_description, product_subtitle, product_type, product_type_id, product_collection, product_handle, variant_sku, variant_barcode, variant_title, variant_option_values, requires_shipping, is_discountable, is_giftcard, is_tax_inclusive, is_custom_price, compare_at_unit_price, unit_price, metadata + adjustments/tax_lines hasMany | **Implemented** | `cart_line_items` | Dropped 12 denormalized columns (product_title, product_description, product_subtitle, product_type, product_type_id, product_collection, product_handle, variant_sku, variant_barcode, variant_title, variant_option_values) → collapsed into `snapshot` JSON column. Dropped: subtitle, thumbnail, requires_shipping, is_discountable, is_giftcard, is_tax_inclusive, is_custom_price, compare_at_unit_price. Dropped relationships: adjustments, tax_lines |
| `cart_line_item_adjustment` | description, code, amount, is_tax_inclusive, provider_id, promotion_id, metadata | **Deferred P2+** | — | No promotions in P1 |
| `cart_line_item_tax_line` | description, code, rate, provider_id, tax_rate_id, metadata | **Deferred P2+** | — | No tax calculation in P1 |
| `cart_shipping_method` | name, description, amount, is_tax_inclusive, shipping_option_id, data, metadata + tax_lines/adjustments hasMany | **Deferred P2+** | — | No shipping in P1 |
| `cart_shipping_method_adjustment` | description, code, amount, provider_id, promotion_id, metadata | **Deferred P2+** | — | Depends on shipping_method |
| `cart_shipping_method_tax_line` | description, code, rate, provider_id, tax_rate_id, metadata | **Deferred P2+** | — | Depends on shipping_method |
| `credit_line` (cart) | reference, reference_id, amount, raw_amount, metadata | **Deferred P2+** | — | No credit system in P1 |

### Order Module

| Medusa Table | Medusa Columns (key) | toko-rs Status | toko-rs Table/Column | Simplification |
|---|---|---|---|---|
| `order` | display_id, custom_display_id, region_id, customer_id, version, sales_channel_id, status, is_draft_order, email, currency_code, locale, no_notification, metadata, canceled_at + shipping/billing address hasOne + summary/items/shipping/transactions/returns hasMany | **Implemented** | `orders` | Dropped: custom_display_id, region_id, version, sales_channel_id, is_draft_order, locale, no_notification. Addresses stored inline as JSON (see order_address below). Dropped relationships: summary (computed), shipping_methods, transactions, credit_lines, returns |
| `order_address` | customer_id, company, first_name, last_name, address_1/2, city, country_code, province, postal_code, phone, metadata | **Collapsed** | `orders.shipping_address` JSON + `orders.billing_address` JSON | Same pattern as cart_address — inline JSONB instead of separate table |
| `order_line_item` | title, subtitle, thumbnail, variant_id, product_id, product_title/description/subtitle/type/type_id/collection/handle, variant_sku/barcode/title/option_values, requires_shipping, is_giftcard, is_discountable, is_tax_inclusive, compare_at_unit_price, unit_price, is_custom_price, metadata + tax_lines/adjustments hasMany | **Implemented** | `order_line_items` | Same simplification as cart_line_item: 12 denormalized columns → `snapshot` JSON. Dropped: subtitle, thumbnail, requires_shipping, is_giftcard, is_discountable, is_tax_inclusive, is_custom_price, compare_at_unit_price. Dropped relationships: tax_lines, adjustments |
| `order_item` | version, unit_price, compare_at_unit_price, quantity, fulfilled/delivered/shipped/return_requested/return_received/return_dismissed/written_off quantities, metadata + order FK + item hasOne | **Collapsed** | `order_line_items` (merged) | Medusa splits: OrderLineItem (static snapshot) + OrderItem (mutable fulfillment tracking per version). P1 merges into single table since there are no order edits/claims/exchanges |
| `order_summary` | version, totals JSON + order FK | **Collapsed** | Computed fields (`item_total`, `total`) | Not stored — calculated as `sum(quantity * unit_price)` at query time |
| `order_shipping_method` | name, description, amount, is_tax_inclusive, is_custom_amount, shipping_option_id, data, metadata + tax_lines/adjustments hasMany | **Deferred P2+** | — | No shipping in P1 |
| `order_shipping_method_adjustment` | description, promotion_id, code, amount, provider_id | **Deferred P2+** | — | Depends on shipping |
| `order_shipping_method_tax_line` | description, tax_rate_id, code, rate, provider_id | **Deferred P2+** | — | Depends on shipping |
| `order_shipping` | version + order/return/exchange/claim FKs + shipping_method hasOne | **Deferred P2+** | — | Join entity for order↔shipping_method |
| `order_line_item_adjustment` | version, description, promotion_id, code, amount, provider_id, is_tax_inclusive | **Deferred P2+** | — | No promotions in P1 |
| `order_line_item_tax_line` | description, tax_rate_id, code, rate, provider_id | **Deferred P2+** | — | No tax calculation in P1 |
| `order_change` | version, change_type, status, internal_note, created/requested/confirmed/declined/canceled by/at, carry_over_promotions, metadata + order FK + actions hasMany | **Deferred P2+** | — | No order edits in P1 |
| `order_change_action` | version, ordering, reference, reference_id, action, details JSON, amount, internal_note, applied + order_change FK | **Deferred P2+** | — | Depends on order_change |
| `return` | order_version, display_id, status, location_id, no_notification, refund_amount, created_by, metadata, requested/received/canceled at + order/exchange/claim FKs + items/shipping/transactions hasMany | **Deferred P2+** | — | No returns in P1 |
| `return_item` | quantity, received_quantity, damaged_quantity, note, metadata + reason/item FKs | **Deferred P2+** | — | Depends on return |
| `return_reason` | value, label, description, metadata + parent self-ref | **Deferred P2+** | — | Depends on return |
| `order_exchange` | order_version, display_id, no_notification, difference_due, allow_backorder, created_by, metadata, canceled_at + order/return FKs + additional_items/shipping/transactions hasMany | **Deferred P2+** | — | No exchanges in P1 |
| `order_exchange_item` | quantity, note, metadata + exchange/item FKs | **Deferred P2+** | — | Depends on order_exchange |
| `order_claim` | order_version, display_id, type (refund/replace), no_notification, refund_amount, created_by, metadata, canceled_at + order/return FKs + items/shipping/transactions hasMany | **Deferred P2+** | — | No claims in P1 |
| `order_claim_item` | reason, quantity, is_additional_item, note, metadata + claim/item FKs + images hasMany | **Deferred P2+** | — | Depends on order_claim |
| `order_claim_item_image` | url, metadata + claim_item FK | **Deferred P2+** | — | Depends on order_claim_item |
| `order_transaction` | version, amount, currency_code, reference, reference_id + order/return/exchange/claim FKs | **Deferred P2+** | — | toko-rs uses simplified `payment_records` instead |
| `order_credit_line` | version, reference, reference_id, amount, raw_amount, metadata + order FK | **Deferred P2+** | — | No credit system in P1 |

### Customer Module

| Medusa Table | Medusa Columns (key) | toko-rs Status | toko-rs Table/Column | Simplification |
|---|---|---|---|---|
| `customer` | company_name, first_name, last_name, email, phone, has_account, metadata, created_by + groups M2M + addresses hasMany | **Implemented** | `customers` | Dropped: company_name, created_by. Dropped relationships: groups. Unique constraint simplified (SQLite: plain email unique; PG: partial composite) |
| `customer_address` | address_name, is_default_shipping, is_default_billing, company, first_name, last_name, address_1/2, city, country_code, province, postal_code, phone, metadata + customer FK | **Active (read)** | `customer_addresses` | Table is read during customer queries — addresses array + default address IDs returned in response. No P1 endpoints write to it (CRUD deferred to P2). Partial unique indexes enforce one default shipping/billing per customer. Column: `province` (was `state_province`, renamed for Medusa compatibility) |
| `customer_group` | name, metadata, created_by + customers M2M | **Deferred P2+** | — | No customer groups in P1 |
| `customer_group_customer` (pivot) | created_by, metadata + customer/group FKs | **Deferred P2+** | — | Depends on customer_group |

### Payment Module

| Medusa Table | Medusa Columns (key) | toko-rs Status | toko-rs Table/Column | Simplification |
|---|---|---|---|---|
| `payment_collection` | amount, authorized_amount, currency_code, region_id, status, type, metadata + payment_providers + payments hasMany | **Collapsed** | `payment_records` (simplified) | Medusa uses a two-level structure (collection → sessions → payments). P1 collapses to a single `payment_records` table per order |
| `payment_session` | amount, currency_code, provider_id, data, status, metadata + payment_collection FK | **Deferred P2+** | — | No payment sessions in P1 (manual provider only) |
| `payment` | amount, authorized_amount, currency_code, amount_captured, amount_refunded, provider_id, data, status, metadata + payment_collection FK + order FK via link | **Collapsed** | `payment_records` | Simplified: id, order_id, amount, currency_code, status, provider, metadata, timestamps |

### Foundation

| Medusa Equivalent | toko-rs Table | Notes |
|---|---|---|
| Autoincrement sequences (Medusa uses `@AutoIncrement` on display_id) | `_sequences` | Application-managed sequence table. Pre-seeded with `order_display_id = 0`. Currently unused by code (uses `MAX(display_id)+1` instead) |
| None (Medusa handles idempotency at framework level) | `idempotency_keys` | toko-rs addition — maps idempotency key to response ID for preventing double-order creation |

### Summary

| Category | Count | Tables |
|---|---|---|
| **Implemented (exact or near-match)** | 8 | `products`, `product_options`, `product_option_values`, `product_variants`, `product_variant_option`, `customers`, `customer_addresses` (dormant), `carts` |
| **Implemented (simplified)** | 4 | `cart_line_items`, `orders`, `order_line_items`, `payment_records` |
| **Collapsed into column** | 3 Medusa tables → 6 JSON columns | `cart_address` → `carts.shipping_address`/`billing_address`, `order_address` → `orders.shipping_address`/`billing_address`, `order_summary` → computed fields |
| **Collapsed (merged table)** | 1 Medusa table → existing table | `order_item` → merged into `order_line_items` |
| **Foundation (toko-rs only)** | 2 | `_sequences`, `idempotency_keys` |
| **Deferred P2+** | 30 Medusa tables | See individual module sections above |

---

# Phase 2b: Database Foundation

Completed 2026-04-08. All 14 tasks done (2b.1–2b.14).

## Architecture

### Repositories Struct (replaces enum dispatch)

The old `DatabaseRepo` enum with `match self { Sqlite {..} => ..., Postgres {..} => ... }` on every method call was replaced with a simple struct:

```
src/db.rs
  AppDb         — enum holding the pool (currently Sqlite only; Postgres variant added in future)
  Repositories  — struct with individual repo instances (product, cart, ...)
  create_db()   — creates pool + repos
  run_migrations() — runs migration directory matching the pool type
  ping()        — health check query
```

```rust
pub struct Repositories {
    pub product: ProductRepository,
    pub cart: CartRepository,
}
```

### AppState

```rust
pub struct AppState {
    pub db: db::AppDb,           // pool for health check
    pub repos: Arc<db::Repositories>,  // shared across handlers
}
```

Routes access repos directly: `state.repos.product.find_by_id(&id)`. No delegation layer, no enum dispatch.

### Module Boundaries

Each module owns a single repository struct:
- `src/product/repository.rs` — `ProductRepository` (SqlitePool)
- `src/cart/repository.rs` — `CartRepository` (SqlitePool)

No cross-module imports. `db.rs` is the only shared coupling point that wires repos together.

### What Was Removed

| Removed | Reason |
|---|---|
| `SqliteProductRepository` / `PostgresProductRepository` dual structs | Single `ProductRepository` per module |
| `SqliteCartRepository` / `PostgresCartRepository` dual structs | Single `CartRepository` per module |
| `DatabaseRepo` enum with 17 delegate methods | Replaced by `Repositories` struct — routes call repos directly |
| All `#[cfg(coverage)]` / `#[cfg(not(coverage))]` guards | No longer needed without dual-repo stub pattern |
| `cfg(coverage)` in `Cargo.toml [lints.rust]` | No longer referenced |

## Migrations

### Two migration sets

| Directory | Purpose | Dialect |
|---|---|---|
| `migrations/` | **PostgreSQL-primary** (production) | `TIMESTAMPTZ`, `JSONB`, `BOOLEAN`, partial unique indexes, `CHECK` constraints |
| `migrations/sqlite/` | **SQLite** (test/dev in-memory) | `DATETIME`, `TEXT` JSON, `INTEGER` booleans |

### PostgreSQL enhancements over SQLite

- `TIMESTAMPTZ DEFAULT now()` instead of `DATETIME DEFAULT CURRENT_TIMESTAMP`
- `JSONB` instead of `TEXT` (JSON) — supports indexing and operators
- `CHECK (status IN (...))` constraints on status columns
- Partial unique indexes: `UNIQUE (handle) WHERE deleted_at IS NULL` — allows reusing handles after soft-delete
- Strategic indexes on foreign keys and filtered indexes on `deleted_at IS NULL`
- `provider TEXT NOT NULL DEFAULT 'manual'` on payment_records (NOT NULL in PG)

### Tables (11 + 1 pivot + 1 sequence)

| Table | Module | Key columns |
|---|---|---|
| `products` | product | id (TEXT PK), handle (UNIQUE WHERE deleted), status (CHECK) |
| `product_options` | product | FK → products CASCADE |
| `product_option_values` | product | FK → product_options CASCADE |
| `product_variants` | product | sku (UNIQUE WHERE deleted+NOT NULL), price (INTEGER cents) |
| `product_variant_options` | product | Pivot: variant ↔ option_value |
| `customers` | customer | email (UNIQUE WHERE deleted), has_account (BOOLEAN) |
| `customer_addresses` | customer | FK → customers CASCADE |
| `carts` | cart | completed_at (nullable), FK → customers SET NULL |
| `cart_line_items` | cart | FK → carts CASCADE, variant_id FK SET NULL, snapshot JSONB |
| `orders` | order | display_id (UNIQUE), status, FK → customers SET NULL |
| `order_line_items` | order | FK → orders CASCADE, snapshot JSONB |
| `payment_records` | payment | FK → orders CASCADE, status (CHECK) |
| `_sequences` | foundation | name/value pairs for display_id auto-increment |
| `idempotency_keys` | foundation | key → response_id mapping |

## Error Response Format

Now matches 3-field OAS Error schema from `specs/store.oas.yaml`:

```json
{
  "code": "invalid_request_error",
  "type": "not_found",
  "message": "Not Found: Product with id prod_xxx was not found"
}
```

| AppError variant | `code` | `type` | HTTP status |
|---|---|---|---|
| `NotFound` | `invalid_request_error` | `not_found` | 404 |
| `InvalidData` | `invalid_request_error` | `invalid_data` | 400 |
| `DuplicateError` | `invalid_request_error` | `duplicate_error` | 409 |
| `UnexpectedState` | `invalid_state_error` | `unexpected_state` | 409 |
| `Unauthorized` | `unknown_error` | `unauthorized` | 401 |
| `DatabaseError` | `api_error` | `database_error` | 500 |
| `MigrationError` | `api_error` | `migration_error` | 500 |

## Docker Integration

`docker-compose.yml` provides PostgreSQL 16 for full compatibility testing:

```bash
make docker-up    # start PG
make docker-down  # stop PG
make test-pg      # run tests against PostgreSQL
```

## Quality Gates

| Metric | Value |
|---|---|
| Tests | 41 passing |
| Clippy | Zero warnings (`-D warnings`) |
| Line coverage | 92.42% (`cargo llvm-cov`) |
| Warnings | Zero compiler warnings |
