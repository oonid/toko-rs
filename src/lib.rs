use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use axum::http::Method;
use axum::Router;
use std::sync::Arc;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;

pub mod cart;
pub mod config;
pub mod customer;
pub mod order;
pub mod payment;
pub mod product;
pub mod seed;
pub mod types;

pub mod db;
pub mod error;

pub mod extract;

#[derive(Clone)]
pub struct AppState {
    pub db: db::AppDb,
    pub repos: Arc<db::Repositories>,
}

fn build_cors_layer(origins: &str) -> CorsLayer {
    let allow_origin = if origins == "*" {
        AllowOrigin::any()
    } else {
        let parsed: Vec<_> = origins
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        AllowOrigin::list(parsed)
    };

    CorsLayer::new()
        .allow_origin(allow_origin)
        .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::OPTIONS])
        .allow_headers([CONTENT_TYPE, AUTHORIZATION])
}

pub fn app_router(state: AppState) -> Router {
    app_router_with_cors(state, "*")
}

pub fn app_router_with_cors(state: AppState, cors_origins: &str) -> Router {
    let order_protected = order::routes::protected_router().layer(axum::middleware::from_fn(
        customer::routes::auth_customer_id,
    ));

    Router::new()
        .merge(product::routes::router())
        .merge(cart::routes::router())
        .merge(customer::routes::router())
        .merge(order::routes::router())
        .merge(order_protected)
        .route("/health", axum::routing::get(health_check))
        .layer(TraceLayer::new_for_http())
        .layer(build_cors_layer(cors_origins))
        .with_state(state)
}

pub async fn build_app_state(
    database_url: &str,
    default_currency_code: &str,
) -> Result<(AppState, db::AppDb), error::AppError> {
    let (app_db, repos) = db::create_db(database_url, default_currency_code).await?;
    db::run_migrations(&app_db).await?;
    let state = AppState {
        db: app_db.clone(),
        repos: Arc::new(repos),
    };
    Ok((state, app_db))
}

#[tracing::instrument(skip_all)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use tower::ServiceExt;

    fn test_db_url() -> String {
        std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/toko_test".to_string())
    }

    #[tokio::test]
    async fn test_health_check_connected() {
        let (state, _) = build_app_state(&test_db_url(), "idr").await.unwrap();
        let app = app_router(state);
        let req = axum::http::Request::builder()
            .uri("/health")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), 200);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let val: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(val["status"], "ok");
        assert_eq!(val["database"], "connected");
        assert_eq!(val["version"], env!("CARGO_PKG_VERSION"));
    }

    #[tokio::test]
    async fn test_build_app_state() {
        let (state, db) = build_app_state(&test_db_url(), "idr").await.unwrap();
        assert!(db::ping(&db).await);
        let _ = &state;
    }

    #[tokio::test]
    async fn test_cors_with_specific_origins() {
        let (state, _) = build_app_state(&test_db_url(), "idr").await.unwrap();
        let app = app_router_with_cors(state, "http://localhost:3000,http://example.com");

        let req = axum::http::Request::builder()
            .uri("/health")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), 200);
    }

    #[tokio::test]
    async fn test_cors_wildcard() {
        let (state, _) = build_app_state(&test_db_url(), "idr").await.unwrap();
        let app = app_router_with_cors(state, "*");

        let req = axum::http::Request::builder()
            .uri("/health")
            .body(axum::body::Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), 200);
    }
}
