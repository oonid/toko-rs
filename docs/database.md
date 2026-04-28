# Database

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
| `order_change` | version, change_type, status, actions | **Deferred P2+** | — | No order edits in P1 |
| `return` / `order_exchange` / `order_claim` | Return, exchange, claim workflows | **Deferred P2+** | — | No returns/exchanges/claims in P1 |
| `order_transaction` | version, amount, currency_code, reference | **Deferred P2+** | — | toko-rs uses simplified `payment_records` instead |

### Customer Module

| Medusa Table | Medusa Columns (key) | toko-rs Status | toko-rs Table/Column | Simplification |
|---|---|---|---|---|
| `customer` | company_name, first_name, last_name, email, phone, has_account, metadata, created_by + groups M2M + addresses hasMany | **Implemented** | `customers` | Dropped: company_name, created_by. Dropped relationships: groups. Partial unique index on (email, has_account) WHERE deleted_at IS NULL |
| `customer_address` | address_name, is_default_shipping, is_default_billing, company, first_name, last_name, address_1/2, city, country_code, province, postal_code, phone, metadata + customer FK | **Active (read)** | `customer_addresses` | Table is read during customer queries — addresses array + default address IDs returned in response. No P1 endpoints write to it (CRUD deferred to P2). Partial unique indexes enforce one default shipping/billing per customer. |
| `customer_group` / `customer_group_customer` | name, metadata, M2M pivot | **Deferred P2+** | — | No customer groups in P1 |

### Payment Module

| Medusa Table | Medusa Columns (key) | toko-rs Status | toko-rs Table/Column | Simplification |
|---|---|---|---|---|
| `payment_collection` + `payment` | Two-level: collection → sessions → payments | **Collapsed** | `payment_records` (simplified) | P1 collapses to single table: id, order_id, amount, currency_code, status, provider, metadata, timestamps |

### Foundation

| Medusa Equivalent | toko-rs Table | Notes |
|---|---|---|
| Autoincrement sequences (Medusa uses `@AutoIncrement`) | `_sequences` | Application-managed sequence table. Pre-seeded with `order_display_id = 0`. Atomic `UPDATE ... SET value = value + 1 ... RETURNING value` |
| None (Medusa handles idempotency at framework level) | `idempotency_keys` | Maps idempotency key to response ID for preventing double-order creation |

### Summary

| Category | Count | Tables |
|---|---|---|
| **Implemented (exact or near-match)** | 8 | `products`, `product_options`, `product_option_values`, `product_variants`, `product_variant_option`, `customers`, `customer_addresses`, `carts` |
| **Implemented (simplified)** | 4 | `cart_line_items`, `orders`, `order_line_items`, `payment_records` |
| **Collapsed into column** | 3 → 6 JSON columns | `cart_address`, `order_address`, `order_summary` |
| **Foundation (toko-rs only)** | 2 | `_sequences`, `idempotency_keys` |
| **Deferred P2+** | 30 | See individual module sections above |

---

## Architecture

### Repositories Struct

```rust
pub struct Repositories {
    pub product: ProductRepository,
    pub cart: CartRepository,
    pub customer: CustomerRepository,
    pub order: OrderRepository,
    pub payment: PaymentRepository,
}
```

```rust
pub struct AppState {
    pub db: db::AppDb,              // pool for health check
    pub repos: Arc<db::Repositories>, // shared across handlers
}
```

Routes access repos directly: `state.repos.product.find_by_id(&id)`. No delegation layer, no enum dispatch.

### Module Boundaries

Each module owns a single repository struct using `DbPool` (type alias resolving to `PgPool` or `SqlitePool` at compile time):
- `src/product/repository.rs` — `ProductRepository`
- `src/cart/repository.rs` — `CartRepository`
- `src/customer/repository.rs` — `CustomerRepository`
- `src/order/repository.rs` — `OrderRepository`
- `src/payment/repository.rs` — `PaymentRepository`

No cross-module imports. `db.rs` is the only shared coupling point.

---

## Migrations

### Two Migration Sets

| Directory | Purpose | Dialect |
|---|---|---|
| `migrations/` | **PostgreSQL-primary** (production) | `TIMESTAMPTZ`, `JSONB`, `BOOLEAN`, partial unique indexes, `CHECK` constraints |
| `migrations/sqlite/` | **SQLite** (test/dev) | `DATETIME`, `TEXT` JSON, `INTEGER` booleans |

### PostgreSQL Enhancements Over SQLite

- `TIMESTAMPTZ DEFAULT now()` instead of `DATETIME DEFAULT CURRENT_TIMESTAMP`
- `JSONB` instead of `TEXT` (JSON) — supports indexing and operators
- `CHECK (status IN (...))` constraints on status columns
- Partial unique indexes: `UNIQUE (handle) WHERE deleted_at IS NULL` — allows reusing handles after soft-delete
- Strategic indexes on foreign keys and filtered indexes on `deleted_at IS NULL`

