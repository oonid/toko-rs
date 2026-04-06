use sqlx::AnyPool;
use crate::error::AppError;

pub async fn run_seed(pool: &AnyPool) -> Result<(), AppError> {
    tracing::info!("Seeding dummy data...");
    // TODO: implement actual seeding logic
    Ok(())
}
