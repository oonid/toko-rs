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

#[allow(dead_code)]
pub mod bool_or_string {
    use serde::de;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct BoolOrString;

        impl<'de> serde::de::Visitor<'de> for BoolOrString {
            type Value = Option<bool>;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a boolean, a string \"true\"/\"false\", or null")
            }

            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(Some(v))
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                match v {
                    "true" => Ok(Some(true)),
                    "false" => Ok(Some(false)),
                    _ => Err(de::Error::custom(format!(
                        "invalid boolean string: \"{}\"",
                        v
                    ))),
                }
            }

            fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(None)
            }

            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(None)
            }

            fn visit_some<D2: serde::Deserializer<'de>>(
                self,
                deserializer: D2,
            ) -> Result<Self::Value, D2::Error> {
                deserializer.deserialize_any(self)
            }
        }

        deserializer.deserialize_any(BoolOrString)
    }
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

    #[test]
    fn test_validate_order_param_single_column() {
        assert_eq!(validate_order_param("created_at").unwrap(), "created_at");
        assert_eq!(validate_order_param("title").unwrap(), "title");
    }

    #[test]
    fn test_validate_order_param_empty_clause() {
        assert!(validate_order_param("  ,  ").is_err());
    }

    #[test]
    fn test_bool_or_string_from_bool() {
        #[derive(Deserialize)]
        struct Test {
            #[serde(default, deserialize_with = "bool_or_string::deserialize")]
            val: Option<bool>,
        }
        let t: Test = serde_json::from_str(r#"{"val": true}"#).unwrap();
        assert_eq!(t.val, Some(true));
        let t: Test = serde_json::from_str(r#"{"val": false}"#).unwrap();
        assert_eq!(t.val, Some(false));
    }

    #[test]
    fn test_bool_or_string_from_str() {
        #[derive(Deserialize)]
        struct Test {
            #[serde(default, deserialize_with = "bool_or_string::deserialize")]
            val: Option<bool>,
        }
        let t: Test = serde_json::from_str(r#"{"val": "true"}"#).unwrap();
        assert_eq!(t.val, Some(true));
        let t: Test = serde_json::from_str(r#"{"val": "false"}"#).unwrap();
        assert_eq!(t.val, Some(false));
    }

    #[test]
    fn test_bool_or_string_invalid_str() {
        #[derive(Deserialize)]
        struct Test {
            #[serde(default, deserialize_with = "bool_or_string::deserialize")]
            val: Option<bool>,
        }
        let result: Result<Test, _> = serde_json::from_str(r#"{"val": "yes"}"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_bool_or_string_null() {
        #[derive(Deserialize)]
        struct Test {
            #[serde(default, deserialize_with = "bool_or_string::deserialize")]
            val: Option<bool>,
        }
        let t: Test = serde_json::from_str(r#"{"val": null}"#).unwrap();
        assert_eq!(t.val, None);
    }

    #[test]
    fn test_bool_or_string_absent() {
        #[derive(Deserialize)]
        struct Test {
            #[serde(default, deserialize_with = "bool_or_string::deserialize")]
            val: Option<bool>,
        }
        let t: Test = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(t.val, None);
    }
}
