## ADDED Requirements

### Requirement: Admin export orders as CSV download
The system SHALL provide `GET /admin/orders/export` that returns all matching orders serialized as a downloadable CSV file. The response SHALL have `Content-Type: text/csv; charset=utf-8` and `Content-Disposition: attachment; filename="orders.csv"`. The first row of the CSV body SHALL be a fixed header row. The route SHALL be registered **before** `GET /admin/orders/:id` in the Axum router so the literal segment `export` is not matched as a path parameter.

This is a **P1 synchronous export** — the CSV is built in-memory during the HTTP request and streamed directly in the response body. No background job, file storage, or notification system is required. See design.md Decision 26.

**Medusa reference**: `vendor/medusa/packages/core/core-flows/src/order/steps/export-orders.ts`, `vendor/medusa/packages/core/core-flows/src/order/workflows/export-orders.ts`. Medusa's equivalent uses an async workflow (`backgroundExecution: true`) with file storage and an in-app notification. toko-rs P1 intentionally uses synchronous HTTP response delivery as a structurally simpler equivalent.

#### Scenario: Export all orders returns CSV response
- **WHEN** a GET request is sent to `/admin/orders/export` with no query parameters and at least one order exists
- **THEN** the system returns 200 with `Content-Type: text/csv; charset=utf-8`, `Content-Disposition: attachment; filename="orders.csv"`, and a CSV body whose first row is exactly `Order ID,Display ID,Email,Currency,Status,Fulfillment Status,Payment Status,Item Count,Total (cents),Created At,Shipped At,Canceled At`

#### Scenario: Each data row maps to one order
- **WHEN** a GET request is sent to `/admin/orders/export` and three orders exist in the database
- **THEN** the CSV body contains one header row and three data rows ordered by `created_at ASC`

#### Scenario: Empty order table returns headers-only CSV
- **WHEN** a GET request is sent to `/admin/orders/export` and no orders exist in the database
- **THEN** the system returns 200 with a CSV body containing only the header row (no 404, no empty body)

### Requirement: Export filter by order status
The system SHALL accept an optional `status` query parameter and include only orders whose `orders.status` matches the given value. Accepted values are `pending`, `completed`, and `canceled`. Any other value SHALL return 400.

#### Scenario: Filter export by status=completed
- **WHEN** a GET request is sent to `/admin/orders/export?status=completed` and orders with mixed statuses exist
- **THEN** the CSV body contains only rows where the Status column is `completed`

#### Scenario: Filter export by status=pending
- **WHEN** a GET request is sent to `/admin/orders/export?status=pending`
- **THEN** the CSV body contains only rows where the Status column is `pending`

#### Scenario: Invalid status value returns 400
- **WHEN** a GET request is sent to `/admin/orders/export?status=unknown`
- **THEN** the system returns 400 with `{"code": "invalid_request_error", "type": "invalid_data", "message": "..."}`

### Requirement: Export filter by date range
The system SHALL accept optional `created_at_from` and `created_at_to` query parameters as ISO 8601 datetime strings (e.g., `2025-01-01T00:00:00Z`) and include only orders whose `created_at` is within the range (bounds inclusive). Either bound may be omitted independently.

#### Scenario: Filter export by created_at_from only
- **WHEN** a GET request is sent to `/admin/orders/export?created_at_from=2025-06-01T00:00:00Z` and orders exist both before and after that timestamp
- **THEN** the CSV body contains only rows for orders created on or after `2025-06-01T00:00:00Z`

#### Scenario: Filter export by both created_at_from and created_at_to
- **WHEN** a GET request is sent to `/admin/orders/export?created_at_from=2025-01-01T00:00:00Z&created_at_to=2025-06-30T23:59:59Z`
- **THEN** the CSV body contains only rows for orders whose `created_at` falls within that range

### Requirement: Export filter by email search
The system SHALL accept an optional `q` query parameter and include only orders whose `email` column contains the search term (case-insensitive partial match via `ILIKE '%q%'` on PostgreSQL, `LIKE '%q%'` on SQLite). Orders with no email are excluded when `q` is provided.

#### Scenario: Filter export by email search term
- **WHEN** a GET request is sent to `/admin/orders/export?q=budi` and orders with emails `budi@example.com` and `other@example.com` exist
- **THEN** the CSV body contains only the row for `budi@example.com`

### Requirement: Export computed columns from related tables
The system SHALL resolve three derived columns from related tables using a single SQL query (LEFT JOIN, not N+1):

- **Payment Status**: resolved from `payment_records.status` using the same mapping as `OrderRepository::resolve_payment_status` — `pending` → `not_paid`, `authorized` → `authorized`, `captured` → `captured`, `failed` → `not_paid`, `refunded` → `refunded`, `canceled` → `canceled`, no record → `not_paid`
- **Item Count**: `COUNT(order_line_items.id)` across all line items for the order (soft-deleted items are excluded via `deleted_at IS NULL`)
- **Total (cents)**: `SUM(order_line_items.quantity * order_line_items.unit_price)` across all non-deleted line items; `0` when the order has no items

#### Scenario: Payment status resolved as captured
- **WHEN** a GET request is sent to `/admin/orders/export` and one order has a `payment_records` row with `status = 'captured'`
- **THEN** the Payment Status column for that order row is `captured`

#### Scenario: Payment status resolved as not_paid when no record exists
- **WHEN** a GET request is sent to `/admin/orders/export` and one order has no associated `payment_records` row
- **THEN** the Payment Status column for that order row is `not_paid`

#### Scenario: Item count and total computed from line items
- **WHEN** a GET request is sent to `/admin/orders/export` and one order has two line items (quantity 2 at unit_price 50000 and quantity 1 at unit_price 100000)
- **THEN** the Item Count column for that order row is `3` and the Total (cents) column is `200000`

### Requirement: Export row ordering
The system SHALL return CSV rows ordered by `orders.created_at ASC` (oldest-to-newest) to produce a consistent, chronological report regardless of database insertion order.

#### Scenario: Rows appear in chronological order
- **WHEN** a GET request is sent to `/admin/orders/export` and three orders were created at different timestamps
- **THEN** the CSV rows appear in ascending `Created At` order, oldest first
