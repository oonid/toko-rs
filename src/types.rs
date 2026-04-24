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

const ALLOWED_ORDER_COLUMNS: &[&str] = &[
    "id",
    "title",
    "handle",
    "status",
    "created_at",
    "updated_at",
    "p.id",
    "p.title",
    "p.handle",
    "p.status",
    "p.created_at",
    "p.updated_at",
    "variant_rank",
    "price",
    "sku",
];

fn validate_order_token(token: &str) -> bool {
    let token = token.trim().to_lowercase();
    ALLOWED_ORDER_COLUMNS.iter().any(|col| *col == token)
}

pub fn validate_order_param(order: &str) -> Result<String, String> {
    let parts: Vec<&str> = order.split(',').collect();
    let mut validated = Vec::with_capacity(parts.len());
    for part in parts {
        let tokens: Vec<&str> = part.split_whitespace().collect();
        if tokens.is_empty() || tokens.len() > 2 {
            return Err(format!("invalid order clause: {}", part.trim()));
        }
        if !validate_order_token(tokens[0]) {
            return Err(format!("invalid order column: {}", tokens[0]));
        }
        if tokens.len() == 2 {
            let dir = tokens[1].to_uppercase();
            if dir != "ASC" && dir != "DESC" {
                return Err(format!("invalid order direction: {}", tokens[1]));
            }
            validated.push(format!("{} {}", tokens[0], dir));
        } else {
            validated.push(tokens[0].to_string());
        }
    }
    Ok(validated.join(", "))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_order_param_valid() {
        assert!(validate_order_param("created_at DESC").is_ok());
        assert!(validate_order_param("title ASC").is_ok());
        assert!(validate_order_param("p.created_at DESC").is_ok());
        assert!(validate_order_param("variant_rank ASC").is_ok());
        assert!(validate_order_param("created_at DESC, title ASC").is_ok());
    }

    #[test]
    fn test_validate_order_param_rejects_injection() {
        assert!(validate_order_param("(SELECT 1)").is_err());
        assert!(validate_order_param("1; DROP TABLE products").is_err());
        assert!(validate_order_param("created_at; DROP TABLE products").is_err());
        assert!(validate_order_param("nonexistent_column ASC").is_err());
        assert!(validate_order_param("created_at INVALID").is_err());
    }
}
