## ADDED Requirements

### Requirement: Medusa-compatible error response format
All API errors SHALL return a JSON response matching the Error schema defined in `specs/store.oas.yaml` and `specs/admin.oas.yaml` (sourced from `vendor/medusa/www/utils/generated/oas-output/base/`). The response SHALL contain three fields: `code` (error category slug), `type` (specific error type slug), and `message` (human-readable description).

The `code` field SHALL be one of: `invalid_state_error`, `invalid_request_error`, `api_error`, `unknown_error`.
The `type` field SHALL be one of: `not_found`, `invalid_data`, `duplicate_error`, `unauthorized`, `unexpected_state`, `database_error`, `unknown_error`.

| HTTP Status | `code` | `type` | When |
|---|---|---|---|
| 400 | `invalid_request_error` | `invalid_data` | Missing required field, validation failure |
| 401 | `unknown_error` | `unauthorized` | Missing X-Customer-Id header on protected endpoint |
| 404 | `invalid_request_error` | `not_found` | Entity does not exist |
| 409 | `invalid_request_error` | `duplicate_error` | Unique constraint (handle, SKU, email) |
| 409 | `invalid_state_error` | `unexpected_state` | Cart already completed, empty cart completion |
| 500 | `api_error` | `database_error` | Internal DB error (message sanitized) |

#### Scenario: NotFound error
- **WHEN** a requested entity does not exist
- **THEN** the system returns HTTP 404 with `{"code": "invalid_request_error", "type": "not_found", "message": "Product with id prod_xxx was not found"}`

#### Scenario: InvalidData error
- **WHEN** a request fails validation (missing required field, invalid value)
- **THEN** the system returns HTTP 400 with `{"code": "invalid_request_error", "type": "invalid_data", "message": "..."}`

#### Scenario: DuplicateError error
- **WHEN** a unique constraint is violated (duplicate handle, SKU, or email)
- **THEN** the system returns HTTP 409 with `{"code": "invalid_request_error", "type": "duplicate_error", "message": "..."}`

#### Scenario: Unauthorized error
- **WHEN** a protected endpoint is accessed without required authentication (X-Customer-Id header)
- **THEN** the system returns HTTP 401 with `{"code": "unknown_error", "type": "unauthorized", "message": "..."}`

#### Scenario: UnexpectedState error
- **WHEN** an invalid state transition is attempted (complete already-completed cart, mutate completed cart)
- **THEN** the system returns HTTP 409 with `{"code": "invalid_state_error", "type": "unexpected_state", "message": "..."}`

#### Scenario: DatabaseError error
- **WHEN** an internal database error occurs
- **THEN** the system returns HTTP 500 with `{"code": "api_error", "type": "database_error", "message": "Internal server error"}` (message sanitized, not exposing internals)

### Requirement: Medusa-compatible response wrappers
Single entity responses SHALL use the root wrapper pattern: `{"product": {...}}`, `{"cart": {...}}`, `{"order": {...}}`, `{"customer": {...}}`. List responses SHALL use: `{"products": [...], "count": N, "offset": N, "limit": N}`. Delete responses SHALL use: `{"id": "...", "object": "product", "deleted": true}`. Cart complete SHALL use: `{"type": "order", "order": {...}}`.

#### Scenario: Single entity response
- **WHEN** a GET request returns a single product
- **THEN** the response body is `{"product": {id, title, handle, ...}}`

#### Scenario: List response with pagination
- **WHEN** a GET request returns a list of products
- **THEN** the response body is `{"products": [...], "count": 42, "offset": 0, "limit": 50}`

#### Scenario: Delete response
- **WHEN** a DELETE request soft-deletes a product
- **THEN** the response body is `{"id": "prod_...", "object": "product", "deleted": true}`

#### Scenario: Cart complete response
- **WHEN** a cart is completed successfully
- **THEN** the response body is `{"type": "order", "order": {id, display_id, status, items, payment, ...}}`
