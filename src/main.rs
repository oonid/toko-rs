use std::net::SocketAddr;
use axum::{routing::get, Json, Router};
use serde_json::json;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use toko_rs::{config, db, seed, AppState, app_router};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Load config
    let config = config::AppConfig::load().unwrap_or_else(|e| {
        eprintln!("Failed to load config: {}", e);
        std::process::exit(1);
    });

    // 2. Init tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}={}", env!("CARGO_PKG_NAME"), config.rust_log).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 3. Init DB and run migrations
    let (app_db, repo) = db::create_db(&config.database_url).await?;
    tracing::info!("Connected to database");
    db::run_migrations(&app_db).await?;
    tracing::info!("Migrations executed successfully");

    // 4. Check for --seed flag
    if std::env::args().any(|arg| arg == "--seed") {
        seed::run_seed(&app_db).await?;
        tracing::info!("Seeding complete. Exiting.");
        return Ok(());
    }

    let state = AppState {
        db: app_db,
        product_repo: std::sync::Arc::new(repo),
    };

    // 5. Build Axum router
    let app = app_router(state).route("/health", get(health_check));

    // 6. Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health_check() -> axum::Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "database": "connected", 
        "version": env!("CARGO_PKG_VERSION")
    }))
}
