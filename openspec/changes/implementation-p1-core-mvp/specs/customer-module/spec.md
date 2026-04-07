## ADDED Requirements

### Requirement: Store register customer
The system SHALL provide `POST /store/customers` that registers a new customer. The `email` field is required and MUST be unique among non-deleted customers with `has_account = true`. The `has_account` field SHALL be set to `true` on registration.

#### Scenario: Register customer successfully
- **WHEN** a POST request is sent to `/store/customers` with body `{"first_name": "Budi", "last_name": "Santoso", "email": "budi@example.com", "phone": "+6281234567890"}`
- **THEN** the system returns 200 with `{"customer": {"id": "cus_...", "first_name": "Budi", "last_name": "Santoso", "email": "budi@example.com", "phone": "+6281234567890", "has_account": true, "metadata": null, "created_at": "...", "updated_at": "..."}}`

#### Scenario: Register with duplicate email
- **WHEN** a POST request is sent to `/store/customers` with an email already registered by another customer with `has_account = true`
- **THEN** the system returns 409 with `{"type": "duplicate_error", "message": "..."}`

#### Scenario: Register without email
- **WHEN** a POST request is sent to `/store/customers` with body `{"first_name": "Budi"}`
- **THEN** the system returns 400 with `{"type": "invalid_data", "message": "..."}`

### Requirement: Store get customer profile
The system SHALL provide `GET /store/customers/me` that returns the customer profile identified by the `X-Customer-Id` header.

#### Scenario: Get profile with valid header
- **WHEN** a GET request is sent to `/store/customers/me` with header `X-Customer-Id: cus_...`
- **THEN** the system returns 200 with `{"customer": {...}}`

#### Scenario: Get profile without header
- **WHEN** a GET request is sent to `/store/customers/me` without `X-Customer-Id` header
- **THEN** the system returns 401 with `{"type": "unauthorized", "message": "..."}`

### Requirement: Store update customer profile
The system SHALL provide `POST /store/customers/me` that updates first_name, last_name, phone, and/or metadata. The customer is identified by the `X-Customer-Id` header. The `updated_at` timestamp SHALL be refreshed.

#### Scenario: Update customer phone
- **WHEN** a POST request is sent to `/store/customers/me` with header `X-Customer-Id: cus_...` and body `{"phone": "+6289876543210"}`
- **THEN** the system returns 200 with `{"customer": {...}}` where phone is updated

#### Scenario: Update without header
- **WHEN** a POST request is sent to `/store/customers/me` without `X-Customer-Id` header
- **THEN** the system returns 401 with `{"type": "unauthorized", "message": "..."}`
