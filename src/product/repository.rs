use super::models::*;
use super::types::*;
use crate::db::DbPool;
use crate::db::DbTransaction;
use crate::error::AppError;
use crate::types::{generate_entity_id, generate_handle, metadata_to_json, FindParams};

#[derive(Clone)]
pub struct ProductRepository {
    pool: DbPool,
}

impl ProductRepository {
    pub fn new(pool: DbPool) -> Self {
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

        let mut tx = self.pool.begin().await?;

        let product = sqlx::query_as::<_, Product>(
            r#"
            INSERT INTO products (id, title, handle, description, status, thumbnail, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(&product_id)
        .bind(&input.title)
        .bind(&handle)
        .bind(&input.description)
        .bind(input.status.as_ref().map(|s| s.as_str()).unwrap_or("draft"))
        .bind(&input.thumbnail)
        .bind(metadata_to_json(input.metadata.clone()))
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| Self::map_unique_violation(e, "Product", &handle))?;

        let mut option_titles: Vec<String> = Vec::new();
        if let Some(opts) = input.options {
            for opt_input in opts {
                option_titles.push(opt_input.title.clone());
                let opt_id = generate_entity_id("opt");
                sqlx::query_as::<_, ProductOption>(
                    "INSERT INTO product_options (id, product_id, title) VALUES ($1, $2, $3) RETURNING *",
                )
                .bind(&opt_id)
                .bind(&product_id)
                .bind(&opt_input.title)
                .fetch_one(&mut *tx)
                .await?;

                for val_str in opt_input.values {
                    let val_id = generate_entity_id("optval");
                    sqlx::query_as::<_, ProductOptionValue>(
                        "INSERT INTO product_option_values (id, option_id, value) VALUES ($1, $2, $3) RETURNING *",
                    )
                    .bind(&val_id)
                    .bind(&opt_id)
                    .bind(&val_str)
                    .fetch_one(&mut *tx)
                    .await?;
                }
            }
        }

        if let Some(vars) = input.variants {
            if !option_titles.is_empty() {
                let mut seen_combos: std::collections::HashSet<Vec<(String, String)>> =
                    std::collections::HashSet::new();
                for var_input in &vars {
                    if let Some(ref opts) = var_input.options {
                        let mut combo: Vec<(String, String)> =
                            opts.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                        combo.sort_by(|a, b| a.0.cmp(&b.0));
                        if !seen_combos.insert(combo.clone()) {
                            return Err(AppError::InvalidData(format!(
                                "Duplicate option combination for variant '{}'",
                                var_input.title
                            )));
                        }
                    }
                }
            }

            for (rank, var_input) in vars.into_iter().enumerate() {
                if !option_titles.is_empty() {
                    if let Some(ref opts) = var_input.options {
                        for opt_title in &option_titles {
                            if !opts.contains_key(opt_title) {
                                return Err(AppError::InvalidData(format!(
                                    "Variant '{}' is missing option '{}'",
                                    var_input.title, opt_title
                                )));
                            }
                        }
                    }
                }

                let variant =
                    Self::insert_variant_tx(&mut tx, &product_id, &var_input, rank as i64).await?;
                Self::resolve_variant_options_tx(
                    &mut tx,
                    &product_id,
                    &variant.id,
                    &var_input.options,
                )
                .await?;
            }
        }

        tx.commit().await?;
        let loaded = self.load_relations(product).await?;
        Ok(loaded)
    }

    pub async fn find_by_id(&self, id: &str) -> Result<ProductWithRelations, AppError> {
        let product = sqlx::query_as::<_, Product>(
            "SELECT * FROM products WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Product with id {} was not found", id)))?;

