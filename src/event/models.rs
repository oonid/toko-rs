use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Serialize, sqlx::FromRow, Clone)]
pub struct EventOutboxRow {
    pub id: String,
    pub event_name: String,
    pub resource_type: String,
    pub resource_id: String,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
}
