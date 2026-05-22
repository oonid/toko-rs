use serde::{Deserialize, Serialize};

use super::models::OrderWithItems;
use crate::cart::models::CartWithItems;
use crate::types;

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderResponse {
    pub order: OrderWithItems,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderListResponse {
    pub orders: Vec<OrderWithItems>,
    pub count: i64,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CartCompleteResponse {
    #[serde(rename = "type")]
    pub response_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<OrderWithItems>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cart: Option<CartWithItems>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<CartCompleteError>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CartCompleteError {
    pub message: String,
    pub name: String,
    #[serde(rename = "type")]
    pub error_type: String,
}

impl CartCompleteResponse {
    pub fn success(order: OrderWithItems) -> Self {
        Self {
            response_type: "order".to_string(),
            order: Some(order),
            cart: None,
            error: None,
        }
    }

    pub fn error(cart: CartWithItems, message: impl Into<String>) -> Self {
        Self {
            response_type: "cart".to_string(),
            order: None,
            cart: Some(cart),
            error: Some(CartCompleteError {
                message: message.into(),
                name: "unknown_error".to_string(),
                error_type: "invalid_data".to_string(),
            }),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ListOrdersParams {
    #[serde(default)]
    pub offset: i64,
    #[serde(default = "types::default_limit")]
    pub limit: i64,
    pub id: Option<String>,
    pub status: Option<String>,
}

impl ListOrdersParams {
    pub fn capped_limit(&self) -> i64 {
        self.limit.min(100)
    }
}

#[derive(Debug, Deserialize)]
pub struct AdminListOrdersParams {
    pub customer_id: Option<String>,
    pub status: Option<String>,
    #[serde(default)]
    pub offset: i64,
    #[serde(default = "types::default_limit")]
    pub limit: i64,
}

impl AdminListOrdersParams {
    pub fn capped_limit(&self) -> i64 {
        self.limit.min(100)
    }
}

#[derive(Debug, Deserialize)]
pub struct AdminExportOrdersParams {
    pub status: Option<String>,
    pub created_at_from: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at_to: Option<chrono::DateTime<chrono::Utc>>,
    pub q: Option<String>,
}

pub struct OrderExportRow {
    pub id: String,
    pub display_id: i64,
    pub email: Option<String>,
    pub currency_code: String,
    pub status: String,
    pub fulfillment_status: String,
    pub payment_status: String,
    pub item_count: i64,
    pub total_cents: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub shipped_at: Option<chrono::DateTime<chrono::Utc>>,
    pub canceled_at: Option<chrono::DateTime<chrono::Utc>>,
}
