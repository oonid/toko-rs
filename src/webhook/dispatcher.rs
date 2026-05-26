use crate::db::DbPool;
use crate::event::models::EventOutboxRow;
use crate::webhook::repository::WebhookRepository;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub async fn dispatch_event(pool: DbPool, event: EventOutboxRow) {
    let repo = WebhookRepository::new(pool);
    let subs = match repo.find_by_event(&event.event_name).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("webhook dispatch: failed to fetch subscriptions: {e}");
            return;
        }
    };

    if subs.is_empty() {
        return;
    }

    let body = match serde_json::to_string(&event) {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!("webhook dispatch: failed to serialize event: {e}");
            return;
        }
    };

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    for sub in subs {
        let sig = compute_hmac(&sub.secret, &body);
        match client
            .post(&sub.url)
            .header("Content-Type", "application/json")
            .header("X-Toko-Signature", format!("sha256={sig}"))
            .body(body.clone())
            .send()
            .await
        {
            Ok(resp) => tracing::info!(
                subscription_id = %sub.id,
                event_id = %event.id,
                status = %resp.status(),
                "webhook delivered"
            ),
            Err(e) => tracing::warn!(
                subscription_id = %sub.id,
                event_id = %event.id,
                error = %e,
                "webhook delivery failed"
            ),
        }
    }
}

fn compute_hmac(secret: &str, body: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key size");
    mac.update(body.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}
