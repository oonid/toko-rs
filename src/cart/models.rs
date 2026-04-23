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
    #[serde(skip)]
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
    #[serde(skip)]
    pub snapshot: Option<sqlx::types::Json<serde_json::Value>>,
    #[serde(skip_deserializing)]
    pub metadata: Option<sqlx::types::Json<serde_json::Value>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip)]
    pub deleted_at: Option<DateTime<Utc>>,
    #[sqlx(skip)]
    pub requires_shipping: bool,
    #[sqlx(skip)]
    pub is_discountable: bool,
    #[sqlx(skip)]
    pub is_tax_inclusive: bool,
    #[sqlx(skip)]
    pub product_title: Option<String>,
    #[sqlx(skip)]
    pub product_description: Option<String>,
    #[sqlx(skip)]
    pub product_subtitle: Option<String>,
    #[sqlx(skip)]
    pub product_handle: Option<String>,
    #[sqlx(skip)]
    pub variant_sku: Option<String>,
    #[sqlx(skip)]
    pub variant_barcode: Option<String>,
    #[sqlx(skip)]
    pub variant_title: Option<String>,
    #[sqlx(skip)]
    pub variant_option_values: Option<serde_json::Value>,
    #[sqlx(skip)]
    pub item_total: i64,
    #[sqlx(skip)]
    pub item_subtotal: i64,
    #[sqlx(skip)]
    pub item_tax_total: i64,
    #[sqlx(skip)]
    pub total: i64,
    #[sqlx(skip)]
    pub subtotal: i64,
    #[sqlx(skip)]
    pub tax_total: i64,
    #[sqlx(skip)]
    pub discount_total: i64,
    #[sqlx(skip)]
    pub discount_tax_total: i64,
    #[sqlx(skip)]
    pub original_total: i64,
    #[sqlx(skip)]
    pub original_subtotal: i64,
    #[sqlx(skip)]
    pub original_tax_total: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CartWithItems {
    #[serde(flatten)]
    pub cart: Cart,
    pub items: Vec<CartLineItem>,
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
}

impl CartWithItems {
    pub fn from_items(cart: Cart, mut items: Vec<CartLineItem>) -> Self {
        for item in &mut items {
            item.requires_shipping = true;
            item.is_discountable = true;
            item.is_tax_inclusive = false;
            if let Some(ref snap) = item.snapshot {
                let s = &snap.0;
                item.product_title = s
                    .get("product_title")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                item.product_description = s
                    .get("product_description")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                item.product_subtitle = s
                    .get("product_subtitle")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                item.product_handle = s
                    .get("product_handle")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                item.variant_sku = s
                    .get("variant_sku")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                item.variant_barcode = s
                    .get("variant_barcode")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                item.variant_title = s
                    .get("variant_title")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                item.variant_option_values = s.get("variant_option_values").cloned();
            }
            let line_total = item.quantity * item.unit_price;
            item.item_total = line_total;
            item.item_subtotal = line_total;
            item.item_tax_total = 0;
            item.total = line_total;
            item.subtotal = line_total;
            item.tax_total = 0;
            item.discount_total = 0;
            item.discount_tax_total = 0;
            item.original_total = line_total;
            item.original_subtotal = line_total;
            item.original_tax_total = 0;
        }
        let item_total = items.iter().map(|i| i.quantity * i.unit_price).sum();
        Self {
            cart,
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
        }
    }
}
