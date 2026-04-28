use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::Validate;

use super::models::{ProductOptionWithValues, ProductVariantWithOptions, ProductWithRelations};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProductStatus {
    #[default]
    Draft,
    Proposed,
    Published,
    Rejected,
}

impl ProductStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Proposed => "proposed",
            Self::Published => "published",
            Self::Rejected => "rejected",
        }
    }
}

// --- API Request Inputs ---

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreateProductInput {
    #[validate(length(min = 1, message = "Title cannot be empty"))]
    pub title: String,
    pub subtitle: Option<String>,
    pub handle: Option<String>,
    pub description: Option<String>,
    pub status: Option<ProductStatus>,
    pub thumbnail: Option<String>,
    #[serde(
        default,
        deserialize_with = "crate::types::bool_or_string::deserialize"
    )]
    pub is_giftcard: Option<bool>,
    #[serde(
        default,
        deserialize_with = "crate::types::bool_or_string::deserialize"
    )]
    pub discountable: Option<bool>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    pub images: Option<Vec<String>>,
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
#[serde(deny_unknown_fields)]
pub struct CreateProductVariantInput {
    #[validate(length(min = 1, message = "Variant title cannot be empty"))]
    pub title: String,
    pub sku: Option<String>,
    pub thumbnail: Option<String>,
    #[validate(range(min = 0, message = "Price cannot be negative"))]
    pub price: i64,
    pub variant_rank: Option<i64>,
    pub options: Option<HashMap<String, String>>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    pub images: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct UpdateProductInput {
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub handle: Option<String>,
    pub description: Option<String>,
    pub status: Option<ProductStatus>,
    pub thumbnail: Option<String>,
    #[serde(
        default,
        deserialize_with = "crate::types::bool_or_string::deserialize"
    )]
    pub is_giftcard: Option<bool>,
    #[serde(
        default,
        deserialize_with = "crate::types::bool_or_string::deserialize"
    )]
    pub discountable: Option<bool>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
    pub images: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateVariantInput {
    pub title: Option<String>,
    pub sku: Option<String>,
    pub thumbnail: Option<String>,
    #[validate(range(min = 0, message = "Price cannot be negative"))]
    pub price: Option<i64>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
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
pub struct VariantListResponse {
    pub variants: Vec<ProductVariantWithOptions>,
    pub count: i64,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Serialize)]
pub struct VariantResponse {
    pub variant: ProductVariantWithOptions,
}

#[derive(Debug, Serialize)]
pub struct VariantDeleteResponse {
    pub id: String,
    pub object: String,
    pub deleted: bool,
    pub parent: ProductWithRelations,
}

#[derive(Debug, Serialize)]
pub struct DeleteResponse {
    pub id: String,
    pub object: String,
    pub deleted: bool,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateOptionInput {
    #[validate(length(min = 1, message = "Option title cannot be empty"))]
    pub title: String,
    pub values: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateOptionInput {
    pub title: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProductOptionResponse {
    pub product_option: ProductOptionWithValues,
}

#[derive(Debug, Serialize)]
pub struct ProductOptionListResponse {
    pub product_options: Vec<ProductOptionWithValues>,
    pub count: i64,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Serialize)]
pub struct ProductOptionDeleteResponse {
    pub id: String,
    pub object: String,
    pub deleted: bool,
    pub parent: ProductWithRelations,
}
