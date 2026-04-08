use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Customer {
    pub id: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: String,
    pub phone: Option<String>,
    pub has_account: bool,
    #[serde(skip_deserializing)]
    pub metadata: Option<sqlx::types::Json<serde_json::Value>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}
