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

async fn admin_create_product(
    State(state): State<AppState>,
    Json(payload): Json<CreateProductInput>,
) -> Result<Json<ProductResponse>, AppError> {
    payload
        .validate()
        .map_err(|e| AppError::InvalidData(e.to_string()))?;

    let product = state.product_repo.create_product(payload).await?;

    Ok(Json(ProductResponse { product }))
}

async fn admin_list_products(
    State(state): State<AppState>,
    Query(params): Query<FindParams>,
) -> Result<Json<ProductListResponse>, AppError> {
    let (products, count) = state.product_repo.list_products(&params).await?;

    Ok(Json(ProductListResponse {
        products,
        count,
        offset: params.offset,
        limit: params.limit,
    }))
}

async fn admin_get_product(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ProductResponse>, AppError> {
    let product = state.product_repo.get_product(&id).await?;
    Ok(Json(ProductResponse { product }))
}

async fn admin_update_product(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<UpdateProductInput>,
) -> Result<Json<ProductResponse>, AppError> {
    let product = state.product_repo.update_product(&id, &payload).await?;
    Ok(Json(ProductResponse { product }))
}

async fn admin_delete_product(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<DeleteResponse>, AppError> {
    state.product_repo.delete_product(&id).await?;

    Ok(Json(DeleteResponse {
        id,
        object: "product".to_string(),
        deleted: true,
    }))
}

async fn admin_add_variant(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(payload): Json<CreateProductVariantInput>,
) -> Result<Json<ProductResponse>, AppError> {
    payload
        .validate()
        .map_err(|e| AppError::InvalidData(e.to_string()))?;

    let product = state.product_repo.add_variant(&id, &payload).await?;
    Ok(Json(ProductResponse { product }))
}

async fn store_list_products(
    State(state): State<AppState>,
    Query(params): Query<FindParams>,
) -> Result<Json<ProductListResponse>, AppError> {
    let (products, count) = state.product_repo.list_published_products(&params).await?;

    Ok(Json(ProductListResponse {
        products,
        count,
        offset: params.offset,
        limit: params.limit,
    }))
}

async fn store_get_product(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ProductResponse>, AppError> {
    let product = state.product_repo.get_published_product(&id).await?;
    Ok(Json(ProductResponse { product }))
}
