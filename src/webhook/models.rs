use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct WebhookSubscription {
    pub id: String,
    pub url: String,
    pub events: serde_json::Value,
    pub secret: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}
