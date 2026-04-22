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

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Validation Error: {0}")]
    ValidationError(String),

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
            AppError::DuplicateError(_) => StatusCode::UNPROCESSABLE_ENTITY,
            AppError::UnexpectedState(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::Forbidden(_) => StatusCode::FORBIDDEN,
            AppError::ValidationError(_) => StatusCode::UNPROCESSABLE_ENTITY,
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
            AppError::Conflict(_) => "conflict",
            AppError::ValidationError(_) => "invalid_data",
            AppError::Forbidden(_) => "forbidden",
            AppError::DatabaseError(_) => "database_error",
            AppError::MigrationError(_) => "database_error",
        }
    }

    fn error_code(&self) -> &str {
        match self {
            AppError::InvalidData(_) => "invalid_request_error",
            AppError::NotFound(_) => "invalid_request_error",
            AppError::DuplicateError(_) => "invalid_request_error",
            AppError::Unauthorized(_) => "unknown_error",
            AppError::UnexpectedState(_) => "invalid_state_error",
            AppError::Conflict(_) => "invalid_state_error",
            AppError::ValidationError(_) => "invalid_request_error",
            AppError::Forbidden(_) => "invalid_state_error",
            AppError::DatabaseError(_) => "api_error",
            AppError::MigrationError(_) => "api_error",
        }
    }
}

pub fn map_db_constraint(e: sqlx::Error) -> AppError {
    if crate::db::is_unique_violation(&e) {
        AppError::DuplicateError("A record with this value already exists".into())
    } else if crate::db::is_fk_violation(&e) {
        AppError::NotFound("Referenced record not found".into())
    } else if crate::db::is_not_null_violation(&e) {
        AppError::InvalidData("A required field is missing".into())
    } else if crate::db::is_serialization_failure(&e) {
        AppError::Conflict("Concurrent modification conflict. Please retry.".into())
    } else {
        AppError::DatabaseError(e)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let message = match &self {
            AppError::DatabaseError(e) => {
                tracing::error!("Database Error: {}", e);
                "Internal server error".to_string()
            }
            AppError::MigrationError(e) => {
                tracing::error!("Migration Error: {}", e);
                "Internal server error".to_string()
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
        assert_eq!(body["_status"], 422);
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
        assert_eq!(body["_status"], 500);
        assert_eq!(body["code"], "invalid_state_error");
        assert_eq!(body["type"], "unexpected_state");
    }

    #[tokio::test]
    async fn test_conflict() {
        let resp = AppError::Conflict("cart already completed".into()).into_response();
        let body = into_body(resp).await;
        assert_eq!(body["_status"], 409);
        assert_eq!(body["code"], "invalid_state_error");
        assert_eq!(body["type"], "conflict");
        assert_eq!(body["message"], "Conflict: cart already completed");
    }

    #[tokio::test]
    async fn test_forbidden() {
        let resp = AppError::Forbidden("not allowed".into()).into_response();
        let body = into_body(resp).await;
        assert_eq!(body["_status"], 403);
        assert_eq!(body["code"], "invalid_state_error");
        assert_eq!(body["type"], "forbidden");
        assert_eq!(body["message"], "Forbidden: not allowed");
    }

    #[tokio::test]
    async fn test_database_error() {
        let db_err = sqlx::Error::Configuration("cfg fail".into());
        let resp = AppError::DatabaseError(db_err).into_response();
        let body = into_body(resp).await;
        assert_eq!(body["_status"], 500);
        assert_eq!(body["code"], "api_error");
        assert_eq!(body["type"], "database_error");
        assert_eq!(body["message"], "Internal server error");
    }

    #[tokio::test]
    async fn test_migration_error() {
        let mig_err = sqlx::migrate::MigrateError::VersionMissing(42);
        let resp = AppError::MigrationError(mig_err).into_response();
        let body = into_body(resp).await;
        assert_eq!(body["_status"], 500);
        assert_eq!(body["code"], "api_error");
        assert_eq!(body["type"], "database_error");
        assert_eq!(body["message"], "Internal server error");
    }

    #[tokio::test]
    async fn test_map_db_constraint() {
        use super::map_db_constraint;
        use crate::db::TestDbError;

        let db_err = sqlx::Error::Database(Box::new(TestDbError {
            code: Some(crate::db::unique_violation_code().to_string()),
            message: "dup".into(),
            ..Default::default()
        }));
        let resp = map_db_constraint(db_err).into_response();
        let body = into_body(resp).await;
        assert_eq!(body["_status"], 422);
        assert_eq!(body["type"], "duplicate_error");

        let db_err = sqlx::Error::Database(Box::new(TestDbError {
            code: Some(crate::db::fk_violation_code().to_string()),
            message: "fk".into(),
            ..Default::default()
        }));
        let resp = map_db_constraint(db_err).into_response();
        let body = into_body(resp).await;
        assert_eq!(body["_status"], 404);
        assert_eq!(body["type"], "not_found");

        let db_err = sqlx::Error::Database(Box::new(TestDbError {
            code: Some(crate::db::not_null_violation_code().to_string()),
            message: "nn".into(),
            ..Default::default()
        }));
        let resp = map_db_constraint(db_err).into_response();
        let body = into_body(resp).await;
        assert_eq!(body["_status"], 400);
        assert_eq!(body["type"], "invalid_data");
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
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            AppError::UnexpectedState("".into()).status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            AppError::Conflict("".into()).status_code(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            AppError::Forbidden("".into()).status_code(),
            StatusCode::FORBIDDEN
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

    #[tokio::test]
    async fn test_map_db_constraint_non_db_error() {
        let e = sqlx::Error::Configuration("cfg fail".into());
        let result = map_db_constraint(e);
        assert!(matches!(result, AppError::DatabaseError(_)));
    }
}
