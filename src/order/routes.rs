use super::types::*;
use crate::customer::routes::CustomerId;
use crate::error::AppError;
use crate::AppState;
use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
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
        .route("/admin/orders", get(admin_list_orders))
        .route("/admin/orders/export", get(admin_export_orders))
        .route("/admin/orders/{id}", get(admin_get_order))
        .route("/admin/orders/{id}/cancel", post(admin_cancel_order))
        .route("/admin/orders/{id}/complete", post(admin_complete_order))
        .route("/admin/orders/{id}/fulfill", post(admin_fulfill_order))
        .route("/admin/orders/{id}/ship", post(admin_ship_order))
        .route(
            "/admin/orders/{id}/capture-payment",
            post(admin_capture_payment),
        )
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
    state.repos.payment.cancel_by_order_id(&id).await?;
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

#[tracing::instrument(skip_all, fields(offset = params.offset, limit = params.limit))]
async fn admin_list_orders(
    State(state): State<AppState>,
    Query(params): Query<AdminListOrdersParams>,
) -> Result<Json<OrderListResponse>, AppError> {
    let limit = params.capped_limit();
    let (orders, count) = state.repos.order.list_all(&params).await?;
    Ok(Json(OrderListResponse {
        orders,
        count,
        offset: params.offset,
        limit,
    }))
}

#[tracing::instrument(skip_all, fields(id = %id))]
async fn admin_get_order(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<OrderResponse>, AppError> {
    let order = state.repos.order.find_by_id(&id).await?;
    Ok(Json(OrderResponse { order }))
}

#[tracing::instrument(skip_all, fields(id = %id))]
async fn admin_fulfill_order(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<OrderResponse>, AppError> {
    let order = state.repos.order.fulfill_order(&id).await?;
    Ok(Json(OrderResponse { order }))
}

#[tracing::instrument(skip_all, fields(id = %id))]
async fn admin_ship_order(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<OrderResponse>, AppError> {
    let order = state.repos.order.ship_order(&id).await?;
    Ok(Json(OrderResponse { order }))
}

#[tracing::instrument(skip_all, fields(id = %id))]
async fn admin_capture_payment(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<OrderResponse>, AppError> {
    state.repos.payment.capture_by_order_id(&id).await?;
    let order = state.repos.order.find_by_id(&id).await?;
    Ok(Json(OrderResponse { order }))
}

#[tracing::instrument(skip_all)]
async fn admin_export_orders(
    State(state): State<AppState>,
    Query(params): Query<AdminExportOrdersParams>,
) -> Result<impl IntoResponse, AppError> {
    let rows = state.repos.order.export_orders(&params).await?;

    let mut wtr = csv::Writer::from_writer(vec![]);

    wtr.write_record([
        "Order ID",
        "Display ID",
        "Email",
        "Currency",
        "Status",
        "Fulfillment Status",
        "Payment Status",
        "Item Count",
        "Total (cents)",
        "Created At",
        "Shipped At",
        "Canceled At",
    ])
    .map_err(|e| AppError::InvalidData(e.to_string()))?;

    for row in &rows {
        wtr.write_record([
            row.id.as_str(),
            &row.display_id.to_string(),
            row.email.as_deref().unwrap_or(""),
            row.currency_code.as_str(),
            row.status.as_str(),
            row.fulfillment_status.as_str(),
            row.payment_status.as_str(),
            &row.item_count.to_string(),
            &row.total_cents.to_string(),
            &row.created_at.to_rfc3339(),
            row.shipped_at
                .map(|t| t.to_rfc3339())
                .as_deref()
                .unwrap_or(""),
            row.canceled_at
                .map(|t| t.to_rfc3339())
                .as_deref()
                .unwrap_or(""),
        ])
        .map_err(|e| AppError::InvalidData(e.to_string()))?;
    }

    let data = wtr
        .into_inner()
        .map_err(|e| AppError::InvalidData(e.to_string()))?;

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"orders.csv\"",
            ),
        ],
        data,
    ))
}
