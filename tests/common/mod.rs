use axum::Router;
use toko_rs::{app_router, db, AppState};

pub async fn setup_test_app() -> (Router, db::AppDb) {
    let db_url = "sqlite::memory:".to_string();

    let (app_db, repos) = db::create_db(&db_url, "idr")
        .await
        .expect("Failed to bind in-memory sqlite pool");

    db::run_migrations(&app_db)
        .await
        .expect("Failed to run migrations on test db");

    let state = AppState {
        db: app_db.clone(),
        repos: std::sync::Arc::new(repos),
    };
    (app_router(state), app_db)
}
