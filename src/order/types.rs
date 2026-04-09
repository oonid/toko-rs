use serde::{Deserialize, Serialize};

use super::models::OrderWithItems;
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
    pub order: OrderWithItems,
}

#[derive(Debug, Deserialize)]
pub struct ListOrdersParams {
    #[serde(default)]
    pub offset: i64,
    #[serde(default = "types::default_limit")]
    pub limit: i64,
}