        self.load_relations(product).await
    }

    pub async fn find_published_by_id(&self, id: &str) -> Result<ProductWithRelations, AppError> {
        let product = sqlx::query_as::<_, Product>(
            "SELECT * FROM products WHERE id = $1 AND status = 'published' AND deleted_at IS NULL",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Product with id {} was not found", id)))?;

        self.load_relations(product).await
    }

    pub async fn find_by_id_any(&self, id: &str) -> Result<ProductWithRelations, AppError> {
        let product = sqlx::query_as::<_, Product>("SELECT * FROM products WHERE id = $1")
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
            "SELECT * FROM products p {} ORDER BY {} LIMIT $1 OFFSET $2",
            where_clause, order
        );
        let products = sqlx::query_as::<_, Product>(&query_sql)
            .bind(params.capped_limit())
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
            "SELECT * FROM products WHERE status = 'published' AND deleted_at IS NULL ORDER BY {} LIMIT $1 OFFSET $2",
            order
        );
        let products = sqlx::query_as::<_, Product>(&query_sql)
            .bind(params.capped_limit())
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
            "SELECT * FROM products WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Product with id {} was not found", id)))?;

        let handle = input.handle.as_deref().unwrap_or("");
        sqlx::query(
            r#"
            UPDATE products SET
                title = COALESCE(NULLIF($1, ''), title),
                handle = COALESCE(NULLIF($2, ''), handle),
                description = COALESCE($3, description),
                status = COALESCE(NULLIF($4, ''), status),
                thumbnail = COALESCE($5, thumbnail),
                metadata = COALESCE($6, metadata),
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $7
            "#,
        )
        .bind(&input.title)
        .bind(handle)
        .bind(&input.description)
        .bind(input.status.as_ref().map(|s| s.as_str()))
        .bind(&input.thumbnail)
        .bind(metadata_to_json(input.metadata.clone()))
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| Self::map_unique_violation(e, "Product", handle))?;

        self.find_by_id_any(id).await
    }

    pub async fn soft_delete(&self, id: &str) -> Result<String, AppError> {
        let result = sqlx::query(
            "UPDATE products SET deleted_at = CURRENT_TIMESTAMP WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            let exists: Option<(i32,)> =
                sqlx::query_as("SELECT 1 FROM products WHERE id = $1 AND deleted_at IS NOT NULL")
                    .bind(id)
                    .fetch_optional(&self.pool)
                    .await?;
            if exists.is_some() {
                return Ok(id.to_string());
            }
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
        let mut tx = self.pool.begin().await?;

        let _product = sqlx::query_as::<_, Product>(
            "SELECT * FROM products WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(product_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!("Product with id {} was not found", product_id))
        })?;

        let rank: (i64,) = sqlx::query_as(
            "SELECT COALESCE(MAX(variant_rank), -1) + 1 FROM product_variants WHERE product_id = $1 AND deleted_at IS NULL",
        )
        .bind(product_id)
        .fetch_one(&mut *tx)
        .await?;

        let variant = Self::insert_variant_tx(&mut tx, product_id, input, rank.0).await?;
        Self::resolve_variant_options_tx(&mut tx, product_id, &variant.id, &input.options).await?;

        tx.commit().await?;
        self.find_by_id_any(product_id).await
    }

    async fn insert_variant_tx(
        tx: &mut DbTransaction<'_>,
        product_id: &str,
        input: &CreateProductVariantInput,
        rank: i64,
    ) -> Result<ProductVariant, AppError> {
        let var_id = generate_entity_id("variant");
        sqlx::query_as::<_, ProductVariant>(
            r#"
            INSERT INTO product_variants (id, product_id, title, sku, price, variant_rank, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(&var_id)
        .bind(product_id)
        .bind(&input.title)
        .bind(&input.sku)
        .bind(input.price)
        .bind(rank)
        .bind(metadata_to_json(input.metadata.clone()))
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| {
            if crate::db::is_unique_violation(&e) {
                return AppError::DuplicateError(format!(
                    "Variant with SKU '{}' already exists",
                    input.sku.as_deref().unwrap_or("")
                ));
            }
            AppError::DatabaseError(e)
        })
    }

    async fn resolve_variant_options_tx(
        tx: &mut DbTransaction<'_>,
        product_id: &str,
        variant_id: &str,
        options_map: &Option<std::collections::HashMap<String, String>>,
    ) -> Result<(), AppError> {
        let Some(map) = options_map else {
            return Ok(());
        };

        for (opt_title, val_str) in map {
            let val = sqlx::query_as::<_, ProductOptionValue>(
                r#"
                SELECT pov.* FROM product_option_values pov
                JOIN product_options po ON pov.option_id = po.id
                WHERE po.product_id = $1 AND po.title = $2 AND pov.value = $3
                "#,
            )
            .bind(product_id)
            .bind(opt_title)
            .bind(val_str)
            .fetch_optional(&mut **tx)
            .await?;

            let val = val.ok_or_else(|| {
                AppError::NotFound(format!(
                    "Option value '{}' not found for option '{}'",
                    val_str, opt_title
                ))
            })?;

            sqlx::query(
                "INSERT INTO product_variant_option (id, variant_id, option_value_id) VALUES ($1, $2, $3)",
            )
            .bind(generate_entity_id("pvo"))
            .bind(variant_id)
            .bind(&val.id)
            .execute(&mut **tx)
            .await?;
        }

        Ok(())
    }

    async fn load_relations(&self, product: Product) -> Result<ProductWithRelations, AppError> {
        let options = sqlx::query_as::<_, ProductOption>(
            "SELECT * FROM product_options WHERE product_id = $1 AND deleted_at IS NULL",
        )
        .bind(&product.id)
        .fetch_all(&self.pool)
        .await?;

        let mut options_with_values = Vec::with_capacity(options.len());
        for opt in &options {
            let values = sqlx::query_as::<_, ProductOptionValue>(
                "SELECT * FROM product_option_values WHERE option_id = $1 AND deleted_at IS NULL",
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
            "SELECT * FROM product_variants WHERE product_id = $1 AND deleted_at IS NULL",
        )
        .bind(&product.id)
        .fetch_all(&self.pool)
        .await?;

        let mut variants_with_options = Vec::with_capacity(variants.len());
        for v in &variants {
            let opts = sqlx::query_as::<_, VariantOptionValue>(
                r#"
                SELECT pov.id, pov.value, pov.option_id
                FROM product_variant_option pvo
                JOIN product_option_values pov ON pvo.option_value_id = pov.id
                WHERE pvo.variant_id = $1
                  AND pov.deleted_at IS NULL
                "#,
            )
            .bind(&v.id)
            .fetch_all(&self.pool)
            .await?;
            variants_with_options.push(ProductVariantWithOptions {
                variant: v.clone(),
                options: opts,
                calculated_price: super::models::CalculatedPrice {
                    calculated_amount: v.price,
                    original_amount: v.price,
                    is_calculated_price_tax_inclusive: false,
                },
            });
        }

        Ok(ProductWithRelations {
            product,
            options: options_with_values,
            variants: variants_with_options,
            images: vec![],
            is_giftcard: false,
            discountable: true,
        })
    }

    fn map_unique_violation(e: sqlx::Error, entity: &str, handle: &str) -> AppError {
        if crate::db::is_unique_violation(&e) {
            return AppError::DuplicateError(format!(
                "{} with handle '{}' already exists",
                entity, handle
            ));
        }
        AppError::DatabaseError(e)
    }
}
