mod common;
use axum::http::StatusCode;
use base64::Engine;
use browserless_html_to_pdf::test_app;
use common::{body_json, post_json};

#[tokio::test]
async fn returns_base64_pdf() {
    let res = post_json(
        test_app(&[]),
        "/v1/pdf",
        r#"{"html":"<html><body><h1>Hello PDF</h1></body></html>"}"#,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    assert!(v.get("filename").is_some());
    assert!(v.get("downloadUrl").is_none());
    let pdf = base64::engine::general_purpose::STANDARD
        .decode(v["pdf"].as_str().unwrap())
        .unwrap();
    assert!(pdf.starts_with(b"%PDF-"));
}

#[tokio::test]
async fn enforces_max_html_size() {
    let app = test_app(&[("MAX_HTML_BYTES", "16")]);
    let html = "a".repeat(64);
    let res = post_json(app, "/v1/pdf", &format!(r#"{{"html":"{html}"}}"#)).await;
    assert_eq!(res.status(), StatusCode::PAYLOAD_TOO_LARGE);
    let v = body_json(res).await;
    assert_eq!(v["maxBytes"], 16);
    assert_eq!(v["actualBytes"], 64);
}

#[tokio::test]
async fn rejects_invalid_options() {
    let res = post_json(
        test_app(&[]),
        "/v1/pdf",
        r#"{"html":"<h1>x</h1>","scale":3,"width":"twelve"}"#,
    )
    .await;
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let v = body_json(res).await;
    assert!(v.get("pdf").is_none());
}

#[tokio::test]
async fn sanitizes_filename() {
    let res = post_json(
        test_app(&[]),
        "/v1/pdf",
        r#"{"html":"<h1>x</h1>","filename":"My Report (final).pdf"}"#,
    )
    .await;
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    assert_eq!(v["filename"], "My_Report__final_.pdf");
}
