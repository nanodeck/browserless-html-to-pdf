use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug)]
pub enum AppError {
    BadRequest(String),
    PayloadTooLarge { max: usize, actual: usize },
    Validation(String),
    NotFound(String),
    Internal(String),
}

impl AppError {
    pub fn html_too_large(max: usize, actual: usize) -> Self {
        AppError::PayloadTooLarge { max, actual }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::BadRequest(m) => {
                (StatusCode::BAD_REQUEST, Json(json!({ "error": m }))).into_response()
            }
            AppError::PayloadTooLarge { max, actual } => (
                StatusCode::PAYLOAD_TOO_LARGE,
                Json(json!({ "error": "HTML payload too large", "maxBytes": max, "actualBytes": actual })),
            )
                .into_response(),
            AppError::Validation(m) => {
                (StatusCode::UNPROCESSABLE_ENTITY, Json(json!({ "error": m }))).into_response()
            }
            AppError::NotFound(m) => {
                (StatusCode::NOT_FOUND, Json(json!({ "error": m }))).into_response()
            }
            AppError::Internal(m) => {
                eprintln!("internal error: {m}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "internal server error" })),
                )
                    .into_response()
            }
        }
    }
}

impl From<axum::extract::rejection::JsonRejection> for AppError {
    fn from(rej: axum::extract::rejection::JsonRejection) -> Self {
        use axum::extract::rejection::JsonRejection::*;
        match rej {
            JsonDataError(e) => AppError::Validation(e.body_text()),
            JsonSyntaxError(e) => AppError::BadRequest(e.body_text()),
            BytesRejection(e) => AppError::BadRequest(e.body_text()),
            other => AppError::BadRequest(other.body_text()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    #[test]
    fn maps_status_codes() {
        assert_eq!(
            AppError::Validation("x".into()).into_response().status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            AppError::html_too_large(10, 5).into_response().status(),
            StatusCode::PAYLOAD_TOO_LARGE
        );
        assert_eq!(
            AppError::BadRequest("x".into()).into_response().status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            AppError::Internal("x".into()).into_response().status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            AppError::NotFound("x".into()).into_response().status(),
            StatusCode::NOT_FOUND
        );
    }

    #[tokio::test]
    async fn internal_errors_do_not_leak_details() {
        let res = AppError::Internal("secret backend detail".into()).into_response();
        let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = String::from_utf8(bytes.to_vec()).unwrap();
        assert!(!body.contains("secret backend detail"));
    }
}
