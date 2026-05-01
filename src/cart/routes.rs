use super::types::*;
use crate::extract;
use crate::{error::AppError, AppState};
use axum::{
    extract::{Path, Query, State},
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
}

pub fn admin_router() -> Router<AppState> {
    Router::new().route("/admin/carts", get(admin_list_carts))
}

#[tracing::instrument(skip_all)]
async fn store_create_cart(
    State(state): State<AppState>,
    extract::Json(payload): extract::Json<CreateCartInput>,
) -> Result<Json<CartResponse>, AppError> {
    payload
        .validate()
        .map_err(|e| AppError::InvalidData(e.to_string()))?;
    let cart_with_items = state.repos.cart.create_cart(payload).await?;
    Ok(Json(CartResponse {
        cart: cart_with_items,
    }))
}

#[tracing::instrument(skip_all, fields(id = %id))]
async fn store_get_cart(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<CartResponse>, AppError> {
    let cart = state.repos.cart.get_cart(&id).await?;
    Ok(Json(CartResponse { cart }))
}

#[tracing::instrument(skip_all, fields(id = %id))]
async fn store_update_cart(
    State(state): State<AppState>,
    Path(id): Path<String>,
    extract::Json(payload): extract::Json<UpdateCartInput>,
) -> Result<Json<CartResponse>, AppError> {
    payload
        .validate()
        .map_err(|e| AppError::InvalidData(e.to_string()))?;
    let cart = state.repos.cart.update_cart(&id, payload).await?;
    Ok(Json(CartResponse { cart }))
}

#[tracing::instrument(skip_all, fields(cart_id = %id))]
async fn store_add_line_item(
    State(state): State<AppState>,
    Path(id): Path<String>,
    extract::Json(payload): extract::Json<AddLineItemInput>,
) -> Result<Json<CartResponse>, AppError> {
    payload
        .validate()
        .map_err(|e| AppError::InvalidData(e.to_string()))?;
    let cart = state.repos.cart.add_line_item(&id, payload).await?;
    Ok(Json(CartResponse { cart }))
}

#[tracing::instrument(skip_all, fields(cart_id = %id, line_id = %line_id))]
async fn store_delete_line_item(
    State(state): State<AppState>,
    Path((id, line_id)): Path<(String, String)>,
) -> Result<Json<LineItemDeleteResponse>, AppError> {
    let cart = state.repos.cart.delete_line_item(&id, &line_id).await?;
    Ok(Json(LineItemDeleteResponse {
        id: line_id,
        object: "line-item".to_string(),
        deleted: true,
        parent: cart,
    }))
}

#[tracing::instrument(skip_all, fields(cart_id = %id, line_id = %line_id))]
async fn store_update_line_item(
    State(state): State<AppState>,
    Path((id, line_id)): Path<(String, String)>,
    extract::Json(payload): extract::Json<UpdateLineItemInput>,
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

#[tracing::instrument(skip_all)]
async fn admin_list_carts(
    State(state): State<AppState>,
    Query(params): Query<AdminCartListParams>,
) -> Result<Json<AdminCartListResponse>, AppError> {
    let limit = params.capped_limit();
    let (carts, count) = state.repos.cart.list(&params).await?;
    Ok(Json(AdminCartListResponse {
        carts,
        count,
        offset: params.offset,
        limit,
    }))
}
