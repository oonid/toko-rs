use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Not Found: {0}")]
    NotFound(String),

    #[error("Invalid Data: {0}")]
    InvalidData(String),

    #[error("Duplicate Error: {0}")]
    DuplicateError(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Unexpected State: {0}")]
    UnexpectedState(String),

    #[error("Database Error: {0}")]
    DatabaseError(#[from] sqlx::Error),
}

impl AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::InvalidData(_) => StatusCode::BAD_REQUEST,
            AppError::DuplicateError(_) | AppError::UnexpectedState(_) => StatusCode::CONFLICT,
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::DatabaseError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_type(&self) -> &str {
        match self {
            AppError::NotFound(_) => "not_found",
            AppError::InvalidData(_) => "invalid_data",
            AppError::DuplicateError(_) => "duplicate_error",
            AppError::Unauthorized(_) => "unauthorized",
            AppError::UnexpectedState(_) => "unexpected_state",
            AppError::DatabaseError(_) => "database_error",
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let message = match &self {
            AppError::DatabaseError(e) => {
                // Do not leak internal DB info, but print to tracing
                tracing::error!("Database Error: {}", e);
                "Internal database error".to_string()
            }
            _ => self.to_string(), // use the Display format for other variants
        };

        let body = Json(json!({
            "type": self.error_type(),
            "message": message,
        }));

        (status, body).into_response()
    }
}
