use crate::cart::repository::CartRepository;
use crate::customer::repository::CustomerRepository;
use crate::error::AppError;
use crate::order::repository::OrderRepository;
use crate::payment::repository::PaymentRepository;
use crate::product::repository::ProductRepository;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

#[derive(Clone)]
pub enum AppDb {
    Postgres(PgPool),
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
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;

    let repos = Repositories {
        product: ProductRepository::new(pool.clone()),
        cart: CartRepository::new(pool.clone(), default_currency_code.to_string()),
        customer: CustomerRepository::new(pool.clone()),
        order: OrderRepository::new(pool.clone()),
        payment: PaymentRepository::new(pool.clone()),
    };

    Ok((AppDb::Postgres(pool), repos))
}

pub async fn run_migrations(db: &AppDb) -> Result<(), AppError> {
    match db {
        AppDb::Postgres(pool) => {
            sqlx::migrate!("./migrations").run(pool).await?;
        }
    }
    Ok(())
}

pub async fn ping(db: &AppDb) -> bool {
    match db {
        AppDb::Postgres(pool) => sqlx::query("SELECT 1").execute(pool).await.is_ok(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db_url() -> String {
        std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/toko_test".to_string())
    }

    #[tokio::test]
    async fn test_create_pg_db() {
        let url = test_db_url();
        let (app_db, _repos) = create_db(&url, "idr").await.unwrap();
        assert!(matches!(app_db, AppDb::Postgres(_)));
    }

    #[tokio::test]
    async fn test_run_migrations_pg() {
        let url = test_db_url();
        let (app_db, _) = create_db(&url, "idr").await.unwrap();
        run_migrations(&app_db).await.unwrap();
    }

    #[tokio::test]
    async fn test_ping_pg() {
        let url = test_db_url();
        let (app_db, _) = create_db(&url, "idr").await.unwrap();
        run_migrations(&app_db).await.unwrap();
        assert!(ping(&app_db).await);
    }

    #[tokio::test]
    async fn test_ping_after_pool_close() {
        let pool = PgPool::connect(&test_db_url()).await.unwrap();
        let app_db = AppDb::Postgres(pool);
        assert!(ping(&app_db).await);
    }
}
