use super::types::*;
use crate::error::AppError;
use crate::AppState;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    routing::{get, post},
    Json, Router,
};
use validator::Validate;

pub fn router() -> Router<AppState> {
    let me_routes = Router::new()
        .route(
            "/store/customers/me",
            get(store_get_me).post(store_update_me),
        )
        .layer(axum::middleware::from_fn(auth_customer_id));

    Router::new()
        .route("/store/customers", post(store_register))
        .merge(me_routes)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CustomerId {
    pub id: String,
}

pub async fn auth_customer_id(
    mut req: Request,
    next: Next,
) -> Result<axum::response::Response, AppError> {
    let customer_id = req
        .headers()
        .get("X-Customer-Id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::Unauthorized("Missing X-Customer-Id header".into()))?;

    req.extensions_mut().insert(CustomerId { id: customer_id });
    Ok(next.run(req).await)
}

async fn store_register(
    State(state): State<AppState>,
    Json(payload): Json<CreateCustomerInput>,
) -> Result<(StatusCode, Json<CustomerResponse>), AppError> {
    payload
        .validate()
        .map_err(|e| AppError::InvalidData(e.to_string()))?;

    let customer = state.repos.customer.create(payload).await?;
    Ok((StatusCode::OK, Json(CustomerResponse { customer })))
}

async fn store_get_me(
    State(state): State<AppState>,
    axum::extract::Extension(cid): axum::extract::Extension<CustomerId>,
) -> Result<Json<CustomerResponse>, AppError> {
    let customer = state.repos.customer.find_by_id(&cid.id).await?;
    Ok(Json(CustomerResponse { customer }))
}

async fn store_update_me(
    State(state): State<AppState>,
    axum::extract::Extension(cid): axum::extract::Extension<CustomerId>,
    Json(payload): Json<UpdateCustomerInput>,
) -> Result<Json<CustomerResponse>, AppError> {
    payload
        .validate()
        .map_err(|e| AppError::InvalidData(e.to_string()))?;

    let customer = state.repos.customer.update(&cid.id, &payload).await?;
    Ok(Json(CustomerResponse { customer }))
}
