use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateInvoiceConfigInput {
    pub company_name: Option<String>,
    pub company_address: Option<String>,
    pub company_phone: Option<String>,
    pub company_email: Option<String>,
    pub company_logo: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct InvoiceConfigResponse {
    pub invoice_config: super::models::InvoiceConfig,
}

#[derive(Debug, serde::Serialize)]
pub struct InvoiceResponse {
    pub invoice: super::models::Invoice,
}
