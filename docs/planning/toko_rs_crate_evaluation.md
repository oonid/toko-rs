# toko-rs — Crate & Library Evaluation

> **Goal**: Single binary, tokio-powered, sqlx with SQLite (dev) / PostgreSQL (prod)

---

## Architecture Decision: Single Crate with Modules

Since the MVP has 11 tables and 14 endpoints, a **single crate** with well-organized modules is simpler and more pragmatic than a full Cargo workspace:

```
toko-rs/
├── Cargo.toml              # Single manifest
├── .env                    # DATABASE_URL, etc.
├── migrations/             # sqlx migrations
└── src/
    ├── main.rs             # Entry point, server setup
    ├── config.rs           # Environment-based config
    ├── db.rs               # Database pool setup (Any)
    ├── error.rs            # AppError type, API error responses
    ├── types.rs            # Common types (Id, Pagination, Timestamps)
    │
    ├── product/
    │   ├── mod.rs
    │   ├── model.rs        # Product, Variant, Option structs
    │   ├── repo.rs         # SQL queries
    │   └── routes.rs       # GET /store/products, GET /store/products/:id
    │
    ├── cart/
    │   ├── mod.rs
    │   ├── model.rs        # Cart, LineItem structs
    │   ├── repo.rs
    │   └── routes.rs       # POST/GET carts, line item CRUD
    │
    ├── order/
    │   ├── mod.rs
    │   ├── model.rs        # Order, OrderLineItem structs
    │   ├── repo.rs
    │   └── routes.rs       # POST complete, GET orders
    │
    ├── customer/
    │   ├── mod.rs
    │   ├── model.rs        # Customer struct
    │   ├── repo.rs
    │   └── routes.rs       # POST register, GET/POST me
    │
    └── payment/
        ├── mod.rs
        ├── model.rs        # PaymentRecord struct
        └── repo.rs         # Internal, no routes in P1
```

> [!TIP]
> **Single crate ≠ monolith**. The module boundaries (`product/`, `cart/`, etc.) mirror the workspace crate layout. If you ever need to split into a workspace (e.g., for independent compilation or shared libraries), each `mod.rs` folder becomes its own crate with zero structural change.

---

## Dependency Stack

### Core Dependencies

| Crate | Version | Purpose | Why This One | Medusa Equivalent |
|---|---|---|---|---|
| **axum** | 0.8 | Web framework | Built by Tokio team, native tower integration, async-first | `express` |
| **tokio** | 1 | Async runtime | Industry standard, required by axum/sqlx | Node.js runtime |
| **sqlx** | 0.8 | Database access | Compile-time checked queries, SQLite + PostgreSQL via `AnyPool` | `@mikro-orm` + `knex` |
| **serde** | 1 | Serialization | De-facto standard for Rust serialization | built-in JSON |
| **serde_json** | 1 | JSON handling | For metadata fields, address snapshots, API responses | built-in JSON |

### Middleware & Observability

| Crate | Version | Purpose | Why This One | Medusa Equivalent |
|---|---|---|---|---|
| **tower-http** | 0.6 | HTTP middleware | CORS, request tracing, timeout — axum's companion | `cors` + `morgan` |
| **tower** | 0.5 | Service abstractions | Required by tower-http layers | Express middleware |
| **tracing** | 0.1 | Structured logging | Tokio ecosystem standard, async-aware | Winston logger |
| **tracing-subscriber** | 0.3 | Log output | Formats tracing output to stdout/JSON | Winston transports |


### Validation

| Crate | Version | Purpose | Why This One | Medusa Equivalent |
|---|---|---|---|---|
| **validator** | 0.19 | Request validation | Derive-based field validation with error messages | `zod` + `zod-validation-error` |

### Utilities

