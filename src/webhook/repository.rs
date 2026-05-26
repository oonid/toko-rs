use super::models::WebhookSubscription;
use super::types::CreateWebhookInput;
use crate::db::DbPool;
use crate::error::AppError;
use crate::types::generate_entity_id;

#[derive(Clone)]
pub struct WebhookRepository {
    pool: DbPool,
}

impl WebhookRepository {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        input: &CreateWebhookInput,
    ) -> Result<WebhookSubscription, AppError> {
        let id = generate_entity_id("whs");
        let events = serde_json::to_value(&input.events).unwrap_or_default();

        #[cfg(feature = "postgres")]
        {
            let webhook = sqlx::query_as::<_, WebhookSubscription>(
                "INSERT INTO webhook_subscriptions (id, url, events, secret, enabled, created_at) \
                 VALUES ($1, $2, $3, $4, true, NOW()) \
                 RETURNING id, url, events, secret, enabled, created_at",
            )
            .bind(&id)
            .bind(&input.url)
            .bind(&events)
            .bind(&input.secret)
            .fetch_one(&self.pool)
            .await?;
            Ok(webhook)
        }

        #[cfg(feature = "sqlite")]
        {
            sqlx::query(
                "INSERT INTO webhook_subscriptions (id, url, events, secret, enabled, created_at) VALUES (?, ?, ?, ?, 1, datetime('now'))"
            )
            .bind(&id)
            .bind(&input.url)
            .bind(events.to_string())
            .bind(&input.secret)
            .execute(&self.pool)
            .await?;

            let webhook = sqlx::query_as::<_, WebhookSubscription>(
                "SELECT id, url, events, secret, enabled, created_at FROM webhook_subscriptions WHERE id = ?"
            )
            .bind(&id)
            .fetch_one(&self.pool)
            .await?;
            Ok(webhook)
        }
    }

    pub async fn list_all(&self) -> Result<Vec<WebhookSubscription>, AppError> {
        let webhooks = sqlx::query_as::<_, WebhookSubscription>(
            "SELECT id, url, events, secret, enabled, created_at FROM webhook_subscriptions ORDER BY created_at ASC"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(webhooks)
    }

    pub async fn find_by_event(
        &self,
        event_name: &str,
    ) -> Result<Vec<WebhookSubscription>, AppError> {
        #[cfg(feature = "postgres")]
        {
            let webhooks = sqlx::query_as::<_, WebhookSubscription>(
                "SELECT id, url, events, secret, enabled, created_at FROM webhook_subscriptions \
                 WHERE enabled = true AND events @> $1::jsonb \
                 ORDER BY created_at ASC",
            )
            .bind(serde_json::json!([event_name]).to_string())
            .fetch_all(&self.pool)
            .await?;
            Ok(webhooks)
        }

        #[cfg(feature = "sqlite")]
        {
            let webhooks = sqlx::query_as::<_, WebhookSubscription>(
                "SELECT id, url, events, secret, enabled, created_at FROM webhook_subscriptions \
                 WHERE enabled = 1 AND events LIKE '%\"' || ? || '\"%' \
                 ORDER BY created_at ASC",
            )
            .bind(event_name)
            .fetch_all(&self.pool)
            .await?;
            Ok(webhooks)
        }
    }

    pub async fn delete(&self, id: &str) -> Result<(), AppError> {
        #[cfg(feature = "postgres")]
        let result = sqlx::query("DELETE FROM webhook_subscriptions WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        #[cfg(feature = "sqlite")]
        let result = sqlx::query("DELETE FROM webhook_subscriptions WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "Webhook subscription with id: {} not found",
                id
            )));
        }

        Ok(())
    }
}
