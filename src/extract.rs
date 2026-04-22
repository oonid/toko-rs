use axum::extract::rejection::JsonRejection;

pub struct Json<T>(pub T);

impl<T: serde::de::DeserializeOwned, S: Send + Sync> axum::extract::FromRequest<S> for Json<T> {
    type Rejection = crate::error::AppError;

    async fn from_request(req: axum::extract::Request, state: &S) -> Result<Self, Self::Rejection> {
        match axum::Json::<T>::from_request(req, state).await {
            Ok(axum::Json(value)) => Ok(Self(value)),
            Err(rejection) => Err(map_json_rejection(rejection)),
        }
    }
}

fn map_json_rejection(rejection: JsonRejection) -> crate::error::AppError {
    match rejection {
        JsonRejection::JsonSyntaxError(msg) => {
            crate::error::AppError::InvalidData(format!("JSON syntax error: {}", msg))
        }
        JsonRejection::MissingJsonContentType(_) => crate::error::AppError::InvalidData(
            "Request must have Content-Type: application/json".into(),
        ),
        JsonRejection::JsonDataError(msg) => {
            crate::error::AppError::ValidationError(format!("{}", msg))
        }
        _ => crate::error::AppError::InvalidData(format!("Invalid request body: {}", rejection)),
    }
}
