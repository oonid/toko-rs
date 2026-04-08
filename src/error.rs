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

    #[error("Migration Error: {0}")]
    MigrationError(#[from] sqlx::migrate::MigrateError),
}

impl AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::InvalidData(_) => StatusCode::BAD_REQUEST,
            AppError::DuplicateError(_) | AppError::UnexpectedState(_) => StatusCode::CONFLICT,
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::DatabaseError(_) | AppError::MigrationError(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
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
            AppError::MigrationError(_) => "migration_error",
        }
    }

    fn error_code(&self) -> &str {
        match self {
            AppError::InvalidData(_) => "invalid_request_error",
            AppError::NotFound(_) => "invalid_request_error",
            AppError::DuplicateError(_) => "invalid_request_error",
            AppError::Unauthorized(_) => "unknown_error",
            AppError::UnexpectedState(_) => "invalid_state_error",
            AppError::DatabaseError(_) => "api_error",
            AppError::MigrationError(_) => "api_error",
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let message = match &self {
            AppError::DatabaseError(e) => {
                tracing::error!("Database Error: {}", e);
                e.to_string()
            }
            AppError::MigrationError(e) => {
                tracing::error!("Migration Error: {}", e);
                e.to_string()
            }
            _ => self.to_string(),
        };

        let body = Json(json!({
            "code": self.error_code(),
            "type": self.error_type(),
            "message": message,
        }));

        (status, body).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn into_body(resp: Response) -> serde_json::Value {
        let (parts, body) = resp.into_parts();
        let bytes = axum::body::to_bytes(body, usize::MAX).await.unwrap();
        let mut val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        val.as_object_mut().unwrap().insert(
            "_status".into(),
            serde_json::Value::Number(parts.status.as_u16().into()),
        );
        val
    }

    #[tokio::test]
    async fn test_not_found() {
        let resp = AppError::NotFound("gone".into()).into_response();
        let body = into_body(resp).await;
        assert_eq!(body["_status"], 404);
        assert_eq!(body["code"], "invalid_request_error");
        assert_eq!(body["type"], "not_found");
        assert_eq!(body["message"], "Not Found: gone");
    }

    #[tokio::test]
    async fn test_invalid_data() {
        let resp = AppError::InvalidData("bad".into()).into_response();
        let body = into_body(resp).await;
        assert_eq!(body["_status"], 400);
        assert_eq!(body["code"], "invalid_request_error");
        assert_eq!(body["type"], "invalid_data");
    }

    #[tokio::test]
    async fn test_duplicate_error() {
        let resp = AppError::DuplicateError("dup".into()).into_response();
        let body = into_body(resp).await;
        assert_eq!(body["_status"], 409);
        assert_eq!(body["code"], "invalid_request_error");
        assert_eq!(body["type"], "duplicate_error");
    }

    #[tokio::test]
    async fn test_unauthorized() {
        let resp = AppError::Unauthorized("nope".into()).into_response();
        let body = into_body(resp).await;
        assert_eq!(body["_status"], 401);
        assert_eq!(body["code"], "unknown_error");
        assert_eq!(body["type"], "unauthorized");
    }

    #[tokio::test]
    async fn test_unexpected_state() {
        let resp = AppError::UnexpectedState("bad state".into()).into_response();
        let body = into_body(resp).await;
        assert_eq!(body["_status"], 409);
        assert_eq!(body["code"], "invalid_state_error");
        assert_eq!(body["type"], "unexpected_state");
    }

    #[tokio::test]
    async fn test_database_error() {
        let db_err = sqlx::Error::Configuration("cfg fail".into());
        let resp = AppError::DatabaseError(db_err).into_response();
        let body = into_body(resp).await;
        assert_eq!(body["_status"], 500);
        assert_eq!(body["code"], "api_error");
        assert_eq!(body["type"], "database_error");
    }

    #[tokio::test]
    async fn test_migration_error() {
        let mig_err = sqlx::migrate::MigrateError::VersionMissing(42);
        let resp = AppError::MigrationError(mig_err).into_response();
        let body = into_body(resp).await;
        assert_eq!(body["_status"], 500);
        assert_eq!(body["code"], "api_error");
        assert_eq!(body["type"], "migration_error");
    }

    #[test]
    fn test_status_codes() {
        assert_eq!(
            AppError::NotFound("".into()).status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            AppError::InvalidData("".into()).status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            AppError::DuplicateError("".into()).status_code(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            AppError::UnexpectedState("".into()).status_code(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            AppError::Unauthorized("".into()).status_code(),
            StatusCode::UNAUTHORIZED
        );
        let db_err = sqlx::Error::Configuration("x".into());
        assert_eq!(
            AppError::DatabaseError(db_err).status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        let mig_err = sqlx::migrate::MigrateError::VersionMissing(1);
        assert_eq!(
            AppError::MigrationError(mig_err).status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}
