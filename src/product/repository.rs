use super::models::*;
use super::types::*;
use crate::error::AppError;
use crate::types::{generate_entity_id, generate_handle, FindParams};
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct ProductRepository {
    pool: SqlitePool,
}

impl ProductRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create_product(
        &self,
        input: CreateProductInput,
    ) -> Result<ProductWithRelations, AppError> {
        let product_id = generate_entity_id("prod");
        let handle = input
            .handle
            .unwrap_or_else(|| generate_handle(&input.title));

        let product = sqlx::query_as::<_, Product>(
            r#"
            INSERT INTO products (id, title, handle, description, status, thumbnail, metadata)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            RETURNING *
            "#,
        )
        .bind(&product_id)
        .bind(&input.title)
        .bind(&handle)
        .bind(&input.description)
        .bind(input.status.as_deref().unwrap_or("draft"))
        .bind(&input.thumbnail)
        .bind(input.metadata.clone().map(sqlx::types::Json))
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Self::map_unique_violation(e, "Product", &handle))?;

        let mut options_out = Vec::new();
        if let Some(opts) = input.options {
            for opt_input in opts {
                let opt_id = generate_entity_id("opt");
                let option = sqlx::query_as::<_, ProductOption>(
                    "INSERT INTO product_options (id, product_id, title) VALUES (?, ?, ?) RETURNING *",
                )
                .bind(&opt_id)
                .bind(&product_id)
                .bind(&opt_input.title)
                .fetch_one(&self.pool)
                .await?;

                let mut values_out = Vec::new();
                for val_str in opt_input.values {
                    let val_id = generate_entity_id("optval");
                    let val = sqlx::query_as::<_, ProductOptionValue>(
                        "INSERT INTO product_option_values (id, option_id, value) VALUES (?, ?, ?) RETURNING *",
                    )
                    .bind(&val_id)
                    .bind(&opt_id)
                    .bind(&val_str)
                    .fetch_one(&self.pool)
                    .await?;
                    values_out.push(val);
                }
                options_out.push(ProductOptionWithValues {
                    option,
                    values: values_out,
                });
            }
        }

        if let Some(vars) = input.variants {
            for (rank, var_input) in vars.into_iter().enumerate() {
                self.insert_variant(&product_id, &var_input, rank as i64)
                    .await?;
                self.resolve_variant_options(&product_id, &var_input, &var_input.options)
                    .await?;
            }
        }

        let loaded = self.load_relations(product.clone()).await?;

        Ok(loaded)
    }

    pub async fn find_by_id(&self, id: &str) -> Result<ProductWithRelations, AppError> {
        let product = sqlx::query_as::<_, Product>(
            "SELECT * FROM products WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Product with id {} was not found", id)))?;

        self.load_relations(product).await
    }

    pub async fn find_published_by_id(&self, id: &str) -> Result<ProductWithRelations, AppError> {
        let product = sqlx::query_as::<_, Product>(
            "SELECT * FROM products WHERE id = ? AND status = 'published' AND deleted_at IS NULL",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Product with id {} was not found", id)))?;

        self.load_relations(product).await
    }

    pub async fn find_by_id_any(&self, id: &str) -> Result<ProductWithRelations, AppError> {
        let product = sqlx::query_as::<_, Product>("SELECT * FROM products WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("Product with id {} was not found", id)))?;

        self.load_relations(product).await
    }

    pub async fn list(
        &self,
        params: &FindParams,
    ) -> Result<(Vec<ProductWithRelations>, i64), AppError> {
        let where_clause = if params.with_deleted == Some(true) {
            ""
        } else {
            "WHERE p.deleted_at IS NULL"
        };
        let order = params.order.as_deref().unwrap_or("p.created_at DESC");

        let count_sql = format!("SELECT COUNT(*) as count FROM products p {}", where_clause);
        let count: (i64,) = sqlx::query_as(&count_sql).fetch_one(&self.pool).await?;

        let query_sql = format!(
            "SELECT * FROM products p {} ORDER BY {} LIMIT ? OFFSET ?",
            where_clause, order
        );
        let products = sqlx::query_as::<_, Product>(&query_sql)
            .bind(params.limit)
            .bind(params.offset)
            .fetch_all(&self.pool)
            .await?;

        let mut results = Vec::with_capacity(products.len());
        for product in products {
            results.push(self.load_relations(product).await?);
        }

        Ok((results, count.0))
    }

    pub async fn list_published(
        &self,
        params: &FindParams,
    ) -> Result<(Vec<ProductWithRelations>, i64), AppError> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM products WHERE status = 'published' AND deleted_at IS NULL",
        )
        .fetch_one(&self.pool)
        .await?;

        let order = params.order.as_deref().unwrap_or("created_at DESC");
        let query_sql = format!(
            "SELECT * FROM products WHERE status = 'published' AND deleted_at IS NULL ORDER BY {} LIMIT ? OFFSET ?",
            order
        );
        let products = sqlx::query_as::<_, Product>(&query_sql)
            .bind(params.limit)
            .bind(params.offset)
            .fetch_all(&self.pool)
            .await?;

        let mut results = Vec::with_capacity(products.len());
        for product in products {
            results.push(self.load_relations(product).await?);
        }

        Ok((results, count.0))
    }

    pub async fn update(
        &self,
        id: &str,
        input: &UpdateProductInput,
    ) -> Result<ProductWithRelations, AppError> {
        let _existing = sqlx::query_as::<_, Product>(
            "SELECT * FROM products WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Product with id {} was not found", id)))?;

        let handle = input.handle.as_deref().unwrap_or("");
        sqlx::query(
            r#"
            UPDATE products SET
                title = COALESCE(NULLIF(?, ''), title),
                handle = COALESCE(NULLIF(?, ''), handle),
                description = COALESCE(?, description),
                status = COALESCE(NULLIF(?, ''), status),
                thumbnail = COALESCE(?, thumbnail),
                metadata = COALESCE(?, metadata),
                updated_at = CURRENT_TIMESTAMP
            WHERE id = ?
            "#,
        )
        .bind(&input.title)
        .bind(handle)
        .bind(&input.description)
        .bind(&input.status)
        .bind(&input.thumbnail)
        .bind(input.metadata.clone().map(sqlx::types::Json))
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| Self::map_unique_violation(e, "Product", handle))?;

        self.find_by_id_any(id).await
    }

    pub async fn soft_delete(&self, id: &str) -> Result<String, AppError> {
        let result = sqlx::query(
            "UPDATE products SET deleted_at = CURRENT_TIMESTAMP WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "Product with id {} was not found",
                id
            )));
        }

        Ok(id.to_string())
    }

    pub async fn add_variant(
        &self,
        product_id: &str,
        input: &CreateProductVariantInput,
    ) -> Result<ProductWithRelations, AppError> {
        let _product = sqlx::query_as::<_, Product>(
            "SELECT * FROM products WHERE id = ? AND deleted_at IS NULL",
        )
        .bind(product_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!("Product with id {} was not found", product_id))
        })?;

        let rank: (i64,) = sqlx::query_as(
            "SELECT COALESCE(MAX(variant_rank), -1) + 1 FROM product_variants WHERE product_id = ?",
        )
        .bind(product_id)
        .fetch_one(&self.pool)
        .await?;

        self.insert_variant(product_id, input, rank.0).await?;
        self.resolve_variant_options(product_id, input, &input.options)
            .await?;

        self.find_by_id_any(product_id).await
    }

    async fn insert_variant(
        &self,
        product_id: &str,
        input: &CreateProductVariantInput,
        rank: i64,
    ) -> Result<ProductVariant, AppError> {
        let var_id = generate_entity_id("variant");
        sqlx::query_as::<_, ProductVariant>(
            r#"
            INSERT INTO product_variants (id, product_id, title, sku, price, variant_rank, metadata)
            VALUES (?, ?, ?, ?, CAST(? AS INTEGER), CAST(? AS INTEGER), ?)
            RETURNING *
            "#,
        )
        .bind(&var_id)
        .bind(product_id)
        .bind(&input.title)
        .bind(&input.sku)
        .bind(input.price)
        .bind(rank)
        .bind(input.metadata.clone().map(sqlx::types::Json))
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(ref db_err) = e {
                if db_err.message().contains("UNIQUE") {
                    return AppError::DuplicateError(format!(
                        "Variant with SKU '{}' already exists",
                        input.sku.as_deref().unwrap_or("")
                    ));
                }
            }
            AppError::DatabaseError(e)
        })
    }

    async fn resolve_variant_options(
        &self,
        product_id: &str,
        input: &CreateProductVariantInput,
        options_map: &Option<std::collections::HashMap<String, String>>,
    ) -> Result<(), AppError> {
        let Some(map) = options_map else {
            return Ok(());
        };

        let variant_id: (String,) = sqlx::query_as(
            "SELECT id FROM product_variants WHERE product_id = ? AND title = ? ORDER BY created_at DESC LIMIT 1",
        )
        .bind(product_id)
        .bind(&input.title)
        .fetch_one(&self.pool)
        .await
        .map_err(|_| AppError::NotFound(format!("Variant '{}' not found", input.title)))?;

        for (opt_title, val_str) in map {
            let val = sqlx::query_as::<_, ProductOptionValue>(
                r#"
                SELECT pov.* FROM product_option_values pov
                JOIN product_options po ON pov.option_id = po.id
                WHERE po.product_id = ? AND po.title = ? AND pov.value = ?
                "#,
            )
            .bind(product_id)
            .bind(opt_title)
            .bind(val_str)
            .fetch_optional(&self.pool)
            .await?;

            if let Some(val) = val {
                sqlx::query(
                    "INSERT INTO product_variant_options (id, variant_id, option_value_id) VALUES (?, ?, ?)",
                )
                .bind(generate_entity_id("pvo"))
                .bind(&variant_id.0)
                .bind(&val.id)
                .execute(&self.pool)
                .await?;
            }
        }

        Ok(())
    }

    async fn load_relations(&self, product: Product) -> Result<ProductWithRelations, AppError> {
        let options = sqlx::query_as::<_, ProductOption>(
            "SELECT * FROM product_options WHERE product_id = ?",
        )
        .bind(&product.id)
        .fetch_all(&self.pool)
        .await?;

        let mut options_with_values = Vec::with_capacity(options.len());
        for opt in &options {
            let values = sqlx::query_as::<_, ProductOptionValue>(
                "SELECT * FROM product_option_values WHERE option_id = ?",
            )
            .bind(&opt.id)
            .fetch_all(&self.pool)
            .await?;
            options_with_values.push(ProductOptionWithValues {
                option: opt.clone(),
                values,
            });
        }

        let variants = sqlx::query_as::<_, ProductVariant>(
            "SELECT * FROM product_variants WHERE product_id = ?",
        )
        .bind(&product.id)
        .fetch_all(&self.pool)
        .await?;

        let mut variants_with_options = Vec::with_capacity(variants.len());
        for v in &variants {
            let opts = sqlx::query_as::<_, VariantOptionValue>(
                r#"
                SELECT pov.id, pov.value, pov.option_id
                FROM product_variant_options pvo
                JOIN product_option_values pov ON pvo.option_value_id = pov.id
                WHERE pvo.variant_id = ?
                "#,
            )
            .bind(&v.id)
            .fetch_all(&self.pool)
            .await?;
            variants_with_options.push(ProductVariantWithOptions {
                variant: v.clone(),
                options: opts,
            });
        }

        Ok(ProductWithRelations {
            product,
            options: options_with_values,
            variants: variants_with_options,
        })
    }

    fn map_unique_violation(e: sqlx::Error, entity: &str, handle: &str) -> AppError {
        if let sqlx::Error::Database(ref db_err) = e {
            if db_err.message().contains("UNIQUE") {
                return AppError::DuplicateError(format!(
                    "{} with handle '{}' already exists",
                    entity, handle
                ));
            }
        }
        AppError::DatabaseError(e)
    }
}
