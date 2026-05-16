# Spec: toko-rs Modifications for sapa-rs P1

> **Context reference**: sapa-rs context doc §18  
> **toko-rs source**: vendor/toko-rs/ (inspected 2026-05-15)  
> **Status**: Ready for implementation — reviewed 2026-05-15  
> **Implement in**: toko-rs repository (parallel task)  
> **Target model**: claude-haiku-4-5

These changes must be complete and deployed before `store-client` integration tests can run.

---

## How to use this spec

Read each modification fully before writing any code. Each section contains:
- **Current state** — exact existing code to understand context
- **Step-by-step changes** — line-level instructions with exact code to write
- **Acceptance criteria** — what to test

After all 4 modifications are done, run `cargo test` and `cargo clippy -- -D warnings`. Fix any errors before finishing.

---

## Modification 1 — Phone unique constraint

### Current state

File `migrations/002_customers.sql` creates a **non-unique** index on phone:
```sql
CREATE INDEX idx_customers_phone ON customers (phone)
WHERE deleted_at IS NULL AND phone IS NOT NULL;
```
Multiple customers can share the same phone number.

The project currently has migrations `001` through `006`. The next migration number is **`007`**.

### Step-by-step changes

#### Step 1a — Create PostgreSQL migration

Create new file **`migrations/007_customers_phone_unique.sql`** with this exact content:

```sql
-- Soft-delete duplicate phone records before adding constraint (keep earliest created_at).
-- Hard DELETE is avoided because duplicates may be referenced by orders/carts via FK.
UPDATE customers
SET deleted_at = now()
WHERE id NOT IN (
    SELECT DISTINCT ON (phone) id
    FROM customers
    WHERE phone IS NOT NULL AND deleted_at IS NULL
    ORDER BY phone, created_at ASC
)
AND phone IS NOT NULL
AND deleted_at IS NULL;

-- Add unique partial index (soft-deleted rows excluded by WHERE clause).
CREATE UNIQUE INDEX uq_customers_phone
ON customers (phone)
WHERE deleted_at IS NULL AND phone IS NOT NULL;
```

#### Step 1b — Create SQLite migration

Create new file **`migrations/sqlite/007_customers_phone_unique.sql`** with this exact content:

```sql
-- Soft-delete duplicate phone records before adding constraint (keep earliest created_at).
-- Uses correlated subquery instead of DISTINCT ON (PostgreSQL syntax not supported in SQLite).
UPDATE customers
SET deleted_at = CURRENT_TIMESTAMP
WHERE id NOT IN (
    SELECT c2.id FROM customers c2
    WHERE c2.phone = customers.phone
      AND c2.phone IS NOT NULL
      AND c2.deleted_at IS NULL
    ORDER BY c2.created_at ASC
    LIMIT 1
)
AND phone IS NOT NULL
AND deleted_at IS NULL;

-- Add unique partial index (SQLite supports partial indexes since 3.8.9).
CREATE UNIQUE INDEX uq_customers_phone
ON customers (phone)
WHERE deleted_at IS NULL AND phone IS NOT NULL;
```

#### Step 1c — Handle phone unique violation in `create`

File: **`src/customer/repository.rs`**, method `create`.

The current `.map_err` block on the INSERT query is:
```rust
        .map_err(|e| {
            if crate::db::is_unique_violation(&e) {
                return AppError::DuplicateError(format!(
                    "Customer with email '{}' already exists",
                    input.email.as_deref().unwrap_or("(none)")
                ));
            }
            AppError::DatabaseError(e)
        })?;
```

Replace it with a version that checks the constraint name to distinguish email vs phone violations:
```rust
        .map_err(|e| {
            if let sqlx::Error::Database(ref db_err) = e {
                if db_err.code().as_deref() == Some(crate::db::unique_violation_code()) {
                    if db_err.constraint() == Some("uq_customers_phone") {
                        return AppError::DuplicateError(format!(
                            "Customer with phone '{}' already exists",
                            input.phone.as_deref().unwrap_or("(none)")
                        ));
                    }
                    return AppError::DuplicateError(format!(
                        "Customer with email '{}' already exists",
                        input.email.as_deref().unwrap_or("(none)")
                    ));
                }
            }
            AppError::DatabaseError(e)
        })?;
```