| Crate | Version | Purpose | Why This One | Medusa Equivalent |
|---|---|---|---|---|
| **ulid** | 1 | ID generation | Time-sortable, prefixed IDs like `prod_01JXXXX` | `ulid` |
| **slug** | 0.1 | Handle generation | URL-safe slugs: `"Classic T-Shirt"` → `"classic-t-shirt"` | `slugify` |
| **dotenvy** | 0.15 | .env file loading | Load `DATABASE_URL` etc. in dev | `dotenv` |
| **thiserror** | 2 | Error types | Derive `Error` for typed error handling | `MedusaError` class |
| **chrono** | 0.4 | Timestamps | `DateTime<Utc>` for created_at, updated_at — sqlx-compatible | built-in Date |

### Dev / Test Dependencies

| Crate | Version | Purpose | Medusa Equivalent |
|---|---|---|---|
| **reqwest** | 0.12 | HTTP client for integration tests | `supertest` |
| **serial_test** | 3 | Run DB tests sequentially (shared SQLite) | Jest `--bail --forceExit` |
| **wiremock** | 0.6 | Mock external HTTP services (P2: payment providers) | Jest mocks |
| **assert-json-diff** | 2 | Compare JSON responses for API contract testing | Jest `expect().toMatchObject()` |

### P2-Only Dependencies (add when needed)

| Crate | Version | Purpose | Medusa Equivalent |
|---|---|---|---|
| **jsonwebtoken** | 9 | JWT token creation/verification for customer auth | `jsonwebtoken` |
| **argon2** | 0.5 | Password hashing for customer accounts | Node.js crypto / scrypt |

---

## Cargo.toml

### MSRV Compatibility

| Crate | Required Rust |
|---|---|
| axum 0.8 | **1.78** |
| sqlx 0.8 | **1.78** |
| thiserror 2 | 1.68 |
| tokio 1 | 1.70 |
| tower-http 0.6 | 1.66 |
| **Effective MSRV** | **1.85** |

> [!TIP]
> We proactively bumped the MSRV to **1.85**. This allows native `async fn` inside `dyn Trait` instances without incurring the `async-trait` dependency cost (important for the repository pattern), and handles newer transitive dependencies gracefully. We maintain Edition 2021 for the project.

```toml
[package]
name = "toko-rs"
version = "0.1.0"
edition = "2021"
rust-version = "1.85"
license = "MIT"
description = "A modular, high-performance headless e-commerce backend inspired by MedusaJS"
repository = "https://github.com/oonid/toko-rs"
readme = "README.md"

[dependencies]
# Web
axum = "0.8"
tokio = { version = "1", features = ["full"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "trace"] }

# Database
sqlx = { version = "0.8", features = [
    "runtime-tokio",
    "tls-rustls",
    "any",          # AnyPool for SQLite/PostgreSQL portability
    "sqlite",
    "postgres",
    "chrono",       # DateTime support
    "json",         # JSON field support
] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Validation (Medusa: zod)
validator = { version = "0.19", features = ["derive"] }

# Utilities
ulid = "1"
slug = "0.1"                  # Handle generation (Medusa: slugify)
dotenvy = "0.15"
thiserror = "2"
chrono = { version = "0.4", features = ["serde"] }

# Observability (Medusa: winston + morgan)
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[dev-dependencies]
reqwest = { version = "0.12", features = ["json"] }
serial_test = "3"             # Sequential DB tests
wiremock = "0.6"              # Mock external services (P2)
assert-json-diff = "2"        # API contract assertions
```

---

## SQLite ↔ PostgreSQL Strategy

Using `sqlx::AnyPool` — the database is selected at **runtime** by the connection URL:

```rust
// src/db.rs
use sqlx::AnyPool;

pub async fn create_pool(database_url: &str) -> Result<AnyPool, sqlx::Error> {
    // Install drivers at startup
    sqlx::any::install_default_drivers();

    let pool = AnyPool::connect(database_url).await?;
    Ok(pool)
}
```

```bash
# .env (development)
DATABASE_URL=sqlite://toko.db?mode=rwc

# .env (production)
DATABASE_URL=postgresql://user:pass@localhost:5432/toko
```

### SQL Compatibility Rules

To keep queries portable between SQLite and PostgreSQL:

| Feature | Use | Avoid |
|---|---|---|
| Placeholders | `$1, $2, $3` (works in both via Any) | `?` (SQLite-only) |
| Text types | `TEXT` | `VARCHAR(n)` |
| Boolean | `INTEGER` (0/1) in SQLite, `BOOLEAN` in PG | Use `$1::bool` casting |
| JSON | `TEXT` (store as string) | `JSONB` (PG-only) |
| Auto-increment | Handled in application | `SERIAL` / `AUTOINCREMENT` |
| Timestamps | `TEXT` ISO-8601 (SQLite) / `TIMESTAMPTZ` (PG) | Handled by chrono |

> [!IMPORTANT]
> **Migration files**: You'll need separate migration directories for SQLite and PostgreSQL if the DDL differs significantly. For MVP, write SQLite-compatible SQL and convert when deploying to PG. Alternatively, use a migration runner that handles dialect differences.

---

## API Interaction Examples (curl)

### Starting the server

```bash
# Development (SQLite)
DATABASE_URL=sqlite://toko.db?mode=rwc cargo run

# Server starts on http://localhost:3000
```

### Product Browsing

```bash
# List all published products
curl http://localhost:3000/store/products

# Response:
{
  "products": [
    {
      "id": "prod_01JX7KZMN0QQVR9E2BT300001",
      "title": "Classic T-Shirt",
      "handle": "classic-t-shirt",
      "status": "published",
      "thumbnail": null,
      "options": [
        {
          "id": "opt_01JX7KZMN0QQVR9E2BT300002",
          "title": "Size",
          "values": [
            {"id": "optval_01JX...", "value": "S"},
            {"id": "optval_02JX...", "value": "M"},
            {"id": "optval_03JX...", "value": "L"}
          ]
        }
      ],
      "variants": [
        {
          "id": "variant_01JX7KZMN0QQVR9E2BT300003",
          "title": "Small",
          "sku": "TS-S",
          "price": 2500,
          "options": [{"id": "optval_01JX...", "value": "S"}]
        }
      ]
    }
  ],
  "count": 1,
  "offset": 0,
  "limit": 20
}

# Get single product with full details
curl http://localhost:3000/store/products/prod_01JX7KZMN0QQVR9E2BT300001
```

### Cart Flow

```bash
# 1. Create a cart
curl -X POST http://localhost:3000/store/carts \
  -H "Content-Type: application/json" \
  -d '{"currency_code": "usd"}'

# Response:
{
  "cart": {
    "id": "cart_01JX7M2VN0QQVR9E2BT300004",
    "currency_code": "usd",
    "items": [],
    "total": 0
  }
}

# 2. Add item to cart
curl -X POST http://localhost:3000/store/carts/cart_01JX.../line-items \
  -H "Content-Type: application/json" \
  -d '{
    "variant_id": "variant_01JX7KZMN0QQVR9E2BT300003",
    "quantity": 2
  }'

# Response:
{
  "cart": {
    "id": "cart_01JX...",
    "items": [
      {
        "id": "cali_01JX...",
        "title": "Classic T-Shirt",
        "variant_id": "variant_01JX...",
        "quantity": 2,
        "unit_price": 2500,
        "total": 5000
      }
    ],
    "total": 5000
  }
}

# 3. Update line item quantity
curl -X POST http://localhost:3000/store/carts/cart_01JX.../line-items/cali_01JX... \
  -H "Content-Type: application/json" \
  -d '{"quantity": 3}'

# 4. Remove line item
curl -X DELETE http://localhost:3000/store/carts/cart_01JX.../line-items/cali_01JX...

# 5. Update cart (set email for guest checkout)
curl -X POST http://localhost:3000/store/carts/cart_01JX... \
  -H "Content-Type: application/json" \
  -d '{"email": "buyer@example.com"}'

# 6. Get cart (check current state)
curl http://localhost:3000/store/carts/cart_01JX...
```

### Checkout