### Tables (11 + 1 pivot + 1 sequence + 1 idempotency)

| Table | Module | Key columns |
|---|---|---|
| `products` | product | id (TEXT PK), handle (UNIQUE WHERE deleted), status (CHECK) |
| `product_options` | product | FK → products CASCADE |
| `product_option_values` | product | FK → product_options CASCADE |
| `product_variants` | product | sku (UNIQUE WHERE deleted+NOT NULL), price (BIGINT cents) |
| `product_variant_option` | product | Pivot: variant ↔ option_value |
| `customers` | customer | email (UNIQUE WHERE deleted), has_account (BOOLEAN) |
| `customer_addresses` | customer | FK → customers CASCADE |
| `carts` | cart | completed_at (nullable), FK → customers SET NULL |
| `cart_line_items` | cart | FK → carts CASCADE, variant_id FK SET NULL, snapshot JSONB |
| `orders` | order | display_id (UNIQUE), status, FK → customers SET NULL |
| `order_line_items` | order | FK → orders CASCADE, snapshot JSONB |
| `payment_records` | payment | FK → orders CASCADE, status (CHECK) |
| `_sequences` | foundation | name/value pairs for display_id auto-increment |
| `idempotency_keys` | foundation | key → response_id mapping |

### PG Migration Fixes (Task 15)

PostgreSQL does not support `WHERE` in inline `UNIQUE` constraints. Extracted to `CREATE UNIQUE INDEX ... WHERE`:

| Table | Constraint | Fix |
|---|---|---|
| `products` | `UNIQUE (handle) WHERE deleted_at IS NULL` | `CREATE UNIQUE INDEX uq_products_handle ON products (handle) WHERE deleted_at IS NULL` |
| `product_variants` | `UNIQUE (sku) WHERE deleted_at IS NULL AND sku IS NOT NULL` | `CREATE UNIQUE INDEX uq_product_variants_sku ON product_variants (sku) WHERE deleted_at IS NULL AND sku IS NOT NULL` |
| `customers` | `UNIQUE (email, has_account) WHERE deleted_at IS NULL` | `CREATE UNIQUE INDEX uq_customers_email ON customers (email, has_account) WHERE deleted_at IS NULL` |

### Type Mapping (INTEGER → BIGINT)

PG `INTEGER` is INT4 (32-bit). Rust `i64` requires INT8. All numeric columns changed to `BIGINT`:

| Migration | Columns |
|---|---|
| `001_products.sql` | `product_variants.price`, `product_variants.variant_rank` |
| `003_carts.sql` | `cart_line_items.quantity`, `cart_line_items.unit_price` |
| `004_orders.sql` | `orders.display_id`, `order_line_items.quantity`, `order_line_items.unit_price`, `_sequences.value` |
| `005_payments.sql` | `payment_records.amount` |

---

## Error Response Format

Matches 3-field OAS Error schema from `specs/store.oas.yaml`:

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
| `DuplicateError` | `invalid_request_error` | `duplicate_error` | 422 |
| `Conflict` | `invalid_state_error` | `conflict` | 409 |
| `Forbidden` | `invalid_state_error` | `forbidden` | 403 |
| `Unauthorized` | `unknown_error` | `unauthorized` | 401 |
| `UnexpectedState` | `invalid_state_error` | `unexpected_state` | 500 |
| `DatabaseError` | `api_error` | `database_error` | 500 |
| `MigrationError` | `api_error` | `database_error` | 500 |

### PG Error Code Mapping

`map_db_constraint()` in `src/error.rs` translates PG error codes:

| PG Code | Name | toko-rs Variant |
|---|---|---|
| `23505` | unique_violation | `DuplicateError` |
| `23503` | foreign_key_violation | `NotFound` |
| `23502` | not_null_violation | `InvalidData` |

Repos also check `db_err.code().as_deref() == Some("23505")` inline for context-specific messages (e.g., "Variant with SKU 'X' already exists").

---

## Docker Integration

`docker-compose.yml` provides PostgreSQL 16 with auto-creation of test databases via `scripts/init-dbs.sh`:

```bash
make docker-up    # start PG (auto-creates toko_test + toko_e2e)
make docker-down  # stop PG
make test-pg      # run tests against PostgreSQL
```

Credentials: `postgres:postgres@localhost:5432`

| Database | Purpose |
|---|---|
| `toko` | Production / manual testing |
| `toko_test` | Integration tests (tower::oneshot) |
| `toko_e2e` | E2E tests (live HTTP) |

