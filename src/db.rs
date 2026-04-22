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
pub(crate) fn unique_violation_code() -> &'static str {
    "23505"
}

#[cfg(feature = "postgres")]
pub(crate) fn fk_violation_code() -> &'static str {
    "23503"
}

#[cfg(feature = "postgres")]
pub(crate) fn not_null_violation_code() -> &'static str {
    "23502"
}

#[cfg(feature = "sqlite")]
pub(crate) fn unique_violation_code() -> &'static str {
    "2067"
}

#[cfg(feature = "sqlite")]
pub(crate) fn fk_violation_code() -> &'static str {
    "787"
}

#[cfg(feature = "sqlite")]
pub(crate) fn not_null_violation_code() -> &'static str {
    "1299"
}

#[cfg(feature = "postgres")]
pub(crate) fn serialization_failure_code() -> &'static str {
    "40001"
}

#[cfg(feature = "sqlite")]
pub(crate) fn serialization_failure_code() -> &'static str {
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

    #[test]
    fn test_is_unique_violation_with_db_error() {
        let db_err = sqlx::Error::Database(Box::new(TestDbError {
            code: Some(unique_violation_code().to_string()),
            message: "dup".into(),
            ..Default::default()
        }));
        assert!(is_unique_violation(&db_err));
        assert!(!is_fk_violation(&db_err));
    }

    #[test]
    fn test_is_fk_violation_with_db_error() {
        let db_err = sqlx::Error::Database(Box::new(TestDbError {
            code: Some(fk_violation_code().to_string()),
            message: "fk".into(),
            ..Default::default()
        }));
        assert!(is_fk_violation(&db_err));
        assert!(!is_unique_violation(&db_err));
    }

    #[test]
    fn test_is_not_null_violation_with_db_error() {
        let db_err = sqlx::Error::Database(Box::new(TestDbError {
            code: Some(not_null_violation_code().to_string()),
            message: "nn".into(),
            ..Default::default()
        }));
        assert!(is_not_null_violation(&db_err));
    }

    #[test]
    fn test_is_serialization_failure_with_db_error() {
        let db_err = sqlx::Error::Database(Box::new(TestDbError {
            code: Some(serialization_failure_code().to_string()),
            message: "ser".into(),
            ..Default::default()
        }));
        assert!(is_serialization_failure(&db_err));
    }

    #[test]
    fn test_non_db_error_returns_false() {
        let err = sqlx::Error::Configuration("cfg fail".into());
        assert!(!is_unique_violation(&err));
        assert!(!is_fk_violation(&err));
        assert!(!is_not_null_violation(&err));
        assert!(!is_serialization_failure(&err));
    }
}

#[derive(Default)]
pub struct TestDbError {
    pub code: Option<String>,
    pub message: String,
}

impl std::fmt::Debug for TestDbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestDbError")
            .field("code", &self.code)
            .field("message", &self.message)
            .finish()
    }
}

impl std::fmt::Display for TestDbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for TestDbError {}

impl sqlx::error::DatabaseError for TestDbError {
    fn message(&self) -> &str {
        &self.message
    }

    fn code(&self) -> Option<std::borrow::Cow<'_, str>> {
        self.code.as_deref().map(std::borrow::Cow::Borrowed)
    }

    fn kind(&self) -> sqlx::error::ErrorKind {
        sqlx::error::ErrorKind::Other
    }

    fn constraint(&self) -> Option<&str> {
        None
    }

    fn as_error(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
        self
    }

    fn as_error_mut(&mut self) -> &mut (dyn std::error::Error + Send + Sync + 'static) {
        self
    }

    fn into_error(
        self: Box<Self>,
    ) -> Box<dyn std::error::Error + Send + Sync + 'static> {
        self
    }
}
