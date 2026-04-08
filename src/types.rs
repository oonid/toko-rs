use serde::Deserialize;
use ulid::Ulid;

/// Generates an entity ID using ULID and an optional prefix.
/// Example: `prod_01ARZ3NDEKTSV4RRFFQ69G5FAV`
pub fn generate_entity_id(prefix: &str) -> String {
    let id = Ulid::new().to_string();
    format!("{}_{}", prefix, id)
}

/// Generates a URL-safe handle from a title.
pub fn generate_handle(title: &str) -> String {
    slug::slugify(title)
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

pub fn default_limit() -> i64 {
    20
}