---

## SQLite Support

SQLite is an optional compile-time backend via Cargo feature flag. See `docs/database-ext-sqlite.md` for full documentation.

### Quick Start

```bash
# Build with SQLite backend
cargo build --features sqlite --no-default-features

# Run tests against SQLite in-memory
make test-sqlite

# Run all tests (PG + SQLite)
make test-all
```

### Feature Flag Architecture

| Cargo Feature | sqlx backend | `DbPool` resolves to | Migration path |
|---|---|---|---|
| `postgres` (default) | `sqlx/postgres` | `PgPool` | `./migrations/` |
| `sqlite` | `sqlx/sqlite` | `SqlitePool` | `./migrations/sqlite/` |

Type aliases (`DbPool`, `DbPoolOptions`, `DbDatabase`, `DbTransaction`) in `src/db.rs` resolve via `#[cfg]` to the appropriate backend types. No method-level cfg guards — all SQL is portable across both backends.

### Error Code Mapping

| Constraint | PG Code | SQLite Code | Helper function |
|---|---|---|---|
| Unique violation | `23505` | `2067` | `is_unique_violation()` |
| FK violation | `23503` | `787` | `is_fk_violation()` |
| Not-null violation | `23502` | `1299` | `is_not_null_violation()` |

---

## Implementation History (from audit-correction.md)

## 4b. Database Schema Alignment with Medusa Models

### Pivot table rename: `product_variant_options` → `product_variant_option`

Medusa's `ProductVariant.options` relation declares `pivotTable: "product_variant_option"`
(singular). toko-rs was using `product_variant_options` (plural).

**Files changed:**
- `migrations/001_products.sql` — CREATE TABLE name
- `migrations/sqlite/001_products.sql` — CREATE TABLE name
- `src/product/repository.rs:345` — INSERT INTO statement
- `src/product/repository.rs:392` — SELECT JOIN statement

### SQLite products.handle: column UNIQUE → partial unique index

The PG migration correctly used a partial unique constraint:
```sql
CONSTRAINT uq_products_handle UNIQUE (handle) WHERE deleted_at IS NULL
```

The SQLite migration used a column-level `UNIQUE` which does not respect `deleted_at`:
```sql
handle TEXT NOT NULL UNIQUE,  -- blocks handle re-use after soft delete
```

**Fix:** Removed column-level UNIQUE, added partial unique index (supported by SQLite 3.8+):
```sql
handle TEXT NOT NULL,
-- ...
CREATE UNIQUE INDEX uq_products_handle ON products (handle) WHERE deleted_at IS NULL;
```

**Bug demonstrated:** `test_admin_create_product_reuse_handle_after_soft_delete` — create
product → soft-delete → create new product with same title (handle auto-generated) → was
returning 422, now returns 200.

### Missing unique indexes on product_options and product_option_values

Medusa defines two partial unique indexes that toko-rs was missing:

| Medusa index name | Columns | Condition |
|---|---|---|
| `IDX_option_product_id_title_unique` | `(product_id, title)` | `WHERE deleted_at IS NULL` |
| `IDX_option_value_option_id_unique` | `(option_id, value)` | `WHERE deleted_at IS NULL` |

These prevent creating two options with the same title on one product, or two option values
with the same value under one option. Added to both PG and SQLite migrations.

### Complete index inventory (001_products)

| Index | PG | SQLite | Medusa reference |
|---|---|---|---|
| `uq_products_handle` partial unique | `CONSTRAINT` | `CREATE UNIQUE INDEX` | `IDX_product_handle_unique` |
| `uq_product_variants_sku` partial unique | `CONSTRAINT` | — (not added, SKU nullable) | `IDX_product_variant_sku_unique` |
| `uq_product_options_product_id_title` partial unique | **Added** | **Added** | `IDX_option_product_id_title_unique` |
| `uq_product_option_values_option_id_value` partial unique | **Added** | **Added** | `IDX_option_value_option_id_unique` |
| `idx_products_status` partial | Yes | — (not needed for SQLite test perf) | `IDX_product_status` |
| `idx_product_options_product_id` | Yes | — | performance index |
| `idx_product_option_values_option_id` | Yes | — | performance index |
| `idx_product_variants_product_id` partial | Yes | — | `IDX_product_variant_product_id` |

### TDD Record (4b)

1. **RED**: `test_admin_create_product_reuse_handle_after_soft_delete` — creates product, soft-deletes, creates again with same title. Failed: 422 (handle unique violation on SQLite)
2. **GREEN**: Fixed all 3 migration issues in one pass — pivot rename, partial unique index, missing indexes. Also fixed 2 SQL references in repository.rs
3. **Verify**: 52 tests pass (1 new), clippy clean

---