```bash
# Complete cart → creates Order + PaymentRecord
curl -X POST http://localhost:3000/store/carts/cart_01JX.../complete

# Response:
{
  "type": "order",
  "order": {
    "id": "order_01JX7P9KN0QQVR9E2BT300005",
    "display_id": 1,
    "status": "pending",
    "currency_code": "usd",
    "email": "buyer@example.com",
    "items": [
      {
        "id": "oli_01JX...",
        "title": "Classic T-Shirt",
        "quantity": 2,
        "unit_price": 2500,
        "total": 5000
      }
    ],
    "total": 5000,
    "payment": {
      "id": "pay_01JX...",
      "status": "pending",
      "amount": 5000
    }
  }
}
```

### Orders

```bash
# List customer's orders (requires customer_id context)
curl http://localhost:3000/store/orders

# Get specific order
curl http://localhost:3000/store/orders/order_01JX7P9KN0QQVR9E2BT300005
```

### Customer

```bash
# Register a customer
curl -X POST http://localhost:3000/store/customers \
  -H "Content-Type: application/json" \
  -d '{
    "first_name": "Budi",
    "last_name": "Santoso",
    "email": "budi@example.com"
  }'

# Get customer profile
curl http://localhost:3000/store/customers/me \
  -H "X-Customer-Id: cus_01JX..."

# Update customer profile
curl -X POST http://localhost:3000/store/customers/me \
  -H "X-Customer-Id: cus_01JX..." \
  -H "Content-Type: application/json" \
  -d '{"phone": "+6281234567890"}'
```

---

## Architectural Patterns — Aligned with Medusa Source Code

Each pattern below is cross-referenced with the actual Medusa implementation.

---

### 1. ID Generation — `prefix_ULID`

**Medusa source**: [`packages/core/utils/src/common/generate-entity-id.ts`](https://github.com/medusajs/medusa/blob/develop/packages/core/utils/src/common/generate-entity-id.ts)

```typescript
// Medusa's actual implementation:
import { ulid } from "ulid"
export function generateEntityId(idProperty?: string, prefix?: string): string {
  if (idProperty) { return idProperty }  // allow user-provided IDs
  const id = ulid()
  prefix = prefix ? `${prefix}_` : ""
  return `${prefix}${id}`
}
```

**toko-rs equivalent**:

```rust
use ulid::Ulid;

/// Generate a prefixed entity ID, matching Medusa's pattern.
/// Allows caller to optionally provide their own ID.
pub fn generate_entity_id(existing_id: Option<&str>, prefix: &str) -> String {
    match existing_id {
        Some(id) => id.to_string(),
        None => format!("{}_{}", prefix, Ulid::new()),
    }
}

// Medusa's actual prefixes (from model definitions):
// Product: "prod", ProductVariant: "variant", ProductOption: "opt"
// Cart: "cart", CartLineItem: "cali", Customer: "cus"
// CustomerAddress: "cuaddr", Order: "order", PaymentCollection: "pay_col"
```

---

### 2. Error Types — `MedusaError`

**Medusa source**: [`packages/core/utils/src/common/errors.ts`](https://github.com/medusajs/medusa/blob/develop/packages/core/utils/src/common/errors.ts)

Medusa defines a standardized error with **type**, **message**, and optional **code**:

```typescript
// Medusa's actual error types:
export const MedusaErrorTypes = {
  DB_ERROR:            "database_error",
  DUPLICATE_ERROR:     "duplicate_error",
  INVALID_ARGUMENT:    "invalid_argument",
  INVALID_DATA:        "invalid_data",
  UNAUTHORIZED:        "unauthorized",
  FORBIDDEN:           "forbidden",
  NOT_FOUND:           "not_found",
  NOT_ALLOWED:         "not_allowed",
  UNEXPECTED_STATE:    "unexpected_state",
  CONFLICT:            "conflict",
}
```

**toko-rs equivalent**:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("not_found: {0}")]
    NotFound(String),

    #[error("invalid_data: {0}")]
    InvalidData(String),

    #[error("duplicate_error: {0}")]
    DuplicateError(String),

    #[error("unauthorized: {0}")]
    Unauthorized(String),

    #[error("database_error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("unexpected_state: {0}")]
    UnexpectedState(String),
}

