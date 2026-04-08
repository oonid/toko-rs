# Seed Data

Spec reference: `openspec/changes/implementation-p1-core-mvp/specs/foundation/spec.md` — "Seed data command" requirement.

## Usage

```bash
# Run seed against configured database
cargo run -- --seed

# Or via Makefile
make seed
```

The `--seed` CLI flag runs the seed function and exits. It does **not** start the HTTP server.

## Design Decisions

### Fixed IDs for idempotency

All seed entities use deterministic, fixed IDs (e.g., `prod_seed_kaos_polos`, `cus_seed_budi`) rather than generated ULIDs. This makes idempotency trivial — the seed function checks if each ID exists before inserting. Running `--seed` multiple times produces the same result.

### Direct SQL, not repositories

The seed function uses raw SQL queries instead of calling repository methods. This follows the module boundary rule — `seed.rs` is shared infrastructure (like `db.rs`), not a domain module. It imports only `crate::db::AppDb` and `crate::error::AppError`. It avoids importing domain repositories while maintaining full control over the insert logic.

### `INSERT OR IGNORE` for sub-entities

Child records (options, option values, variants, variant-option bindings) use `INSERT OR IGNORE` since their existence is guaranteed by the parent check. Parent records (products, customers) use an explicit `SELECT COUNT(*)` check with tracing logs.

### Incrementing variant_rank

Each variant within a product receives an incrementing `variant_rank` (0, 1, 2, ...) matching the order they appear in the seed array. This mirrors the `COALESCE(MAX(variant_rank), -1) + 1` pattern used by `product/repository.rs:add_variant`.

## Seed Data Inventory

### Products (3, all published)

| ID | Title | Handle | Variants | Price Range (IDR) |
|---|---|---|---|---|
| `prod_seed_kaos_polos` | Kaos Polos | `kaos-polos` | 4 (S/M/L/XL) | 75,000 – 80,000 |
| `prod_seed_jeans_slim` | Jeans Slim Fit | `jeans-slim-fit` | 4 (28/30/32/34) | 250,000 – 275,000 |
| `prod_seed_sneakers` | Sneakers Classic | `sneakers-classic` | 5 (39–43) | 450,000 – 475,000 |

Each product has:
- 1 option ("Ukuran" / Size) with 4–5 values
- 4–5 variants with option bindings to the size option
- Status: `published` (visible via `/store/products`)
- Variant SKUs follow the pattern `{PRODUCT_PREFIX}-{SIZE}` (e.g., `KAOS-P-M`, `JEANS-S-30`, `SNKR-41`)

### Entity ID inventory

| Entity type | Count | ID pattern |
|---|---|---|
| Products | 3 | `prod_seed_{product}` |
| Options | 3 | `opt_seed_{product}_size` |
| Option values | 13 (4+4+5) | `optval_seed_{product}_s_{index}` |
| Variants | 13 | `var_seed_{product}_{size}` |
| Variant-option bindings | 13 | `pvo_seed_{product}_{rank}` |
| Customer | 1 | `cus_seed_budi` |

### Customer (1)

| ID | Name | Email | Phone |
|---|---|---|---|
| `cus_seed_budi` | Budi Santoso | budi@example.com | +6281234567890 |

- `has_account = true`
- Can be used with `X-Customer-Id: cus_seed_budi` header for order history endpoints

## Test Coverage

### Unit tests (`src/seed.rs` — 7 tests)

| Test | What it verifies |
|---|---|
| `test_seed_creates_products_and_customer` | Correct counts: 3 products, 13 variants, 1 customer |
| `test_seed_is_idempotent` | Running seed twice produces same counts for products, variants, options, bindings, and customer |
| `test_seed_products_are_published` | No seed product has status other than `published` |
| `test_seed_variants_have_option_bindings` | All 13 variants have corresponding `product_variant_option` rows |
| `test_seed_customer_has_account` | Seed customer has `has_account = true` |
| `test_seed_variant_ranks_are_ordered` | Variant ranks within a product are 0, 1, 2, 3 (not all zero) |

### Integration smoke tests (`tests/seed_flow_test.rs` — 2 tests)

| Test | Flow exercised |
|---|---|
| `test_full_browse_cart_checkout_flow` | `GET /store/products` → `GET /store/products/:id` → `POST /store/carts` → `POST /store/carts/:id/line-items` → `POST /store/carts/:id/complete` → verify order + payment |
| `test_customer_browse_order_history_flow` | Browse → `POST /store/carts` with `customer_id` → complete → `GET /store/orders` with auth header → `GET /store/orders/:id` with auth header |

Both tests call `run_seed()` against an in-memory SQLite database before exercising the endpoints, verifying that seeded data works correctly through the entire API stack.

## TDD Record

1. **RED**: Wrote 6 seed unit tests + 2 seed flow integration tests first
2. **GREEN**: Implemented `run_seed()` with product/customer insertion, option+variant wiring
3. **Verify**: 79 tests pass, clippy clean, zero warnings
