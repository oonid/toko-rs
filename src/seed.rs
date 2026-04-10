use crate::db::AppDb;
use crate::error::AppError;

pub async fn run_seed(db: &AppDb) -> Result<(), AppError> {
    let AppDb::Postgres(pool) = db;

    tracing::info!("Seeding sample data...");

    seed_products(pool).await?;
    seed_customer(pool).await?;

    tracing::info!("Seeding complete.");
    Ok(())
}

async fn seed_products(pool: &sqlx::PgPool) -> Result<(), AppError> {
    let products = [
        (
            "prod_seed_kaos_polos",
            "Kaos Polos",
            "kaos-polos",
            "Kaos polos berbahan katun combed 30s, nyaman untuk sehari-hari.",
            Some("https://example.com/kaos-polos.jpg"),
        ),
        (
            "prod_seed_jeans_slim",
            "Jeans Slim Fit",
            "jeans-slim-fit",
            "Celana jeans slim fit dengan bahan denim stretch.",
            Some("https://example.com/jeans-slim.jpg"),
        ),
        (
            "prod_seed_sneakers",
            "Sneakers Classic",
            "sneakers-classic",
            "Sepatu sneakers klasik cocok untuk olahraga dan kasual.",
            Some("https://example.com/sneakers.jpg"),
        ),
    ];

    for (id, title, handle, description, thumbnail) in &products {
        let exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM products WHERE id = $1")
            .bind(id)
            .fetch_one(pool)
            .await?;

        if exists.0 > 0 {
            tracing::info!(product = id, "Product already exists, skipping");
            continue;
        }

        sqlx::query(
            r#"
            INSERT INTO products (id, title, handle, description, status, thumbnail)
            VALUES ($1, $2, $3, $4, 'published', $5)
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(id)
        .bind(title)
        .bind(handle)
        .bind(description)
        .bind(thumbnail)
        .execute(pool)
        .await?;

        tracing::info!(product = id, "Created product");
    }

    seed_kaos_polos_variants(pool).await?;
    seed_jeans_slim_variants(pool).await?;
    seed_sneakers_variants(pool).await?;

    Ok(())
}

async fn seed_kaos_polos_variants(pool: &sqlx::PgPool) -> Result<(), AppError> {
    let product_id = "prod_seed_kaos_polos";

    let options = [("opt_seed_kaos_size", "Ukuran", &["S", "M", "L", "XL"][..])];

    for (opt_id, opt_title, values) in &options {
        sqlx::query(
            "INSERT INTO product_options (id, product_id, title) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING",
        )
        .bind(opt_id)
        .bind(product_id)
        .bind(opt_title)
        .execute(pool)
        .await?;

        for (i, val) in values.iter().enumerate() {
            let val_id = format!("optval_seed_kaos_s_{}", i);
            sqlx::query(
                "INSERT INTO product_option_values (id, option_id, value) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING",
            )
            .bind(&val_id)
            .bind(opt_id)
            .bind(val)
            .execute(pool)
            .await?;
        }
    }

    let variants = [
        (
            "var_seed_kaos_s",
            "Kaos Polos - S",
            Some("KAOS-P-S"),
            75000i64,
            "S",
        ),
        (
            "var_seed_kaos_m",
            "Kaos Polos - M",
            Some("KAOS-P-M"),
            75000,
            "M",
        ),
        (
            "var_seed_kaos_l",
            "Kaos Polos - L",
            Some("KAOS-P-L"),
            80000,
            "L",
        ),
        (
            "var_seed_kaos_xl",
            "Kaos Polos - XL",
            Some("KAOS-P-XL"),
            80000,
            "XL",
        ),
    ];

    for (rank, (var_id, var_title, sku, price, opt_val)) in variants.iter().enumerate() {
        sqlx::query(
            "INSERT INTO product_variants (id, product_id, title, sku, price, variant_rank) VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (id) DO NOTHING",
        )
        .bind(var_id)
        .bind(product_id)
        .bind(var_title)
        .bind(sku)
        .bind(price)
        .bind(rank as i64)
        .execute(pool)
        .await?;

        let val_id = sqlx::query_as::<_, (String,)>(
            "SELECT id FROM product_option_values WHERE option_id = 'opt_seed_kaos_size' AND value = $1",
        )
        .bind(opt_val)
        .fetch_optional(pool)
        .await?;

        if let Some((vid,)) = val_id {
            sqlx::query(
                "INSERT INTO product_variant_option (id, variant_id, option_value_id) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING",
            )
            .bind(format!("pvo_seed_kaos_{}", rank))
            .bind(var_id)
            .bind(&vid)
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}

async fn seed_jeans_slim_variants(pool: &sqlx::PgPool) -> Result<(), AppError> {
    let product_id = "prod_seed_jeans_slim";

    sqlx::query("INSERT INTO product_options (id, product_id, title) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING")
        .bind("opt_seed_jeans_size")
        .bind(product_id)
        .bind("Ukuran")
        .execute(pool)
        .await?;

    let sizes = ["28", "30", "32", "34"];
    for (i, val) in sizes.iter().enumerate() {
        let val_id = format!("optval_seed_jeans_s_{}", i);
        sqlx::query(
            "INSERT INTO product_option_values (id, option_id, value) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING",
        )
        .bind(&val_id)
        .bind("opt_seed_jeans_size")
        .bind(val)
        .execute(pool)
        .await?;
    }

    let variants = [
        (
            "var_seed_jeans_28",
            "Jeans Slim - 28",
            Some("JEANS-S-28"),
            250000i64,
            "28",
        ),
        (
            "var_seed_jeans_30",
            "Jeans Slim - 30",
            Some("JEANS-S-30"),
            250000,
            "30",
        ),
        (
            "var_seed_jeans_32",
            "Jeans Slim - 32",
            Some("JEANS-S-32"),
            250000,
            "32",
        ),
        (
            "var_seed_jeans_34",
            "Jeans Slim - 34",
            Some("JEANS-S-34"),
            275000,
            "34",
        ),
    ];

    for (rank, (var_id, var_title, sku, price, opt_val)) in variants.iter().enumerate() {
        sqlx::query(
            "INSERT INTO product_variants (id, product_id, title, sku, price, variant_rank) VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (id) DO NOTHING",
        )
        .bind(var_id)
        .bind(product_id)
        .bind(var_title)
        .bind(sku)
        .bind(price)
        .bind(rank as i64)
        .execute(pool)
        .await?;

        let val_id = sqlx::query_as::<_, (String,)>(
            "SELECT id FROM product_option_values WHERE option_id = 'opt_seed_jeans_size' AND value = $1",
        )
        .bind(opt_val)
        .fetch_optional(pool)
        .await?;

        if let Some((vid,)) = val_id {
            sqlx::query(
                "INSERT INTO product_variant_option (id, variant_id, option_value_id) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING",
            )
            .bind(format!("pvo_seed_jeans_{}", rank))
            .bind(var_id)
            .bind(&vid)
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}

async fn seed_sneakers_variants(pool: &sqlx::PgPool) -> Result<(), AppError> {
    let product_id = "prod_seed_sneakers";

    sqlx::query("INSERT INTO product_options (id, product_id, title) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING")
        .bind("opt_seed_sneakers_size")
        .bind(product_id)
        .bind("Ukuran")
        .execute(pool)
        .await?;

    let sizes = ["39", "40", "41", "42", "43"];
    for (i, val) in sizes.iter().enumerate() {
        let val_id = format!("optval_seed_snkr_s_{}", i);
        sqlx::query(
            "INSERT INTO product_option_values (id, option_id, value) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING",
        )
        .bind(&val_id)
        .bind("opt_seed_sneakers_size")
        .bind(val)
        .execute(pool)
        .await?;
    }

    let variants = [
        (
            "var_seed_snkr_39",
            "Sneakers - 39",
            Some("SNKR-39"),
            450000i64,
            "39",
        ),
        (
            "var_seed_snkr_40",
            "Sneakers - 40",
            Some("SNKR-40"),
            450000,
            "40",
        ),
        (
            "var_seed_snkr_41",
            "Sneakers - 41",
            Some("SNKR-41"),
            450000,
            "41",
        ),
        (
            "var_seed_snkr_42",
            "Sneakers - 42",
            Some("SNKR-42"),
            475000,
            "42",
        ),
        (
            "var_seed_snkr_43",
            "Sneakers - 43",
            Some("SNKR-43"),
            475000,
            "43",
        ),
    ];

    for (rank, (var_id, var_title, sku, price, opt_val)) in variants.iter().enumerate() {
        sqlx::query(
            "INSERT INTO product_variants (id, product_id, title, sku, price, variant_rank) VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (id) DO NOTHING",
        )
        .bind(var_id)
        .bind(product_id)
        .bind(var_title)
        .bind(sku)
        .bind(price)
        .bind(rank as i64)
        .execute(pool)
        .await?;

        let val_id = sqlx::query_as::<_, (String,)>(
            "SELECT id FROM product_option_values WHERE option_id = 'opt_seed_sneakers_size' AND value = $1",
        )
        .bind(opt_val)
        .fetch_optional(pool)
        .await?;

        if let Some((vid,)) = val_id {
            sqlx::query(
                "INSERT INTO product_variant_option (id, variant_id, option_value_id) VALUES ($1, $2, $3) ON CONFLICT (id) DO NOTHING",
            )
            .bind(format!("pvo_seed_snkr_{}", rank))
            .bind(var_id)
            .bind(&vid)
            .execute(pool)
            .await?;
        }
    }

    Ok(())
}

async fn seed_customer(pool: &sqlx::PgPool) -> Result<(), AppError> {
    let id = "cus_seed_budi";

    sqlx::query(
        r#"
        INSERT INTO customers (id, first_name, last_name, email, phone, has_account)
        VALUES ($1, $2, $3, $4, $5, TRUE)
        ON CONFLICT (id) DO NOTHING
        "#,
    )
    .bind(id)
    .bind("Budi")
    .bind("Santoso")
    .bind("budi@example.com")
    .bind("+6281234567890")
    .execute(pool)
    .await?;

    tracing::info!(customer = id, "Seeded customer");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db_url() -> String {
        std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/toko_test".to_string())
    }

    async fn setup_seed_db() -> sqlx::PgPool {
        let pool = sqlx::PgPool::connect(&test_db_url()).await.unwrap();
        sqlx::migrate!("./migrations").run(&pool).await.unwrap();
        pool
    }

    async fn clean_seed_data(pool: &sqlx::PgPool) {
        sqlx::query("DELETE FROM product_variant_option WHERE id LIKE 'pvo_seed_%'")
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM product_option_values WHERE id LIKE 'optval_seed_%'")
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM product_options WHERE id LIKE 'opt_seed_%'")
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM product_variants WHERE id LIKE 'var_seed_%'")
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM products WHERE id LIKE 'prod_seed_%'")
            .execute(pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM customers WHERE id LIKE 'cus_seed_%'")
            .execute(pool)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_seed_creates_products_and_customer() {
        let pool = setup_seed_db().await;
        clean_seed_data(&pool).await;
        let app_db = AppDb::Postgres(pool.clone());
        run_seed(&app_db).await.unwrap();

        let product_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM products WHERE id LIKE 'prod_seed_%'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(product_count.0, 3, "should have 3 seed products");

        let variant_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM product_variants WHERE id LIKE 'var_seed_%'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(variant_count.0, 13, "should have 13 seed variants (4+4+5)");

        let customer_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM customers WHERE id LIKE 'cus_seed_%'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(customer_count.0, 1, "should have 1 seed customer");
    }

    #[tokio::test]
    async fn test_seed_is_idempotent() {
        let pool = setup_seed_db().await;
        clean_seed_data(&pool).await;
        let app_db = AppDb::Postgres(pool.clone());

        run_seed(&app_db).await.unwrap();
        run_seed(&app_db).await.unwrap();

        let product_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM products WHERE id LIKE 'prod_seed_%'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            product_count.0, 3,
            "products should not duplicate on second run"
        );

        let variant_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM product_variants WHERE id LIKE 'var_seed_%'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            variant_count.0, 13,
            "variants should not duplicate on second run"
        );

        let option_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM product_options WHERE id LIKE 'opt_seed_%'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            option_count.0, 3,
            "options should not duplicate on second run"
        );

        let binding_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM product_variant_option WHERE id LIKE 'pvo_seed_%'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            binding_count.0, 13,
            "variant-option bindings should not duplicate on second run"
        );

        let customer_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM customers WHERE id = 'cus_seed_budi'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(
            customer_count.0, 1,
            "customer should not duplicate on second run"
        );
    }

    #[tokio::test]
    async fn test_seed_products_are_published() {
        let pool = setup_seed_db().await;
        clean_seed_data(&pool).await;
        let app_db = AppDb::Postgres(pool.clone());
        run_seed(&app_db).await.unwrap();

        let draft_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM products WHERE id LIKE 'prod_seed_%' AND status != 'published'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(draft_count.0, 0, "all seed products should be published");
    }

    #[tokio::test]
    async fn test_seed_variants_have_option_bindings() {
        let pool = setup_seed_db().await;
        clean_seed_data(&pool).await;
        let app_db = AppDb::Postgres(pool.clone());
        run_seed(&app_db).await.unwrap();

        let binding_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM product_variant_option WHERE id LIKE 'pvo_seed_%'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            binding_count.0, 13,
            "each variant should have an option binding"
        );
    }

    #[tokio::test]
    async fn test_seed_customer_has_account() {
        let pool = setup_seed_db().await;
        clean_seed_data(&pool).await;
        let app_db = AppDb::Postgres(pool.clone());
        run_seed(&app_db).await.unwrap();

        let has_account: (bool,) =
            sqlx::query_as("SELECT has_account FROM customers WHERE id = 'cus_seed_budi'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(
            has_account.0,
            "seed customer should have has_account = true"
        );
    }

    #[tokio::test]
    async fn test_seed_variant_ranks_are_ordered() {
        let pool = setup_seed_db().await;
        clean_seed_data(&pool).await;
        let app_db = AppDb::Postgres(pool.clone());
        run_seed(&app_db).await.unwrap();

        let ranks: Vec<(String, i64)> = sqlx::query_as(
            "SELECT id, variant_rank FROM product_variants WHERE product_id = 'prod_seed_kaos_polos' ORDER BY variant_rank",
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(ranks.len(), 4);
        assert_eq!(ranks[0].1, 0, "first variant should have rank 0");
        assert_eq!(ranks[1].1, 1, "second variant should have rank 1");
        assert_eq!(ranks[2].1, 2, "third variant should have rank 2");
        assert_eq!(ranks[3].1, 3, "fourth variant should have rank 3");
    }
}