#### Step 1d — Handle phone unique violation in `update`

File: **`src/customer/repository.rs`**, method `update`.

The current `sqlx::query(...)` for UPDATE does not handle unique violations. Find this block:
```rust
        sqlx::query(
            r#"
            UPDATE customers SET
                first_name = COALESCE($1, first_name),
                last_name = COALESCE($2, last_name),
                email = COALESCE($3, email),
                phone = COALESCE($4, phone),
                company_name = COALESCE($5, company_name),
                metadata = COALESCE($6, metadata),
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $7 AND deleted_at IS NULL
            "#,
        )
        .bind(&input.first_name)
        .bind(&input.last_name)
        .bind(&input.email)
        .bind(&input.phone)
        .bind(&input.company_name)
        .bind(metadata_to_json(input.metadata.clone()))
        .bind(id)
        .execute(&self.pool)
        .await?;
```

Replace `.await?;` with `.await.map_err(|e| {` + a constraint check:
```rust
        sqlx::query(
            r#"
            UPDATE customers SET
                first_name = COALESCE($1, first_name),
                last_name = COALESCE($2, last_name),
                email = COALESCE($3, email),
                phone = COALESCE($4, phone),
                company_name = COALESCE($5, company_name),
                metadata = COALESCE($6, metadata),
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $7 AND deleted_at IS NULL
            "#,
        )
        .bind(&input.first_name)
        .bind(&input.last_name)
        .bind(&input.email)
        .bind(&input.phone)
        .bind(&input.company_name)
        .bind(metadata_to_json(input.metadata.clone()))
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(ref db_err) = e {
                if db_err.code().as_deref() == Some(crate::db::unique_violation_code()) {
                    if db_err.constraint() == Some("uq_customers_phone") {
                        return AppError::DuplicateError(format!(
                            "Customer with phone '{}' already exists",
                            input.phone.as_deref().unwrap_or("(none)")
                        ));
                    }
                    return AppError::DuplicateError(format!(
                        "Customer with email '{}' already exists",
                        input.email.as_deref().unwrap_or("(none)")
                    ));
                }
            }
            AppError::DatabaseError(e)
        })?;
```

### HTTP response for duplicate phone

`AppError::DuplicateError` maps to **HTTP 422** (not 400/409). Response body:
```json
{
  "code": "invalid_request_error",
  "type": "duplicate_error",
  "message": "Customer with phone '...' already exists"
}
```

### Acceptance criteria

- [ ] Migration `007` runs cleanly on an empty database
- [ ] Migration `007` runs cleanly on a populated database (dedup step runs first)
- [ ] `POST /store/customers` with a duplicate phone returns HTTP 422
- [ ] `POST /store/customers/me` (update) with a duplicate phone returns HTTP 422
- [ ] `POST /store/customers` with `phone = null` does not conflict with another `null` phone
- [ ] `POST /store/customers` with duplicate email still returns 422 (unchanged)

---

## Modification 2 — Phone filter on `GET /admin/customers`

### Current state

**`src/customer/types.rs`** — `AdminCustomerListParams` (lines 44–60):
```rust
#[derive(Debug, Deserialize)]
pub struct AdminCustomerListParams {
    pub q: Option<String>,
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub has_account: Option<bool>,
    #[serde(default)]
    pub offset: i64,
    #[serde(default = "crate::types::default_limit")]
    pub limit: i64,
}
```

**`src/customer/repository.rs`** — `list` method builds dynamic SQL. After `last_name` is handled, the code processes `has_account`:
```rust
        if has_account_val.is_some() {
            conditions.push(format!("c.has_account = ${param_idx}"));
            param_idx += 1;
        }
```
No `phone` filter exists.

