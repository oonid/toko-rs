use super::models::Invoice;
use super::types::*;
use crate::error::AppError;
use crate::AppState;
use axum::{
    extract::{Path, State},
    routing::get,
    Json, Router,
};

pub fn admin_router() -> Router<AppState> {
    Router::new()
        .route("/admin/invoice-config", get(admin_get_config).post(admin_update_config))
        .route("/admin/orders/{id}/invoice", get(admin_get_invoice))
}

#[tracing::instrument(skip_all)]
async fn admin_get_config(
    State(state): State<AppState>,
) -> Result<Json<InvoiceConfigResponse>, AppError> {
    let config = state.repos.invoice.get_config().await?;
    Ok(Json(InvoiceConfigResponse { invoice_config: config }))
}

#[tracing::instrument(skip_all)]
async fn admin_update_config(
    State(state): State<AppState>,
    Json(payload): Json<UpdateInvoiceConfigInput>,
) -> Result<Json<InvoiceConfigResponse>, AppError> {
    let config = state
        .repos
        .invoice
        .upsert_config(
            payload.company_name,
            payload.company_address,
            payload.company_phone,
            payload.company_email,
            payload.company_logo,
            payload.notes,
        )
        .await?;
    Ok(Json(InvoiceConfigResponse { invoice_config: config }))
}

#[tracing::instrument(skip_all, fields(id = %id))]
async fn admin_get_invoice(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<InvoiceResponse>, AppError> {
    let config = state.repos.invoice.get_config().await?;
    let order = state.repos.order.find_by_id(&id).await?;
    let invoice = Invoice::from_order(&config, order);
    Ok(Json(InvoiceResponse { invoice }))
}
