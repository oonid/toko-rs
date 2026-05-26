use super::models::*;
use super::types::*;
use crate::db::DbPool;
use crate::error::AppError;
use crate::payment::repository::PaymentRepository;
use crate::types::generate_entity_id;
use sqlx::Row;

#[derive(Clone)]
pub struct OrderRepository {
    pool: DbPool,
}

impl OrderRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn create_from_cart(&self, cart_id: &str) -> Result<OrderWithItems, AppError> {
        let mut tx = self.pool.begin().await?;

        let cart = sqlx::query_as::<_, crate::cart::models::Cart>(
            #[cfg(feature = "postgres")]
            "SELECT * FROM carts WHERE id = $1 AND deleted_at IS NULL FOR UPDATE",
            #[cfg(feature = "sqlite")]
            "SELECT * FROM carts WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(cart_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Cart not found".into()))?;

        if cart.completed_at.is_some() {
            let existing: Option<Order> =
                sqlx::query_as("SELECT * FROM orders WHERE cart_id = $1 AND deleted_at IS NULL")
                    .bind(cart_id)
                    .fetch_optional(&mut *tx)
                    .await?;

            if let Some(order) = existing {
                let items = sqlx::query_as::<_, OrderLineItem>(
                    "SELECT * FROM order_line_items WHERE order_id = $1 AND deleted_at IS NULL",
                )
                .bind(&order.id)
                .fetch_all(&mut *tx)
                .await?;

                tx.commit().await?;
                return Ok(OrderWithItems::from_items(order, items, "not_paid", 0));
            }

            return Err(AppError::InvalidData("Cart is already completed".into()));
        }

        #[cfg(feature = "sqlite")]
        {
            let guard = sqlx::query(
                "UPDATE carts SET updated_at = CURRENT_TIMESTAMP WHERE id = $1 AND completed_at IS NULL",
            )
            .bind(cart_id)
            .execute(&mut *tx)
            .await?;

            if guard.rows_affected() == 0 {
                return Err(AppError::InvalidData("Cart is already completed".into()));
            }
        }

        let cart_items = sqlx::query_as::<_, crate::cart::models::CartLineItem>(
            "SELECT * FROM cart_line_items WHERE cart_id = $1 AND deleted_at IS NULL",
        )
        .bind(cart_id)
        .fetch_all(&mut *tx)
        .await?;

        if cart_items.is_empty() {
            return Err(AppError::InvalidData(
                "Cannot complete an empty cart".into(),
            ));
        }

        let existing: Option<Order> =
            sqlx::query_as("SELECT * FROM orders WHERE cart_id = $1 AND deleted_at IS NULL")
                .bind(cart_id)
                .fetch_optional(&mut *tx)
                .await?;

        if let Some(order) = existing {
            let items = sqlx::query_as::<_, OrderLineItem>(
                "SELECT * FROM order_line_items WHERE order_id = $1 AND deleted_at IS NULL",
            )
            .bind(&order.id)
            .fetch_all(&mut *tx)
            .await?;

            sqlx::query("UPDATE carts SET completed_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE id = $1")
                .bind(cart_id)
                .execute(&mut *tx)
                .await?;

            tx.commit().await?;
            return Ok(OrderWithItems::from_items(order, items, "not_paid", 0));
        }

        let display_id: (i64,) = sqlx::query_as(
            "UPDATE _sequences SET value = value + 1 WHERE name = 'order_display_id' RETURNING value",
        )
        .fetch_one(&mut *tx)
        .await?;

        let order_id = generate_entity_id("order");
        let order = sqlx::query_as::<_, Order>(
            r#"
            INSERT INTO orders (id, display_id, cart_id, customer_id, email, currency_code, status,
                                shipping_address, billing_address, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, 'pending', $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(&order_id)
        .bind(display_id.0)
        .bind(cart_id)
        .bind(&cart.customer_id)
        .bind(&cart.email)
        .bind(&cart.currency_code)
        .bind(&cart.shipping_address)
        .bind(&cart.billing_address)
        .bind(&cart.metadata)
        .fetch_one(&mut *tx)
        .await
        .map_err(Self::map_display_id_conflict)?;

        let mut order_items = Vec::with_capacity(cart_items.len());
        for ci in &cart_items {
            let item_id = generate_entity_id("ordli");
            let item = sqlx::query_as::<_, OrderLineItem>(
                r#"
                INSERT INTO order_line_items (id, order_id, title, quantity, unit_price, compare_at_unit_price,
                                               variant_id, product_id, snapshot, metadata)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                RETURNING *
                "#,
            )
            .bind(&item_id)
            .bind(&order_id)
            .bind(&ci.title)
            .bind(ci.quantity)
            .bind(ci.unit_price)
            .bind(ci.compare_at_unit_price)
            .bind(&ci.variant_id)
            .bind(&ci.product_id)
            .bind(&ci.snapshot)
            .bind(&ci.metadata)
            .fetch_one(&mut *tx)
            .await?;
            order_items.push(item);
        }

        let item_total: i64 = order_items.iter().map(|i| i.quantity * i.unit_price).sum();

        let _payment =
            PaymentRepository::create_with_tx(&mut tx, &order_id, item_total, &cart.currency_code)
                .await?;

        sqlx::query("UPDATE carts SET completed_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE id = $1")
            .bind(cart_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        let order_with_items = OrderWithItems::from_items(order, order_items, "not_paid", 0);

        Ok(order_with_items)
    }

    fn map_display_id_conflict(e: sqlx::Error) -> AppError {
        if crate::db::is_unique_violation(&e) {
            return AppError::Conflict(
                "Order creation failed due to concurrent request. Please retry.".into(),
            );
        }
        AppError::DatabaseError(e)
    }

    pub async fn find_by_id(&self, id: &str) -> Result<OrderWithItems, AppError> {
        let order =
            sqlx::query_as::<_, Order>("SELECT * FROM orders WHERE id = $1 AND deleted_at IS NULL")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?
                .ok_or_else(|| AppError::NotFound(format!("Order with id {} was not found", id)))?;

        self.load_items(order).await
    }

    pub async fn list_by_customer(
        &self,
        customer_id: &str,
        params: &ListOrdersParams,
    ) -> Result<(Vec<OrderWithItems>, i64), AppError> {
        let mut where_parts = vec![
            "customer_id = $1".to_string(),
            "deleted_at IS NULL".to_string(),
        ];
        let mut idx = 2u32;

        let id_filter = if let Some(ref id) = params.id {
            where_parts.push(format!("id = ${}", idx));
            idx += 1;
            Some(id.clone())
        } else {
            None
        };

        let status_filter = if let Some(ref status) = params.status {
            where_parts.push(format!("status = ${}", idx));
            idx += 1;
            Some(status.clone())
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
        count_q = count_q.bind(customer_id);
        if let Some(ref v) = id_filter {
            count_q = count_q.bind(v);
        }
        if let Some(ref v) = status_filter {
            count_q = count_q.bind(v);
        }
        let count = count_q.fetch_one(&self.pool).await?;

        let mut data_q = sqlx::query_as::<_, Order>(&query_sql);
        data_q = data_q.bind(customer_id);
        if let Some(ref v) = id_filter {
            data_q = data_q.bind(v);
        }
        if let Some(ref v) = status_filter {
            data_q = data_q.bind(v);
        }
        data_q = data_q.bind(params.capped_limit());
        data_q = data_q.bind(params.offset);
        let orders = data_q.fetch_all(&self.pool).await?;

        let mut result = Vec::with_capacity(orders.len());
        for order in orders {
            result.push(self.load_items(order).await?);
        }

        Ok((result, count.0))
    }

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

    async fn load_items(&self, order: Order) -> Result<OrderWithItems, AppError> {
        let items = sqlx::query_as::<_, OrderLineItem>(
            "SELECT * FROM order_line_items WHERE order_id = $1 AND deleted_at IS NULL",
        )
        .bind(&order.id)
        .fetch_all(&self.pool)
        .await?;

        let (payment_status, paid_total) = self.resolve_payment_status(&order.id).await;

        Ok(OrderWithItems::from_items(
            order,
            items,
            &payment_status,
            paid_total,
        ))
    }

    async fn resolve_payment_status(&self, order_id: &str) -> (String, i64) {
        let result: Option<(String, i64)> = sqlx::query_as(
            "SELECT status, amount FROM payment_records WHERE order_id = $1 AND deleted_at IS NULL",
        )
        .bind(order_id)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten();

        match result {
            Some((ref status, _amount)) if status == "authorized" => ("authorized".to_string(), 0),
            Some((ref status, amount)) if status == "captured" => ("captured".to_string(), amount),
            Some((ref status, _)) if status == "refunded" => ("refunded".to_string(), 0),
            Some((ref status, _)) if status == "canceled" => ("canceled".to_string(), 0),
            _ => ("not_paid".to_string(), 0),
        }
    }

    pub async fn cancel_order(&self, id: &str) -> Result<OrderWithItems, AppError> {
        // Initial existence check to give a proper 404
        let _order = self.find_by_id(id).await?;

        let result = sqlx::query(
            "UPDATE orders SET status = 'canceled', fulfillment_status = 'canceled', canceled_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE id = $1 AND status != 'canceled' AND status != 'completed'",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::InvalidData(
                "Order cannot be canceled (already canceled or completed)".to_string(),
            ));
        }

        self.find_by_id(id).await
    }

    pub async fn complete_order(&self, id: &str) -> Result<OrderWithItems, AppError> {
        // Initial existence check to give a proper 404
        let _order = self.find_by_id(id).await?;

        let result = sqlx::query(
            "UPDATE orders SET status = 'completed', updated_at = CURRENT_TIMESTAMP WHERE id = $1 AND status != 'completed' AND status != 'canceled'",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::InvalidData(
                "Order cannot be completed (already completed or canceled)".to_string(),
            ));
        }

        self.find_by_id(id).await
    }

    pub async fn fulfill_order(&self, id: &str) -> Result<OrderWithItems, AppError> {
        // Initial existence check to give a proper 404
        let _order = self.find_by_id(id).await?;

        let result = sqlx::query(
            "UPDATE orders SET fulfillment_status = 'fulfilled', updated_at = CURRENT_TIMESTAMP WHERE id = $1 AND fulfillment_status = 'not_fulfilled' AND status != 'canceled'",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::InvalidData(
                "Order cannot be fulfilled (already fulfilled or canceled)".to_string(),
            ));
        }

        self.find_by_id(id).await
    }

    pub async fn ship_order(&self, id: &str) -> Result<OrderWithItems, AppError> {
        // Initial existence check to give a proper 404
        let _order = self.find_by_id(id).await?;

        let result = sqlx::query(
            "UPDATE orders SET fulfillment_status = 'shipped', shipped_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE id = $1 AND fulfillment_status = 'fulfilled' AND status != 'canceled'",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::InvalidData(
                "Order cannot be shipped (must be fulfilled and not canceled)".to_string(),
            ));
        }

        self.find_by_id(id).await
    }

    pub async fn export_orders(
        &self,
        params: &AdminExportOrdersParams,
    ) -> Result<Vec<OrderExportRow>, AppError> {
        // Validate status filter
        if let Some(ref s) = params.status {
            match s.as_str() {
                "pending" | "completed" | "canceled" => {}
                _ => {
                    return Err(AppError::InvalidData(format!(
                        "Invalid status filter: '{}'. Must be one of: pending, completed, canceled",
                        s
                    )))
                }
            }
        }

        let mut where_parts = vec!["o.deleted_at IS NULL".to_string()];
        let mut binds: Vec<String> = Vec::new();
        let mut idx = 1u32;

        if let Some(ref s) = params.status {
            where_parts.push(format!("o.status = ${}", idx));
            binds.push(s.clone());
            idx += 1;
        }

        if let Some(ref dt) = params.created_at_from {
            where_parts.push(format!("o.created_at >= ${}::TIMESTAMPTZ", idx));
            binds.push(dt.to_rfc3339());
            idx += 1;
        }

        if let Some(ref dt) = params.created_at_to {
            where_parts.push(format!("o.created_at <= ${}::TIMESTAMPTZ", idx));
            binds.push(dt.to_rfc3339());
            idx += 1;
        }

        if let Some(ref q) = params.q {
            #[cfg(feature = "postgres")]
            where_parts.push(format!("o.email ILIKE ${}", idx));
            #[cfg(feature = "sqlite")]
            where_parts.push(format!("o.email LIKE ${}", idx));
            binds.push(format!("%{}%", q));
            idx += 1;
        }

        let _ = idx; // suppress unused warning
        let where_sql = where_parts.join(" AND ");

        let query_sql = format!(
            r#"
            SELECT
                o.id,
                o.display_id,
                o.email,
                o.currency_code,
                o.status,
                o.fulfillment_status,
                (SELECT status FROM payment_records WHERE order_id = o.id AND deleted_at IS NULL ORDER BY created_at DESC LIMIT 1) AS raw_payment_status,
                (SELECT COALESCE(CAST(SUM(quantity) AS BIGINT), 0) FROM order_line_items WHERE order_id = o.id AND deleted_at IS NULL) AS item_count,
                (SELECT COALESCE(CAST(SUM(quantity * unit_price) AS BIGINT), 0) FROM order_line_items WHERE order_id = o.id AND deleted_at IS NULL) AS total_cents,
                o.created_at,
                o.shipped_at,
                o.canceled_at
            FROM orders o
            WHERE {where_sql}
            ORDER BY o.created_at ASC
            "#
        );

        let mut q = sqlx::query(&query_sql);
        for b in &binds {
            q = q.bind(b);
        }

        let rows: Vec<ExportRow> = q
            .fetch_all(&self.pool)
            .await?
            .iter()
            .map(|row| {
                let id: String = row.get("id");
                let display_id: i64 = row.get("display_id");
                let email: Option<String> = row.get("email");
                let currency_code: String = row.get("currency_code");
                let status: String = row.get("status");
                let fulfillment_status: String = row.get("fulfillment_status");
                let raw_payment_status: Option<String> = row.get("raw_payment_status");
                let item_count: i64 = row.get("item_count");
                let total_cents: i64 = row.get("total_cents");
                let created_at: chrono::DateTime<chrono::Utc> = row.get("created_at");
                let shipped_at: Option<chrono::DateTime<chrono::Utc>> = row.get("shipped_at");
                let canceled_at: Option<chrono::DateTime<chrono::Utc>> = row.get("canceled_at");
                ExportRow {
                    id,
                    display_id,
                    email,
                    currency_code,
                    status,
                    fulfillment_status,
                    raw_payment_status,
                    item_count,
                    total_cents,
                    created_at,
                    shipped_at,
                    canceled_at,
                }
            })
            .collect();

        let result = rows
            .into_iter()
            .map(|row| {
                let payment_status = match row.raw_payment_status.as_deref() {
                    Some("authorized") => "authorized".to_string(),
                    Some("captured") => "captured".to_string(),
                    Some("refunded") => "refunded".to_string(),
                    Some("canceled") => "canceled".to_string(),
                    _ => "not_paid".to_string(),
                };
                OrderExportRow {
                    id: row.id,
                    display_id: row.display_id,
                    email: row.email,
                    currency_code: row.currency_code,
                    status: row.status,
                    fulfillment_status: row.fulfillment_status,
                    payment_status,
                    item_count: row.item_count,
                    total_cents: row.total_cents,
                    created_at: row.created_at,
                    shipped_at: row.shipped_at,
                    canceled_at: row.canceled_at,
                }
            })
            .collect();

        Ok(result)
    }

    pub async fn cancel_order_tx(
        &self,
        id: &str,
        tx: &mut sqlx::Transaction<'_, crate::db::DbDatabase>,
    ) -> Result<Order, AppError> {
        let result = sqlx::query(
            "UPDATE orders SET status = 'canceled', fulfillment_status = 'canceled', canceled_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE id = $1 AND status != 'canceled' AND status != 'completed'",
        )
        .bind(id)
        .execute(&mut **tx)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::InvalidData(
                "Order cannot be canceled (already canceled or completed)".to_string(),
            ));
        }

        let order =
            sqlx::query_as::<_, Order>("SELECT * FROM orders WHERE id = $1 AND deleted_at IS NULL")
                .bind(id)
                .fetch_one(&mut **tx)
                .await?;

        Ok(order)
    }

    pub async fn complete_order_tx(
        &self,
        id: &str,
        tx: &mut sqlx::Transaction<'_, crate::db::DbDatabase>,
    ) -> Result<Order, AppError> {
        let result = sqlx::query(
            "UPDATE orders SET status = 'completed', updated_at = CURRENT_TIMESTAMP WHERE id = $1 AND status != 'completed' AND status != 'canceled'",
        )
        .bind(id)
        .execute(&mut **tx)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::InvalidData(
                "Order cannot be completed (already completed or canceled)".to_string(),
            ));
        }

        let order =
            sqlx::query_as::<_, Order>("SELECT * FROM orders WHERE id = $1 AND deleted_at IS NULL")
                .bind(id)
                .fetch_one(&mut **tx)
                .await?;

        Ok(order)
    }

    pub async fn fulfill_order_tx(
        &self,
        id: &str,
        tx: &mut sqlx::Transaction<'_, crate::db::DbDatabase>,
    ) -> Result<Order, AppError> {
        let result = sqlx::query(
            "UPDATE orders SET fulfillment_status = 'fulfilled', updated_at = CURRENT_TIMESTAMP WHERE id = $1 AND fulfillment_status = 'not_fulfilled' AND status != 'canceled'",
        )
        .bind(id)
        .execute(&mut **tx)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::InvalidData(
                "Order cannot be fulfilled (already fulfilled or canceled)".to_string(),
            ));
        }

        let order =
            sqlx::query_as::<_, Order>("SELECT * FROM orders WHERE id = $1 AND deleted_at IS NULL")
                .bind(id)
                .fetch_one(&mut **tx)
                .await?;

        Ok(order)
    }

    pub async fn ship_order_tx(
        &self,
        id: &str,
        tx: &mut sqlx::Transaction<'_, crate::db::DbDatabase>,
    ) -> Result<Order, AppError> {
        let result = sqlx::query(
            "UPDATE orders SET fulfillment_status = 'shipped', shipped_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE id = $1 AND fulfillment_status = 'fulfilled' AND status != 'canceled'",
        )
        .bind(id)
        .execute(&mut **tx)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::InvalidData(
                "Order cannot be shipped (must be fulfilled and not canceled)".to_string(),
            ));
        }

        let order =
            sqlx::query_as::<_, Order>("SELECT * FROM orders WHERE id = $1 AND deleted_at IS NULL")
                .bind(id)
                .fetch_one(&mut **tx)
                .await?;

        Ok(order)
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ExportRow {
    id: String,
    display_id: i64,
    email: Option<String>,
    currency_code: String,
    status: String,
    fulfillment_status: String,
    raw_payment_status: Option<String>,
    item_count: i64,
    total_cents: i64,
    created_at: chrono::DateTime<chrono::Utc>,
    shipped_at: Option<chrono::DateTime<chrono::Utc>>,
    canceled_at: Option<chrono::DateTime<chrono::Utc>>,
}
