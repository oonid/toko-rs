use crate::db::AppDb;
use crate::error::AppError;

pub async fn run_seed(_db: &AppDb) -> Result<(), AppError> {
    tracing::info!("Seeding dummy data...");
    // TODO: implement actual seeding logic
    Ok(())
}
