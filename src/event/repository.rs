use super::models::EventOutboxRow;
use super::types::EventListParams;
use crate::db::{DbDatabase, DbPool};
use crate::error::AppError;
use crate::types::generate_entity_id;
use chrono::Utc;
use sqlx::Transaction;

#[derive(Clone)]
pub struct EventRepository {
    pool: DbPool,
}

impl EventRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn insert_event(
        &self,
        tx: &mut Transaction<'_, DbDatabase>,
        event_name: &str,
        resource_type: &str,
        resource_id: &str,
        payload: serde_json::Value,
    ) -> Result<EventOutboxRow, AppError> {
        let id = generate_entity_id("evout");
        let created_at = Utc::now();

        let row = sqlx::query_as::<_, EventOutboxRow>(
            "INSERT INTO event_outbox (id, event_name, resource_type, resource_id, payload, created_at)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING id, event_name, resource_type, resource_id, payload, created_at, processed_at",
        )
        .bind(&id)
        .bind(event_name)
        .bind(resource_type)
        .bind(resource_id)
        .bind(&payload)
        .bind(created_at)
        .fetch_one(&mut **tx)
        .await?;

        Ok(row)
    }

    #[cfg(feature = "postgres")]
    pub async fn notify_event(&self, event_id: &str) -> Result<(), AppError> {
        sqlx::query("SELECT pg_notify('toko_events', $1)")
            .bind(event_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn list_all(
        &self,
        params: &EventListParams,
    ) -> Result<(Vec<EventOutboxRow>, i64), AppError> {
        let limit = params.limit.unwrap_or(50).min(100);
        let offset = params.offset.unwrap_or(0);

        // Get events with dynamic filtering
        let events = if let Some(ref after) = params.after {
            if let Some(ref resource_type) = params.resource_type {
                if params.unprocessed_only.unwrap_or(false) {
                    sqlx::query_as::<_, EventOutboxRow>(
                        "SELECT id, event_name, resource_type, resource_id, payload, created_at, processed_at FROM event_outbox WHERE created_at > $1 AND resource_type = $2 AND processed_at IS NULL ORDER BY created_at DESC LIMIT $3 OFFSET $4",
                    )
                    .bind(after)
                    .bind(resource_type)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(&self.pool)
                    .await?
                } else {
                    sqlx::query_as::<_, EventOutboxRow>(
                        "SELECT id, event_name, resource_type, resource_id, payload, created_at, processed_at FROM event_outbox WHERE created_at > $1 AND resource_type = $2 ORDER BY created_at DESC LIMIT $3 OFFSET $4",
                    )
                    .bind(after)
                    .bind(resource_type)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(&self.pool)
                    .await?
                }
            } else if params.unprocessed_only.unwrap_or(false) {
                sqlx::query_as::<_, EventOutboxRow>(
                    "SELECT id, event_name, resource_type, resource_id, payload, created_at, processed_at FROM event_outbox WHERE created_at > $1 AND processed_at IS NULL ORDER BY created_at DESC LIMIT $2 OFFSET $3",
                )
                .bind(after)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
            } else {
                sqlx::query_as::<_, EventOutboxRow>(
                    "SELECT id, event_name, resource_type, resource_id, payload, created_at, processed_at FROM event_outbox WHERE created_at > $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
                )
                .bind(after)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
            }
        } else if let Some(ref resource_type) = params.resource_type {
            if params.unprocessed_only.unwrap_or(false) {
                sqlx::query_as::<_, EventOutboxRow>(
                    "SELECT id, event_name, resource_type, resource_id, payload, created_at, processed_at FROM event_outbox WHERE resource_type = $1 AND processed_at IS NULL ORDER BY created_at DESC LIMIT $2 OFFSET $3",
                )
                .bind(resource_type)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
            } else {
                sqlx::query_as::<_, EventOutboxRow>(
                    "SELECT id, event_name, resource_type, resource_id, payload, created_at, processed_at FROM event_outbox WHERE resource_type = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
                )
                .bind(resource_type)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?
            }
        } else if params.unprocessed_only.unwrap_or(false) {
            sqlx::query_as::<_, EventOutboxRow>(
                "SELECT id, event_name, resource_type, resource_id, payload, created_at, processed_at FROM event_outbox WHERE processed_at IS NULL ORDER BY created_at DESC LIMIT $1 OFFSET $2",
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, EventOutboxRow>(
                "SELECT id, event_name, resource_type, resource_id, payload, created_at, processed_at FROM event_outbox ORDER BY created_at DESC LIMIT $1 OFFSET $2",
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?
        };

        // Get count
        let total: (i64,) = if let Some(ref after) = params.after {
            if let Some(ref resource_type) = params.resource_type {
                if params.unprocessed_only.unwrap_or(false) {
                    sqlx::query_as("SELECT COUNT(*) FROM event_outbox WHERE created_at > $1 AND resource_type = $2 AND processed_at IS NULL")
                        .bind(after)
                        .bind(resource_type)
                        .fetch_one(&self.pool)
                        .await?
                } else {
                    sqlx::query_as("SELECT COUNT(*) FROM event_outbox WHERE created_at > $1 AND resource_type = $2")
                        .bind(after)
                        .bind(resource_type)
                        .fetch_one(&self.pool)
                        .await?
                }
            } else if params.unprocessed_only.unwrap_or(false) {
                sqlx::query_as("SELECT COUNT(*) FROM event_outbox WHERE created_at > $1 AND processed_at IS NULL")
                    .bind(after)
                    .fetch_one(&self.pool)
                    .await?
            } else {
                sqlx::query_as("SELECT COUNT(*) FROM event_outbox WHERE created_at > $1")
                    .bind(after)
                    .fetch_one(&self.pool)
                    .await?
            }
        } else if let Some(ref resource_type) = params.resource_type {
            if params.unprocessed_only.unwrap_or(false) {
                sqlx::query_as("SELECT COUNT(*) FROM event_outbox WHERE resource_type = $1 AND processed_at IS NULL")
                    .bind(resource_type)
                    .fetch_one(&self.pool)
                    .await?
            } else {
                sqlx::query_as("SELECT COUNT(*) FROM event_outbox WHERE resource_type = $1")
                    .bind(resource_type)
                    .fetch_one(&self.pool)
                    .await?
            }
        } else if params.unprocessed_only.unwrap_or(false) {
            sqlx::query_as("SELECT COUNT(*) FROM event_outbox WHERE processed_at IS NULL")
                .fetch_one(&self.pool)
                .await?
        } else {
            sqlx::query_as("SELECT COUNT(*) FROM event_outbox")
                .fetch_one(&self.pool)
                .await?
        };

        Ok((events, total.0))
    }

    pub async fn mark_processed(&self, id: &str) -> Result<EventOutboxRow, AppError> {
        let row = sqlx::query_as::<_, EventOutboxRow>(
            "UPDATE event_outbox SET processed_at = NOW() WHERE id = $1 RETURNING id, event_name, resource_type, resource_id, payload, created_at, processed_at",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(AppError::NotFound("Event not found".to_string()))?;

        Ok(row)
    }
}
