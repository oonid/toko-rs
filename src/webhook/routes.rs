use super::types::{CreateWebhookInput, WebhookListResponse, WebhookResponse};
use crate::{error::AppError, AppState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

#[tracing::instrument(skip_all)]
pub async fn admin_create_webhook(
    State(state): State<AppState>,
    Json(input): Json<CreateWebhookInput>,
) -> Result<Json<WebhookResponse>, AppError> {
    let webhook = state.repos.webhook.create(&input).await?;
    Ok(Json(WebhookResponse { webhook }))
}

#[tracing::instrument(skip_all)]
pub async fn admin_list_webhooks(
    State(state): State<AppState>,
) -> Result<Json<WebhookListResponse>, AppError> {
    let webhooks = state.repos.webhook.list_all().await?;
    let count = webhooks.len() as i64;
    Ok(Json(WebhookListResponse { webhooks, count }))
}

#[tracing::instrument(skip_all)]
pub async fn admin_delete_webhook(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    state.repos.webhook.delete(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}
