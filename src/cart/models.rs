use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Cart {
    pub id: String,
    pub customer_id: Option<String>,
    pub email: Option<String>,
    pub currency_code: String,
    #[serde(skip_deserializing)]
    pub shipping_address: Option<sqlx::types::Json<serde_json::Value>>,
    #[serde(skip_deserializing)]
    pub billing_address: Option<sqlx::types::Json<serde_json::Value>>,
    #[serde(skip_deserializing)]
    pub metadata: Option<sqlx::types::Json<serde_json::Value>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct CartLineItem {
    pub id: String,
    pub cart_id: String,
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
pub struct CartWithItems {
    #[serde(flatten)]
    pub cart: Cart,
    pub items: Vec<CartLineItem>,
}
