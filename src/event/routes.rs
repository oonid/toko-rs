use super::types::{EventListParams, EventListResponse, EventResponse};
use crate::{error::AppError, AppState};
use axum::{
    extract::{Path, Query, State},
    Json,
};

#[tracing::instrument(skip_all, fields(limit = ?params.limit, offset = ?params.offset))]
pub async fn admin_list_events(
    State(state): State<AppState>,
    Query(params): Query<EventListParams>,
) -> Result<Json<EventListResponse>, AppError> {
    let limit = params.limit.unwrap_or(50).min(100);
    let offset = params.offset.unwrap_or(0);
    let (events, count) = state.repos.event.list_all(&params).await?;
    Ok(Json(EventListResponse {
        events,
        count,
        limit,
        offset,
    }))
}

#[tracing::instrument(skip_all, fields(id = %id))]
pub async fn admin_mark_event_processed(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<EventResponse>, AppError> {
    let event = state.repos.event.mark_processed(&id).await?;
    Ok(Json(EventResponse { event }))
}
