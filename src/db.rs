use sqlx::{sqlite::SqlitePoolOptions, postgres::PgPoolOptions, SqlitePool, PgPool};
use crate::error::AppError;
use crate::product::repository::{SqliteProductRepository, PostgresProductRepository};

#[derive(Clone)]
pub enum AppDb {
    Sqlite(SqlitePool),
    Postgres(PgPool),
}

#[derive(Clone)]
pub enum DatabaseRepo {
    Sqlite(SqliteProductRepository),
    Postgres(PostgresProductRepository),
}

impl DatabaseRepo {
    // We delegate the method calls internally to the concrete driver implementations
    pub async fn create_product(&self, input: crate::product::types::CreateProductInput) -> Result<crate::product::models::ProductWithRelations, AppError> {
        match self {
            Self::Sqlite(repo) => repo.create_product(input).await,
            Self::Postgres(repo) => repo.create_product(input).await,
        }
    }
}

pub async fn create_db(database_url: &str) -> Result<(AppDb, DatabaseRepo), AppError> {
    if database_url.starts_with("postgres://") || database_url.starts_with("postgresql://") {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;
        let repo = DatabaseRepo::Postgres(PostgresProductRepository::new(pool.clone()));
        Ok((AppDb::Postgres(pool), repo))
    } else {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(database_url)
            .await?;
        let repo = DatabaseRepo::Sqlite(SqliteProductRepository::new(pool.clone()));
        Ok((AppDb::Sqlite(pool), repo))
    }
}

pub async fn run_migrations(db: &AppDb) -> Result<(), AppError> {
    match db {
        AppDb::Sqlite(pool) => {
            sqlx::migrate!("./migrations").run(pool).await?;
        }
        AppDb::Postgres(pool) => {
            sqlx::migrate!("./migrations").run(pool).await?;
        }
    }
    Ok(())
}
