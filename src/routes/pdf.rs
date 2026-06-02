use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use base64::Engine;

use crate::app::AppState;
use crate::error::AppError;
use crate::models::dto::{PdfInlineResponse, PdfRequest, PdfUrlResponse, sanitize_filename};
use crate::services::html_to_pdf::{PdfBuildOptions, render_html};
use crate::services::storage;

pub async fn create_pdf(
    State(state): State<AppState>,
    body: Result<Json<PdfRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<Response, AppError> {
    let Json(req) = body.map_err(AppError::from)?;

    let actual = req.html.len();
    if actual > state.config.max_html_bytes {
        return Err(AppError::html_too_large(
            state.config.max_html_bytes,
            actual,
        ));
    }

    let filename = sanitize_filename(req.filename.as_deref().unwrap_or(""));

    let opts = PdfBuildOptions {
        page: req.page,
        header: req.header,
        footer: req.footer,
    };
    let html = req.html;

    let _permit = state
        .render_limiter
        .clone()
        .acquire_owned()
        .await
        .map_err(|_| AppError::Internal("render limiter closed".into()))?;
    let pdf = tokio::task::spawn_blocking(move || render_html(&html, &opts))
        .await
        .map_err(|e| AppError::Internal(format!("render task panicked: {e}")))?
        .map_err(AppError::Validation)?;

    if let Some(op) = &state.operator {
        let key = format!("pdfs/{}/{}", uuid::Uuid::new_v4(), filename);
        storage::put(op, &key, pdf)
            .await
            .map_err(AppError::Internal)?;
        let now = now_unix();
        let url = storage::signed_url(
            op,
            &state.config.scheme,
            &state.config.public_base_url,
            &state.config.signing_key,
            &key,
            state.config.url_ttl_secs,
            now,
        )
        .await
        .map_err(AppError::Internal)?;
        Ok(Json(PdfUrlResponse {
            filename,
            download_url: url,
        })
        .into_response())
    } else {
        let b64 = base64::engine::general_purpose::STANDARD.encode(&pdf);
        Ok(Json(PdfInlineResponse { filename, pdf: b64 }).into_response())
    }
}

pub(crate) fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
