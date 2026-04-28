# Phase 1-A — Product Module

## Overview

The product module provides 8 API endpoints for managing products with options, option values, and variants. It supports both admin (full CRUD) and store (published-only read) access patterns, matching Medusa's API contract.

## Endpoints

| Method | Path | Handler | Description |
|---|---|---|---|
| POST | `/admin/products` | `admin_create_product` | Create product with nested options/variants |
| GET | `/admin/products` | `admin_list_products` | Paginated product list with relations |
| GET | `/admin/products/{id}` | `admin_get_product` | Single product with full relations |
| POST | `/admin/products/{id}` | `admin_update_product` | Partial update (COALESCE pattern) |
| DELETE | `/admin/products/{id}` | `admin_delete_product` | Soft delete (Medusa delete response) |
| POST | `/admin/products/{id}/variants` | `admin_add_variant` | Add variant with option binding |
| GET | `/store/products` | `store_list_products` | Published-only product list |
| GET | `/store/products/{id}` | `store_get_product` | Published-only single product |

## File Structure

```
src/product/
├── mod.rs           # Module declarations
├── models.rs        # Database row models + composite relation models
├── types.rs         # Request/response types, validation
├── repository.rs    # SqliteProductRepository with all query methods
└── routes.rs        # Axum route handlers
```

## Repository Methods

`SqliteProductRepository` provides these methods:

| Method | Description |
|---|---|
| `create_product(input)` | Transactional insert: product → options → option_values → variants → variant_options pivot |
| `find_by_id(id)` | Fetch product with relations (excludes deleted) |
| `find_published_by_id(id)` | Fetch published product only (status='published' AND deleted_at IS NULL) |
| `find_by_id_any(id)` | Fetch product with relations (including deleted, for internal use) |
| `list(params)` | Paginated list with offset/limit/order/with_deleted support |
| `list_published(params)` | Paginated list of published products only |
| `update(id, input)` | COALESCE-based partial update with duplicate handle detection |
| `soft_delete(id)` | Set deleted_at, return product ID |
| `add_variant(product_id, input)` | Insert variant + resolve option bindings to pivot table |

## Data Model

```
Product 1──* ProductOption 1──* ProductOptionValue
    │                                   │
    └──* ProductVariant ────*─── product_variant_options (pivot)
```

- `product_variant_options` is a pivot table linking variants to option values
- `VariantOptionValue` is read via JOIN across the pivot → `product_option_values`
- `ProductWithRelations` assembles: Product + options (with values) + variants (with option bindings)

## Key Patterns

### ID Generation
All entity IDs use `types::generate_entity_id(prefix)` producing `{prefix}_{lowercase_ulid}`.

### Handle Generation
Auto-generated handles use `types::generate_handle(title)` via the `slug` crate (handles unicode, special characters).

### Duplicate Detection
SQLite UNIQUE constraint violations are caught in `map_unique_violation()` and converted to `AppError::DuplicateError` (HTTP 409).

### Partial Updates
Uses the `COALESCE(NULLIF(?, ''), column)` pattern to only update fields that are provided and non-empty.

### Variant Option Binding
When creating a variant with `"options": {"Size": "S"}`, the system:
1. Looks up the option by title and value for the product
2. Inserts a row into `product_variant_options` linking variant_id → option_value_id
3. Unresolved bindings are silently skipped (no error)

### Store Filtering
Store endpoints filter by `status = 'published' AND deleted_at IS NULL`. Admin endpoints filter only by `deleted_at IS NULL` (unless `with_deleted=true`).

## Response Formats

### Single product
```json
{"product": {id, title, handle, description, status, thumbnail, metadata,
             created_at, updated_at, deleted_at,
             options: [{id, product_id, title, values: [{id, option_id, value}]}],
             variants: [{id, product_id, title, sku, price, variant_rank,
                         options: [{id, value, option_id}]}]}}
```

### Product list
```json
{"products": [...], "count": N, "offset": 0, "limit": 50}
```

### Delete response
```json
{"id": "prod_...", "object": "product", "deleted": true}
```

## Test Coverage

14 integration tests covering all 8 endpoints and key scenarios:

- `test_admin_create_product_success` — Full product creation with options/variants/pivot
- `test_admin_create_product_validation_failure` — Empty title + negative price → 400
- `test_admin_create_product_duplicate_handle` — Duplicate handle → 409
- `test_admin_get_product` — Get by ID with full relations
- `test_admin_get_product_not_found` — Non-existent ID → 404
- `test_admin_list_products` — Paginated list with count
- `test_admin_update_product` — Partial update (status + title)
- `test_admin_update_product_not_found` — Non-existent ID → 404
- `test_admin_delete_product` — Soft delete + verify GET returns 404
- `test_admin_delete_product_not_found` — Non-existent ID → 404
- `test_admin_add_variant` — Add variant with option binding
- `test_store_list_published_only` — Draft excluded, published included
- `test_store_get_published_product` — Draft → 404, published → 200
- `test_store_deleted_product_returns_404` — Published+deleted → 404

---

## Implementation History (from audit-correction.md)

## 4c. Product Repository Transactional Safety

`create_product` and `add_variant` were inserting product + options + option values + variants
+ variant option bindings across multiple non-transactional queries. A failure mid-way (e.g.,
duplicate SKU on variant #2) would leave partial data — a product with options but no variants.

**Fix:** Wrapped both methods in `self.pool.begin()` transactions. Refactored `insert_variant`
and `resolve_variant_options` from `&self` methods into static `fn(tx: &mut Transaction)` so
they can run within the transaction context.

**Files changed:**
- `src/product/repository.rs` — `create_product` uses `tx`, `add_variant` uses `tx`, new `insert_variant_tx` and `resolve_variant_options_tx` static methods

**Behavior:** No API-visible change — existing tests continue to pass. The fix prevents
partial data on failure paths.

---
