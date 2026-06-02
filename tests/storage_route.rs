mod common;
use axum::http::StatusCode;
use browserless_html_to_pdf::test_app;
use common::{body_json, post_json};

#[tokio::test]
async fn storage_off_returns_inline_data() {
    let res = post_json(test_app(&[]), "/v1/pdf", r#"{"html":"<h1>x</h1>"}"#).await;
    let v = body_json(res).await;
    assert!(v.get("pdf").is_some());
    assert!(v.get("downloadUrl").is_none());
}

#[tokio::test]
async fn storage_on_returns_signed_url() {
    let dir = std::env::temp_dir().join(format!("h2p-test-{}", std::process::id()));
    let app = test_app(&[
        ("STORAGE_ENABLED", "true"),
        ("OPENDAL_SCHEME", "fs"),
        ("OPENDAL_ROOT", dir.to_str().unwrap()),
        ("SIGNING_KEY", "test-key"),
        ("PUBLIC_BASE_URL", "http://localhost:3000"),
    ]);
    let res = post_json(app, "/v1/pdf", r#"{"html":"<h1>x</h1>"}"#).await;
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    assert!(v.get("pdf").is_none());
    let url = v["downloadUrl"].as_str().unwrap();
    assert!(url.contains("/downloads/pdfs/"));
    assert!(url.contains("sig="));
}
