use super::types::*;
use crate::extract;
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
        .route(
            "/admin/products/{id}/variants",
            post(admin_add_variant).get(admin_list_variants),
        )
        .route(
            "/admin/products/{id}/variants/{variant_id}",
            get(admin_get_variant)
                .post(admin_update_variant)
                .delete(admin_delete_variant),
        )
        .route("/store/products", get(store_list_products))
        .route("/store/products/{id}", get(store_get_product))
}

#[tracing::instrument(skip_all)]
async fn admin_create_product(
    State(state): State<AppState>,
    extract::Json(payload): extract::Json<CreateProductInput>,
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
    extract::Json(payload): extract::Json<UpdateProductInput>,
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

#[tracing::instrument(skip_all, fields(product_id = %product_id))]
async fn admin_add_variant(
    State(state): State<AppState>,
    Path(product_id): Path<String>,
    extract::Json(payload): extract::Json<CreateProductVariantInput>,
) -> Result<Json<ProductResponse>, AppError> {
    payload
        .validate()
        .map_err(|e| AppError::InvalidData(e.to_string()))?;

    let product = state
        .repos
        .product
        .add_variant(&product_id, &payload)
        .await?;
    Ok(Json(ProductResponse { product }))
}

#[tracing::instrument(skip_all, fields(product_id = %product_id, offset = params.offset, limit = params.limit))]
async fn admin_list_variants(
    State(state): State<AppState>,
    Path(product_id): Path<String>,
    Query(params): Query<FindParams>,
) -> Result<Json<VariantListResponse>, AppError> {
    let (variants, count) = state
        .repos
        .product
        .list_variants(&product_id, &params)
        .await?;

    Ok(Json(VariantListResponse {
        variants,
        count,
        offset: params.offset,
        limit: params.capped_limit(),
    }))
}

#[tracing::instrument(skip_all, fields(product_id = %product_id, variant_id = %variant_id))]
async fn admin_get_variant(
    State(state): State<AppState>,
    Path((product_id, variant_id)): Path<(String, String)>,
) -> Result<Json<VariantResponse>, AppError> {
    let variant = state
        .repos
        .product
        .get_variant(&product_id, &variant_id)
        .await?;
    Ok(Json(VariantResponse { variant }))
}

#[tracing::instrument(skip_all, fields(product_id = %product_id, variant_id = %variant_id))]
async fn admin_update_variant(
    State(state): State<AppState>,
    Path((product_id, variant_id)): Path<(String, String)>,
    extract::Json(payload): extract::Json<UpdateVariantInput>,
) -> Result<Json<crate::product::types::ProductResponse>, AppError> {
    payload
        .validate()
        .map_err(|e| AppError::InvalidData(e.to_string()))?;

    let _variant = state
        .repos
        .product
        .update_variant(&product_id, &variant_id, &payload)
        .await?;
    let parent = state.repos.product.find_by_id(&product_id).await?;
    Ok(Json(crate::product::types::ProductResponse {
        product: parent,
    }))
}

#[tracing::instrument(skip_all, fields(product_id = %product_id, variant_id = %variant_id))]
async fn admin_delete_variant(
    State(state): State<AppState>,
    Path((product_id, variant_id)): Path<(String, String)>,
) -> Result<Json<VariantDeleteResponse>, AppError> {
    let (id, parent) = state
        .repos
        .product
        .soft_delete_variant(&product_id, &variant_id)
        .await?;

    Ok(Json(VariantDeleteResponse {
        id,
        object: "variant".to_string(),
        deleted: true,
        parent,
    }))
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
