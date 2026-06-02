use axum::Json;
use axum::response::Html;
use scalar_doc::Documentation;
use serde_json::Value;

use crate::openapi;

pub async fn scalar() -> Html<String> {
    let html = Documentation::new("Browserless HTML to PDF", "/openapi.json")
        .build()
        .unwrap_or_else(|e| format!("<pre>failed to render docs: {e}</pre>"));
    Html(html)
}

pub async fn openapi_spec() -> Json<Value> {
    Json(openapi::spec())
}
