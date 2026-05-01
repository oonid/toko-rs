use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::Validate;

use super::models::{Customer, CustomerAddress};

#[derive(Debug, Deserialize, Validate)]
pub struct CreateCustomerInput {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    #[validate(email(message = "Invalid email format"))]
    pub email: Option<String>,
    pub phone: Option<String>,
    pub company_name: Option<String>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateCustomerInput {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    #[validate(email(message = "Invalid email format"))]
    pub email: Option<String>,
    pub phone: Option<String>,
    pub company_name: Option<String>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize)]
pub struct CustomerResponse {
    pub customer: CustomerWithAddresses,
}

#[derive(Debug, Serialize)]
pub struct CustomerWithAddresses {
    #[serde(flatten)]
    pub customer: Customer,
    pub addresses: Vec<CustomerAddress>,
    pub default_billing_address_id: Option<String>,
    pub default_shipping_address_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AdminCustomerListParams {
    pub q: Option<String>,
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub has_account: Option<bool>,
    #[serde(default)]
    pub offset: i64,
    #[serde(default = "crate::types::default_limit")]
    pub limit: i64,
}

impl AdminCustomerListParams {
    pub fn capped_limit(&self) -> i64 {
        self.limit.min(100)
    }
}

#[derive(Debug, Serialize)]
pub struct AdminCustomerListResponse {
    pub customers: Vec<CustomerWithAddresses>,
    pub count: i64,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Serialize)]
pub struct AdminCustomerResponse {
    pub customer: CustomerWithAddresses,
}
