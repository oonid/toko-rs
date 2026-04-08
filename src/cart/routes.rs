use super::types::*;
use crate::{error::AppError, AppState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use validator::Validate;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/store/carts", post(store_create_cart))
        .route(
            "/store/carts/{id}",
            get(store_get_cart).post(store_update_cart),
        )
        .route("/store/carts/{id}/line-items", post(store_add_line_item))
        .route(
            "/store/carts/{id}/line-items/{line_id}",
            post(store_update_line_item).delete(store_delete_line_item),
        )
        .route("/store/carts/{id}/complete", post(store_complete_cart))
}

async fn store_create_cart(
    State(state): State<AppState>,
    Json(payload): Json<CreateCartInput>,
) -> Result<Json<CartResponse>, AppError> {
    payload
        .validate()
        .map_err(|e| AppError::InvalidData(e.to_string()))?;
    let cart_with_items = state.repos.cart.create_cart(payload).await?;
    Ok(Json(CartResponse {
        cart: cart_with_items,
    }))
}

async fn store_get_cart(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<CartResponse>, AppError> {
    let cart = state.repos.cart.get_cart(&id).await?;
    Ok(Json(CartResponse { cart }))
}

async fn store_update_cart(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateCartInput>,
) -> Result<Json<CartResponse>, AppError> {
    payload
        .validate()
        .map_err(|e| AppError::InvalidData(e.to_string()))?;
    let cart = state.repos.cart.update_cart(&id, payload).await?;
    Ok(Json(CartResponse { cart }))
}

async fn store_add_line_item(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<AddLineItemInput>,
) -> Result<Json<CartResponse>, AppError> {
    payload
        .validate()
        .map_err(|e| AppError::InvalidData(e.to_string()))?;
    let cart = state.repos.cart.add_line_item(&id, payload).await?;
    Ok(Json(CartResponse { cart }))
}

async fn store_delete_line_item(
    State(state): State<AppState>,
    Path((id, line_id)): Path<(String, String)>,
) -> Result<Json<CartResponse>, AppError> {
    let cart = state.repos.cart.delete_line_item(&id, &line_id).await?;
    Ok(Json(CartResponse { cart }))
}

async fn store_update_line_item(
    State(state): State<AppState>,
    Path((id, line_id)): Path<(String, String)>,
    Json(payload): Json<UpdateLineItemInput>,
) -> Result<Json<CartResponse>, AppError> {
    payload
        .validate()
        .map_err(|e| AppError::InvalidData(e.to_string()))?;
    let cart = state
        .repos
        .cart
        .update_line_item(&id, &line_id, payload)
        .await?;
    Ok(Json(CartResponse { cart }))
}

// Stub for Phase 1-C
async fn store_complete_cart(
    State(_state): State<AppState>,
    Path(_id): Path<String>,
) -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}
