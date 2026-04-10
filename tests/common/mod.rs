pub async fn setup_test_app() -> (axum::Router, toko_rs::db::AppDb) {
    use toko_rs::app_router;
    use toko_rs::db;
    use toko_rs::AppState;

    #[cfg(feature = "postgres")]
    let default_url = "postgres://postgres:postgres@localhost:5432/toko_test".to_string();
    #[cfg(feature = "sqlite")]
    let default_url = "sqlite:toko_test.db".to_string();
    let db_url = std::env::var("DATABASE_URL").unwrap_or(default_url);
    let (app_db, repos) = db::create_db(&db_url, "idr")
        .await
        .expect("Failed to create pool");
    db::run_migrations(&app_db)
        .await
        .expect("Failed to run migrations");

    clean_all_tables(&app_db.pool).await;

    let state = AppState {
        db: app_db.clone(),
        repos: std::sync::Arc::new(repos),
    };
    (app_router(state), app_db)
}

pub async fn clean_all_tables(pool: &toko_rs::db::DbPool) {
    sqlx::query("DELETE FROM payment_records")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM order_line_items")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM orders")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM cart_line_items")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM carts")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM customer_addresses")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM customers")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM product_variant_option")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM product_option_values")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM product_options")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM product_variants")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM products")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM idempotency_keys")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("UPDATE _sequences SET value = 0 WHERE name = 'order_display_id'")
        .execute(pool)
        .await
        .unwrap();
}
