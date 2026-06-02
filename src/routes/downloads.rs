use axum::body::Body;
use axum::extract::{Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use crate::app::AppState;
use crate::error::AppError;
use crate::routes::pdf::now_unix;
use crate::services::storage::verify;

#[derive(Deserialize)]
pub struct SignedParams {
    expires: u64,
    sig: String,
}

pub async fn serve(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Query(p): Query<SignedParams>,
) -> Result<Response, AppError> {
    if now_unix() > p.expires {
        return Err(AppError::BadRequest("link expired".into()));
    }
    if !verify(&state.config.signing_key, &key, p.expires, &p.sig) {
        return Err(AppError::BadRequest("invalid signature".into()));
    }
    let op = state
        .operator
        .as_ref()
        .ok_or_else(|| AppError::Internal("storage not enabled".into()))?;
    let bytes = op
        .read(&key)
        .await
        .map_err(|_| AppError::Internal("not found".into()))?
        .to_vec();

    let ct = if key.ends_with(".pdf") {
        "application/pdf"
    } else if key.ends_with(".jpeg") || key.ends_with(".jpg") {
        "image/jpeg"
    } else {
        "image/png"
    };

    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, ct)],
        Body::from(bytes),
    )
        .into_response())
}
