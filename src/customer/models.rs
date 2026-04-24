use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Customer {
    pub id: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: String,
    pub phone: Option<String>,
    pub company_name: Option<String>,
    pub has_account: bool,
    #[serde(skip_deserializing)]
    pub metadata: Option<sqlx::types::Json<serde_json::Value>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct CustomerAddress {
    pub id: String,
    pub customer_id: String,
    pub address_name: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub company: Option<String>,
    pub address_1: Option<String>,
    pub address_2: Option<String>,
    pub city: Option<String>,
    pub province: Option<String>,
    pub postal_code: Option<String>,
    pub country_code: Option<String>,
    pub phone: Option<String>,
    pub is_default_shipping: bool,
    pub is_default_billing: bool,
    #[serde(skip_deserializing)]
    pub metadata: Option<sqlx::types::Json<serde_json::Value>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip)]
    pub deleted_at: Option<DateTime<Utc>>,
}
