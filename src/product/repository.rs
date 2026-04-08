use super::models::*;
use super::types::*;
use crate::error::AppError;
use sqlx::{PgPool, SqlitePool};
use ulid::Ulid;

#[derive(Clone)]
pub struct SqliteProductRepository {
    pool: SqlitePool,
}

impl SqliteProductRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create_product(
        &self,
        input: CreateProductInput,
    ) -> Result<ProductWithRelations, AppError> {
        let mut tx = self.pool.begin().await?;

        let product_id = format!("prod_{}", Ulid::new().to_string().to_lowercase());
        let handle = input
            .handle
            .unwrap_or_else(|| input.title.to_lowercase().replace(" ", "-"));

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
        .fetch_one(&mut *tx)
        .await?;

        let mut options_out = Vec::new();
        if let Some(opts) = input.options {
            for opt_input in opts {
                let opt_id = format!("opt_{}", Ulid::new().to_string().to_lowercase());
                let option = sqlx::query_as::<_, ProductOption>(
                    "INSERT INTO product_options (id, product_id, title) VALUES (?, ?, ?) RETURNING *"
                )
                .bind(&opt_id).bind(&product_id).bind(&opt_input.title)
                .fetch_one(&mut *tx).await?;

                let mut values_out = Vec::new();
                for val_str in opt_input.values {
                    let val_id = format!("optval_{}", Ulid::new().to_string().to_lowercase());
                    let val = sqlx::query_as::<_, ProductOptionValue>(
                        "INSERT INTO product_option_values (id, option_id, value) VALUES (?, ?, ?) RETURNING *"
                    )
                    .bind(&val_id).bind(&opt_id).bind(&val_str)
                    .fetch_one(&mut *tx).await?;
                    values_out.push(val);
                }
                options_out.push(ProductOptionWithValues {
                    option,
                    values: values_out,
                });
            }
        }

        let mut variants_out = Vec::new();
        if let Some(vars) = input.variants {
            for (rank, var_input) in vars.into_iter().enumerate() {
                let var_id = format!("variant_{}", Ulid::new().to_string().to_lowercase());
                let variant = sqlx::query_as::<_, ProductVariant>(
                    r#"
                    INSERT INTO product_variants (id, product_id, title, sku, price, variant_rank, metadata)
                    VALUES (?, ?, ?, ?, CAST(? AS INTEGER), CAST(? AS INTEGER), ?)
                    RETURNING *
                    "#
                )
                .bind(&var_id).bind(&product_id).bind(&var_input.title).bind(&var_input.sku)
                .bind(var_input.price as i64).bind(rank as i64)
                .bind(var_input.metadata.clone().map(sqlx::types::Json))
                .fetch_one(&mut *tx).await?;

                variants_out.push(ProductVariantWithOptions {
                    variant,
                    options: vec![],
                });
            }
        }

        tx.commit().await?;

        Ok(ProductWithRelations {
            product,
            options: options_out,
            variants: variants_out,
        })
    }
}

#[derive(Clone)]
pub struct PostgresProductRepository {
    pool: PgPool,
}

impl PostgresProductRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_product(
        &self,
        input: CreateProductInput,
    ) -> Result<ProductWithRelations, AppError> {
        let mut tx = self.pool.begin().await?;

        let product_id = format!("prod_{}", Ulid::new().to_string().to_lowercase());
        let handle = input
            .handle
            .unwrap_or_else(|| input.title.to_lowercase().replace(" ", "-"));

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
        .bind(input.status.as_deref().unwrap_or("draft"))
        .bind(&input.thumbnail)
        .bind(input.metadata.clone().map(sqlx::types::Json))
        .fetch_one(&mut *tx)
        .await?;

        let mut options_out = Vec::new();
        if let Some(opts) = input.options {
            for opt_input in opts {
                let opt_id = format!("opt_{}", Ulid::new().to_string().to_lowercase());
                let option = sqlx::query_as::<_, ProductOption>(
                    "INSERT INTO product_options (id, product_id, title) VALUES ($1, $2, $3) RETURNING *"
                ).bind(&opt_id).bind(&product_id).bind(&opt_input.title).fetch_one(&mut *tx).await?;

                let mut values_out = Vec::new();
                for val_str in opt_input.values {
                    let val_id = format!("optval_{}", Ulid::new().to_string().to_lowercase());
                    let val = sqlx::query_as::<_, ProductOptionValue>(
                        "INSERT INTO product_option_values (id, option_id, value) VALUES ($1, $2, $3) RETURNING *"
                    ).bind(&val_id).bind(&opt_id).bind(&val_str).fetch_one(&mut *tx).await?;
                    values_out.push(val);
                }
                options_out.push(ProductOptionWithValues {
                    option,
                    values: values_out,
                });
            }
        }

        let mut variants_out = Vec::new();
        if let Some(vars) = input.variants {
            for (rank, var_input) in vars.into_iter().enumerate() {
                let var_id = format!("variant_{}", Ulid::new().to_string().to_lowercase());
                let variant = sqlx::query_as::<_, ProductVariant>(
                    r#"
                    INSERT INTO product_variants (id, product_id, title, sku, price, variant_rank, metadata)
                    VALUES ($1, $2, $3, $4, $5, $6, $7)
                    RETURNING *
                    "#
                ).bind(&var_id).bind(&product_id).bind(&var_input.title).bind(&var_input.sku)
                .bind(var_input.price as i64).bind(rank as i64)
                .bind(var_input.metadata.clone().map(sqlx::types::Json))
                .fetch_one(&mut *tx).await?;

                variants_out.push(ProductVariantWithOptions {
                    variant,
                    options: vec![],
                });
            }
        }

        tx.commit().await?;

        Ok(ProductWithRelations {
            product,
            options: options_out,
            variants: variants_out,
        })
    }
}
