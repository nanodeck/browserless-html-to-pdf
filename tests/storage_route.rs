mod common;
use axum::http::StatusCode;
use browserless_html_to_pdf::{services::storage, test_app};
use common::{body_json, get, post_json};

const TEST_SIGNING_KEY: &str = "0123456789abcdef0123456789abcdef";

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
        ("SIGNING_KEY", TEST_SIGNING_KEY),
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

#[tokio::test]
async fn missing_download_returns_not_found() {
    let dir = std::env::temp_dir().join(format!("h2p-test-missing-{}", std::process::id()));
    let app = test_app(&[
        ("STORAGE_ENABLED", "true"),
        ("OPENDAL_SCHEME", "fs"),
        ("OPENDAL_ROOT", dir.to_str().unwrap()),
        ("SIGNING_KEY", TEST_SIGNING_KEY),
    ]);
    let key = "pdfs/missing/report.pdf";
    let expires = crate::common::future_expires();
    let sig = storage::sign(TEST_SIGNING_KEY.as_bytes(), key, expires);
    let res = get(
        app,
        &format!("/downloads/{key}?expires={expires}&sig={sig}"),
    )
    .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
