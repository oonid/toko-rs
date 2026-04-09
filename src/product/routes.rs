use super::types::*;
use crate::{error::AppError, types::FindParams, AppState};
use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use validator::Validate;

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/admin/products",
            post(admin_create_product).get(admin_list_products),
        )
        .route(
            "/admin/products/{id}",
            get(admin_get_product)
                .post(admin_update_product)
                .delete(admin_delete_product),
        )
        .route("/admin/products/{id}/variants", post(admin_add_variant))
        .route("/store/products", get(store_list_products))
        .route("/store/products/{id}", get(store_get_product))
}

#[tracing::instrument(skip_all)]
async fn admin_create_product(
    State(state): State<AppState>,
    Json(payload): Json<CreateProductInput>,
) -> Result<Json<ProductResponse>, AppError> {
    payload
        .validate()
        .map_err(|e| AppError::InvalidData(e.to_string()))?;

    let product = state.repos.product.create_product(payload).await?;

    Ok(Json(ProductResponse { product }))
}

#[tracing::instrument(skip_all, fields(offset = params.offset, limit = params.limit))]
async fn admin_list_products(
    State(state): State<AppState>,
    Query(params): Query<FindParams>,
) -> Result<Json<ProductListResponse>, AppError> {
    let (products, count) = state.repos.product.list(&params).await?;

    Ok(Json(ProductListResponse {
        products,
        count,
        offset: params.offset,
        limit: params.capped_limit(),
    }))
}

#[tracing::instrument(skip_all, fields(id = %id))]
async fn admin_get_product(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ProductResponse>, AppError> {
    let product = state.repos.product.find_by_id(&id).await?;
    Ok(Json(ProductResponse { product }))
}

#[tracing::instrument(skip_all, fields(id = %id))]
async fn admin_update_product(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateProductInput>,
) -> Result<Json<ProductResponse>, AppError> {
    payload
        .validate()
        .map_err(|e| AppError::InvalidData(e.to_string()))?;

    let product = state.repos.product.update(&id, &payload).await?;
    Ok(Json(ProductResponse { product }))
}

#[tracing::instrument(skip_all, fields(id = %id))]
async fn admin_delete_product(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<DeleteResponse>, AppError> {
    state.repos.product.soft_delete(&id).await?;

    Ok(Json(DeleteResponse {
        id,
        object: "product".to_string(),
        deleted: true,
    }))
}

#[tracing::instrument(skip_all, fields(product_id = %id))]
async fn admin_add_variant(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<CreateProductVariantInput>,
) -> Result<Json<ProductResponse>, AppError> {
    payload
        .validate()
        .map_err(|e| AppError::InvalidData(e.to_string()))?;

    let product = state.repos.product.add_variant(&id, &payload).await?;
    Ok(Json(ProductResponse { product }))
}

#[tracing::instrument(skip_all, fields(offset = params.offset, limit = params.limit))]
async fn store_list_products(
    State(state): State<AppState>,
    Query(params): Query<FindParams>,
) -> Result<Json<ProductListResponse>, AppError> {
    let (products, count) = state.repos.product.list_published(&params).await?;

    Ok(Json(ProductListResponse {
        products,
        count,
        offset: params.offset,
        limit: params.capped_limit(),
    }))
}

#[tracing::instrument(skip_all, fields(id = %id))]
async fn store_get_product(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ProductResponse>, AppError> {
    let product = state.repos.product.find_published_by_id(&id).await?;
    Ok(Json(ProductResponse { product }))
}
