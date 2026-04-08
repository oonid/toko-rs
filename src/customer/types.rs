use serde::{Deserialize, Serialize};
use validator::Validate;

use super::models::Customer;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateCustomerInput {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    pub phone: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateCustomerInput {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub phone: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct CustomerResponse {
    pub customer: Customer,
}
