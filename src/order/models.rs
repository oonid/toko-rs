use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Order {
    pub id: String,
    pub display_id: i64,
    pub customer_id: Option<String>,
    pub email: Option<String>,
    pub currency_code: String,
    pub status: String,
    #[serde(skip_deserializing)]
    pub shipping_address: Option<sqlx::types::Json<serde_json::Value>>,
    #[serde(skip_deserializing)]
    pub billing_address: Option<sqlx::types::Json<serde_json::Value>>,
    #[serde(skip_deserializing)]
    pub metadata: Option<sqlx::types::Json<serde_json::Value>>,
    pub canceled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct OrderLineItem {
    pub id: String,
    pub order_id: String,
    pub title: String,
    pub quantity: i64,
    pub unit_price: i64,
    pub variant_id: Option<String>,
    pub product_id: Option<String>,
    #[serde(skip_deserializing)]
    pub snapshot: Option<sqlx::types::Json<serde_json::Value>>,
    #[serde(skip_deserializing)]
    pub metadata: Option<sqlx::types::Json<serde_json::Value>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderWithItems {
    #[serde(flatten)]
    pub order: Order,
    pub items: Vec<OrderLineItem>,
    pub item_total: i64,
    pub total: i64,
}
