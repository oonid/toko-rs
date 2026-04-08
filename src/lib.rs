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

pub async fn build_app_state(database_url: &str) -> Result<(AppState, db::AppDb), error::AppError> {
    let (app_db, repo) = db::create_db(database_url).await?;
    db::run_migrations(&app_db).await?;
    let repo_arc = Arc::new(repo);
    let state = AppState {
        db: app_db.clone(),
        product_repo: repo_arc.clone(),
        cart_repo: repo_arc,
    };
    Ok((state, app_db))
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

#[cfg(test)]
mod tests {
    use super::*;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_health_check_connected() {
        let (state, _) = build_app_state("sqlite::memory:").await.unwrap();
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
        let (state, db) = build_app_state("sqlite::memory:").await.unwrap();
        assert!(db::ping(&db).await);
        assert!(matches!(db, db::AppDb::Sqlite(_)));
        let _ = &state;
    }
}