// Serializes to Medusa-compatible JSON:
// {"type": "not_found", "message": "Product with id prod_xxx not found"}
// {"type": "invalid_data", "message": "quantity must be greater than 0"}

impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_type, message) = match &self {
            AppError::NotFound(msg) =>        (StatusCode::NOT_FOUND, "not_found", msg),
            AppError::InvalidData(msg) =>     (StatusCode::BAD_REQUEST, "invalid_data", msg),
            AppError::DuplicateError(msg) =>  (StatusCode::CONFLICT, "duplicate_error", msg),
            AppError::Unauthorized(msg) =>    (StatusCode::UNAUTHORIZED, "unauthorized", msg),
            AppError::DatabaseError(_) =>     (StatusCode::INTERNAL_SERVER_ERROR, "database_error", &"Internal server error".to_string()),
            AppError::UnexpectedState(msg) => (StatusCode::CONFLICT, "unexpected_state", msg),
        };
        (status, Json(json!({"type": error_type, "message": message}))).into_response()
    }
}
```

---

### 3. Pagination & Query — `createFindParams` pattern

**Medusa source**: [`packages/medusa/src/api/utils/validators.ts`](https://github.com/medusajs/medusa/blob/develop/packages/medusa/src/api/utils/validators.ts)

Medusa's `createFindParams` standardizes all list queries with:
- `offset` (default: 0)
- `limit` (default: 20)
- `order` (sort field)
- `fields` (sparse fieldset selector)
- `with_deleted` (include soft-deleted records)

```typescript
// Medusa's actual createFindParams:
export const createFindParams = ({ offset, limit, order }) => {
  return selectParams.merge(z.object({
    offset: z.number().optional().default(offset ?? 0),
    limit:  z.number().optional().default(limit ?? 20),
    order:  z.string().optional(),
    with_deleted: z.boolean().optional(),
  }))
}
```

**toko-rs equivalent**:

```rust
#[derive(Debug, Deserialize)]
pub struct FindParams {
    #[serde(default)]
    pub offset: i64,                        // default: 0
    #[serde(default = "default_limit")]
    pub limit: i64,                         // default: 20
    pub order: Option<String>,              // e.g. "-created_at" for DESC
    pub fields: Option<String>,             // sparse fieldset (future)
    #[serde(default)]
    pub with_deleted: bool,                 // include soft-deleted
}

fn default_limit() -> i64 { 20 }

// List response matches Medusa's format exactly:
#[derive(Debug, Serialize)]
pub struct ListResponse<T: Serialize> {
    // The key varies by entity: "products", "carts", "orders", etc.
    // This is handled at the route level, not here.
    pub count: i64,
    pub offset: i64,
    pub limit: i64,
}
```

**Medusa response format** (from `route.ts`):
```json
{
  "products": [...],
  "count": 42,
  "offset": 0,
  "limit": 20
}
```

> [!NOTE]
> Medusa uses `count`/`offset`/`limit` — NOT `total`/`page`/`per_page`. The product list default limit is **50** (see query-config.ts), while the general default is **20**.

---

### 4. Query Config — Per-route field defaults

**Medusa source**: [`packages/medusa/src/api/store/products/query-config.ts`](https://github.com/medusajs/medusa/blob/develop/packages/medusa/src/api/store/products/query-config.ts)

Each route defines which fields to include by default and the default limit:

```typescript
// Medusa's actual query config for products:
export const defaultStoreProductFields = [
  "id", "title", "subtitle", "description", "handle",
  "is_giftcard", "discountable", "thumbnail",
  "collection_id", "type_id", "weight", "length", "height", "width",
  "hs_code", "origin_country", "mid_code", "material",
  "created_at", "updated_at",
  "*type", "*collection", "*options", "*options.values",
  "*tags", "*images", "*variants", "*variants.options",
]