### Step-by-step changes

#### Step 2a — Add `phone` field to `AdminCustomerListParams`

File: **`src/customer/types.rs`**

Find the `AdminCustomerListParams` struct. Add `pub phone: Option<String>,` after `last_name` and before `has_account`:

```rust
#[derive(Debug, Deserialize)]
pub struct AdminCustomerListParams {
    pub q: Option<String>,
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub phone: Option<String>,      // ← add this line
    pub has_account: Option<bool>,
    #[serde(default)]
    pub offset: i64,
    #[serde(default = "crate::types::default_limit")]
    pub limit: i64,
}
```

#### Step 2b — Add `phone` variable in `list` method

File: **`src/customer/repository.rs`**, method `list`.

Find the block where filter variables are declared (lines ~69–75):
```rust
        let q_pattern: Option<String> = params.q.as_ref().map(|q| format!("%{}%", q));
        let email_pattern: Option<String> = params.email.as_ref().map(|v| format!("%{}%", v));
        let first_name_pattern: Option<String> =
            params.first_name.as_ref().map(|v| format!("%{}%", v));
        let last_name_pattern: Option<String> =
            params.last_name.as_ref().map(|v| format!("%{}%", v));
        let has_account_val = params.has_account;
```

Add `phone_filter` after `last_name_pattern`:
```rust
        let q_pattern: Option<String> = params.q.as_ref().map(|q| format!("%{}%", q));
        let email_pattern: Option<String> = params.email.as_ref().map(|v| format!("%{}%", v));
        let first_name_pattern: Option<String> =
            params.first_name.as_ref().map(|v| format!("%{}%", v));
        let last_name_pattern: Option<String> =
            params.last_name.as_ref().map(|v| format!("%{}%", v));
        let phone_filter: Option<String> = params.phone.clone();   // ← add this line
        let has_account_val = params.has_account;
```

#### Step 2c — Add `phone` condition to WHERE clause builder

In the same `list` method, find the `has_account` condition block:
```rust
        if has_account_val.is_some() {
            conditions.push(format!("c.has_account = ${param_idx}"));
            param_idx += 1;
        }
```

Add a `phone` condition block **before** the `has_account` block:
```rust
        if phone_filter.is_some() {
            conditions.push(format!("c.phone = ${param_idx}"));   // exact match, not ILIKE
            param_idx += 1;
        }
        if has_account_val.is_some() {
            conditions.push(format!("c.has_account = ${param_idx}"));
            param_idx += 1;
        }
```

#### Step 2d — Bind `phone_filter` in count query

In the same `list` method, find the count query binding section. Currently it binds in this order: `q_pattern`, `email_pattern`, `first_name_pattern`, `last_name_pattern`, `has_account_val`.

Find this block:
```rust
        if let Some(ref v) = last_name_pattern {
            count_q = count_q.bind(v.as_str());
        }
        if let Some(v) = has_account_val {
            count_q = count_q.bind(v);
        }
        let count = count_q.fetch_one(&self.pool).await?;
```

Add binding for `phone_filter` before `has_account_val`:
```rust
        if let Some(ref v) = last_name_pattern {
            count_q = count_q.bind(v.as_str());
        }
        if let Some(ref v) = phone_filter {            // ← add these two lines
            count_q = count_q.bind(v.as_str());
        }
        if let Some(v) = has_account_val {
            count_q = count_q.bind(v);
        }
        let count = count_q.fetch_one(&self.pool).await?;
```

#### Step 2e — Bind `phone_filter` in data query

In the same `list` method, find the data query binding section. Currently it binds in the same order.

Find this block:
```rust
        if let Some(ref v) = last_name_pattern {
            data_q = data_q.bind(v.as_str());
        }
        if let Some(v) = has_account_val {
            data_q = data_q.bind(v);
        }
        let customers = data_q
            .bind(params.offset)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;
```

