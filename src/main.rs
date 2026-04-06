use std::net::SocketAddr;
use std::sync::Arc;
use axum::{routing::get, Json, Router};
use serde_json::json;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod db;
mod error;
mod types;
mod product;
mod cart;
mod order;
mod customer;
mod payment;
mod seed;

// Temporary stub for AppState until repos are implemented
#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::AnyPool,
}

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
    let pool = db::create_pool(&config.database_url).await?;
    tracing::info!("Connected to database");
    db::run_migrations(&pool).await?;
    tracing::info!("Migrations executed successfully");

    // 4. Check for --seed flag
    if std::env::args().any(|arg| arg == "--seed") {
        seed::run_seed(&pool).await?;
        tracing::info!("Seeding complete. Exiting.");
        return Ok(());
    }

    let state = AppState { pool };

    // 5. Build Axum router
    let app = Router::new()
        .route("/health", get(health_check))
        // TODO: mount modules here
        .layer(TraceLayer::new_for_http())
        .with_state(state);

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
        "database": "connected", // Simplified for now
        "version": env!("CARGO_PKG_VERSION")
    }))
}
