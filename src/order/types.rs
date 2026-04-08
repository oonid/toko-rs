use serde::{Deserialize, Serialize};

use super::models::OrderWithItems;
use crate::payment::models::PaymentRecord;
use crate::types;

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderResponse {
    pub order: OrderWithItems,
    pub payment: Option<PaymentRecord>,
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
    pub payment: PaymentRecord,
}

#[derive(Debug, Deserialize)]
pub struct ListOrdersParams {
    #[serde(default)]
    pub offset: i64,
    #[serde(default = "types::default_limit")]
    pub limit: i64,
}
