## ADDED Requirements

### Requirement: Store create cart
The system SHALL provide `POST /store/carts` that creates a new cart. The `currency_code` field defaults to `"usd"`. The response SHALL return the cart with empty items, `item_total: 0`, and `total: 0`.

#### Scenario: Create cart with defaults
- **WHEN** a POST request is sent to `/store/carts` with body `{}`
- **THEN** the system returns 200 with `{"cart": {"id": "cart_...", "currency_code": "usd", "items": [], "item_total": 0, "total": 0}}`

#### Scenario: Create cart with email
- **WHEN** a POST request is sent to `/store/carts` with body `{"currency_code": "eur", "email": "buyer@example.com"}`
- **THEN** the system returns 200 with the cart having `currency_code: "eur"` and `email: "buyer@example.com"`

### Requirement: Store get cart with items
The system SHALL provide `GET /store/carts/:id` that returns the cart with all non-deleted line items and computed totals (`item_total` = sum of `quantity * unit_price` per item, `total` = `item_total`).

#### Scenario: Get cart with line items
- **WHEN** a GET request is sent to `/store/carts/:id` for a cart with line items
- **THEN** the system returns 200 with `{"cart": {...}}` where items array is populated, `item_total` and `total` are computed correctly

#### Scenario: Get non-existent cart
- **WHEN** a GET request is sent to `/store/carts/:id` with an ID that does not exist
- **THEN** the system returns 404 with `{"type": "not_found", "message": "..."}`

### Requirement: Store update cart
The system SHALL provide `POST /store/carts/:id` that updates email and/or metadata. The `updated_at` timestamp SHALL be refreshed.

#### Scenario: Update cart email
- **WHEN** a POST request is sent to `/store/carts/:id` with body `{"email": "new@example.com"}`
- **THEN** the system returns 200 with `{"cart": {...}}` where email is updated

#### Scenario: Update completed cart
- **WHEN** a POST request is sent to `/store/carts/:id` for a cart where `completed_at IS NOT NULL`
- **THEN** the system returns 409 with `{"type": "unexpected_state", "message": "..."}`

### Requirement: Store add line item to cart
The system SHALL provide `POST /store/carts/:id/line-items` that adds a line item. The `variant_id` and `quantity` fields are required. The system SHALL look up the variant's price and product info to create a snapshot. The `quantity` MUST be greater than 0. Returns the full updated cart.

#### Scenario: Add line item successfully
- **WHEN** a POST request is sent to `/store/carts/:id/line-items` with body `{"variant_id": "variant_...", "quantity": 2}`
- **THEN** the system returns 200 with the full cart including the new line item with `unit_price` from variant, computed `total`, and a `snapshot` containing product_title, variant_title, and variant_sku

#### Scenario: Add line item with quantity zero
- **WHEN** a POST request is sent to `/store/carts/:id/line-items` with body `{"variant_id": "variant_...", "quantity": 0}`
- **THEN** the system returns 400 with `{"type": "invalid_data", "message": "..."}`

#### Scenario: Add line item with non-existent variant
- **WHEN** a POST request is sent to `/store/carts/:id/line-items` with a `variant_id` that does not exist
- **THEN** the system returns 404 with `{"type": "not_found", "message": "..."}`

### Requirement: Store update line item quantity
The system SHALL provide `POST /store/carts/:id/line-items/:line_id` that updates the line item quantity. The `quantity` field is required and MUST be >= 0. If `quantity` is 0, the line item SHALL be soft-deleted. Returns the full updated cart.

#### Scenario: Update quantity to positive value
- **WHEN** a POST request is sent to `/store/carts/:id/line-items/:line_id` with body `{"quantity": 5}`
- **THEN** the system returns 200 with the full cart, line item has `quantity: 5`, and totals are recalculated

#### Scenario: Update quantity to zero removes item
- **WHEN** a POST request is sent to `/store/carts/:id/line-items/:line_id` with body `{"quantity": 0}`
- **THEN** the system soft-deletes the line item and returns 200 with the cart, item removed from items array, totals recalculated

### Requirement: Store remove line item
The system SHALL provide `DELETE /store/carts/:id/line-items/:line_id` that soft-deletes the line item. Returns the full updated cart.

#### Scenario: Remove existing line item
- **WHEN** a DELETE request is sent to `/store/carts/:id/line-items/:line_id`
- **THEN** the system soft-deletes the line item and returns 200 with the cart, item removed, totals recalculated

### Requirement: Store complete cart to order
The system SHALL provide `POST /store/carts/:id/complete` that converts a cart to an order. This SHALL: validate the cart exists, is not already completed, and has at least one item; create an order with a `display_id` (auto-increment via MAX+1); copy line items from cart to order; create a payment record with status `"pending"`; mark the cart as completed. Returns `{"type": "order", "order": {...}}`.

#### Scenario: Complete cart with items
- **WHEN** a POST request is sent to `/store/carts/:id/complete` for a cart with items
- **THEN** the system returns 200 with `{"type": "order", "order": {...}}` containing the order with items, computed totals, payment record with status `"pending"`, and the cart is marked completed

#### Scenario: Complete empty cart
- **WHEN** a POST request is sent to `/store/carts/:id/complete` for a cart with no items
- **THEN** the system returns 409 with `{"type": "unexpected_state", "message": "..."}`

#### Scenario: Complete already-completed cart
- **WHEN** a POST request is sent to `/store/carts/:id/complete` for a cart where `completed_at IS NOT NULL`
- **THEN** the system returns 409 with `{"type": "unexpected_state", "message": "..."}`