Add binding for `phone_filter` before `has_account_val`:
```rust
        if let Some(ref v) = last_name_pattern {
            data_q = data_q.bind(v.as_str());
        }
        if let Some(ref v) = phone_filter {            // ← add these two lines
            data_q = data_q.bind(v.as_str());
        }
        if let Some(v) = has_account_val {
            data_q = data_q.bind(v);
        }
        let customers = data_q
            .bind(params.offset)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;
```

### Acceptance criteria

- [ ] `GET /admin/customers?phone=%2B6281234567890` returns only the matching customer
- [ ] `GET /admin/customers?phone=nonexistent` returns `{ "customers": [], "count": 0, "offset": 0, "limit": 50 }`
- [ ] `phone` filter is **exact match** (not ILIKE) — phone numbers are structured strings
- [ ] All existing filters (`q`, `email`, `first_name`, `last_name`, `has_account`) still work unchanged
- [ ] `phone` can be combined with other filters (e.g. `?phone=X&has_account=true`)

---

## Modification 3 — `GET /admin/orders` (list all orders)

### Current state

**`src/order/types.rs`** — existing types:
```rust
pub struct OrderListResponse {          // used by store list
    pub orders: Vec<OrderWithItems>,
    pub count: i64,
    pub offset: i64,
    pub limit: i64,
}

pub struct ListOrdersParams {           // used by store list (requires customer_id)
    pub offset: i64,
    pub limit: i64,
    pub id: Option<String>,
    pub status: Option<String>,
}
```
No admin-specific list params type exists.

**`src/order/repository.rs`** — existing methods:
- `list_by_customer(customer_id, params)` — filters by mandatory `customer_id`
- `find_by_id(id)` — get single order

No `list_all` method exists.

**`src/order/routes.rs`** — `admin_router()`:
```rust
pub fn admin_router() -> Router<AppState> {
    Router::new()
        .route("/admin/orders/{id}/cancel", post(admin_cancel_order))
        .route("/admin/orders/{id}/complete", post(admin_complete_order))
        .route("/admin/orders/{id}/fulfill", post(admin_fulfill_order))
        .route("/admin/orders/{id}/ship", post(admin_ship_order))
        .route("/admin/orders/{id}/capture-payment", post(admin_capture_payment))
}
```
No `GET /admin/orders` route exists.

### Step-by-step changes

#### Step 3a — Add `AdminListOrdersParams` to types

File: **`src/order/types.rs`**

Add this struct and impl at the end of the file (after the existing `ListOrdersParams` impl block):

```rust
#[derive(Debug, Deserialize)]
pub struct AdminListOrdersParams {
    pub customer_id: Option<String>,
    pub status: Option<String>,
    #[serde(default)]
    pub offset: i64,
    #[serde(default = "types::default_limit")]
    pub limit: i64,
}

impl AdminListOrdersParams {
    pub fn capped_limit(&self) -> i64 {
        self.limit.min(100)
    }
}
```

Note: `OrderListResponse` (already defined) has the correct shape for the admin list response — reuse it, do **not** create a new `AdminOrderListResponse` type.

#### Step 3b — Add `list_all` method to `OrderRepository`

File: **`src/order/repository.rs`**

Add this method to `impl OrderRepository`, after the `list_by_customer` method and before `load_items`:

