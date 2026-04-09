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
    #[sqlx(skip)]
    pub requires_shipping: bool,
    #[sqlx(skip)]
    pub is_discountable: bool,
    #[sqlx(skip)]
    pub is_tax_inclusive: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrderWithItems {
    #[serde(flatten)]
    pub order: Order,
    pub items: Vec<OrderLineItem>,
    pub item_total: i64,
    pub item_subtotal: i64,
    pub item_tax_total: i64,
    pub total: i64,
    pub subtotal: i64,
    pub tax_total: i64,
    pub discount_total: i64,
    pub discount_tax_total: i64,
    pub shipping_total: i64,
    pub shipping_subtotal: i64,
    pub shipping_tax_total: i64,
    pub original_total: i64,
    pub original_subtotal: i64,
    pub original_tax_total: i64,
    pub original_item_total: i64,
    pub original_item_subtotal: i64,
    pub original_item_tax_total: i64,
    pub original_shipping_total: i64,
    pub original_shipping_subtotal: i64,
    pub original_shipping_tax_total: i64,
    pub gift_card_total: i64,
    pub gift_card_tax_total: i64,
    pub payment_status: String,
    pub fulfillment_status: String,
    pub fulfillments: Vec<serde_json::Value>,
    pub shipping_methods: Vec<serde_json::Value>,
}

impl OrderWithItems {
    pub fn from_items(order: Order, mut items: Vec<OrderLineItem>) -> Self {
        for item in &mut items {
            item.requires_shipping = true;
            item.is_discountable = true;
            item.is_tax_inclusive = false;
        }
        let item_total = items.iter().map(|i| i.quantity * i.unit_price).sum();
        Self {
            order,
            item_total,
            item_subtotal: item_total,
            item_tax_total: 0,
            total: item_total,
            subtotal: item_total,
            tax_total: 0,
            discount_total: 0,
            discount_tax_total: 0,
            shipping_total: 0,
            shipping_subtotal: 0,
            shipping_tax_total: 0,
            original_total: item_total,
            original_subtotal: item_total,
            original_tax_total: 0,
            original_item_total: item_total,
            original_item_subtotal: item_total,
            original_item_tax_total: 0,
            original_shipping_total: 0,
            original_shipping_subtotal: 0,
            original_shipping_tax_total: 0,
            gift_card_total: 0,
            gift_card_tax_total: 0,
            items,
            payment_status: "not_paid".to_string(),
            fulfillment_status: "not_fulfilled".to_string(),
            fulfillments: vec![],
            shipping_methods: vec![],
        }
    }
}
