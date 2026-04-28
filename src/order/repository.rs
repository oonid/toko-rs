use super::models::*;
use super::types::*;
use crate::db::DbPool;
use crate::error::AppError;
use crate::payment::repository::PaymentRepository;
use crate::types::generate_entity_id;

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
                return Ok(OrderWithItems::from_items(order, items));
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
            return Ok(OrderWithItems::from_items(order, items));
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

        let order_with_items = OrderWithItems::from_items(order, order_items);

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

    async fn load_items(&self, order: Order) -> Result<OrderWithItems, AppError> {
        let items = sqlx::query_as::<_, OrderLineItem>(
            "SELECT * FROM order_line_items WHERE order_id = $1 AND deleted_at IS NULL",
        )
        .bind(&order.id)
        .fetch_all(&self.pool)
        .await?;

        Ok(OrderWithItems::from_items(order, items))
    }
}
