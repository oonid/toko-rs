use super::types::*;
use crate::customer::routes::CustomerId;
use crate::error::AppError;
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};

pub fn router() -> Router<AppState> {
    Router::new().route("/store/carts/{id}/complete", post(store_complete_cart))
}

pub fn protected_router() -> Router<AppState> {
    Router::new()
        .route("/store/orders", get(store_list_orders))
        .route("/store/orders/{id}", get(store_get_order))
}

pub fn admin_router() -> Router<AppState> {
    Router::new()
        .route("/admin/orders/{id}/cancel", post(admin_cancel_order))
        .route("/admin/orders/{id}/complete", post(admin_complete_order))
}

#[tracing::instrument(skip_all, fields(cart_id = %cart_id))]
async fn store_complete_cart(
    State(state): State<AppState>,
    Path(cart_id): Path<String>,
) -> Result<(StatusCode, Json<CartCompleteResponse>), AppError> {
    let order_with_items = state.repos.order.create_from_cart(&cart_id).await?;

    Ok((
        StatusCode::OK,
        Json(CartCompleteResponse::success(order_with_items)),
    ))
}

#[tracing::instrument(skip_all, fields(customer_id = %customer.id, offset = params.offset, limit = params.limit))]
async fn store_list_orders(
    State(state): State<AppState>,
    axum::Extension(customer): axum::Extension<CustomerId>,
    Query(params): Query<ListOrdersParams>,
) -> Result<Json<OrderListResponse>, AppError> {
    let (orders, count) = state
        .repos
        .order
        .list_by_customer(&customer.id, &params)
        .await?;

    Ok(Json(OrderListResponse {
        orders,
        count,
        offset: params.offset,
        limit: params.capped_limit(),
    }))
}

#[tracing::instrument(skip_all, fields(id = %id))]
async fn store_get_order(
    State(state): State<AppState>,
    axum::Extension(customer): axum::Extension<CustomerId>,
    Path(id): Path<String>,
) -> Result<Json<OrderResponse>, AppError> {
    let order = state.repos.order.find_by_id(&id).await?;

    if order.order.customer_id.as_deref() != Some(customer.id.as_str()) {
        return Err(AppError::NotFound(format!(
            "Order with id: {} was not found",
            id
        )));
    }

    Ok(Json(OrderResponse { order }))
}

#[tracing::instrument(skip_all, fields(id = %id))]
async fn admin_cancel_order(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<OrderResponse>, AppError> {
    let order = state.repos.order.cancel_order(&id).await?;
    let _ = state.repos.payment.cancel_by_order_id(&id).await;
    Ok(Json(OrderResponse { order }))
}

#[tracing::instrument(skip_all, fields(id = %id))]
async fn admin_complete_order(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<OrderResponse>, AppError> {
    let order = state.repos.order.complete_order(&id).await?;
    Ok(Json(OrderResponse { order }))
}
