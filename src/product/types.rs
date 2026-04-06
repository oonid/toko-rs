use serde::{Deserialize, Serialize};
use validator::Validate;
use std::collections::HashMap;

use super::models::ProductWithRelations;

// --- API Request Inputs ---

#[derive(Debug, Deserialize, Validate)]
pub struct CreateProductInput {
    #[validate(length(min = 1, message = "Title cannot be empty"))]
    pub title: String,
    pub handle: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>, // 'draft' by default
    pub thumbnail: Option<String>,
    pub metadata: Option<serde_json::Value>,
    #[validate(nested)]
    pub options: Option<Vec<CreateProductOptionInput>>,
    #[validate(nested)]
    pub variants: Option<Vec<CreateProductVariantInput>>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateProductOptionInput {
    #[validate(length(min = 1, message = "Option title cannot be empty"))]
    pub title: String,
    pub values: Vec<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateProductVariantInput {
    #[validate(length(min = 1, message = "Variant title cannot be empty"))]
    pub title: String,
    pub sku: Option<String>,
    #[validate(range(min = 0, message = "Price cannot be negative"))]
    pub price: i64,
    // Maps Option Title -> Option Value (e.g. "Size" -> "S")
    pub options: Option<HashMap<String, String>>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProductInput {
    pub title: Option<String>,
    pub handle: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub thumbnail: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

// --- API Responses ---

#[derive(Debug, Serialize)]
pub struct ProductResponse {
    pub product: ProductWithRelations,
}

#[derive(Debug, Serialize)]
pub struct ProductListResponse {
    pub products: Vec<ProductWithRelations>,
    pub count: i64,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub id: String,
    pub object: String,
    pub deleted: bool,
}
