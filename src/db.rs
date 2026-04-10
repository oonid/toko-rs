use crate::cart::repository::CartRepository;
use crate::customer::repository::CustomerRepository;
use crate::error::AppError;
use crate::order::repository::OrderRepository;
use crate::payment::repository::PaymentRepository;
use crate::product::repository::ProductRepository;

#[cfg(feature = "postgres")]
use sqlx::postgres::{PgPool, PgPoolOptions};

#[cfg(feature = "sqlite")]
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};

#[cfg(feature = "postgres")]
pub type DbPool = PgPool;

#[cfg(feature = "postgres")]
pub type DbPoolOptions = PgPoolOptions;

#[cfg(feature = "sqlite")]
pub type DbPool = SqlitePool;

#[cfg(feature = "sqlite")]
pub type DbPoolOptions = SqlitePoolOptions;

#[cfg(feature = "postgres")]
pub type DbDatabase = sqlx::Postgres;

#[cfg(feature = "sqlite")]
pub type DbDatabase = sqlx::Sqlite;

pub type DbTransaction<'a> = sqlx::Transaction<'a, DbDatabase>;

#[derive(Clone)]
pub struct AppDb {
    pub pool: DbPool,
}

#[derive(Clone)]
pub struct Repositories {
    pub product: ProductRepository,
    pub cart: CartRepository,
    pub customer: CustomerRepository,
    pub order: OrderRepository,
    pub payment: PaymentRepository,
}

pub async fn create_db(
    database_url: &str,
    default_currency_code: &str,
) -> Result<(AppDb, Repositories), AppError> {
    #[cfg(feature = "postgres")]
    let pool = DbPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;

    #[cfg(feature = "sqlite")]
    let pool = {
        let p = DbPoolOptions::new()
            .max_connections(1)
            .connect(database_url)
            .await?;
        let _ = sqlx::query("PRAGMA foreign_keys = ON").execute(&p).await;
        p
    };

    let repos = Repositories {
        product: ProductRepository::new(pool.clone()),
        cart: CartRepository::new(pool.clone(), default_currency_code.to_string()),
        customer: CustomerRepository::new(pool.clone()),
        order: OrderRepository::new(pool.clone()),
        payment: PaymentRepository::new(pool.clone()),
    };

    Ok((AppDb { pool }, repos))
}

pub async fn run_migrations(db: &AppDb) -> Result<(), AppError> {
    #[cfg(feature = "postgres")]
    {
        sqlx::migrate!("./migrations").run(&db.pool).await?;
    }

    #[cfg(feature = "sqlite")]
    {
        sqlx::migrate!("./migrations/sqlite").run(&db.pool).await?;
    }

    Ok(())
}

pub async fn ping(db: &AppDb) -> bool {
    sqlx::query("SELECT 1").execute(&db.pool).await.is_ok()
}

pub fn is_unique_violation(e: &sqlx::Error) -> bool {
    if let sqlx::Error::Database(ref db_err) = e {
        db_err.code().as_deref() == Some(unique_violation_code())
    } else {
        false
    }
}

pub fn is_fk_violation(e: &sqlx::Error) -> bool {
    if let sqlx::Error::Database(ref db_err) = e {
        db_err.code().as_deref() == Some(fk_violation_code())
    } else {
        false
    }
}

pub fn is_not_null_violation(e: &sqlx::Error) -> bool {
    if let sqlx::Error::Database(ref db_err) = e {
        db_err.code().as_deref() == Some(not_null_violation_code())
    } else {
        false
    }
}

pub fn is_serialization_failure(e: &sqlx::Error) -> bool {
    if let sqlx::Error::Database(ref db_err) = e {
        db_err.code().as_deref() == Some(serialization_failure_code())
    } else {
        false
    }
}

#[cfg(feature = "postgres")]
fn unique_violation_code() -> &'static str {
    "23505"
}

#[cfg(feature = "postgres")]
fn fk_violation_code() -> &'static str {
    "23503"
}

#[cfg(feature = "postgres")]
fn not_null_violation_code() -> &'static str {
    "23502"
}

#[cfg(feature = "sqlite")]
fn unique_violation_code() -> &'static str {
    "2067"
}

#[cfg(feature = "sqlite")]
fn fk_violation_code() -> &'static str {
    "787"
}

#[cfg(feature = "sqlite")]
fn not_null_violation_code() -> &'static str {
    "1299"
}

#[cfg(feature = "postgres")]
fn serialization_failure_code() -> &'static str {
    "40001"
}

#[cfg(feature = "sqlite")]
fn serialization_failure_code() -> &'static str {
    ""
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db_url() -> String {
        #[cfg(feature = "postgres")]
        let default = "postgres://postgres:postgres@localhost:5432/toko_test".to_string();
        #[cfg(feature = "sqlite")]
        let default = "sqlite:toko_test.db".to_string();
        std::env::var("DATABASE_URL").unwrap_or_else(|_| default)
    }

    #[tokio::test]
    async fn test_create_db() {
        let url = test_db_url();
        let (app_db, _repos) = create_db(&url, "idr").await.unwrap();
        let _ = &app_db.pool;
    }

    #[tokio::test]
    async fn test_run_migrations() {
        let url = test_db_url();
        let (app_db, _) = create_db(&url, "idr").await.unwrap();
        run_migrations(&app_db).await.unwrap();
    }

    #[tokio::test]
    async fn test_ping() {
        let url = test_db_url();
        let (app_db, _) = create_db(&url, "idr").await.unwrap();
        run_migrations(&app_db).await.unwrap();
        assert!(ping(&app_db).await);
    }

    #[tokio::test]
    async fn test_ping_after_pool_close() {
        let pool = DbPool::connect(&test_db_url()).await.unwrap();
        let app_db = AppDb { pool };
        assert!(ping(&app_db).await);
    }
}
