use crate::webhook::models::WebhookSubscription;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreateWebhookInput {
    pub url: String,
    pub events: Vec<String>,
    pub secret: String,
}

#[derive(Serialize)]
pub struct WebhookResponse {
    pub webhook: WebhookSubscription,
}

#[derive(Serialize)]
pub struct WebhookListResponse {
    pub webhooks: Vec<WebhookSubscription>,
    pub count: i64,
}
