use axum::Router;
use tower_http::trace::TraceLayer;
use std::sync::Arc;

pub mod config;
pub mod db;
pub mod error;
pub mod types;
pub mod product;
pub mod cart;
pub mod order;
pub mod customer;
pub mod payment;
pub mod seed;

#[derive(Clone)]
pub struct AppState {
    pub db: db::AppDb,
    pub product_repo: Arc<db::DatabaseRepo>,
}

pub fn app_router(state: AppState) -> Router {
    Router::new()
        .merge(product::routes::router())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
