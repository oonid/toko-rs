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
