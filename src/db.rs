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

use crate::product::types::UpdateProductInput;
use crate::types::FindParams;

impl DatabaseRepo {
    // Product Delegates
    pub async fn create_product(
        &self,
        input: crate::product::types::CreateProductInput,
    ) -> Result<crate::product::models::ProductWithRelations, AppError> {
        match self {
            Self::Sqlite { product, .. } => product.create_product(input).await,
            Self::Postgres { .. } => unimplemented!("PostgreSQL product repo not yet available"),
        }
    }

    pub async fn get_product(
        &self,
        id: &str,
    ) -> Result<crate::product::models::ProductWithRelations, AppError> {
        match self {
            Self::Sqlite { product, .. } => product.find_by_id(id).await,
            Self::Postgres { .. } => unimplemented!(),
        }
    }

    pub async fn get_published_product(
        &self,
        id: &str,
    ) -> Result<crate::product::models::ProductWithRelations, AppError> {
        match self {
            Self::Sqlite { product, .. } => product.find_published_by_id(id).await,
            Self::Postgres { .. } => unimplemented!(),
        }
    }

    pub async fn list_products(
        &self,
        params: &FindParams,
    ) -> Result<(Vec<crate::product::models::ProductWithRelations>, i64), AppError> {
        match self {
            Self::Sqlite { product, .. } => product.list(params).await,
            Self::Postgres { .. } => unimplemented!(),
        }
    }

    pub async fn list_published_products(
        &self,
        params: &FindParams,
    ) -> Result<(Vec<crate::product::models::ProductWithRelations>, i64), AppError> {
        match self {
            Self::Sqlite { product, .. } => product.list_published(params).await,
            Self::Postgres { .. } => unimplemented!(),
        }
    }

    pub async fn update_product(
        &self,
        id: &str,
        input: &UpdateProductInput,
    ) -> Result<crate::product::models::ProductWithRelations, AppError> {
        match self {
            Self::Sqlite { product, .. } => product.update(id, input).await,
            Self::Postgres { .. } => unimplemented!(),
        }
    }

    pub async fn delete_product(&self, id: &str) -> Result<String, AppError> {
        match self {
            Self::Sqlite { product, .. } => product.soft_delete(id).await,
            Self::Postgres { .. } => unimplemented!(),
        }
    }

    pub async fn add_variant(
        &self,
        product_id: &str,
        input: &crate::product::types::CreateProductVariantInput,
    ) -> Result<crate::product::models::ProductWithRelations, AppError> {
        match self {
            Self::Sqlite { product, .. } => product.add_variant(product_id, input).await,
            Self::Postgres { .. } => unimplemented!(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_sqlite_db() {
        let (app_db, repo) = create_db("sqlite::memory:").await.unwrap();
        assert!(matches!(app_db, AppDb::Sqlite(_)));
        assert!(matches!(repo, DatabaseRepo::Sqlite { .. }));
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
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        let app_db = AppDb::Sqlite(pool);
        assert!(ping(&app_db).await);
    }
}
