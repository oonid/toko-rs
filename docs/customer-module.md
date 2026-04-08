# Phase 1-D: Customer Module

Completed 2026-04-08. All 8 tasks done (3.1–3.8).

## Endpoints

| Method | Path | Auth | Description |
|---|---|---|---|
| POST | `/store/customers` | — | Register new customer |
| GET | `/store/customers/me` | X-Customer-Id | Get current profile |
| POST | `/store/customers/me` | X-Customer-Id | Update current profile |

## Module Structure

```
src/customer/
  mod.rs         — pub mod declarations
  models.rs      — Customer struct (sqlx FromRow)
  types.rs       — CreateCustomerInput, UpdateCustomerInput, CustomerResponse
  repository.rs  — CustomerRepository (SqlitePool)
  routes.rs      — Router + auth_customer_id middleware
```

## Customer Model

```rust
pub struct Customer {
    pub id: String,           // cus_<ulid>
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: String,        // UNIQUE, NOT NULL
    pub phone: Option<String>,
    pub has_account: bool,    // TRUE on registration
    pub metadata: Option<Json<Value>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}
```

## X-Customer-Id Auth Middleware

`/me` endpoints use a layer-scoped Axum middleware (`axum::middleware::from_fn`):

1. Extracts `X-Customer-Id` header from request
2. Returns 401 `unauthorized` if missing
3. Injects `CustomerId` into request extensions
4. Handlers extract via `Extension<CustomerId>`

The middleware is applied only to the `/store/customers/me` route group, not to registration.

## Repository Methods

| Method | SQL | Notes |
|---|---|---|
| `create(input)` | INSERT with `has_account = TRUE` | UNIQUE violation → `DuplicateError` |
| `find_by_id(id)` | SELECT WHERE deleted_at IS NULL | 404 if not found |
| `update(id, input)` | COALESCE partial update | Refreshes updated_at |

## Response Shapes

Single customer (Medusa wrapper):
```json
{"customer": {"id": "cus_...", "email": "...", "has_account": true, ...}}
```

Error responses use 3-field OAS schema: `{"code": "...", "type": "...", "message": "..."}`.

## Tests

10 integration tests in `tests/customer_test.rs`:

| Test | Scenario | Status |
|---|---|---|
| `test_register_customer_success` | Register with all fields | 200 + customer wrapper |
| `test_register_customer_duplicate_email` | Same email twice | 409 duplicate_error |
| `test_register_customer_missing_email` | No email field | 422 (Axum JSON rejection) |
| `test_register_customer_invalid_email` | Bad email format | 400 invalid_data |
| `test_get_profile_with_valid_header` | Valid X-Customer-Id | 200 + customer |
| `test_get_profile_without_header` | No header | 401 unauthorized |
| `test_get_profile_not_found` | Nonexistent customer ID | 404 not_found |
| `test_update_customer_profile` | Update phone | 200, phone changed, name preserved |
| `test_update_customer_without_header` | No header | 401 unauthorized |
| `test_customer_response_format` | Verify JSON structure | All expected fields present |

## Quality

| Metric | Value |
|---|---|
| Tests | 51 total (10 customer) |
| Coverage (customer) | repository 95.83%, routes 98.33% |
| Clippy | Zero warnings |
