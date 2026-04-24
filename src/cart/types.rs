use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct CreateCartInput {
    pub customer_id: Option<String>,
    #[validate(email)]
    pub email: Option<String>,
    #[validate(length(min = 3, max = 3))]
    pub currency_code: Option<String>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
#[serde(deny_unknown_fields)]
pub struct UpdateCartInput {
    pub customer_id: Option<String>,
    #[validate(email)]
    pub email: Option<String>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct AddLineItemInput {
    #[validate(length(min = 1, message = "variant_id is required"))]
    pub variant_id: String,
    #[validate(range(min = 1))]
    pub quantity: i64,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize, Validate)]
pub struct UpdateLineItemInput {
    #[validate(range(min = 1, message = "Quantity must be at least 1"))]
    pub quantity: i64,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CartResponse {
    pub cart: super::models::CartWithItems,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LineItemDeleteResponse {
    pub id: String,
    pub object: String,
    pub deleted: bool,
    pub parent: super::models::CartWithItems,
}
