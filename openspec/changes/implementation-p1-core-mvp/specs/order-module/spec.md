## ADDED Requirements

### Requirement: Store list customer orders
The system SHALL provide `GET /store/orders` that returns paginated orders for a customer identified by the `X-Customer-Id` header. Each order SHALL include its line items and computed totals. Default limit SHALL be 20.

#### Scenario: List orders for customer
- **WHEN** a GET request is sent to `/store/orders` with header `X-Customer-Id: cus_...`
- **THEN** the system returns 200 with `{"orders": [...], "count": N, "offset": 0, "limit": 20}` where each order has items, payment, and totals

#### Scenario: List orders without customer header
- **WHEN** a GET request is sent to `/store/orders` without `X-Customer-Id` header
- **THEN** the system returns 401 with `{"type": "unauthorized", "message": "..."}`

### Requirement: Store get order detail
The system SHALL provide `GET /store/orders/:id` that returns a single order with line items, computed totals, and the associated payment record.

#### Scenario: Get existing order
- **WHEN** a GET request is sent to `/store/orders/:id` for an existing order
- **THEN** the system returns 200 with `{"order": {...}}` containing items, `item_total`, `total`, and `payment` object with `id`, `status`, `amount`, `currency_code`, `provider`

#### Scenario: Get non-existent order
- **WHEN** a GET request is sent to `/store/orders/:id` with an ID that does not exist
- **THEN** the system returns 404 with `{"type": "not_found", "message": "..."}`

### Requirement: Order display_id auto-increment
The system SHALL assign `display_id` to each new order as `MAX(display_id) + 1` across all existing orders. The first order SHALL have `display_id: 1`.

#### Scenario: First order gets display_id 1
- **WHEN** the first order is created via cart completion
- **THEN** the order has `display_id: 1`

#### Scenario: Subsequent orders increment display_id
- **WHEN** a second order is created after the first (display_id: 1)
- **THEN** the second order has `display_id: 2`
