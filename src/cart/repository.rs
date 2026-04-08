use super::models::*;
use super::types::*;
use crate::error::AppError;
use crate::types::generate_entity_id;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct CartRepository {
    pool: SqlitePool,
}

impl CartRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create_cart(&self, input: CreateCartInput) -> Result<CartWithItems, AppError> {
        let cart_id = generate_entity_id("cart");
        let currency = input
            .currency_code
            .unwrap_or_else(|| "usd".to_string())
            .to_lowercase();

        let cart = sqlx::query_as::<_, Cart>(
            r#"
            INSERT INTO carts (id, customer_id, email, currency_code, metadata)
            VALUES (?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(&cart_id)
        .bind(&input.customer_id)
        .bind(&input.email)
        .bind(&currency)
        .bind(input.metadata.clone().map(sqlx::types::Json))
        .fetch_one(&self.pool)
        .await?;

        Ok(CartWithItems {
            cart,
            items: vec![],
            item_total: 0,
            total: 0,
        })
    }

    pub async fn get_cart(&self, cart_id: &str) -> Result<CartWithItems, AppError> {
        let cart =
            sqlx::query_as::<_, Cart>(r#"SELECT * FROM carts WHERE id = ? AND deleted_at IS NULL"#)
                .bind(cart_id)
                .fetch_optional(&self.pool)
                .await?
                .ok_or_else(|| AppError::NotFound("Cart not found".into()))?;

        let items = sqlx::query_as::<_, CartLineItem>(
            r#"SELECT * FROM cart_line_items WHERE cart_id = ? AND deleted_at IS NULL"#,
        )
        .bind(cart_id)
        .fetch_all(&self.pool)
        .await?;

        let item_total = items.iter().map(|i| i.quantity * i.unit_price).sum();
        let total = item_total;

        Ok(CartWithItems {
            cart,
            items,
            item_total,
            total,
        })
    }

    pub async fn update_cart(
        &self,
        cart_id: &str,
        input: UpdateCartInput,
    ) -> Result<CartWithItems, AppError> {
        let cart =
            sqlx::query_as::<_, Cart>(r#"SELECT * FROM carts WHERE id = ? AND deleted_at IS NULL"#)
                .bind(cart_id)
                .fetch_optional(&self.pool)
                .await?
                .ok_or_else(|| AppError::NotFound("Cart not found".into()))?;

        if cart.completed_at.is_some() {
            return Err(AppError::Conflict("Cannot update a completed cart".into()));
        }

        sqlx::query(
            r#"
            UPDATE carts 
            SET 
                email = COALESCE(?, email),
                customer_id = COALESCE(?, customer_id),
                metadata = COALESCE(?, metadata),
                updated_at = CURRENT_TIMESTAMP
            WHERE id = ? AND deleted_at IS NULL
            "#,
        )
        .bind(&input.email)
        .bind(&input.customer_id)
        .bind(input.metadata.map(sqlx::types::Json))
        .bind(cart_id)
        .execute(&self.pool)
        .await?;

        self.get_cart(cart_id).await
    }
    pub async fn add_line_item(
        &self,
        cart_id: &str,
        input: AddLineItemInput,
    ) -> Result<CartWithItems, AppError> {
        let mut tx = self.pool.begin().await?;

        let cart_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM carts WHERE id = ? AND deleted_at IS NULL)",
        )
        .bind(cart_id)
        .fetch_one(&mut *tx)
        .await?;
        if !cart_exists {
            return Err(AppError::NotFound("Cart not found".into()));
        }

        let row = sqlx::query(
            r#"
            SELECT v.id as variant_id, v.title as variant_title, v.sku as variant_sku, v.price,
                   p.id as product_id, p.title as product_title
            FROM product_variants v
            JOIN products p ON p.id = v.product_id
            WHERE v.id = ? AND v.deleted_at IS NULL AND p.deleted_at IS NULL
            "#,
        )
        .bind(&input.variant_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Variant not found".into()))?;

        let variant_id: String = sqlx::Row::get(&row, "variant_id");
        let variant_title: String = sqlx::Row::get(&row, "variant_title");
        let variant_sku: Option<String> = sqlx::Row::get(&row, "variant_sku");
        let price: i64 = sqlx::Row::get(&row, "price");
        let product_id: String = sqlx::Row::get(&row, "product_id");
        let product_title: String = sqlx::Row::get(&row, "product_title");

        let line_id = generate_entity_id("cali");
        let snapshot = serde_json::json!({
            "product_title": product_title,
            "variant_title": variant_title,
            "variant_sku": variant_sku
        });

        // See if same variant already exists, if so update quantity
        let existing = sqlx::query("SELECT id, quantity FROM cart_line_items WHERE cart_id = ? AND variant_id = ? AND deleted_at IS NULL")
            .bind(cart_id)
            .bind(&input.variant_id)
            .fetch_optional(&mut *tx)
            .await?;

        if let Some(ext) = existing {
            let ext_id: String = sqlx::Row::get(&ext, "id");
            sqlx::query("UPDATE cart_line_items SET quantity = quantity + ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                .bind(input.quantity)
                .bind(&ext_id)
                .execute(&mut *tx)
                .await?;
        } else {
            sqlx::query(
                r#"
                INSERT INTO cart_line_items (id, cart_id, title, quantity, unit_price, variant_id, product_id, snapshot, metadata)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#
            )
            .bind(&line_id)
            .bind(cart_id)
            .bind(&product_title)
            .bind(input.quantity)
            .bind(price)
            .bind(&variant_id)
            .bind(&product_id)
            .bind(sqlx::types::Json(snapshot))
            .bind(input.metadata.map(sqlx::types::Json))
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        self.get_cart(cart_id).await
    }

    pub async fn update_line_item(
        &self,
        cart_id: &str,
        line_id: &str,
        input: UpdateLineItemInput,
    ) -> Result<CartWithItems, AppError> {
        if input.quantity == 0 {
            return self.delete_line_item(cart_id, line_id).await;
        }

        sqlx::query(
            r#"
            UPDATE cart_line_items 
            SET quantity = ?, 
                metadata = COALESCE(?, metadata),
                updated_at = CURRENT_TIMESTAMP 
            WHERE id = ? AND cart_id = ? AND deleted_at IS NULL
            "#,
        )
        .bind(input.quantity)
        .bind(input.metadata.map(sqlx::types::Json))
        .bind(line_id)
        .bind(cart_id)
        .execute(&self.pool)
        .await?;

        self.get_cart(cart_id).await
    }

    pub async fn delete_line_item(
        &self,
        cart_id: &str,
        line_id: &str,
    ) -> Result<CartWithItems, AppError> {
        sqlx::query(
            "UPDATE cart_line_items SET deleted_at = CURRENT_TIMESTAMP WHERE id = ? AND cart_id = ? AND deleted_at IS NULL"
        )
        .bind(line_id)
        .bind(cart_id)
        .execute(&self.pool)
        .await?;

        self.get_cart(cart_id).await
    }
}
