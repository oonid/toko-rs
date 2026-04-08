## ADDED Requirements

### Requirement: Environment configuration
The system SHALL load configuration from environment variables (with `.env` file support via dotenvy): `DATABASE_URL` (SQLite or PostgreSQL connection string), `HOST` (default `0.0.0.0`), `PORT` (default 3000), `RUST_LOG` (tracing filter, default `toko_rs=debug,tower_http=debug`).

#### Scenario: Load config from .env file
- **WHEN** the server starts with a `.env` file containing `DATABASE_URL=postgres://user:pass@localhost:5432/toko` and `PORT=3000`
- **THEN** the config is loaded with those values

### Requirement: Database connection pool
The system SHALL create an sqlx `PgPool` from the `DATABASE_URL` for production use. Pool config: min connections 2, max connections 10, idle timeout 5 minutes. The system SHALL support PostgreSQL as the primary database (matching Medusa's PostgreSQL dependency).

For local development and integration testing, the system SHALL also support SQLite via `SqlitePool` with a placeholder translation adapter (see Decision 2 in design.md).

#### Scenario: Connect to PostgreSQL
- **WHEN** `DATABASE_URL=postgres://user:pass@localhost:5432/toko`
- **THEN** the system creates a PgPool connected to the PostgreSQL database

#### Scenario: Connect to SQLite for testing
- **WHEN** `DATABASE_URL=sqlite::memory:`
- **THEN** the system creates a SqlitePool for in-memory testing with placeholder translation

### Requirement: Migration runner with dual DDL support
The system SHALL execute SQL migration files from the `migrations/` directory on startup. Migrations are written for PostgreSQL as primary (using `timestamptz`, `jsonb`, `$1` placeholders). SQLite-compatible migrations SHALL be maintained in a separate adapter path for in-memory testing.

#### Scenario: Run all migrations on fresh PostgreSQL
- **WHEN** the server starts against a new PostgreSQL database
- **THEN** all 12 tables + 1 pivot table + 1 sequence table + 1 idempotency table are created with PostgreSQL-native DDL (timestamptz, jsonb, BOOLEAN)

#### Scenario: Run all migrations on SQLite in-memory for tests
- **WHEN** the test infrastructure starts against `sqlite::memory:`
- **THEN** all 12 tables + 1 pivot + 1 sequence + 1 idempotency table are created with SQLite-compatible DDL (DATETIME, TEXT for JSON, INTEGER for BOOLEAN)

### Requirement: Health check endpoint
The system SHALL provide `GET /health` that returns `{"status": "ok"|"degraded", "database": "connected"|"disconnected", "version": "0.1.0"}`. The status is `"degraded"` if the database query `SELECT 1` fails.

#### Scenario: Healthy server
- **WHEN** a GET request is sent to `/health` and the database is connected
- **THEN** the system returns 200 with `{"status": "ok", "database": "connected", "version": "0.1.0"}`

### Requirement: Tracing initialization
The system SHALL initialize a tracing subscriber with three layers: HTTP request tracing (tower-http TraceLayer), application-level tracing (instrumented handlers), and SQL query tracing (sqlx built-in). The `RUST_LOG` environment variable controls verbosity.

#### Scenario: Request tracing
- **WHEN** a request is made to any endpoint
- **THEN** a tracing event is emitted with method, URI, status code, and latency

### Requirement: Seed data command
The system SHALL accept a `--seed` CLI flag that runs an idempotent seed function and exits. Seed data: 3-5 sample products (all published), 1 sample customer.

#### Scenario: Run seed command
- **WHEN** the server is started with `--seed` flag
- **THEN** sample products and customer are inserted (skipping if already exists), then the process exits

### Requirement: Graceful shutdown
The system SHALL handle SIGINT (Ctrl+C) to gracefully shut down the HTTP server, completing in-flight requests before exiting.

#### Scenario: Graceful shutdown on Ctrl+C
- **WHEN** SIGINT is received while the server is running
- **THEN** in-flight requests complete, then the server stops

### Requirement: CORS middleware
The system SHALL apply CORS middleware allowing all origins, methods, and headers in development mode.

#### Scenario: CORS headers present
- **WHEN** a preflight OPTIONS request is sent to any endpoint
- **THEN** the response includes appropriate CORS headers

### Requirement: Makefile commands
The project SHALL include a Makefile with targets: `dev` (cargo run), `test` (cargo test), `check` (cargo check), `lint` (cargo clippy -- -D warnings), `fmt` (cargo fmt), `seed` (cargo run -- --seed), `clean-db` (rm -f toko.db), `docker-up` (docker compose up -d), `docker-down` (docker compose down), `test-pg` (run tests against Docker PostgreSQL).

#### Scenario: Run make test
- **WHEN** `make test` is executed
- **THEN** cargo test runs all unit and integration tests

### Requirement: Medusa vendor reference and OpenAPI specs
The project SHALL maintain `vendor/medusa/` as a git submodule tracking the Medusa `develop` branch as the authoritative implementation reference. The `specs/` directory SHALL contain copies of the OpenAPI 3.0 base schemas from `vendor/medusa/www/utils/generated/oas-output/base/` (`store.oas.yaml` and `admin.oas.yaml`). These specs define the canonical Error schema, response patterns, and security schemes that toko-rs SHALL comply with.

#### Scenario: OpenAPI specs match vendor source
- **WHEN** `diff specs/store.oas.yaml vendor/medusa/www/utils/generated/oas-output/base/store.oas.base.yaml` is run
- **THEN** there is no diff (files are identical)

#### Scenario: Error response matches OAS Error schema
- **WHEN** any API error is returned
- **THEN** the response body matches the Error schema from specs/store.oas.yaml: includes `code` (enum: invalid_state_error, invalid_request_error, api_error, unknown_error), `type` (enum: not_found, invalid_data, duplicate_error, unauthorized, unexpected_state, database_error, etc.), and `message` (string)

### Requirement: Shared utility functions
The system SHALL provide reusable utility functions in `src/types.rs` that all modules SHALL use:
- `generate_entity_id(prefix: &str) -> String`: Generates `{prefix}_{lowercase_ulid}` for all entity IDs. All repositories SHALL use this instead of inline `format!()`.
- `generate_handle(title: &str) -> String`: Generates URL-safe handles via the `slug` crate. All handle auto-generation SHALL use this instead of inline string manipulation.

#### Scenario: Entity ID generation is centralized
- **WHEN** any module creates a new entity (product, variant, cart, order, customer, etc.)
- **THEN** the ID is generated via `types::generate_entity_id(prefix)` ensuring consistent format

#### Scenario: Handle generation uses slug crate
- **WHEN** a product is created without a handle
- **THEN** the handle is generated via `types::generate_handle(&title)` producing URL-safe output (handles unicode, special characters)

### Requirement: Module boundary rules
Each domain module (product, cart, order, customer, payment) SHALL be internally organized with consistent structure: `mod.rs`, `models.rs`, `repository.rs`, `routes.rs`, `types.rs`. Modules SHALL NOT import types, models, or repositories from other domain modules. Modules MAY import from shared infrastructure: `types.rs` (top-level), `error.rs`, `db.rs`. This mirrors Medusa's module isolation principle where modules are independent service packages that communicate through shared interfaces, not direct imports.

#### Scenario: Module imports are self-contained
- **WHEN** a module's code is reviewed for imports
- **THEN** it does not contain `use crate::product::*` from within the cart module (or any other cross-module import)

### Requirement: Medusa vendor reference mapping
Each module spec SHALL reference the specific Medusa source files that define the API contract for that module. This ensures implementers can verify response shapes, validation rules, and error handling against the authoritative source.

| Module | Medusa API routes | Medusa models | Medusa validators |
|---|---|---|---|
| Product | `vendor/medusa/packages/medusa/src/api/admin/products/route.ts` | `vendor/medusa/packages/modules/product/src/models/` | `vendor/medusa/packages/medusa/src/api/admin/products/validators.ts` |
| Cart | `vendor/medusa/packages/medusa/src/api/store/carts/route.ts` | `vendor/medusa/packages/modules/cart/src/models/` | `vendor/medusa/packages/medusa/src/api/store/carts/validators.ts` |
| Order | `vendor/medusa/packages/medusa/src/api/store/orders/route.ts` | `vendor/medusa/packages/modules/order/src/models/` | `vendor/medusa/packages/medusa/src/api/store/orders/validators.ts` |
| Customer | `vendor/medusa/packages/medusa/src/api/store/customers/route.ts` | `vendor/medusa/packages/modules/customer/src/models/` | `vendor/medusa/packages/medusa/src/api/store/customers/validators.ts` |

#### Scenario: Implementation matches Medusa reference
- **WHEN** a product endpoint response is compared to the Medusa route handler output
- **THEN** the JSON wrapper, field names, and pagination structure match

### Requirement: HTTP method conventions matching Medusa
Both create and update operations SHALL use POST (not PUT), matching Medusa's convention. DELETE is used for soft-delete operations. GET is used for retrieval and list operations.

#### Scenario: Update uses POST not PUT
- **WHEN** a product is updated via `POST /admin/products/:id`
- **THEN** the system processes it as a partial update, matching Medusa's `POST` pattern

The project SHALL include a `docker-compose.yml` with a PostgreSQL 16 service for integration testing. The Makefile SHALL include `docker-up` (start PG), `docker-down` (stop PG), and `test-pg` (run tests against Docker PostgreSQL) targets.

#### Scenario: Start PostgreSQL for testing
- **WHEN** `docker compose up -d` is executed
- **THEN** a PostgreSQL 16 container is running on port 5432 with an empty `toko` database

#### Scenario: Run tests against PostgreSQL
- **WHEN** `make test-pg` is executed with Docker PostgreSQL running
- **THEN** cargo test runs all integration tests against the PostgreSQL container using `DATABASE_URL=postgres://postgres:postgres@localhost:5432/toko_test`
