use axum::Json;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::header::CONTENT_TYPE;
use axum::response::{IntoResponse, Response};
use base64::Engine;

use crate::app::AppState;
use crate::error::AppError;
use crate::models::dto::{ImageFormat, ImageItem, ImageRequest, ImagesResponse, parse_pages};
use crate::routes::pdf::now_unix;
use crate::services::pdf_to_image::{RenderOptions, render_pdf};
use crate::services::storage;

pub async fn create_images(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, AppError> {
    let ct = headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let (pdf_bytes, format, scale, pages_spec) = if ct.starts_with("multipart/form-data") {
        parse_multipart(&state, ct, body).await?
    } else {
        let req: ImageRequest = serde_json::from_slice(&body)
            .map_err(|e| AppError::Validation(format!("invalid body: {e}")))?;
        req.validate().map_err(AppError::Validation)?;
        let pdf = base64::engine::general_purpose::STANDARD
            .decode(req.pdf_base64.as_bytes())
            .map_err(|e| AppError::Validation(format!("invalid base64: {e}")))?;
        (pdf, req.format, req.scale_factor(), req.pages)
    };

    let max_pages = state.config.max_image_pages;
    let spec_empty = pages_spec.trim().is_empty();

    let render = move || -> Result<Vec<crate::services::pdf_to_image::RenderedImage>, String> {
        let pdf = hayro::hayro_syntax::Pdf::new(pdf_bytes.clone())
            .map_err(|e| format!("failed to parse PDF: {e:?}"))?;
        let total = pdf.pages().len();
        let selected = parse_pages(&pages_spec, total)?;
        let selected = if spec_empty {
            selected.into_iter().take(max_pages).collect()
        } else if selected.len() > max_pages {
            return Err(format!(
                "too many pages requested: {} (max {max_pages})",
                selected.len()
            ));
        } else {
            selected
        };
        render_pdf(
            pdf_bytes,
            &RenderOptions {
                format,
                scale,
                pages: selected,
            },
        )
    };

    let images = tokio::task::spawn_blocking(render)
        .await
        .map_err(|e| AppError::Internal(format!("render task panicked: {e}")))?
        .map_err(AppError::Validation)?;

    let mut items = Vec::with_capacity(images.len());
    if let Some(op) = &state.operator {
        let folder = uuid::Uuid::new_v4();
        let now = now_unix();
        for img in images {
            let key = format!("images/{folder}/page-{}.{}", img.page, format.ext());
            storage::put(op, &key, img.bytes)
                .await
                .map_err(AppError::Internal)?;
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
            items.push(ImageItem {
                page: img.page,
                width: img.width,
                height: img.height,
                format,
                data: None,
                download_url: Some(url),
            });
        }
    } else {
        for img in images {
            let b64 = base64::engine::general_purpose::STANDARD.encode(&img.bytes);
            items.push(ImageItem {
                page: img.page,
                width: img.width,
                height: img.height,
                format,
                data: Some(b64),
                download_url: None,
            });
        }
    }

    Ok(Json(ImagesResponse { images: items }).into_response())
}

async fn parse_multipart(
    _state: &AppState,
    ct: &str,
    body: Bytes,
) -> Result<(Vec<u8>, ImageFormat, f32, String), AppError> {
    let boundary = multer_boundary(ct)
        .ok_or_else(|| AppError::BadRequest("missing multipart boundary".into()))?;
    let mut mp = multer::Multipart::new(axum::body::Body::from(body).into_data_stream(), boundary);

    let mut pdf: Option<Vec<u8>> = None;
    let mut format = ImageFormat::Png;
    let mut scale: Option<f32> = None;
    let mut pages = String::new();

    while let Some(field) = mp
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("multipart error: {e}")))?
    {
        match field.name().unwrap_or("").to_string().as_str() {
            "file" => {
                pdf = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|e| AppError::BadRequest(format!("file read: {e}")))?
                        .to_vec(),
                )
            }
            "format" => {
                let v = field.text().await.unwrap_or_default();
                format = if v.eq_ignore_ascii_case("jpeg") {
                    ImageFormat::Jpeg
                } else {
                    ImageFormat::Png
                };
            }
            "scale" => scale = field.text().await.ok().and_then(|v| v.parse().ok()),
            "pages" => pages = field.text().await.unwrap_or_default(),
            _ => {}
        }
    }

    let factor = scale.unwrap_or(1.0);
    let pdf = pdf.ok_or_else(|| AppError::Validation("multipart `file` part required".into()))?;
    Ok((pdf, format, factor, pages))
}

fn multer_boundary(ct: &str) -> Option<String> {
    ct.split(';')
        .filter_map(|p| p.trim().strip_prefix("boundary="))
        .map(|b| b.trim_matches('"').to_string())
        .next()
}
