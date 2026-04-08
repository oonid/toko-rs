use crate::db::AppDb;
use crate::error::AppError;

pub async fn run_seed(_db: &AppDb) -> Result<(), AppError> {
    tracing::info!("Seeding dummy data...");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_seed_returns_ok() {
        let pool = sqlx::SqlitePool::connect(":memory:").await.unwrap();
        let db = AppDb::Sqlite(pool);
        let result = run_seed(&db).await;
        assert!(result.is_ok());
    }
}
