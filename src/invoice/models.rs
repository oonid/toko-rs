use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::order::models::OrderWithItems;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct InvoiceConfig {
    pub id: String,
    pub company_name: String,
    pub company_address: String,
    pub company_phone: String,
    pub company_email: String,
    pub company_logo: Option<String>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Clone)]
pub struct Invoice {
    pub invoice_number: String,
    pub date: DateTime<Utc>,
    pub status: String,
    pub issuer: InvoiceIssuer,
    pub order: OrderWithItems,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct InvoiceIssuer {
    pub company_name: String,
    pub company_address: String,
    pub company_phone: String,
    pub company_email: String,
    pub company_logo: Option<String>,
}

impl From<&InvoiceConfig> for InvoiceIssuer {
    fn from(config: &InvoiceConfig) -> Self {
        Self {
            company_name: config.company_name.clone(),
            company_address: config.company_address.clone(),
            company_phone: config.company_phone.clone(),
            company_email: config.company_email.clone(),
            company_logo: config.company_logo.clone(),
        }
    }
}

impl Invoice {
    pub fn from_order(config: &InvoiceConfig, order: OrderWithItems) -> Self {
        Self {
            invoice_number: format!("INV-{:04}", order.order.display_id),
            date: order.order.created_at,
            status: "latest".to_string(),
            issuer: InvoiceIssuer::from(config),
            notes: config.notes.clone(),
            order,
        }
    }
}
