use crate::event::models::EventOutboxRow;
use chrono::{DateTime, Utc};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct EventListParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub after: Option<DateTime<Utc>>,
    pub resource_type: Option<String>,
    pub unprocessed_only: Option<bool>,
}

#[derive(serde::Serialize)]
pub struct EventResponse {
    pub event: EventOutboxRow,
}

#[derive(serde::Serialize)]
pub struct EventListResponse {
    pub events: Vec<EventOutboxRow>,
    pub count: i64,
    pub limit: i64,
    pub offset: i64,
}
