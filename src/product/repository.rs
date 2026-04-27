use super::models::*;
use super::types::*;
use crate::db::DbPool;
use crate::db::DbTransaction;
use crate::error::AppError;
use crate::types::{generate_entity_id, generate_handle, metadata_to_json, FindParams};

#[derive(Clone)]
pub struct ProductRepository {
    pool: DbPool,
    default_currency_code: String,
}

impl ProductRepository {
    pub fn new(pool: DbPool, default_currency_code: String) -> Self {
        Self {
            pool,
            default_currency_code,
        }
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
            INSERT INTO products (id, title, handle, description, subtitle, status, thumbnail, metadata,
                                  is_giftcard, discountable)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING *
            "#,
        )
        .bind(&product_id)
        .bind(&input.title)
        .bind(&handle)
        .bind(&input.description)
        .bind(&input.subtitle)
        .bind(input.status.as_ref().map(|s| s.as_str()).unwrap_or("draft"))
        .bind(&input.thumbnail)
        .bind(metadata_to_json(input.metadata.clone()))
        .bind(input.is_giftcard.unwrap_or(false))
        .bind(input.discountable.unwrap_or(true))
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
                    let opts = var_input.options.as_ref().ok_or_else(|| {
                        AppError::InvalidData(format!(
                            "Variant '{}' must specify options for all product option titles",
                            var_input.title
                        ))
                    })?;
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