export const listProductQueryConfig = {
  defaults: defaultStoreProductFields,
  defaultLimit: 50,
  isList: true,
}
```

**toko-rs approach**: For MVP, we return all fields always (no sparse fieldset). But the per-module `query_config` pattern should be kept as a constant for future compatibility.

---

### 5. Soft Delete — `deleted_at` with `with_deleted` flag

**Medusa pattern**: All models have `deleted_at`. The `with_deleted` query parameter controls whether to include soft-deleted records. This is set in the `createFindParams` validator (see above).

**toko-rs implementation**:

```rust
// In every repo query:
pub async fn list(pool: &AnyPool, params: &FindParams) -> Result<(Vec<Product>, i64)> {
    let deleted_clause = if params.with_deleted {
        ""  // no filter
    } else {
        "AND deleted_at IS NULL"
    };

    let sql = format!(
        "SELECT * FROM products WHERE 1=1 {} ORDER BY created_at DESC LIMIT $1 OFFSET $2",
        deleted_clause
    );
    // ...
}

// Soft delete (never hard delete):
pub async fn delete(pool: &AnyPool, id: &str) -> Result<()> {
    sqlx::query("UPDATE products SET deleted_at = CURRENT_TIMESTAMP WHERE id = $1 AND deleted_at IS NULL")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
```

---

### 6. Route File Structure — Medusa's per-endpoint organization

**Medusa source**: `packages/medusa/src/api/store/products/`

Each route group in Medusa has a consistent file structure:

```
store/products/
├── route.ts          # GET handler (list)
├── [id]/
│   └── route.ts      # GET handler (single)
├── query-config.ts   # Default fields, limits
├── validators.ts     # Request validation (zod schemas)
├── middlewares.ts     # Route-specific middleware
└── helpers.ts        # Shared logic
```

**toko-rs equivalent** (per module):

```
src/product/
├── mod.rs            # Module exports
├── model.rs          # Structs (Product, Variant, etc.)
├── repo.rs           # SQL queries (replaces Medusa's query/service layer)
├── routes.rs         # Axum handlers (replaces route.ts)
├── params.rs         # Request validation structs (replaces validators.ts)
└── config.rs         # Default fields, limits (replaces query-config.ts)
```

---

### 7. Tracing — Core Infrastructure

**Medusa approach**: Uses Winston logger with levels (`info`, `warn`, `error`, `debug`, `activity`, `progress`). Logger is registered in the DI container and injected everywhere.

**toko-rs approach**: Uses `tracing` — the Rust ecosystem standard for structured, async-aware observability. This is a **core architectural pillar**, not an afterthought.

```rust
// src/main.rs — Initialize tracing FIRST
use tracing_subscriber::{fmt, EnvFilter};

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("toko_rs=debug,tower_http=debug"))
        )
        .with_target(true)           // show module path
        .with_thread_ids(false)
        .with_file(true)             // show source file
        .with_line_number(true)      // show line number
        .init();
}
```

**Three layers of tracing**:

```rust
// Layer 1: HTTP request tracing (tower-http)
// Automatic: method, uri, status, latency
let app = Router::new()
    .merge(product_routes())
    .merge(cart_routes())
    .layer(TraceLayer::new_for_http());

// Layer 2: Application-level tracing (in handlers)
#[tracing::instrument(skip(pool))]
pub async fn get_product(
    State(pool): State<AnyPool>,
    Path(id): Path<String>,
) -> Result<Json<ProductResponse>, AppError> {
    tracing::info!(product_id = %id, "Fetching product");
    let product = product::repo::find_by_id(&pool, &id).await?;
    // ...
}

// Layer 3: Database query tracing (sqlx built-in)
// sqlx emits tracing events for every query when tracing feature is enabled
// Shows: query text, bind parameters, execution time
```

**Output example**:
```
2026-04-06T00:45:00.123Z DEBUG tower_http::trace: ← GET /store/products 200 OK 3.2ms
2026-04-06T00:45:00.456Z  INFO toko_rs::product::routes: Fetching product product_id=prod_01JX...
2026-04-06T00:45:00.458Z DEBUG sqlx::query: SELECT * FROM products WHERE id = $1 … rows=1 elapsed=1.2ms
```

**Control via environment**:
```bash
# Show everything
RUST_LOG=toko_rs=trace,tower_http=trace,sqlx=trace cargo run

