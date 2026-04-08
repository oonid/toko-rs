use crate::cart::repository::CartRepository;
use crate::error::AppError;
use crate::product::repository::ProductRepository;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;

#[derive(Clone)]
pub enum AppDb {
    Sqlite(SqlitePool),
}

#[derive(Clone)]
pub struct Repositories {
    pub product: ProductRepository,
    pub cart: CartRepository,
}

pub async fn create_db(database_url: &str) -> Result<(AppDb, Repositories), AppError> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(database_url)
        .await?;

    let repos = Repositories {
        product: ProductRepository::new(pool.clone()),
        cart: CartRepository::new(pool.clone()),
    };

    Ok((AppDb::Sqlite(pool), repos))
}

pub async fn run_migrations(db: &AppDb) -> Result<(), AppError> {
    match db {
        AppDb::Sqlite(pool) => {
            sqlx::migrate!("./migrations/sqlite").run(pool).await?;
        }
    }
    Ok(())
}

pub async fn ping(db: &AppDb) -> bool {
    match db {
        AppDb::Sqlite(pool) => sqlx::query("SELECT 1").execute(pool).await.is_ok(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_sqlite_db() {
        let (app_db, _repos) = create_db("sqlite::memory:").await.unwrap();
        assert!(matches!(app_db, AppDb::Sqlite(_)));
    }

    #[tokio::test]
    async fn test_run_migrations_sqlite() {
        let (app_db, _) = create_db("sqlite::memory:").await.unwrap();
        run_migrations(&app_db).await.unwrap();
    }

    #[tokio::test]
    async fn test_ping_sqlite() {
        let (app_db, _) = create_db("sqlite::memory:").await.unwrap();
        run_migrations(&app_db).await.unwrap();
        assert!(ping(&app_db).await);
    }

    #[tokio::test]
    async fn test_ping_after_pool_close() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let app_db = AppDb::Sqlite(pool);
        assert!(ping(&app_db).await);
    }
}
