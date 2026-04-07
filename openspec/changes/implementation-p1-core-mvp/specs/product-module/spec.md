## ADDED Requirements

### Requirement: Admin create product with options and variants
The system SHALL provide `POST /admin/products` that creates a product with nested options (each with values) and variants (each with option bindings). The `title` field is required. The `handle` field SHALL be auto-generated from the title via slugification if not provided. The `status` SHALL default to `"draft"`. All IDs SHALL use prefixed ULID format (`prod_`, `opt_`, `optval_`, `variant_`).

#### Scenario: Create product with options and variants
- **WHEN** a POST request is sent to `/admin/products` with body `{"title": "Classic T-Shirt", "options": [{"title": "Size", "values": ["S", "M", "L"]}], "variants": [{"title": "Small", "sku": "TS-S", "price": 2500, "options": {"Size": "S"}}]}`
- **THEN** the system returns 200 with `{"product": {...}}` containing the product with nested options, option values, and variants with option bindings

#### Scenario: Create product without title
- **WHEN** a POST request is sent to `/admin/products` with body `{}`
- **THEN** the system returns 400 with `{"type": "invalid_data", "message": "..."}`

#### Scenario: Create product with duplicate handle
- **WHEN** a POST request is sent to `/admin/products` with a `handle` that already exists on a non-deleted product
- **THEN** the system returns 409 with `{"type": "duplicate_error", "message": "..."}`

### Requirement: Admin update product
The system SHALL provide `POST /admin/products/:id` that updates any combination of title, handle, description, status, thumbnail, and metadata. The `updated_at` timestamp SHALL be set to the current time.

#### Scenario: Update product status to published
- **WHEN** a POST request is sent to `/admin/products/:id` with body `{"status": "published"}`
- **THEN** the system returns 200 with `{"product": {...}}` where `status` is `"published"` and `updated_at` is refreshed

#### Scenario: Update non-existent product
- **WHEN** a POST request is sent to `/admin/products/:id` with an ID that does not exist
- **THEN** the system returns 404 with `{"type": "not_found", "message": "..."}`

### Requirement: Admin soft delete product
The system SHALL provide `DELETE /admin/products/:id` that sets `deleted_at` to the current timestamp. The product SHALL remain in the database but be excluded from default queries. The response SHALL follow the Medusa delete pattern: `{"id": "...", "object": "product", "deleted": true}`.

#### Scenario: Soft delete existing product
- **WHEN** a DELETE request is sent to `/admin/products/:id` for an existing product
- **THEN** the system returns 200 with `{"id": "prod_...", "object": "product", "deleted": true}`

#### Scenario: Delete non-existent product
- **WHEN** a DELETE request is sent to `/admin/products/:id` with an ID that does not exist
- **THEN** the system returns 404 with `{"type": "not_found", "message": "..."}`

### Requirement: Admin list products
The system SHALL provide `GET /admin/products` that returns paginated products with nested options, option values, and variants. Default limit SHALL be 50. Supports `offset`, `limit`, `order`, and `with_deleted` query parameters.

#### Scenario: List products with pagination
- **WHEN** a GET request is sent to `/admin/products?offset=0&limit=10`
- **THEN** the system returns 200 with `{"products": [...], "count": N, "offset": 0, "limit": 10}` where each product includes nested options and variants

#### Scenario: List products including soft-deleted
- **WHEN** a GET request is sent to `/admin/products?with_deleted=true`
- **THEN** the system returns products where `deleted_at IS NOT NULL` are included in results

### Requirement: Admin get single product
The system SHALL provide `GET /admin/products/:id` that returns a single product with nested options (with values) and variants (with option bindings).

#### Scenario: Get existing product
- **WHEN** a GET request is sent to `/admin/products/:id` for an existing product
- **THEN** the system returns 200 with `{"product": {...}}` containing full product with relations

### Requirement: Admin add variant to product
The system SHALL provide `POST /admin/products/:id/variants` that adds a new variant to an existing product. The `title` and `price` fields are required. Returns the full updated product.

#### Scenario: Add variant with option binding
- **WHEN** a POST request is sent to `/admin/products/:id/variants` with body `{"title": "XL Blue", "sku": "TS-XL-BLUE", "price": 2900, "options": {"Size": "XL"}}`
- **THEN** the system returns 200 with `{"product": {...}}` containing the new variant with option binding

#### Scenario: Add variant with duplicate SKU
- **WHEN** a POST request is sent to `/admin/products/:id/variants` with an SKU that already exists on a non-deleted variant
- **THEN** the system returns 409 with `{"code": "invalid_request_error", "type": "duplicate_error", "message": "..."}`

### Requirement: Variant-to-option binding persisted
When a variant is created with an `options` map (option title → value), the system SHALL resolve each option title to its `product_option` and each value to its `product_option_value`, then insert a row into the `product_variant_options` pivot table (variant_id, option_value_id). The variant response SHALL include the resolved option bindings.

#### Scenario: Variant with option binding persisted to pivot table
- **WHEN** a variant is created with `"options": {"Size": "S"}` and a "Size" option with value "S" exists for the product
- **THEN** a row is inserted into `product_variant_options` linking the variant_id to the optval_id for "S", and the variant's `options` array in the response contains `{"id": "optval_...", "value": "S", "option_id": "opt_..."}`

#### Scenario: Variant option value not found
- **WHEN** a variant is created with `"options": {"Color": "Green"}` but no "Color" option with value "Green" exists for the product
- **THEN** the system skips the unresolved binding (no error) — variant is created with empty options for that mapping

### Requirement: Store list published products
The system SHALL provide `GET /store/products` that returns only products with `status = "published"` and `deleted_at IS NULL`. Default limit SHALL be 50.

#### Scenario: Browse published products
- **WHEN** a GET request is sent to `/store/products`
- **THEN** the system returns 200 with `{"products": [...], "count": N, "offset": 0, "limit": 50}` where all products have `status: "published"`

#### Scenario: Draft products excluded from store
- **WHEN** a product with `status: "draft"` exists
- **THEN** that product SHALL NOT appear in the `/store/products` response

### Requirement: Store get single published product
The system SHALL provide `GET /store/products/:id` that returns a published product with nested relations. Draft or soft-deleted products SHALL return 404.

#### Scenario: Get published product via store
- **WHEN** a GET request is sent to `/store/products/:id` for a published product
- **THEN** the system returns 200 with `{"product": {...}}`

#### Scenario: Get draft product via store returns 404
- **WHEN** a GET request is sent to `/store/products/:id` for a product with `status: "draft"`
- **THEN** the system returns 404 with `{"type": "not_found", "message": "..."}`

#### Scenario: Get soft-deleted product via store returns 404
- **WHEN** a GET request is sent to `/store/products/:id` for a soft-deleted product
- **THEN** the system returns 404 with `{"type": "not_found", "message": "..."}`