# Production: only warnings and above
RUST_LOG=toko_rs=info,tower_http=warn cargo run

# Debug specific module
RUST_LOG=toko_rs::cart=debug cargo run
```

---

### 8. Health Check

```rust
// GET /health — standard operational endpoint
async fn health_check(State(pool): State<AnyPool>) -> Json<serde_json::Value> {
    let db_ok = sqlx::query("SELECT 1").execute(&pool).await.is_ok();
    Json(json!({
        "status": if db_ok { "ok" } else { "degraded" },
        "database": if db_ok { "connected" } else { "disconnected" },
        "version": env!("CARGO_PKG_VERSION"),
    }))
}
```

---

### 9. Graceful Shutdown

```rust
// Medusa handles this via Node.js process signals.
// toko-rs does it via tokio signal handling:
async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
    tracing::info!("Shutdown signal received, starting graceful shutdown");
}

// In main:
axum::serve(listener, app)
    .with_graceful_shutdown(shutdown_signal())
    .await?;
```

---

### 10. Seed Data

```rust
// src/main.rs — check CLI args
let args: Vec<String> = std::env::args().collect();
if args.contains(&"--seed".to_string()) {
    tracing::info!("Seeding database...");
    seed::run(&pool).await?;
    tracing::info!("Seeding complete");
    return Ok(());
}
```

---

### 11. Timestamps — Application-managed

Medusa uses the DLL framework to auto-manage `created_at`, `updated_at`, `deleted_at`. In toko-rs, this is handled explicitly:

```rust
// On INSERT: created_at and updated_at set by DEFAULT CURRENT_TIMESTAMP in SQL
// On UPDATE: always set updated_at explicitly
sqlx::query(
    "UPDATE products SET title = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2"
)
```

---

## Pattern Summary — Medusa → toko-rs Mapping

| Pattern | Medusa (TypeScript) | toko-rs (Rust) | Source File |
|---|---|---|---|
| ID generation | `ulid()` with prefix | `ulid` crate + `format!` | `generate-entity-id.ts` |
| Error types | `MedusaError` class with `.type` | `AppError` enum with `thiserror` | `errors.ts` |
| Pagination | `createFindParams` (offset/limit/order/with_deleted) | `FindParams` struct | `validators.ts` |
| Response format | `{products: [...], count, offset, limit}` | Same JSON shape | `route.ts` |
| Soft delete | `deleted_at` + `with_deleted` param | Same pattern | `validators.ts` |
| Query config | `defaultStoreProductFields` + `defaultLimit` | Per-module `config.rs` | `query-config.ts` |
| Validation | Zod schemas (`StoreGetProductsParams`) | Serde `Deserialize` + custom validation | `validators.ts` |
| Logging | Winston (info/warn/error/debug) | `tracing` (trace/debug/info/warn/error) | `logger/index.ts` |
| Route structure | route.ts + validators.ts + query-config.ts + helpers.ts | routes.rs + params.rs + config.rs + mod.rs | `api/store/products/` |

---

## Crate Count Summary

| Category | Crates | Count |
|---|---|---|
| Web | axum, tokio, tower, tower-http | 4 |
| Database | sqlx | 1 |
| Serialization | serde, serde_json | 2 |
| Validation | validator | 1 |
| Observability | tracing, tracing-subscriber | 2 |
| Utilities | ulid, slug, dotenvy, thiserror, chrono | 5 |
| **Total runtime** | | **15** |
| | | |
| **Dev deps** | reqwest, serial_test, wiremock, assert-json-diff | **4** |
| **P2-only** *(add later)* | jsonwebtoken, argon2 | **2** |

> [!NOTE]
> **15 runtime dependencies** — every one cross-referenced against Medusa's actual `package.json` files. Each has a direct Medusa equivalent (`slugify` → `slug`, `zod` → `validator`, `winston` → `tracing`, etc.). No unnecessary crates.
