# toko-rs

A modular, high-performance headless e-commerce backend written in Rust, API-compatible with [MedusaJS v2](https://medusajs.com/).

Implements the core **Browse → Cart → Checkout** flow with 38 endpoint methods across 6 domain modules, backed by PostgreSQL (primary) or SQLite (optional).

## Quick Start

```bash
# Start PostgreSQL
docker compose up -d

# Copy config
cp .env.example .env

# Seed sample data (3 products + 1 customer)
cargo run -- --seed

# Start server
cargo run
```

```bash
# Verify
curl http://localhost:3000/health

# Browse products
curl http://localhost:3000/store/products | jq '.products[].title'
```

## Architecture

Single-binary modular monolith. Each domain module (`product/`, `cart/`, `customer/`, `order/`, `payment/`, `invoice/`) follows the same internal structure:

```
src/
  main.rs              Entry point, config, graceful shutdown
  lib.rs               AppState, router composition, health check
  config.rs            Environment-based configuration (envy + dotenvy)
  db.rs                Database pool, migrations, constraint helpers
  error.rs             AppError enum → Medusa-compatible JSON errors
  extract.rs           Custom JSON extractor with clean error mapping
  types.rs             Shared: ULID generation, slugify, pagination
  seed.rs              Idempotent seed data

  product/
    routes.rs          Axum handlers (admin + store)
    types.rs           Request/response DTOs
    models.rs          Database row structs (FromRow)
    repository.rs      SQL queries, business logic
  cart/                (same structure)
  customer/            (same structure)
  order/               (same structure)
  payment/             (internal only — no routes in P1)
  invoice/             (admin only — config + on-the-fly generation)
```

**Layers**: Routes → Types → Repository → Database. No service layer in P1 — handlers call repositories directly.

### Key Design Decisions

- **ULID-prefixed IDs**: `prod_01KQ...`, `cart_01KQ...`, `order_01KQ...`, `cus_01KQ...`
- **Soft deletes**: `deleted_at` column with partial unique indexes (`WHERE deleted_at IS NULL`)
- **Cart completion**: Atomic SQL transaction with `SELECT ... FOR UPDATE` (PostgreSQL) — creates order + payment record + idempotency key in one commit
- **Line item merging**: Same variant + same price + same metadata merges quantity; different metadata creates separate line items
- **Snapshot pattern**: Cart line items store a JSON snapshot of variant data at add-time
- **POST for updates**: Follows Medusa convention — both create and update use `POST`
- **Error format**: Medusa OAS-compatible 3-field JSON: `{"code", "type", "message"}`

## API Reference

### Admin: Products (17 endpoints)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/admin/products` | Create product (with options, variants, images) |
| GET | `/admin/products` | List products (paginated, `with_deleted` filter) |
| GET | `/admin/products/:id` | Get product |
| POST | `/admin/products/:id` | Update product |
| DELETE | `/admin/products/:id` | Soft delete (cascades to variants, options) |
| POST | `/admin/products/:id/variants` | Add variant |
| GET | `/admin/products/:id/variants` | List variants |
| GET | `/admin/products/:id/variants/:vid` | Get variant |
| POST | `/admin/products/:id/variants/:vid` | Update variant |
| DELETE | `/admin/products/:id/variants/:vid` | Delete variant |
| GET | `/admin/products/:id/options` | List options |
| POST | `/admin/products/:id/options` | Create option |
| GET | `/admin/products/:id/options/:oid` | Get option |
| POST | `/admin/products/:id/options/:oid` | Update option |
| DELETE | `/admin/products/:id/options/:oid` | Delete option |
| GET | `/store/products` | List published products |
| GET | `/store/products/:id` | Get published product |

### Store: Cart (7 endpoints)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/store/carts` | Create cart |
| GET | `/store/carts/:id` | Get cart with line items |
| POST | `/store/carts/:id` | Update cart (email, metadata) |
| POST | `/store/carts/:id/line-items` | Add line item |
| POST | `/store/carts/:id/line-items/:lid` | Update quantity |
| DELETE | `/store/carts/:id/line-items/:lid` | Remove line item |
| POST | `/store/carts/:id/complete` | Complete cart → order (idempotent) |

### Store: Orders (2 endpoints)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/store/orders` | List customer orders (`X-Customer-Id` header) |
| GET | `/store/orders/:id` | Get order detail (`X-Customer-Id` header) |

### Store: Customers (3 endpoints)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/store/customers` | Register customer |
| GET | `/store/customers/me` | Get profile (`X-Customer-Id` header) |
| POST | `/store/customers/me` | Update profile (`X-Customer-Id` header) |

### Admin: Customers (2 endpoints)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/admin/customers` | List customers (`q`, `email`, `first_name`, `last_name`, `has_account` filters) |
| GET | `/admin/customers/:id` | Get customer with addresses |

### Admin: Carts (1 endpoint)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/admin/carts` | List carts (`id`, `customer_id` filters) |

### Admin: Orders (2 endpoints)

| Method | Path | Description |
|--------|------|-------------|
| POST | `/admin/orders/:id/cancel` | Cancel order (sets canceled_at, updates payment) |
| POST | `/admin/orders/:id/complete` | Complete order |

### Admin: Invoice (3 endpoints)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/admin/invoice-config` | Get issuer company config (from env vars) |
| POST | `/admin/invoice-config` | Returns current config (read-only; env-based) |
| GET | `/admin/orders/:id/invoice` | Generate invoice on-the-fly from order data |

### Health

| Method | Path | Description |
|--------|------|-------------|
| GET | `/health` | Database ping + version |

## Database

5 migrations, 14 tables:

| Migration | Tables |
|-----------|--------|
| `001_products` | `products`, `product_options`, `product_option_values`, `product_variants`, `product_variant_option`, `product_images` |
| `002_customers` | `customers`, `customer_addresses` |
| `003_carts` | `carts`, `cart_line_items` |
| `004_orders` | `_sequences`, `orders`, `order_line_items` |
| `005_payments` | `payment_records` |

Invoice config is stored as environment variables (not a DB table): `INVOICE_COMPANY_NAME`, `INVOICE_COMPANY_ADDRESS`, `INVOICE_COMPANY_PHONE`, `INVOICE_COMPANY_EMAIL`, `INVOICE_COMPANY_LOGO`, `INVOICE_NOTES`.

PostgreSQL is the default. SQLite is available behind a feature flag:

```bash
cargo run --features sqlite --no-default-features
```

## Testing

```bash
make docker-up                        # Start PostgreSQL

# Integration tests (191 tests, requires PostgreSQL)
DATABASE_URL=postgres://postgres:postgres@localhost:5432/toko_test \
  cargo test -- --test-threads=1

# SQLite tests
DATABASE_URL="sqlite::memory:" \
  cargo test --features sqlite --no-default-features -- --test-threads=1

# E2E tests (spawns live HTTP server)
E2E_DATABASE_URL=postgres://postgres:postgres@localhost:5432/toko_e2e \
  cargo test --test e2e -- --test-threads=1

# Coverage (requires cargo-llvm-cov)
make cov
```

Tests run single-threaded (`--test-threads=1`) for database isolation. Each test cleans its own tables via `clean_all_tables()`.

### Test Organization

```
tests/
  common/mod.rs           Shared test helpers (setup_test_app, clean_all_tables)
  product_test.rs         Product admin + store integration tests
  cart_test.rs            Cart lifecycle tests
  order_test.rs           Order + payment tests
  customer_test.rs        Customer registration + profile tests
  invoice_test.rs         Invoice config + generation tests
  contract_test.rs        Response shape validation against Medusa OAS
  e2e/                    End-to-end tests against live HTTP server
```

## Configuration

Configured via environment variables or `.env` file:

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | — | PostgreSQL or SQLite connection string |
| `HOST` | `0.0.0.0` | Bind address |
| `PORT` | `3000` | Bind port |
| `RUST_LOG` | `toko_rs=debug` | Tracing filter |
| `DEFAULT_CURRENCY_CODE` | `idr` | ISO 4217 currency code (lowercase) |
| `CORS_ORIGINS` | `*` | Comma-separated allowed origins |

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Web framework | [Axum](https://github.com/tokio-rs/axum) 0.8 |
| Async runtime | [Tokio](https://tokio.rs/) 1 |
| Database | [SQLx](https://github.com/launchbadge/sqlx) 0.8 (compile-time checked queries) |
| Serialization | serde + serde_json |
| Validation | validator (derive) |
| ID generation | ulid |
| Error handling | thiserror |
| Observability | tracing + tracing-subscriber |

## Makefile

```bash
make dev          # cargo run
make test         # cargo test
make lint         # cargo clippy -- -D warnings
make fmt          # cargo fmt
make seed         # cargo run -- --seed
make docker-up    # docker compose up -d
make docker-down  # docker compose down
make test-pg      # Test against PostgreSQL
make test-sqlite  # Test against SQLite
make test-all     # Both databases
make cov          # cargo llvm-cov
```

## Project Status

**P1 (Core MVP) — Complete.** 238 tests, clippy-clean, 38 endpoint methods, 14 tables, 5 migrations.

The following are out of scope for P1 and planned for future phases:

- Admin authentication / RBAC
- Regions, multi-currency, tax calculation
- Shipping providers and fulfillment
- Payment provider integrations
- Inventory management
- Promotions / discounts
- Product collections
- File/image upload service
- Order edits and returns

## License

MIT
