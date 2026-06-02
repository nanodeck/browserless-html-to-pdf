mod common;
use axum::http::StatusCode;
use browserless_html_to_pdf::test_app;
use common::{body_json, body_text, get};

#[tokio::test]
async fn health_returns_ok() {
    let res = get(test_app(&[]), "/health").await;
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    assert_eq!(v["status"], "ok");
}

#[tokio::test]
async fn home_renders_docs_ui() {
    let res = get(test_app(&[]), "/").await;
    assert_eq!(res.status(), StatusCode::OK);
    let html = body_text(res).await;
    assert!(html.contains("HTML to PDF"));
}

#[tokio::test]
async fn openapi_spec_lists_paths() {
    let res = get(test_app(&[]), "/openapi.json").await;
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    assert!(v["paths"]["/v1/pdf"].is_object());
    assert!(v["paths"]["/v1/images"].is_object());
}