            for (rank, var_input) in vars.into_iter().enumerate() {
                if !option_titles.is_empty() {
                    let opts = var_input.options.as_ref().ok_or_else(|| {
                        AppError::InvalidData(format!(
                            "Variant '{}' must specify options for all product option titles",
                            var_input.title
                        ))
                    })?;
                    for opt_title in &option_titles {
                        if !opts.contains_key(opt_title) {
                            return Err(AppError::InvalidData(format!(
                                "Variant '{}' is missing option '{}'",
                                var_input.title, opt_title
                            )));
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
        let order = crate::types::validate_order_param(order).map_err(AppError::InvalidData)?;

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
        let order = crate::types::validate_order_param(order).map_err(AppError::InvalidData)?;
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
        let is_giftcard = input.is_giftcard;
        let discountable = input.discountable;
        sqlx::query(
            r#"
            UPDATE products SET
                title = COALESCE(NULLIF($1, ''), title),
                handle = COALESCE(NULLIF($2, ''), handle),
                description = COALESCE($3, description),
                subtitle = COALESCE($4, subtitle),
                status = COALESCE(NULLIF($5, ''), status),
                thumbnail = COALESCE($6, thumbnail),
                metadata = COALESCE($7, metadata),
                is_giftcard = COALESCE($8, is_giftcard),
                discountable = COALESCE($9, discountable),
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $10
            "#,
        )
        .bind(&input.title)
        .bind(handle)
        .bind(&input.description)
        .bind(&input.subtitle)
        .bind(input.status.as_ref().map(|s| s.as_str()))
        .bind(&input.thumbnail)
        .bind(metadata_to_json(input.metadata.clone()))
        .bind(is_giftcard)
        .bind(discountable)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| Self::map_unique_violation(e, "Product", handle))?;

        self.find_by_id_any(id).await
    }

    pub async fn soft_delete(&self, id: &str) -> Result<String, AppError> {
        let mut tx = self.pool.begin().await?;

        let result = sqlx::query(
            "UPDATE products SET deleted_at = CURRENT_TIMESTAMP WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() == 0 {
            let exists: Option<(i32,)> =
                sqlx::query_as("SELECT 1 FROM products WHERE id = $1 AND deleted_at IS NOT NULL")
                    .bind(id)
                    .fetch_optional(&mut *tx)
                    .await?;
            if exists.is_some() {
                tx.rollback().await?;
                return Ok(id.to_string());
            }
            tx.rollback().await?;
            return Err(AppError::NotFound(format!(
                "Product with id {} was not found",
                id
            )));
        }

        sqlx::query(
            "UPDATE product_variants SET deleted_at = CURRENT_TIMESTAMP WHERE product_id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "UPDATE product_options SET deleted_at = CURRENT_TIMESTAMP WHERE product_id = $1 AND deleted_at IS NULL",
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            UPDATE product_option_values SET deleted_at = CURRENT_TIMESTAMP
            WHERE option_id IN (SELECT id FROM product_options WHERE product_id = $1)
              AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            DELETE FROM product_variant_option
            WHERE variant_id IN (SELECT id FROM product_variants WHERE product_id = $1)
            "#,
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
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

        let defined_options: Vec<String> = sqlx::query_scalar(
            "SELECT title FROM product_options WHERE product_id = $1 AND deleted_at IS NULL",
        )
        .bind(product_id)
        .fetch_all(&mut *tx)
        .await?;

        if !defined_options.is_empty() {
            let opts = input.options.as_ref().ok_or_else(|| {
                AppError::InvalidData(
                    "Variant must specify options for all product option titles".into(),
                )
            })?;
            for opt_title in &defined_options {
                if !opts.contains_key(opt_title) {
                    return Err(AppError::InvalidData(format!(
                        "Variant is missing option '{}'",
                        opt_title
                    )));
                }
            }
            Self::check_db_variant_option_combo(&mut tx, product_id, opts).await?;
        }

        let rank: i64 = if let Some(r) = input.variant_rank {
            r
        } else {
            let computed: (i64,) = sqlx::query_as(
                "SELECT COALESCE(MAX(variant_rank), -1) + 1 FROM product_variants WHERE product_id = $1 AND deleted_at IS NULL",
            )
            .bind(product_id)
            .fetch_one(&mut *tx)
            .await?;
            computed.0
        };

        let variant = Self::insert_variant_tx(&mut tx, product_id, input, rank).await?;
        Self::resolve_variant_options_tx(&mut tx, product_id, &variant.id, &input.options).await?;

        tx.commit().await?;
        self.find_by_id_any(product_id).await
    }

    pub async fn list_variants(
        &self,
        product_id: &str,
        params: &FindParams,
    ) -> Result<(Vec<ProductVariantWithOptions>, i64), AppError> {
        let _product = sqlx::query_as::<_, Product>(
            "SELECT * FROM products WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(product_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!("Product with id {} was not found", product_id))
        })?;

        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM product_variants WHERE product_id = $1 AND deleted_at IS NULL",
        )
        .bind(product_id)
        .fetch_one(&self.pool)
        .await?;

        let order = params.order.as_deref().unwrap_or("variant_rank ASC");
        let order = crate::types::validate_order_param(order).map_err(AppError::InvalidData)?;
        let query_sql = format!(
            "SELECT * FROM product_variants WHERE product_id = $1 AND deleted_at IS NULL ORDER BY {} LIMIT $2 OFFSET $3",
            order
        );
        let variants = sqlx::query_as::<_, ProductVariant>(&query_sql)
            .bind(product_id)
            .bind(params.capped_limit())
            .bind(params.offset)
            .fetch_all(&self.pool)
            .await?;

        let mut results = Vec::with_capacity(variants.len());
        for v in &variants {
            let opts = Self::load_variant_options(&self.pool, &v.id).await?;
            results.push(ProductVariantWithOptions {
                variant: v.clone(),
                options: opts,
                calculated_price: super::models::CalculatedPrice {
                    calculated_amount: v.price,
                    original_amount: v.price,
                    is_calculated_price_tax_inclusive: false,
                    currency_code: self.default_currency_code.clone(),
                },
            });
        }

        Ok((results, count.0))
    }

    pub async fn get_variant(
        &self,
        product_id: &str,
        variant_id: &str,
    ) -> Result<ProductVariantWithOptions, AppError> {
        let _product = sqlx::query_as::<_, Product>(
            "SELECT * FROM products WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(product_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!("Product with id {} was not found", product_id))
        })?;

        let variant = sqlx::query_as::<_, ProductVariant>(
            "SELECT * FROM product_variants WHERE id = $1 AND product_id = $2 AND deleted_at IS NULL",
        )
        .bind(variant_id)
        .bind(product_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!(
                "Variant with id {} was not found",
                variant_id
            ))
        })?;

        let opts = Self::load_variant_options(&self.pool, &variant.id).await?;
        Ok(ProductVariantWithOptions {
            calculated_price: super::models::CalculatedPrice {
                calculated_amount: variant.price,
                original_amount: variant.price,
                is_calculated_price_tax_inclusive: false,
                currency_code: self.default_currency_code.clone(),
            },
            variant,
            options: opts,
        })
    }

    pub async fn update_variant(
        &self,
        product_id: &str,
        variant_id: &str,
        input: &UpdateVariantInput,
    ) -> Result<ProductVariantWithOptions, AppError> {
        let _product = sqlx::query_as::<_, Product>(
            "SELECT * FROM products WHERE id = $1 AND deleted_at IS NULL",
        )
        .bind(product_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!("Product with id {} was not found", product_id))
        })?;

        let _existing = sqlx::query_as::<_, ProductVariant>(
            "SELECT * FROM product_variants WHERE id = $1 AND product_id = $2 AND deleted_at IS NULL",
        )
        .bind(variant_id)
        .bind(product_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| {
            AppError::NotFound(format!(
                "Variant with id {} was not found",
                variant_id
            ))
        })?;

        sqlx::query(
            r#"
            UPDATE product_variants SET
                title = COALESCE($1, title),
                sku = COALESCE($2, sku),
                price = COALESCE($3, price),
                metadata = COALESCE($4, metadata),
                updated_at = CURRENT_TIMESTAMP
            WHERE id = $5
            "#,
        )
        .bind(&input.title)
        .bind(&input.sku)
        .bind(input.price)
        .bind(metadata_to_json(input.metadata.clone()))
        .bind(variant_id)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            if crate::db::is_unique_violation(&e) {
                return AppError::DuplicateError(format!(
                    "Variant with SKU '{}' already exists",
                    input.sku.as_deref().unwrap_or("")
                ));
            }
            AppError::DatabaseError(e)
        })?;

        self.get_variant(product_id, variant_id).await
    }

    pub async fn soft_delete_variant(
        &self,
        product_id: &str,
        variant_id: &str,
    ) -> Result<(String, ProductWithRelations), AppError> {
        let _product = self.find_by_id(product_id).await?;

        let mut tx = self.pool.begin().await?;

        let result = sqlx::query(
            "UPDATE product_variants SET deleted_at = CURRENT_TIMESTAMP WHERE id = $1 AND product_id = $2 AND deleted_at IS NULL",
        )
        .bind(variant_id)
        .bind(product_id)
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() == 0 {
            let exists: Option<(i32,)> = sqlx::query_as(
                "SELECT 1 FROM product_variants WHERE id = $1 AND deleted_at IS NOT NULL",
            )
            .bind(variant_id)
            .fetch_optional(&mut *tx)
            .await?;
            if exists.is_some() {
                tx.rollback().await?;
                let parent = self.find_by_id_any(product_id).await?;
                return Ok((variant_id.to_string(), parent));
            }
            tx.rollback().await?;
            return Err(AppError::NotFound(format!(
                "Variant with id {} was not found",
                variant_id
            )));
        }

        sqlx::query("DELETE FROM product_variant_option WHERE variant_id = $1")
            .bind(variant_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        let parent = self.find_by_id_any(product_id).await?;
        Ok((variant_id.to_string(), parent))
    }

    async fn check_db_variant_option_combo(
        tx: &mut DbTransaction<'_>,
        product_id: &str,
        new_opts: &std::collections::HashMap<String, String>,
    ) -> Result<(), AppError> {
        let existing_rows: Vec<(String, String, String)> = sqlx::query_as(
            r#"
            SELECT v.id, po.title, pov.value
            FROM product_variants v
            JOIN product_variant_option pvo ON pvo.variant_id = v.id
            JOIN product_option_values pov ON pvo.option_value_id = pov.id
            JOIN product_options po ON pov.option_id = po.id
            WHERE v.product_id = $1 AND v.deleted_at IS NULL AND pov.deleted_at IS NULL
            "#,
        )
        .bind(product_id)
        .fetch_all(&mut **tx)
        .await?;

        let mut variant_combos: std::collections::HashMap<
            String,
            std::collections::HashSet<(String, String)>,
        > = std::collections::HashMap::new();
        for (variant_id, opt_title, val) in &existing_rows {
            variant_combos
                .entry(variant_id.clone())
                .or_default()
                .insert((opt_title.clone(), val.clone()));
        }

        let new_combo: std::collections::HashSet<(String, String)> = new_opts
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        for (variant_id, combo) in &variant_combos {
            if combo == &new_combo {
                return Err(AppError::DuplicateError(format!(
                    "Variant with the same option combination already exists (variant {})",
                    variant_id
                )));
            }
        }

        Ok(())
    }

    async fn load_variant_options(
        pool: &DbPool,
        variant_id: &str,
    ) -> Result<Vec<VariantOptionValue>, AppError> {
        let rows = sqlx::query(
            r#"
            SELECT pov.id, pov.value, po.id AS option_id, po.title AS option_title
            FROM product_variant_option pvo
            JOIN product_option_values pov ON pvo.option_value_id = pov.id
            JOIN product_options po ON pov.option_id = po.id
            WHERE pvo.variant_id = $1
              AND pov.deleted_at IS NULL
              AND po.deleted_at IS NULL
            "#,
        )
        .bind(variant_id)
        .fetch_all(pool)
        .await
        .map_err(AppError::DatabaseError)?;

        let mut result = Vec::with_capacity(rows.len());
        for row in &rows {
            let id: String = sqlx::Row::get(row, "id");
            let value: String = sqlx::Row::get(row, "value");
            let option_id: String = sqlx::Row::get(row, "option_id");
            let option_title: String = sqlx::Row::get(row, "option_title");
            result.push(VariantOptionValue {
                id,
                value,
                option: super::models::NestedOption {
                    id: option_id,
                    title: option_title,
                },
            });
        }
        Ok(result)
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
            let opts = Self::load_variant_options(&self.pool, &v.id).await?;
            variants_with_options.push(ProductVariantWithOptions {
                variant: v.clone(),
                options: opts,
                calculated_price: super::models::CalculatedPrice {
                    calculated_amount: v.price,
                    original_amount: v.price,
                    is_calculated_price_tax_inclusive: false,
                    currency_code: self.default_currency_code.clone(),
                },
            });
        }

        Ok(ProductWithRelations {
            product,
            options: options_with_values,
            variants: variants_with_options,
            images: vec![],
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
