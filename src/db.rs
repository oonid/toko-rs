use sqlx::any::{AnyConnectOptions, AnyPoolOptions};
use sqlx::AnyPool;
use std::str::FromStr;
use crate::error::AppError;

pub async fn create_pool(database_url: &str) -> Result<AnyPool, AppError> {
    sqlx::any::install_default_drivers();
    
    let connect_options = AnyConnectOptions::from_str(database_url)
        .map_err(|e| AppError::DatabaseError(sqlx::Error::Configuration(Box::new(e))))?;

    let pool = AnyPoolOptions::new()
        .min_connections(2)
        .max_connections(10)
        .idle_timeout(std::time::Duration::from_secs(300))
        .connect_with(connect_options)
        .await?;

    Ok(pool)
}

pub async fn run_migrations(pool: &AnyPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
}