```rust
    pub async fn list_all(
        &self,
        params: &AdminListOrdersParams,
    ) -> Result<(Vec<OrderWithItems>, i64), AppError> {
        let limit = params.capped_limit();

        let mut where_parts = vec!["deleted_at IS NULL".to_string()];
        let mut idx = 1u32;

        let customer_id_filter = if let Some(ref v) = params.customer_id {
            where_parts.push(format!("customer_id = ${}", idx));
            idx += 1;
            Some(v.clone())
        } else {
            None
        };

        let status_filter = if let Some(ref v) = params.status {
            where_parts.push(format!("status = ${}", idx));
            idx += 1;
            Some(v.clone())
        } else {
            None
        };

        let where_sql = where_parts.join(" AND ");

        let count_sql = format!("SELECT COUNT(*) FROM orders WHERE {}", where_sql);
        let query_sql = format!(
            "SELECT * FROM orders WHERE {} ORDER BY created_at DESC LIMIT ${} OFFSET ${}",
            where_sql,
            idx,
            idx + 1
        );

        let mut count_q = sqlx::query_as::<_, (i64,)>(&count_sql);
        if let Some(ref v) = customer_id_filter {
            count_q = count_q.bind(v);
        }
        if let Some(ref v) = status_filter {
            count_q = count_q.bind(v);
        }
        let count = count_q.fetch_one(&self.pool).await?;

        let mut data_q = sqlx::query_as::<_, Order>(&query_sql);
        if let Some(ref v) = customer_id_filter {
            data_q = data_q.bind(v);
        }
        if let Some(ref v) = status_filter {
            data_q = data_q.bind(v);
        }
        data_q = data_q.bind(limit);
        data_q = data_q.bind(params.offset);
        let orders = data_q.fetch_all(&self.pool).await?;

        let mut result = Vec::with_capacity(orders.len());
        for order in orders {
            result.push(self.load_items(order).await?);
        }

        Ok((result, count.0))
    }
```

#### Step 3c — Add import for `AdminListOrdersParams` in routes

File: **`src/order/routes.rs`**

The file already has `use super::types::*;` on line 1 — `AdminListOrdersParams` will be imported automatically via the glob. No import change needed.

#### Step 3d — Add route and handler to `admin_router`

File: **`src/order/routes.rs`**

Find `admin_router()` and add the new route at the top:
```rust
pub fn admin_router() -> Router<AppState> {
    Router::new()
        .route("/admin/orders", get(admin_list_orders))                            // ← add
        .route("/admin/orders/{id}/cancel", post(admin_cancel_order))
        .route("/admin/orders/{id}/complete", post(admin_complete_order))
        .route("/admin/orders/{id}/fulfill", post(admin_fulfill_order))
        .route("/admin/orders/{id}/ship", post(admin_ship_order))
        .route("/admin/orders/{id}/capture-payment", post(admin_capture_payment))
}
```

Add the handler function after `admin_complete_order` (or at the end of the file, before the closing brace):
```rust
#[tracing::instrument(skip_all, fields(offset = params.offset, limit = params.limit))]
async fn admin_list_orders(
    State(state): State<AppState>,
    Query(params): Query<AdminListOrdersParams>,
) -> Result<Json<OrderListResponse>, AppError> {
    let limit = params.capped_limit();
    let (orders, count) = state.repos.order.list_all(&params).await?;
    Ok(Json(OrderListResponse {
        orders,
        count,
        offset: params.offset,
        limit,
    }))
}
```

### Acceptance criteria

- [ ] `GET /admin/orders` returns all orders (no customer filter), response shape:
  ```json
  { "orders": [...], "count": N, "offset": 0, "limit": 20 }
  ```
- [ ] Each order object includes `items` array (same shape as `/store/orders/:id`)
- [ ] `GET /admin/orders?customer_id=cus_xxx` filters to that customer's orders only
- [ ] `GET /admin/orders?status=pending` filters by status
- [ ] `GET /admin/orders?limit=200` is capped at 100
- [ ] `GET /admin/orders?offset=5&limit=3` paginates correctly
- [ ] `GET /admin/orders` with no orders returns `{ "orders": [], "count": 0, "offset": 0, "limit": 20 }`
- [ ] Does **not** require `X-Customer-Id` header

---

## Modification 4 — `GET /admin/orders/{id}` (admin order detail)

### Current state

