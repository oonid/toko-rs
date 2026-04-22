use serde::Deserialize;
use std::collections::HashMap;
use ulid::Ulid;

pub fn generate_entity_id(prefix: &str) -> String {
    let id = Ulid::new().to_string();
    format!("{}_{}", prefix, id)
}

pub fn generate_handle(title: &str) -> String {
    slug::slugify(title)
}

pub fn metadata_to_json(
    m: Option<HashMap<String, serde_json::Value>>,
) -> Option<sqlx::types::Json<serde_json::Value>> {
    m.map(|map| sqlx::types::Json(serde_json::to_value(map).unwrap()))
}

#[derive(Debug, Deserialize)]
pub struct FindParams {
    #[serde(default)]
    pub offset: i64,
    #[serde(default = "default_limit")]
    pub limit: i64,
    pub order: Option<String>,
    pub fields: Option<String>,
    pub with_deleted: Option<bool>,
}

impl FindParams {
    pub fn capped_limit(&self) -> i64 {
        self.limit.min(100)
    }
}

pub fn default_limit() -> i64 {
    50
}
