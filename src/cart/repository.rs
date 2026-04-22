use super::models::*;
use super::types::*;
use crate::db::DbPool;
use crate::error::AppError;
use crate::types::{generate_entity_id, metadata_to_json};

#[derive(Clone)]
pub struct CartRepository {
    pool: DbPool,
    default_currency_code: String,
}

impl CartRepository {
    pub fn new(pool: DbPool, default_currency_code: String) -> Self {
        Self {
            pool,
            default_currency_code,
        }
    }

    pub async fn create_cart(&self, input: CreateCartInput) -> Result<CartWithItems, AppError> {
        let cart_id = generate_entity_id("cart");
        let currency = input
            .currency_code
            .unwrap_or_else(|| self.default_currency_code.clone())
            .to_lowercase();

        let cart = sqlx::query_as::<_, Cart>(
            r#"
            INSERT INTO carts (id, customer_id, email, currency_code, metadata)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(&cart_id)
        .bind(&input.customer_id)
        .bind(&input.email)
        .bind(&currency)
        .bind(metadata_to_json(input.metadata))
        .fetch_one(&self.pool)
        .await?;

        Ok(CartWithItems::from_items(cart, vec![]))
    }

    pub async fn get_cart(&self, cart_id: &str) -> Result<CartWithItems, AppError> {
        let cart = sqlx::query_as::<_, Cart>(
            r#"SELECT * FROM carts WHERE id = $1 AND deleted_at IS NULL"#,
        )
        .bind(cart_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Cart not found".into()))?;

        let items = sqlx::query_as::<_, CartLineItem>(
            r#"SELECT * FROM cart_line_items WHERE cart_id = $1 AND deleted_at IS NULL"#,
        )
        .bind(cart_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(CartWithItems::from_items(cart, items))
    }

    pub async fn update_cart(
        &self,
        cart_id: &str,
        input: UpdateCartInput,
    ) -> Result<CartWithItems, AppError> {
        let cart = sqlx::query_as::<_, Cart>(
            r#"SELECT * FROM carts WHERE id = $1 AND deleted_at IS NULL"#,
        )
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
                email = COALESCE($1, email),
                customer_id = COALESCE($2, customer_id),
                metadata = COALESCE($3, metadata),
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $4 AND deleted_at IS NULL
            "#,
        )
        .bind(&input.email)
        .bind(&input.customer_id)
        .bind(metadata_to_json(input.metadata))
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

        let cart =
            sqlx::query_as::<_, Cart>("SELECT * FROM carts WHERE id = $1 AND deleted_at IS NULL")
                .bind(cart_id)
                .fetch_optional(&mut *tx)
                .await?
                .ok_or_else(|| AppError::NotFound("Cart not found".into()))?;

        if cart.completed_at.is_some() {
            return Err(AppError::Conflict(
                "Cannot add items to a completed cart".into(),
            ));
        }

        let row = sqlx::query(
            r#"
            SELECT v.id as variant_id, v.title as variant_title, v.sku as variant_sku, v.price,
                   p.id as product_id, p.title as product_title,
                   p.description as product_description, p.handle as product_handle
            FROM product_variants v
            JOIN products p ON p.id = v.product_id
            WHERE v.id = $1 AND v.deleted_at IS NULL AND p.deleted_at IS NULL
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
        let product_description: Option<String> = sqlx::Row::get(&row, "product_description");
        let product_handle: Option<String> = sqlx::Row::get(&row, "product_handle");

        let option_rows = sqlx::query(
            r#"
            SELECT po.title AS opt_title, pov.value AS opt_value
            FROM product_variant_option pvo
            JOIN product_option_values pov ON pvo.option_value_id = pov.id
            JOIN product_options po ON pov.option_id = po.id
            WHERE pvo.variant_id = $1
              AND pov.deleted_at IS NULL
              AND po.deleted_at IS NULL
            "#,
        )
        .bind(&variant_id)
        .fetch_all(&mut *tx)
        .await?;

        let variant_option_values: serde_json::Map<String, serde_json::Value> = option_rows
            .iter()
            .map(|r| {
                let title: String = sqlx::Row::get(r, "opt_title");
                let value: String = sqlx::Row::get(r, "opt_value");
                (title, serde_json::Value::String(value))
            })
            .collect();

        let line_id = generate_entity_id("cali");
        let snapshot = serde_json::json!({
            "product_title": product_title,
            "product_description": product_description,
            "product_handle": product_handle,
            "variant_title": variant_title,
            "variant_sku": variant_sku,
            "variant_option_values": variant_option_values
        });

        let input_metadata = metadata_to_json(input.metadata.clone());

        let existing = sqlx::query("SELECT id, quantity, metadata FROM cart_line_items WHERE cart_id = $1 AND variant_id = $2 AND unit_price = $3 AND deleted_at IS NULL")
            .bind(cart_id)
            .bind(&input.variant_id)
            .bind(price)
            .fetch_optional(&mut *tx)
            .await?;

        let metadata_matches = if let Some(ref ext) = existing {
            let ext_meta: Option<sqlx::types::Json<serde_json::Value>> =
                sqlx::Row::get(ext, "metadata");
            match (&ext_meta, &input_metadata) {
                (None, None) => true,
                (Some(a), Some(b)) => a.0 == b.0,
                _ => false,
            }
        } else {
            false
        };

        if let Some(ext) = existing {
            if metadata_matches {
                let ext_id: String = sqlx::Row::get(&ext, "id");
                sqlx::query("UPDATE cart_line_items SET quantity = quantity + $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
                    .bind(input.quantity)
                    .bind(&ext_id)
                    .execute(&mut *tx)
                    .await?;
            } else {
                sqlx::query(
                    r#"
                    INSERT INTO cart_line_items (id, cart_id, title, quantity, unit_price, variant_id, product_id, snapshot, metadata)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
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
                .bind(&input_metadata)
                .execute(&mut *tx)
                .await?;
            }
        } else {
            sqlx::query(
                r#"
                INSERT INTO cart_line_items (id, cart_id, title, quantity, unit_price, variant_id, product_id, snapshot, metadata)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
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
            .bind(&input_metadata)
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

        let cart =
            sqlx::query_as::<_, Cart>("SELECT * FROM carts WHERE id = $1 AND deleted_at IS NULL")
                .bind(cart_id)
                .fetch_optional(&self.pool)
                .await?
                .ok_or_else(|| AppError::NotFound("Cart not found".into()))?;

        if cart.completed_at.is_some() {
            return Err(AppError::Conflict(
                "Cannot update items in a completed cart".into(),
            ));
        }

        sqlx::query(
            r#"
            UPDATE cart_line_items 
            SET quantity = $1, 
                metadata = COALESCE($2, metadata),
                updated_at = CURRENT_TIMESTAMP 
            WHERE id = $3 AND cart_id = $4 AND deleted_at IS NULL
            "#,
        )
        .bind(input.quantity)
        .bind(metadata_to_json(input.metadata))
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
        let cart =
            sqlx::query_as::<_, Cart>("SELECT * FROM carts WHERE id = $1 AND deleted_at IS NULL")
                .bind(cart_id)
                .fetch_optional(&self.pool)
                .await?
                .ok_or_else(|| AppError::NotFound("Cart not found".into()))?;

        if cart.completed_at.is_some() {
            return Err(AppError::Conflict(
                "Cannot delete items from a completed cart".into(),
            ));
        }

        sqlx::query(
            "UPDATE cart_line_items SET deleted_at = CURRENT_TIMESTAMP WHERE id = $1 AND cart_id = $2 AND deleted_at IS NULL"
        )
        .bind(line_id)
        .bind(cart_id)
        .execute(&self.pool)
        .await?;

        self.get_cart(cart_id).await
    }

    pub async fn mark_completed(&self, cart_id: &str) -> Result<(), AppError> {
        let result = sqlx::query(
            "UPDATE carts SET completed_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE id = $1 AND deleted_at IS NULL AND completed_at IS NULL",
        )
        .bind(cart_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(
                "Cart not found or already completed".into(),
            ));
        }

        Ok(())
    }
}