**`src/order/routes.rs`** — `store_get_order` (line 65):
```rust
async fn store_get_order(
    State(state): State<AppState>,
    axum::Extension(customer): axum::Extension<CustomerId>,
    Path(id): Path<String>,
) -> Result<Json<OrderResponse>, AppError> {
    let order = state.repos.order.find_by_id(&id).await?;

    if order.order.customer_id.as_deref() != Some(customer.id.as_str()) {
        return Err(AppError::NotFound(format!(
            "Order with id: {} was not found",
            id
        )));
    }

    Ok(Json(OrderResponse { order }))
}
```
There is no admin version. Admin needs to retrieve any order without customer ownership check.

### Step-by-step changes

#### Step 4a — Add route to `admin_router`

File: **`src/order/routes.rs`**

After adding the route for Modification 3, add the detail route:
```rust
pub fn admin_router() -> Router<AppState> {
    Router::new()
        .route("/admin/orders", get(admin_list_orders))
        .route("/admin/orders/{id}", get(admin_get_order))                         // ← add
        .route("/admin/orders/{id}/cancel", post(admin_cancel_order))
        .route("/admin/orders/{id}/complete", post(admin_complete_order))
        .route("/admin/orders/{id}/fulfill", post(admin_fulfill_order))
        .route("/admin/orders/{id}/ship", post(admin_ship_order))
        .route("/admin/orders/{id}/capture-payment", post(admin_capture_payment))
}
```

#### Step 4b — Add handler function

File: **`src/order/routes.rs`**

Add the handler after `admin_list_orders`:
```rust
#[tracing::instrument(skip_all, fields(id = %id))]
async fn admin_get_order(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<OrderResponse>, AppError> {
    let order = state.repos.order.find_by_id(&id).await?;
    Ok(Json(OrderResponse { order }))
}
```

No `X-Customer-Id` required. No customer ownership check. Uses the same `find_by_id` as the store route.

### Acceptance criteria

- [ ] `GET /admin/orders/{id}` returns the order regardless of `customer_id`
- [ ] Response shape matches `GET /store/orders/{id}`:
  ```json
  { "order": { "id": "...", "items": [...], ... } }
  ```
- [ ] `GET /admin/orders/nonexistent-id` returns HTTP 404:
  ```json
  { "code": "invalid_request_error", "type": "not_found", "message": "Order with id nonexistent-id was not found" }
  ```
- [ ] Does **not** require `X-Customer-Id` header
- [ ] Admin can retrieve orders belonging to any customer

---

## Implementation order

Implement in this sequence to avoid compilation errors:

1. **Mod 2a** — Add `phone` to `AdminCustomerListParams` in `src/customer/types.rs`
2. **Mod 3a** — Add `AdminListOrdersParams` to `src/order/types.rs`
3. **Mod 1a + 1b** — Create migration files (no Rust changes yet)
4. **Mod 1c + 1d** — Update `src/customer/repository.rs` for phone constraint errors
5. **Mod 2b–2e** — Update `src/customer/repository.rs` for phone filter
6. **Mod 3b** — Add `list_all` to `src/order/repository.rs`
7. **Mod 3c–3d + 4a–4b** — Update `src/order/routes.rs` with both new routes and handlers

After step 7, run `cargo build`. Fix any compilation errors before running tests.

---

## Final verification checklist

```bash
cargo build                          # must compile without errors
cargo clippy -- -D warnings          # must be zero warnings
cargo test                           # all tests must pass
```

| # | Modification | Files changed | Migration |
|---|---|---|---|
| 1 | Phone unique constraint | `migrations/007_customers_phone_unique.sql`, `migrations/sqlite/007_customers_phone_unique.sql`, `src/customer/repository.rs` | Yes — `007` |
| 2 | Phone filter on admin customers | `src/customer/types.rs`, `src/customer/repository.rs` | No |
| 3 | `GET /admin/orders` list | `src/order/types.rs`, `src/order/repository.rs`, `src/order/routes.rs` | No |
| 4 | `GET /admin/orders/{id}` admin detail | `src/order/routes.rs` | No |

All four modifications can be implemented and merged in toko-rs as a single PR.
