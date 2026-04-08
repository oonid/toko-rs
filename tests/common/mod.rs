use toko_rs::{AppState, app_router, db};
use axum::Router;

pub async fn setup_test_app() -> (Router, db::AppDb) {
    let db_url = format!("sqlite::memory:");
    
    // Create connection pool using the formal db layer
    let (app_db, repo) = db::create_db(&db_url).await.expect("Failed to bind in-memory sqlite pool");

    // Run migrations
    db::run_migrations(&app_db).await.expect("Failed to run migrations on test db");

    let repo_arc = std::sync::Arc::new(repo);
    let state = AppState { 
        db: app_db.clone(),
        product_repo: repo_arc.clone(), 
        cart_repo: repo_arc,
    };
    (app_router(state), app_db)
}
