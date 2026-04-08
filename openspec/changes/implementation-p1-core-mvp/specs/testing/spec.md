## ADDED Requirements

### Requirement: Test-Driven Development process
All implementation SHALL follow TDD: tests are written before implementation. The spec scenarios (WHEN/THEN format in each module spec) serve as test contracts. Each scenario maps 1:1 to an integration test case.

#### Scenario: TDD cycle per feature
- **WHEN** a new feature is specified with WHEN/THEN scenarios
- **THEN** the developer writes integration tests matching each scenario first, verifies they fail (red), implements the feature, then verifies tests pass (green)

### Requirement: Coverage target
The project SHALL maintain >90% line coverage as measured by `cargo llvm-cov --summary-only`. Coverage is a hard gate — no phase is considered complete unless this threshold is met. The `main.rs` binary entry point (signal handlers, CLI arg parsing) is excluded from the coverage calculation as it contains no testable business logic.

#### Scenario: Coverage gate check
- **WHEN** `cargo llvm-cov --summary-only` is run after completing a phase
- **THEN** the TOTAL line coverage is >= 90%

### Requirement: Zero warnings and clippy clean
The project SHALL compile with zero warnings and pass `cargo clippy -- -D warnings` at all times. No phase is considered complete with warnings.

#### Scenario: Clean compilation check
- **WHEN** `cargo clippy -- -D warnings` is run
- **THEN** it exits with code 0 (no warnings or errors)

### Requirement: Test file structure
Integration tests SHALL be organized as `tests/{module}_test.rs` with shared infrastructure in `tests/common/mod.rs`. Unit tests SHALL be in `#[cfg(test)] mod tests` blocks within each source file (for error.rs, config.rs, db.rs, lib.rs, seed.rs, types.rs). Each test file SHALL cover all spec scenarios for its module.

#### Scenario: Test file per module
- **WHEN** a module spec defines N scenarios
- **THEN** the corresponding `tests/{module}_test.rs` contains at least N test functions, each matching a scenario

### Requirement: Contract tests verify Medusa response shapes
For each endpoint, integration tests SHALL verify that the response JSON matches Medusa's response shape. This includes: root wrapper key name (`"product"`, `"cart"`, `"order"`, `"customer"`), list wrapper key name and pagination fields (`"products"`, `"count"`, `"offset"`, `"limit"`), delete response shape (`"id"`, `"object"`, `"deleted"`), and cart complete response (`"type"`, `"order"`).

Reference files for contract verification:
- `vendor/medusa/packages/medusa/src/api/admin/products/route.ts` — admin product response patterns
- `vendor/medusa/packages/medusa/src/api/admin/products/helpers.ts` — response transformation
- `vendor/medusa/packages/medusa/src/api/store/products/route.ts` — store product response patterns
- `vendor/medusa/packages/medusa/src/api/store/carts/route.ts` — store cart response patterns

#### Scenario: Single entity response wrapper
- **WHEN** a GET request returns a single product
- **THEN** the response body uses the root wrapper `{"product": {id, title, handle, ...}}`

#### Scenario: List response with pagination
- **WHEN** a GET request returns a list
- **THEN** the response body uses `{"items": [...], "count": N, "offset": N, "limit": N}` (where `items` is the pluralized entity name)

#### Scenario: Delete response pattern
- **WHEN** a DELETE request soft-deletes an entity
- **THEN** the response body is `{"id": "...", "object": "entity_type", "deleted": true}`

### Requirement: Error contract tests
All error responses SHALL be verified to match the 3-field OAS Error schema from `specs/store.oas.yaml`: `code` (enum: `invalid_state_error`, `invalid_request_error`, `api_error`, `unknown_error`), `type` (enum: `not_found`, `invalid_data`, `duplicate_error`, `unauthorized`, `unexpected_state`, `database_error`), and `message` (string).

#### Scenario: Error response format validation
- **WHEN** any API error is returned
- **THEN** the response body contains exactly three fields: `code` (string, one of the allowed enum values), `type` (string, one of the allowed enum values), and `message` (string)

### Requirement: Unit tests for shared infrastructure
Shared infrastructure files SHALL have unit tests covering their core logic: `error.rs` (all AppError variants, status codes, error_type strings, IntoResponse output), `config.rs` (AppConfig loading from env vars), `db.rs` (create_db, run_migrations, ping), `types.rs` (generate_entity_id format, generate_handle slugification), `seed.rs` (run_seed returns Ok), `lib.rs` (health_check, build_app_state, app_router).

#### Scenario: Error variant coverage
- **WHEN** unit tests are run for error.rs
- **THEN** all 7 AppError variants are tested: NotFound, InvalidData, DuplicateError, Unauthorized, UnexpectedState, DatabaseError, MigrationError

### Requirement: Edge case test coverage
Integration tests SHALL cover edge cases beyond the happy path: not-found for all GET/POST/DELETE endpoints, validation failures (empty title, negative price, invalid email), duplicate constraint violations, soft-delete behavior (excluded from lists, 404 on store get), pagination boundary conditions (offset beyond results, limit=0), and completed cart mutation guard.

#### Scenario: Edge case test for each endpoint
- **WHEN** an endpoint has defined error scenarios in its spec
- **THEN** integration tests exist for each error scenario with the expected status code and error body
