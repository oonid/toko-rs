use super::models::*;
use super::types::*;
use crate::error::AppError;
use crate::payment::repository::PaymentRepository;
use crate::types::generate_entity_id;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct OrderRepository {
    pool: SqlitePool,
}

impl OrderRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create_from_cart(&self, cart_id: &str) -> Result<OrderWithItems, AppError> {
        let mut tx = self.pool.begin().await?;

        let cart = sqlx::query_as::<_, crate::cart::models::Cart>(
            "SELECT * FROM carts WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(cart_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Cart not found".into()))?;

        if cart.completed_at.is_some() {
            return Err(AppError::Conflict("Cart is already completed".into()));
        }

        let cart_items = sqlx::query_as::<_, crate::cart::models::CartLineItem>(
            "SELECT * FROM cart_line_items WHERE cart_id = ? AND deleted_at IS NULL",
        )
        .bind(cart_id)
        .fetch_all(&mut *tx)
        .await?;

        if cart_items.is_empty() {
            return Err(AppError::InvalidData(
                "Cannot complete an empty cart".into(),
            ));
        }

        let display_id: (i64,) = sqlx::query_as(
            "UPDATE _sequences SET value = value + 1 WHERE name = 'order_display_id' RETURNING value",
        )
        .fetch_one(&mut *tx)
        .await?;

        let order_id = generate_entity_id("order");
        let order = sqlx::query_as::<_, Order>(
            r#"
            INSERT INTO orders (id, display_id, customer_id, email, currency_code, status)
            VALUES (?, ?, ?, ?, ?, 'pending')
            RETURNING *
            "#,
        )
        .bind(&order_id)
        .bind(display_id.0)
        .bind(&cart.customer_id)
        .bind(&cart.email)
        .bind(&cart.currency_code)
        .fetch_one(&mut *tx)
        .await
        .map_err(Self::map_display_id_conflict)?;

        let mut order_items = Vec::with_capacity(cart_items.len());
        for ci in &cart_items {
            let item_id = generate_entity_id("oli");
            let item = sqlx::query_as::<_, OrderLineItem>(
                r#"
                INSERT INTO order_line_items (id, order_id, title, quantity, unit_price, variant_id, product_id, snapshot)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                RETURNING *
                "#,
            )
            .bind(&item_id)
            .bind(&order_id)
            .bind(&ci.title)
            .bind(ci.quantity)
            .bind(ci.unit_price)
            .bind(&ci.variant_id)
            .bind(&ci.product_id)
            .bind(&ci.snapshot)
            .fetch_one(&mut *tx)
            .await?;
            order_items.push(item);
        }

        let item_total: i64 = order_items.iter().map(|i| i.quantity * i.unit_price).sum();

        let _payment =
            PaymentRepository::create_with_tx(&mut tx, &order_id, item_total, &cart.currency_code)
                .await?;

        sqlx::query(
            "UPDATE carts SET completed_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        )
        .bind(cart_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        let order_with_items = OrderWithItems {
            order,
            items: order_items,
            item_total,
            total: item_total,
        };

        Ok(order_with_items)
    }

    fn map_display_id_conflict(e: sqlx::Error) -> AppError {
        if let sqlx::Error::Database(db_err) = &e {
            if db_err.code().as_deref() == Some("2067") {
                return AppError::Conflict(
                    "Order creation failed due to concurrent request. Please retry.".into(),
                );
            }
        }
        AppError::DatabaseError(e)
    }

    pub async fn find_by_id(&self, id: &str) -> Result<OrderWithItems, AppError> {
        let order =
            sqlx::query_as::<_, Order>("SELECT * FROM orders WHERE id = ? AND deleted_at IS NULL")
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
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM orders WHERE customer_id = ? AND deleted_at IS NULL",
        )
        .bind(customer_id)
        .fetch_one(&self.pool)
        .await?;

        let orders = sqlx::query_as::<_, Order>(
            "SELECT * FROM orders WHERE customer_id = ? AND deleted_at IS NULL ORDER BY created_at DESC LIMIT ? OFFSET ?",
        )
        .bind(customer_id)
        .bind(params.limit)
        .bind(params.offset)
        .fetch_all(&self.pool)
        .await?;

        let mut result = Vec::with_capacity(orders.len());
        for order in orders {
            result.push(self.load_items(order).await?);
        }

        Ok((result, count.0))
    }

    async fn load_items(&self, order: Order) -> Result<OrderWithItems, AppError> {
        let items = sqlx::query_as::<_, OrderLineItem>(
            "SELECT * FROM order_line_items WHERE order_id = ? AND deleted_at IS NULL",
        )
        .bind(&order.id)
        .fetch_all(&self.pool)
        .await?;

        let item_total: i64 = items.iter().map(|i| i.quantity * i.unit_price).sum();

        Ok(OrderWithItems {
            order,
            items,
            item_total,
            total: item_total,
        })
    }
}
