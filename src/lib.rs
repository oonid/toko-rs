use axum::Router;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub mod cart;
pub mod config;
pub mod customer;
pub mod db;
pub mod error;
pub mod order;
pub mod payment;
pub mod product;
pub mod seed;
pub mod types;

#[derive(Clone)]
pub struct AppState {
    pub db: db::AppDb,
    pub product_repo: Arc<db::DatabaseRepo>,
    pub cart_repo: Arc<db::DatabaseRepo>,
}

pub fn app_router(state: AppState) -> Router {
    Router::new()
        .merge(product::routes::router())
        .merge(cart::routes::router())
        .route("/health", axum::routing::get(health_check))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn health_check(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> axum::Json<serde_json::Value> {
    let db_ok = db::ping(&state.db).await;

    let (status, database) = if db_ok {
        ("ok", "connected")
    } else {
        ("degraded", "disconnected")
    };

    axum::Json(serde_json::json!({
        "status": status,
        "database": database,
        "version": env!("CARGO_PKG_VERSION")
    }))
}
