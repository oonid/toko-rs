use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Product {
    pub id: String,
    pub title: String,
    pub handle: String,
    pub description: Option<String>,
    pub status: String,
    pub thumbnail: Option<String>,
    #[serde(skip_deserializing)]
    pub metadata: Option<sqlx::types::Json<serde_json::Value>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip)]
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct ProductOption {
    pub id: String,
    pub product_id: String,
    pub title: String,
    #[serde(skip_deserializing)]
    pub metadata: Option<sqlx::types::Json<serde_json::Value>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip)]
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct ProductOptionValue {
    pub id: String,
    pub option_id: String,
    pub value: String,
    #[serde(skip_deserializing)]
    pub metadata: Option<sqlx::types::Json<serde_json::Value>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip)]
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct ProductVariant {
    pub id: String,
    pub product_id: String,
    pub title: String,
    pub sku: Option<String>,
    pub price: i64,
    pub variant_rank: i64,
    #[serde(skip_deserializing)]
    pub metadata: Option<sqlx::types::Json<serde_json::Value>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip)]
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImageStub {
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProductWithRelations {
    #[serde(flatten)]
    pub product: Product,
    pub options: Vec<ProductOptionWithValues>,
    pub variants: Vec<ProductVariantWithOptions>,
    pub images: Vec<ImageStub>,
    pub is_giftcard: bool,
    pub discountable: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProductOptionWithValues {
    #[serde(flatten)]
    pub option: ProductOption,
    pub values: Vec<ProductOptionValue>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProductVariantWithOptions {
    #[serde(flatten)]
    pub variant: ProductVariant,
    pub options: Vec<VariantOptionValue>,
    pub calculated_price: CalculatedPrice,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CalculatedPrice {
    pub calculated_amount: i64,
    pub original_amount: i64,
    pub is_calculated_price_tax_inclusive: bool,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct VariantOptionValue {
    pub id: String,
    pub value: String,
    pub option_id: String,
}
