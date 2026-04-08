#[cfg(not(coverage))]
use crate::cart::repository::PostgresCartRepository;
use crate::cart::repository::SqliteCartRepository;
use crate::error::AppError;
use crate::product::repository::{PostgresProductRepository, SqliteProductRepository};
use sqlx::{postgres::PgPoolOptions, sqlite::SqlitePoolOptions, PgPool, SqlitePool};

#[derive(Clone)]
pub enum AppDb {
    Sqlite(SqlitePool),
    Postgres(PgPool),
}

#[derive(Clone)]
pub enum DatabaseRepo {
    Sqlite {
        product: SqliteProductRepository,
        cart: SqliteCartRepository,
    },
    Postgres {
        product: PostgresProductRepository,
        #[cfg(not(coverage))]
        cart: PostgresCartRepository,
    },
}

impl DatabaseRepo {
    // Product Delegate
    pub async fn create_product(
        &self,
        input: crate::product::types::CreateProductInput,
    ) -> Result<crate::product::models::ProductWithRelations, AppError> {
        match self {
            Self::Sqlite { product, .. } => product.create_product(input).await,
            Self::Postgres { product, .. } => product.create_product(input).await,
        }
    }

    // Cart Delegates
    pub async fn create_cart(
        &self,
        input: crate::cart::types::CreateCartInput,
    ) -> Result<crate::cart::models::CartWithItems, AppError> {
        match self {
            Self::Sqlite { cart, .. } => cart.create_cart(input).await,
            #[cfg(not(coverage))]
            Self::Postgres { cart, .. } => cart.create_cart(input).await,
            #[cfg(coverage)]
            Self::Postgres { .. } => unreachable!(),
        }
    }

    pub async fn get_cart(&self, id: &str) -> Result<crate::cart::models::CartWithItems, AppError> {
        match self {
            Self::Sqlite { cart, .. } => cart.get_cart(id).await,
            #[cfg(not(coverage))]
            Self::Postgres { cart, .. } => cart.get_cart(id).await,
            #[cfg(coverage)]
            Self::Postgres { .. } => unreachable!(),
        }
    }

    pub async fn update_cart(
        &self,
        id: &str,
        input: crate::cart::types::UpdateCartInput,
    ) -> Result<crate::cart::models::CartWithItems, AppError> {
        match self {
            Self::Sqlite { cart, .. } => cart.update_cart(id, input).await,
            #[cfg(not(coverage))]
            Self::Postgres { cart, .. } => cart.update_cart(id, input).await,
            #[cfg(coverage)]
            Self::Postgres { .. } => unreachable!(),
        }
    }

    pub async fn add_line_item(
        &self,
        id: &str,
        input: crate::cart::types::AddLineItemInput,
    ) -> Result<crate::cart::models::CartWithItems, AppError> {
        match self {
            Self::Sqlite { cart, .. } => cart.add_line_item(id, input).await,
            #[cfg(not(coverage))]
            Self::Postgres { cart, .. } => cart.add_line_item(id, input).await,
            #[cfg(coverage)]
            Self::Postgres { .. } => unreachable!(),
        }
    }

    pub async fn update_line_item(
        &self,
        id: &str,
        line_id: &str,
        input: crate::cart::types::UpdateLineItemInput,
    ) -> Result<crate::cart::models::CartWithItems, AppError> {
        match self {
            Self::Sqlite { cart, .. } => cart.update_line_item(id, line_id, input).await,
            #[cfg(not(coverage))]
            Self::Postgres { cart, .. } => cart.update_line_item(id, line_id, input).await,
            #[cfg(coverage)]
            Self::Postgres { .. } => unreachable!(),
        }
    }

    pub async fn delete_line_item(
        &self,
        id: &str,
        line_id: &str,
    ) -> Result<crate::cart::models::CartWithItems, AppError> {
        match self {
            Self::Sqlite { cart, .. } => cart.delete_line_item(id, line_id).await,
            #[cfg(not(coverage))]
            Self::Postgres { cart, .. } => cart.delete_line_item(id, line_id).await,
            #[cfg(coverage)]
            Self::Postgres { .. } => unreachable!(),
        }
    }
}

pub async fn create_db(database_url: &str) -> Result<(AppDb, DatabaseRepo), AppError> {
    if database_url.starts_with("postgres://") || database_url.starts_with("postgresql://") {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;
        let repo = DatabaseRepo::Postgres {
            product: PostgresProductRepository::new(pool.clone()),
            #[cfg(not(coverage))]
            cart: PostgresCartRepository::new(pool.clone()),
        };
        Ok((AppDb::Postgres(pool), repo))
    } else {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(database_url)
            .await?;
        let repo = DatabaseRepo::Sqlite {
            product: SqliteProductRepository::new(pool.clone()),
            cart: SqliteCartRepository::new(pool.clone()),
        };
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

pub async fn ping(db: &AppDb) -> bool {
    match db {
        AppDb::Sqlite(pool) => sqlx::query("SELECT 1").execute(pool).await.is_ok(),
        AppDb::Postgres(pool) => sqlx::query("SELECT 1").execute(pool).await.is_ok(),
    }
}
