use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post, delete},
    Json, Router,
};
use validator::Validate;
use crate::{AppState, error::AppError, types::FindParams};
use super::types::*;

pub fn router() -> Router<AppState> {
    Router::new()
        // Admin routes
        .route("/admin/products", post(admin_create_product).get(admin_list_products))
        .route("/admin/products/{id}", get(admin_get_product).post(admin_update_product).delete(admin_delete_product))
        .route("/admin/products/{id}/variants", post(admin_add_variant))
        // Store routes
        .route("/store/products", get(store_list_products))
        .route("/store/products/{id}", get(store_get_product))
}

async fn admin_create_product(
    State(state): State<AppState>, 
    Json(payload): Json<CreateProductInput>
) -> Result<Json<ProductResponse>, AppError> {
    payload.validate().map_err(|e| AppError::InvalidData(e.to_string()))?;
    
    let product_with_relations = state.product_repo.create_product(payload).await?;
    
    Ok(Json(ProductResponse {
        product: product_with_relations,
    }))
}

async fn admin_list_products(State(_state): State<AppState>, Query(_params): Query<FindParams>) -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

async fn admin_get_product(State(_state): State<AppState>, Path(_id): Path<String>) -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

async fn admin_update_product(State(_state): State<AppState>, Path(_id): Path<String>, Json(_payload): Json<UpdateProductInput>) -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

async fn admin_delete_product(State(_state): State<AppState>, Path(_id): Path<String>) -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

async fn admin_add_variant(State(_state): State<AppState>, Path(_id): Path<String>, Json(_payload): Json<CreateProductVariantInput>) -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

async fn store_list_products(State(_state): State<AppState>, Query(_params): Query<FindParams>) -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}

async fn store_get_product(State(_state): State<AppState>, Path(_id): Path<String>) -> StatusCode {
    StatusCode::NOT_IMPLEMENTED
}
